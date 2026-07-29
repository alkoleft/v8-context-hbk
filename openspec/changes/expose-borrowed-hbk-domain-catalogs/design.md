## Context

The analyzer hot path needs borrowed access to documented HBK BSL-context and
SDBL query-domain facts without materializing generic owned resolver DTOs for
each lookup. ADR-0008 accepted a source-neutral Rust `ContextResolver` boundary
for in-process solution analysis. Later snapshot changes made
`syntax-helper-search::HbkFactSnapshot` and `HbkFactReadHandle` the
provider-owned immutable storage/read boundary for worker-safe analyzer paths.

This change refines, rather than replaces, ADR-0008. Generic resolver DTOs stay
as the source-neutral compatibility contract. Two borrowed domain catalogs
become the hot-path HBK contract for BSL and SDBL facts that already have a
typed snapshot representation.

## Goals/Non-Goals

Goals:

- Expose exact public borrowed catalog APIs in `context-resolver-search` as
  `HbkBslContextCatalog` and `HbkSdblQueryCatalog`.
- Reuse existing `syntax-helper-search` snapshot ID and record types directly:
  `HbkPlatformTypeId`, `HbkPlatformType`, `HbkTypeMemberId`,
  `HbkTypeMember`, `HbkCallableId`, `HbkCallable`, `HbkGlobalFactId`,
  `HbkGlobalFact`, `HbkQueryTableId`, `HbkQueryTable`, `HbkQueryFieldId`,
  `HbkQueryField`, `HbkQueryParameterId`, `HbkQueryParameter`, `StringId`,
  source IDs and locale.
- Keep `HbkFactSnapshot`/`HbkFactReadHandle` as the sole arena, record and
  source-index owner.
- Move catalog-covered BSL availability/module context key retrieval and SDBL
  query-source classification behavior behind the catalog boundary.
- Make existing snapshot resolver adapters delegate to the catalogs and project
  to `context-resolver-core` DTOs only once at the generic resolver boundary.
- Keep SQL/SearchIndex adapters explicit for CLI, debug and local sequential
  flows, while preventing hidden SQL fallback from snapshot catalog paths.

Non-goals:

- No public `HbkFactRef` domain API.
- No catalog-specific DTO mirror for snapshot records.
- No analyzer-side shim, private metadata read, SQLite fallback or selector
  mapping table.
- No universal borrowed catalog trait that hides BSL and SDBL domain
  differences.
- No immediate removal of `ContextResolver`.
- No CLI/debug/index-inspection migration away from explicit `SearchIndex`
  flows in this change.

## Lower-Level Lifetime Requirement

The catalog API must not return iterators borrowing a temporary
`snapshot.worker_handle()` value. Before or with catalog implementation,
`syntax-helper-search::HbkFactReadHandle<'a>` lookup methods that return
iterators or borrowed slices SHALL take `self` by value and return values tied
to the handle's snapshot lifetime `'a` where needed. This applies to relevant
`Copy` handle methods such as global, platform-type, member/callable,
module-event, query-table, query-field, query-parameter and availability
lookups.

This is a signature/lifetime correction on the existing read handle, not a
second public read API. `HbkFactReadHandle<'a>` remains `Copy`, still wraps the
same `&'a HbkFactSnapshot`, and still exposes the same underlying snapshot
indexes.

## Public API Contract

`context-resolver-search` SHALL add a narrow `hbk_catalogs` module and re-export
only the two catalog handle types from the crate facade:

```rust
pub struct HbkBslContextCatalog {
    source_id: SourceId,
    snapshot: Arc<HbkFactSnapshot>,
}

pub struct HbkSdblQueryCatalog {
    source_id: SourceId,
    platform_source_id: SourceId,
    snapshot: Arc<HbkFactSnapshot>,
}
```

The catalog structs are the only new public semantic structures in this change.
They are handles over existing storage, not record mirrors. They expose
`source_id`, `platform_source_id` where applicable, `source_locale`, borrowed
string lookup, and constructors matching the existing snapshot adapter source
ID defaults. They do not expose the underlying `Arc<HbkFactSnapshot>` publicly;
snapshot-backed compatibility adapters in the same crate may use a
crate-private accessor where retained projection/relation behavior requires it.

