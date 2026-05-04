# Source Evidence

This file records the current evidence anchors behind the specification. It is not a task plan.

## Platform Files

Current target platform baseline: `8.5.1.1150`.

Small real HBK smoke files:

- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`

Syntax Assistant books:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

The implementation must not hard-code only these names. `fmtdui_*` is the first fast smoke pair for
generic HBK behavior. `shcntx_*` is reserved for Syntax Assistant stages. Broad acceptance covers all
`*.hbk` files in the target platform directory.

## HBK Container Observations

`hbk-reader` established the current container facts used by this project:

- HBK is a binary container with a 16-byte container header.
- File descriptions are 12-byte records: header address, body address and reserved splitter.
- Blocks have a 31-byte header.
- Numeric fields are little-endian or hexadecimal string fields depending on the header region.
- Entity names are UTF-16LE.
- Important help-book entities are `PackBlock`, `FileStorage` and `Book`.

Reference anchors:

- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/hbk/reader/ContainerReader.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/hbk/reader/HbkContentReader.kt`
- `/home/alko/develop/open-source/hbk-reader/doc/hbk-format.md`
- `/home/alko/develop/open-source/hbk-reader/doc/hbk-binary-format.md`

These local paths are evidence anchors for the current planning stage, not normative dependencies.
Replace them with repo-local validated evidence when the Rust implementation covers the same facts.

## Book and Navigation Observations

Useful `hbk-reader` concepts:

- `Toc` and page tree with localized titles and HTML paths.
- Page lookup by HTML path and by index path.
- Book metadata fields such as `bookName`, `description` and `tags`.
- Filename locale inference: `_ru` maps to `ru`, `_root` is the default/root source and exports as
  `en`.

Reference anchors:

- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/hbk/reader/toc/TocParser.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/hbk/reader/toc/Toc.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/hbk/reader/meta/BookMetaParser.kt`
- `/home/alko/develop/open-source/hbk-reader/doc/models.md`

## Syntax Assistant Observations

`hbk-reader` splits Syntax Assistant parsing by page type:

- object/type pages
- method pages
- property pages
- constructor pages
- enum pages
- enum value pages
- global context pages

Use these observations to validate root discovery, catalog traversal and specialized parsing. Prefer
data-driven detection over copying brittle filename or title assumptions.

Reference anchors:

- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/shctx/PlatformContextReader.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/shctx/PlatformContextPagesVisitor.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/shctx/parsers/PlatformContextPagesParser.kt`
- `/home/alko/develop/open-source/hbk-reader/src/main/kotlin/ru/alkoleft/v8/platform/shctx/parsers/specialized/*.kt`

## Export and Lookup Observations

`platform-context-exporter` gives useful consumer-oriented DTO and lookup examples:

- global properties
- global methods
- platform types
- signatures, parameters, properties and methods
- exact search and member lookup contracts

Reference anchors:

- `/home/alko/develop/open-source/bsl-context-multi-project/platform-context-exporter/documentation/formats.md`
- `/home/alko/develop/open-source/bsl-context-multi-project/platform-context-exporter/src/main/java/ru/alkoleft/context/platform/dto/*.java`
- `/home/alko/develop/open-source/bsl-context-multi-project/platform-context-exporter/src/main/java/ru/alkoleft/context/platform/mcp/PlatformApiSearchService.java`

Legacy DTO shape is an adapter option for concrete consumers, not a constraint on the internal Rust
model.

## Syntax Assistant Query CLI Evidence

