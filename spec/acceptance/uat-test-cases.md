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
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
```

Expected result:

- Exit code is `0`.
- Required JSON files from FR-EXPORT-001 exist.
- The command summary reports parser-maintenance warning count as `parser_warnings`.
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
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/uat/shcntx-en
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
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
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
for file in \
  target/uat/shcntx-ru/module-events.json \
  target/uat/shcntx-ru/type-events.json \
  target/uat/shcntx-ru/unknown-events.json; do
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
  ' "$file"
  jq -e 'has("source_hbk") | not' "$file"
done
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
' target/uat/shcntx-ru/query-tables.json
jq -e 'has("source_hbk") | not' target/uat/shcntx-ru/query-tables.json
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
- The `v8-context-hbk syntax index` command is runnable.
- `target/uat/sh-search-ru.sqlite` can be created or removed.
- `.v8-context-hbk/syntax/index.sqlite` can be created or removed.

Steps:

```bash
rm -f target/uat/sh-search-ru.sqlite
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk

rm -rf .v8-context-hbk
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk
test -f .v8-context-hbk/syntax/index.sqlite
test ! -e .v8-context-hbk/syntax/index.sqlite-wal
test ! -e .v8-context-hbk/syntax/index.sqlite-shm
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --name "ОтборКомпоновкиДанных" --format json
```

Expected result:

- Exit code is `0`.
- The index artifact is a SQLite database.
- The database contains schema metadata plus deterministic document, exact-name, FTS and
  relationship-edge data.
- The index build records locale `ru`, source locale `ru`, source HBK identity and index/extraction
  schema version.
- The index command uses the effective index path from `V8_CONTEXT_HBK_SYNTAX_INDEX` when `--output`
  is omitted.
- The index command creates `.v8-context-hbk/syntax/index.sqlite` when both `--output` and
  `V8_CONTEXT_HBK_SYNTAX_INDEX` are absent.
- The completed replacement index does not leave active SQLite WAL/SHM sidecars beside the default
  artifact.
- The default-path lookup command resolves `.v8-context-hbk/syntax/index.sqlite` when `--index` and
  `V8_CONTEXT_HBK_SYNTAX_INDEX` are absent.
- Later query commands do not require the HBK file path.

Cleanup:

- `target/uat/sh-search-ru.sqlite` and `.v8-context-hbk/` are service data and may be deleted after
  the run.

## UAT-SH-005: Exact Syntax Assistant Lookup

Related use case: UC-SH-003.

Related requirements: FR-SH-SEARCH-001, NFR-QUERY-001.

Preconditions:

- `target/uat/sh-search-ru.sqlite` exists from UAT-SH-004.

Steps:

```bash
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --name "ОтборКомпоновкиДанных" --format json > target/uat/get-filter-primary.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --name "DataCompositionFilter" --format json > target/uat/get-filter-alias.json
cmp target/uat/get-filter-primary.json target/uat/get-filter-alias.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json
```

Expected result:

- Exit code is `0`.
- The first two commands return the same platform type fact.
- The unqualified exact lookup does not mix same-name owned member facts into the returned platform
  type result.
- The returned type fact contains primary name `ОтборКомпоновкиДанных`, alias
  `DataCompositionFilter` and a non-empty description.
- The owner/member command returns a type property whose type reference includes
  `ОтборКомпоновкиДанных`.
- The commands resolve the index path from `V8_CONTEXT_HBK_SYNTAX_INDEX` when `--index` is omitted.
- The commands return within the NFR-QUERY-001 provisional target when measured on the target
  workstation.

## UAT-SH-006: Relationship Search for SKD Filter Creation

Related use case: UC-SH-004.

Related requirements: FR-SH-SEARCH-001, FR-SH-SEARCH-002, NFR-QUERY-001.

Preconditions:

- `target/uat/sh-search-ru.sqlite` exists from UAT-SH-004.

Steps:

```bash
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax search --query "отбор скд" --mode keywords --format json > target/uat/search-filter-keywords-1.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax search --query "отбор скд" --mode keywords --format json > target/uat/search-filter-keywords-2.json
cmp target/uat/search-filter-keywords-1.json target/uat/search-filter-keywords-2.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --name "ОтборКомпоновкиДанных" --format json > target/uat/related-filter-1.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --name "ОтборКомпоновкиДанных" --format json > target/uat/related-filter-2.json
cmp target/uat/related-filter-1.json target/uat/related-filter-2.json
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

