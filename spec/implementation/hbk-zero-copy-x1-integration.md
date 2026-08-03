# Интеграция X1 и условный переход на canonical runtime

Этот документ исполняет ADR-0012. Он отделяет решение пользователя от трёх
последующих milestones: non-canonical X1-INT, canonical cutover и scoped
cleanup. Следующий milestone не начинается, пока предыдущий не завершён и не
прошёл отдельные plan/review gates.

## Зафиксированное решение

- Единственный интеграционный кандидат: `X1`.
- Отклонённый вариант: `X1-PROJECTED`.
- H0 SQLite-to-owned остаётся X1-INT baseline до прохождения gates.
- X1 становится canonical только после полного X1-INT pass.
- При pass SQLite остаётся только private build input и отдельным CLI/search/
  debug storage, но не источником snapshot runtime анализатора.

## Владение операциями

HBK владеет исходными platform facts, их payload, availability words,
provider-native `ANY`/`ALL` predicate и ordered borrowed traversal:

- filtered global methods/properties;
- type-by-name lookup;
- filtered непосредственные members одного уже найденного platform type;
- доступ к type/member/callable/signature/parameter payload.

Пустая availability означает universal. `ANY` принимает universal либо хотя бы
один запрошенный контекст; `ALL` принимает universal либо все запрошенные
контексты. `ModuleContextKind` не является availability-фильтром.

`v8-context` владеет lexical/source/provider tiers, precedence, ambiguity,
shadowing и effective selection. Он может удерживать только canonical keys,
tier/ordinal и существующие source-owned locators в пределах запроса. Имена,
signatures, parameters, members и provider records не копируются в retained
selected view или cache.

## Production-source ledger X1

Код переносится как поведение, а не cherry-pick полного commit или merge
экспериментальной ветви.

Допустимые источники для сверки физического поведения:

| Источник | Допустимое поведение |
|---|---|
| `flat_r1.rs`, `ffcb990` | fixed record heads, checked ranges, mapped nested arenas |
| `0c4f8d1`, `59e6d5e`, `c97d06e`, `3379391`, `eb46868` | availability mask/AoS и compact locator mechanics |
| `8d04234`, `0d1bd65`, `50b4faf` | provenance, owner-contiguous member range, сохранение порядка context evidence |
| `8f2cfbe` | X1 global SoA, type-name hash, member AoS composition |
| `d85cab4` | collision-safe построение и проверка type-name hash |
| `snapshot/types.rs` | существующие typed dense IDs; experiment-названия в production не переносятся |

Запрещено переносить:

- `examples/measure_*`, `examples/dump_*`, `examples/produce_*` и
  `examples/s83_*_common`;
- `scripts/benchmark-*`, `scripts/summarize-*`, `scripts/verify-*` и их tests;
- `REQUIRED_SOURCE_*`, `REQUIRED_PROVIDER_*`, hard-coded path/hash/platform/
  locale и любые другие corpus-specific константы;
- `experiment_*` методы, имена типов, feature gates и result schemas;
- экспериментальный широкий `semantic_read.rs` trait family как готовый
  production interface. Он является только доказательством осуществимости.

Перед commit diff review обязан доказать отсутствие этих форм в production
source.

## API inventory и миграция

| Категория | Сохраняется | Удаляется только после canonical cutover | Потребитель/замена |
|---|---|---|---|
| Build | `SearchIndexBuilder`, index build, explicit X1 build/ensure из принятого provider input | скрытый build внутри runtime open | setup/index-refresh |
| Search/debug | `SearchIndex`, `SearchIndex::open_read_only`, `PlatformSearchSource`, `LanguageSearchSource` | ничего, пока отдельный product contract существует | CLI, index inspection, debug, explicit sequential resolver |
| Snapshot owner | имя `HbkFactSnapshot`, typed IDs/domain records и `HbkFactReadHandle` как единственный owner/interface | owned `Vec<String>`/fact arenas и их binary-cache deserializer | validated read-only mapped X1 |
| Snapshot construction | explicit builder с временным build state | `HbkFactSnapshot::from_path`, `from_index`, `from_path_with_binary_cache` как analyzer runtime open | build artifact, drop builder state, `HbkFactSnapshot::open` |
| Catalogs | `HbkBslContextCatalog`, `HbkSdblQueryCatalog`, общий mapping owner | constructors, которые неявно открывают SQLite для snapshot runtime | mapped snapshot owner |
| Resolver adapters | `PlatformSnapshotSource`, `QueryTableSnapshotSource` | snapshot constructors из index path/`SearchIndex` | mapped snapshot owner |
| Analyzer | существующий `BslContext` и effective selection | `platform_catalog` materialization из SQLite | open готового X1 artifact |

API удаляется только после поиска всех production/test consumers и миграции.
Tests, проверяющие отдельный build/search contract, не считаются runtime-
мусором и не удаляются.

## Единственный runtime module и interface

`syntax-helper-search::HbkFactSnapshot` становится владельцем read-only mmap и
shared slot lock. `HbkFactReadHandle` остаётся малым borrowed interface к
проверенным typed ranges/views. Существующие catalog и resolver adapters
используют этот interface и не знают section IDs, offsets или layout.

Допустимо изменить конкретный record return type на borrowed view, если это
необходимо для zero-copy, но недопустимо оставить рядом owned и mapped семьи,
создать второй публичный catalog или добавить pass-through adapter только для
старой формы. Entity-shaped view потребляется синхронно в callback/операции и
не покидает её.

## Header, validation и lifecycle

Header X1 содержит фактические, а не скомпилированные под один corpus:

- magic и X1 binary layout version;
- byte-order/layout flags и directory integrity metadata;
- extraction schema и provider schema;
- exact platform version;
- locale/source locale;
- source HBK identity: размер и SHA-256;
- build-input identity, если SQLite используется producer;
- section count/directory, artifact length и принятые checksum/integrity поля.

Reader до typed access проверяет header, версии, source/platform/locale,
artifact length, section order/non-overlap, bounds, alignment, stride, range
overflow, UTF-8, enum/tag/reserved bits, sort/hash/owner-range invariants и
integrity metadata.

Логический slot имеет стабильный lock target. Reader удерживает shared lock всю
сессию. Writer получает exclusive lock fail-fast; при активном reader возвращает
typed `snapshot-in-use`, ничего не ждёт и не меняет. Новый артефакт пишется во
временный файл, проверяется, `fsync`-ится, публикуется новым immutable generation
file, затем current pointer меняется атомарно под тем же lock. Уже отображённый
файл никогда не усекается и не перезаписывается.

Open работает fail-closed. Отсутствующий/невалидный artifact возвращает typed
error. Отдельная setup-операция ensure/rebuild может построить новое поколение
из private SQLite build input и затем повторить open; catalog/analyzer не
выполняет fallback.

