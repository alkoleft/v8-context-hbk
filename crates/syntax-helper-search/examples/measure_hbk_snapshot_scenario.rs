use std::fs;
use std::hint::black_box;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, process};

use serde::Serialize;
use syntax_helper_search::{
    HbkFactReadHandle, HbkFactRef, HbkFactSnapshot, HbkFactSnapshotCacheStatus,
    HbkFactSnapshotCounts, HbkFactSnapshotStageTimings, HbkGlobalFactKind, HbkLanguageDomain,
    HbkPlatformTypeId, HbkQueryFieldId, HbkQueryTableId, HbkSnapshotExperimentAllocationDelta,
    HbkSnapshotExperimentAllocationSnapshot, HbkSnapshotExperimentAllocator, HbkTypeMemberKind,
    experiment_allocation_snapshot,
};

#[global_allocator]
static ALLOCATOR: HbkSnapshotExperimentAllocator = HbkSnapshotExperimentAllocator;

const REPORT_SCHEMA_VERSION: &str = "hbk-snapshot-benchmark/v1";
const WORKLOAD_VERSION: &str = "hbk-snapshot-warm-lookups/v1";
const PREPARE_PHASE_ORDER: &[&str] = &["open", "cache_write"];
const LOADED_PHASE_ORDER: &[&str] = &[
    "entry_to_ready",
    "first_lookup_including_handle",
    "anchor_resolution",
    "warmup",
    "workload",
];
const DEFAULT_ITERATIONS: usize = 20_000;
const FIRST_LOOKUP_ID: &str = "platform_type:Запрос";
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn main() {
    let parent_start_unix_ns = parent_start_unix_ns();
    let entry_allocations = experiment_allocation_snapshot();
    let entry_started_at = Instant::now();
    let entry_faults = match read_process_faults() {
        Ok(faults) => faults,
        Err(error) => {
            eprintln!("failed to read initial process counters: {error}");
            process::exit(1);
        }
    };

    match run(
        entry_started_at,
        entry_faults,
        entry_allocations,
        parent_start_unix_ns,
    ) {
        Ok(report) => {
            if let Err(error) = serde_json::to_writer(io::stdout().lock(), &report) {
                eprintln!("failed to write benchmark report: {error}");
                process::exit(1);
            }
            println!();
        }
        Err(error) => {
            eprintln!("benchmark scenario failed: {error}");
            process::exit(1);
        }
    }
}

fn run(
    entry_started_at: Instant,
    entry_faults: ProcessFaults,
    entry_allocations: HbkSnapshotExperimentAllocationSnapshot,
    parent_start_unix_ns: Option<u128>,
) -> Result<BenchmarkReport, Box<dyn std::error::Error>> {
    let hold = HoldConfig::from_env()?;
    match parse_args(env::args().skip(1))? {
        Command::PrepareCache {
            index_path,
            cache_path,
            iterations,
        } => prepare_cache(
            entry_started_at,
            entry_faults,
            index_path,
            cache_path,
            iterations,
            hold,
            entry_allocations,
            parent_start_unix_ns,
        ),
        Command::SqlOwned {
            index_path,
            iterations,
        } => measure_sql_owned(
            entry_started_at,
            entry_faults,
            index_path,
            iterations,
            hold,
            entry_allocations,
            parent_start_unix_ns,
        ),
        Command::CacheOwned {
            index_path,
            cache_path,
            iterations,
        } => measure_cache_owned(
            entry_started_at,
            entry_faults,
            index_path,
            cache_path,
            iterations,
            hold,
            entry_allocations,
            parent_start_unix_ns,
        ),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Command {
    PrepareCache {
        index_path: PathBuf,
        cache_path: PathBuf,
        iterations: usize,
    },
    SqlOwned {
        index_path: PathBuf,
        iterations: usize,
    },
    CacheOwned {
        index_path: PathBuf,
        cache_path: PathBuf,
        iterations: usize,
    },
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, io::Error> {
    let mut args = args.into_iter();
    let mode = args.next().ok_or_else(usage_error)?;
    let command = match mode.as_str() {
        "prepare-cache" => {
            let index_path = required_path(&mut args)?;
            let cache_path = required_path(&mut args)?;
            let iterations = optional_iterations(&mut args)?;
            Command::PrepareCache {
                index_path,
                cache_path,
                iterations,
            }
        }
        "sql-owned" => {
            let index_path = required_path(&mut args)?;
            let iterations = optional_iterations(&mut args)?;
            Command::SqlOwned {
                index_path,
                iterations,
            }
        }
        "cache-owned" => {
            let index_path = required_path(&mut args)?;
            let cache_path = required_path(&mut args)?;
            let iterations = optional_iterations(&mut args)?;
            Command::CacheOwned {
                index_path,
                cache_path,
                iterations,
            }
        }
        _ => return Err(usage_error()),
    };
    if args.next().is_some() {
        return Err(usage_error());
    }
    Ok(command)
}

fn required_path(args: &mut impl Iterator<Item = String>) -> Result<PathBuf, io::Error> {
    args.next().map(PathBuf::from).ok_or_else(usage_error)
}

fn optional_iterations(args: &mut impl Iterator<Item = String>) -> Result<usize, io::Error> {
    let Some(value) = args.next() else {
        return Ok(DEFAULT_ITERATIONS);
    };
    let iterations = value.parse::<usize>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid iteration count {value:?}: {error}"),
        )
    })?;
    if iterations == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "iteration count must be greater than zero",
        ));
    }
    Ok(iterations)
}

