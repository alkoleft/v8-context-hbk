use super::*;

mod indexes;
mod materialize;
mod memory;
mod read;
mod types;

use indexes::{
    CsrIndex, FactStringLookup, GlobalNameKindLookup, IdLookup, MemberNameKindLookup,
    ModuleContextLookup, NameLookup, OwnerNameLookup, RelationLookupKey, TypeTemplateLookup,
};

pub use memory::{HbkFactSnapshotIndexMemory, HbkFactSnapshotMemory, HbkFactSnapshotMemoryEntry};
pub use read::HbkFactSnapshotCounts;
pub use types::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkFactSnapshot {
    strings: Vec<String>,
    platform_types: Vec<HbkPlatformType>,
    type_members: Vec<HbkTypeMember>,
    callables: Vec<HbkCallable>,
    globals: Vec<HbkGlobalFact>,
    query_tables: Vec<HbkQueryTable>,
    query_fields: Vec<HbkQueryField>,
    query_parameters: Vec<HbkQueryParameter>,
    language_facts: Vec<HbkLanguageFact>,
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
    availability_by_fact: CsrIndex<HbkFactRef, StringId>,
    availability_since_by_fact: Vec<FactStringLookup>,
    relations_by_source_kind: CsrIndex<RelationLookupKey, HbkFactRef>,
}

#[derive(Debug, Clone, Copy)]
pub struct HbkFactReadHandle<'a> {
    snapshot: &'a HbkFactSnapshot,
}