The BSL catalog SHALL expose methods with these contracts:

```rust
impl HbkBslContextCatalog {
    pub fn new(snapshot: Arc<HbkFactSnapshot>) -> Self;
    pub fn with_source_id(snapshot: Arc<HbkFactSnapshot>, source_id: SourceId) -> Self;
    pub fn source_id(&self) -> &SourceId;
    pub fn source_locale(&self) -> Option<&str>;
    pub fn string(&self, id: StringId) -> &str;

    pub fn platform_type_by_id(
        &self,
        id: &str,
    ) -> Option<(HbkPlatformTypeId, &HbkPlatformType)>;

    pub fn platform_types_by_name(
        &self,
        name: &str,
    ) -> impl Iterator<Item = (HbkPlatformTypeId, &HbkPlatformType)> + '_;

    pub fn member_by_id(
        &self,
        id: &str,
    ) -> Option<(HbkTypeMemberId, &HbkTypeMember)>;

    pub fn callable_by_id(
        &self,
        id: &str,
    ) -> Option<(HbkCallableId, &HbkCallable)>;

    pub fn global_by_id(
        &self,
        id: &str,
    ) -> Option<(HbkGlobalFactId, &HbkGlobalFact)>;

    pub fn platform_types_by_template_key(
        &self,
        family: &str,
        variant: &str,
    ) -> impl Iterator<Item = (HbkPlatformTypeId, &HbkPlatformType)> + '_;

    pub fn generated_self_types(
        &self,
        role: &str,
    ) -> impl Iterator<Item = (HbkPlatformTypeId, &HbkPlatformType)> + '_;

    pub fn members(
        &self,
        owner: HbkPlatformTypeId,
    ) -> impl ExactSizeIterator<Item = (HbkTypeMemberId, &HbkTypeMember)> + '_;

    pub fn member_by_name(
        &self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> impl Iterator<Item = (HbkTypeMemberId, &HbkTypeMember)> + '_;

    pub fn member_by_name_kind(
        &self,
        owner: HbkPlatformTypeId,
        name: &str,
        kind: Option<HbkTypeMemberKind>,
    ) -> impl Iterator<Item = (HbkTypeMemberId, &HbkTypeMember)> + '_;

    pub fn callables(
        &self,
        owner: HbkPlatformTypeId,
    ) -> impl ExactSizeIterator<Item = (HbkCallableId, &HbkCallable)> + '_;

    pub fn callable_by_name(
        &self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> impl Iterator<Item = (HbkCallableId, &HbkCallable)> + '_;

    pub fn constructors(
        &self,
        owner: HbkPlatformTypeId,
    ) -> impl ExactSizeIterator<Item = (HbkCallableId, &HbkCallable)> + '_;

    pub fn global_properties(
        &self,
    ) -> impl Iterator<Item = (HbkGlobalFactId, &HbkGlobalFact)> + '_;

    pub fn global_property_by_name(
        &self,
        name: &str,
    ) -> impl Iterator<Item = (HbkGlobalFactId, &HbkGlobalFact)> + '_;

    pub fn global_methods(
        &self,
    ) -> impl Iterator<Item = (HbkGlobalFactId, &HbkGlobalFact, HbkCallableId, &HbkCallable)> + '_;

    pub fn global_method_by_name(
        &self,
        name: &str,
    ) -> impl Iterator<Item = (HbkGlobalFactId, &HbkGlobalFact, HbkCallableId, &HbkCallable)> + '_;

    pub fn module_context_events(
        &self,
        kind: ModuleContextKind,
    ) -> impl Iterator<Item = (HbkCallableId, &HbkCallable)> + '_;

    pub fn module_context_event_by_name(
        &self,
        kind: ModuleContextKind,
        name: &str,
    ) -> impl Iterator<Item = (HbkCallableId, &HbkCallable)> + '_;

    pub fn platform_type_availability(
        &self,
        id: HbkPlatformTypeId,
    ) -> (&[StringId], Option<StringId>);

    pub fn member_availability(
        &self,
        id: HbkTypeMemberId,
    ) -> (&[StringId], Option<StringId>);

    pub fn callable_availability(
        &self,
        id: HbkCallableId,
    ) -> (&[StringId], Option<StringId>);

    pub fn global_availability(
        &self,
        id: HbkGlobalFactId,
    ) -> (&[StringId], Option<StringId>);
}
```

