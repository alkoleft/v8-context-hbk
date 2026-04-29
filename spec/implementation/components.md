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

## Dependency Rules

- `hbk-container` must not depend on book, docs, extraction or export concerns.
- `hbk-book` must not depend on Syntax Assistant extraction.
- `hbk-docs` may depend on book-level page/TOC abstractions but must not know export schema details.
- `syntax-helper-model` must not depend on HBK container, HTML parsing or CLI code.
- `syntax-helper-extract` owns traversal and parser behavior for Syntax Assistant pages.
- `hbk-export` owns output adapters for the Rust domain model.
- `v8-context-hbk-cli` wires commands and error presentation only.

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
reuse ZIP archive state inside the `hbk-book` boundary, but it must not expose Syntax Assistant
extraction, export or CLI concerns.

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
- `GlobalMethod`
- `GlobalProperty`
- `PlatformType`
- `PlatformMethod`
- `PlatformProperty`
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

### syntax-helper-extract

Expected public concept:

- `SyntaxHelperReader`

Owns FR-SH-001 and FR-SH-002.

### hbk-export

Expected public concepts:

- `JsonExporter`
- `PlatformContextExporter`
- lean consumer export DTOs derived from the provenance-rich domain model
- optional separate diagnostic/debug adapters when a concrete maintenance workflow requires them

Owns FR-EXPORT-001.

### v8-context-hbk-cli

Owns FR-CLI-001.

The installed binary name remains `v8-context-hbk`. Accepted command names are `inspect`, `toc`,
`page` and `syntax-helper`.

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
