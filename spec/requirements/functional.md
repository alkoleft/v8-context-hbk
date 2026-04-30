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
- Provide a separate Syntax Assistant query CLI for fast retrieval of extracted platform API facts,
  including exact lookup, description/keyword search and relationship exploration.
- Preserve provenance for diagnostics: HBK file path, entity name, TOC path, HTML path and page
  title.
- Keep public library, CLI and export contracts provisional until real-platform acceptance and
  downstream consumer feedback justify stabilization.

## Non-Goals

- Writing or modifying `.hbk` containers.
- Rendering the full HTML help UI.
- MCP server implementation.
- Runtime extraction from 1C processes.
- Complete compatibility proof for every platform version.
- Backward-compatible reproduction of Java/Kotlin public APIs, class names, DTOs or CLI behavior.
- Immediate merge into `/home/alko/develop/open-source/v8-context/`.
- General-purpose question answering that is not grounded in extracted Syntax Assistant facts.
- Network-hosted semantic search or embedding-provider integration as the first search CLI slice.

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

The system must open a help book on top of the container reader, inflate `PackBlock`, validate the
`FileStorage` entity body, open `FileStorage` as ZIP for stored-file reads, parse `Book` metadata,
infer locale from filename and read stored files by HTML/resource path. The source HBK file must
remain readable for page/file access after `open`.

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
- Known unsupported Syntax Assistant source families must use stable family-specific diagnostics
  rather than remaining generic `UNKNOWN_PAGE_CLASS` records.

## FR-SH-002: Syntax Assistant Extraction

The system must extract:

- global methods and properties
- global context events
- platform types/objects
- type methods, properties and constructors
- query/table fields and query/table parameters
- enum definitions and enum values
- signatures, parameters, required flags and return types when present
- localized names/aliases when present
- normalized descriptions
- structured availability/application contexts when present, such as thin client, web client,
  mobile client, server, thick client, external connection and mobile application modes
- syntax examples when present, preserving them separately from descriptions
- "see also" relationships when present, preserving them separately from descriptions
- availability/version-introduced text when present
- source provenance for every extracted item

The extractor must not synthesize consumer records from TOC-only pages when source HTML cannot be
loaded or parsed safely. Such in-scope-but-unsupported pages remain visible as recoverable
diagnostics with source provenance until a typed extraction contract is added.

Multiple signatures are overloads. If real pages expose multiple return types for one overload while
the model assumes one return type per overload, report it as a parser/data-contract gap instead of
silently truncating.

Syntax Assistant HTML section parsing must be locale-aware for both Russian and root/English source
books. Section boundaries must prevent description, parameter and signature fields from swallowing
later sections such as availability, examples, see-also links, version information or overload
variant descriptions. English labels used by root books, including `Type:` and `Returned value:`,
must be parsed with the same semantic completeness as Russian labels.

Acceptance:

- Reading `shcntx_ru.hbk` returns non-empty global methods, global properties, platform types and
  enums when the fixture exists.
- Reading `shcntx_ru.hbk` returns non-empty global context events, query/table fields and
  query/table parameters when the fixture exists.
- Fixture tests cover at least one known global method, global property, type and enum.
- Russian and root/English exports preserve return types and property/parameter type references for
  representative pages that contain them.
- Descriptions do not contain raw section labels for availability, examples, see-also links,
  available-since text or overload variant text.
- Overload/syntax-variant pages produce only real callable signatures as signatures, with variant
  metadata attached separately when present.

## FR-EXPORT-001: Canonical JSON Export

The system must serialize extracted Syntax Assistant platform facts to JSON as the canonical
consumer machine format.

The consumer export is not a help-book, TOC or parser-trace dump. It must expose platform API facts
needed by downstream context/indexing tools:

- names and aliases
- descriptions
- signatures
- parameters and required flags
- return types and property types
- owner relationships for type members, constructors and enum values
- owner relationships for query/table fields and query/table parameters
- structured availability/application contexts, examples, see-also relationships, available-since
  text and overload variant metadata when extracted

Consumer record files must not expose book hierarchy or per-record parser provenance:

- source HBK path
- source locale on every record
- TOC/index path
- HTML path
- page title
- root/global context link catalogs
- method, constructor or enum value navigation links that duplicate dedicated record-family files

