# S83-AV2: lookup, filtered members и полный payload

Этот документ фиксирует дополнительный неранжированный измеренный проход T183
по выбранной пользователем форме результата A:

1. storage-native/borrowed iteration без materialization;
2. request-local `Vec<Av2MemberLocator(u32)>`;
3. отдельное чтение полного payload по заранее подготовленному locator.

Документ не выбирает кандидата, не меняет ранее замороженные критерии допуска,
не разрешает merge и не назначает zero-copy-артефакт каноническим. H0 остаётся
единственным baseline, C0 — только control. Решение остаётся за пользователем.

## Контракт и происхождение

- corpus: `shcntx_ru.hbk` платформы `8.3.27.1859`, локаль `ru`,
  `40,744,845` bytes, SHA-256
  `5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48`;
- provider: schema `16`, extraction schema `11`, `204,288,000` bytes,
  SHA-256
  `55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab`;
- harness commit: `80ec7bbaf62cb2fdbce98908d48891f3064413cc`;
- manifest: `16,546,537` bytes, SHA-256
  `37588404dbf3e6d0973968f19d38696bc506072b9342eca09fa68b6dc6cc061f`;
- manifest охватывает 1,749 типов и 18,004 непосредственных members;
- 81/81 полных parity-записей, 9/9 schema/operation smoke и
  5,508/5,508 performance-записей;
- десять операций, девять `AvailabilityContext`, девять warm и девять
  cold-best-effort процессов для каждой применимой строки;
- point lookup и payload выполняют 100 corpus-wide итераций в процессе;
  borrowed iteration и compact materialization — 1,000;
- все performance-процессы выполнялись последовательно и чередовали backends
  round-robin внутри одной operation/context/stance/sample ячейки;
- фильтр использует только availability записи `TypeMember`; пустой список
  означает universal; `ModuleContextKind` отсутствует;
- members являются непосредственными записями owner, без inherited traversal,
  precedence и `effective_members`;
- ОС: Linux `6.8.0-111-generic`, `x86_64`, 8 logical CPU;
  Rust `1.95.0`;
- raw SHA-256:
  `c733603e373a82745f6a10a1a661925b3c7335dbf1868bf91f6e91d86c3581de`;
- parity SHA-256:
  `fe81c4f9715e5b562a836600044a4b25a458681671292dcbaa6d11df775f73b1`;
- smoke SHA-256:
  `1270b9161321acb8d58d51275127475b6e4abee53f30a7edace6b5bfb57d7a64`;
- JSON summary SHA-256:
  `479c33e79cd9f06f2a0bf3894581180825ef6e77e357f9ce77ba366245453ce1`;
- Markdown summary SHA-256:
  `68aec85088905535442bca2e926f1cca3708ecbeae852e439118e985ce91c66c`.

Служебные данные находятся в
`target/hbk-s83-av2/run-80ec7bb-9x/`: строгая машиночитаемая сводка —
`summary.json`, полный описательный вывод — `summary.md`, исходные записи —
`raw/measurements.jsonl`, parity — `parity/parity.jsonl`, smoke —
`preflight/smoke.jsonl`.

После 4,734 полностью записанных строк процесс-координатор был прерван между
двумя независимыми process rows. Перед продолжением все 4,734 строки были
повторно проверены как точный префикс frozen `measurement_plan`: порядок,
schema, projection, count/checksum, harness commit/hash, manifest и stderr hash
совпали. Продолжение началось с единственной следующей строки, не повторяло
измерения и сохранило те же исполняемые файлы и runtime-артефакты. Граница
зафиксирована в `resume-audit.json`, SHA-256
`bd00d6d1405ece9b2a514f296ac16c5f89e5d053aacf941d062f73ba6b62b5ea`.

## Измеренные строки и артефакты

Порядок совпадает с frozen registry и не является рангом.

