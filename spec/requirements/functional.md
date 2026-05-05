# Functional Requirements

## Scope

`v8-context-hbk` reads 1C `*.hbk` help books and extracts structured platform documentation/context
from Syntax Assistant books. The first target platform baseline is `8.5.1.1150`.

The `syntax` command scope is product-oriented, not only source-oriented. ADR-0006 owns its goal:
successful assistance during BSL code development and analysis. Syntax Assistant facts, query
commands and machine-readable outputs should help a developer or tool answer practical code
questions such as which platform API exists, which constructor or method signature is valid, which
parameter names and types are expected, and which related types or members are needed to use an API
correctly.

The project stays independently testable until the HBK extraction model and provisional contracts are
validated on real HBK data. Future `v8-context` integration must use an explicit boundary, currently
the file-level export decided in ADR-0001.

Future BSL analyzer integration is an intended consumer direction for the `syntax` scope. This
repository still extracts documentation from HBK sources and does not implement BSL parsing or
runtime introspection, but `syntax` query/export contracts should be designed so they can become a
typed local provider for a BSL analyzer without re-parsing HBK books in analyzer query paths.

## Goals

- Provide Rust APIs for opening `.hbk` files, enumerating container entities and reading help-book
  content.
- Expose documentation navigation and page content from compatible `.hbk` files.
- Extract structured Syntax Assistant data from `shcntx_*.hbk`.
- Read Syntax Assistant pages with TOC-aware classification and ownership so the internal platform
  facts represent the book hierarchy, not only local HTML path/title patterns.
- Provide a Syntax Assistant query command surface for fast retrieval of extracted platform API facts,
  including exact lookup, description/keyword search and relationship exploration.
- Orient `syntax` command and index behavior toward BSL development and code-analysis assistance:
  precise signatures, constructor parameters, type references, owner/member relationships and
  deterministic tool-readable output are more important than generic documentation search breadth.
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
- Implementing a full BSL parser, linter or analyzer in this repository.
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

## FR-SH-003: TOC-Aware Syntax Assistant Reading

Decision record: ADR-0005.

The Syntax Assistant reader must treat the help-book TOC as the authoritative structural context for
classification and ownership. HTML paths, page headings and localized titles are useful evidence,
but they must not be the only source of truth when the TOC hierarchy distinguishes otherwise
identical page names.

For every Syntax Assistant page that becomes a typed platform fact, the reader must derive an
internal semantic reading context from the TOC ancestor chain before exporting or indexing the fact.
The context must include, where applicable:

- root section kind, such as global context, type/object catalog, enum catalog or query-language
  table catalog;
- source family, such as global context event, platform type/object, type member, constructor,
  enum/value, query table field or query table parameter;
- semantic owner or owner path for owned facts;
- TOC branch labels needed to distinguish same-title pages under different Syntax Assistant
  branches;
- semantic branch kind, such as managed forms, metadata/application objects, primitive types,
  query/SDBL tables, ordinary platform objects or Automation/external API;
- the original parser provenance already required for diagnostics: HBK path, locale, TOC path,
  HTML path and page title.

Classification rules:

- Page classification must prefer TOC branch context over suffix-only HTML path checks when a path
  segment such as `/fields/`, `/params/`, `/events/`, `/methods/`, `/properties/` or `/ctors/`
  appears under more than one semantic branch.
- Query-language/SDBL table pages under the `Работа с запросами.Таблицы запросов` branch must be
  read as query table metadata, not as ordinary platform object members. Field and parameter owners
  must be derived from the nearest semantic table ancestor plus any required parent table family
  context, not only by stripping `/fields/` or `/params/` from the HTML path.
- Events under module-event groups, including global context module-event groups, session module
  events, ordinary/managed application module events, metadata object module events, form module
  events and web/HTTP service module events, must be read as `module_event` facts with module kind
  and semantic owner context. They must not be modeled as global context members solely because
  some groups are placed under the global context TOC root.
- Automation/external API TOC branches are category context for ordinary platform types and
  members. They must not become a separate record family unless a later requirement defines one.
- `Расширение...` / `Extension...` pages must be classified as extension platform types when the
  TOC/HTML/link evidence shows that they extend or mix into a base type or base role. The reader may
  record the proven base as an `extends` relationship, but must not synthesize an unproven base.