`generated_self_types(role)` is derived from the existing
`template_key_for_generated_self_role(role)` mapping and
`HbkFactReadHandle::platform_types_by_template_key`. Generated-self type access
is therefore not a materialization gap. The mapping may stay in
`context-resolver-search` because it maps analyzer/query role strings to
platform type-template keys already used by search adapters; it must not be
duplicated downstream.

`context-resolver-core` remains the sole owner of
`metadata.module-role.*` selector to `ModuleContextKind` translation. If catalog
consumers need that conversion directly, the existing helper in
`context-resolver-core` must be made public instead of adding a second mapping
in HBK or analyzer code. The BSL catalog privately owns only
`ModuleContextKind` to HBK module-context relation key retrieval; that
storage-key mapping is not a public escape hatch.

The SDBL catalog SHALL expose methods with these contracts:

```rust
impl HbkSdblQueryCatalog {
    pub fn new(snapshot: Arc<HbkFactSnapshot>) -> Self;
    pub fn with_source_ids(
        snapshot: Arc<HbkFactSnapshot>,
        source_id: SourceId,
        platform_source_id: SourceId,
    ) -> Self;
    pub fn source_id(&self) -> &SourceId;
    pub fn platform_source_id(&self) -> &SourceId;
    pub fn source_locale(&self) -> Option<&str>;
    pub fn string(&self, id: StringId) -> &str;

    pub fn query_table_by_id(
        &self,
        id: &str,
    ) -> Option<(HbkQueryTableId, &HbkQueryTable)>;

    pub fn query_field_by_id(
        &self,
        id: &str,
    ) -> Option<(HbkQueryFieldId, &HbkQueryField)>;

    pub fn query_parameter_by_id(
        &self,
        id: &str,
    ) -> Option<(HbkQueryParameterId, &HbkQueryParameter)>;

    pub fn query_tables(
        &self,
    ) -> impl Iterator<Item = (HbkQueryTableId, &HbkQueryTable)> + '_;

    pub fn query_tables_by_name(
        &self,
        name: &str,
    ) -> impl Iterator<Item = (HbkQueryTableId, &HbkQueryTable)> + '_;

    pub fn query_tables_by_syntax(
        &self,
        syntax: &str,
    ) -> impl Iterator<Item = (HbkQueryTableId, &HbkQueryTable)> + '_;

    pub fn query_tables_by_identifier(
        &self,
        identifier: &str,
    ) -> impl Iterator<Item = (HbkQueryTableId, &HbkQueryTable)> + '_;

    pub fn query_fields(
        &self,
        table: HbkQueryTableId,
    ) -> impl ExactSizeIterator<Item = (HbkQueryFieldId, &HbkQueryField)> + '_;

    pub fn query_field_by_name(
        &self,
        table: HbkQueryTableId,
        name: &str,
    ) -> impl Iterator<Item = (HbkQueryFieldId, &HbkQueryField)> + '_;

    pub fn query_parameters(
        &self,
        table: HbkQueryTableId,
    ) -> impl ExactSizeIterator<Item = (HbkQueryParameterId, &HbkQueryParameter)> + '_;

    pub fn query_parameter_by_name(
        &self,
        table: HbkQueryTableId,
        name: &str,
    ) -> impl Iterator<Item = (HbkQueryParameterId, &HbkQueryParameter)> + '_;

    pub fn metadata_source_selector(
        &self,
        table: HbkQueryTableId,
    ) -> Option<&'static str>;

    pub fn metadata_source_selector_for_identifier(
        &self,
        identifier: Option<&str>,
    ) -> Option<&'static str>;
}
```

