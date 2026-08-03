use std::borrow::Borrow;
use std::fs;
use std::hint::black_box;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{env, process};

use serde::Serialize;
use syntax_helper_search::{
    HbkCallable, HbkCallableKind, HbkFactReadHandle, HbkFactRef, HbkFactSnapshot,
    HbkFactSnapshotCacheStatus, HbkGlobalFact, HbkGlobalFactKind, HbkLanguageDomain, HbkName,
    HbkParameter, HbkPlatformTypeTemplateKey, HbkSignature, HbkSnapshotExperimentAllocationDelta,
    HbkSnapshotExperimentAllocationSnapshot, HbkSnapshotExperimentAllocator, HbkTypeRef,
    HbkTypeRefTarget, HbkTypeTemplateBinding, StringId, experiment_allocation_snapshot, model,
};

#[global_allocator]
static ALLOCATOR: HbkSnapshotExperimentAllocator = HbkSnapshotExperimentAllocator;

const REPORT_SCHEMA_VERSION: &str = "hbk-s83-av1-benchmark/v1";
const WORKLOAD_VERSION: &str = "s83-av1-filtered-global-method-enumeration/v1";
const PLATFORM_VERSION: &str = "8.3.27.1859";
const PROVIDER_SCHEMA_VERSION: u32 = 16;
const EXTRACTION_SCHEMA_VERSION: u32 = 11;
const SOURCE_LOCALE: &str = "ru";
const SOURCE_HBK_PATH: &str = "/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk";
const SOURCE_HBK_BYTES: u64 = 40_744_845;
const SOURCE_HBK_SHA256: &str = "5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48";
const PROVIDER_BYTES: u64 = 204_288_000;
const PROVIDER_SHA256: &str = "55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab";
const DEFAULT_ITERATIONS: usize = 1_000;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const AVAILABILITY_CONTEXTS: &[&str] = &[
    "thin_client",
    "web_client",
    "mobile_client",
    "server",
    "thick_client",
    "external_connection",
    "mobile_application_client",
    "mobile_application_server",
    "mobile_standalone_server",
];

fn main() {
    let entry_started_at = Instant::now();
    let entry_faults = match read_process_faults() {
        Ok(faults) => faults,
        Err(error) => {
            eprintln!("failed to read initial process counters: {error}");
            process::exit(1);
        }
    };
    let entry_allocations = experiment_allocation_snapshot();

    match run(entry_started_at, entry_faults, entry_allocations) {
        Ok(report) => {
            if let Err(error) = serde_json::to_writer(io::stdout().lock(), &report) {
                eprintln!("failed to write benchmark report: {error}");
                process::exit(1);
            }
            println!();
        }
        Err(error) => {
            eprintln!("benchmark failed: {error}");
            process::exit(1);
        }
    }
}