| ID | Роль | Измеренный commit | Runtime-файл, bytes | SHA-256 |
| --- | --- | --- | ---: | --- |
| `S83-H0` | baseline | `80ec7bbaf62cb2fdbce98908d48891f3064413cc` | provider, 204,288,000 | `55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab` |
| `S83-C0` | control | `80ec7bbaf62cb2fdbce98908d48891f3064413cc` | provider + current cache, 204,288,000 + 11,186,057 | `55c2e099…f6f0fab` + `20fc94b58ce3307947e48f5119f27aa8253a5c82fbd8fa7c55508358d67b05b4` |
| `S83-F0` | candidate | `86628dd37a0e1e550a208df8e9ac88538ab2e771` | 11,304,567 | `20bc6ff8bf922b129233cafdcb4abbec51496697ee086788aacaf1eb00bd74b2` |
| `S83-A0` | candidate | `bc1c950210bc8a103395e0c64094288f90e18b6e` | 13,936,492 | `6fbd33ab0d58c2197e324b0b61193d873bc777def0087ae42b178cd8b53e00d1` |
| `S83-L1` | candidate | `08190c9ae0a2c56ed17b3eb0bebf9e2f8f5d0691` | 11,304,567 | `cd0bfd19ae7592232f0eafb300a3f61c356ebdadaa600573245ff2144f14bc73` |
| `S83-I1` | candidate | `950fbe9e50c5b3599ca9061b7ab8657d93c11b07` | 23,694,119 | `991b9e056c09defb8e12632cd83a709df5873b4383dbaea284c5f5dc64438c85` |
| `S83-D1` | candidate | `1e9194d9e8b864ee8cd8f73eba5a4d515d9cac9d` | 11,304,567 | `20bc6ff8bf922b129233cafdcb4abbec51496697ee086788aacaf1eb00bd74b2` |
| `S83-P1` | candidate | `e49de61a0d00731cdb17e36b63c4394b0c037a56` | 11,304,567 | `20bc6ff8bf922b129233cafdcb4abbec51496697ee086788aacaf1eb00bd74b2` |
| `S83-R1` | candidate | `477c6af0ae844ec13517c7a3f9bb02b8a1351a1c` | 12,061,887 | `7bd06fd9bd0388b1d157c3fd38374c93654084cef7193b9f637abfb3cf8702d9` |

## Полная поведенческая эквивалентность AV2

Parity сравнивает не numeric ID и не только counts. Для каждого контекста H0
создаёт ordered canonical transcript из 86,630 записей: lookup primary/alias и
misses; последовательность borrowed/compact members; universal/explicit
правило; полный payload типов, свойств, методов, callable, signatures,
parameters, requiredness, type refs, returns, template bindings, availability
и available-since. Все локальные ID нормализуются в логическую идентичность.

Все девять строк побайтно совпали с H0 во всех девяти контекстах.

| Контекст | Просмотрено | Возвращено | Universal | Explicit | Исключено | Property / Method / Event | Canonical bytes | SHA-256 canonical bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `thin_client` | 18,004 | 8,081 | 1,019 | 7,062 | 9,923 | 4,879 / 2,966 / 236 | 53,529,178 | `ea2b140352d7784cd6665c4a7a5b1776c1e8cdfbaf8ab6a1142c9018437a87f3` |
| `web_client` | 18,004 | 4,871 | 1,019 | 3,852 | 13,133 | 3,245 / 1,391 / 235 | 50,628,839 | `8f0eb54efaed06a75573ab6840f80d7b96c6db740e4511c25bce9b1e6ecfea41` |
| `mobile_client` | 18,004 | 5,587 | 1,019 | 4,568 | 12,417 | 3,427 / 2,049 / 111 | 51,235,400 | `47445f1cca41a3f43061878c50378c13e7e8ae9f664ebe2b5c1ba0f3ed406fbd` |
| `server` | 18,004 | 15,513 | 1,019 | 14,494 | 2,491 | 9,531 / 5,560 / 422 | 61,049,555 | `797ae8d32e9a75e50f27ad70cb3998ad154a9e4bc935f4cc6e0bca1da3be55ad` |
| `thick_client` | 18,004 | 17,308 | 1,019 | 16,289 | 696 | 10,349 / 6,351 / 608 | 62,571,496 | `6b0785056e120841abc0bda0fc53a491be47ab0ee2f21445d094d16af1f035c4` |
| `external_connection` | 18,004 | 13,222 | 1,019 | 12,203 | 4,782 | 7,897 / 5,129 / 196 | 58,748,719 | `d6db000de1969a33e79d46387f3ed245f5dcd44220383530d47aff004eb0dc3b` |
| `mobile_application_client` | 18,004 | 5,279 | 1,019 | 4,260 | 12,725 | 3,258 / 1,921 / 100 | 50,908,456 | `e3970dafb1f747a6aa1dc605e190817ae5bbb46f52e5b88fb79c62fa3f18cfea` |
| `mobile_application_server` | 18,004 | 8,134 | 1,019 | 7,115 | 9,870 | 5,013 / 2,991 / 130 | 53,732,354 | `985a94b88dce018ad967ad7289c3f890ecf2bcd4cce70e080aa3c5245e837ee9` |
| `mobile_standalone_server` | 18,004 | 8,095 | 1,019 | 7,076 | 9,909 | 4,996 / 2,968 / 131 | 53,691,665 | `4007fbe9268afd49ee295b950dc10b3eadd396776237a94a793659aaf863811f` |

