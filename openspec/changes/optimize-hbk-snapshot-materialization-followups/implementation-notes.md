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