## UAT-SH-015: Fuzzy Syntax Assistant Name Search

Related use case: UC-SH-003.

Related requirements: FR-SH-SEARCH-001, NFR-QUERY-001.

Preconditions:

- `target/uat/sh-search-ru.sqlite` exists from UAT-SH-004.

Steps:

```bash
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax search --query "ОтборКомпоновкиДаных" --mode fuzzy --format json > target/uat/search-filter-fuzzy-1.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax search --query "ОтборКомпоновкиДаных" --mode fuzzy --format json > target/uat/search-filter-fuzzy-2.json
cmp target/uat/search-filter-fuzzy-1.json target/uat/search-filter-fuzzy-2.json
```

Expected result:

- Exit code is `0`.
- The result set includes the platform type fact with primary name `ОтборКомпоновкиДанных` and
  alias `DataCompositionFilter`.
- `ОтборКомпоновкиДанных` is ranked ahead of unrelated facts.
- Repeated JSON output is byte-identical for the same index and query.
- The command resolves the index path from `V8_CONTEXT_HBK_SYNTAX_INDEX` when `--index` is omitted.
- The command returns within the NFR-QUERY-001 provisional target when measured on the target
  workstation.

## UAT-SH-007: Locale-Complete Syntax Assistant Type References and Clean Descriptions

Related use case: UC-SH-001.

Related requirements: FR-SH-002, FR-SH-003, FR-EXPORT-001, NFR-DIAG-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` exists.
- `target/uat/shcntx-ru` and `target/uat/shcntx-en` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-ru target/uat/shcntx-en
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/uat/shcntx-en
```

Expected result:

- `XMLСтрока` / `XMLString` has non-empty return types in both locales.
- `Массив.Добавить` / `Array.Add` has a parameter type reference in both locales.
- `ОткрытьФорму` / `OpenForm` keeps parameter type references in both locales.
- Descriptions do not contain raw section labels such as `Доступность:`, `Availability:`,
  `Пример:`, `Example:`, `См. также:`, `See also:`, `Использование в версии:` or
  `Available since:`.
- Parameter descriptions and signature text do not contain raw `Returned value:`, `Return value:`,
  `Возвращаемое значение:`, `Параметры:` or `Parameters:` section labels.

Cleanup:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` are service data and may be deleted after the
  run.

## UAT-SH-008: Structured Availability, Examples and See-Also Facts

Related use case: UC-SH-001.

Related requirements: FR-SH-002, FR-EXPORT-001.

Preconditions:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` exist from UAT-SH-007.

Steps:

```bash
jq -e '.records[] | select(.name.primary == "XMLСтрока" or .name.primary == "XMLString")' target/uat/shcntx-ru/global-methods.json target/uat/shcntx-en/global-methods.json
jq -e '.records[] | select(.name.primary == "Массив" or .name.primary == "Array")' target/uat/shcntx-ru/platform-types.json target/uat/shcntx-en/platform-types.json
jq -e '.records[] | select(.name.primary == "XMLСтрока") | (.availability.contexts | index("thin_client") != null and index("server") != null) and (.examples | length > 0) and (.see_also | index("Глобальный контекст.XMLЗначение") != null) and (.availability.since == "8.0") and (has("available_since") | not)' target/uat/shcntx-ru/global-methods.json
jq -e '.records[] | select(.name.primary == "XMLString") | (.availability.contexts | index("thin_client") != null and index("server") != null) and (.examples | length > 0) and (.see_also | index("Global context.XMLValue") != null) and (.availability.since == "8.0") and (has("available_since") | not)' target/uat/shcntx-en/global-methods.json
jq -e '.records[] | select(.name.primary == "Массив" or .name.primary == "Array") | (.availability.contexts | index("web_client") != null and index("server") != null) and (.examples | length > 0) and (.availability.since == "8.0") and (has("available_since") | not)' target/uat/shcntx-ru/platform-types.json target/uat/shcntx-en/platform-types.json
```

Expected result:

- The selected method/type records expose structured availability/application-context facts.
- Availability includes normalized client/server modes instead of only localized free text embedded
  in `description`.
- Syntax examples are preserved as dedicated example/code blocks when the source page contains an
  example.
- See-also links or relationships are preserved separately from `description` as target primary
  names when the source page contains them, without exposing target HTML paths in consumer
  record-family JSON.