fn run(
    entry_started_at: Instant,
    entry_faults: ProcessFaults,
    entry_allocations: HbkSnapshotExperimentAllocationSnapshot,
) -> Result<BenchmarkReport, Box<dyn std::error::Error>> {
    let command = parse_args(env::args().skip(1))?;
    match command {
        Command::SqlOwned {
            index_path,
            context,
            iterations,
        } => {
            let build_report = HbkFactSnapshot::from_path_with_stage_timings(&index_path)?;
            let ready_elapsed = entry_started_at.elapsed();
            let ready_faults = read_process_faults()?;
            let ready_allocations = experiment_allocation_snapshot();
            let scenario =
                measure_loaded_snapshot(&build_report.snapshot, ready_faults, context, iterations)?;
            let final_allocations = experiment_allocation_snapshot();
            let transcript = transcript_objects(
                &build_report.snapshot,
                build_report.snapshot.worker_handle(),
                context,
            );
            Ok(BenchmarkReport {
                schema_version: REPORT_SCHEMA_VERSION,
                workload_version: WORKLOAD_VERSION,
                mode: "sql-owned",
                backend: "S83-H0",
                decision_role: "baseline",
                baseline_role: "h0",
                module_context_filter_used: false,
                empty_availability_rule: "universal",
                availability_context: context.code,
                iterations,
                input_identity: input_identity(&index_path)?,
                index: artifact_report(&index_path)?,
                cache: None,
                cache_status: None,
                counts: scenario.counts,
                timings: TimingReport {
                    phase_order: &["entry_to_ready", "first_enumeration", "warmup", "workload"],
                    entry_to_ready_ns: duration_ns(ready_elapsed),
                    open: phase_report(ready_elapsed, entry_faults.delta_to(ready_faults)),
                    first_enumeration: scenario.first_enumeration,
                    warmup: scenario.warmup,
                    workload: scenario.workload,
                },
                allocations: AllocationEvidenceReport {
                    enabled: cfg!(feature = "snapshot-experiment-alloc"),
                    entry_to_ready: ready_allocations.delta_since(entry_allocations).into(),
                    first_enumeration: scenario.first_allocations.into(),
                    warmup: scenario.warmup_allocations.into(),
                    workload: scenario.workload_allocations.into(),
                    final_snapshot: final_allocations.into(),
                },
                transcript,
            })
        }
        Command::CacheOwned {
            index_path,
            cache_path,
            context,
            iterations,
        } => {
            let load_report =
                HbkFactSnapshot::from_path_with_binary_cache(&index_path, &cache_path)?;
            if let HbkFactSnapshotCacheStatus::Rebuilt { reason } = &load_report.status {
                return Err(io::Error::other(format!(
                    "cache-owned requires status Loaded, but the cache was rebuilt: {reason}"
                ))
                .into());
            }
            let ready_elapsed = entry_started_at.elapsed();
            let ready_faults = read_process_faults()?;
            let ready_allocations = experiment_allocation_snapshot();
            let scenario =
                measure_loaded_snapshot(&load_report.snapshot, ready_faults, context, iterations)?;
            let final_allocations = experiment_allocation_snapshot();
            let transcript = transcript_objects(
                &load_report.snapshot,
                load_report.snapshot.worker_handle(),
                context,
            );
            Ok(BenchmarkReport {
                schema_version: REPORT_SCHEMA_VERSION,
                workload_version: WORKLOAD_VERSION,
                mode: "cache-owned",
                backend: "S83-C0",
                decision_role: "control",
                baseline_role: "h0",
                module_context_filter_used: false,
                empty_availability_rule: "universal",
                availability_context: context.code,
                iterations,
                input_identity: input_identity(&index_path)?,
                index: artifact_report(&index_path)?,
                cache: Some(artifact_report(&cache_path)?),
                cache_status: Some("loaded"),
                counts: scenario.counts,
                timings: TimingReport {
                    phase_order: &["entry_to_ready", "first_enumeration", "warmup", "workload"],
                    entry_to_ready_ns: duration_ns(ready_elapsed),
                    open: phase_report(ready_elapsed, entry_faults.delta_to(ready_faults)),
                    first_enumeration: scenario.first_enumeration,
                    warmup: scenario.warmup,
                    workload: scenario.workload,
                },
                allocations: AllocationEvidenceReport {
                    enabled: cfg!(feature = "snapshot-experiment-alloc"),
                    entry_to_ready: ready_allocations.delta_since(entry_allocations).into(),
                    first_enumeration: scenario.first_allocations.into(),
                    warmup: scenario.warmup_allocations.into(),
                    workload: scenario.workload_allocations.into(),
                    final_snapshot: final_allocations.into(),
                },
                transcript,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    SqlOwned {
        index_path: PathBuf,
        context: AvailabilityContext,
        iterations: usize,
    },
    CacheOwned {
        index_path: PathBuf,
        cache_path: PathBuf,
        context: AvailabilityContext,
        iterations: usize,
    },
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, io::Error> {
    let mut args = args.into_iter();
    let mode = args.next().ok_or_else(usage_error)?;
    let command = match mode.as_str() {
        "sql-owned" => {
            let index_path = required_path(&mut args)?;
            let context = required_context(&mut args)?;
            let iterations = optional_iterations(&mut args)?;
            Command::SqlOwned {
                index_path,
                context,
                iterations,
            }
        }
        "cache-owned" => {
            let index_path = required_path(&mut args)?;
            let cache_path = required_path(&mut args)?;
            let context = required_context(&mut args)?;
            let iterations = optional_iterations(&mut args)?;
            Command::CacheOwned {
                index_path,
                cache_path,
                context,
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

fn required_context(
    args: &mut impl Iterator<Item = String>,
) -> Result<AvailabilityContext, io::Error> {
    let value = args.next().ok_or_else(usage_error)?;
    AvailabilityContext::parse(&value).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "unknown availability context {value:?}; expected one of {}",
                AVAILABILITY_CONTEXTS.join(", ")
            ),
        )
    })
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
        "usage: measure_hbk_s83_av1 \
         sql-owned <index.sqlite> <availability-context> [iterations] | \
         cache-owned <index.sqlite> <cache.bin> <availability-context> [iterations]",
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AvailabilityContext {
    code: &'static str,
}

impl AvailabilityContext {
    fn parse(value: &str) -> Option<Self> {
        AVAILABILITY_CONTEXTS
            .iter()
            .copied()
            .find(|candidate| *candidate == value)
            .map(|code| Self { code })
    }
}

struct LoadedScenario {
    counts: EnumerationCountsReport,
    first_enumeration: EnumerationPhaseReport,
    first_allocations: HbkSnapshotExperimentAllocationDelta,
    warmup: EnumerationPhaseReport,
    warmup_allocations: HbkSnapshotExperimentAllocationDelta,
    workload: WorkloadReport,
    workload_allocations: HbkSnapshotExperimentAllocationDelta,
}

fn measure_loaded_snapshot(
    snapshot: &HbkFactSnapshot,
    ready_faults: ProcessFaults,
    context: AvailabilityContext,
    iterations: usize,
) -> Result<LoadedScenario, io::Error> {
    let before_first_allocations = experiment_allocation_snapshot();
    let first_started_at = Instant::now();
    let handle = snapshot.worker_handle();
    let first_sample = black_box(enumerate_filtered_methods(snapshot, handle, context));
    let first_elapsed = first_started_at.elapsed();
    let after_first_allocations = experiment_allocation_snapshot();
    let after_first_faults = read_process_faults()?;

    let warmup_start_faults = read_process_faults()?;
    let before_warmup_allocations = experiment_allocation_snapshot();
    let warmup_started_at = Instant::now();
    let warmup_sample = black_box(enumerate_filtered_methods(snapshot, handle, context));
    let warmup_elapsed = warmup_started_at.elapsed();
    let after_warmup_allocations = experiment_allocation_snapshot();
    let after_warmup_faults = read_process_faults()?;
    ensure_same_enumeration("warmup", None, &first_sample, &warmup_sample)?;

    let workload_start_faults = read_process_faults()?;
    let before_workload_allocations = experiment_allocation_snapshot();
    let workload_started_at = Instant::now();
    let mut checksum = FNV_OFFSET_BASIS;
    let mut returned_total = 0_u64;
    for iteration in 0..iterations {
        let sample = black_box(enumerate_filtered_methods(snapshot, handle, context));
        ensure_same_enumeration("workload", Some(iteration + 1), &first_sample, &sample)?;
        checksum = checksum_sample(checksum, &sample);
        returned_total = returned_total.wrapping_add(sample.returned_objects as u64);
    }
    let workload_elapsed = workload_started_at.elapsed();
    let after_workload_allocations = experiment_allocation_snapshot();
    let after_workload_faults = read_process_faults()?;

    let counts = first_sample.counts();
    if !counts.universal_assertion || !counts.excluded_assertion {
        return Err(io::Error::other(format!(
            "S83-AV1 guard failed for {}: universal_assertion={}, excluded_assertion={}",
            context.code, counts.universal_assertion, counts.excluded_assertion
        )));
    }

    Ok(LoadedScenario {
        counts,
        first_enumeration: enumeration_phase_report(
            first_elapsed,
            ready_faults.delta_to(after_first_faults),
            first_sample,
        ),
        first_allocations: after_first_allocations.delta_since(before_first_allocations),
        warmup: enumeration_phase_report(
            warmup_elapsed,
            warmup_start_faults.delta_to(after_warmup_faults),
            warmup_sample,
        ),
        warmup_allocations: after_warmup_allocations.delta_since(before_warmup_allocations),
        workload: WorkloadReport {
            elapsed_ns: duration_ns(workload_elapsed),
            average_ns: duration_ns(workload_elapsed) / iterations as u64,
            ns_per_object: ns_per_object(duration_ns(workload_elapsed), returned_total),
            faults: workload_start_faults.delta_to(after_workload_faults),
            iterations,
            returned_total,
            checksum,
        },
        workload_allocations: after_workload_allocations.delta_since(before_workload_allocations),
    })
}

fn ensure_same_enumeration(
    phase: &'static str,
    iteration: Option<usize>,
    expected: &EnumerationSample,
    actual: &EnumerationSample,
) -> Result<(), io::Error> {
    if expected.returned_objects != actual.returned_objects || expected.checksum != actual.checksum
    {
        let phase = match iteration {
            Some(iteration) => format!("{phase} iteration {iteration}"),
            None => phase.to_owned(),
        };
        return Err(io::Error::other(format!(
            "S83-AV1 enumeration changed during {phase}: expected count/checksum {}/{:#018x}, got {}/{:#018x}",
            expected.returned_objects, expected.checksum, actual.returned_objects, actual.checksum,
        )));
    }
    Ok(())
}

fn enumerate_filtered_methods(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    context: AvailabilityContext,
) -> EnumerationSample {
    let mut sample = EnumerationSample {
        scanned_globals: 0,
        candidate_methods: 0,
        universal_methods: 0,
        explicit_context_methods: 0,
        excluded_methods: 0,
        returned_objects: 0,
        checksum: FNV_OFFSET_BASIS,
    };
    for global_id in handle.global_fact_ids() {
        sample.scanned_globals += 1;
        let global = snapshot.global_fact(global_id);
        if global.domain != HbkLanguageDomain::Bsl || global.kind != HbkGlobalFactKind::Method {
            continue;
        }
        let Some(callable_id) = global.callable else {
            continue;
        };
        sample.candidate_methods += 1;
        let callable = snapshot.callable(callable_id);
        let fact = HbkFactRef::Global(global_id);
        match availability_match(snapshot, handle.availability_contexts(fact), context) {
            AvailabilityMatch::Universal => sample.universal_methods += 1,
            AvailabilityMatch::Explicit => sample.explicit_context_methods += 1,
            AvailabilityMatch::Excluded => {
                sample.excluded_methods += 1;
                continue;
            }
        }
        sample.returned_objects += 1;
        sample.checksum = checksum_object(
            sample.checksum,
            snapshot,
            global,
            callable,
            handle.availability_contexts(fact),
        );
    }
    sample
}

fn availability_match<I>(
    snapshot: &HbkFactSnapshot,
    contexts: I,
    requested: AvailabilityContext,
) -> AvailabilityMatch
where
    I: IntoIterator,
    I::Item: std::borrow::Borrow<StringId>,
{
    availability_match_codes(
        contexts
            .into_iter()
            .map(|context| snapshot.string(*context.borrow())),
        requested,
    )
}

fn availability_match_codes<'a>(
    contexts: impl IntoIterator<Item = &'a str>,
    requested: AvailabilityContext,
) -> AvailabilityMatch {
    let mut contexts = contexts.into_iter().peekable();
    if contexts.peek().is_none() {
        return AvailabilityMatch::Universal;
    }
    if contexts.any(|context| context == requested.code) {
        AvailabilityMatch::Explicit
    } else {
        AvailabilityMatch::Excluded
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AvailabilityMatch {
    Universal,
    Explicit,
    Excluded,
}

fn checksum_sample(hash: u64, sample: &EnumerationSample) -> u64 {
    let hash = fnv1a(hash, &sample.returned_objects.to_le_bytes());
    fnv1a(hash, &sample.checksum.to_le_bytes())
}

fn checksum_object<I>(
    mut hash: u64,
    snapshot: &HbkFactSnapshot,
    global: &HbkGlobalFact,
    callable: &HbkCallable,
    availability: I,
) -> u64
where
    I: IntoIterator,
    I::IntoIter: ExactSizeIterator,
    I::Item: std::borrow::Borrow<StringId>,
{
    hash = hash_string_id(hash, snapshot, global.id);
    hash = hash_name(hash, snapshot, &global.name);
    hash = hash_global_kind(hash, global.kind);
    hash = hash_language_domain(hash, global.domain);
    hash = hash_type_refs(hash, snapshot, &global.type_refs);
    hash = hash_string_ids(hash, snapshot, availability);
    hash = hash_string_id(hash, snapshot, callable.id);
    hash = hash_callable_owner(hash, snapshot, callable);
    hash = hash_callable_kind(hash, callable.kind);
    hash = hash_name(hash, snapshot, &callable.name);
    hash = hash_string_ids(hash, snapshot, &callable.availability_contexts);
    hash = hash_signatures(hash, snapshot, &callable.signatures);
    hash_type_refs(hash, snapshot, &callable.return_type_refs)
}

fn hash_callable_owner(mut hash: u64, snapshot: &HbkFactSnapshot, callable: &HbkCallable) -> u64 {
    match callable.owner {
        Some(owner) => {
            hash = fnv1a(hash, &[1]);
            hash_string_id(hash, snapshot, snapshot.platform_type(owner).id)
        }
        None => fnv1a(hash, &[0]),
    }
}

fn hash_signatures(mut hash: u64, snapshot: &HbkFactSnapshot, signatures: &[HbkSignature]) -> u64 {
    hash = fnv1a(hash, &(signatures.len() as u64).to_le_bytes());
    for signature in signatures {
        hash = hash_string_id(hash, snapshot, signature.text);
        hash = fnv1a(hash, &(signature.parameters.len() as u64).to_le_bytes());
        for parameter in &signature.parameters {
            hash = hash_parameter(hash, snapshot, parameter);
        }
        hash = hash_type_refs(hash, snapshot, &signature.return_type_refs);
    }
    hash
}

fn hash_parameter(mut hash: u64, snapshot: &HbkFactSnapshot, parameter: &HbkParameter) -> u64 {
    hash = hash_string_id(hash, snapshot, parameter.name);
    hash = fnv1a(hash, &[u8::from(parameter.required)]);
    hash_type_refs(hash, snapshot, &parameter.type_refs)
}

fn hash_type_refs(mut hash: u64, snapshot: &HbkFactSnapshot, refs: &[HbkTypeRef]) -> u64 {
    hash = fnv1a(hash, &(refs.len() as u64).to_le_bytes());
    for type_ref in refs {
        hash = hash_string_id(hash, snapshot, type_ref.name);
        hash = hash_type_ref_target(hash, snapshot, &type_ref.target);
        match type_ref.type_template_key {
            Some(key) => {
                hash = fnv1a(hash, &[1]);
                hash = hash_string_id(hash, snapshot, key.family);
                hash = hash_string_id(hash, snapshot, key.variant);
            }
            None => hash = fnv1a(hash, &[0]),
        }
        if let Some(binding) = &type_ref.template_binding {
            hash = fnv1a(hash, &[1]);
            hash = hash_string_id(hash, snapshot, binding.template_key.family);
            hash = hash_string_id(hash, snapshot, binding.template_key.variant);
            hash = fnv1a(hash, &(binding.arguments.len() as u64).to_le_bytes());
            for argument in &binding.arguments {
                hash = hash_template_argument(hash, argument);
            }
        } else {
            hash = fnv1a(hash, &[0]);
        }
    }
    hash
}

fn hash_template_argument(mut hash: u64, argument: &model::TemplateParameterBinding) -> u64 {
    match argument {
        model::TemplateParameterBinding::OwnerParameter {
            owner_parameter_index,
            target_parameter_index,
        } => {
            hash = fnv1a(hash, b"owner_parameter");
            hash = fnv1a(hash, &(*owner_parameter_index as u64).to_le_bytes());
            fnv1a(hash, &(*target_parameter_index as u64).to_le_bytes())
        }
    }
}

fn hash_type_ref_target(
    mut hash: u64,
    snapshot: &HbkFactSnapshot,
    target: &HbkTypeRefTarget,
) -> u64 {
    match target {
        HbkTypeRefTarget::Ok(id) => {
            hash = fnv1a(hash, b"ok");
            hash_string_id(hash, snapshot, *id)
        }
        HbkTypeRefTarget::Unresolved => fnv1a(hash, b"unresolved"),
        HbkTypeRefTarget::Ambiguous(ids) => {
            hash = fnv1a(hash, b"ambiguous");
            hash = fnv1a(hash, &(ids.len() as u64).to_le_bytes());
            for id in ids {
                hash = hash_string_id(hash, snapshot, *id);
            }
            hash
        }
    }
}

fn hash_name(mut hash: u64, snapshot: &HbkFactSnapshot, name: &HbkName) -> u64 {
    hash = hash_string_id(hash, snapshot, name.primary);
    match name.alias {
        Some(alias) => {
            hash = fnv1a(hash, &[1]);
            hash_string_id(hash, snapshot, alias)
        }
        None => fnv1a(hash, &[0]),
    }
}

fn hash_string_id(hash: u64, snapshot: &HbkFactSnapshot, id: StringId) -> u64 {
    fnv1a(hash, snapshot.string(id).as_bytes())
}

fn hash_string_ids<I>(mut hash: u64, snapshot: &HbkFactSnapshot, ids: I) -> u64
where
    I: IntoIterator,
    I::IntoIter: ExactSizeIterator,
    I::Item: std::borrow::Borrow<StringId>,
{
    let ids = ids.into_iter();
    hash = fnv1a(hash, &(ids.len() as u64).to_le_bytes());
    for id in ids {
        hash = hash_string_id(hash, snapshot, *id.borrow());
    }
    hash
}

fn hash_global_kind(hash: u64, kind: HbkGlobalFactKind) -> u64 {
    fnv1a(
        hash,
        match kind {
            HbkGlobalFactKind::Method => b"method",
            HbkGlobalFactKind::Property => b"property",
        },
    )
}

fn hash_callable_kind(hash: u64, kind: HbkCallableKind) -> u64 {
    fnv1a(
        hash,
        match kind {
            HbkCallableKind::Method => b"method",
            HbkCallableKind::Constructor => b"constructor",
            HbkCallableKind::GlobalMethod => b"global_method",
            HbkCallableKind::Event => b"event",
            HbkCallableKind::LanguageFunction => b"language_function",
        },
    )
}

fn hash_language_domain(hash: u64, domain: HbkLanguageDomain) -> u64 {
    fnv1a(
        hash,
        match domain {
            HbkLanguageDomain::Bsl => b"bsl",
            HbkLanguageDomain::Query => b"query",
            HbkLanguageDomain::DataComposition => b"data_composition",
            HbkLanguageDomain::Unknown => b"unknown",
        },
    )
}

fn transcript_objects(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    context: AvailabilityContext,
) -> Vec<TranscriptObject> {
    let mut objects = Vec::new();
    for global_id in handle.global_fact_ids() {
        let global = snapshot.global_fact(global_id);
        if global.domain != HbkLanguageDomain::Bsl || global.kind != HbkGlobalFactKind::Method {
            continue;
        }
        let Some(callable_id) = global.callable else {
            continue;
        };
        let callable = snapshot.callable(callable_id);
        let fact = HbkFactRef::Global(global_id);
        let availability_rule =
            match availability_match(snapshot, handle.availability_contexts(fact), context) {
                AvailabilityMatch::Universal => "universal",
                AvailabilityMatch::Explicit => "explicit_context",
                AvailabilityMatch::Excluded => continue,
            };
        objects.push(TranscriptObject {
            global_id: snapshot.string(global.id).to_owned(),
            global_kind: global_kind_code(global.kind),
            global_domain: language_domain_code(global.domain),
            global_name: string_name(snapshot, &global.name),
            global_type_refs: transcript_type_refs(snapshot, &global.type_refs),
            availability_contexts: string_ids(snapshot, handle.availability_contexts(fact)),
            callable_id: snapshot.string(callable.id).to_owned(),
            callable_owner: callable
                .owner
                .map(|owner| snapshot.string(snapshot.platform_type(owner).id).to_owned()),
            callable_kind: callable_kind_code(callable.kind),
            callable_name: string_name(snapshot, &callable.name),
            callable_availability_contexts: string_ids(snapshot, &callable.availability_contexts),
            availability_rule,
            signatures: callable
                .signatures
                .iter()
                .map(|signature| TranscriptSignature {
                    text: snapshot.string(signature.text).to_owned(),
                    parameters: signature
                        .parameters
                        .iter()
                        .map(|parameter| TranscriptParameter {
                            name: snapshot.string(parameter.name).to_owned(),
                            required: parameter.required,
                            type_refs: transcript_type_refs(snapshot, &parameter.type_refs),
                        })
                        .collect(),
                    return_type_refs: transcript_type_refs(snapshot, &signature.return_type_refs),
                })
                .collect(),
            callable_return_type_refs: transcript_type_refs(snapshot, &callable.return_type_refs),
        });
    }
    objects
}

fn string_name(snapshot: &HbkFactSnapshot, name: &HbkName) -> TranscriptName {
    TranscriptName {
        primary: snapshot.string(name.primary).to_owned(),
        alias: name.alias.map(|alias| snapshot.string(alias).to_owned()),
    }
}

fn transcript_type_refs(snapshot: &HbkFactSnapshot, refs: &[HbkTypeRef]) -> Vec<TranscriptTypeRef> {
    refs.iter()
        .map(|type_ref| TranscriptTypeRef {
            name: snapshot.string(type_ref.name).to_owned(),
            target: match &type_ref.target {
                HbkTypeRefTarget::Ok(id) => TranscriptTypeRefTarget {
                    status: "ok",
                    values: vec![snapshot.string(*id).to_owned()],
                },
                HbkTypeRefTarget::Unresolved => TranscriptTypeRefTarget {
                    status: "unresolved",
                    values: Vec::new(),
                },
                HbkTypeRefTarget::Ambiguous(ids) => TranscriptTypeRefTarget {
                    status: "ambiguous",
                    values: ids
                        .iter()
                        .map(|id| snapshot.string(*id).to_owned())
                        .collect(),
                },
            },
            type_template_key: type_ref
                .type_template_key
                .map(|key| transcript_template_key(snapshot, key)),
            template_binding: type_ref
                .template_binding
                .as_ref()
                .map(|binding| transcript_template_binding(snapshot, binding)),
        })
        .collect()
}

fn transcript_template_key(
    snapshot: &HbkFactSnapshot,
    key: HbkPlatformTypeTemplateKey,
) -> TranscriptTemplateKey {
    TranscriptTemplateKey {
        family: snapshot.string(key.family).to_owned(),
        variant: snapshot.string(key.variant).to_owned(),
    }
}

fn transcript_template_binding(
    snapshot: &HbkFactSnapshot,
    binding: &HbkTypeTemplateBinding,
) -> TranscriptTemplateBinding {
    TranscriptTemplateBinding {
        template_key: transcript_template_key(snapshot, binding.template_key),
        arguments: binding
            .arguments
            .iter()
            .map(|argument| match argument {
                model::TemplateParameterBinding::OwnerParameter {
                    owner_parameter_index,
                    target_parameter_index,
                } => TranscriptTemplateArgument {
                    kind: "owner_parameter",
                    owner_parameter_index: *owner_parameter_index as u64,
                    target_parameter_index: *target_parameter_index as u64,
                },
            })
            .collect(),
    }
}

fn string_ids<I>(snapshot: &HbkFactSnapshot, ids: I) -> Vec<String>
where
    I: IntoIterator,
    I::Item: std::borrow::Borrow<StringId>,
{
    ids.into_iter()
        .map(|id| snapshot.string(*id.borrow()).to_owned())
        .collect()
}

fn global_kind_code(kind: HbkGlobalFactKind) -> &'static str {
    match kind {
        HbkGlobalFactKind::Method => "method",
        HbkGlobalFactKind::Property => "property",
    }
}

fn language_domain_code(domain: HbkLanguageDomain) -> &'static str {
    match domain {
        HbkLanguageDomain::Bsl => "bsl",
        HbkLanguageDomain::Query => "query",
        HbkLanguageDomain::DataComposition => "data_composition",
        HbkLanguageDomain::Unknown => "unknown",
    }
}

fn callable_kind_code(kind: HbkCallableKind) -> &'static str {
    match kind {
        HbkCallableKind::Method => "method",
        HbkCallableKind::Constructor => "constructor",
        HbkCallableKind::GlobalMethod => "global_method",
        HbkCallableKind::Event => "event",
        HbkCallableKind::LanguageFunction => "language_function",
    }
}

fn enumeration_phase_report(
    elapsed: Duration,
    faults: ProcessFaults,
    sample: EnumerationSample,
) -> EnumerationPhaseReport {
    EnumerationPhaseReport {
        elapsed_ns: duration_ns(elapsed),
        ns_per_object: ns_per_object(duration_ns(elapsed), sample.returned_objects as u64),
        faults,
        returned_objects: sample.returned_objects as u64,
        checksum: sample.checksum,
    }
}

fn ns_per_object(elapsed_ns: u64, objects: u64) -> Option<u64> {
    if objects == 0 {
        None
    } else {
        Some(elapsed_ns / objects)
    }
}

#[derive(Debug, Clone)]
struct EnumerationSample {
    scanned_globals: usize,
    candidate_methods: usize,
    universal_methods: usize,
    explicit_context_methods: usize,
    excluded_methods: usize,
    returned_objects: usize,
    checksum: u64,
}

impl EnumerationSample {
    fn counts(&self) -> EnumerationCountsReport {
        EnumerationCountsReport {
            scanned_globals: self.scanned_globals as u64,
            candidate_methods: self.candidate_methods as u64,
            returned_objects: self.returned_objects as u64,
            universal_objects: self.universal_methods as u64,
            explicit_context_objects: self.explicit_context_methods as u64,
            excluded_objects: self.excluded_methods as u64,
            universal_assertion: self.universal_methods > 0
                && self.universal_methods <= self.returned_objects,
            excluded_assertion: self.excluded_methods > 0
                && self.candidate_methods == self.returned_objects + self.excluded_methods,
        }
    }
}

fn input_identity(provider_path: &Path) -> Result<InputIdentityReport, io::Error> {
    Ok(InputIdentityReport {
        platform_version: PLATFORM_VERSION,
        source_locale: SOURCE_LOCALE,
        provider_schema_version: PROVIDER_SCHEMA_VERSION,
        extraction_schema_version: EXTRACTION_SCHEMA_VERSION,
        hbk: expected_artifact_report(
            Path::new(SOURCE_HBK_PATH),
            SOURCE_HBK_BYTES,
            SOURCE_HBK_SHA256,
        )?,
        provider: expected_artifact_report(provider_path, PROVIDER_BYTES, PROVIDER_SHA256)?,
    })
}

fn expected_artifact_report(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
) -> Result<ArtifactReport, io::Error> {
    let report = artifact_report(path)?;
    if report.bytes != expected_bytes || report.sha256 != expected_sha256 {
        return Err(io::Error::other(format!(
            "S83-AV1 input identity mismatch for {}: expected {} bytes / {}, observed {} bytes / {}",
            path.display(),
            expected_bytes,
            expected_sha256,
            report.bytes,
            report.sha256
        )));
    }
    Ok(report)
}

fn artifact_report(path: &Path) -> Result<ArtifactReport, io::Error> {
    Ok(ArtifactReport {
        path: path.display().to_string(),
        bytes: fs::metadata(path)?.len(),
        sha256: sha256sum(path)?,
    })
}

fn sha256sum(path: &Path) -> Result<String, io::Error> {
    let output = process::Command::new("sha256sum").arg(path).output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "sha256sum failed for {} with status {}",
            path.display(),
            output.status
        )));
    }
    let stdout = String::from_utf8(output.stdout).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("sha256sum output is not utf-8: {error}"),
        )
    })?;
    stdout
        .split_whitespace()
        .next()
        .map(str::to_owned)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "empty sha256sum output"))
}

