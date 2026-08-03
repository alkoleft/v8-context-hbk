use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs;
use std::hint::black_box;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::{env, process};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use syntax_helper_search::{
    HbkCallableId, HbkCallableKind, HbkFactReadHandle, HbkFactRef, HbkFactSnapshot,
    HbkFactSnapshotCacheStatus, HbkName, HbkSnapshotExperimentAllocationDelta,
    HbkSnapshotExperimentAllocationSnapshot, HbkSnapshotExperimentAllocator, HbkTypeMemberId,
    HbkTypeMemberKind, HbkTypeRef, HbkTypeRefTarget, HbkTypeTemplateBinding, StringId,
    experiment_allocation_snapshot, model, write_owned_snapshot_oracle_jsonl,
};

#[global_allocator]
static ALLOCATOR: HbkSnapshotExperimentAllocator = HbkSnapshotExperimentAllocator;

const MANIFEST_SCHEMA_VERSION: &str = "hbk-s83-av2-query-manifest/v1";
const REPORT_SCHEMA_VERSION: &str = "hbk-s83-av2-benchmark/v1";
const PARITY_SCHEMA_VERSION: &str = "hbk-s83-av2-parity/v1";
const WORKLOAD_VERSION: &str = "s83-av2-context-member-access/v1";
const DATASET: &str = "shcntx_ru-8.3.27.1859-schema16-extraction11";
const PLATFORM_VERSION: &str = "8.3.27.1859";
const PROVIDER_SCHEMA_VERSION: u32 = 16;
const EXTRACTION_SCHEMA_VERSION: u32 = 11;
const SOURCE_LOCALE: &str = "ru";
const SOURCE_HBK_PATH: &str = "/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk";
const SOURCE_HBK_BYTES: u64 = 40_744_845;
const SOURCE_HBK_SHA256: &str = "5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48";
const PROVIDER_BYTES: u64 = 204_288_000;
const PROVIDER_SHA256: &str = "55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab";
const DEFAULT_ITERATIONS: usize = 100;
const DEFAULT_MEMBER_ITERATIONS: usize = 1_000;
const LOCATOR_SIZE: u64 = 4;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const PHASE_ORDER: &[&str] = &[
    "entry_to_ready",
    "anchor_resolution",
    "first_operation",
    "warmup",
    "steady_workload",
    "memory_sample",
];
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
        Ok(Output::Report(report)) => write_json(&report),
        Ok(Output::Manifest(manifest)) => write_json(&manifest),
        Ok(Output::Parity(parity)) => write_json(&parity),
        Err(error) => {
            eprintln!("benchmark failed: {error}");
            process::exit(1);
        }
    }
}

fn write_json(value: &impl Serialize) {
    if let Err(error) = serde_json::to_writer(io::stdout().lock(), value) {
        eprintln!("failed to write json: {error}");
        process::exit(1);
    }
    println!();
}

fn run(
    entry_started_at: Instant,
    entry_faults: ProcessFaults,
    entry_allocations: HbkSnapshotExperimentAllocationSnapshot,
) -> Result<Output, Box<dyn std::error::Error>> {
    let command = parse_args(env::args().skip(1))?;
    match command {
        Command::Manifest { source } => {
            let loaded = load_snapshot(source)?;
            Ok(Output::Manifest(build_manifest(&loaded)?))
        }
        Command::Parity {
            source,
            manifest_path,
            context,
        } => {
            let loaded = load_snapshot(source)?;
            let manifest = read_manifest(&manifest_path)?;
            let prepared =
                PreparedManifest::from_manifest(&loaded.snapshot, &manifest, Some(context))?;
            Ok(Output::Parity(build_parity(
                &loaded,
                &manifest_path,
                context,
                &loaded.snapshot,
                &manifest,
                &prepared,
            )?))
        }
        Command::Measure {
            source,
            manifest_path,
            operation,
            context,
            iterations,
        } => {
            let loaded = load_snapshot(source)?;
            let ready_elapsed = entry_started_at.elapsed();
            let ready_faults = read_process_faults()?;
            let ready_allocations = experiment_allocation_snapshot();
            let anchor =
                measure_anchor_resolution(&loaded.snapshot, &manifest_path, operation, context)?;
            let report = measure_operation(
                &loaded,
                &manifest_path,
                operation,
                context,
                iterations,
                ready_elapsed,
                entry_faults.delta_to(ready_faults),
                ready_allocations.delta_since(entry_allocations),
                anchor,
            )?;
            Ok(Output::Report(report))
        }
    }
}

enum Output {
    Report(BenchmarkReport),
    Manifest(QueryManifest),
    Parity(ParityReport),
}

struct LoadedSnapshot {
    snapshot: HbkFactSnapshot,
    backend: &'static str,
    decision_role: &'static str,
    index_path: PathBuf,
    cache_path: Option<PathBuf>,
}

fn load_snapshot(source: Source) -> Result<LoadedSnapshot, Box<dyn std::error::Error>> {
    match source {
        Source::SqlOwned { index_path } => {
            let report = HbkFactSnapshot::from_path_with_stage_timings(&index_path)?;
            Ok(LoadedSnapshot {
                snapshot: report.snapshot,
                backend: "S83-H0",
                decision_role: "baseline",
                index_path,
                cache_path: None,
            })
        }
        Source::CacheOwned {
            index_path,
            cache_path,
        } => {
            let report = HbkFactSnapshot::from_path_with_binary_cache(&index_path, &cache_path)?;
            if let HbkFactSnapshotCacheStatus::Rebuilt { reason } = &report.status {
                return Err(io::Error::other(format!(
                    "cache-owned requires status Loaded, but cache was rebuilt: {reason}"
                ))
                .into());
            }
            Ok(LoadedSnapshot {
                snapshot: report.snapshot,
                backend: "S83-C0",
                decision_role: "control",
                index_path,
                cache_path: Some(cache_path),
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Manifest {
        source: Source,
    },
    Parity {
        source: Source,
        manifest_path: PathBuf,
        context: AvailabilityContext,
    },
    Measure {
        source: Source,
        manifest_path: PathBuf,
        operation: Operation,
        context: Option<AvailabilityContext>,
        iterations: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    SqlOwned {
        index_path: PathBuf,
    },
    CacheOwned {
        index_path: PathBuf,
        cache_path: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Operation {
    TypeByName,
    PropertyByOwnerNameKind,
    MethodByOwnerNameKind,
    CallableByOwnerName,
    MembersByOwnerAvailabilityBorrowed,
    MembersByOwnerAvailabilityCollect,
    TypePayload,
    MethodPayload,
    PropertyPayload,
    FilteredMembersPayload,
}

impl Operation {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "type_by_name" => Some(Self::TypeByName),
            "property_by_owner_name_kind" => Some(Self::PropertyByOwnerNameKind),
            "method_by_owner_name_kind" => Some(Self::MethodByOwnerNameKind),
            "callable_by_owner_name" => Some(Self::CallableByOwnerName),
            "members_by_owner_availability_borrowed" => {
                Some(Self::MembersByOwnerAvailabilityBorrowed)
            }
            "members_by_owner_availability_collect" => {
                Some(Self::MembersByOwnerAvailabilityCollect)
            }
            "type_payload" => Some(Self::TypePayload),
            "method_payload" => Some(Self::MethodPayload),
            "property_payload" => Some(Self::PropertyPayload),
            "filtered_members_payload" => Some(Self::FilteredMembersPayload),
            _ => None,
        }
    }

    fn code(self) -> &'static str {
        match self {
            Self::TypeByName => "type_by_name",
            Self::PropertyByOwnerNameKind => "property_by_owner_name_kind",
            Self::MethodByOwnerNameKind => "method_by_owner_name_kind",
            Self::CallableByOwnerName => "callable_by_owner_name",
            Self::MembersByOwnerAvailabilityBorrowed => "members_by_owner_availability_borrowed",
            Self::MembersByOwnerAvailabilityCollect => "members_by_owner_availability_collect",
            Self::TypePayload => "type_payload",
            Self::MethodPayload => "method_payload",
            Self::PropertyPayload => "property_payload",
            Self::FilteredMembersPayload => "filtered_members_payload",
        }
    }

    fn requires_context(self) -> bool {
        matches!(
            self,
            Self::MembersByOwnerAvailabilityBorrowed
                | Self::MembersByOwnerAvailabilityCollect
                | Self::FilteredMembersPayload
        )
    }

    fn default_iterations(self) -> usize {
        if self.requires_context() {
            DEFAULT_MEMBER_ITERATIONS
        } else {
            DEFAULT_ITERATIONS
        }
    }

    fn projection(self) -> ProjectionReport {
        let source = if matches!(
            self,
            Self::TypePayload
                | Self::MethodPayload
                | Self::PropertyPayload
                | Self::FilteredMembersPayload
        ) {
            "owned-reference"
        } else {
            "owned-id-slice"
        };
        ProjectionReport {
            source,
            compact: (self == Self::MembersByOwnerAvailabilityCollect)
                .then_some("av2-member-locator-u32"),
        }
    }
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, io::Error> {
    let mut args = args.into_iter();
    let command = args.next().ok_or_else(usage_error)?;
    let parsed = match command.as_str() {
        "manifest" => Command::Manifest {
            source: parse_source(&mut args)?,
        },
        "parity" => {
            let source = parse_source(&mut args)?;
            let manifest_path = required_path(&mut args)?;
            Command::Parity {
                source,
                manifest_path,
                context: required_context(&mut args)?,
            }
        }
        "measure" | "performance" => {
            let source = parse_source(&mut args)?;
            let manifest_path = required_path(&mut args)?;
            let operation = required_operation(&mut args)?;
            let context = if operation.requires_context() {
                Some(required_context(&mut args)?)
            } else {
                None
            };
            let iterations = optional_iterations(&mut args, operation.default_iterations())?;
            Command::Measure {
                source,
                manifest_path,
                operation,
                context,
                iterations,
            }
        }
        _ => return Err(usage_error()),
    };
    if args.next().is_some() {
        return Err(usage_error());
    }
    Ok(parsed)
}

fn parse_source(args: &mut impl Iterator<Item = String>) -> Result<Source, io::Error> {
    let mode = args.next().ok_or_else(usage_error)?;
    match mode.as_str() {
        "sql-owned" => Ok(Source::SqlOwned {
            index_path: required_path(args)?,
        }),
        "cache-owned" => Ok(Source::CacheOwned {
            index_path: required_path(args)?,
            cache_path: required_path(args)?,
        }),
        _ => Err(usage_error()),
    }
}

fn required_path(args: &mut impl Iterator<Item = String>) -> Result<PathBuf, io::Error> {
    args.next().map(PathBuf::from).ok_or_else(usage_error)
}

fn required_operation(args: &mut impl Iterator<Item = String>) -> Result<Operation, io::Error> {
    let value = args.next().ok_or_else(usage_error)?;
    Operation::parse(&value).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown S83-AV2 operation {value:?}"),
        )
    })
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

fn optional_iterations(
    args: &mut impl Iterator<Item = String>,
    default: usize,
) -> Result<usize, io::Error> {
    let Some(value) = args.next() else {
        return Ok(default);
    };
    if value.is_empty() {
        return optional_iterations(args, default);
    }
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
        "usage: measure_hbk_s83_av2 manifest <sql-owned <index.sqlite> | cache-owned <index.sqlite> <cache.bin>> | \
         parity <sql-owned <index.sqlite> | cache-owned <index.sqlite> <cache.bin>> <manifest.json> <availability-context> | \
         measure <sql-owned <index.sqlite> | cache-owned <index.sqlite> <cache.bin>> <manifest.json> <operation> [availability-context] [iterations]",
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

#[derive(Debug, Deserialize)]
struct OracleTypeRecord {
    record: String,
    key: LogicalKey,
    name: TranscriptName,
}

#[derive(Debug, Deserialize)]
struct OracleMemberRecord {
    record: String,
    key: LogicalKey,
    owner: LogicalKey,
    kind: String,
    name: TranscriptName,
}

#[derive(Debug, Deserialize)]
struct LogicalKey(String, String);

fn build_manifest(loaded: &LoadedSnapshot) -> Result<QueryManifest, Box<dyn std::error::Error>> {
    let snapshot = &loaded.snapshot;
    let mut oracle = Vec::new();
    write_owned_snapshot_oracle_jsonl(snapshot, &mut oracle)?;
    let mut types = Vec::new();
    let mut members = Vec::new();

    for line in String::from_utf8(oracle)?.lines() {
        let value: Value = serde_json::from_str(line)?;
        match value.get("record").and_then(Value::as_str) {
            Some("platform_type") => {
                let record: OracleTypeRecord = serde_json::from_value(value)?;
                debug_assert_eq!(record.record, "platform_type");
                types.push(ManifestType {
                    logical_id: logical_key_id(record.key, "platform_type")?,
                    primary: record.name.primary,
                    alias: record.name.alias,
                    member_count: 0,
                });
            }
            Some("type_member") => {
                let record: OracleMemberRecord = serde_json::from_value(value)?;
                debug_assert_eq!(record.record, "type_member");
                if let Some(kind) = manifest_member_kind(&record.kind) {
                    members.push(ManifestMember {
                        logical_id: logical_key_id(record.key, "type_member")?,
                        owner_logical_id: logical_key_id(record.owner, "platform_type")?,
                        kind: kind.to_owned(),
                        primary: record.name.primary,
                        alias: record.name.alias,
                    });
                }
            }
            _ => {}
        }
    }

    let mut member_counts = HashMap::<String, u64>::new();
    for member in &members {
        *member_counts
            .entry(member.owner_logical_id.clone())
            .or_default() += 1;
    }
    for ty in &mut types {
        ty.member_count = member_counts.remove(&ty.logical_id).unwrap_or(0);
    }
    let lookup_queries = lookup_queries(&types, &members);
    let anchors = ManifestAnchors {
        type_primary: "Запрос".to_owned(),
        property_owner: "platform_type:Запрос".to_owned(),
        property_name: "Текст".to_owned(),
        method_owner: "platform_type:Запрос".to_owned(),
        method_name: "Выполнить".to_owned(),
        enumeration_owner: "platform_type:ФормаКлиентскогоПриложения".to_owned(),
    };
    Ok(QueryManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_owned(),
        workload_version: WORKLOAD_VERSION.to_owned(),
        input_identity: input_identity(&loaded.index_path)?,
        availability_contexts: AVAILABILITY_CONTEXTS
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        member_kinds: ["property", "method", "event", "enum_value"]
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        empty_availability_rule: "universal".to_owned(),
        module_context_filter_used: false,
        types,
        members,
        lookup_queries,
        fixed_misses: ManifestMisses {
            type_name: "__hbk_s83_av2_missing_type__".to_owned(),
            member_name: "__hbk_s83_av2_missing_member__".to_owned(),
            callable_name: "__hbk_s83_av2_missing_callable__".to_owned(),
        },
        anchors,
    })
}

fn logical_key_id(key: LogicalKey, expected_family: &str) -> Result<String, io::Error> {
    if key.0 != expected_family {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "snapshot oracle key family {:?} differs from expected {expected_family:?}",
                key.0
            ),
        ));
    }
    Ok(key.1)
}