- Available-since/version information is preserved separately from `description` when the source
  page contains it, as `availability.since` rather than top-level `available_since`.

## UAT-SH-009: Structured Syntax Variants and Overloads

Related use case: UC-SH-001.

Related requirements: FR-SH-002, FR-EXPORT-001.

Preconditions:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` exist from UAT-SH-007.

Steps:

```bash
jq -e '.records[] | select(.owner == "ДокументDOM" and .name.primary == "СоздатьРазыменовательПИ")' target/uat/shcntx-ru/type-methods.json
jq -e '.records[] | select(.owner == "DOMDocument" and .name.primary == "CreateNSResolver")' target/uat/shcntx-en/type-methods.json
jq -e '.records[] | select(.owner == "ДокументDOM" and .name.primary == "СоздатьРазыменовательПИ") | (.signatures | length == 4) and all(.signatures[]; (.title | length > 0) and (has("variant") | not) and (has("text") | not)) and any(.signatures[]; .title == "На основании узла DOM" and any(.parameters[]; .name == "УзелКонтекста" and (.types | index("ДокументDOM") != null))) and any(.signatures[]; .title == "На основании конкретного префикса и URI пространства имен" and (.parameters | length == 2)) and (.return | index("РазыменовательПространствИменDOM") != null)' target/uat/shcntx-ru/type-methods.json
jq -e '.records[] | select(.owner == "DOMDocument" and .name.primary == "CreateNSResolver") | (.signatures | length == 4) and all(.signatures[]; (.title | length > 0) and (has("variant") | not) and (has("text") | not)) and any(.signatures[]; .title == "On the basis of DOM node" and any(.parameters[]; .name == "ContextNode" and (.types | index("DOMDocument") != null))) and any(.signatures[]; .title == "On the basis of specific prefix and namespace URI" and (.parameters | length == 2)) and (.return | index("DOMNamespaceResolver") != null)' target/uat/shcntx-en/type-methods.json
```

Expected result:

- The method exposes each source syntax variant as a structured overload/variant.
- Signature records do not expose `text`; callable structure is represented by parameters and
  return/type facts.
- Variant titles and variant descriptions are preserved as direct signature metadata without a
  nested `variant` object.
- Parameters belong to the correct variant and do not absorb following variant descriptions.
- Return type extraction works for this page in both locales.

## UAT-SH-010: Classified Syntax Assistant Diagnostic Families

Related use case: UC-SH-001.

Related requirements: FR-SH-001, FR-SH-002, NFR-DIAG-001.

Preconditions:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` exist from UAT-SH-007.

Steps:

```bash
jq -e '
  def counts: reduce .records[].code as $code ({}; .[$code] = (.[$code] // 0) + 1);
  counts == {
    "UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE": 4
  }
' target/uat/shcntx-ru/diagnostics.json
jq -e '
  def counts: reduce .records[].code as $code ({}; .[$code] = (.[$code] // 0) + 1);
  counts == {
    "UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE": 4
  }
' target/uat/shcntx-en/diagnostics.json
```

Expected result:

- The export keeps deterministic diagnostic counts for both locales.
- Direct global-context method-like TOC-only pages remain classified with a family-specific
  diagnostic code.
- `UNKNOWN_PAGE_CLASS`, `OUT_OF_SCOPE_GLOBAL_CONTEXT_EVENT`, `OUT_OF_SCOPE_TABLE_FIELD` and
  `OUT_OF_SCOPE_TABLE_PARAMETER` are absent.
- Diagnostic records keep source provenance needed by NFR-DIAG-001.

Cleanup:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` are service data and may be deleted after the
  run.

## UAT-SH-011: Event and Query/Table Metadata Export

Related use case: UC-SH-001.

Related requirements: FR-SH-002, FR-SH-003, FR-EXPORT-001, NFR-DIAG-001.

Preconditions:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` exist from UAT-SH-007.

Steps:

```bash
jq -e '.schema_version == 11 and (.records | length) == 47' target/uat/shcntx-ru/module-events.json
jq -e '.schema_version == 11 and (.records | length) == 650' target/uat/shcntx-ru/type-events.json
jq -e '.schema_version == 11 and (.records | length) > 0' target/uat/shcntx-ru/query-tables.json
jq -e '.schema_version == 11 and (.records | length) == 47' target/uat/shcntx-en/module-events.json
jq -e '.schema_version == 11 and (.records | length) == 650' target/uat/shcntx-en/type-events.json
jq -e '.schema_version == 11 and (.records | length) > 0' target/uat/shcntx-en/query-tables.json

jq -e 'all(.records[]; .record_family == "module_event" and has("module"))' target/uat/shcntx-ru/module-events.json
jq -e 'all(.records[]; .record_family == "module_event" and has("module"))' target/uat/shcntx-en/module-events.json
jq -e 'all(.records[]; .record_family == "type_event" and has("owner") and (has("module") | not))' target/uat/shcntx-ru/type-events.json
jq -e 'all(.records[]; .record_family == "type_event" and has("owner") and (has("module") | not))' target/uat/shcntx-en/type-events.json

jq -e '.records[] | select(.name.primary == "ПередЗавершениемРаботыСистемы" and .availability.since == "8.2") | (.signatures[0].parameters | length == 2) and (.signatures[0] | has("text") | not) and any(.signatures[0].parameters[]; .name == "Отказ" and .required == true and (.types | index("Булево") != null))' target/uat/shcntx-ru/module-events.json
jq -e '.records[] | select(.name.primary == "BeforeExit" and .availability.since == "8.2") | (.signatures[0].parameters | length == 2) and (.signatures[0] | has("text") | not) and any(.signatures[0].parameters[]; .name == "Cancel" and .required == true and (.types | index("Boolean") != null))' target/uat/shcntx-en/module-events.json

jq -e '.records[] | select(.name == "Таблица бизнес-процессов" and .table_role == "primary" and .identifier == "БизнесПроцесс" and .syntax.primary == "БизнесПроцесс.<Имя бизнес-процесса>" and .syntax.alias == "BusinessProcess.<Имя бизнес-процесса>") | any(.fields[]; .name == "Представление" and (.types | index("Строка") != null) and (.description | test("строку-представление")))' target/uat/shcntx-ru/query-tables.json
jq -e '.records[] | select(.name == "Business Process Table" and .table_role == "primary" and .identifier == "BusinessProcess" and .syntax.primary == "BusinessProcess.<Business process name>" and (.syntax | has("alias") | not)) | any(.fields[]; .name == "Presentation" and (.types | index("String") != null) and (.description | test("presentation")))' target/uat/shcntx-en/query-tables.json
jq -e '.records[] | select(.syntax.primary == "БизнесПроцесс.<Имя бизнес-процесса>.Точки" and .syntax.alias == "BusinessProcess.<Имя бизнес-процесса>.Points") | .table_role == "additional" and .identifier == "БизнесПроцессТаблицаТочекБизнесПроцессов"' target/uat/shcntx-ru/query-tables.json
jq -e '.records[] | select(.syntax.primary == "BusinessProcess.<Business process name>.Points" and (.syntax | has("alias") | not)) | .table_role == "additional" and .identifier == "BusinessProcessBusinessProcessPointTable"' target/uat/shcntx-en/query-tables.json
jq -e '.records[] | select(.name == "Таблица изменений бизнес-процессов") | .identifier == "БизнесПроцессТаблицаИзмененийБизнесПроцессов"' target/uat/shcntx-ru/query-tables.json

jq -e '.records[] | select(.name == "Таблица критерия отбора") | any(.parameters[]; .name == "Значение" and (has("required") | not) and (.description | test("отбор")))' target/uat/shcntx-ru/query-tables.json
jq -e '.records[] | select(.name == "Filter Criterion Table") | any(.parameters[]; .name == "Value" and (has("required") | not) and (.description | test("filtering")))' target/uat/shcntx-en/query-tables.json
```

Expected result:

- Module events, type events and query tables are exported as typed consumer facts in both locales.
  `module-events.json` is the required FR-EXPORT-001 adapter filename for `module_event` records,
  and `type-events.json` is the required adapter filename for `type_event` records.
- Event signatures and parameters are parsed structurally.
- Query table records preserve table name, semantic owner path, table role, field names, field type
  references, field descriptions, localized syntax and deterministic identifier.
- Query table parameter records are nested under their owning table, preserve parameter names, type
  references when present, descriptions and default values when present, and do not expose
  `required`.
- These records do not appear only as parser diagnostics.

## UAT-SH-012: Lean Schema Version 10 Consumer JSON Shape

Related use case: UC-SH-001.