- Metadata/application-object template types such as `ДокументОбъект.<Имя документа>`,
  `СправочникСсылка.<Имя справочника>` and external-data-source table types must be classified
  separately from regular platform types. Their semantic context may include metadata kind and
  template parameters derived from TOC/name evidence.
- Primitive types are shallow. Direct children of the `Примитивные типы` branch are primitive
  platform types; nested pages under a primitive type, such as `Булево > Истина` and
  `Булево > Ложь`, must not be traversed as platform types by ordinary object-catalog recursion.
- Placeholder-like names such as `<Имя измерения>`, `<Имя элемента управления>` and generic table
  branch labels such as `Основная таблица` are valid source titles only when their semantic owner
  path disambiguates them. The reader must not collapse such records into a single owner/name fact.
- Global context event pages with the same primary name and alias under different TOC branches are
  distinct event variants unless a later requirement defines a typed merge rule. Their reading
  context must preserve the branch-level distinction before any consumer adapter sees the record.
- Platform type/object pages with the same localized name under different TOC branches are distinct
  source facts unless the reader can prove they represent the same semantic type. Differences in
  branch, owner context, availability or section facts must not be silently lost by name-only
  merging.
- If two source pages still map to the same semantic fact identity after TOC-aware classification,
  the reader may merge them only when the merge rule is explicit for that source family and the
  merged facts are deterministic. Otherwise it must keep them distinct or emit a recoverable
  diagnostic for an ambiguous reading context.

Non-solutions:

- Adding raw `toc_path`, `html_path`, `page_title` or source HBK paths back to consumer record files
  is not a reading fix. Those fields are parser provenance. The reader must derive semantic
  ownership/classification from them inside the domain model.
- Consumer export adapters may change later to expose semantic disambiguators, but they must not be
  used as a substitute for correct Syntax Assistant reading.

Acceptance:

- The reader preserves distinct semantic contexts for duplicate global context event names such as
  `ПриНачалеРаботыСистемы` / `OnStart` that appear under different module-event TOC branches.
- Module event records expose module kind and semantic owner context when applicable.
- Query table fields and parameters from `Работа с запросами.Таблицы запросов` use TOC-derived
  query table ownership that remains unambiguous for exact lookup.
- Placeholder-like query table fields, form-element properties and external data source
  constructors remain distinguishable by semantic owner/context.
- Same-name platform type/object pages such as event-like `ПередЗаписью` / `BeforeWrite` entries
  are either distinct semantic facts with explicit context or explicitly merged by a documented
  source-family rule.
- Primitive type extraction does not turn nested primitive literal pages such as
  `Булево > Истина` and `Булево > Ложь` into platform type records.
- Extension and metadata-template platform types are distinguishable from regular platform types.
- Reading diagnostics remain provenance-rich when the reader cannot classify or disambiguate a
  source page safely.

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

Required files for the current consumer export contract:

- `metadata.json`
- `global-methods.json`
- `global-properties.json`
- `module-events.json`
- `type-events.json`
- `unknown-events.json`
- `platform-types.json`
- `type-methods.json`
- `type-properties.json`
- `query-tables.json`
- `constructors.json`
- `enums.json`
- `diagnostics.json`

The current accepted consumer export schema is `schema_version: 11`. Each consumer record-family file
is a JSON object with `schema_version`, `locale`, `source_locale`, `record_kind` and `records`.
`metadata.json` contains export-level metadata and file inventory; it must not expose source HBK
paths or book hierarchy. `diagnostics.json` may keep parser source context because its audience is
parser maintenance, not downstream platform API consumption.

Schema version 5 introduced the lean consumer-record rule, and schema version 6 preserves it.
Consumer records must omit `null` fields and empty arrays. This omission rule applies to platform
API consumer records; it does not remove the top-level `records` array from record-family envelopes
and does not weaken the parser-maintenance diagnostics contract.

Schema version 7 adds TOC-derived semantic identity fields for source families that cannot be
looked up safely by title alone. These fields are platform/documentation semantics derived by the
Syntax Assistant reader before export, not raw parser provenance:

- `record_family`: stable snake_case source-family value when a consumer needs to distinguish
  closely related facts, such as `module_event`, `type_event` and `unknown_event`.
- `branch_kind`: stable snake_case TOC branch category used for classification, such as
  `global_context`, `query_tables`, `primitive_types`, `metadata_objects`, `managed_forms`,
  `platform_objects` or `automation_external_api`.
- `owner_path`: deterministic array of localized semantic owner labels when a single `owner`
  string is insufficient for exact lookup, such as nested query table groups or placeholder-like
  metadata/form owners. It must not contain numeric TOC indexes, HBK paths or HTML paths.
- `module`: module-event context object with `kind` and optional `owner_path`.
- `type_kind`: platform type kind, one of `regular`, `extension`, `primitive` or
  `metadata_template`.
- `object_kind`: optional source-backed owner/object classification on platform type/object
  records, such as `regular_platform_type`, `managed_form`, `form_extension` or
  `metadata_object`. It is emitted only when TOC evidence proves the classification.
- `extends`: deterministic array of proven base type or base-role names for extension types. It is
  omitted when the source does not prove a base.
- `metadata_kind` and `template_parameters`: metadata-template type details when derivable from
  TOC/name evidence.

Schema version 8 keeps the TOC-derived semantic model but narrows where `owner_path` appears in
consumer records:

- `owner_path` is emitted on records that represent an owning semantic context, such as
  `platform-types.json`, `module-events.json.records[].module.owner_path` and
  `query-tables.json`.
- `owner_path` is not emitted on derivative records whose owner is already represented by `owner`,
  including `type-events.json`, `type-methods.json`, `type-properties.json` and
  `constructors.json`.
- query table fields and parameters do not repeat `owner_path`; their table context is the enclosing
  `query-tables.json` record.

Schema version 8 replaces `table-fields.json` and `table-parameters.json` with
`query-tables.json`. Query table records represent the real query-language/SDBL table pages from the
Syntax Assistant TOC, including generic "Основная таблица" / "Main table" pages and additional table
pages under the same owner family. The shape is:

- `branch_kind`: `query_tables`.
- `name`: a string table name. Query table names, field names and parameter names use strings, not
  `{ primary, alias }`, unless future source evidence proves aliases for this source family.
- `syntax`: the Syntax Assistant `Синтаксис` / `Syntax` section for the table page when present,
  exposed as a localized-name object with `primary` and optional `alias`. Russian source pages may
  contain both syntax variants in one section, such as
  `БизнесПроцесс.<Имя бизнес-процесса> (BusinessProcess.<Имя бизнес-процесса>)`; the Russian form
  is `syntax.primary` and the parenthesized English form is `syntax.alias`.
- `identifier`: a deterministic table identifier derived from the Syntax Assistant syntax and table
  page name. Primary table identifiers use the leading syntax segment before the first dot, such as
  `БизнесПроцесс` for `БизнесПроцесс.<Имя бизнес-процесса>`. Additional table identifiers use the
  primary table identifier plus the table `name` normalized to CamelCase. Whitespace, hyphens and
  other punctuation are word separators and are not copied into the identifier.
- `owner_path`: deterministic semantic owner labels for the table family, such as
  `["Таблицы задач"]`, not raw TOC indexes, HBK paths or HTML paths.
- `table_role`: `primary`, `additional` or `unknown`. A query table page whose syntax has at most
  two dot-separated semantic segments, such as `БизнесПроцесс.<Имя бизнес-процесса>` or
  `Task.<Task name>`, maps to `primary`. A page with a longer syntax, such as
  `БизнесПроцесс.<Имя бизнес-процесса>.Точки`, maps to `additional`. Generic
  "Основная таблица" / "Main table" page names remain a fallback primary signal when syntax is not
  available.
- `description`: optional table description when parsed from the table page.
- `fields`: array of table fields with string `name`, `types`, optional `description` and optional
  `note`.
- `parameters`: array of table parameters with string `name`, `types`, optional `description` and
  optional `default_value`. Query table parameters do not expose a `required` field unless later
  source evidence defines a reliable requiredness contract.

