## Why

The repository currently records active work in both OpenSpec changes and
`spec/IMPLEMENTATION_TODO.md`, which creates two competing task interfaces and
leaves the declared source-of-truth policy inconsistent with the workflow that
is actually used. OpenSpec is already established and validated in the
repository, so it should become the primary source of truth before new work is
started.

## What Changes

- **BREAKING**: Make OpenSpec the primary source of truth for requirements,
  proposed and active changes, implementation design, and task status.
- **BREAKING**: Remove `spec/IMPLEMENTATION_TODO.md`; OpenSpec change
  `tasks.md` files become the only active task ledger.
- Keep `spec/` as supporting legacy documentation, research, acceptance
  evidence, and historical records. Existing legacy contracts remain the
  binding baseline until imported; do not add new normative requirements or
  active task state there, and import the task-relevant contract into
  `openspec/specs/` before editing implementation in that legacy-only area.
- Supersede ADR-0002 with a new accepted decision that defines OpenSpec
  precedence and the incremental legacy-spec migration rule.
- Make live OpenSpec discovery, artifact reading, strict validation, task
  completion, and archiving the repository implementation workflow.
- Require `mattpocock-skills:codebase-design` before implementation and again
  on the actual diff before an implementation task is completed.
- Require a task-scoped Conventional Commit after successful repository-changing
  work; analysis-, review-, and planning-only work remains non-committing.
- Require a minor workspace version bump when a completed OpenSpec change adds
  shipped user-facing functionality and a patch bump for other completed
  OpenSpec changes. This governance migration therefore bumps `0.2.4` to
  `0.2.5`.

## Capabilities

### New Capabilities

- `project-governance`: Source-of-truth precedence, OpenSpec lifecycle,
  supporting-document migration, mandatory design review, versioning, and
  commit completion rules.

### Modified Capabilities

- `hbk-zero-copy-snapshot-cache`: Replace the remaining requirement that names
  legacy `spec/` as the owner of a durable HBK dependency decision with the
  canonical OpenSpec capability plus supporting ADR/acceptance evidence.

## Impact

- Affected workflow files: `AGENTS.md`, `openspec/config.yaml`,
  `scripts/infr/impl-prompt.md`, and obsolete cleanup prompt material.
- Affected decision and navigation files: `spec/decisions/ADR-0002-*`, a new
  ADR, `spec/decisions/README.md`, `spec/README.md`, and live legacy references
  to `spec/IMPLEMENTATION_TODO.md`.
- Removed active ledger: `spec/IMPLEMENTATION_TODO.md`.
- Version files: workspace `Cargo.toml` and generated `Cargo.lock` entries.
- Canonical capability wording: the dependency-decision requirement in
  `hbk-zero-copy-snapshot-cache` is reconciled to OpenSpec precedence.
- No runtime, CLI, parser, export, storage, or public data-contract behavior
  changes.
