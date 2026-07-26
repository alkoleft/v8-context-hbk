## Context

`HbkFactSnapshot` is the provider-owned immutable read model for repeated
analyzer lookups. `NFR-RESOLVE-001` requires its nodes and physical indexes to
remain in the provider crate and requires separation of snapshot-owned heap
from transient SQLite/materialization memory.

The source index is:

```text
/home/alko/develop/open-source/v8-maintain-projects/v8-context/.v8-context/platform-indexes/8.3.27.1859/shcntx_ru.sqlite
```

P5a DHAT classifies each allocation program point to its first project-owned
frame, so its attribution is disjoint rather than a sum of overlapping stacks.
The direct materializer evidence is:

| Frame | Allocated bytes | Allocations | Live at global peak |
| --- | ---: | ---: | ---: |
| `SnapshotMaterializer::type_refs` | 42,720,841 | 116,594 | 25,420,393 |
| `SnapshotMaterializer::materialize_inner` | 16,226,682 | 151,882 | 9,505,489 |
| `SnapshotMaterializer::query_owner_edges` | 6,954,078 | 42,622 | 0 |
| `SnapshotBuilder::intern` | 6,291,360 | 16 | 3,145,728 |
| `split_lines` | 5,811,810 | 124,079 | 4,566,018 |

The SQLite source contains 25,052 documents and 46,863 type-reference rows;
the selected type-reference text columns total 7,596,457 bytes. Three warm
release baseline processes measure `692 ms` median snapshot construction,
`105592 KiB` peak RSS and unchanged snapshot accounting: `23144545` bytes
total (`7500929` strings, `9034176` node arenas, `6609440` indexes).

## Goals / Non-Goals

**Goals:**

- Remove the full owned type-reference row collection from the materialization
  peak while preserving every existing type-reference group and lookup result.
- Retain the snapshot as the sole provider-fact owner and preserve SQL order.
- Prove behavior, time and memory effect with release measurements.

**Non-Goals:**

- Changing 1C platform facts, BSL/query semantics, source ranking or resolver
  outcomes.
- Adding a cache, changing binary-cache layout/version, SQLite schema/indexes,
  dependencies or analyzer-side fact mirrors.
- Combining independent query-owner, interner, capacity or signature-text
  changes with this primary fix.

## Hypothesis Ledger

| ID | Hypothesis | Evidence / status | Decision |
| --- | --- | --- | --- |
| H1 | Stream `type_refs` into existing groups. | 42.72 MB allocated, 25.42 MB live at global peak and 46,863 owned source rows. Final median: 609 ms and 78,820 KiB RSS versus 692 ms and 105,592 KiB. | Confirmed. |
| H2 | Stream `query_owner_edges`. | 6.95 MB allocated, zero bytes live at global peak. | Deferred; measure after H1. |
| H3 | Remove builder's `Vec<String>` plus `BTreeMap<String, StringId>` duplicate ownership. | 6.29 MB attributed; exact duplicate share is not isolated. | Deferred. |
| H4 | Pre-size temporary collections. | Plausible but no material capacity-growth attribution. | Deferred. |
| H5 | Load the existing binary cache in analyzer startup. | Earlier cache benefit exists, but analyzer intentionally materializes SQLite today. | Separate integration decision. |
| H6 | Explicitly drop current `type_refs` after grouping. | Shortens lifetime only; retains 42.72 MB source allocation and duplicate group work. | Rejected in favor of H1. |
| H7 | Borrow `signature_text` in `split_lines`. | 5.81 MB direct allocation from cloning full text per signature. | Deferred as an independent small fix. |
| H8 | Correct 1C semantic mismatch. | Profile shows storage lifetime; no wrong 1C fact or lookup behavior is evidenced. | Out of scope. |

## Decision

### One private row-at-a-time type-reference collector

Replace `TypeRefRowSnapshot` plus four `group_*type_refs` passes with a
private `TypeRefGroups` value. Its collector keeps the existing ordered SQL
statement and decodes every row through `snapshot_type_ref_from_row` before
classifying it. This preserves the current error behavior even for a decoded
row that no group consumes. It calls `map_type_ref` only after a row matches
one existing group predicate, then appends it to that group family:

- document `(source_document_id, ref_kind)` references;
- document return references;
- signature return references;
- `(signature_id, parameter_ordinal)` parameter references.

