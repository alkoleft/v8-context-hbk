## 1. Design And Measurement Gate

- [x] 1.1 Record the SQLite-first snapshot source decision in repo specs/ADR notes.
- [x] 1.2 Add a measurement harness that bulk-reads an existing provider SQLite index without public lookup APIs.
- [x] 1.3 Measure build time, RSS, estimated heap and node counts on a representative `shcntx_ru` index.
- [x] 1.4 Compare against existing HBK/index build measurements and the downstream N+1 lookup spike.

## 2. Narrow Snapshot Contract

- [ ] 2.1 Add compact provider-owned snapshot DTOs/node ids for platform type/member/callable/global/module/query/language facts.
- [ ] 2.2 Add `Send + Sync` compile-time coverage for the immutable snapshot.
- [ ] 2.3 Add worker-local handle construction from shared immutable snapshot.

## 3. Lookup Surface

- [ ] 3.1 Implement representative snapshot lookups for type, members, callable, global context, module context, query table and language facts.
- [ ] 3.2 Add concurrent deterministic read tests across multiple threads.
- [ ] 3.3 Add adapter/projection tests for existing resolver DTO compatibility where needed.

## 4. Spec Reconciliation

- [x] 4.1 Record measurement results in the OpenSpec design/tasks and `spec/acceptance/baseline.md`.
- [x] 4.2 Update `spec/implementation/solution-context-resolve.md` and `spec/implementation/components.md`.
- [x] 4.3 Update `spec/IMPLEMENTATION_TODO.md` with completion notes or blockers.