Замена HBK создаёт новое session-local ID space. Числовые IDs не сравниваются,
не мигрируют и не сохраняются между поколениями.

## Frozen X1-INT protocol

### Входы и среда

| Поле | Значение |
|---|---|
| Host/CPU | `alko-home`, Intel Core i7-4770 3.40 GHz, Linux |
| Rust/Cargo | `1.95.0` |
| Build/threads | `release`, `RAYON_NUM_THREADS=1`, test threads `1` |
| HBK | `/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk`, `40,744,845` bytes, SHA-256 `5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48` |
| Platform/locale | `8.3.27.1859`, `ru` |
| H0 index | `/home/alko/develop/open-source/v8-maintain-projects/v8-context/.v8-context/platform-indexes/8.3.27.1859/shcntx_ru.sqlite`, `204,288,000` bytes, SHA-256 `317f3cdd914e635c89b975bf9ebcf28238bdbabd54e455121a083558d4e05f5e` |
| Project | `/home/alko/develop/типовые/ssl_3_1/src/cf`; repository revision `b7e627f02fe10028e27bfec99dbc1afa7fd8324d`; generated `.v8-context` excluded |
| Module | `ОбщегоНазначенияКлиентСервер` |
| H0 code | `v8-context` `43f4f50f`; `v8-context-hbk` `d6f3960` |

### Сценарии и запуски

Используются неизменённые scenario lifecycle и oracle из
`crates/analyze-project/src/benchmark`:

| Сценарий | Samples | Warmups | Batch |
|---|---:|---:|---:|
| `prepared_module_context_handle` | 9 | 2 | 50 |
| `cold_module_context_handle` | 9 | 2 | 1 |
| `prepared_full_module_resolution` | 9 | 2 | 1 |

`cold` означает новое provider state в scenario; OS page cache не очищается.
Wall-time и `dhat` heap запускаются отдельными процессами. Два независимых
повтора A/B выполняются в порядке `H0-A, X1-A, X1-B, H0-B`. Готовый X1 artifact
строится до timing. Его первое производство измеряется и публикуется отдельно.

Для каждой строки сохраняются raw samples, median, MAD, min/max, CPU, peak RSS,
allocation blocks/bytes, retained и peak heap. При заметной посторонней нагрузке
или изменении inputs аннулируется и повторяется вся соответствующая пара; gate
не получает исключение. Raw logs живут вне source до принятия, после чего
числа добавляются в существующую private JSONL history/checkpoint и русское
acceptance summary без нового DTO/schema.

### Semantic и structural gates

- effective context: `1798`, SHA-256
  `4006d1c39dd3f767f2d8f2f88917123df4215dd091b146c6d27b201fa628478f`;
- full resolution: total/resolved/unresolved `2490/2286/204`, SHA-256
  `b37bd7885b01262821fb4f929a8ad576fc53de23c61264adfaeff82552bd3287`;
- полный provider storage + BSL/SDBL catalog + snapshot resolver transcript
  совпадает с H0 после нормализации локальных ID;
- parity probe продолжает работать после недоступности SQLite и HBK;
- borrowed filtered global/type-member traversal имеет zero provider
  allocations;
- production runtime не удерживает полный owned HBK graph рядом с mmap.

### Performance/resource gates

Для каждого A/B повтора отдельно:

- cold module context median X1 `<= 0.50x` парного H0;
- prepared handle median X1 `<= 1.10x` H0;
- prepared full resolution median X1 `<= 1.05x` H0;
- peak RSS wall-процесса X1 строго ниже H0;
- cold `dhat` peak heap X1 строго ниже H0.

Все gates обязательны. Aggregate score, ranking, waiver и перенос выигрыша одной
метрики на провал другой запрещены.

## Условный cutover и cleanup

При полном pass отдельная проверенная задача:

1. делает mapped X1 единственным canonical snapshot open;
2. мигрирует все snapshot catalogs/adapters и реальный analyzer construction;
3. фиксирует immutable HBK base dictionary в downstream task 1.14;
4. доказывает отсутствие SQL/HBK fallback и owned graph;
5. только затем открывает scoped cleanup.

Cleanup удаляет лишь inventory-proven replaced runtime constructors, owned
binary-cache reader/model/indexes, stale tests/features/dependencies и лишние
conversion paths. Экспериментальные ветки и durable evidence сохраняются;
отдельные search/debug contracts и несвязанные examples не удаляются.

## Reintroduction guards

- Analyzer и snapshot catalogs не могут открывать `SearchIndex` или читать HBK.
- Canonical snapshot open не может materialize полный `Vec<String>`/fact graph
  и не может fallback к owned/SQL path.
- Production artifact не содержит X1-PROJECTED sections или готовые context
  combinations.
- Нет второй provider entity/read/catalog family или копии HBK dictionary.
- `v8-context` не удерживает copied selected entity shapes: после операции
  допустимы только canonical key, source tier/ordinal и существующий locator;
  provider view, name/signature/parameters и selected entity cache не живут
  дольше вызова. Owned resolver DTO допустим только на существующей
  compatibility boundary и не является HBK storage.
- Benchmark module не содержит private provider reader, parser или verifier.

## Task-local plan: OpenSpec 3.1 — writer и полная byte-validation

Этот slice реализует только детерминированное производство X1 generation и его
проверку. Он не добавляет runtime mmap-open, catalog views, slot/current
publication или переключение потребителей: это отдельные пункты 3.2–3.5.

### Контракт slice

1. Добавить один закрытый deep module `snapshot/x1_format.rs`. Он владеет X1
   header/directory/record encoding и полной проверкой байтов, но не вводит
   второй snapshot owner или public record/view family.
2. Добавить на существующий `HbkFactSnapshotBuildReport` build-only операцию
   записи нового immutable generation file. Writer не принимает свободную
   строку версии. Report сохраняет provider path и cache metadata, а explicit
   X1 build boundary выводит typed platform version из канонического
   version-directory исходного HBK (`…/<major.minor.patch.build>/
   shcntx_*.hbk`) и отклоняет источник, для которого привязка отсутствует или
   неоднозначна. Привязка намеренно не дублируется в report как отдельное
   потенциально устаревшее поле: writer повторно выводит и сверяет её после
   записи. Writer выводит
   provenance из фактического SQLite/HBK input, повторно сверяет canonical HBK
   path, вычисляет SHA-256 внутри процесса, пишет только в несуществующий
   generation path и не является runtime open/fallback. В task 3.2 runtime
   caller обязан передать ожидаемую platform version из выбранной установки и
   reader сравнивает её с проверенной header-привязкой.
3. Кодировать только принятые X1 части: основной flat payload, global SoA,
   collision-safe type-name hash и owner-contiguous member AoS/range. Не
   кодировать X1-PROJECTED, готовые context combinations или experiment-only
   metadata.
