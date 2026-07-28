## 1. Contract, Consumer Inventory, And Guards

- [x] 1.1 Inventory every current BSL catalog-covered consumer/path before implementation: `PlatformSnapshotSource::{global_context,module_context,module_context_member,module_context_members,availability,callable,resolve}`, corresponding `PlatformSearchSource` SQL/SearchIndex methods, `ContextResolver`/`ContextSource` composite consumers, downstream analyzer consumers that currently materialize generic `ContextFact`/`Resolved*` DTOs, tests/fixtures, and ADR-0008 references.
- [x] 1.2 Inventory every current SDBL catalog-covered consumer/path before implementation: `QueryTableSnapshotSource::{global_context,resolve,related,query_fields,query_fields_by_name,query_parameters,query_parameters_by_name}`, `LanguageSearchSource::{query_tables,open_query_tables_read_only,new_query_tables,global_context,resolve,related}`, exact `sdbl_metadata_source_selector` behavior, generic resolver consumers, SQL/SearchIndex CLI/debug/index-inspection flows, tests/fixtures, and downstream analyzer handoff consumers.
- [x] 1.3 Reconcile ADR-0008 and implementation docs before implementation starts, explicitly naming borrowed BSL/SDBL catalogs as hot-path snapshot APIs, `ContextResolver`/generic DTO projection as a retained compatibility/composition API, and `PlatformSearchSource`/`LanguageSearchSource` as explicit SQL/SearchIndex flows retained for CLI/debug/index inspection and local sequential resolver use.
- [x] 1.4 Record the task-local Structure impact note before any implementation edit: searched owners, representative field/result shapes, inputs/outputs, real consumers, exact first BSL slice, exact first SDBL slice, structures/behaviors/conversions reused/added/deleted, and why no new arena, index, cache, DTO mirror, universal trait, public `HbkFactRef` API, SQL fallback, analyzer shim, selector enum/wrapper, or generic resolver replacement is introduced.
- [x] 1.5 Record the Reintroduction guard before implementation: `HbkFactSnapshot` remains the only storage owner; borrowed BSL/SDBL catalogs own catalog-covered snapshot behavior; generic resolver adapters delegate and project once; `PlatformSearchSource`/`LanguageSearchSource` SQL paths are explicit and non-fallback; verification must fail on duplicate flattened facts, hidden SQLite/SearchIndex fallback, analyzer selector mapping, selector enum/wrapper mirror, universal catalog trait, public `HbkFactRef`, or new parallel cache/index.
- [x] 1.6 Capture preimplementation counters and wall time for the first BSL and SDBL slices: owned generic DTO materializations, `ContextResolver::global_context` calls, `ContextFact`/`Resolved*` projections, `HbkFactSnapshot::worker_handle` reads, SQL/SearchIndex calls in snapshot paths, and controlled wall time for existing parity fixtures.
- [x] 1.7 Inventory and behavior-test the existing `syntax-helper-search` snapshot read-handle lifetime contract before adding catalogs: only `facts_by_id`, `platform_types_by_name`, `platform_types_by_template_key`, `members_of_type`, `member_by_owner_name`, `member_by_owner_name_kind`, `callables_of_type`, `callable_by_owner_name`, `constructors_of_type`, `global_fact_ids`, `globals_by_name`, `globals_by_domain_name_kind`, `module_event_by_context_name`, `module_context_events`, `query_table_ids`, `query_tables_by_name`, `query_tables_by_syntax`, `query_tables_by_identifier`, `query_fields`, `query_fields_by_name`, `query_parameters`, `query_parameters_by_name`, `availability_contexts` and `available_since` may consume the `Copy` `HbkFactReadHandle<'a>` and return borrowed iterators/slices tied to lifetime `'a`; add RED compile/runtime contract tests for representative BSL iterator and SDBL slice paths proving catalog construction can borrow through those existing methods without collecting or adding a new read API.

Completion evidence (2026-07-28): `inventory.md` records the BSL/SDBL,
generic-resolver, SQL/SearchIndex and downstream consumer paths; ADR-0008 records
the refined catalog boundary; `measurements.md` records the reproducible static
and wall-time baseline while task 1.6 remains open for runtime probes. The
approved lifetime batch added no semantic structure or alternate read API.
`cargo test -p syntax-helper-search` passed 68 tests,
`cargo test -p context-resolver-search` passed 32 unit tests plus the consumer
smoke test, formatting and strict OpenSpec validation passed. A warnings-as-
errors clippy run is still blocked by pre-existing
`snapshot/memory.rs::vec_payload_bytes(&Vec<T>)` and pre-existing redundant
`.into_iter()` calls in syntax-helper-search tests; the batch introduced no new
clippy diagnostic after its test helper lifetime was elided.

