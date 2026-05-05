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

The active open tasks are T91-T103. T66 completed the required non-platform HBK domain-analysis gate
before T67. T67 completed the first resolver core and HBK-backed platform adapter slice. T89
completed the first shared language-fact extraction/index fixture slice. T90 completed the remaining
T66 follow-up for non-platform language resolver adapters. T87 completed the residual
query/provider mechanism classification. T88 completed the current-toolchain clippy drift fix. T91
is now the first unchecked planned task; T98-T103 are user-requested book-content export slices
handled outside the first-unchecked cleanup/provider sequence.

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

### [x] T90. Implement first language-domain resolver adapter slice

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

Completion notes:

- Added `LanguageSearchSource` in `context-resolver-search` for source-specific `shlang`,
  `shquery` and `dcsui` adapters over the prebuilt `syntax-helper-search::SearchIndex`.
  `shlang` maps to `BslLanguage`; `shquery` and `dcsui` map to `QueryLanguage` while preserving
  distinct source ids.
- Kept the source-neutral resolver core model intact. The only core behavior change is composition
  hygiene: when one source reports `ambiguous`, already found `ok` facts from other active sources
  are preserved as ambiguity candidates instead of being dropped.
- Made T89 language facts resolver-usable in the search index by preserving extracted
  `type_refs` / `return_types`, normalizing `language_function` signatures and parameters as
  callable rows, and deriving relation rows from explicit extracted type references. No SQLite
  schema version, public table contract, CLI JSON, consumer export JSON or query-table resolver
  exposure was added.
- Focused tests prove unconstrained `Строка` ambiguity across active language sources, constrained
  BSL `def_String` lookup, exact id distinction for query `STRING`, query `LitString` and SKD
  `SKD_Functions_Strings#StringLength`, and explicit SKD parameter-type relation traversal to
  `shlang:def_String`.
- NFR-RESOLVE-001 focused checks for exact BSL type lookup and SKD relation traversal stayed under
  `100 ms` after source open.
- Verification passed:
  - `cargo test -p context-resolver-core`
  - `cargo test -p context-resolver-search`
  - `cargo test -p syntax-helper-search --lib`
  - `cargo test --workspace`

## Cleanup Follow-up

### [x] T86. Audit and narrow stale public convenience APIs

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

Completion notes:

- Narrowed `syntax-helper-extract` crate-root exports to the supported facade:
  `SyntaxHelperReader` plus error types. Parser functions, root discovery loader helpers,
  `extract_with_loader*` and materializing `SyntaxHelperReader::extract` are now internal/test
  support; production CLI/index/export flows continue to use `SyntaxHelperReader::extract_into()`
  over `SyntaxHelperSink`.
- Kept `syntax-helper-model` as the domain-model import surface instead of reexporting the entire
  model through `syntax-helper-extract`.
- Recorded thin synthetic/test convenience wrappers as outside the ordinary public contract:
  `HbkContainer::from_bytes` already used the test/test-utils boundary, and `HbkBook::read_pages`
  now uses the same boundary. Ordinary supported callers use `HbkContainer::open`,
  `HbkBook::read_file`, `HbkBook::read_page` and `FileStorageReader`.
- Removed serde serialization from `syntax-helper-search` `SearchHit`, `SearchDocument`,
  `RelatedHit` and `RelationStep`. They remain Rust query result structs for search/resolver
  adapters; public provider JSON continues to be assembled explicitly in `v8-context-hbk-cli` from
  normalized index facts and export-compatible field shapes.
- Reviewed exact lookup helpers: `get_by_owner_member`, `member_by_owner_type_id`,
  `callable_by_owner_type_id`, `owner_type_id_for_document` and `target_type_ids_for_document`
  remain supported component primitives because CLI/provider and resolver adapters use them.
  Test-side `PlatformContext` to `SearchIndexBuilder` bridge helpers stay private test helpers.
