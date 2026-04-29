# UAT Test Cases

These test cases describe user-visible behavior. They are black-box scenarios and should be usable
by a human or agent without knowing the internal crate structure.

## UAT-HBK-001: Inspect Small Root Help Book

Related use case: UC-HBK-001.

Related requirements: FR-HBK-001, FR-CLI-001, NFR-DIAG-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk` exists.
- The CLI is runnable through Cargo.

Steps:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk
```

Expected result:

- Exit code is `0`.
- Output includes `PackBlock`, `FileStorage` and `Book`.
- Output is readable as an inspection result, not a panic/backtrace.

Skip rule:

- If the fixture is absent, record the skip reason and do not mark the case failed.

## UAT-HBK-002: Print Russian Help Book TOC as JSON

Related use case: UC-HBK-002.

Related requirements: FR-HBK-002, FR-HBK-003, FR-CLI-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists.

Steps:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json
```

Expected result:

- Exit code is `0`.
- Output parses as JSON.
- At least one TOC item contains a title and an `html_path`.

## UAT-HBK-003: Read a Known Help Page

Related use case: UC-HBK-002.

Related requirements: FR-HBK-002, FR-DOC-001, FR-CLI-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists.
- `tests/fixtures/known-pages/fmtdui_ru.page` exists.

Steps:

```bash
PAGE_PATH="$(cat tests/fixtures/known-pages/fmtdui_ru.page)"
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --path "$PAGE_PATH"
```

Expected result:

- Exit code is `0`.
- Output is non-empty HTML/text content from the requested page.

## UAT-SH-001: Export Russian Syntax Assistant Data

Related use case: UC-SH-001.

Related requirements: FR-SH-001, FR-SH-002, FR-EXPORT-001, NFR-DIAG-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- `target/uat/shcntx-ru` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-ru
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
```

Expected result:

- Exit code is `0`.
- Required JSON files from FR-EXPORT-001 exist.
- `metadata.json` records locale `ru`.
- Core record-family files parse as JSON and contain non-empty `records`.
- `diagnostics.json` is present and parser gaps are visible if any remain.
- Consumer record-family files expose platform API facts only and do not include book hierarchy,
  per-record source provenance or duplicate navigation-link catalogs.

Cleanup:

- `target/uat/shcntx-ru` is service data and may be deleted after the run.

## UAT-SH-002: Export Root Syntax Assistant Data as English Locale

Related use case: UC-SH-001.

Related requirements: FR-SH-002, FR-EXPORT-001, ADR-0001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` exists.
- `target/uat/shcntx-en` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-en
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/uat/shcntx-en
```

Expected result:

- Exit code is `0`.
- Required JSON files from FR-EXPORT-001 exist.
- `metadata.json` records export locale `en`.
- Core record-family files parse as JSON and contain non-empty `records`.
- Consumer record-family files expose platform API facts only and do not include book hierarchy,
  per-record source provenance or duplicate navigation-link catalogs.

Cleanup:

- `target/uat/shcntx-en` is service data and may be deleted after the run.

## UAT-SH-003: Export Shape Omits HBK Navigation and Provenance from Consumer Records

Related use case: UC-SH-001.

Related requirements: FR-EXPORT-001, NFR-PERF-001, NFR-DIAG-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- `target/uat/shcntx-ru` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-ru
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
test ! -f target/uat/shcntx-ru/global-contexts.json
jq -e 'has("source_hbk") | not' target/uat/shcntx-ru/metadata.json
jq -e '
  def forbidden:
    has("source") or
    has("source_hbk") or
    has("toc_path") or
    has("html_path") or
    has("page_title") or
    has("method_links") or
    has("constructor_links") or
    has("value_links");
  all(.records[]; forbidden | not)
' target/uat/shcntx-ru/platform-types.json
jq -e 'has("source_hbk") | not' target/uat/shcntx-ru/platform-types.json
jq -e '
  def forbidden:
    has("source") or
    has("source_hbk") or
    has("toc_path") or
    has("html_path") or
    has("page_title") or
    has("method_links") or
    has("constructor_links") or
    has("value_links");
  all(.records[]; forbidden | not)
' target/uat/shcntx-ru/type-methods.json
jq -e 'has("source_hbk") | not' target/uat/shcntx-ru/type-methods.json
jq -e '
  def forbidden:
    has("source") or
    has("source_hbk") or
    has("toc_path") or
    has("html_path") or
    has("page_title") or
    has("method_links") or
    has("constructor_links") or
    has("value_links");
  all(.records[]; forbidden | not)
