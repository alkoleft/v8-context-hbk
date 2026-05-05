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

Current status: T35-T65 and T68-T85 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design and cleanup
conclusions live in `acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`requirements/non-functional.md`, `implementation/components.md`,
`implementation/syntax-helper-query-cli.md`, `implementation/syntax-bsl-provider-plan.md` and
`implementation/solution-context-resolve.md`.

The active open tasks are T86-T90. T66 completed the required non-platform HBK domain-analysis gate
before T67. T67 completed the first resolver core and HBK-backed platform adapter slice. T89
completed the first shared language-fact extraction/index fixture slice. T90 is now the first
unchecked task. T90 is the remaining T66 follow-up for non-platform language resolver adapters.
T86-T88 are cleanup follow-ups from the May 2026 solution review and should not bypass the active
first unchecked task unless explicitly selected.

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

### [x] T66. Analyze non-platform Syntax Assistant domains from HBK books

Spec refs:

- ADR-0008
- UC-CTX-001
- FR-CTX-RESOLVE-001
- `spec/source-evidence.md`
- `spec/implementation/solution-context-resolve.md`

Problem:

- The current `syntax` export/index implementation extracts primarily platform API facts from
  `shcntx_*`.
- Installed HBK sources also contain BSL language syntax, query-language syntax and data
  composition system expression/query-extension syntax:
  - `shlang_ru.hbk` / `shlang_root.hbk`;
  - `shquery_ru.hbk` / `shquery_root.hbk`;
  - `dcsui_ru.hbk` / `dcsui_root.hbk`.
- A source-neutral resolver cannot correctly distinguish platform, BSL-language and query-language
  facts until these books are analyzed as separate source domains.

Scope:

- Inspect the TOC and representative pages of `shlang_*`, `shquery_*` and `dcsui_*` on the current
  platform baseline.
- Define which fact families should be extracted for:
  - BSL language constructs and language-level types;
  - query-language keywords, clauses, functions, operators and type/value facts;
  - data composition system expression language and query-language extension constructs.
- Decide whether these facts need new domain-specific model crates/export families/index document
  kinds, or whether a minimal shared language-fact model is enough.
- Record source-domain identity rules so same-display-name facts such as `Строка` remain distinct
  across platform API, BSL language and query language.
- Decide whether current `query_table`, `query_table_field` and `query_table_parameter` facts from
  `shcntx_*` become the first `QueryLanguage` resolver source, remain CLI-only provider facts for
  now, or require a separate domain-specific extraction/index shape after `shquery_*` and `dcsui_*`
  analysis.
- Add follow-up implementation tasks for the selected first extraction/indexing slice and for the
  first resolver adapter work that depends on those facts.
- Do not implement parsers, exports, resolver adapters, diagnostics or a public Rust API in this
  analysis task.

Verification:

- Updated requirements/implementation notes describe the selected source-domain boundaries and
  first extractable fact families.
- The model/export/index decision is explicit: domain-specific model crates/export families/index
  document kinds versus a minimal shared language-fact model.
- Updated UAT or acceptance notes name at least one real page from each source book family.
- Follow-up implementation task(s) are added with exact HBK fixtures, expected outputs and
  non-goals.
- No code changes are required for this task unless needed for read-only inspection tooling.

### [x] T67. Implement first Rust resolver core and platform adapter slice

Spec refs:

- ADR-0008
- UC-CTX-001
- FR-CTX-RESOLVE-001
- NFR-RESOLVE-001
- `spec/implementation/solution-context-resolve.md`
- `spec/implementation/components.md`

Scope:

- Add the source-neutral resolver core crate with typed ids, domains, fact kinds, query/response
  types, diagnostics, identity-preserving resolved wrappers and traits described by ADR-0008.
- Implement the first HBK-backed platform source adapter over `syntax-helper-search::SearchIndex`.
- Include explicit relation traversal in the platform adapter for `has_type`, `returns`,
  `constructs` and `member_of`.
- Add focused behavior tests proving source/domain ambiguity, BSL-vs-query type separation using
  fake providers, owner-id member lookup isolation, callable identity preservation and platform
  adapter lookup over an existing search-index fixture.
- Add an explicit platform callable adapter check for a constructor or method with ordered
  parameters and return or constructor type references, using a source-backed fixture selected by
  T66 or an existing stable search-index fixture.
- Do not expose existing `query_table`, `query_table_field` or `query_table_parameter` documents
  through the platform adapter. T66 selected them to remain CLI/provider facts for now; later
  language-domain work must define an explicit resolver mapping or relation shape first.
- Keep CLI JSON, SQLite public contracts, BSL parsing, query parsing, configuration/source parsing,
  diagnostics and service boundaries out of this task.

Verification:

- `cargo test -p <new-resolver-core-crate>`
- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- Tests demonstrate no hidden winner selection for same-name facts across domains or sources.
- BSL `Строка` and query-language `Строка` are separate `TypeId`s.
- Member listing by resolved owner id does not mix members from another source/domain with the same
  owner display name.
- Callable lookup preserves callable identity, ordered parameters and return or constructor type
  references.
- Platform adapter relation traversal preserves source-backed edges, including
  `НастройкиКомпоновкиДанных.Отбор` -> `ОтборКомпоновкиДанных` through `has_type` and one selected
  callable `returns` or `constructs` edge when the selected source fixture exposes it.
- A fake query table field can reference a BSL/query/platform type through an explicit relation.
- The platform adapter resolves `platform_type:ОтборКомпоновкиДанных`, lists its members and
  resolves the selected callable using a test index built through existing `syntax-helper-search`
  fixtures.
- NFR-RESOLVE-001 latency check measures exact type resolution, member listing, callable lookup and
  relation traversal after source open. Each operation should stay under the provisional `100 ms`
  target; if not, record the measured value, environment/input, suspected blocker and a follow-up
  task instead of adding cache/config work outside this task.

Completion notes:

- Added `context-resolver-core` as the source-neutral resolver core crate with typed ids, domains,
  fact kinds, query/response types, diagnostics, identity-preserving resolved wrappers and
  synchronous `ContextResolver` / `ContextSource` traits.
- Added `context-resolver-search` as the first HBK-backed platform adapter crate over
  `syntax-helper-search::SearchIndex`, keeping `syntax-helper-search` as the local index/query
  implementation rather than the generic resolver model.
- The adapter exposes platform type/member/callable facts and relation traversal for `has_type`,
  `returns`, `constructs` and `member_of`; existing `query_table`, `query_table_field` and
  `query_table_parameter` documents remain hidden from the platform adapter.
- Focused tests cover same-name ambiguity, preservation of source-level `ambiguous`/`unsupported`
  responses, BSL/query `Строка` type separation, owner-id member isolation, callable identity with
  ordered parameters and return/constructor type refs, explicit fake cross-domain type relation,
  platform adapter lookup over a `SearchIndex` fixture and the provisional `100 ms` latency target
  on exact type resolution, member listing, callable lookup and relation traversal.
- Verification passed:
  - `cargo test -p context-resolver-core`
  - `cargo test -p context-resolver-search`
  - `cargo test -p syntax-helper-search --lib`
  - `cargo test --workspace`

## Language Domain Follow-up

### [x] T89. Implement first shared language-fact extraction/index fixture slice

Spec refs:

- ADR-0008
- UC-CTX-001
- FR-CTX-RESOLVE-001
- `spec/source-evidence.md`
- `spec/implementation/solution-context-resolve.md`

Depends on:

- T66

Scope:

- Add a minimal shared language-fact model for non-platform HBK books with fact families selected
  by T66: `language_construct`, `language_type`, `language_function`, `language_operator`,
  `language_keyword` and `language_literal`.
- Add real-source parser fixtures from the current 8.5.1.1150 HBK pages:
  - `shlang_ru.hbk` `def_String` and `def_Func`;
  - `shlang_root.hbk` `def_String` and `def_Func`;
  - `shquery_ru.hbk` `SELECTStatement`, `SUM`, `STRING` and `LitString`;
  - `shquery_root.hbk` `SELECTStatement`, `SUM` and `STRING`;
  - `dcsui_ru.hbk` `SKD_Functions_Strings` and `SKD_ExtQueryLangv`;
  - `dcsui_root.hbk` `SKD_Functions_Strings` and `SKD_ExtQueryLangv`.
- Extract deterministic language facts from those fixtures, including source family, resolver
  language domain, fact family, localized name/alias when present, syntax text when present,
  parameters/return or type references when source-backed, description text and explicit source
  links as internal provenance.
- Add search/index document kinds for language facts so later resolver adapters can open a prebuilt
  local artifact without parsing HBK pages in lookup hot paths.
- Keep the existing `syntax export` platform consumer JSON unchanged.

Expected outputs:

- Behavior tests or snapshot assertions prove:
  - `shlang:def_String` is a `BslLanguage` `language_type`;
  - `shlang:def_Func` is a `BslLanguage` `language_construct` with syntax text;
  - `shquery:SELECTStatement` is a `QueryLanguage` `language_construct` or `language_keyword`
    root for the `ВЫБРАТЬ` / `SELECT` clause;
  - `shquery:STRING` is a `QueryLanguage` `language_function`;
  - `shquery:LitString` is a `QueryLanguage` `language_literal` or `language_type` according to
    the implemented source-backed classifier;
  - `dcsui:SKD_Functions_Strings#ДлинаСтроки` or the equivalent root-source anchor is a
    `QueryLanguage` language function under the distinct `dcsui` source family;
  - `dcsui:SKD_ExtQueryLangv` exposes query-extension constructs such as `{ВЫБРАТЬ}` and `{ГДЕ}`
    without overwriting base `shquery` clauses.