Related requirements: FR-EXPORT-001.

Preconditions:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` exist from UAT-SH-007.

Steps:

```bash
for file in target/uat/shcntx-ru/metadata.json target/uat/shcntx-en/metadata.json; do
  jq -e '
    .schema_version == 11
    and (.files | any(.[]; .file_name == "query-tables.json"))
    and (.files | any(.[]; .file_name == "module-events.json"))
    and (.files | any(.[]; .file_name == "type-events.json"))
    and (.files | any(.[]; .file_name == "unknown-events.json"))
    and (.files | all(.[]; .file_name != "enum-values.json"
                         and .file_name != "table-fields.json"
                         and .file_name != "table-parameters.json"
                         and .file_name != "global-context-events.json"))
  ' "$file"
done

for file in \
  target/uat/shcntx-ru/global-methods.json \
  target/uat/shcntx-ru/global-properties.json \
  target/uat/shcntx-ru/module-events.json \
  target/uat/shcntx-ru/type-events.json \
  target/uat/shcntx-ru/unknown-events.json \
  target/uat/shcntx-ru/platform-types.json \
  target/uat/shcntx-ru/type-methods.json \
  target/uat/shcntx-ru/type-properties.json \
  target/uat/shcntx-ru/query-tables.json \
  target/uat/shcntx-ru/constructors.json \
  target/uat/shcntx-ru/enums.json \
  target/uat/shcntx-en/global-methods.json \
  target/uat/shcntx-en/global-properties.json \
  target/uat/shcntx-en/module-events.json \
  target/uat/shcntx-en/type-events.json \
  target/uat/shcntx-en/unknown-events.json \
  target/uat/shcntx-en/platform-types.json \
  target/uat/shcntx-en/type-methods.json \
  target/uat/shcntx-en/type-properties.json \
  target/uat/shcntx-en/query-tables.json \
  target/uat/shcntx-en/constructors.json \
  target/uat/shcntx-en/enums.json; do
  jq -e '([.records[] | .. | objects | to_entries[] | select(.value == null or .value == [])] | length) == 0' "$file"
done

jq -e '.records[] | select(.name.primary == "ТипЗначенияJSON") | (.values | any(.name.primary == "КонецМассива")) and all(.values[]; (has("owner") | not) and (has("available_since") | not))' target/uat/shcntx-ru/enums.json
jq -e '.records[] | select(.usage == "Read" and (.types | index("СправочникиМенеджер") != null)) | (.description | startswith("Тип:") | not)' target/uat/shcntx-ru/global-properties.json
for file in \
  target/uat/shcntx-ru/global-methods.json \
  target/uat/shcntx-ru/global-properties.json \
  target/uat/shcntx-ru/module-events.json \
  target/uat/shcntx-ru/type-events.json \
  target/uat/shcntx-ru/unknown-events.json \
  target/uat/shcntx-ru/type-methods.json \
  target/uat/shcntx-ru/type-properties.json \
  target/uat/shcntx-ru/query-tables.json \
  target/uat/shcntx-ru/constructors.json \
  target/uat/shcntx-en/global-methods.json \
  target/uat/shcntx-en/global-properties.json \
  target/uat/shcntx-en/module-events.json \
  target/uat/shcntx-en/type-events.json \
  target/uat/shcntx-en/unknown-events.json \
  target/uat/shcntx-en/type-methods.json \
  target/uat/shcntx-en/type-properties.json \
  target/uat/shcntx-en/query-tables.json \
  target/uat/shcntx-en/constructors.json; do
  jq -e '([.records[] | .. | objects | keys[] | select(. == "type_refs" or . == "return_types")] | length) == 0' "$file"
