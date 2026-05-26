# Implementation Component Specification

Current status: the repository is a Cargo workspace split into the crates below. The split preserves
context boundaries and keeps CLI/export behavior provisional.

## Workspace Crates

1. `hbk-container`: binary container parsing, entity enumeration and entity byte access.
2. `hbk-book`: book metadata, locale inference, ZIP-backed `FileStorage`, TOC parsing and page reads.
3. `hbk-docs`: documentation HTML/page parsing, normalized text/link extraction and page diagnostics.
4. `hbk-book-export`: ordinary book-content export layouts and Markdown conversion adapters.
5. `hbk-doc-site`: documentation-site data generation, corpus discovery, global TOC merge, stable
   site ids and generated site data artifact layout.
6. `docs-web-app` (planned): separate documentation web application that serves the site UI and
   generated data artifacts; later owns search and Syntax Assistant API endpoints.
7. `syntax-helper-model`: provenance-rich platform context domain model and record sink boundary.
8. `syntax-helper-extract`: Syntax Assistant root discovery, catalog traversal and specialized page parsers.
9. `syntax-helper-language`: shared non-platform HBK language-fact model and fixture-backed parsers
   for `shlang_*`, `shquery_*` and `dcsui_*` pages.
10. `hbk-syntax-export`: canonical Syntax Assistant JSON export adapters.
11. `syntax-helper-search`: local SQLite/FTS5 index and query library for Syntax Assistant exact
   lookup, keyword/fuzzy search and bounded relationship traversal.
12. `context-resolver-core`: source-neutral Rust resolver API with typed identities, domains,
   fact kinds, response statuses, diagnostics and resolver/source traits.
13. `context-resolver-search`: HBK-backed platform and language-domain source adapters over
   `syntax-helper-search::SearchIndex`.
14. `v8-context-hbk-cli`: command wiring for the `v8-context-hbk` binary.

Search/query components are described in
[`syntax-helper-query-cli.md`](syntax-helper-query-cli.md).

Solution-context Rust resolution is described in
[`solution-context-resolve.md`](solution-context-resolve.md). ADR-0008 owns this boundary.

## Dependency Rules

- `hbk-container` must not depend on book, docs, extraction or export concerns.
- `hbk-book` must not depend on Syntax Assistant extraction.
- `hbk-docs` may depend on book-level page/TOC abstractions but must not know export schema details.
- `hbk-book-export` may depend on `hbk-book`, `hbk-docs`, the approved narrow
  HTML-to-Markdown conversion utility and narrow HTML escaping/entity utilities, owns ordinary
  book-content export layout/Markdown adapters and must not depend on Syntax Assistant extraction,
  `hbk-syntax-export` or CLI presentation code.
- `hbk-doc-site` may depend on `hbk-book`, `hbk-docs`, `hbk-book-export` and narrow serialization
  utilities needed for generated documentation-site data artifacts. It owns multi-book corpus
  discovery, global TOC merge, stable site ids, manifest/page artifact layout and web-app data
  contracts. It must not depend on frontend framework internals, resolver crates or web-server
  request handling. It may gain Syntax Assistant/search index generation only after a later spec
  task defines that generated artifact boundary.
- `docs-web-app` consumes generated site data artifacts. It must not depend on HBK container/book
  parsing crates or perform Syntax Assistant extraction in request paths. Later search and Syntax
  Assistant API endpoints must read generated/indexed data rather than parse HBK sources live.
- `syntax-helper-model` must not depend on HBK container, HTML parsing or CLI code.
- `syntax-helper-model` owns shared Syntax Assistant semantic identity helpers selected by
  ADR-0011. It may define identity construction over typed names, semantic context and source
  family evidence, but it must not read HBK files, parse HTML or depend on search/export crates.
- `syntax-helper-extract` owns traversal and parser behavior for Syntax Assistant pages. It may use
  narrow HTML parser/entity helpers for element text, anchors and entity decoding, but Syntax
  Assistant section-label boundaries and page-shape rules remain extractor-owned domain logic.
- `syntax-helper-language` owns the first shared language-fact model and source-family parsers for
  non-platform HBK language pages. It must not add language facts to `PlatformContext` or
  `syntax export` consumer JSON. Language callable fact assembly may be shared after
  source-family-specific page discovery and parsing have selected the callable name, syntax,
  parameters, return/type references, description and anchor; the shared helper must not absorb
  page-shape-specific parser rules.
- `hbk-syntax-export` owns Syntax Assistant output adapters for the Rust domain model. It consumes
  model-owned semantic identity/projection helpers and must not reimplement parent-owner identity
  rules locally.
- `syntax-helper-search` owns search-index schema, ranking and relationship traversal. It must not
  parse HBK files or perform CLI presentation. It may accept `syntax-helper-language` facts as
  pre-parsed documents for the T89 language-index fixture slice. Relation graph row construction is
  a single streaming internal builder reused by SQLite insertion and focused relation tests; tests
  must not carry a copied relation algorithm.
- `syntax-helper-search` builds search-specific document id strings from domain fact identity. It
  must not be the owner of Syntax Assistant parent identity rules. Parent records may derive their
  own identity from model helpers when a fixture or input record has no precomputed identity, but
  child/member records must arrive with `owner_identity` filled by `syntax-helper-extract`
  according to ADR-0011; missing child parent identity is an index-build error, not a search/export
  fallback.
- `context-resolver-core` owns the generic in-process resolver model. It must not depend on HBK,
  SQLite, CLI, parser or Syntax Assistant storage crates.
- `context-resolver-search` owns translation between `syntax-helper-search::SearchIndex` platform
  and language facts and the source-neutral resolver model. It must not expose SQLite tables or FTS
  fields, and it must not expose query-table provider facts as `PlatformApi` facts. Query-table
  provider facts are exposed only through the explicit `QueryLanguage` query-table source with
  resolver DTOs for templates, fields, parameters, type references and source-neutral evidence.
  Module-context
  lookup may consume private search-index module-context state, but the public contract is the
  resolver DTO shape: provider-neutral module context kind, source-qualified facts, signatures,
  availability diagnostics and exact-id lookup for resolved module context handles.
- `v8-context-hbk-cli` wires commands and error presentation only.
- `v8-context-hbk-cli` owns the current CLI provider JSON assembly boundary for `syntax get`,
  `syntax constructors`, `syntax search`, `syntax related`, `syntax related --graph` and
  `syntax type-ref-gaps`. Command handlers should stay responsible for argument classification,
  index-path resolution, query execution and text-versus-JSON dispatch, while private provider JSON
  helpers shape the versioned envelope, `results[].fact`, `results[].meta` and diagnostics. This
  boundary must translate `syntax-helper-search` DTOs into export-compatible provider facts instead
  of serializing internal search/model DTOs wholesale. SQLite schema details, FTS/search tokens,
  HBK provenance and downstream analyzer concepts must not leak through this layer.
- Syntax Assistant search/query code must not make `hbk-syntax-export` carry search-only fields in the
  lean consumer export. Use a search-specific index when structured links or provenance are required
  for query workflows.