Borrowed versus copied contract:

- IDs are existing snapshot copy IDs.
- Records are borrowed from `HbkFactSnapshot`.
- Names, aliases, identifiers, syntax, locale, availability contexts, type
  references, signatures, defaults and notes remain in snapshot records as
  `StringId`, `HbkName`, `HbkTypeRef` and vectors owned by the snapshot.
- The only copied values returned by catalogs are small `Copy` IDs and
  `Option<&'static str>` for the accepted SDBL selector literals.
- Iterators are lazy views over snapshot ID indexes. They may allocate only when
  a method must merge multiple existing snapshot indexes and prove
  deterministic de-duplication, such as matching query table name plus syntax
  plus identifier. Such allocation is local to the call result order and is not
  a retained catalog cache.
- `HbkFactRef` may be used privately inside catalog implementation to query
  existing snapshot availability or relations, but no catalog method may expose
  it.

`HbkSdblQueryCatalog::metadata_source_selector_for_identifier` applies the
mapping only when `source_locale() == Some("ru")`. The catalog and the retained
SQL adapter reuse one crate-private `(locale, identifier) -> selector` function
in `hbk_catalogs::sdbl`; the SQL adapter passes
`SearchDocument::source.locale`. This keeps one selector behavior owner without
requiring a snapshot-backed catalog in an explicit SQL/SearchIndex flow.

The catalogs preserve provenance inputs rather than adding a provenance DTO:
`source_id`, `source_locale`, the typed record ID and the record's stored
`StringId` identity remain directly available. Generic adapters may project
those borrowed inputs once into `FactProvenance` at their compatibility
boundary.

## Consumer Inventory

| Consumer | Current path | Catalog decision |
| --- | --- | --- |
| `PlatformSnapshotSource::resolve_type` | Uses `platform_type_by_id`, `platform_types_by_name`, `platform_types_by_template_key` and generated-self role mapping. | Delegate type acquisition to `HbkBslContextCatalog`, including `generated_self_types(role)`; retain `ResolvedType` projection. |
| `PlatformSnapshotSource::members` | Uses `members_of_type`, `member_by_owner_name_kind`, `callables_of_type`, `callable_by_owner_name`, constructors and enum values. | Delegate platform member/callable acquisition to `HbkBslContextCatalog`; retain generic `MemberQuery` validation and DTO projection. |
| `PlatformSnapshotSource::callable` | Uses callable/global method lookup paths. | Delegate callable acquisition to `HbkBslContextCatalog`; retain `CallableLookup` validation and DTO projection. |
| `PlatformSnapshotSource::global_context` | Scans `HbkFactReadHandle::global_fact_ids`, maps to `ResolvedGlobalContext`. | Delegate acquisition to `HbkBslContextCatalog::{global_methods,global_properties}`; keep DTO projection in `PlatformSnapshotSource`. |
| `PlatformSnapshotSource::module_context` | Uses `search_module_context_relation_key`, `module_context_events`, and `global_context`. | Delegate BSL global/member/event acquisition to `HbkBslContextCatalog`; BSL catalog owns `ModuleContextKind` to HBK key retrieval. |
| `PlatformSnapshotSource::module_context_member(s)` | Repeats global method/property/event lookup. | Delegate point and enumeration acquisition to `HbkBslContextCatalog`; retain only `ResolvedBslContextMember` projection. |
| `PlatformSnapshotSource::availability` | Resolves local `FactId` to private `HbkFactRef`, then maps availability. | Delegate typed platform type/member/callable/global availability to `HbkBslContextCatalog`; retain generic `FactId` validation and DTO projection. |
| `QueryTableSnapshotSource::{resolve,global_context}` | Scans query table/field/parameter IDs and maps to `ContextFact`. | Delegate acquisition to `HbkSdblQueryCatalog`; retain generic `ContextFact` projection. |
| `QueryTableSnapshotSource::{query_fields,query_parameters,*_by_name}` | Converts generic table `FactId` to `HbkQueryTableId`, then maps records. | Use `HbkSdblQueryCatalog` for table/member acquisition; retain generic table ID validation. |
| `QueryTableSnapshotSource::related` | Uses private `HbkFactRef` relation lookup and maps target facts. | Keep private relation lookup in snapshot adapter until a typed relation catalog is specified; do not expose `HbkFactRef` or add a second relation API here. |
| `LanguageSearchSource` and `PlatformSearchSource` | Explicit SQL/SearchIndex adapters for local sequential, CLI/debug and index inspection. | Retain as explicit adapter family. Move shared selector literal mapping behind the SDBL catalog module so SQL and snapshot projections do not own separate selector tables. |
| Analyzer `analyze-project` hot path | Needs borrowed BSL/SDBL facts for context-provider simplification. | Consume catalogs or snapshot adapters that delegate to catalogs; no analyzer-side selector mapping or private HBK reads. |
| CLI/search/debug tools | Need artifact inspection and existing generic resolver behavior. | Continue constructing `SearchIndex`/SQL adapters explicitly; not migrated by this change. |

