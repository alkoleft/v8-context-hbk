# HBK components requirements and implementation plan

## 1. Executive Summary

`v8-context-hbk` builds a Rust component set for reading 1C `.hbk` help books and extracting platform documentation/context from them. The first target is platform `8.5.1.1150` at `/opt/1cv8/x86_64/8.5.1.1150/`.

The early HBK container/book/navigation stages should use small real HBK files, not the large Syntax Assistant books, so failures are fast and easy to isolate. The first small smoke pair is `fmtdui_root.hbk` and `fmtdui_ru.hbk`. The large `shcntx_ru.hbk` and `shcntx_root.hbk` files are the acceptance inputs for Syntax Assistant extraction and the final parser-gap report.

The project is expected to become a component of `/home/alko/develop/open-source/v8-context/` after the HBK extraction model and contracts are validated. Until that integration point, it should stay independently testable and avoid coupling its internal model to unfinished `v8-context` contracts.

The implementation is split into reusable crates that preserve the same context boundaries:

1. `hbk-container`: binary container parsing, entity enumeration and entity byte access.
2. `hbk-book`: book metadata, locale inference, ZIP-backed `FileStorage`, TOC parsing and page reads.
3. `hbk-docs`: documentation HTML/page parsing, normalized text/link extraction and page diagnostics.
4. `syntax-helper-model`: provenance-rich platform context domain model and lookup helpers.
5. `syntax-helper-extract`: extraction of the 1C platform object model from Syntax Assistant pages: global methods/properties, types, methods, properties, constructors, enums, signatures, parameters, return types and descriptions.
6. `hbk-export`: canonical JSON export adapters.
7. `v8-context-hbk-cli`: command wiring for the installed `v8-context-hbk` binary.

Primary implementation reference: `/home/alko/develop/open-source/hbk-reader`.

Secondary model/search/export reference: `/home/alko/develop/open-source/bsl-context-multi-project/platform-context-exporter`.

## 2. Source Evidence

### 2.1. Platform files

For the current target platform the first small real HBK smoke files exist:

- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`

The relevant Syntax Assistant books also exist:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

The implementation must not hard-code only these names. `fmtdui_*` is the first fast smoke pair for generic HBK behavior; `shcntx_*` is reserved for Syntax Assistant stages; the last acceptance stage should smoke all `*.hbk` files in the target platform directory.

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

- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/hbk/reader/ContainerReader.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/hbk/reader/HbkContentReader.kt`
- `hbk-reader/doc/hbk-format.md`
- `hbk-reader/doc/hbk-binary-format.md`

These local reference paths are current evidence anchors, not long-term normative dependencies. They should be removed or replaced by repo-local evidence after the Rust implementation validates the HBK model.

### 2.3. Documentation/navigation model from `hbk-reader`

`hbk-reader` has these reusable concepts:

- `Toc` and `Page` tree with localized titles and HTML paths.
- Page lookup by HTML path and by index path.
- Book metadata with `bookName`, `description`, `tags`.
- Locale inference from filename suffix: `_ru` maps to `ru`; `_root` is the default English/root source. Export-facing locale for `_root` must be `en`; the internal representation may keep `root` or another typed default-locale marker.

Reference files:

- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/hbk/reader/toc/TocParser.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/hbk/reader/toc/Toc.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/hbk/reader/meta/BookMetaParser.kt`
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

It gives observations for root-section discovery, catalog traversal and specialized page parsing. These observations are examples, not a public API to reproduce exactly; the Rust implementation should prefer data-driven detection over copying brittle filename/title assumptions.

Reference files:

- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/shctx/PlatformContextReader.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/shctx/PlatformContextPagesVisitor.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/shctx/parsers/PlatformContextPagesParser.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/shctx/parsers/specialized/*.kt`

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

The external reference projects are the current source of truthful observations for this planning phase. They are not intended to stay as permanent specification dependencies after this repository records its own validated contracts and evidence.

## 3. Goals

1. Provide a Rust library API for `.hbk` files that can be reused by CLI, MCP, indexers and other `v8-context` tooling.
2. Read HBK containers without depending on Java/Kotlin libraries at runtime.
3. Expose documentation navigation and page content from any compatible `.hbk`.
4. Extract structured Syntax Assistant data from `shcntx_*.hbk`.
5. Preserve enough source provenance to debug parser gaps: file path, entity name, TOC path, HTML path and page title.
6. Make the first implementation verifiable against platform `8.5.1.1150` with tiered checks: small real HBK smoke files first, Syntax Assistant extraction later, and all-HBK smoke at the end.
7. Use `hbk-reader` and `platform-context-exporter` as observation sources. Port behavior where it is the shortest path to validated extraction, but keep limitations explicit and do not freeze their public APIs as Rust contracts.
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

