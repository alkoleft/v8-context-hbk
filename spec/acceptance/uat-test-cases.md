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
' target/uat/shcntx-ru/global-context-events.json
jq -e 'has("source_hbk") | not' target/uat/shcntx-ru/global-context-events.json
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
' target/uat/shcntx-ru/table-fields.json
jq -e 'has("source_hbk") | not' target/uat/shcntx-ru/table-fields.json
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
' target/uat/shcntx-ru/table-parameters.json
jq -e 'has("source_hbk") | not' target/uat/shcntx-ru/table-parameters.json
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
- `target/uat/shcntx-ru` can be created or removed.
- `target/uat/sh-search-ru.sqlite` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-ru target/uat/sh-search-ru.sqlite
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
v8-sh index target/uat/shcntx-ru --output target/uat/sh-search-ru.sqlite
```

Expected result:

- Exit code is `0`.
- The index artifact is a SQLite database.
- The database contains schema metadata plus deterministic document, exact-name, FTS and
  relationship-edge data.
- The index build records locale `ru`, source locale `ru` and source export schema version.
- The query CLI does not require the HBK file path for later lookup commands.

Cleanup:

- `target/uat/shcntx-ru` and `target/uat/sh-search-ru.sqlite` are service data and may be deleted
  after the run.

## UAT-SH-005: Exact Syntax Assistant Lookup

Related use case: UC-SH-003.

Related requirements: FR-SH-SEARCH-001, NFR-QUERY-001.

Preconditions:

- `target/uat/sh-search-ru.sqlite` exists from UAT-SH-004.

Steps:

```bash
v8-sh get --index target/uat/sh-search-ru.sqlite --name "ОтборКомпоновкиДанных" --format json
v8-sh get --index target/uat/sh-search-ru.sqlite --name "DataCompositionFilter" --format json
v8-sh get --index target/uat/sh-search-ru.sqlite --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json
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

- `target/uat/sh-search-ru.sqlite` exists from UAT-SH-004.

Steps:

```bash
v8-sh search --index target/uat/sh-search-ru.sqlite --query "отбор скд" --mode keywords --format json
v8-sh related --index target/uat/sh-search-ru.sqlite --name "ОтборКомпоновкиДанных" --format json
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

## UAT-SH-007: Locale-Complete Syntax Assistant Type References and Clean Descriptions

Related use case: UC-SH-001.

Related requirements: FR-SH-002, FR-EXPORT-001, NFR-DIAG-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` exists.
- `target/uat/shcntx-ru` and `target/uat/shcntx-en` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-ru target/uat/shcntx-en
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/uat/shcntx-en
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
jq -e '.records[] | select(.name.primary == "XMLСтрока") | (.availability.contexts | index("thin_client") != null and index("server") != null) and (.examples | length > 0) and (.see_also | index("XMLЗначение") != null) and (.availability.since == "8.0") and (has("available_since") | not)' target/uat/shcntx-ru/global-methods.json
jq -e '.records[] | select(.name.primary == "XMLString") | (.availability.contexts | index("thin_client") != null and index("server") != null) and (.examples | length > 0) and (.see_also | index("XMLValue") != null) and (.availability.since == "8.0") and (has("available_since") | not)' target/uat/shcntx-en/global-methods.json
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
jq -e '.records[] | select(.owner == "ДокументDOM" and .name.primary == "СоздатьРазыменовательПИ") | (.signatures | length == 4) and all(.signatures[]; (.title | length > 0) and (has("variant") | not) and (has("text") | not)) and any(.signatures[]; .title == "На основании узла DOM" and any(.parameters[]; .name == "УзелКонтекста" and (.type_refs | index("ДокументDOM") != null))) and any(.signatures[]; .title == "На основании конкретного префикса и URI пространства имен" and (.parameters | length == 2)) and (.return_types | index("РазыменовательПространствИменDOM") != null)' target/uat/shcntx-ru/type-methods.json
jq -e '.records[] | select(.owner == "DOMDocument" and .name.primary == "CreateNSResolver") | (.signatures | length == 4) and all(.signatures[]; (.title | length > 0) and (has("variant") | not) and (has("text") | not)) and any(.signatures[]; .title == "On the basis of DOM node" and any(.parameters[]; .name == "ContextNode" and (.type_refs | index("DOMDocument") != null))) and any(.signatures[]; .title == "On the basis of specific prefix and namespace URI" and (.parameters | length == 2)) and (.return_types | index("DOMNamespaceResolver") != null)' target/uat/shcntx-en/type-methods.json
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

Related requirements: FR-SH-002, FR-EXPORT-001, NFR-DIAG-001.