fn read_manifest(path: &Path) -> Result<QueryManifest, io::Error> {
    let text = fs::read_to_string(path)?;
    if text.contains("ModuleContextKind") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "S83-AV2 manifest must not contain ModuleContextKind",
        ));
    }
    let manifest: QueryManifest = serde_json::from_str(&text).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid S83-AV2 manifest json: {error}"),
        )
    })?;
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION
        || manifest.workload_version != WORKLOAD_VERSION
        || manifest.module_context_filter_used
        || manifest.empty_availability_rule != "universal"
        || !same_strings(&manifest.availability_contexts, AVAILABILITY_CONTEXTS)
        || !same_strings(
            &manifest.member_kinds,
            &["property", "method", "event", "enum_value"],
        )
        || manifest.fixed_misses.type_name != "__hbk_s83_av2_missing_type__"
        || manifest.fixed_misses.member_name != "__hbk_s83_av2_missing_member__"
        || manifest.fixed_misses.callable_name != "__hbk_s83_av2_missing_callable__"
        || manifest.anchors.type_primary != "Запрос"
        || manifest.anchors.property_owner != "platform_type:Запрос"
        || manifest.anchors.property_name != "Текст"
        || manifest.anchors.method_owner != "platform_type:Запрос"
        || manifest.anchors.method_name != "Выполнить"
        || manifest.anchors.enumeration_owner != "platform_type:ФормаКлиентскогоПриложения"
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "manifest identity or AV2 invariants do not match hbk-s83-av2-query-manifest/v1",
        ));
    }
    Ok(manifest)
}

fn same_strings(values: &[String], expected: &[&str]) -> bool {
    values.len() == expected.len()
        && values
            .iter()
            .zip(expected)
            .all(|(value, expected)| value == expected)
}

fn lookup_queries(types: &[ManifestType], members: &[ManifestMember]) -> LookupManifest {
    let mut type_names = Vec::new();
    for ty in types {
        type_names.push(ManifestTypeNameQuery {
            logical_id: ty.logical_id.clone(),
            query_name: ty.primary.clone(),
            query_role: "primary".to_owned(),
        });
        if let Some(alias) = &ty.alias
            && alias != &ty.primary
        {
            type_names.push(ManifestTypeNameQuery {
                logical_id: ty.logical_id.clone(),
                query_name: alias.clone(),
                query_role: "alias".to_owned(),
            });
        }
    }
    let mut properties = Vec::new();
    let mut methods = Vec::new();
    for member in members {
        match member.kind.as_str() {
            "property" => push_member_lookup_queries(&mut properties, member),
            "method" => push_member_lookup_queries(&mut methods, member),
            _ => {}
        }
    }
    LookupManifest {
        type_names,
        properties,
        methods,
    }
}

fn push_member_lookup_queries(values: &mut Vec<ManifestMemberQuery>, member: &ManifestMember) {
    values.push(ManifestMemberQuery {
        logical_id: member.logical_id.clone(),
        owner_logical_id: member.owner_logical_id.clone(),
        kind: member.kind.clone(),
        query_name: member.primary.clone(),
        query_role: "primary".to_owned(),
    });
    if let Some(alias) = &member.alias
        && alias != &member.primary
    {
        values.push(ManifestMemberQuery {
            logical_id: member.logical_id.clone(),
            owner_logical_id: member.owner_logical_id.clone(),
            kind: member.kind.clone(),
            query_name: alias.clone(),
            query_role: "alias".to_owned(),
        });
    }
}

fn manifest_member_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "property" => Some("property"),
        "method" => Some("method"),
        "event" => Some("event"),
        "enum_value" => Some("enum_value"),
        _ => None,
    }
}

struct PreparedManifest {
    owners: Vec<PreparedOwner>,
    member_ids: Vec<HbkTypeMemberId>,
    callable_ids: Vec<HbkCallableId>,
    type_locator_by_id: HashMap<syntax_helper_search::HbkPlatformTypeId, Av2TypeLocator>,
    member_locator_by_id: HashMap<HbkTypeMemberId, Av2MemberLocator>,
    callable_locator_by_id: HashMap<HbkCallableId, Av2CallableLocator>,
    type_queries: Vec<PreparedTypeQuery>,
    property_queries: Vec<PreparedMemberQuery>,
    method_queries: Vec<PreparedMemberQuery>,
    callable_queries: Vec<PreparedCallableQuery>,
    type_payloads: Vec<PreparedTypePayload>,
    method_payloads: Vec<PreparedMethodPayload>,
    property_payloads: Vec<Av2MemberLocator>,
    filtered_members_by_context: Vec<PreparedContextMemberSets>,
}

impl PreparedManifest {
    fn from_manifest(
        snapshot: &HbkFactSnapshot,
        manifest: &QueryManifest,
        filtered_context: Option<AvailabilityContext>,
    ) -> Result<Self, io::Error> {
        let handle = snapshot.worker_handle();
        let mut type_ids = Vec::new();
        let mut type_locator_by_id = HashMap::new();
        let mut member_ids = Vec::new();
        let mut member_locator_by_id = HashMap::new();
        let mut callable_ids = Vec::new();
        let mut callable_locator_by_id = HashMap::new();
        let mut owners = Vec::with_capacity(manifest.types.len());
        let mut type_payloads = Vec::with_capacity(manifest.types.len());

        for ty in &manifest.types {
            let owner = resolve_type_by_logical_id(snapshot, handle, &ty.logical_id, &ty.primary)?;
            type_locator_by_id.entry(owner).or_insert_with(|| {
                let locator = Av2TypeLocator(type_ids.len() as u32);
                type_ids.push(owner);
                locator
            });
            let members = handle.members_of_type(owner);
            let mut owner_members = Vec::with_capacity(members.len());
            let mut owner_member_kinds = Vec::with_capacity(members.len());
            for member_id in members {
                let locator = *member_locator_by_id.entry(member_id).or_insert_with(|| {
                    let locator = Av2MemberLocator(member_ids.len() as u32);
                    member_ids.push(member_id);
                    locator
                });
                owner_members.push(locator);
                owner_member_kinds.push(snapshot.type_member(member_id).kind);
            }
            owners.push(PreparedOwner {
                logical_id: ty.logical_id.clone(),
                owner,
                members: owner_members,
                member_kinds: owner_member_kinds,
            });
            type_payloads.push(PreparedTypePayload { owner });
        }

        let type_queries = prepare_type_queries(handle, manifest, &type_locator_by_id);

        let property_queries = prepare_member_queries(
            snapshot,
            handle,
            &manifest.lookup_queries.properties,
            HbkTypeMemberKind::Property,
            &member_locator_by_id,
        )?;
        let method_queries = prepare_member_queries(
            snapshot,
            handle,
            &manifest.lookup_queries.methods,
            HbkTypeMemberKind::Method,
            &member_locator_by_id,
        )?;
        let callable_queries = manifest
            .lookup_queries
            .methods
            .iter()
            .map(|query| {
                let owner =
                    resolve_type_by_logical_id(snapshot, handle, &query.owner_logical_id, "")?;
                let expected = handle
                    .callable_by_owner_name(owner, &query.query_name)
                    .map(|callable| {
                        *callable_locator_by_id.entry(callable).or_insert_with(|| {
                            let locator = Av2CallableLocator(callable_ids.len() as u32);
                            callable_ids.push(callable);
                            locator
                        })
                    })
                    .collect();
                Ok(PreparedCallableQuery {
                    owner,
                    name: query.query_name.clone(),
                    expected,
                })
            })
            .collect::<Result<Vec<_>, io::Error>>()?;
        let method_payloads = manifest
            .members
            .iter()
            .filter(|member| member.kind == "method")
            .map(|member| {
                let member_id = resolve_member_by_logical_id(snapshot, handle, member)?;
                let member_locator = *member_locator_by_id.get(&member_id).ok_or_else(|| {
                    io::Error::other(format!(
                        "method locator not prepared: {}",
                        member.logical_id
                    ))
                })?;
                let callables = handle
                    .callable_by_owner_name(
                        snapshot.type_member(member_id).owner,
                        snapshot.string(snapshot.type_member(member_id).name.primary),
                    )
                    .map(|callable| {
                        *callable_locator_by_id.entry(callable).or_insert_with(|| {
                            let locator = Av2CallableLocator(callable_ids.len() as u32);
                            callable_ids.push(callable);
                            locator
                        })
                    })
                    .collect();
                Ok(PreparedMethodPayload {
                    member: member_locator,
                    callables,
                })
            })
            .collect::<Result<Vec<_>, io::Error>>()?;
        let property_payloads = manifest
            .members
            .iter()
            .filter(|member| member.kind == "property")
            .map(|member| {
                let member_id = resolve_member_by_logical_id(snapshot, handle, member)?;
                member_locator_by_id
                    .get(&member_id)
                    .copied()
                    .ok_or_else(|| {
                        io::Error::other(format!(
                            "property locator not prepared: {}",
                            member.logical_id
                        ))
                    })
            })
            .collect::<Result<Vec<_>, io::Error>>()?;
        let filtered_members_by_context = filtered_context
            .map(|context| {
                vec![prepare_filtered_member_sets(
                    snapshot,
                    handle,
                    context,
                    &owners,
                    &member_ids,
                )]
            })
            .unwrap_or_default();
        Ok(Self {
            owners,
            member_ids,
            callable_ids,
            type_locator_by_id,
            member_locator_by_id,
            callable_locator_by_id,
            type_queries,
            property_queries,
            method_queries,
            callable_queries,
            type_payloads,
            method_payloads,
            property_payloads,
            filtered_members_by_context,
        })
    }
}

fn resolve_type_by_logical_id(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    logical_id: &str,
    primary: &str,
) -> Result<syntax_helper_search::HbkPlatformTypeId, io::Error> {
    let name = primary.strip_prefix("platform_type:").unwrap_or(primary);
    handle
        .platform_types_by_name(name)
        .find(|id| snapshot.string(snapshot.platform_type(*id).id) == logical_id)
        .or_else(|| handle.platform_type_by_id(logical_id))
        .ok_or_else(|| io::Error::other(format!("manifest type not found: {logical_id}")))
}

fn prepare_type_queries(
    handle: HbkFactReadHandle<'_>,
    manifest: &QueryManifest,
    locator_by_id: &HashMap<syntax_helper_search::HbkPlatformTypeId, Av2TypeLocator>,
) -> Vec<PreparedTypeQuery> {
    manifest
        .lookup_queries
        .type_names
        .iter()
        .map(|query| {
            let expected = handle
                .platform_types_by_name(&query.query_name)
                .filter_map(|id| locator_by_id.get(&id).copied())
                .collect();
            PreparedTypeQuery {
                name: query.query_name.clone(),
                expected,
            }
        })
        .collect()
}

fn resolve_member_by_logical_id(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    manifest_member: &ManifestMember,
) -> Result<HbkTypeMemberId, io::Error> {
    let owner =
        resolve_type_by_logical_id(snapshot, handle, &manifest_member.owner_logical_id, "")?;
    let kind = match manifest_member.kind.as_str() {
        "property" => HbkTypeMemberKind::Property,
        "method" => HbkTypeMemberKind::Method,
        "event" => HbkTypeMemberKind::Event,
        "enum_value" => HbkTypeMemberKind::EnumValue,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown manifest member kind {}", manifest_member.kind),
            ));
        }
    };
    handle
        .member_by_owner_name_kind(owner, &manifest_member.primary, Some(kind))
        .find(|id| snapshot.string(snapshot.type_member(*id).id) == manifest_member.logical_id)
        .ok_or_else(|| {
            io::Error::other(format!(
                "manifest member not found: {}",
                manifest_member.logical_id
            ))
        })
}

fn prepare_member_queries(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    members: &[ManifestMemberQuery],
    kind: HbkTypeMemberKind,
    locator_by_id: &HashMap<HbkTypeMemberId, Av2MemberLocator>,
) -> Result<Vec<PreparedMemberQuery>, io::Error> {
    members
        .iter()
        .map(|member| {
            let owner = resolve_type_by_logical_id(snapshot, handle, &member.owner_logical_id, "")?;
            let expected = handle
                .member_by_owner_name_kind(owner, &member.query_name, Some(kind))
                .filter_map(|member_id| locator_by_id.get(&member_id).copied())
                .collect();
            Ok(PreparedMemberQuery {
                owner,
                name: member.query_name.clone(),
                kind,
                expected,
            })
        })
        .collect()
}

fn prepare_filtered_member_sets(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    context: AvailabilityContext,
    owners: &[PreparedOwner],
    member_ids: &[HbkTypeMemberId],
) -> PreparedContextMemberSets {
    let owners = owners
        .iter()
        .map(|owner| {
            let members = owner
                .members
                .iter()
                .copied()
                .filter(|locator| {
                    let member = member_ids[locator.0 as usize];
                    availability_match(
                        snapshot,
                        handle.availability_contexts(HbkFactRef::TypeMember(member)),
                        context,
                    )
                    .included()
                })
                .collect();
            PreparedFilteredOwner { members }
        })
        .collect();
    PreparedContextMemberSets { context, owners }
}