- `HbkContainer::open("/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk")` and `HbkContainer::open("/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk")` succeed.
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

- `HbkBook::open(fmtdui_ru.hbk)` returns `BookMeta` and locale `ru`.
- `HbkBook::open(fmtdui_root.hbk)` returns root/default source locale; export-facing locale mapping for that source is `en`.
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

- TOC parse succeeds for `fmtdui_root.hbk` and `fmtdui_ru.hbk`.
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

- A page from a small real HBK book can be loaded as HTML.
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
- Start from observed `hbk-reader` parser behavior where it is useful, but record known limitations explicitly.
- Treat multiple signatures as overloads. The current observed model has one return type per overload; if real pages expose multiple return types for one overload, report it as a parser/data-contract gap instead of silently truncating.

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
- At least one known global method, one known global property, one known type and one known enum are covered by tests from fixture HTML.

### 6.6. Export model

Required API capabilities:

- Serialize the extracted model to JSON as the canonical machine format.
- Use an internal export shape that best represents the Rust domain model, including provenance and localization.
- Export locale for `_root` books is `en`, even if the internal book locale keeps a `root` marker.
- Write one metadata file plus one JSON file per exported record family. The provisional canonical file names are `metadata.json`, `global-contexts.json`, `global-methods.json`, `global-properties.json`, `platform-types.json`, `type-methods.json`, `type-properties.json`, `constructors.json`, `enums.json`, `enum-values.json` and `diagnostics.json`.
- Each record-family file is a JSON object with `schema_version`, `locale`, `source_locale`, `source_hbk`, `record_kind` and `records`. Individual records keep parser provenance under `source`: HBK path, source locale, TOC path, HTML path and page title.
- A legacy-shaped export inspired by `platform-context-exporter` may be added only when a real consumer needs it; it must remain an adapter, not a constraint on the internal model.
- Do not preserve old DTO field names or file layout when a clearer schema is available for current consumers.
- Mark first-stage export schemas as provisional until M7/T9 acceptance data and downstream consumer needs are reviewed.

Expected Rust module boundary:

- `export`
  - `JsonExporter`
  - `PlatformContextExporter`
  - optional legacy export adapters if needed

Acceptance:

- CLI/test helper can export `shcntx_ru.hbk` and `shcntx_root.hbk` to JSON files, with `_root` written as export locale `en`.
- Output contains documented field names suitable for downstream experiments, but they are not yet a stable compatibility promise.

### 6.7. CLI

The initial CLI should be small and verification-oriented:

```bash
v8-context-hbk inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk
v8-context-hbk toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json
v8-context-hbk page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --path "..."
v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/ru
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
- Real-file smoke tests for early HBK layers should use small target-platform books such as `fmtdui_root.hbk` and `fmtdui_ru.hbk`.
- Syntax Assistant integration tests should use `shcntx_ru.hbk` and `shcntx_root.hbk`, because these files contain the platform API documentation model.
- Fixture HTML tests may be copied/adapted from `hbk-reader/src/test/resources`, but the Syntax Assistant fixture corpus must be curated from real 8.5 pages with a manifest that records source HBK file, HTML path, page title and parser kind.
- Broad all-HBK smoke is a final acceptance/reporting stage, not a prerequisite for early parser work.

Verification tiers:

1. Unit fixtures: deterministic binary/parser fixtures committed to the repository.
2. Small real-HBK smoke: selected small books from `/opt/1cv8/x86_64/8.5.1.1150/`.
3. Syntax Assistant smoke: `shcntx_ru.hbk` and `shcntx_root.hbk`.
4. All-HBK smoke: enumerate every `*.hbk` in the target platform directory and report per-file results.

### 7.4. Compatibility policy

- First supported platform baseline: `8.5.1.1150`.
- Parser should avoid assumptions that are only true for a single HTML filename if the TOC carries a more reliable relationship.
- Root section detection should be data-driven and tested against Russian/root books.
- Do not preserve backward compatibility for its own sake. If a different parser strategy, data structure, API or output schema is simpler, more correct, more observable or more performant, prefer it and document the migration impact.
- Contract stability is intentionally deferred. Before the first stable API/export release, contracts may be redesigned based on parser evidence, consumer feedback and better model boundaries.

### 7.5. Diagnostics policy

- Fatal errors stop the current command/test: missing file, invalid container structure, missing required HBK entities, unreadable ZIP storage, malformed book metadata or TOC corruption.
- Recoverable extraction diagnostics do not abort a full Syntax Assistant pass when partial extraction is still meaningful: unknown page class, unsupported HTML block, unresolved link, missing optional section or parser field that cannot be mapped safely.
- Data-contract violations are recoverable during broad extraction but must be explicit parser gaps: for example, if a page exposes multiple return types for one overload while the current model assumes one return type per overload.
- Every recoverable diagnostic must include severity, stable code, source HBK path, locale/source-locale, TOC path when known, HTML path when known, page title when known and parser stage.
- CLI commands should return non-zero for fatal errors. Reporting commands that scan many files may continue after per-file failures, but their final summary must make failures visible and return non-zero when the requested acceptance contract is not met.

### 7.6. Licensing and attribution

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

### Roadmap framing

The implementation roadmap should be read as a Now/Next/Later plan:

- Now: M0-M3. Prove the HBK binary/container, book, TOC and documentation-page layers on small real 8.5 HBK files.
- Next: M4-M6. Curate Syntax Assistant fixtures from real `shcntx_*` pages, implement extraction and add provisional export/lookup helpers.
- Later: M7-M9. Run `shcntx_*` acceptance, smoke all target-platform HBK files, then decide `v8-context` integration.
- Structural follow-up: M10 splits the validated monolithic implementation into the reusable workspace crates listed above without changing accepted CLI behavior.

The first release is not an API-stability release. It is an evidence-building release whose main output is a working reader, parser gap inventory and a better model for the stable contract.

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

- `inspect fmtdui_root.hbk` and `inspect fmtdui_ru.hbk` show `PackBlock`, `FileStorage`, `Book`.
- Missing/corrupt file tests produce typed errors.

### M2. Book reader, TOC and file access

Scope:

- Implement `hbk::book` and `hbk::toc`.
- Inflate `PackBlock`.
- Parse `Book` metadata.
- Open `FileStorage` ZIP and read page files.
- Add `toc` and `page` CLI.

Exit criteria:

- TOC parse succeeds for `fmtdui_root.hbk` and `fmtdui_ru.hbk`.
- A known TOC page can be read from `FileStorage`.
- Locale and metadata are returned.

### M3. Documentation HTML layer

Scope:

- Implement `hbk::docs`.
- Parse HTML into page content.
- Add basic body extraction and link normalization.
- Add fixture tests for representative pages.

Exit criteria:

- Raw HTML and normalized text can be obtained for pages from the small real HBK smoke pair.
- Link handling has tests for relative and unresolved links.

### M4. Syntax Assistant fixture corpus

Scope:

- Inspect real pages from `shcntx_ru.hbk` and `shcntx_root.hbk`.
- Select the minimal fixture set for root/catalog pages and each specialized parser kind.
- Record a fixture manifest with source HBK file, HTML path, page title, parser kind and reason for inclusion.
- Copy only the minimal generated documentation fragments needed for parser tests.

Exit criteria:

- Fixture manifest exists.
- Fixture inputs cover global context, global method, global property, object/type, object method, object property, constructor, enum, enum value and root/catalog pages.

### M5. Syntax helper extraction

Scope:

- Use `hbk-reader` page-type detection and specialized parsers as observation sources.
- Define internal Rust domain model with provenance.
- Extract global methods/properties, types, enums, members, constructors and signatures.
- Add fixture tests from the curated Syntax Assistant corpus.

Exit criteria:

- Full in-memory extraction against `shcntx_ru.hbk` returns non-empty global methods, global properties, platform types and enums.
- Known object/type, method, property, constructor, enum, enum-value and global-context fixture assertions pass.
- Errors report source page path and parser stage.

### M6. Export schema and consumer helpers

Scope:

- Add canonical JSON export for the Rust domain model.
- Export `_root` Syntax Assistant data as locale `en`.
- Add legacy-shaped export only if a concrete downstream consumer requires the `platform-context-exporter` JSON shape.
- Add simple lookup helpers:
  - find exact API element by name/type
  - find type member by type/member name
  - list constructors by type
- Keep MCP/search server out of scope, but avoid blocking it.

Exit criteria:

- Canonical JSON export is written and covered by snapshot/shape tests.
- Any legacy-shaped export is covered by adapter tests and clearly marked as non-authoritative.

### M7. Real-platform Syntax Assistant acceptance pass

Scope:

- Run full extraction against:
  - `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`
- Record counts, parse gaps and unresolved pages.
- Confirm export-locale mapping: `shcntx_ru.hbk` as `ru`, `shcntx_root.hbk` as `en`.
- Decide remaining localization merge rules for downstream consumers.

Exit criteria:

- Acceptance report records counts and known gaps.
- Parser gaps are represented as actionable tasks, not hidden warnings.

### M8. All-HBK smoke pass

Scope:

- Enumerate all `*.hbk` files under `/opt/1cv8/x86_64/8.5.1.1150/`.
- Run container/book/TOC smoke checks for every file.
- Record per-file success, fatal failures and unsupported structures.

Exit criteria:

- All-HBK smoke report records file count, command summary and per-file failures.
- Unsupported structures become follow-up tasks when they are relevant to the supported scope.

### M9. `v8-context` integration decision

Scope:

- Compare the accepted HBK export model with current `v8-context` source models and decisions.
- Decide whether this crate remains standalone, becomes a workspace member, or exports a file-level integration artifact first.

Exit criteria:

- Decision artifact exists and references M7/M8 evidence.

### M10. Reusable workspace crate split

Scope:

- Convert the repository to a Cargo workspace.
- Move the validated implementation into `hbk-container`, `hbk-book`, `hbk-docs`, `syntax-helper-model`, `syntax-helper-extract`, `hbk-export` and `v8-context-hbk-cli`.
- Preserve the installed binary name `v8-context-hbk` and the accepted `inspect`, `toc`, `page` and `syntax-helper` command behavior.
- Keep lower-level crates independent from higher-level concerns.

Exit criteria:

- `cargo test --workspace` passes.
- Package-level checks for every workspace crate pass.
- The accepted CLI smoke commands still work through `cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- ...`.

## 10. Acceptance Baseline

The first implementation is acceptable when:

1. `cargo test --workspace` passes.
2. `cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk` and `cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` succeed and list core entities.
3. `cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json` succeeds, and a known page from `fmtdui_ru.hbk` can be read through `page`.
4. A Syntax Assistant fixture corpus exists with a manifest for each parser kind:
   - object
   - method
   - property
   - constructor
   - enum
   - enum value
   - global context
   - root/catalog
5. `cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/ru` and `cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/context/en` create JSON files with non-empty methods/properties/types/enums.
6. Errors and recoverable diagnostics include enough context to identify the failing HBK file, locale/source locale, HTML page and parser stage.
7. The final all-HBK smoke report covers every `*.hbk` file under `/opt/1cv8/x86_64/8.5.1.1150/`.

## 11. Epic Specifications

### Epic E1. HBK container foundation

Hypothesis: if the project provides a small, typed Rust container reader, then all later readers can be implemented and tested without carrying Java/Kotlin runtime dependencies because entity access, block chaining and diagnostics are isolated in one layer.

Primary users:

- library consumer
- parser maintainer

Requirements:

- Implement the `hbk::container` API from section 6.1.
- Keep entity access byte-oriented and independent from book/document semantics.
- Preserve offsets and entity names in diagnostics.
- Add `inspect` as the first CLI smoke path.

Acceptance:

- Real `fmtdui_root.hbk` and `fmtdui_ru.hbk` open successfully.
- Entity list contains `PackBlock`, `FileStorage`, `Book`.
- Missing entity and corrupt input return typed errors.
- Unit tests cover descriptor parsing, UTF-16LE entity names and chained block reading.

Out of scope:

- ZIP handling.
- TOC parsing.
- Syntax Assistant semantics.

### Epic E2. Help book navigation

Hypothesis: if the project exposes metadata, TOC and page storage through a book-level API, then both documentation tooling and Syntax Assistant extraction can share a single navigation source instead of each parser re-solving paths independently.

Primary users:

- documentation consumer
- platform-context consumer

Requirements:

- Implement `hbk::book` and `hbk::toc`.
- Inflate `PackBlock`.
- Open `FileStorage` through a ZIP reader.
- Parse `Book` metadata and locale.
- Normalize page paths at the book boundary.
- Expose TOC tree, flattened traversal and lookups by HTML/index path.

Acceptance:

- `toc` CLI returns JSON for `fmtdui_ru.hbk`.
- At least one TOC page can be loaded from `FileStorage`.
- `fmtdui_ru.hbk` maps to locale `ru`; `fmtdui_root.hbk` maps to root/default source locale and export locale `en`.

Out of scope:

- HTML text extraction beyond raw page access.
- Merging localized books.

### Epic E3. Documentation page model

Hypothesis: if raw HTML is normalized into a documentation page model with source refs and deterministic link handling, then parser failures can be debugged from small fixtures and search/indexing consumers can use page previews before the full Syntax Assistant model is complete.

Primary users:

- documentation consumer
- parser maintainer

Requirements:

- Implement `hbk::docs`.
- Load raw page HTML lazily.
- Extract title/path/body preview.
- Resolve relative and `v8help://` links where possible.
- Preserve unresolved links as diagnostics.

