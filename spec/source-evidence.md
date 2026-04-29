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
- `signatures`, parameter names and parameter `type_refs` support signature search;
- `return_types` and property `type_refs` support type-reference relationships;
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
