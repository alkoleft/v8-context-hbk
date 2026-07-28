## Purpose

This inventory is the durable preimplementation evidence for tasks 1.1 and
1.2. It names the current behavior owners and real consumers before the
borrowed HBK catalogs are implemented.

## BSL Paths

| Consumer | Current owned/materialized path | Decision |
| --- | --- | --- |
| `PlatformSnapshotSource::resolve` | `facts_by_id` followed by generic `ContextFact` projection. | Delegate typed type/member/callable/global acquisition to `HbkBslContextCatalog`; retain source/domain/kind validation and projection. |
| `PlatformSnapshotSource::resolve_type` | Snapshot ID/name/template lookup, including `template_key_for_generated_self_role`, followed by `ResolvedType` projection. | Move typed acquisition and generated-self lookup behind the BSL catalog; retain generic response/status projection. |
| `PlatformSnapshotSource::{members,callable}` | Owner/name scans over snapshot indexes followed by `ResolvedMember`/`ResolvedCallable` materialization. | Delegate typed point/enumeration acquisition; retain query validation and DTO projection. |
| `PlatformSnapshotSource::global_context` | Scans every global and builds owned methods/properties. | Delegate global acquisition; generic `ResolvedGlobalContext` remains compatibility output only. |
| `PlatformSnapshotSource::{module_context,module_context_member,module_context_members}` | Repeats global and module-event acquisition and then builds owned module-context answers. | Compose catalog globals with `ModuleContextKind`-scoped events; retain generic answer projection. |
| `PlatformSnapshotSource::availability` | Resolves a generic `FactId` through private `HbkFactRef`, then copies availability into generic DTOs. | Keep generic ID validation in the adapter; use typed catalog availability methods without exposing `HbkFactRef`. |
| `PlatformSearchSource` | Explicit SQLite/`SearchIndex` equivalents of platform resolution and context formation. | Retain for CLI/debug/index inspection and local sequential flows; never use as snapshot fallback. |

The existing single behavior helpers are retained, not copied:

- `template_key_for_generated_self_role` is the generated-self selector to
  platform-template mapping shared by the catalog and retained SQL adapter.
- `context-resolver-core::metadata_module_context_kind` is the sole raw
  `metadata.module-role.*` to `ModuleContextKind` translation.
- `search_module_context_relation_key` is moved behind the BSL catalog when
  module-context acquisition migrates; the SQL adapter calls the same
  crate-private behavior.

## SDBL Paths

| Consumer | Current owned/materialized path | Decision |
| --- | --- | --- |
| `QueryTableSnapshotSource::resolve` | `facts_by_id` followed by query table/field/parameter `ContextFact` projection. | Delegate typed point acquisition to `HbkSdblQueryCatalog`; retain generic ID validation and projection. |
| `QueryTableSnapshotSource::global_context` | Enumerates all table/field/parameter IDs into one owned `Vec<ContextFact>`. | Delegate borrowed table/member traversal; keep flattening only at the generic compatibility boundary. |
| `QueryTableSnapshotSource::{query_fields,query_fields_by_name,query_parameters,query_parameters_by_name}` | Repeats owner-scoped snapshot lookup and generic mapping. | Delegate typed lookup/enumeration; retain generic owner `FactId` validation and DTO projection. |
| `QueryTableSnapshotSource::related` | Uses private heterogeneous `HbkFactRef` relations and projects targets. | Retain unchanged until a typed relation catalog is specified; do not expose `HbkFactRef`. |
| `LanguageSearchSource` | Explicit SQLite/`SearchIndex` query-language adapter. | Retain for CLI/debug/index inspection and local sequential flows; reuse the one locale-aware selector function without becoming snapshot fallback. |
| `mapping.rs::sdbl_metadata_source_selector` | Owns six Russian identifier-to-selector literals and allocates `String`. | Move the literals to one crate-private locale-aware function in `hbk_catalogs::sdbl`; catalog returns `&'static str`, generic adapters allocate only at DTO projection. |

## Generic Resolver Consumers Retained

| Consumer | Why the generic resolver remains |
| --- | --- |
| `v8-context/crates/analyze-project/src/type_resolution` | Source-neutral platform type/member resolution, ambiguity/status handling and generic availability DTOs remain its contract. |
| `v8-context/crates/platform-adapter/src/{query_api,platform_references}.rs` | Cross-provider platform reference and query-role composition consume generic resolver identities and responses. |
| `v8-context/crates/analyze-project/src/platform_import.rs` | Imports generic platform facts and availability into analyzer-owned facts. |
| Analyzer CLI/validator composition | Constructs and passes provider/resolver state; it does not own HBK mappings. |
| `CompositeResolver` and `WorkerSafeCompositeResolver` | Preserve deterministic source-neutral composition and ambiguity reporting. |

## Analyzer Hot Paths To Migrate

| Consumer | Current generic hot path | Catalog handoff |
| --- | --- | --- |
| `v8-context/crates/context-provider/src/bsl` | `global_context`, generated-self `resolve_type`, owner `members`/`callable`, `metadata_module_member(s)` and `availability` repeatedly materialize generic answers during point/enumeration context formation. | Consume `HbkBslContextCatalog` through the provider registration; no analyzer selector map or private snapshot read. |
| `v8-context/crates/context-provider/src/sdbl` | Calls generic `global_context(Sdbl)` and scans a flattened table/field/parameter fact vector for each query-bound operation. | Consume `HbkSdblQueryCatalog` and traverse only the resolved table plus owner fields/parameters. |

Runtime/LSP/web UI layers consume analyzer/runtime facts and answers rather than
composing HBK providers. They are not catalog consumers and require no DTO or
transport change. `handler-entry-points` has no production HBK resolver
consumer. HBK CLI/search commands intentionally use `SearchIndex` directly.

## Searched Surfaces

- HBK Rust crates: `context-resolver-core`, `context-resolver-search`,
  `syntax-helper-search`, CLI and tests.
- Analyzer Rust crates: `context-provider`, `analyze-project`,
  `platform-adapter`, `handler-entry-points`, runtime/LSP/web/CLI boundaries.
- OpenSpec and ADR evidence for generated-self, metadata module lookup,
  query-table member enumeration and worker-safe snapshots.
- No frontend, schema, generator or serialized transport consumer owns or
  mirrors the catalog-covered HBK structures.

## Reintroduction Evidence

The implementation review must reject:

- a second snapshot/read facade or catalog-local arena/index/cache;
- analyzer imports of `HbkFactSnapshot`, `HbkFactReadHandle` or `HbkFactRef`;
- analyzer or adapter copies of generated-self, module-role or SDBL selector
  mappings;
- retained snapshot acquisition loops beside catalog acquisition;
- SQL/`SearchIndex` fallback from a snapshot catalog path;
- a stored flattened SDBL `Vec<ContextFact>` outside generic resolver answer
  projection.