Task 1.6 final evidence (2026-07-28): `measurements.md` now records identical
baseline/current helper workloads, repeat-stable helper-scoped heap allocation
calls, GDB-bracketed `worker_handle`, explicit projection,
`global_context` and `SearchIndex` method-entry counters, one warmup plus five
wall-time runs, exact tool/profile/binary hashes and limitations for both BSL
and SDBL. The measurement-only GDB script is commit `84e49ab`; the tracked
`artifacts/runtime-counters/` tree retains 177 raw payload files plus a
SHA-256 manifest, so every reported counter can be recomputed independently.
SearchIndex entries inside every compat/direct helper were zero; the document
deliberately keeps that distinct from uninstrumented rusqlite/SQL calls and
retains deleted-SQLite/no-handle evidence for the latter.

Prohibited scope for every batch: do not add new arenas, indexes, caches, DTO mirrors, universal traits, public `HbkFactRef` APIs, SQL/SearchIndex fallbacks, analyzer-side shims, selector enum/wrapper mirrors, private provider reads, or generic resolver replacements.

## 2. First BSL Catalog Slice

- [x] 2.1 Add RED tests for the exact first BSL slice over `PlatformSnapshotSource`: borrowed catalog construction from existing `Arc<HbkFactSnapshot>`/`HbkFactReadHandle<'a>`, `Send + Sync`, no unavailable-SQLite fallback after deleting the source SQLite file, `global_context(Bsl)` parity for global methods/properties, generated-self type lookup via template key, owner member/callable point+enumeration parity, `module_context(Form)` plus `module_context_member(s)` parity for metadata module members/events, and typed availability for type/member/callable/global records.
- [x] 2.2 Expose the first borrowed BSL catalog API over existing snapshot/read-handle records only, covering global BSL methods/properties, generated-self platform type lookup by existing template key, generated-self owner members/callables, metadata module context members by `ModuleContextKind`, platform module context events, and typed availability; reuse existing snapshot arenas and typed source identities, and do not materialize generic `ContextFact`/`Resolved*` DTOs inside the borrowed catalog.
- [x] 2.3 Keep raw `metadata.module-role.*` selector translation owned by `context-resolver-core`; expose only a narrow public helper from that owner if catalog code needs translation, and add a guard rejecting duplicate catalog-side or analyzer-side selector maps.
- [x] 2.4 Move typed availability and `ModuleContextKind`-scoped HBK module-context key/event access used by this first slice into the BSL catalog behavior owner while retaining raw module-role translation in the `context-resolver-core` owner from 2.3; keep source identity, locale and typed record identities available as provenance inputs and avoid analyzer-side selector tables or SQL fallback.
- [x] 2.5 Make `PlatformSnapshotSource::{resolve,resolve_type,members,callable,global_context,module_context,module_context_member,module_context_members,availability}` delegate catalog-covered behavior to the borrowed BSL catalog and project to `context-resolver-core` DTOs only at the `ContextResolver` compatibility boundary.
- [x] 2.6 Delete duplicate BSL snapshot adapter behavior made obsolete by the first catalog slice, specifically direct loops/mapping branches in `PlatformSnapshotSource` that re-own catalog-covered global/module-event traversal, generated-self template lookup, owner member/callable traversal, module-context member/event traversal and typed availability; retain only projection helpers needed for generic DTO output, and retain the single raw module-role selector translation in `context-resolver-core`.
- [x] 2.7 Preserve `PlatformSearchSource::{resolve,resolve_type,members,callable,global_context,module_context,module_context_member,module_context_members,availability}` as explicit SQL/SearchIndex behavior for non-migrated CLI/debug/index-inspection and local sequential resolver flows; add tests proving snapshot catalog paths do not call or require these SQL flows.
- [x] 2.8 Verify first BSL slice independently with `cargo test -p syntax-helper-search read_handle`, `cargo test -p context-resolver-search bsl_catalog -- --nocapture`, `cargo test -p context-resolver-search tests::platform_snapshot_source_resolves_hot_paths_without_search_index_backend -- --exact`, `cargo test -p context-resolver-search tests::platform_adapter_exposes_bsl_global_context_and_ownerless_global_callable -- --exact`, `cargo test -p context-resolver-search tests::platform_adapter_exposes_provider_backed_module_context_events -- --exact`, and `cargo test -p context-resolver-search tests::platform_adapters_enumerate_module_members_without_context_snapshot_filtering -- --exact`.
- [x] 2.9 Run the identical compatibility-adapter probe at the preimplementation commit and after the BSL slice; compare observable returned owned DTO/projection counts, `global_context` invocations and wall time, record deleted-SQLite/no-`SearchIndex`-handle evidence without mislabelling it as an instrumented SQL-call counter, and report the direct borrowed catalog sequence separately from the baseline comparison.
- [x] 2.10 Run a fresh review for the BSL slice, update ADR/docs/tasks with accepted evidence, then commit the independent BSL batch before starting SDBL implementation.

