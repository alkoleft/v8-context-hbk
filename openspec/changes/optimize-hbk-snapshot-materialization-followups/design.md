## Context

The downstream P5a allocation profile identified several independent costs in
the provider-owned SQLite-to-`HbkFactSnapshot` path. T174 has already removed
the dominant raw type-reference vector, reducing normal provider median RSS
from 105,592 KiB to 78,820 KiB without changing final snapshot accounting.
The remaining profile evidence is not a license to combine unrelated changes:

| ID | Candidate | Current evidence | Owner / status |
| --- | --- | --- | --- |
| H2 | Filter `query_owner_edges` by target document kind. | The analyzer index has 21,304 `owns` rows and its DHAT points fall from 6,954,078 to 1,012,703 bytes (-85.43%). The separate provider artifact has 21,613 rows; only 498 query-table fields, 56 query-table parameters and 3,087 enum values reach this reader's consumers. | `syntax-helper-search` materializer; accepted with matched provider and downstream no-regression evidence. |
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

### 2. H2 filters the existing reader; H3 and H4 remain independent

`query_owner_edges` feeds two existing consumer loops: query-table
fields/parameters and enum values. In the provider release artifact, 17,972 of
its 21,613 `owns` rows target facts that neither loop can consume. H2 keeps the private
`Vec<(String, String)>` interface and both loops, but constrains its SQL reader
through a target-id predicate derived from the three existing target
`SearchDocumentKind` values. It does not introduce
a streaming callback or move enum construction, because either would add a
new seam without reducing the caller interface.

The H2 acceptance gate is exact snapshot/read-handle parity; a
`query_owner_edges` first-frame DHAT result no greater than 3,477,039 bytes
(50% of the 6,954,078-byte baseline); and provider/downstream median time and
RSS no more than 5% above their matched counterfactual baselines. The earlier
T175 `0.75 s / 89,424 KiB` downstream observation is historical context only:
it is not the H2 comparator. A gate failure reverts the
source change and leaves H2 rejected or deferred. H3 changes the temporary
interner's implementation and H4 needs allocation-growth evidence. No task
combines them, because their resource signature, behavior oracle and rollback
differ.

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

H2 keeps `SnapshotMaterializer::query_owner_edges` as the sole reader owner
and its existing `Vec<(String, String)>` as the only intermediate shape. The
`documents` table and `SearchDocumentKind::as_str()` remain the owners of target
kind facts; the reader adds only a bound SQL predicate. No schema/index,
cache, snapshot field, serializer, adapter, mapping, public re-export or
production helper is added. The materializer-local test may reuse existing
`cfg(test)` fixture construction only to call this private reader directly; it
does not reproduce index construction or introduce a production test seam.

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
- H2 root cause: selecting every `owns` row although only
  `query_table_field`, `query_table_parameter` and `enum_value` targets reach
  the two existing consumers. The only valid flow constrains `relations` with a
  target-id predicate derived from `documents`, binds those existing kind owners
  and orders by `source_id`, `target_id`, followed by the unchanged loops. A
  private-reader
  fixture test and source guard reject an unconditional full-`owns` query.

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

H2 first evaluated a target-document `JOIN`: it reduced direct allocation but
measured a 668 ms provider median against the historic 601 ms observation. It
was rejected before the matched counterfactual protocol was corrected, so it
does not supply H2 pass/fail evidence. The accepted target-id subquery uses the
existing `relations_target_idx` without a schema or index change.

The direct-allocation and normal measurements use two deliberately distinct
artifacts. DHAT runs use the analyzer provider index with 21,304 `owns` rows;
the five `query_owner_edges` points decrease from 6,954,078 to 1,012,703
allocated bytes (-85.43%), while global peak is unchanged within two bytes.
Provider release runs use the 21,613-row snapshot artifact and improve from
659 ms / 80,280 KiB to 608 ms / 79,512 KiB (-7.74% time, -0.96% RSS), with
unchanged 23,254,254-byte snapshot accounting and 498/56/3,087
query-field/query-parameter/enum-value facts. The separately matched five-run
downstream experiment improves from 0.86 s / 92,500 KiB to 0.80 s / 91,444 KiB
(-6.98% time, -1.14% RSS), preserving the exact zero-finding digest in every
run. The explicit 5% ceilings are 691.95 ms / 84,294 KiB for provider and
0.903 s / 97,125 KiB for downstream. H2 therefore passes its 50%
direct-allocation and matched 5% normal no-regression gates; the T175 0.75 s
downstream result is not used as an H2 gate.
