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
