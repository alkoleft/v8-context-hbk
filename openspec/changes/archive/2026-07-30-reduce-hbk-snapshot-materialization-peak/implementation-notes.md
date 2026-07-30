## Task-Local Implementation Plan

1. Keep `HbkFactSnapshot` and `HbkFactReadHandle` as the only external
   interface. Add no adapter or public type.
2. In `snapshot/materialize.rs`, delete the bulk raw type-reference row model,
   its `Vec`-returning reader and four later grouping functions. Add private
   `TypeRefGroups` holding the exact four existing `BTreeMap` group families.
3. Add one private `&mut SnapshotMaterializer` collector using the existing
   ordered SQL query. For each row, first call `snapshot_type_ref_from_row`;
   only then apply the exact existing group predicates and call `map_type_ref`
   for a matching row. This keeps errors from otherwise ignored rows terminal.
4. Consume the four groups at the existing assembly sites. Do not touch
   query-owner streaming, the string interner, signature splitting, binary
   cache, SQLite schema, resolver adapters or analyzer code.
5. Add public read-handle tests for all four groups and their order; mutate an
   ignored type-reference row into an invalid one and assert the existing
   `SearchError`; preserve cache roundtrip behavior. Add the specific structural
   absence guard stated in `design.md`.
6. Pass gates: three release final runs must reduce peak RSS by at least 10% or
   1 MiB from `105592 KiB`, with median build time no worse than 10% above
   `692 ms`. The downstream five-run check is parity evidence only.

## Design Review

`HbkFactSnapshot` is the deep module: callers use one stable read handle while
the materializer owns decoding, grouping and index assembly. `TypeRefGroups` is
a private implementation value that consolidates existing temporary maps;
exposing it or adding an adapter would make the interface shallower without a
second consumer. The plan improves locality and allocation lifetime while
preserving the sole external seam.

## Skeptic Review Revision

The initial plan incorrectly permitted filtering before decoding. The revised
plan decodes every row first, adds invalid-ignored-row and per-group ordering
tests, treats prior-cache compatibility as an untouched-layout diff invariant,
and makes numeric acceptance gates explicit. It retains the documented
structure impact and reintroduction guard.

## Verification Results

- Focused ordering, invalid-ignored-row, representative snapshot/read-handle
  and binary-cache tests pass; the source guard rejects every prohibited bulk
  raw-row shape.
- `cargo test -p syntax-helper-search` passes with 59 tests, and `cargo fmt
  --all --check` passes.
- Release provider measurements on the exact index improve build median from
  692 ms to 609 ms and peak RSS median from 105,592 KiB to 78,820 KiB. The
  23,144,545-byte final snapshot accounting is unchanged.
- The rebuilt downstream five-run `project-fast` workload preserves the
  `c2b4465a4c66a8939d40febd117061959e85dcde77078be386c1e73ae97f60a3`
  finding digest and zero findings, with 0.75 s / 89,108 KiB medians versus
  P5a's 0.83 s / 108,332 KiB.
- The implementation diff leaves binary-cache code/layout, SQLite schema,
  resolver adapters and analyzer code unchanged.