done
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-ru/type-methods.json
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-ru/type-properties.json
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-ru/constructors.json
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-ru/type-events.json
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-en/type-methods.json
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-en/type-properties.json
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-en/constructors.json
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-en/type-events.json
jq -e 'all(.records[]; (.fields // [] | all(.[]; (has("owner_path") | not) and (.name | type == "string"))) and (.parameters // [] | all(.[]; (has("owner_path") | not) and (has("required") | not) and (.name | type == "string"))))' target/uat/shcntx-ru/query-tables.json
jq -e 'all(.records[]; (.fields // [] | all(.[]; (has("owner_path") | not) and (.name | type == "string"))) and (.parameters // [] | all(.[]; (has("owner_path") | not) and (has("required") | not) and (.name | type == "string"))))' target/uat/shcntx-en/query-tables.json
jq -e 'all(.records[]; (.syntax.primary | type == "string") and ((.syntax.alias? // "" | type) == "string") and (.identifier | type == "string") and (.identifier | test("[\\s-]") | not))' target/uat/shcntx-ru/query-tables.json
jq -e 'all(.records[]; (.syntax.primary | type == "string") and ((.syntax.alias? // "" | type) == "string") and (.identifier | type == "string") and (.identifier | test("[\\s-]") | not))' target/uat/shcntx-en/query-tables.json
jq -e '.records[] | select(.owner == "ТабличноеПоле" and .name.primary == "СоздатьКолонки") | .examples[0].text == "ЭлементыФормы.ТабличноеПоле1.Значение = ТаблицаДанных;\nЭлементыФормы.ТабличноеПоле1.СоздатьКолонки();"' target/uat/shcntx-ru/type-methods.json
jq -e '.records[] | select(.owner == "ЗадачаОбъект.<Имя задачи>" and .name.primary == "Записать") | (.examples[0].text | contains("ОписаниеОшибки ( )") | not) and (.examples[0].text | contains("ОписаниеОшибки(), 60);"))' target/uat/shcntx-ru/type-methods.json
jq -e '.records[] | select(.owner == "Расширение поля формы для поля ввода" and .name.primary == "ПараметрыВыбора") | (.examples[0].text | startswith("НовыйПараметр = Новый ПараметрВыбора")) and (.examples[0].text | contains("Тонкий клиент") | not)' target/uat/shcntx-ru/type-properties.json
jq -e '.records[] | select(.name.primary == "ЭлементИзбранногоРаботыПользователя") | (.see_also | index("ИзбранноеРаботыПользователя.Вставить") != null) and (.see_also | index("ИзбранноеРаботыПользователя.Добавить") != null) and (.see_also | index("ИзбранноеРаботыПользователя.Индекс") != null)' target/uat/shcntx-ru/platform-types.json
jq -e '.records[] | select(.name.primary == "МенеджерИсторииРаботыПользователя") | (.see_also | index("Глобальный контекст.ИсторияРаботыПользователя") != null)' target/uat/shcntx-ru/platform-types.json
```

Expected result:

- Consumer record-family files use `schema_version: 11`.
- `metadata.json.files` contains `query-tables.json` and does not contain old schema files
  `enum-values.json`, `table-fields.json` or `table-parameters.json`. Physical stale files from a
  reused output directory are not part of the current export contract and are not deleted by the
  exporter.
- Enum values are nested under owning enum records as `values`.
- Nested enum value names keep the localized-name object shape with `primary` and optional `alias`.
- Platform API consumer records do not emit `null` fields or empty arrays in any record family.
- Type event, derivative type member and constructor records do not emit `owner_path`.
- Query table fields and parameters are nested under `query-tables.json` table records, use string
  `name` values, do not repeat `owner_path` and do not expose parameter `required`.
- Query table records include localized `syntax` objects and string `identifier` values without
  whitespace or hyphens; additional table identifier suffixes are CamelCase-normalized from page
  `name`.
- `usage` is a stable enum string.
- Property descriptions do not keep leading type-reference prose that already appears in `types`.
- Type-reference facts are exposed as `types`; method return facts are exposed as `return`; legacy
  `type_refs` and `return_types` are absent from consumer JSON.
- Syntax examples are extracted from the example/code section, not from following availability
  sections, and code examples do not contain HTML-coloring spaces around BSL punctuation.
- See-also owner/member link pairs are exported as composed `Owner.Member` target strings.

Cleanup:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` are service data and may be deleted after the
  run.

## UAT-SH-013: TOC-Aware Syntax Assistant Reading Disambiguation

Related use case: UC-SH-001.

Related requirements: FR-SH-003, FR-SH-002, NFR-COMPAT-001, NFR-DIAG-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` exists.
- `target/uat/shcntx-ru` and `target/uat/shcntx-en` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-ru target/uat/shcntx-en
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/uat/shcntx-en
```

Then inspect the exported records as the black-box observable result of Syntax Assistant reading.
Run deterministic checks for the accepted semantic identity fields:

```bash
jq -e '
  [.records[]
   | select(.name == "Таблица остатков и оборотов"
            and any(.parameters[]; .name == "Метод дополнения периодов"))
   | (.owner_path // []) | join(" > ")]
  | length == 3 and (unique | length) == 3
' target/uat/shcntx-ru/query-tables.json

jq -e '
  [.records[]
   | select(.name.primary == "ПриНачалеРаботыСистемы"
            and .name.alias == "OnStart")
   | .module.kind]
  | length == 3 and (unique | length) == 3
' target/uat/shcntx-ru/module-events.json

jq -e '
  [.records[]
   | select(.name.primary == "ПередЗаписью"
            and .name.alias == "BeforeWrite")
   | .owner]
  | length > 1 and (unique | length) == length
' target/uat/shcntx-ru/type-events.json

jq -e '
  ([.records[] | [.owner, .name.primary, (.name.alias // "")] | @tsv] | length)
  == ([.records[] | [.owner, .name.primary, (.name.alias // "")] | @tsv] | unique | length)
' target/uat/shcntx-ru/type-events.json

jq -e '
  ([.records[] | [.owner, .name.primary, (.name.alias // "")] | @tsv] | length)
  == ([.records[] | [.owner, .name.primary, (.name.alias // "")] | @tsv] | unique | length)
' target/uat/shcntx-en/type-events.json

jq -e '
  [.records[]
   | select(.name == "Основная таблица"
            and any(.fields[]; .name == "<Имя измерения>"))
   | (.owner_path // []) | join(" > ")]
  | length > 1 and (unique | length) == length
' target/uat/shcntx-ru/query-tables.json

jq -e '
  [.records[]
   | select(.name.primary == "Ключ"
            and .name.alias == "Key")
   | (.owner_path // []) | join(" > ")]
  | length > 1 and (unique | length) == length
' target/uat/shcntx-ru/platform-types.json

jq -e '
  [.records[]
   | select(.owner == "ЭлементыФормы"
            and .name.primary == "Количество"
            and .name.alias == "Count")
   | has("owner_path")]
  | length > 1 and all(.[]; . == false)
' target/uat/shcntx-ru/type-methods.json

jq -e '
  [.records[]
   | select(.owner | test("<Имя"; ""))
   | select(has("owner_path") | not)]
  | length > 0
' target/uat/shcntx-ru/type-properties.json

jq -e '
  [.records[]
   | select(.owner | test("<Имя"; ""))
   | select(has("owner_path") | not)]
  | length > 0
' target/uat/shcntx-ru/constructors.json

jq -e '
  all(.records[]; .name.primary != "Истина" and .name.primary != "Ложь") and
  all(.records[] | select(.branch_kind == "primitive_types"); .type_kind == "primitive")
' target/uat/shcntx-ru/platform-types.json

jq -e 'any(.records[]; .type_kind == "extension")' target/uat/shcntx-ru/platform-types.json
jq -e 'any(.records[]; .type_kind == "metadata_template" and .name.primary == "ДокументОбъект.<Имя документа>")' target/uat/shcntx-ru/platform-types.json

jq -e '
  any(.records[];
      (.owner | test("Client application form"))
      and .record_family == "type_event"
      and (has("module") | not))
' target/uat/shcntx-en/type-events.json

jq -e '
  any(.records[];
      .name.primary == "BinaryDataStorageInformation"
      and .branch_kind == "platform_objects")
' target/uat/shcntx-en/platform-types.json
```

Expected result:

- Exit code is `0`.
- The reader uses TOC-derived semantic context for classification and ownership before records are
  exported.
- The reader distinguishes branch kind from record family; branch categories such as
  Automation/external API do not become separate record families by themselves.
- The observed record families do not contain exact-lookup collisions caused by name/title-only
  reading.
- Any remaining ambiguous source pages are reported as recoverable diagnostics with provenance
  rather than silently collapsed or emitted as indistinguishable facts.

Cleanup:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` are service data and may be deleted after the
  run.

## UAT-SH-014: Event File Split and Owner Classification

Related use case: UC-SH-001.

Related requirements: FR-SH-003, FR-EXPORT-001, NFR-COMPAT-001, NFR-DIAG-001.

Purpose:

- Validate the post-schema-v8 event contract after T37 and the owner-classification boundary owned
  by T38.
- Keep this UAT independent from schema version 8 `owner_path` removal on derivative records.

Preconditions:

- T36 has completed and the schema version 8 `owner_path` narrowing is the baseline.
- `target/uat/shcntx-ru` and `target/uat/shcntx-en` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-ru target/uat/shcntx-en
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/uat/shcntx-en
```