Acceptance:

- Fixture tests cover normalized text and link handling from representative real pages.
- Normalized text is stable enough for snapshot tests.
- Unresolved links are observable in diagnostics.

Out of scope:

- Full search ranking.
- Rendering a help UI.

### Epic E4. Syntax Assistant extraction

Hypothesis: if Syntax Assistant extraction is modeled around page types and provenance-rich domain records, then downstream AI/context consumers can use HBK as a structured source while parser gaps remain actionable instead of hidden in lossy exports.

Primary users:

- platform-context consumer
- parser maintainer

Requirements:

- Implement `syntax_helper::SyntaxHelperReader`.
- Build parser tests from the curated Syntax Assistant fixture corpus.
- Detect root sections for global context, enums and type/object catalog.
- Parse object, method, property, constructor, enum and enum-value pages.
- Extract signatures, parameters, required flags, return types and descriptions when present.
- Preserve localized names/aliases and `SourceRef` for every extracted record.
- Report page-level parser warnings without aborting the entire extraction when partial extraction is possible.

Acceptance:

- Full extraction against `shcntx_ru.hbk` returns non-empty global methods, global properties, platform types and enums.
- Representative fixture tests exist for every specialized parser: object/type, method, property, constructor, enum, enum-value and global-context.
- Parser warnings include page path, page title and parser stage.

