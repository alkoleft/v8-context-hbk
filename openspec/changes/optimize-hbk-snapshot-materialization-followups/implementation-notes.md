## T175 First Experiment And Revised Task-Local Plan

Post-T174 downstream DHAT keeps the `split_lines` first-frame total at
5,811,810 allocated bytes. The first experiment changed its input from
`String` to `&str` and passed borrowed document text, but the full DHAT point
table was byte-for-byte allocation-identical. Release inlining already removes
the input clone, so the experiment was reverted and is rejected.

The measured source path is instead
`split_lines(document.signature_text.clone()).get(ordinal).cloned()`: it owns
every line, owns the selected line again, then asks `SnapshotBuilder` to own it
once more. The refined plan is:

1. Keep `HbkFactSnapshot` and `HbkFactReadHandle` as the sole external
   interface. In `signatures_by_callable`, select the existing non-empty
   ordinal `&str` directly from `DocumentRow.signature_text` and pass it to
   `SnapshotBuilder::intern`.
2. Do not change `split_lines` or any of its other call sites; do not move the
   field out of `DocumentRow`, borrow data into the snapshot, add a helper,
   cache, adapter, DTO, schema/index or public re-export.
3. Add a focused snapshot test with multiple signature lines and empty
   separators, proving the materialized callable keeps the expected non-empty
   strings in order. The structural source guard is not metadata-provenance
   behavior: it requires direct `lines`/filter/ordinal selection and builder
   interning, while rejecting `split_lines`, cloned signature text and an owned
   selected line in `signatures_by_callable`.
4. Preserve existing snapshot/read-handle/binary-cache behavior. Run format,
   package tests, strict OpenSpec validation, release provider measurements and
   five-run downstream finding parity. Reprofile the exact process to show a
   material drop in the materializer `split_lines` points; report normal
   time/RSS only as measured and reject regressions above 5%.

## Structure Impact

Searched `split_lines`, `signature_text`, `DocumentRow`, all helper callers,
snapshot/read-handle/cache/memory consumers, downstream snapshot loading,
tests and the fixed workload. `HbkFactSnapshot` owns final strings and facts;
`split_lines` remains a private reusable parsing behavior. This task changes no
semantic structure, conversion, mapping, schema, cache key, reader, loader,
adapter, public re-export or output contract. It removes only materializer-local
temporary lines and passes the selected borrowed source to the existing owner.

## Reintroduction Guard

Root cause: all-lines temporary materialization plus selected-line cloning
before existing builder interning. The single valid flow is
`signature_text.lines()` -> existing non-empty filter -> ordinal selection ->
builder interning. The guard must fail if `signatures_by_callable` uses
`split_lines`, clones the document text or constructs an owned selected line.

## Codebase-Design Review

`HbkFactSnapshot` remains the deep module: callers continue to see the same
read-handle interface while line-source ownership is localized to its private
materializer. Direct selection removes temporary work at the existing
implementation seam; adding a wrapper or an alternate line model would reduce
locality without another consumer.

## Skeptic Review

The first skeptic-approved plan was invalidated by exact DHAT: the borrowed
input change did not remove an allocation point and was reverted. The refined
all-lines-materialization plan passed a fresh skeptic review. Completion still
requires focused signature and structural tests, package/cache/read-handle
coverage, strict OpenSpec validation, exact DHAT comparison and normal
provider/downstream no-regression evidence.

## T175 Verification Results

- The direct-selection implementation preserves non-empty multi-line signature
  order, and its structural guard rejects materializer use of `split_lines`, a
  document-text clone and owned selected-line construction.
- `cargo test -p syntax-helper-search` passes with 61 tests; focused
  read-handle and binary-cache coverage, `cargo fmt --all --check` and strict
  OpenSpec validation pass.
- The input-borrow experiment was allocation-identical and was reverted. The
  final direct-selection DHAT run reduces total bytes by 2,690,047 but leaves
  the `split_lines` first-frame total at 5,811,810 bytes and process global
  peak unchanged. It is therefore P5b supportability evidence, not a causal
  memory/time result.