Enum values в текущем corpus отсутствуют. Порядок compact set совпадает с
borrowed iteration и H0; locator имеет ровно четыре bytes.

## Основной результат: steady filtered members

В таблице показана медиана из девяти отдельных контекстных median времени
одного полного corpus-pass. В квадратных скобках — диапазон отношения к H0 по
всем девяти контекстам. Значение меньше `1.0×` быстрее H0, больше `1.0×`
медленнее. Это описательная сводка, не score и не правило выбора.

| ID | Borrowed warm, ms [H0 range] | Borrowed cold, ms [H0 range] | Compact warm, ms [H0 range] | Compact cold, ms [H0 range] |
| --- | ---: | ---: | ---: | ---: |
| `S83-H0` | 1.361 [1.000–1.000×] | 1.355 [1.000–1.000×] | 1.403 [1.000–1.000×] | 1.391 [1.000–1.000×] |
| `S83-C0` | 1.363 [0.991–1.008×] | 1.363 [0.984–1.006×] | 1.389 [0.990–1.008×] | 1.397 [0.989–1.007×] |
| `S83-F0` | 4.923 [2.905–3.872×] | 4.928 [2.919–3.876×] | 4.873 [2.773–3.782×] | 4.938 [2.792–3.780×] |
| `S83-A0` | 6.406 [4.588–4.788×] | 6.397 [4.596–4.767×] | 6.433 [4.500–4.718×] | 6.422 [4.487–4.720×] |
| `S83-L1` | 4.924 [2.990–3.913×] | 4.893 [2.964–3.977×] | 4.876 [2.828–3.769×] | 4.870 [2.854–3.775×] |
| `S83-I1` | 4.814 [2.898–3.793×] | 4.840 [2.921–3.794×] | 4.787 [2.792–3.640×] | 4.792 [2.814–3.684×] |
| `S83-D1` | 4.912 [2.893–3.963×] | 4.894 [2.916–3.997×] | 4.895 [2.774–3.677×] | 4.812 [2.804–3.768×] |
| `S83-P1` | 4.920 [2.946–3.880×] | 4.883 [2.971–3.831×] | 4.902 [2.776–3.660×] | 4.835 [2.836–3.737×] |
| `S83-R1` | 4.942 [2.954–3.885×] | 4.951 [2.978–3.868×] | 4.875 [2.823–3.700×] | 4.869 [2.845–3.744×] |

Borrowed steady не аллоцирует ни в одной строке. Compact steady во всех строках
выполняет одинаковые 1,651,000 allocation calls / 72,016,000 bytes на batch из
1,000 проходов: различие времени не объясняется разной формой результата.

Ни один текущий zero-copy-кандидат не улучшил H0/C0 на основной операции AV2.
Это не выбирает H0 как будущий формат: H0 остаётся только baseline, а факт
ограничивает продвижение именно измеренных zero-copy-структур без новой
гипотезы горячего availability/member layout.

## Steady point lookup

Таблица показывает warm median nanoseconds на один логический запрос и
отношение к H0. Все primary, distinct alias и фиксированные misses входят в
workload.

| ID | Type by name | Property owner/name/kind | Method owner/name/kind | Callable owner/name |
| --- | ---: | ---: | ---: | ---: |
| `S83-H0` | 862 ns / 1.00× | 550 ns / 1.00× | 546 ns / 1.00× | 504 ns / 1.00× |
| `S83-C0` | 861 ns / 1.00× | 539 ns / 0.98× | 536 ns / 0.98× | 493 ns / 0.98× |
| `S83-F0` | 1,499 ns / 1.74× | 937 ns / 1.70× | 924 ns / 1.69× | 854 ns / 1.69× |
| `S83-A0` | 2,590 ns / 3.00× | 1,026 ns / 1.87× | 1,175 ns / 2.15× | 1,082 ns / 2.15× |
| `S83-L1` | 1,453 ns / 1.69× | 916 ns / 1.67× | 900 ns / 1.65× | 829 ns / 1.64× |
| `S83-I1` | 799 ns / 0.93× | 544 ns / 0.99× | 508 ns / 0.93× | 509 ns / 1.01× |
| `S83-D1` | 1,474 ns / 1.71× | 941 ns / 1.71× | 924 ns / 1.69× | 857 ns / 1.70× |
| `S83-P1` | 1,490 ns / 1.73× | 928 ns / 1.69× | 909 ns / 1.66× | 859 ns / 1.70× |
| `S83-R1` | 1,472 ns / 1.71× | 1,067 ns / 1.94× | 1,057 ns / 1.94× | 847 ns / 1.68× |

