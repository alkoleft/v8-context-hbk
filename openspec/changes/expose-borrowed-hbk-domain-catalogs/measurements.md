## Measurement Contract

This file is durable OpenSpec evidence. It separates reproducible structural
baseline counts from runtime counters that cannot be claimed before catalog
probes exist.

## Preimplementation Structural Baseline

The following commands are run against the commit immediately before catalog
implementation:

```text
rg -n -F 'worker_handle()' crates/context-resolver-search/src/snapshot_adapter.rs
rg -n -F 'collect::<Vec' crates/context-resolver-search/src/snapshot_adapter.rs
rg -n -F 'ContextFact {' crates/context-resolver-search/src/snapshot_adapter.rs
rg -n 'Resolved[A-Za-z]+' crates/context-resolver-search/src/snapshot_adapter.rs
rg -n -F '.global_context(' crates/context-resolver-search/src/snapshot_adapter.rs
```

Exact counts and focused test wall times are recorded below before the lifetime
implementation edit.

Snapshot adapter structural counts:

| Literal/regex | Count |
| --- | ---: |
| `worker_handle()` | 20 |
| `collect::<Vec` | 11 |
| `ContextFact {` | 24 |
| `Resolved[A-Za-z]+` | 30 |
| `.global_context(` | 1 |

Retained explicit search-adapter structural counts:

| File/literal | Count |
| --- | ---: |
| `platform_context_source.rs`: `search index` | 3 |
| `language_adapter.rs`: `SearchIndex` | 8 |
| `language_adapter.rs`: `open_read_only` | 6 |

Focused warm-build baseline:

| Existing observable contract | Command | Wall time |
| --- | --- | ---: |
| Snapshot hot paths without a search-index backend | `cargo test -p context-resolver-search tests::platform_snapshot_source_resolves_hot_paths_without_search_index_backend -- --exact` | 0.10 s |
| BSL global context and ownerless global callable | `cargo test -p context-resolver-search tests::platform_adapter_exposes_bsl_global_context_and_ownerless_global_callable -- --exact` | 0.11 s |
| Provider-backed module context events | `cargo test -p context-resolver-search tests::platform_adapter_exposes_provider_backed_module_context_events -- --exact` | 0.11 s |
| Module member enumeration | `cargo test -p context-resolver-search tests::platform_adapters_enumerate_module_members_without_context_snapshot_filtering -- --exact` | 0.11 s |
| Query table templates, fields, parameters and type refs | `cargo test -p context-resolver-search tests::query_table_snapshot_source_exposes_templates_fields_parameters_and_type_refs -- --exact` | 0.10 s |
| Separate BSL/SDBL global contexts | `cargo test -p context-resolver-search tests::language_adapter_exposes_bsl_and_sdbl_global_contexts_separately -- --exact` | 0.11 s |

All six commands passed with one selected test and zero failures. Times were
captured with `/usr/bin/time -f 'elapsed=%e'`; they are controlled
warm-compilation smoke baselines, not microbenchmarks.

## Runtime Counters Still Required

Task 1.6 remains open after the lifetime foundation. The BSL and SDBL catalog
batches must add or reuse controlled probes that distinguish:

- owned generic DTO materializations;
- `ContextResolver::global_context` calls;
- `ContextFact` and `Resolved*` projections;
- `HbkFactSnapshot::worker_handle` reads;
- SQL/`SearchIndex` calls reached from snapshot paths.

Static source occurrences are not substituted for those runtime counters.
After-measurements must use the same fixtures and probe definitions as their
preimplementation values.

## First BSL Catalog Slice After Evidence

The first BSL catalog slice was measured after introducing
`HbkBslContextCatalog` and delegating catalog-covered
`PlatformSnapshotSource` behavior to it.

Snapshot adapter structural counts after the BSL slice:

| Literal/regex | Count |
| --- | ---: |
| `worker_handle()` in `PlatformSnapshotSource` section | 12 |
| `worker_handle()` in retained `QueryTableSnapshotSource` section | 8 |
| `worker_handle()` in `HbkBslContextCatalog` | 20 |
| `collect::<Vec` in `snapshot_adapter.rs` | 13 |
| `ContextFact {` in `snapshot_adapter.rs` | 24 |
| `Resolved[A-Za-z]+` in `snapshot_adapter.rs` | 30 |
| `.global_context(` in `snapshot_adapter.rs` | 1 |

The adapter still contains `worker_handle()` calls for enum-as-type,
availability/relation projection and the not-yet-migrated SDBL slice. The
catalog-owned BSL traversals for global BSL properties/methods, generated-self
type lookup, owner members/callables, module-context events and typed
availability are centralized in `HbkBslContextCatalog`; generic DTO
materialization remains only in `PlatformSnapshotSource` projection helpers.

Focused warm-build after measurements:

| Observable contract | Command | Wall time |
| --- | --- | ---: |
| Direct BSL borrowed catalog with deleted SQLite file | `cargo test -p context-resolver-search bsl_catalog -- --nocapture` | 0.15 s |
| Snapshot hot paths without a search-index backend | `cargo test -p context-resolver-search tests::platform_snapshot_source_resolves_hot_paths_without_search_index_backend -- --exact` | 0.10 s |
| BSL global context and ownerless global callable | `cargo test -p context-resolver-search tests::platform_adapter_exposes_bsl_global_context_and_ownerless_global_callable -- --exact` | 0.11 s |
| Provider-backed module context events | `cargo test -p context-resolver-search tests::platform_adapter_exposes_provider_backed_module_context_events -- --exact` | 0.11 s |
| Module member enumeration | `cargo test -p context-resolver-search tests::platform_adapters_enumerate_module_members_without_context_snapshot_filtering -- --exact` | 0.11 s |

All five commands passed. These measurements prove the BSL snapshot path does
not require the SQLite file and retains the baseline smoke-test wall time. They
do not close task 1.6's broader runtime probe requirement for distinguishing
owned DTO projection counts and read-handle calls across both BSL and SDBL.