4. Перед успешным возвратом проверить in-memory bytes и повторно прочитанный
   generation file: magic/layout/extraction/provider schema, exact platform
   version, locale/source identity, artifact/payload integrity, directory
   order/non-overlap/alignment/bounds, UTF-8, tags, ranges, CSR/sort/hash и
   owner-contiguous invariants.
5. Сохранить детерминизм: одинаковый snapshot и compatibility input дают
   одинаковые байты независимо от пути output и времени запуска. В header не
   попадают `built_at`, mtime или абсолютный путь provider SQLite; source HBK
   path сохраняется только как provenance.
6. Добавить behavior tests для byte-for-byte repeat, collision chain в
   type-name hash, отсутствия
   corpus-specific constants/projected sections, неверных версий/identity,
   truncation/checksum/directory/range/tag/UTF-8 corruption и отказа
   перезаписывать существующий generation. Unit fixtures создают реальный
   минимальный HBK-файл внутри version-directory во временном каталоге; они не
   зависят от `/opt/1cv8`.

### Полный section manifest

Task 3.1 кодирует весь observable owned snapshot, а не только hot path. Имена
ниже являются private physical roles; ни один из них не входит в public API.

| Observable owner / lookup | X1 sections, кодируемые в 3.1 |
|---|---|
| Строковый словарь | `Strings`, `StringOrder`, `SourceLocale` в header |
| Platform types | `PlatformTypes`, `PlatformTypeIds`, `PlatformTypeNames`, `PlatformTypeTemplates`, `PlatformTypeNameHash` |
| Type members | `TypeMembers`, `MemberIds`, `MembersByOwnerKeys/Offsets/Values`, `TypeMemberRanges`, `MemberAvailabilityHot`, `MembersByOwnerName`, `MembersByOwnerNameKind` |
| Callables/constructors/module events | `Callables`, `CallableIds`, `CallablesByOwnerKeys/Offsets/Values`, `CallablesByOwnerName`, `ConstructorsByTypeKeys/Offsets/Values`, `ModuleEventNames`, `ModuleContextsByDomainLanguageKind` |
| Globals | `Globals`, `GlobalNames`, `GlobalsByDomainNameKind`, `GlobalAvailabilityLocators/Masks/Kinds` |
| SDBL tables | `QueryTables`, `QueryTableIds`, `QueryTableNames`, `QueryTableSyntaxNames`, `QueryTableIdentifiers` |
| SDBL fields/parameters | `QueryFields`, `QueryFieldsByTableKeys/Offsets/Values`, `QueryFieldsByTableName`, `QueryParameters`, `QueryParametersByTableKeys/Offsets/Values`, `QueryParametersByTableName` |
| Language facts | `LanguageFacts`, `LanguageIds`, `LanguageNames` |
| Enums | `Enums`, `EnumIds`, `EnumNames`, `EnumValues`, `EnumValueIds`, `EnumValuesByEnumKeys/Offsets/Values`, `EnumValuesByEnumName` |
| Nested payload | `MetadataTemplates`, `Signatures`, `Parameters`, `TypeRefs`, `TemplateBindings`, `TemplateArguments`, `Names`, `StringIds` |
| Fact state/provenance | `FactIds`, `AvailabilityByFactKeys/Offsets/Values`, `AvailabilitySinceByFact`, `SourceByFact`, `RelationsBySourceKindKeys/Offsets/Values` |
| Artifact provenance | header identity fields и `CompatibilityMetadata` |

Ничего из текущих `HbkFactSnapshot`/`HbkFactReadHandle` observable facts или
lookup не откладывается. `X1-PROJECTED` sections отсутствуют. Task 3.2
переиспользует этот же validator перед созданием typed mmap views; второй
validator или ослабленный runtime subset запрещён.

### Structure impact

- Единственный владелец semantic facts остаётся `HbkFactSnapshot`; writer
  читает его приватные поля внутри `snapshot` и создаёт только transient bytes.
- Новый публичный surface ограничен build input/report/error и методом
  существующего build report. Section IDs, offsets и flat record types остаются
  private.
- В runtime не добавляется второй graph, cache, dictionary, catalog, adapter
  или fallback. `SearchIndex` используется только до завершения build report.
- Новая dependency допустима только для SHA-256 provenance; storage/index crate
  не добавляется.

### Reintroduction guard

- Production source scan не находит `REQUIRED_*`, `experiment_*`,
  `X1_PROJECTED`, benchmark paths/schema или готовые availability projections.
- Tests доказывают `create_new`: существующий generation не изменяется.
- Platform-version test доказывает отказ extraction/build boundary при
  несовпадении version-directory и заявленной/выбранной установки, а task 3.2
  добавит отказ open при несовпадении expected version с header.
- Collision test использует детерминированные разные ключи с одним initial
  bucket при штатной bucket-width; поиск настоящей 64-bit hash collision и
  test-only production hasher не требуются.
- Writer module не экспортирует reader/views и не импортируется catalog/
  resolver crates до task 3.2–3.4.
- Commit diff ограничен format/writer, его behavior tests, dependency и
  обязательной актуализацией spec/task ledger.

### Проверка и commit gate

- `cargo fmt --check`;
- focused `syntax-helper-search` X1 writer/validator tests;
- `cargo test -p syntax-helper-search`;
- `cargo clippy -p syntax-helper-search --all-targets -- -D warnings`;
- real frozen input: два generation получают одинаковый SHA-256, затем один
  generation проходит standalone validation без runtime fallback;
- strict OpenSpec validation, `git diff --check`, независимые skeptic-review и
  codebase-design review; только после этого пункт 3.1 и долгосрочный ledger
  отмечаются завершёнными отдельным commit.

### Результат OpenSpec 3.1

Production X1 writer и единый полный byte-validator реализованы в закрытом
`snapshot/x1_format.rs`; runtime mmap-open, typed views, slot publication и
consumer switching не добавлены. Writer использует `create_new`, проверяет
in-memory и повторно прочитанные bytes, ставит generation read-only и повторно
сверяет SHA/size/schema/platform/source identity после записи.

`SourceByFact` сохраняется, когда provider index уже содержит нормализованную
fact provenance. Frozen H0 index был создан до её заполнения, поэтому пустая
секция является допустимым legacy input; каждая присутствующая запись обязана
указывать на проверенный HBK и допустимую locale. Artifact-level source path,
locale, source locale, platform version, HBK/SQLite size и SHA обязательны
всегда.

