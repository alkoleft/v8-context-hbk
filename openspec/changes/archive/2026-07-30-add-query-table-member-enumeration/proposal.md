## Why

The public snapshot resolver can find a query table and point-resolve a named
field or parameter, but it cannot enumerate the fields or parameters of one
resolved table. A downstream context provider would otherwise either scan the
global SDBL context or reach into `HbkFactReadHandle`, bypassing the owning
resolver boundary and its owner-scoped index.

## What Changes

- Add deterministic, owner-scoped field and parameter enumeration methods to
  `context-resolver-search::QueryTableSnapshotSource`.
- Preserve the existing point lookup, source/domain activation checks,
  provenance, aliases, type references, normal absence and snapshot-only
  read-path behavior.
- Cover empty, unknown, inactive/wrong-owner, deterministic order and
  point/enumeration parity through the public adapter API.

## Capabilities

### New Capabilities

- `query-table-member-enumeration`: Provider-owned, deterministic enumeration
  of fields and parameters for a resolved HBK query table.

### Modified Capabilities

- None.

## Impact

- `crates/context-resolver-search`: public `QueryTableSnapshotSource` API and
  its focused contract tests.
- No new dependency, storage format, snapshot index, analyzer model, cache or
  resolver facade. Existing provider-owned `query_fields_by_table` and
  `query_parameters_by_table` indexes are reused.