Parser provenance remains part of the internal model and diagnostics contract. `diagnostics.json`
keeps enough source context for parser maintenance; consumer record files stay focused on platform
facts.

Required files:

- `metadata.json`
- `global-methods.json`
- `global-properties.json`
- `global-context-events.json`
- `platform-types.json`
- `type-methods.json`
- `type-properties.json`
- `table-fields.json`
- `table-parameters.json`
- `constructors.json`
- `enums.json`
- `diagnostics.json`

The current accepted consumer export schema is `schema_version: 5`. Each consumer record-family
file is a JSON object with `schema_version`, `locale`, `source_locale`, `record_kind` and
`records`.
`metadata.json` contains export-level metadata and file inventory; it must not expose source HBK
paths or book hierarchy. `diagnostics.json` may keep parser source context because its audience is
parser maintenance, not downstream platform API consumption.

Schema version 5 keeps consumer records lean. Consumer records must omit `null` fields and empty
arrays. This omission rule applies to platform API consumer records; it does not remove the
top-level `records` array from record-family envelopes and does not weaken the parser-maintenance
diagnostics contract.

- `owner`: string with the owner's primary name, such as `"ГруппаФормы"`, for type members,
  constructors, table fields, table parameters and other owned consumer records.
- `type_refs`: deterministic array of type-name strings, such as `["Строка", "Массив"]`, wherever
  type references are exposed, including properties, table fields, table parameters and signature
  parameters.
- `return_types`: deterministic array of type-name strings, such as `["Строка"]`.
- `signatures`: array of callable signatures with `parameters` and optional variant metadata.
  `signatures[].text` is not part of the consumer JSON contract for methods, global context events
  or constructors. Syntax-variant `title` and `description` are written directly on the signature
  object when present; the nested `variant` object is not emitted. Variant metadata must not expose
  HBK, TOC or HTML provenance.
- `availability`: object with `contexts`, a deterministic array of normalized snake_case execution
  context values such as `thin_client`, `web_client`, `mobile_client`, `server`, `thick_client`,
  `external_connection`, `mobile_application_client`, `mobile_application_server` and
  `mobile_standalone_server`, and optional `since`, a normalized version string such as `"8.3.6"`.
  If neither `contexts` nor `since` is present, the whole `availability` field is omitted.
- `examples`: array of objects with `text` containing extracted Syntax Assistant example/code text
- `see_also`: deterministic array of target primary-name strings, such as `["Форма",
  "ОбработкаПроверкиЗаполнения"]`; consumer records still omit target HTML paths.
- `available_since`: not emitted as a top-level consumer record field. Recognized version facts are
  serialized as `availability.since`.

Property records in `global-properties.json` and `type-properties.json` share the same semantic
shape: `name`, optional `owner` for type properties, `usage`, `type_refs`, `description` and shared
section facts. `usage` is a stable enum string with values `Read`, `Write`, `ReadWrite` or
`Unknown`, not localized free text. Property descriptions must not retain leading type-reference
prose such as `Тип: ВидГруппыФормы . ` / `Type: ... .`; that fact belongs to `type_refs`.

Method records in `global-methods.json` and `type-methods.json` share the same semantic shape:
`name`, optional `owner` for type methods, `signatures`, `return_types`, `description` and shared
section facts.

Constructors use the same signature shape as methods: `signatures[].text` is not emitted, and
variant metadata, if ever present, is direct signature metadata.

Schema version 4 adds Syntax Assistant source families that were previously diagnostic-only:

- `global-context-events.json`: global context event handler facts with `name`, `signatures`,
  `description`, structured section facts and no return types.
- `table-fields.json`: query/table metadata fields with `owner`, `name`, `type_refs`,
  `description` and `note`.
- `table-parameters.json`: query/table metadata parameters with `owner`, `name`, `required`,
  `type_refs`, `description` and `default_value`.

Global context events, table fields and table parameters are first-class consumer record families.
They must no longer be reported as `OUT_OF_SCOPE_GLOBAL_CONTEXT_EVENT`,
`OUT_OF_SCOPE_TABLE_FIELD` or `OUT_OF_SCOPE_TABLE_PARAMETER` for the target platform source books.