- Identity assertions prove same-display-name `Строка` / `String` facts from BSL, query and SKD
  sources remain separate ids.

Non-goals:

- Do not expose a new public language export JSON contract unless a spec update in this task
  explicitly defines it.
- Do not implement a BSL parser, query parser, analyzer diagnostics, runtime 1C introspection,
  MCP, network search, graph database or storage-selection knobs.
- Do not expose raw HBK path, TOC path, HTML path or page title in consumer export JSON.
- Do not move existing `shcntx_*` `query_table` facts into the resolver in this task.

Verification:

- `cargo test -p <language-fact-model-or-extract-crate>`
- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- Spec/baseline updated with implemented schema/fact-family conclusions and fixture anchors.

Completion notes:

- Added `syntax-helper-language` as the first shared language-fact model/parser crate for
  non-platform HBK pages. It extracts source-qualified language facts from committed real-source
  fixtures without adding those facts to `PlatformContext` or `syntax export` consumer JSON.
- Added T89 fixtures from current `8.5.1.1150` HBK pages for `shlang_ru/root` `def_String` and
  `def_Func`, `shquery_ru/root` `SELECTStatement`, `SUM` and `STRING`, `shquery_ru` `LitString`,
  and `dcsui_ru/root` `SKD_Functions_Strings` and `SKD_ExtQueryLangv`.