If the table page can be identified from TOC but its HTML description cannot be parsed safely, the
export must still emit the table record with `name`, `owner_path`, `table_role`, `fields` and
`parameters`, and report parser gaps through diagnostics when appropriate.

The export adapter writes the current contract files and `metadata.json.files` is the authoritative
file inventory for that export. The exporter must not delete stale files from earlier schema versions
that happen to exist in a reused output directory.

- `owner`: string with the owner's primary name, such as `"ГруппаФормы"`, for type members,
  constructors and other owned consumer records.
- `types`: deterministic array of type-name strings, such as `["Строка", "Массив"]`, wherever
  type references are exposed, including properties, query table fields, query table parameters and signature
  parameters.
- `return`: deterministic array of type-name strings, such as `["Строка"]`.
- `signatures`: array of callable signatures with `parameters` and optional variant metadata.
  `signatures[].text` is not part of the consumer JSON contract for methods, global context events
  or constructors. Syntax-variant `title` and `description` are written directly on the signature
  object when present; the nested `variant` object is not emitted. Variant metadata must not expose
  HBK, TOC or HTML provenance. Parser section boundaries must preserve all parameters in a callable
  signature even when a parameter description contains label-like inline text such as `Примечание:`.
- `availability`: object with `contexts`, a deterministic array of normalized snake_case execution
  context values such as `thin_client`, `web_client`, `mobile_client`, `server`, `thick_client`,
  `external_connection`, `mobile_application_client`, `mobile_application_server` and
  `mobile_standalone_server`, and optional `since`, a normalized version string such as `"8.3.6"`.
  If neither `contexts` nor `since` is present, the whole `availability` field is omitted.
- `examples`: array of objects with `text` containing extracted Syntax Assistant example/code text.
  Inline example sections inside source descriptions are still examples; they must not absorb later
  sections such as availability/application-context lists. Code examples must not contain
  HTML-coloring artifacts such as extra spaces before dots, commas, semicolons, brackets or
  parentheses.
- `see_also`: deterministic array of target primary-name strings, such as `["Форма",
  "ОбработкаПроверкиЗаполнения"]`. When source see-also HTML expresses a target as an owner link
  followed by a member link, the consumer target is composed as `Owner.Member`, such as
  `ИзбранноеРаботыПользователя.Вставить` or
  `Глобальный контекст.ИсторияРаботыПользователя`; consumer records still omit target HTML paths.
- `available_since`: not emitted as a top-level consumer record field. Recognized version facts are
  serialized as `availability.since`.

Property records in `global-properties.json` and `type-properties.json` share the same semantic
shape: `name`, optional `owner` for type properties, `usage`, `types`, `description` and shared
section facts. `usage` is a stable enum string with values `Read`, `Write`, `ReadWrite` or
`Unknown`, not localized free text. Property descriptions must not retain leading type-reference
prose such as `Тип: ВидГруппыФормы . ` / `Type: ... .`; that fact belongs to `types`.

Method records in `global-methods.json` and `type-methods.json` share the same semantic shape:
`name`, optional `owner` for type methods, `signatures`, `return`, `description` and shared
section facts.

Constructors use the same signature shape as methods: `signatures[].text` is not emitted, and
variant metadata, if ever present, is direct signature metadata.

Schema version 4 adds Syntax Assistant source families that were previously diagnostic-only:

- `global-context-events.json`: required adapter file for module-event handler facts with
  `record_family="module_event"`, `name`, semantic module context, `signatures`, `description`,
  structured section facts and no return types.
- `table-fields.json`: schema v4-v7 query/table metadata fields with `owner`, `name`, `types`,
  `owner_path`, `description` and `note`.
- `table-parameters.json`: schema v4-v7 query/table metadata parameters with `owner`, `name`,
  `required`, `owner_path`, `types`, `description` and `default_value`.

Module events and query tables are first-class consumer facts. Query table fields and parameters
are nested under their owning `query-tables.json` records in schema v8. They must no longer be
reported as `OUT_OF_SCOPE_GLOBAL_CONTEXT_EVENT`, `OUT_OF_SCOPE_TABLE_FIELD` or
`OUT_OF_SCOPE_TABLE_PARAMETER` for the target platform source books.