fn measure_anchor_resolution(
    snapshot: &HbkFactSnapshot,
    manifest_path: &Path,
    operation: Operation,
    context: Option<AvailabilityContext>,
) -> Result<MeasuredPhase<AnchorResolution>, io::Error> {
    let start_faults = read_process_faults()?;
    let before_allocations = experiment_allocation_snapshot();
    let started_at = Instant::now();
    let manifest = read_manifest(manifest_path)?;
    let operation_anchor = resolve_operation_anchor(snapshot, &manifest, operation, context)?;
    let checksum = operation_anchor.checksum(operation);
    let elapsed = started_at.elapsed();
    let allocations = experiment_allocation_snapshot().delta_since(before_allocations);
    let faults = start_faults.delta_to(read_process_faults()?);
    Ok(MeasuredPhase {
        value: AnchorResolution {
            manifest,
            operation_anchor,
            checksum,
        },
        elapsed,
        faults,
        allocations,
    })
}

fn measure_operation(
    loaded: &LoadedSnapshot,
    manifest_path: &Path,
    operation: Operation,
    context: Option<AvailabilityContext>,
    iterations: usize,
    ready_elapsed: Duration,
    ready_faults: ProcessFaults,
    ready_allocations: HbkSnapshotExperimentAllocationDelta,
    anchor: MeasuredPhase<AnchorResolution>,
) -> Result<BenchmarkReport, Box<dyn std::error::Error>> {
    let first = measure_first_operation(
        &loaded.snapshot,
        &anchor.value.operation_anchor,
        operation,
        context,
    )?;
    let warmup =
        measure_prepare_and_warmup(&loaded.snapshot, &anchor.value.manifest, operation, context)?;
    let workload = measure_steady(
        &loaded.snapshot,
        &warmup.value.prepared,
        operation,
        context,
        iterations,
    )?;
    let memory =
        measure_memory_sample(&loaded.snapshot, &warmup.value.prepared, operation, context)?;
    let workload_value = workload.value.clone();
    let checksum = fnv1a(
        workload_value.checksum,
        &anchor.value.checksum.to_le_bytes(),
    );
    Ok(BenchmarkReport {
        schema_version: REPORT_SCHEMA_VERSION,
        workload_version: WORKLOAD_VERSION,
        mode: "performance",
        backend: loaded.backend,
        decision_role: loaded.decision_role,
        operation: operation.code(),
        availability_context: context.map(|context| context.code),
        iterations,
        module_context_filter_used: false,
        empty_availability_rule: "universal",
        input_identity: input_identity(&loaded.index_path)?,
        manifest: manifest_identity(manifest_path)?,
        runtime_artifacts: runtime_artifacts(loaded)?,
        projection: operation.projection(),
        phase_order: PHASE_ORDER,
        timings: TimingReport {
            entry_to_ready: TimingPhaseReport::phase(ready_elapsed, 1, 0, None),
            anchor_resolution: TimingPhaseReport::phase(
                anchor.elapsed,
                1,
                anchor.value.checksum,
                None,
            ),
            first_operation: TimingPhaseReport::sample(first.elapsed, &first.value),
            warmup: TimingPhaseReport::sample(warmup.elapsed, &warmup.value.sample),
            steady_workload: TimingPhaseReport::steady(
                workload.elapsed,
                iterations,
                &workload_value,
            ),
            memory_sample: TimingPhaseReport::phase(memory.elapsed, 1, 0, None),
        },
        faults: PhaseFaultsReport {
            entry_to_ready: ready_faults,
            anchor_resolution: anchor.faults,
            first_operation: first.faults,
            warmup: warmup.faults,
            steady_workload: workload.faults,
            memory_sample: memory.faults,
        },
        allocations: AllocationEvidenceReport {
            enabled: cfg!(feature = "snapshot-experiment-alloc"),
            entry_to_ready: ready_allocations.into(),
            anchor_resolution: anchor.allocations.into(),
            first_operation: first.allocations.into(),
            warmup: warmup.allocations.into(),
            steady_workload: workload.allocations.into(),
            memory_sample: memory.allocations.into(),
        },
        memory,
        counts: workload_value.counts,
        checksum: ChecksumReport {
            value: checksum,
            algorithm: "rolling-u64",
        },
        operation_data: operation_data(operation, workload_value),
    })
}

fn resolve_operation_anchor(
    snapshot: &HbkFactSnapshot,
    manifest: &QueryManifest,
    operation: Operation,
    context: Option<AvailabilityContext>,
) -> Result<OperationAnchor, io::Error> {
    let handle = snapshot.worker_handle();
    match operation {
        Operation::TypeByName => Ok(OperationAnchor::TypeByName {
            query_name: manifest.anchors.type_primary.clone(),
        }),
        Operation::PropertyByOwnerNameKind => Ok(OperationAnchor::PropertyLookup {
            owner: resolve_type_by_logical_id(
                snapshot,
                handle,
                &manifest.anchors.property_owner,
                "",
            )?,
            name: manifest.anchors.property_name.clone(),
        }),
        Operation::MethodByOwnerNameKind | Operation::CallableByOwnerName => {
            Ok(OperationAnchor::MethodLikeLookup {
                owner: resolve_type_by_logical_id(
                    snapshot,
                    handle,
                    &manifest.anchors.method_owner,
                    "",
                )?,
                name: manifest.anchors.method_name.clone(),
            })
        }
        Operation::TypePayload => Ok(OperationAnchor::TypePayload {
            type_id: resolve_anchor_type(snapshot, handle, manifest)?,
        }),
        Operation::PropertyPayload => Ok(OperationAnchor::PropertyPayload {
            member: resolve_anchor_member(snapshot, handle, manifest, HbkTypeMemberKind::Property)?,
        }),
        Operation::MethodPayload => {
            let member =
                resolve_anchor_member(snapshot, handle, manifest, HbkTypeMemberKind::Method)?;
            let owner = snapshot.type_member(member).owner;
            let callables = handle
                .callable_by_owner_name(owner, &manifest.anchors.method_name)
                .collect();
            Ok(OperationAnchor::MethodPayload { member, callables })
        }
        Operation::MembersByOwnerAvailabilityBorrowed
        | Operation::MembersByOwnerAvailabilityCollect => {
            let owner = resolve_type_by_logical_id(
                snapshot,
                handle,
                &manifest.anchors.enumeration_owner,
                "",
            )?;
            let member_kinds = handle
                .members_of_type(owner)
                .map(|member| snapshot.type_member(member).kind)
                .collect();
            Ok(OperationAnchor::MemberOwner {
                owner,
                member_kinds,
            })
        }
        Operation::FilteredMembersPayload => {
            let context =
                context.ok_or_else(|| io::Error::other("missing availability context"))?;
            let owner = resolve_type_by_logical_id(
                snapshot,
                handle,
                &manifest.anchors.enumeration_owner,
                "",
            )?;
            let members = handle
                .members_of_type(owner)
                .filter(|member| {
                    availability_match(
                        snapshot,
                        handle.availability_contexts(HbkFactRef::TypeMember(*member)),
                        context,
                    )
                    .included()
                })
                .collect();
            Ok(OperationAnchor::FilteredMembersPayload { members })
        }
    }
}

fn resolve_anchor_type(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    manifest: &QueryManifest,
) -> Result<syntax_helper_search::HbkPlatformTypeId, io::Error> {
    manifest
        .types
        .iter()
        .find(|ty| ty.primary == manifest.anchors.type_primary)
        .ok_or_else(|| io::Error::other("anchor type is absent from manifest"))
        .and_then(|ty| resolve_type_by_logical_id(snapshot, handle, &ty.logical_id, &ty.primary))
}

fn resolve_anchor_member(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    manifest: &QueryManifest,
    kind: HbkTypeMemberKind,
) -> Result<HbkTypeMemberId, io::Error> {
    let owner = if kind == HbkTypeMemberKind::Property {
        &manifest.anchors.property_owner
    } else {
        &manifest.anchors.method_owner
    };
    let name = if kind == HbkTypeMemberKind::Property {
        &manifest.anchors.property_name
    } else {
        &manifest.anchors.method_name
    };
    let kind_code = match kind {
        HbkTypeMemberKind::Property => "property",
        HbkTypeMemberKind::Method => "method",
        HbkTypeMemberKind::Event => "event",
        HbkTypeMemberKind::EnumValue => "enum_value",
    };
    let manifest_member = manifest
        .members
        .iter()
        .find(|member| {
            member.owner_logical_id == *owner && member.primary == *name && member.kind == kind_code
        })
        .ok_or_else(|| io::Error::other("anchor member is absent from manifest"))?;
    resolve_member_by_logical_id(snapshot, handle, manifest_member)
}

fn measure_first_operation(
    snapshot: &HbkFactSnapshot,
    anchor: &OperationAnchor,
    operation: Operation,
    context: Option<AvailabilityContext>,
) -> Result<MeasuredPhase<OperationSample>, io::Error> {
    let faults_before = read_process_faults()?;
    let allocations_before = experiment_allocation_snapshot();
    let started_at = Instant::now();
    let sample = black_box(run_first_operation(snapshot, anchor, operation, context)?);
    let elapsed = started_at.elapsed();
    let allocations = experiment_allocation_snapshot().delta_since(allocations_before);
    let faults = faults_before.delta_to(read_process_faults()?);
    Ok(MeasuredPhase {
        value: sample,
        elapsed,
        faults,
        allocations,
    })
}

fn run_first_operation(
    snapshot: &HbkFactSnapshot,
    anchor: &OperationAnchor,
    operation: Operation,
    context: Option<AvailabilityContext>,
) -> Result<OperationSample, io::Error> {
    let handle = snapshot.worker_handle();
    let mut sample = OperationSample::default();
    match operation {
        Operation::TypeByName => {
            let mut found_count = 0;
            let OperationAnchor::TypeByName { query_name } = anchor else {
                return Err(io::Error::other("wrong first-operation anchor"));
            };
            for index in handle.platform_types_by_name(query_name) {
                black_box(index);
                sample.checksum = hash_locator(sample.checksum, found_count);
                sample.objects += 1;
                found_count += 1;
            }
            sample.record_lookup(found_count as usize, found_count == 0);
        }
        Operation::PropertyByOwnerNameKind => {
            let OperationAnchor::PropertyLookup { owner, name } = anchor else {
                return Err(io::Error::other("wrong first-operation anchor"));
            };
            let mut found_count = 0;
            for id in
                handle.member_by_owner_name_kind(*owner, name, Some(HbkTypeMemberKind::Property))
            {
                black_box(id);
                sample.checksum = hash_locator(sample.checksum, found_count);
                sample.objects += 1;
                found_count += 1;
            }
            sample.record_lookup(found_count as usize, found_count == 0);
        }
        Operation::MethodByOwnerNameKind | Operation::CallableByOwnerName => {
            let OperationAnchor::MethodLikeLookup { owner, name } = anchor else {
                return Err(io::Error::other("wrong first-operation anchor"));
            };
            let mut found_count = 0;
            if operation == Operation::CallableByOwnerName {
                for id in handle.callable_by_owner_name(*owner, name) {
                    black_box(id);
                    sample.checksum = hash_locator(sample.checksum, found_count);
                    sample.objects += 1;
                    found_count += 1;
                }
            } else {
                for id in
                    handle.member_by_owner_name_kind(*owner, name, Some(HbkTypeMemberKind::Method))
                {
                    black_box(id);
                    sample.checksum = hash_locator(sample.checksum, found_count);
                    sample.objects += 1;
                    found_count += 1;
                }
            }
            sample.record_lookup(found_count as usize, found_count == 0);
        }
        _ => match operation {
            Operation::TypePayload => {
                let OperationAnchor::TypePayload { type_id } = anchor else {
                    return Err(io::Error::other("wrong first-operation anchor"));
                };
                sample.input_count += 1;
                consume_type_payload(&mut sample, snapshot, handle, *type_id);
                sample.objects += 1;
            }
            Operation::PropertyPayload => {
                let OperationAnchor::PropertyPayload { member } = anchor else {
                    return Err(io::Error::other("wrong first-operation anchor"));
                };
                sample.input_count += 1;
                consume_member_payload(&mut sample, snapshot, handle, *member);
                sample.objects += 1;
            }
            Operation::MethodPayload => {
                let OperationAnchor::MethodPayload { member, callables } = anchor else {
                    return Err(io::Error::other("wrong first-operation anchor"));
                };
                sample.input_count += 1;
                consume_member_payload(&mut sample, snapshot, handle, *member);
                sample.objects += 1;
                for callable in callables {
                    consume_callable_payload(&mut sample, snapshot, handle, *callable);
                    sample.objects += 1;
                }
            }
            Operation::MembersByOwnerAvailabilityBorrowed
            | Operation::MembersByOwnerAvailabilityCollect
            | Operation::FilteredMembersPayload => match anchor {
                OperationAnchor::MemberOwner {
                    owner,
                    member_kinds,
                } => {
                    let context = context.ok_or_else(|| {
                        io::Error::other("missing first-operation availability context")
                    })?;
                    sample.owner_count = 1;
                    let native_members = handle.members_of_type(*owner);
                    let mut collected = (operation == Operation::MembersByOwnerAvailabilityCollect)
                        .then(|| Vec::with_capacity(native_members.len()));
                    if let Some(collected) = &collected {
                        sample.total_capacity += collected.capacity() as u64;
                    }
                    for (index, member) in native_members.enumerate() {
                        sample.scanned_count += 1;
                        let kind = member_kinds.get(index).copied().ok_or_else(|| {
                            io::Error::other("first-operation member kind range is too short")
                        })?;
                        if record_filtered_member(
                            &mut sample,
                            snapshot,
                            handle,
                            member,
                            kind,
                            context,
                        ) {
                            sample.checksum = hash_locator(sample.checksum, index as u32);
                            if let Some(collected) = &mut collected {
                                collected.push(Av2MemberLocator(index as u32));
                            }
                        }
                    }
                    if let Some(collected) = collected {
                        sample.total_len += collected.len() as u64;
                        sample.logical_bytes += collected.len() as u64 * LOCATOR_SIZE;
                        sample.allocated_bytes += collected.capacity() as u64 * LOCATOR_SIZE;
                        black_box(collected);
                    }
                }
                OperationAnchor::FilteredMembersPayload { members } => {
                    sample.owner_count = 1;
                    sample.input_count += members.len() as u64;
                    for member in members {
                        consume_member_payload(&mut sample, snapshot, handle, *member);
                        sample.objects += 1;
                    }
                }
                _ => return Err(io::Error::other("wrong first-operation anchor")),
            },
            _ => {}
        },
    }
    Ok(sample)
}

