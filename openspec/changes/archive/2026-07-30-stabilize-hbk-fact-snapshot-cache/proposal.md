## Why

T169 stabilized the provider-owned `HbkFactSnapshot` read model and explicit snapshot-backed resolver
adapters. The final T169 measurements accepted a startup/materialization regression because it is
isolated from steady-state analyzer lookup, but they also showed that a derived binary cache can load
the same snapshot much faster than rebuilding it from SQLite.

The current cache code is still measurement-only: it proves the direction, but it does not define a
final provider-owned cache contract, invalidation metadata, corruption handling or runtime loading
boundary.

## What Changes

- Stabilize the persisted snapshot cache as a provider-owned derived artifact over the canonical
  SQLite provider index.
- Define cache metadata and invalidation rules before accepting a runtime cache path.
- Compare the current SQLite materializer with the derived cache on the post-T169 snapshot shape.
- Keep resolver adapters independent from cache layout details: they receive `Arc<HbkFactSnapshot>`
  or read handles, not cache files or binary-layout internals.
- Keep non-query-table `BslLanguage` snapshot migration out of scope. The T171 resolver backend
  split remains complete with `PlatformSnapshotSource` and `QueryTableSnapshotSource`.

## Capabilities

### New Capabilities

- `hbk-fact-snapshot-cache`: provider-owned derived cache for `HbkFactSnapshot` startup/load
  latency, with explicit invalidation and fallback to canonical SQLite rebuild.

### Modified Capabilities

- None.

## Impact

- Affected crates: `syntax-helper-search`; optionally `context-resolver-search` tests only if the
  cache-loaded snapshot must be proven through the existing snapshot-backed resolver boundary.
- Affected specs: `spec/IMPLEMENTATION_TODO.md`, `spec/implementation/solution-context-resolve.md`,
  `spec/acceptance/baseline.md`.
- The SQLite provider index remains the source of truth. The cache is private rebuildable provider
  state and is not a downstream analyzer contract.
