# Исследование layout доступности members HBK

Дата: 2026-08-01.

Назначение: собрать первичные основания для следующей гипотезы горячего
представления `TypeMember` availability и owner/member lookup после S83-AV2.
Документ не выбирает canonical backend и не меняет frozen gates T183.

## Локальный контекст

S83-AV2 измерял форму результата A:

- borrowed iteration без materialization;
- request-local compact `Vec<Av2MemberLocator(u32)>`;
- отдельное чтение полного payload по locator.

Corpus S83: 9 `AvailabilityContext`, 18,004 непосредственных members, 1,749
owners. H0 остаётся SQL/owned baseline. Все zero-copy-кандидаты AV2 были
медленнее H0/C0 на `members_by_owner_availability_borrowed/collect`, при этом
borrowed steady не аллоцировал ни в одной строке; значит, следующая гипотеза
должна изолировать именно CPU/cache/indirection layout горячего пути, а не
heap allocation. См. локальные evidence:
[S83-AV2](../acceptance/hbk-s83-av2-evidence.md),
[S83-AV1](../acceptance/hbk-s83-av1-evidence.md),
[T183 experiment contract](hbk-zero-copy-snapshot-experiment.md).

## Доказано первичными источниками

1. Плотные битовые маски подходят для предиката из девяти контекстов.

   Стандартная библиотека Rust предоставляет для целых примитивов операции
   `count_ones` и `trailing_zeros`: [Rust `u64`](https://doc.rust-lang.org/std/primitive.u64.html).
   Это позволяет хранить компактную `u16`/`u32` availability mask на member и
   использовать лёгкий предикат `(word & context_bit) != 0`.

2. Плотный bitmap позволяет заменить проверку разреженного предиката проходом
   по машинным словам и декодированием выставленных битов.

   Статья Roaring описывает применение bitmap-индексов и битового параллелизма
   в базах данных и поиске, а спецификация формата — переносимый compressed
   bitmap layout: [статья](https://arxiv.org/abs/1402.6407),
   [спецификация](https://github.com/RoaringBitmap/RoaringFormatSpec/). Для
   этого corpus обычные плотные bitmaps уже малы, поэтому полезная идея здесь —
   не сжатие, а проход по машинным словам и декодирование выставленных битов.

3. CSR является прямым прецедентом структуры «offsets + contiguous IDs».

   NVIDIA cuSPARSE описывает CSR, где offsets строк заменяют явные индексы
   строк, а элементы строки лежат непрерывно:
   [cuSPARSE CSR](https://docs.nvidia.com/cuda/cusparse/index.html#compressed-sparse-row-csr).
   Для этого проекта `row = (context, owner)`, а `column/value =
   member_locator`; это даёт прямой slice строки без прохода по другим owners.

4. Колоночный layout и раздельные буферы — стандартный инструмент организации
   памяти.

   Apache Arrow задаёт колоночный in-memory формат, отдельные value/offset
   buffers и validity bitmaps:
   [Arrow columnar format](https://arrow.apache.org/docs/format/Columnar.html),
   [Arrow intro](https://arrow.apache.org/docs/format/Intro.html). Прямой аналог
   для HBK members — SoA: hot-поля (`owner_id`, `kind`, `availability_mask`,
   `payload_locator`) в узких массивах, а локализованный текст, сигнатуры и
   provenance читаются как cold payload по locator.

5. Поведение mmap и страниц необходимо измерять, а не предполагать.

   Linux `mmap(2)` описывает file mappings, page-aligned placement и флаги
   наподобие `MAP_POPULATE`; random access от этого не становится бесплатным:
   [Linux mmap(2)](https://man7.org/linux/man-pages/man2/mmap.2.html).
   Rust `memmap2` предоставляет file-backed mapping API, но безопасность и
   выбор access pattern остаются ответственностью вызывающего кода:
   [документация memmap2](https://docs.rs/memmap2).

6. SIMD доступен, но не оправдан автоматически.

   Стабильный `std::arch` Rust явно зависит от target и непереносим:
   [Rust `std::arch`](https://doc.rust-lang.org/std/arch/index.html). Модуль
   portable SIMD пока доступен только в nightly:
   [Rust `std::simd`](https://doc.rust-lang.org/std/simd/index.html). Intel
   предупреждает, что intrinsic может развернуться в последовательность,
   работающую хуже одной native instruction:
   [Intel Intrinsics Guide](https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html).

## Проектные выводы

1. Inline availability mask — самая компактная первая гипотеза с низким
   риском.

   Для девяти контекстов достаточно одного `u16` на member. Footprint S83:
   `18,004 * 2 = 36,008 bytes`; `u32` стоил бы `72,016 bytes`. Для
   корректного различения universal и явно заданного набора word использует
   биты `0..8` для включения контекста и бит `15` как
   `HAS_EXPLICIT_DECLARATION`: universal имеет все девять context bits и
   сброшенный флаг, explicit — флаг и непустой набор известных context bits.
   Неизвестный explicit-код отклоняется на producer boundary, а не превращается
   в universal. Порядок H0 сохраняется, если member array остаётся в текущем
   owner/member order.

2. Плотные bitmaps контекстов ещё компактнее, но всё равно требуют owner
   slicing.

   Bitmap одного контекста для всех 18,004 members требует
   `ceil(18004 / 64) * 8 = 2,256 bytes`; девять контекстов — `20,304 bytes`.
   Отдельный universal bitmap увеличивает размер до `22,560 bytes`. Это удобно
   для corpus-wide enumeration, но owner/member API всё равно нужен
   `owner -> [member_id]` range или per-context owner offsets, иначе для каждого
   owner придётся просматривать все 18,004 бита.

3. Предварительно построенный `(context, owner) -> member locators` — самый
   прямой способ попытаться обогнать H0 enumeration ценой дополнительных байт
   и работы producer.

   CSR offsets для `9 * 1,749` строк требуют
   `9 * (1,749 + 1) * 4 = 63,000 bytes`. По returned counts S83-AV2 locators
   займут `86,090 * 4 = 344,360 bytes`. Итого hot index — около
   `407,360 bytes` до headers/alignment. Отдельные пары start/end стоят
   `125,928 bytes` только для ranges и всё равно требуют concatenated locators,
   если member array физически не дублируется для каждого контекста.

4. Owner-major layout members остаётся обязательным для parity порядка H0.

   Самый дешёвый базовый layout — один owner offset array и один contiguous
   member-locator array:
   `(1,749 + 1) * 4 + 18,004 * 4 = 79,016 bytes`. Каждый owner slice затем
   фильтруется inline mask или пересекается с context bitmap. Borrowed iteration
   не требует per-query allocation, а compact materialization остаётся
   request-local `Vec` из AV2.

5. Разделение hot/cold, вероятно, важнее сжатия всего снапшота.

   AV2 показал, что payload reads mapped custom formats во многих строках
   проиграли H0. Проверяемая причина — лишняя archive/view indirection через
   cold-поля во время hot enumeration/lookup. Hot records должны быть
   fixed-width и не зависеть от names/signatures; cold payload читается только
   после выбора locator.

6. Branchless/SIMD следует оставить поздним экспериментом.

   Hot-предикат мал, а контекстов всего девять. Сначала измеряются scalar
   mask/range/bitmap layouts и perf counters. SIMD имеет смысл проверять, только
   если после удаления indirection доминируют branch misses или векторизуемые
   сравнения ID/name.

## Layout-кандидаты для опровержения

| ID | Hot layout | Ожидаемый эффект | Основной риск |
| --- | --- | --- | --- |
| `M0-mask` | owner-major members + `u16 availability_mask` | устраняет availability slice traversal и поиск по explicit-list | всё ещё сканирует каждый member owner slice |
| `M1-bitmap` | global owner-major IDs + 9 dense bitmaps | corpus-wide проход читает 282 слова на контекст | per-owner listing требует range intersection |
| `M2-csr` | CSR rows `(context, owner)` -> `u32 locator` | прямой borrowed row slice без фильтра | hot index около 0.4 MiB и producer verification |
| `M3-direct-ranges` | порядок `(context, owner)` с start/end ranges | минимальная цена row slicing при допустимом дублировании | дублирует locators либо теряет единый member order |
| `M4-soa-hotcold` | fixed-width hot columns + cold payload arena | lookup/enumeration затрагивает меньше cache lines | payload получает дополнительный locator hop |

## Найденный локальный hot path

`S83-H0` является SQL baseline по происхождению startup: SQLite используется
для построения владеющего `HbkFactSnapshot`, но steady enumeration уже работает
по Rust `Vec`/CSR в памяти. Поэтому обгон H0 требует выполнять меньше работы,
чем его direct owned-slice path, а не только заменить SQLite на mmap.

В текущем R1 fixed member head уже хранит `availability_contexts: R1Range`, но
AV2 для каждого member обращается к отдельному общему
`availability_by_fact` CSR, выполняет binary search по `HbkFactRef`, декодирует
`StringId` и сравнивает строку контекста. Все zero-copy-строки AV2 имели нулевые
steady allocations на borrowed iteration, поэтому этот повторный CSR/string
path является более сильной проверяемой причиной отрыва, чем heap allocation.

AV3 должен отдельно изолировать четыре формы одного и того же R1 payload:

- owner-contiguous hot AoS record с locator, kind и `u16 availability_word`;
- SoA с отдельными owner-contiguous locator/mask/kind columns;
- dense included-context bitmaps, ограниченные owner range;
- заранее сформированный ordered locator row для каждой пары
  `(AvailabilityContext, owner)`.

Это выбор общего экспериментального носителя, а не выбор R1 победителем.

## Опровергаемые гипотезы

1. `M0-mask` сократит медиану нового AV3 borrowed member-pass минимум на 20%
   против свежего R1 parent control при нулевых steady allocations и точном H0
   transcript.

2. `M2-csr` обгонит H0 steady borrowed enumeration минимум на 10% хотя бы в
   семи из девяти контекстов, потому что читает только prefiltered locator rows
   и не выполняет availability predicate в query path.

3. `M1-bitmap` обгонит H0 только для corpus-wide enumeration и не обгонит его
   для per-owner listing без слияния owner ranges с bitmap scan.

4. `M4-soa-hotcold` улучшит lookup/payload только если hot lookup не затрагивает
   локализованные cold strings/signatures до выбора итогового locator; иначе он
   повторит проигрыш payload custom formats из AV2.

5. SIMD/branchless filtering не улучшит `M0-mask` материально, пока perf не
   покажет branch misses или compare throughput главным hotspot после удаления
   range/indirection.

## Следующий замер

Нужно построить узкий набор producer/runtime-ветвей, меняющих только member
availability layout:

1. сохранить порядок H0 и каноническую форму transcript AV2;
2. реализовать AoS mask, SoA columns, bitmap и context-owner CSR под отдельными
   runtime ID;
3. измерить lookup, borrowed members, compact members и payload по locator;
4. записать artifact bytes, ready/first lookup, allocations, page faults и,
   если доступны, perf counters branches/cache misses;
5. не принимать performance строки кандидата, потерявшего H0-order parity или
   полный payload transcript.

## Уверенность и пробелы

Уверенность средне-высокая для оценок footprint и выбора проверяемых структур,
средняя — для направления изменения скорости. Первичные источники подтверждают
bitmap, CSR и columnar/hot-cold layout как подходящие инструменты, но не
доказывают победителя на этом corpus. Недостающее evidence — контролируемая
реализация, удаляющая только availability/member indirection при неизменных
producer, payload arena и lookup table.