- Added language document kinds to `syntax-helper-search`: `language_type`, `language_construct`,
  `language_function`, `language_operator`, `language_keyword` and `language_literal`. The first
  tests cover `shlang:def_String`, `shlang:def_Func`, `shquery:SELECTStatement`, `shquery:STRING`,
  `shquery:LitString`, `dcsui:SKD_Functions_Strings#StringLength` and SKD query-extension keywords
  `{ВЫБРАТЬ}` / `{ГДЕ}`.
- Same-display-name `Строка` facts from BSL, query function and query literal fixtures stay
  separate source-qualified ids. Existing `shcntx_*` `query_table`, `query_table_field` and
  `query_table_parameter` provider facts remain outside the resolver/language source.
- Verification passed:
  - `cargo test -p syntax-helper-language`
  - `cargo test -p syntax-helper-search --lib`
  - `cargo test --workspace`

### [ ] T90. Implement first language-domain resolver adapter slice

Spec refs:

- ADR-0008
- UC-CTX-001
- FR-CTX-RESOLVE-001
- NFR-RESOLVE-001
- `spec/implementation/solution-context-resolve.md`

Depends on:

- T67
- T89

Scope:

- Add resolver source adapter(s) for the T89 language-fact index shape without changing the
  source-neutral resolver core model selected by T67.
- Resolve exact ids and exact names for at least BSL primitive language types, query-language
  functions/literals and SKD expression/query-extension facts.
- Preserve source-family identity for `shlang`, `shquery` and `dcsui` facts under resolver
  `LanguageDomain::BslLanguage` and `LanguageDomain::QueryLanguage`.
- Add explicit relation traversal only for source-backed links extracted in T89, such as query
  function parameter/return type links to BSL/query language type facts.
- Leave existing `shcntx_*` `query_table`, `query_table_field` and `query_table_parameter`
  documents outside the resolver unless this task adds an explicit source-backed relation from a
  language fact to a query-table provider fact.

Expected outputs:

- Resolver tests prove:
  - unconstrained exact-name lookup for `Строка` returns `ambiguous` when BSL, query and SKD/source
    candidates are active;
  - constraining to `BslLanguage` resolves the BSL `def_String` type;
  - constraining to `QueryLanguage` can distinguish `shquery:STRING`, `shquery:LitString` and SKD
    string-function facts by exact id or fact family;
  - relation traversal from a query/SKD function parameter or return type uses explicit extracted
    edges instead of same-name merging.

Non-goals:

- Do not implement BSL/source-code parsing, query-text parsing, analyzer diagnostics or project
  metadata extraction.
- Do not add a public service boundary, async runtime, global cache, MCP server or downstream
  analyzer implementation.
- Do not expose SQLite tables, FTS tokens or raw HBK/TOC/HTML/page-title provenance as public
  resolver facts.

Verification:

- `cargo test -p <resolver-core-crate>`
- `cargo test -p <language-adapter-or-search-crate>`
- `cargo test --workspace`
- NFR-RESOLVE-001 latency notes for exact language fact lookup after source open, or a recorded
  measured blocker and follow-up task.

## Cleanup Follow-up

