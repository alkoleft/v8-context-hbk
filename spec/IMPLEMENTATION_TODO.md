# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history:

- [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md)
- [archive/completed-tasks-t13-t17-t19-t24.md](archive/completed-tasks-t13-t17-t19-t24.md)
- [archive/completed-tasks-t25-t34.md](archive/completed-tasks-t25-t34.md)
- [archive/implementation-todo-2026-05-04.md](archive/implementation-todo-2026-05-04.md)
- [archive/completed-tasks-t41-t47.md](archive/completed-tasks-t41-t47.md)
- [archive/completed-tasks-t48-t56.md](archive/completed-tasks-t48-t56.md)

Current status: T35-T56 and the T18 first slice are archived historical tasks. Their durable
export, schema, data-quality, performance, parser, provider, storage and query-search conclusions
live in `acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`implementation/components.md`, `implementation/syntax-helper-query-cli.md` and
`implementation/syntax-bsl-provider-plan.md`.
T62-T64 completed the RAT review-ergonomics slice described in
`implementation/syntax-bsl-provider-plan.md`. T65 adds the ADR-0008 Rust solution-context resolver
API design. T66 records the required analysis of non-platform HBK syntax domains before the first
resolver implementation slice. All `syntax` scope work remains oriented toward successful help
during BSL development and code analysis, and toward typed local provider roles for future analyzers
and context-building tools.

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

### [x] T57. Define analyzer query primitives over normalized storage

Spec refs:

- ADR-0006
- ADR-0007
- UC-SH-005A
- UC-SH-005B
- UC-SH-005D
- FR-SH-PROVIDER-001
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/implementation/syntax-bsl-provider-plan.md`

Problem:

- T56 normalized the storage needed for type/member inference, but the accepted provider contract
  still exposes mostly human-oriented commands: `get`, `constructors`, `search` and `related`.
- A future BSL analyzer needs stable primitive operations that map directly to type inference and
  member completion: resolve type identity, list members, resolve owner/member, inspect callable
  overloads and follow type references.
- These primitives must preserve ADR-0007: CLI JSON is the first external boundary, while SQLite
  table names remain internal implementation details.

Scope:

- Define provider-level query primitives and JSON shapes for:
  - resolving a type by exact id/name/alias;
  - listing members for a resolved type identity;
  - resolving one member by `owner_type_id` or exact owner plus member name;
  - retrieving callable overloads, ordered parameters and return/constructor result types;
  - exposing type-reference edges needed for expression-chain inference.
- Define ambiguity, missing-result and unsupported-query behavior for each primitive.
- Decide whether the first implementation extends existing commands or adds new command names such
  as `syntax type`, `syntax members` and `syntax callable`.
- Keep the task spec-only unless the primitive contract is already clear enough to implement safely
  in the same task; if implementation is deferred, add a follow-up task with the selected command
  shape.

Verification:

- Updated implementation spec records primitive names, inputs, outputs, ambiguity behavior and
  non-goals.
- UAT or acceptance notes identify at least one source-backed BSL expression-chain scenario that
  the primitives must support.
- No BSL parser, analyzer diagnostics, Rust public API or SQLite public table contract is added.

Completion notes:

- `syntax get`, `syntax constructors` and `syntax related` remain the selected CLI command surface;
  analyzer primitives are represented as normalized provider `query.kind` shapes over the same
  CLI JSON envelope.
- UAT-SH-018 records the SKD expression-chain and `Новый HTTPСоединение(...)` constructor-chain
  scenarios for T58/T59 implementation and verification.

### [x] T58. Implement analyzer provider primitives in CLI JSON

Spec refs:

- T57
- ADR-0007
- FR-SH-PROVIDER-001
- UC-SH-005A
- UC-SH-005B
- UC-SH-005D

Scope:

- Implement the provider primitives selected by T57 over the normalized schema-v4 tables.
- Preserve existing `syntax get`, `syntax constructors`, `syntax search` and `syntax related`
  behavior unless T57 explicitly changes their contract.
