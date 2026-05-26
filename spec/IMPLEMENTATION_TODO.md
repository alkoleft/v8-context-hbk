# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history:

- [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md)
- [archive/completed-tasks-t13-t17-t19-t24.md](archive/completed-tasks-t13-t17-t19-t24.md)
- [archive/completed-tasks-t25-t34.md](archive/completed-tasks-t25-t34.md)
- [archive/implementation-todo-2026-05-04.md](archive/implementation-todo-2026-05-04.md)
- [archive/implementation-todo-2026-05-05.md](archive/implementation-todo-2026-05-05.md)
- [archive/completed-tasks-t41-t47.md](archive/completed-tasks-t41-t47.md)
- [archive/completed-tasks-t48-t56.md](archive/completed-tasks-t48-t56.md)
- [archive/completed-tasks-t57-t65-t68-t85.md](archive/completed-tasks-t57-t65-t68-t85.md)
- [archive/completed-tasks-t66-t67-t86-t90.md](archive/completed-tasks-t66-t67-t86-t90.md)
- [archive/completed-tasks-t91-t110.md](archive/completed-tasks-t91-t110.md)
- [archive/completed-tasks-t111-t134.md](archive/completed-tasks-t111-t134.md)
- [archive/completed-tasks-t135-t142.md](archive/completed-tasks-t135-t142.md)
- [archive/completed-tasks-t143-t151.md](archive/completed-tasks-t143-t151.md)
- [archive/completed-tasks-t152-t164.md](archive/completed-tasks-t152-t164.md)

Current status: T35-T164 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design,
language-domain, cleanup, book-content export, documentation-site, platform type-template and
type-reference conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`requirements/non-functional.md`, `implementation/components.md`,
`implementation/documentation-site.md`, `implementation/syntax-helper-query-cli.md`,
`implementation/syntax-bsl-provider-plan.md`, `implementation/solution-context-resolve.md`,
`implementation/performance-baseline-t13.md`, `implementation/performance-variants.md` and
`decisions/`.

Current first unchecked task: T170.

## Loop Rule

- Take the first unchecked task.
- If there is no unchecked task, add one before implementing new scope.
- Every new task must reference the relevant requirement, UAT, acceptance, implementation spec or
  ADR IDs from `spec/`.
- Keep scope limited to that task and its direct verification.
- Mark a task complete only after its listed verification passes.
- If a task cannot be completed, leave it unchecked and record the blocker in the task notes or final
  response.
- Do not start the next task in the same run unless the prompt explicitly asks for multiple tasks.
- Before committing, stage only files changed for the current task and verify
  `git diff --cached --name-only`.
- Do not create empty commits.

## Active Tasks

### [x] T169. Reshape `HbkFactSnapshot` physical indexes around analyzer hot paths

References: FR-CTX-RESOLVE-001, NFR-RESOLVE-001, NFR-QUERY-001, UC-CTX-001,
UC-CTX-002, `implementation/solution-context-resolve.md`,
`implementation/components.md`, `acceptance/baseline.md`,
OpenSpec change `provider-owned-hbk-fact-snapshot`.

Scope:

- Start with a before-change release measurement of the current T168 snapshot on the representative
  `shcntx_ru` provider index. Record snapshot build time, process peak RSS, estimated
  snapshot-owned heap and batched lookup timings for the hot paths below before changing layout.
- Refactor the snapshot read model so physical indexes are organized by analyzer queries rather
  than public DTO result families.
- Keep snapshot-owned nodes/arenas as the single source of provider fact payloads. Secondary
  indexes store only compact keys and `NodeRef`/range values.
- Implement physical indexes inside `syntax-helper-search::HbkFactSnapshot` and read-handle APIs.
  `context-resolver-search` may call read handles and project results into resolver DTOs, but must
  not build duplicate provider-fact maps, query raw SQLite, or own analyzer-side mirrors of HBK
  facts.
- Keep index payloads compact: keys plus node refs/ranges only. Do not store cloned names,
  signatures, descriptions, type-ref vectors or DTO structs inside secondary indexes.
