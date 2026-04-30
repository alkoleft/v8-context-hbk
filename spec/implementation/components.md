# Implementation Component Specification

Current status: the repository is a Cargo workspace split into the crates below. The split preserves
context boundaries and keeps CLI/export behavior provisional.

## Workspace Crates

1. `hbk-container`: binary container parsing, entity enumeration and entity byte access.
2. `hbk-book`: book metadata, locale inference, ZIP-backed `FileStorage`, TOC parsing and page reads.
3. `hbk-docs`: documentation HTML/page parsing, normalized text/link extraction and page diagnostics.
4. `syntax-helper-model`: provenance-rich platform context domain model and lookup helpers.
5. `syntax-helper-extract`: Syntax Assistant root discovery, catalog traversal and specialized page parsers.
6. `hbk-export`: canonical JSON export adapters.
7. `v8-context-hbk-cli`: command wiring for the `v8-context-hbk` binary.

Planned search/query components are described in
[`syntax-helper-query-cli.md`](syntax-helper-query-cli.md). They are not current workspace members
until an implementation task adds them.

## Dependency Rules

- `hbk-container` must not depend on book, docs, extraction or export concerns.
- `hbk-book` must not depend on Syntax Assistant extraction.
- `hbk-docs` may depend on book-level page/TOC abstractions but must not know export schema details.
- `syntax-helper-model` must not depend on HBK container, HTML parsing or CLI code.
- `syntax-helper-extract` owns traversal and parser behavior for Syntax Assistant pages.
- `hbk-export` owns output adapters for the Rust domain model.
- `v8-context-hbk-cli` wires commands and error presentation only.
- Planned Syntax Assistant search/query code must not make `hbk-export` carry search-only fields in
  the lean consumer export. Use a search-specific index or maintenance export when structured links
  or provenance are required for query workflows.

## Public Contract Policy

- Public contracts are provisional unless an ADR or requirement explicitly stabilizes them.
- Legacy-shaped DTOs or exports are adapters for concrete consumers, not constraints on the internal
  model.
- Runtime 1C introspection is out of scope for this repository.
- Validation belongs at file/container input, external command input, parsing boundaries,
  serialization/export boundaries and public API boundaries.

## Component Requirements

### hbk-container

Expected public concepts:

- `HbkContainer`
- `ContainerHeader`
- `BlockHeader`
- `EntityDescriptor`
- `EntityName`
- `ContainerError`

Owns FR-HBK-001.

Implementation note: ordinary entity byte reads use a byte-only block path. Offset-aware block reads
remain internal to descriptor parsing and validation/diagnostic paths that genuinely need source
offsets.

### hbk-book

Expected public concepts:

- `HbkBook`
- `BookMeta`
- `BookLocale`
- `BookEntityKind`
- `BookError`
- `FileStorageReader`
- `Toc`
- `TocPage`
- `LocalizedTitle`
- `TocPath`
- `TocParser`
- `TocError`

Owns FR-HBK-002 and FR-HBK-003.

`FileStorageReader` is a narrow book-level reader for repeated `FileStorage` page reads. It may
own `FileStorage` bytes and reuse ZIP archive state for the reader lifetime inside the `hbk-book`
boundary, but it must not expose Syntax Assistant extraction, export or CLI concerns.

`HbkBook` owns the book-level state needed after open: path, metadata, locale and TOC. It validates
the `FileStorage` entity body during open, but must not retain the lower-level `HbkContainer` mmap
or `FileStorage` bytes after construction. Page/file reads are path-backed: the source HBK file must
remain readable after `open` so `HbkBook` can create a short-lived `FileStorageReader` for access.

### hbk-docs

Expected public concepts:

- `DocumentationReader`
- `PageContent`
- `ResolvedLink`
- `DocumentationError`

Owns FR-DOC-001.

### syntax-helper-model

Expected public concepts:

- `PlatformContext`
- `SyntaxHelperSink`
- `SyntaxHelperRecordDetailMode`
- `GlobalMethod`
- `GlobalProperty`
- `GlobalContextEvent`
- `PlatformType`
- `PlatformMethod`
- `PlatformProperty`
- `QueryTableField`
- `QueryTableParameter`
- `Constructor`
- `EnumDefinition`
- `EnumValue`
- `Signature`
- `Parameter`
- `TypeRef`
- `SourceRef`