## Delegation And Deletion

The following duplicate behavior owners are removed or delegated:

- `context-resolver-search/src/mapping.rs::sdbl_metadata_source_selector` stops
  being a generic mapping owner. Its six literal rules move to
  one crate-private locale-aware function in `hbk_catalogs::sdbl`, returning
  `Option<&'static str>`. The catalog and retained SQL projection call that
  function with their HBK source locale.
- `QueryTableSnapshotSource::map_query_table` stops owning SDBL selector
  classification. It copies the selector from the SDBL catalog during DTO
  projection.
- `PlatformSnapshotSource::{resolve_type,members,callable,global_context,module_context,module_context_member,module_context_members}`
  stop owning BSL acquisition loops. They call BSL catalog methods and keep only
  `context-resolver-core` validation/projection.
- `QueryTableSnapshotSource::{resolve,global_context,query_fields,query_parameters,query_fields_by_name,query_parameters_by_name}`
  stop owning SDBL acquisition loops. They call SDBL catalog methods and keep
  only source/domain validation plus DTO projection.

The following behavior remains retained, explicitly named and out of the
catalog hot path:

- `LanguageSearchSource` and `PlatformSearchSource` keep SQL/SearchIndex
  adapter behavior for explicit CLI/debug/local sequential use.
- `PlatformSnapshotSource::map_platform_type`, `map_member`, `map_callable`,
  `map_type_refs` and corresponding SDBL DTO mappers remain compatibility
  projection helpers until a separate typed resolver-output change removes
  generic DTO projection.
- `QueryTableSnapshotSource::related` keeps private relation projection because
  typed borrowed relation catalogs are not part of this change.

## Snapshot Sufficiency

The existing snapshot is sufficient for the catalog scope:

- BSL type point/enumeration: `platform_type_by_id`, `platform_types_by_name`,
  `platform_types_by_template_key`, `HbkPlatformType`.
- BSL generated-self types: existing generated-self role to template-key
  mapping plus `platform_types_by_template_key`.
- BSL member/callable point and enumeration: `members_of_type`,
  `member_by_owner_name`, `member_by_owner_name_kind`, `callables_of_type`,
  `callable_by_owner_name`, `constructors_of_type`, `HbkTypeMember`,
  `HbkCallable`.
- BSL global methods/properties: `global_fact_ids`,
  `globals_by_domain_name_kind`, `HbkGlobalFact`, `HbkCallable`.
- BSL module events for accepted module context kinds: `module_context_events`,
  `module_event_by_context_name`, `HbkCallable`.
- BSL availability for platform type/member/callable/global facts:
  `availability_contexts` and `available_since` through private typed-ID to
  internal-fact conversion inside the BSL catalog.
- SDBL tables, fields and parameters: `query_table_ids`, `query_table_by_id`,
  name/syntax/identifier indexes, `query_fields`, `query_fields_by_name`,
  `query_parameters`, `query_parameters_by_name`, and existing `HbkQuery*`
  records.
- SDBL metadata source selector for the six accepted cases can be derived from
  existing `HbkQueryTable.identifier`.