### [ ] T86. Audit and narrow stale public convenience APIs

Spec refs:

- ADR-0002
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`

Problem:

- Several public convenience APIs appear to be unused by production code in this workspace, including
  broad extraction helpers and thin book/container wrappers.
- Public surface that is only test-facing makes provisional contracts harder to reason about before
  the resolver and non-platform HBK work expands the project.
- The solution review also found public serializable search DTOs and test-only in-memory bridge
  helpers that may be legacy-shaped implementation details rather than supported library contracts.

Scope:

- Audit public functions and reexports that have no production call sites, starting with
  `SyntaxHelperReader::discover_roots`, `SyntaxHelperReader::extract`,
  `extract_with_loader`, `extract_with_loader_into`, broad `syntax-helper-extract` reexports,
  and thin `hbk-book` / `hbk-container` convenience wrappers.
- Include `syntax-helper-search` public convenience surfaces in the audit, especially
  `SearchHit`/`SearchDocument`/`RelatedHit` serialization, exact lookup helpers over primitive
  owner/member strings, and the test-side `PlatformContext` to `SearchIndexBuilder` bridge.
- For each candidate, decide whether it is a supported component contract, a test utility, or
  removable legacy surface.
- Move test-only helpers to test/support modules or crate features where practical.
- Keep the audit/change narrow; do not remove user-facing CLI commands, provider JSON envelopes,
  documented behavior, SQLite schema or resolver planning contracts.

Verification:

- `spec/implementation/components.md` records every retained supported public component contract and
  every removed/narrowed public contract decision.
- Task notes may list reviewed candidates that require no durable component-contract change, such as
  test utilities, no-op decisions or candidates intentionally left for later review, but they must
  not be the only record for retained/removed public contract decisions.
- `cargo test --workspace`

### [ ] T87. Classify residual duplicate query and provider mechanisms

Spec refs:

- ADR-0006
- ADR-0007
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/archive/completed-tasks-t57-t65-t68-t85.md`

Problem:

- The solution review found mechanisms that still look duplicated after completed cleanup tasks:
  `syntax get` classification/execution/provider-status mapping, search lookup key normalization,
  provider DTO shaping, `display_name` helpers and link/path normalization.
- T71, T72, T73, T77, T78 and T85 are already archived as completed cleanup work. Before adding
  more cleanup implementation, each residual candidate must be classified as accepted boundary
  separation, stale duplication, T86 public-surface follow-up, or no-op due archived evidence.

Scope:

- Review the residual candidates from the May 2026 solution review:
  - CLI `syntax get` classifier, execution lookup and provider status/result mapping;
  - `syntax-helper-search` name/owner/member/relation key normalization helpers;
  - public serializable provider/search DTO shape versus CLI provider JSON envelopes;
  - duplicated `display_name` presentation helpers;
  - HBK page path, documentation link and Syntax Assistant member-link normalization.
- For each candidate, record the classification and selected action. Accepted boundary separation and
  durable cleanup boundary decisions must be recorded in `spec/implementation/components.md`; T86
  follow-up, new cleanup task and no-op classifications may remain in this task ledger.
- Add a new narrow follow-up task only when the residual candidate is demonstrably stale and not
  already covered by T86 or the archived cleanup tasks.
- Do not re-open completed T71/T72/T73/T77/T78/T85 without new evidence.
- Do not change CLI text/JSON output, provider envelopes, SQLite schema, export schema, extraction
  behavior or resolver contracts in this classification task.

Verification:

- `spec/implementation/components.md` lists every reviewed residual candidate classified as an
  accepted boundary or durable cleanup boundary decision.
- `spec/IMPLEMENTATION_TODO.md` may list residual candidates classified as T86 follow-up, new cleanup
  task or no-op, but not as the only durable record for accepted boundary classifications.
- `git diff --check`
- If the classification creates code follow-up work, the new task names the targeted tests/UATs that
  must pass when that follow-up is implemented.

### [ ] T88. Fix current-toolchain clippy drift in `syntax-helper-extract`

Spec refs:

- NFR-TEST-001
- `spec/implementation/components.md`

Problem:

- The solution review found `cargo clippy --workspace --all-targets -- -D warnings` failing on the
  current toolchain in `syntax-helper-extract`.
- Earlier dependency/clippy cleanup is archived under T77, so this task is a narrow lint-drift fix,
  not a broad dependency or style cleanup.

Scope:

- Fix the current clippy failures in `crates/syntax-helper-extract/src/reader.rs`, including
  replacing the flagged `let...else` patterns with `?` where behavior is unchanged and keeping test
  modules after non-test items.
- Keep this task mechanical and behavior-preserving.
- Do not run broad clippy-driven refactors outside the currently failing diagnostics.

Verification:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
