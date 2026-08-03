# ADR-0013: Adopt OpenSpec as the Primary Source of Truth

Date: 2026-08-03.

Status: Accepted.

Decision maker: repository owner.

Supersedes: [ADR-0002](ADR-0002-spec-source-of-truth.md).

## Context

ADR-0002 made `spec/` the only durable source of truth and established
`spec/IMPLEMENTATION_TODO.md` as a separate task-sequencing ledger. Since then,
the repository has adopted OpenSpec for proposals, design, capability deltas,
implementation tasks, strict validation, synchronization, and archival.

T183 exposed the resulting duplication directly: its scope and progress were
recorded both in an OpenSpec change and in `spec/IMPLEMENTATION_TODO.md`. Two
workflow interfaces can diverge and force agents to decide which task state is
current. The repository owner has explicitly chosen OpenSpec as the primary
source of truth and approved removal of the parallel TODO ledger.

## Decision Drivers

- One discoverable interface for current requirements, change design, and task
  status.
- Existing use of validated OpenSpec changes and canonical capability specs.
- Preservation of valuable legacy research and acceptance evidence without a
  bulk rewrite.
- No compatibility ledger or pointer file that recreates the old duplication.
- Explicit completion gates for design quality, version provenance, and Git
  traceability.

## Considered Options

1. Keep `spec/` authoritative and continue the separate TODO ledger.
2. Adopt OpenSpec immediately and delete all existing `spec/` content.
3. Adopt OpenSpec as primary, remove the TODO ledger, retain `spec/` as
   supporting material, and import legacy contracts incrementally when changed.

## Decision

Choose option 3.

- `openspec/specs/` owns canonical capability requirements.
- `openspec/changes/` owns proposed and active change artifacts and task state;
  `openspec/changes/archive/` owns completed change history.
- `spec/IMPLEMENTATION_TODO.md` is removed and MUST NOT be recreated under
  another name or as a pointer-only checklist.
- Existing `spec/` files remain supporting legacy documentation, research,
  acceptance evidence, ADR rationale, and history. They do not receive new
  normative requirements or active task state. A legacy-only contract remains
  a binding baseline until imported.
- Before implementation edits an area governed only by legacy `spec/`, the
  smallest task-relevant contract is imported into an OpenSpec delta spec,
  including preservation scenarios for behavior that must remain unchanged.
  OpenSpec then has precedence for that task scope.
- Every implementation task applies `mattpocock-skills:codebase-design` before
  implementation and to the actual diff before completion. Both pass records
  live in the active change `design.md`; open duplicate-owner, shallow-module,
  unjustified-seam/adapter, or structural-divergence findings block completion.
- Each completed OpenSpec change bumps the workspace version exactly once:
  minor for new shipped user-facing functionality, patch otherwise.
- Successful repository-changing work ends in a verified, task-scoped
  Conventional Commit. Analysis-, review-, and planning-only work creates no
  empty commit.
- A completed change is archived before its completion commit so synchronized
  canonical specs and the archive move are included in that commit.

The repository owner explicitly approved on 2026-08-03 both the migration
boundary and the use of workspace package versions as completed-change
provenance, including documentation and governance changes.

## Consequences

- Agents have one workflow interface and no longer reconcile two task ledgers.
- Existing legacy specifications are not automatically canonical OpenSpec
  capabilities; the next change to each legacy-only contract carries the cost
  of importing it.
- `spec/` remains useful evidence and historical context, but its navigation and
  wording must not imply that it owns new requirements or task state.
- Every completed change creates an explicit version transition and commit.
- Governance checks become stricter even for maintenance changes, adding small
  process cost in exchange for reproducible scope and provenance.

## Implementation Plan

- **Affected paths**: `AGENTS.md`, `openspec/config.yaml`,
  `openspec/changes/adopt-openspec-source-of-truth/`, `spec/README.md`,
  `spec/decisions/`, live operational references under `spec/`,
  `scripts/infr/`, `Cargo.toml`, and `Cargo.lock`.
- **Dependencies**: add none; use the existing OpenSpec CLI and repository
  skills.
- **Pattern to follow**: live discovery through `openspec list --json`, context
  discovery through `openspec instructions apply`, strict validation, then
  archive/synchronize and commit.
- **Pattern to avoid**: no parallel active ledger, hard-coded current change,
  pointer-only replacement for the deleted TODO, bulk legacy-spec conversion,
  or duplicate governance registry.
- **Configuration**: populate `openspec/config.yaml` with repository context and
  artifact rules that make the precedence and completion gates visible while
  generating future changes.
- **Migration**: archive the completed X1 change; add the project-governance
  capability; update live workflow/navigation; remove the obsolete ledger and
  cleanup prompt; bump `0.2.4` to `0.2.5`; validate, review, archive, and commit.
- **Completion order**: check the final ready-to-archive task only after all
  implementation/review/version gates pass; validate the complete change;
  archive and synchronize it; validate canonical state; inspect/stage the final
  task scope; commit. Archive and commit are lifecycle gates outside the
  self-contained task ledger.

## Verification Record

The governance change recorded the following completed evidence before
archival:

- strict validation passed for the active change and for all synchronized
  canonical OpenSpec specs;
- live change discovery no longer reported the completed X1 change, and no
  active changes remained after governance archival;
- the explicit denylist covered `AGENTS.md`, the implementation prompt, legacy
  navigation, UAT rules, and standard verification gates without finding an
  active reference to the removed ledger;
- `spec/IMPLEMENTATION_TODO.md` and the obsolete cleanup prompt were absent;
- `AGENTS.md` required OpenSpec discovery, both codebase-design passes,
  change-level versioning, archival, and a task-scoped commit;
- `Cargo.toml` and `Cargo.lock` agreed on workspace version `0.2.5`;
- ADR readiness, both codebase-design passes, independent review, formatting,
  workspace compilation, and diff checks passed; and
- structural review found no task checklist, first-unchecked selection rule,
  or task-status pointer owned outside `openspec/changes/`.

The Conventional Commit containing this ADR is the lifecycle record for the
final staged-scope inspection. It is not represented as pending task state in
this legacy decision document.

The exact live-reference check is:

```bash
! rg -n 'IMPLEMENTATION_TODO\.md|first[- ]unchecked|active task ledger|Take the first unchecked' \
  AGENTS.md \
  scripts/infr/impl-prompt.md \
  spec/README.md \
  spec/acceptance/test-case-requirements.md \
  spec/acceptance/baseline.md
test ! -e spec/IMPLEMENTATION_TODO.md
```

Historical archives, superseded ADR text, and migration rationale are not live
workflow surfaces and are intentionally outside this command.

## Alternatives Rejected

### Keep `spec/` and the TODO ledger authoritative

Rejected because it preserves the demonstrated duplicate task interface and
makes current OpenSpec usage subordinate to a ledger that already repeats it.

### Delete or bulk-convert all `spec/` content now

Rejected because research, source evidence, acceptance baselines, and detailed
historical implementation notes are not equivalent to capability requirements.
A bulk rewrite would create large review risk without improving the first
deliverable: one owner for new requirements and active work.

## More Information

The normative governance requirements are defined by the OpenSpec capability
`project-governance`. This ADR records the reasoning and supersedes ADR-0002;
it is not a second task ledger.