Then verify the event split and owner-classification boundaries:

```bash
for file in target/uat/shcntx-ru/metadata.json target/uat/shcntx-en/metadata.json; do
  jq -e '
    .schema_version == 11
    and (.files | all(.[]; .file_name != "global-context-events.json"))
    and (.files | any(.[]; .file_name == "module-events.json"))
    and (.files | any(.[]; .file_name == "type-events.json"))
    and (.files | any(.[]; .file_name == "unknown-events.json"))
  ' "$file"
done

for file in \
  target/uat/shcntx-ru/module-events.json \
  target/uat/shcntx-ru/type-events.json \
  target/uat/shcntx-ru/unknown-events.json \
  target/uat/shcntx-en/module-events.json \
  target/uat/shcntx-en/type-events.json \
  target/uat/shcntx-en/unknown-events.json; do
  jq -e '([.records[] | .. | objects | keys[] | select(. == "id" or . == "owner_ref" or . == "source_hbk" or . == "toc_path" or . == "html_path" or . == "page_title")] | length) == 0' "$file"
done

jq -e 'all(.records[]; .record_family == "module_event") and any(.records[]; .name.primary == "ПриНачалеРаботыСистемы")' target/uat/shcntx-ru/module-events.json
jq -e 'all(.records[]; .record_family == "type_event") and any(.records[]; .name.primary == "ПередЗаписью")' target/uat/shcntx-ru/type-events.json
jq -e 'all(.records[]; .record_family == "unknown_event")' target/uat/shcntx-ru/unknown-events.json

jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-ru/type-methods.json
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-ru/type-properties.json
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-ru/constructors.json
jq -e 'all(.records[]; has("owner_path") | not)' target/uat/shcntx-ru/type-events.json

jq -e '([.records[] | .. | objects | keys[] | select(. == "owner_kind")] | length) == 0' target/uat/shcntx-ru/module-events.json
jq -e '([.records[] | .. | objects | keys[] | select(. == "owner_kind")] | length) == 0' target/uat/shcntx-ru/type-events.json
jq -e '([.records[] | .. | objects | keys[] | select(. == "object_kind")] | length) == 0' target/uat/shcntx-ru/module-events.json
jq -e '([.records[] | .. | objects | keys[] | select(. == "object_kind")] | length) == 0' target/uat/shcntx-ru/type-events.json

jq -e '
  any(.records[]; .name.primary == "Массив" and .object_kind == "regular_platform_type") and
  any(.records[]; .name.primary == "ГруппаФормы" and .object_kind == "managed_form") and
  any(.records[]; .name.primary == "Расширение поля формы для поля ввода" and .object_kind == "form_extension") and
  any(.records[]; .name.primary == "ДокументОбъект.<Имя документа>" and .object_kind == "metadata_object")
' target/uat/shcntx-ru/platform-types.json

jq -e '
  any(.records[]; .name.primary == "UserWorkFavorites" and .object_kind == "regular_platform_type") and
  any(.records[]; .name.primary == "FormGroup" and .object_kind == "managed_form") and
  any(.records[]; .name.primary == "Extension for controls located in a form" and .object_kind == "form_extension") and
  any(.records[]; .name.primary == "MetadataObject: Document" and .object_kind == "metadata_object")
' target/uat/shcntx-en/platform-types.json
```

Expected result:

- Exit code is `0`.
- `metadata.json.files` contains the three event files and no longer lists
  `global-context-events.json`.
- Event files do not expose cross-cutting `id` or `owner_ref` fields.
- Event files do not expose raw HBK, TOC, HTML or page-title provenance.
- Module-level events are routed to `module-events.json`; type/form/object event-like facts are
  routed to `type-events.json`.
- `unknown-events.json` contains only diagnostic-backed fallback event records when classification
  is not safe.
- Owner/object classification, when implemented, belongs to the owner platform type/object record
  as `object_kind` and not to an event-only `owner.kind`, `owner_kind` or `object_kind` field.
  Source-backed owner classifications cover regular platform types, managed forms, form extensions
  and metadata objects when the TOC proves them.
- Type events, derivative type methods, type properties and constructors still omit `owner_path`;
  the event split does not weaken the schema version 8 omission rule.

Cleanup:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` are service data and may be deleted after the
  run.

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
