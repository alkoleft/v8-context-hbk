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

## Коррекция consumer workload после сверки с `v8-context`

Предварительный AV3 ошибочно сделал основной member-нагрузкой суммарный обход
members всех 1,749 типов. Такой запрос удобен как стресс-диагностика layout, но
не соответствует основному потребителю и не должен участвовать в решении.
Решающий workload S83-AV4 проверяет только:

1. формирование platform global scope из 601 глобального BSL-факта с
   фильтром по одному `AvailabilityContext`;
2. точечный lookup одного platform type;
3. формирование candidate scope только для этого найденного типа: его
   непосредственные properties/methods, отфильтрованные по
   `AvailabilityContext`;
4. отдельное чтение полного payload уже выбранного type/member/callable
   locator.

Дальнейший lookup по сформированному scope, неоднозначность, precedence и
effective selection принадлежат `v8-context`. В рамках T183 термин
`inherited_members` не означает транзитивное раскрытие других типов: это тот же
provider-owned ordered поток candidate members одного найденного типа с
сохранением owner/provenance. Он не выбирает, какое одноимённое объявление
«побеждает».

Module events не входят в AV4: они требуют `ModuleContextKind`, который не
является availability-фильтром. Их добавление к module context остаётся
downstream-операцией и не влияет на сравнение hot layout global/type facts.

H0 называется SQL baseline по происхождению startup, но его steady path уже
работает над владеющим Rust snapshot в памяти. Поэтому AV4 пытается обогнать не
SQLite VM в каждом запросе, а очень сильный direct owned-memory baseline.

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

