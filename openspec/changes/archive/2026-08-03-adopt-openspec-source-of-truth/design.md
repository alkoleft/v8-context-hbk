## Context

The repository has two workflow interfaces. OpenSpec already owns change
proposals, designs, capability deltas, task status, validation, and archived
changes, while `spec/IMPLEMENTATION_TODO.md` separately repeats active task
sequencing. ADR-0002 and `AGENTS.md` still declare `spec/` to be the only
durable source of truth even though the completed T183/X1 work was primarily
executed and validated through OpenSpec.

At the start of this migration, T183 and its OpenSpec change are complete, the
working tree is clean, and there are no unfinished legacy-ledger tasks. The
completed X1 change is synchronized to canonical OpenSpec specs and archived
before this governance change is applied.

The migration changes repository governance only. Existing `spec/` documents
contain valuable requirements, research, acceptance evidence, ADRs, and
historical conclusions that must remain available without continuing as a
parallel place for new normative work.

## Goals / Non-Goals

**Goals:**

- Give agents one small workflow interface for discovering, planning,
  implementing, validating, completing, and archiving work: OpenSpec.
- Remove the parallel active-task ledger instead of retaining a pass-through
  compatibility file.
- Define unambiguous precedence between canonical OpenSpec specs, active change
  artifacts, accepted ADR rationale, and supporting legacy documentation.
- Preserve existing evidence and migrate legacy contracts incrementally when
  they are next changed.
- Make design review, versioning, and task-scoped commits explicit completion
  gates.

**Non-Goals:**

- Bulk-convert every existing file under `spec/` in this change.
- Delete historical task archives, acceptance evidence, research, or existing
  implementation notes.
- Change runtime code, public APIs, CLI behavior, parsers, exports, snapshot
  formats, or dependency choices.
- Add repository-scope prohibitions about metadata readers, BSL grammars, or
  downstream compatibility models.
- Introduce another registry, checklist file, wrapper, or compatibility ledger
  beside OpenSpec.

## Decisions

### OpenSpec is the single workflow interface

`openspec/specs/` owns canonical capability requirements. A directory under
`openspec/changes/` owns the proposal, design, requirement deltas, and task
state for proposed or active work. `openspec/changes/archive/` owns completed
change history.

Agents discover live state with `openspec list --json`; they do not hard-code a
current change name in repository rules. Before implementation they read every
context file returned by `openspec instructions apply --change <name> --json`.

This replaces the old two-ledger design. `spec/IMPLEMENTATION_TODO.md` is
deleted rather than retained as a pointer, because a pointer-only ledger would
be a shallow pass-through module and would keep two apparent workflow seams.

### Legacy `spec/` content is supporting material

Existing documents under `spec/` remain available as legacy requirements,
acceptance evidence, research, ADR rationale, implementation history, and
navigation. They do not receive new active task state or new normative
requirements.

Until imported, a legacy-only contract remains a binding baseline rather than
optional evidence. Before an implementation task edits code, tests, fixtures,
schemas, generators, or adapters in an area governed only by legacy `spec/`,
the task imports the smallest relevant contract into its OpenSpec delta spec,
including preservation scenarios when behavior is not intended to change.
OpenSpec then has precedence for that task scope. Existing evidence may
continue to be updated only as evidence or explanatory documentation
referenced from the OpenSpec change.

Accepted ADRs retain the rationale for hard-to-reverse decisions, but the
OpenSpec proposal/design/spec/task set owns implementation scope and current
contract state. ADR-0013 supersedes ADR-0002's source-of-truth hierarchy.

### Codebase design is checked twice

Every implementation task uses `mattpocock-skills:codebase-design` before
implementation begins and on the actual diff before the task is completed. The
first pass checks proposed module interfaces and seams; the second checks that
the implementation did not add shallow pass-through modules, duplicate owners,
or unnecessary adapters. Actionable findings are resolved before completion.

