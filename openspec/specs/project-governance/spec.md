# project-governance Specification

## Purpose
Define OpenSpec as the repository's primary capability and active-change
source of truth, including task ownership and archival rules.
## Requirements
### Requirement: OpenSpec is the primary source of truth
The repository MUST use canonical specs under `openspec/specs/` as the primary
source of truth for capability requirements and MUST use artifacts under
`openspec/changes/` as the source of truth for proposed or active change scope,
design, and task status.

#### Scenario: Agent starts repository-changing work
- **WHEN** an agent prepares to plan or implement repository-changing work
- **THEN** it discovers live changes with `openspec list --json` and reads the selected change artifacts returned by OpenSpec before editing

#### Scenario: OpenSpec and supporting documentation conflict
- **WHEN** an OpenSpec capability or active change conflicts with supporting material under `spec/`
- **THEN** the OpenSpec contract takes precedence and the supporting material is reconciled without creating another active ledger

### Requirement: Active tasks have one owner
OpenSpec change `tasks.md` files MUST be the only active implementation task
ledger. The repository MUST NOT recreate `spec/IMPLEMENTATION_TODO.md` or an
equivalent parallel checklist, pointer ledger, or first-unchecked-task file.

#### Scenario: New implementation scope is requested
- **WHEN** requested repository-changing work is not covered by an applicable active OpenSpec change
- **THEN** a new OpenSpec change is created and made apply-ready before implementation begins

#### Scenario: Work is completed
- **WHEN** all tasks and completion gates for an OpenSpec change pass
- **THEN** the change is archived and its capability deltas are synchronized to canonical OpenSpec specs

### Requirement: Legacy specifications are supporting material
Existing documents under `spec/` MUST remain supporting legacy documentation,
research, acceptance evidence, ADR rationale, and history. New normative
requirements and active task state MUST NOT be added there. A legacy-only
contract MUST remain a binding baseline until its task-relevant requirements
are imported into OpenSpec.

#### Scenario: Implementation touches a legacy-only contract area
- **WHEN** an implementation task will edit code, tests, fixtures, schemas, generators, or adapters governed only by legacy `spec/` material
- **THEN** the smallest task-relevant contract, including preservation scenarios for unchanged behavior, is imported into the change's OpenSpec delta spec before the first implementation edit

#### Scenario: Evidence changes after implementation
- **WHEN** implementation produces durable measurements or acceptance evidence
- **THEN** supporting evidence under `spec/` may be updated and linked from the OpenSpec change without becoming a parallel requirement or task source

### Requirement: Codebase design review is mandatory
Every implementation task MUST apply `mattpocock-skills:codebase-design` to the
task-local plan before implementation and to the actual diff before completion.
Both passes MUST be recorded in the active change `design.md` with reviewed
scope, module interfaces/seams/adapters and owners, findings, resolutions, and
a `PASS` or `BLOCKED` outcome. Duplicate ownership, shallow pass-through
modules, unjustified seams/adapters, and structural divergence from the approved
design MUST block completion unless the repository owner explicitly accepts and
records an exception.

#### Scenario: Implementation is about to begin
- **WHEN** a task will edit implementation code, tests, fixtures, schemas, generators, or adapters
- **THEN** its planned module interfaces, seams, adapters, ownership, and expected structural impact are reviewed with `mattpocock-skills:codebase-design` before editing

#### Scenario: Implementation diff is ready
- **WHEN** implementation and direct verification are complete
- **THEN** the actual diff is reviewed with `mattpocock-skills:codebase-design`, its durable record reports `PASS`, and no blocking finding remains before task completion

### Requirement: Completed changes update the workspace version
Each completed OpenSpec change MUST update the workspace version exactly once.
The change MUST use a minor bump when it adds shipped user-facing functionality
and a patch bump otherwise, and `Cargo.toml` and `Cargo.lock` MUST agree.

#### Scenario: User-facing functionality ships
- **WHEN** a completed OpenSpec change adds shipped user-facing functionality
- **THEN** the workspace minor version is incremented once for that change

#### Scenario: Non-user-facing change completes
- **WHEN** a completed OpenSpec change does not add shipped user-facing functionality
- **THEN** the workspace patch version is incremented once for that change

#### Scenario: Documentation or governance change completes
- **WHEN** a completed OpenSpec change changes only documentation or governance
- **THEN** the workspace patch version is incremented because the repository owner uses the workspace version as completed-change provenance

### Requirement: Repository-changing work is committed
Successful repository-changing work MUST end with a verified task-scoped
Conventional Commit. The staged file list and staged diff MUST be inspected
before commit, and unrelated changes MUST NOT be included.

#### Scenario: Repository-changing work passes its gates
- **WHEN** task implementation, verification, review, versioning, and OpenSpec completion gates pass
- **THEN** the completed change is archived and synchronized, canonical OpenSpec state is validated, and only task-scoped changes are staged and committed before the work is reported complete

#### Scenario: Work produces no repository changes
- **WHEN** the request is analysis-, review-, or planning-only and leaves no repository changes
- **THEN** no empty commit is created

#### Scenario: Work is blocked or verification fails
- **WHEN** required verification or review has not passed
- **THEN** the work is not marked or committed as completed work

### Requirement: Parallel ledgers are rejected structurally
The repository MUST reject a replacement active-task file, pointer ledger, or
first-unchecked-task workflow outside OpenSpec, regardless of its filename.

#### Scenario: Completion review inspects workflow ownership
- **WHEN** a governance or workflow diff is reviewed for completion
- **THEN** the reviewer checks all added and modified live workflow files for task checkboxes, first-unchecked selection rules, or task-status pointers owned outside `openspec/changes/` and blocks any parallel owner

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