Schema version 5 merges enum values into `enums.json`. `enum-values.json` is no longer emitted.
Each enum record has `values`, a deterministic array of enum value records. Nested enum value
records include `name`, `description` when present and `availability.since` only when the value's
version differs from the owning enum's `availability.since`. Enum and enum value `name` fields keep
the localized-name object shape with `primary` and optional `alias`.

Acceptance:

- `shcntx_ru.hbk` exports as locale `ru`.
- `shcntx_root.hbk` exports as locale `en`.
- Output files are non-empty and parse successfully.
- Consumer record-family files do not contain `source_hbk` at the top level.
- Consumer records do not contain `source`, `source_hbk`, `toc_path`, `html_path`, `page_title`,
  `method_links`, `constructor_links` or `value_links`.
- `metadata.json` does not expose source HBK paths.

## FR-LOOKUP-001: Exact Lookup Helpers

The system must provide exact lookup helpers for:

- global member by name
- type by name
- type member by type/member name
- constructors by type name

Search ranking is out of scope for these in-memory lookup helpers. FR-SH-SEARCH-001 covers the
separate query CLI search behavior.

## FR-SH-SEARCH-001: Fast Syntax Assistant Query CLI

The system must provide a separate Syntax Assistant-focused CLI surface for interactive retrieval
over extracted platform API facts.

Query commands must operate on a prebuilt local export or search index. The first durable index
format is expected to be a local SQLite/FTS5 database unless ADR-0004 is revised. Query commands
must not open and parse large `shcntx_*.hbk` books on every query.

Required query modes:

- exact lookup by primary name or alias;
- exact owner/member lookup, such as `НастройкиКомпоновкиДанных.Отбор`;
- keyword/full-text search over names, aliases, signatures, parameter names, return/type references
  and descriptions;
- fuzzy name search for small spelling differences;
- purpose-oriented search over descriptions, such as finding APIs related to filtering, reports or
  data composition;
- relationship search from one API fact to related facts.

The first implementation may use lexical ranking only. Semantic search is a planned extension point
after the local index and relationship graph prove useful on real extracted data.

Acceptance:

- Exact lookup for `ОтборКомпоновкиДанных` or `DataCompositionFilter` returns the platform type and
  its description from the Russian Syntax Assistant export.
- Exact lookup for `НастройкиКомпоновкиДанных.Отбор` returns the property with type reference
  `ОтборКомпоновкиДанных`.
- Keyword search for `отбор скд` returns data-composition filter facts ahead of unrelated filter
  facts when the Russian Syntax Assistant fixture exists.
- Query output is available as readable text and deterministic JSON.

## FR-SH-SEARCH-002: Syntax Assistant Relationship Graph

The system must derive a relationship graph for Syntax Assistant facts.

Required relationship sources:

- owner-to-member edges for type methods, type properties, constructors and enum values;
- member-to-type edges from property type references, method return types, constructor owners and
  parameter type references;
- collection/item edges visible through property type references and descriptions;
- Syntax Assistant navigation links such as section member links and "see also" links when they are
  extracted from the HBK page HTML;
- TOC/page provenance when the index was built from provenance-rich extraction data.

Relationship search must be able to answer deterministic graph-style questions before any semantic
model is introduced. For example, "how is an SKD filter created" should be explainable through:

- `НастройкиКомпоновкиДанных.Отбор` -> `ОтборКомпоновкиДанных`;
- `ОтборКомпоновкиДанных.Элементы` -> `КоллекцияЭлементовОтбораКомпоновкиДанных`;
- `КоллекцияЭлементовОтбораКомпоновкиДанных.Добавить`;
- `ЭлементОтбораКомпоновкиДанных` fields such as `ЛевоеЗначение`, `ВидСравнения`,
  `ПравоеЗначение` and `Использование`.

Acceptance:

- Relationship output for `ОтборКомпоновкиДанных` includes its properties, methods and constructor.
- Relationship output for `НастройкиКомпоновкиДанных.Отбор` includes the target type
  `ОтборКомпоновкиДанных`.
- Relationship output remains deterministic and does not depend on query-time HBK parsing.

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
