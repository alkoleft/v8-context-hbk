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
T62-T64 are queued from the RAT review-ergonomics smoke described in
`implementation/syntax-bsl-provider-plan.md`. All `syntax` scope work is oriented toward successful
help during BSL development and code analysis, and toward a future typed local provider role for a
BSL analyzer.

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
