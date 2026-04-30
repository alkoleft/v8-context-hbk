# Non-Functional Requirements

## NFR-REL-001: Reliability

- User-controlled HBK/HTML input must not trigger parser `unwrap()` or panic.
- Errors must include path, entity or page context where relevant.
- Unsupported structures must fail explicitly or produce visible recoverable diagnostics.

## NFR-PERF-001: Resource Use and Throughput

Minimum resource consumption and high throughput are first-class requirements for follow-up work.
Large Syntax Assistant books must remain usable on a developer workstation without avoidable
whole-book copies, unbounded page buffering or unbounded worker fan-out.

Requirements:

- Container opening should not eagerly decompress all pages.
- Page content should be read lazily from `FileStorage` where practical.
- Before architecture refactors, measure wall-clock time, peak RSS or equivalent, command exit
  status and output counts for `shcntx_ru.hbk`, `shcntx_root.hbk` and the all-HBK smoke path.
- Prefer bounded streaming before broader concurrency.
- Parallel work must be bounded and deterministic.
- If page parsing or export is parallelized, preserve stable diagnostics, stable JSON output order
  and typed error context.
- Do not add caches, generic pipeline frameworks, plugin systems or tuning knobs until measurements
  show a concrete bottleneck and a concrete consumer needs the behavior.

Implementation hypotheses to evaluate after baseline measurement:

1. Narrower streaming or batched reads across `FileStorage` and page loading.
2. Keep memory-mapped container access only if it remains the simplest low-copy strategy.
3. Bounded parallel Syntax Assistant page parsing with deterministic diagnostics and output order.
4. Streaming record-family JSON export if serialization is a measured bottleneck.

Saved variants and selection rules live in
[`spec/implementation/performance-variants.md`](../implementation/performance-variants.md). Treat
that document as a candidate plan, not as approval to skip the baseline measurement.

## NFR-QUERY-001: Search Query Latency

Fast Syntax Assistant lookup is a separate requirement from HBK extraction throughput.

Query commands must use a prebuilt local export or search index and must not parse `shcntx_*.hbk`
inside the query path. Index build commands may be slower and may reuse extraction/export pipelines,
but interactive query commands must be optimized for repeated use.

Provisional targets for the first indexed CLI slice on the target developer workstation:

- exact name or owner/member lookup returns in under 1 second for the `shcntx_ru.hbk` data set;
- keyword/fuzzy/relationship search returns in under 2 seconds for the `shcntx_ru.hbk` data set;
- JSON output order is deterministic across runs;
- if the command cannot meet these targets, the implementation task must record exact measurements
  and identify the limiting index or ranking component before adding broader optimization.

The first semantic-search experiment must preserve the local deterministic search path. Embeddings
or model-backed ranking may rerank or supplement results, but exact lookup and relationship graph
queries must continue to work without network access or an embedding provider.

The first query index storage must remain local and rebuildable. A SQLite/FTS5 index is the current
preferred implementation direction because it supports exact lookup, full-text search and bounded
relationship traversal without running a service. External search engines or graph databases require
a measured limitation in the SQLite-backed slice and a separate ADR update.

## NFR-TEST-001: Testability

- Test behavior, not implementation details.
- Unit fixtures cover deterministic binary/parser behavior.
- Small real-HBK smoke tests use `fmtdui_root.hbk` and `fmtdui_ru.hbk`.
- Syntax Assistant integration tests use `shcntx_ru.hbk` and `shcntx_root.hbk`.
- Syntax Assistant fixture corpus must come from real 8.5 pages and include a manifest with source
  HBK file, HTML path, page title, parser kind and inclusion reason.
- Broad all-HBK smoke is an acceptance/reporting stage, not a prerequisite for early parser work.

Verification tiers:

1. Unit fixtures committed to the repository.
2. Small real-HBK smoke.
3. Syntax Assistant smoke.
4. All-HBK smoke.
5. UAT cases from `spec/acceptance/uat-test-cases.md`.

## NFR-COMPAT-001: Compatibility Policy

- First supported platform baseline is `8.5.1.1150`.
- Parser logic should avoid assumptions that are only true for one HTML filename when TOC carries a
  more reliable relationship.
- Root section detection should be data-driven and tested against Russian/root books.
- Do not preserve backward compatibility for its own sake.
- Contract stability is intentionally deferred until parser evidence, consumer feedback and model
  boundaries justify it.

## NFR-DIAG-001: Diagnostics

Fatal errors stop the current command/test:

- missing file
- invalid container structure
- missing required HBK entities
- unreadable ZIP storage
- malformed book metadata
- TOC corruption

Path-backed help-book access may surface `FileStorage` ZIP read errors at the page/file access
boundary after the initial book metadata/TOC open has succeeded.

Recoverable extraction diagnostics must not abort a full Syntax Assistant pass when partial
extraction remains meaningful:

- unknown page class
- unsupported HTML block
- unresolved link
- missing optional section
- parser field that cannot be mapped safely
- data-contract gaps such as multiple return types per overload when unsupported

Every recoverable diagnostic must include:

- severity
- stable code
- source HBK path
- locale/source locale
- TOC path when known
- HTML path when known
- page title when known
- parser stage

CLI commands return non-zero for fatal errors. Reporting commands that scan many files may continue
after per-file failures, but the final summary must make failures visible and return non-zero when
the requested acceptance contract is not met.

## NFR-LIC-001: Licensing and Attribution

- `hbk-reader` is MIT-licensed and can be used as a reference.
- Ported logic should preserve attribution where appropriate.
- Generated platform documentation must not be copied into the repository except minimal fixtures
  required for parser behavior tests.
