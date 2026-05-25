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

Current first unchecked task: none.

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