На frozen input `8.3.27.1859/ru` два независимых generation дали одинаковые
`12,430,416` bytes и SHA-256
`0f5843f95401ba9cb5421b2ecc58a101779e43b17a86909d484bd6123ce3ffd7`.
Header подтвердил исходный HBK SHA-256
`5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48`
и provider SHA-256
`317f3cdd914e635c89b975bf9ebcf28238bdbabd54e455121a083558d4e05f5e`.
Следующий последовательный slice 3.2 зафиксирован ниже и завершён: immutable
mmap-open переиспользует тот же validator без второй ослабленной проверки.

## Task-local plan: OpenSpec 3.2 — validated mmap generation

Этот slice добавляет низкоуровневого владельца отображённого X1 generation,
но ещё не делает его публичным `HbkFactSnapshot` runtime и не открывает
entity/catalog access. Такое разделение необходимо: полный существующий
`HbkFactReadHandle` становится mapped только в задачах 3.3–3.4, а безопасный
межпроцессный lifecycle со stable-slot lock — в задаче 3.5.

### Контракт slice

1. Закрытый `snapshot/x1_format.rs` владеет `memmap2::Mmap`, исходным read-only
   `File`, проверенным directory/counts и compatibility identity. Section IDs,
   offsets, raw records и mmap не экспортируются из snapshot module.
2. Open принимает явный generation path и runtime expectation: точные platform
   version, locale, source locale и SHA-256 выбранного HBK. Он не читает HBK или
   SQLite, не вызывает `SearchIndex`, owned materialization/binary cache и не
   выполняет fallback/rebuild.
3. Сначала проверяются обычный файл, read-only permissions и заявленный размер,
   затем создаётся read-only file mapping. До сохранения какого-либо typed
   доступа вызывается тот же полный `validate_mmap_expected`; отдельного
   облегчённого runtime-validator нет. Header/compatibility metadata обязаны
   совпасть с runtime expectation, а provider identity проверяется на внутреннюю
   согласованность artifact без доступа к provider input.
4. В этом slice mapping доступен только внутри format module и его behavior
   tests. Без shared stable-slot lock он является low-level building block для
   гарантированно неизменяемого explicit generation path, а не законченным
   concurrent runtime API. Safe public `HbkFactSnapshot::open` появляется
   только после 3.3–3.5; 3.2 не заявляет защиту от внешнего truncate/chmod/write.
5. Ошибки filesystem/map сохраняют path и source; повреждение или
   compatibility mismatch возвращаются отдельной typed snapshot-artifact
   ошибкой. Invalid open ничего не изменяет и не запускает setup.

### Non-goals и guards

- Не добавлять catalog/entity views, dictionary lookup, availability traversal,
  slot/current pointer, locks, publication, ensure/rebuild или consumer switch.
- Не добавлять второй публичный snapshot owner, mapped DTO/record family,
  `X1-PROJECTED`, experiment API или owned fallback.
- Единственный `unsafe` участок — file-backed mmap constructor с `SAFETY`
  контрактом immutable generation; raw struct casts/transmute не используются.
- Runtime-open path source scan не находит `SearchIndex`, HBK reader,
  `from_path`, `from_index`, `from_path_with_binary_cache` или owned graph
  construction.

### Behavior tests и commit gate

- Mapping живёт вместе с private owner, проходит full validator и даёт те же
  counts/identity, что standalone byte validation.
- Open проходит после удаления/недоступности SQLite и HBK inputs.
- Open отклоняет writable/non-regular generation, неверные magic/layout/schema,
  platform/locale/source-locale/source-SHA expectation, truncation, checksum и
  section corruption.
- Отдельный test подтверждает, что validation failure не возвращает mapping;
  borrow/lifetime typed views в 3.2 ещё отсутствуют и поэтому не могут пережить
  owner.
- `cargo fmt --check`, focused mmap tests, package tests/clippy, full workspace
  tests, strict OpenSpec validation, `git diff --check`, unsafe/skeptic/code
  review проходят до отметки 3.2 отдельным commit.

### Результат OpenSpec 3.2

Закрытый `X1MappedGeneration` удерживает исходный read-only `File`,
`memmap2::Mmap`, проверенный directory/counts и compatibility identity. Open
проверяет regular/read-only metadata до и после mapping, повторно использует
полный task 3.1 validator и отдельно сверяет runtime expectation по platform
version, locale, source locale и HBK SHA-256. Corruption и compatibility
mismatch отделены от filesystem/map ошибок типом snapshot-artifact error.

Mapping ещё не экспортирован как `HbkFactSnapshot::open`: его `unsafe` boundary
явно требует неизменяемости explicit generation на всё время жизни owner, что
в production будет обеспечено stable-slot shared lock в 3.5. В этом slice нет
typed entity views, SQL/HBK access, owned materialization, fallback, rebuild,
publication или consumer switching. Focused tests и полный workspace подтвердили
magic/layout/schema, expectation, truncation/checksum/section, writable и
non-regular negative cases; независимые unsafe/code/skeptic reviews не нашли
блокеров. Следующий slice — OpenSpec 3.3, borrowed payload access внутри
единственного snapshot/read-handle interface.

## Task-local plan: OpenSpec 3.3 — borrowed forward payload

Этот slice реализует только forward/ID/range чтение уже проверенного mapped
generation и две приоритетные provider-native операции фильтрации. Reverse
dictionary/name/relation lookup остаётся в 3.4, а safe public open, slot lock и
publication — в 3.5. До них текущие owned catalogs остаются H0 baseline и не
маскируют mapped path fallback-ом.

### Контракт slice

1. `X1MappedGeneration` получает закрытый read handle. View возвращается by
   value и содержит только `&X1MappedGeneration` плюс Copy head/range; ссылка на
   временно декодированный record не возвращается. Nested names, signatures,
   parameters, type refs, template bindings/arguments и string-id arenas
   обходятся ленивыми exact-size iterators, привязанными к lifetime owner.
2. Forward views покрывают все поля уже существующих `HbkPlatformType`,
   `HbkTypeMember`, `HbkCallable`, `HbkGlobalFact`, `HbkQueryTable`,
   `HbkQueryField`, `HbkQueryParameter`, `HbkLanguageFact`, `HbkEnum`,
   `HbkEnumValue` и их nested records. «Поля документации» в этом slice означают
   только уже принятые source provenance `hbk_path/locale/toc_path/html_path/
   page_title`; новые markdown/body/description payload не добавляются.
3. После полного validator доступ к проверенным IDs/ranges является
   infallible internal invariant. Record bytes по-прежнему декодируются через
   `BinaryValue` by value; raw casts, `transmute`, self-references и owned
   `Hbk*`/`Vec` materialization запрещены.
4. Добавить borrowed filtered global traversal и filtered members одного уже
   известного type owner. Filter принимает маску frozen `AvailabilityContext`
   и режим `ANY`/`ALL`; universal проходит всегда, `ANY(empty)` принимает только
   universal, `ALL(empty)` принимает все. `ModuleContextKind` не принимается.
   Возвращаются только session-local typed IDs/views в исходном порядке.