Owns the domain model used by FR-SH-002, FR-EXPORT-001 and FR-LOOKUP-001.

The model remains provenance-rich for diagnostics and parser maintenance. Consumer export shape is
owned by `hbk-export` and may intentionally omit internal provenance and navigation scaffolding.

`SyntaxHelperSink` is the shared record-family boundary used both by the in-memory
`PlatformContext` lookup path and by streaming export adapters. It must stay typed by domain record
families rather than becoming a generic pipeline abstraction. A sink may request a narrower
`SyntaxHelperRecordDetailMode` only to avoid building fields that its concrete consumer omits; the
default mode remains the full provenance-rich domain model.

### syntax-helper-extract

Expected public concept:

- `SyntaxHelperReader`

Owns FR-SH-001 and FR-SH-002.

Syntax Assistant section parsing is locale-aware for Russian and root/English books. The extractor
recognizes root/English `Type:` and `Returned value:` type-reference sections and treats
availability, examples, see-also, available-since and overload variant headings as section
boundaries so descriptions, signatures and parameter descriptions do not absorb later sections.
T26 extracts structured availability/application contexts, examples, see-also relationships and
available-since facts into the domain model. T27 extracts structured syntax-variant metadata and
binds parameters to the variant signature that owns them. T28 classifies the remaining known
diagnostic-only Syntax Assistant source families with stable family-specific diagnostic codes while
leaving consumer record-family JSON unchanged. T29 promotes global context events, query/table
fields and query/table parameters into typed extraction records while keeping direct TOC-only
global-context method-like pages as recoverable diagnostics.

### hbk-export

Expected public concepts:

- `JsonExporter`
- `PlatformContextExporter`
- `StreamingSyntaxHelperExport`
- lean consumer export DTOs derived from the provenance-rich domain model
- optional separate diagnostic/debug adapters when a concrete maintenance workflow requires them

Owns FR-EXPORT-001.

The streaming export adapter consumes the `SyntaxHelperSink` boundary and writes canonical
record-family JSON without retaining the full `PlatformContext`. The existing `PlatformContext`
exporter remains available for in-memory model consumers and tests. Streaming export may use the
lean sink detail mode to skip consumer-omitted navigation fields, but omission from JSON remains an
`hbk-export` adapter concern rather than an internal model constraint.

Schema version 5 record-family JSON exposes structured `availability`, `examples`, `see_also`,
signature variant metadata, enum values and type-reference facts from the domain model while still
omitting source HBK paths, TOC paths, HTML paths, page titles and duplicate navigation-link catalogs
from consumer records. The export adapter owns consumer-shape simplification: `owner`,
`type_refs`, `return_types`, `availability.since`, `see_also`, property `usage`, signature metadata
and nested enum values are serialized in the lean FR-EXPORT-001 form without forcing the internal
model to discard richer provenance or localized names. It also includes
`global-context-events.json`, `table-fields.json` and `table-parameters.json` for Syntax Assistant
event and query/table metadata families.

### v8-context-hbk-cli

Owns FR-CLI-001.

The installed binary name remains `v8-context-hbk`. Accepted command names are `inspect`, `toc`,
`page` and `syntax-helper`.

### Planned Syntax Assistant query CLI

Owns FR-SH-SEARCH-001 and FR-SH-SEARCH-002 after implementation.

The separate query CLI must read a prebuilt export or index artifact for interactive commands. It
must not parse `shcntx_*.hbk` in exact lookup, text search, fuzzy search or relationship search
commands.

## Implementation Dependencies

Current dependency choices may use:

- `thiserror` for typed errors
- `serde` and `serde_json` for models/export
- `memmap2` or direct `Read + Seek` for container access
- native little-endian reads or a small helper for numeric fields
- `zip` for `FileStorage` and `PackBlock`
- `scraper` or the `html5ever` stack for HTML parsing
- `encoding_rs` if page charset handling requires more than UTF-8
- `clap` for CLI
- `tracing` for diagnostics when diagnostics outgrow direct error values

Do not introduce new broad frameworks or knobs without a requirement, ADR or measured bottleneck.
