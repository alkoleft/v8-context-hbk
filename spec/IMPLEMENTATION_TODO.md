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

The active open tasks are T66, T67 and T86-T88. T66 is the first unchecked task and remains the
required non-platform HBK domain-analysis gate before T67. T67 is the first resolver implementation
slice. T86-T88 are cleanup follow-ups from the May 2026 solution review and should not bypass the
T66/T67 resolver sequence unless explicitly selected.

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

### [ ] T66. Analyze non-platform Syntax Assistant domains from HBK books

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

### [ ] T67. Implement first Rust resolver core and platform adapter slice

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
  through the platform adapter unless T66 explicitly selected them as a query-language resolver
  source.
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