- Provider final median is 601 ms / 79,756 KiB with unchanged
  23,144,545-byte snapshot accounting. Downstream five-run median is 0.75 s /
  89,424 KiB with the unchanged
  `c2b4465a4c66a8939d40febd117061959e85dcde77078be386c1e73ae97f60a3`
  zero-finding digest; both remain within the 5% no-regression guard.

## T176 Approved H2 Task-Local Plan

The provider release artifact
`target/snapshot-materialization/shcntx_ru.schema16.release.sqlite` has 21,613
`owns` relations. Only 498 `query_table_field`, 56 `query_table_parameter` and
3,087 `enum_value` targets reach `query_owner_edges` consumers; 17,972 rows are
discarded after owned `String` materialization. The analyzer provider index used
for direct DHAT has a distinct 21,304-row inventory. The prior H2 name,
"streaming", was inaccurate: the approved minimum change is target-kind-filtered
materialization, not an iterator/callback seam.

1. Keep `SnapshotMaterializer::query_owner_edges` and its
   `Vec<(String, String)>` output. Restrict its existing `relations` query by
   a target-id predicate derived from the target `documents` rows, bind the
   three existing `SearchDocumentKind` storage values, and retain qualified
   `source_id`, `target_id` ordering.
2. Keep both consuming loops, their source-owner lookup and `continue`
   behavior unchanged. Do not add a reader/helper, stream, DTO, schema/index,
   cache change, source-kind filter or public re-export.
3. Add a materializer-local test that calls the private reader on a
   production-built index containing accepted and rejected target kinds. It
   must prove that only the three accepted kinds remain and that rows preserve
   source/target order; pair it with snapshot/read-handle parity and a narrow
   source guard. Existing `cfg(test)` fixture construction may be reused
   without recreating the schema or production pipeline.
4. H2 passes only with exact behavior parity, a direct DHAT first-frame result
   at or below 3,477,039 bytes (50% of 6,954,078 baseline), and provider plus
   downstream median time/RSS no more than 5% above their matched
   counterfactual baselines. The historic T175 601 ms / 79,756 KiB and 0.75 s /
   89,424 KiB observations do not supply the H2 comparator. Any gate miss
   reverts the source change and records H2 rejected/deferred rather than
   complete.

Structure impact: `SearchDocumentKind` remains the sole storage-kind owner;
the materializer's existing reader and vector remain the sole data path. The
test-only access is local fixture reuse, not a production interface. The
skeptic re-reviewed this plan after the H2 name, gate and direct-reader test
were corrected and approved it. The codebase-design review preserves the
single private reader interface, so filtering remains local to the existing
deep materializer module.

The first JOIN form passed the 50% DHAT gate but failed the normal provider
time gate at 668 ms median versus 601 ms (+11.1%). It is rejected before
completion. `EXPLAIN QUERY PLAN` shows an equivalent target-id subquery can
use the existing `relations_target_idx` without an index or schema change. A
fresh skeptic review approved testing only that SQL-form correction; it keeps
the parity, allocation and 5% threshold unchanged while correcting the
counterfactual evidence comparator.

## T176 Verification Results

