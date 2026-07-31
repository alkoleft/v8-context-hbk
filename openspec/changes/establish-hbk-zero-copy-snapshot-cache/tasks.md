> **BOUNDED T183 EXPERIMENT / NO PRODUCTION SELECTION**
>
> Tasks in section 1 are executable under durable task T183. Sections
> 2–6 are a non-executable follow-up backlog: their items deliberately are not
> checkboxes and SHALL NOT be selected or implemented until task 1.15 is
> accepted in the durable HBK specification or ADR, the user has selected an
> outcome and production implementation is explicitly authorized.

## 1. Discovery And Architecture Gate

Tasks 1.1–1.6 are ordered prerequisites. Candidate prototype tasks 1.7–1.12
SHALL NOT start until the comparison protocol, current baselines, numerical
gates, inventory, lifecycle and behavior oracle have been recorded and the
durable task ledger authorizes the bounded prototype work.

- [x] 1.1 Freeze a reproducible comparison protocol before accepting candidate
  results: exact HBK/provider artifact identity and checksum, platform version,
  locale, extraction-schema version, build profile, host/OS, commands, run
  count, warm-up policy, cold-cache method, summary statistic, raw-result
  format and measurement-tool fallbacks.
- [x] 1.2 Capture separate baselines and run-to-run noise for SQLite-to-owned
  materialization as the SQL baseline row,
  current-binary-cache-to-owned cold/warm open and representative
  post-workload steady state on the selected datasets.
- [x] 1.3 Define task-local numerical material-benefit and non-regression
  thresholds for production/rebuild, cold and warm ready-for-query startup
  under the frozen protocol boundary, first lookup, batched warm lookup,
  allocations, peak/steady RSS, PSS, aggregate multi-process PSS, page
  faults/bytes touched and snapshot size before using prototype results to
  choose a format.
- [x] 1.4 Inventory the current SQLite-to-binary-cache-to-runtime data flow,
  including every materialized `String`, `Vec`, lookup index and duplicated
  live representation.
- [x] 1.5 Decide and document the snapshot production lifecycle: build-time,
  release artifact, installation step, or first-run rebuildable cache.
- [x] 1.6 Freeze the behavior oracle: normalize session-local numeric IDs
  through logical fact identity/text; compare full-corpus fact families and
  observable fields; cover exact/name/owner/callable/global/module/language/
  enum/query/availability/relation lookups; preserve hit, miss, ambiguity,
  unsupported and deterministic-order behavior; and explicitly decide whether
  the canonical snapshot includes documentation beyond current
  snapshot/catalog fields.
- [x] 1.7 Record the hypothesis registry, branch ancestry and result-table
  template before prototyping: H0 SQL-to-owned baseline, C0 current cache
  control, H1 custom flat mapped sections, H2 “H1 layout + typed reader” with
  `zerocopy` only if H1 exposes decoding cost, H3 archive candidate such as
  `rkyv`/`bytecheck`, and separate reverse-index sub-variants.
- [x] 1.8 Prototype H1 as a custom flat, sectioned snapshot with checked
  offsets, flat arenas, ranges, sorted lookup indexes and an interned HBK string
  dictionary.
- [x] 1.9 Prototype H2 from the exact measured H1 commit only when H1
  measurements justify isolating fixed-record decode overhead; report it as
  “H1 layout + typed reader”, otherwise record the no-go evidence instead of
  adding the dependency.
- [x] 1.10 Prototype H3 as an archive candidate, including
  `rkyv`/`bytecheck` or an equivalent format, against the same catalog and
  lookup contracts. Do not call it validated until the checked safe-access
  boundary and compatibility proof pass.
- [ ] 1.11 Evaluate reverse string lookup implementations for the immutable
  HBK dictionary, starting with sorted indexes, then mapped hash indexes, and
  adding FST only when the simpler variants leave a measured lookup or size
  problem. The sorted H1/H2 variants and linear H3 variant are measured; H2
  still misses the reverse-hit gate, so a mapped-hash follow-up remains
  unresolved. FST remains unjustified until that simpler follow-up is measured.
- [x] 1.12 Specify snapshot binary-layout and extraction-schema compatibility,
  exact platform-version/source checks, structural validation, immutable
  publication, shared session-long reader locks, fail-fast exclusive writer
  locking, the stable logical snapshot-slot lock key and protected discovery
  metadata/current-pointer operations, mapping lifetime and the invariant that
  a mapped snapshot is never modified.
- [ ] 1.13 Measure every viable candidate against the frozen protocol and produce
  one comparison table with SQL-to-owned as baseline, current-cache-to-owned as
  control and each zero-copy hypothesis as a separate row. Report
  production/rebuild, cold/warm ready-for-query startup, first lookup, batched
  warm lookup, page faults/bytes touched, allocations, peak/steady RSS, PSS,
  aggregate multi-process PSS, file/section/index sizes and post-workload
  retained state. Record harness/candidate commit SHAs and branch ancestry, and
  do not rank or name a winner. The consolidated table is recorded, but
  candidate production allocations and per-section/dictionary/index byte
  footprints were not instrumented and remain evidence gaps.
- [x] 1.14 Define the ownership boundary between the provider-owned immutable
  HBK base dictionary and a downstream request/project-scoped overlay for BSL
  and metadata strings; do not add the overlay to HBK.
- [ ] 1.15 Present the unranked evidence to the user. Only after the user
  selects an outcome, accept or reject the snapshot format, canonical runtime
  promotion and base-dictionary handoff in the durable HBK specification or ADR. On
  acceptance, SQLite may remain only in its explicitly accepted private
  rebuild/index-production role. Retain every candidate branch and its evidence
  until the user decides, including candidates that fail a mandatory gate.