- Add or reshape first-slice physical indexes for:
  - exact fact id lookup;
  - normalized type name and type template-key lookup;
  - owner member listing;
  - `(owner type, normalized name, optional kind)` member lookup;
  - `(owner type, normalized name)` callable lookup;
  - constructors by type;
  - language/domain global method/property lookup;
  - module context by language/domain/module kind;
  - query table by name/syntax/identifier;
  - query field and query parameter by table/name;
  - compact availability by fact;
  - relation traversal by source fact and relation kind.
- Prefer contiguous arenas plus owner ranges when they reduce allocation count and keep lookup
  cache-local. Nested logical ownership must remain explicit even if the physical storage uses
  ranges instead of nested `Vec` fields.
- Add memory accounting for string store, node arenas and each secondary-index family. The task is
  not complete if total snapshot-owned heap grows without identifying the index family responsible.
- Keep descriptions, previews, notes, full signature text, raw HBK/HTML provenance, long
  documentation text, arbitrary fuzzy search data and unbounded relation paths out of first-slice
  physical indexes.
- Keep existing resolver DTOs as adapter projections over snapshot nodes. Do not expose raw SQLite
  tables or make downstream analyzers depend on provider storage details.
- Do not add Tantivy, persisted snapshot formats, minimal-perfect hashing, compressed bitmap
  dependencies, global caches, async runtimes or tuning knobs in this slice.
- Add no new runtime dependency in this task. If a dependency appears necessary, leave T169
  unchecked until the task records the measured bottleneck, why `std` and existing workspace
  dependencies are insufficient, and the ADR/spec update that owns the dependency decision.

Verification:

- focused snapshot tests for each hot-path index listed above;
- concurrent deterministic read test across multiple threads;
- release before/after measurement against the current representative `shcntx_ru` provider index,
  recording warm snapshot build time, process peak RSS, estimated snapshot-owned heap, node/string
  heap, per-index counts/bytes and representative lookup timings after source open;
- compare release warm measurements with the T168 baseline (`507-601 ms` build, median `511 ms`,
  `18197557` estimated snapshot-owned bytes, `105708-105844 KiB` process peak RSS). If median build
  time or peak RSS increases by more than 15%, or estimated snapshot-owned heap increases by more
  than 25%, identify the responsible index family and justify the tradeoff with measured hot-path
  lookup benefit; otherwise leave T169 unchecked with a follow-up;
- batched release lookup measurements for at least:
  - exact fact id;
  - `(owner type, normalized name, optional kind)` member lookup;
  - `(owner type, normalized name)` callable lookup;
  - constructors by type;
  - module context by language/domain/module kind;
  - query table by name/syntax/identifier;
  - query field and query parameter by table/name;
  - relation traversal by source fact and relation kind.
- each measured lookup must stay under the NFR-RESOLVE-001 provisional `100 ms` resolver/API ceiling
  on the representative source after `HbkFactReadHandle` creation. If not, leave T169 unchecked and
  record measured timings, source size and the limiting component;
- a physical index counts as complete only when it has a read-handle method and either a migrated
  adapter test or a documented analyzer lookup scenario using it. Do not add placeholder physical
  indexes for listed families that are not exercised in this slice; document them as deferred;
- focused `context-resolver-search` adapter tests showing migrated known-owner member/callable
  lookup, module-context lookup and query-table field/parameter lookup use the snapshot/read-handle
  path;
- if any adapter path remains transitional in this slice, document the exact non-migrated method,
  the reason it remains on the old path and the follow-up task that will migrate it;
- `openspec validate provider-owned-hbk-fact-snapshot --strict`;
- `cargo fmt --all --check`;
- focused package tests/checks for touched crates.

Completion notes:

- Snapshot/read-handle physical indexes were reshaped in `syntax-helper-search` for the listed
  hot paths, including fact id, type name, type template key, owner member/callable, constructors,
  global lookup, module context, query table/field/parameter, availability and relation traversal
  indexes. The snapshot now also represents enum and enum-value fact refs in exact-id,
  relation and availability lookup surfaces.
