# Performance and Resource Variants

This document preserves the current optimization options for Syntax Assistant export. It does not
replace the T13 measurement baseline. Use the baseline to choose the first implementation slice.

## Selection Rules

- Measure before refactoring: wall-clock time, peak RSS or equivalent, exit status, output counts
  and output sizes.
- Prefer the smallest change that removes a measured bottleneck.
- Preserve deterministic diagnostics and deterministic JSON record order.
- Keep consumer export focused on platform API facts, not HBK book hierarchy or parser traces.
- Do not introduce caches, broad pipeline frameworks, plugin systems, compatibility adapters or
  tuning knobs without a measured bottleneck and a concrete consumer requirement.

## Variant A: Lean Consumer Export and Streaming JSON Writer

Goal: reduce output size and export-time memory/CPU overhead while aligning the output contract with
the actual consumer need.

Changes:

- Introduce explicit export DTOs in `hbk-export` instead of serializing the internal
  provenance-rich model directly.
- Remove per-record `source` data from consumer record-family files.
- Remove navigation scaffolding that duplicates dedicated files:
  `global-contexts.json`, `method_links`, `constructor_links` and `value_links`.
- Keep parser source context in `diagnostics.json`.
- Write compact JSON through a writer, preferably `BufWriter` plus `serde_json::to_writer`, instead
  of materializing pretty-printed JSON bytes first.

Use first when:

- consumer-facing output contains book/TOC/provenance fields that downstream tools do not need;
- output size or JSON serialization shows up in the T13 baseline;
- the team wants to correct the provisional export contract before deeper extraction refactors.

Risks:

- This intentionally changes the provisional JSON shape.
- Acceptance tests and README examples must be updated in the same task.

## Variant B: Lazy or Batched Page Loading

Goal: avoid holding all extraction HTML pages in memory at once.

Changes:

- Replace extraction-wide `read_pages(...)->BTreeMap<String, String>` usage with a bounded page
  loader that reads only the current page or a small deterministic batch.
- Keep page traversal order driven by TOC/root discovery.
- Keep missing-page and invalid-UTF-8 diagnostics at the book/page boundary.

Use first when:

- T13 shows peak RSS dominated by accumulated page HTML;
- page parsing is not clearly CPU-bound;
- export shape has already been corrected or is not the active bottleneck.

Risks:

- Reopening ZIP state per page may trade memory for CPU/IO.
- A small batch abstraction may be needed, but it must stay local to `hbk-book` or
  `syntax-helper-extract`.

## Variant C: Streaming Extraction Into Record-Family Sinks

Goal: avoid accumulating the full `PlatformContext` before export.

Changes:

- Let extraction emit typed records in stable traversal order.
- Let `hbk-export` consume record-family streams/sinks while preserving the existing crate
  boundaries.
- Keep lookup helpers on the in-memory model as a separate library use case.

Use first when:

- T13 shows domain model accumulation is a material memory bottleneck after page loading is bounded;
- the primary command path is export, not in-process lookup.

Risks:

- This is a wider boundary change than Variant A or B.
- It must not collapse extractor, model and export crates into one broad pipeline.

## Variant D: Bounded Parallel Page Parsing

Goal: reduce wall-clock time when HTML parsing is CPU-bound.

Changes:

- Parse independent Syntax Assistant pages with bounded worker count.
- Reorder parsed records and diagnostics back into deterministic TOC traversal order before export.
- Keep parser errors typed with source context.

Use first when:

- T13 shows CPU-bound parsing and acceptable memory behavior;
- single-threaded streaming/batching is too slow for the target workstation.

Risks:

- Parallelism can make diagnostics ordering unstable if the collection boundary is not explicit.
- Worker fan-out must be fixed or bounded by a measured, documented rule.

## Variant E: Container and FileStorage Access Model Review

Goal: reduce avoidable whole-entity copies while keeping the reader simple.

Changes to evaluate:

- Keep memory-mapped container access if it remains the simplest low-copy option.
- Avoid copying the entire `FileStorage` entity if a direct block-backed or seekable reader is
  simpler after measurement.
- Reuse ZIP metadata only if repeated archive initialization is measured as a bottleneck.

Use first when:

- T13 shows memory dominated by `FileStorage` ownership or repeated ZIP setup;
- Variant B alone cannot keep resource use within target workstation limits.

Risks:

- This can spread low-level IO concerns into book/extraction code if boundaries are not preserved.
- Direct `Read + Seek` designs must still provide clear typed errors and provenance.

## Initial Ordering

Unless T13 contradicts it, use this order:

1. Variant A, because it fixes known consumer-output bloat and has a narrow export boundary.
2. Variant B, because current extraction reads page sets into memory before parsing.
3. Variant D only if wall-clock time remains dominated by parsing CPU.
4. Variant C only if full-model accumulation remains a measured problem.
5. Variant E only if container/FileStorage ownership is proven to be the limiting resource.
