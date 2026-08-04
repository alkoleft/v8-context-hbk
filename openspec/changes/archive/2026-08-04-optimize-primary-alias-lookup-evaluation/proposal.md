## Why

The first primary/alias experiment retained parallel key, interner, membership
and entity-ID structures, so its lookup and memory measurements describe an
over-modelled candidate rather than the smallest representation required by
the identity hypothesis. The experiment must be corrected before its resource
results can inform a production decision.

## What Changes

- Replace the retained `KeyId -> name token -> primary ID membership` lookup
  chain with direct primary and alias indexes whose values are the completed
  typed IDs.
- Reuse the snapshot-owned interned normalized string IDs, lookup records and
  matching-range behavior instead of benchmark-local mirrors.
- Keep family-local primary-name token allocation construction-only; aliases
  reference completed IDs and never allocate tokens.
- Remove the redundant retained primary-name interner, primary-ID membership
  set, entity-ID table, lookup record and range-search implementation.
- Re-run the frozen 8.3.27 benchmark and compare the optimized candidate with
  both the same-run dense merged baseline and the recorded over-modelled
  candidate.
- Preserve the experiment-only scope and bump the workspace patch version when
  the completed change is archived.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `primary-alias-lookup-evaluation`: require a direct, non-duplicated retained
  representation and reproducible before/after resource evidence.

## Impact

Only the feature-gated snapshot experiment, its tests and OpenSpec measurement
evidence change. Production snapshot fields, X1/cache layout, provider facts,
public read interfaces, resolver contracts and the shared semantic-role crate
remain unchanged.