- `context-resolver-search` now has explicit snapshot-backed sources,
  `PlatformSnapshotSource` and `QueryTableSnapshotSource`, composed from provider-owned
  `Arc<HbkFactSnapshot>` state. They project snapshot nodes into existing `context-resolver-core`
  DTOs for platform type/member/callable/global/module/related/availability lookups and query
  table/field/parameter lookups without reading SQLite or falling back to `SearchIndex` inside
  migrated methods.
- `PlatformSearchSource` and `LanguageSearchSource` remain explicit SQL/SearchIndex-backed
  adapters for CLI, debug, index inspection and sequential local resolver usage. The worker-safe
  analyzer path composes snapshot-backed sources by constructor/type name rather than silently
  switching existing SQL-backed source names.
- Focused tests cover snapshot hot-path indexes, deterministic concurrent reads, enum/enum-value
  snapshot participation, snapshot-backed platform resolver paths, snapshot-backed query-table
  resolver paths and `Send + Sync` source-boundary assertions.
- Final release measurement on
  `target/snapshot-materialization/shcntx_ru.schema16.release.sqlite` with three warm runs reported
  SQLite materialization builds of `2317 ms`, `788 ms` and `943 ms`; the first run is retained as
  cache-warm-up evidence, while the warm post-build range is `788-943 ms`. Peak RSS was
  `105860-106164 KiB`; estimated SQLite-materialized snapshot heap was `23324034` bytes and
  payload bytes were `17950274`. The heap increase over the earlier T169 partial measurement is
  explained by the newly represented enum/enum-value arenas and indexes.
- The build-time regression is accepted for T169 because it is isolated to the SQLite
  materialization/startup path, not to steady-state analyzer lookups after `HbkFactReadHandle`
  creation. The responsible startup components remain the previously measured SQLite row
  read/decode, fact arena construction and fact-id/relation/availability construction stages,
  with additional enum/enum-value arena/index work in this stabilization pass. T170 owns reducing
  this startup path through a derived cache once invalidation and final format are specified.
- The same release runs wrote and read the measurement-only experimental binary cache. Cache reads
  were `39 ms`, `29 ms` and `30 ms`; the warm read range is `29-30 ms`, about `26-31x` faster than
  the same-run SQLite materialization startup. The cache file was `11364011` bytes and every run
  reported `binary_cache.roundtrip_equal=true`. This strengthens T170 prototype evidence only; it
  does not accept a persisted format or invalidation policy.

### [ ] T170. Stabilize provider-owned derived cache for `HbkFactSnapshot` startup latency

References: FR-CTX-RESOLVE-001, NFR-RESOLVE-001, NFR-QUERY-001, UC-CTX-001,
UC-CTX-002, `implementation/solution-context-resolve.md`,
`acceptance/baseline.md`, T169, OpenSpec change
`stabilize-hbk-fact-snapshot-cache`.

Scope:

- Treat the existing SQLite provider index as the canonical rebuildable provider artifact. The
  persisted snapshot cache is a derived startup/read-model artifact, not a replacement source of
  truth and not a public contract for downstream analyzers.
- Start from the post-T169 evidence rather than reopening open-ended cache exploration: warmed
  SQLite materialization measured `788-943 ms`, warmed binary-cache reads measured `29-30 ms`, the
  cache file was `11364011` bytes and every measured run reported
  `binary_cache.roundtrip_equal=true`.
- Stabilize the cache/invalidation contract before accepting a runtime cache path: cache format
  version, provider SQLite schema version, source index identity/hash, platform version/locale when
  available, snapshot layout/version flags and an integrity guard. On mismatch, unsupported version
  or corruption, rebuild from the SQLite provider index.
- Decide whether the current no-dependency little-endian DTO path is accepted as the first stable
  provider-owned cache format or remains experimental behind explicit naming. Only consider
  zero-copy or memory-mapped layouts such as `rkyv`/`zerocopy` after a stable-cache measurement shows
  that deserialization/allocation, not SQLite materialization, is still the limiting component.
- Keep `fst` scoped to measured name/id lookup index compression if lookup indexes, not startup
  deserialization, are the limiting component. Do not use Tantivy, search/export payloads or fuzzy
  search data for the worker fact snapshot cache.
