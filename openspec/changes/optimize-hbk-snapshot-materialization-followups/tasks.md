## 1. H7 Direct Signature-Line Selection

- [x] 1.1 Record the post-T174 baseline, reject the allocation-identical
  borrowed-input experiment, and revise the task-local Structure impact and
  reintroduction guard for direct selected-line extraction; obtain fresh
  skeptic and codebase-design review before editing implementation.
- [x] 1.2 Replace only the materializer's all-lines temporary vector plus
  selected-line clone with borrowed ordinal selection passed directly to the
  existing builder, preserving order and empty-line filtering.
- [x] 1.3 Add focused signature-line and structural-absence coverage; run
  package/cache/read-handle tests, release provider measurement, fixed
  downstream parity and strict OpenSpec validation. Record direct-allocation
  removal separately from total RSS/time and reject a claimed process gain
  without a normal release comparison.

## 2. H2 Target-Kind-Filtered Owner-Edge Materialization

- [x] 2.1 Record the current owns-edge cardinality, consumer oracle, Structure
  impact and acceptance metric; obtain a fresh task-local plan review before
  implementation. The provider release artifact has 21,613 rows: 498
  query-table fields, 56 query-table parameters and 3,087 enum values are
  relevant; 17,972 rows are always skipped. The separate analyzer index has
  21,304 rows and supplies the direct DHAT evidence. The skeptic-approved hard
  gate is exact parity, at most 3,477,039 direct DHAT bytes and no more than 5%
  normal time/RSS regression over a matched counterfactual.
- [x] 2.2 Keep the existing ordered private vector and two consumer loops, but
  restrict its SQL reader to existing query-field, query-parameter and
  enum-value target kinds. Preserve source-owner skips and final CSR/index
  behavior; add no stream/callback/helper seam.
- [x] 2.3 Add private-reader target-kind/order coverage plus snapshot parity;
  compare DHAT, release provider and downstream metrics. Revert the source
  change and record H2 rejected/deferred if parity, the 50% allocation gate or
  a 5% time/RSS guard fails. The first JOIN form was rejected at 668 ms provider
  median. The accepted target-id predicate reaches 1,012,703 direct bytes
  (-85.43%), 608 ms / 79,512 KiB provider and 0.80 s / 91,444 KiB downstream
  medians against matched 659 ms / 80,280 KiB and 0.86 s / 92,500 KiB
  counterfactual baselines; all findings retain the exact zero digest. The
  historical T175 0.75 s downstream result is not an H2 acceptance comparator.

## 3. H3/H4 Evidence Before Implementation

- [ ] 3.1 Measure short-lived interner duplicate ownership after H7/H2 and
  select or reject one build-only design that preserves finished snapshot and
  cache representation.
- [ ] 3.2 Instrument or otherwise attribute temporary collection-capacity
  growth without retaining production telemetry; record whether any
  row-count-driven reservation passes its own gate.

## 4. Boundary And Semantic Dispositions

- [ ] 4.1 Record H5's provider-owned cache-startup lifecycle/invalidation
  decision and the exact cross-repository prerequisite; do not add analyzer
  cache policy or format exposure in this change.
- [ ] 4.2 Record H8's official 1C semantic non-merge evidence and retain
  distinct type-reference context tests; do not claim a runtime 1C performance
  conclusion from documentation alone.