There is no generated-self or module-context-event materialization gap for this
catalog change. If implementation discovers a lookup that cannot be expressed
over the existing snapshot records and indexes without a repeated full scan, the
implementation must stop. Adding any derived index requires an explicit spec
and Structure impact revision naming its measured need, key/value shape, cache
and memory-accounting impact, followed by a repeated skeptic/design review.
This accepted slice adds no index. Analyzer-side fallback readers remain
forbidden.

## Module Boundary

Implementation module layout:

- `crates/context-resolver-search/src/hbk_catalogs/mod.rs` declares the catalog
  module and re-exports `bsl::HbkBslContextCatalog` and
  `sdbl::HbkSdblQueryCatalog`.
- `hbk_catalogs/bsl.rs` owns BSL type/member/callable/global/module-event
  acquisition, generated-self type lookup and typed BSL availability methods
  over `Arc<HbkFactSnapshot>`.
- `hbk_catalogs/sdbl.rs` owns SDBL table/member acquisition and the six
  accepted `metadata.sdbl.query-source.*` selector literals.
- `snapshot_adapter.rs` imports the catalogs and contains only generic
  `ContextResolver` validation/projection logic for catalog-covered facts.
- `language_adapter.rs` may call the SDBL selector function for SQL projection,
  but must not own a second selector table.
- `context-resolver-core` owns metadata module-role selector to
  `ModuleContextKind` translation; make its helper public if direct catalog
  consumers need it.
- `syntax-helper-search` remains the only crate that owns arenas, source
  records, typed IDs, snapshot indexes, binary cache shape and memory
  accounting.

## Independent Implementation Slices

The implementation can be completed in independent slices:

1. Lifetime slice: change relevant `HbkFactReadHandle<'a>` lookup methods to
   take `self` by value and return iterators/slices tied to `'a`, proving the
   catalog API can return borrowed records without E0515/E0716 temporary-handle
   borrows.
2. SDBL slice: add `HbkSdblQueryCatalog`, move the six selector literals into
   it, delegate `QueryTableSnapshotSource` table/member acquisition to it, and
   add SDBL parity/selector tests.
3. BSL type/member slice: add `HbkBslContextCatalog` type, generated-self,
   member/callable and availability methods, then delegate
   `PlatformSnapshotSource::{resolve_type,members,callable,availability}`.
4. BSL global/module-context slice: delegate
   `PlatformSnapshotSource::{global_context,module_context,module_context_member,module_context_members}`
   to the BSL catalog and add point/enumeration parity tests.
5. Documentation slice: reconcile ADR-0008 or companion implementation notes
   after both catalogs and delegation are implemented.

Downstream analyzer work is complete only after both catalogs are implemented,
snapshot adapters delegate catalog-covered behavior, and analyzer hot paths can
consume the borrowed catalogs or delegating adapters without analyzer-side BSL
availability/module selector/SDBL selector mappings. Completing only the SDBL
or only the BSL slice is an upstream partial milestone, not downstream
completeness.

The upstream handoff gate was accepted on 2026-07-28. The exact typed APIs are
`HbkBslContextCatalog` (`072a65f`, evidence `ff70367`) and
`HbkSdblQueryCatalog` (`c140838`, evidence `b0d83ed`). Ordered differential
parity and upstream structural ownership guards are committed as `30fecd4`.
The downstream analyzer must own its own executable absence guard before task
4.4 is complete; this repository intentionally does not inspect a sibling
checkout from its test suite. No architecture document update is required for
this handoff batch because it verifies and records the catalog/generic-resolver
responsibilities already established in ADR-0008 and
`spec/implementation/components.md`; it adds no responsibility or dependency
direction.

## ADR-0008 Reconciliation

ADR-0008 remains accepted with a refined responsibility split:

- `ContextResolver` is the generic source-neutral composition and compatibility
  boundary.
- `HbkBslContextCatalog` and `HbkSdblQueryCatalog` are the domain-specific
  hot-path HBK boundary over `Arc<HbkFactSnapshot>`.
- Generic resolver DTO projection is a compatibility boundary, not HBK storage
  and not the primary hot-path model.
