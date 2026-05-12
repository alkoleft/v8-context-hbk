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

## UAT-HBK-004: Export Markdown TOC Corpus from Representative HBK Books

Related use case: UC-HBK-003.

Related requirements: FR-HBK-004, FR-DOC-001, FR-CLI-001.

Purpose:

- Validate Markdown conversion on real pages from different HBK book families instead of only on a
  synthetic or single-page fixture.
- Cover ordinary UI help, BSL language syntax help, query-language help and data-composition
  system help.

Preconditions:

- The following 8.5.1.1150 books exist:
  - `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/htmlui_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/moxelui_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/shlang_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/shquery_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/dcsui_ru.hbk`
- `target/uat/book-md-corpus` can be created or removed.

Steps:

```bash
rm -rf target/uat/book-md-corpus
for book in fmtdui_ru htmlui_ru moxelui_ru shlang_ru shquery_ru dcsui_ru; do
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
    "/opt/1cv8/x86_64/8.5.1.1150/${book}.hbk" \
    --output "target/uat/book-md-corpus/${book}" \
    --format markdown \
    --hierarchy toc
done

find target/uat/book-md-corpus -name '*.md' -type f | sort > target/uat/book-md-corpus/files.txt
test "$(wc -l < target/uat/book-md-corpus/files.txt)" -ge 10
for book in fmtdui_ru htmlui_ru moxelui_ru shlang_ru shquery_ru dcsui_ru; do
  test "$(find "target/uat/book-md-corpus/${book}" -name '*.md' -type f | wc -l)" -gt 0
done
! rg -n '<(HTML|BODY)\b|v8help://service_book/service_style|&nbsp;' \
  target/uat/book-md-corpus -g '*.md'
! rg -n '/opt/1cv8|\.hbk\b|objects/.+\.html|#[0-9]+|toc[_ -]?index' \
  target/uat/book-md-corpus -g '*.md'
```

Expected result:

- All export commands exit with code `0`.
- Every exported book directory contains Markdown files.
- Markdown files are written under deterministic TOC-derived directories, not under raw HBK storage
  paths.
- The corpus contains pages from all listed books.
- No exported Markdown file contains raw service HTML scaffolding such as `<HTML`, `<BODY`,
  `v8help://service_book/service_style` or `&nbsp;`.
- No exported Markdown file contains raw HBK file paths, raw TOC indexes or raw HTML page paths.

Skip rule:

- If one or more listed HBK books are absent, record the missing paths and do not mark the case
  failed.

Cleanup:

- `target/uat/book-md-corpus` is service data and may be deleted after the run.

## UAT-HBK-005: Markdown Export Converts Tables and Function Lists

Related use case: UC-HBK-003.

Related requirements: FR-HBK-004, FR-DOC-001, FR-CLI-001.

Source pages:

- `dcsui_ru.hbk` `PresentSKD`: table of Russian/English data-composition keywords.
- `dcsui_ru.hbk` `SKD_Functions_Strings`: string-function index and function sections.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/dcsui_ru.hbk` exists.
- `target/uat/dcsui-md` can be created or removed.

Steps:

```bash
rm -rf target/uat/dcsui-md
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
  /opt/1cv8/x86_64/8.5.1.1150/dcsui_ru.hbk \
  --output target/uat/dcsui-md \
  --format markdown \
  --hierarchy toc

KEYWORDS_PAGE="$(rg -l '^# Двуязычное представление ключевых слов системы компоновки данных' target/uat/dcsui-md -g '*.md' | head -n 1)"
STRINGS_PAGE="$(rg -l '^# Работа со строками' target/uat/dcsui-md -g '*.md' | head -n 1)"
test -n "$KEYWORDS_PAGE"
test -n "$STRINGS_PAGE"

rg -q 'ВЫБОР' "$KEYWORDS_PAGE"
rg -q 'CASE' "$KEYWORDS_PAGE"
rg -q 'ДЛИНАСТРОКИ' "$KEYWORDS_PAGE"
rg -q 'STRINGLENGTH' "$KEYWORDS_PAGE"
rg -q '\|' "$KEYWORDS_PAGE"

rg -q 'ДлинаСтроки' "$STRINGS_PAGE"
rg -q 'StringLength' "$STRINGS_PAGE"
rg -q 'ДлинаСтроки.*Строка' "$STRINGS_PAGE"
rg -q 'Подстрока' "$STRINGS_PAGE"
! rg -n '<(TABLE|TR|TD|A|P|H1|H2)\b|&nbsp;' "$KEYWORDS_PAGE" "$STRINGS_PAGE"
```

Expected result:

- `PresentSKD` is exported as a Markdown page with a heading and readable keyword table content.
- Russian/English keyword pairs remain visible in Markdown.
- `SKD_Functions_Strings` keeps the string-function names and at least one function syntax or
  parameter description.
- Raw table/link/paragraph HTML tags and `&nbsp;` do not leak into the Markdown.

Cleanup:

- `target/uat/dcsui-md` is service data and may be deleted after the run.

## UAT-HBK-006: Markdown Export Preserves Language Syntax Blocks and Links

Related use case: UC-HBK-003.

Related requirements: FR-HBK-004, FR-DOC-001, FR-CLI-001.

Source pages:

- `shlang_ru.hbk` `def_Func`: BSL function declaration syntax.
- `shlang_ru.hbk` `struct_IfThenElif`: conditional operator syntax and internal links.
- `shquery_ru.hbk` `syntax_diagram.html`: query-language syntax diagram examples.
- `shquery_ru.hbk` `SUM`: query aggregate function with see-also link.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shlang_ru.hbk` exists.
- `/opt/1cv8/x86_64/8.5.1.1150/shquery_ru.hbk` exists.
- `target/uat/shlang-md` and `target/uat/shquery-md` can be created or removed.

Steps:

```bash
rm -rf target/uat/shlang-md target/uat/shquery-md
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
  /opt/1cv8/x86_64/8.5.1.1150/shlang_ru.hbk \
  --output target/uat/shlang-md \
  --format markdown \
  --hierarchy toc
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
  /opt/1cv8/x86_64/8.5.1.1150/shquery_ru.hbk \
  --output target/uat/shquery-md \
  --format markdown \
  --hierarchy toc

FUNC_PAGE="$(rg -l '^# Функция' target/uat/shlang-md -g '*.md' | head -n 1)"
IF_PAGE="$(rg -l '^# Если' target/uat/shlang-md -g '*.md' | head -n 1)"
QUERY_SYNTAX_PAGE="$(rg -l '^# Синтаксическая диаграмма конструкций языка запросов' target/uat/shquery-md -g '*.md' | head -n 1)"
SUM_PAGE="$(rg -l '^# Агрегатная функция СУММА' target/uat/shquery-md -g '*.md' | head -n 1)"
test -n "$FUNC_PAGE"
test -n "$IF_PAGE"
test -n "$QUERY_SYNTAX_PAGE"
test -n "$SUM_PAGE"

rg -q 'Синтаксис' "$FUNC_PAGE"
rg -q 'Функция <Имя_функции>' "$FUNC_PAGE"
rg -q 'Возврат <Возвращаемое значение>' "$FUNC_PAGE"
rg -q 'КонецФункции' "$FUNC_PAGE"

rg -q 'Если <Логическое выражение> Тогда' "$IF_PAGE"
rg -q 'ИначеЕсли <Логическое выражение> Тогда' "$IF_PAGE"
rg -q 'КонецЕсли' "$IF_PAGE"
rg -q 'логического выражения' "$IF_PAGE"

rg -q '<Конструкция языка>' "$QUERY_SYNTAX_PAGE"
rg -q 'ЭТО_КЛЮЧЕВОЕ_СЛОВО' "$QUERY_SYNTAX_PAGE"
rg -q 'Агрегатные функции' "$SUM_PAGE"
rg -q 'NULL' "$SUM_PAGE"
! rg -n '<(HTML|BODY|P|A|H1|DIV|BR)\b|&nbsp;' "$FUNC_PAGE" "$IF_PAGE" "$QUERY_SYNTAX_PAGE" "$SUM_PAGE"
```

Expected result:

- BSL and query-language syntax examples remain readable as Markdown text.
- Angle-bracket syntax placeholders such as `<Имя_функции>` and `<Конструкция языка>` are not lost
  or interpreted as raw HTML tags.
- See-also/internal link text remains visible.
- Raw HTML page structure and `&nbsp;` do not leak into the Markdown.

Cleanup:

- `target/uat/shlang-md` and `target/uat/shquery-md` are service data and may be deleted after the
  run.

## UAT-HBK-007: Markdown Export Preserves Ordinary UI Help Text

Related use case: UC-HBK-003.

Related requirements: FR-HBK-004, FR-DOC-001, FR-CLI-001.

Source pages:

- `fmtdui_ru.hbk` `form_formattedstringedit`: short page with paragraphs and line breaks.
- `htmlui_ru.hbk` `form_addtable`: ordinary HTML editor help page.
- `moxelui_ru.hbk` `form_moxelpagesetupdialog`: longer UI page with list/definition-like content.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists.
- `/opt/1cv8/x86_64/8.5.1.1150/htmlui_ru.hbk` exists.
- `/opt/1cv8/x86_64/8.5.1.1150/moxelui_ru.hbk` exists.
- `target/uat/ui-md` can be created or removed.

Steps:

```bash
rm -rf target/uat/ui-md
for book in fmtdui_ru htmlui_ru moxelui_ru; do
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
    "/opt/1cv8/x86_64/8.5.1.1150/${book}.hbk" \
    --output "target/uat/ui-md/${book}" \
    --format markdown \
    --hierarchy toc
done

FMTDUI_PAGE="$(rg -l '^# Конструктор строк на разных языках' target/uat/ui-md/fmtdui_ru -g '*.md' | head -n 1)"
HTMLUI_PAGE="$(rg -l '^# Вставка таблицы' target/uat/ui-md/htmlui_ru -g '*.md' | head -n 1)"
MOXEL_PAGE="$(rg -l '^# Параметры страницы табличного документа' target/uat/ui-md/moxelui_ru -g '*.md' | head -n 1)"
test -n "$FMTDUI_PAGE"
test -n "$HTMLUI_PAGE"
test -n "$MOXEL_PAGE"

rg -q 'интерфейсных языков' "$FMTDUI_PAGE"
rg -q 'Обычная строка' "$FMTDUI_PAGE"
rg -q 'Форматированная строка' "$FMTDUI_PAGE"

rg -q 'HTML-документы можно вставлять таблицы' "$HTMLUI_PAGE"
rg -q 'Таблица - Вставить таблицу' "$HTMLUI_PAGE"
rg -q 'Ячейки можно объединять и делить' "$HTMLUI_PAGE"

rg -q 'Файл - Параметры страницы' "$MOXEL_PAGE"
rg -q 'Колонтитулы' "$MOXEL_PAGE"
rg -q 'Авто' "$MOXEL_PAGE"
! rg -n '<(HTML|BODY|P|A|H1|UL|LI|DL|DT|DD|STRONG|SPAN)\b|&nbsp;' "$FMTDUI_PAGE" "$HTMLUI_PAGE" "$MOXEL_PAGE"
```

