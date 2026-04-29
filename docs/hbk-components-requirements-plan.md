# HBK components requirements and implementation plan

## 1. Executive Summary

`v8-context-hbk` builds a Rust component set for reading 1C `.hbk` help books and extracting platform documentation/context from them. The first target is platform `8.5.1.1150` at `/opt/1cv8/x86_64/8.5.1.1150/`, especially `shcntx_ru.hbk` and `shcntx_root.hbk`.

The project is expected to become a component of `/home/alko/develop/open-source/v8-context/` after the HBK extraction model and contracts are validated. Until that integration point, it should stay independently testable and avoid coupling its internal model to unfinished `v8-context` contracts.

The implementation should be split into three reusable layers:

1. HBK container reader: binary container parsing, entity enumeration, entity byte access, ZIP-backed files inside `FileStorage`.
2. Documentation reader: book metadata, TOC parsing, page navigation, HTML/page access, link resolution.
3. Syntax helper context reader: extraction of the 1C platform object model from Syntax Assistant pages: global methods/properties, types, methods, properties, constructors, enums, signatures, parameters, return types and descriptions.

Primary implementation reference: `/home/alko/develop/open-source/hbk-reader`.

Secondary model/search/export reference: `/home/alko/develop/open-source/bsl-context-multi-project/platform-context-exporter`.

## 2. Source Evidence

### 2.1. Platform files

For the current target platform the relevant Syntax Assistant books exist:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

The implementation must not hard-code only these names, but these files are the first acceptance fixtures.

### 2.2. HBK container model from `hbk-reader`

`hbk-reader` establishes the following container facts:

- HBK is a binary container with a 16-byte container header.
- File descriptions are 12-byte records: header address, body address, reserved splitter.
- Blocks have a 31-byte header.
- Numeric fields are little-endian or hexadecimal string fields depending on the header region.
- Entity names are stored as UTF-16LE.
- Important entities for help books:
  - `PackBlock`: compressed TOC block, ZIP stream containing UTF-8 TOC data.
  - `FileStorage`: ZIP archive with HTML pages and related resources.
  - `Book`: UTF-8 metadata.

Reference files:

- `hbk-reader/src/main/kotlin/.../hbk/reader/ContainerReader.kt`
- `hbk-reader/src/main/kotlin/.../hbk/reader/HbkContentReader.kt`
- `hbk-reader/doc/hbk-format.md`
- `hbk-reader/doc/hbk-binary-format.md`

### 2.3. Documentation/navigation model from `hbk-reader`

`hbk-reader` has these reusable concepts:

- `Toc` and `Page` tree with localized titles and HTML paths.
- Page lookup by HTML path and by index path.
- Book metadata with `bookName`, `description`, `tags`.
- Locale inference from filename suffix: `_ru` maps to `ru`, `_root` maps to default English/root locale.

Reference files:

- `hbk-reader/src/main/kotlin/.../hbk/reader/toc/TocParser.kt`
- `hbk-reader/src/main/kotlin/.../hbk/reader/toc/Toc.kt`
- `hbk-reader/src/main/kotlin/.../hbk/reader/meta/BookMetaParser.kt`
- `hbk-reader/doc/models.md`

### 2.4. Syntax helper extraction from `hbk-reader`

`hbk-reader` already splits Syntax Assistant page parsing by page type:

- object pages
- method pages
- property pages
- constructor pages
- enum pages
- enum value pages
- global context pages

It identifies root sections as global context, enum catalog and type catalog, then drills down catalog pages before parsing concrete pages.

Reference files:

- `hbk-reader/src/main/kotlin/.../shctx/PlatformContextReader.kt`
- `hbk-reader/src/main/kotlin/.../shctx/PlatformContextPagesVisitor.kt`
- `hbk-reader/src/main/kotlin/.../shctx/parsers/PlatformContextPagesParser.kt`
- `hbk-reader/src/main/kotlin/.../shctx/parsers/specialized/*.kt`

### 2.5. Export/search model from `platform-context-exporter`

