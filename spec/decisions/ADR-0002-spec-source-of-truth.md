# ADR-0002: Use spec/ as the Only Durable Source of Truth

Date: 2026-04-30.

Status: Accepted.

## Context

The repository originally used one large planning document,
`spec/hbk-components-requirements-plan.md`, plus `spec/IMPLEMENTATION_TODO.md`. That document mixed
requirements, source evidence, users/jobs, non-functional requirements, milestones, epic specs,
historical task records, acceptance conclusions and next steps.

This made it harder for agents to know which layer was normative:

- requirements and task history lived together;
- the active task ledger duplicated specification text;
- completed milestone records competed with current acceptance rules;
- UAT test cases had no explicit home;
- README and agent instructions could accidentally become competing truth surfaces.

## Decision

`spec/` is the only durable source of truth for product, architecture, acceptance and implementation
intent.

The source-of-truth layers are:

1. Accepted ADRs in `spec/decisions/`.
2. Requirements, use cases, acceptance and implementation specifications under `spec/`.
3. `spec/IMPLEMENTATION_TODO.md` as task sequencing only.
4. `spec/archive/` as historical evidence, not active scope.

`README.md` remains end-user documentation. `AGENTS.md` remains process guidance for agents. Neither
file may define product or implementation contracts independently from `spec/`.

## Consequences

- The former monolithic requirements/plan document is split into layered specification files.
- Active tasks reference requirement, UAT and ADR IDs instead of carrying full contract text.
- Completed T0-T12 task history moves to `spec/archive/`.
- New user-visible behavior gets UAT coverage in `spec/acceptance/uat-test-cases.md`.
- Durable conclusions from acceptance runs live in `spec/acceptance/baseline.md`, not in one-off run
  reports.

## Alternatives Considered

### Keep the monolithic requirements/plan document

Rejected. It preserves too much ambiguity between requirements, implementation tasks and history.

### Make README the project source of truth

Rejected. README should stay focused on user-facing CLI usage and should not become a dense
engineering contract.

### Use IMPLEMENTATION_TODO.md as the source of truth

Rejected. The task ledger should answer "what is the next scoped work item", not "what is the
system contract".

## Implementation Plan

- Replace `spec/hbk-components-requirements-plan.md` with layered files:
  - `spec/source-evidence.md`
  - `spec/requirements/functional.md`
  - `spec/requirements/non-functional.md`
  - `spec/use-cases.md`
  - `spec/implementation/components.md`
  - `spec/acceptance/baseline.md`
  - `spec/acceptance/test-case-requirements.md`
  - `spec/acceptance/uat-test-cases.md`
- Rename the existing integration decision to numbered ADR form.
- Keep `spec/README.md` as the index and precedence policy.
- Reduce `spec/IMPLEMENTATION_TODO.md` to the active implementation ledger.
- Update `README.md`, `AGENTS.md` and `scripts/infr/impl-prompt.md` to point to the new structure.

## Verification

- [x] `rg "hbk-components-requirements-plan|v8-context-integration-decision" README.md AGENTS.md spec/README.md scripts` returns no live references.
- [x] `spec/README.md` defines source-of-truth precedence.
- [x] `spec/IMPLEMENTATION_TODO.md` contains active tasks only and points completed T0-T12 history to `spec/archive/`.
- [x] UAT test case rules and at least one UAT catalog file exist under `spec/acceptance/`.
- [x] `git diff --check` passes.
