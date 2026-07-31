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
    6. run an independent S83 comparison set on exact platform
       `8.3.27.1859`, adding separate typed-flat/archive references and
       one-variable layout, mapped-index, checked-dynamic-read and
       direct-formation hypotheses in their own worktrees.
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
    experiment contract. Candidate branches `experiment/hbk-zero-copy-flat-h1`
    (`a2431254ee5d90a6e77c877e329bbb8d0ca50e84`),
    `experiment/hbk-zero-copy-flat-typed-h2`
    (`826991395a508e36b7a684dc987ead218ef27184`) and
    `experiment/hbk-zero-copy-rkyv-h3`
    (`497afa52344fb318a4f27c94762cc7eafa1126ca`) have generated unranked
    evidence. All remain unselected and unmerged. H1 is ineligible due parity,
    validation and workload-equivalence blockers; H2 and H3 preserve the
    representative workload totals but still lack full mapped canonical parity
    and complete first-use lifecycle proof. H2 releases its writer lock before
    post-publication self-validation, and its `ModuleEventNames` validation
    orders owner IDs differently from the owned text-order contract. H3 does
    not prove sorted order for every binary-searched name/id array. Candidate
    production allocations and per-section/dictionary/index byte footprints
    remain uninstrumented. The complete unranked gate table is in
    `acceptance/hbk-zero-copy-snapshot-evidence.md`.
    The user requested a second independent comparison set for
    `/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk`. Its HBK identity is
    `40,744,845` bytes /
    `5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48`;
    its schema-16 provider SQLite identity is `204,288,000` bytes /
    `55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab`.
    S83 uses a separate target root and will freeze new H0/C0 noise/gates
    before F0/A0/L1/I1/D1/P1 candidate work. Parallel agents may implement
    separate worktrees, but all performance runs are serialized. No S83
    candidate is created, ranked, selected or merged yet.
    T183 remains open until the evidence is presented and the user explicitly
    selects an outcome or rejects production adoption.

OpenSpec changes archived and synchronized on 2026-07-30:
the completed change records are under `../openspec/changes/archive/`, and their
delta specifications are synchronized under `../openspec/specs/`.