fn duration_ns(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
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
    decision_role: &'static str,
    baseline_role: &'static str,
    module_context_filter_used: bool,
    empty_availability_rule: &'static str,
    availability_context: &'static str,
    iterations: usize,
    input_identity: InputIdentityReport,
    index: ArtifactReport,
    cache: Option<ArtifactReport>,
    cache_status: Option<&'static str>,
    counts: EnumerationCountsReport,
    timings: TimingReport,
    allocations: AllocationEvidenceReport,
    transcript: Vec<TranscriptObject>,
}

#[derive(Serialize)]
struct InputIdentityReport {
    platform_version: &'static str,
    source_locale: &'static str,
    provider_schema_version: u32,
    extraction_schema_version: u32,
    hbk: ArtifactReport,
    provider: ArtifactReport,
}

#[derive(Serialize)]
struct ArtifactReport {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Serialize)]
struct EnumerationCountsReport {
    scanned_globals: u64,
    candidate_methods: u64,
    returned_objects: u64,
    universal_objects: u64,
    explicit_context_objects: u64,
    excluded_objects: u64,
    universal_assertion: bool,
    excluded_assertion: bool,
}

#[derive(Serialize)]
struct TimingReport {
    phase_order: &'static [&'static str],
    entry_to_ready_ns: u64,
    open: PhaseReport,
    first_enumeration: EnumerationPhaseReport,
    warmup: EnumerationPhaseReport,
    workload: WorkloadReport,
}