5. В 3.3 mapped view family остаётся закрытой внутри единственного
   `snapshot/x1_format.rs` building block и не создаёт второй public owner,
   catalog/source или generic semantic-read trait. В 3.5 она становится
   внутренним storage единственного `HbkFactSnapshot`/`HbkFactReadHandle`; public
   catalog migration проверяется end-to-end в 4.4.
6. Локальное operation-lifetime правило: downstream может скопировать только
   canonical key, source tier/ordinal и существующий locator, но не может
   удерживать provider entity view, name, signature, parameters или selected
   entity cache после операции. Owned resolver DTO допустим только на
   существующей compatibility boundary и не является HBK storage.

### Non-goals и guards

- Не реализовывать name/alias/template/relation lookup, reverse string ID,
  public `HbkFactSnapshot::open`, locks/publication, ensure/rebuild, resolver
  switch или SQL cleanup.
- Не переносить широкий экспериментальный `semantic_read.rs`, public flat/X1
  record family, boxed/dyn iterator bridge, `Arc<Mutex<_>>`, `Box::leak`,
  `'static`, retained copied entity cache или `X1-PROJECTED`.
- Не кэшировать decoded provider entities. Допустимо один раз сохранить только
  малую таблицу проверенных section descriptors/views внутри mmap owner.

### Behavior tests и commit gate

- Fixture parity сравнивает каждое forward observable поле и nested order
  mapped views с owned H0, включая property/method, callable overloads,
  signatures/parameters/return refs, global method/property, SDBL/language/
  enums, provenance и universal/explicit availability.
- Filter tests покрывают `ANY`/`ALL`, один и несколько контекстов, empty request,
  universal, non-match, kind filtering и только members одного owner.
- Отдельный allocation-enabled focused binary/test подтверждает ноль provider
  allocation blocks/bytes для steady filtered globals, known-type members и
  полного nested payload traversal; setup/open не входит в steady interval.
- Views/iterators заимствуют owner; compile-smoke и API-shape test не допускают
  `'static`, owned entity return или view после drop owner.
- Source scan нового path не находит SQL/HBK/materialization/fallback,
  `experiment_*`, projected sections, generic semantic trait, boxed iterator
  или copied provider cache. Package/workspace tests, clippy, strict OpenSpec,
  diff check и независимый code/skeptic review обязательны до отметки 3.3.

### Результат OpenSpec 3.3

Slice завершён. Закрытый `X1MappedReadHandle` читает из проверенного mapping
узкие by-value views всех существующих payload families, лениво обходит
signatures, parameters, type refs, template bindings, names и string-ID ranges,
а также выполняет provider-native `ANY`/`ALL` enumeration globals и
непосредственных members одного известного type owner. `ModuleContextKind` не
участвует в фильтре.

Fixture parity покрывает все forward поля и provenance для всех десяти
вариантов `HbkFactRef`. Отдельный allocation-enabled тест после прогрева
проходит весь nested payload, обе filtered operations и provenance с нулём
allocation/reallocation calls и нулём allocated bytes. Package tests `93/93`,
полный workspace, clippy, strict OpenSpec и независимое ревью прошли. Family
остаётся private: public open, reverse lookup и migration каталогов не
выполнялись. Следующий slice — OpenSpec 3.4, base dictionary и provider lookup.

## Task-local plan: OpenSpec 3.4 — base dictionary и provider lookup

Этот slice дополняет private mapped read owner обратной стороной существующего
`HbkFactReadHandle`: разрешением текста в generation-local `StringId`,
name/alias/template/owner lookup и чтением CSR availability/relation indexes.
Public snapshot owner, locks/publication и migration catalogs остаются в 3.5 и
4.4; отдельный X1 API наружу не публикуется.

### Контракт slice

1. Exact reverse dictionary использует `StringOrder` и бинарный поиск по
   проверенному UTF-8 словарю. Результат — только `StringId` текущего generation;
   отсутствие возвращает `None`, линейное сканирование и копия словаря
   запрещены.
2. Type-by-name использует единственный persisted `PlatformTypeNameHash`,
   включая collision/probe-chain и все значения одинакового normalized key.
   Остальные уже persisted sorted indexes/CSR читаются напрямую; новый sidecar
   index или runtime rebuild запрещён.
3. Private handle покрывает существующий provider lookup surface по точной
   таблице ниже. Результаты сохраняют исходный порядок, hit/miss и ambiguity как
   lazy exact-size iterators typed IDs либо owner-tied range views там, где
   owned API сейчас возвращает borrowed slice.
4. Raw compatibility methods выполняют ту же name normalization, что текущий
   `normalize_lookup_key`: удаляют whitespace и применяют Unicode lowercase,
   создавая не более одного request-local normalized buffer на каждый raw
   аргумент, который требует нормализации. Они делегируют
   private pre-normalized helper, который существует только для проверки
   steady analyzer path и не становится вторым public API family.
   Relation-kind повторяет текущую семантику: нормализуется перед reverse
   dictionary lookup. Exact fact/entity IDs, template family/variant и прямой
   dictionary reverse lookup ничего не нормализуют. Entity-shaped DTO,
   candidate/result `Vec`, boxed/dynamic iterator и retained query/result cache
   запрещены.
5. CSR helper сначала бинарно ищет key, затем возвращает owner-tied view
   существующего values range. Empty/miss не создаёт allocation или временный
   owned slice. После полного validator все decoded lookup records считаются
   infallible internal invariant.
6. `AvailabilityContext` evidence и relations только читаются; provider-native
   `ANY`/`ALL` filtering и называемый в T183 `inherited_members` ordered stream
   непосредственных candidates одного owner остаются механизмом 3.3.
   Транзитивное раскрытие других типов, cross-source precedence,
   `effective_members` и resolve остаются `v8-context`.

### Таблица совместимости `HbkFactReadHandle`