- Return the existing provider envelope with `schema_version`, `command`, `status`, `query`,
  `results` and `diagnostics`.
- Keep SQLite table names internal; public JSON must expose stable provider facts and metadata, not
  storage rows.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- rebuild a real RU index from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- JSON assertions cover type resolution, member listing, owner/member resolution and callable
  overload details from normalized rows.
- Existing UAT-SH-017 assertions still pass.

Completion notes:

- Implemented analyzer-oriented provider roots through the existing CLI JSON boundary:
  `--kind platform_type --id|--name|--alias`, `--members-of`, `--owner-type-id --member`,
  `--callable-id`, `--owner-type-id --callable` and `related --id --edge`.
- Provider facts continue to use export-compatible `results[].fact`; analyzer-only
  `owner_type_id` and `target_type_ids` are emitted under `results[].meta`.
- Verification passed with a fresh RU index at `target/uat/t58-sh-search-ru.sqlite` containing
  `25082` documents.

### [x] T59. Add expression-chain provider UAT without a BSL parser

Spec refs:

- UC-SH-005A
- UC-SH-005B
- UC-SH-005C
- UC-SH-005D
- UAT-SH-017
- UAT-SH-018
- ADR-0006

Scope:

- Add a black-box UAT scenario that models BSL expression-chain inference as a sequence of provider
  calls, not by parsing BSL source inside this repository.
- Start with the accepted SKD chain:
  - `НастройкиКомпоновкиДанных.Отбор` resolves to `ОтборКомпоновкиДанных`;
  - `ОтборКомпоновкиДанных.Элементы` resolves to the filter item collection type;
  - collection item creation resolves to `ЭлементОтбораКомпоновкиДанных`;
  - member completion for the resulting item exposes source-backed fields needed by the scenario.
- Add one constructor-chain scenario, for example `Новый HTTPСоединение(...)`, that verifies
  constructor result type plus callable parameter facts.
- Promote only stable commands/assertions/conclusions into `spec/`; keep raw outputs under
  `target/`.

Verification:

- Updated `spec/acceptance/uat-test-cases.md`.
- Updated `spec/acceptance/baseline.md` after running the scenario.
- UAT passes against a freshly rebuilt RU index.
- The scenario uses provider commands/JSON only and does not depend on SQLite table names.

Completion notes:

- UAT-SH-018 now has a dedicated T59 expression-chain scenario that derives each next provider root
  from previous provider JSON rather than parsing BSL source or querying SQLite tables.
- Verification passed against a fresh RU index at `target/uat/t59-sh-search-ru.sqlite` with `25082`
  documents and `52698 ms` build time.
- The scenario verifies the SKD chain through `НастройкиКомпоновкиДанных.Отбор`,
  `ОтборКомпоновкиДанных`, `Элементы`, collection `Добавить` and
  `ЭлементОтбораКомпоновкиДанных` fields, plus the `Новый HTTPСоединение(...)` constructor chain.

### [x] T60. Harden ambiguity handling for analyzer type/member inference

Spec refs:

- ADR-0006
- ADR-0007
- UC-SH-005B
- UC-SH-005D
- FR-SH-PROVIDER-001

Scope:

- Audit duplicate type names, aliases, owner variants, metadata-template types and extension types
  that can affect type/member inference.
- Ensure analyzer primitives return `status: "ambiguous"` with deterministic candidates when a type
  or member cannot be resolved uniquely.
- Do not introduce hidden winner selection based on FTS rank, row order or first-seen source page.
- Add focused fixtures or real-index assertions for at least one duplicate-name case.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- targeted real-index JSON assertions for ambiguous and unambiguous type/member lookups.
- Existing provider/UAT scenarios still pass.

Completion notes:

- Exact-name lookup no longer collapses mixed ownerless/owned matches to the ownerless fact; `get`
  and `related` JSON now report `ambiguous` with deterministic candidate summaries.
