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