| Existing method/group | Реализация 3.4 | Порядок/особенность |
|---|---|---|
| `experiment_string`, `experiment_string_id` | `Strings` + `StringOrder` | exact UTF-8, generation-local ID |
| `global_fact_ids`, `query_table_ids`, `query_field_ids`, `query_parameter_ids` | validated section counts | ascending local ID |
| `facts_by_id` | `FactIds` | все duplicate fact refs, порядок persisted index/value |
| `platform_type_by_id` | `PlatformTypeIds` | exact, `Option` |
| `platform_types_by_name` | `PlatformTypeNameHash` + `PlatformTypeNames` | normalized primary/alias; весь persisted same-key range, value order как H0 |
| `platform_types_by_template_key` | `PlatformTypeTemplates` | exact family/variant, all candidates |
| `members_of_type`, `member_by_owner_name`, `member_by_owner_name_kind` | owner CSR + оба owner/name indexes | raw range; normalized name; optional kind |
| `callables_of_type`, `callable_by_owner_name`, `constructors_of_type` | callable/constructor CSR + owner/name index | raw range и normalized name |
| `globals_by_name`, `globals_by_domain_name_kind` | оба global indexes | normalized name, optional kind, all candidates |
| `module_events`, `module_event_by_context_name`, `module_context_events` | module event/context indexes | normalized owner/name/language/module-kind как H0 |
| `query_table_by_id`, `query_tables_by_name`, `query_tables_by_syntax`, `query_tables_by_identifier` | четыре table indexes | exact ID; normalized display/syntax/identifier |
| `query_fields`, `query_fields_by_name`, `query_parameters`, `query_parameters_by_name` | два owner CSR + два owner/name indexes | raw range и normalized name |
| `language_fact_by_id`, `language_facts_by_name` | language indexes | exact ID, normalized primary/alias |
| `enum_by_id`, `enums_by_name`, `enum_value_by_id`, `enum_values`, `enum_values_by_name` | enum/value indexes + owner CSR | exact ID, normalized primary/alias, raw range |
| `availability_contexts`, `available_since` | availability CSR + sorted fact lookup | persisted evidence order; miss empty/`None` |
| `relations_by_source_kind` | relation CSR + reverse dictionary | normalized kind, persisted target order; miss empty |

Forward entity accessors и `source` уже покрыты 3.3. Safe public
`worker_handle`/catalog wiring не добавляется до 3.5/4.4.

### Behavior tests и commit gate

- Owned-vs-mapped fixture parity покрывает каждую строку таблицы: primary/alias,
  exact/normalized hit, miss, duplicate-key multi-hit/ambiguity для type names,
  globals, members, enum values, query fields/parameters, optional kind, empty
  CSR и deterministic persisted order. `facts_by_id` отдельно проверяет
  несколько разных `HbkFactRef` одного exact ID, relation-kind — normalized hit.
- Отдельный hash-collision тест доказывает успешный hit/miss через probe chain;
  reverse dictionary проверяет UTF-8 hit/miss и generation-local ID.
- Allocation-enabled focused tests после прогрева разделены: pre-normalized
  steady lookup и обход ID/ranges дают ноль allocations; raw compatibility
  lookup допускает ровно один normalized `String` buffer без result allocation.
- Structural review ограничен добавленными production lookup methods/helpers
  mapped read path: там нет linear dictionary scan, rebuilt `HashMap`/
  `BTreeMap`, candidate/result `Vec`, boxed/dyn iterators, SQL/HBK/fallback,
  projected sections или второго public read/catalog family. Существующие
  writer/validator/test fixtures и их build-time `Vec` вне этого scan.
  Package/workspace tests, clippy, strict OpenSpec, diff check и независимое
  ревью обязательны до отметки 3.4.

### Результат OpenSpec 3.4

Slice завершён. Private mapped handle теперь покрывает всю таблицу lookup
существующего `HbkFactReadHandle`: exact ID, normalized primary/alias,
owner/name/kind, templates, module contexts/events, query/language/enum,
availability/available-since и relations. Reverse dictionary использует
`StringOrder`, type name — единственный persisted X1 hash с проверенной probe
chain, остальные операции — persisted sorted indexes и CSR ranges. Runtime
sidecar/rebuild, candidate/result collections и entity DTO не добавлены.

Owned-vs-mapped fixture parity покрывает все method groups, multi-hit order,
ambiguity, hit/miss, optional kind и normalized relation kind; отдельный mapped
тест проходит реальную hash collision chain. Package tests `95/95`, полный
workspace, clippy и strict OpenSpec прошли. Allocation-enabled проверки
подтвердили ноль allocations для pre-normalized steady lookup/range traversal и
ровно один request-local `String` allocation без reallocation для одного raw
name argument. X1 остаётся private и non-canonical. Следующий slice — 3.5,
stable-slot shared/exclusive lock и atomic immutable generation publication.

## Task-local plan: OpenSpec 3.5 — stable slot и immutable publication

Этот slice закрывает unsafe precondition task 3.2: mapped generation открывается
только под shared lock стабильного логического slot, а единственная product
операция изменения публикует новый immutable файл под fail-fast exclusive lock.
Catalog/analyzer migration и превращение mapped storage в public
`HbkFactSnapshot::worker_handle` остаются в 4.4, чтобы незавершённый mapped
snapshot нельзя было открыть через public API с пустым/owned fallback surface.

### Layout и lock contract

Один slot root содержит только:

- `snapshot.lock` — стабильный inode/lock target; не переименовывается и не
  удаляется;
- `generations/generation-<artifact-sha256>.x1` — read-only immutable files;
- `current` — UTF-8 имя одного generation без `/`, `\\`, `..` или абсолютного
  пути;
- временные файлы с process/counter suffix, удаляемые только под exclusive lock.

Используются стабилизированные в Rust 1.89 `std::fs::File::{lock_shared,
try_lock}`; новая crate dependency не нужна. Reader блокирующе получает shared
lock до чтения `current` и удерживает сам `File` до drop mapped owner. Writer
использует только `try_lock`; `TryLockError::WouldBlock` немедленно становится
typed `SearchError::SnapshotInUse { path }`, без retry/sleep/timeout.

Lock является OS advisory coordination boundary для всех product операций, а
read-only permissions generation/current — только дополнительный диагностический
инвариант, не safety mechanism. Threat model ограничен доверенным service-data
slot, который изменяют только cooperating HBK setup/runtime процессы. Любая
поддерживаемая попытка обновления при активном reader получает
`snapshot-in-use`; произвольный same-user или privileged процесс, игнорирующий
lock и меняющий permissions/inode/bytes, находится вне принятого contract.
Portable advisory lock физически не запрещает такую внешнюю мутацию, поэтому
3.5 не заявляет OS-level immutability против произвольного локального процесса
и не делает mapped owner public для недоверенного произвольного path.

### Reader и publication algorithm

1. Private trusted-slot open открывает существующий `snapshot.lock`, получает
   shared lock, читает/проверяет `current`, затем вызывает единственный полный
   validator и `X1MappedGeneration::open` для указанного generation.
   Lock file и mmap owner живут в одном owner; SQL/HBK/rebuild/fallback нет.
2. `HbkFactSnapshotBuildReport::publish_x1_generation(slot)` создаёт slot layout,
   получает exclusive lock fail-fast, пишет generation через task 3.1 writer в
   `create_new` temp и полностью перепроверяет его. Operation удаляет при
   ошибке только temp paths, которые сама успешно создала; чужие/stale temp и
   любые другие slot files не сканируются и не удаляются в 3.5.
