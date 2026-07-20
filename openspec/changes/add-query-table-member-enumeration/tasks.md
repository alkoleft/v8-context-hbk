## 1. Provider API

- [x] 1.1 Add owner-scoped deterministic field and parameter enumeration to the existing `QueryTableSnapshotSource`, reusing the snapshot's owner indexes and existing mapping functions.
- [x] 1.2 Preserve normal `NotFound` for inactive, mismatched and unknown table identities and `Ok([])` for known empty owners without reading unrelated table data.

## 2. Contract verification

- [x] 2.1 Add public-adapter tests for deterministic enumeration, mapping/provenance, point/enumeration parity, empty owners and invalid/inactive identities.
- [x] 2.2 Run focused resolver tests, formatting and strict OpenSpec validation; record the verification and mark completed tasks.