fn usage_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: measure_hbk_snapshot_scenario \
         prepare-cache <index.sqlite> <cache.bin> [iterations] | \
         sql-owned <index.sqlite> [iterations] | \
         cache-owned <index.sqlite> <cache.bin> [iterations]",
    )
}

fn prepare_cache(
    entry_started_at: Instant,
    entry_faults: ProcessFaults,
    index_path: PathBuf,
    cache_path: PathBuf,
    iterations: usize,
    hold: HoldConfig,
    entry_allocations: HbkSnapshotExperimentAllocationSnapshot,
    parent_start_unix_ns: Option<u128>,
) -> Result<BenchmarkReport, Box<dyn std::error::Error>> {
    eprintln!("preparing owned binary cache from {}", index_path.display());
    let open_started_at = Instant::now();
    let build_report = HbkFactSnapshot::from_path_with_stage_timings(&index_path)?;
    let open_elapsed = open_started_at.elapsed();
    let after_open_faults = read_process_faults()?;
    let after_open_smaps = read_smaps_rollup()?;
    let build_timings = build_report.timings;

    let write_started_at = Instant::now();
    build_report.write_binary_cache(&cache_path)?;
    let cache_write_elapsed = write_started_at.elapsed();
    let ready_elapsed = entry_started_at.elapsed();
    let process_start_to_ready_ns = capture_process_start_to_ready_ns(parent_start_unix_ns);
    let ready_allocations = experiment_allocation_snapshot();
    let after_ready_faults = read_process_faults()?;
    let after_ready_smaps = read_smaps_rollup()?;

    let snapshot = snapshot_report(&build_report.snapshot);
    let index = artifact_report(&index_path)?;
    let cache = artifact_report(&cache_path)?;
    let hold_report = hold.activate()?;
    let final_allocations = experiment_allocation_snapshot();
    Ok(BenchmarkReport {
        schema_version: REPORT_SCHEMA_VERSION,
        workload_version: WORKLOAD_VERSION,
        mode: "prepare-cache",
        backend: "sql-owned",
        iterations,
        index,
        cache: Some(cache),
        cache_status: Some("written"),
        timings: TimingReport {
            phase_order: PREPARE_PHASE_ORDER,
            process_start_to_ready_ns,
            entry_to_ready_ns: duration_ns(ready_elapsed),
            open: phase_report(open_elapsed, entry_faults.delta_to(after_open_faults)),
            cache_write: Some(phase_report(
                cache_write_elapsed,
                after_open_faults.delta_to(after_ready_faults),
            )),
            first_lookup: None,
            anchor_resolution: None,
            warmup_ns: None,
            workload: None,
        },
        smaps: SmapsReport {
            after_open: after_open_smaps,
            after_workload: after_ready_smaps,
        },
        snapshot,
        build_stages: Some(stage_timing_report(build_timings)),
        hold: hold_report,
        allocations: AllocationEvidenceReport {
            enabled: cfg!(feature = "snapshot-experiment-alloc"),
            entry_to_ready: ready_allocations.delta_since(entry_allocations).into(),
            first_lookup: None,
            anchor_resolution: None,
            workload: None,
            final_snapshot: final_allocations.into(),
        },
    })
}