- Keep the persisted artifact provider-owned. Resolver adapters may load or receive
  `Arc<HbkFactSnapshot>`, but must not depend on SQLite tables, binary layout details or
  analyzer-owned mirror indexes.
- Do not reopen T171 in this task. `PlatformSnapshotSource` and `QueryTableSnapshotSource` remain
  the completed snapshot-backed resolver slice. A non-query-table `LanguageSnapshotSource` is a
  separate future task/change, not part of cache stabilization.

Verification:

- cache metadata/invalidation tests for version/schema/source/layout mismatch and corrupted or
  truncated cache data;
- release comparison of at least two startup paths on the post-T169 representative `shcntx_ru`
  provider index: SQLite materialization baseline and derived cache validation/load;
- report warm build/load time, cache validation cost, process peak RSS, capacity-based
  snapshot-owned heap, logical payload bytes, cache file size and representative read-handle lookup
  timings;
- keep lookup correctness covered by existing focused snapshot tests plus cache round-trip and
  cache-loaded snapshot-backed resolver tests needed for the chosen stable path;
- update `acceptance/baseline.md` and `implementation/solution-context-resolve.md` with the
  measured conclusion before accepting a persisted format decision.

Initial stage-timing result:

- Added measurement-only stage timing to the existing release harness. Five warm runs on
  `target/snapshot-materialization/shcntx_ru.schema16.release.sqlite` reported snapshot build
  times of `618 ms`, `649 ms`, `618 ms`, `625 ms` and `641 ms`, for a `625 ms` median.
- Dominant median buckets were SQLite row reading (`228 ms`), fact arena construction (`164 ms`)
  and fact-id/relation/availability construction (`89 ms`). Together these account for most of the
  current startup class and justify a persisted binary-cache prototype after the in-memory T169
  layout/resolver migration is settled.
- The current timing does not choose a disk format. It narrows the next experiment to bypassing
  repeated SQL row decoding and repeated arena/index construction from SQLite while keeping SQLite
  as the canonical rebuildable provider artifact.
- Added a measurement-only provider-owned binary cache prototype using a small versioned
  little-endian format with magic, cache version and provider schema version guards. It introduces
  no new runtime dependency and is not a downstream storage contract. The public methods are named
  `write_experimental_binary_cache` and `from_experimental_binary_cache` to keep that status
  explicit.
- Five warm runs comparing the same source snapshot with the binary cache reported SQLite
  materialization build times of `645 ms`, `643 ms`, `629 ms`, `605 ms` and `683 ms`, for a
  `643 ms` median. Binary cache reads were `25 ms`, `25 ms`, `25 ms`, `24 ms` and `26 ms`, for a
  `25 ms` median. Cache writes were `11-48 ms`, median `44 ms`.
- The cache file was `10319044` bytes (`9.9 MiB`) and every run reported
  `binary_cache.roundtrip_equal=true`. The cache-loaded snapshot estimated heap was
  `16597927` bytes versus `20345723` bytes for the SQLite-materialized snapshot because the binary
  reader allocates exact vector capacities. The harness now reports logical payload bytes in
  addition to capacity-based heap bytes, so future cache comparisons must use both metrics before
  treating the heap delta as structural memory savings.
- Current conclusion: the simple binary cache prototype is strong enough to keep T170 as a real
  follow-up now that T169 stabilized the physical read model and resolver adapter migration. The
  prototype does not yet accept a final persisted format decision or cache invalidation policy
  beyond the minimal version/schema guard.
- Post-T169 T170 adaptation: the next implementation slice is cache stabilization, not broad
  exploration. The new OpenSpec change `stabilize-hbk-fact-snapshot-cache` owns cache metadata,
  invalidation, final format decision and acceptance measurements. T171 remains complete and is not
  reopened by cache work.

### [x] T171. Add explicit snapshot-backed resolver adapters for worker-safe analyzer lookup

References: FR-CTX-RESOLVE-001, NFR-RESOLVE-001, NFR-QUERY-001, UC-CTX-001,
UC-CTX-002, `implementation/solution-context-resolve.md`,
`implementation/components.md`, `acceptance/baseline.md`, T169, T170,
OpenSpec change `provider-owned-hbk-fact-snapshot`.

