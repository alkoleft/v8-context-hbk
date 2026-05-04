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
Next active unchecked task is T57. The queued roadmap comes from
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

### [ ] T57. Define analyzer query primitives over normalized storage

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

### [ ] T58. Implement analyzer provider primitives in CLI JSON

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

### [ ] T59. Add expression-chain provider UAT without a BSL parser

Spec refs:

- UC-SH-005A
- UC-SH-005B
- UC-SH-005C
- UC-SH-005D
- UAT-SH-017
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

### [ ] T60. Harden ambiguity handling for analyzer type/member inference

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

### [ ] T61. Evaluate analyzer batch lookup needs after primitive UAT

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