- Measurement identity and comparability: the provider baseline is commit
  `7437059126d4b041351679aa16ebb4675ffced2e`; the H2 production executable
  differs only in the `query_owner_edges` SQL selection (test-only coverage is
  not linked into it). Both use release binary
  `target/release/examples/measure_hbk_fact_snapshot`, the same
  `target/snapshot-materialization/shcntx_ru.schema16.release.sqlite` artifact
  (`sha256:cc9b2b8aaf31f64c880b92cc3a02fd3166541f10f8d209faf8c7a7c22cac0d55`),
  the same `cargo build --release -p syntax-helper-search --example
  measure_hbk_fact_snapshot` build protocol and the same `/usr/bin/time`
  invocation. The downstream baseline temporarily reverts only that SQL
  selection in the same source checkout, rebuilds
  `target/release/v8-analyzer` with `cargo build --release -p
  v8-analyzer-cli --features heap-profile`, then runs
  `diagnostics --project openspec/changes/add-context-provider-boundary/fixtures/context-benchmark/v1/fixture-project --profile project-fast --platform-version 8.3.27.1859 --platform-install-root /opt/1cv8/x86_64 --findings <path>`.
  It uses the same analyzer index
  `.v8-context/platform-indexes/8.3.27.1859/shcntx_ru.sqlite`
  (`sha256:317f3cdd914e635c89b975bf9ebcf28238bdbabd54e455121a083558d4e05f5e`)
  and the same binary mode. These downstream A/B samples are sequential, not
  interleaved; they are accepted only because source index, command, profile,
  artifact, build mode and exact digest are controlled in one adjacent window.
- The private-reader fixture proves that `type_property` and `constructor`
  `owns` edges exist but are excluded before `(target_id, source_id)`
  materialization; query-table field, query-table parameter and enum-value
  rows retain their ordered result. The package has 63 passing tests, including
  existing read-handle and binary-cache coverage.
- Final DHAT on the unchanged downstream 21,304-row provider index reduces the
  five `query_owner_edges` points from 6,954,078 to 1,012,703 bytes (-85.43%).
  Total allocation is 261,358,832 bytes and global peak is 69,614,846 bytes,
  effectively unchanged from the post-T175 point. The zero-finding digest is
  `c2b4465a4c66a8939d40febd117061959e85dcde77078be386c1e73ae97f60a3`.
- The original JOIN query is explicitly rejected: its 668 ms provider median
  was compared to the earlier 601 ms historical observation before the matched
  baseline was established. It is retained as a rejected design trial, not as
  H2 pass/fail evidence. The accepted target-id subquery uses the existing
  `relations_target_idx`; matched provider runs are 589 / 687 / 608
  ms and 79,512 / 79,516 / 79,512 KiB, versus 659 / 634 / 751 ms and 80,280 /
  80,456 / 80,204 KiB on the same 21,613-row artifact. The matched 5% ceilings
  are 691.95 ms and 84,294 KiB, so its 608 ms / 79,512 KiB median passes.
- Matched downstream baseline runs are 0.79 / 0.86 / 0.93 / 0.88 / 0.80 s and
  92,500 / 92,372 / 92,572 / 92,540 / 92,372 KiB; accepted H2 runs are
  0.79 / 0.84 / 0.80 / 0.79 / 0.87 s and 91,700 / 91,444 / 91,316 / 91,444 /
  91,572 KiB. Median comparison is 0.86 s / 92,500 KiB to 0.80 s / 91,444 KiB;
  the 5% ceilings are 0.903 s and 97,125 KiB. Every baseline and H2 run has
  the exact `c2b4465a4c66a8939d40febd117061959e85dcde77078be386c1e73ae97f60a3`
  digest. The earlier T175 0.75 s / 89,424 KiB downstream result is historical
  only and would not be a valid H2 comparator.
- Two later final-binary downstream attempts (`6.44 s / 91,456 KiB` and
  `6.20 s / 91,580 KiB`) overlapped unrelated Rust builds consuming CPU. They
  are invalid, are excluded from pass/fail and are retained only to document
  the disqualifying host load.
- No architecture document changes: crate responsibilities, dependency
  direction, provider boundary, cache/schema and public fact contracts are
  unchanged. Workspace version is bumped from 0.1.0 to 0.1.1 for completed
  internal work.
- Final codebase-design review confirms that the materializer remains the deep
  module: callers retain the same snapshot interface, the private reader retains
  its same vector interface, and the predicate hides the selection detail inside
  that implementation. The test-only helper visibility creates no production
  seam, adapter or parallel data flow.