- Verification passed:
  - `cargo test -p syntax-helper-extract --lib`
  - `cargo test -p syntax-helper-search --lib`
  - `cargo test -p hbk-container --lib`
  - `cargo test -p hbk-book --lib`
  - `cargo test --workspace`

### [x] T87. Classify residual duplicate query and provider mechanisms

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

Completion notes:

- Classified `syntax get` root classification, lookup execution and provider status/result mapping
  as accepted CLI-boundary separation. T71 already removed the stale duplicated classifier/lookup
  tuple matching, so no new cleanup task is needed.
- Classified `syntax-helper-search` lookup-key normalization as accepted search-index boundary
  behavior. It is intentionally separate from export DTO shaping and from HBK/documentation path
  normalization.
- Classified provider/search DTO shaping as accepted CLI/provider boundary behavior:
  `SearchHit`/`SearchDocument`/`RelatedHit`/`RelationStep` remain Rust query result structs, while
  `v8-context-hbk-cli` owns the public provider JSON envelope and export-compatible fact shape.
  T72 and T86 already covered the stale adapter/serde public-surface cleanup.
- Classified duplicated `display_name` helpers as stale presentation duplication, not identity,
  provider JSON or lookup contracts. Added T91 as the narrow follow-up because this exact localized
  display rule remains duplicated in `syntax-helper-search` and `v8-context-hbk-cli`.
- Classified HBK storage path, documentation link-target and Syntax Assistant member-link
  normalization as distinct accepted component boundaries. T73 already consolidated only the shared
  storage/page normalization layer.
- No T86 follow-up was created. The only new cleanup follow-up is T91 for the stale duplicated
  localized display-name presentation helper; all other reviewed residuals are accepted boundary
  separation or already covered by archived cleanup evidence.
- Verification passed:
  - `git diff --check`
  - `cargo test --workspace`

### [x] T88. Fix current-toolchain clippy drift in `syntax-helper-extract`

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

Completion notes:

- Reproduced the current-toolchain clippy drift in `syntax-helper-extract`: `question_mark` warnings
  for `query_table_identifier` and `items_after_test_module` in `reader.rs`.
- Replaced the behavior-preserving `let...else` early `None` returns with `?` and moved the
  `reader.rs` test module after non-test helpers. Parser behavior, public contracts, fixtures,
  provider JSON, export JSON and resolver facts were unchanged.
- Verification passed:
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`

### [ ] T91. Collapse duplicated localized display-name presentation helper

Spec refs:

- T87
- `spec/implementation/components.md`

Problem:

- T87 classified most duplicate-looking mechanisms as accepted boundary separation or already
  covered by archived cleanup evidence.
- The remaining stale duplication is a small localized-name `display_name` helper duplicated in
  `syntax-helper-search` and `v8-context-hbk-cli`.
- The helper is presentation logic only, but keeping the same rule in two crates makes later
  presentation changes easy to drift.

Scope:

- Move the localized-name display rule to one narrow shared helper in an existing crate that both
  `syntax-helper-search` and `v8-context-hbk-cli` already depend on.
- Replace the duplicated local helpers with calls to the shared helper.
- Preserve current human text output, search document text, relation labels and provider JSON.
- Do not change lookup keys, search ranking, provider envelopes, export JSON, resolver facts,
  SQLite schema or public identity rules.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

### [ ] T92. Remove hidden resolver edge and constructor return fallbacks

Spec refs:

- ADR-0008
- FR-CTX-RESOLVE-001
- `spec/implementation/solution-context-resolve.md`

Problem:

- The platform resolver adapter still has fallback behavior after the first source-backed edge
  implementation:
  - `edge_refs()` falls back from edge-specific traversal to generic `related_by_id()` filtering;
  - `related()` repeats the same generic traversal fallback;
  - constructor callable mapping synthesizes a return type from the owner when explicit
    `constructs`/return data is absent.
- ADR-0008 and the resolver implementation notes require identity-preserving, source-backed
  relation traversal. Hidden fallback edges make it harder to distinguish missing source evidence
  from real resolver facts.

Scope:

- Remove the generic traversal fallback from platform adapter edge-specific relation lookup.
- Remove constructor return-type synthesis from owner name unless a spec update explicitly records
  it as a source-backed rule.
- Adjust tests so missing `returns`/`constructs` evidence is observable as missing relation/return
  data, not as a synthesized fact.
- Preserve current source/domain identity, query-table exclusion, resolver public types and
  `syntax-helper-search` relation storage.

Verification:

- `cargo test -p context-resolver-search`
- `cargo test --workspace`

### [ ] T93. Keep provider JSON assembly out of `syntax-helper-search` structs

Spec refs:

- ADR-0007
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`

