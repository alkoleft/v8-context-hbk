# Project Rules for Agents

These rules apply to the whole repository.

## Project Context

`v8-context-hbk` is a Rust workspace for reading 1C `*.hbk` help books and extracting structured platform documentation/context from Syntax Assistant books.

The project is currently an independently testable component with provisional contracts. It may later become an HBK-backed source for `/home/alko/develop/open-source/v8-context/`, but do not couple it to unfinished downstream contracts before the extraction model is validated on real HBK data.

OpenSpec is the primary project source of truth.

- Canonical capability requirements live under `openspec/specs/`.
- Proposed and active change scope, design, requirement deltas and task status
  live under `openspec/changes/`; completed change history lives under
  `openspec/changes/archive/`.
- Before planning or implementation, run `openspec list --json`, select the
  applicable live change, and read every context file returned by
  `openspec instructions apply --change <name> --json`.
- Do not hard-code a current change name or archived status in this file. Use
  live OpenSpec state.
- If requested repository-changing work is not covered by an applicable
  apply-ready change, create or update the OpenSpec artifacts before
  implementation.

Existing `spec/` content is supporting legacy documentation, research,
acceptance evidence, ADR rationale and history. A legacy-only contract remains
binding until imported. Before editing code, tests, fixtures, schemas,
generators or adapters in an area governed only by legacy `spec/`, import the
smallest task-relevant contract into the active OpenSpec delta, including
preservation scenarios for behavior that must not change. Do not add new
normative requirements or active task state under `spec/`.

`README.md` remains user-facing CLI documentation. When chat, README, code
comments or supporting documentation conflict with OpenSpec, reconcile them to
the OpenSpec contract before implementation.

## Context Boundaries

Keep context boundaries explicit and narrow.

- Separate HBK container reading, documentation navigation, Syntax Assistant extraction, domain modeling, export adapters and CLI/UI concerns.
- Validation belongs at boundaries: file/container input, external command input, parsing boundaries, serialization/export boundaries and public API boundaries.
- Do not scatter defensive validation through internal flows when data has already crossed a checked boundary and is represented by domain types.
- Prefer typed domain structures over repeated primitive tuples, stringly typed markers or duplicated ad hoc checks.
- Preserve provenance where it matters for diagnostics: HBK file path, entity name, TOC path, HTML path and page title.
- Keep runtime 1C introspection out of this repository unless the project plan explicitly changes. This project extracts documentation from HBK sources.
- Legacy-shaped DTOs or exports are adapters for concrete consumers, not constraints on the internal model.

## Design Principles

Optimize for simple, direct, maintainable code.

- KISS: choose the smallest design that expresses the current behavior clearly.
- YAGNI: do not add compatibility layers, extension points, caches, generic pipelines or configuration knobs until there is a concrete requirement.
- DRY: extract shared behavior when duplication starts encoding the same rule in more than one place. Do not create premature abstractions for merely similar code.
- Big Design Up Front: do enough upfront design to define context boundaries, data contracts, invariants and failure modes before implementation. Do not turn this into speculative architecture for unvalidated future integrations.
- Occam's Razor: when two designs satisfy the same contract, choose the one with fewer moving parts.
- Править не следствия, а причины: при возникновении проблем определяем
  причину и правим именно ее.
- Law of Demeter: keep modules talking through their immediate public
interfaces. Avoid reaching through nested provider internals or binding one
context to another context's representation details.
- SRP: each module/type should have one reason to change.
- ISP: expose small interfaces focused on real consumers. Do not force CLI, extractor, exporter and library users through one broad facade.
- Minimize boilerplate. Prefer language features, small helpers and clear domain types over repetitive templates.

## Testing Rules

Test behavior, not implementation.