Out of scope:

- Runtime verification against 1C process objects.
- Stable MCP/search API.

### Epic E5. Provisional export and lookup helpers

Hypothesis: if the first export is canonical to the new Rust domain model and explicitly provisional, then downstream experiments can start without freezing legacy DTO constraints too early.

Primary users:

- platform-context consumer
- future `v8-context` integration

Requirements:

- Serialize canonical JSON from the internal model.
- Include source provenance and localization fields.
- Add exact lookup helpers for name/type/member/constructor access.
- Keep any legacy-shaped export as an adapter only after a concrete consumer requires it.

Acceptance:

- `syntax-helper --output target/context/ru` and `syntax-helper --output target/context/en` write documented JSON files for `shcntx_ru.hbk` and `shcntx_root.hbk`.
- Shape tests cover core record kinds.
- Export documentation marks compatibility as provisional.

Out of scope:

- Search index.
- MCP server.
- Long-term schema compatibility promise.

### Epic E6. Real-platform Syntax Assistant acceptance and parser gap report

Hypothesis: if the project records extraction counts, parser warnings and unresolved pages for both `shcntx_ru.hbk` and `shcntx_root.hbk`, then the team can make an evidence-based decision about stable contracts and `v8-context` integration.

Primary users:

- parser maintainer
- future `v8-context` maintainer

Requirements:

- Run all CLI acceptance commands against both target files.
- Record counts by: global methods, global properties, types, type methods, type properties, constructors, enums and enum values.
- Record parser warnings and unresolved pages as follow-up tasks.
- Confirm root/default export as locale `en` and list any remaining localization merge decisions.

Acceptance:

- A checked-in acceptance report exists under `docs/` or `artifacts/`.
- Known parser gaps are linked to actionable backlog items.
- Stable-contract decision points are listed before integration into `v8-context`.

### Epic E7. All-HBK smoke coverage

Hypothesis: if the project runs a final broad smoke pass across every target-platform HBK file, then generic container/book regressions and unsupported book variants become visible without forcing every early implementation task to handle all files immediately.

