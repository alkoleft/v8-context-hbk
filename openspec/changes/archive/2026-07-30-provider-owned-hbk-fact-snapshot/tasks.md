## 1. Design And Measurement Gate

- [x] 1.1 Record the SQLite-first snapshot source decision in repo specs/ADR notes.
- [x] 1.2 Add a measurement harness that bulk-reads an existing provider SQLite index without public lookup APIs.
- [x] 1.3 Measure build time, RSS, estimated heap and node counts on a representative `shcntx_ru` index.
- [x] 1.4 Compare against existing HBK/index build measurements and the downstream N+1 lookup spike.

## 2. Narrow Snapshot Contract

- [x] 2.1 Add compact provider-owned snapshot DTOs/node ids for platform type/member/callable/global/module/query/language facts.
- [x] 2.2 Add `Send + Sync` compile-time coverage for the immutable snapshot.
- [x] 2.3 Add worker-local handle construction from shared immutable snapshot.

## 3. Lookup Surface

- [x] 3.1 Implement representative snapshot lookups for type, members, callable, global context, module context, query table and language facts.
- [x] 3.2 Add concurrent deterministic read tests across multiple threads.
- [x] 3.3 Add adapter/projection tests for existing resolver DTO compatibility where needed.

## 4. Spec Reconciliation

- [x] 4.1 Record measurement results in the OpenSpec design/tasks and `spec/acceptance/baseline.md`.
- [x] 4.2 Update `spec/implementation/solution-context-resolve.md` and `spec/implementation/components.md`.
- [x] 4.3 Update `spec/IMPLEMENTATION_TODO.md` with completion notes or blockers.

## 5. Explicit Resolver Backend Split

- [x] 5.1 Complete and align explicit snapshot-backed `context-resolver-search` adapters, including
  the already introduced `PlatformSnapshotSource` and `QueryTableSnapshotSource`, composed from
  provider-owned `HbkFactSnapshot` / `HbkFactReadHandle` state. Add or rename a broader
  `LanguageSnapshotSource` only if the migrated slice covers non-query-table language facts.
- [x] 5.2 Keep `PlatformSearchSource` and `LanguageSearchSource` as explicit
  SQL/SearchIndex-backed backends for CLI, debug, index inspection and sequential local resolver
  scenarios, explicitly excluding downstream analyzer hot paths.
- [x] 5.3 Project snapshot facts into existing `context-resolver-core` DTOs without duplicating
  provider-fact mirror indexes, copying broad DTO payloads into snapshot storage or adding
  analyzer-owned fallback tables.
- [x] 5.4 Prove the snapshot-backed adapter/resolver boundary is `Send + Sync`, or prove an
  explicit scoped-worker borrow contract. The proof must cover the source/resolver composition, not
  only `HbkFactSnapshot`, and must not rely on broad `Arc<Mutex<_>>` / `Arc<RwLock<_>>` wrappers
  around resolver/search state, SQLite connections or mutable adapter internals.
- [x] 5.5 Add focused resolver tests for platform type lookup, member lookup by owner/name/kind,
  callable lookup by owner/name, global context lookup, module context lookup,
  related/availability lookup, query table lookup by name/syntax/identifier, query field and query
  parameter lookup by table/name, and source/domain identity preservation for all source families
  migrated into snapshot-backed adapters. If non-query-table BSL-language facts are not migrated,
  add explicit unsupported/empty tests for those lookups and prove they do not fall back to
  `LanguageSearchSource` or SQL/SearchIndex.
- [x] 5.6 Add regression tests proving SQL/SearchIndex-backed scenarios still work and are selected
  explicitly, with no hidden fallback from snapshot-backed resolver methods to SQL/SearchIndex.
  Compose snapshot-backed sources from an already materialized in-memory snapshot, make the source
  SQLite path unavailable or absent, verify migrated lookups still work, and verify missing snapshot
  coverage returns the documented unsupported/empty result rather than using SQL/SearchIndex.
- [x] 5.7 Either migrate enum and enum-value exact-id/relation participation through the
  snapshot-backed adapter slice, or explicitly document and test that the migrated slice excludes
  those facts with the documented unsupported/empty result.