- Owner-name/member roots now resolve the owner as a platform type identity first and report
  ambiguous owner candidates before filtering by member name.
- Constructor lookup by ambiguous type name returns the provider envelope with
  `status: "ambiguous"` instead of a non-provider error or hidden owner selection.
- Verification passed with a fresh RU index at `target/uat/t60-sh-search-ru.sqlite` containing
  `25082` documents.

### [x] T61. Evaluate analyzer batch lookup needs after primitive UAT

Spec refs:

- ADR-0007
- UC-SH-005D
- NFR-QUERY-001

Scope:

- Measure or estimate the cost of expression-chain and member-completion workflows when they call
  CLI JSON primitives one at a time.
- Decide whether a batch command is needed for analyzer use, such as resolving many types/members
  in one process invocation.
- If a batch provider boundary is needed, add a follow-up ADR or task with concrete input/output
  shapes, error handling and verification.
- Do not add a Rust API, daemon, MCP service or SQLite public table contract in this task.

Verification:

- Recorded measurement or reasoned no-op conclusion in implementation/acceptance docs.
- If batch is deferred, the reason references actual primitive/UAT usage.
- If batch is selected, a follow-up task or ADR captures the exact boundary before implementation.

Completion notes:

- Measured the accepted UAT-SH-018 expression-chain and constructor-chain workflow as nine separate
  CLI JSON calls against the prebuilt T60 Russian index
  `target/uat/t60-sh-search-ru.sqlite`.
- Individual debug command timings were `0.00-0.39 s`; five repeated full-chain runs took
  `745-830 ms` total and emitted `48390` bytes across the nine JSON responses.
- Batch lookup is deferred: the accepted primitive/UAT workflow stays within NFR-QUERY-001, and
  ADR-0007 still keeps local CLI JSON as the first analyzer-provider boundary.
- No Rust API, daemon, service boundary, public SQLite contract or batch command was added.

### [x] T62. Improve review-oriented search ranking

Spec refs:

- ADR-0006
- ADR-0007
- UC-SH-005C
- FR-SH-SEARCH-001
- FR-SH-PROVIDER-001
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/implementation/syntax-bsl-provider-plan.md`

Problem:

- A RAT code-review smoke showed that `syntax search --query "Структура" --mode keywords
  --format json` ranks less useful SKD property facts above the exact `platform_type:Структура`
  identity.
- For review and code-analysis assistance, simple symbol queries should prefer exact platform type,
  method, property or constructor identities before broader description/owner matches.
- The ranking fix must not regress accepted task-oriented searches such as `отбор скд` and
  `таблица регистра бухгалтерии`.

Scope:

- Adjust deterministic search ranking so exact primary/alias identity matches outrank partial
  property/owner/description matches for simple symbol queries.
- Keep ranking metadata under `results[].meta`; do not expose internal FTS tokens under
  `results[].fact`.
- Add focused tests or UAT assertions for `Структура` and at least one existing accepted
  task-oriented query.
- Do not add semantic search, project-symbol indexing or a BSL parser.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- Rebuild a real RU index from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`.
- JSON assertions show `platform_type:Структура` ranks ahead of non-identity facts for the simple
  `Структура` query.
- Existing accepted `отбор скд` and `таблица регистра бухгалтерии` search assertions still pass.

Completion notes:

- Keyword search now applies an exact primary/alias identity tier before broader prefix, token,
  owner and description matches; exact same-name facts are still ordered by provider kind priority.
- `UAT-SH-020` records the real-index review-ranking scenario for `Структура` and regression checks
  for `отбор скд` and `таблица регистра бухгалтерии`.
- Verification passed against a fresh RU index at `target/uat/t62-sh-search-ru.sqlite` with `25082`
  documents and `52851 ms` build time.

### [x] T63. Add bounded and compact output controls for search/related

Spec refs:

- ADR-0007
- UC-SH-005C
- UC-SH-005D
- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- FR-SH-PROVIDER-001
- NFR-QUERY-001
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/implementation/syntax-bsl-provider-plan.md`

Problem:

- `syntax related` can return very large result sets for narrow review questions; RAT smoke against
  `type_property:platform_type:Символы:ПС` returned 200 related facts.
- Human reviewers and coding agents need a way to request a bounded or compact result without
  losing deterministic provider behavior.

Scope:

- Define and implement explicit bounded output controls for `syntax search` and `syntax related`,
  such as `--limit <N>`.
- Define and implement a compact mode for `syntax related` that keeps stable fact identity and
  enough path summary to explain relevance while omitting bulky fields that are not needed for a
  review triage view.
- Preserve the current full provider JSON as the default unless the spec is deliberately updated.
- Keep SQLite table names, graph internals and FTS details out of public facts.
- Do not add a BSL parser, project-symbol analyzer, service boundary or batch provider command.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- Rebuild or reuse a current RU index built from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`.
- JSON assertions show `--limit` bounds result count deterministically for both `search` and
  `related`.
- Compact `related` output remains deterministic and includes fact identity plus relationship
  explanation sufficient for review.
- Existing full-output provider/UAT assertions still pass.

Completion notes:

- `syntax search` and `syntax related` now accept `--limit <N>`; omitted limits preserve the
  previous defaults of `20` search results and `200` related results.
- `syntax related --compact` keeps stable fact identity plus `results[].meta.depth/path`, while
  omitting bulky fact fields such as descriptions, signatures, `types` and `return`.
- Verification passed against a fresh RU index at `target/uat/t63-sh-search-ru.sqlite` with `25082`
  documents and `52665 ms` build time.

### [x] T64. Align relationship edge filters with the public graph contract

Spec refs:

- ADR-0007
- UC-SH-005B
- UC-SH-005C
- FR-SH-SEARCH-002
- FR-SH-PROVIDER-001
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/implementation/syntax-bsl-provider-plan.md`

Problem:

- `spec/implementation/syntax-helper-query-cli.md` lists `member_of` as a supported first-slice
  edge kind in the relationship model.
- The current CLI rejects `syntax related --edge member_of` with `UNSUPPORTED_QUERY` and says only
  `has_type`, `returns` and `constructs` are supported.
- This mismatch is confusing during review because `member_of` is a natural inverse-navigation
  question for owned facts.

Scope:

- Decide whether `member_of` is a public provider edge filter now or only an internal/storage edge
  for the current implementation.
- If public, implement `syntax related --edge member_of` with deterministic JSON and text behavior.
- If not public, update the implementation spec, CLI help and unsupported-query diagnostic so the
  supported edge list is unambiguous.
- Add UAT or focused real-index assertions for the selected behavior.
- Do not broaden `related --edge` into a general graph-query language.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- Real-index command against `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` demonstrates the selected
  `member_of` behavior.
- Provider diagnostics and help text agree with the implemented supported edge list.

Completion notes:

- `member_of` is public inverse owner navigation for exact `syntax related --id` roots, not a
  storage-only edge.
- CLI help, provider unsupported-edge diagnostics and UAT-SH-022 now agree on the supported edge
  filter set: `has_type`, `returns`, `constructs` and `member_of`.
- Verification passed against a fresh RU index at `target/uat/t64-sh-search-ru.sqlite` with
  `25082` documents and `56075 ms` build time.

### [x] T65. Define Rust solution-context resolver API

Spec refs:

- ADR-0008
- UC-CTX-001
- FR-CTX-RESOLVE-001
- NFR-RESOLVE-001
- `spec/implementation/solution-context-resolve.md`
- `spec/implementation/components.md`

Problem:

- ADR-0007 deliberately deferred Rust library APIs while the analyzer consumer was still
  future-facing.
- A concrete Rust application now needs to form a complete solution context for validation, review,
  development assistance and diagnostics.
- The resolver cannot be platform-only: it must cover platform API facts plus separate BSL-language
  and query-language type domains, and later configuration/source-code providers.

Scope:

- Define the source-neutral Rust resolver boundary and domain model in `spec/`.
- Preserve ADR-0007 CLI JSON as the first language-agnostic provider boundary.
- Require source-qualified identities and explicit ambiguity across domains/sources.
- Keep implementation, configuration parsing, BSL parsing, query parsing and diagnostics out of
  this spec-only task.

Verification:

- ADR-0008 records the decision, alternatives, non-goals and implementation plan.
- Requirements, use cases, non-functional requirements and component specs reference the resolver
  boundary.
- The implementation spec defines resolver traits, fact/request/response concepts, domain
  separation, composition rules and the first platform adapter mapping.
- The active ledger records a follow-up implementation task.

Completion notes:

- Accepted `ContextResolver` / `ContextSource` as the Rust API direction for a future
  source-neutral resolver core.
- `PlatformApi`, `BslLanguage`, `QueryLanguage`, `Configuration` and `SourceCode` are separate
  domains; BSL language types and query-language types must not collapse into platform API types by
  name.
- `syntax-helper-search` remains the HBK/Syntax Assistant search implementation. A platform adapter
  may wrap `SearchIndex`, but the generic resolver model belongs in a separate thin core layer.

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

## Cleanup Sequence: Legacy Removal Before Rework

Execution guard:

- T68-T78 are cleanup tasks for removing provisional legacy paths before broader rework.
- While T66 or T67 remain unchecked, these cleanup tasks are not the default first unchecked task.
- A cleanup task may be selected only after T66/T67 are complete, or when the user explicitly says
  to run the cleanup sequence now.
- Every cleanup task references the public-contract policy in
  `spec/implementation/components.md`: provisional legacy paths may be removed without
  compatibility fallback when no accepted ADR or requirement stabilizes them.

### [x] T68. Record cleanup boundary for pre-rework legacy removal

Spec refs:

- `spec/implementation/components.md`
- `spec/README.md`

Scope:

- Confirm the no-backward-compatibility cleanup policy is durable in spec/ and not only in task
  text or chat.
- Record cleanup sequencing, non-goals and implementation boundaries for T69-T78.
- Do not remove code in this task.
- Do not change T66/T67 ordering unless the user explicitly selected cleanup work before them.

Verification:

- Cleanup policy is present in durable spec/ documentation.
- T69-T78 reference the policy and stay scoped to one cleanup concern each.
- `git diff --check`

Completion notes:

- `spec/implementation/components.md` now records the pre-rework legacy cleanup boundary, narrow
  sequencing, non-goals and the T69-T78 cleanup concerns outside the active ledger.
- T69-T78 continue to reference T68 and the durable public-contract policy; T66/T67 ordering remains
  unchanged except for explicitly selected cleanup work.
- Verification passed with `git diff --check`.

### [x] T69. Remove legacy in-memory search-index path

Spec refs:

- T68
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Remove the duplicate `build_index(context)` / `documents_from_context` path in
  `syntax-helper-search`.
- Keep the streaming `SearchIndexBuilder` / `SyntaxHelperSink` path as the single index-build
  mechanism.
- Update tests to build indexes through the surviving path.
- Do not change CLI query behavior, SQLite public assumptions or provider JSON shape.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`

Completion notes:

- Removed the provisional `build_index(context)` / `documents_from_context` path from
  `syntax-helper-search`.
- `SearchIndexBuilder` / `SyntaxHelperSink` is now the single index-build input path for library
  tests and CLI wiring.
- Verification passed with `cargo test -p syntax-helper-search --lib` and
  `cargo test --workspace`.

### [x] T70. Remove legacy in-memory export path

Spec refs:

- T68
- `spec/implementation/components.md`
- FR-EXPORT-001

Scope:

- Remove duplicate in-memory export APIs such as `export_platform_context`, `export_syntax_helper`
  and `PlatformContextExporter` when no repo-local accepted contract requires them.
- Keep `StreamingSyntaxHelperExport` as the single canonical export path.
- Update export tests and spec notes to reflect the surviving path.
- Do not change consumer JSON record-family shape in this task.

Verification:

- `cargo test -p hbk-export --lib`
- `cargo test --workspace`

Completion notes:

- Removed the provisional in-memory export API surface from `hbk-export`: `export_syntax_helper`,
  `export_platform_context`, `PlatformContextExporter` and the now-unused record-envelope helper.
- Removed the remaining `hbk-book` runtime dependency from `hbk-export`; locale inference stays at
  the CLI/book boundary before starting the streaming export writer.
- Kept `StreamingSyntaxHelperExport` / `SyntaxHelperSink` as the canonical export writer; repo-local
  export tests now feed records through the streaming sink instead of materializing an export from
  `PlatformContext`.
- Consumer JSON shape stayed unchanged; `spec/implementation/components.md` now records that the
  previous in-memory `PlatformContext` exporter was provisional and removed.
- Verification passed with `cargo test -p hbk-export --lib` and `cargo test --workspace`.

### [x] T71. Collapse duplicated `syntax get` query dispatch

Spec refs:

- T68
- ADR-0007
- FR-SH-PROVIDER-001
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Replace the duplicated tuple-match logic in `get_query_value` and `get_lookup` with one typed
  query classification for `syntax get`.
- Preserve accepted provider query kinds, status behavior and text output.
- Do not move provider JSON fact serialization in this task.

Verification:

- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

Completion notes:

- Collapsed `syntax get` root classification into one typed query classifier that owns both the
  provider `query` JSON shape and the lookup variant.
- Preserved existing provider query kinds, unsupported-query messages, lookup behavior, status
  behavior, text output and provider JSON fact serialization.
- Added focused CLI unit tests for valid type/callable roots and invalid/unsupported root
  classification.
- Verification passed with `cargo test -p v8-context-hbk-cli` and `cargo test --workspace`.

### [x] T72. Collapse provider JSON adapter duplication

Spec refs:

- T68
- ADR-0007
- FR-SH-PROVIDER-001
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Deduplicate CLI `document_fact` / `compact_document_fact` provider JSON mapping.
- Keep provider JSON deterministic and export-compatible for shared platform fact fields.
- Do not change lookup dispatch, SQLite schema or search ranking in this task.

Verification:

- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

Completion notes:

- Collapsed full and compact provider fact mapping into one `document_fact` adapter with an explicit
  detail mode.
- Preserved full provider JSON shape for export-compatible shared fields and compact `related`
  output shape for identity/owner-only review results.
- Added focused CLI tests for both compact and full provider fact shapes.
- Verification passed with `cargo test -p v8-context-hbk-cli` and `cargo test --workspace`.

### [x] T73. Normalize HBK/page path handling boundaries

Spec refs:

- T68
- FR-HBK-002
- FR-HBK-003
- FR-DOC-001
- FR-SH-003
- `spec/implementation/components.md`

Scope:

- Consolidate path-normalization rules currently split across `hbk-book`, `hbk-docs` and
  `syntax-helper-extract`.
- Keep distinct functions only where the semantics genuinely differ, such as storage path,
  documentation link target and Syntax Assistant member link.
- Preserve existing observable behavior unless a difference is promoted into spec/UAT first.
- Do not remove query-table syntax fallback behavior in this task.

Verification:

- `cargo test -p hbk-book --lib`
- `cargo test -p hbk-docs --lib`
- `cargo test -p syntax-helper-extract --lib`
- `cargo test --workspace`

Completion notes:

- Consolidated shared storage/page path normalization in `hbk-book` and reused it from TOC lookup,
  FileStorage reads, documentation page parsing and Syntax Assistant page-source parsing.