- `SearchIndex` is an explicit artifact/SQL source for CLI, debug and local
  sequential flows, not a hidden fallback for snapshot catalogs.

The ADR or companion implementation note must link this OpenSpec change when
the API is implemented.

## Risks/Trade-offs

- The catalog API adds a second public surface beside `ContextResolver`. The
  trade-off is a deeper domain API that avoids generic owned DTO materialization
  on analyzer hot paths.
- Returning existing snapshot records means consumers must understand snapshot
  IDs and `StringId` lookups. This is intentional: it avoids a parallel record
  model.
- The required `HbkFactReadHandle<'a>` lifetime signature change is mechanical
  but broad across lookup methods. It must preserve the existing handle as the
  only public read API and must not create a second borrowed-read facade.
- The accepted scope relies on existing snapshot indexes. Discovering a missing
  hot-path index stops implementation and requires an explicit OpenSpec,
  Structure impact and review revision before cache/memory accounting changes.
- SQL and snapshot adapters will both exist. Their shared catalog-covered
  semantics must be owned by catalog modules; SQL adapters remain explicit
  artifact readers.

## Alternatives Rejected

- Public `HbkFactRef` domain API: rejected because it exposes heterogeneous
  storage mechanics instead of typed domain records.
- Direct analyzer access to `HbkFactSnapshot::worker_handle`: rejected for
  analyzer hot paths because it would push availability, selector and parity
  rules downstream.
- One universal borrowed catalog trait: rejected because BSL availability/module
  context and SDBL query-source rules have different invariants and result
  shapes.
- Analyzer shim or private metadata provider read: rejected because HBK owns
  documented platform, BSL-language and query-language facts.
- Replacing `ContextResolver` immediately: rejected because existing
  source-neutral composition, platform adapter and compatibility consumers still
  need the generic DTO contract.
- Catalog-local cache/index/arena: rejected because storage and the existing
  indexes belong to `HbkFactSnapshot`; this scope adds no index.
- Treating generated-self as missing snapshot materialization: rejected because
  it is already derivable from generated-self role to platform-template key and
  `platform_types_by_template_key`.

## Structure Impact

Existing owners searched and retained: `syntax-helper-search::HbkFactSnapshot`
arenas and `HbkFactReadHandle` methods for storage and indexes;
`context-resolver-search::{PlatformSnapshotSource,QueryTableSnapshotSource}` for
generic snapshot adapter projection; `context-resolver-search::{PlatformSearchSource,LanguageSearchSource}`
for explicit SQL/SearchIndex adapter flows; `context-resolver-core` for generic
DTOs, `ContextResolver`, `ModuleContextKind` and metadata module-role selector
translation; existing snapshot IDs/records for BSL and SDBL facts.

Search evidence and representative shapes: `HbkFactRef`, `ContextFact`,
`Resolved*`, `QueryTableSnapshotSource`, `PlatformSnapshotSource`,
`LanguageSearchSource`, `sdbl_metadata_source_selector`,
`metadata.sdbl.query-source.*`, `metadata.module-role.*`,
`metadata.generated-self.*`, `template_key_for_generated_self_role`,
`platform_types_by_template_key`, `HbkQueryTable.identifier`,
`globals_by_domain_name_kind`, `module_context_events`,
`availability_contexts`, `available_since`, query table/field/parameter IDs and
records.

Added structures: exactly two public handle types, `HbkBslContextCatalog` and
`HbkSdblQueryCatalog`. They contain only `SourceId` values and
`Arc<HbkFactSnapshot>`.

Reused structures: all public method results use existing snapshot IDs and
borrowed snapshot records, plus borrowed slices and optional copied `StringId`
values already owned by the snapshot. Generic DTOs remain only at existing
`ContextResolver` projection boundaries.

Deleted/delegated behavior: duplicate snapshot adapter acquisition loops and
SDBL selector literal ownership are delegated to catalog modules as described
above. No new parser, loader, serializer, cache key, validation helper,
transport schema, DTO mirror, universal trait, analyzer mapping table or public
fact-reference wrapper is added.