Schema version 9 replaces the historical `global-context-events.json` adapter with event-specific
record-family files. It must not introduce a cross-cutting semantic identifier; stable IDs and
cross-file references are a separate future contract if a concrete consumer needs them. The event
files are:

- `module-events.json`: module-level events, including current global-context event groups and
  object/manager module events when the TOC identifies them as module handlers.
- `type-events.json`: event-like facts owned by platform types, forms, form extensions, form
  elements, type extensions or other type/object branches that are not module-level handlers.
- `unknown-events.json`: recoverable fallback records only when TOC/HTML evidence is insufficient
  to classify an event as module-level or type-level. Unknown event consumer records still omit raw
  HBK, TOC and HTML provenance; parser-maintenance diagnostics remain provenance-rich.

Type event records expose source-backed semantic owner context as a single `owner` string. When a
terminal owner label is ambiguous in the source, `owner` is the deterministic localized semantic
owner chain composed into one string; it must remain sufficient for exact `(owner, event name)`
lookup without a separate `owner_path`. Type events must not expose an event-local `owner.kind` /
`owner_kind`, `id`, `owner_ref`, source HBK path, TOC path, HTML path or page title.

Schema version 10 tightens the event split contract by removing semantic `owner_path` from
`type-events.json`. Event splitting must preserve the schema version 8 `owner_path` narrowing. It
must not reintroduce `owner_path` on type events, derivative type members, constructors or nested
query table records. Event records may keep only the owner context that is explicitly defined for
the event export contract; if exact event lookup later requires broader owner disambiguation, that
requirement must be specified in the event task without weakening the schema version 8
derivative-record omission rule.

Schema version 11 promotes query table syntax and identifier to the consumer JSON contract and fixes
query table role classification to prefer the `syntax.primary` shape over table page title. Tables such as
`Таблица бизнес-процессов` / `Business Process Table`, whose syntax is
`БизнесПроцесс.<Имя бизнес-процесса>` / `BusinessProcess.<Business process name>`, are primary
tables even though their page title is not "Основная таблица" / "Main table". Additional tables such
as `БизнесПроцесс.<Имя бизнес-процесса>.Точки` derive their identifier from the primary table
identifier plus the CamelCase-normalized page `name`.

Owner classification belongs to the owner object/type model, not to a local event-only
`owner.kind` field. Platform type/object records should expose a source-backed owner/object
classification field when the TOC proves it, and event records should reuse that owner semantics
through their record family and owner context rather than inventing a parallel owner taxonomy.
The current owner/object classification field is `object_kind` on `platform-types.json` records;
event files must not expose `owner.kind`, `owner_kind` or `object_kind`.

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
indexed query command behavior.

## FR-SH-SEARCH-001: Fast Syntax Assistant Query Commands

The system must provide a Syntax Assistant-focused command surface for interactive retrieval over
extracted platform API facts. Its primary success criterion is whether it helps BSL developers and
code-analysis tools resolve platform API usage questions quickly and accurately.

Index build commands must read Syntax Assistant HBK sources through the normal extraction pipeline
and write a prebuilt local search index. Query commands must operate only on that prebuilt search
index. The first durable index format is expected to be a local SQLite/FTS5 database unless ADR-0004
is revised. Query commands must not open and parse large `shcntx_*.hbk` books on every query.

The query commands must provide a default index path so common lookup and search commands do not
require `--index` every time. The default may be overlaid by an explicit command-line option or
environment variable, but the first slice must resolve to one effective index path per command
rather than merging multiple indexes.

Required query modes:

- exact lookup by primary name or alias;
- exact owner/member lookup, such as `НастройкиКомпоновкиДанных.Отбор`;
- constructor lookup by type name for direct signature retrieval;
- keyword/full-text search over names, aliases, signatures, parameter names, return/type references
  and descriptions;
- fuzzy name search for small spelling differences;
- purpose-oriented search over descriptions, such as finding APIs related to filtering, reports or
  data composition;
- relationship search from one API fact to related facts.

Machine-readable query output must be suitable for BSL development and code-analysis tools. Public
JSON fields must represent typed facts rather than internal search tokens. In particular, callable
parameter output must preserve parameter names separately from parameter type references,
requiredness and descriptions when those facts are available.