Preconditions:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` exist from UAT-SH-007.

Steps:

```bash
jq -e '.schema_version == 5 and (.records | length) == 33' target/uat/shcntx-ru/global-context-events.json
jq -e '.schema_version == 5 and (.records | length) == 588' target/uat/shcntx-ru/table-fields.json
jq -e '.schema_version == 5 and (.records | length) == 78' target/uat/shcntx-ru/table-parameters.json
jq -e '.schema_version == 5 and (.records | length) == 33' target/uat/shcntx-en/global-context-events.json
jq -e '.schema_version == 5 and (.records | length) == 588' target/uat/shcntx-en/table-fields.json
jq -e '.schema_version == 5 and (.records | length) == 78' target/uat/shcntx-en/table-parameters.json

jq -e '.records[] | select(.name.primary == "ПередЗавершениемРаботыСистемы" and .availability.since == "8.2") | (.signatures[0].parameters | length == 2) and (.signatures[0] | has("text") | not) and any(.signatures[0].parameters[]; .name == "Отказ" and .required == true and (.type_refs | index("Булево") != null))' target/uat/shcntx-ru/global-context-events.json
jq -e '.records[] | select(.name.primary == "BeforeExit" and .availability.since == "8.2") | (.signatures[0].parameters | length == 2) and (.signatures[0] | has("text") | not) and any(.signatures[0].parameters[]; .name == "Cancel" and .required == true and (.type_refs | index("Boolean") != null))' target/uat/shcntx-en/global-context-events.json

jq -e '.records[] | select(.owner == "Таблица бизнес-процессов" and .name.primary == "Представление") | (.type_refs | index("Строка") != null) and (.description | test("строку-представление"))' target/uat/shcntx-ru/table-fields.json
jq -e '.records[] | select(.owner == "Business Process Table" and .name.primary == "Presentation") | (.type_refs | index("String") != null) and (.description | test("presentation"))' target/uat/shcntx-en/table-fields.json

jq -e '.records[] | select(.owner == "Таблица критерия отбора" and .name.primary == "Значение") | .required == true and (.description | test("отбор"))' target/uat/shcntx-ru/table-parameters.json
jq -e '.records[] | select(.owner == "Filter Criterion Table" and .name.primary == "Value") | .required == true and (.description | test("filtering"))' target/uat/shcntx-en/table-parameters.json
```

Expected result:

- Global context events, query/table fields and query/table parameters are exported as typed
  consumer record families in both locales.
- Event signatures and parameters are parsed structurally.
- Table field records preserve owner table, field name, type references and descriptions.
- Table parameter records preserve owner table, parameter name, required flag, type references when
  present, descriptions and default values when present.
- These records do not appear only as parser diagnostics.

## UAT-SH-012: Lean Schema Version 5 Consumer JSON Shape

Related use case: UC-SH-001.

Related requirements: FR-EXPORT-001.

Preconditions:

- `target/uat/shcntx-ru` and `target/uat/shcntx-en` exist from UAT-SH-007.

Steps:

```bash
jq -e '.schema_version == 5 and (.files | all(.[]; .file_name != "enum-values.json"))' target/uat/shcntx-ru/metadata.json target/uat/shcntx-en/metadata.json
test ! -e target/uat/shcntx-ru/enum-values.json
test ! -e target/uat/shcntx-en/enum-values.json

jq -e '([.records[] | .. | objects | to_entries[] | select(.value == null or .value == [])] | length) == 0' target/uat/shcntx-ru/global-methods.json target/uat/shcntx-ru/type-methods.json target/uat/shcntx-ru/constructors.json target/uat/shcntx-ru/enums.json
jq -e '([.records[] | .. | objects | to_entries[] | select(.value == null or .value == [])] | length) == 0' target/uat/shcntx-en/global-methods.json target/uat/shcntx-en/type-methods.json target/uat/shcntx-en/constructors.json target/uat/shcntx-en/enums.json

jq -e '.records[] | select(.name.primary == "ТипЗначенияJSON") | (.values | any(.name.primary == "КонецМассива")) and all(.values[]; (has("owner") | not) and (has("available_since") | not))' target/uat/shcntx-ru/enums.json
jq -e '.records[] | select(.usage == "Read" and (.type_refs | index("СправочникиМенеджер") != null)) | (.description | startswith("Тип:") | not)' target/uat/shcntx-ru/global-properties.json
```

Expected result:

- Consumer record-family files use `schema_version: 5`.
- `enum-values.json` is absent; enum values are nested under owning enum records as `values`.
- Nested enum value names keep the localized-name object shape with `primary` and optional `alias`.
- Consumer records do not emit `null` fields or empty arrays.
- `usage` is a stable enum string.
- Property descriptions do not keep leading type-reference prose that already appears in
  `type_refs`.

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