fn measure_sql_owned(
    entry_started_at: Instant,
    entry_faults: ProcessFaults,
    index_path: PathBuf,
    iterations: usize,
    hold: HoldConfig,
    entry_allocations: HbkSnapshotExperimentAllocationSnapshot,
    parent_start_unix_ns: Option<u128>,
) -> Result<BenchmarkReport, Box<dyn std::error::Error>> {
    eprintln!("measuring SQL-owned snapshot from {}", index_path.display());
    let build_report = HbkFactSnapshot::from_path_with_stage_timings(&index_path)?;
    let ready_elapsed = entry_started_at.elapsed();
    let process_start_to_ready_ns = capture_process_start_to_ready_ns(parent_start_unix_ns);
    let ready_allocations = experiment_allocation_snapshot();
    let ready_faults = read_process_faults()?;
    let after_open_smaps = read_smaps_rollup()?;
    let build_timings = build_report.timings;
    let snapshot = build_report.snapshot;
    let scenario = measure_loaded_snapshot(&snapshot, ready_faults, iterations)?;

    let snapshot_report = snapshot_report(&snapshot);
    let index = artifact_report(&index_path)?;
    let hold_report = hold.activate()?;
    let final_allocations = experiment_allocation_snapshot();
    Ok(BenchmarkReport {
        schema_version: REPORT_SCHEMA_VERSION,
        workload_version: WORKLOAD_VERSION,
        mode: "sql-owned",
        backend: "sql-owned",
        iterations,
        index,
        cache: None,
        cache_status: None,
        timings: TimingReport {
            phase_order: LOADED_PHASE_ORDER,
            process_start_to_ready_ns,
            entry_to_ready_ns: duration_ns(ready_elapsed),
            open: phase_report(ready_elapsed, entry_faults.delta_to(ready_faults)),
            cache_write: None,
            first_lookup: Some(scenario.first_lookup),
            anchor_resolution: Some(scenario.anchor_resolution),
            warmup_ns: Some(scenario.warmup_ns),
            workload: Some(scenario.workload),
        },
        smaps: SmapsReport {
            after_open: after_open_smaps,
            after_workload: scenario.after_workload_smaps,
        },
        snapshot: snapshot_report,
        build_stages: Some(stage_timing_report(build_timings)),
        hold: hold_report,
        allocations: AllocationEvidenceReport {
            enabled: cfg!(feature = "snapshot-experiment-alloc"),
            entry_to_ready: ready_allocations.delta_since(entry_allocations).into(),
            first_lookup: Some(scenario.first_allocations.into()),
            anchor_resolution: Some(scenario.anchor_allocations.into()),
            workload: Some(scenario.workload_allocations.into()),
            final_snapshot: final_allocations.into(),
        },
    })
}

fn measure_cache_owned(
    entry_started_at: Instant,
    entry_faults: ProcessFaults,
    index_path: PathBuf,
    cache_path: PathBuf,
    iterations: usize,
    hold: HoldConfig,
    entry_allocations: HbkSnapshotExperimentAllocationSnapshot,
    parent_start_unix_ns: Option<u128>,
) -> Result<BenchmarkReport, Box<dyn std::error::Error>> {
    eprintln!(
        "measuring owned cache snapshot {} against {}",
        cache_path.display(),
        index_path.display()
    );
    let load_report = HbkFactSnapshot::from_path_with_binary_cache(&index_path, &cache_path)?;
    let ready_elapsed = entry_started_at.elapsed();
    let process_start_to_ready_ns = capture_process_start_to_ready_ns(parent_start_unix_ns);
    let ready_allocations = experiment_allocation_snapshot();
    let ready_faults = read_process_faults()?;
    let after_open_smaps = read_smaps_rollup()?;
    if let HbkFactSnapshotCacheStatus::Rebuilt { reason } = &load_report.status {
        return Err(io::Error::other(format!(
            "cache-owned requires status Loaded, but the cache was rebuilt: {reason}"
        ))
        .into());
    }
    let snapshot = load_report.snapshot;
    let scenario = measure_loaded_snapshot(&snapshot, ready_faults, iterations)?;

    let snapshot_report = snapshot_report(&snapshot);
    let index = artifact_report(&index_path)?;
    let cache = artifact_report(&cache_path)?;
    let hold_report = hold.activate()?;
    let final_allocations = experiment_allocation_snapshot();
    Ok(BenchmarkReport {
        schema_version: REPORT_SCHEMA_VERSION,
        workload_version: WORKLOAD_VERSION,
        mode: "cache-owned",
        backend: "cache-owned",
        iterations,
        index,
        cache: Some(cache),
        cache_status: Some("loaded"),
        timings: TimingReport {
            phase_order: LOADED_PHASE_ORDER,
            process_start_to_ready_ns,
            entry_to_ready_ns: duration_ns(ready_elapsed),
            open: phase_report(ready_elapsed, entry_faults.delta_to(ready_faults)),
            cache_write: None,
            first_lookup: Some(scenario.first_lookup),
            anchor_resolution: Some(scenario.anchor_resolution),
            warmup_ns: Some(scenario.warmup_ns),
            workload: Some(scenario.workload),
        },
        smaps: SmapsReport {
            after_open: after_open_smaps,
            after_workload: scenario.after_workload_smaps,
        },
        snapshot: snapshot_report,
        build_stages: None,
        hold: hold_report,
        allocations: AllocationEvidenceReport {
            enabled: cfg!(feature = "snapshot-experiment-alloc"),
            entry_to_ready: ready_allocations.delta_since(entry_allocations).into(),
            first_lookup: Some(scenario.first_allocations.into()),
            anchor_resolution: Some(scenario.anchor_allocations.into()),
            workload: Some(scenario.workload_allocations.into()),
            final_snapshot: final_allocations.into(),
        },
    })
}