#[derive(Serialize)]
struct PhaseReport {
    elapsed_ns: u64,
    faults: ProcessFaults,
}

#[derive(Serialize)]
struct EnumerationPhaseReport {
    elapsed_ns: u64,
    ns_per_object: Option<u64>,
    faults: ProcessFaults,
    returned_objects: u64,
    checksum: u64,
}

#[derive(Serialize)]
struct WorkloadReport {
    elapsed_ns: u64,
    average_ns: u64,
    ns_per_object: Option<u64>,
    faults: ProcessFaults,
    iterations: usize,
    returned_total: u64,
    checksum: u64,
}

#[derive(Serialize)]
struct AllocationEvidenceReport {
    enabled: bool,
    entry_to_ready: AllocationDeltaReport,
    first_enumeration: AllocationDeltaReport,
    warmup: AllocationDeltaReport,
    workload: AllocationDeltaReport,
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

#[derive(Serialize)]
struct TranscriptObject {
    global_id: String,
    global_kind: &'static str,
    global_domain: &'static str,
    global_name: TranscriptName,
    global_type_refs: Vec<TranscriptTypeRef>,
    availability_contexts: Vec<String>,
    callable_id: String,
    callable_owner: Option<String>,
    callable_kind: &'static str,
    callable_name: TranscriptName,
    callable_availability_contexts: Vec<String>,
    availability_rule: &'static str,
    signatures: Vec<TranscriptSignature>,
    callable_return_type_refs: Vec<TranscriptTypeRef>,
}

#[derive(Serialize)]
struct TranscriptName {
    primary: String,
    alias: Option<String>,
}

#[derive(Serialize)]
struct TranscriptSignature {
    text: String,
    parameters: Vec<TranscriptParameter>,
    return_type_refs: Vec<TranscriptTypeRef>,
}

#[derive(Serialize)]
struct TranscriptParameter {
    name: String,
    required: bool,
    type_refs: Vec<TranscriptTypeRef>,
}

#[derive(Serialize)]
struct TranscriptTypeRef {
    name: String,
    target: TranscriptTypeRefTarget,
    type_template_key: Option<TranscriptTemplateKey>,
    template_binding: Option<TranscriptTemplateBinding>,
}

#[derive(Serialize)]
struct TranscriptTypeRefTarget {
    status: &'static str,
    values: Vec<String>,
}

#[derive(Serialize)]
struct TranscriptTemplateKey {
    family: String,
    variant: String,
}

#[derive(Serialize)]
struct TranscriptTemplateBinding {
    template_key: TranscriptTemplateKey,
    arguments: Vec<TranscriptTemplateArgument>,
}

#[derive(Serialize)]
struct TranscriptTemplateArgument {
    kind: &'static str,
    owner_parameter_index: u64,
    target_parameter_index: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_exact_nine_context_manifest_codes() {
        for code in AVAILABILITY_CONTEXTS {
            assert_eq!(
                AvailabilityContext::parse(code).map(|context| context.code),
                Some(*code)
            );
        }
        assert_eq!(AVAILABILITY_CONTEXTS.len(), 9);
    }