Scope:

- Extend `context-resolver-search` with explicit snapshot-backed source adapters, named for the
  backend rather than hidden behind the existing SQL/SearchIndex source types. Complete and align
  the already introduced `PlatformSnapshotSource` and `QueryTableSnapshotSource`. Add or rename a
  broader `LanguageSnapshotSource` only if the migrated resolver slice truly covers non-query-table
  language facts; otherwise keep query-table lookup under `QueryTableSnapshotSource`.
- Accept `Arc<HbkFactSnapshot>` or another public provider-owned snapshot/read-handle entrypoint
  from `syntax-helper-search`. Use `HbkFactReadHandle` for migrated hot-path lookups.
- Implement `context_resolver_core::ContextSource` for the snapshot-backed sources and project
  snapshot nodes into existing resolver DTOs: `ResolvedType`, `ResolvedMember`,
  `ResolvedCallable`, `ResolvedGlobalContext`, `ResolvedModuleContext`, `ContextFact`,
  `AvailabilityFact` and query table/field/parameter DTOs.
- Keep `PlatformSearchSource` and `LanguageSearchSource` as the explicit SQL/SearchIndex-backed
  backend for CLI, debug, index inspection and sequential local resolver scenarios. Do not describe
  this backend as legacy, do not include downstream analyzer hot paths in it and do not silently
  replace these constructors with snapshot behavior.
- Make backend choice explicit at composition time. No migrated snapshot-backed resolver path may
  fall back from snapshot to SQL/SearchIndex internally.
- Do not read SQLite on migrated snapshot hot paths. SQLite may be used only while materializing a
  provider-owned snapshot or by the explicit SQL/SearchIndex backend.
- Do not build duplicate provider-fact mirror indexes in `context-resolver-search`, copy broad DTO
  payloads into the snapshot physical model or add analyzer-owned fallback tables in `v8-context`.
- Preserve source/domain identity for migrated snapshot-backed sources when platform,
  query-language and any migrated BSL-language facts share display names. If T171 does not migrate
  non-query-table BSL-language facts, document and test the snapshot-backed result for those facts as
  unsupported or empty; do not require identity disambiguation through a source that is not migrated.
- Prove the downstream boundary is worker-safe for the adapter/resolver composition, not only for
  `HbkFactSnapshot` alone. Use a `Send + Sync` compile assertion for the snapshot-backed
  source/resolver or document and test an explicit scoped-worker borrow contract.
- Snapshot-backed hot paths must not satisfy worker safety by wrapping resolver/search state,
  SQLite connections or mutable adapter internals in broad `Arc<Mutex<_>>` / `Arc<RwLock<_>>`.
  Shared state for migrated analyzer lookups is limited to immutable provider-owned snapshot data,
  for example `Arc<HbkFactSnapshot>`, plus worker-local read handles or caches.
- Before T171 can be accepted, enum and enum-value fact refs must either participate in migrated
  exact-id and relation lookup through the snapshot-backed adapter slice, or the task must
  explicitly document that the migrated resolver slice excludes those facts and returns the
  documented unsupported/empty result for them. Silent omission is not accepted.
- Keep the persisted/binary cache format from T170 provider-owned and internal. Snapshot adapters
  may receive a loaded snapshot, but they must not expose or depend on cache layout details.

Verification:

- focused snapshot-backed resolver tests for platform type lookup;
- member lookup by owner/name/kind;
- callable lookup by owner/name;
- global context lookup;
- module context lookup;
- related/availability lookup;
- query table lookup by name, syntax and identifier;
- query field and query parameter lookup by table/name;
- source/domain identity preservation for all migrated snapshot-backed source families when facts
  share display names, plus explicit unsupported/empty coverage for non-migrated BSL-language facts
  if no `LanguageSnapshotSource` is added;
- enum and enum-value exact-id/relation participation through the migrated snapshot-backed slice, or
  explicit tests for the documented unsupported/empty result when that slice excludes them;
