# Performance and Resource Variants

This document preserves the current optimization options for Syntax Assistant export. It does not
replace the T13 measurement baseline. Use the baseline to choose the first implementation slice.

Current status:

- Variant A was implemented in T14.
- Variant B was implemented in T15.
- T16 attributed the remaining post-T15 memory and selected Variant C for T17 because the
  extraction/export command path accumulated the full `PlatformContext` before writing record-family
  JSON.
- Variant C was implemented in T17. It reduced the `shcntx_ru.hbk` debug export peak to
  `386304 KiB` while preserving deterministic canonical JSON output and the in-memory lookup model.
- The first Variant E slice was implemented in T19. Byte-only entity reads removed the per-byte
  `source_offsets` allocation from ordinary `FileStorage` and entity-body reads while keeping
  descriptor offsets available for diagnostics.
- T20 measured the remaining owned `FileStorage` copy after T19 and did not justify a direct
  seekable `FileStorage` view on the pre-T22 baseline: the exact retained `Vec<u8>` capacity was
  material but not dominant against retained `HbkBook::open` RSS or the full
  `syntax-helper --output` peak.
- T21 measured retained TOC/root-discovery structures and did not justify a production refactor:
  the largest T21-specific retained structure was public `RootDiscovery` at about 9 MiB.
- T22 measured lower-level book state retained during streaming export and removed the avoidable
  `HbkContainer` mmap retention from `HbkBook`. The public book API and export shape stayed stable.
  This makes the T20 FileStorage percentage stale for current `HbkBook::open` attribution, although
  the T20 result remains pre-T22 evidence for the broader full-export peak.
- T23 remeasured retained `FileStorage` ownership on the post-T22 baseline. A user-directed
  production follow-up then removed retained `FileStorage` bytes from `HbkBook` and moved ownership
  to short-lived `FileStorageReader` values. The direct/seekable block-backed storage design remains
  unimplemented because repeated page reads and full `syntax-helper --output` did not show a
  material peak-RSS benefit from broader storage work.

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

The original T13 ordering was:

1. Variant A, because it fixes known consumer-output bloat and has a narrow export boundary.
2. Variant B, because current extraction reads page sets into memory before parsing.
3. Variant D only if wall-clock time remains dominated by parsing CPU.
4. Variant C only if full-model accumulation remains a measured problem.
5. Variant E only if container/FileStorage ownership is proven to be the limiting resource.

T16 memory attribution selected Variant C from measured evidence:

- the actual CLI peak on `shcntx_ru.hbk` was `588892 KiB`;
- the `extract` probe reached the same peak class as the full `export` path;
- export adapter allocation did not materially raise high-water RSS after extraction;
- container/FileStorage opening still has a measured temporary spike, but Variant E alone would not
  reduce the current `shcntx_ru.hbk` extraction peak.

Variant E remains a later candidate only if new measurements show retained `FileStorage` ownership
or ZIP access costs dominate after the T19 byte-only path. T19 did not justify a broader seekable
direct `FileStorage` view.

T20 rechecked that later candidate after explicit memory reprioritization. On the pre-T22 baseline,
the owned `FileStorage` vector remained about one third of retained `HbkBook::open` RSS and less
than one quarter of the full Syntax Assistant export peak for both measured books, so a broader
direct seekable view was not justified by that evidence.

T21 rechecked the next memory-structure candidate after T20. The public `RootDiscovery` graph was
about 9 MiB, the private `syntax_toc_index` shape about 5 MiB, retained flat-page metadata about
2 MiB, and the required public `Toc` tree about 8 MiB. These structures are bounded and do not
justify a production refactor without new evidence that TOC/root-discovery retention dominates the
remaining export peak.

T22 rechecked the remaining lower-level book state retained by the streaming export path. The
container mmap inside `HbkBook` was avoidable after metadata, TOC and `FileStorage` bytes were
extracted, so `HbkBook` now stores the source path directly and releases `HbkContainer` during open.
That change makes the T20 `HbkBook::open` percentage stale: after T22, the same retained
`FileStorage` vector is about half of current open-path RSS. Further lifetime splitting for TOC/root
discovery or parser traversal is not justified by the current measurements, while a direct or
shorter-lived `FileStorage` design needs a post-T22 measurement pass before it is accepted or
rejected again.

T23 performed that post-T22 measurement pass. The exact `FileStorage` entity size stayed about
`38048 KiB` for `shcntx_ru.hbk` and about `31856 KiB` for `shcntx_root.hbk`. The user-directed
production follow-up removed those bytes from retained `HbkBook` state: current RSS after
`book-open` dropped to `33164 KiB` and `32928 KiB` for the measured books. Open-path high-water RSS
stayed in the previous class because `HbkBook::open` still validates the `FileStorage` entity body.
Repeated page reads through one `FileStorageReader` stayed in the `book-open` high-water class, and
the full export path kept stable record counts and output sizes without a material peak-RSS win.
Variant E therefore consists of the T19 byte-only entity path plus the T23 path-backed reader
lifetime; no direct/seekable block-backed `FileStorage` refactor is justified by current evidence.
