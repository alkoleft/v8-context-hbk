## Why

Useful follow-up ideas currently remain scattered across experiment reports and
conversation context, so they are easy to lose or mistake for approved work.
The repository needs one durable discovery surface for evidence-backed
opportunities while preserving OpenSpec changes as the only source of active
scope and task status.

## What Changes

- Add one non-authoritative improvement-opportunity registry under
  `project-governance/` for performance, architecture, correctness,
  reliability, diagnostics, documentation, and other improvement categories.
- Give every opportunity a stable identifier, evidence-maturity disposition,
  evidence and constraints, and an explicit trigger for promotion into an
  apply-ready OpenSpec change.
- Forbid task checkboxes, priority or scheduling labels, assignees, execution
  status, and next-task selection in the registry so it cannot become a
  parallel backlog.
- Seed the registry with the measured ID-only hash-index opportunity and the
  conditional `string-interner` ownership opportunity.
- Classify completion as a patch version bump because this changes only
  documentation and project governance.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `project-governance`: Define the ownership, allowed content, lifecycle, and
  OpenSpec promotion boundary for non-authoritative improvement opportunities.

## Impact

The change adds one governance document, modifies the canonical
`project-governance` contract after archive, and updates workspace version
provenance. It changes no Rust code, public API, serialized format, runtime
behavior, or dependency.
