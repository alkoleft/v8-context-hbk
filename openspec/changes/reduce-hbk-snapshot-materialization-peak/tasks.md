## 1. Evidence and Contract

- [x] 1.1 Record three warm release baseline runs of the exact 8.3.27.1859
  provider index, including snapshot build timing, process peak RSS and
  snapshot memory accounting; retain P5a DHAT attribution as upstream evidence.
- [x] 1.2 Validate H1 against the snapshot/read-handle/cache contract and
  record the task-local structure impact and reintroduction guard. The plan
  must decode every SQLite row before classification; cache compatibility is
  verified by unchanged cache/version/field diff, not only roundtrip.

## 2. Provider Implementation

- [x] 2.1 Replace bulk `TypeRefRowSnapshot` collection and grouping passes with
  the private row-at-a-time `TypeRefGroups` collector. Preserve SQL order, row
  validation and existing error/status behavior without a dependency, public
  model, SQLite schema/index, cache owner or analyzer mirror.

## 3. Verification and Handoff

- [x] 3.1 Add focused behavior tests for every type-reference group and its
  order, plus an invalid ignored source row; retain cache/read-handle coverage
  and the named structural guard against every bulk raw-row shape.
- [x] 3.2 Run focused package tests, release measurements and strict OpenSpec
  validation. Accept only if behavior and downstream findings are unchanged,
  normal release RSS improves by at least 10% or 1 MiB, and median build time
  does not regress by more than 10%. Diff review must show no cache layout,
  snapshot/read-handle, SQLite schema, provider-owner or downstream adapter
  changes.
- [x] 3.3 Rebuild the downstream analyzer against this checkout and rerun the
  fixed project-fast workload. Record output digest, median/MAD time and RSS;
  update upstream specification/task ledger and handoff notes.