Problem:

- T87 classifies provider JSON DTO shaping as a CLI boundary, but `SearchSignature` and
  `SearchParameter` still derive serde traits and `v8-context-hbk-cli` serializes
  `SearchDocument.signatures` directly into provider JSON.
- A `syntax-helper-search` test still describes structured signatures as public JSON. That keeps a
  stale provider DTO concern in the search/index crate after T72/T86 cleanup.

Scope:

- Remove serde derives and serde field attributes from search result nested structs when no longer
  needed by `syntax-helper-search`.
- Move provider signature/parameter JSON assembly into `v8-context-hbk-cli`, preserving the current
  provider envelope and export-compatible fact field names.
- Update tests so provider JSON shape is asserted at the CLI/provider boundary, while
  `syntax-helper-search` tests assert Rust query structs and normalized storage behavior.
- Do not change provider response schema version, SQLite schema, search ranking, lookup behavior or
  export JSON.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

### [ ] T94. Deduplicate search relation graph construction

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-PROVIDER-001
- `spec/implementation/components.md`

Problem:

- `syntax-helper-search` keeps production relation insertion and the test-only
  `relations_from_documents()` helper as two separate implementations of the same relation graph
  rules.
- Tests that call the duplicate helper can drift from the SQLite insertion path and accidentally
  verify a copied algorithm instead of the actual indexed relation output.

Scope:

- Extract one narrow relation-building function or iterator that produces typed relation rows from
  search documents.
- Use that shared builder for SQLite relation insertion and focused tests.
- Preserve current relation edge kinds, labels, evidence, weights and deduplication semantics.
- Do not change SQLite schema, query result ordering, provider JSON or resolver facts.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test -p context-resolver-search`
- `cargo test --workspace`

### [ ] T95. Replace stringly search document kinds with a typed internal kind

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-PROVIDER-001
- FR-CTX-RESOLVE-001
- `spec/implementation/components.md`

Problem:

- Search document kinds such as `platform_type`, `type_property`, `language_function` and
  `query_table_field` are repeated as raw strings across search indexing, kind priority,
  language-fact mapping, CLI filtering and resolver adapter mapping.
- This duplicates the same closed set of internal index document families and makes new fact
  families easy to add inconsistently.

Scope:

- Introduce an internal typed document-kind model in `syntax-helper-search` with explicit string
  conversion at SQLite/provider boundaries.
- Replace local string matches in search/index logic and resolver adapter mapping where the typed
  kind can cross the crate boundary without expanding public JSON contracts.
- Preserve existing stored string values, provider `kind` values, search ordering, resolver fact
  mapping and export JSON.
- Do not turn this into a generic cross-domain ontology or public stable enum unless the spec is
  updated first.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test -p context-resolver-search`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

### [ ] T96. Narrow or retire `HbkBook::read_pages` test-support API

Spec refs:

- FR-HBK-002
- FR-HBK-003
- `spec/implementation/components.md`

Problem:

- The supported ordinary page/file surface is `read_file`, `read_page` and `FileStorageReader`.
  `HbkBook::read_pages` is currently gated behind `test-utils`, and the only in-repo use is a
  deterministic unit test.
- Keeping this as a public feature API makes a non-contract convenience method look like a supported
  book-reading surface.

Scope:

- Either move `read_pages` into test-only support or remove it in favor of direct
  `FileStorageReader` usage in tests.