3. Artifact SHA-256 определяет immutable target name. Если target уже есть,
   writer требует stable regular non-symlink file, повторно вычисляет его full
   artifact SHA-256, проверяет полную identity/bytes и переиспользует только при
   exact match; corrupt target отклоняется. Существующий generation никогда не
   усекается и не заменяется.
4. Новый target публикуется rename temp -> generation и `fsync` generation dir.
   Затем новый read-only pointer temp записывается/`fsync`-ится и атомарно
   заменяет `current`; slot dir `fsync` завершает publication. До generation
   rename ошибка оставляет old current; после generation rename, но до pointer
   rename допустим orphan immutable generation и old current; после pointer
   rename, но до финального directory `fsync` recovery может увидеть old либо
   new pointer, но ни один принятый pointer не ссылается на partial generation.
   Partial/corrupt pointer никогда не принимается и fail-closed.
5. Старые generation не удаляются в 3.5. Замена HBK/source публикует новый
   generation только после drop всех старых readers и создаёт новое
   session-local ID space для последующих opens. Retention/GC и видимый учёт
   orphan/stale generation bytes явно принадлежат conditional cleanup 7.2, а не
   скрытой автоматической уборке этого slice.

### API и guards

- Public build/setup surface получает только publication report и typed
  `snapshot-in-use`; private slot reader возвращает единственный
  `X1MappedGeneration`, не вторую public entity/catalog family.
- Slot root и `generations/` проверяются `symlink_metadata` как реальные
  directories, не symlink. `snapshot.lock`, `current`, target generation и
  созданные temp проверяются как non-symlink regular files; после open file
  metadata сверяется с pre-open metadata по supported Linux device/inode, чтобы
  обнаружить replacement race. Pointer parser принимает ровно
  `generation-<64 lowercase hex>.x1\n`, затем присоединяет только basename к
  уже проверенному `generations/`; свободный path, separator, `..` и symlink
  запрещены. Missing/corrupt components fail-closed typed artifact error.
- Первый writer создаёт directories idempotently, затем пытается `create_new`
  стабильный empty `snapshot.lock`; `AlreadyExists` разрешает только проверенный
  regular non-symlink lock. Concurrent first writers открывают один и тот же
  stable inode и расходятся на `try_lock`; reader никогда не создаёт missing
  lock. Замена lock inode во время open обнаруживается pre/post metadata check.
- Lock acquisition предшествует чтению discovery metadata и mapping; writer
  lock предшествует cleanup/build/publication. Lock target никогда не меняется.
- Единственный `unsafe Mmap::map` остаётся в `X1MappedGeneration::open`. Его
  private caller proof действует внутри явно принятого trusted/cooperating slot
  contract: shared lock удерживается тем же owner, product writer не может
  получить exclusive lock, generation никогда не изменяется in-place,
  pre/post-open inode проверен, mutable mapping/raw cast/transmute отсутствуют.
  Это не доказательство против внешнего процесса, нарушающего threat model.
- Explicit task 3.1 `write_x1_generation` остаётся build-only `create_new` API;
  runtime consumer не может открыть его без slot lifecycle.

### Behavior tests и commit gate

- Два concurrent reader handle удерживают shared lock; publisher немедленно
  получает exact `SnapshotInUse`, не создаёт temp/generation и не меняет
  `current`. После drop всех readers publication проходит.
- Reader видит только old либо new fully validated generation. Publication не
  может идти одновременно с живым старым owner; после его drop следующая
  session открывает новый current.
- Missing/corrupt/traversal pointer, identity mismatch, existing exact/corrupt
  generation и injected failures до generation rename, между generation и
  pointer rename, после pointer rename проверяют old-or-new valid recovery и
  отсутствие partial pointer.
- Symlink tests покрывают slot root, generations dir, lock, current, generation
  target и temp candidate; unrelated/stale files остаются нетронутыми.
- Source replacement test проверяет изоляцию двух последовательных sessions и
  новое logical mapping, но не сравнивает numeric IDs как durable identity и не
  выполняет их migration.
- Source/unsafe review перечисляет каждый private mmap caller, поле owner с
  shared lock и drop/lifetime chain views -> mapped generation; product writer
  всегда exclusive, нет in-place write, wait/retry,
  SQL/HBK fallback или public второй snapshot owner. Package/workspace tests,
  clippy, strict OpenSpec и diff check обязательны до отметки 3.5.

### Результат OpenSpec 3.5

Slice завершён. Public build/setup API публикует content-addressed immutable
generation через fail-fast exclusive `std::fs::File` lock и возвращает
`SnapshotInUse`, пока хотя бы один private mapped session удерживает shared
lock. `current` заменяется атомарно только после полной записи, проверки и
`fsync` generation; старые и orphan generation намеренно не удаляются до
условной уборки 7.2.

Private stable-slot reader проверяет точный pointer grammar, platform/locale/
source identity, content address, размер, permissions, структуру X1 и Linux
device/inode до typed access. Slot path и его существующие ancestors, lock,
pointer, generation и temp components не могут быть symlink; oversized pointer
и generation отклоняются до неограниченного чтения или хеширования. Mmap,
read-only file и shared-lock file принадлежат одному owner и уничтожаются в
порядке mapping -> generation file -> lock.

Lifecycle tests покрывают два concurrent reader, fail-fast writer, concurrent
first setup, exact generation reuse, corrupt/missing/non-regular/symlink
components и ancestors, три publication failure window с old-or-new recovery,
source replacement между sessions, сохранение чужих temp и open после удаления
HBK/SQLite. Package tests `104/104`, all-features `106 passed` с тремя штатно
ignored allocation probes, полный workspace, package clippy, fmt, strict
OpenSpec и независимое unsafe/code review прошли. Workspace-wide clippy на
Rust 1.95 по-прежнему имеет два unrelated pre-existing lint в
`syntax-helper-extract`; slice их не меняет. X1 остаётся private и
non-canonical. Следующий gate — OpenSpec 4.1 compatibility/lifecycle matrix.

## Task-local plan: OpenSpec 4.1 — compatibility/lifecycle matrix

Этот slice не добавляет второй validator и не открывает public runtime API. Он
собирает уже реализованные 3.2/3.5 инварианты в проверку реального stable-slot
open, чтобы дальнейшая catalog integration опиралась на один доказанный
lifecycle boundary.

- Valid published slot открывается повторно и из нескольких concurrent
  sessions с одной внешней expectation. Старый session удерживает shared lock,
  replacement fail-fast отклоняется, после drop новая generation открывается
  только с новой expectation; старые session-local ID не мигрируются и не
  сравниваются как durable identity.
- На valid generation каждая mismatch expectation отдельно проверяет exact
  platform version, locale, source locale и HBK SHA. Ошибка обязана сохранять
  поле `CompatibilityMismatch`, а не превращаться в rebuild/fallback.