Both passes are recorded in the active change `design.md`. Each record names
the reviewed scope, affected module interfaces/seams/adapters and owners, every
finding and resolution, and a `PASS` or `BLOCKED` outcome. Duplicate ownership,
shallow pass-through modules, unjustified seams/adapters, or structural changes
that diverge from the approved design are blocking. An agent cannot waive them
silently; an owner-approved exception must be recorded in the design or an ADR.

Documentation-only analysis, review, and planning that does not edit
implementation files does not create a fictional implementation interface and
therefore does not require the implementation gate.

### Versioning happens once per completed OpenSpec change

The workspace version is bumped exactly once when an OpenSpec change is
completed:

- minor when the change adds shipped user-facing functionality;
- patch when it completes maintenance, refactoring, performance, governance,
  documentation, or other work without new shipped user-facing functionality.

The change design or tasks records the classification. Cargo manifests and
the lockfile must agree before completion. This governance change is a patch
bump from `0.2.4` to `0.2.5`. The repository owner explicitly accepts workspace
package versions as completed-change provenance even for governance and
documentation changes.

### Repository-changing work ends in a task-scoped commit

Successful repository-changing work is not complete until its verified,
reviewed, task-scoped files are committed with a Conventional Commit. The main
session inspects the staged file list and staged diff before committing.

Analysis-, review-, and planning-only work with no repository changes does not
create an empty commit. Blocked or failing work is not falsely marked complete
and is not committed as completed work.

### Archive after tasks and before the completion commit

The executable completion sequence is:

1. finish implementation and verification, record the actual-diff
   codebase-design `PASS`, and check the final ready-to-archive task;
2. validate the now-complete active change strictly;
3. archive it, synchronizing capability deltas to `openspec/specs/`;
4. validate all remaining active changes and canonical specs strictly;
5. inspect the final diff, stage only task-scoped files, inspect the staged file
   list and staged diff, then create the Conventional Commit.

Archive and commit are repository lifecycle gates recorded by the canonical
`project-governance` capability and `AGENTS.md`, not self-referential items in
the change's own task ledger. Work is reported complete only after the commit
succeeds.

## Risks / Trade-offs

- **Legacy contracts are not bulk-imported immediately** → The relevant
  legacy baseline remains binding and is imported before any implementation
  edit in its area; only task-relevant requirements are imported.
- **Historical documents may contain obsolete workflow wording** → Update live
  navigation, prompts, and accepted operational references now; retain archived
  task records unchanged as historical evidence.
- **Deleting the TODO file may appear to lose history** → Its content remains
  recoverable from Git and the completed T183 task state exists in the archived
  OpenSpec change and durable evidence documents.
- **Mandatory version bumps can create churn** → Bump once per completed
  OpenSpec change, not once per individual task or commit.
- **Mandatory commits can conflict with a dirty worktree** → Inspect branch and
  worktree before edits, stage only task files, and stop rather than include
  unrelated changes.

## Migration Plan

1. Synchronize and archive the completed X1 OpenSpec change.
2. Create and validate this governance change, its `project-governance`
   capability, and ADR-0013.
3. Update OpenSpec configuration, `AGENTS.md`, live prompts, legacy-spec
   navigation, and accepted operational references to the new precedence.
   Reconcile the one canonical OpenSpec requirement that still names legacy
   `spec/` as the durable dependency-contract owner.
4. Delete `spec/IMPLEMENTATION_TODO.md` and the obsolete cleanup prompt; do not
   rewrite historical archives solely to modernize their wording.
5. Apply the patch workspace version bump and regenerate the lockfile.
6. Run strict OpenSpec validation, the explicit live-reference denylist check,
   ADR readiness review, mandatory codebase-design diff review, and independent
   review.
7. Mark the final ready-to-archive task complete, validate, archive this change
   with spec synchronization, validate canonical state, inspect the staged
   scope, and create one Conventional Commit.

Rollback is `git revert` of the task commit. The removed ledger and both
archive moves remain recoverable from Git history.

The live-reference denylist is the following zero-match check over current
workflow surfaces; historical archives, superseded decision text, and this
change's migration rationale are intentionally outside its path set:

```bash
! rg -n 'IMPLEMENTATION_TODO\.md|first[- ]unchecked|active task ledger|Take the first unchecked' \
  AGENTS.md \
  scripts/infr/impl-prompt.md \
  spec/README.md \
  spec/acceptance/test-case-requirements.md \
  spec/acceptance/baseline.md
test ! -e spec/IMPLEMENTATION_TODO.md
```

The final diff review additionally inspects all added and modified live
workflow files for equivalent task checklists or task-status pointers outside
`openspec/changes/`; literal filename checks alone are not sufficient.

## Open Questions

None. The migration boundary and completion rules were explicitly confirmed by
the repository owner on 2026-08-03.

## Migration Evidence

- Before this change was created,
  `openspec status --change establish-hbk-zero-copy-snapshot-cache --json`
  reported all artifacts complete and `51/51` tasks complete.
- `openspec validate establish-hbk-zero-copy-snapshot-cache --strict` passed,
  and the change was synchronized and archived at
  `openspec/changes/archive/2026-08-03-establish-hbk-zero-copy-snapshot-cache/`.
- The terminal state of `spec/IMPLEMENTATION_TODO.md` says there are no
  unfinished tasks and T183 is complete.
- The working tree was clean before the archive and this governance change.

## Codebase-Design Review Record

### Pre-Implementation — 2026-08-03

- **Scope**: source-of-truth and implementation-workflow migration only.
- **Module interface**: OpenSpec CLI plus `openspec/specs/` and
  `openspec/changes/` artifacts is the single workflow interface.
- **Seams/adapters**: `AGENTS.md` and `scripts/infr/impl-prompt.md` are guidance
  at the existing OpenSpec seam; no new adapter or abstraction is introduced.
- **Ownership/deletion test**: OpenSpec owns requirements/change/task state;
  deleting `spec/IMPLEMENTATION_TODO.md` removes an interface rather than
  distributing its complexity, because task state already exists in OpenSpec.
- **Findings**: a pointer-only replacement ledger and a new governance registry
  would be shallow duplicate modules and are prohibited by the design.
- **Outcome**: `PASS`; no open blocking structural finding.

### Actual Diff — 2026-08-03

- **Scope**: governance artifacts and live workflow/navigation, archival and
  canonical synchronization of the already completed X1 change, removal of the
  parallel ledger/obsolete cleanup prompt, and workspace version provenance.
  No runtime or public behavior file changed.
- **Module interface**: OpenSpec remains the only interface that owns
  requirements, change design, and task state. `AGENTS.md`, OpenSpec config,
  and the implementation prompt describe entry into that interface but store
  no independent task status.
- **Seams/adapters**: no new seam or adapter was added. The existing agent-rule
  and prompt surfaces point directly to OpenSpec live discovery and artifact
  instructions.
- **Owners**: `project-governance` owns normative workflow requirements;
  ADR-0013 owns rationale; `spec/` owns only supporting legacy/evidence content.
  The archived X1 delta and synchronized canonical X1 capability have distinct
  standard OpenSpec history/current-contract roles rather than duplicate task
  ownership.
- **Deletion test**: removing `spec/IMPLEMENTATION_TODO.md` and the obsolete
  cleanup prompt removed the former workflow interface. No pointer ledger,
  replacement checklist, first-unchecked rule, registry, wrapper, or
  compatibility adapter was introduced.
- **Verification evidence**: strict active-change validation, workspace check,
  formatting, live-reference denylist, absence checks, workspace version
  agreement, `git diff --check`, and structural scan of modified live workflow
  files pass.
- **Findings and resolutions**: repeated guidance in `AGENTS.md`, OpenSpec
  config, and the invocation prompt is consumer-specific instruction at one
  OpenSpec seam, not duplicated state or an alternative interface. The X1 sync
  exposed one canonical requirement that still assigned durable dependency
  decisions to legacy `spec/`; the current change modifies that requirement to
  name canonical OpenSpec with ADR/acceptance as supporting evidence. No
  blocking duplicate-owner, shallow-module, unjustified-seam/adapter, or
  approved-design divergence finding remains.
- **Outcome**: `PASS`.