fn measure_prepare_and_warmup(
    snapshot: &HbkFactSnapshot,
    manifest: &QueryManifest,
    operation: Operation,
    context: Option<AvailabilityContext>,
) -> Result<MeasuredPhase<WarmupValue>, io::Error> {
    let faults_before = read_process_faults()?;
    let allocations_before = experiment_allocation_snapshot();
    let started_at = Instant::now();
    let filtered_context = (operation == Operation::FilteredMembersPayload)
        .then_some(context)
        .flatten();
    let prepared = PreparedManifest::from_manifest(snapshot, manifest, filtered_context)?;
    let sample = black_box(run_operation(snapshot, &prepared, operation, context)?);
    let elapsed = started_at.elapsed();
    let allocations = experiment_allocation_snapshot().delta_since(allocations_before);
    let faults = faults_before.delta_to(read_process_faults()?);
    Ok(MeasuredPhase {
        value: WarmupValue { prepared, sample },
        elapsed,
        faults,
        allocations,
    })
}

fn measure_steady(
    snapshot: &HbkFactSnapshot,
    manifest: &PreparedManifest,
    operation: Operation,
    context: Option<AvailabilityContext>,
    iterations: usize,
) -> Result<MeasuredPhase<OperationSample>, io::Error> {
    let faults_before = read_process_faults()?;
    let allocations_before = experiment_allocation_snapshot();
    let started_at = Instant::now();
    let mut total = OperationSample::default();
    for _ in 0..iterations {
        total.merge(black_box(run_operation(
            snapshot, manifest, operation, context,
        )?));
    }
    let elapsed = started_at.elapsed();
    let allocations = experiment_allocation_snapshot().delta_since(allocations_before);
    let faults = faults_before.delta_to(read_process_faults()?);
    Ok(MeasuredPhase {
        value: total,
        elapsed,
        faults,
        allocations,
    })
}

fn run_operation(
    snapshot: &HbkFactSnapshot,
    manifest: &PreparedManifest,
    operation: Operation,
    context: Option<AvailabilityContext>,
) -> Result<OperationSample, io::Error> {
    let handle = snapshot.worker_handle();
    let mut sample = OperationSample::default();
    match operation {
        Operation::TypeByName => {
            for query in &manifest.type_queries {
                let mut found_count = 0;
                for (index, id) in handle.platform_types_by_name(&query.name).enumerate() {
                    let locator = manifest.type_locator_by_id.get(&id).ok_or_else(|| {
                        io::Error::other("type lookup returned ID outside prepared manifest")
                    })?;
                    if query.expected.get(index) != Some(locator) {
                        return Err(io::Error::other("type lookup order differs from manifest"));
                    }
                    sample.checksum = hash_locator(sample.checksum, locator.0);
                    sample.objects += 1;
                    found_count += 1;
                }
                if found_count != query.expected.len() {
                    return Err(io::Error::other(
                        "type lookup cardinality differs from manifest",
                    ));
                }
                sample.record_lookup(found_count, found_count == 0);
            }
            let mut missing_count = 0;
            for id in handle.platform_types_by_name("__hbk_s83_av2_missing_type__") {
                black_box(id);
                missing_count += 1;
            }
            sample.record_lookup(missing_count, true);
        }
        Operation::PropertyByOwnerNameKind | Operation::MethodByOwnerNameKind => {
            let queries = if operation == Operation::PropertyByOwnerNameKind {
                &manifest.property_queries
            } else {
                &manifest.method_queries
            };
            for query in queries {
                let mut found_count = 0;
                for (index, id) in handle
                    .member_by_owner_name_kind(query.owner, &query.name, Some(query.kind))
                    .enumerate()
                {
                    let locator = manifest.member_locator_by_id.get(&id).ok_or_else(|| {
                        io::Error::other("member lookup returned ID outside prepared manifest")
                    })?;
                    if query.expected.get(index) != Some(locator) {
                        return Err(io::Error::other(
                            "member lookup order differs from manifest",
                        ));
                    }
                    sample.checksum = hash_locator(sample.checksum, locator.0);
                    sample.objects += 1;
                    found_count += 1;
                }
                if found_count != query.expected.len() {
                    return Err(io::Error::other(
                        "member lookup cardinality differs from manifest",
                    ));
                }
                sample.record_lookup(found_count, found_count == 0);
            }
            if let Some(query) = queries.first() {
                let mut missing_count = 0;
                for id in handle.member_by_owner_name_kind(
                    query.owner,
                    "__hbk_s83_av2_missing_member__",
                    Some(query.kind),
                ) {
                    black_box(id);
                    missing_count += 1;
                }
                sample.record_lookup(missing_count, true);
            }
        }
        Operation::CallableByOwnerName => {
            for query in &manifest.callable_queries {
                let mut found_count = 0;
                for (index, id) in handle
                    .callable_by_owner_name(query.owner, &query.name)
                    .enumerate()
                {
                    let locator = manifest.callable_locator_by_id.get(&id).ok_or_else(|| {
                        io::Error::other("callable lookup returned ID outside prepared manifest")
                    })?;
                    if query.expected.get(index) != Some(locator) {
                        return Err(io::Error::other(
                            "callable lookup order differs from manifest",
                        ));
                    }
                    sample.checksum = hash_locator(sample.checksum, locator.0);
                    sample.objects += 1;
                    found_count += 1;
                }
                if found_count != query.expected.len() {
                    return Err(io::Error::other(
                        "callable lookup cardinality differs from manifest",
                    ));
                }
                sample.record_lookup(found_count, found_count == 0);
            }
            if let Some(query) = manifest.callable_queries.first() {
                let mut missing_count = 0;
                for id in
                    handle.callable_by_owner_name(query.owner, "__hbk_s83_av2_missing_callable__")
                {
                    black_box(id);
                    missing_count += 1;
                }
                sample.record_lookup(missing_count, true);
            }
        }
        Operation::MembersByOwnerAvailabilityBorrowed => {
            let context =
                context.ok_or_else(|| io::Error::other("missing availability context"))?;
            for owner in &manifest.owners {
                sample.owner_count += 1;
                for (index, member) in handle.members_of_type(owner.owner).enumerate() {
                    sample.scanned_count += 1;
                    let locator = owner.members.get(index).ok_or_else(|| {
                        io::Error::other("native member range longer than prepared owner range")
                    })?;
                    let kind = owner.member_kinds.get(index).copied().ok_or_else(|| {
                        io::Error::other("prepared member kind range is shorter than native range")
                    })?;
                    if record_filtered_member(&mut sample, snapshot, handle, member, kind, context)
                    {
                        sample.checksum = hash_locator(sample.checksum, locator.0);
                    }
                }
                if handle.members_of_type(owner.owner).len() != owner.members.len() {
                    return Err(io::Error::other(
                        "native member range cardinality differs from prepared owner range",
                    ));
                }
            }
        }
        Operation::MembersByOwnerAvailabilityCollect => {
            let context =
                context.ok_or_else(|| io::Error::other("missing availability context"))?;
            for owner in &manifest.owners {
                sample.owner_count += 1;
                let native_members = handle.members_of_type(owner.owner);
                let native_len = native_members.len();
                let mut collected = Vec::with_capacity(native_len);
                sample.total_capacity += native_len as u64;
                for (index, member) in native_members.enumerate() {
                    sample.scanned_count += 1;
                    let locator = owner.members.get(index).ok_or_else(|| {
                        io::Error::other("native member range longer than prepared owner range")
                    })?;
                    let kind = owner.member_kinds.get(index).copied().ok_or_else(|| {
                        io::Error::other("prepared member kind range is shorter than native range")
                    })?;
                    if record_filtered_member(&mut sample, snapshot, handle, member, kind, context)
                    {
                        sample.checksum = hash_locator(sample.checksum, locator.0);
                        collected.push(*locator);
                    }
                }
                if native_len != owner.members.len() {
                    return Err(io::Error::other(
                        "native member range cardinality differs from prepared owner range",
                    ));
                }
                sample.total_len += collected.len() as u64;
                sample.logical_bytes += collected.len() as u64 * LOCATOR_SIZE;
                sample.allocated_bytes += collected.capacity() as u64 * LOCATOR_SIZE;
                black_box(collected);
            }
        }
        Operation::TypePayload => {
            for item in &manifest.type_payloads {
                sample.input_count += 1;
                consume_type_payload(&mut sample, snapshot, handle, item.owner);
                sample.objects += 1;
            }
        }
        Operation::MethodPayload => {
            for item in &manifest.method_payloads {
                sample.input_count += 1;
                let member = manifest_member_id(manifest, item.member);
                consume_member_payload(&mut sample, snapshot, handle, member);
                for callable in &item.callables {
                    let callable = manifest_callable_id(manifest, *callable);
                    consume_callable_payload(&mut sample, snapshot, handle, callable);
                    sample.objects += 1;
                }
                sample.objects += 1;
            }
        }
        Operation::PropertyPayload => {
            for locator in &manifest.property_payloads {
                sample.input_count += 1;
                let member = manifest_member_id(manifest, *locator);
                consume_member_payload(&mut sample, snapshot, handle, member);
                sample.objects += 1;
            }
        }
        Operation::FilteredMembersPayload => {
            let context =
                context.ok_or_else(|| io::Error::other("missing availability context"))?;
            for owner in filtered_sets(manifest, context)?.iter() {
                sample.input_count += owner.members.len() as u64;
                for locator in &owner.members {
                    let member = manifest_member_id(manifest, *locator);
                    consume_member_payload(&mut sample, snapshot, handle, member);
                    sample.objects += 1;
                }
            }
        }
    }
    Ok(sample)
}

fn manifest_member_id(manifest: &PreparedManifest, locator: Av2MemberLocator) -> HbkTypeMemberId {
    manifest.member_ids[locator.0 as usize]
}

fn manifest_callable_id(manifest: &PreparedManifest, locator: Av2CallableLocator) -> HbkCallableId {
    manifest.callable_ids[locator.0 as usize]
}