On 2026-04-30 the current CLI export was rechecked against the Russian Syntax Assistant source:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/analysis/syntax-helper-cli-requirement-20260430/ru
```

The command exited successfully and measured `19.12s / 588836 KiB` through GNU `time` in the debug
profile. The generated directory was service data and contained 10 JSON files, about 21 MiB total.

Current record counts:

| File | Record kind | Records |
| --- | --- | ---: |
| `global-methods.json` | `global_method` | 500 |
| `global-properties.json` | `global_property` | 101 |
| `platform-types.json` | `platform_type` | 2533 |
| `type-methods.json` | `type_method` | 6702 |
| `type-properties.json` | `type_property` | 10732 |
| `constructors.json` | `constructor` | 445 |
| `enums.json` | `enum` | 713 |
| `enum-values.json` | `enum_value` | 3110 |
| `diagnostics.json` | `diagnostic` | 703 |

Current consumer record fields are enough for a first local search index:

- `name.primary` and `name.alias` support exact and fuzzy name search;
- `owner` on type members, constructors and enum values supports owner/member lookup;
- `signatures`, parameter names and parameter `types` support signature search;
- callable `return` and property `types` support type-reference relationships;
- `description` supports keyword and purpose-oriented lexical search.

Observed data-composition filter facts for relationship-search design:

- `ОтборКомпоновкиДанных` / `DataCompositionFilter` exists as a `platform_type` and is described as
  the object used for record filtering.
- `НастройкиКомпоновкиДанных.Отбор` exists as a type property whose type reference is
  `ОтборКомпоновкиДанных`.
- `ОтборКомпоновкиДанных.Элементы` exists as a type property whose type reference is
  `КоллекцияЭлементовОтбораКомпоновкиДанных`.
- `КоллекцияЭлементовОтбораКомпоновкиДанных.Добавить` exists as a method that adds a new item and
  returns it.
- `ЭлементОтбораКомпоновкиДанных` exposes comparison-item properties such as `ЛевоеЗначение`,
  `ВидСравнения`, `ПравоеЗначение` and `Использование`.

Current export limitations for the query CLI:

- Consumer record-family files intentionally omit per-record HBK provenance and navigation links per
  FR-EXPORT-001.
- `hbk-docs` can resolve generic page links, but the current Syntax Assistant extraction path
  creates `PageContent` with empty generic `links`.
- `syntax-helper-extract` parses section member links for global context, platform types and enums
  in the provenance-rich model, but the lean consumer export omits those link lists.
- "See also" links are currently flattened into description text, not emitted as structured
  relationships.

Therefore the first search CLI can be built from the current canonical export for exact lookup,
keyword/fuzzy search and owner/type-reference relationships. A richer relationship graph requires
either a search-specific indexed artifact built during extraction or an enriched maintenance/search
export that preserves structured section and "see also" links outside the lean consumer files.

## Syntax Assistant Export Completeness Audit

On 2026-04-30 the current release CLI export was rechecked against both Syntax Assistant source
locales:

```bash
target/release/v8-context-hbk syntax-helper \
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/analysis/export-audit-20260430/shcntx-ru
target/release/v8-context-hbk syntax-helper \
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk \
  --output target/analysis/export-audit-20260430/shcntx-en