When query JSON exposes the same platform facts as `syntax export`, it should use export-compatible
field names and shapes. Existing query JSON remains provisional; compatibility with the current
search-result serialization is not a goal when it conflicts with the accepted export shape.

The first implementation may use lexical ranking only. Semantic search is a planned extension point
after the local index and relationship graph prove useful on real extracted data.

Acceptance:

- Exact lookup for `ОтборКомпоновкиДанных` or `DataCompositionFilter` returns the platform type and
  its description from an index built from the Russian Syntax Assistant HBK.
- Exact lookup for `НастройкиКомпоновкиДанных.Отбор` returns the property with type reference
  `ОтборКомпоновкиДанных`.
- Constructor lookup for `HTTPСоединение` returns its constructor signatures without requiring users
  to post-process relationship JSON.
- Constructor lookup offers a detailed text mode that includes available owner and description
  context while preserving signature-only output as the default.
- Constructor JSON for `HTTPСоединение` exposes parameter names and type references without
  interleaving both kinds of values in one ambiguous array, preferably using the export-compatible
  parameter shape with `name`, `required`, `types` and optional `description`.
- Keyword search for `отбор скд` returns data-composition filter facts ahead of unrelated filter
  facts when the Russian Syntax Assistant fixture exists.
- Search and relationship JSON can be explicitly bounded for review-oriented use without changing
  the default full provider output.
- Relationship JSON offers an explicit compact mode that preserves fact identity and relationship
  explanation while omitting bulky fact fields not needed for triage.
- Query output is available as readable text and deterministic JSON.

## FR-SH-PROVIDER-001: Syntax Provider Contract for BSL Tooling

The system must evolve the `syntax` query surface as a local platform-API fact provider for BSL
development and code-analysis workflows.

Decision record: ADR-0007.

The selected first analyzer-facing provider boundary is local CLI JSON over a prebuilt `syntax`
index. Analyzer-oriented consumers should call `syntax get`, `syntax constructors`, `syntax search`
or `syntax related` and consume the versioned provider envelope. The SQLite index remains a
rebuildable internal provider artifact, not a public table-level contract. Rust library APIs,
batch-only analyzer artifacts and service boundaries for this CLI provider require a future ADR or
task with a concrete consumer need. ADR-0008 separately defines the in-process Rust
solution-context resolver boundary without changing this CLI JSON contract.

Provider-oriented outputs must:

- be deterministic for the same index and query;
- use a versioned provider response envelope for JSON output from `syntax get`, `syntax
  constructors`, `syntax search`, `syntax related` and analyzer-oriented provider primitives;
- include stable document identity and fact kind;
- expose names, aliases and owner identity where applicable;
- expose callable signatures as structured facts for methods, constructors and events where source
  data contains structured signatures, using the `syntax export` signature shape where applicable;
- expose parameter name, requiredness, type references and description as separate fields;
- expose return/type references as typed arrays, not only prose;
- report ambiguity or missing facts explicitly instead of silently choosing hidden matches;
- report unsupported query-root combinations explicitly in JSON mode instead of falling out of the
  provider envelope;
- keep FTS/ranking/search-only tokens internal unless a future task deliberately exposes them under
  an explicit debug field.

Analyzer-oriented primitives must cover the direct operations needed for type inference and member
completion over a prebuilt local index:

- resolve a platform type by exact provider id, primary name or alias;
- list members for one resolved type identity;
- resolve one member by resolved owner type id or exact owner name plus member name;
- retrieve callable overloads with ordered parameters and return or constructor result types;
- expose type-reference edges needed to follow expression chains.

These primitives are still CLI JSON provider commands. They must not make normalized SQLite table
names, Rust structs, BSL parser internals or source provenance fields part of the public contract.

The provider contract remains provisional until real BSL task scenarios are accepted. This
repository must not implement BSL parsing, linting or diagnostics as part of this requirement.

Acceptance:

- `syntax constructors "HTTPСоединение" --format json` can be consumed by a tool to identify the
  overload containing `ИспользоватьАутентификациюОС` and its `Булево` type reference.
- `syntax get --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --format json`
  exposes the same exact property fact as owner/member lookup.