- Kept documentation link-target normalization and Syntax Assistant member-link normalization as
  distinct boundary functions because they resolve fragments, schemes, relative paths and
  owner/member anchors with different semantics.
- Preserved observable CLI/export/parser behavior; no UAT or acceptance baseline shape changed.
- Verification passed with `cargo test -p hbk-book --lib`, `cargo test -p hbk-docs --lib`,
  `cargo test -p syntax-helper-extract --lib` and `cargo test --workspace`.

### [x] T74. Specify query-table syntax fallback removal

Spec refs:

- T68
- FR-EXPORT-001
- FR-SH-003
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/acceptance/uat-test-cases.md`

Scope:

- Define the observable contract for pages where query table syntax is missing or empty.
- Decide the JSON/diagnostic behavior before removing `query_table_identifier` /
  `query_table_role` fallback-to-name behavior.
- Add or update UAT/acceptance notes for at least one source-backed query-table scenario.
- Do not implement parser/export changes in this task.

Verification:

- Updated spec/UAT notes define the selected missing-syntax behavior and non-goals.
- Follow-up implementation scope remains limited to the selected contract.
- `git diff --check`

Completion notes:

- Selected the missing/empty query-table syntax contract for T75: keep the query table record and
  nested field/parameter facts, omit consumer `syntax` and `identifier`, set
  `table_role="unknown"` and emit a parser-maintenance `MISSING_QUERY_TABLE_SYNTAX` diagnostic with
  source provenance.
- Removed the old spec allowance that generic `Основная таблица` / `Main Table` names could act as
  role fallback when syntax is missing.
- UAT-SH-011 and UAT-SH-012 now describe the source-backed `Таблицы задач > Основная таблица` /
  `Task Tables > Main Table` missing-syntax behavior that T75 must implement.
- Verification passed with `git diff --check`.

### [x] T75. Implement query-table syntax fallback removal

Spec refs:

- T68
- T74
- FR-EXPORT-001
- FR-SH-003
- `spec/implementation/components.md`

Scope:

- Remove fallback-to-name behavior in `query_table_identifier` and `query_table_role` according to
  the T74-approved contract.
- Emit the selected `MISSING_QUERY_TABLE_SYNTAX` diagnostic for missing/empty syntax while keeping
  the query table record and nested field/parameter facts.
- Update focused parser/export tests and any affected acceptance baseline notes.

Verification:

- `cargo test -p syntax-helper-extract --lib`
- `cargo test -p hbk-export --lib`
- `cargo test --workspace`

Completion notes:

- Removed fallback-to-name behavior for query table `identifier` and `table_role`: missing or empty
  syntax now keeps an empty internal identifier and `QueryTableRole::Unknown`.
- `hbk-export` omits empty consumer `identifier`, preserving the T74 JSON contract of no synthesized
  `syntax` or `identifier` for missing-syntax query tables.
- The extraction stream emits one `MISSING_QUERY_TABLE_SYNTAX` parser-maintenance diagnostic per
  affected query table while still streaming the query table record and nested field/parameter
  facts.
- `syntax-helper-search` keeps non-empty internal document ids for missing-syntax query tables by
  using semantic owner-path identity, without restoring parser/export fallback identifiers.
- Verification passed with `cargo test -p syntax-helper-extract --lib`,
  `cargo test -p hbk-export --lib`, `cargo test -p syntax-helper-search --lib` and
  `cargo test --workspace`.

### [x] T76. Replace in-memory type lookup scan with indexed SQL lookup

Spec refs:

- T68
- ADR-0004
- ADR-0007
- FR-SH-PROVIDER-001
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Change `type_identities_by_lookup_key` to use indexed SQLite lookup instead of loading all type
  identities and filtering in memory.
- Preserve deterministic ambiguity behavior and provider JSON results.
- Do not change index schema unless a focused migration is required and recorded in spec.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`

Completion notes:

- `type_identities_by_lookup_key` now uses the indexed `document_names` lookup table joined to
  `type_identities` instead of loading every platform type identity and filtering in memory.