```

Both commands exited successfully and produced the same record-family counts:

| File | RU records | EN/root records |
| --- | ---: | ---: |
| `global-methods.json` | 500 | 500 |
| `global-properties.json` | 101 | 101 |
| `platform-types.json` | 2533 | 2533 |
| `type-methods.json` | 6702 | 6702 |
| `type-properties.json` | 10732 | 10732 |
| `constructors.json` | 445 | 445 |
| `enums.json` | 713 | 713 |
| `enum-values.json` | 3110 | 3110 |
| `diagnostics.json` | 703 | 703 |

Audit findings promoted to follow-up tasks:

- Root/English type extraction is incomplete. `global-methods.json`, `type-methods.json`,
  `global-properties.json` and `type-properties.json` contain zero return/type references in the
  root export even when source HTML contains English `Type:` and `Returned value:` sections.
- Russian type extraction is materially better but still incomplete on some records: 143 global
  methods, 2494 type methods, 1 global property and 169 type properties have empty return/type
  reference arrays in the current export.
- Description fields currently swallow later HTML sections. For example `XMLString` / `XMLСтрока`
  and `Array` / `Массив` descriptions include availability, examples and see-also/version text
  instead of preserving those facts separately.
- No consumer record-family file currently exposes structured `availability`, `examples`,
  `see_also`, `available_since` or overload/variant metadata fields.
- Overload/syntax-variant parsing is not structurally correct on variant-heavy pages. For
  `ДокументDOM.СоздатьРазыменовательПИ` / `DOMDocument.CreateNSResolver`, signature text contains
  raw labels/prose such as variant descriptions and `Syntax:`, and parameter descriptions absorb
  following variant text. The current export reports 9 Russian type methods and 1690 English/root
  type methods with signature lines that contain raw section labels or returned-value prose.
- Diagnostics remain deterministic and provenance-rich, but all 703 diagnostics in each locale are
  currently `UNKNOWN_PAGE_CLASS`. They include 4 direct `objects/Global context/*.html` pages that
  look like global context methods and 33 global-context event pages; table field/parameter pages
  dominate the remaining diagnostics. These pages need an explicit in-scope/out-of-scope
  classification pass before the extraction can be called complete.

T25 follow-up on 2026-04-30 fixed the locale-aware section parser for root/English `Type:` and
`Returned value:` labels and extended shared section boundaries for availability, examples,
see-also, available-since and overload variant labels. The consumer JSON schema remains
`schema_version: 1`; no HBK provenance, TOC paths, HTML paths or page titles were added to consumer
record-family files.

Post-T25 empty type-reference counts:

| File / field | RU before | RU after | EN/root before | EN/root after |
| --- | ---: | ---: | ---: | ---: |
| `global-methods.json` / empty `return_types` | 143 | 143 | 500 | 147 |
| `type-methods.json` / empty `return_types` | 2494 | 2494 | 6702 | 2520 |
| `global-properties.json` / empty `type_refs` | 1 | 1 | 101 | 1 |
| `type-properties.json` / empty `type_refs` | 169 | 169 | 10732 | 165 |

T25 verification used the existing real-source audit fixtures from
`tests/fixtures/syntax-helper/manifest.tsv` and full CLI exports for both
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`. `XMLСтрока` / `XMLString`,
`Массив.Добавить` / `Array.Add` and `ОткрытьФорму` / `OpenForm` now retain the T25 type facts in
both locales.

T26 follow-up on 2026-04-30 added schema version 2 structured section facts for
`availability`, `examples`, `see_also` and `available_since` in consumer record-family files.
Consumer records still omit HBK provenance, TOC paths, HTML paths and page titles; `see_also`
consumer targets expose names only. Overload variant metadata remains intentionally pending for
T27.

Post-T26 structured fact counts from full CLI exports:

| File | RU availability | RU examples | RU see_also | RU available_since | EN availability | EN examples | EN see_also | EN available_since |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `global-methods.json` | 499 | 185 | 180 | 500 | 499 | 193 | 180 | 500 |
| `global-properties.json` | 101 | 0 | 21 | 101 | 101 | 0 | 21 | 101 |
| `platform-types.json` | 2384 | 140 | 895 | 2532 | 2384 | 140 | 900 | 2532 |
| `type-methods.json` | 6586 | 1103 | 690 | 6701 | 6587 | 1102 | 691 | 6701 |
| `type-properties.json` | 9918 | 19 | 222 | 10731 | 9918 | 31 | 222 | 10731 |
| `constructors.json` | 2 | 54 | 10 | 315 | 2 | 55 | 10 | 315 |
| `enums.json` | 713 | 3 | 341 | 713 | 713 | 2 | 341 | 713 |
| `enum-values.json` | 28 | 0 | 36 | 3109 | 28 | 3 | 36 | 3109 |

T27 follow-up on 2026-04-30 added schema version 3 structured syntax-variant metadata as
`signatures[].variant` with `title` and `description`. Consumer records still omit HBK provenance,
TOC paths, HTML paths and page titles. `ДокументDOM.СоздатьРазыменовательПИ` /
`DOMDocument.CreateNSResolver` now exports four callable variant signatures in both locales, with
parameters bound to the owning variant and return types preserved. `ОткрытьФорму` / `OpenForm`
now exports both current syntax variants.

Post-T27 CLI export counts remained stable for both books. Full RU/root exports expose structured
variant metadata on 266 records and 604 signatures in each locale:

| File | RU records with variants | RU variant signatures | EN/root records with variants | EN/root variant signatures |
| --- | ---: | ---: | ---: | ---: |
| `global-methods.json` | 23 | 60 | 23 | 60 |
| `type-methods.json` | 243 | 544 | 243 | 544 |
| `constructors.json` | 0 | 0 | 0 | 0 |

Signature text containing raw overload section labels or returned-value labels stayed at zero in
the post-T26 baseline and remains zero after T27 for both locales.

T28 follow-up on 2026-04-30 classified the remaining 703 diagnostics in each locale into stable
source families. Record-family counts and schema version 3 stayed unchanged. The remaining
diagnostics are no longer generic `UNKNOWN_PAGE_CLASS` records for the audited families:

| Diagnostic code | RU count | EN/root count | T28 decision |
| --- | ---: | ---: | --- |
| `UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE` | 4 | 4 | In FR-SH-002 scope, but current source TOC entries are direct `objects/Global context/*.html` method-like pages outside the supported method catalog layout. The audited HBK FileStorage does not contain these page HTML entries, so the extractor reports an explicit recoverable gap instead of synthesizing incomplete method records from TOC only. |
| `OUT_OF_SCOPE_GLOBAL_CONTEXT_EVENT` | 33 | 33 | T28 classified this family as out of scope; T29 later promoted it into `global-context-events.json`. |
| `OUT_OF_SCOPE_TABLE_FIELD` | 588 | 588 | T28 classified this family as out of scope; T29 later promoted it into `table-fields.json`. |
| `OUT_OF_SCOPE_TABLE_PARAMETER` | 78 | 78 | T28 classified this family as out of scope; T29 later promoted it into `table-parameters.json`. |

The T28 classification used the existing Syntax Assistant export audit fixture set in
`tests/fixtures/syntax-helper/manifest.tsv`. No new parser HTML fixture was added because the only
new in-scope family found in diagnostics is represented by TOC entries whose corresponding page
HTML is absent from both audited FileStorage archives.

T29 follow-up on 2026-04-30 promoted global context events, query/table fields and query/table
parameters into typed extraction/export records. Full CLI exports for
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` now produce:

| Record family | RU count | EN/root count |
| --- | ---: | ---: |
| `global-context-events.json` | 33 | 33 |
| `table-fields.json` | 588 | 588 |
| `table-parameters.json` | 78 | 78 |

The remaining diagnostic count is 4 in each locale, all
`UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE`. `UNKNOWN_PAGE_CLASS`,
`OUT_OF_SCOPE_GLOBAL_CONTEXT_EVENT`, `OUT_OF_SCOPE_TABLE_FIELD` and
`OUT_OF_SCOPE_TABLE_PARAMETER` are absent from the T29 exports.

T32 follow-up on 2026-04-30 switched the consumer export to lean schema version 5. Full CLI exports
for both Syntax Assistant source locales still produce the same platform API record-family counts,
but `enum-values.json` is no longer emitted and all 3110 enum values are nested under owning records
in `enums.json` for both locales.

| File | RU records | EN/root records |
| --- | ---: | ---: |
| `global-methods.json` | 500 | 500 |
| `global-properties.json` | 101 | 101 |
| `global-context-events.json` | 33 | 33 |
| `platform-types.json` | 2533 | 2533 |
| `type-methods.json` | 6702 | 6702 |
| `type-properties.json` | 10732 | 10732 |
| `table-fields.json` | 588 | 588 |
| `table-parameters.json` | 78 | 78 |
| `constructors.json` | 445 | 445 |
| `enums.json` | 713 records / 3110 nested values | 713 records / 3110 nested values |
| `diagnostics.json` | 4 | 4 |

The current lean consumer records omit `null` fields and empty arrays, expose `owner` as a
primary-name string, expose type references as `types` and callable returns as `return`, emit
recognized version facts as `availability.since`, flatten `see_also` to target primary-name
strings, normalize property `usage`, remove callable `signatures[].text`, flatten syntax-variant
metadata onto signatures and keep parser provenance only in `diagnostics.json`.

T30 follow-up on 2026-04-30 removed the post-T29 table-owner lookup regression without changing the
consumer JSON output. The pre-fix T32 release-profile exports measured `8.00s / 181124 KiB` for
`shcntx_ru.hbk` and `7.71s / 126064 KiB` for `shcntx_root.hbk`. After replacing per-record
`Toc::find_by_html_path` owner resolution with one extraction-scope TOC HTML-path index, the same
release-profile commands measured `4.76s / 167452 KiB` and `3.62s / 131748 KiB`. The post-fix
exports were byte-identical to the pre-fix T32 exports, including 588 table fields, 78 table
parameters and 4 remaining `UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE` diagnostics per locale.

T31 follow-up on 2026-04-30 remeasured the residual post-T30 parser/export path before changing
parser code. Release-profile exports measured `4.96s / 151644 KiB` for `shcntx_ru.hbk` and
`3.68s / 128828 KiB` for `shcntx_root.hbk`; a root repeat measured `3.90s / 127780 KiB` and was
byte-identical to the first root export. The residual path stayed in the T28/T30 performance class,
so no additional parser/export optimization was accepted in T31.

T33 follow-up on 2026-04-30 changed the consumer export to schema version 6 and fixed data-quality
issues found in real `shcntx_ru.hbk` JSON exports. Type-reference facts are now serialized as
`types`; callable return facts are serialized as `return`; legacy `type_refs` and `return_types`
are absent from consumer JSON. Inline example sections embedded in descriptions are extracted
without absorbing later availability sections, syntax-colored code examples no longer contain
extra spaces around BSL punctuation, and see-also owner/member source links are composed as
`Owner.Member` target strings. Full RU/root CLI exports kept the T32/T31 record-family counts and
4 remaining diagnostics per locale.

## Syntax Assistant TOC-Aware Reading Findings

On 2026-05-01 the `schema_version: 6` Russian consumer export under `/tmp/shcntx/` was reviewed
against FR-EXPORT-001 and the T33/T34 baseline. The basic file/count invariants still matched the
accepted export contract: 11 record-family files plus `metadata.json`, `locale=ru`, record counts
matching the baseline and 4 `UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE` diagnostics.

The review found reading-level ambiguity that the lean export only makes visible:

- `table-parameters.json` records `21`, `45` and `66` all expose
  `owner="Таблица остатков и оборотов"` and `name.primary="Метод дополнения периодов"`. They are
  not distinguishable as consumer facts, which points to insufficient TOC-derived query table
  ownership.
- `global-context-events.json` repeats event names and aliases such as
  `ПриНачалеРаботыСистемы` / `OnStart`, `ПриЗавершенииРаботыСистемы` / `OnExit`,
  `ОбработкаВнешнегоСобытия` / `ExternEventProcessing` and
  `ПередНачаломРаботыСистемы` / `BeforeStart` with different descriptions and availability facts.
  The reader currently does not preserve the TOC branch distinction as a semantic context.
- `platform-types.json` contains same-name entries such as `ПередЗаписью` / `BeforeWrite`,
  `ПослеЗаписи` / `AfterWrite`, `ПриЧтенииНаСервере` / `OnReadAtServer` and
  `Расширение элементов управления, расположенных в форме.<Имя события>` where the source pages are
  under different TOC branches but the extracted fact identity is name-only.
- Placeholder-like records in query tables, form elements and external data source constructors are
  only safe if the reader carries their semantic owner path. Examples include repeated
  `owner="Основная таблица", name.primary="<Имя измерения>"` table fields and repeated
  `ЭлементыФормы.<Имя элемента управления>` type properties.

The durable conclusion is FR-SH-003: fix Syntax Assistant reading/classification first. Raw
`toc_path`, `html_path`, page title and source HBK path are parser provenance, not semantic
disambiguators for consumer records. The reader must derive source family, semantic owner and
branch context from the TOC hierarchy before an export or index adapter sees the record.

The accepted classification direction for T35 is:

- classify TOC in two layers: branch kind and record family;
- model events under session/application/object/form/service module branches as `module_event`,
  not as global context members;
- treat Automation/external API as a branch category containing ordinary platform types and
  members, not as its own record family;
- distinguish platform type kinds at least as `regular`, `extension`, `primitive` and
  `metadata_template`;
- treat `Расширение...` / `Extension...` pages as extension types and derive `extends` only from
  reliable TOC/HTML/link evidence;
- treat application/metadata placeholder types such as `ДокументОбъект.<Имя документа>` as
  metadata-template types, optionally with metadata kind and template parameters when derivable;
- read `Примитивные типы` shallowly: direct children are primitive types, nested literals such as
  `Булево > Истина` and `Булево > Ложь` are not ordinary platform types.

T35 follow-up implemented the accepted TOC-aware reading direction and raised the lean consumer
export to `schema_version: 7`. Fresh debug CLI exports for
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` produced:

| File | RU records | EN/root records |
| --- | ---: | ---: |
| `global-context-events.json` (`record_kind=module_event`) | 697 | 697 |
| `platform-types.json` | 1869 | 1869 |
| `table-fields.json` | 588 | 588 |
| `table-parameters.json` | 78 | 78 |
| `diagnostics.json` | 4 | 4 |

The Russian platform-type kind split was 1470 `regular`, 278 `extension` and 121
`metadata_template`. The target source did not expose a primitive-type catalog as typed platform
records in this pass; primitive traversal is still guarded so nested literal pages such as
`Истина` and `Ложь` are not emitted as platform types.

The T35 UAT checks confirmed that the three `Метод дополнения периодов` table parameters under
`Таблица остатков и оборотов` now have distinct TOC-derived owner paths, repeated
`ПриНачалеРаботыСистемы` records carry distinct module kinds, and `ПередЗаписью` / `BeforeWrite`
records moved from name-only platform-type facts into module-event records with semantic owner
paths. Consumer records still omit `source_hbk`, `toc_path`, `html_path` and `page_title`; parser
diagnostics remain provenance-rich.

The follow-up review found two root/English string-classification edge cases and the guard was
extended accordingly. `Client application form...` owner paths must classify module events as
`form`, not `managed_application`, and `Information` suffixes must not match the managed-forms
branch by substring. After the fix, the root/English platform-type branch split was 1383
`platform_objects`, 288 `managed_forms`, 101 `system_enums`, 96 `metadata_objects` and 1
`automation_external_api`; `BinaryDataStorageInformation` classified as `platform_objects`.

## Query Table Export Shape Findings

A 2026-05-04 review of the Syntax Assistant query table TOC structure found that the flat
`table-fields.json` / `table-parameters.json` shape is not the right consumer representation for
schema v8. Query table owners are real table pages, and some owner families contain both a generic
primary table and additional tables. Observed examples include:

- `Таблицы задач > Основная таблица`
- `Таблицы задач > Таблица задач по исполнителю`
- `Таблицы последовательностей > Основная таблица`
- `Таблицы последовательностей > Таблица границ`
- `Таблицы внешнего источника данных > Таблица внешнего источника данных`

The durable conclusion for T36 is that query tables should be exported as owning records in
`query-tables.json`, with fields and parameters nested under the owning table. `owner_path` belongs
on the table record, not on every nested field or parameter. Generic names such as `Основная таблица`
remain safe because the table record carries the semantic owner path.

The reviewed query table families did not show a need for localized-name alias objects on table,
field or parameter names, so schema v8 should use string names for this source family unless future
source evidence proves aliases. The existing query table parameter `required` field also lacks a
clear source-backed contract and should be removed from schema v8 consumer JSON.

T36 implemented this shape on 2026-05-04. Fresh debug CLI exports for
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` both produced 59 `query-tables.json` records with
588 nested fields and 78 nested parameters. The T36 checks confirmed:

- `metadata.json.files` lists `query-tables.json` and does not list `table-fields.json`,
  `table-parameters.json` or `enum-values.json`.
- table records use string `name`, table-family `owner_path` and `table_role`;
- nested field and parameter records use string `name`, omit `owner_path`, and query table
  parameters omit `required`;
- normal exports preserve stale files from older schema versions when an output directory is reused.

A later 2026-05-04 review found that schema v10 still classified query table roles from page names
instead of the table syntax. The source page `tables/table58.html` has title
`БизнесПроцесс.<Имя бизнес-процесса> (BusinessProcess.<Имя бизнес-процесса>)`, a `Синтаксис`
section with the same expression and display table name `Таблица бизнес-процессов`. Its root-source
counterpart has `Syntax` value `BusinessProcess.<Business process name>` and display table name
`Business Process Table`. These are primary tables by syntax shape even though their page names are
not `Основная таблица` / `Main table`.

The same review found additional table pages whose syntax extends the primary table syntax, for
example `БизнесПроцесс.<Имя бизнес-процесса>.Точки` /
`BusinessProcess.<Business process name>.Points`. Query table extraction therefore needs to keep the
table syntax and derive a deterministic table identifier from syntax plus page name rather than
using the display table name as the only consumer lookup key. A follow-up T40 review clarified that
the page-name suffix must be CamelCase-normalized, not only whitespace-compacted; for example
`Таблица изменений бизнес-процессов` contributes `ТаблицаИзмененийБизнесПроцессов`.

The table syntax itself follows the same dual-language source pattern as other Syntax Assistant
facts: Russian pages may contain a Russian expression plus a parenthesized English alias. For
example, `РегистрРасчета.<Имя регистра расчета>.<Имя перерасчета>.Изменения
(CalculationRegister.<Имя регистра расчета>.<Имя перерасчета>.Changes)` should be split into
localized `syntax.primary` and `syntax.alias` instead of exported as one combined string.

## Event File Split Findings

T37 implemented the schema version 9 event file split on 2026-05-04. Fresh CLI exports for
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` both produced:

- 47 records in `module-events.json`;
- 650 records in `type-events.json`;
- 0 records in `unknown-events.json`.

The durable source-backed split is:

- Global context event groups and explicit module branches such as session/application/service
  modules are module events.
- Event pages under type/object/form/form-extension/control branches without explicit module TOC
  context are type events.
- No target 8.5.1.1150 event pages required the unknown-event fallback after TOC-aware
  classification.

The T37 checks confirmed that `metadata.json.files` no longer lists
`global-context-events.json`, event consumer records omit raw HBK, TOC, HTML and page-title
provenance, and no event record carries `id`, `owner_ref` or event-local `owner_kind`. Type event
records carry `owner` and semantic `owner_path` as event owner context only; source-backed
owner/object classification for the owning platform type/object records remains a separate T38
task.

T39 changed the canonical export to schema version 10 and removed `owner_path` from
`type-events.json`. The fresh Russian export kept 47 module events, 650 type events and 0 unknown
events. Type-event owner context is now composed into a single `owner` string, so
`type-events.json`, type methods, type properties, constructors and nested query table records omit
`owner_path` while type events remain unique by `(owner, name.primary, name.alias)`.

## Owner/Object Classification Findings

T38 implemented source-backed `object_kind` classification on `platform-types.json` records only.
The field is derived from TOC branch context after platform type classification and is omitted when
the source branch does not prove a supported owner/object kind. Event records still do not carry
`owner_kind`, `object_kind`, `id`, `owner_ref` or raw HBK/TOC/HTML/page-title provenance.

Fresh debug CLI exports for `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` kept the T37 record-family counts and produced the
following platform owner/object classifications:

| `object_kind` | RU records | EN/root records |
| --- | ---: | ---: |
| `regular_platform_type` | 1305 | 1357 |
| `managed_form` | 77 | 286 |
| `form_extension` | 174 | 2 |
| `metadata_object` | 287 | 96 |

The T38 pass also narrowed system-enum branch detection to the actual `objects/catalog2` enum root
and children. This prevents ordinary `objects/catalog2xx...` platform object paths such as
universal collection pages from being classified as system-enum branch records by prefix alone.

## Query Index Identity Findings

T41 verified the first search-index identity contract against
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` on 2026-05-04.

The real source data contains duplicate query table identifiers in semantic table families. The
accounting-register families with and without correspondence support share identifiers such as
`РегистрБухгалтерии`, so the search index must append the minimal table-family `owner_path` variant
for those duplicated identities. Query table fields and parameters can then use that final table
identity as their owner and relation endpoint without raw source-path suffixes.

The same pass found that some form/form-extension `Параметры формы` pages do not use `/params/` in
their HTML path. Classification must therefore use the semantic TOC ancestor `Параметры формы` /
`Form parameters`, not only path fragments. Those pages represent form parameters or attributes
owned by the preceding form or form-extension type and must not be indexed or exported as platform
types.