- The future solution-context resolver core must be a thin source-neutral integration layer above
  platform/search crates. It must not live inside `syntax-helper-search` and must not force BSL
  language, query-language, configuration or source-code providers to depend on HBK or SQLite
  implementation details.

## Internal Module Decomposition Targets

Large `src/lib.rs` files should remain facade modules after decomposition. Splits are structural
only unless a separate requirement changes behavior or public contracts.

T151 implementation note: the first decomposition pass keeps existing crate root scopes intact and
moves implementation sections into responsibility-named files included by the facade. This preserves
the provisional root-level Rust API and private helper visibility while separating code by context
boundary. `context-resolver-core` and `syntax-helper-model` remain unsplit after evaluation because
their current source-neutral/model surfaces are smaller than the mandatory split targets and a file
split would not reduce coupling in this task.

- `syntax-helper-search` should split by internal index responsibilities:
  - public DTOs and facade exports;
  - index builder and extracted-record ingestion;
  - SQLite schema, metadata validation, read/write lifecycle and writer lock;
  - read-only query methods, exact lookups, keyword/fuzzy search and ranking;
  - relation graph construction/traversal;
  - type-reference storage, target resolution and gap reports;
  - platform type-template classification.
- `context-resolver-search` should split adapter families and shared mapping:
  - platform source adapter over `SearchIndex`;
  - BSL/query-language source adapters over `SearchIndex`;
  - BSL and SDBL global-context adapter support;
  - shared `SearchDocument` to resolver DTO mapping and relation/type-reference conversion.
- `syntax-helper-language` should keep one crate but split source-family parsers:
  - shared language-fact model and parser helpers;
  - `shlang_*` BSL facts;
  - `shquery_*` SDBL/query facts;
  - `dcsui_*` SKD/query-extension facts.
- `hbk-book-export` should split ordinary book-export phases:
  - request validation and export planning;
  - raw storage export;
  - Markdown rendering;
  - link-target rewriting;
  - HTML/code-example normalization;
  - filesystem writes and export error taxonomy.
- `hbk-doc-site` should split generated documentation-site phases:
  - source corpus discovery and source-book loading;
  - site data model and global TOC merge;
  - page/link-target planning;
  - artifact writing;
  - stable id, slug and hash helpers.
- `context-resolver-core` may be split after T146 if size keeps obscuring the public API:
  - ids/facts;
  - query/response DTOs;
  - resolver/source traits;
  - composite resolver orchestration.
- `syntax-helper-model` may be split by file for readability only:
  - root/catalog discovery DTOs;
  - identity helpers;
  - platform context records;
  - sink traits and diagnostics.

## Public Contract Policy

- Public contracts are provisional unless an ADR or requirement explicitly stabilizes them.
- Legacy-shaped DTOs or exports are adapters for concrete consumers, not constraints on the internal
  model.
- Before a planned rework, provisional legacy paths may be removed without compatibility fallback
  when no accepted ADR or requirement stabilizes them. Cleanup tasks must reference this policy,
  keep observable contract changes spec-first, and avoid adding replacement compatibility layers.
- Runtime 1C introspection is out of scope for this repository.
- Validation belongs at file/container input, external command input, parsing boundaries,
  serialization/export boundaries and public API boundaries.

## Static-Analysis Dependency Surface

ADR-0008 owns the in-process Rust surface for a downstream static-analysis project that consumes
this workspace as library dependencies. The selected integration shape is direct Cargo dependency
or workspace membership, not HTTP, daemon, MCP or CLI transport in the analyzer lookup hot path.

Recommended dependency layers:

- `context-resolver-core`: the source-neutral public surface for static-analysis code. Consumers
  should depend on its typed ids, fact DTOs, `ContextSource`, `ContextResolver`,
  `CompositeResolver`, `ResolveContext` and `ResolveResponse`.
- `context-resolver-search`: HBK-backed resolver source adapters. Downstream analyzer worker hot
  paths should compose explicit snapshot-backed sources such as `PlatformSnapshotSource` and
  `QueryTableSnapshotSource` over provider-owned `HbkFactSnapshot` / `HbkFactReadHandle` state.
  Broader non-query language facts require a dedicated snapshot-backed language source before they
  are part of the worker-safe analyzer surface. SQL/SearchIndex-backed constructors such as
  `PlatformSearchSource::open_read_only*` and `LanguageSearchSource::open_*_read_only` remain
  explicit local resolver, CLI, debug, index-inspection and sequential-use backends, not the
  downstream analyzer hot path.
- `syntax-helper-search`: provider index open/build primitives and provider-owned snapshot
  materialization/read-model APIs. The Rust API may be used to create or open the rebuildable
  provider index, or to materialize/load a provider-owned `HbkFactSnapshot`, but SQLite table names,
  FTS columns, row ids, schema internals and experimental binary-cache layout remain private
  implementation details.
- `hbk-book`, `syntax-helper-extract` and `syntax-helper-language`: setup/index-refresh phase only,
  when the embedding application chooses to rebuild HBK-backed provider indexes in process.

Static-analysis hot-path code should not depend on `v8-context-hbk-cli`, `hbk-syntax-export`,
`hbk-book-export`, `hbk-doc-site`, web-app code, Syntax Assistant page parser internals or
container/page provenance fields. Those components remain CLI/export/documentation or
setup-boundary concerns unless a later ADR creates a concrete adapter for them.

A broad facade crate is not selected yet. Add one only if a real downstream integration proves that
`context-resolver-core` plus concrete source-adapter crates creates avoidable coupling or repeated
boilerplate.

## Type Boundary Decision

T138 defers a separate workspace crate for type identities, type-reference resolution DTOs and
type-template binding DTOs. No new ADR is required because this does not change the accepted
workspace architecture; it records the current smallest ownership boundary for the existing
ADR-0008 and ADR-0011 contracts.

The deferred crate would be premature now because the current type concepts live at different
source-of-truth layers:

- `syntax-helper-model` owns Syntax Assistant domain facts before indexing/export. It owns raw
  source type-reference spelling, platform type-template keys, template binding DTOs that are
  derived from HBK facts, and shared semantic identity helpers. These values may be serialized by
  adapters, but the model crate must not depend on SQLite, resolver domains, CLI/provider JSON or
  downstream analyzer concepts.
- `syntax-helper-search` owns index-time resolution of those source-backed type references into
  an explicit target resolution result: `ok` with `target_type_id`, `unresolved` with no candidates
  or `ambiguous` with deterministic candidate ids. Type-template classification persistence and
  private rebuildable SQLite layout also belong here. Resolved target ids, ambiguous candidate
  reporting and quality-gap counters are index/provider state, not extraction-domain identity.
- `context-resolver-core` owns source-neutral resolver ids and DTOs for in-process static-analysis
  integration: `FactId`, `TypeId`, resolver `TypeRef`, `PlatformTypeTemplateKey`,
  `TypeTemplateBinding`, response statuses and domain separation. These DTOs must remain independent
  of HBK, Syntax Assistant parser records, SQLite tables and CLI provider JSON.