- Added an internal `type_identities(document_id)` index so the lookup plan does not scan all type
  identities after filtering the lookup key.
- Raised the internal search-index schema to version `5`; existing schema version `4` indexes are
  rebuildable service data and must be rebuilt before query commands open them.
- Deterministic same-name ambiguity behavior is preserved; a focused regression test verifies that
  same-display-name platform type variants are returned in stable identity order, and a query-plan
  test verifies indexed lookup usage.
- No provider JSON shape, CLI behavior or public contract changed.
- Verification passed with `cargo test -p syntax-helper-search --lib` and `cargo test --workspace`.

### [x] T77. Clean `syntax-helper-search` dependency scope

Spec refs:

- T68
- `spec/implementation/components.md`

Scope:

- Move `syntax-helper-search` `serde_json` dependency to dev-dependencies when it is still only used
  by tests, or remove it if no longer needed.
- Keep dependency cleanup limited to this crate and this finding.
- Do not include broad clippy cleanup or unrelated dependency updates.

Verification:

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Completion notes:

- Moved `syntax-helper-search` `serde_json` from production dependencies to dev-dependencies; the
  crate uses it only in tests that assert provider/search JSON does not expose internal search
  terms.
- No `syntax-helper-search` runtime dependency, SQLite schema, provider JSON shape or CLI query
  behavior changed.
- The required workspace clippy gate also required minimal current-toolchain lint compatibility
  fixes in existing code paths: boxed the large `hbk-docs` documentation read source error,
  collapsed repeated dash-normalization `replace` calls, removed one redundant closure and added
  targeted CLI boundary lint allowances without changing command behavior.
- Verification passed with `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace`.

### [ ] T78. Deduplicate property usage and type-prose cleanup

Spec refs:

- T68
- FR-SH-002
- FR-EXPORT-001
- `spec/implementation/components.md`

Scope:

- Remove duplicated parser/export handling for property `usage` normalization and leading type
  prose cleanup.
- Keep the rule at the boundary selected by spec: parser/domain if it is extraction truth,
  exporter only if it is consumer-shape adaptation.
- Do not perform broad parser rewrites in this task.

Verification:

- `cargo test -p syntax-helper-extract --lib`
- `cargo test -p hbk-export --lib`
- `cargo test --workspace`

### [x] T79. Report search document identity collisions instead of dropping duplicates

Spec refs:

- T68
- ADR-0004
- ADR-0007
- ADR-0008
- FR-SH-PROVIDER-001
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`

Problem:

- `SearchIndexBuilder::into_documents` sorts documents and then silently removes duplicate ids with
  `dedup_by`.
- Duplicate ids mean that two extracted facts collapsed to one provider identity. Silent removal can
  hide parser, TOC-classification or identity-model bugs and conflicts with the identity-preserving
  resolver direction in ADR-0008.

Scope:

- Replace silent duplicate-id removal with explicit collision detection.
- Return an index-build error or a deterministic parser/index diagnostic before writing SQLite when
  two distinct documents resolve to the same id.
- Add a focused regression test that proves duplicate ids are not silently lost.
- Preserve normal deterministic ordering and provider JSON shape for non-colliding indexes.
- Do not redesign document identity rules in this task unless the regression exposes a concrete
  source-backed collision.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`

Completion notes:

- Removed the silent `SearchIndexBuilder::into_documents` duplicate-id collapse; finalized search
  documents are now validated for unique ids before index writes.
- Added a typed `DuplicateDocumentId` index-build error and validate direct document-list builds as
  well as streaming builder builds before SQLite creation.
- Added focused regressions proving duplicate ids do not create an index file and TOC-marker
  identity collisions are reported instead of silently dropping a document.
- Provider JSON, normal deterministic ordering and non-colliding index behavior stayed unchanged.
- Verification passed with `cargo test -p syntax-helper-search --lib` and `cargo test --workspace`.