Expected result:

- Ordinary UI help pages retain their headings and essential user-facing prose.
- Inline formatting and line breaks are converted into readable Markdown/text without raw HTML
  scaffolding.
- Longer list/definition-like UI pages keep their important option labels and descriptions.

Cleanup:

- `target/uat/ui-md` is service data and may be deleted after the run.

## UAT-HBK-008: Raw Export Unpacks Ordinary Help Book Storage

Related use case: UC-HBK-003.

Related requirements: FR-HBK-004, FR-CLI-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists.
- `target/uat/book-raw` can be created or removed.

Steps:

```bash
rm -rf target/uat/book-raw
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
  /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk \
  --output target/uat/book-raw/fmtdui_ru \
  --format raw \
  --hierarchy raw

test -f target/uat/book-raw/fmtdui_ru/form_formattedstringedit
test "$(find target/uat/book-raw/fmtdui_ru -type f | wc -l)" -gt 0
```

Expected result:

- Exit code is `0`.
- The command summary reports the output directory, `format=raw`, `hierarchy=raw` and a non-zero
  exported file count.
- Stored `FileStorage` entries are written under normalized raw storage paths.
- The command does not create Syntax Assistant JSON export files such as `metadata.json` or
  `platform-types.json`.

Skip rule:

- If the fixture is absent, record the skip reason and do not mark the case failed.

Cleanup:

- `target/uat/book-raw` is service data and may be deleted after the run.

## UAT-HBK-009: Book Export Reports Unsupported Matrix Diagnostics

Related use case: UC-HBK-003.

Related requirements: FR-HBK-004, FR-CLI-001.

Preconditions:

- `target/uat/book-export-unsupported.err` can be created or removed.

Steps:

```bash
rm -f target/uat/book-export-unsupported.err
rm -f target/uat/missing-book-for-unsupported.hbk
! cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
  target/uat/missing-book-for-unsupported.hbk \
  --output target/uat/book-raw-unsupported \
  --format raw \
  --hierarchy toc \
  2> target/uat/book-export-unsupported.err

rg -q 'unsupported book export combination: format=raw, hierarchy=toc' \
  target/uat/book-export-unsupported.err

! cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
  target/uat/missing-book-for-unsupported.hbk \
  --output target/uat/book-md-unsupported \
  --format markdown \
  --hierarchy raw \
  2> target/uat/book-export-unsupported.err

rg -q 'unsupported book export combination: format=markdown, hierarchy=raw' \
  target/uat/book-export-unsupported.err
```

Expected result:

- Exit code is non-zero.
- The error is a stable readable unsupported-combination diagnostic, not a panic/backtrace.
- The unsupported-combination diagnostic is returned before attempting to open the HBK source file.
- The command does not invoke Syntax Assistant extraction and does not write Syntax Assistant JSON
  export files.

Cleanup:

- `target/uat/book-export-unsupported.err`, `target/uat/book-raw-unsupported` and
  `target/uat/book-md-unsupported` are service data and may be deleted after the run.

## UAT-HBK-010: Markdown Export Uses TOC Titles for Shared Content Nodes

Related use case: UC-HBK-003.

Related requirements: FR-HBK-004, FR-HBK-003, FR-CLI-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk` exists.
- `target/uat/shclang-md` can be created or removed.

Steps:

```bash
rm -rf target/uat/shclang-md
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
  /opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk \
  --output target/uat/shclang-md \
  --format markdown \
  --hierarchy toc

test -f target/uat/shclang-md/встроенный-язык/общие-объекты/index.md
test -f target/uat/shclang-md/встроенный-язык/работа-с-запросами/index.md
rg -q '^# Общие объекты$' target/uat/shclang-md/встроенный-язык/общие-объекты/index.md
rg -q '^# Работа с запросами$' target/uat/shclang-md/встроенный-язык/работа-с-запросами/index.md
! rg -n '^# Общее описание встроенного языка$' \
  target/uat/shclang-md/встроенный-язык/общие-объекты/index.md \
  target/uat/shclang-md/встроенный-язык/работа-с-запросами/index.md
```

Expected result:

- Shared service content-node placeholder pages export as heading-only Markdown.
- Each placeholder page uses its own TOC title.
- The exported headings are not borrowed from the first TOC item that points to the same service
  placeholder path.

Skip rule:

- If the fixture is absent, record the skip reason and do not mark the case failed.

Cleanup:

- `target/uat/shclang-md` is service data and may be deleted after the run.

## UAT-HBK-011: Markdown Export Converts Courier Code Tables to Code Blocks

Related use case: UC-HBK-003.

Related requirements: FR-HBK-004, FR-DOC-001, FR-CLI-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk` exists.
- `target/uat/shclang-code-md` can be created or removed.

Steps:

```bash
rm -rf target/uat/shclang-code-md
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
  /opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk \
  --output target/uat/shclang-code-md \
  --format markdown \
  --hierarchy toc

PAGE=target/uat/shclang-code-md/встроенный-язык/работа-с-запросами/выполнение-и-работа-с-запросами-во-встроенном-языке/работа-с-пакетными-запросами/index.md
test -f "$PAGE"
rg -q '^# Работа с пакетными запросами$' "$PAGE"
rg -q '^```bsl$' "$PAGE"
rg -q '^Запрос = Новый Запрос;' "$PAGE"
rg -q '^Запрос.Текст = "ВЫБРАТЬ' "$PAGE"
rg -q '^    \| УчетНоменклатуры' "$PAGE"
rg -q '^Результат=Запрос\.Выполнить\(\);' "$PAGE"
! rg -n '^\| .*Запрос = Новый Запрос|^\| -+' "$PAGE"
```

Expected result:

- The package-query example is exported as a Markdown `bsl` code block.
- Query text line breaks and leading spaces before `|` markers remain readable.
- The code example is not exported as a one-cell Markdown table.

Skip rule:

- If the fixture is absent, record the skip reason and do not mark the case failed.

Cleanup:

- `target/uat/shclang-code-md` is service data and may be deleted after the run.

## UAT-HBK-012: Markdown Export Preserves Internal Link Fragments

Related use case: UC-HBK-003.

Related requirements: FR-HBK-004, FR-DOC-001, FR-CLI-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk` exists.
- `target/uat/shclang-anchor-md` can be created or removed.

Steps:

```bash
rm -rf target/uat/shclang-anchor-md
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
  /opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk \
  --output target/uat/shclang-anchor-md \
  --format markdown \
  --hierarchy toc

PAGE=target/uat/shclang-anchor-md/встроенный-язык/общие-объекты/xbase/основные-понятия-xbase/index.md
test -f "$PAGE"
rg -q '^# Основные понятия XBASE$' "$PAGE"
rg -q '\[Поля и записи\]\(index\.md#FieldsRecords\)' "$PAGE"
rg -q '\[Работа с индексными файлами\]\(index\.md#WorkWithIndexFile\)' "$PAGE"
rg -q '\[Ограничения\]\(index\.md#constraint\)' "$PAGE"
! rg -n '\[Поля и записи\]\(index\.md\)' "$PAGE"
```

Expected result:

- Same-page table-of-contents links preserve source HTML anchor fragments.
- The link target remains the exported Markdown page path and appends the original fragment.
- Fragment anchors are not collapsed to plain `index.md`.

Skip rule:

- If the fixture is absent, record the skip reason and do not mark the case failed.

Cleanup:

- `target/uat/shclang-anchor-md` is service data and may be deleted after the run.

## UAT-HBK-013: Markdown Export Converts Courier Query Blockquotes to SDBL Code Blocks

Related use case: UC-HBK-003.

Related requirements: FR-HBK-004, FR-DOC-001, FR-CLI-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk` exists.
- `target/uat/shclang-sdbl-md` can be created or removed.

Steps:

```bash
rm -rf target/uat/shclang-sdbl-md
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- export \
  /opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk \
  --output target/uat/shclang-sdbl-md \
  --format markdown \
  --hierarchy toc

PAGE=target/uat/shclang-sdbl-md/встроенный-язык/работа-с-запросами/выполнение-и-работа-с-запросами-во-встроенном-языке/работа-с-временными-таблицами/index.md
test -f "$PAGE"
rg -q '^# Работа с временными таблицами$' "$PAGE"
rg -q '\[Менеджер временных таблиц\]\(index\.md#Manager\)' "$PAGE"
rg -q '^<a id="Manager"></a>$' "$PAGE"
rg -q '^<a id="Create"></a>$' "$PAGE"
rg -q '^<a id="Used"></a>$' "$PAGE"
rg -q '^<a id="Delete"></a>$' "$PAGE"
rg -q '^```sdbl$' "$PAGE"
rg -q '^ВЫБРАТЬ$' "$PAGE"
rg -q '^ +Код,$' "$PAGE"
rg -q '^ПОМЕСТИТЬ ВременнаяТаблица$' "$PAGE"
rg -q '^ИЗ Справочник\.Номенклатура$' "$PAGE"
! rg -n '^> ВЫБРАТЬ|^> ПОМЕСТИТЬ|^> УНИЧТОЖИТЬ' "$PAGE"
```

Expected result:

- Same-page fragment links point to materialized Markdown anchor targets.
- Courier query-language examples are exported as Markdown `sdbl` code blocks.
- Query text line breaks and indentation remain readable.
- Query-language examples are not exported as Markdown blockquotes.

Skip rule:

- If the fixture is absent, record the skip reason and do not mark the case failed.

Cleanup:

- `target/uat/shclang-sdbl-md` is service data and may be deleted after the run.

## UAT-HBK-014: Generate Documentation Site Data Artifacts

Related use case: UC-HBK-004.

Related requirements: FR-HBK-005, FR-HBK-003, FR-HBK-004, FR-DOC-001, FR-CLI-001, NFR-SITE-001.

Purpose:

- Validate that the project can generate custom documentation-site data artifacts from multiple HBK
  books without invoking MkDocs or Docusaurus.
- Validate the global TOC data contract and page-data split before web UI behavior is broadened.

Preconditions:

- The following 8.5.1.1150 books exist:
  - `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/shlang_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/shquery_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/dcsui_ru.hbk`
- `target/uat/doc-site-data` can be created or removed.

Steps:

```bash
rm -rf target/uat/doc-site-data
mkdir -p target/uat
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- site generate \
  /opt/1cv8/x86_64/8.5.1.1150 \
  --output target/uat/doc-site-data \
  --include 'fmtdui_ru.hbk' \
  --include 'shlang_ru.hbk' \
  --include 'shquery_ru.hbk' \
  --include 'dcsui_ru.hbk' \
  2> target/uat/doc-site-data.progress.log

rg -q 'progress: source books discovered' target/uat/doc-site-data.progress.log
rg -q 'progress: loading source books' target/uat/doc-site-data.progress.log
rg -q 'progress: site data planned' target/uat/doc-site-data.progress.log
rg -q 'progress: writing artifacts' target/uat/doc-site-data.progress.log