- `context-resolver-search` owns the adapter mapping between `syntax-helper-search` provider facts
  and `context-resolver-core` resolver DTOs. The mapping is intentional because the resolver adds
  source/domain identity around provider-local facts and hides provider storage details.

A future separate type crate may be reconsidered only after a concrete implementation task proves
that these mappings are repeated across more than the current search/resolver adapter boundary or
that multiple non-HBK providers need the same Rust type model. That future task must first specify
which layer owns raw spelling, resolved target identity, ambiguity/candidate data, template
bindings and domain-qualified resolver ids. The crate must still keep HBK parsing, SQLite storage,
CLI/provider JSON assembly and downstream analyzer logic out of its boundary.

## Pre-Rework Legacy Cleanup Boundary

Before the resolver and non-platform Syntax Assistant rework expands the implementation surface, the
project may remove provisional legacy paths that duplicate the accepted streaming export, streaming
indexing, provider JSON and boundary-normalization directions. These removals are intentionally
breaking when no accepted ADR or requirement has stabilized the old path.

Cleanup work must stay narrow:

- remove the legacy path instead of adding a compatibility shim or adapter;
- keep one cleanup concern per task and preserve unrelated CLI, JSON, parser and export behavior;
- update spec or UAT first when a cleanup changes observable behavior;
- keep downstream analyzer behavior out of scope unless an accepted resolver/provider task selects
  it explicitly;
- avoid broad clippy cleanup, dependency updates, storage knobs, caches or service boundaries.

The current cleanup sequence is limited to:

- removing duplicate in-memory search-index construction in favor of `SearchIndexBuilder` /
  `SyntaxHelperSink`;
- removing duplicate in-memory export APIs in favor of `StreamingSyntaxHelperExport`;
- collapsing duplicated `syntax get` dispatch and provider JSON adapter mapping;
- normalizing HBK/page path handling at the owning component boundaries;
- specifying and then removing query-table syntax fallback-to-name behavior;
- replacing in-memory type lookup scans with indexed SQL lookup where the query contract is already
  accepted;
- narrowing `syntax-helper-search` dependencies to actual production needs;
- deduplicating property usage normalization and leading type prose cleanup at the selected
  parser/export boundary;
- retiring legacy in-memory `PlatformContext` lookup helpers in favor of accepted SQLite/provider
  primitives and the future ADR-0008 resolver boundary.

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

`HbkContainer::from_bytes` is test support only. Production/library callers open HBK containers
from a path through `HbkContainer::open`; in-memory synthetic containers belong behind tests or the
`test-utils` feature, not in the ordinary public component contract.

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
`hbk-book` must not trust ZIP entry metadata sizes for unbounded allocation when reading `PackBlock`
or `FileStorage` entries; entry bytes are read from the actual ZIP stream at the HBK input boundary.
Book metadata and TOC text parsing use a narrow `winnow`-backed cursor over the original input
instead of pre-tokenizing into an owned token vector. The internal parser must preserve the accepted
HBK text grammar: BOM and commas are trivia outside strings, doubled quotes inside strings decode to
one quote, Book metadata keeps the existing field order/trailing-zero validation, and TOC parsing
still builds navigation from parsed chunk parent ids while normalizing storage paths.

The supported ordinary page/file surface is `read_file`, `read_page` and `FileStorageReader`.
`HbkBook::read_pages` is retired; deterministic repeated-page fixture checks should use
`FileStorageReader` directly. `FileStorageReader::file_paths` is the narrow raw-storage
enumeration surface for ordinary export: it lists non-directory stored FileStorage entry names
without applying TOC fallback filters or Syntax Assistant semantics. Export crates must still
validate those storage paths at their filesystem-write boundary before writing files.

### hbk-docs

Expected public concepts:

- `DocumentationReader`
- `PageContent`
- `ResolvedLink`
- `DocumentationError`

Owns FR-DOC-001.

### hbk-book-export

Expected public concepts:

- `BookExportRequest`
- `BookExportFormat`
- `BookExportHierarchy`
- `BookExporter`
- `BookExportError`

Owns FR-HBK-004.

`hbk-book-export` adapts ordinary HBK book content into output files. It owns output path planning,
safe output-root validation, raw `FileStorage` unpacking and Markdown conversion for TOC pages. It
must consume book/documentation abstractions from `hbk-book` and `hbk-docs` instead of reaching into
container internals or Syntax Assistant extraction state.

The first supported combinations are `format=raw` with `hierarchy=raw` and `format=markdown` with
`hierarchy=toc`. Unsupported format/hierarchy pairs return typed export errors for CLI presentation
rather than silently falling back to another layout.

T99 introduced the crate boundary and request model. `BookExportRequest` validates the output root
at the public export boundary: the root must contain at least one directory name and must not
contain `..` segments. The request model recognizes only the specified future-supported
combinations, `raw/raw` and `markdown/toc`; `raw/toc` and `markdown/raw` return a typed
unsupported-combination error.

T100 implemented `format=raw` with `hierarchy=raw` inside `BookExporter::export`. Raw export
enumerates non-directory FileStorage entries through `hbk-book`, validates all storage paths before
any filesystem writes, rejects parent segments, absolute/rooted paths, Windows drive-like paths,
backslash separators, empty paths, duplicate normalized output paths and file/directory prefix
collisions, then writes the original stored bytes under normalized relative paths.
`BookExporter::export` also validates that the request source path matches the opened `HbkBook` path
so callers cannot accidentally export bytes from a different book than the request names. Markdown
conversion and CLI wiring remain owned by later FR-HBK-004 tasks.

T101 wired the top-level CLI command
`v8-context-hbk export <book.hbk> --output <dir> --format <raw|markdown> --hierarchy <raw|toc>` to
`hbk-book-export`. The CLI maps only the ordinary book-content export path to `hbk-book-export`;
`syntax export` remains wired to `hbk-syntax-export` and the Syntax Assistant extraction/export
pipeline. The CLI validates unsupported format/hierarchy combinations through
its command boundary before opening the HBK source file, so unsupported matrix diagnostics are
stable and do not depend on source-file availability. `format=raw` with `hierarchy=raw` is the only
implemented top-level book export behavior after T101; Markdown/TOC export remains a later
FR-HBK-004 task.

T102 selected `quick_html2md` 0.2.1 as the approved stable HTML-to-Markdown library candidate and
implemented `BookExporter::markdown_page()` for individual TOC pages. The converter consumes
`hbk-docs::DocumentationReader` page content, preserves visible headings, body text, link text,
lists, GFM tables and angle-bracket syntax placeholders, and normalizes output so regular Markdown
does not contain raw HBK file paths, raw TOC indexes, raw HTML page paths or service HTML
scaffolding. Internal link and image targets are intentionally not emitted until T103 defines the
deterministic Markdown/TOC file layout and link-target mapping. At the end of T102, the top-level
CLI still rejected `markdown/toc` before opening the source HBK; full Markdown layout and UAT wiring
remained T103 scope.

