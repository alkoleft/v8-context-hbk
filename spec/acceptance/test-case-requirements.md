# Test Case Requirements

This file defines how UAT and acceptance test cases are specified. It is about test-case shape, not
test execution reports.

## Scope

UAT test cases validate externally observable behavior:

- CLI commands
- HBK input files
- JSON export files
- diagnostics
- exit codes
- cleanup behavior

UAT cases must not validate private implementation details, internal helper call order, crate layout
or incidental decomposition.

## Required Fields

Each UAT case must include:

- stable ID, for example `UAT-HBK-001`
- title
- related use cases
- related requirements or ADRs
- preconditions
- inputs
- steps
- expected result
- pass/fail criteria
- cleanup rules
- notes for skipped execution when platform fixtures are unavailable

## Traceability

- Every UAT case must link to at least one use case or requirement.
- OpenSpec change tasks may reference UAT IDs as verification gates, but must
  not duplicate full UAT steps.
- If a UAT case reveals a contract change, update the applicable OpenSpec
  requirement first, then update the UAT case and change task.

## Artifact Policy

Raw command outputs and generated export directories are service data. Keep
them under `target/` or another ignored runtime location unless an OpenSpec
change explicitly asks for a durable artifact. Durable conclusions belong in
canonical OpenSpec specs or change artifacts and may be supported by
`acceptance/baseline.md` or ADR rationale.
