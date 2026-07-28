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

## Runtime Counter Requirement

Task 1.6 remained open after the lifetime foundation. The BSL and SDBL catalog
batches had to add or reuse controlled probes that distinguish:

- owned generic DTO materializations;
- `ContextResolver::global_context` calls;
- `ContextFact` and `Resolved*` projections;
- `HbkFactSnapshot::worker_handle` reads;
- SQL/`SearchIndex` calls reached from snapshot paths.

Static source occurrences are not substituted for those runtime counters.
The final runtime-counter section below uses the same fixture/helper bodies
before and after each catalog slice and completes this requirement.

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

### Controlled BSL compatibility probe

The durable baseline patch is
`artifacts/bsl-compat-baseline.patch`. It applies to commit `b0841e6` and adds
the exact same `compat_adapter_sequence` helper used by the current ignored
test. A normalized source comparison was not needed: direct `diff` of the two
helper bodies returned no differences. Both runs use
`bsl-catalog-measurement-probe.sqlite`, remove that SQLite file after building
the snapshot and then construct `PlatformSnapshotSource` from the shared
`Arc<HbkFactSnapshot>`.

This is an intentionally duplicated differential-measurement helper, not a
second production implementation: the patch exists only to run the unchanged
public compatibility workload against the fixed historical commit.

Baseline:

```text
git worktree add --detach <temporary-worktree> b0841e6
git apply <catalog-change-worktree>/openspec/changes/expose-borrowed-hbk-domain-catalogs/artifacts/bsl-compat-baseline.patch
/usr/bin/time -f 'baseline_elapsed_seconds=%e baseline_max_rss_kib=%M' \
  cargo test -p context-resolver-search bsl_compat_measurement_probe \
  -- --ignored --nocapture
```

After:

```text
/usr/bin/time -f 'after_elapsed_seconds=%e after_max_rss_kib=%M' \
  cargo test -p context-resolver-search bsl_catalog_measurement_probe \
  -- --ignored --nocapture
```

The compatibility counts are observable returned owned DTO/projection values,
not allocator-level construction counters:

| Compatibility metric | `b0841e6` | BSL catalog slice |
| --- | ---: | ---: |
| `compat_deleted_sqlite_success` | 1 | 1 |
| `compat_global_context_invocations` | 1 | 1 |
| `compat_global_context_responses` | 1 | 1 |
| `compat_global_methods` | 3 | 3 |
| `compat_global_properties` | 1 | 1 |
| `compat_generated_self_responses` | 1 | 1 |
| `compat_exact_member_responses` | 1 | 1 |
| `compat_member_enum_responses` | 1 | 1 |
| `compat_callable_responses` | 1 | 1 |
| `compat_module_context_responses` | 1 | 1 |
| `compat_module_context_methods` | 3 | 3 |
| `compat_module_context_properties` | 1 | 1 |
| `compat_module_context_events` | 1 | 1 |
| `compat_module_event_responses` | 1 | 1 |
| `compat_availability_responses` | 1 | 1 |
| `compat_availability_contexts` | 0 | 0 |
| `compat_availability_since_present` | 0 | 0 |
| Warm command wall time | 0.10 s | 0.10 s |
| Command max RSS | 39,524 KiB | 39,400 KiB |

The direct catalog sequence is after-only and is not compared with the
preimplementation generic adapter:

| Direct borrowed metric | Count |
| --- | ---: |
| `direct_deleted_sqlite_success` | 1 |
| `direct_source_locale_present` | 1 |
| `direct_generated_self_records` | 1 |
| `direct_exact_member_records` | 1 |
| `direct_member_enum_records` | 1 |
| `direct_callable_records` | 1 |
| `direct_global_method_records` | 3 |
| `direct_global_property_records` | 1 |
| `direct_module_event_records` | 1 |
| `direct_exact_module_event_records` | 1 |
| `direct_availability_contexts` | 0 |
| `direct_availability_since_present` | 0 |

`catalog_generic_dto_surface=absent` is structural API evidence, not a runtime
counter: the direct helper consumes only typed IDs and borrowed HBK records and
counts iterators without collecting them into owned vectors. At `b0841e6`,
`PlatformSnapshotSource` stored only `SourceId` plus `Arc<HbkFactSnapshot>`;
after the slice it stores only `HbkBslContextCatalog`. Neither shape contains a
`SearchIndex` handle, so the deleted-file success proves SQLite is not required
by this snapshot path. It is not reported as an instrumented SQL-call count.

