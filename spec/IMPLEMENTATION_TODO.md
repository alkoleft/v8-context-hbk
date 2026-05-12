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

Current status: T35-T151 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design,
language-domain, cleanup, book-content export, documentation-site, platform type-template and
type-reference conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`requirements/non-functional.md`, `implementation/components.md`,
`implementation/documentation-site.md`, `implementation/syntax-helper-query-cli.md`,
`implementation/syntax-bsl-provider-plan.md`, `implementation/solution-context-resolve.md` and
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

- [x] T152: Add public resolver module-context boundary for HBK-owned platform module facts.
  - Scope: extend `context-resolver-core` with provider-neutral module context DTOs/query and
    extend `context-resolver-search` so HBK-backed indexes expose platform global
    methods/properties, module events, event signatures and availability through the public Rust
    resolver API when indexed evidence exists.
  - Boundaries: no dependency on `v8-context` or `v8-context-metadata`; no public SQLite/storage
    table contract; no metadata-owned form/module/generated-type facts; no analyzer fallback lists
    for `ЭтотОбъект` / `ThisObject` or other predefined members.
  - Spec refs: `FR-CTX-RESOLVE-001`, `implementation/solution-context-resolve.md`,
    `implementation/components.md`, ADR-0008.
  - Verification: focused `context-resolver-core`, `syntax-helper-search` and
    `context-resolver-search` tests; `cargo fmt --all --check`; `cargo test --workspace`.
  - Result: `context-resolver-core` exposes `ModuleContextKind`, `ModuleContextQuery` and
    `ResolvedModuleContext`; `context-resolver-search` exposes provider-backed BSL module contexts
    for indexed module-event kinds; `syntax-helper-search` schema version 14 preserves module event
    context kind as private provider state; resolved module context handles round-trip through
    exact id lookup; unsupported/self-member gaps remain explicit.
