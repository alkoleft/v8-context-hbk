# Implementation Component Specification

Current status: the repository is a Cargo workspace split into the crates below. The split preserves
context boundaries and keeps CLI/export behavior provisional.

## Workspace Crates

1. `hbk-container`: binary container parsing, entity enumeration and entity byte access.
2. `hbk-book`: book metadata, locale inference, ZIP-backed `FileStorage`, TOC parsing and page reads.
3. `hbk-docs`: documentation HTML/page parsing, normalized text/link extraction and page diagnostics.
4. `syntax-helper-model`: provenance-rich platform context domain model and record sink boundary.
5. `syntax-helper-extract`: Syntax Assistant root discovery, catalog traversal and specialized page parsers.
6. `syntax-helper-language`: shared non-platform HBK language-fact model and fixture-backed parsers
   for `shlang_*`, `shquery_*` and `dcsui_*` pages.
7. `hbk-export`: canonical JSON export adapters.
8. `syntax-helper-search`: local SQLite/FTS5 index and query library for Syntax Assistant exact
   lookup, keyword/fuzzy search and bounded relationship traversal.
9. `context-resolver-core`: source-neutral Rust resolver API with typed identities, domains,
   fact kinds, response statuses, diagnostics and resolver/source traits.
10. `context-resolver-search`: HBK-backed platform and language-domain source adapters over
   `syntax-helper-search::SearchIndex`.
11. `v8-context-hbk-cli`: command wiring for the `v8-context-hbk` binary.

Search/query components are described in
[`syntax-helper-query-cli.md`](syntax-helper-query-cli.md).

Solution-context Rust resolution is described in
[`solution-context-resolve.md`](solution-context-resolve.md). ADR-0008 owns this boundary.

## Dependency Rules

- `hbk-container` must not depend on book, docs, extraction or export concerns.
- `hbk-book` must not depend on Syntax Assistant extraction.
- `hbk-docs` may depend on book-level page/TOC abstractions but must not know export schema details.
- `syntax-helper-model` must not depend on HBK container, HTML parsing or CLI code.
- `syntax-helper-extract` owns traversal and parser behavior for Syntax Assistant pages.
- `syntax-helper-language` owns the first shared language-fact model and source-family parsers for
  non-platform HBK language pages. It must not add language facts to `PlatformContext` or
  `syntax export` consumer JSON.
- `hbk-export` owns output adapters for the Rust domain model.
- `syntax-helper-search` owns search-index schema, ranking and relationship traversal. It must not
  parse HBK files or perform CLI presentation. It may accept `syntax-helper-language` facts as
  pre-parsed documents for the T89 language-index fixture slice.
- `context-resolver-core` owns the generic in-process resolver model. It must not depend on HBK,
  SQLite, CLI, parser or Syntax Assistant storage crates.
- `context-resolver-search` owns translation between `syntax-helper-search::SearchIndex` platform
  and language facts and the source-neutral resolver model. It must not expose SQLite tables, FTS
  fields, query-table provider facts or Syntax Assistant provenance as generic resolver facts.
- `v8-context-hbk-cli` wires commands and error presentation only.
- Syntax Assistant search/query code must not make `hbk-export` carry search-only fields in the
  lean consumer export. Use a search-specific index when structured links or provenance are required
  for query workflows.
- The future solution-context resolver core must be a thin source-neutral integration layer above
  platform/search crates. It must not live inside `syntax-helper-search` and must not force BSL
  language, query-language, configuration or source-code providers to depend on HBK or SQLite
  implementation details.

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
owned by `hbk-export` and may intentionally omit internal provenance and navigation scaffolding.

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

### hbk-export

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
`hbk-export` adapter concern rather than an internal model constraint.

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

For schema version 8, `hbk-export` must emit `owner_path` only on records that represent semantic
owner context: platform types, module-event module context and query table records. It must not emit
`owner_path` on derivative type methods, type properties, constructors or nested query table
fields/parameters. `metadata.json.files` is the authoritative inventory for the current schema; the
exporter writes current files but must not delete stale files from older schemas in a reused output
directory.