T103 defines the Markdown/TOC layout as a TOC-title-derived directory tree. Each TOC item maps to a
directory segment derived from its displayed localized title, and each exported page is written as
`index.md` inside that page directory. Sibling title collisions are disambiguated by a stable
TOC-order suffix such as `-2`, without using raw TOC indexes or raw HTML paths as public output
names. TOC items without an HTML path, or with a TOC HTML path that is absent from `FileStorage`,
still produce a heading-only Markdown page so the layout remains TOC-ordered and deterministic.

For full `markdown/toc` export, same-book internal links whose normalized target resolves to an
exported TOC page are rewritten to the corresponding relative Markdown `index.md` path. External
links remain as external Markdown links. Cross-book `v8help://` links, unresolved internal links,
links to non-exported storage entries and image targets do not emit raw HBK storage paths; their
visible text remains readable where the converter can preserve it. Binary resource export is not
part of T103.

T103 implements that `markdown/toc` layout in `hbk-book-export` and wires the top-level CLI to allow
`--format markdown --hierarchy toc`. The CLI still rejects `raw/toc` and `markdown/raw` before
opening the source HBK. UAT-HBK-004 through UAT-HBK-007 passed on the local 8.5.1.1150 corpus.

T104 treats shared service content-node placeholder paths, such as `_CONTENTS_NODE_fileConf`, as
TOC section markers rather than ordinary documentation pages. Markdown/TOC export writes those TOC
items as heading-only Markdown using each item's own TOC title, because loading the shared
placeholder through the normal documentation reader can borrow the first TOC title that uses the
same storage path. Real HTML pages, missing-page heading-only behavior and unsupported export
combinations remain unchanged.

T105 keeps `quick_html2md` as the normal Markdown converter but adds a narrow pre-conversion
normalization for HBK code-example tables. A table is treated as a code example only when it has one
data/header cell and the source HTML marks it with Courier-family font content. Such tables are
rewritten to `<pre><code class="language-bsl">` before conversion so line breaks and query `|`
markers remain readable in `bsl` fenced code blocks. Multi-cell documentation tables, including DCS
keyword tables, continue through the normal GFM table path.

T107 keeps Markdown link-target lookup path-based, but preserves source HTML `#fragment` suffixes
when composing the final Markdown href. Same-page anchors such as `href="#FieldsRecords"` therefore
become `index.md#FieldsRecords`, and same-book links with fragments append the fragment to the
relative exported Markdown target. Fragments are not added to the TOC lookup map.

T108 extends the pre-conversion normalization with a separate query-language path for Courier
blockquotes. A blockquote is treated as an SDBL query example only when it has Courier-family font
content and no links. Such blockquotes are rewritten to `<pre><code class="language-sdbl">` before
conversion. Navigation blockquotes with links remain regular quoted/link content.

T109 keeps the T107 link-fragment href behavior but also materializes source heading anchors in the
generated Markdown. Heading-local HTML anchors, such as `<a name="Manager"></a>` inside an `h2`, are
exported as explicit Markdown-compatible HTML anchor targets immediately before the corresponding
Markdown heading. This is an export adapter concern: the internal TOC/link lookup remains
path-based, and generated anchor tags are limited to anchors owned by source headings.

### hbk-doc-site

Expected public concepts:

- `SiteGenerationRequest`
- `DocSiteGenerator`
- `SiteGenerationResult`
- `SiteGenerationError`
- `SiteTocNode`
- `SitePageId`
- `SiteTocNodeId`
- `SiteBookId`

Owns FR-HBK-005 and NFR-SITE-001.

`hbk-doc-site` is the generator component selected by ADR-0010. It adapts a corpus of HBK books into
documentation-site data artifacts. T111 implements the first crate boundary as
`SiteGenerationRequest`, `DocSiteGenerator`, `SiteGenerationResult`, `SiteGenerationError`,
`SiteTocNode`, `SitePageId`, `SiteTocNodeId` and `SiteBookId`.

The implemented T111 artifact slice writes:

- `data/manifest.json`;
- `data/locales/<locale>/toc-root.json`;
- `data/locales/<locale>/toc-sections/<section-id>.json`.

The T111 manifest includes schema version, generator name/version, deterministic build id, locales,
source book inventory with `book_id`, file name, title, locale and file size, root TOC paths and
future page-root paths. Locale-derived artifact path segments are validated before writing.

T112 adds page Markdown files under `data/locales/<locale>/pages/<page-id>.md` for page-bearing
global TOC nodes and keeps those files addressed by stable generated `page_id` values. Markdown
conversion reuses the accepted single-book Markdown conversion behavior through `hbk-book-export`;
the site component owns global page identity, generated page-id link target planning, output path
planning and the generated data split.

The component must keep these responsibilities separate:

- source discovery and source book inventory;
- per-book HBK open/TOC access through `hbk-book`;
- global TOC merge and stable site identity;
- page content writing and link target planning in the later page-data slice;
- generated data artifact writing.

The first implementation should reuse the accepted Markdown conversion rules from `hbk-book-export`
where possible, but the site component owns global cross-book identity and global TOC link mapping.
If sharing the existing Markdown conversion requires moving helper functions, do that as a narrow
extraction without changing the single-book `export --format markdown --hierarchy toc` contract.

The generated site data contract is not a public stable protocol yet, but it must be versioned in
`manifest.json` so later schema changes are explicit. The web app consumes generated data files; it
must not parse HBK containers or invoke Syntax Assistant extraction in request paths.

Global TOC merge behavior is specified in
[`documentation-site.md`](documentation-site.md). The first implementation must preserve source book
identity on page-bearing nodes and must not silently collapse page-bearing nodes solely by title.

The first generator command shape is expected to be
`v8-context-hbk site generate <source-dir> --output <data-dir>`, with repeated
`--include <file-name>` filters for deterministic UAT and focused generation. T112 wires this
command through the CLI and prints source book count, locale count, TOC node count, page count,
generated file count, output bytes, elapsed milliseconds and peak RSS when available. User-facing
README details may follow the accepted command behavior.

### docs-web-app

Owns the documentation web application boundary selected by ADR-0010.

The first site slice needs only navigation and page viewing:

- serve or load `manifest.json`, locale TOC data and page Markdown artifacts generated by
  `hbk-doc-site`;
- render locale selection, global TOC, lazy section loading and Markdown page content;
- avoid embedding all generated page Markdown into the initial bundle or server response;
- keep search, Syntax Assistant API endpoints, indexing status and backend compatibility out of the
  first slice.

Later web-app slices may add search and Syntax Assistant API endpoints, but those endpoints must use
generated/indexed artifacts. They must not parse HBK files or run extraction pipelines in request
paths.