Completion evidence (2026-07-28): `HbkBslContextCatalog` is the only new public
BSL semantic handle and returns existing `syntax-helper-search` IDs/records and
borrowed strings from the shared `Arc<HbkFactSnapshot>` storage owner. It owns
catalog-covered BSL traversal for global methods/properties, generated-self
template lookup, owner member/callable lookup and enumeration, module-context
events and typed availability. `PlatformSnapshotSource` now stores only the
catalog handle for BSL snapshot access, delegates catalog-covered hot paths to
it and keeps generic `ContextFact`/`Resolved*` projection at the
`ContextSource` compatibility boundary. The retained direct `worker_handle()`
uses in `PlatformSnapshotSource` are enum/relation/projection glue outside the
first BSL catalog slice; the retained SQL/SearchIndex adapters stay explicit in
`PlatformSearchSource`. The direct catalog no-fallback test deletes the source
SQLite file after snapshot construction and still resolves BSL records, events
and availability. `measurements.md` records structural after-counts, focused
wall times and an identical `b0841e6`/after compatibility probe. Every
observable returned DTO/projection count stayed equal, warm wall time remained
0.10 s and the after-only direct sequence returned borrowed IDs/records without
collecting them into owned vectors. The broader task 1.6 remains open for SDBL
and for any allocator-level, `worker_handle`-invocation or instrumented
SQL-call claims. Fresh implementation and evidence reviews completed with no
findings; `spec/implementation/components.md`, ADR-0008 and active T178 record
the implemented BSL boundary and the still-open SDBL slice.

## 3. First SDBL Catalog Slice

- [x] 3.1 Add RED tests for the exact first SDBL slice over `QueryTableSnapshotSource`: borrowed catalog construction from existing `Arc<HbkFactSnapshot>`, `Send + Sync`, no unavailable-SQLite fallback after deleting the source SQLite file, table point/enumeration parity, owner-scoped field/parameter parity, six exact opaque selectors, and unknown identifier returning `None`.
- [x] 3.2 Expose the first borrowed SDBL catalog API over existing snapshot/read-handle records only, covering query table point/enumeration, owner fields, owner parameters, query identifiers, syntax, type references and provenance; do not build a flattened `Vec<ContextFact>`, second arena, duplicate index or result DTO mirror.
- [x] 3.3 Move exact `sdbl_metadata_source_selector` behavior into one crate-private locale-aware function in `hbk_catalogs::sdbl`, reused by `HbkSdblQueryCatalog` and the retained SQL adapter, for only these six opaque values: `metadata.sdbl.query-source.catalog`, `metadata.sdbl.query-source.document`, `metadata.sdbl.query-source.information-register`, `metadata.sdbl.query-source.accumulation-register`, `metadata.sdbl.query-source.accounting-register`, and `metadata.sdbl.query-source.calculation-register`; the mapping applies only to `source_locale=ru`, and another locale or unknown identifier returns normal `None`, not an analyzer unknown reason.
- [x] 3.4 Make `QueryTableSnapshotSource::{global_context,resolve,related,query_fields,query_fields_by_name,query_parameters,query_parameters_by_name}` delegate catalog-covered behavior to the borrowed SDBL catalog and project to `context-resolver-core` DTOs only at the `ContextResolver` compatibility boundary.
- [x] 3.5 Delete duplicate SDBL flattening and selector mapping made obsolete by the catalog, specifically direct `QueryTableSnapshotSource` table/field/parameter enumeration and local selector branches that re-own catalog-covered behavior; retain only generic DTO projection helpers and non-catalog resolver glue.
- [x] 3.6 Preserve `LanguageSearchSource::{query_tables,open_query_tables_read_only,new_query_tables,global_context,resolve,related}` as explicit SQL/SearchIndex behavior for non-migrated CLI/debug/index-inspection and local sequential resolver flows; add tests proving snapshot catalog paths do not call or require these SQL flows.
- [x] 3.7 Verify first SDBL slice independently with `cargo test -p context-resolver-search sdbl_catalog -- --nocapture`, `cargo test -p context-resolver-search tests::query_table_snapshot_source_exposes_templates_fields_parameters_and_type_refs -- --exact`, and `cargo test -p context-resolver-search tests::language_adapter_exposes_bsl_and_sdbl_global_contexts_separately -- --exact`.
- [x] 3.8 Run an identical pre-SDBL/after compatibility probe and compare observable returned generic DTO/projection counts, one explicit `global_context` invocation, returned flattened fact counts, selector-bearing query-table projections, wall time and RSS. Record deleted-SQLite/no-required-`SearchIndex` evidence without reporting it as an instrumented SQL-call counter, and report the after-only direct borrowed catalog sequence separately. Keep task 1.6 open for allocator-level materialization, exact `worker_handle` invocation and instrumented SQL-call counters that this bounded probe does not provide.
- [x] 3.9 Run a fresh review for the SDBL slice, update ADR/docs/tasks with accepted evidence, then commit the independent SDBL batch before downstream analyzer handoff.

