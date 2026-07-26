## Why

The analyzer's exact project-fast workload constructs the provider-owned
`HbkFactSnapshot` from the 8.3.27.1859 SQLite index at process startup. Its
downstream DHAT profile attributes 212,585,620 allocated bytes and 98,129,653
bytes live at the global heap maximum to the HBK snapshot materializer. This is
an upstream provider cost, not an analyzer-owned effective-context cost.

The largest directly attributed path is `SnapshotMaterializer::type_refs`:
42,720,841 allocated bytes and 25,420,393 bytes live at the process maximum.
It retains 46,863 fully owned SQLite type-reference rows, then builds four
more grouped snapshot representations from them. The final snapshot is the
correct provider-owned read model; the raw row collection is the avoidable peak.

## What Changes

- Materialize existing type-reference groups as the ordered SQLite rows are
  read, instead of retaining `Vec<TypeRefRowSnapshot>` and grouping afterwards.
- Preserve public read-handle results, error propagation, ordering and binary
  cache readability.
- Verify the change with release provider measurements and downstream finding
  parity on the fixed project-fast workload.

## Capabilities

### New Capabilities

- `hbk-snapshot-materialization-efficiency`: bounded temporary materialization
  of HBK snapshot type references without changing the provider fact contract.

### Modified Capabilities

- None.

## Impact

Only private implementation in `syntax-helper-search::snapshot` changes. No
new crate, dependency, public model, cache owner, analyzer-side mirror, SQLite
schema or 1C semantic rule is introduced.