- Preserve production `HbkBook::open`, `read_file`, `read_page` and `FileStorageReader` behavior.
- Do not reintroduce retained `FileStorage` bytes or broader in-memory book state.

Verification:

- `cargo test -p hbk-book`
- `cargo test --workspace`

### [ ] T97. Deduplicate first language callable parsing paths

Spec refs:

- ADR-0008
- FR-CTX-RESOLVE-001
- `spec/implementation/solution-context-resolve.md`

Problem:

- `syntax-helper-language` now has two similar callable extraction paths:
  `query_function_fact()` for `shquery_*` functions and `extract_dcsui_string_functions()` for the
  first SKD string function fixture.
- Both produce `LanguageFactFamily::Function` facts with syntax, signatures, parameters and return
  or type references, but the DCSUI path uses ad hoc body slicing.

Scope:

- Extract a small shared helper for language callable fact assembly where the source shapes already
  match.
- Keep source-family-specific page discovery, page-key matching and fixture expectations separate.
- Preserve current source-qualified ids, language domains, fact families, signatures, parameter
  names, return/type refs and provenance.
- Do not expand parser coverage beyond the existing fixture-backed pages in this task.

Verification:

- `cargo test -p syntax-helper-language`
- `cargo test -p syntax-helper-search --lib`
- `cargo test -p context-resolver-search`
- `cargo test --workspace`

### [ ] T98. Rename Syntax Assistant export crate to `hbk-syntax-export`

Spec refs:

- ADR-0009
- FR-EXPORT-001
- `spec/implementation/components.md`

Scope:

- Rename the existing Syntax Assistant JSON export crate from `hbk-export` to
  `hbk-syntax-export`.
- Update workspace membership, dependency aliases, package names and Rust imports.
- Preserve `v8-context-hbk syntax export` behavior and `schema_version` unchanged.
- Do not add ordinary book export behavior in this task.

Verification:

- `cargo test -p hbk-syntax-export`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

### [ ] T99. Add `hbk-book-export` crate boundary and request model

Spec refs:

- UC-HBK-003
- FR-HBK-004
- ADR-0009
- `spec/implementation/components.md`

Scope:

- Add a separate `hbk-book-export` crate for ordinary book-content export responsibility.
- Define typed request, format, hierarchy, result and error types.
- Implement safe output-root validation and unsupported format/hierarchy diagnostics.
- Keep the crate dependent only on `hbk-book`, `hbk-docs` and narrow utility dependencies needed by
  export behavior.
- Do not wire the CLI command and do not implement actual file export in this task.

Verification:

- Focused tests for request validation, output-root safety and unsupported combinations.
- Dependency-boundary check proves `hbk-book-export` depends only on `hbk-book`, `hbk-docs` and
  narrow utility dependencies, and does not depend on Syntax Assistant extraction,
  `hbk-syntax-export`, search/index or resolver crates (`cargo tree -p hbk-book-export` or
  equivalent `cargo metadata` query).
- `cargo test -p hbk-book-export`
- `cargo test --workspace`

### [ ] T100. Implement raw/raw book unpack in `hbk-book-export`

Spec refs:

- UC-HBK-003
- FR-HBK-004
- FR-HBK-002
- ADR-0009
- `spec/implementation/components.md`

Scope:

- Implement `format=raw` with `hierarchy=raw` as normalized `FileStorage` unpacking.
- Preserve original stored bytes for exported entries.
- Reject unsafe or escaping storage paths before writing.
- Keep TOC traversal and Markdown conversion out of this task.

Verification:

- Focused tests for raw/raw export behavior on a small deterministic HBK fixture or small real HBK
  source.
- `cargo test -p hbk-book`
- `cargo test -p hbk-book-export`
- `cargo test --workspace`

### [ ] T101. Wire top-level raw export CLI and unsupported matrix diagnostics

Spec refs:

- UC-HBK-003
- FR-HBK-004
- FR-CLI-001
- ADR-0009