Primary users:

- library consumer
- parser maintainer

Requirements:

- Enumerate every `*.hbk` file under `/opt/1cv8/x86_64/8.5.1.1150/`.
- Run generic container/book/TOC smoke checks without Syntax Assistant extraction.
- Record per-file results and unsupported structures.

Acceptance:

- Checked-in smoke report records file count, command summary, successes and failures.
- Any relevant unsupported structure is linked to a follow-up task.

### Epic E8. `v8-context` integration decision

Hypothesis: if integration is decided only after Syntax Assistant acceptance and all-HBK smoke evidence, then `v8-context` does not inherit provisional HBK assumptions too early.

Primary users:

- future `v8-context` maintainer
- platform-context consumer

Requirements:

- Inspect current `/home/alko/develop/open-source/v8-context` model and decision artifacts.
- Compare accepted HBK export data with existing context-source contracts.
- Decide whether this crate remains standalone, becomes a workspace member, or exposes a file-level integration artifact first.

Acceptance:

- Decision artifact exists and references T9/T10 evidence.

## 12. Implementation Task Set

### T0. Baseline repository shape

Depends on: none.

Tasks:

- Keep `README.md` aligned with the current baseline files and reference projects.
- Keep this document as the planning source of truth until a later ADR/spec split is needed.
- Add a minimal `cargo test` baseline.

Verification:

- `cargo test`
- `git diff --check`

### T1. Container reader and inspect command

Depends on: T0.

Tasks:

- Add library crate modules under `src/lib.rs`.
- Implement typed container errors with source path/entity context.
- Implement header/descriptor/block parsing.
- Implement entity enumeration and byte reads.
- Add `inspect` CLI through `clap`.
- Add unit fixtures for binary parsing.
- Add real-file smoke checks for `fmtdui_root.hbk` and `fmtdui_ru.hbk`, ignored by default or gated by an explicit environment variable.

Verification:

- `cargo test`
- If the small smoke files exist: `cargo run -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk`
- If the small smoke files exist: `cargo run -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`
- If the files are absent: document that the real-platform smoke was skipped because the platform fixture is unavailable.
- `git diff --check`

### T2. Book, ZIP storage and TOC reader

Depends on: T1.

Tasks:

- Implement `HbkBook` on top of `HbkContainer`.
- Inflate `PackBlock`.
- Open `FileStorage` as ZIP.
- Parse `Book` metadata.
- Implement locale inference.
- Implement TOC tree and lookup APIs.
- Add `toc` and `page` CLI commands.
- Add committed deterministic known-page path fixtures for `fmtdui_root.hbk` and `fmtdui_ru.hbk` so page smoke verification is reproducible.

Verification:

- `cargo test`
- If `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists: `cargo run -- toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json`
- If `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists: `cargo run -- page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --path "<committed-known-ru-page>"`
- If `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk` exists: `cargo run -- page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk --path "<committed-known-root-page>"`
- If the files are absent: document that real-platform TOC/page smoke was skipped because the platform fixture is unavailable.
- `git diff --check`

### T3. Documentation page parser

Depends on: T2.

Tasks:

- Implement HTML loading and parsing abstraction.
- Extract page title and normalized text preview.
- Implement deterministic link normalization.
- Add diagnostics for unresolved links.
- Add fixture tests for representative pages.

Verification:

- `cargo test`
- Fixture snapshot tests for normalized page text and links.
- `git diff --check`

### T4. Syntax Assistant fixture corpus

Depends on: T2, T3.

Tasks:

- Inspect representative pages from `shcntx_ru.hbk` and `shcntx_root.hbk`.
- Select the minimal committed fixture set for root/catalog pages and every specialized parser kind.
- Add a fixture manifest with source HBK file, HTML path, page title, parser kind and reason for inclusion.
- Copy only minimal real HTML fragments needed for parser behavior tests.

Verification:

- `cargo test`
- Fixture manifest covers global context, global method, global property, object/type, object method, object property, constructor, enum, enum value and root/catalog pages.
- `git diff --check`

### T5. Syntax Assistant root discovery

Depends on: T4.

Tasks:

- Implement root section detection for global context, enum catalog and type/object catalog.
- Implement catalog traversal before specialized parsing.
- Add diagnostics for unknown page classes.
- Add fixture coverage for root/catalog pages.

Verification:

