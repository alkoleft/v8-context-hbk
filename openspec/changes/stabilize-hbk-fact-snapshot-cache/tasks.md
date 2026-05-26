## 1. Cache Contract And Invalidation

- [ ] 1.1 Define the provider-owned cache metadata carried by the binary artifact: cache format
  version, provider SQLite schema version, source index identity/hash, platform version/locale when
  available, snapshot layout version/flags and integrity guard.
- [ ] 1.2 Implement mismatch, unsupported-version and corruption/truncation handling so the provider
  layer rebuilds from the canonical SQLite index.
- [ ] 1.3 Keep cache loading behind `syntax-helper-search` provider-owned APIs. Resolver adapters
  receive `Arc<HbkFactSnapshot>` or read handles and do not depend on cache files or binary layout.

## 2. Format Stabilization

- [ ] 2.1 Decide whether the current no-dependency little-endian DTO path is accepted as the first
  stable cache format or remains experimental behind explicit naming.
- [ ] 2.2 If the no-dependency path is not accepted, record the measured bottleneck and ADR/spec
  reason before adding any serialization or zero-copy dependency.
- [ ] 2.3 Add round-trip, invalidation and corrupted-cache tests for the accepted prototype or
  stable format.

## 3. Measurement And Acceptance

- [ ] 3.1 Re-run release measurements on the post-T169 representative `shcntx_ru` provider index,
  comparing SQLite materialization with cache validation/load and cache write when applicable.
- [ ] 3.2 Report warm build/load time, validation cost, process peak RSS, capacity-based heap bytes,
  logical payload bytes, cache file size and representative read-handle lookup timings.
- [ ] 3.3 Prove cache-loaded snapshots preserve existing snapshot and snapshot-backed resolver
  correctness without reopening the T171 backend split.

## 4. Spec Reconciliation

- [ ] 4.1 Update `spec/acceptance/baseline.md` with final T170 measurements and decision.
- [ ] 4.2 Update `spec/implementation/solution-context-resolve.md` with the accepted cache boundary.
- [ ] 4.3 Update `spec/IMPLEMENTATION_TODO.md` with completion notes or blockers.