T113 implements this first boundary as `web/docs-viewer`, a dependency-free Node/static web app with
a small static server. The server accepts `--data <dir>` and `--listen <host:port>`, serves generated
data under `/data/*`, confines paths to the provided data root and serves the static viewer from the
production build output. The browser code consumes only generated data artifacts and keeps page
Markdown out of the initial bundle.

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
- `QueryTable`
- `QueryTableField`
- `QueryTableParameter`
- `Constructor`
- `EnumDefinition`
- `EnumValue`
- `Signature`
- `Parameter`
- `TypeRef`
- `SourceRef`

Owns the domain model used by FR-SH-002 and FR-EXPORT-001.

The model remains provenance-rich for diagnostics and parser maintenance. Consumer export shape is
owned by `hbk-syntax-export` and may intentionally omit internal provenance and navigation
scaffolding.

`PlatformContext` is a provenance-rich in-memory aggregate and sink for parser/tests that need the
full domain model. It no longer owns public exact lookup helpers; accepted interactive lookup
behavior belongs to `syntax-helper-search` provider primitives, and the future in-process API
belongs to the ADR-0008 source-neutral resolver boundary.

`SyntaxHelperSink` is the shared record-family boundary used by the in-memory `PlatformContext`
aggregate and by streaming export/index adapters. It must stay typed by domain record families
rather than becoming a generic pipeline abstraction. A sink may request a narrower
`SyntaxHelperRecordDetailMode` only to avoid building fields that its concrete consumer omits; the
default mode remains the full provenance-rich domain model.

### syntax-helper-extract

Expected public concept:

- `SyntaxHelperReader`

Owns FR-SH-001, FR-SH-002 and FR-SH-003.

The supported extraction facade is `SyntaxHelperReader::extract_into()` over `SyntaxHelperSink`.
Materializing helpers (`SyntaxHelperReader::extract`, `extract_with_loader`,
`extract_with_loader_into`) and root-discovery loader helpers are internal/test-support surfaces.
Specialized page-parser functions are parser internals, not crate-root public reexports. The crate
root intentionally exposes only `SyntaxHelperReader` and error types; domain types should be
imported from `syntax-helper-model`.

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
global-context method-like pages as recoverable diagnostics. T30 resolves query/table owner names
through one extraction-scope TOC HTML-path index per run instead of repeatedly flattening the TOC for
each table field and table parameter. T31 remeasured residual parser overhead after T30 and did not
justify changing parser-helper behavior.

Syntax Assistant reading must derive source family and semantic ownership from the TOC ancestor
chain before parsing a page into a domain record. HTML path patterns remain evidence, but
`syntax-helper-extract` must not classify or own records solely by suffix checks when the same path
shape appears under multiple Syntax Assistant branches. Query-language/SDBL table fields and
parameters are owned by TOC-derived query table context, not by a path-stripped table title alone.
Same-name global context events, event-like platform type/object pages and placeholder-like records
remain distinct until an explicit source-family merge rule says otherwise. ADR-0005 owns this
reading boundary.

For query table records, page parsers own page-local syntax and description only. The extraction
reader owns TOC-derived semantic context, display name and the derived query table `identifier` /
`table_role` assignment. Missing or empty syntax keeps the T75 contract: no synthesized consumer
identifier and `table_role=unknown`.

The extractor must model TOC classification in two layers: branch kind and record family. Branches
such as Automation/external API are category context for ordinary platform types, while module
event groups produce `module_event` records. Platform type records must be able to distinguish at
least regular, extension, primitive and metadata-template types. Primitive type traversal is
shallow: direct children of `Примитивные типы` are primitive platform types, but nested literal
pages are not ordinary platform types.

### hbk-syntax-export

Expected public concepts:

- `JsonExporter`
- `StreamingSyntaxHelperExport`
- lean consumer export DTOs derived from the provenance-rich domain model
- optional separate diagnostic/debug adapters when a concrete maintenance workflow requires them

Owns FR-EXPORT-001.

The streaming export adapter consumes the `SyntaxHelperSink` boundary and writes canonical
record-family JSON without retaining the full `PlatformContext`. The previous in-memory
`PlatformContext` exporter was provisional and is removed; repo-local exports and tests use the
streaming `SyntaxHelperSink` path as the canonical writer. Streaming export may use the lean sink
detail mode to skip consumer-omitted navigation fields, but omission from JSON remains an
`hbk-syntax-export` adapter concern rather than an internal model constraint.

Schema version 7 record-family JSON exposes structured `availability`, `examples`, `see_also`,
signature variant metadata, enum values, type-reference facts and TOC-derived semantic identity
fields from the domain model while still omitting source HBK paths, TOC paths, HTML paths, page
titles and duplicate navigation-link catalogs from consumer records. The export adapter owns
consumer-shape simplification: `owner`, `types`, `return`, `availability.since`, `see_also`,
property `usage`, signature metadata, nested enum values, `record_family`, `module`, `owner_path`
and `type_kind` are serialized in the lean FR-EXPORT-001 form without forcing the internal model to
discard richer provenance or localized names. It also includes `global-context-events.json`,
`table-fields.json` and `table-parameters.json`; the global-context-events adapter filename is kept
for compatibility while its records represent the `module_event` family.

Schema version 8 changes the query table export from separate field/parameter record families to
`query-tables.json`. `syntax-helper-model` should represent query tables as typed owners with string
names, `owner_path`, `table_role`, optional description, fields and parameters. Query table names,
field names and parameter names should be strings instead of `LocalizedName` unless real source
evidence later proves aliases for this family. Query table parameters should not carry a `required`
flag unless a reliable source contract is found.

For schema version 8, `hbk-syntax-export` must emit `owner_path` only on records that represent semantic
owner context: platform types, module-event module context and query table records. It must not emit
`owner_path` on derivative type methods, type properties, constructors or nested query table
fields/parameters. `metadata.json.files` is the authoritative inventory for the current schema; the
exporter writes current files but must not delete stale files from older schemas in a reused output
directory.

Schema version 9 stops using the historical `global-context-events.json` filename for event facts.
The split is `module-events.json`, `type-events.json` and `unknown-events.json`; `hbk-syntax-export` routes
records by source-backed event classification without adding global semantic IDs. Type events carry
`owner` as a single semantic owner string, while module events carry `module`. Any owner/object kind
needed by events belongs on the owner type/object model, not as a duplicated event-only taxonomy.
The split preserves the schema version 8 rule that derivative records do not emit `owner_path`.

T38 adds optional `object_kind` to `platform-types.json` only. `syntax-helper-extract` derives it
from TOC-backed platform type context after `branch_kind` and `type_kind` are known; `hbk-syntax-export`
passes it through when present. Event files do not expose `object_kind`, `owner_kind`, `id` or
`owner_ref`, and derivative type members, constructors and nested query table records keep the
schema version 8 `owner_path` omission rule.

Schema version 10 removes semantic `owner_path` from `type-events.json`. `syntax-helper-model` owns
the shared type-event owner projection from TOC-derived semantic context: remove a trailing generic
event-group label such as `События` / `Events`, then compose the remaining localized semantic owner
chain into the single owner string used by JSON export and search identity. `hbk-syntax-export`
emits this owner in `type-events.json`; `syntax-helper-search` may prefix it for search document
ids, but must not reimplement type-event owner classification locally.

