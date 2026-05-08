# ADR-0001: Keep v8-context-hbk Standalone and Integrate Through File Export First

Task: T11 integration decision for `/home/alko/develop/open-source/v8-context/`.

Date: 2026-04-29.

Status: Accepted for the current provisional HBK extraction stage.

## Decision

`v8-context-hbk` remains a standalone Rust workspace for now.

The first integration surface for `v8-context` is a file-level normalized export produced by
`v8-context-hbk syntax export --output`, not a workspace merge and not direct HBK parsing from
`v8-context` query code. Historical T9/T17-T24 evidence used the earlier `syntax-helper --output`
command name; ADR-0004 and T18 moved the current Syntax Assistant command surface under
`syntax export/index/get/search/related` without changing this file-level integration boundary.

This repository should continue to own HBK container reading, help-book navigation, Syntax
Assistant parsing, parser diagnostics and the provisional canonical JSON export. `v8-context`
should later consume a derived platform model through its HBK ingestion / platform-model-store
boundary.

## Evidence

T9 validated Syntax Assistant extraction for the target platform books:

- `cargo run -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/acceptance/t9/ru`
- `cargo run -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/acceptance/t9/en`

Both commands exited successfully. The export contained 500 global methods, 101 global properties,
2533 platform types, 6702 type methods, 10732 type properties, 445 constructors, 713 enums and
3110 enum values for each source book. The `_root` source exported as locale `en`.

T9 also recorded remaining parser gaps: 703 `UNKNOWN_PAGE_CLASS` diagnostics in each Syntax
Assistant source, mostly global context event pages and common table color palette pages. These
gaps make the current export useful for integration experiments, but not a final stable platform API
contract.

T10 validated generic HBK container/book/TOC behavior across all `*.hbk` files under
`/opt/1cv8/x86_64/8.5.1.1150/`: 116 files discovered, 116 `inspect` successes, 116
`toc --format json` successes, no fatal failures and no unsupported structures reported by the
generic smoke commands.

The T9 `v8-context-hbk` export model was intentionally provisional:

- record-family files are `metadata.json`, `global-contexts.json`, `global-methods.json`,
  `global-properties.json`, `platform-types.json`, `type-methods.json`, `type-properties.json`,
  `constructors.json`, `enums.json`, `enum-values.json` and `diagnostics.json`;
- every record-family file includes `schema_version`, `locale`, `source_locale`, `source_hbk`,
  `record_kind` and `records`;
- records preserve HBK provenance under `source`: HBK path, source locale, TOC path, HTML path and
  page title.

This concrete T9 shape is historical evidence, not the current export contract. ADR-0003 and
FR-EXPORT-001 own the T14 lean consumer export shape: consumer files omit book hierarchy,
per-record provenance and duplicate navigation scaffolding, while `diagnostics.json` keeps parser
source context for maintenance.

The current `v8-context` decision artifacts define a compatible boundary:

- HBK files are the authoritative source for documented platform API facts.
- HBK-specific availability and traceability must be normalized during ingestion.
- normal context queries must read a normalized `PlatformModelStore`, not HBK containers, HTML pages
  or category files directly;
- the platform model store is a rebuildable derived artifact;
- runtime observations, EDT models and other auxiliary sources must not silently override
  HBK-derived documented API facts.

The `v8-context` agent-facing contract also keeps HBK ingestion out of the public query surface:
the context service consumes a normalized platform API provider and an environment descriptor, while
HBK containers, ingestion rules and freshness metadata stay behind the platform-source boundary.

## Alternatives Considered

### Make `v8-context-hbk` a `v8-context` workspace member now

Rejected for the current stage.

The extraction model still has parser gaps, export schemas are provisional and `v8-context` has not
yet finalized the normalized platform model store. A workspace merge would couple two unfinished
contracts before the ingestion boundary is validated.

### Let `v8-context` call the HBK reader directly

Rejected.

This would violate the `v8-context` boundary that normal context queries use a normalized platform
model store and do not parse HBK containers, HTML pages or category files directly.

### Add legacy-shaped DTO exports first

Rejected for now.

The requirements explicitly treat legacy-shaped exports as adapters for concrete consumers, not as
constraints on the internal model. No current consumer requires the legacy shape yet.

## Consequences

- `v8-context-hbk` can continue validating HBK extraction independently.
- `v8-context` can define its normalized platform model without depending on this crate's internal
  Rust structs or provisional JSON layout.
- The next integration task should be a consumer-side importer or adapter that reads the
  `syntax export --output` directory and maps it into the `v8-context` platform model store.
- That adapter must decide how to represent current parser limitations, missing HBK availability
  normalization, source freshness metadata and localization merge rules.

## Follow-up Decisions Before Direct Integration

- Define the normalized `PlatformModelStore` item and provenance types in `v8-context`.
- Define how `v8-context` records partial or unknown HBK applicability while `v8-context-hbk` does
  not yet extract availability into the target environment descriptor model.
- Define freshness metadata for exported platform model builds: HBK input paths, platform version,
  extractor version and schema version.
- Decide whether `v8-context-hbk` stays a separate crate consumed through CLI/file artifacts, is
  published as a library dependency, or is moved into the `v8-context` workspace after the importer
  proves stable.

## Implementation Plan

- Keep this repository independently buildable and testable.
- Keep `syntax export --output` as the first integration surface.
- Keep the concrete export shape aligned with current FR-EXPORT-001 and later ADRs; this ADR
  preserves the file-level integration boundary, not the historical T9 field list.
- Keep HBK container reading, TOC/page navigation, Syntax Assistant extraction and export ownership
  inside this repository until a downstream importer proves a more direct integration boundary.
- Do not let `v8-context` query paths parse HBK containers, HTML pages or category files directly.

## Verification

- [x] T11 decision exists under `spec/decisions/`.
- [x] Decision references T9 Syntax Assistant acceptance evidence.
- [x] Decision references T10 all-HBK smoke evidence.
- [x] `README.md` and `spec/README.md` point to the numbered ADR path.

## More Information

### 2026-05-08: Static-Analysis Library Integration Is Separate

This ADR continues to own the first `/home/alko/develop/open-source/v8-context/` batch-ingestion
surface: file-level `syntax export` remains the initial normalized export path for that downstream
platform-model import experiment.

It does not prohibit the separate ADR-0008 Rust resolver boundary. For a Rust static-analysis
application that includes this workspace as Cargo dependencies, normal lookup should use
`context-resolver-core` and concrete source adapters in process, not HTTP, daemon, MCP or CLI
transport. That dependency-based static-analysis surface is governed by ADR-0008 and does not make
`v8-context` query paths parse HBK containers or Syntax Assistant HTML directly.