Cold-best-effort отношения дают тот же качественный профиль. Все 612 steady
time rows имеют `MAD / median <= 5%`. Отдельные first-operation значения малы
и 90/612 first rows отмечены как noisy; их нельзя использовать как единственный
критерий. Исключение по масштабу — A0: первый lookup четырёх фиксированных
anchors стабильно занимает 4.77–4.90 ms, тогда как H0 — 3.80–5.09 µs.

## Полный payload

Payload читается по заранее подготовленному locator: lookup, фильтрация и
формирование compact set в эти интервалы не входят. Таблица показывает warm
median ns на один объект и выделенные MB на один corpus-pass.

| ID | Type payload | Method payload | Property payload |
| --- | ---: | ---: | ---: |
| `S83-H0` | 424 ns / 1.00× / 0 MB | 1,074 ns / 1.00× / 0 MB | 710 ns / 1.00× / 0 MB |
| `S83-C0` | 419 ns / 0.99× / 0 MB | 916 ns / 0.85× / 0 MB | 615 ns / 0.87× / 0 MB |
| `S83-F0` | 1,017 ns / 2.40× / 0.03 MB | 2,026 ns / 1.89× / 3.30 MB | 1,474 ns / 2.08× / 1.27 MB |
| `S83-A0` | 3,201 ns / 7.55× / 0 MB | 1,239 ns / 1.15× / 0 MB | 1,053 ns / 1.48× / 0 MB |
| `S83-L1` | 1,076 ns / 2.54× / 0.03 MB | 2,066 ns / 1.92× / 3.30 MB | 1,512 ns / 2.13× / 1.27 MB |
| `S83-I1` | 1,018 ns / 2.40× / 0.03 MB | 2,001 ns / 1.86× / 3.30 MB | 1,500 ns / 2.11× / 1.27 MB |
| `S83-D1` | 1,023 ns / 2.41× / 0.03 MB | 2,024 ns / 1.88× / 3.30 MB | 1,480 ns / 2.08× / 1.27 MB |
| `S83-P1` | 1,018 ns / 2.40× / 0.03 MB | 1,996 ns / 1.86× / 3.30 MB | 1,471 ns / 2.07× / 1.27 MB |
| `S83-R1` | 975 ns / 2.30× / 0 MB | 1,837 ns / 1.71× / 0 MB | 1,328 ns / 1.87× / 0 MB |

Для `filtered_members_payload` медиана по девяти контекстам составляет:

| ID | ns/object, median [range] | H0 ratio, median [range] | Allocated MB/corpus-pass |
| --- | ---: | ---: | ---: |
| `S83-H0` | 600 [554–636] | 1.00 [1.00–1.00×] | 0 |
| `S83-C0` | 558 [527–567] | 0.93 [0.89–0.96×] | 0 |
| `S83-F0` | 1,396 [1,366–1,420] | 2.33 [2.20–2.53×] | 0.81 |
| `S83-A0` | 1,164 [828–1,567] | 1.94 [1.31–2.83×] | 0 |
| `S83-L1` | 1,435 [1,400–1,474] | 2.43 [2.24–2.56×] | 0.81 |
| `S83-I1` | 1,378 [1,356–1,404] | 2.34 [2.19–2.45×] | 0.81 |
| `S83-D1` | 1,386 [1,382–1,422] | 2.37 [2.19–2.50×] | 0.81 |
| `S83-P1` | 1,364 [1,341–1,394] | 2.32 [2.17–2.44×] | 0.81 |
| `S83-R1` | 1,238 [1,216–1,258] | 2.10 [1.95–2.21×] | 0 |

A0 и R1 подтверждают borrowed payload без steady allocation, но это не
превращается в меньшую CPU latency относительно H0/C0 на измеренном corpus.

## Startup и память формы A