    #[test]
    fn rejects_context_codes_outside_manifest() {
        assert!(AvailabilityContext::parse("not_an_availability_context").is_none());
        assert!(AvailabilityContext::parse("thin-client").is_none());
        assert!(AvailabilityContext::parse("THIN_CLIENT").is_none());
    }

    #[test]
    fn parses_sql_owned_command_context_before_iterations() {
        let command = parse_args([
            "sql-owned".to_string(),
            "index.sqlite".to_string(),
            "server".to_string(),
            "7".to_string(),
        ])
        .unwrap();
        assert_eq!(
            command,
            Command::SqlOwned {
                index_path: PathBuf::from("index.sqlite"),
                context: AvailabilityContext { code: "server" },
                iterations: 7,
            }
        );
    }

    #[test]
    fn parses_cache_owned_command_context_before_iterations() {
        let command = parse_args([
            "cache-owned".to_string(),
            "index.sqlite".to_string(),
            "cache.bin".to_string(),
            "web_client".to_string(),
        ])
        .unwrap();
        assert_eq!(
            command,
            Command::CacheOwned {
                index_path: PathBuf::from("index.sqlite"),
                cache_path: PathBuf::from("cache.bin"),
                context: AvailabilityContext { code: "web_client" },
                iterations: DEFAULT_ITERATIONS,
            }
        );
    }