At this intermediate BSL milestone task 1.6 remained open for the SDBL
baseline/after sequence and exact allocation/`worker_handle`/SearchIndex
counters. The final section below supplies those counters without production
instrumentation.

## First SDBL Catalog Slice After Evidence

The SDBL slice introduced `HbkSdblQueryCatalog`, delegated catalog-covered
`QueryTableSnapshotSource` acquisition to it and moved the six opaque
`metadata.sdbl.query-source.*` selectors to one locale-aware production owner.

The durable baseline patch is
`artifacts/sdbl-compat-baseline.patch`. It applies to pre-SDBL commit
`ff70367` and adds the same `compat_sdbl_adapter_sequence` helper used by the
current ignored probe. The two extracted helper bodies were byte-identical
(`cmp` exit code `0`, 131 lines each). Both runs use the deterministic
`fixture_index_path` query fixture, construct one shared snapshot and delete
the SQLite file before the compatibility sequence.

Baseline:

```text
git worktree add --detach <temporary-worktree> ff70367
git apply <catalog-change-worktree>/openspec/changes/expose-borrowed-hbk-domain-catalogs/artifacts/sdbl-compat-baseline.patch
/usr/bin/time -f 'baseline_wall_seconds=%e baseline_maxrss_kb=%M' \
  cargo test -p context-resolver-search sdbl_compat_measurement_probe \
  -- --ignored --nocapture
```

After:

```text
/usr/bin/time -f 'current_wall_seconds=%e current_maxrss_kb=%M' \
  cargo test -p context-resolver-search sdbl_catalog_measurement_probe \
  -- --ignored --nocapture
```

The compatibility metrics count observable returned generic DTO/projection
values. They are not allocator-level construction counters:

| Compatibility metric | `ff70367` | SDBL catalog slice |
| --- | ---: | ---: |
| `compat_deleted_sqlite_success` | 1 | 1 |
| `compat_global_context_invocations` | 1 | 1 |
| `compat_global_response_count` | 1 | 1 |
| `compat_global_fact_total` | 13 | 13 |
| `compat_global_query_table_count` | 10 | 10 |
| `compat_global_query_field_count` | 2 | 2 |
| `compat_global_query_parameter_count` | 1 | 1 |
| `compat_table_id_response_count` | 1 | 1 |
| `compat_table_exact_response_count` | 1 | 1 |
| `compat_field_enum_count` | 2 | 2 |
| `compat_field_exact_count` | 1 | 1 |
| `compat_parameter_enum_count` | 1 | 1 |
| `compat_parameter_exact_count` | 1 | 1 |
| `compat_selector_projection_count` | 6 | 6 |
| Warm command wall time | 0.11 s | 0.11 s |
| Command max RSS | 39,524 KiB | 39,400 KiB |

The direct borrowed sequence is after-only and is not compared with the
pre-SDBL generic adapter:

| Direct borrowed metric | Count |
| --- | ---: |
| `direct_deleted_sqlite_success` | 1 |
| `direct_source_locale_present` | 1 |
| `direct_all_table_count` | 10 |
| `direct_primary_table_point_count` | 1 |
| `direct_table_name_count` | 1 |
| `direct_table_syntax_count` | 1 |
| `direct_table_identifier_count` | 1 |
| `direct_field_enum_count` | 2 |
| `direct_field_id_count` | 1 |
| `direct_field_name_count` | 1 |
| `direct_parameter_enum_count` | 1 |
| `direct_parameter_id_count` | 1 |
| `direct_parameter_name_count` | 1 |
| `direct_selector_present_count` | 6 |
| `direct_unknown_selector_absent` | 1 |

The direct helper counts typed IDs and borrowed records without collecting
them solely to report counts. `QueryTableSnapshotSource` and
`HbkSdblQueryCatalog` contain no `SearchIndex` handle; both the compatibility
and direct sequences succeeded after the fixture SQLite file was removed.
This proves that SQLite is not required by the snapshot path, but it is not
reported as an instrumented zero-SQL-call counter.

At this intermediate SDBL milestone task 1.6 remained open for helper-scoped
allocation, exact `worker_handle` and instrumented SearchIndex-entry counters.
The final section below supplies them. Rusqlite/SQL entry counts are not
claimed; deleted-SQLite/no-handle evidence remains labelled separately.