#[derive(Clone)]
struct HoldConfig {
    duration: Duration,
    ready_file: Option<PathBuf>,
}

impl HoldConfig {
    fn from_env() -> Result<Self, io::Error> {
        let duration_ms = env::var("HBK_BENCH_HOLD_MS")
            .ok()
            .map(|value| {
                value.parse::<u64>().map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid HBK_BENCH_HOLD_MS {value:?}: {error}"),
                    )
                })
            })
            .transpose()?
            .unwrap_or(0);
        let ready_file = env::var_os("HBK_BENCH_READY_FILE")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        Ok(Self {
            duration: Duration::from_millis(duration_ms),
            ready_file,
        })
    }

    fn activate(&self) -> Result<HoldReport, io::Error> {
        if let Some(path) = &self.ready_file {
            fs::write(path, process::id().to_string())?;
        }
        if !self.duration.is_zero() {
            std::thread::sleep(self.duration);
        }
        Ok(HoldReport {
            requested_ms: self.duration.as_millis() as u64,
            ready_file: self
                .ready_file
                .as_ref()
                .map(|path| path.display().to_string()),
        })
    }
}

struct LoadedScenario {
    first_lookup: LookupPhaseReport,
    first_allocations: HbkSnapshotExperimentAllocationDelta,
    anchor_resolution: LookupPhaseReport,
    anchor_allocations: HbkSnapshotExperimentAllocationDelta,
    warmup_ns: u64,
    workload: WorkloadReport,
    workload_allocations: HbkSnapshotExperimentAllocationDelta,
    after_workload_smaps: SmapsSample,
}

fn measure_loaded_snapshot(
    snapshot: &HbkFactSnapshot,
    ready_faults: ProcessFaults,
    iterations: usize,
) -> Result<LoadedScenario, io::Error> {
    // Handle creation belongs to the first usable read path. Candidate
    // implementations must not move lazy mapping/validation outside this
    // phase.
    let before_first_allocations = experiment_allocation_snapshot();
    let first_started_at = Instant::now();
    let handle = snapshot.worker_handle();
    let first_observed = black_box(handle.facts_by_id(FIRST_LOOKUP_ID).len());
    let first_elapsed = first_started_at.elapsed();
    let after_first_allocations = experiment_allocation_snapshot();
    let after_first_faults = read_process_faults()?;
    let first_lookup = LookupPhaseReport {
        operation: "exact_fact_id",
        key: FIRST_LOOKUP_ID,
        elapsed_ns: duration_ns(first_elapsed),
        faults: ready_faults.delta_to(after_first_faults),
        observed: first_observed as u64,
        checksum: checksum_observation("exact_fact_id", first_observed as u64),
    };

    let before_anchor_faults = read_process_faults()?;
    let before_anchor_allocations = experiment_allocation_snapshot();
    let anchor_started_at = Instant::now();
    let anchors = WorkloadAnchors::resolve(handle);
    let anchor_elapsed = anchor_started_at.elapsed();
    let after_anchor_allocations = experiment_allocation_snapshot();
    let after_anchor_faults = read_process_faults()?;
    let anchor_observed = anchors.observed_count();
    let anchor_resolution = LookupPhaseReport {
        operation: "workload_anchor_resolution",
        key: WORKLOAD_VERSION,
        elapsed_ns: duration_ns(anchor_elapsed),
        faults: before_anchor_faults.delta_to(after_anchor_faults),
        observed: anchor_observed,
        checksum: checksum_observation("workload_anchor_resolution", anchor_observed),
    };

    let warmup_started_at = Instant::now();
    let _ = black_box(measure_workload(handle, anchors, 1));
    let warmup_ns = duration_ns(warmup_started_at.elapsed());

    let workload_start_faults = read_process_faults()?;
    let before_workload_allocations = experiment_allocation_snapshot();
    let workload_started_at = Instant::now();
    let mut workload = measure_workload(handle, anchors, iterations);
    workload.elapsed_ns = duration_ns(workload_started_at.elapsed());
    let after_workload_allocations = experiment_allocation_snapshot();
    let after_workload_faults = read_process_faults()?;
    workload.faults = workload_start_faults.delta_to(after_workload_faults);

    Ok(LoadedScenario {
        first_lookup,
        first_allocations: after_first_allocations.delta_since(before_first_allocations),
        anchor_resolution,
        anchor_allocations: after_anchor_allocations.delta_since(before_anchor_allocations),
        warmup_ns,
        workload,
        workload_allocations: after_workload_allocations.delta_since(before_workload_allocations),
        after_workload_smaps: read_smaps_rollup()?,
    })
}

#[derive(Clone, Copy)]
struct WorkloadAnchors {
    query_type: Option<HbkPlatformTypeId>,
    filter_type: Option<HbkPlatformTypeId>,
    query_table_with_field: Option<HbkQueryTableId>,
    query_table_with_parameter: Option<HbkQueryTableId>,
    query_field_with_type: Option<HbkQueryFieldId>,
}

