## Context

The downstream P5a allocation profile identified several independent costs in
the provider-owned SQLite-to-`HbkFactSnapshot` path. T174 has already removed
the dominant raw type-reference vector, reducing normal provider median RSS
from 105,592 KiB to 78,820 KiB without changing final snapshot accounting.
The remaining profile evidence is not a license to combine unrelated changes:

| ID | Candidate | Current evidence | Owner / status |
| --- | --- | --- | --- |
| H2 | Stream `query_owner_edges`. | 6,954,078 allocated bytes; 21,304 owns rows, of which only 2,934 enum-value rows survive. | `syntax-helper-search` materializer; ready for a separate task. |
| H3 | Remove temporary string-interner duplicate ownership. | 6,291,360 allocated bytes; map dies at snapshot assembly but exact reclaim design is unproven. | `SnapshotBuilder`; measure/design first. |
| H4 | Pre-size temporary collections. | `Vec::new`/`BTreeMap::new` sites exist, but no attributable growth metric. | Unmeasured; do not implement speculatively. |
| H5 | Use derived binary cache in analyzer startup. | Cache loading is 26-31x faster in earlier provider evidence, but normal analyzer startup never selects it. | Cross-repository provider startup decision; deferred. |
| H6 | Explicit drop after grouping. | Superseded by T174's row-at-a-time flow. | Rejected. |
| H7 | Extract only the selected signature line. | Borrowed-input experiment was allocation-identical and reverted. Direct selection deletes the source-level all-lines vector and selected-line clone; parity/no-regression passes, but `split_lines` first-frame attribution remains 5,811,810 bytes, so no allocation/time/RSS improvement is claimed. | Accepted as P5b supportability deletion. |
| H8 | Merge type-reference groups based on 1C semantics. | No semantic defect is evidenced. Official 1C material separates document type families, function returns and parameter descriptions. | Rejected. |