    #[test]
    fn counts_report_tracks_universal_and_excluded_assertions() {
        let sample = EnumerationSample {
            scanned_globals: 11,
            candidate_methods: 5,
            universal_methods: 2,
            explicit_context_methods: 1,
            excluded_methods: 2,
            returned_objects: 3,
            checksum: 42,
        };
        let counts = sample.counts();
        assert_eq!(counts.returned_objects, 3);
        assert_eq!(counts.universal_objects, 2);
        assert_eq!(counts.explicit_context_objects, 1);
        assert_eq!(counts.excluded_objects, 2);
        assert!(counts.universal_assertion);
        assert!(counts.excluded_assertion);
    }

    #[test]
    fn availability_filter_treats_empty_as_universal() {
        assert_eq!(
            availability_match_codes([], AvailabilityContext { code: "server" }),
            AvailabilityMatch::Universal
        );
    }

    #[test]
    fn availability_filter_accepts_only_an_explicit_context_hit() {
        let contexts = ["thin_client", "server"];
        assert_eq!(
            availability_match_codes(contexts, AvailabilityContext { code: "server" }),
            AvailabilityMatch::Explicit
        );
        assert_eq!(
            availability_match_codes(contexts, AvailabilityContext { code: "web_client" }),
            AvailabilityMatch::Excluded
        );
    }

    #[test]
    fn enumeration_consistency_requires_count_and_ordered_payload_checksum() {
        let expected = EnumerationSample {
            scanned_globals: 10,
            candidate_methods: 4,
            universal_methods: 1,
            explicit_context_methods: 2,
            excluded_methods: 1,
            returned_objects: 3,
            checksum: 42,
        };
        let mut actual = expected.clone();
        assert!(ensure_same_enumeration("warmup", None, &expected, &actual).is_ok());

        actual.checksum = 43;
        assert!(ensure_same_enumeration("workload", Some(1), &expected, &actual).is_err());
    }
}