impl WorkloadAnchors {
    fn resolve(handle: HbkFactReadHandle<'_>) -> Self {
        let query_type = handle.platform_type_by_id("platform_type:Запрос");
        let filter_type = handle.platform_type_by_id("platform_type:ОтборКомпоновкиДанных");
        let query_table_with_field = handle
            .query_tables_by_identifier("Справочник")
            .next()
            .or_else(|| handle.query_tables_by_identifier("ОсновнаяТаблица").next());
        let query_table_with_parameter = handle
            .query_tables_by_identifier("ЗадачаТаблицаЗадачПоИсполнителю")
            .next();
        let query_field_with_type = handle
            .query_tables_by_identifier("БизнесПроцесс")
            .next()
            .and_then(|table| handle.query_fields_by_name(table, "Ссылка").next());
        Self {
            query_type,
            filter_type,
            query_table_with_field,
            query_table_with_parameter,
            query_field_with_type,
        }
    }

    fn observed_count(self) -> u64 {
        [
            self.query_type.is_some(),
            self.filter_type.is_some(),
            self.query_table_with_field.is_some(),
            self.query_table_with_parameter.is_some(),
            self.query_field_with_type.is_some(),
        ]
        .into_iter()
        .map(u64::from)
        .sum()
    }
}

fn measure_workload(
    handle: HbkFactReadHandle<'_>,
    anchors: WorkloadAnchors,
    iterations: usize,
) -> WorkloadReport {
    let mut operations = Vec::with_capacity(22);
    macro_rules! operation {
        ($name:literal, $body:expr) => {
            operations.push(measure_operation($name, iterations, || $body));
        };
    }

    operation!("exact_fact_id", handle.facts_by_id(FIRST_LOOKUP_ID).len());
    operation!(
        "type_by_name",
        handle.platform_types_by_name("Запрос").len()
    );
    operation!(
        "type_template_by_key",
        handle
            .platform_types_by_template_key("Catalog", "Manager")
            .len()
    );
    operation!(
        "members_by_owner",
        anchors
            .query_type
            .map_or(0, |owner| handle.members_of_type(owner).len())
    );
    operation!(
        "member_by_owner_name_kind",
        anchors.query_type.map_or(0, |owner| {
            handle
                .member_by_owner_name_kind(owner, "Текст", Some(HbkTypeMemberKind::Property))
                .len()
        })
    );
    operation!(
        "callable_by_owner_name",
        anchors.query_type.map_or(0, |owner| {
            handle.callable_by_owner_name(owner, "Выполнить").len()
        })
    );
    operation!(
        "constructors_by_type",
        anchors
            .query_type
            .map_or(0, |owner| handle.constructors_of_type(owner).len())
    );
    operation!(
        "global_by_domain_name_kind",
        handle
            .globals_by_domain_name_kind(
                HbkLanguageDomain::Bsl,
                "Сообщить",
                Some(HbkGlobalFactKind::Method),
            )
            .len()
    );
    operation!(
        "module_context_by_kind",
        handle
            .module_context_events(HbkLanguageDomain::Bsl, "bsl", "managed_application")
            .len()
    );
    operation!(
        "query_table_by_name",
        handle.query_tables_by_name("Таблица справочника").len()
    );
    operation!(
        "query_field_by_table_name",
        anchors.query_table_with_field.map_or(0, |table| {
            handle.query_fields_by_name(table, "Ссылка").len()
        })
    );
    operation!(
        "query_param_by_table_name",
        anchors.query_table_with_parameter.map_or(0, |table| {
            handle.query_parameters_by_name(table, "Исполнитель").len()
        })
    );
    operation!(
        "availability_by_fact",
        anchors.filter_type.map_or(0, |fact| {
            let fact = HbkFactRef::PlatformType(fact);
            handle.availability_contexts(fact).len()
                + usize::from(handle.available_since(fact).is_some())
        })
    );
    operation!(
        "relation_by_source_kind",
        anchors.query_field_with_type.map_or(0, |field| {
            handle
                .relations_by_source_kind(HbkFactRef::QueryField(field), "has_type")
                .len()
        })
    );
    operation!(
        "language_by_name",
        handle.language_facts_by_name("Строка").len()
    );
    operation!("enum_by_name", handle.enums_by_name("CurrentRowUse").len());
    operation!(
        "query_table_by_syntax",
        handle
            .query_tables_by_syntax("Справочник.<Имя справочника>")
            .len()
    );
    operation!(
        "query_table_by_identifier",
        handle.query_tables_by_identifier("Справочник").len()
    );
    operation!(
        "exact_fact_id_miss",
        handle.facts_by_id("__hbk_benchmark_missing_fact__").len()
    );
    operation!(
        "type_by_name_miss",
        handle
            .platform_types_by_name("__hbk_benchmark_missing_type__")
            .len()
    );
    operation!(
        "language_by_name_miss",
        handle
            .language_facts_by_name("__hbk_benchmark_missing_language__")
            .len()
    );
    operation!(
        "enum_by_name_miss",
        handle.enums_by_name("__hbk_benchmark_missing_enum__").len()
    );

    let checksum = operations.iter().fold(FNV_OFFSET_BASIS, |hash, operation| {
        let hash = fnv1a(hash, operation.name.as_bytes());
        fnv1a(hash, &operation.observed_total.to_le_bytes())
    });
    WorkloadReport {
        elapsed_ns: 0,
        faults: ProcessFaults::default(),
        checksum,
        operations,
    }
}

