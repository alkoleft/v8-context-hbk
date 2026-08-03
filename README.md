# v8-context-hbk

Превратите книги Синтакс-помощника 1C в локальную структурированную базу знаний по API платформы.

`v8-context-hbk` читает установленные `*.hbk`-файлы 1C, извлекает факты из Синтакс-помощника,
строит индекс для быстрых запросов и отдает детерминированный JSON, который можно использовать в
инструментах для BSL-разработки, code review и будущих анализаторов.

Вместо того чтобы каждый раз разбирать справочный текст про `HTTPСоединение`, конструкторы,
свойства, таблицы запросов или API системы компоновки данных, можно один раз построить локальный
индекс и затем задавать точные вопросы к фактам платформы.

## Зачем Это Нужно

Знания о платформе 1C обычно заперты в справке, удобной для человека, но неудобной для
инструментов, coding agents и анализаторов. Им нужны не страницы с текстом, а структурированные
ответы:

- Какие перегрузки конструктора существуют?
- Какие параметры обязательны и какие типы они принимают?
- Какой реквизит или метод принадлежит какому типу платформы?
- К каким связанным типам ведет цепочка выражения?
- Имя однозначно или оно встречается в разных доменах: API платформы, языке BSL и языке запросов?

`v8-context-hbk` превращает документацию в локальные факты с идентичностями, сигнатурами, ссылками
на типы, связями владелец/член, обходом отношений и машинно-читаемыми диагностическими данными.

## Что Вы Получаете

- Локальное извлечение данных из установленных HBK-файлов без runtime-интроспекции 1C.
- Структурированный JSON-экспорт фактов Синтакс-помощника для последующей загрузки в другие
  инструменты.
- SQLite/FTS5-индекс для быстрых повторяемых запросов.
- Точный поиск по имени, поиск владелец/член, поиск конструкторов, keyword search, fuzzy search и
  обход отношений.
- Детерминированный provider JSON для инструментов, которым нужны стабильные ответы, а не
  человеко-ориентированный текст справки.
- Resolver-блоки, которые учитывают источник факта и не смешивают API платформы, язык BSL и язык
  запросов.
- Диагностику с provenance для сопровождения парсеров без протекания внутренних HBK-путей в
  потребительские факты.

Текущий подтвержденный baseline платформы: `8.5.1.1150`. Другие версии могут работать, но CLI,
экспорт и provider/resolver-контракты пока остаются provisional, пока модель проверяется на
реальных HBK-данных.

## Когда Это Подходит

Используйте `v8-context-hbk`, если вы строите:

- BSL coding assistant, которому нужны обоснованные факты API платформы;
- локальный provider для анализатора, который должен отвечать без повторного разбора больших
  HBK-файлов на каждый запрос;
- pipeline загрузки platform context data;
- регрессионные тесты вокруг извлечения документации платформы 1C;
- developer tooling, которому нужен детерминированный JSON вместо скопированного текста справки.

Проект намеренно остается самостоятельным. Ему не нужен запущенный процесс 1C, он не изменяет
HBK-файлы и не реализует полноценный BSL-парсер или диагностический движок.

## Быстрый Старт

Соберите CLI из репозитория:

```bash
cargo build -p v8-context-hbk-cli
```

Дальше примеры предполагают, что бинарь `v8-context-hbk` уже собран и доступен в `PATH`
или запускается из каталога сборки:

```bash
v8-context-hbk <command>
```

Примеры ниже предполагают, что файлы справки платформы установлены в
`/opt/1cv8/x86_64/8.5.1.1150/`.

## Инспекция Help Books

Показать сущности HBK-контейнера:

```bash
v8-context-hbk inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk
```

Вывести оглавление в JSON:

```bash
v8-context-hbk toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json
```

Прочитать страницу по HTML-пути из storage книги:

```bash
v8-context-hbk page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --path "<html-path>"
```

## Экспорт Обычных Help Books

Распаковать обычную книгу справки в исходной структуре `FileStorage`:

```bash
v8-context-hbk export /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk \
  --output target/book-raw/fmtdui_ru \
  --format raw \
  --hierarchy raw
```

Команда верхнего уровня `export` не извлекает факты Синтакс-помощника и не пишет JSON-схему
platform context.
Сейчас поддержаны две пары:

- `--format raw --hierarchy raw`: распаковка исходных `FileStorage` entry paths;
- `--format markdown --hierarchy toc`: Markdown-страницы по TOC-derived directory tree.

Экспорт Markdown:

```bash
v8-context-hbk export /opt/1cv8/x86_64/8.5.1.1150/dcsui_ru.hbk \
  --output target/book-md/dcsui_ru \
  --format markdown \
  --hierarchy toc
```

Markdown/TOC пишет каждую TOC-страницу как `index.md` в каталоге, построенном из заголовков TOC.
Внутренние ссылки на экспортируемые TOC-страницы переписываются в относительные Markdown-ссылки;
сырые HBK paths, HTML storage paths и service HTML scaffolding в Markdown не выводятся.
Сочетания `--format raw --hierarchy toc` и `--format markdown --hierarchy raw` остаются
неподдержанными и возвращают читаемую ошибку.

## Экспорт Фактов Платформы

Экспортировать русскоязычные данные Синтакс-помощника:

```bash
v8-context-hbk syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/ru
```

