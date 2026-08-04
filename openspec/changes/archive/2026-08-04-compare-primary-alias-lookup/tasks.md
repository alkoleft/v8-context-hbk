## 1. Lookup contract

- [x] 1.1 Add the test-only snapshot experiment module with a four-byte interned-name `TypeId`, compact `OwnerId`, separate `CallableNameId` / `PropertyNameId`, eight-byte composite `CallableId` / `PropertyId`, and one generic `PrimaryAliasLookup<Scope, Id>` used by three independent instances.
- [x] 1.2 Add deterministic behavior/layout tests for primary hits, alias fallback, alias ambiguity, primary-over-alias precedence, missing aliases, same-name token reuse across different owners, global/type owner isolation and callable/property interner isolation.
- [x] 1.3 Add the deliberate dense-ordinal merged-name reference and exact canonical-entity differential checks for every non-colliding query class without remapping either timed lookup.

## 2. Frozen corpus comparison

- [x] 2.1 Project the exact fact families from the design projection table through the named snapshot-owned views into one shared benchmark corpus without SQL-schema, storage-internal or downstream-analyzer reads.
- [x] 2.2 Implement the explicitly temporary stable duplicate-primary filter, canonical type-owner mapping and per-family duplicate counters shared by both variants.
- [x] 2.3 Add the exact `V8_CONTEXT_HBK_PRIMARY_ALIAS_INDEX` ignored runner from the design with metadata validation, common key preparation, deterministic query classes, warm-up, at least seven measured samples, checksums, local vector/key retained-byte accounting and observations from the one existing experiment allocator; verify missing/mismatched corpus input fails clearly.

## 3. Evidence and verification

- [x] 3.1 Run focused debug/release behavior tests with and without snapshot experiment features, compile the ignored runner under `snapshot-experiment-alloc`, prove expected construction allocations are non-zero and verify existing snapshot structural guards remain green without declaring another global allocator.
- [x] 3.2 Run the frozen 8.3.27 comparison alone with allocation observation enabled and record commands, provenance, environment, corpus/invariant counters and median old/new results in `measurements.md`.
- [x] 3.3 Record the bounded conclusion, including lookup/resource trade-offs, collision semantics and whether the hypothesis merits a separate production change; any non-zero family duplicate count must explicitly block cutover on formation/extension composition.

## 4. Completion gates

- [x] 4.1 Review the actual diff with the codebase-design and reviewer gates, reconcile every structure/conversion with `Structure impact`, run the exact manual structural commands in `Reintroduction guard`, resolve findings and update the actual-diff review record to PASS.
- [x] 4.2 Bump the workspace patch version for completed internal experimental work, run strict OpenSpec/workspace validation and inspect the staged task scope.
- [x] 4.3 Mark tasks complete, archive/synchronize the change and create the required task-scoped Conventional Commit.