fn measure_operation(
    name: &'static str,
    iterations: usize,
    mut operation: impl FnMut() -> usize,
) -> OperationReport {
    let started_at = Instant::now();
    let mut observed_total = 0_u64;
    for _ in 0..iterations {
        let observed = black_box(operation()) as u64;
        observed_total = observed_total.wrapping_add(observed);
    }
    let elapsed_ns = duration_ns(started_at.elapsed());
    OperationReport {
        name,
        iterations,
        elapsed_ns,
        average_ns: elapsed_ns / iterations as u64,
        observed_total,
    }
}

fn snapshot_report(snapshot: &HbkFactSnapshot) -> SnapshotReport {
    let counts = snapshot.counts();
    let memory = snapshot.memory_accounting();
    SnapshotReport {
        counts: counts_report(counts),
        estimated_heap_bytes: memory.total_bytes() as u64,
        logical_payload_bytes: memory.total_payload_bytes() as u64,
    }
}

fn counts_report(counts: HbkFactSnapshotCounts) -> SnapshotCountsReport {
    SnapshotCountsReport {
        strings: counts.strings as u64,
        platform_types: counts.platform_types as u64,
        type_members: counts.type_members as u64,
        callables: counts.callables as u64,
        globals: counts.globals as u64,
        query_tables: counts.query_tables as u64,
        query_fields: counts.query_fields as u64,
        query_parameters: counts.query_parameters as u64,
        language_facts: counts.language_facts as u64,
        enums: counts.enums as u64,
        enum_values: counts.enum_values as u64,
    }
}

fn artifact_report(path: &Path) -> Result<ArtifactReport, io::Error> {
    Ok(ArtifactReport {
        path: path.display().to_string(),
        bytes: fs::metadata(path)?.len(),
    })
}

fn stage_timing_report(timings: HbkFactSnapshotStageTimings) -> BuildStageReport {
    BuildStageReport {
        total_ns: duration_ns(timings.total),
        open_index_ns: duration_ns(timings.open_index),
        read_sql_rows_ns: duration_ns(timings.read_sql_rows),
        build_lookup_maps_ns: duration_ns(timings.build_lookup_maps),
        build_platform_types_ns: duration_ns(timings.build_platform_types),
        group_type_refs_ns: duration_ns(timings.group_type_refs),
        build_signatures_ns: duration_ns(timings.build_signatures),
        build_fact_arenas_ns: duration_ns(timings.build_fact_arenas),
        build_fact_ids_relations_availability_ns: duration_ns(
            timings.build_fact_ids_relations_availability,
        ),
        sort_secondary_indexes_ns: duration_ns(timings.sort_secondary_indexes),
        assemble_snapshot_ns: duration_ns(timings.assemble_snapshot),
    }
}

fn duration_ns(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn parent_start_unix_ns() -> Option<u128> {
    env::var("HBK_BENCH_PARENT_START_UNIX_NS")
        .ok()?
        .parse()
        .ok()
}

fn capture_process_start_to_ready_ns(parent_start_unix_ns: Option<u128>) -> Option<u64> {
    let parent_start_unix_ns = parent_start_unix_ns?;
    let ready_unix_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let elapsed = ready_unix_ns.checked_sub(parent_start_unix_ns)?;
    Some(u64::try_from(elapsed).unwrap_or(u64::MAX))
}

fn phase_report(elapsed: Duration, faults: ProcessFaults) -> PhaseReport {
    PhaseReport {
        elapsed_ns: duration_ns(elapsed),
        faults,
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
struct ProcessFaults {
    minor: u64,
    major: u64,
}

impl ProcessFaults {
    fn delta_to(self, later: Self) -> Self {
        Self {
            minor: later.minor.saturating_sub(self.minor),
            major: later.major.saturating_sub(self.major),
        }
    }
}

fn read_process_faults() -> Result<ProcessFaults, io::Error> {
    parse_process_stat(&fs::read_to_string("/proc/self/stat")?)
}

fn parse_process_stat(input: &str) -> Result<ProcessFaults, io::Error> {
    let command_end = input.rfind(')').ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "/proc/self/stat has no command terminator",
        )
    })?;
    let fields = input
        .get(command_end + 1..)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid /proc/self/stat"))?
        .split_whitespace()
        .collect::<Vec<_>>();
    let minor = parse_proc_u64(&fields, 7, "minflt")?;
    let major = parse_proc_u64(&fields, 9, "majflt")?;
    Ok(ProcessFaults { minor, major })
}