Schema version 9 stops using the historical `global-context-events.json` filename for event facts.
The split is `module-events.json`, `type-events.json` and `unknown-events.json`; `hbk-export` routes
records by source-backed event classification without adding global semantic IDs. Type events carry
`owner` as a single semantic owner string, while module events carry `module`. Any owner/object kind
needed by events belongs on the owner type/object model, not as a duplicated event-only taxonomy.
The split preserves the schema version 8 rule that derivative records do not emit `owner_path`.

T38 adds optional `object_kind` to `platform-types.json` only. `syntax-helper-extract` derives it
from TOC-backed platform type context after `branch_kind` and `type_kind` are known; `hbk-export`
passes it through when present. Event files do not expose `object_kind`, `owner_kind`, `id` or
`owner_ref`, and derivative type members, constructors and nested query table records keep the
schema version 8 `owner_path` omission rule.

Schema version 10 removes semantic `owner_path` from `type-events.json`. `hbk-export` composes the
type-event semantic owner chain into the single `owner` string so exact owner/event lookup remains
unambiguous without adding a second owner field.

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
`inspect`, `toc` and `page`. The target Syntax Assistant command group for new export/index/query
work is `syntax`.

### Syntax Assistant query commands

Owns FR-SH-SEARCH-001 and FR-SH-SEARCH-002 after implementation.

The `v8-context-hbk syntax` query commands must read a prebuilt search index artifact for
interactive commands. They must not parse `shcntx_*.hbk` in exact lookup, text search, fuzzy search
or relationship search commands. Index build commands may parse Syntax Assistant HBK sources through
the extraction pipeline and must pass typed extracted facts into the search/index library rather
than building from consumer JSON export directories.

Implemented first slice:

- `syntax-helper-search` owns `index.sqlite` schema version `6`, read-only query opens, FTS5 keyword
  search, prefix-bounded fuzzy candidate selection, exact name/alias and owner/member lookup, and
  directed owner/type-reference relationship traversal.
- `v8-context-hbk syntax export/index/get/search/related` owns CLI argument parsing, index path
  resolution and text/JSON presentation.
- `syntax index` builds a replacement index beside the target file and atomically renames it after
  validation. Concurrent writers are serialized by a lock file.
- `syntax index` feeds extraction records into a search-index builder through
  `SyntaxHelperReader::extract_into()`. The builder keeps only search-index drafts and identity
  inputs, then writes documents and streams relation inserts into SQLite. The build path does not
  retain a full `PlatformContext`, complete search-document vector and complete relation vector at
  the same time.
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

The resolver core is implemented as `context-resolver-core`, a separate crate with no HBK, SQLite,
CLI or parser dependencies. The platform and first language-domain adapters are implemented in
`context-resolver-search`, a sibling adapter crate over `SearchIndex`; `syntax-helper-search`
remains the HBK/Syntax Assistant query implementation and not the generic cross-domain resolver
model.

The first resolver API must keep BSL language types and query-language types separate from platform
API types. Cross-domain links require explicit relations; same-name facts across domains or sources
must not be silently merged.

The platform adapter over `syntax-helper-search` initially exposes platform API type, member and
callable facts only. Existing query-table documents in the search index remain outside that adapter.
T66 selected current `shcntx_*` query-table documents to remain CLI/provider facts for now, not the
first `QueryLanguage` resolver source. A later language-domain task must define an explicit mapping
or relation shape before exposing them through the source-neutral resolver.

The first language-domain adapter slice exposes T89 language facts through source-specific
`shlang`, `shquery` and `dcsui` resolver sources. `shlang` maps to `BslLanguage`; `shquery` and
`dcsui` map to `QueryLanguage` while keeping distinct source ids. The adapter resolves exact ids and
exact names for language types, query/SKD functions, keywords, operators, constructs and literals
using the existing resolver fact kinds without adding new core model variants. `language_function`
facts are exposed as callable facts with ordered signatures and parameters; `language_type` and the
current `language_literal` facts are exposed through type lookup. Relation traversal uses only
explicit index edges derived from extracted language type references, for example
`dcsui:SKD_Functions_Strings#StringLength` parameter type `Строка` to `shlang:def_String`.

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