Schema version 11 adds localized query table `syntax` and `identifier` to `query-tables.json`. The
extractor parses the `Синтаксис` / `Syntax` section on query table pages, splits parenthesized
source aliases into `syntax.alias`, and derives `table_role` from the `syntax.primary` shape when
syntax is present. A primary syntax with at most two dot-separated segments is a primary table; a
longer primary syntax is an additional table. Query table identifiers are
query-table-local consumer keys: primary tables use the first primary syntax segment, while
additional tables use the primary identifier plus the table `name` normalized to CamelCase with
whitespace, hyphens and punctuation treated as word separators. This does not introduce a
cross-family semantic ID or cross-file reference model.

The T74/T75 cleanup removes the provisional fallback that derived query-table `identifier` and
`table_role` from the display page name when syntax was missing or empty. After that cleanup,
missing or empty table syntax is observable as an omitted `syntax`, omitted `identifier`,
`table_role="unknown"` and a parser-maintenance diagnostic with source provenance. The extractor
must not use generic names such as `Основная таблица` / `Main table` or any other table title as a
replacement syntax source or consumer identifier fallback, and no compatibility adapter should
restore that old behavior. Internally, the domain model represents the missing query-table
identifier as typed absence, not as an empty-string sentinel; search/index identity may still derive
deterministic internal document ids from TOC-derived semantic owner context when syntax is missing.

### v8-context-hbk-cli

Owns FR-CLI-001.

The installed binary name remains `v8-context-hbk`. Accepted inspection/navigation command names are
`inspect`, `toc` and `page`. Ordinary single-book content export is the top-level `export` command.
Documentation-site data generation is the `site generate` command group; it belongs to the
generator boundary and supports repeated `--include <file-name>` filters for deterministic source
selection. The target Syntax Assistant command group for export/index/query work is `syntax`.

### Syntax Assistant query commands

Owns FR-SH-SEARCH-001 and FR-SH-SEARCH-002 after implementation.

The `v8-context-hbk syntax` query commands must read a prebuilt search index artifact for
interactive commands. They must not parse `shcntx_*.hbk` in exact lookup, text search, fuzzy search
or relationship search commands. Index build commands may parse Syntax Assistant HBK sources through
the extraction pipeline and must pass typed extracted facts into the search/index library rather
than building from consumer JSON export directories.

Implemented first slice:

- `syntax-helper-search` owns `index.sqlite` schema version `15`, read-only query opens, FTS5 keyword
  search, prefix-bounded fuzzy candidate selection, exact name/alias and owner/member lookup, and
  directed owner/type-reference relationship traversal.
- `SearchHit`, `SearchDocument`, `RelatedHit` and `RelationStep` are Rust query result structs for
  in-workspace search/resolver adapters, not the public provider JSON contract. They intentionally
  do not derive serde serialization; `v8-context-hbk-cli` assembles provider JSON explicitly from
  normalized index facts and export-compatible field shapes.
- `v8-context-hbk syntax export/index/get/search/related` owns CLI argument parsing, index path
  resolution and text/JSON presentation.
- `syntax index` builds a replacement index beside the target file and atomically renames it after
  validation. Concurrent writers are serialized by a lock file.
- `syntax index` feeds extraction records into a search-index builder through
  `SyntaxHelperReader::extract_into()`. The builder keeps only search-index drafts and identity
  inputs, then writes documents and streams relation inserts into SQLite. The build path does not
  retain a full `PlatformContext`, complete search-document vector and complete relation vector at
  the same time.
- ADR-0011 requires Syntax Assistant parent fact identities to be computed in the read phase and
  stored on extracted domain records before they reach `SyntaxHelperSink`. The search builder may
  still normalize duplicate final document ids as an index-build recovery step, but it must not
  derive parent ownership by reinterpreting TOC labels independently from the model/extractor.
- `syntax index` bulk-loads FTS input into an ordinary `document_search` content table and rebuilds
  the external-content `document_fts` table before validating and atomically replacing the target
  database. The index remains one SQLite artifact.
- `syntax index` stores analyzer-critical facts in normalized relational tables:
  `type_identities`, `members`, `callables`, `signatures`, `parameters` and `type_refs`. Provider
  JSON is assembled from those rows; `documents.signature_json` and `documents.preview` are no
  longer part of the SQLite schema.
- Schema version `5` adds the internal `type_identities(document_id)` lookup index used by provider
  type identity resolution. Older schema version `4` indexes are rebuildable service data and are
  rejected by read-only query opens with a rebuild instruction.
- Schema version `6` adds internal document/owner indexes for exact owner-type member and callable
  lookup. Older schema version `5` indexes are rebuildable service data and are rejected by
  read-only query opens with a rebuild instruction.
- T89 adds language-fact document kinds to the existing SQLite document/search projection without a
  schema-version change: `language_type`, `language_construct`, `language_function`,
  `language_operator`, `language_keyword` and `language_literal`. These documents are indexed by
  source-qualified ids such as `shlang:def_String`, `shquery:STRING` and
  `dcsui:SKD_Functions_Strings#StringLength`.
- T90 keeps the same schema version and makes language facts resolver-usable by preserving
  extracted language `type_refs` / `return_types`, normalizing `language_function` signatures and
  parameters as callable rows, and deriving relation rows from explicit extracted type references.
  This does not create a public SQLite table contract.
- T131-T134 raised the private rebuildable search-index schema through versions `8`, `9`, `10`,
  `11` and `12` to store metadata-template facts, open type-template family/variant keys,
  persisted type-template classification evidence and multi-argument owner-parameter template
  bindings. Consumers must use the Rust search/resolver APIs for those facts; SQLite tables and
  columns remain private provider state and older indexes are rejected with rebuild instructions.
- T139 raises the private rebuildable search-index schema to version `13` so each normalized
  `type_refs` row stores source spelling separately from target resolution status and ambiguous
  candidate ids. Provider JSON keeps export-compatible `types` / `return` name arrays, while Rust
  resolver DTOs expose target resolution as data instead of `Option<TypeId>`.
- T152 raises the private rebuildable search-index schema to version `14` so module-event documents
  preserve provider-neutral module context relation keys for resolver module-context lookup.
- T162 raises the private rebuildable search-index schema to version `15` so source-backed enum
  definition documents are stored in `type_identities` and participate as provider-owned type-like
  targets for normalized `type_refs`. Enum targets keep their existing `enum:system:*` or
  `enum:metadata_property:*` identities; they are not converted to `platform_type:*`, and duplicate
  enum matches remain explicit `ambiguous` rows.
- T163 keeps schema version `15` and narrows search-index build optimization to non-observable
  allocation/data-structure choices: newline-joined storage fields and searchable text avoid
  intermediate vectors, per-document name-key dedup uses a tiny vector instead of a `BTreeSet`, and
  relation-build membership/dedup uses hash sets where output order is still driven by document
  traversal. Stable document sorting is deliberately preserved because duplicate-id recovery keeps
  the last source document.