Only final `HbkTypeRef` group payloads remain after a SQLite row advances.
`TypeRefGroups` privately owns four existing temporary maps; it is not a fact
model, public DTO, cache record or adapter. `HbkFactSnapshot` and its read
handle remain the single public interface. The materializer stays a deep
private implementation behind that interface.

### Ordering and Cache Compatibility

The four existing category predicates and SQL ordering remain unchanged, so
every group preserves its deterministic row order. `StringId` insertion order
is private physical state and may change while read-handle behavior remains
unchanged. The binary-cache layout and invalidation metadata are not modified.
Existing-cache readability is a diff-review invariant: `binary_cache.rs`,
serialized snapshot fields, cache layout version and cache metadata code must
remain untouched. A normal cache round-trip remains behavior coverage, not
proof alone that an earlier cache is readable.

## Structure Impact

Searched owners and consumers: `syntax-helper-search` materializer, indexes,
binary cache, read handle, memory accounting, examples and tests; downstream
`analyze-project` snapshot loader and `context-resolver-search` adapters;
workspace manifests and sibling analyzer DHAT artifacts. Search terms:
`TypeRefRowSnapshot`, `type_refs`, `group_*type_refs`, `HbkTypeRef`,
`HbkFactSnapshot`, `SnapshotBuilder`, `from_index`, `from_path`,
`write_binary_cache` and `worker_handle`.

The existing semantic owner remains `HbkFactSnapshot` with its strings and
fact arenas. The task deletes the private raw `TypeRefRowSnapshot` row family
and its bulk reader/grouping behavior. It adds one private `TypeRefGroups`
holder and one row-at-a-time collector, reusing existing `SearchTypeRef`,
`HbkTypeRef`, `SnapshotBuilder`, indexes, binary serialization and read-handle
projections. No public/JSON contract, cache record/key, schema, adapter,
mapping, parser, resolver, serializer or dependency changes.

## Reintroduction Guard

Root cause: a full raw SQL type-reference collection overlapped with its grouped
snapshot projection. The sole valid flow is ordered row -> `TypeRefGroups` ->
existing `HbkFactSnapshot`. Implementation review must reject a restored
`TypeRefRowSnapshot`, a `fn type_refs(&self) -> Result<Vec<...>>` reader, or a
full raw `Vec` carrying source document/ref kind/signature/parameter/fact values
before grouping. Verification includes read-handle parity, cache behavior and a
source search for every prohibited shape.

## Risks / Trade-offs

- [Classification/validation drift] decode every row before filtering; test
  document, return, signature-return and parameter references in order, plus
  an invalid row in an otherwise ignored category.
- [Memory moves rather than falls] compare process RSS and snapshot accounting.
- [Startup regression] reject material build-time regression despite RSS gain.
- [Private `StringId` order] verify read-handle and cache behavior, not ids.

## Results

Three warm release processes used the exact same 8.3.27.1859 SQLite index and
the `measure_hbk_fact_snapshot` example before and after the change.

| Metric | Baseline | Final runs | Final median | Delta |
| --- | ---: | --- | ---: | ---: |
| Snapshot build | 692 ms | 613 / 609 / 605 ms | 609 ms | -12.0% |
| Process peak RSS | 105,592 KiB | 78,820 / 78,824 / 78,820 KiB | 78,820 KiB | -26,772 KiB (-25.35%) |
| Snapshot-accounted heap | 23,144,545 bytes | 23,144,545 bytes each | 23,144,545 bytes | unchanged |

The acceptance gate passes: RSS improves by more than 10%, and median build
time improves rather than regressing. The unchanged final snapshot accounting
confirms that the reduction is in transient raw-row materialization, not a
change to provider facts or the published snapshot model.

The downstream fixed `project-fast` workload also retained the exact finding
digest `c2b4465a4c66a8939d40febd117061959e85dcde77078be386c1e73ae97f60a3`
with zero findings. Its five final runs had a 0.75 s median and 89,108 KiB
median RSS, compared with the P5a 0.83 s and 108,332 KiB medians: -9.6% time
and -17.7% RSS. This is parity evidence; the provider acceptance result above
is the T174 gate.

`binary_cache.rs`, cache layout/version and serialized snapshot fields remain
outside the diff. Existing cache/read-handle behavior tests pass, but the
untouched-layout diff is the compatibility evidence for earlier cache files.

`HbkFactSnapshotStageTimings::group_type_refs` now intentionally measures the
type-reference SQL read, decode and grouping as one stage; `read_sql_rows`
excludes that work. Future stage-level comparisons must use this post-T174
meaning rather than treating the two timing buckets as directly comparable to
older reports.