`entry_to_ready` ниже — медиана 34 operation/context-медиан для строки; каждая
из этих медиан построена по девяти отдельным процессам. Artifact MiB для H0 —
размер SQLite; для C0 — SQLite + current cache; для кандидатов — единственный
runtime snapshot.

| ID | Artifact MiB | Ready warm, ms | Ready cold-best-effort, ms |
| --- | ---: | ---: | ---: |
| `S83-H0` | 194.8 | 624.2 | 1,864.6 |
| `S83-C0` | 205.5 | 41.1 | 69.3 |
| `S83-F0` | 10.8 | 55.0 | 65.9 |
| `S83-A0` | 13.3 | 40.4 | 49.7 |
| `S83-L1` | 10.8 | 54.9 | 62.3 |
| `S83-I1` | 22.6 | 270.0 | 287.0 |
| `S83-D1` | 10.8 | 40.0 | 47.6 |
| `S83-P1` | 10.8 | 54.9 | 61.6 |
| `S83-R1` | 11.5 | 52.2 | 59.5 |

Для compact memory sample `thin_client` warm одновременно удерживается один
`Vec<Av2MemberLocator>` на каждый owner. Форма набора одинакова, поэтому у всех
строк `logical = 31.6 KiB`, `capacity = 70.3 KiB`, container overhead =
`41.0 KiB`, live delta = `111.3 KiB`.

| ID | Process RSS, MiB | Process PSS, MiB | Private, MiB | Compact live delta, KiB |
| --- | ---: | ---: | ---: | ---: |
| `S83-H0` | 73.1 | 71.2 | 71.2 | 111.3 |
| `S83-C0` | 54.9 | 53.0 | 52.9 | 111.3 |
| `S83-F0` | 44.4 | 43.0 | 43.0 | 111.3 |
| `S83-A0` | 46.9 | 45.4 | 45.4 | 111.3 |
| `S83-L1` | 44.4 | 43.0 | 43.0 | 111.3 |
| `S83-I1` | 56.3 | 54.9 | 54.9 | 111.3 |
| `S83-D1` | 44.4 | 43.0 | 43.0 | 111.3 |
| `S83-P1` | 44.4 | 43.0 | 43.0 | 111.3 |
| `S83-R1` | 45.1 | 43.7 | 43.7 | 111.3 |

RSS/PSS/private включают загруженный snapshot, prepared manifest и состояние
страниц процесса. Эти значения полезны как process-boundary evidence, но не
как точная стоимость отдельного nanosecond lookup. `cold-best-effort` также не
является cold boot ОС.

## Интерпретация без выбора

- AV2 полностью подтвердил семантику формы A и показал, что compact
  materialization добавляет одинаковую request-local стоимость результата во
  всех строках.
- Текущие zero-copy-кандидаты существенно уменьшают startup относительно H0 и
  снижают process PSS в данном процессе.
- I1 демонстрирует отдельный эффект отображённого open-address индекса на
  steady point lookup, но его основной filtered member pass остаётся заметно
  медленнее H0/C0, а ready требует проверки более крупного артефакта.
- F0/L1/I1/D1/P1/R1 имеют близкий профиль основной enumeration; изменение
  page layout, индекса, момента проверки или writer само по себе не устраняет
  CPU-стоимость прохода member/availability.
- Во всех borrowed-pass отсутствуют steady allocations, поэтому наблюдаемый
  проигрыш не является heap-эффектом. Аудит горячего пути показывает прямые
  borrowed slices members/availability у H0 и дополнительные view/CSR/archive
  indirection у кандидатов. Какая именно часть indirection доминирует, этим
  прогоном не изолирована и остаётся гипотезой для следующего эксперимента.
- A0 предоставляет archived views и отсутствие payload allocation, но имеет
  отдельную высокую стоимость первого lookup и самый высокий CPU time основной
  enumeration среди измеренных строк.
- AV2 не даёт основания объявить один из текущих zero-copy snapshots основным
  каноническим артефактом. Для этого нужна новая гипотеза горячего
  member/availability представления либо явное пользовательское решение о
  допустимом компромиссе startup/memory против lookup/enumeration CPU.

`selection = pending-user-decision`. Ни F0, ни A0, ни L1, ни I1, ни D1, ни P1,
ни R1 не назначены первым, рекомендованным или каноническим вариантом. Основной
owned-путь остаётся каноническим до явного решения пользователя и отдельного
долговечного архитектурного решения.