`platform-context-exporter` gives a target DTO shape and consumer-oriented usage:

- `global-properties`
- `global-methods`
- `types`
- `PlatformTypeDefinition`
- `MethodDefinition`
- `PropertyDefinition`
- `Signature`
- `ParameterDefinition`

It also contains useful search/MCP contracts:

- `search(query, type, limit)`
- `info(name, type)`
- `getMember(typeName, memberName)`
- constructor lookup

Reference files:

- `platform-context-exporter/documentation/formats.md`
- `platform-context-exporter/src/main/java/ru/alkoleft/context/platform/dto/*.java`
- `platform-context-exporter/src/main/java/ru/alkoleft/context/platform/mcp/PlatformApiSearchService.java`

## 3. Goals

1. Provide a Rust library API for `.hbk` files that can be reused by CLI, MCP, indexers and other `v8-context` tooling.
2. Read HBK containers without depending on Java/Kotlin libraries at runtime.
3. Expose documentation navigation and page content from any compatible `.hbk`.
4. Extract structured Syntax Assistant data from `shcntx_*.hbk`.
5. Preserve enough source provenance to debug parser gaps: file path, entity name, TOC path, HTML path and page title.
6. Make the first implementation verifiable against platform `8.5.1.1150`.
7. Prefer the best Rust-native algorithms, data models and APIs over preserving backward compatibility with `hbk-reader`, `platform-context-exporter` or their DTO/API shapes.
8. Treat initial contracts as implementation scaffolding: public library, CLI and export contracts are expected to be reworked later after the extraction model is validated on real HBK data.
9. Keep the design suitable for future integration into `/home/alko/develop/open-source/v8-context/` as an HBK-backed context source component.

## 4. Non-goals for the first delivery

1. Writing or modifying `.hbk` containers.
2. Rendering the full HTML help UI.
3. Full-text search ranking/indexing beyond simple lookup helpers.
4. MCP server implementation. The data model should allow it later, but MCP is a follow-up.
5. Runtime extraction from 1C processes. This project reads HBK documentation; runtime probes belong to a separate track.
6. Complete compatibility proof for every platform version. The first acceptance baseline is `8.5.1.1150`; older versions can be added as fixtures later.
7. Backward-compatible reproduction of legacy Java/Kotlin public APIs, class names, JSON DTOs or CLI behavior.
8. Immediate merge into `v8-context` before the HBK reader, Syntax Assistant extractor and provisional contracts are validated.

## 5. Users and jobs

### 5.1. Library consumer

As a Rust tool author, I need to open an HBK file, inspect its entities, read files inside `FileStorage`, and parse TOC/metadata without knowing the binary format.

### 5.2. Documentation consumer

As a documentation tool, I need to traverse the book TOC, resolve a page by path, read page HTML, and follow links consistently.

### 5.3. Platform-context consumer

As an AI/indexing tool, I need structured platform API data from Syntax Assistant: methods, properties, types, constructors, enums, signatures, parameters and return types.

### 5.4. Parser maintainer

As a maintainer, I need deterministic parser tests with small fixtures and clear failure context when platform HTML changes.

## 6. Functional Requirements

### 6.1. HBK container reader

Required API capabilities:

- Open an HBK file by path.
- Validate the container header and block headers enough to fail early on unsupported/corrupt input.
- Enumerate entity names and entity metadata.
- Read entity bytes by name.
- Read chained block bodies correctly.
- Preserve source offsets for diagnostics.
- Support large files without unnecessary whole-file copies where practical.
- Return typed errors instead of panics.

Expected Rust module boundary:

- `hbk::container`
  - `HbkContainer`
  - `ContainerHeader`
  - `BlockHeader`
  - `EntityDescriptor`
  - `EntityName`
  - `ContainerError`

Acceptance:

- `HbkContainer::open("/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk")` succeeds.
- Entity enumeration includes at least `PackBlock`, `FileStorage`, `Book`.
- Reading `Book` returns parseable UTF-8 metadata bytes.
- Reading a missing entity returns a domain error.