## Reintroduction Guard

Root cause to prevent: analyzer hot-path consumers had to reach catalog-covered
HBK facts through generic owned DTO materialization or SQL/SearchIndex-backed
adapters, which encouraged duplicate selector mappings, fallback reads and
parallel resolver behavior.

Single allowed owners:

- `syntax-helper-search::HbkFactSnapshot` owns stored facts, typed IDs, borrowed
  records, existing indexes, binary cache shape and memory accounting.
- `HbkBslContextCatalog` owns BSL catalog-covered type/member/callable/global/
  module-event, generated-self and availability acquisition behavior.
- `HbkSdblQueryCatalog` owns SDBL table/member acquisition and the six accepted
  opaque `metadata.sdbl.query-source.*` selector literals.
- `context-resolver-core` owns generic resolver DTO contracts and
  `metadata.module-role.*` selector to `ModuleContextKind` translation.
- `SearchIndex`/SQL adapters own explicit artifact access for CLI/debug/local
  sequential flows, not snapshot hot-path fallback.

Verification must fail if a future change introduces any of these prohibited
shapes in catalog-covered paths:

- analyzer-side BSL availability, metadata module selector or SDBL selector
  mapping table;
- catalog-specific DTO mirror for `HbkPlatformType`, `HbkTypeMember`,
  `HbkCallable`, `HbkGlobalFact`, `HbkQueryTable`, `HbkQueryField` or
  `HbkQueryParameter`;
- public `HbkFactRef` catalog API;
- second public snapshot read API beside `HbkFactReadHandle`;
- catalog-local retained cache/index/arena;
- SQL/SearchIndex fallback inside `Hbk*Catalog` or snapshot adapters after
  delegation;
- second copy of the six `metadata.sdbl.query-source.*` literal mapping outside
  the SDBL catalog module;
- second copy of `metadata.module-role.*` selector to `ModuleContextKind`
  mapping outside `context-resolver-core`;
- universal borrowed catalog trait that makes BSL and SDBL invariants opaque.

Required tests for implementation: lifetime/borrow tests proving catalog
iterators can outlive temporary handles without E0515/E0716; catalog-vs-
snapshot-adapter parity for SDBL tables, fields and parameters; six exact SDBL
selector literals plus unknown identifier returning `None`; BSL type/member/
callable/global/module-event catalog parity including generated-self roles;
structural absence checks for public `HbkFactRef`, a second read API, SQL
fallback in borrowed catalogs and duplicate selector literals outside the SDBL
catalog module.

## Direct BSL Analyzer Handoff Follow-up

The first downstream analyzer migration exposed one incomplete portion of the
accepted BSL catalog contract: public availability methods still lend raw
`StringId` values, forcing a consumer to understand the private HBK
availability code protocol. `HbkBslContextCatalog` must instead decode those
values through the existing HBK mapping owner and return existing typed
`context_resolver_core::AvailabilityContext` values plus borrowed
available-since text. This changes no snapshot record, arena, index or
availability identity.

The same migration needs stable generic `FactId`, `HbkTypeRef -> TypeRef` and
`HbkSignature -> Signature` projection for the analyzer's already-existing
owned effective view. Those conversions currently live as private methods in
`snapshot_adapter.rs`. They are non-trivial HBK-to-core boundary behavior and
must gain one selected upstream public owner reused by both
`PlatformSnapshotSource` and the downstream concrete output boundary. Do not
add a projection struct, catalog wrapper, DTO family or second adapter module;
expose the narrow functions from the existing projection/mapping owner.
`HbkBslContextCatalog` itself continues to return only typed HBK IDs/records and
typed availability, never `ContextFact`, `Resolved*` or `HbkFactRef`.

The follow-up is one separately committed upstream stage before analyzer
integration. Required parity covers platform types, generated-self
members/callables, global properties/methods, module members/events,
availability contexts and available-since text. Structural guards reject raw
availability string IDs in the public catalog contract, a second mapping copy,
a projection holder/DTO, generic DTO exposure from the catalog and changes to
snapshot storage.
