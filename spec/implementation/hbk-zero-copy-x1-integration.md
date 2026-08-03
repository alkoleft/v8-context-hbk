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
- `v8-context` не удерживает copied selected entity shapes; только AIR-001/
  AIR-002 operation-local flow.
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