7. Co-location должна проверяться как конкретный физический layout, а не как
   общее обещание локальности.

   Arrow прямо связывает последовательный доступ с adjacency, задаёт
   relocatable offsets без pointer swizzling и рекомендует выравнивать hot
   buffers на 8 или 64 bytes:
   [Arrow columnar format](https://arrow.apache.org/docs/format/Columnar.html).
   SQLite показывает, почему baseline силён: covering index может ответить без
   чтения исходной table row:
   [SQLite query planner](https://www.sqlite.org/queryplanner.html).
   Следовательно, HBK-кандидату недостаточно быть mmap-backed — он должен
   свести запрос к меньшему числу прямых последовательных чтений, чем H0.

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

7. Предложение хранить type и его members рядом полезно, если выразить его как
   fixed head плюс диапазон, а не как variable-size interleaved block.

   В S83 распределение непосредственных members на тип: 1,749 типов, 98 типов
   без members, median 6, p90 23, p99 62, maximum 295, average 10.29. Эти
   значения относятся к исходному непосредственному member domain, по которому
   выбраны anchors. У p90-anchor три из 23 записей имеют kind `Event`, поэтому
   AV4 scope после обязательного ограничения до `Property`/`Method` содержит
   20 записей; именно это число хранит AV4 manifest и проверяет parity. Поэтому
   отдельный binary search owner-range особенно заметен на обычных коротких
   диапазонах. AV4 проверяет следующий relocatable layout:

   ```text
   TypeHot[]:   ... member_start:u32, member_count:u32 ...
                         │
                         └──────┐
   MemberHot[]: [ owner-major contiguous member records/locators ]
   ColdArena[]: [ names, signatures, type refs, provenance, availability payload ]
   ```

   `TypeHot` и `MemberHot` являются соседними hot sections, но не чередуются
   блоками `[Type][variable Members]`: фиксированный массив type heads сохраняет
   O(1) доступ по locator, простую проверку bounds/alignment и стабильный stride.
   Встроенные `member_start/member_count` стоят не более `1,749 * 8 = 13,992`
   bytes до padding и устраняют отдельный owner-key search. Полный cold payload
   читается только после выбора locator.

8. Global scope достаточно мал, чтобы честно проверить все четыре формы без
   compressed bitmap.

   В S83 есть 500 глобальных методов и 101 глобальное свойство. Количество
   возвращаемых объектов по контекстам `thin_client`, `web_client`,
   `mobile_client`, `server`, `thick_client`, `external_connection`,
   `mobile_application_client`, `mobile_application_server`,
   `mobile_standalone_server` равно соответственно 361, 314, 354, 427, 567,
   410, 341, 308 и 312; суммарно 3,394 locator, universal — 1. Оценка hot
   footprint:

   | Layout | Global hot bytes до headers/alignment |
   | --- | ---: |
   | AoS `u32 locator + u16 mask + u8 kind + pad` | 4,808 |
   | SoA `u32[] + u16[] + u8[]` | 4,207 |
   | 9 context bitmaps + universal bitmap | 800 |
   | CSR `context -> ordered u32 locator row` | 13,616 |

   Для 601 плотного ID-domain обычный bitmap из десяти `u64` на набор проще
   Roaring и не требует container metadata. CSR тратит больше bytes, но в
   query path читает только возвращаемые locator и не выполняет predicate.

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

Global facts также не хранят availability inline: H0 получает его через
`availability_by_fact(HbkFactRef::Global)`. Поэтому global scope допускает то
же причинное сравнение mask/bitmap/CSR, но на отдельном плотном domain из 601
элемента.

## Предварительный AV3

AV3 построил четыре формы одного и того же R1 member payload:

- owner-contiguous hot AoS record с locator, kind и `u16 availability_word`;
- SoA с отдельными owner-contiguous locator/mask/kind columns;
- dense included-context bitmaps, ограниченные owner range;
- заранее сформированный ordered locator row для каждой пары
  `(AvailabilityContext, owner)`.

Его corpus-wide member operation сохраняется только как недецизионная
`all_types_member_stress_diagnostic`. Предварительные AV3 artifacts и smoke не
дают основания для выбора и не заменяют AV4.

## Опровергаемые гипотезы S83-AV4

AV4 сохраняет четыре R1-derived layout-варианта и добавляет не участвующий в
выборе causal control прямого диапазона в type head:

| ID | Файл/память | Проверяемая причина |
| --- | --- | --- |
| `R1-DIRECT` | исходный R1 payload, `member_start/count` в `TypeHot` | цена отдельного owner-range lookup |
| `R2-AOS` | owner-major `MemberHot { locator, mask, kind }` | одно последовательное чтение записи на member |
| `R2-SOA` | отдельные locator/mask/kind columns | меньше hot bytes и узкие последовательные mask reads |
| `R2-BITSET` | dense bitmap на context, ограниченный type range | битовый параллелизм без дублирования locator rows |
| `R2-CSR` | direct ordered row `(context, type) -> locators` | отсутствие predicate ценой дополнительных bytes |

Те же четыре физические формы отдельно применяются к global scope. Это
layout-гипотезы поверх R1 и не возвращает исключённые F0/L1/D1 в shortlist.
A0/I1/P1/R1 остаются четырьмя активными cache-кандидатами; I1 остаётся
lookup-reference. Никакой layout не получает ранг или первое место без решения
пользователя.

Проверяемые ожидания:

1. `R1-DIRECT` уменьшит steady время `type_scope_borrowed` для median/p90 типов
   без изменения filtered member loop; это изолирует эффект `start/count`.
2. `R2-AOS` или `R2-SOA` обгонит H0 на коротких type ranges, если один `u16`
   predicate и последовательная загрузка дешевле owned H0 access path.
3. `R2-BITSET` выиграет на global scope и типах p99/max, но может проиграть AoS
   на median type из-за маскирования boundary words и декодирования битов.
4. `R2-CSR` выиграет на `global_scope_borrowed` и больших type scopes, если
   уменьшение `physical_entries_examined` окупит чтение большего артефакта;
   для median type его дополнительный index footprint может не окупиться.
5. Hot/cold co-location считается подтверждённой только если одновременно
   уменьшаются steady time и physical hot bytes touched, а full-payload access
   не получает материальной регрессии.
6. SIMD/auto-vectorization не входят в первый AV4 implementation. Их можно
   добавить отдельной
   производной веткой только если scalar SoA/mask остаётся bottleneck и доступны
   сравнимые hardware-counter или disassembly evidence.

Решающий workload не перечисляет members всех разных типов. Он использует
фиксированные типы с 0/median/p90/p99/max диапазонами (`COMОбъект`,
`ЗначенияПараметровВыводаГруппировкиТаблицыКомпоновкиДанных`,
`ПоследовательностьНаборЗаписей.<Имя последовательности>`,
`ОбъектМетаданных: ПланВидовРасчета`, `БиблиотекаКартинок`) и все девять
`AvailabilityContext`. Каждый anchor/context образует отдельную measurement-
строку. Значения разных типов не агрегируются в один score или «средний type».

## Воспроизводимость corpus facts AV4

Counts выше получены только из frozen provider
`target/snapshot-materialization/shcntx_ru.8.3.27.1859.schema16.release.sqlite`
с SHA-256
`55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab`:

```bash
sqlite3 -readonly PROVIDER \
  "SELECT kind, COUNT(*) FROM documents
   WHERE kind IN ('global_method','global_property') GROUP BY kind;"

sqlite3 -readonly PROVIDER \
  "WITH contexts(code) AS (VALUES
     ('thin_client'),('web_client'),('mobile_client'),('server'),
     ('thick_client'),('external_connection'),
     ('mobile_application_client'),('mobile_application_server'),
     ('mobile_standalone_server'))
   SELECT code, SUM(CASE WHEN d.availability_contexts='' OR
     instr(char(10)||d.availability_contexts||char(10),
           char(10)||code||char(10))>0 THEN 1 ELSE 0 END)
   FROM contexts CROSS JOIN documents d
   WHERE d.kind IN ('global_method','global_property') GROUP BY code;"

jq '[.types[].member_count] | sort
    | {count:length,zero:map(select(.==0))|length,
       median:.[length/2|floor],p90:.[length*90/100|floor],
       p99:.[length*99/100|floor],max:.[-1],sum:add,avg:(add/length)}' \
  target/hbk-s83-av3/query-manifest.json

jq '[.types[] | select(.primary=="COMОбъект" or
      .primary=="ЗначенияПараметровВыводаГруппировкиТаблицыКомпоновкиДанных" or
      .primary=="ПоследовательностьНаборЗаписей.<Имя последовательности>" or
      .primary=="ОбъектМетаданных: ПланВидовРасчета" or
      .primary=="БиблиотекаКартинок")
      | {logical_id,primary,member_count}]' \
  target/hbk-s83-av3/query-manifest.json
```

Здесь `PROVIDER` заменяется указанным точным путём. AV4 manifest до любого
performance run обязан повторить эти counts, записать logical ID и member count
каждого anchor и canonical checksum каждой global/type context row. Числовые
locator являются session-local и поэтому не сохраняются как долговечная
идентичность; manifest связывает locator с logical ID внутри конкретного run.

## Уверенность и пробелы

Уверенность средне-высокая для оценок footprint и выбора проверяемых структур,
средняя — для направления изменения скорости. Первичные источники подтверждают
bitmap, CSR и columnar/hot-cold layout как подходящие инструменты, но не
доказывают победителя на этом corpus.

Недостающее evidence — AV4 harness, точный parity global/type scopes и
последовательные измерения отдельных ветвей. Linux perf counters на текущем
хосте недоступны из-за `kernel.perf_event_paranoid=4`; это фиксируется как
evidence gap, а не устраняется изменением sysctl. Обязательными остаются wall
time, allocation calls/bytes, faults, RSS/PSS, artifact bytes и
`logical_domain_count`/`physical_entries_examined`/`returned_count`.