Implementation and measurement evidence (2026-07-28): commit `c140838`
introduced the only public SDBL semantic handle, `HbkSdblQueryCatalog`, over
the existing snapshot arenas. `QueryTableSnapshotSource` stores only that
catalog, delegates table/field/parameter acquisition and keeps generic DTO
projection plus private relation glue at the compatibility boundary. The six
opaque selector literals have one production owner in `hbk_catalogs::sdbl`;
the snapshot catalog and retained SQL adapter share its locale-aware behavior.
Direct tests cover borrowed lifetimes, `Send + Sync`, table/member parity,
Russian/unknown/non-Russian selectors and operation after deleting SQLite.
The identical `ff70367`/after compatibility probe preserved all observable
counts, 0.11 s warm wall time and changed command RSS from 39,524 to
39,400 KiB. `measurements.md` records the exact commands and after-only direct
counts. Task 1.6 remains open for the explicitly uninstrumented counter classes;
the fresh implementation re-review and final evidence review both completed
with no findings.

## 4. Generic Resolver And Downstream Handoff

- [x] 4.0 Do not start downstream `v8-context` handoff, analyzer integration notes or analyzer absence guards until both independent gates are complete: BSL task 2.10 is reviewed/committed and SDBL task 3.9 is reviewed/committed.
- [x] 4.1 Name and retain generic resolver consumers that still require source-neutral composition or generic DTO contracts, including `ContextResolver`, `ContextSource`, `CompositeResolver`, `WorkerSafeCompositeResolver`, analyzer platform-adapter composition, and SQL/SearchIndex-backed CLI/debug/index-inspection flows.
- [x] 4.2 Add migration/parity tests proving `PlatformSnapshotSource` and `QueryTableSnapshotSource` generic `ContextResolver` results match the borrowed catalogs for catalog-covered facts while allocating generic DTOs only at the compatibility boundary.
- [x] 4.3 Add downstream handoff notes for `v8-context`: analyzer hot paths should consume borrowed BSL/SDBL catalogs where available, must not add analyzer-local HBK shims or selector mappings, and may keep generic resolver use only for the retained source-neutral consumers named in 4.1.
- [x] 4.4 Add absence guards in tests or review scripts for analyzer-facing contracts: no `HbkFactRef` public domain API, no catalog-covered analyzer selector mapping, no flattened `Vec<ContextFact>` hot-path store, no SQL/SearchIndex fallback in snapshot-backed catalog/resolver adapter paths, and no universal catalog trait.

Upstream handoff evidence (2026-07-28): the independent BSL API/evidence
commits are `072a65f`/`ff70367`; the independent SDBL API/evidence commits are
`c140838`/`b0d83ed`. Commit `30fecd4` adds ordered differential parity for the
downstream handoff fact families: BSL generated self, owner members/callables,
globals, form events and availability, plus SDBL tables, owner
fields/parameters and selectors. It also adds an executable upstream ownership
guard. The guard proves that the catalog structs remain typed snapshot owners,
generic DTO projection remains in `snapshot_adapter.rs`, neither catalog
exports `HbkFactRef`, snapshot resolver structs retain only their catalog
handles, and the six SDBL selectors have one production owner. `inventory.md`
names the retained source-neutral generic consumers and gives the downstream
handoff contract. Task 4.4 deliberately remains open until the downstream
`v8-context` repository contains and verifies its own executable absence guard;
an upstream source scan is not a substitute for that consumer-owned check.
Downstream `v8-context` commit `5e599dd` supplies the executable guard and
seeded negative controls; `0418bff` accepts the exact upstream APIs and commits
in the downstream OpenSpec while keeping transient SDBL materialization open
under its tasks 6.2/6.6.

