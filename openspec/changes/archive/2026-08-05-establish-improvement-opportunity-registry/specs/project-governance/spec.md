## ADDED Requirements

### Requirement: Improvement opportunities are non-authoritative evidence
The repository MUST keep retained but unapproved improvement opportunities in
one registry at `project-governance/IMPROVEMENT_OPPORTUNITIES.md`. A registry
entry MUST NOT be treated as a requirement, roadmap commitment, approved
design, or authorization to implement. Canonical OpenSpec specs and apply-ready
OpenSpec changes MUST take precedence if they conflict with a registry entry.

#### Scenario: Evidence-backed idea is retained
- **WHEN** a potentially useful improvement is not accepted as implementation scope
- **THEN** it can be recorded in the registry without adding it to an OpenSpec task ledger

#### Scenario: Registry entry conflicts with OpenSpec
- **WHEN** an opportunity statement conflicts with a canonical spec or an apply-ready change
- **THEN** OpenSpec controls and the registry entry is reviewed for rejection, supersession, or correction

#### Scenario: Implementation is proposed from an entry
- **WHEN** someone intends to implement a registered opportunity
- **THEN** they create or update a separate apply-ready OpenSpec change before implementation begins

### Requirement: Opportunity records expose evidence maturity and provenance
Every opportunity MUST have a unique stable `IMP-NNNN` identifier, one
evidence disposition, categories, affected areas, an opportunity statement, a
hypothesis, expected value, evidence or origin, constraints and trade-offs, a
promotion trigger, a review trigger, an OpenSpec relationship, and a
last-reviewed date. Evidence disposition MUST be one of `captured`,
`needs-evidence`, `validated-candidate`, `conditional-candidate`, `promoted`,
`rejected`, or `superseded` and MUST describe evidence or decision maturity
rather than execution progress.

#### Scenario: Performance idea has reusable measurements
- **WHEN** benchmark evidence supports a hypothesis but production adoption is not approved
- **THEN** the entry records the measurements and uses an evidence disposition without claiming active work

#### Scenario: Non-performance idea is captured
- **WHEN** a correctness, reliability, security, diagnostics, ergonomics, architecture, testing, documentation, research, or maintenance opportunity has concrete origin or evidence
- **THEN** the same registry schema retains it without requiring performance fields

#### Scenario: Affected contract or promotion intent changes
- **WHEN** an entry is selected for promotion or a material affected contract changes
- **THEN** its evidence is reviewed and the last-reviewed date and disposition are updated before it can inform a proposal

### Requirement: Registry lifecycle cannot become an active task ledger
The registry MUST NOT contain task checkboxes, implementation steps, priority
or scheduling labels, assignees, execution states such as `in-progress` or
`done`, next-entry selection rules, or any other task-status pointer. Promoted,
rejected, and superseded records MUST remain decision provenance rather than
owning execution status.

#### Scenario: Opportunity is promoted
- **WHEN** an apply-ready OpenSpec change accepts scope derived from an opportunity
- **THEN** the registry marks the opportunity `promoted`, links the owning change, and leaves all implementation state in that change's `tasks.md`

#### Scenario: Opportunity is rejected or superseded
- **WHEN** evidence rejects an opportunity or another record or decision replaces it
- **THEN** the registry retains the terminal disposition, rationale, and replacement reference when applicable without creating follow-up tasks

#### Scenario: Registry structure is reviewed
- **WHEN** the registry or its governing contract changes
- **THEN** verification rejects active-task syntax or workflow ownership outside `openspec/changes/*/tasks.md`