T87 classifies the remaining duplicate-looking query/provider mechanisms as boundary decisions
rather than immediate cleanup work:

- `syntax get` root classification, lookup execution and provider status/result mapping are accepted
  CLI-boundary separation. The classifier owns provider `query` JSON and lookup variant selection,
  lookup execution owns `SearchIndex` calls and provider status/result mapping owns envelope
  presentation. T71 already collapsed the stale duplicated classifier/lookup tuple matching.
- `syntax-helper-search` lookup-key normalization is accepted search-index boundary behavior.
  Exact name, owner/member, fuzzy candidate and relation-key lookups normalize through the local
  search/index rules; this remains separate from export DTO shaping and from documentation/storage
  path normalization.
- Provider JSON DTO shaping is accepted CLI-boundary behavior. `SearchHit`, `SearchDocument`,
  `RelatedHit` and `RelationStep` are Rust query result structs for in-workspace search/resolver
  adapters, while `v8-context-hbk-cli` assembles the public provider envelope and fact JSON from
  normalized facts using export-compatible field names. T72 removed the stale full/compact provider
  adapter duplication, and T86 removed serde serialization from the search result structs.
- T91 collapsed the stale duplicated localized-name `display_name` helper into
  `syntax-helper-model::LocalizedName`. The shared helper is presentation logic only, not a public
  identity, lookup or JSON contract mechanism. `syntax-helper-search` and `v8-context-hbk-cli` now
  call that helper without changing search ranking, relation labels, text output or provider JSON.
- T95 replaced raw in-workspace search document kind strings with
  `syntax-helper-search::SearchDocumentKind`. The typed kind is the Rust search/resolver adapter
  boundary only; SQLite `documents.kind` values, provider `kind` values, search ordering, resolver
  fact mapping and consumer export JSON remain unchanged. `v8-context-hbk-cli` still assembles
  provider JSON explicitly from the existing string values, and the platform resolver adapter still
  hides `query_table*` provider documents.
- HBK storage/page path normalization, documentation link-target normalization and Syntax Assistant
  member-link normalization are accepted distinct component boundaries. T73 consolidated shared
  storage/page path handling in `hbk-book`, while retaining separate documentation-link and
  member-link rules because they resolve different source syntaxes.

### Solution Context resolver

Owns FR-CTX-RESOLVE-001 and NFR-RESOLVE-001.

Expected source-neutral public concepts:

- `ContextResolver`
- `ContextSource`
- `ResolveContext`
- `ResolveQuery`
- `ResolveResponse`
- `ResolveStatus`
- `ResolveDiagnostic`
- `SourceId`
- `FactId`
- `TypeId`
- `MemberId`
- `CallableId`
- `LanguageDomain`
- `FactKind`
- `ContextFact`
- `FactRelation`
- `ModuleContextKind`
- `ModuleContextQuery`
- `ResolvedModuleContext`

The resolver core is implemented as `context-resolver-core`, a separate crate with no HBK, SQLite,
CLI or parser dependencies. The platform and first language-domain adapters are implemented in
`context-resolver-search`, a sibling adapter crate over `SearchIndex`; `syntax-helper-search`
remains the HBK/Syntax Assistant query implementation and not the generic cross-domain resolver
model.

The first resolver API must keep BSL language types and query-language types separate from platform
API types. Cross-domain links require explicit relations; same-name facts across domains or sources
must not be silently merged.

Domain separation is the resolver boundary, not a naming convention. `shcntx_*` platform API facts,
`shlang_*` BSL-language facts, `shquery_*` query-language facts, `dcsui_*` data-composition
language facts, downstream configuration metadata facts and downstream source-code declarations are
different source/domain identities even when they share a display name. The composite resolver may
use deterministic source order for candidate ordering only; omitted source, domain, kind or owner
constraints that leave multiple candidates must produce ambiguity rather than a hidden winner.

The platform adapter over `syntax-helper-search` exposes platform API type, member and callable
facts, global context and provider-backed module context facts only. Existing query-table documents
in the search index remain outside that adapter and are exposed through a distinct QueryLanguage
query-table source.

For module context, HBK owns platform facts that come from Syntax Assistant evidence: platform
global methods/properties, module events, event signatures and availability. Downstream
`v8-context-metadata` owns raw metadata module/form facts such as concrete form attributes, form
elements, module ownership and generated configuration types. `v8-context` composes both providers
and must not read private HBK/search-index storage or maintain analyzer-side fallback lists for
platform module facts.

The first language-domain adapter slice exposes T89 language facts through source-specific
`shlang`, `shquery` and `dcsui` resolver sources. `shlang` maps to `BslLanguage`; `shquery` and
`dcsui` map to `QueryLanguage` while keeping distinct source ids. The adapter resolves exact ids and
exact names for language types, query/SKD functions, keywords, operators, constructs and literals
using the existing resolver fact kinds without adding new core model variants. `language_function`
facts are exposed as callable facts with ordered signatures and parameters; `language_type` and the
current `language_literal` facts are exposed through type lookup. Relation traversal uses only
explicit index edges derived from extracted language type references, for example
`dcsui:SKD_Functions_Strings#StringLength` parameter type `Строка` to `shlang:def_String`.

The QueryLanguage query-table source exposes `shcntx_*` query table template/family documents as
`QueryTable`, `QueryField` and `QueryParameter` facts. It preserves template identity, syntax,
identifier, table role, owner semantic path, source-derived template parameter slots, owned
field/parameter identities, field/parameter type references and source-neutral evidence. It does not
instantiate concrete metadata query tables and does not make query tables platform API members.
The source-neutral resolver DTOs identify evidence by resolver source id, evidence id and locale;
raw HBK, TOC and HTML parser provenance stays inside the search adapter/index layer.

### Provider-Owned HBK Fact Snapshot

The next resolver implementation layer is a provider-owned immutable HBK fact snapshot. The snapshot
belongs with the provider/index boundary, not with downstream analyzers:

- `syntax-helper-search` owns SQLite schema knowledge and the first bulk materializer;
- compact snapshot nodes own platform, language and query facts in arena/id form;
- materialization selects only columns required by snapshot lookup contracts and excludes
  search/export/index-maintenance payloads;
- secondary lookup indexes reference owned nodes and are derived state only;
- `context-resolver-search` may adapt snapshot nodes into `context-resolver-core` DTOs;
- downstream `v8-context` consumes the snapshot/read handle and must not query raw SQLite tables or
  keep analyzer-side mirror ownership for documented HBK facts.

The first measured source is the existing SQLite provider index, not direct HBK book parsing.
T167 measured release schema-16 `shcntx_ru` index build at `14.50s` / `284360 KiB` peak RSS and
release compact SQLite -> snapshot probe materialization at `474 ms` / `49112 KiB` peak RSS. This makes
SQLite bulk materialization the accepted first implementation path. Direct HBK extraction remains
setup/index-refresh input and a comparison baseline.

