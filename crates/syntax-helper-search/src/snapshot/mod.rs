use super::*;

mod binary_cache;
#[cfg(feature = "snapshot-experiment")]
mod experiment_allocator;
#[cfg(feature = "snapshot-experiment")]
mod experiment_oracle;
mod indexes;
mod materialize;
mod memory;
mod read;
mod types;
mod views;
mod x1_format;

use indexes::{
    CsrIndex, FactSourceLookup, FactStringLookup, GlobalNameKindLookup, IdLookup,
    MemberNameKindLookup, ModuleContextLookup, NameLookup, OwnerNameLookup, RelationLookupKey,
    TypeTemplateLookup,
};
use x1_format::{X1MappedReadHandle, X1StableSlotGeneration};

pub use binary_cache::{HbkFactSnapshotCacheLoadReport, HbkFactSnapshotCacheStatus};
#[cfg(feature = "snapshot-experiment")]
#[doc(hidden)]
pub use experiment_allocator::{
    HbkSnapshotExperimentAllocationDelta, HbkSnapshotExperimentAllocationSnapshot,
    HbkSnapshotExperimentAllocator, experiment_allocation_snapshot,
};
#[cfg(feature = "snapshot-experiment")]
#[doc(hidden)]
pub use experiment_oracle::{
    write_owned_snapshot_lookup_transcript_jsonl, write_owned_snapshot_oracle_jsonl,
};
pub use memory::{HbkFactSnapshotIndexMemory, HbkFactSnapshotMemory, HbkFactSnapshotMemoryEntry};
pub use read::HbkFactSnapshotCounts;
pub use types::*;
pub use views::*;
pub use x1_format::{HbkFactSnapshotArtifactPublicationReport, HbkFactSnapshotArtifactWriteReport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkFactSnapshot {
    mapped_generation: Option<std::sync::Arc<X1StableSlotGeneration>>,
    strings: Vec<String>,
    source_locale: Option<StringId>,
    platform_types: Vec<HbkPlatformType>,
    type_members: Vec<HbkTypeMember>,
    callables: Vec<HbkCallable>,
    globals: Vec<HbkGlobalFact>,
    query_tables: Vec<HbkQueryTable>,
    query_fields: Vec<HbkQueryField>,
    query_parameters: Vec<HbkQueryParameter>,
    language_facts: Vec<HbkLanguageFact>,
    enums: Vec<HbkEnum>,
    enum_values: Vec<HbkEnumValue>,
    fact_ids: Vec<IdLookup<HbkFactRef>>,
    platform_type_ids: Vec<IdLookup<HbkPlatformTypeId>>,
    platform_type_names: Vec<NameLookup<HbkPlatformTypeId>>,
    platform_type_templates: Vec<TypeTemplateLookup<HbkPlatformTypeId>>,
    member_ids: Vec<IdLookup<HbkTypeMemberId>>,
    members_by_owner: CsrIndex<HbkPlatformTypeId, HbkTypeMemberId>,
    members_by_owner_name: Vec<OwnerNameLookup<HbkPlatformTypeId, HbkTypeMemberId>>,
    members_by_owner_name_kind: Vec<MemberNameKindLookup>,
    callable_ids: Vec<IdLookup<HbkCallableId>>,
    callables_by_owner: CsrIndex<HbkPlatformTypeId, HbkCallableId>,
    callables_by_owner_name: Vec<OwnerNameLookup<HbkPlatformTypeId, HbkCallableId>>,
    constructors_by_type: CsrIndex<HbkPlatformTypeId, HbkCallableId>,
    global_names: Vec<NameLookup<HbkGlobalFactId>>,
    globals_by_domain_name_kind: Vec<GlobalNameKindLookup>,
    module_event_names: Vec<OwnerNameLookup<StringId, HbkCallableId>>,
    module_contexts_by_domain_language_kind: Vec<ModuleContextLookup>,
    query_table_ids: Vec<IdLookup<HbkQueryTableId>>,
    query_table_names: Vec<NameLookup<HbkQueryTableId>>,
    query_table_syntax_names: Vec<NameLookup<HbkQueryTableId>>,
    query_table_identifiers: Vec<NameLookup<HbkQueryTableId>>,
    query_fields_by_table: CsrIndex<HbkQueryTableId, HbkQueryFieldId>,
    query_fields_by_table_name: Vec<OwnerNameLookup<HbkQueryTableId, HbkQueryFieldId>>,
    query_parameters_by_table: CsrIndex<HbkQueryTableId, HbkQueryParameterId>,
    query_parameters_by_table_name: Vec<OwnerNameLookup<HbkQueryTableId, HbkQueryParameterId>>,
    language_ids: Vec<IdLookup<HbkLanguageFactId>>,
    language_names: Vec<NameLookup<HbkLanguageFactId>>,
    enum_ids: Vec<IdLookup<HbkEnumId>>,
    enum_names: Vec<NameLookup<HbkEnumId>>,
    enum_value_ids: Vec<IdLookup<HbkEnumValueId>>,
    enum_values_by_enum: CsrIndex<HbkEnumId, HbkEnumValueId>,
    enum_values_by_enum_name: Vec<OwnerNameLookup<HbkEnumId, HbkEnumValueId>>,
    availability_by_fact: CsrIndex<HbkFactRef, StringId>,
    availability_since_by_fact: Vec<FactStringLookup>,
    source_by_fact: Vec<FactSourceLookup>,
    relations_by_source_kind: CsrIndex<RelationLookupKey, HbkFactRef>,
}

impl HbkFactSnapshot {
    fn from_mapped_generation(generation: X1StableSlotGeneration) -> Self {
        fn empty_csr<K, V>() -> CsrIndex<K, V> {
            CsrIndex {
                keys: Vec::new(),
                offsets: vec![0],
                values: Vec::new(),
            }
        }

        Self {
            mapped_generation: Some(std::sync::Arc::new(generation)),
            strings: Vec::new(),
            source_locale: None,
            platform_types: Vec::new(),
            type_members: Vec::new(),
            callables: Vec::new(),
            globals: Vec::new(),
            query_tables: Vec::new(),
            query_fields: Vec::new(),
            query_parameters: Vec::new(),
            language_facts: Vec::new(),
            enums: Vec::new(),
            enum_values: Vec::new(),
            fact_ids: Vec::new(),
            platform_type_ids: Vec::new(),
            platform_type_names: Vec::new(),
            platform_type_templates: Vec::new(),
            member_ids: Vec::new(),
            members_by_owner: empty_csr(),
            members_by_owner_name: Vec::new(),
            members_by_owner_name_kind: Vec::new(),
            callable_ids: Vec::new(),
            callables_by_owner: empty_csr(),
            callables_by_owner_name: Vec::new(),
            constructors_by_type: empty_csr(),
            global_names: Vec::new(),
            globals_by_domain_name_kind: Vec::new(),
            module_event_names: Vec::new(),
            module_contexts_by_domain_language_kind: Vec::new(),
            query_table_ids: Vec::new(),
            query_table_names: Vec::new(),
            query_table_syntax_names: Vec::new(),
            query_table_identifiers: Vec::new(),
            query_fields_by_table: empty_csr(),
            query_fields_by_table_name: Vec::new(),
            query_parameters_by_table: empty_csr(),
            query_parameters_by_table_name: Vec::new(),
            language_ids: Vec::new(),
            language_names: Vec::new(),
            enum_ids: Vec::new(),
            enum_names: Vec::new(),
            enum_value_ids: Vec::new(),
            enum_values_by_enum: empty_csr(),
            enum_values_by_enum_name: Vec::new(),
            availability_by_fact: empty_csr(),
            availability_since_by_fact: Vec::new(),
            source_by_fact: Vec::new(),
            relations_by_source_kind: empty_csr(),
        }
    }

    fn mapped_read_handle(&self) -> Option<X1MappedReadHandle<'_>> {
        self.mapped_generation
            .as_deref()
            .map(X1StableSlotGeneration::read_handle)
    }

    fn assert_owned(&self) {
        assert!(
            self.mapped_generation.is_none(),
            "owned-only snapshot access is unavailable for an X1 mapped snapshot; use HbkFactReadHandle views"
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkFactSnapshotBuildReport {
    pub snapshot: HbkFactSnapshot,
    pub timings: HbkFactSnapshotStageTimings,
    cache_index_path: PathBuf,
    cache_metadata: binary_cache::CacheMetadata,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HbkFactSnapshotStageTimings {
    pub total: Duration,
    pub open_index: Duration,
    pub read_sql_rows: Duration,
    pub build_lookup_maps: Duration,
    pub build_platform_types: Duration,
    pub group_type_refs: Duration,
    pub build_signatures: Duration,
    pub build_fact_arenas: Duration,
    pub build_fact_ids_relations_availability: Duration,
    pub sort_secondary_indexes: Duration,
    pub assemble_snapshot: Duration,
}

#[derive(Debug, Clone, Copy)]
pub struct HbkFactReadHandle<'a> {
    snapshot: &'a HbkFactSnapshot,
}