fn parse_proc_u64(fields: &[&str], index: usize, name: &str) -> Result<u64, io::Error> {
    fields
        .get(index)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("/proc/self/stat is missing {name}"),
            )
        })?
        .parse()
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid {name} in /proc/self/stat: {error}"),
            )
        })
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
struct SmapsSample {
    rss_kib: u64,
    pss_kib: u64,
    shared_kib: u64,
    private_kib: u64,
    anonymous_kib: u64,
    swap_kib: u64,
}

fn read_smaps_rollup() -> Result<SmapsSample, io::Error> {
    parse_smaps_rollup(&fs::read_to_string("/proc/self/smaps_rollup")?)
}

fn parse_smaps_rollup(input: &str) -> Result<SmapsSample, io::Error> {
    let mut sample = SmapsSample::default();
    for line in input.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let Some(value) = value.split_whitespace().next() else {
            continue;
        };
        let value = value.parse::<u64>().map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid {name} in /proc/self/smaps_rollup: {error}"),
            )
        })?;
        match name {
            "Rss" => sample.rss_kib = value,
            "Pss" => sample.pss_kib = value,
            "Shared_Clean" | "Shared_Dirty" => {
                sample.shared_kib = sample.shared_kib.saturating_add(value);
            }
            "Private_Clean" | "Private_Dirty" => {
                sample.private_kib = sample.private_kib.saturating_add(value);
            }
            "Anonymous" => sample.anonymous_kib = value,
            "Swap" => sample.swap_kib = value,
            _ => {}
        }
    }
    Ok(sample)
}

fn checksum_observation(name: &str, observed: u64) -> u64 {
    let hash = fnv1a(FNV_OFFSET_BASIS, name.as_bytes());
    fnv1a(hash, &observed.to_le_bytes())
}

fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[derive(Serialize)]
struct BenchmarkReport {
    schema_version: &'static str,
    workload_version: &'static str,
    mode: &'static str,
    backend: &'static str,
    iterations: usize,
    index: ArtifactReport,
    cache: Option<ArtifactReport>,
    cache_status: Option<&'static str>,
    timings: TimingReport,
    smaps: SmapsReport,
    snapshot: SnapshotReport,
    build_stages: Option<BuildStageReport>,
    hold: HoldReport,
    allocations: AllocationEvidenceReport,
}

#[derive(Serialize)]
struct ArtifactReport {
    path: String,
    bytes: u64,
}

#[derive(Serialize)]
struct TimingReport {
    phase_order: &'static [&'static str],
    process_start_to_ready_ns: Option<u64>,
    entry_to_ready_ns: u64,
    open: PhaseReport,
    cache_write: Option<PhaseReport>,
    first_lookup: Option<LookupPhaseReport>,
    anchor_resolution: Option<LookupPhaseReport>,
    warmup_ns: Option<u64>,
    workload: Option<WorkloadReport>,
}

#[derive(Serialize)]
struct PhaseReport {
    elapsed_ns: u64,
    faults: ProcessFaults,
}

#[derive(Serialize)]
struct LookupPhaseReport {
    operation: &'static str,
    key: &'static str,
    elapsed_ns: u64,
    faults: ProcessFaults,
    observed: u64,
    checksum: u64,
}

#[derive(Serialize)]
struct WorkloadReport {
    elapsed_ns: u64,
    faults: ProcessFaults,
    checksum: u64,
    operations: Vec<OperationReport>,
}

#[derive(Serialize)]
struct OperationReport {
    name: &'static str,
    iterations: usize,
    elapsed_ns: u64,
    average_ns: u64,
    observed_total: u64,
}

#[derive(Serialize)]
struct SmapsReport {
    after_open: SmapsSample,
    after_workload: SmapsSample,
}

#[derive(Serialize)]
struct SnapshotReport {
    counts: SnapshotCountsReport,
    estimated_heap_bytes: u64,
    logical_payload_bytes: u64,
}

#[derive(Serialize)]
struct SnapshotCountsReport {
    strings: u64,
    platform_types: u64,
    type_members: u64,
    callables: u64,
    globals: u64,
    query_tables: u64,
    query_fields: u64,
    query_parameters: u64,
    language_facts: u64,
    enums: u64,
    enum_values: u64,
}

#[derive(Serialize)]
struct BuildStageReport {
    total_ns: u64,
    open_index_ns: u64,
    read_sql_rows_ns: u64,
    build_lookup_maps_ns: u64,
    build_platform_types_ns: u64,
    group_type_refs_ns: u64,
    build_signatures_ns: u64,
    build_fact_arenas_ns: u64,
    build_fact_ids_relations_availability_ns: u64,
    sort_secondary_indexes_ns: u64,
    assemble_snapshot_ns: u64,
}

