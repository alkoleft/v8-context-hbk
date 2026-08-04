## Why

The HBK snapshot currently materializes some source-backed semantic entities through more than one typed record family. On the frozen 8.3.27 corpus, 7,312 type methods/events are present as both member and callable records and 500 global methods are present as both global and callable records with the same provider source key; the repository needs a complete audit and an explicit single-owner identity invariant before these parallel projections spread into more provider traits and consumers.

## What Changes

- Inventory every HBK semantic entity, generation-local locator, source key, arena, lookup index, relation endpoint, borrowed view and real consumer across owned H0 and mapped X1 snapshots.
- Classify repeated representations as legitimate indexes/views, distinct semantic entities, or true duplicate identity/payload ownership; record deterministic real-corpus evidence for every identified duplicate family.
- For each true duplicate, select one narrow HBK-owned semantic identity and record owner, migrate lookup and relation paths to it, then delete the parallel identity, payload, conversion and consumer pairing in the same implementation slice.
- Preserve provider-owned lookup semantics, source provenance, exact-name/alias behavior, owned/X1 parity and generation locality while removing duplicates.
- Add behavior and structural-absence regressions that fail if one source semantic entity is materialized again under multiple identity owners or equivalent payload records.
- Measure snapshot build time, retained bytes, allocation volume, mapped artifact size and representative lookup latency before and after each accepted removal.
- **BREAKING** Provider-native HBK locator/view combinations may change where the audit proves that a public pair represents one semantic entity; consumers migrate directly without compatibility identities or mirrored adapters.
- Keep cross-provider IDs, shared registries, name-derived identity, persistent/cross-session identity and extension/inheritance/composition rules outside this change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `hbk-fact-snapshot`: Require one canonical HBK identity owner for each source-backed semantic entity while allowing secondary indexes and borrowed role projections that do not create another entity identity or payload owner.
- `borrowed-semantic-role-capabilities`: Require common semantic roles and HBK-specific views to reuse the canonical provider identity and prevent parallel locator pairs or identity conversions from escaping the selected borrowed operation.

## Impact

- Primary owner: `crates/syntax-helper-search` snapshot records, materialization, indexes, relations, memory accounting, borrowed views and H0/X1 serialization/validation.
- Direct in-repository consumers: `crates/context-resolver-search`, focused snapshot/catalog tests, X1 parity fixtures and developer measurement probes.
- External consumers to inventory before implementation: `v8-context` provider composition and any other crates importing concrete `Hbk*Id`, `HbkFactRef`, `StringId` or paired HBK views.
- No new dependency, registry, interner, cache schema or provider-neutral identity family is introduced.
- Completion uses a patch version bump because the change removes duplicated internals/contracts without adding shipped user-facing functionality.
