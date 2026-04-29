# Functional Requirements

## Scope

`v8-context-hbk` reads 1C `*.hbk` help books and extracts structured platform documentation/context
from Syntax Assistant books. The first target platform baseline is `8.5.1.1150`.

The project stays independently testable until the HBK extraction model and provisional contracts are
validated on real HBK data. Future `v8-context` integration must use an explicit boundary, currently
the file-level export decided in ADR-0001.

## Goals

- Provide Rust APIs for opening `.hbk` files, enumerating container entities and reading help-book
  content.
- Expose documentation navigation and page content from compatible `.hbk` files.
- Extract structured Syntax Assistant data from `shcntx_*.hbk`.
- Preserve provenance for diagnostics: HBK file path, entity name, TOC path, HTML path and page
  title.
- Keep public library, CLI and export contracts provisional until real-platform acceptance and
  downstream consumer feedback justify stabilization.

## Non-Goals

- Writing or modifying `.hbk` containers.
- Rendering the full HTML help UI.
- Full-text search ranking or indexing beyond exact lookup helpers.
- MCP server implementation.
- Runtime extraction from 1C processes.
- Complete compatibility proof for every platform version.
- Backward-compatible reproduction of Java/Kotlin public APIs, class names, DTOs or CLI behavior.
- Immediate merge into `/home/alko/develop/open-source/v8-context/`.

## FR-HBK-001: Container Reader

The system must open an HBK file by path, validate the container enough to fail early on corrupt or
unsupported input, enumerate entity names and metadata, read entity bytes by name and read chained
block bodies.

Required diagnostics:

- source path
- entity name when known
- source offsets where relevant
- typed errors instead of panics

Acceptance:

- `fmtdui_root.hbk` and `fmtdui_ru.hbk` open successfully when platform fixtures exist.
- Entity enumeration includes at least `PackBlock`, `FileStorage` and `Book`.
- Reading `Book` returns parseable UTF-8 metadata bytes.
- Reading a missing entity returns a domain error.

## FR-HBK-002: Help Book Reader

The system must open a help book on top of the container reader, inflate `PackBlock`, open
`FileStorage` as ZIP, parse `Book` metadata, infer locale from filename and read stored files by
HTML/resource path.

Acceptance:

- `fmtdui_ru.hbk` returns locale `ru`.
- `fmtdui_root.hbk` returns root/default source locale and maps to export locale `en`.
- A page path from TOC can be read from `FileStorage`.

## FR-HBK-003: TOC and Navigation

The system must parse inflated `PackBlock` TOC text, preserve a hierarchical page tree, store
localized page titles and HTML paths, find pages by HTML path and index path, and expose flattened
traversal with parent path/provenance.

Acceptance:

- TOC parse succeeds for `fmtdui_root.hbk` and `fmtdui_ru.hbk`.
- Lookup by a known page path returns the same page as tree traversal.

## FR-DOC-001: Documentation Page Reader

The system must read raw page HTML, parse it into a documentation representation, extract title,
normalized text preview and deterministic links, and preserve unresolved links as recoverable
diagnostics.

Acceptance:

- A page from a small real HBK book loads as HTML.
- The reader returns title, path, content and provenance.
- Link resolution is deterministic and covered by fixture tests.

## FR-SH-001: Syntax Assistant Root Discovery

The system must locate Syntax Assistant root sections for global context, system enums/value sets and
type/object catalogs.

Acceptance:

- `shcntx_ru.hbk` root discovery finds candidates for global context, enum catalog and type/object
  catalog when the platform fixture exists.
- Unknown page classes become diagnostics rather than hidden skips.

## FR-SH-002: Syntax Assistant Extraction

The system must extract:

- global methods and properties
- platform types/objects
- type methods, properties and constructors
- enum definitions and enum values
- signatures, parameters, required flags and return types when present
- localized names/aliases when present
- normalized descriptions
- source provenance for every extracted item

Multiple signatures are overloads. If real pages expose multiple return types for one overload while
the model assumes one return type per overload, report it as a parser/data-contract gap instead of
silently truncating.

Acceptance:

- Reading `shcntx_ru.hbk` returns non-empty global methods, global properties, platform types and
  enums when the fixture exists.
- Fixture tests cover at least one known global method, global property, type and enum.

## FR-EXPORT-001: Canonical JSON Export

The system must serialize the extracted model to JSON as the canonical machine format.

Required files:

- `metadata.json`
- `global-contexts.json`
- `global-methods.json`
- `global-properties.json`
- `platform-types.json`
- `type-methods.json`
- `type-properties.json`
- `constructors.json`
- `enums.json`
- `enum-values.json`
- `diagnostics.json`

Each record-family file is a JSON object with `schema_version`, `locale`, `source_locale`,
`source_hbk`, `record_kind` and `records`. Individual records keep parser provenance under `source`.

Acceptance:

- `shcntx_ru.hbk` exports as locale `ru`.
- `shcntx_root.hbk` exports as locale `en`.
- Output files are non-empty and parse successfully.

## FR-LOOKUP-001: Exact Lookup Helpers

The system must provide exact lookup helpers for:

- global member by name
- type by name
- type member by type/member name
- constructors by type name

Search ranking is out of scope.

## FR-CLI-001: Verification-Oriented CLI

The initial CLI must support:

```bash
v8-context-hbk inspect <book.hbk>
v8-context-hbk toc <book.hbk> --format json
v8-context-hbk page <book.hbk> --path <html-path>
v8-context-hbk syntax-helper <shcntx.hbk> --output <dir>
```

Acceptance:

- Commands fail with non-zero exit and readable error on missing/corrupt input.
- `inspect` prints entity names and basic metadata.
- `syntax-helper` writes canonical JSON export files.