- `cargo test`
- Stable automated assertion that discovered root sections for `shcntx_ru.hbk` include candidates for global context, enum catalog and type/object catalog.
- If the file is absent: document that real-platform root discovery smoke was skipped because the platform fixture is unavailable.
- `git diff --check`

### T6. Specialized Syntax Assistant parsers

Depends on: T5.

Tasks:

- Implement object/type parser.
- Implement method parser.
- Implement property parser.
- Implement constructor parser.
- Implement enum parser.
- Implement enum value parser.
- Implement global context parser.
- Add fixtures for every parser kind.

Verification:

- `cargo test`
- Known representative assertions pass for object/type, method, property, constructor, enum, enum-value and global-context parsers.
- Full in-memory extraction against `shcntx_ru.hbk` returns non-empty global methods, global properties, platform types and enums when the file exists.
- `git diff --check`

### T7. Domain model and canonical JSON export

Depends on: T6.

Tasks:

- Finalize provisional internal domain structs.
- Add `serde` serialization.
- Add source provenance fields to all exported records.
- Implement `syntax-helper --output`.
- Map `_root` source locale to export locale `en`.
- Document export file names and schema intent.

Verification:

- `cargo test`
- `cargo run -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/ru`
- `cargo run -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/context/en`
- Output files are non-empty and parse as JSON.
- `git diff --check`

### T8. Lookup helpers

Depends on: T7.

Tasks:

- Add exact lookup by global member name.
- Add exact lookup by type name.
- Add exact lookup by type/member name.
- Add constructor lookup by type name.
- Keep search ranking out of scope.

Verification:

- `cargo test`
- Unit tests for lookup ambiguity and missing items.
- `git diff --check`

### T9. Real-platform Syntax Assistant acceptance report

Depends on: T7, T8.

Tasks:

- Run acceptance commands against `shcntx_ru.hbk`.
- Run acceptance commands against `shcntx_root.hbk`.
- Record counts by: global methods, global properties, types, type methods, type properties, constructors, enums and enum values; record parser warnings.
- Record unresolved pages/links.
- Convert parser gaps into follow-up tasks.
- Confirm that `shcntx_root.hbk` exports as locale `en` and list remaining localization merge decisions.

Verification:

- Checked-in report with commands, exit codes, counts and gaps.
- `cargo test`
- `git diff --check`

### T10. All-HBK smoke report

Depends on: T9.

Tasks:

- Enumerate every `*.hbk` file under `/opt/1cv8/x86_64/8.5.1.1150/`.
- Run generic container/book/TOC smoke checks for every file.
- Record per-file successes, fatal failures and unsupported structures.
- Convert relevant unsupported structures into follow-up tasks.

Verification:

- Checked-in all-HBK smoke report with file count, commands, exit codes and per-file failures.
- `cargo test`
- `git diff --check`

### T11. Integration decision for `v8-context`

Depends on: T9, T10.

Tasks:

- Compare HBK export model with existing `v8-context` source model.
- Inspect current `/home/alko/develop/open-source/v8-context` model/decision artifacts before making the integration decision.
- Decide whether this crate remains standalone, becomes a workspace member, or exposes a file-level integration artifact first.
- Record the decision in an ADR or integration note before implementation.

Verification:

- Decision artifact exists and references T9/T10 evidence.
- `cargo test`
- `git diff --check`

### T12. Split implementation into reusable workspace crates

Depends on: T11.

Tasks:

- Convert the repository to a Cargo workspace without changing accepted CLI behavior.
- Split the current implementation into `hbk-container`, `hbk-book`, `hbk-docs`, `syntax-helper-model`, `syntax-helper-extract`, `hbk-export` and `v8-context-hbk-cli`.
- Preserve the installed binary name `v8-context-hbk`.
- Keep lower-level crates independent from higher-level concerns.
- Move behavior tests with the crate that owns the public behavior being tested.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- Package-level checks for every workspace crate.
- Real-platform CLI smokes through `cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- ...` when fixtures exist.
- Negative missing-file CLI smoke.
- `git diff --check`

## 13. Requirement Traceability