Scope:

- Add a top-level `v8-context-hbk export <book.hbk> --output <dir> --format <raw|markdown>
  --hierarchy <raw|toc>` command.
- Wire `format=raw --hierarchy=raw` through `hbk-book-export`.
- Return stable readable diagnostics for unsupported combinations until later tasks implement them.
- Keep `syntax export` unchanged; book-content export must not invoke Syntax Assistant extraction or
  change JSON schema contracts.

Verification:

- Focused CLI tests for raw/raw success and unsupported combination diagnostics.
- `cargo test -p hbk-book-export`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

### [ ] T102. Implement Markdown conversion for TOC pages

Spec refs:

- UC-HBK-003
- FR-HBK-004
- FR-DOC-001
- ADR-0009
- UAT-HBK-005
- UAT-HBK-006
- UAT-HBK-007
- `spec/implementation/components.md`

Scope:

- Select and add the approved stable HTML-to-Markdown library candidate.
- Implement HTML-to-Markdown conversion for individual TOC pages inside `hbk-book-export`.
- Preserve readable page headings, body text, links, lists, tables and syntax placeholders.
- Ensure normal Markdown output does not leak raw HBK file paths, raw TOC indexes, raw HTML page
  paths or service HTML scaffolding.
- Keep output directory layout and CLI UAT wiring out of this task unless needed for focused tests.

Verification:

- Focused conversion tests based on representative real HBK page HTML from `dcsui_ru.hbk`,
  `shlang_ru.hbk`, `shquery_ru.hbk`, `fmtdui_ru.hbk`, `htmlui_ru.hbk` and `moxelui_ru.hbk`.
- Focused conversion tests assert normal Markdown contains no raw HBK file paths, raw TOC indexes,
  raw HTML page paths or service HTML scaffolding.
- Dependency-boundary check proves `hbk-book-export` does not depend on Syntax Assistant
  extraction, `hbk-syntax-export`, search/index or resolver crates (`cargo tree -p hbk-book-export`
  or equivalent `cargo metadata` query).
- `cargo test -p hbk-book-export`
- `cargo test --workspace`

### [ ] T103. Implement markdown/toc export layout and UAT corpus

Spec refs:

- UC-HBK-003
- FR-HBK-004
- FR-HBK-003
- FR-DOC-001
- FR-CLI-001
- ADR-0009
- UAT-HBK-004
- UAT-HBK-005
- UAT-HBK-006
- UAT-HBK-007
- `spec/implementation/components.md`

Scope:

- Implement `hierarchy=toc` as TOC-ordered page export under deterministic page directories.
- Implement `format=markdown --hierarchy=toc` as readable Markdown page files for TOC HTML content.
- Preserve the FR-HBK-004 Markdown invariant across full layout export: Markdown files must not
  contain raw HBK file paths, raw TOC indexes, raw HTML page paths or service HTML scaffolding.
- Run the first Markdown UAT corpus on representative 8.5.1.1150 pages from `fmtdui_ru.hbk`,
  `htmlui_ru.hbk`, `moxelui_ru.hbk`, `shlang_ru.hbk`, `shquery_ru.hbk` and `dcsui_ru.hbk`.
- Keep `format=raw --hierarchy=toc` and `format=markdown --hierarchy=raw` unsupported unless a
  later spec task defines them.

Verification:

- Focused tests for markdown/toc export layout and deterministic file naming.
- UAT-HBK-004 through UAT-HBK-007 pass or are skipped only for missing installed HBK books.
- UAT-HBK-004 includes a corpus-level negative search for raw HBK file paths, raw TOC indexes, raw
  HTML page paths and service HTML scaffolding in exported Markdown files.
- Dependency-boundary check proves `hbk-book-export` does not depend on Syntax Assistant
  extraction, `hbk-syntax-export`, search/index or resolver crates (`cargo tree -p hbk-book-export`
  or equivalent `cargo metadata` query).
- `cargo test -p hbk-book-export`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`