- `syntax get --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json` exposes the
  property kind, owner and `ОтборКомпоновкиДанных` type reference in deterministic JSON.
- `syntax related --id <document-id> --format json` and `syntax related --owner
  "НастройкиКомпоновкиДанных" --member "Отбор" --format json` can traverse relationships from an
  analyzer-safe exact root without relying on a plain-name lookup.
- `syntax related --id <document-id> --edge member_of --format json` exposes deterministic
  inverse owner navigation for owned facts that have a source-backed owner relationship.
- Search-only token fields used to populate FTS are not exposed as misleading public fields in the
  provider JSON contract.
- Query/provider JSON that returns constructor, method or event signatures uses field names
  compatible with `syntax export` for shared facts.
- Query/provider JSON separates provider facts from query metadata: shared platform facts are under
  result facts, while score, rank, relationship depth, relationship path, ambiguity and missing
  result diagnostics are envelope or per-result metadata.

## FR-CTX-RESOLVE-001: Rust Context Resolver API

The system must define a Rust API for resolving complete solution-context facts in process when a
Rust application needs repeated low-latency lookups across platform, BSL-language, query-language,
configuration and source-code providers.

Decision record: ADR-0008.

The resolver API must be source-neutral and fact-oriented:

- resolve context facts by source-qualified id, exact name, owner/member pair and callable query;
- resolve types in a requested source and language domain;
- list members for one resolved owner identity;
- retrieve callable overloads with ordered parameters and return or constructor result types;
- expose explicit relation edges such as ownership, type reference, return type, construction,
  generated-from, augments or maps-to when a provider has source-backed evidence;
- distinguish `PlatformApi`, `BslLanguage`, `QueryLanguage`, `Configuration` and `SourceCode`
  domains instead of folding all same-name facts into platform API types;
- use typed id wrappers for facts, types, members and callables; display names are lookup keys, not
  stable identities;
- return identity-preserving typed results for type, member and callable convenience methods instead
  of naked detail structs that drop source/domain identity;
- return ordinary lookup outcomes as data: `ok`, `not_found`, `ambiguous` and `unsupported`;
- reserve Rust errors for infrastructure failures such as missing indexes, unsupported schema
  versions, unreadable source artifacts or invalid source routing.

The first implementation may provide only the source-neutral core API and the HBK-backed platform
adapter over `syntax-helper-search`. Configuration metadata extraction, BSL parser/source indexing,
query parser/source indexing, diagnostics and code actions are out of scope for this repository
until a later spec assigns those providers.

Acceptance:

- The implementation spec defines the resolver traits, request/response model, domain model,
  source composition rules and first platform adapter mapping.
- Same-name facts from platform, BSL-language, query-language, configuration and source-code
  domains are source-qualified and report ambiguity unless the caller constrains the query.
- BSL language types and query-language types remain separate domains even when their display names
  match.
- Member lookup by resolved owner id does not mix members from another source or language domain
  with the same owner display name.
- Callable lookup preserves callable identity, ordered parameters and return or constructor type
  references.
- Platform adapter relation traversal preserves source-backed `has_type`, `returns`, `constructs`
  and `member_of` edges needed by resolver clients.
- The platform adapter can be implemented without exposing SQLite tables, FTS fields, HBK paths,
  TOC paths, HTML paths or page titles as public resolver facts.
- Existing `query_table`, `query_table_field` and `query_table_parameter` provider facts are not
  exposed through the platform adapter until the non-platform HBK domain analysis selects their
  query-language resolver shape.

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
- Edge-filtered relationship output supports `member_of` as public inverse navigation from an owned
  fact to its owning platform type or object.
- Relationship output remains deterministic and does not depend on query-time HBK parsing.

## FR-CLI-001: Verification-Oriented CLI

The initial CLI must support:

```bash
v8-context-hbk inspect <book.hbk>
v8-context-hbk toc <book.hbk> --format json
v8-context-hbk page <book.hbk> --path <html-path>
v8-context-hbk syntax export <shcntx.hbk> --output <dir>
```

Acceptance:

- Commands fail with non-zero exit and readable error on missing/corrupt input.
- `inspect` prints entity names and basic metadata.
- `syntax export` writes canonical JSON export files.