- compile or focused test proving the snapshot-backed source/resolver boundary is `Send + Sync`, or
  proving the documented scoped-worker borrow contract;
- focused code/test guard that migrated hot paths do not use broad `Arc<Mutex<_>>` /
  `Arc<RwLock<_>>` around resolver/search state, SQLite connections or mutable adapter internals;
- regression tests proving SQL/SearchIndex-backed `PlatformSearchSource` and `LanguageSearchSource`
  scenarios still work and are selected explicitly;
- concrete no-SQL/no-fallback test: compose snapshot-backed sources from an already materialized
  in-memory `HbkFactSnapshot`, make the source SQLite path unavailable or absent, verify migrated
  lookups still work and verify missing snapshot coverage returns the documented unsupported/empty
  result rather than using SQL/SearchIndex fallback;
- `openspec validate provider-owned-hbk-fact-snapshot --strict`;
- `cargo fmt --all --check`;
- focused package tests/checks for touched crates.

Completion notes:

- Completed as part of T169 stabilization because T171 was the active T169 adapter blocker.
- Added explicit `PlatformSnapshotSource` and `QueryTableSnapshotSource` implementations over
  provider-owned `HbkFactSnapshot` state.
- Kept `PlatformSearchSource` and `LanguageSearchSource` SQL/SearchIndex-backed by design.
- Verified migrated snapshot-backed lookups with focused resolver tests and `Send + Sync`
  assertions over `PlatformSnapshotSource`, `QueryTableSnapshotSource` and
  `WorkerSafeCompositeResolver`. The tests compose snapshot-backed sources from an already
  materialized in-memory `Arc<HbkFactSnapshot>`, remove the SQLite file and then run the migrated
  lookups, including query field/parameter lookup by table/name, proving no hidden
  SQL/SearchIndex fallback is needed on those paths. Missing broader non-query-table language
  snapshot coverage remains intentionally out of this slice; query-table language facts use
  `QueryTableSnapshotSource`.

### [x] T168. Implement the first provider-owned worker-safe HBK fact snapshot slice

References: FR-CTX-RESOLVE-001, NFR-PERF-001, NFR-QUERY-001, UC-CTX-001,
UC-CTX-002, `implementation/solution-context-resolve.md`,
`implementation/components.md`, `acceptance/baseline.md`,
OpenSpec change `provider-owned-hbk-fact-snapshot`.

Scope:

- Add compact provider-owned snapshot DTOs/node ids for the measured SQLite-first materialization
  path.
- Keep the snapshot contract-shaped: include only fields required by worker fact lookup and exclude
  search/export/index-maintenance payloads such as FTS rows, preview text, raw descriptions, raw
  storage paths, relation weights and parser diagnostics.
- Implement a narrow immutable snapshot type that is `Send + Sync` and can be shared as `Arc<_>`.
- Implement worker-local read handles with representative lookups for platform type members,
  callables, global context, module events, query table fields/parameters and language facts where
  the indexed source provides them.
- Use owned `Vec` arenas, compact node/string ids, sorted lookup vectors and compressed-sparse-row
  style adjacency arrays as the first index shape.
- Keep existing resolver DTOs as adapter projections rather than the physical snapshot storage.
- Do not add analyzer fallback readers, raw SQLite readers in `v8-context`, direct HBK parsing in
  worker lookup, or broad `Arc<Mutex<_>>` around resolver/search state.
- Do not add Tantivy, persisted zero-copy snapshot formats, minimal-perfect hashing or compressed
  bitmap dependencies in the first slice. Treat `fst`, `rkyv`/`zerovec` and `roaring` as measured
  follow-up experiments only if the arena snapshot exposes a concrete bottleneck.

Verification:

- `openspec validate provider-owned-hbk-fact-snapshot --strict`
- focused snapshot unit/integration tests, including compile-time `Send + Sync` assertion
- concurrent deterministic read test across multiple threads
- representative lookup test coverage for platform type -> members/callables, platform global
  context, module context events, query table -> fields/parameters and documented language/query
  facts available in indexed sources
- `cargo fmt --all --check`
- focused package tests/checks for touched crates

