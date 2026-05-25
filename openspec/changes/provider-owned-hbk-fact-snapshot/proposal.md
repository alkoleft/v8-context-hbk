## Why

Downstream source-backed module workers in `v8-context` need shared access to documented HBK platform and language facts, but the current resolver path keeps lookup state behind `SearchIndex -> rusqlite::Connection`, which is not `Sync`.

The provider should own a worker-safe immutable read model so analyzers can share documented facts as `Arc<_>` without shared SQLite connections, broad resolver locks, or analyzer-owned mirror tables.

## What Changes

- Add a provider-owned HBK fact snapshot contract for documented platform, BSL language and query language facts.
- Materialize the first snapshot from existing `syntax-helper-search` SQLite provider indexes through a bulk loader, not through public N+1 lookup APIs.
- Treat direct HBK book extraction as setup/index-refresh input and measurement comparison, not as the first worker hot-path snapshot source.
- Add compact snapshot DTOs and node ids for owned storage; keep existing resolver DTOs as adapter/projection output.
- Add worker-local resolver handles that borrow/share an immutable snapshot and keep only local trace/cache state.
- Add measurements for build time, RSS delta, estimated heap, node counts, lookup coverage and representative lookup latency.
- Do not change downstream analyzer behavior or implement parallel module analysis in this repository.

## Capabilities

### New Capabilities

- `hbk-fact-snapshot`: provider-owned immutable HBK fact snapshot materialized from provider indexes and safe to share across workers.

### Modified Capabilities

- None.

## Impact

- Affected crates: `syntax-helper-search`, `context-resolver-search`, `context-resolver-core` if public snapshot contracts need shared ids/DTOs.
- Affected specs: `spec/implementation/solution-context-resolve.md`, `spec/implementation/components.md`, `spec/requirements/non-functional.md`, `spec/acceptance/baseline.md`, `spec/IMPLEMENTATION_TODO.md`.
- The SQLite schema remains private rebuildable provider state. The public contract is the immutable snapshot/read-model API, not SQL tables.
