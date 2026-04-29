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