T168 implemented the first `HbkFactSnapshot` arena/read-handle slice. On the same release
`shcntx_ru` SQLite index, stable warm in-memory snapshot builds measured `507-601 ms`, median
`511 ms`, after excluding first-run/cache warm-up observations from the baseline. The snapshot-owned
heap estimate was `18197557` bytes, while process-level peak RSS was about `105708-105844 KiB`.

The next snapshot index shape should be analyzer-query-shaped, not shaped around public
resolver/provider DTO result families. The physical model has two layers:

- owned nodes/arenas are the single source of provider facts and preserve natural nesting for
  platform types, constructors, members, callables, globals, module contexts and query-language
  facts;
- secondary physical indexes store only compact keys and `NodeRef`/range values into those arenas.

The first hot-path indexes are:

| Index | Key | Value | Consumer path |
| --- | --- | --- | --- |
| `by_fact_id` | source-qualified fact/local id | `NodeRef` | exact lookup, provenance and resolver cache references |
| `type_by_name` | normalized primary or alias | type refs | `Тип("...")`, constructor lookup and template evidence lookup for downstream composition |
| `type_by_template_key` | `(family, variant)` | type refs | provider-backed template evidence for downstream metadata composition |
| `members_by_owner` | resolved type ref | member refs/range | complete context/type member scan |
| `member_by_owner_name_kind` | `(resolved type ref, normalized name, optional kind)` | member refs | property/method/event lookup from access expressions |
| `callable_by_owner_name` | `(resolved type ref, normalized name)` | callable refs | overload and return-type lookup for method calls |
| `constructors_by_type` | resolved type ref | callable refs/range | `Новый Тип(...)` constructor lookup |
| `global_by_language_name_kind` | `(language/domain, normalized name, optional kind)` | global refs | BSL/SDBL global method and property lookup |
| `module_context_by_kind` | `(language, domain, module kind)` | module-context ref | module globals and events |
| `query_table_by_name` | normalized query table id/name/syntax/identifier | query-table refs | static query `FROM` resolution |
| `query_field_by_table_name` | `(query-table ref, normalized field)` | query-field refs | query field resolution |
| `query_param_by_table_name` | `(query-table ref, normalized parameter)` | query-parameter refs | virtual-table parameter lookup |
| `availability_by_fact` | fact id/ref | compact availability | context applicability checks |
| `relations_by_source_kind` | `(fact id/ref, relation kind)` | node refs/range | bounded related/type/module traversals |

The first implementation may keep nodes in contiguous arenas with owner ranges instead of nested
`Vec` fields when that improves cache locality and reduces allocation count. Logical nesting remains
provider-owned: type nodes own ranges for their members, constructors and callables, while indexes
only point at those owned nodes.

Performance and memory accounting is part of the snapshot contract. A reshaped snapshot must report:

- immutable node/arena bytes, string-store bytes and secondary-index bytes separately;
- per-index counts and estimated bytes for the hot-path indexes above;
- materialization wall-clock time, process peak RSS and estimated snapshot-owned heap across warm
  release runs comparable to the T168 baseline;
- batched release lookup timings for representative analyzer paths after source open.

If a new index materially increases snapshot-owned heap or process peak RSS, the implementation must
identify the responsible index and either justify it with a measured lookup benefit or split that
index into a separate follow-up. Do not trade a small point-lookup improvement for broad duplicated
payload storage.

T169 implemented these index families inside `syntax-helper-search::HbkFactSnapshot` and exposed
them through `HbkFactReadHandle`, with per-index memory accounting and a release measurement
harness. It also added enum and enum-value fact refs to the exact-id, relation and availability
lookup surfaces.

The resolver-adapter boundary is now explicitly split. `context-resolver-search` owns
snapshot-backed `PlatformSnapshotSource` and `QueryTableSnapshotSource` adapters over
provider-owned `Arc<HbkFactSnapshot>` state for downstream analyzer hot paths. The existing
`PlatformSearchSource` and `LanguageSearchSource` remain explicit SQL/SearchIndex-backed adapters
for CLI, debug, index inspection and sequential local resolver usage. Snapshot-backed adapters use
read-handle calls and project into `context-resolver-core` DTOs without analyzer-side HBK mirrors,
broad `Arc<Mutex<_>>` / `Arc<RwLock<_>>` wrappers, raw SQLite reads for migrated paths or broad
provenance/description payloads in the snapshot node arenas.

Physical indexes remain a `syntax-helper-search` provider concern. `context-resolver-search` may
adapt read-handle results into source-neutral DTOs, but it must not build a second provider-fact
index, keep analyzer-owned mirrors of HBK facts, or bypass the snapshot with raw SQLite table reads.
The first query-shaped snapshot slice should use `std` and existing workspace dependencies only;
new runtime dependencies require a measured bottleneck and a separate ADR/spec decision.

Snapshot fields that are not analyzer hot-path keys or compact node payloads remain DTO/provenance
projection data. Descriptions, previews, notes, full signature text, raw HTML paths, page titles,
long documentation text, arbitrary fuzzy search data and all unbounded relation paths must not
become first-slice physical indexes.

Do not introduce Tantivy, a graph database, a new persisted snapshot format, minimal-perfect hashing
or compressed bitmap indexes in the first slice. Evaluate `fst` for name/id maps, `rkyv` or
`zerovec` for persisted zero-copy snapshots, and `roaring` for large set intersections only after
the owned arena snapshot has concrete memory/latency measurements.

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

T161 evaluated library-backed link/path rewriting for the ordinary documentation reader, single-book
Markdown export and generated documentation-site data. The current ownership remains split by
domain boundary:

- `hbk-book` owns HBK virtual storage path normalization.
- `hbk-docs` owns documentation-page link extraction and unresolved-link diagnostics.
- `hbk-book-export` owns single-book Markdown link rewriting, fragment preservation and relative
  Markdown targets.
- `hbk-doc-site` owns generated page-id targets, source-book aliases, placeholder aliases and
  same-page generated-fragment collapse.

`url` is not selected for HBK link/path rewriting because the project must treat `v8help://`
targets as HBK book-id plus virtual storage path evidence, not as ordinary network URLs. It also
does not own fragment-only same-page semantics or unresolved HBK page diagnostics.

`path-clean` is not selected for `normalize_storage_path*` because the current storage path rules
are HBK virtual-entry rules, not filesystem path rules. Empty/root results, fragment-looking local
targets and unsafe parent traversal remain explicit project behavior.

`lol_html` remains the only candidate with a plausible future use: replacing the narrow
`href`-attribute scanner in `hbk-book-export` with an HTML-aware rewriter. That future change must
be a separate implementation task with fixture and real-HBK parity evidence for quoted attributes,
case-insensitive `href`, removed unresolved links, preserved external links, same-book
`v8help://` links, cross-book/generated aliases and `#fragment` targets. T161 does not add
`lol_html` as a product dependency because no behavior-preserving replacement was implemented.
