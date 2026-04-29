# ADR-0003: Start Performance Work with Lean Consumer Export

Date: 2026-04-30.

Status: Accepted for the first post-baseline performance slice.

Post-decision status: T14 completed Variant A. T15 completed Variant B after post-T14 measurements
showed peak RSS was still high. The current active follow-up is T16 memory attribution before
choosing Variant C, Variant E or no immediate refactor.

## Context

T13 measured the current implementation before any streaming or parallel extraction refactor.

The largest measured path is `syntax-helper --output` on the target Syntax Assistant books:

- `shcntx_ru.hbk`: 20.80 seconds, 752392 KiB peak RSS, about 40 MB of exported JSON.
- `shcntx_root.hbk`: 16.15 seconds, 518972 KiB peak RSS, about 26 MB of exported JSON.

The aggregate all-HBK generic smoke over 116 `*.hbk` files completed successfully in 14.69 seconds
with 386304 KiB peak RSS.

Code review found multiple plausible hotspots:

- whole `FileStorage` entity ownership in `hbk-book`;
- full-page `BTreeMap` accumulation before Syntax Assistant parsing;
- full `PlatformContext` accumulation before export;
- pretty JSON materialization in `hbk-export`;
- consumer record-family files still include internal provenance and navigation scaffolding.

The measurement therefore points at both export bloat and page-loading memory pressure.

## Decision

Start with Variant A from `spec/implementation/performance-variants.md`: lean consumer export and a
streaming compact JSON writer.

Do this before lazy page loading, parallel parsing, streaming extraction sinks or replacing the
container/FileStorage access model.

Existing T14 is the implementation task for this decision.

## Rationale

Variant A is the narrowest first slice that addresses a measured problem and an already documented
consumer-contract problem:

- output directories are large enough to justify shrinking consumer JSON;
- current export materializes pretty JSON bytes before writing;
- FR-EXPORT-001 already says consumer files must omit book hierarchy, per-record source provenance
  and duplicate navigation-link catalogs;
- the change is isolated mostly to `hbk-export` and CLI/README expectations, without changing
  extraction traversal or parser behavior.

Variant B remains likely after Variant A because peak RSS is still plausibly dominated by page
loading and accumulation. It should be selected only after Variant A is measured, so the project does
not mix export-shape effects with page-loading effects.

Variant D is deferred because this baseline does not prove CPU-bound parsing, and parallel parsing
would add deterministic ordering and bounded-worker risks.

Variant C is deferred because it crosses extractor, model and export boundaries.

Variant E is deferred because `memmap2` is still the simplest low-copy container-open strategy, and
the current evidence is not enough to replace the lower-level access model first.

## Consequences

- At decision time, T14 became the next active task.
- T14 must keep diagnostics provenance available while removing provenance from consumer record
  files.
- T14 must write compact JSON through a writer instead of materializing pretty JSON bytes.
- After T14, rerun the T13 `syntax-helper` measurements for `shcntx_ru.hbk` and `shcntx_root.hbk`.
- If peak RSS remains high after T14, promote Variant B as the next implementation task.

## Verification

- T13 baseline exists under `spec/implementation/performance-baseline-t13.md`.
- T13 baseline records exact commands, fixture availability, exit status, wall-clock time, peak RSS,
  output counts and output sizes.
- T14 points to Variant A.
