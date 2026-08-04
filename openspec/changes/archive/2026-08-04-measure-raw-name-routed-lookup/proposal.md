## Why

The completed experiment compared prepared numeric `StringId` keys. It did not
test the proposed identity model in which the primary-name table itself owns
typed identity: a primary type-name row is addressed by `TypeId`, and the row
index is that `TypeId`. A controlled follow-up must compare both variants from
the same raw name and remove `StringId` from the candidate representation and
lookup path.

## What Changes

- Add an experiment-only typed primary table whose rows are unique normalized
  interned names and whose row index is the typed ID.
- Keep aliases in a separate text-sorted `alias -> typed ID` index.
- Normalize the same raw `&str` inside both timed variants, route ASCII-Latin
  names to aliases and other names to primaries, and perform one binary search.
- Apply the same family separation to type, callable and property names. Type
  IDs are table ordinals; callable/property identity includes its owner and
  its family-specific interned primary-name ordinal.
- Restrict conclusions to the requested bilingual corpus: non-ASCII-Latin
  primary names and ASCII-Latin aliases. English-only primaries are excluded.
- Record correctness, construction/allocation/retained-size evidence and raw
  end-to-end latency against the current lookup in the same frozen run.
- Keep production snapshot indexes, public APIs and serialized/X1 layouts
  unchanged; bump the workspace patch version on completion.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `primary-alias-lookup-evaluation`: add the same-input, typed-primary-table
  hypothesis without `StringId` in the candidate.

## Impact

Only the private feature-gated snapshot experiment, deterministic tests,
OpenSpec evidence and workspace patch version change. No production HBK fact,
provider identity, index, reader, cache, X1 schema, public API or downstream
analyzer contract changes.