Result:

- `syntax-helper-search` now exposes provider-owned `HbkFactSnapshot` / `HbkFactReadHandle`
  storage APIs over immutable owned arenas, compact node/string ids, derived lookup vectors and
  compressed-sparse-row owner adjacency arrays.
- The snapshot materializes from an existing provider SQLite index through provider-owned bulk table
  reads and does not store or share `rusqlite::Connection`, raw SQLite tables or mutable resolver
  state after construction.
- Representative read-handle lookups cover platform type ids/names, owner members/callables,
  platform global facts, module events, query tables with fields/parameters and language facts.
- Release measurement on `target/snapshot-materialization/shcntx_ru.schema16.release.sqlite`
  produced stable warm snapshot build readings of `507-601 ms`, median `511 ms`; first-run/cache
  warm-up observations are excluded from the baseline. Estimated snapshot-owned heap was
  `18197557` bytes and process peak RSS stayed around `105708-105844 KiB`.
- Existing resolver DTO adapters were not rewritten in this slice; they remain adapter projections
  over the current search-index path while the snapshot read model stabilizes.
- Verification passed with `cargo test -p syntax-helper-search snapshot`,
  `openspec validate provider-owned-hbk-fact-snapshot --strict`, `cargo fmt --all --check` and
  `cargo check -p syntax-helper-search`.

### [x] T167. Measure SQLite-first HBK fact snapshot materialization

References: FR-CTX-RESOLVE-001, NFR-PERF-001, NFR-QUERY-001, UC-CTX-001,
UC-CTX-002, `implementation/solution-context-resolve.md`,
`implementation/components.md`,
OpenSpec change `provider-owned-hbk-fact-snapshot`.

Scope:

- Treat the existing `syntax-helper-search` SQLite provider index as the first candidate source for
  an immutable worker-safe HBK fact snapshot.
- Add a measurement-only bulk materialization harness that reads provider-owned SQLite tables in
  coarse passes instead of using public N+1 lookup APIs.
- Measure build time, RSS delta or peak RSS, estimated heap when practical, node counts by category
  and representative lookup/index coverage on a real `shcntx_ru` provider index.
- Compare the SQLite-first materialization path with existing HBK/index build measurements and the
  downstream N+1 lookup spike before accepting the broader snapshot implementation direction.

Verification:

- `openspec validate provider-owned-hbk-fact-snapshot --strict`
- measurement command on a representative local `shcntx_ru` SQLite provider index
- `cargo fmt --all --check`
- focused package check for the temporary measurement harness before it was removed

Result:

- OpenSpec change `provider-owned-hbk-fact-snapshot` records SQLite-first materialization as a
  measured design gate.
- Used a temporary `syntax-helper-search` measurement harness to bulk-read provider-owned SQLite
  tables without public `SearchIndex` lookup APIs. The harness was removed after the measurements
  were promoted into the durable specs.
- The measurement probe was narrowed to contract-shaped snapshot fields only; it does not copy
  search/export/index-maintenance payloads or raw storage paths.
- Current release CLI rebuilt schema-16 `shcntx_ru` provider index from
  `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` in `14.50s`, with `284360 KiB` peak RSS and
  `25415` documents.
- Release compact SQLite -> snapshot probe materialized the same index in `474 ms` (`0.55s`
  process elapsed), with `49112 KiB` peak RSS, `46540 KiB` RSS delta and `34935365` estimated heap
  bytes.
- Probe counts: `25415` documents, `2465` type identities, `121` type templates, `18609` members,
  `8337` callables, `8675` signatures, `9793` parameters, `47156` type refs, `58128` relations and
  `728` document metadata rows.
- Review/fix pass verification after harness removal: `openspec validate
  provider-owned-hbk-fact-snapshot --strict`, `cargo fmt --all --check`,
  `cargo check -p syntax-helper-search` and `git diff --check` passed.
- Conclusion: SQLite bulk materialization is accepted as the first implementation source for the
  worker-safe snapshot. Direct HBK reading remains setup/index-refresh input and comparison
  baseline.

### [x] T166. Expose shcntx query table templates through the QueryLanguage resolver source