The documented 1C sources are [Documents](https://its.1c.ru/db/pubdevguide83/content/615/hdoc),
[Procedure and function descriptions](https://its.1c.ru/db/content/v8std/src/400/100/i8100453.htm),
[documentation comments](https://its.1c.ru/db/edtdoc/content/10359/hdoc) and
[TypeDescription use](https://its.1c.ru/db/metod8dev/content/2611/hdoc). They
support context-specific analyzer facts; `signature return` remains an internal
normalization term whose mapping to the documented function return is an
inference, not an external 1C name.

## Goals / Non-Goals

**Goals:**

- Remove individually measured temporary copies with a direct owner-local
  change and observable snapshot/read-handle parity.
- Use release measurements and the fixed downstream workload to distinguish
  reduced allocation from improved process RSS or time.
- Keep an explicit result/disposition for every remaining H2-H8 hypothesis.
- Preserve the semantic distinction among document, callable return, signature
  return and parameter type references.

**Non-Goals:**

- Changing 1C/BSL facts, source ranking, invalid/unknown semantics, grouping
  meaning or fact deduplication.
- Changing SQLite schema/indexes, binary-cache layout/version, serialized
  snapshot fields, resolver adapters, analyzer-owned cache policy or public
  provider API without a dedicated accepted change.
- Adding an interner replacement, capacity hints, cache plumbing, telemetry or
  compatibility layer before its owner and gate are demonstrated.

## Decisions

### 1. H7 borrows only the selected signature line into the existing builder

The release DHAT experiment proves that changing `split_lines(String)` to
`split_lines(&str)` is allocation-identical: inlining already removes the
owned-input clone. The real root cause is
`split_lines(...).get(ordinal).cloned()`: it creates owned strings for all
lines and then creates another owned selected string before `SnapshotBuilder`
interns it.

`DocumentRow.signature_text` already lives through signature construction.
`signatures_by_callable` will select the same non-empty `ordinal` line with
`lines`, the existing empty-line predicate and `nth`, then pass that `&str`
directly to the existing builder. It adds no helper, no borrowed snapshot
state and no alternate representation.

Alternative rejected: the borrowed-input `split_lines` experiment. It changes
multiple helper call sites yet removes no measured allocation. Moving
`signature_text` out of `DocumentRow` is also rejected because it broadens
ownership changes without eliminating the all-lines temporary path.

### 2. H2, H3 and H4 remain independent follow-up tasks

H2's reader is a filtered transient vector and can be streamed into the final
enum-value owner pairs. H3 changes the temporary interner's implementation and
H4 needs allocation-growth evidence. No task combines them, because their
resource signature, behavior oracle and rollback differ.

Alternative rejected: one broad materializer rewrite. It would obscure which
allocation/lifetime caused a measured result and complicate cache/read-handle
parity review.

### 3. H5 is a provider-startup architecture decision, not a local cache flag

The existing cache validates source/schema/layout metadata inside the provider,
but the analyzer currently calls the non-cache `from_path` seam. Any startup
selection must remain provider-owned and return loaded snapshot state, never
expose cache format or discovery policy to analyzer/resolver callers. This
change records the boundary and defers code until a dedicated cross-repository
proposal accepts the lifecycle and invalidation contract.

### 4. H8 is rejected by semantic evidence

The static analyzer's context-specific type facts shall not be merged just
because the textual type happens to match. The 1C sources distinguish document
types from function parameters and returns; they contain no performance rule
that permits collapsing those domains. Retain separate snapshot groups and
explicit unknown behavior.

## Structure Impact

Searched owners/consumers: `syntax-helper-search` identity helpers, snapshot
materializer, builder, indexes, binary cache, memory accounting and tests;
downstream `analyze-project` provider composition and `context-resolver-search`
snapshot adapter; OpenSpec cache decisions and the fixed project-fast fixture.
Search terms: `split_lines`, `signature_text`, `query_owner_edges`,
`SnapshotBuilder::intern`, `string_ids`, `from_path_with_binary_cache`,
`open_read_only_with_source_id`, `HbkFactSnapshot`, `HbkTypeRef` and
`TypeDescription`.

H7 adds no semantic structure, conversion, cache key, adapter, reader or
public re-export: it removes a private temporary line vector and selected-line
clone from `signatures_by_callable`, passing a borrowed selected `&str` to the
existing builder. `HbkFactSnapshot` remains the sole fact owner. Later H2/H3/H4
tasks must update this note before adding any iterator, owner map, capacity
source or data flow. H5 requires a separate provider-owned lifecycle design.
H8 adds no code.

## Reintroduction Guards

- H7 root cause: materializing `Vec<String>` for all document signature lines
  and cloning the selected line immediately before builder interning. The only
  valid flow is `DocumentRow.signature_text.lines()` -> existing non-empty-line
  filter -> ordinal selection -> `SnapshotBuilder::intern(&str)`. A focused
  structural source guard treats this as source code, not metadata provenance:
  `signatures_by_callable` must not call `split_lines`, clone signature text or
  create an owned selected `String` before the builder.
- H8 root cause: treating equal type strings as permission to merge facts. The
  only valid owner flow keeps the four existing context-specific groups; tests
  retain distinct document-return, signature-return and parameter results.

## Risks / Trade-offs

- [Borrow lifetime reaches snapshot state] -> pass the selected `&str`
  immediately to the existing builder and verify snapshot/cache/read-handle
  behavior.
- [Small direct allocation reduction does not lower RSS] -> report direct
  allocation removal separately from normal process time/RSS; reject any
  claimed whole-process improvement not demonstrated by release runs.
- [H2/H3/H4 scope expands] -> one task/commit per candidate and a fresh
  Structure impact plus skeptic/codebase-design gate before implementation.
- [Cache optimization crosses owners] -> keep H5 deferred until a provider
  lifecycle/invalidation contract is accepted.
- [1C documentation is used as a runtime-performance claim] -> use it only as
  semantic non-merge evidence; provider-backed fixtures remain required for
  execution-context pruning.

## Results

The first H7 experiment changed `split_lines(String)` to `split_lines(&str)`.
Its post-T174 DHAT point table was allocation-identical, so it was reverted;
release inlining already removes that input clone.

The accepted direct-selection implementation removes the source-level
all-lines `Vec<String>` and selected-line `String` from
`signatures_by_callable`, while passing the selected borrowed line directly to
the existing builder. It passes focused multiline/empty-line behavior and
structural-absence tests, full package/cache/read-handle coverage, and fixed
downstream finding parity.

DHAT total allocation changed from 269,940,065 to 267,250,018 bytes, but the
`split_lines` first-frame aggregate remains exactly 5,811,810 bytes and global
peak remains effectively unchanged (69,614,838 to 69,614,844 bytes). The
profile cannot causally assign that total delta to H7, so this change makes no
memory or time improvement claim.

Normal release checks establish the P5b no-regression result: provider final
runs are 600 / 601 / 611 ms (601 ms median) and 79,756 / 79,884 / 79,752 KiB
(79,756 KiB median), with unchanged 23,144,545-byte snapshot accounting.
The five-run downstream workload retains the exact zero-finding digest with a
0.75 s / 89,424 KiB median, versus the post-T174 0.75 s / 89,108 KiB. The
0.35% RSS difference is inside the 5% guard.