- Для magic, binary layout, extraction/provider schema, truncation, checksum и
  section corruption тест строит content-addressed read-only generation и
  корректный `current`, чтобы пройти discovery/content-address guards и
  доказать отказ именно общего full byte-validator через stable-slot open.
- Missing/corrupt pointer, concurrent readers, blocked update, atomic
  publication, source replacement и symlink/oversize guards переиспользуют
  behavior tests 3.5; прямые unsafe explicit-generation tests остаются только
  unit evidence validator, не поддерживаемым runtime path.
- Commit gate: package/workspace tests, package clippy, fmt, strict OpenSpec,
  diff check и независимый review. X1 остаётся private/non-canonical; следующий
  slice 4.2 проверяет полный production corpus, а не fixture-only counts.

### Результат OpenSpec 4.1

Stable-slot compatibility/lifecycle matrix завершена. На valid published
generation отдельные opens возвращают точный `CompatibilityMismatch` для
platform version, locale, source locale и HBK SHA без rebuild/fallback. Для
magic, layout, extraction/provider schema, truncation, payload checksum и
section corruption тест сначала отдельно доказывает успешные pointer discovery
и content-address validation, а затем получает `Invalid` от общего full
byte-validator через private stable-slot open.

Матрица переиспользует lifecycle evidence 3.5 для concurrent/repeated sessions,
fail-fast replacement, atomic publication и нового generation после source
replacement; numeric session-local IDs не сравниваются и не мигрируются.
Package tests `106/106`, полный workspace, package clippy, fmt, strict OpenSpec,
diff check и независимый review прошли. X1 остаётся private/non-canonical.
Следующий slice — 4.2 full-corpus storage parity на frozen S83 input.

## Task-local plan: OpenSpec 4.2 — full-corpus storage parity

Slice проверяет весь frozen S83 provider corpus до открытия catalog/runtime
API. Он не добавляет production exporter, DTO или второй oracle format.

- Общий test-only comparator обходит все records owned build snapshot и
  private mapped generation по typed ID ranges и сравнивает каждое наблюдаемое
  поле: name/alias, owner/kind/domain, metadata/template data, signatures,
  parameters, return/type refs и ambiguous targets, query table syntax/role/
  owner path, language facts, enums/values, availability и provenance.
- Dictionary сравнивается по полному ID range внутри одного generation build;
  это проверка сохранения writer layout, а не обещание durable numeric IDs
  между независимыми sessions. Counts и source locale сравниваются отдельно.
- Existing fixture forward-payload test использует тот же comparator, чтобы
  corpus path не имел отдельной логики сравнения. Full-corpus test является
  explicit ignored acceptance probe с provider path только через environment;
  corpus-specific absolute path не попадает в production code.
- Probe materializes owned H0 только как build/oracle side, публикует X1,
  открывает его через stable slot и сравнивает весь corpus. После будущего
  canonical cutover этот owned oracle остаётся test/build-only и не разрешает
  runtime coexistence.
- Evidence фиксирует точные frozen input/artifact identities, counts, команду и
  успешный результат; raw stdout остаётся service data. Commit gate включает
  fixture/package/workspace tests, explicit full-corpus run, package clippy,
  fmt, strict OpenSpec, diff check и независимое review.

### Результат OpenSpec 4.2

Full-corpus storage parity прошёл на frozen S83 provider. Общий comparator
совпадает для dictionary и всех десяти fact families, каждого nested payload,
availability, available-since и provenance. X1 generation снова имеет
`12,430,416` bytes и SHA-256
`0f5843f95401ba9cb5421b2ecc58a101779e43b17a86909d484bd6123ce3ffd7`.

Counts: `71,073` strings, `1,749` types, `18,004` members, `8,299`
callables, `601` globals, `53/498/56` query tables/fields/parameters, `0`
language facts и `670/2,934` enums/values. Fixture тем же comparator покрывает
language facts, отсутствующие во frozen corpus. Explicit ignored probe прошёл
за диагностические `71.73 s`; это build/comparison time, не performance gate.
Package tests `106 passed / 1 ignored`, package clippy, fmt, diff check и
независимый review прошли. Полная команда и counts записаны в
`acceptance/hbk-x1-int-evidence.md`. Следующий slice — 4.3 full-corpus lookup
parity; X1 остаётся private/non-canonical.

## Task-local plan: OpenSpec 4.3 — full-corpus lookup parity

Slice применяет всю private mapped lookup surface 3.4 к frozen corpus. Fixture
остаётся oracle для специально сконструированных duplicate/ambiguity/language/
module cases, которых может не быть в S83.

- Общий test-only comparator выводит hit keys из owned persisted indexes, но
  вызывает только semantic methods обоих read handles. Для каждого distinct
  key сравнивается ordered result: full fact/exact type, primary+alias type,
  template, owner/member/name/kind, callable/constructor, global/domain/kind,
  module context/event, query table/syntax/identifier/field/parameter,
  language, enum/value, availability/available-since и relation source/kind.
- Owner ranges перечисляются для каждого valid owner, включая пустые. Exact ID
  проверяется для каждой record identity через `facts_by_id` и family methods,
  а не только sample первого/последнего record.
- Один fixed absent Unicode-safe key на method family проверяет deterministic
  miss/empty; fixture parity сохраняет duplicate multi-hit order, optional
  kind, ambiguity и unsupported/unknown cases. Никакой winner не выбирается.
- Full-corpus probe переиспользует env-driven build/publish/stable-slot open
  4.2 и печатает число выполненных semantic calls и SHA-256 нормализованного
  ordered result transcript. Numeric IDs не сравниваются между sessions;
  owned/mapped стороны принадлежат одному build generation.
- Production indexes/format/API не меняются. Commit gate: fixture/package/
  workspace tests, explicit frozen probe, package clippy, fmt, strict OpenSpec,
  diff check и независимый review. Следующий slice 4.4 впервые открывает единый
  `HbkFactReadHandle`/catalog seam; X1 остаётся non-canonical.

Slice завершён. Fixture comparator сохраняет duplicate,
ambiguity, optional-kind, language/module и miss cases. Frozen S83 probe
сравнил `280,317` semantic call pairs (`560,634` вызова двух
handles); SHA-256 нормализованного ordered transcript —
`ce7e5bf73e497703fba7c9000ac827ac07db1d3783d712eb4d7b656e45bd5847`.
Артефакт остался `12,430,416` bytes с SHA-256
`0f5843f95401ba9cb5421b2ecc58a101779e43b17a86909d484bd6123ce3ffd7`.
Package all-feature tests, explicit frozen probe, clippy, fmt, diff check и
независимый review прошли. Следующий slice — 4.4 unified
read/catalog/resolver seam; X1 остаётся non-canonical.