- Treat the unit of testing as a unit of behavior: a public API contract, parser outcome, CLI result, export shape, error contract or documented requirement.
- Do not test private implementation details, helper call order, internal struct layout or incidental decomposition.
- Favor externally observable contracts: returned data, errors, diagnostics, serialized output and CLI behavior.
- Use small deterministic fixtures for parser behavior. Fixtures should represent real HBK/Syntax Assistant structures, not invented cases that only satisfy current code.
- Regression tests should describe the user-visible or contract-visible behavior being protected.
- When refactoring without behavior change, existing behavior tests should remain valid.
- Add broader tests when a change crosses module boundaries or changes a public contract; keep tests focused for local implementation changes.

## Subagent Usage

- The user explicitly authorizes and requests subagent use for this repository.
  Treat this section as standing explicit permission to use `spawn_agent` for
  analysis, implementation assistance, test execution and code review when the
  work is non-trivial and subagent tooling is available.
- Use subagents for non-trivial implementation, performance, parser, export, architecture or
  cross-crate changes when subagents are available.
- Prefer independent subagent passes for evidence gathering, test execution and code review before
  finalizing a task with meaningful behavioral or resource-impact risk.
- Keep deterministic repository operations in the main session: spec updates, final verification,
  staging, commits and reconciliation of subagent findings.
- Delegate only bounded, self-contained work with clear read/write scope. Do not use subagents for
  trivial docs-only edits or tasks where delegation would add more coordination than value.
- If subagents are unavailable, continue in the main session and mention the skipped subagent pass
  in the final response or task notes when the task expected one.

## Implementation Workflow

1. Inspect the current branch and worktree. Preserve unrelated user changes and
   do not mix them into the selected task.
2. Discover live state with `openspec list --json`, select the applicable
   change, run `openspec instructions apply --change <name> --json`, and read
   every returned proposal/spec/design/task context file.
3. If the change touches a legacy-only contract, import the smallest relevant
   contract into its OpenSpec delta before the first implementation edit.
4. Form a task-local plan for exactly one pending OpenSpec task and record the
   pre-implementation `mattpocock-skills:codebase-design` pass in the change
   `design.md`. The record must name reviewed scope, module interfaces, seams,
   adapters, owners, findings, resolutions, and a `PASS` or `BLOCKED` outcome.
5. Implement only that task and its direct verification unless the prompt
   explicitly requests a broader batch. Add or update UAT cases when behavior
   is user-visible through CLI, files, exports or diagnostics.
6. Promote requirement changes into OpenSpec. Supporting `spec/` evidence may
   be updated for durable measurements, acceptance results or rationale, but it
   must not become a second requirement or task source.
7. Review the actual diff with `mattpocock-skills:codebase-design` and record the
   second pass in the change `design.md`. Duplicate ownership, shallow
   pass-through modules, unjustified seams/adapters, or structural divergence
   from the approved design block completion unless an owner-approved exception
   is recorded.
8. Run task verification and strict OpenSpec validation, then mark the task
   complete. Use typed errors instead of panics for recoverable input, parsing
   and export failures; do not introduce unrelated refactors.
9. Commit every successful repository-changing task with a task-scoped
   Conventional Commit after inspecting `git diff --cached --name-only` and the
   staged diff. Analysis-, review- and planning-only work does not create an
   empty commit; blocked or failing work is not committed as completed work.

When the last task of an OpenSpec change completes:

1. Classify the version change: bump the workspace minor version for shipped
   user-facing functionality and the patch version otherwise. Bump exactly once
   per completed change and keep `Cargo.toml` and `Cargo.lock` consistent.
2. Complete required review and validation gates, mark the final task
   ready-to-archive, and validate the completed active change strictly.
3. Archive the change with capability-spec synchronization, validate canonical
   OpenSpec state strictly, inspect and stage only task-scoped files, and create
   the completion commit. Do not report the change complete before that commit
   succeeds.

Keep public contracts provisional unless OpenSpec explicitly stabilizes them.
Prefer Rust-native models and algorithms over reproducing Java/Kotlin reference
interfaces. Keep generated outputs and one-off experimental data out of source
files unless the change explicitly requires durable artifacts.
