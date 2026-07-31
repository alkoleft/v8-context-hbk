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
- [archive/completed-tasks-t165-t182.md](archive/completed-tasks-t165-t182.md)

Current status: T35-T182 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design,
language-domain, cleanup, book-content export, documentation-site, platform type-template and
type-reference conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`requirements/non-functional.md`, `implementation/components.md`,
`implementation/documentation-site.md`, `implementation/syntax-helper-query-cli.md`,
`implementation/syntax-bsl-provider-plan.md`, `implementation/solution-context-resolve.md`,
`implementation/performance-baseline-t13.md`, `implementation/performance-variants.md` and
`decisions/`. Detailed records for T165-T182 are in
`archive/completed-tasks-t165-t182.md`.

Current first unchecked task: T183.

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

- [ ] **T183 — Compare isolated zero-copy snapshot hypotheses without
  selecting a winner**
  - Requirements:
    [NFR-RESOLVE-001](requirements/non-functional.md#nfr-resolve-001-in-process-resolver-latency-and-determinism),
    [NFR-SNAPSHOT-001](requirements/non-functional.md#nfr-snapshot-001-evidence-gated-file-backed-snapshot-experiment).
  - Implementation:
    [T183 experiment contract](implementation/hbk-zero-copy-snapshot-experiment.md),
    [Provider-Owned HBK Fact Snapshot](implementation/components.md#provider-owned-hbk-fact-snapshot),
    [T183 Zero-Copy Candidate Isolation](implementation/components.md#t183-zero-copy-candidate-isolation).
  - OpenSpec:
    `openspec/changes/establish-hbk-zero-copy-snapshot-cache`.
  - Scope:
    1. commit a standalone, versioned release benchmark/parity base for H0
       SQLite-to-owned and C0 current-cache-to-owned;
    2. capture baseline noise and freeze numerical gates before candidate code;
    3. create isolated H1 custom-flat and H3 archive-candidate worktrees from
       the frozen base, then H2 “H1 layout + typed reader” from measured H1;
    4. require parity before accepting candidate performance evidence;
    5. publish raw evidence plus one unranked comparison table with branch
       ancestry and commit SHAs.
  - Verification:
    strict OpenSpec validation; format/check/test for the frozen base and each
    candidate branch; exact corpus/checksum verification; versioned canonical
    content and lookup transcripts; repeated median/MAD release measurements;
    independent safety/performance review.
  - Completion boundary:
    update the durable acceptance baseline with all measured rows and gate
    outcomes, but do not name a winner, merge a candidate into `master`, accept
    a new production dependency or change the canonical runtime path without
    the user's explicit selection.
  - Progress:
    frozen benchmark/parity base `051df7979e3cf5f6431b4d13829f436c98c47054`;
    H0/C0 protocol, noise, production lifecycle, owned-cache inventory,
    behavior oracle and predeclared numerical gates are recorded in the T183
    experiment contract. Candidate branches have not yet been selected,
    ranked or merged.

OpenSpec changes archived and synchronized on 2026-07-30:
the completed change records are under `../openspec/changes/archive/`, and their
delta specifications are synchronized under `../openspec/specs/`.