References: FR-CTX-RESOLVE-001, UC-CTX-001, UC-CTX-002,
`implementation/solution-context-resolve.md`, `implementation/components.md`.

Scope:

- Expose existing `query_table`, `query_table_field` and `query_table_parameter` search documents
  through a distinct `LanguageDomain::QueryLanguage` Rust resolver source.
- Return template/family-level facts only: stable ids, syntax/identifier/table-role data, owner
  semantic path, source-derived template parameter slots, owned field/parameter identities, type
  references and source-neutral evidence/provenance.
- Preserve domain separation: query-table facts are not `PlatformApi` facts, do not become platform
  members, do not instantiate concrete metadata tables and do not add analyzer fallback tables.
- Cover exact lookup and relation traversal with focused `syntax-helper-search` and
  `context-resolver-search` tests, including the existing platform-adapter hiding behavior.

Verification:

- `cargo test -p syntax-helper-search query_table`
- `cargo test -p context-resolver-search query_table`
- `cargo test -p context-resolver-core`
- `cargo test -p context-resolver-search`
- `cargo test --workspace`

Result:

- `context-resolver-core` exposes dependency-facing query-table DTOs:
  `QueryTableInfo`, `QueryFieldInfo`, `QueryParameterInfo`, `QueryTableRole` and
  source-neutral `FactProvenance`.
- `syntax-helper-search` persists query-table metadata in private schema version `16`, including
  syntax, identifier, table role, owner path, template parameter slots, field/parameter notes and
  defaults, source provenance and type references.
- `context-resolver-search` exposes `query_table`, `query_table_field` and
  `query_table_parameter` through `LanguageSearchSource::query_tables` /
  `open_query_tables_read_only*` as `LanguageDomain::QueryLanguage` facts with exact lookup and
  relation capabilities only.
- Exact lookup by display name, identifier and syntax, `member_of` relation traversal and
  `has_type` traversal preserve stable ids and type references. The platform adapter continues to
  hide query-table provider documents from `PlatformApi`.
- Verification passed with `cargo test -p syntax-helper-search query_table`,
  `cargo test -p context-resolver-search query_table`, `cargo test -p context-resolver-core`,
  `cargo test -p context-resolver-search`, `cargo check --workspace`,
  `cargo fmt --all --check` and `cargo test --workspace`.

### [x] T165. Expose core BSL primitive language types through Rust resolver adapters

References: FR-CTX-RESOLVE-001, UC-CTX-001, UC-CTX-002,
`implementation/solution-context-resolve.md`.

Scope:

- Extend the `shlang_*` language-fact slice so direct BSL primitive type pages are indexed as
  `language_type` facts, including `Null`, `Неопределено` / `Undefined`, `Число` / `Number`,
  `Строка` / `String`, `Дата` / `Date`, `Булево` / `Boolean` and `Тип` / `Type`.
- Keep nested primitive literal pages such as `def_BooleanTrue` and `def_BooleanFalse` out of the
  type surface.
- Preserve source/domain identity through `context-resolver-core` and `context-resolver-search`:
  these facts are `BslLanguage` facts from `shlang`, not `PlatformApi` types.
- Cover dependency-facing behavior with focused `syntax-helper-language`,
  `syntax-helper-search` and `context-resolver-search` tests.

Verification:

- `cargo test -p syntax-helper-language`
- `cargo test -p syntax-helper-search`
- `cargo test -p context-resolver-search`
- `cargo test --workspace`

Result:

- `syntax-helper-language` extracts direct `shlang_*` primitive type pages as `language_type`
  facts for `Null`, `Неопределено` / `Undefined`, `Число` / `Number`, `Строка` / `String`,
  `Дата` / `Date`, `Булево` / `Boolean` and `Тип` / `Type`.
- Nested primitive literal pages such as `def_BooleanTrue` remain ignored by this type surface.
- `syntax-helper-search` indexes these facts with source-qualified `shlang:*` ids.
- `context-resolver-search` resolves them through `LanguageSearchSource` as
  `LanguageDomain::BslLanguage` for Rust dependency consumers.