| Requirement area | Primary epic | Primary tasks | First verification |
| --- | --- | --- | --- |
| HBK container reader | E1 | T1 | `inspect fmtdui_root.hbk` and `inspect fmtdui_ru.hbk` list core entities |
| Book metadata and storage | E2 | T2 | `toc` and `page` commands load real book data |
| TOC navigation | E2 | T2 | TOC lookup tests and real-file smoke |
| HTML page model | E3 | T3 | Fixture snapshot tests |
| Syntax Assistant fixture corpus | E4 | T4 | Fixture manifest covers parser kinds |
| Syntax Assistant extraction | E4 | T5, T6 | Parser fixture tests and non-empty real extraction |
| JSON export | E5 | T7 | `syntax-helper --output` writes valid JSON |
| Lookup helpers | E5 | T8 | Exact lookup unit tests |
| Real-platform Syntax Assistant acceptance | E6 | T9 | Checked-in acceptance report |
| All-HBK smoke | E7 | T10 | Checked-in all-HBK smoke report |
| `v8-context` integration decision | E8 | T11 | ADR/integration note |
| Reusable crate boundaries | E8 | T12 | `cargo test --workspace` and package-level checks |

## 14. Success Metrics

The project is successful for the first delivery when:

- Reader correctness: the small real HBK smoke pair opens and exposes expected core entities.
- Generic coverage: the final all-HBK smoke report covers every target-platform `*.hbk` file.
- Extraction coverage: real `shcntx_ru.hbk` and `shcntx_root.hbk` extraction returns non-empty records for all top-level model families, with `_root` exported as locale `en`.
- Parser observability: parser warnings and unresolved pages are counted and source-linked.
- Test confidence: every specialized parser has at least one representative fixture.
- Consumer usability: downstream tooling can consume canonical JSON without reading HBK directly.
- Contract discipline: stable API/export commitments are deferred until after the real-platform acceptance report.

## 15. Risks and Mitigations

### 15.1. HBK binary format drift

Risk: 8.5 HBK files may differ from examples tested in `hbk-reader`.

Mitigation:

- Start M1 with small `8.5.1.1150` smoke files before using the large Syntax Assistant books.
- Keep block/header parsing strict but diagnostic.
- Record unsupported fields instead of ignoring suspicious values silently.

### 15.2. HTML shape drift

Risk: Syntax Assistant page markup may differ across platform versions.

Mitigation:

- Build parser fixtures from real 8.5 pages.
- Keep page-type parsers isolated.
- Return parse warnings/gaps with source refs.

### 15.3. Legacy DTO reuse

Risk: `platform-context-exporter` DTOs may be convenient to copy, but they can freeze old constraints and lose localization/provenance.

Mitigation:

- Use the richer internal model as the only authoritative model.
- Treat legacy DTO/export shapes as optional adapters for concrete consumers.
- Prefer new algorithms and schemas when they reduce complexity or improve correctness.

### 15.4. Mixing documentation and runtime context truth

Risk: HBK documentation data may be confused with runtime platform introspection.

Mitigation:

- Mark all extracted items with source `hbk`.
- Keep runtime extraction out of this project milestone.
- Later compare against runtime/static sources in separate artifacts.

## 16. Open Questions

1. Should the crate expose only a library plus tiny CLI, or should the CLI be a first-class supported tool?
2. Should `shcntx_root.hbk`/export-locale `en` and `shcntx_ru.hbk` be merged into bilingual records, or exported as separate localized views?
3. What is the canonical internal field naming for the new Rust/domain model: English field names with `ru_name`/`en_name`, or another explicit localization structure?
4. Should search/indexing live in this crate later, or remain in a separate consumer crate?
5. Which minimal real HTML pages from 8.5 may be committed as parser fixtures without overloading the repository?
6. Should the first acceptance report live under `docs/` for visibility or under `artifacts/` to match evidence/report conventions from `v8-context`?
7. Should ignored real-platform tests be enabled automatically when `/opt/1cv8/x86_64/8.5.1.1150/` exists, or only through an explicit environment flag?
8. Which unsupported structures found by the all-HBK smoke pass belong to this repository's supported scope?

## 17. Immediate Next Steps

1. Execute T0 first and do not start T1+ until T0 is completed per `spec/IMPLEMENTATION_TODO.md` loop rule.
2. After T0 completion, execute T1/M1 container reader with the small `fmtdui_*` smoke pair.
3. After T1 completion, execute T2/M2 book, TOC and page access using committed deterministic known-page paths for the small smoke pair.
4. Do not start `shcntx_*` extraction until the Syntax Assistant fixture corpus task is complete.
5. Keep export/API names provisional until T9 acceptance data exists.