## Final Runtime Counters For Task 1.6

### Counter definitions and reproducibility

No production or normal test instrumentation was added. Existing ignored
probes and their byte-identical baseline patches are the measurement boundary:

| Domain | Baseline commit + patch | Baseline probe/helper | Current probe/helper | Helper body comparison |
| --- | --- | --- | --- | --- |
| BSL | `b0841e6` + `artifacts/bsl-compat-baseline.patch` | `bsl_compat_measurement_probe` / `compat_adapter_sequence` | `bsl_catalog_measurement_probe` / `compat_adapter_sequence` | `cmp` exit `0`, 144 lines |
| SDBL | `ff70367` + `artifacts/sdbl-compat-baseline.patch` | `sdbl_compat_measurement_probe` / `compat_sdbl_adapter_sequence` | `sdbl_catalog_measurement_probe` / `compat_sdbl_adapter_sequence` | `cmp` exit `0`, 131 lines |

The baseline procedure was `git worktree add --detach`, `git apply --check`,
`git apply`, exact probe execution and required-symbol inspection with
`nm -anC`. Temporary worktrees were removed after measurement.

Toolchain and tools:

```text
rustc 1.95.0 (59807616e 2026-04-14)
cargo 1.95.0 (f2d3ce0bd 2026-03-21)
heaptrack 1.3.0
heaptrack_print 1.3.0
GNU nm 2.38
GNU sha256sum 8.32
GNU /usr/bin/time
```

All counter binaries used test-profile `opt-level=0`, `debuginfo=2` and forced
frame pointers:

```text
RUSTFLAGS='-C opt-level=0 -C debuginfo=2 -C force-frame-pointers=yes'
```

The measurement-only script
`artifacts/count-catalog-probe-calls.py` is owned by this OpenSpec change and
archives with it. It is not product or general regression infrastructure.
Commit `84e49ab` contains the reviewed script. It requires each test/compat/
direct marker to enter and finish exactly once, requires inferior exit `0`,
fails on missing symbols or entry/exit mismatch, and emits deterministic JSON.
A seeded missing test-marker run exited `2` with
`missing-required-symbol`; valid runs had `invalid_hits=[]`.

The complete raw evidence is retained under
`artifacts/runtime-counters/`: 178 files (15,995,318 bytes) containing build
commands and binary identities, required-symbol listings, GDB JSON and
summaries, both heaptrack repetitions and filtered stacks, one warmup plus five
wall-time runs, and the known-allocation negative control. Its
`artifact-manifest.tsv` has SHA-256
`5a73d15cb21792f8d2cfbfc8dc7c6560456e8b83147c59e9a75bd3c9d1ec6507`.
The tabular values below are derived from those tracked raw files.

### Helper-scoped heap allocation calls

Heaptrack extraction used:

```text
heaptrack -- "$binary" --ignored --nocapture --exact tests::<probe>
heaptrack_print --filter-bt-function <helper-symbol> \
  -F <helper>.stacks --flamegraph-cost-type allocations <trace>
awk '{sum += $NF} END {print sum + 0}' <helper>.stacks
```

The summation method was first validated against a temporary no-inline fixture
performing exactly seven `malloc` calls: the unfiltered process reported nine
allocations, while the symbol-filtered stack sum was exactly seven. Every real
helper was recorded twice; both repetitions were identical and produced
non-empty symbolized stacks.

| Domain/run | Binary SHA-256 | Helper-scoped allocation calls, run 1 / run 2 | Symbolized stack lines |
| --- | --- | ---: | ---: |
| BSL baseline compat | `f07e4a4284066bf6a31d907ae062b5822ab6b676062f2a747e119b4268beff2b` | 305 / 305 | 255 / 255 |
| BSL current compat | `6645a7016ffce17fdcb58256547181d53b3d9fe2d3929e5986d01cb1cfa03945` | 285 / 285 | 263 / 263 |
| BSL current direct borrowed | same current binary | 12 / 12 | 10 / 10 |
| SDBL baseline compat | `a09220b551b8d20a35e8d80a8f2ace3936886ffc62a0a30bb61463bb1a31feef` | 414 / 414 | 178 / 178 |
| SDBL current compat | same current binary | 415 / 415 | 191 / 191 |
| SDBL current direct borrowed | same current binary | 13 / 13 | 9 / 9 |