### 6.2. HBK book/content reader

Required API capabilities:

- Open a help book on top of `HbkContainer`.
- Inflate `PackBlock`.
- Open `FileStorage` as a ZIP archive.
- Parse `Book` metadata.
- Infer locale from filename.
- Read a stored file by HTML/resource path.
- Normalize leading slash in page paths.

Expected Rust module boundary:

- `hbk::book`
  - `HbkBook`
  - `BookMeta`
  - `BookLocale`
  - `BookEntityKind`
  - `BookError`

Acceptance:

- `HbkBook::open(shcntx_ru.hbk)` returns `BookMeta` and locale `ru`.
- `HbkBook::open(shcntx_root.hbk)` returns root/default locale.
- A page path from TOC can be read from `FileStorage`.

### 6.3. TOC and documentation navigation

Required API capabilities:

- Parse inflated `PackBlock` TOC text.
- Preserve hierarchical page tree.
- Store localized page titles.
- Store HTML page path.
- Find page by HTML path.
- Find page by index path.
- Return children for a page.
- Expose a flattened iterator with parent path/provenance.

Expected Rust module boundary:

- `hbk::toc`
  - `Toc`
  - `TocPage`
  - `LocalizedTitle`
  - `TocPath`
  - `TocParser`
  - `TocError`

Acceptance:

- TOC parse succeeds for `shcntx_ru.hbk`.
- Root pages include global context, enum catalog and type catalog candidates.
- Lookup by a known page path returns the same page as tree traversal.

### 6.4. HTML/documentation reader

Required API capabilities:

- Read raw page HTML.
- Parse HTML into a document representation for extraction.
- Extract a text/markdown-like body for diagnostics and search previews.
- Resolve internal `v8help://`/relative links to TOC or storage paths where possible.
- Preserve unresolved links as diagnostics instead of silently dropping them.

Expected Rust module boundary:

- `hbk::docs`
  - `DocumentationReader`
  - `PageContent`
  - `ResolvedLink`
  - `DocumentationError`

Acceptance:

- A page from global methods can be loaded as HTML.
- The reader returns title/path/content/provenance.
- Link resolution is deterministic and covered by fixture tests.

### 6.5. Syntax Assistant object model reader

Required API capabilities:

- Locate Syntax Assistant root sections:
  - global context
  - system enums/value sets
  - type/object catalog
- Extract global methods and properties.
- Extract platform types/objects.
- Extract type methods, properties and constructors.
- Extract enum definitions and enum values.
- Extract signatures, parameters, required flags and return types when present.
- Preserve Russian and English names/aliases when the page exposes them.
- Preserve descriptions in normalized text/markdown form.
- Attach provenance to each extracted item.

Expected Rust module boundary:

- `syntax_helper`
  - `SyntaxHelperReader`
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
  - `SyntaxHelperError`

Acceptance:

- Reading `shcntx_ru.hbk` returns non-empty:
  - global methods
  - global properties
  - platform types
  - enums
- Extracted data can be serialized to JSON.
- At least one known global method, one known global property, one known type and one known enum are covered by tests from fixture HTML.

### 6.6. Export model

Required API capabilities:

- Serialize the extracted model to JSON as the canonical machine format.
- Use an internal export shape that best represents the Rust domain model, including provenance and localization.
- A legacy-shaped export inspired by `platform-context-exporter` may be added only when a real consumer needs it; it must remain an adapter, not a constraint on the internal model.
- Do not preserve old DTO field names or file layout when a clearer schema is available for current consumers.
- Mark first-stage export schemas as provisional until M6 acceptance data and downstream consumer needs are reviewed.

Expected Rust module boundary:

- `export`
  - `JsonExporter`
  - `PlatformContextExporter`
  - optional legacy export adapters if needed

Acceptance:

- CLI/test helper can export `shcntx_ru.hbk` to JSON files.
- Output contains documented field names suitable for downstream experiments, but they are not yet a stable compatibility promise.

### 6.7. CLI

The initial CLI should be small and verification-oriented:

```bash
v8-context-hbk inspect /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk
v8-context-hbk toc /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --format json
v8-context-hbk page /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --path "..."
v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context
```

Acceptance:

- Commands fail with non-zero exit and readable error on missing/corrupt input.
- `inspect` prints entity names and basic metadata.
- `syntax-helper` writes JSON export files.

## 7. Non-functional Requirements

### 7.1. Reliability

- No parser `unwrap()`/panic on user-controlled HBK/HTML input.
- Errors include path/entity/page context.
- Unsupported structures fail explicitly.

### 7.2. Performance

- Container opening should not eagerly decompress all pages.
- Page content should be read lazily from `FileStorage`.
- Full Syntax Assistant extraction can be eager for the first milestone, but the lower layers must remain lazy.

### 7.3. Testability

- Small fixture tests for binary/block parsing.
- Fixture HTML tests copied/adapted from `hbk-reader/src/test/resources`.
- Integration tests against `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` may be ignored by default unless the file exists.

### 7.4. Compatibility policy

- First supported platform baseline: `8.5.1.1150`.
- Parser should avoid assumptions that are only true for a single HTML filename if the TOC carries a more reliable relationship.
- Root section detection should be data-driven and tested against Russian/root books.
- Do not preserve backward compatibility for its own sake. If a different parser strategy, data structure, API or output schema is simpler, more correct, more observable or more performant, prefer it and document the migration impact.
- Contract stability is intentionally deferred. Before the first stable API/export release, contracts may be redesigned based on parser evidence, consumer feedback and better model boundaries.

### 7.5. Licensing and attribution

- `hbk-reader` is MIT-licensed and can be used as a reference.
- Any ported logic should preserve attribution where appropriate.
- Do not copy generated platform documentation into the repository except minimal test fixtures required for parser tests.

## 8. Proposed Rust Dependencies

Initial candidates:

- `thiserror` for typed errors.
- `serde`, `serde_json` for models/export.
- `memmap2` or direct `Read + Seek` for container access. Start with `Read + Seek` unless mmap clearly simplifies large-file access.
- `byteorder` or native little-endian reads for numeric fields.
- `zip` for `FileStorage` and `PackBlock` ZIP handling.
- `scraper` or `html5ever` stack for HTML parsing.
- `encoding_rs` if page charset handling requires more than UTF-8.
- `clap` for CLI.
- `tracing` for diagnostics.

Decision to make during implementation: whether the container reader should be generic over `Read + Seek` from the start, or file-backed only for the first milestone.

## 9. Milestone Plan

### M0. Project baseline and documentation

Scope:

- Keep this requirements/plan document as the first source of truth.
- Add README with project purpose and current target platform.
- Add minimal CI/test command contract if the repository does not have one.

Exit criteria:

- `cargo test` passes.
- Requirements document is committed or otherwise treated as baseline.

### M1. HBK container reader

Scope:

- Implement `hbk::container`.
- Parse container header, file descriptors, entity names and chained block bodies.
- Add `inspect` CLI.
- Add tests with small fixtures and optional real-platform smoke.

Exit criteria:

- `inspect shcntx_ru.hbk` shows `PackBlock`, `FileStorage`, `Book`.
- Missing/corrupt file tests produce typed errors.

### M2. Book reader, TOC and file access

Scope:

- Implement `hbk::book` and `hbk::toc`.
- Inflate `PackBlock`.
- Parse `Book` metadata.
- Open `FileStorage` ZIP and read page files.
- Add `toc` and `page` CLI.

Exit criteria:

- TOC parse succeeds for `shcntx_ru.hbk`.
- A known TOC page can be read from `FileStorage`.
- Locale and metadata are returned.

### M3. Documentation HTML layer

Scope:

- Implement `hbk::docs`.
- Parse HTML into page content.
- Add basic body extraction and link normalization.
- Add fixture tests for representative pages.

Exit criteria:

- Raw HTML and normalized text can be obtained for global method/property/type pages.
- Link handling has tests for relative and unresolved links.

### M4. Syntax helper extraction

