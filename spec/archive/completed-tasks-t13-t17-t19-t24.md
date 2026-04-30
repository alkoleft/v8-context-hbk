# Completed Implementation Tasks T13-T17 and T19-T24

This archive preserves completed task history moved out of the active implementation ledger.
It is evidence, not active implementation scope.

T18 remains active and is not archived here.

Raw command logs, generated exports, temporary probes and `target/...` paths are service data and
are intentionally omitted from this archive. Durable measurements and conclusions live in
`../acceptance/baseline.md`, `../implementation/performance-baseline-t13.md` and
`../implementation/performance-variants.md`.

For active task sequencing, use `../IMPLEMENTATION_TODO.md`.

## T13. Performance/resource baseline and implementation hypotheses

Status: completed.

Outcome:

- Established the first measured performance/resource baseline for the current implementation.
- Recorded exact fixture availability, representative commands and hotspot hypotheses in the
  performance baseline.
- Selected the lean consumer export as the first optimization slice.

## T14. Lean Syntax Assistant consumer export

Status: completed.

Outcome:

- Implemented the lean consumer export shape for Syntax Assistant record-family JSON files.
- Removed `global-contexts.json`, per-record source provenance and duplicate navigation-link fields
  from consumer records while preserving diagnostics provenance.
- Confirmed that output size decreased but peak RSS stayed high enough to justify the lazy page
  loading slice.

## T15. Lazy or batched Syntax Assistant page loading

Status: completed.

Outcome:

- Introduced on-demand page loading through a reusable file-storage reader instead of preloading all
  extraction pages into a `BTreeMap`.
- Reduced the debug `syntax-helper --output` peak from the T14 class while preserving accepted
  export counts and shape.
- Left enough Russian Syntax Assistant memory pressure to justify explicit post-T15 attribution.

## T16. Attribute post-T15 Syntax Assistant memory

Status: completed.

Outcome:

- Attributed remaining post-T15 memory and confirmed that full model accumulation was the dominant
  export-command cost.
- Confirmed that export adapter allocation did not materially add to the high-water RSS after
  extraction.
- Selected Variant C, streaming extraction into export sinks, for T17.

## T17. Implement selected post-T16 memory optimization

Status: completed.

Outcome:

- Implemented shared `SyntaxHelperSink` extraction so the CLI export path streams record-family
  events directly to JSON writers.
- Preserved the in-memory `PlatformContext` lookup use case and the accepted FR-EXPORT-001 consumer
  export shape.
- Reduced the debug `shcntx_ru.hbk` export peak from `590988 KiB` after T15 to `386304 KiB` while
  preserving deterministic JSON output and record counts.

## T19. Reduce HBK open-time FileStorage memory spike

Status: completed.

Outcome:

- Implemented the narrow byte-only container entity read path for ordinary entity-body reads.
- Kept offset-aware block reads for descriptor diagnostics and validation paths.
- Reduced the measured `HbkBook::open` high-water mark and full `syntax-helper --output` peak enough
  that no immediate direct seekable `FileStorage` follow-up was added from T19.

## T20-prep. Split large modules before memory optimization

Status: completed.

Outcome:

- Split oversized crate roots into focused modules for `hbk-container`, `syntax-helper-extract` and
  `hbk-export`.
- Preserved public re-exports, CLI behavior, JSON export shape, diagnostics and deterministic output.
- Made no performance or contract change.

## T20. Evaluate direct seekable FileStorage view

Status: completed.

Outcome:

- Measured the remaining owned `FileStorage` vector on the post-T19/pre-T22 baseline.
- Concluded that the vector was material but not dominant for the full export peak, so a broader
  direct seekable `FileStorage` view was not justified.
- Promoted the no-go conclusion to the performance and acceptance baselines.

## T21. Reduce TOC and root-discovery retained memory

Status: completed.

Outcome:

- Measured retained TOC, flattened traversal metadata and public root-discovery structures for both
  Syntax Assistant books.
- Concluded that the measured structures were bounded and did not justify a production refactor.
- Promoted the no-go conclusion to the performance and acceptance baselines.

## T22. Release lower-level book state earlier in Syntax Helper export

Status: completed.

Outcome:

- Released the avoidable `HbkContainer` mmap retained by `HbkBook` after book metadata, TOC and
  `FileStorage` bytes are extracted.
- Preserved public `HbkBook` behavior, page reads, diagnostics provenance, deterministic order and
  the accepted export shape.
- Changed the open-path attribution baseline: the T20 `FileStorage` no-go remained useful
  pre-T22 evidence for the broader export peak, but no longer described current `HbkBook::open`
  memory ownership.

## T23. Re-evaluate FileStorage lifetime after T22 baseline shift

Status: completed.

Outcome:

- Re-measured retained `FileStorage` cost on the post-T22 baseline and then implemented the
  user-directed production follow-up.
- Removed retained `FileStorage` bytes from `HbkBook`; the book now remains path-backed for later
  page access while still validating the `FileStorage` entity body during open.
- Reused readers in documentation and Syntax Assistant flows to avoid repeated full file-storage
  loads.
- Kept the broader direct or seekable block-backed `FileStorage` design unimplemented because the
  accepted measurements did not justify it.

## T24. Apply targeted parser, lookup and lean export optimizations

Status: completed.

Outcome:

- Applied targeted allocation/time reductions in ZIP entry buffer sizing, signature matching,
  type-reference handling, Syntax Assistant TOC lookup and lean streaming export.
- Preserved byte-identical consumer JSON output compared with the local T23 production baseline.
- Accepted only measurement-stable micro-optimizations and rejected changes that did not preserve the
  resource profile.
- Promoted current T24 performance, release-profile and export-count conclusions to the acceptance
  and performance baselines.
