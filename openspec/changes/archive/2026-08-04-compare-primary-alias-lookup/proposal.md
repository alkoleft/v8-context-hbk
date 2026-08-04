## Why

The current HBK snapshot stores normalized primary names and aliases in one
merged lookup index while assigning `Hbk*Id` ordinals independently. The
target identity direction instead makes `TypeId`, `CallableId` and
`PropertyId` the HBK-internal identities, treats scoped primary names as
unique, and uses aliases only as secondary lookup keys; the resource and
behavioral effect of a reusable primary-first/alias-fallback mechanism must be
measured before changing production snapshot storage.

## What Changes

- Add a feature-gated differential benchmark that compares the current merged
  name-index behavior with one generic primary-first/alias-fallback lookup
  implementation over identical canonical rows, normalized keys and query
  mixes. Old/new behavior is compared by canonical source entity outside the
  timed paths because the ID layouts intentionally differ.
- Exercise separate type, callable and property index instances while reusing
  the same generic lookup mechanics; callable/property keys remain scoped by
  their owning `TypeId`, and no common identity registry is introduced.
- Model `TypeId` as the interned canonical type-name token and model
  `CallableId` / `PropertyId` as compact composites of `OwnerId` plus a
  family-local interned canonical-name token, so equal member names under
  different type/global owners remain distinct identities.
- Treat type primaries as unique and verify the corresponding scoped uniqueness
  premise for callable/property identities. Measure primary hits, alias
  fallback hits, misses and primary/alias collisions separately.
- Record release wall time, allocation count, peak heap, retained
  identity/key/index bytes, construction cost and behavioral equivalence on
  non-colliding keys.
- Keep the experiment isolated: it does not rename production IDs, change the
  snapshot schema, alter public lookup contracts or retain both old and new ID
  families in production.

## Capabilities

### New Capabilities

- `primary-alias-lookup-evaluation`: Defines the reproducible old-versus-new
  HBK lookup comparison and the evidence required before an identity/snapshot
  cutover.

### Modified Capabilities

None.

## Impact

- Affected code is limited to feature-gated benchmark/test support and durable
  OpenSpec measurement artifacts in `v8-context-hbk`.
- The benchmark reuses provider-owned normalization semantics and current HBK
  cardinality/alias-shape evidence; it does not read downstream analyzer
  internals.
- No public Rust interface, production reader/materializer, X1 section, cache,
  serializer, CLI behavior or dependency changes.
- Completion is internal experimental work and therefore requires a patch
  version bump, not a minor bump.