## 5. Full Verification And Completion

- [x] 5.1 Run `openspec validate expose-borrowed-hbk-domain-catalogs --strict` before and after each implementation batch.
- [x] 5.2 Run `cargo fmt --all --check`.
- [x] 5.3 Run `cargo test -p context-resolver-search` including focused BSL/SDBL catalog, parity, unavailable-SQLite/no-fallback, selectors+unknown and `Send + Sync` tests.
- [x] 5.4 Run `cargo test -p syntax-helper-search` when snapshot/read-handle APIs or arena exposure change.
- [x] 5.5 Run `cargo test --workspace` after the BSL and SDBL commits are both integrated.
- [x] 5.6 Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` or record existing unrelated blockers with exact diagnostics.
- [x] 5.7 Reconcile final diff against the Structure impact and Reintroduction guard, accounting for every structure, behavior owner, conversion, mapping, adapter, public re-export, test fixture and documentation change.
- [x] 5.8 Update ADR-0008 or companion implementation notes, spec references, measurements/counter notes and versioning as required by the accepted behavior and completed commits.
- [x] 5.9 Run a fresh final review covering SKEP-003/008/009/010/011, independent BSL/SDBL commit boundaries, generic resolver retention, SQL/SearchIndex non-fallback, selector ownership and downstream handoff.
- [x] 5.10 Commit the final documentation/verification batch after staged files are limited to this OpenSpec change and required docs/versioning updates.

Verification and reconciliation evidence (2026-07-28):

- Strict OpenSpec validation passed before the final version/test batch and
  after every implementation/evidence batch. `cargo fmt --all --check`,
  `cargo test -p context-resolver-search` (38 passed, 2 ignored, plus the
  static-analysis consumer smoke test), `cargo test -p syntax-helper-search`
  (68 passed) and the post-version, post-lint-fix `cargo test --workspace`
  all passed. The real `shcntx` extraction test completed in 85.82 seconds.
- The exact full clippy command was
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  it exited `101` only on three diagnostics already present at base commit
  `37b1b43`: `crates/syntax-helper-extract/src/html.rs:142`
  `clippy::needless_borrow`,
  `crates/syntax-helper-extract/src/error.rs:67`
  `clippy::io_other_error`, and
  `crates/syntax-helper-search/src/snapshot/memory.rs:450`
  `clippy::ptr_arg`. The 15 `clippy::useless_conversion` findings caused by
  this change's iterator-returning read APIs were fixed in the affected tests,
  not misclassified as unrelated cleanup; a repeat full clippy run contained
  none of them.
- The final `37b1b43..824015c` structure review accounts for the only two new
  semantic handles, `HbkBslContextCatalog` and `HbkSdblQueryCatalog`, over the
  existing `HbkFactSnapshot` storage owner and existing typed IDs/records. It
  accounts for the borrowed read-lifetime refinement, deleted duplicate
  snapshot traversals and selector mapping, retained generic projection
  helpers/adapters, the single SDBL selector owner, BSL/SDBL parity and
  source-absence guards, differential measurement fixtures, raw runtime
  evidence, downstream guard/handoff, and the `0.2.0` workspace release.
  No second arena/index/cache, DTO or enum mirror, universal trait, public
  `HbkFactRef`, analyzer shim, fallback reader, duplicate conversion chain,
  alternate public re-export surface or SQL/SearchIndex fallback was added.
- Codebase-design reconciliation found one typed catalog module with useful
  domain interfaces and explicit generic compatibility adapters; it introduced
  no shallow pass-through layer or new dependency edge. ADR-0008 and
  `spec/implementation/components.md` already record the changed API ownership
  and compatibility boundary. This HBK repository has no separate
  `docs/architecture` tree, so creating one solely for completion bookkeeping
  would duplicate those existing architecture owners.
- The fresh final review covered the complete production, test and
  documentation diff plus the independent commit boundaries and downstream
  handoff. Its only finding was a stale raw-artifact label/hash after
  distinguishing `current-bsl` from `current-sdbl`; the command labels,
  manifest and measurement summary were reconciled, all 177 payload hashes
  passed, strict validation passed, and the follow-up review returned
  `NO FINDINGS`. Commit `35ea001` is the staged-file-limited final
  documentation/verification batch required by task 5.10.