test -f target/uat/doc-site-data/data/manifest.json
test -f target/uat/doc-site-data/data/locales/ru/toc-root.json
test "$(find target/uat/doc-site-data/data/locales/ru/pages -name '*.md' -type f | wc -l)" -gt 10
{
  echo target/uat/doc-site-data/data/locales/ru/toc-root.json
  find target/uat/doc-site-data/data/locales/ru/toc-sections -name '*.json' -type f | sort
} > target/uat/doc-site-data/toc-json-files.txt

jq -e '.schema_version >= 1' target/uat/doc-site-data/data/manifest.json
jq -e '.locales | index("ru")' target/uat/doc-site-data/data/manifest.json
jq -e '.books.ru | length >= 4' target/uat/doc-site-data/data/manifest.json
jq -s -e '
  [.. | objects | select((.page_id // "") != "")] as $pages
  | ($pages | length) > 10
  and all($pages[]; ((.book_id // "") | length) > 0)
' $(cat target/uat/doc-site-data/toc-json-files.txt)
jq -s -r '.. | objects | select((.page_id // "") != "") | .page_id' \
  $(cat target/uat/doc-site-data/toc-json-files.txt) |
while IFS= read -r page_id; do
  test -f "target/uat/doc-site-data/data/locales/ru/pages/${page_id}.md"
done

! rg -n '/opt/1cv8|\.hbk\b|objects/.+\.html|toc[_ -]?index' \
  target/uat/doc-site-data/data/locales/ru -g '*.json' -g '*.md'
```

Expected result:

- The command exits with code `0`.
- The generated artifact contains a generated data manifest, locale TOC data and page Markdown
  files.
- The manifest records source book inventory and available locales.
- TOC data uses stable generated ids and includes source book identity for page-bearing nodes.
- Visible generated page/TOC content does not leak raw installed HBK paths, raw TOC indexes or raw
  HTML storage paths.
- The redirected progress stream reports source discovery, source-book loading, site planning and
  sparse artifact-writing milestones on `stderr`.
- The command summary reports source book count, generated page count, output size, build timing and
  peak RSS or equivalent.
- The implementation task records the first real measurement or skip reason in
  `spec/acceptance/baseline.md`.

Skip rule:

- If one or more listed HBK books are absent, record the missing paths and do not mark the case
  failed.

Cleanup:

- `target/uat/doc-site-data` is service data and may be deleted after the run.

## UAT-HBK-015: Serve Documentation Web App and Open a Generated Page

Related use case: UC-HBK-004.

Related requirements: FR-HBK-005, FR-CLI-001, NFR-SITE-001.

Purpose:

- Validate that the separate web app serves/loads generated documentation-site data.
- Validate that the web bundle or initial server response loads TOC/page data lazily.

Preconditions:

- UAT-HBK-014 has generated `target/uat/doc-site-data`.
- The documentation web app can be run against that generated data directory.
- Playwright or an equivalent browser smoke runner is available.

Steps:

```bash
npm --prefix web/docs-viewer run build
npm --prefix web/docs-viewer start -- \
  --data "$PWD/target/uat/doc-site-data/data" \
  --listen 127.0.0.1:4173
```

In a browser or smoke runner:

- open `http://127.0.0.1:4173/`;
- wait for the root TOC to load;
- expand a section under the Russian locale;
- open a documentation page;
- verify that page content appears.

Expected result:

- The documentation web app opens and serves/loads generated data artifacts.
- No HBK parsing or Syntax Assistant extraction is invoked in web request paths.
- Network requests show separate loads for manifest, TOC data and page content.
- The initial JavaScript bundle or server response does not contain representative page Markdown
  strings from generated pages.
- Navigation from global TOC to a page works without a full page reload.

Skip rule:

- If browser automation or the first web app slice is unavailable, record the missing prerequisite
  and keep UAT-HBK-014 as the file-level acceptance gate for the current environment.

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
python3 - <<'PY'
import sqlite3
con = sqlite3.connect("file:target/uat/sh-search-ru.sqlite?mode=ro", uri=True)
cur = con.cursor()
assert cur.execute("select value from meta where key='schema_version'").fetchone()[0] == "6"
document_columns = {row[1] for row in cur.execute("pragma table_info(documents)")}
assert "signature_json" not in document_columns
assert "preview" not in document_columns
assert not cur.execute(
    "select 1 from documents where id like '%.html%' or id like '%/%' or id like '%#&^@^%&*^#%' limit 1"
).fetchone()
assert not cur.execute(
    "select 1 from documents where kind='platform_type' and id like '%Параметры формы%' limit 1"
).fetchone()
assert cur.execute(
    "select 1 from documents where kind='query_table' and id like 'query_table:РегистрБухгалтерии:%' limit 1"
).fetchone()
assert cur.execute(
    "select 1 from relations where source_id like 'query_table:РегистрБухгалтерии:%' and target_id like 'query_table_field:query_table:РегистрБухгалтерии:%' limit 1"
).fetchone()
assert not cur.execute(
    "select 1 from documents where id like 'query_table_field:query_table:Основная таблица:%' or id like 'query_table_parameter:query_table:Основная таблица:%' limit 1"
).fetchone()
assert cur.execute(
    "select 1 from relations where source_id='query_table:Задача' and target_id='query_table_field:query_table:Задача:<Имя общего реквизита>' limit 1"
).fetchone()
assert cur.execute(
    "select count(*) from documents where id='type_event:owner:События:ОбработкаВыбора'"
).fetchone()[0] == 0
assert cur.execute(
    "select count(*) from documents where kind='type_event' and name_primary='ОбработкаВыбора' and id like 'type_event:owner:%.%:%'"
).fetchone()[0] >= 2
assert cur.execute(
    "select count(*) from documents where id='constructor:platform_type:МенеджерКриптографии:Новый МенеджерКриптографии(<ИспользованиеИнтерактивногоРежима>)'"
).fetchone()[0] == 1
assert cur.execute(
    "select count(*) from documents where id in ('enum:system:ИспользованиеТекущейСтроки:SelectedRowsUse', 'enum:system:ИспользованиеТекущейСтроки:CurrentRowUse')"
).fetchone()[0] == 2
for table in ["type_identities", "members", "callables", "signatures", "parameters", "type_refs"]:
    assert cur.execute(f"select count(*) from {table}").fetchone()[0] > 0, table
assert cur.execute("""
select 1 from sqlite_master
where type='index'
  and name='type_identities_document_idx'
limit 1
""").fetchone()
for index_name in ["members_document_owner_idx", "callables_document_owner_idx"]:
    assert cur.execute("""
    select 1 from sqlite_master
    where type='index' and name=?
    limit 1
    """, (index_name,)).fetchone()
assert cur.execute("""
select 1 from parameters p
join type_refs r on r.source_signature_id = p.signature_id
 and r.source_parameter_ordinal = p.ordinal
where p.name='ИспользоватьАутентификациюОС'
  and r.ref_kind='parameter_type'
  and r.target_type_name='Булево'
limit 1
""").fetchone()
assert cur.execute("""
select 1 from members
where owner_type_id='platform_type:НастройкиКомпоновкиДанных'
  and member_kind='type_property'
  and name_primary='Отбор'
limit 1
""").fetchone()
assert cur.execute("""
select 1 from type_refs
where source_document_id='type_property:platform_type:НастройкиКомпоновкиДанных:Отбор'
  and ref_kind='property_type'
  and target_type_id='platform_type:ОтборКомпоновкиДанных'
limit 1
""").fetchone()
assert not cur.execute("""
select 1 from type_refs r
where r.target_type_name in (
  select name_primary from type_identities group by name_primary having count(*) > 1
)
and r.target_type_id is not null
limit 1
""").fetchone()
PY
```

Expected result:

- Exit code is `0`.
- The index artifact is a SQLite database.
- The database contains schema metadata plus deterministic document, exact-name, FTS and
  relationship-edge data.
- The database uses current search-index schema version `13`; analyzer-critical callable, parameter,
  member and type-reference facts are present in normalized relational tables rather than
  `documents.signature_json` or presentation-only `documents.preview` columns.
- Type identity and exact owner-type member/callable lookup have indexed joins from lookup keys back
  to normalized rows.
- Type references to duplicate platform type names keep source `target_type_name`, store
  `target_resolution_status="ambiguous"` plus deterministic candidate ids, and do not silently pin
  `target_type_id` to one hidden winner.
- The index build records locale `ru`, source locale `ru`, source HBK identity and index/extraction
  schema version.
- The index command uses the effective index path from `V8_CONTEXT_HBK_SYNTAX_INDEX` when `--output`
  is omitted.
- The index command creates `.v8-context-hbk/syntax/index.sqlite` when both `--output` and
  `V8_CONTEXT_HBK_SYNTAX_INDEX` are absent.
- The index command removes stale temporary replacement database artifacts before creating a new
  replacement index.
- The completed replacement index does not leave active SQLite WAL/SHM sidecars beside the default
  artifact.
- The default-path lookup command resolves `.v8-context-hbk/syntax/index.sqlite` when `--index` and
  `V8_CONTEXT_HBK_SYNTAX_INDEX` are absent.
- Later query commands do not require the HBK file path.
- Document ids do not contain HBK/HTML path fragments or TOC duplicate-title markers.
- Duplicated query table identifiers use semantic table-family variants in document ids and
  relation endpoints.
- Query table field and parameter ids use the final parent query-table id; generic table titles
  such as `query_table:Основная таблица` are not used as member owners.
- Type event ids use the composed TOC-derived semantic owner and do not collapse under generic
  event-group owners such as `type_event:owner:События`.
- Duplicate source pages for the `МенеджерКриптографии` constructor signature are reported during
  index build and produce one constructor document instead of aborting the rebuild.
- Distinct `ИспользованиеТекущейСтроки` system enums with different aliases produce separate
  enum documents.
- Form/form-extension `Параметры формы` pages are not indexed as `platform_type` records.

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
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --format json > target/uat/related-filter-by-id.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json > target/uat/related-filter-by-owner-member.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax constructors "HTTPСоединение" > target/uat/constructors-httpconnection.txt
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax constructors "HTTPСоединение" --details > target/uat/constructors-httpconnection-details.txt
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax constructors "HTTPСоединение" --format json > target/uat/constructors-httpconnection.json
jq -e '.schema_version == 1 and .command == "constructors" and .status == "ok"' target/uat/constructors-httpconnection.json
jq -e 'all(.results[]; .fact | has("parameters") | not)' target/uat/constructors-httpconnection.json
jq -e 'any(.results[].fact.signatures[]?.parameters[]?; .name == "ИспользоватьАутентификациюОС" and .required == false and (.types | index("Булево") != null))' target/uat/constructors-httpconnection.json
jq -e 'all(.results[].fact.signatures[]?; has("text") | not)' target/uat/constructors-httpconnection.json
jq -e '.status == "ok" and any(.results[].fact; .kind == "platform_type" and .name.primary == "ОтборКомпоновкиДанных")' target/uat/related-filter-by-id.json
jq -e '.status == "ok" and any(.results[].fact; .kind == "platform_type" and .name.primary == "ОтборКомпоновкиДанных")' target/uat/related-filter-by-owner-member.json
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
- Constructor output for `HTTPСоединение` contains its overload signatures directly, including the
  overload with `<Таймаут>`, `<ЗащищенноеСоединение>` and `<ИспользоватьАутентификациюОС>`.
- Constructor details output still contains the overload signatures and adds available owner and
  description context without requiring JSON post-processing.
- Constructor JSON for `HTTPСоединение` does not expose mixed `document.parameters`; callable
  details use structured `signatures[].parameters[]` objects with `name`, `required`, `types` and
  optional `description`, and signature text remains presentation data.
- The command returns within the NFR-QUERY-001 provisional target when measured on the target
  workstation.
- Relationship traversal can start from the exact property document id and from
  `--owner "НастройкиКомпоновкиДанных" --member "Отбор"`, so analyzer workflows do not have to
  rely on ambiguous plain-name roots.

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

## UAT-SH-016: Provider JSON Contract Review

Related use case: UC-SH-005D.

Related requirements: FR-SH-PROVIDER-001.

Preconditions:

- `target/uat/sh-search-ru.sqlite` exists from UAT-SH-004.
- Provider-envelope implementation is in scope for the task being verified.

Steps:

```bash
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax constructors "HTTPСоединение" --format json > target/uat/provider-constructors-httpconnection.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --format json > target/uat/provider-get-skd-filter-by-id.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json > target/uat/provider-get-skd-filter.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax search --query "отбор скд" --mode keywords --format json > target/uat/provider-search-skd-filter.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --name "ОтборКомпоновкиДанных" --format json > target/uat/provider-related-skd-filter.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --format json > target/uat/provider-related-skd-filter-by-id.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json > target/uat/provider-related-skd-filter-by-owner-member.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --name "НЕСУЩЕСТВУЮЩИЙ_API_ДЛЯ_UAT" --format json > target/uat/provider-get-missing.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --name "Добавить" --format json > target/uat/provider-get-ambiguous.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --name "ОтборКомпоновкиДанных" --format json > target/uat/provider-related-unsupported-root.json

jq -e '.schema_version == 1 and .status == "ok" and .command == "constructors" and (.results | length > 0)' target/uat/provider-constructors-httpconnection.json
jq -e 'all(.results[]; has("fact") and (.fact | has("parameters") | not) and (.fact | has("type_refs") | not) and (.fact | has("return_types") | not))' target/uat/provider-constructors-httpconnection.json
jq -e 'any(.results[].fact.signatures[]?.parameters[]?; .name == "ИспользоватьАутентификациюОС" and .required == false and (.types | index("Булево") != null))' target/uat/provider-constructors-httpconnection.json
jq -e '.command == "get" and .query.id == "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" and any(.results[].fact; .kind == "type_property" and .owner == "НастройкиКомпоновкиДанных" and (.types | index("ОтборКомпоновкиДанных") != null))' target/uat/provider-get-skd-filter-by-id.json
jq -e '.command == "get" and .status == "ok" and any(.results[].fact; .kind == "type_property" and .owner == "НастройкиКомпоновкиДанных" and (.types | index("ОтборКомпоновкиДанных") != null))' target/uat/provider-get-skd-filter.json
jq -e '.command == "search" and all(.results[]; .meta.rank >= 1 and (.meta | has("score")))' target/uat/provider-search-skd-filter.json
jq -e '.command == "related" and all(.results[]; (.meta.depth >= 0) and (.meta | has("path")))' target/uat/provider-related-skd-filter.json
jq -e '.command == "related" and .query.root.id == "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" and any(.results[].fact; .name.primary == "ОтборКомпоновкиДанных")' target/uat/provider-related-skd-filter-by-id.json
jq -e '.command == "related" and .query.root.owner == "НастройкиКомпоновкиДанных" and .query.root.member == "Отбор" and any(.results[].fact; .name.primary == "ОтборКомпоновкиДанных")' target/uat/provider-related-skd-filter-by-owner-member.json
jq -e '.command == "get" and .status == "not_found" and (.results | length == 0) and any(.diagnostics[]; .code == "NOT_FOUND")' target/uat/provider-get-missing.json
jq -e '.command == "get" and .status == "ambiguous" and (.results | length == 0) and any(.diagnostics[]; .code == "AMBIGUOUS" and (.candidates | length > 1))' target/uat/provider-get-ambiguous.json
jq -e '.command == "related" and .status == "unsupported" and (.results | length == 0) and any(.diagnostics[]; .code == "UNSUPPORTED_QUERY")' target/uat/provider-related-unsupported-root.json
jq -s -e 'def forbidden_internal:
  has("source") or has("source_hbk") or has("toc_path") or has("html_path") or has("page_title") or
  has("rowid") or has("parameter_text") or has("parameter_terms") or has("relation_keys") or
  has("type_refs") or has("return_types");
  all(.[]; all(.results[].fact; (has("parameters") | not) and ((.. | objects | forbidden_internal) | not)))
' target/uat/provider-constructors-httpconnection.json target/uat/provider-get-skd-filter-by-id.json target/uat/provider-get-skd-filter.json target/uat/provider-search-skd-filter.json target/uat/provider-related-skd-filter.json
```

Expected result:

- Exit code is `0`.
- All provider JSON responses use the same versioned envelope with `schema_version`, `command`,
  `status`, `query`, `results` and `diagnostics`.
- Shared platform facts are under `results[].fact` and use export-compatible field names such as
  `signatures`, `signatures[].parameters[]`, `types` and `return`.
- Callable return facts preserve source scope: page-level/shared return evidence stays in
  fact-level `return`; source-proven overload return evidence, when present, uses the same
  export-compatible `return` field under the matching `signatures[]` item.
- Query-only data is under `results[].meta`: search score/rank for `syntax search`, relationship
  depth/path for `syntax related`.
- Owned facts use the export-compatible `owner` string shape. Richer owner identity needed only by
  provider queries belongs in `results[].meta`, not in the shared fact shape.
- Public facts do not expose FTS/search token fields, mixed `parameters` arrays, `type_refs`,
  `return_types`, SQLite rowids or HBK/TOC/HTML/page-title provenance.
- Missing and ambiguous exact lookups are represented through `status` and `diagnostics` rather
  than by silently choosing an internal winner.

## UAT-SH-017: BSL Task Scenario Provider Queries

Related use case: UC-SH-005A, UC-SH-005B, UC-SH-005C.

Related requirements: FR-SH-SEARCH-001, FR-SH-SEARCH-002, FR-SH-PROVIDER-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- `target/uat/t53-sh-search-ru.sqlite` can be created or removed.

Steps:

```bash
rm -f target/uat/t53-sh-search-ru.sqlite
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/uat/t53-sh-search-ru.sqlite

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t53-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax constructors "HTTPСоединение" --format json \
  > target/uat/t53-constructors-httpconnection.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t53-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json \
  > target/uat/t53-get-skd-filter.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t53-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json \
  > target/uat/t53-related-skd-filter.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t53-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax search --query "таблица регистра бухгалтерии" --mode keywords --format json \
  > target/uat/t53-search-accounting-register-table.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t53-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --id "query_table:РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии" --format json \
  > target/uat/t53-related-accounting-register-changes.json

jq -e '
  .status == "ok"
  and any(.results[].fact.signatures[]?.parameters[]?;
    .name == "Таймаут" and (.types | index("Число") != null))
  and any(.results[].fact.signatures[]?.parameters[]?;
    .name == "ЗащищенноеСоединение" and (.types | index("ЗащищенноеСоединениеOpenSSL") != null))
  and any(.results[].fact.signatures[]?.parameters[]?;
    .name == "ИспользоватьАутентификациюОС" and (.types | index("Булево") != null))
' target/uat/t53-constructors-httpconnection.json
jq -e '
  .status == "ok"
  and any(.results[].fact;
    .kind == "type_property"
    and .owner == "НастройкиКомпоновкиДанных"
    and .name.primary == "Отбор"
    and (.types | index("ОтборКомпоновкиДанных") != null))
' target/uat/t53-get-skd-filter.json
jq -e '
  .status == "ok"
  and any(.results[].fact; .name.primary == "ОтборКомпоновкиДанных")
  and any(.results[].fact; .name.primary == "Элементы")
  and any(.results[].fact;
    .kind == "type_method"
    and .owner == "КоллекцияЭлементовОтбораКомпоновкиДанных"
    and .name.primary == "Добавить")
  and any(.results[].fact;
    .kind == "type_property"
    and .owner == "ЭлементОтбораКомпоновкиДанных"
    and .name.primary == "ЛевоеЗначение")
  and any(.results[].fact;
    .kind == "type_property"
    and .owner == "ЭлементОтбораКомпоновкиДанных"
    and .name.primary == "ВидСравнения")
  and any(.results[].fact;
    .kind == "type_property"
    and .owner == "ЭлементОтбораКомпоновкиДанных"
    and .name.primary == "ПравоеЗначение")
  and any(.results[].fact;
    .kind == "type_property"
    and .owner == "ЭлементОтбораКомпоновкиДанных"
    and .name.primary == "Использование")
' target/uat/t53-related-skd-filter.json
jq -e '
  .status == "ok"
  and .results[0].fact.id == "query_table:РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии"
  and any(.results[].fact;
    .kind == "query_table"
    and (.id | startswith("query_table:РегистрБухгалтерии")))
' target/uat/t53-search-accounting-register-table.json
jq -e '
  .status == "ok"
  and any(.results[].fact; .kind == "query_table_field" and .name.primary == "Регистратор")
  and any(.results[].fact; .kind == "query_table_field" and .name.primary == "НомерСообщения")
' target/uat/t53-related-accounting-register-changes.json
```

Expected result:

- Exit code is `0`.
- The constructor-call scenario for `Новый HTTPСоединение(...)` exposes structured parameters for
  timeout, secure connection and OS authentication, with type references under `types`.
- The owner/member scenario for `НастройкиКомпоновкиДанных.Отбор` returns the exact property fact,
  its owner and the `ОтборКомпоновкиДанных` type reference.
- Relationship traversal from the SKD filter property reaches the referenced filter type, its
  `Элементы` property, collection `Добавить` method and filter-item properties
  `ЛевоеЗначение`, `ВидСравнения`, `ПравоеЗначение` and `Использование`.
- The task-oriented query `таблица регистра бухгалтерии` ranks a source-backed accounting-register
  query table first and keeps other accounting-register table facts in the result set.
- Relationship traversal from the accepted accounting-register query table id exposes documented
  query-table fields such as `Регистратор` and `НомерСообщения`.
- Raw JSON and SQLite artifacts are service data under `target/uat`; only these commands,
  assertions and conclusions are durable.

Cleanup:

- `target/uat/t53-sh-search-ru.sqlite` and `target/uat/t53-*.json` are service data and may be
  deleted after the run.

## UAT-SH-018: Expression-Chain Provider Primitives

Related use case: UC-SH-005A, UC-SH-005B, UC-SH-005D.

Related requirements: FR-SH-PROVIDER-001, FR-SH-SEARCH-001, FR-SH-SEARCH-002.

Status: implementation UAT for T58 and expression-chain scenario UAT for T59.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- A fresh schema-v4 or later provider index can be built under `target/uat/`.

Source-backed scenario:

- Model the BSL expression chain as explicit provider calls. This repository does not parse BSL
  source in this UAT.
- Resolve `НастройкиКомпоновкиДанных.Отбор` to a property fact and the type identity
  `ОтборКомпоновкиДанных`.
- List members for `ОтборКомпоновкиДанных` and verify the `Элементы` property.
- Follow the `Элементы` type reference to the filter item collection type.
- Resolve the collection `Добавить` method, retrieve its callable facts and verify the creation path
  to `ЭлементОтбораКомпоновкиДанных`.
- List members for `ЭлементОтбораКомпоновкиДанных` and verify fields needed by the accepted SKD
  filter scenario: `ЛевоеЗначение`, `ВидСравнения`, `ПравоеЗначение` and `Использование`.
- Resolve `Новый HTTPСоединение(...)` through type identity and callable-overload provider queries,
  verifying constructor result type plus ordered parameters such as `Таймаут`,
  `ЗащищенноеСоединение` and `ИспользоватьАутентификациюОС`.

T58 primitive implementation steps:

```bash
rm -f target/uat/t58-sh-search-ru.sqlite target/uat/t58-*.json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/uat/t58-sh-search-ru.sqlite

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t58-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get --kind platform_type --name "ОтборКомпоновкиДанных" --format json \
  > target/uat/t58-type-filter.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t58-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get --kind platform_type --alias "DataCompositionFilter" --format json \
  > target/uat/t58-type-filter-alias.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t58-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get --members-of "platform_type:ОтборКомпоновкиДанных" --format json \
  > target/uat/t58-members-filter.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t58-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get \
    --owner-type-id "platform_type:НастройкиКомпоновкиДанных" --member "Отбор" --format json \
  > target/uat/t58-owner-type-member-filter.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t58-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get \
    --owner-type-id "platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных" \
    --callable "Добавить" --format json \
  > target/uat/t58-callable-add.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t58-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax constructors "HTTPСоединение" --format json \
  > target/uat/t58-constructors-httpconnection.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t58-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax related \
    --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" \
    --edge has_type --format json \
  > target/uat/t58-related-filter-type.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t58-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get --members-of "ОтборКомпоновкиДанных" --format json \
  > target/uat/t58-members-unsupported-name.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t58-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax related \
    --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" \
    --name "ОтборКомпоновкиДанных" --edge has_type --format json \
  > target/uat/t58-related-unsupported-mixed-root.json

jq -e '.schema_version == 1 and .command == "get" and .status == "ok" and
  .query.kind == "type_identity" and
  .results[0].fact.id == "platform_type:ОтборКомпоновкиДанных"' \
  target/uat/t58-type-filter.json
jq -e '.status == "ok" and .query.alias == "DataCompositionFilter" and
  .results[0].fact.id == "platform_type:ОтборКомпоновкиДанных"' \
  target/uat/t58-type-filter-alias.json
jq -e '.status == "ok" and .query.kind == "member_list" and
  any(.results[].fact; .kind == "type_property" and .name.primary == "Элементы" and
    (.types | index("КоллекцияЭлементовОтбораКомпоновкиДанных") != null))' \
  target/uat/t58-members-filter.json
jq -e '.status == "ok" and .query.kind == "owner_type_member" and
  any(.results[]; .fact.id == "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" and
    (.fact.types | index("ОтборКомпоновкиДанных") != null) and
    (.meta.target_type_ids | index("platform_type:ОтборКомпоновкиДанных") != null))' \
  target/uat/t58-owner-type-member-filter.json
jq -e '.status == "ok" and .query.kind == "callable_overloads" and
  any(.results[]; .fact.kind == "type_method" and
    .fact.owner == "КоллекцияЭлементовОтбораКомпоновкиДанных" and
    .fact.name.primary == "Добавить" and
    (.fact.return | index("ЭлементОтбораКомпоновкиДанных") != null))' \
  target/uat/t58-callable-add.json
jq -e '.status == "ok" and .query.kind == "constructor" and
  any(.results[].fact.signatures[]?.parameters[]?;
    .name == "ИспользоватьАутентификациюОС" and .required == false and
    (.types | index("Булево") != null))' \
  target/uat/t58-constructors-httpconnection.json
jq -e '.status == "ok" and .query.kind == "type_references" and .query.edge == "has_type" and
  any(.results[]; .fact.id == "platform_type:ОтборКомпоновкиДанных")' \
  target/uat/t58-related-filter-type.json
jq -e '.status == "unsupported" and
  any(.diagnostics[]; .code == "UNSUPPORTED_QUERY")' \
  target/uat/t58-members-unsupported-name.json
jq -e '.status == "unsupported" and
  any(.diagnostics[]; .code == "UNSUPPORTED_QUERY")' \
  target/uat/t58-related-unsupported-mixed-root.json
```

T59 expression-chain scenario steps:

```bash
rm -f target/uat/t59-sh-search-ru.sqlite target/uat/t59-*.json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/uat/t59-sh-search-ru.sqlite

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t59-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get \
    --owner-type-id "platform_type:НастройкиКомпоновкиДанных" --member "Отбор" --format json \
  > target/uat/t59-01-filter-property.json
FILTER_TYPE_ID="$(jq -r '
  .results[]
  | select(.fact.id == "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор")
  | .meta.target_type_ids[]?
' target/uat/t59-01-filter-property.json)"
test "$FILTER_TYPE_ID" = "platform_type:ОтборКомпоновкиДанных"

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t59-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get --members-of "$FILTER_TYPE_ID" --format json \
  > target/uat/t59-02-filter-members.json
COLLECTION_TYPE_ID="$(jq -r '
  .results[]
  | select(.fact.kind == "type_property" and .fact.name.primary == "Элементы")
  | .meta.target_type_ids[]?
' target/uat/t59-02-filter-members.json)"
test "$COLLECTION_TYPE_ID" = "platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных"
ELEMENTS_PROPERTY_ID="$(jq -r '
  .results[]
  | select(.fact.kind == "type_property" and .fact.name.primary == "Элементы")
  | .fact.id
' target/uat/t59-02-filter-members.json)"
test "$ELEMENTS_PROPERTY_ID" = "type_property:platform_type:ОтборКомпоновкиДанных:Элементы"

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t59-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax related --id "$ELEMENTS_PROPERTY_ID" --edge has_type --format json \
  > target/uat/t59-03-elements-type.json

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t59-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get --members-of "$COLLECTION_TYPE_ID" --format json \
  > target/uat/t59-04-collection-members.json
ADD_CALLABLE_ID="$(jq -r '
  .results[]
  | select(.fact.kind == "type_method" and .fact.name.primary == "Добавить")
  | .fact.id
' target/uat/t59-04-collection-members.json)"
test "$ADD_CALLABLE_ID" = "type_method:platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных:Добавить"
ITEM_TYPE_ID="$(jq -r '
  .results[]
  | select(.fact.id == "type_method:platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных:Добавить")
  | .meta.target_type_ids[]?
  | select(. == "platform_type:ЭлементОтбораКомпоновкиДанных")
' target/uat/t59-04-collection-members.json)"
test "$ITEM_TYPE_ID" = "platform_type:ЭлементОтбораКомпоновкиДанных"

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t59-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get --callable-id "$ADD_CALLABLE_ID" --format json \
  > target/uat/t59-05-add-callable.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t59-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get --members-of "$ITEM_TYPE_ID" --format json \
  > target/uat/t59-06-filter-item-members.json

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t59-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax get --kind platform_type --name "HTTPСоединение" --format json \
  > target/uat/t59-07-http-type.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t59-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax constructors "HTTPСоединение" --format json \
  > target/uat/t59-08-http-constructors.json
HTTP_CONSTRUCTOR_ID="$(jq -r '
  .results[]
  | select(any(.fact.signatures[]?.parameters[]?;
      .name == "ИспользоватьАутентификациюОС" and (.types | index("Булево") != null)))
  | .fact.id
' target/uat/t59-08-http-constructors.json)"
test -n "$HTTP_CONSTRUCTOR_ID"
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t59-sh-search-ru.sqlite \
  target/debug/v8-context-hbk syntax related --id "$HTTP_CONSTRUCTOR_ID" --edge constructs --format json \
  > target/uat/t59-09-http-constructor-result.json

jq -e '.status == "ok" and .query.kind == "owner_type_member" and
  any(.results[]; .fact.id == "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" and
    (.fact.types | index("ОтборКомпоновкиДанных") != null) and
    (.meta.target_type_ids | index("platform_type:ОтборКомпоновкиДанных") != null))' \
  target/uat/t59-01-filter-property.json
jq -e '.status == "ok" and .query.kind == "member_list" and
  any(.results[]; .fact.kind == "type_property" and .fact.name.primary == "Элементы" and
    (.fact.types | index("КоллекцияЭлементовОтбораКомпоновкиДанных") != null) and
    (.meta.target_type_ids | index("platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных") != null))' \
  target/uat/t59-02-filter-members.json
jq -e '.status == "ok" and .query.kind == "type_references" and .query.edge == "has_type" and
  any(.results[]; .fact.id == "platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных")' \
  target/uat/t59-03-elements-type.json
jq -e '.status == "ok" and .query.kind == "member_list" and
  any(.results[]; .fact.id == "type_method:platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных:Добавить" and
    (.fact.return | index("ЭлементОтбораКомпоновкиДанных") != null) and
    (.meta.target_type_ids | index("platform_type:ЭлементОтбораКомпоновкиДанных") != null))' \
  target/uat/t59-04-collection-members.json
jq -e '.status == "ok" and .query.kind == "callable_overloads" and
  any(.results[]; .fact.id == "type_method:platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных:Добавить" and
    (.fact.return | index("ЭлементОтбораКомпоновкиДанных") != null))' \
  target/uat/t59-05-add-callable.json
jq -e '.status == "ok" and .query.kind == "member_list" and
  any(.results[].fact; .kind == "type_property" and .name.primary == "ЛевоеЗначение") and
  any(.results[].fact; .kind == "type_property" and .name.primary == "ВидСравнения") and
  any(.results[].fact; .kind == "type_property" and .name.primary == "ПравоеЗначение") and
  any(.results[].fact; .kind == "type_property" and .name.primary == "Использование")' \
  target/uat/t59-06-filter-item-members.json
jq -e '.status == "ok" and .query.kind == "type_identity" and
  .results[0].fact.id == "platform_type:HTTPСоединение"' \
  target/uat/t59-07-http-type.json
jq -e '.status == "ok" and .query.kind == "constructor" and
  any(.results[].fact.signatures[]?.parameters[]?;
    .name == "Таймаут" and (.types | index("Число") != null)) and
  any(.results[].fact.signatures[]?.parameters[]?;
    .name == "ЗащищенноеСоединение" and (.types | index("ЗащищенноеСоединениеOpenSSL") != null)) and
  any(.results[].fact.signatures[]?.parameters[]?;
    .name == "ИспользоватьАутентификациюОС" and .required == false and
      (.types | index("Булево") != null))' \
  target/uat/t59-08-http-constructors.json
jq -e '.status == "ok" and .query.kind == "type_references" and .query.edge == "constructs" and
  any(.results[]; .fact.id == "platform_type:HTTPСоединение")' \
  target/uat/t59-09-http-constructor-result.json
jq -s -e 'def forbidden_internal:
  has("source") or has("source_hbk") or has("toc_path") or has("html_path") or has("page_title") or
  has("rowid") or has("table") or has("sqlite") or has("parameter_text") or has("parameter_terms") or
  has("relation_keys") or has("type_refs") or has("return_types");
  all(.[]; ((.. | objects | forbidden_internal) | not))
' target/uat/t59-*.json
```

Expected result:

- All calls use provider commands and JSON only; no SQLite table names, rowids, HBK paths, TOC
  paths, HTML paths or page titles are asserted.
- Type identity, member listing, owner-type/member lookup, callable lookup and type-reference
  traversal use provider `query.kind` values rather than public SQLite table names.
- The expression-chain scenario derives `ОтборКомпоновкиДанных`,
  `КоллекцияЭлементовОтбораКомпоновкиДанных` and `ЭлементОтбораКомпоновкиДанных` from provider
  JSON returned by earlier calls instead of parsing BSL source or reading SQLite tables.
- The constructor-chain scenario verifies `HTTPСоединение` type identity, constructor result
  traversal through the `constructs` edge and structured constructor parameters.
- Analyzer resolution aids such as `owner_type_id` and `target_type_ids` are returned only under
  `results[].meta`; shared facts stay under `results[].fact`.
- Ambiguous, missing or unsupported primitive calls return provider `status` and diagnostics
  instead of selecting hidden winners.
- Raw command outputs remain service data under `target/uat`; only the commands, assertions and
  conclusions are durable.

## UAT-SH-019: Analyzer Provider Ambiguity Handling

Related use case: UC-SH-005B, UC-SH-005D.

Related requirements: FR-SH-PROVIDER-001, FR-SH-SEARCH-001.

Status: implementation UAT for T60.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- A fresh schema-v4 or later provider index can be built under `target/uat/`.

Steps:

```bash
rm -f target/uat/t60-sh-search-ru.sqlite target/uat/t60-*.json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/uat/t60-sh-search-ru.sqlite

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t60-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --kind platform_type --name "ЭлементыФормы" --format json \
  > target/uat/t60-get-type-ambiguous.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t60-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --owner "ЭлементыФормы" --member "Добавить" --format json \
  > target/uat/t60-get-owner-member-ambiguous-owner.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t60-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --owner "ЭлементыФормы" --member "Добавить" --format json \
  > target/uat/t60-related-owner-member-ambiguous-owner.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t60-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax constructors "ЭлементыФормы" --format json \
  > target/uat/t60-constructors-ambiguous-type.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t60-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --name "ОтборКомпоновкиДанных" --format json \
  > target/uat/t60-get-name-ownerless-collision.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t60-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax get --owner-type-id "platform_type:ЭлементыФормы:Форма" --member "Добавить" --format json \
  > target/uat/t60-get-owner-type-member-ok.json

jq -e '.status == "ambiguous" and (.results | length == 0)
  and any(.diagnostics[]; .code == "AMBIGUOUS"
    and ([.candidates[].id] | index("platform_type:ЭлементыФормы:Форма") != null)
    and ([.candidates[].id] | index("platform_type:ЭлементыФормы:Форма клиентского приложения") != null))' \
  target/uat/t60-get-type-ambiguous.json
jq -e '.status == "ambiguous" and (.results | length == 0)
  and any(.diagnostics[]; .code == "AMBIGUOUS"
    and (.candidates | length == 2)
    and all(.candidates[]; .kind == "platform_type"))' \
  target/uat/t60-get-owner-member-ambiguous-owner.json
jq -e '.status == "ambiguous" and (.results | length == 0)
  and any(.diagnostics[]; .code == "AMBIGUOUS"
    and (.candidates | length == 2)
    and all(.candidates[]; .kind == "platform_type"))' \
  target/uat/t60-related-owner-member-ambiguous-owner.json
jq -e '.status == "ambiguous" and (.results | length == 0)
  and any(.diagnostics[]; .code == "AMBIGUOUS"
    and (.candidates | length == 2)
    and all(.candidates[]; .kind == "platform_type"))' \
  target/uat/t60-constructors-ambiguous-type.json
jq -e '.status == "ambiguous" and (.results | length == 0)
  and any(.diagnostics[]; .code == "AMBIGUOUS"
    and ([.candidates[].id] | index("platform_type:ОтборКомпоновкиДанных") != null)
    and ([.candidates[].id] | index("type_property:platform_type:БиблиотекаКартинок:ОтборКомпоновкиДанных") != null))' \
  target/uat/t60-get-name-ownerless-collision.json
jq -e '.status == "ok"
  and any(.results[].fact; .id == "type_method:platform_type:ЭлементыФормы:Форма:Добавить")' \
  target/uat/t60-get-owner-type-member-ok.json
```

Expected result:

- Duplicate platform type names return `status: "ambiguous"` with deterministic candidate
  summaries, not a hidden first match.
- Owner-name/member and related owner-name/member lookups report owner ambiguity before filtering
  by the requested member.
- Constructor lookup by ambiguous type name returns a provider `ambiguous` envelope instead of a
  non-provider error or a hidden owner selection.
- Exact name lookup reports ownerless/owned same-name collisions as ambiguity instead of keeping
  only the ownerless candidate.
- The analyzer-preferred `--owner-type-id --member` path remains unambiguous when the caller has
  already resolved the owner type id.
- Raw command outputs remain service data under `target/uat`; only the commands, assertions and
  conclusions are durable.

## UAT-SH-020: Review-Oriented Search Ranking

Related use case: UC-SH-005C.

Related requirements: FR-SH-SEARCH-001, FR-SH-PROVIDER-001.

Status: implementation UAT for T62.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- A fresh schema-v4 or later provider index can be built under `target/uat/`.

Steps:

```bash
rm -f target/uat/t62-sh-search-ru.sqlite target/uat/t62-search-*.json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/uat/t62-sh-search-ru.sqlite

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t62-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax search --query "Структура" --mode keywords --format json \
  > target/uat/t62-search-structure.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t62-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax search --query "отбор скд" --mode keywords --format json \
  > target/uat/t62-search-skd-filter.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t62-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax search --query "таблица регистра бухгалтерии" --mode keywords --format json \
  > target/uat/t62-search-accounting-register-table.json

jq -e '.status == "ok"
  and .results[0].fact.id == "platform_type:Структура"
  and .results[0].fact.kind == "platform_type"
  and (.results[0].meta | has("rank"))
  and (.results[0].fact | has("score") | not)' \
  target/uat/t62-search-structure.json
jq -e '.status == "ok"
  and (.results[0].fact.id | test("КомпоновкиДанных"))
  and any(.results[].fact; .id == "platform_type:ОтборКомпоновкиДанных")' \
  target/uat/t62-search-skd-filter.json
jq -e '.status == "ok"
  and .results[0].fact.id == "query_table:РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии"' \
  target/uat/t62-search-accounting-register-table.json
```

Expected result:

- The simple symbol query `Структура` ranks the exact platform type identity first, ahead of
  broader owned properties, owners, descriptions or prefix matches.
- Ranking metadata stays under `results[].meta`; provider facts do not expose internal search
  scores or FTS tokens.
- The accepted task-oriented search `отбор скд` still ranks an SKD/data-composition fact first and
  keeps the platform type identity in the result set.
- The accepted task-oriented search `таблица регистра бухгалтерии` keeps its previously accepted
  top hit.
- Raw command outputs remain service data under `target/uat`; only the commands, assertions and
  conclusions are durable.

## UAT-SH-021: Bounded and Compact Query Output

Related use case: UC-SH-005C and UC-SH-005D.

Related requirements: FR-SH-SEARCH-001, FR-SH-SEARCH-002, FR-SH-PROVIDER-001, NFR-QUERY-001.

Status: implementation UAT for T63.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- A current schema-v4 or later provider index exists under `target/uat/` or can be rebuilt there.

Steps:

```bash
rm -f target/uat/t63-search-limit.json target/uat/t63-related-limit.json target/uat/t63-related-compact.json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/uat/t63-sh-search-ru.sqlite

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t63-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax search --query "Структура" --mode keywords --limit 3 --format json \
  > target/uat/t63-search-limit.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t63-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --id "type_property:platform_type:Символы:ПС" --limit 5 --format json \
  > target/uat/t63-related-limit.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t63-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --id "type_property:platform_type:Символы:ПС" --limit 5 --compact --format json \
  > target/uat/t63-related-compact.json

jq -e '.status == "ok"
  and .query.limit == 3
  and (.results | length) == 3
  and all(.results[]; (.meta | has("rank")) and (.fact | has("score") | not))' \
  target/uat/t63-search-limit.json
jq -e '.status == "ok"
  and .query.limit == 5
  and (.results | length) == 5
  and all(.results[]; (.meta | has("depth")) and (.meta | has("path")))' \
  target/uat/t63-related-limit.json
jq -e '.status == "ok"
  and .query.limit == 5
  and .query.output == "compact"
  and (.results | length) == 5
  and all(.results[];
    (.fact | has("id") and has("kind") and has("name"))
    and (.fact | has("description") | not)
    and (.fact | has("signatures") | not)
    and (.fact | has("types") | not)
    and (.fact | has("return") | not)
    and (.meta | has("depth") and has("path"))
  )' target/uat/t63-related-compact.json
```

Expected result:

- `--limit` bounds `syntax search` and `syntax related` provider result arrays deterministically.
- `syntax related --compact` keeps stable fact identity and relationship explanation under
  `results[].meta`, while omitting bulky fact fields not needed for review triage.
- Full `syntax related --format json` remains the default provider fact shape when `--compact` is
  omitted.
- Query output continues to use the provider envelope and does not expose SQLite table names, FTS
  tokens, HBK paths, TOC paths, HTML paths or page titles.

## UAT-SH-022: Public Relationship Edge Filters

Related use case: UC-SH-005B and UC-SH-005C.

Related requirements: FR-SH-SEARCH-002, FR-SH-PROVIDER-001.

Status: implementation UAT for T64.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- A current schema-v4 or later provider index exists under `target/uat/` or can be rebuilt there.

Steps:

```bash
rm -f target/uat/t64-related-member-of.json target/uat/t64-related-member-of.txt \
  target/uat/t64-related-unsupported-edge.json target/uat/t64-related-help.txt
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/uat/t64-sh-search-ru.sqlite

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t64-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" \
  --edge member_of --format json \
  > target/uat/t64-related-member-of.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t64-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" \
  --edge member_of \
  > target/uat/t64-related-member-of.txt
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t64-sh-search-ru.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" \
  --edge unknown_edge --format json \
  > target/uat/t64-related-unsupported-edge.json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax related --help \
  > target/uat/t64-related-help.txt

jq -e '.status == "ok"
  and .query.kind == "related"
  and .query.edge == "member_of"
  and (.results | length) == 1
  and .results[0].fact.id == "platform_type:НастройкиКомпоновкиДанных"
  and .results[0].meta.path[0].edge_kind == "member_of"' \
  target/uat/t64-related-member-of.json
grep -q "НастройкиКомпоновкиДанных" target/uat/t64-related-member-of.txt
jq -e '.status == "unsupported"
  and (.results | length) == 0
  and any(.diagnostics[]; .code == "UNSUPPORTED_QUERY"
    and (.message | contains("member_of")))' \
  target/uat/t64-related-unsupported-edge.json
grep -q "member_of" target/uat/t64-related-help.txt
```

Expected result:

- `member_of` is accepted as a public `syntax related --edge` filter for exact `--id` roots.
- JSON output uses the provider envelope, records `query.edge == "member_of"` and returns the owning
  platform fact through relationship metadata rather than storage rows.
- Text output uses the normal related presentation and includes the owner fact.
- Unsupported edge diagnostics and CLI help list the same supported edge set:
  `has_type`, `returns`, `constructs` and `member_of`.
- The command remains bounded to exact `--id` roots for edge-filtered traversal and does not become
  a general graph-query language.

## UAT-SH-023: Type-Reference Gap Measurement Is Deterministic

Related use case: UC-SH-005D.

Related requirements: FR-SH-002, FR-SH-003, FR-SH-PROVIDER-001, NFR-QUERY-001.

Status: implementation UAT for T135 and the externally observable report source for the T136
quality gates.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- `target/uat/t135-type-ref.sqlite` can be created or removed.

Steps:

```bash
rm -f target/uat/t135-type-ref.sqlite target/uat/t135-type-ref-1.json target/uat/t135-type-ref-2.json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/uat/t135-type-ref.sqlite

cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax type-ref-gaps --index target/uat/t135-type-ref.sqlite --format json \
  > target/uat/t135-type-ref-1.json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax type-ref-gaps --index target/uat/t135-type-ref.sqlite --format json \
  > target/uat/t135-type-ref-2.json

cmp target/uat/t135-type-ref-1.json target/uat/t135-type-ref-2.json
jq -e '
  .schema_version == 1
  and .command == "type-ref-gaps"
  and .total == (.resolved + .unresolved + .ambiguous)
  and (.roles | length > 0)
  and all(.roles[]; .total == (.resolved + .unresolved + .ambiguous))
  and (.template_bindings >= 0)
  and (.top_unresolved | type == "array")
  and (.top_ambiguous | type == "array")
' target/uat/t135-type-ref-1.json
```

Expected result:

- Both measurement commands exit with code `0`.
- Repeated JSON output for the same index is byte-identical.
- The report contains overall totals, role totals, template-binding subset count and top
  unresolved/ambiguous names with source fact context.
- The command reads the prebuilt index path and does not accept an HBK source path.
- T136 baseline gates consume this report as an acceptance measurement. The report remains a
  measurement command output, not a provider JSON expansion for `syntax get`, `syntax constructors`,
  `syntax search` or `syntax related`.
- Source-backed exact type-reference spelling may reduce ambiguous row counts when it selects one
  candidate without changing public plain-name lookup semantics. Remaining ambiguous rows must keep
  deterministic candidate ids and must not be hidden by first-match selection.

Cleanup:

- `target/uat/t135-type-ref.sqlite` and `target/uat/t135-type-ref-*.json` are service data and may
  be deleted after the run.

## UAT-SH-024: Type Graph Query for Expression-Chain Workflow

Related use case: UC-SH-005B, UC-SH-005C and UC-SH-005D.

Related requirements: FR-SH-SEARCH-001, FR-SH-SEARCH-002, FR-SH-PROVIDER-001, NFR-QUERY-001.

Status: implementation UAT for T142 and broadened consumer-workflow coverage for T145.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- `target/uat/t142-type-graph.sqlite` can be created or removed.
- `target/uat/t145-type-graph.sqlite` can be created or removed when running the broadened T145
  workflow coverage.
- `jq` is available for JSON assertions.

Steps:

```bash
rm -f target/uat/t142-type-graph.sqlite target/uat/t142-type-graph.json \
  target/uat/t142-type-graph.time target/uat/t142-type-graph-unsupported*.json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/uat/t142-type-graph.sqlite

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t142-type-graph.sqlite \
  /usr/bin/time -f '%e' -o target/uat/t142-type-graph.time \
  target/debug/v8-context-hbk syntax related \
    --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" \
    --graph --limit 200 --format json \
  > target/uat/t142-type-graph.json

awk 'BEGIN { ok = 0 } { if ($1 < 2.0) ok = 1 } END { exit ok ? 0 : 1 }' \
  target/uat/t142-type-graph.time

jq -e '
  .schema_version == 1
  and .command == "related"
  and .status == "ok"
  and .query.kind == "type_graph"
  and .query.root.id == "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"
  and .query.limit == 200
  and .results[0].fact.id == "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"
  and .results[0].meta.root == true
' target/uat/t142-type-graph.json
jq -e '
  any(.results[];
    .fact.id == "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"
    and .fact.owner == "НастройкиКомпоновкиДанных"
    and (.fact.types | index("ОтборКомпоновкиДанных") != null)
    and any(.meta.type_references[]?;
      .role == "type"
      and .name == "ОтборКомпоновкиДанных"
      and .status == "ok"
      and .target_type_id == "platform_type:ОтборКомпоновкиДанных"))
  and any(.results[]; .fact.id == "platform_type:ОтборКомпоновкиДанных")
  and any(.results[];
    .fact.id == "type_property:platform_type:ОтборКомпоновкиДанных:Элементы"
    and (.fact.types | index("КоллекцияЭлементовОтбораКомпоновкиДанных") != null))
  and any(.results[];
    .fact.id == "type_method:platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных:Добавить"
    and (.fact.return | index("ЭлементОтбораКомпоновкиДанных") != null))
  and any(.results[];
    .fact.kind == "type_property"
    and .fact.owner == "ЭлементОтбораКомпоновкиДанных"
    and .fact.name.primary == "ЛевоеЗначение")
  and any(.results[];
    .fact.kind == "type_property"
    and .fact.owner == "ЭлементОтбораКомпоновкиДанных"
    and .fact.name.primary == "ВидСравнения")
  and any(.results[];
    .fact.kind == "type_property"
    and .fact.owner == "ЭлементОтбораКомпоновкиДанных"
    and .fact.name.primary == "ПравоеЗначение")
  and any(.results[];
    .fact.kind == "type_property"
    and .fact.owner == "ЭлементОтбораКомпоновкиДанных"
    and .fact.name.primary == "Использование")
' target/uat/t142-type-graph.json
jq -e '
  all(.results[];
    (.meta | has("depth"))
    and (.meta | has("path"))
    and (.fact | has("type_references") | not)
    and (.fact | has("type_refs") | not)
    and (.fact | has("return_types") | not)
    and (.fact | has("source") | not)
    and (.fact | has("source_hbk") | not)
    and (.fact | has("toc_path") | not)
    and (.fact | has("html_path") | not)
    and (.fact | has("page_title") | not)
    and (.fact | has("rowid") | not)
    and (.fact | has("parameter_text") | not)
    and (.fact | has("parameter_terms") | not)
    and (.fact | has("relation_keys") | not))
  and all(.diagnostics[]?;
    (.code == "UNRESOLVED_TYPE_REFERENCE" or .code == "AMBIGUOUS_TYPE_REFERENCE")
    and (.source_id | type == "string")
    and (.role | type == "string")
    and (.name | type == "string"))
' target/uat/t142-type-graph.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t142-type-graph.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related \
    --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" \
    --graph --compact --format json \
  > target/uat/t142-type-graph-unsupported.json
jq -e '.status == "unsupported" and any(.diagnostics[]; .code == "UNSUPPORTED_QUERY")' \
  target/uat/t142-type-graph-unsupported.json
V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t142-type-graph.sqlite \
  cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax related --id "query_table:БизнесПроцесс" --graph --limit 1 --format json \
  > target/uat/t142-type-graph-unsupported-root.json
jq -e '.status == "unsupported" and any(.diagnostics[]; .code == "UNSUPPORTED_QUERY")' \
  target/uat/t142-type-graph-unsupported-root.json
```

Additional T145 graph workflow coverage:

```bash
rm -f target/uat/t145-type-graph.sqlite target/uat/t145-type-graph*.json \
  target/uat/t145-type-graph*.time
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- \
  syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/uat/t145-type-graph.sqlite

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t145-type-graph.sqlite \
  /usr/bin/time -f '%e' -o target/uat/t145-type-graph-query-execute.time \
  target/debug/v8-context-hbk syntax related \
    --id "type_method:platform_type:Запрос:Выполнить" \
    --graph --limit 200 --format json \
  > target/uat/t145-type-graph-query-execute.json
awk 'BEGIN { ok = 0 } { if ($1 < 2.0) ok = 1 } END { exit ok ? 0 : 1 }' \
  target/uat/t145-type-graph-query-execute.time
jq -e '
  .status == "ok"
  and .query.kind == "type_graph"
  and .query.root.id == "type_method:platform_type:Запрос:Выполнить"
  and .results[0].fact.id == "type_method:platform_type:Запрос:Выполнить"
  and any(.results[]; .fact.id == "platform_type:РезультатЗапроса")
  and any(.results[]; .fact.id == "type_method:platform_type:РезультатЗапроса:Выбрать")
  and any(.results[]; .fact.id == "platform_type:ВыборкаИзРезультатаЗапроса")
  and any(.results[]; .fact.id == "type_method:platform_type:ВыборкаИзРезультатаЗапроса:Следующий")
  and any(.results[]; .fact.id == "type_property:platform_type:ВыборкаИзРезультатаЗапроса:<Имя поля>")
' target/uat/t145-type-graph-query-execute.json

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t145-type-graph.sqlite \
  /usr/bin/time -f '%e' -o target/uat/t145-type-graph-http-get.time \
  target/debug/v8-context-hbk syntax related \
    --id "type_method:platform_type:HTTPСоединение:Получить" \
    --graph --limit 200 --format json \
  > target/uat/t145-type-graph-http-get.json
awk 'BEGIN { ok = 0 } { if ($1 < 2.0) ok = 1 } END { exit ok ? 0 : 1 }' \
  target/uat/t145-type-graph-http-get.time
jq -e '
  .status == "ok"
  and .query.kind == "type_graph"
  and .query.root.id == "type_method:platform_type:HTTPСоединение:Получить"
  and .results[0].fact.id == "type_method:platform_type:HTTPСоединение:Получить"
  and any(.results[]; .fact.id == "platform_type:HTTPОтвет")
  and any(.results[]; .fact.id == "type_property:platform_type:HTTPОтвет:КодСостояния")
  and any(.results[]; .fact.id == "type_property:platform_type:HTTPОтвет:Заголовки")
  and any(.results[]; .fact.id == "type_method:platform_type:HTTPОтвет:ПолучитьТелоКакСтроку")
  and any(.results[]; .fact.id == "type_method:platform_type:HTTPОтвет:ПолучитьТелоКакДвоичныеДанные")
' target/uat/t145-type-graph-http-get.json

V8_CONTEXT_HBK_SYNTAX_INDEX=target/uat/t145-type-graph.sqlite \
  /usr/bin/time -f '%e' -o target/uat/t145-type-graph-binary-stream.time \
  target/debug/v8-context-hbk syntax related \
    --id "type_method:platform_type:ДвоичныеДанные:ОткрытьПотокДляЧтения" \
    --graph --limit 200 --format json \
  > target/uat/t145-type-graph-binary-stream.json
awk 'BEGIN { ok = 0 } { if ($1 < 2.0) ok = 1 } END { exit ok ? 0 : 1 }' \
  target/uat/t145-type-graph-binary-stream.time
jq -e '
  .status == "ok"
  and .query.kind == "type_graph"
  and .query.root.id == "type_method:platform_type:ДвоичныеДанные:ОткрытьПотокДляЧтения"
  and .results[0].fact.id == "type_method:platform_type:ДвоичныеДанные:ОткрытьПотокДляЧтения"
  and any(.results[]; .fact.id == "platform_type:Поток")
  and any(.results[]; .fact.id == "type_method:platform_type:Поток:Прочитать")
  and any(.results[]; .fact.id == "type_method:platform_type:Поток:Закрыть")
  and any(.results[]; .fact.id == "type_property:platform_type:Поток:ДоступноЧтение")
  and any(.results[]; .fact.id == "type_method:platform_type:Поток:ПолучитьПотокТолькоДляЧтения")
' target/uat/t145-type-graph-binary-stream.json

jq -e '
  all(.results[];
    (.meta | has("depth"))
    and (.meta | has("path"))
    and (.fact | has("type_references") | not)
    and (.fact | has("type_refs") | not)
    and (.fact | has("return_types") | not)
    and (.fact | has("source") | not)
    and (.fact | has("source_hbk") | not)
    and (.fact | has("toc_path") | not)
    and (.fact | has("html_path") | not)
    and (.fact | has("page_title") | not)
    and (.fact | has("rowid") | not)
    and (.fact | has("parameter_text") | not)
    and (.fact | has("parameter_terms") | not)
    and (.fact | has("relation_keys") | not))
' target/uat/t145-type-graph-query-execute.json
jq -e '
  all(.results[];
    (.meta | has("depth"))
    and (.meta | has("path"))
    and (.fact | has("type_references") | not)
    and (.fact | has("type_refs") | not)
    and (.fact | has("return_types") | not)
    and (.fact | has("source") | not)
    and (.fact | has("source_hbk") | not)
    and (.fact | has("toc_path") | not)
    and (.fact | has("html_path") | not)
    and (.fact | has("page_title") | not)
    and (.fact | has("rowid") | not)
    and (.fact | has("parameter_text") | not)
    and (.fact | has("parameter_terms") | not)
    and (.fact | has("relation_keys") | not))
' target/uat/t145-type-graph-http-get.json
jq -e '
  all(.results[];
    (.meta | has("depth"))
    and (.meta | has("path"))
    and (.fact | has("type_references") | not)
    and (.fact | has("type_refs") | not)
    and (.fact | has("return_types") | not)
    and (.fact | has("source") | not)
    and (.fact | has("source_hbk") | not)
    and (.fact | has("toc_path") | not)
    and (.fact | has("html_path") | not)
    and (.fact | has("page_title") | not)
    and (.fact | has("rowid") | not)
    and (.fact | has("parameter_text") | not)
    and (.fact | has("parameter_terms") | not)
    and (.fact | has("relation_keys") | not))
' target/uat/t145-type-graph-binary-stream.json
```

Expected result:

- The graph command exits with code `0`, reads the prebuilt index and does not accept an HBK source
  path.
- The JSON response uses the provider envelope with `command="related"` and
  `query.kind="type_graph"`.
- The first result is the exact root fact; `--limit` bounds the total `results[]` array including
  that root.
- The single response contains the accepted SKD expression-chain facts: the settings `Отбор`
  property, `ОтборКомпоновкиДанных`, `Элементы`, collection `Добавить` and filter-item fields.
- The broadened T145 responses cover three additional expression-chain workflows in one bounded
  `syntax related --graph` call each:
  - `Запрос.Выполнить` reaches `РезультатЗапроса`, `РезультатЗапроса.Выбрать`,
    `ВыборкаИзРезультатаЗапроса`, `Следующий` and `<Имя поля>`.
  - `HTTPСоединение.Получить` reaches `HTTPОтвет`, response status/header facts and body-access
    methods.
  - `ДвоичныеДанные.ОткрытьПотокДляЧтения` reaches `Поток`, read/close methods and readable-stream
    capability facts.
- Shared platform fact fields stay export-compatible under `results[].fact`.
- Graph and resolution details, including type-reference target status and relationship paths, stay
  under `results[].meta`.
- Recoverable unresolved or ambiguous type-reference diagnostics may be present while
  `status="ok"` when the root exists; they are graph-quality diagnostics, not lookup failures.
- Public JSON does not expose SQLite table names, rowids, FTS/search token fields, HBK paths, TOC
  paths, HTML paths or page titles.
- `syntax related --graph --compact` is unsupported; non-graph `syntax related --compact` remains
  covered by UAT-SH-021.
- Query-table, language, enum and global-property facts are not accepted as type-graph roots.
- The graph query meets NFR-QUERY-001 on the accepted corpus, or the task records a measured
  blocker and remains incomplete.

Cleanup:

- `target/uat/t142-type-graph.sqlite`, `target/uat/t142-type-graph*.json`,
  `target/uat/t142-type-graph.time`, `target/uat/t145-type-graph.sqlite`,
  `target/uat/t145-type-graph*.json` and `target/uat/t145-type-graph*.time` are service data and
  may be deleted after the run.

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
  counts.UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE == 4
  and (counts.MISSING_QUERY_TABLE_SYNTAX // 0) >= 1
  and all(counts | keys[]; . == "UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE" or . == "MISSING_QUERY_TABLE_SYNTAX")
' target/uat/shcntx-ru/diagnostics.json
jq -e '
  def counts: reduce .records[].code as $code ({}; .[$code] = (.[$code] // 0) + 1);
  counts.UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE == 4
  and (counts.MISSING_QUERY_TABLE_SYNTAX // 0) >= 1
  and all(counts | keys[]; . == "UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE" or . == "MISSING_QUERY_TABLE_SYNTAX")
' target/uat/shcntx-en/diagnostics.json
```

Expected result:

- The export keeps deterministic unsupported global-context method diagnostics for both locales.
- Missing or empty query-table syntax is reported through `MISSING_QUERY_TABLE_SYNTAX`
  diagnostics, not by synthesizing syntax or identifiers from table display names.
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
jq -e '.records[] | select(.name == "Основная таблица" and (.owner_path | index("Таблицы задач"))) | .table_role == "unknown" and (has("syntax") | not) and (has("identifier") | not) and any(.fields[]; .name == "Наименование")' target/uat/shcntx-ru/query-tables.json
jq -e '.records[] | select(.name == "Main Table" and (.owner_path | index("Task Tables"))) | .table_role == "unknown" and (has("syntax") | not) and (has("identifier") | not) and any(.fields[]; .name == "Description")' target/uat/shcntx-en/query-tables.json

jq -e '.records[] | select(.name == "Таблица критерия отбора") | any(.parameters[]; .name == "Значение" and (has("required") | not) and (.description | test("отбор")))' target/uat/shcntx-ru/query-tables.json
jq -e '.records[] | select(.name == "Filter Criterion Table") | any(.parameters[]; .name == "Value" and (has("required") | not) and (.description | test("filtering")))' target/uat/shcntx-en/query-tables.json
jq -e 'any(.records[]; .code == "MISSING_QUERY_TABLE_SYNTAX" and (.source.page_title == "Основная таблица"))' target/uat/shcntx-ru/diagnostics.json
jq -e 'any(.records[]; .code == "MISSING_QUERY_TABLE_SYNTAX" and (.source.page_title == "Main Table"))' target/uat/shcntx-en/diagnostics.json
```

Expected result:

- Module events, type events and query tables are exported as typed consumer facts in both locales.
  `module-events.json` is the required FR-EXPORT-001 adapter filename for `module_event` records,
  and `type-events.json` is the required adapter filename for `type_event` records.
- Event signatures and parameters are parsed structurally.
- Query table records preserve table name, semantic owner path, table role, field names, field type
  references and field descriptions. Records with source syntax also preserve localized syntax and a
  deterministic identifier.
- Query table records whose source page has no syntax section remain exported with their nested
  field/parameter facts, but do not synthesize `syntax` or `identifier` from the table display name;
  their `table_role` is `unknown` and `diagnostics.json` contains a source-provenance
  `MISSING_QUERY_TABLE_SYNTAX` diagnostic.
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
jq -e 'all(.records[]; if has("syntax") then (.syntax.primary | type == "string") and ((.syntax.alias? // "" | type) == "string") and (.identifier | type == "string") and (.identifier | test("[\\s-]") | not) else (has("identifier") | not) and .table_role == "unknown" end)' target/uat/shcntx-ru/query-tables.json
jq -e 'all(.records[]; if has("syntax") then (.syntax.primary | type == "string") and ((.syntax.alias? // "" | type) == "string") and (.identifier | type == "string") and (.identifier | test("[\\s-]") | not) else (has("identifier") | not) and .table_role == "unknown" end)' target/uat/shcntx-en/query-tables.json
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
- Query table records with a source syntax section include localized `syntax` objects and string
  `identifier` values without whitespace or hyphens; additional table identifier suffixes are
  CamelCase-normalized from page `name`. Query table records without source syntax omit both
  `syntax` and `identifier`, keep nested field/parameter facts, and use `table_role="unknown"`.
- `usage` is a stable enum string.
- Property descriptions do not keep leading type-reference prose that already appears in `types`.
- Type-reference facts are exposed as `types`; shared method return facts are exposed as fact-level
  `return`; source-proven overload return facts may be exposed as `signatures[].return`; legacy
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
  .records[]
  | select(.owner == "HTTPСоединение"
      and any(.signatures[]?; any(.parameters[]?; .name == "ИспользоватьАутентификациюОС")))
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
- Constructor parameter parsing is not truncated by inline label-like text in parameter bodies.

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