' target/uat/shcntx-ru/enums.json
jq -e 'has("source_hbk") | not' target/uat/shcntx-ru/enums.json
jq -e 'has("source_hbk") | not' target/uat/shcntx-ru/diagnostics.json
jq -e '
  (.records | length > 0) and
  all(.records[];
    has("source") and
    (.source | has("hbk_path") and has("locale") and has("html_path") and has("page_title"))
  )
' target/uat/shcntx-ru/diagnostics.json
```

Expected result:

- Exit code is `0`.
- `global-contexts.json` is not produced as a consumer export file.
- Consumer record-family files parse as JSON.
- `metadata.json` and consumer record-family envelopes do not expose source HBK paths.
- Consumer records contain only platform API facts and do not contain the forbidden provenance,
  hierarchy or navigation-link fields.
- `diagnostics.json` remains present, has no top-level `source_hbk` envelope field and keeps parser
  source context on diagnostic records.

Cleanup:

- `target/uat/shcntx-ru` is service data and may be deleted after the run.

## UAT-SH-004: Build a Syntax Assistant Search Index

Related use case: UC-SH-003.

Related requirements: FR-SH-SEARCH-001, NFR-QUERY-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- The Syntax Assistant export command is runnable.
- The separate Syntax Assistant query CLI is runnable as `v8-sh` or the accepted ADR-0004 binary
  name.
- `target/uat/shcntx-ru` and `target/uat/sh-search-ru` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-ru target/uat/sh-search-ru
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
v8-sh index target/uat/shcntx-ru --output target/uat/sh-search-ru
```

Expected result:

- Exit code is `0`.
- The index directory contains a manifest and deterministic search/relationship artifacts.
- The index build records locale `ru`, source locale `ru` and source export schema version.
- The query CLI does not require the HBK file path for later lookup commands.

Cleanup:

- `target/uat/shcntx-ru` and `target/uat/sh-search-ru` are service data and may be deleted after the
  run.

## UAT-SH-005: Exact Syntax Assistant Lookup

Related use case: UC-SH-003.

Related requirements: FR-SH-SEARCH-001, NFR-QUERY-001.

Preconditions:

- `target/uat/sh-search-ru` exists from UAT-SH-004.

Steps:

```bash
v8-sh get --index target/uat/sh-search-ru --name "ОтборКомпоновкиДанных" --format json
v8-sh get --index target/uat/sh-search-ru --name "DataCompositionFilter" --format json
v8-sh get --index target/uat/sh-search-ru --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json
```

Expected result:

- Exit code is `0`.
- The first two commands return the same platform type fact.
- The returned type fact contains primary name `ОтборКомпоновкиДанных`, alias
  `DataCompositionFilter` and a non-empty description.
- The owner/member command returns a type property whose type reference includes
  `ОтборКомпоновкиДанных`.
- The commands return within the NFR-QUERY-001 provisional target when measured on the target
  workstation.

## UAT-SH-006: Relationship Search for SKD Filter Creation

Related use case: UC-SH-004.

Related requirements: FR-SH-SEARCH-001, FR-SH-SEARCH-002, NFR-QUERY-001.

Preconditions:

- `target/uat/sh-search-ru` exists from UAT-SH-004.

Steps:

```bash
v8-sh search --index target/uat/sh-search-ru --query "отбор скд" --mode keywords --format json
v8-sh related --index target/uat/sh-search-ru --name "ОтборКомпоновкиДанных" --format json
```

Expected result:

- Exit code is `0`.
- Keyword search ranks data-composition filter facts ahead of unrelated DOM/file dialog filter
  facts.
- Relationship output for `ОтборКомпоновкиДанных` includes:
  - constructor `Новый ОтборКомпоновкиДанных()`;
  - property `Элементы` with type reference `КоллекцияЭлементовОтбораКомпоновкиДанных`;
  - method facts for `ПолучитьКоличествоИспользуемых`, `ПолучитьИдентификаторПоОбъекту` and
    `ПолучитьОбъектПоИдентификатору`.
- The relationship path needed for filter item creation is discoverable through
  `КоллекцияЭлементовОтбораКомпоновкиДанных.Добавить` and `ЭлементОтбораКомпоновкиДанных`
  properties such as `ЛевоеЗначение`, `ВидСравнения`, `ПравоеЗначение` and `Использование`.
- The command returns within the NFR-QUERY-001 provisional target when measured on the target
  workstation.

## UAT-ERR-001: Missing File Produces Readable CLI Error

Related use cases: UC-HBK-001, UC-HBK-002, UC-SH-001.

Related requirements: FR-CLI-001, NFR-REL-001, NFR-DIAG-001.

Steps:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect target/uat/does-not-exist.hbk
```

Expected result:

- Exit code is non-zero.
- Error includes the requested path.
- Error is a readable diagnostic, not a panic/backtrace.
