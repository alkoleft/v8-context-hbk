## 1. Remove the duplicated candidate representation

- [x] 1.1 Replace benchmark-local key/lookup/range mirrors and retained interner/membership/entity-ID projections with snapshot-owned `StringId`, existing lookup records, `matching_range` and direct primary/alias vectors containing completed typed IDs.
- [x] 1.2 Update deterministic layout, owner-isolation, collision, differential and structural-absence tests so construction-only token allocation and transient oracle mappings cannot become retained candidate state again.

## 2. Re-measure the frozen corpus

- [x] 2.1 Run focused/full feature verification and the isolated allocation-enabled release benchmark against the frozen 8.3.27 corpus with the same query classes and sample count.
- [x] 2.2 Record the fresh control, optimized measurements, same-run baseline comparison, cross-run candidate comparison, corpus provenance, duplicate blockers and bounded conclusion in `measurements.md`.

## 3. Completion gates

- [x] 3.1 Review the actual diff with codebase-design and a fresh reviewer, reconcile it with `Structure impact`, run the exact `Reintroduction guard` searches and resolve every finding.
- [x] 3.2 Bump the workspace patch version, run strict OpenSpec/workspace validation, mark tasks complete, archive/synchronize the change, inspect staged scope and create the task-scoped Conventional Commit.