#[derive(Serialize)]
struct HoldReport {
    requested_ms: u64,
    ready_file: Option<String>,
}

#[derive(Serialize)]
struct AllocationEvidenceReport {
    enabled: bool,
    entry_to_ready: AllocationDeltaReport,
    first_lookup: Option<AllocationDeltaReport>,
    anchor_resolution: Option<AllocationDeltaReport>,
    workload: Option<AllocationDeltaReport>,
    final_snapshot: AllocationSnapshotReport,
}

#[derive(Serialize)]
struct AllocationSnapshotReport {
    allocation_calls: u64,
    reallocation_calls: u64,
    deallocation_calls: u64,
    allocated_bytes: u64,
    deallocated_bytes: u64,
    current_live_bytes: u64,
    peak_live_bytes: u64,
}

impl From<HbkSnapshotExperimentAllocationSnapshot> for AllocationSnapshotReport {
    fn from(snapshot: HbkSnapshotExperimentAllocationSnapshot) -> Self {
        Self {
            allocation_calls: snapshot.allocation_calls,
            reallocation_calls: snapshot.reallocation_calls,
            deallocation_calls: snapshot.deallocation_calls,
            allocated_bytes: snapshot.allocated_bytes,
            deallocated_bytes: snapshot.deallocated_bytes,
            current_live_bytes: snapshot.current_live_bytes,
            peak_live_bytes: snapshot.peak_live_bytes,
        }
    }
}

#[derive(Serialize)]
struct AllocationDeltaReport {
    allocation_calls: u64,
    reallocation_calls: u64,
    deallocation_calls: u64,
    allocated_bytes: u64,
    deallocated_bytes: u64,
    live_bytes_before: u64,
    live_bytes_after: u64,
    peak_live_bytes_before: u64,
    peak_live_bytes_after: u64,
    peak_live_bytes_growth: u64,
}

impl From<HbkSnapshotExperimentAllocationDelta> for AllocationDeltaReport {
    fn from(delta: HbkSnapshotExperimentAllocationDelta) -> Self {
        Self {
            allocation_calls: delta.allocation_calls,
            reallocation_calls: delta.reallocation_calls,
            deallocation_calls: delta.deallocation_calls,
            allocated_bytes: delta.allocated_bytes,
            deallocated_bytes: delta.deallocated_bytes,
            live_bytes_before: delta.live_bytes_before,
            live_bytes_after: delta.live_bytes_after,
            peak_live_bytes_before: delta.peak_live_bytes_before,
            peak_live_bytes_after: delta.peak_live_bytes_after,
            peak_live_bytes_growth: delta.peak_live_bytes_growth,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_proc_stat_when_command_contains_spaces_and_parentheses() {
        let input = "123 (benchmark (worker)) R 1 2 3 4 5 6 17 8 19 10";

        let faults = parse_process_stat(input).unwrap();

        assert_eq!(faults.minor, 17);
        assert_eq!(faults.major, 19);
    }

    #[test]
    fn parses_smaps_rollup_and_combines_private_and_shared_memory() {
        let input = "\
Rss:                 100 kB
Pss:                  70 kB
Shared_Clean:         11 kB
Shared_Dirty:          3 kB
Private_Clean:        17 kB
Private_Dirty:        19 kB
Anonymous:            31 kB
Swap:                  5 kB
";

        let sample = parse_smaps_rollup(input).unwrap();

        assert_eq!(sample.rss_kib, 100);
        assert_eq!(sample.pss_kib, 70);
        assert_eq!(sample.shared_kib, 14);
        assert_eq!(sample.private_kib, 36);
        assert_eq!(sample.anonymous_kib, 31);
        assert_eq!(sample.swap_kib, 5);
    }

    #[test]
    fn parses_scenario_arguments_and_rejects_zero_iterations() {
        let command = parse_args([
            "cache-owned".to_owned(),
            "index.sqlite".to_owned(),
            "cache.bin".to_owned(),
            "42".to_owned(),
        ])
        .unwrap();
        assert_eq!(
            command,
            Command::CacheOwned {
                index_path: PathBuf::from("index.sqlite"),
                cache_path: PathBuf::from("cache.bin"),
                iterations: 42,
            }
        );

        let error = parse_args([
            "sql-owned".to_owned(),
            "index.sqlite".to_owned(),
            "0".to_owned(),
        ])
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn loaded_phase_order_keeps_lazy_work_inside_reported_boundaries() {
        assert_eq!(
            LOADED_PHASE_ORDER,
            [
                "entry_to_ready",
                "first_lookup_including_handle",
                "anchor_resolution",
                "warmup",
                "workload",
            ]
        );
    }
}