fn filtered_sets(
    manifest: &PreparedManifest,
    context: AvailabilityContext,
) -> Result<&[PreparedFilteredOwner], io::Error> {
    manifest
        .filtered_members_by_context
        .iter()
        .find(|sets| sets.context == context)
        .map(|sets| sets.owners.as_slice())
        .ok_or_else(|| io::Error::other("availability context was not prepared"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
struct Av2MemberLocator(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
struct Av2TypeLocator(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
struct Av2CallableLocator(u32);

#[derive(Clone)]
struct PreparedOwner {
    logical_id: String,
    owner: syntax_helper_search::HbkPlatformTypeId,
    members: Vec<Av2MemberLocator>,
    member_kinds: Vec<HbkTypeMemberKind>,
}

struct PreparedTypeQuery {
    name: String,
    expected: Vec<Av2TypeLocator>,
}

struct PreparedMemberQuery {
    owner: syntax_helper_search::HbkPlatformTypeId,
    name: String,
    kind: HbkTypeMemberKind,
    expected: Vec<Av2MemberLocator>,
}

struct PreparedCallableQuery {
    owner: syntax_helper_search::HbkPlatformTypeId,
    name: String,
    expected: Vec<Av2CallableLocator>,
}

struct PreparedTypePayload {
    owner: syntax_helper_search::HbkPlatformTypeId,
}

struct PreparedMethodPayload {
    member: Av2MemberLocator,
    callables: Vec<Av2CallableLocator>,
}

struct PreparedContextMemberSets {
    context: AvailabilityContext,
    owners: Vec<PreparedFilteredOwner>,
}

struct PreparedFilteredOwner {
    members: Vec<Av2MemberLocator>,
}

struct AnchorResolution {
    manifest: QueryManifest,
    operation_anchor: OperationAnchor,
    checksum: u64,
}

enum OperationAnchor {
    TypeByName {
        query_name: String,
    },
    PropertyLookup {
        owner: syntax_helper_search::HbkPlatformTypeId,
        name: String,
    },
    MethodLikeLookup {
        owner: syntax_helper_search::HbkPlatformTypeId,
        name: String,
    },
    MemberOwner {
        owner: syntax_helper_search::HbkPlatformTypeId,
        member_kinds: Vec<HbkTypeMemberKind>,
    },
    TypePayload {
        type_id: syntax_helper_search::HbkPlatformTypeId,
    },
    MethodPayload {
        member: HbkTypeMemberId,
        callables: Vec<HbkCallableId>,
    },
    PropertyPayload {
        member: HbkTypeMemberId,
    },
    FilteredMembersPayload {
        members: Vec<HbkTypeMemberId>,
    },
}

impl OperationAnchor {
    fn checksum(&self, operation: Operation) -> u64 {
        let seed = fnv1a(FNV_OFFSET_BASIS, operation.code().as_bytes());
        match self {
            OperationAnchor::TypeByName { query_name } => fnv1a(seed, query_name.as_bytes()),
            OperationAnchor::PropertyLookup { name, .. }
            | OperationAnchor::MethodLikeLookup { name, .. } => fnv1a(seed, name.as_bytes()),
            OperationAnchor::MemberOwner { member_kinds, .. } => {
                fnv1a(seed, &(member_kinds.len() as u64).to_le_bytes())
            }
            OperationAnchor::TypePayload { .. } => fnv1a(seed, b"type-payload"),
            OperationAnchor::MethodPayload { callables, .. } => {
                fnv1a(seed, &(callables.len() as u64).to_le_bytes())
            }
            OperationAnchor::PropertyPayload { .. } => fnv1a(seed, b"property-payload"),
            OperationAnchor::FilteredMembersPayload { members } => {
                fnv1a(seed, &(members.len() as u64).to_le_bytes())
            }
        }
    }
}

struct WarmupValue {
    prepared: PreparedManifest,
    #[allow(dead_code)]
    sample: OperationSample,
}

struct MeasuredPhase<T> {
    value: T,
    elapsed: Duration,
    faults: ProcessFaults,
    allocations: HbkSnapshotExperimentAllocationDelta,
}

#[derive(Debug, Clone, Default)]
struct OperationSample {
    counts: OperationCountsReport,
    checksum: u64,
    input_count: u64,
    objects: u64,
    query_count: u64,
    candidate_count: u64,
    miss_count: u64,
    owner_count: u64,
    scanned_count: u64,
    returned_count: u64,
    universal_count: u64,
    explicit_count: u64,
    excluded_count: u64,
    property_count: u64,
    method_count: u64,
    event_count: u64,
    enum_value_count: u64,
    total_len: u64,
    total_capacity: u64,
    logical_bytes: u64,
    allocated_bytes: u64,
    string_bytes: u64,
    canonical_payload_bytes: u64,
}

impl OperationSample {
    fn record_lookup(&mut self, candidate_count: usize, miss: bool) {
        self.query_count += 1;
        self.candidate_count += candidate_count as u64;
        self.miss_count += u64::from(miss);
        self.counts.query_count = self.query_count;
        self.counts.candidate_count = self.candidate_count;
    }

    fn merge(&mut self, other: Self) {
        self.checksum = fnv1a(self.checksum, &other.checksum.to_le_bytes());
        self.input_count += other.input_count;
        self.objects += other.objects;
        self.query_count += other.query_count;
        self.candidate_count += other.candidate_count;
        self.miss_count += other.miss_count;
        self.owner_count += other.owner_count;
        self.scanned_count += other.scanned_count;
        self.returned_count += other.returned_count;
        self.universal_count += other.universal_count;
        self.explicit_count += other.explicit_count;
        self.excluded_count += other.excluded_count;
        self.property_count += other.property_count;
        self.method_count += other.method_count;
        self.event_count += other.event_count;
        self.enum_value_count += other.enum_value_count;
        self.total_len += other.total_len;
        self.total_capacity += other.total_capacity;
        self.logical_bytes += other.logical_bytes;
        self.allocated_bytes += other.allocated_bytes;
        self.string_bytes += other.string_bytes;
        self.canonical_payload_bytes += other.canonical_payload_bytes;
        self.counts = OperationCountsReport {
            query_count: self.query_count,
            candidate_count: self.candidate_count,
            object_count: self.objects,
            checksum_count: self.objects,
            property_count: self.property_count,
            method_count: self.method_count,
            event_count: self.event_count,
            enum_value_count: self.enum_value_count,
        };
    }
}

fn record_filtered_member(
    sample: &mut OperationSample,
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    member_id: HbkTypeMemberId,
    kind: HbkTypeMemberKind,
    context: AvailabilityContext,
) -> bool {
    let availability = availability_match(
        snapshot,
        handle.availability_contexts(HbkFactRef::TypeMember(member_id)),
        context,
    );
    match availability {
        AvailabilityMatch::Universal => sample.universal_count += 1,
        AvailabilityMatch::Explicit => sample.explicit_count += 1,
        AvailabilityMatch::Excluded => {
            sample.excluded_count += 1;
            return false;
        }
    }
    sample.returned_count += 1;
    count_member_kind(sample, kind);
    sample.objects += 1;
    true
}

fn count_member_kind(sample: &mut OperationSample, kind: HbkTypeMemberKind) {
    match kind {
        HbkTypeMemberKind::Property => sample.property_count += 1,
        HbkTypeMemberKind::Method => sample.method_count += 1,
        HbkTypeMemberKind::Event => sample.event_count += 1,
        HbkTypeMemberKind::EnumValue => sample.enum_value_count += 1,
    }
}

fn operation_data(operation: Operation, sample: OperationSample) -> OperationData {
    match operation {
        Operation::TypeByName
        | Operation::PropertyByOwnerNameKind
        | Operation::MethodByOwnerNameKind
        | Operation::CallableByOwnerName => OperationData::Lookup {
            query_count: sample.query_count,
            candidate_count: sample.candidate_count,
            miss_count: sample.miss_count,
        },
        Operation::MembersByOwnerAvailabilityBorrowed => OperationData::Iteration {
            owner_count: sample.owner_count,
            scanned_count: sample.scanned_count,
            returned_count: sample.returned_count,
            universal_count: sample.universal_count,
            explicit_count: sample.explicit_count,
            excluded_count: sample.excluded_count,
            property_count: sample.property_count,
            method_count: sample.method_count,
            event_count: sample.event_count,
            enum_value_count: sample.enum_value_count,
        },
        Operation::MembersByOwnerAvailabilityCollect => OperationData::CompactMaterialization {
            owner_count: sample.owner_count,
            scanned_count: sample.scanned_count,
            returned_count: sample.returned_count,
            universal_count: sample.universal_count,
            explicit_count: sample.explicit_count,
            excluded_count: sample.excluded_count,
            property_count: sample.property_count,
            method_count: sample.method_count,
            event_count: sample.event_count,
            enum_value_count: sample.enum_value_count,
            locator_size: LOCATOR_SIZE,
            total_len: sample.total_len,
            total_capacity: sample.total_capacity,
            logical_bytes: sample.logical_bytes,
            allocated_bytes: sample.allocated_bytes,
        },
        _ => OperationData::Payload {
            input_count: sample.input_count,
            object_count: sample.objects,
            string_bytes_touched: sample.string_bytes,
            canonical_payload_bytes_touched: sample.canonical_payload_bytes,
        },
    }
}

fn measure_memory_sample(
    snapshot: &HbkFactSnapshot,
    manifest: &PreparedManifest,
    operation: Operation,
    context: Option<AvailabilityContext>,
) -> Result<MemoryReport, io::Error> {
    let faults_before = read_process_faults()?;
    let allocations_before = experiment_allocation_snapshot();
    let started_at = Instant::now();
    let before = experiment_allocation_snapshot();
    let before_kib = read_process_memory()?;
    let mut held_sets = Vec::<Vec<Av2MemberLocator>>::new();
    let mut len_bytes = 0;
    let mut capacity_bytes = 0;
    if operation == Operation::MembersByOwnerAvailabilityCollect {
        let context = context.ok_or_else(|| io::Error::other("missing availability context"))?;
        let handle = snapshot.worker_handle();
        held_sets.reserve(manifest.owners.len());
        for owner in &manifest.owners {
            let native_members = handle.members_of_type(owner.owner);
            let mut set = Vec::with_capacity(native_members.len());
            for (index, member) in native_members.enumerate() {
                let locator = owner.members.get(index).ok_or_else(|| {
                    io::Error::other("native member range longer than prepared owner range")
                })?;
                if availability_match(
                    snapshot,
                    handle.availability_contexts(HbkFactRef::TypeMember(member)),
                    context,
                )
                .included()
                {
                    set.push(*locator);
                }
            }
            len_bytes += set.len() as u64 * LOCATOR_SIZE;
            capacity_bytes += set.capacity() as u64 * LOCATOR_SIZE;
            held_sets.push(set);
        }
    }
    let live = experiment_allocation_snapshot();
    let live_kib = read_process_memory()?;
    black_box(&held_sets);
    drop(held_sets);
    let after_drop = experiment_allocation_snapshot();
    let after_drop_kib = read_process_memory()?;
    let elapsed = started_at.elapsed();
    let allocation_delta = experiment_allocation_snapshot().delta_since(allocations_before);
    let live_delta = live.delta_since(before);
    let post_drop_delta = after_drop.delta_since(before);
    Ok(MemoryReport {
        elapsed,
        faults: faults_before.delta_to(read_process_faults()?),
        allocations: allocation_delta,
        before_kib,
        live_kib,
        after_drop_kib,
        container_overhead_bytes: if operation == Operation::MembersByOwnerAvailabilityCollect {
            (manifest.owners.len() * std::mem::size_of::<Vec<Av2MemberLocator>>()) as u64
        } else {
            0
        },
        logical_bytes: len_bytes,
        capacity_bytes,
        live_delta_bytes: live_delta
            .live_bytes_after
            .saturating_sub(live_delta.live_bytes_before),
        peak_live_delta_bytes: live_delta.peak_live_bytes_growth,
        post_drop_delta_bytes: post_drop_delta
            .live_bytes_after
            .saturating_sub(post_drop_delta.live_bytes_before),
    })
}

fn build_parity_transcript(
    context: AvailabilityContext,
    snapshot: &HbkFactSnapshot,
    query_manifest: &QueryManifest,
    manifest: &PreparedManifest,
    _operation: Operation,
) -> Result<Vec<ParityRecord>, io::Error> {
    let handle = snapshot.worker_handle();
    let mut records = Vec::new();
    records.extend(build_lookup_parity_records(
        snapshot,
        handle,
        query_manifest,
    )?);
    for item in &manifest.type_payloads {
        records.push(ParityRecord::Type(ParityType {
            payload: transcript_type(snapshot, handle, item.owner),
        }));
    }
    for item in &manifest.method_payloads {
        let member = manifest_member_id(manifest, item.member);
        records.push(ParityRecord::Member(ParityMember {
            owner_logical_id: snapshot
                .string(
                    snapshot
                        .platform_type(snapshot.type_member(member).owner)
                        .id,
                )
                .to_owned(),
            payload: transcript_member(snapshot, handle, member),
        }));
        for callable in &item.callables {
            records.push(ParityRecord::Callable(ParityCallable {
                payload: transcript_callable(
                    snapshot,
                    handle,
                    manifest_callable_id(manifest, *callable),
                ),
            }));
        }
    }
    for locator in &manifest.property_payloads {
        let member = manifest_member_id(manifest, *locator);
        records.push(ParityRecord::Member(ParityMember {
            owner_logical_id: snapshot
                .string(
                    snapshot
                        .platform_type(snapshot.type_member(member).owner)
                        .id,
                )
                .to_owned(),
            payload: transcript_member(snapshot, handle, member),
        }));
    }
    for owner in &manifest.owners {
        let mut borrowed_members = Vec::new();
        for (index, member_id) in handle.members_of_type(owner.owner).enumerate() {
            let locator = *owner.members.get(index).ok_or_else(|| {
                io::Error::other("native member range longer than prepared owner range")
            })?;
            if manifest_member_id(manifest, locator) != member_id {
                return Err(io::Error::other(
                    "native member range order differs from prepared owner range",
                ));
            }
            let availability = availability_match(
                snapshot,
                handle.availability_contexts(HbkFactRef::TypeMember(member_id)),
                context,
            );
            if availability.included() {
                let logical_id = snapshot
                    .string(snapshot.type_member(member_id).id)
                    .to_owned();
                borrowed_members.push(logical_id);
                records.push(ParityRecord::FilteredMember(ParityFilteredMember {
                    owner_logical_id: owner.logical_id.clone(),
                    availability_rule: availability.code(),
                    payload: transcript_member(snapshot, handle, member_id),
                }));
            }
        }
        let compact_members = owner
            .members
            .iter()
            .copied()
            .filter_map(|locator| {
                let member_id = manifest_member_id(manifest, locator);
                availability_match(
                    snapshot,
                    handle.availability_contexts(HbkFactRef::TypeMember(member_id)),
                    context,
                )
                .included()
                .then(|| {
                    snapshot
                        .string(snapshot.type_member(member_id).id)
                        .to_owned()
                })
            })
            .collect();
        if borrowed_members != compact_members {
            return Err(io::Error::other(
                "borrowed iteration and compact materialization differ",
            ));
        }
        records.push(ParityRecord::MemberSet(ParityMemberSet {
            owner_logical_id: owner.logical_id.clone(),
            borrowed_members,
            compact_members,
        }));
    }
    Ok(records)
}

fn build_parity(
    loaded: &LoadedSnapshot,
    manifest_path: &Path,
    context: AvailabilityContext,
    snapshot: &HbkFactSnapshot,
    query_manifest: &QueryManifest,
    manifest: &PreparedManifest,
) -> Result<ParityReport, io::Error> {
    Ok(ParityReport {
        schema_version: PARITY_SCHEMA_VERSION,
        workload_version: WORKLOAD_VERSION,
        mode: "parity",
        backend: loaded.backend,
        decision_role: loaded.decision_role,
        availability_context: context.code,
        module_context_filter_used: false,
        empty_availability_rule: "universal",
        input_identity: input_identity(&loaded.index_path)?,
        manifest: manifest_identity(manifest_path)?,
        runtime_artifacts: runtime_artifacts(loaded)?,
        transcript: build_parity_transcript(
            context,
            snapshot,
            query_manifest,
            manifest,
            Operation::FilteredMembersPayload,
        )?,
    })
}

#[derive(Serialize)]
#[serde(tag = "record")]
enum ParityRecord {
    #[serde(rename = "lookup")]
    Lookup(ParityLookup),
    #[serde(rename = "type_payload")]
    Type(ParityType),
    #[serde(rename = "callable_payload")]
    Callable(ParityCallable),
    #[serde(rename = "filtered_member_payload")]
    FilteredMember(ParityFilteredMember),
    #[serde(rename = "member_payload")]
    Member(ParityMember),
    #[serde(rename = "member_set")]
    MemberSet(ParityMemberSet),
}

impl From<ParityMemberSet> for ParityRecord {
    fn from(value: ParityMemberSet) -> Self {
        Self::MemberSet(value)
    }
}

fn build_lookup_parity_records(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    manifest: &QueryManifest,
) -> Result<Vec<ParityRecord>, io::Error> {
    let mut records = Vec::new();
    for query in &manifest.lookup_queries.type_names {
        records.push(ParityRecord::Lookup(ParityLookup {
            operation: Operation::TypeByName.code(),
            owner_logical_id: None,
            member_kind: None,
            query_name: query.query_name.clone(),
            query_role: query.query_role.clone(),
            declared_logical_id: Some(query.logical_id.clone()),
            results: handle
                .platform_types_by_name(&query.query_name)
                .map(|id| snapshot.string(snapshot.platform_type(id).id).to_owned())
                .collect(),
        }));
    }
    records.push(ParityRecord::Lookup(ParityLookup {
        operation: Operation::TypeByName.code(),
        owner_logical_id: None,
        member_kind: None,
        query_name: manifest.fixed_misses.type_name.clone(),
        query_role: "miss".to_owned(),
        declared_logical_id: None,
        results: handle
            .platform_types_by_name(&manifest.fixed_misses.type_name)
            .map(|id| snapshot.string(snapshot.platform_type(id).id).to_owned())
            .collect(),
    }));

    for query in &manifest.lookup_queries.properties {
        records.push(member_lookup_parity_record(
            snapshot,
            handle,
            Operation::PropertyByOwnerNameKind,
            query,
            HbkTypeMemberKind::Property,
        )?);
    }
    if let Some(query) = manifest.lookup_queries.properties.first() {
        records.push(missing_member_lookup_parity_record(
            snapshot,
            handle,
            Operation::PropertyByOwnerNameKind,
            query,
            HbkTypeMemberKind::Property,
            &manifest.fixed_misses.member_name,
        )?);
    }

    for query in &manifest.lookup_queries.methods {
        records.push(member_lookup_parity_record(
            snapshot,
            handle,
            Operation::MethodByOwnerNameKind,
            query,
            HbkTypeMemberKind::Method,
        )?);
        records.push(callable_lookup_parity_record(snapshot, handle, query)?);
    }
    if let Some(query) = manifest.lookup_queries.methods.first() {
        records.push(missing_member_lookup_parity_record(
            snapshot,
            handle,
            Operation::MethodByOwnerNameKind,
            query,
            HbkTypeMemberKind::Method,
            &manifest.fixed_misses.member_name,
        )?);
        records.push(missing_callable_lookup_parity_record(
            snapshot,
            handle,
            query,
            &manifest.fixed_misses.callable_name,
        )?);
    }
    Ok(records)
}

fn member_lookup_parity_record(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    operation: Operation,
    query: &ManifestMemberQuery,
    kind: HbkTypeMemberKind,
) -> Result<ParityRecord, io::Error> {
    let owner = resolve_type_by_logical_id(snapshot, handle, &query.owner_logical_id, "")?;
    Ok(ParityRecord::Lookup(ParityLookup {
        operation: operation.code(),
        owner_logical_id: Some(query.owner_logical_id.clone()),
        member_kind: Some(member_kind_code(kind)),
        query_name: query.query_name.clone(),
        query_role: query.query_role.clone(),
        declared_logical_id: Some(query.logical_id.clone()),
        results: handle
            .member_by_owner_name_kind(owner, &query.query_name, Some(kind))
            .map(|id| snapshot.string(snapshot.type_member(id).id).to_owned())
            .collect(),
    }))
}

fn missing_member_lookup_parity_record(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    operation: Operation,
    query: &ManifestMemberQuery,
    kind: HbkTypeMemberKind,
    missing_name: &str,
) -> Result<ParityRecord, io::Error> {
    let owner = resolve_type_by_logical_id(snapshot, handle, &query.owner_logical_id, "")?;
    Ok(ParityRecord::Lookup(ParityLookup {
        operation: operation.code(),
        owner_logical_id: Some(query.owner_logical_id.clone()),
        member_kind: Some(member_kind_code(kind)),
        query_name: missing_name.to_owned(),
        query_role: "miss".to_owned(),
        declared_logical_id: None,
        results: handle
            .member_by_owner_name_kind(owner, missing_name, Some(kind))
            .map(|id| snapshot.string(snapshot.type_member(id).id).to_owned())
            .collect(),
    }))
}

fn callable_lookup_parity_record(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    query: &ManifestMemberQuery,
) -> Result<ParityRecord, io::Error> {
    let owner = resolve_type_by_logical_id(snapshot, handle, &query.owner_logical_id, "")?;
    Ok(ParityRecord::Lookup(ParityLookup {
        operation: Operation::CallableByOwnerName.code(),
        owner_logical_id: Some(query.owner_logical_id.clone()),
        member_kind: Some("callable"),
        query_name: query.query_name.clone(),
        query_role: query.query_role.clone(),
        declared_logical_id: None,
        results: handle
            .callable_by_owner_name(owner, &query.query_name)
            .map(|id| snapshot.string(snapshot.callable(id).id).to_owned())
            .collect(),
    }))
}

fn missing_callable_lookup_parity_record(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    query: &ManifestMemberQuery,
    missing_name: &str,
) -> Result<ParityRecord, io::Error> {
    let owner = resolve_type_by_logical_id(snapshot, handle, &query.owner_logical_id, "")?;
    Ok(ParityRecord::Lookup(ParityLookup {
        operation: Operation::CallableByOwnerName.code(),
        owner_logical_id: Some(query.owner_logical_id.clone()),
        member_kind: Some("callable"),
        query_name: missing_name.to_owned(),
        query_role: "miss".to_owned(),
        declared_logical_id: None,
        results: handle
            .callable_by_owner_name(owner, missing_name)
            .map(|id| snapshot.string(snapshot.callable(id).id).to_owned())
            .collect(),
    }))
}

fn hash_locator(hash: u64, locator: u32) -> u64 {
    fnv1a(fnv1a(hash, b"locator:u32"), &locator.to_le_bytes())
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

fn member_kind_code(kind: HbkTypeMemberKind) -> &'static str {
    match kind {
        HbkTypeMemberKind::Property => "property",
        HbkTypeMemberKind::Method => "method",
        HbkTypeMemberKind::Event => "event",
        HbkTypeMemberKind::EnumValue => "enum_value",
    }
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

impl AvailabilityMatch {
    fn included(self) -> bool {
        matches!(self, Self::Universal | Self::Explicit)
    }

    fn code(self) -> &'static str {
        match self {
            Self::Universal => "universal",
            Self::Explicit => "explicit",
            Self::Excluded => "excluded",
        }
    }
}

fn transcript_member(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    id: HbkTypeMemberId,
) -> TranscriptMember {
    let member = snapshot.type_member(id);
    TranscriptMember {
        logical_id: snapshot.string(member.id).to_owned(),
        owner_logical_id: snapshot
            .string(snapshot.platform_type(member.owner).id)
            .to_owned(),
        kind: member_kind_code(member.kind),
        name: transcript_name(snapshot, &member.name),
        type_refs: transcript_type_refs(snapshot, &member.type_refs),
        availability_contexts: string_ids(
            snapshot,
            handle.availability_contexts(HbkFactRef::TypeMember(id)),
        ),
        available_since: handle
            .available_since(HbkFactRef::TypeMember(id))
            .map(|id| snapshot.string(id).to_owned()),
    }
}

fn consume_type_payload(
    sample: &mut OperationSample,
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    id: syntax_helper_search::HbkPlatformTypeId,
) {
    let ty = snapshot.platform_type(id);
    feed_string_id(sample, snapshot, ty.id);
    feed_name(sample, snapshot, &ty.name);
    match &ty.metadata_template {
        Some(template) => {
            feed_bytes(sample, b"metadata_template", false);
            feed_string_id(sample, snapshot, template.metadata_kind);
            feed_string_ids(sample, snapshot, &template.template_parameters);
        }
        None => feed_bytes(sample, b"no_metadata_template", false),
    }
    feed_optional_template_key(sample, snapshot, ty.type_template_key);
    feed_string_ids(
        sample,
        snapshot,
        handle.availability_contexts(HbkFactRef::PlatformType(id)),
    );
    feed_optional_string_id(
        sample,
        snapshot,
        handle.available_since(HbkFactRef::PlatformType(id)),
    );
}

fn consume_member_payload(
    sample: &mut OperationSample,
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    id: HbkTypeMemberId,
) {
    let member = snapshot.type_member(id);
    feed_string_id(sample, snapshot, member.id);
    feed_string_id(sample, snapshot, snapshot.platform_type(member.owner).id);
    feed_bytes(sample, member_kind_code(member.kind).as_bytes(), false);
    feed_name(sample, snapshot, &member.name);
    feed_type_refs(sample, snapshot, &member.type_refs);
    feed_string_ids(
        sample,
        snapshot,
        handle.availability_contexts(HbkFactRef::TypeMember(id)),
    );
    feed_optional_string_id(
        sample,
        snapshot,
        handle.available_since(HbkFactRef::TypeMember(id)),
    );
}

fn consume_callable_payload(
    sample: &mut OperationSample,
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    id: HbkCallableId,
) {
    let callable = snapshot.callable(id);
    feed_string_id(sample, snapshot, callable.id);
    match callable.owner {
        Some(owner) => {
            feed_bytes(sample, b"some", false);
            feed_string_id(sample, snapshot, snapshot.platform_type(owner).id);
        }
        None => feed_bytes(sample, b"none", false),
    }
    feed_bytes(sample, callable_kind_code(callable.kind).as_bytes(), false);
    feed_name(sample, snapshot, &callable.name);
    feed_string_ids(
        sample,
        snapshot,
        handle.availability_contexts(HbkFactRef::Callable(id)),
    );
    feed_optional_string_id(
        sample,
        snapshot,
        handle.available_since(HbkFactRef::Callable(id)),
    );
    feed_bytes(
        sample,
        &(callable.signatures.len() as u64).to_le_bytes(),
        false,
    );
    for signature in &callable.signatures {
        feed_string_id(sample, snapshot, signature.text);
        feed_bytes(
            sample,
            &(signature.parameters.len() as u64).to_le_bytes(),
            false,
        );
        for parameter in &signature.parameters {
            feed_string_id(sample, snapshot, parameter.name);
            feed_bytes(sample, &[u8::from(parameter.required)], false);
            feed_type_refs(sample, snapshot, &parameter.type_refs);
        }
        feed_type_refs(sample, snapshot, &signature.return_type_refs);
    }
    feed_type_refs(sample, snapshot, &callable.return_type_refs);
}

fn feed_name(sample: &mut OperationSample, snapshot: &HbkFactSnapshot, name: &HbkName) {
    feed_string_id(sample, snapshot, name.primary);
    feed_optional_string_id(sample, snapshot, name.alias);
}

fn feed_type_refs(sample: &mut OperationSample, snapshot: &HbkFactSnapshot, refs: &[HbkTypeRef]) {
    feed_bytes(sample, &(refs.len() as u64).to_le_bytes(), false);
    for type_ref in refs {
        feed_string_id(sample, snapshot, type_ref.name);
        match &type_ref.target {
            HbkTypeRefTarget::Ok(id) => {
                feed_bytes(sample, b"ok", false);
                feed_string_id(sample, snapshot, *id);
            }
            HbkTypeRefTarget::Unresolved => feed_bytes(sample, b"unresolved", false),
            HbkTypeRefTarget::Ambiguous(ids) => {
                feed_bytes(sample, b"ambiguous", false);
                feed_string_ids(sample, snapshot, ids);
            }
        }
        match type_ref.type_template_key {
            Some(key) => feed_template_key(sample, snapshot, key),
            None => feed_bytes(sample, b"no_template_key", false),
        }
        match &type_ref.template_binding {
            Some(binding) => {
                feed_bytes(sample, b"template_binding", false);
                feed_template_binding(sample, binding);
            }
            None => feed_bytes(sample, b"no_template_binding", false),
        }
    }
}

fn feed_optional_template_key(
    sample: &mut OperationSample,
    snapshot: &HbkFactSnapshot,
    key: Option<syntax_helper_search::HbkPlatformTypeTemplateKey>,
) {
    match key {
        Some(key) => feed_template_key(sample, snapshot, key),
        None => feed_bytes(sample, b"no_type_template_key", false),
    }
}

fn feed_template_key(
    sample: &mut OperationSample,
    snapshot: &HbkFactSnapshot,
    key: syntax_helper_search::HbkPlatformTypeTemplateKey,
) {
    feed_bytes(sample, b"template_key", false);
    feed_string_id(sample, snapshot, key.family);
    feed_string_id(sample, snapshot, key.variant);
}

fn feed_template_binding(sample: &mut OperationSample, binding: &HbkTypeTemplateBinding) {
    feed_bytes(
        sample,
        &(binding.arguments.len() as u64).to_le_bytes(),
        false,
    );
    for argument in &binding.arguments {
        match argument {
            model::TemplateParameterBinding::OwnerParameter {
                owner_parameter_index,
                target_parameter_index,
            } => {
                feed_bytes(sample, b"owner_parameter", false);
                feed_bytes(
                    sample,
                    &(*owner_parameter_index as u64).to_le_bytes(),
                    false,
                );
                feed_bytes(
                    sample,
                    &(*target_parameter_index as u64).to_le_bytes(),
                    false,
                );
            }
        }
    }
}

fn feed_string_ids<I>(sample: &mut OperationSample, snapshot: &HbkFactSnapshot, ids: I)
where
    I: IntoIterator,
    I::IntoIter: ExactSizeIterator,
    I::Item: std::borrow::Borrow<StringId>,
{
    let ids = ids.into_iter();
    feed_bytes(sample, &(ids.len() as u64).to_le_bytes(), false);
    for id in ids {
        feed_string_id(sample, snapshot, *id.borrow());
    }
}

fn feed_optional_string_id(
    sample: &mut OperationSample,
    snapshot: &HbkFactSnapshot,
    id: Option<StringId>,
) {
    match id {
        Some(id) => {
            feed_bytes(sample, &[1], false);
            feed_string_id(sample, snapshot, id);
        }
        None => feed_bytes(sample, &[0], false),
    }
}

fn feed_string_id(sample: &mut OperationSample, snapshot: &HbkFactSnapshot, id: StringId) {
    feed_bytes(sample, snapshot.string(id).as_bytes(), true);
}

fn feed_bytes(sample: &mut OperationSample, bytes: &[u8], is_string: bool) {
    sample.checksum = fnv1a(sample.checksum, bytes);
    sample.canonical_payload_bytes += bytes.len() as u64;
    if is_string {
        sample.string_bytes += bytes.len() as u64;
    }
}

fn transcript_type(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    id: syntax_helper_search::HbkPlatformTypeId,
) -> TranscriptType {
    let ty = snapshot.platform_type(id);
    TranscriptType {
        logical_id: snapshot.string(ty.id).to_owned(),
        name: transcript_name(snapshot, &ty.name),
        metadata_template: ty.metadata_template.as_ref().map(|template| {
            TranscriptMetadataTemplate {
                metadata_kind: snapshot.string(template.metadata_kind).to_owned(),
                template_parameters: string_ids(snapshot, &template.template_parameters),
            }
        }),
        type_template_key: ty.type_template_key.map(|key| TranscriptTemplateKey {
            family: snapshot.string(key.family).to_owned(),
            variant: snapshot.string(key.variant).to_owned(),
        }),
        availability_contexts: string_ids(
            snapshot,
            handle.availability_contexts(HbkFactRef::PlatformType(id)),
        ),
        available_since: handle
            .available_since(HbkFactRef::PlatformType(id))
            .map(|id| snapshot.string(id).to_owned()),
    }
}

fn transcript_callable(
    snapshot: &HbkFactSnapshot,
    handle: HbkFactReadHandle<'_>,
    id: HbkCallableId,
) -> TranscriptCallable {
    let callable = snapshot.callable(id);
    TranscriptCallable {
        logical_id: snapshot.string(callable.id).to_owned(),
        owner_logical_id: callable
            .owner
            .map(|owner| snapshot.string(snapshot.platform_type(owner).id).to_owned()),
        kind: callable_kind_code(callable.kind),
        name: transcript_name(snapshot, &callable.name),
        availability_contexts: string_ids(
            snapshot,
            handle.availability_contexts(HbkFactRef::Callable(id)),
        ),
        available_since: handle
            .available_since(HbkFactRef::Callable(id))
            .map(|id| snapshot.string(id).to_owned()),
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
        return_type_refs: transcript_type_refs(snapshot, &callable.return_type_refs),
    }
}

fn transcript_name(snapshot: &HbkFactSnapshot, name: &HbkName) -> TranscriptName {
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
                    values: string_ids(snapshot, ids),
                },
            },
            template_key: type_ref.type_template_key.map(|key| TranscriptTemplateKey {
                family: snapshot.string(key.family).to_owned(),
                variant: snapshot.string(key.variant).to_owned(),
            }),
            template_binding: type_ref
                .template_binding
                .as_ref()
                .map(transcript_template_binding),
        })
        .collect()
}

fn transcript_template_binding(binding: &HbkTypeTemplateBinding) -> TranscriptTemplateBinding {
    TranscriptTemplateBinding {
        arguments: binding
            .arguments
            .iter()
            .map(|argument| match argument {
                model::TemplateParameterBinding::OwnerParameter {
                    owner_parameter_index,
                    target_parameter_index,
                } => TranscriptTemplateArgument {
                    kind: "owner_parameter",
                    owner_parameter_index: *owner_parameter_index,
                    target_parameter_index: *target_parameter_index,
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

fn input_identity(provider_path: &Path) -> Result<InputIdentityReport, io::Error> {
    let mut identity = static_input_identity();
    identity.provider = expected_artifact_report(provider_path, PROVIDER_BYTES, PROVIDER_SHA256)?;
    identity.hbk = expected_artifact_report(
        Path::new(SOURCE_HBK_PATH),
        SOURCE_HBK_BYTES,
        SOURCE_HBK_SHA256,
    )?;
    Ok(identity)
}

fn static_input_identity() -> InputIdentityReport {
    InputIdentityReport {
        dataset: DATASET.to_owned(),
        platform_version: PLATFORM_VERSION.to_owned(),
        source_locale: SOURCE_LOCALE.to_owned(),
        provider_schema_version: PROVIDER_SCHEMA_VERSION,
        extraction_schema_version: EXTRACTION_SCHEMA_VERSION,
        hbk: ArtifactReport {
            path: SOURCE_HBK_PATH.to_owned(),
            bytes: SOURCE_HBK_BYTES,
            sha256: SOURCE_HBK_SHA256.to_owned(),
        },
        provider: ArtifactReport {
            path: String::new(),
            bytes: PROVIDER_BYTES,
            sha256: PROVIDER_SHA256.to_owned(),
        },
    }
}

fn manifest_identity(path: &Path) -> Result<ManifestIdentityReport, io::Error> {
    Ok(ManifestIdentityReport {
        schema_version: MANIFEST_SCHEMA_VERSION,
        sha256: sha256sum(path)?,
        bytes: fs::metadata(path)?.len(),
    })
}

fn runtime_artifacts(loaded: &LoadedSnapshot) -> Result<Vec<ArtifactReport>, io::Error> {
    let mut artifacts = vec![artifact_report(&loaded.index_path)?];
    if let Some(cache_path) = &loaded.cache_path {
        artifacts.push(artifact_report(cache_path)?);
    }
    Ok(artifacts)
}

fn expected_artifact_report(
    path: &Path,
    expected_bytes: u64,
    expected_sha256: &str,
) -> Result<ArtifactReport, io::Error> {
    let report = artifact_report(path)?;
    if report.bytes != expected_bytes || report.sha256 != expected_sha256 {
        return Err(io::Error::other(format!(
            "S83-AV2 input identity mismatch for {}: expected {} bytes / {}, observed {} bytes / {}",
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

fn ns_per_object(elapsed_ns: u64, objects: u64) -> Option<u64> {
    if objects == 0 {
        None
    } else {
        Some(elapsed_ns / objects)
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

fn read_process_memory() -> Result<ProcessMemoryReport, io::Error> {
    parse_smaps_rollup(&fs::read_to_string("/proc/self/smaps_rollup")?)
}

fn parse_smaps_rollup(input: &str) -> Result<ProcessMemoryReport, io::Error> {
    let mut report = ProcessMemoryReport::default();
    let mut private_clean = 0;
    let mut private_dirty = 0;
    for line in input.lines() {
        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        if !matches!(
            key,
            "Rss:" | "Pss:" | "Private_Clean:" | "Private_Dirty:" | "Anonymous:"
        ) {
            continue;
        }
        let Some(value) = parts.next() else {
            continue;
        };
        let value = value.parse::<u64>().map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid smaps_rollup value {value:?}: {error}"),
            )
        })?;
        match key {
            "Rss:" => report.rss_kib = value,
            "Pss:" => report.pss_kib = value,
            "Private_Clean:" => private_clean = value,
            "Private_Dirty:" => private_dirty = value,
            "Anonymous:" => report.anonymous_kib = value,
            _ => {}
        }
    }
    report.private_kib = private_clean + private_dirty;
    report.file_backed_kib = report.rss_kib.saturating_sub(report.anonymous_kib);
    Ok(report)
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

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct QueryManifest {
    schema_version: String,
    workload_version: String,
    input_identity: InputIdentityReport,
    availability_contexts: Vec<String>,
    member_kinds: Vec<String>,
    empty_availability_rule: String,
    module_context_filter_used: bool,
    types: Vec<ManifestType>,
    members: Vec<ManifestMember>,
    lookup_queries: LookupManifest,
    fixed_misses: ManifestMisses,
    anchors: ManifestAnchors,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestType {
    logical_id: String,
    primary: String,
    alias: Option<String>,
    member_count: u64,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestMember {
    logical_id: String,
    owner_logical_id: String,
    kind: String,
    primary: String,
    alias: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestTypeNameQuery {
    logical_id: String,
    query_name: String,
    query_role: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestMemberQuery {
    logical_id: String,
    owner_logical_id: String,
    kind: String,
    query_name: String,
    query_role: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LookupManifest {
    type_names: Vec<ManifestTypeNameQuery>,
    properties: Vec<ManifestMemberQuery>,
    methods: Vec<ManifestMemberQuery>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestMisses {
    type_name: String,
    member_name: String,
    callable_name: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ManifestAnchors {
    type_primary: String,
    property_owner: String,
    property_name: String,
    method_owner: String,
    method_name: String,
    enumeration_owner: String,
}

#[derive(Serialize)]
struct BenchmarkReport {
    schema_version: &'static str,
    workload_version: &'static str,
    mode: &'static str,
    backend: &'static str,
    decision_role: &'static str,
    operation: &'static str,
    availability_context: Option<&'static str>,
    iterations: usize,
    module_context_filter_used: bool,
    empty_availability_rule: &'static str,
    input_identity: InputIdentityReport,
    manifest: ManifestIdentityReport,
    runtime_artifacts: Vec<ArtifactReport>,
    projection: ProjectionReport,
    phase_order: &'static [&'static str],
    timings: TimingReport,
    faults: PhaseFaultsReport,
    allocations: AllocationEvidenceReport,
    memory: MemoryReport,
    counts: OperationCountsReport,
    checksum: ChecksumReport,
    operation_data: OperationData,
}

#[derive(Serialize)]
struct ManifestIdentityReport {
    schema_version: &'static str,
    sha256: String,
    bytes: u64,
}

#[derive(Serialize)]
struct ProjectionReport {
    source: &'static str,
    compact: Option<&'static str>,
}

#[derive(Serialize)]
struct TimingReport {
    entry_to_ready: TimingPhaseReport,
    anchor_resolution: TimingPhaseReport,
    first_operation: TimingPhaseReport,
    warmup: TimingPhaseReport,
    steady_workload: TimingPhaseReport,
    memory_sample: TimingPhaseReport,
}

#[derive(Serialize)]
struct TimingPhaseReport {
    elapsed_ns: u64,
    average_ns: Option<u64>,
    ns_per_query: Option<u64>,
    ns_per_object: Option<u64>,
    count: u64,
    checksum: u64,
}

impl TimingPhaseReport {
    fn phase(duration: Duration, count: u64, checksum: u64, ns_per_object: Option<u64>) -> Self {
        let elapsed_ns = duration_ns(duration);
        Self {
            elapsed_ns,
            average_ns: Some(elapsed_ns / count.max(1)),
            ns_per_query: None,
            ns_per_object,
            count,
            checksum,
        }
    }

    fn sample(duration: Duration, sample: &OperationSample) -> Self {
        let elapsed_ns = duration_ns(duration);
        let count = sample.query_count.max(sample.objects).max(1);
        Self {
            elapsed_ns,
            average_ns: Some(elapsed_ns),
            ns_per_query: ns_per_object(elapsed_ns, sample.query_count),
            ns_per_object: ns_per_object(elapsed_ns, sample.objects),
            count,
            checksum: sample.checksum,
        }
    }

    fn steady(duration: Duration, iterations: usize, sample: &OperationSample) -> Self {
        let elapsed_ns = duration_ns(duration);
        Self {
            elapsed_ns,
            average_ns: Some(elapsed_ns / iterations as u64),
            ns_per_query: ns_per_object(elapsed_ns, sample.query_count),
            ns_per_object: ns_per_object(elapsed_ns, sample.objects),
            count: iterations as u64,
            checksum: sample.checksum,
        }
    }
}

#[derive(Serialize)]
struct PhaseFaultsReport {
    entry_to_ready: ProcessFaults,
    anchor_resolution: ProcessFaults,
    first_operation: ProcessFaults,
    warmup: ProcessFaults,
    steady_workload: ProcessFaults,
    memory_sample: ProcessFaults,
}

#[derive(Serialize)]
struct AllocationEvidenceReport {
    enabled: bool,
    entry_to_ready: AllocationDeltaReport,
    anchor_resolution: AllocationDeltaReport,
    first_operation: AllocationDeltaReport,
    warmup: AllocationDeltaReport,
    steady_workload: AllocationDeltaReport,
    memory_sample: AllocationDeltaReport,
}

#[derive(Serialize)]
struct MemoryReport {
    #[serde(skip)]
    elapsed: Duration,
    #[serde(skip)]
    faults: ProcessFaults,
    #[serde(skip)]
    allocations: HbkSnapshotExperimentAllocationDelta,
    before_kib: ProcessMemoryReport,
    live_kib: ProcessMemoryReport,
    after_drop_kib: ProcessMemoryReport,
    container_overhead_bytes: u64,
    logical_bytes: u64,
    capacity_bytes: u64,
    live_delta_bytes: u64,
    peak_live_delta_bytes: u64,
    post_drop_delta_bytes: u64,
}

#[derive(Default, Serialize)]
struct ProcessMemoryReport {
    rss_kib: u64,
    pss_kib: u64,
    private_kib: u64,
    anonymous_kib: u64,
    file_backed_kib: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
struct OperationCountsReport {
    query_count: u64,
    candidate_count: u64,
    object_count: u64,
    checksum_count: u64,
    property_count: u64,
    method_count: u64,
    event_count: u64,
    enum_value_count: u64,
}

#[derive(Serialize)]
#[serde(tag = "tag")]
enum OperationData {
    #[serde(rename = "lookup")]
    Lookup {
        query_count: u64,
        candidate_count: u64,
        miss_count: u64,
    },
    #[serde(rename = "iteration")]
    Iteration {
        owner_count: u64,
        scanned_count: u64,
        returned_count: u64,
        universal_count: u64,
        explicit_count: u64,
        excluded_count: u64,
        property_count: u64,
        method_count: u64,
        event_count: u64,
        enum_value_count: u64,
    },
    #[serde(rename = "compact_materialization")]
    CompactMaterialization {
        owner_count: u64,
        scanned_count: u64,
        returned_count: u64,
        universal_count: u64,
        explicit_count: u64,
        excluded_count: u64,
        property_count: u64,
        method_count: u64,
        event_count: u64,
        enum_value_count: u64,
        locator_size: u64,
        total_len: u64,
        total_capacity: u64,
        logical_bytes: u64,
        allocated_bytes: u64,
    },
    #[serde(rename = "payload")]
    Payload {
        input_count: u64,
        object_count: u64,
        string_bytes_touched: u64,
        canonical_payload_bytes_touched: u64,
    },
}

#[derive(Serialize)]
struct ChecksumReport {
    value: u64,
    algorithm: &'static str,
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

#[derive(Clone, Deserialize, Serialize)]
struct InputIdentityReport {
    dataset: String,
    platform_version: String,
    source_locale: String,
    provider_schema_version: u32,
    extraction_schema_version: u32,
    hbk: ArtifactReport,
    provider: ArtifactReport,
}

#[derive(Clone, Deserialize, Serialize)]
struct ArtifactReport {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Serialize)]
struct ParityReport {
    schema_version: &'static str,
    workload_version: &'static str,
    mode: &'static str,
    backend: &'static str,
    decision_role: &'static str,
    availability_context: &'static str,
    module_context_filter_used: bool,
    empty_availability_rule: &'static str,
    input_identity: InputIdentityReport,
    manifest: ManifestIdentityReport,
    runtime_artifacts: Vec<ArtifactReport>,
    transcript: Vec<ParityRecord>,
}

#[derive(Serialize)]
struct ParityMemberSet {
    owner_logical_id: String,
    borrowed_members: Vec<String>,
    compact_members: Vec<String>,
}

#[derive(Serialize)]
struct ParityType {
    payload: TranscriptType,
}

#[derive(Serialize)]
struct ParityCallable {
    payload: TranscriptCallable,
}

#[derive(Serialize)]
struct ParityFilteredMember {
    owner_logical_id: String,
    availability_rule: &'static str,
    payload: TranscriptMember,
}

#[derive(Serialize)]
struct ParityMember {
    owner_logical_id: String,
    payload: TranscriptMember,
}

impl From<ParityMember> for ParityRecord {
    fn from(value: ParityMember) -> Self {
        Self::Member(value)
    }
}

#[derive(Serialize)]
struct ParityLookup {
    operation: &'static str,
    owner_logical_id: Option<String>,
    member_kind: Option<&'static str>,
    query_name: String,
    query_role: String,
    declared_logical_id: Option<String>,
    results: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TranscriptName {
    primary: String,
    alias: Option<String>,
}

#[derive(Serialize)]
struct TranscriptMember {
    logical_id: String,
    owner_logical_id: String,
    kind: &'static str,
    name: TranscriptName,
    type_refs: Vec<TranscriptTypeRef>,
    availability_contexts: Vec<String>,
    available_since: Option<String>,
}

#[derive(Serialize)]
struct TranscriptType {
    logical_id: String,
    name: TranscriptName,
    metadata_template: Option<TranscriptMetadataTemplate>,
    type_template_key: Option<TranscriptTemplateKey>,
    availability_contexts: Vec<String>,
    available_since: Option<String>,
}

#[derive(Serialize)]
struct TranscriptMetadataTemplate {
    metadata_kind: String,
    template_parameters: Vec<String>,
}

#[derive(Serialize)]
struct TranscriptCallable {
    logical_id: String,
    owner_logical_id: Option<String>,
    kind: &'static str,
    name: TranscriptName,
    availability_contexts: Vec<String>,
    available_since: Option<String>,
    signatures: Vec<TranscriptSignature>,
    return_type_refs: Vec<TranscriptTypeRef>,
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
    template_key: Option<TranscriptTemplateKey>,
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
    arguments: Vec<TranscriptTemplateArgument>,
}

#[derive(Serialize)]
struct TranscriptTemplateArgument {
    kind: &'static str,
    owner_parameter_index: usize,
    target_parameter_index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_av2_command_family_as_separate_operation_process() {
        let command = parse_args([
            "measure".to_string(),
            "cache-owned".to_string(),
            "index.sqlite".to_string(),
            "cache.bin".to_string(),
            "manifest.json".to_string(),
            "members_by_owner_availability_collect".to_string(),
            "server".to_string(),
            "3".to_string(),
        ])
        .unwrap();
        assert_eq!(
            command,
            Command::Measure {
                source: Source::CacheOwned {
                    index_path: PathBuf::from("index.sqlite"),
                    cache_path: PathBuf::from("cache.bin"),
                },
                manifest_path: PathBuf::from("manifest.json"),
                operation: Operation::MembersByOwnerAvailabilityCollect,
                context: Some(AvailabilityContext { code: "server" }),
                iterations: 3,
            }
        );
    }

    #[test]
    fn context_is_required_only_for_filtered_member_operations() {
        assert!(
            parse_args([
                "measure".to_string(),
                "sql-owned".to_string(),
                "index.sqlite".to_string(),
                "manifest.json".to_string(),
                "members_by_owner_availability_borrowed".to_string(),
            ])
            .is_err()
        );
        assert!(
            parse_args([
                "measure".to_string(),
                "sql-owned".to_string(),
                "index.sqlite".to_string(),
                "manifest.json".to_string(),
                "type_by_name".to_string(),
                "5".to_string(),
            ])
            .is_ok()
        );
    }

    #[test]
    fn parses_standalone_parity_transcript_command() {
        let command = parse_args([
            "parity".to_string(),
            "sql-owned".to_string(),
            "index.sqlite".to_string(),
            "manifest.json".to_string(),
            "server".to_string(),
        ])
        .unwrap();
        assert_eq!(
            command,
            Command::Parity {
                source: Source::SqlOwned {
                    index_path: PathBuf::from("index.sqlite"),
                },
                manifest_path: PathBuf::from("manifest.json"),
                context: AvailabilityContext { code: "server" },
            }
        );
    }

    #[test]
    fn accepts_only_frozen_availability_contexts() {
        for code in AVAILABILITY_CONTEXTS {
            assert_eq!(
                AvailabilityContext::parse(code).map(|context| context.code),
                Some(*code)
            );
        }
        assert_eq!(AVAILABILITY_CONTEXTS.len(), 9);
        assert!(AvailabilityContext::parse("ModuleContextKind").is_none());
        assert!(AvailabilityContext::parse("thin-client").is_none());
    }

    #[test]
    fn availability_filter_treats_empty_as_universal() {
        assert_eq!(
            availability_match_codes([], AvailabilityContext { code: "server" }),
            AvailabilityMatch::Universal
        );
    }

    #[test]
    fn compact_operation_data_is_the_only_variant_with_retained_fields() {
        let json = serde_json::to_value(operation_data(
            Operation::MembersByOwnerAvailabilityCollect,
            OperationSample {
                total_len: 2,
                total_capacity: 4,
                logical_bytes: 8,
                allocated_bytes: 16,
                ..OperationSample::default()
            },
        ))
        .unwrap();
        assert_eq!(json["tag"], "compact_materialization");
        assert_eq!(json["locator_size"], 4);
        assert!(json.get("total_capacity").is_some());

        let lookup = serde_json::to_value(operation_data(
            Operation::TypeByName,
            OperationSample::default(),
        ))
        .unwrap();
        assert_eq!(lookup["tag"], "lookup");
        assert!(lookup.get("total_capacity").is_none());
        assert!(lookup.get("allocated_bytes").is_none());
    }

    #[test]
    fn payload_operation_data_separates_input_count_from_object_count() {
        let json = serde_json::to_value(operation_data(
            Operation::MethodPayload,
            OperationSample {
                input_count: 1,
                objects: 3,
                string_bytes: 17,
                canonical_payload_bytes: 41,
                ..OperationSample::default()
            },
        ))
        .unwrap();

        assert_eq!(json["tag"], "payload");
        assert_eq!(json["input_count"], 1);
        assert_eq!(json["object_count"], 3);
        assert_eq!(json["string_bytes_touched"], 17);
        assert_eq!(json["canonical_payload_bytes_touched"], 41);
    }

    #[test]
    fn benchmark_report_schema_uses_frozen_tags_and_no_module_context_text() {
        let report = BenchmarkReport {
            schema_version: REPORT_SCHEMA_VERSION,
            workload_version: WORKLOAD_VERSION,
            mode: "performance",
            backend: "S83-H0",
            decision_role: "baseline",
            operation: "type_by_name",
            availability_context: None,
            iterations: 1,
            module_context_filter_used: false,
            empty_availability_rule: "universal",
            input_identity: static_input_identity(),
            manifest: ManifestIdentityReport {
                schema_version: MANIFEST_SCHEMA_VERSION,
                sha256: "d".repeat(64),
                bytes: 100,
            },
            runtime_artifacts: vec![ArtifactReport {
                path: "index.sqlite".to_owned(),
                bytes: 1,
                sha256: "d".repeat(64),
            }],
            projection: Operation::TypeByName.projection(),
            phase_order: PHASE_ORDER,
            timings: TimingReport {
                entry_to_ready: timing_phase(1),
                anchor_resolution: timing_phase(2),
                first_operation: timing_phase(3),
                warmup: timing_phase(4),
                steady_workload: timing_phase(5),
                memory_sample: timing_phase(6),
            },
            faults: PhaseFaultsReport {
                entry_to_ready: ProcessFaults::default(),
                anchor_resolution: ProcessFaults::default(),
                first_operation: ProcessFaults::default(),
                warmup: ProcessFaults::default(),
                steady_workload: ProcessFaults::default(),
                memory_sample: ProcessFaults::default(),
            },
            allocations: AllocationEvidenceReport {
                enabled: false,
                entry_to_ready: empty_delta(),
                anchor_resolution: empty_delta(),
                first_operation: empty_delta(),
                warmup: empty_delta(),
                steady_workload: empty_delta(),
                memory_sample: empty_delta(),
            },
            memory: MemoryReport {
                elapsed: Duration::ZERO,
                faults: ProcessFaults::default(),
                allocations: HbkSnapshotExperimentAllocationDelta::default(),
                before_kib: ProcessMemoryReport::default(),
                live_kib: ProcessMemoryReport::default(),
                after_drop_kib: ProcessMemoryReport::default(),
                container_overhead_bytes: 0,
                logical_bytes: 0,
                capacity_bytes: 0,
                live_delta_bytes: 0,
                peak_live_delta_bytes: 0,
                post_drop_delta_bytes: 0,
            },
            counts: OperationCountsReport::default(),
            checksum: ChecksumReport {
                value: 0,
                algorithm: "rolling-u64",
            },
            operation_data: OperationData::Lookup {
                query_count: 1,
                candidate_count: 1,
                miss_count: 0,
            },
        };

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"schema_version\":\"hbk-s83-av2-benchmark/v1\""));
        assert!(json.contains("\"phase_order\":[\"entry_to_ready\",\"anchor_resolution\",\"first_operation\",\"warmup\",\"steady_workload\",\"memory_sample\"]"));
        assert!(!json.contains("ModuleContextKind"));
    }

    #[test]
    fn query_manifest_deserialization_rejects_unknown_fields_and_anchor_drift() {
        let path = temp_manifest_path("strict");
        fs::write(&path, minimal_manifest_json("\"unexpected\":true,")).unwrap();
        assert!(read_manifest(&path).is_err());

        fs::write(
            &path,
            minimal_manifest_json("\"anchors\":{\"type_primary\":\"Другой\",\"property_owner\":\"platform_type:Запрос\",\"property_name\":\"Текст\",\"method_owner\":\"platform_type:Запрос\",\"method_name\":\"Выполнить\",\"enumeration_owner\":\"platform_type:ФормаКлиентскогоПриложения\"},"),
        )
        .unwrap();
        assert!(read_manifest(&path).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn smaps_rollup_parser_ignores_mapping_header() {
        let report = parse_smaps_rollup(
            "5705fb459000-7ffe36b9b000 ---p 00000000 00:00 0 [rollup]\n\
             Rss: 100 kB\n\
             Pss: 80 kB\n\
             Private_Clean: 10 kB\n\
             Private_Dirty: 20 kB\n\
             Anonymous: 40 kB\n",
        )
        .unwrap();
        assert_eq!(report.rss_kib, 100);
        assert_eq!(report.pss_kib, 80);
        assert_eq!(report.private_kib, 30);
        assert_eq!(report.anonymous_kib, 40);
        assert_eq!(report.file_backed_kib, 60);
    }

    fn empty_delta() -> AllocationDeltaReport {
        HbkSnapshotExperimentAllocationDelta::default().into()
    }

    fn timing_phase(value: u64) -> TimingPhaseReport {
        TimingPhaseReport {
            elapsed_ns: value,
            average_ns: Some(value),
            ns_per_query: None,
            ns_per_object: None,
            count: 1,
            checksum: value,
        }
    }

    fn temp_manifest_path(label: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "measure_hbk_s83_av2_{label}_{}_{}.json",
            process::id(),
            Instant::now().elapsed().as_nanos()
        ))
    }

    fn minimal_manifest_json(extra: &str) -> String {
        format!(
            r#"{{
                "schema_version":"{MANIFEST_SCHEMA_VERSION}",
                "workload_version":"{WORKLOAD_VERSION}",
                "input_identity":{{
                    "dataset":"{DATASET}",
                    "platform_version":"{PLATFORM_VERSION}",
                    "source_locale":"{SOURCE_LOCALE}",
                    "provider_schema_version":{PROVIDER_SCHEMA_VERSION},
                    "extraction_schema_version":{EXTRACTION_SCHEMA_VERSION},
                    "hbk":{{"path":"{SOURCE_HBK_PATH}","bytes":{SOURCE_HBK_BYTES},"sha256":"{SOURCE_HBK_SHA256}"}},
                    "provider":{{"path":"provider","bytes":{PROVIDER_BYTES},"sha256":"{PROVIDER_SHA256}"}}
                }},
                "availability_contexts":["thin_client","web_client","mobile_client","server","thick_client","external_connection","mobile_application_client","mobile_application_server","mobile_standalone_server"],
                "member_kinds":["property","method","event","enum_value"],
                "empty_availability_rule":"universal",
                "module_context_filter_used":false,
                "types":[],
                "members":[],
                "lookup_queries":{{"type_names":[],"properties":[],"methods":[]}},
                "fixed_misses":{{"type_name":"__hbk_s83_av2_missing_type__","member_name":"__hbk_s83_av2_missing_member__","callable_name":"__hbk_s83_av2_missing_callable__"}},
                {extra}
                "anchors":{{"type_primary":"Запрос","property_owner":"platform_type:Запрос","property_name":"Текст","method_owner":"platform_type:Запрос","method_name":"Выполнить","enumeration_owner":"platform_type:ФормаКлиентскогоПриложения"}}
            }}"#
        )
    }
}