Scope:

- Port page-type detection and specialized parsers from `hbk-reader`.
- Define internal Rust domain model with provenance.
- Extract global methods/properties, types, enums, members, constructors and signatures.
- Add fixture tests from representative HTML pages.

Exit criteria:

- `syntax-helper shcntx_ru.hbk --output target/context` produces non-empty JSON.
- Known method/property/type/enum fixture assertions pass.
- Errors report source page path and parser stage.

### M5. Export schema and consumer helpers

Scope:

- Add canonical JSON export for the Rust domain model.
- Add legacy-shaped export only if a concrete downstream consumer requires the `platform-context-exporter` JSON shape.
- Add simple lookup helpers:
  - find exact API element by name/type
  - find type member by type/member name
  - list constructors by type
- Keep MCP/search server out of scope, but avoid blocking it.

Exit criteria:

- Canonical JSON export is written and covered by snapshot/shape tests.
- Any legacy-shaped export is covered by adapter tests and clearly marked as non-authoritative.

### M6. Real-platform acceptance pass

Scope:

- Run full extraction against:
  - `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`
- Record counts, parse gaps and unresolved pages.
- Decide whether root/English book should be a supported equal source or only a fallback/localization input.

Exit criteria:

- Acceptance report records counts and known gaps.
- Parser gaps are represented as actionable tasks, not hidden warnings.

## 10. Acceptance Baseline

The first implementation is acceptable when:

1. `cargo test` passes.
2. `cargo run -- inspect /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` succeeds and lists core entities.
3. `cargo run -- toc /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --format json` succeeds.
4. `cargo run -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context` creates JSON files with non-empty methods/properties/types/enums.
5. At least one representative fixture exists for each specialized parser:
   - object
   - method
   - property
   - constructor
   - enum
   - enum value
6. Errors include enough context to identify the failing HBK file and HTML page.

## 11. Risks and Mitigations

### 11.1. HBK binary format drift

Risk: 8.5 HBK files may differ from examples tested in `hbk-reader`.

Mitigation:

- Start M1 with `8.5.1.1150` smoke tests.
- Keep block/header parsing strict but diagnostic.
- Record unsupported fields instead of ignoring suspicious values silently.

### 11.2. HTML shape drift

Risk: Syntax Assistant page markup may differ across platform versions.

Mitigation:

- Build parser fixtures from real 8.5 pages.
- Keep page-type parsers isolated.
- Return parse warnings/gaps with source refs.

### 11.3. Legacy DTO reuse

Risk: `platform-context-exporter` DTOs may be convenient to copy, but they can freeze old constraints and lose localization/provenance.

Mitigation:

- Use the richer internal model as the only authoritative model.
- Treat legacy DTO/export shapes as optional adapters for concrete consumers.
- Prefer new algorithms and schemas when they reduce complexity or improve correctness.

### 11.4. Mixing documentation and runtime context truth

Risk: HBK documentation data may be confused with runtime platform introspection.

Mitigation:

- Mark all extracted items with source `hbk`.
- Keep runtime extraction out of this project milestone.
- Later compare against runtime/static sources in separate artifacts.

## 12. Open Questions

1. Should the crate expose only a library plus tiny CLI, or should the CLI be a first-class supported tool?
2. Should `shcntx_root.hbk` and `shcntx_ru.hbk` be merged into bilingual records, or treated as separate localized books?
3. What is the canonical internal field naming for the new Rust/domain model: English field names with `ru_name`/`en_name`, or another explicit localization structure?
4. Should search/indexing live in this crate later, or remain in a separate consumer crate?
5. Which minimal real HTML pages from 8.5 may be committed as parser fixtures without overloading the repository?

## 13. Immediate Next Steps

1. Add README that points to this document and declares the current baseline.
2. Implement M1 container reader.
3. Copy or synthesize minimal binary fixture tests for block parsing.
4. Add ignored smoke test for `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`.
5. After M1 passes, port TOC parsing and validate `PackBlock` against real `shcntx_ru.hbk`.