Экспортировать root/English-source данные Синтакс-помощника. Локаль экспорта будет записана как
`en`:

```bash
v8-context-hbk syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/context/en
```

Каталог результата содержит JSON-файлы по семействам записей:

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

Текущая provisional-схема экспорта: `schema_version: 11`.

Потребительские файлы включают структурированную availability-информацию, примеры, see-also связи,
варианты сигнатур, ссылки на типы и возвращаемые типы, когда эти факты есть на исходной странице.
TOC-derived semantic fields, такие как `record_family`, `module`, `owner`, `type_kind` и
`object_kind` для типов платформы, выводятся там, где одного заголовка недостаточно для
однозначной идентификации.

Потребительские записи не содержат HBK file paths, TOC paths, HTML paths и page titles.
`diagnostics.json` сохраняет parser provenance для сопровождения. Сводка `syntax export` показывает
количество диагностик как `parser_warnings`, потому что это предупреждения сопровождения парсера, а
не экспортируемые факты API платформы.

## Локальный Syntax Index

Построить SQLite/FTS5-индекс из HBK Синтакс-помощника:

```bash
v8-context-hbk syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/sh-search-ru.sqlite
```

Если `--output` или `--index` не указаны, команды используют `V8_CONTEXT_HBK_SYNTAX_INDEX`, а затем
`.v8-context-hbk/syntax/index.sqlite` в текущем рабочем каталоге.

Query-команды читают только заранее построенный индекс. Они не открывают и не парсят
`shcntx_*.hbk` при каждом запросе.

## Практические Вопросы к API

Точный поиск типа:

```bash
v8-context-hbk syntax get --index target/context/sh-search-ru.sqlite --name "ОтборКомпоновкиДанных" --format json
```

Поиск владелец/член:

```bash
v8-context-hbk syntax get --index target/context/sh-search-ru.sqlite --owner "НастройкиКомпоновкиДанных" --member "Отбор"
```

Сигнатуры конструкторов:

```bash
v8-context-hbk syntax constructors --index target/context/sh-search-ru.sqlite "HTTPСоединение"
v8-context-hbk syntax constructors --index target/context/sh-search-ru.sqlite "HTTPСоединение" --details
```

Поиск под задачу и восстановление при опечатке:

```bash
v8-context-hbk syntax search --index target/context/sh-search-ru.sqlite --query "отбор скд" --mode keywords
v8-context-hbk syntax search --index target/context/sh-search-ru.sqlite --query "ОтборКомпоновкиДаных" --mode fuzzy --format json
```

Обход отношений:

```bash
v8-context-hbk syntax related --index target/context/sh-search-ru.sqlite --name "ОтборКомпоновкиДанных"
```

## Сделано для Tooling

`v8-context-hbk` - не просто просмотрщик документации. Поверхность `syntax` спроектирована как
локальный provider для BSL-инструментов:

- результаты запросов используют явные статусы `ok`, `not_found`, `ambiguous` и `unsupported`;
- JSON-вывод разделяет metadata запроса и факты платформы;
- callable-сигнатуры сохраняют порядок параметров, обязательность и ссылки на типы;
- обход отношений использует source-backed edges, а не скрытые догадки по имени;
- provider output детерминирован для одного и того же индекса и запроса.

Для Rust-приложений, которым нужны повторяемые in-process lookup-операции, workspace также содержит
resolver-крейты:

- `context-resolver-core`: source-neutral typed resolver model без зависимостей от HBK, SQLite, CLI
  или парсеров;
- `context-resolver-search`: адаптеры поверх локального search index для фактов платформы и
  language-domain facts.

Resolver-направление сохраняет одноименные факты раздельными между `PlatformApi`, `BslLanguage`,
`QueryLanguage`, configuration и source-code domains, пока явные отношения не связывают их между
собой.

## Текущие Ограничения

- Export, provider и resolver-контракты provisional.
- Подтвержденный baseline: платформа 1C `8.5.1.1150`.
- Инструмент читает существующие HBK-файлы; он не создает и не изменяет HBK-файлы.
- Извлечение данных Синтакс-помощника evidence-based и может потребовать обновления парсера для
  других версий платформы.
- Runtime-интроспекция 1C вне scope. Инструмент извлекает документацию только из HBK-файлов.
- BSL parsing, linting и diagnostics вне scope этого репозитория.

## Документация

- Канонические OpenSpec-требования: [openspec/specs](openspec/specs)
- Индекс legacy-документации и evidence: [spec/README.md](spec/README.md)
- Функциональные требования: [spec/requirements/functional.md](spec/requirements/functional.md)
- Нефункциональные требования: [spec/requirements/non-functional.md](spec/requirements/non-functional.md)
- Acceptance baseline: [spec/acceptance/baseline.md](spec/acceptance/baseline.md)
- UAT test cases: [spec/acceptance/uat-test-cases.md](spec/acceptance/uat-test-cases.md)
- Решение об интеграции: [spec/decisions/ADR-0001-v8-context-integration.md](spec/decisions/ADR-0001-v8-context-integration.md)
- Provider boundary: [spec/decisions/ADR-0007-bsl-analyzer-provider-boundary.md](spec/decisions/ADR-0007-bsl-analyzer-provider-boundary.md)
- Resolver boundary: [spec/decisions/ADR-0008-rust-context-resolver-api.md](spec/decisions/ADR-0008-rust-context-resolver-api.md)