These are all heap allocation calls whose symbolized stack contains the exact
helper. They are not DTO-constructor counts. The compatibility tables above
remain the observable returned owned DTO/projection counts; the GDB projection
entries below identify exact generic projection-helper invocations. The direct
borrowed helpers still allocate lookup-normalization strings, so “borrowed”
does not mean zero allocations.

### Exact function-entry counters

The same forced-frame-pointer binaries were used for heaptrack, GDB and wall
time:

| Run | Binary SHA-256 |
| --- | --- |
| BSL baseline | `f07e4a4284066bf6a31d907ae062b5822ab6b676062f2a747e119b4268beff2b` |
| SDBL baseline | `a09220b551b8d20a35e8d80a8f2ace3936886ffc62a0a30bb61463bb1a31feef` |
| BSL/SDBL current | `6645a7016ffce17fdcb58256547181d53b3d9fe2d3929e5986d01cb1cfa03945` |

Every valid run resolved 39 `syntax_helper_search::SearchIndex` method symbols
at 41 breakpoint locations. Marker entry/exit counts were `1/1`; every counted
function entry had a matching finish. The exact
`ContextSource::global_context` trait-implementation symbol was supplied from
the binary's demangled `nm` output.

BSL counts:

| Counter | Baseline compat | Current compat | Current direct borrowed |
| --- | ---: | ---: | ---: |
| `HbkFactSnapshot::worker_handle` | 11 | 17 | 12 |
| `ContextSource::global_context` | 2 | 2 | 0 |
| `map_platform_type` | 1 | 1 | 0 |
| `map_member` | 3 | 2 | 0 |
| `map_callable` | 9 | 9 | 0 |
| `map_global_property` | 2 | 2 | 0 |
| `map_event_as_member` | 0 | 0 | 0 |
| `map_availability` | 1 | 0 | 0 |
| SearchIndex method entries inside helper | 0 | 0 | 0 |

SDBL counts:

| Counter | Baseline compat | Current compat | Current direct borrowed |
| --- | ---: | ---: | ---: |
| `HbkFactSnapshot::worker_handle` | 17 | 41 | 17 |
| `ContextSource::global_context` | 1 | 1 | 0 |
| `map_query_table` | 18 | 18 | 0 |
| `map_query_field` | 5 | 5 | 0 |
| `map_query_parameter` | 3 | 3 | 0 |
| SearchIndex method entries inside helper | 0 | 0 | 0 |

Fixture setup, outside compat/direct markers, invoked only
`SearchIndex::{open_read_only=1,metadata=1,metadata_value=6,
metadata_optional_value=1}`. Those nine setup entries are not snapshot-helper
fallback calls. The zero rows above are instrumented **SearchIndex method
entries**, not SQL/rusqlite-call counters. Separate structural evidence shows
the snapshot sources/catalogs retain no `SearchIndex` handle, and both helper
sequences succeeded after deleting the source SQLite file. No SQL-call count
is claimed.

### Controlled wall time

Wall time is advisory, not an acceptance threshold. Each binary received one
warmup followed by five uninstrumented exact-probe runs with
`/usr/bin/time -f '%e'`; compilation was excluded.

| Domain/run | Five elapsed values, seconds | Median | Min-max |
| --- | --- | ---: | ---: |
| BSL baseline compat | `0.01 0.01 0.01 0.02 0.01` | 0.01 | 0.01-0.02 |
| BSL current compat+direct | `0.01 0.02 0.01 0.01 0.01` | 0.01 | 0.01-0.02 |
| SDBL baseline compat | `0.01 0.01 0.01 0.01 0.02` | 0.01 | 0.01-0.02 |
| SDBL current compat+direct | `0.02 0.02 0.01 0.01 0.01` | 0.01 | 0.01-0.02 |

The current values include both compatibility and direct borrowed sequences
and therefore are not used to claim a microbenchmark speedup.

### Task 1.6 conclusion

Task 1.6 is a capture requirement, not an optimization threshold. It is closed
because both domains now have reproducible baseline/current observable
materialization counts, helper-scoped allocation calls, exact
`global_context`/projection/`worker_handle`/SearchIndex entries and controlled
wall time. Structural source guards separately prove ownership movement. No
production counter, cache, arena, DTO mirror or fallback path was introduced.