- [x] 1.16 Freeze the independent S83 comparison set for exact platform
  `8.3.27.1859`: separate service-data root, exact HBK/provider identities,
  parameterized harness commit, host-load evidence, H0/C0 noise and concrete
  numerical gates. Do not reuse S85 values.
- [ ] 1.17 Implement corrected S83-F0 typed-flat and S83-A0 checked-archive
  format/lifecycle references in separate worktrees. Require exact platform
  checks, immutable locked publication, rebuild-before-map, complete mapped
  canonical parity, section footprints and producer allocation evidence.
- [ ] 1.18 Implement S83-L1 page layout, S83-I1 mapped indexes, S83-D1 checked
  dynamic reading and S83-P1 direct formation in separate F0-derived
  branches/worktrees. Change only the registered primary variable in each
  branch and implement in parallel while serializing all performance runs.
- [ ] 1.19 Run the complete S83 parity/lifecycle/resource protocol and add the
  rows to one unranked comparison table. Preserve every branch, record
  incomplete/failed gates and wait for the user's selection before any merge,
  deletion, dependency acceptance or canonical promotion.

## 2. Provider Prerequisites (Non-Executable Follow-Up)

- 2.1 Define the selected snapshot header, section directory, compatibility
  metadata, validation rules and corruption errors.
- 2.2 Define borrowed provider views that preserve existing catalog
  semantics without reconstructing entity-shaped owned records.
- 2.3 Define generation-scoped base string identifiers and make their
  session lifetime explicit; do not serialize, migrate or compare numeric IDs
  across snapshot sessions and do not treat them as durable entity identity.
- 2.4 Define the single canonical snapshot production path from accepted
  HBK/provider build inputs without introducing a second runtime source of
  truth.

## 3. Zero-Copy Snapshot Implementation (Non-Executable Follow-Up)

- 3.1 Implement deterministic snapshot production and validation in the
  HBK provider.
- 3.2 Implement immutable memory-mapped loading with checked bounds,
  alignment and section validation before typed access.
- 3.3 Implement borrowed catalog, signature, parameter, property and
  accepted documentation-field access over the selected layout.
- 3.4 Implement base dictionary resolution and reverse lookup required by
  provider queries without allocating entity-shaped DTOs.
- 3.5 Implement shared session-long reader locking and fail-fast exclusive
  writer locking around safe rebuild/publication; return a typed
  snapshot-in-use error instead of modifying an active mapped artifact.

## 4. Catalog Migration And Duplicate Removal (Non-Executable Follow-Up)

- 4.1 Migrate each runtime catalog consumer from heap-deserialized storage
  to borrowed snapshot views while preserving observable query behavior.
- 4.2 Remove the replaced heap runtime model, duplicate indexes and
  multi-hop conversions; do not retain a compatibility mirror.
- 4.3 Add a structural reintroduction guard proving that one process does
  not keep both the mapped snapshot and a fully materialized HBK catalog.

## 5. Downstream Base-Dictionary Handoff (Non-Executable Follow-Up)

- 5.1 Expose only the minimal borrowed base-dictionary boundary accepted by
  the downstream semantic model; keep project overlay ownership outside HBK.
- 5.2 Verify that BSL and metadata overlays can compare and resolve names
  against the HBK base without copying the HBK dictionary or leaking
  provider-private storage types.
- 5.3 Record the accepted dependency outcome in
  `v8-context/openspec/changes/establish-unified-semantic-entity-model`.

## 6. Verification And Completion (Non-Executable Follow-Up)

- 6.1 Add compatibility/lifecycle tests for wrong magic, binary-layout
  version, extraction-schema version, source identity, locale and platform
  version; truncated/corrupt sections; concurrent readers; fail-fast locked
  updates; publication; reopen; and a new session-local ID space after source
  replacement.
- 6.2 Add full-corpus parity tests covering logical fact counts/sets and
  every observable field for platform types, members, callables, constructors,
  overloads, signatures, parameters, globals, module contexts/events, language
  facts, enums/values and SDBL tables/fields/parameters.
- 6.3 Add lookup parity tests for exact identity, type name/alias,
  template key, owner/member/kind, callable/constructor, global,
  module-context/event, language, enum/value, query-table/field/parameter,
  availability and relation lookup, including hit, miss/empty, multiple
  candidates, ambiguity, unsupported and deterministic ordering.
- 6.4 Run parity through `HbkFactReadHandle`, the borrowed BSL/SDBL
  catalogs, `PlatformSnapshotSource` and `QueryTableSnapshotSource`; repeat
  sequentially and concurrently; then make the SQLite/HBK sources unavailable
  to the running probe and prove there is no hidden runtime fallback.
- 6.5 Re-run the frozen comparison protocol for production/rebuild,
  cold/warm startup, first lookup, batched warm lookup, page faults/bytes
  touched, allocation count/bytes, peak/steady RSS, PSS, aggregate
  multi-process PSS, post-workload retained state, file/section/index sizes and
  base-dictionary lookup.
- 6.6 Apply the predeclared acceptance gates and present the unranked result
  table. Promote a candidate to the single canonical HBK runtime context
  artifact and remove the replaced owned runtime path only after the user
  selects it and the durable decision is accepted; otherwise retain the
  current canonical runtime path. Do not delete experimental branches or
  evidence before that decision.
- 6.7 Update the HBK durable requirements, acceptance baseline,
  implementation specification, ADRs and `spec/IMPLEMENTATION_TODO.md`, plus
  record the accepted dependency outcome in the downstream unified semantic
  entity change.
- 6.8 Review the final diff for duplicate models, dictionaries, indexes,
  loaders, fallback reads and conversion chains before marking the change
  complete.
