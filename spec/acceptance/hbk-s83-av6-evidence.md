# Неранжированные свидетельства S83-AV6

## Статус

S83-AV6 завершён как отдельная проверка составных фильтров
`AvailabilityContext` и гипотезы сохраняемых базовых проекций. Сравниваются
ровно три backend:

- `S83-H0` — SQL-to-owned baseline;
- `S83-X1` — неизменённый X1, который один раз сканирует общий
  availability-word;
- `S83-X1-PROJECTED` — X1 с девятью сохраняемыми поконтекстными ordered
  projections для global scope и непосредственных members всех platform
  types.

Результат не присваивает score/rank, не выбирает победителя, не разрешает
merge и не назначает canonical runtime. `selection = pending-user-decision`;
пункты 1.15 и T183 остаются открытыми.

## Воспроизводимость

Чистый последовательный run имеет идентичность
`s83-av6-final-sequential-2026-08-03`. Performance начался только после exact
parity и smoke обоих профилей; процессы выполнялись последовательно
round-robin. Время и process memory взяты только из counters-disabled timing
binary, allocations — только из отдельного counters-enabled binary.

| Объект | Значение |
| --- | --- |
| workload | `s83-av6-multi-context-scope/v1` |
| measurement harness commit | `2298c74d7151fc54047b9c3567168dc71d7782ab` |
| summary-only correction | `810f674696089dd9ba7c7d3e48e3993430daef31`; undefined `ns_per_object` пустых scopes исключён из агрегации, raw не менялся |
| HBK | `40,744,845` bytes; `5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48` |
| platform / locale | `8.3.27.1859` / `ru` |
| provider | schema `16`, extraction `11`, `220,270,592` bytes; `f626207a93f99eecbb6c76fb13482058e32c4c4d404c84bb33b45abde45233bc` |
| AV6 manifest | `6,856` bytes; `71e06fd7b4fdad58cc4b3b496ab3553be93d71ff815b052ade12a70194804335` |
| orchestration | `8,588` bytes; `67a29645561fa14c734eafaf286f0266403c45d7418a26a5dde7e124c11ed057` |
| timing raw | `1,296/1,296` строк; schema `hbk-s83-av6-raw/v1` |
| allocation raw | `432/432` строки; schema `hbk-s83-av6-allocation-raw/v1` |
| raw SHA-256 | `9396b912f933b2f9bfd1b6b411ae72cf5c5c681f17c5266adbde2f99eaf2ea40` |
| timing resource | `420/420` строк; schema `hbk-s83-av6-resource-raw/v1` |
| allocation resource | `252/252` строки; schema `hbk-s83-av6-allocation-resource-raw/v1` |
| resource SHA-256 | `870e035b5b1c816c93d48efc1116d7c947f61823b72e45d27abc39ad420cecfe` |
| parity / smoke | `12/12 pass`; `6/6 pass` |
| parity SHA-256 | `c9d7b120bfa18f54941743616457a16b4ca48b9b561c01edc687f8b411e18e3f` |
| summary JSON | `54f39c79fb528276c3c1f06257d1c3a9052948f63ec23b4585e2e9ed9293e356` |
| summary Markdown | `ed060bb5ddb9a45232fbc1e1c03a521b02e7f27c16df6ed91eebf5d7ea8f2a41` |

Финальные raw/parity/summary находятся в
`../v8-context-hbk-wt-86-av6-multicontext/target/hbk-s83-av6-frozen/run-3/`,
manifest и orchestration — на один уровень выше. Неудачные preflight/run до
финальной заморозки сохранены отдельно и не входят в evidence.

## Registry и проекционный артефакт

| Backend | Commit | Artifact bytes | Artifact SHA-256 |
| --- | --- | ---: | --- |
| `S83-H0` | `2298c74d7151fc54047b9c3567168dc71d7782ab` | 220,270,592 | `f626207a93f99eecbb6c76fb13482058e32c4c4d404c84bb33b45abde45233bc` |
| `S83-X1` | `c4dec51648ed3ffb57e79e0492ae11ef7e3cd5d0` | 19,033,826 | `933a5cf4649200dea15f7b15160d4bc50eedf98de242b7ba164e7c4410acdf17` |
| `S83-X1-PROJECTED` | `08c2c9df86fce12adeedf5f398a360630f554693` | 19,446,247 | `ee7b0ecd642bc7fd04eaa55122fe2fbf8c0b53bb87e7b88aefe9c45b623a0487` |

X1-PROJECTED имеет отдельную identity `HBKFXP`, layout
`fixed-head-range-x1-global-soa-type-hash-member-aos-projected-base-context-rows-v1`.
В артефакте находятся девять context IDs, 10 global offsets, 3,394 global
locators, `9 * 1,749 + 1 = 15,742` member offsets и 83,921 member locators.
Encoded projected sections занимают `412,317` bytes; полный артефакт больше X1
на `412,421` bytes (`2.167%`). Готовых benchmark-комбинаций и таблицы 512
масок нет.

## Поведенческая эквивалентность

Все 12 backend/selector записей прошли exact parity до performance. Для
каждого из четырёх selectors сравнивались ordered global scope и пять type
scopes, причём как borrowed stream, так и реально материализованный compact
set. Session-local `u32` locators перед сравнением разрешались в
`{kind, logical_id}`; числовые ID между сессиями не сравнивались.

| Selector | Global rows | Type rows zero / median / p90 / p99 / max |
| --- | ---: | --- |
| `server_thick_client / ANY` | 571 | 0 / 6 / 20 / 62 / 295 |
| `server_thick_client / ALL` | 423 | 0 / 6 / 20 / 62 / 295 |
| `thin_web_thick_client / ANY` | 570 | 0 / 6 / 20 / 62 / 295 |
| `thin_web_thick_client / ALL` | 313 | 0 / 6 / 0 / 0 / 295 |

Пустой availability действительно обрабатывается как universal, `ANY` и
`ALL` не создают дубликатов и сохраняют порядок H0. `ModuleContextKind` не
участвует.

## Steady global scope

Время — `us` на один полный scope. Значения — медиана девяти timing samples;
отношение меньше `1.0x` означает меньшее время относительно H0.

| Selector | H0 borrowed / collect | X1 borrowed / collect | X1-PROJECTED borrowed / collect | X1/H0 borrowed / collect | PROJECTED/H0 borrowed / collect |
| --- | ---: | ---: | ---: | ---: | ---: |
| `server_thick_client / ANY` | 39.572 / 41.519 | 9.030 / 9.994 | 21.168 / 21.512 | 0.228 / 0.241x | 0.535 / 0.518x |
| `server_thick_client / ALL` | 39.937 / 40.278 | 9.494 / 9.842 | 19.921 / 20.469 | 0.238 / 0.244x | 0.499 / 0.508x |
| `thin_web_thick_client / ANY` | 40.771 / 42.650 | 9.161 / 9.505 | 27.286 / 27.582 | 0.225 / 0.223x | 0.669 / 0.647x |
| `thin_web_thick_client / ALL` | 40.934 / 41.430 | 9.365 / 9.760 | 20.499 / 21.036 | 0.229 / 0.236x | 0.501 / 0.508x |

Неизменённый X1 сканирует 601 physical inputs. Проекционный merge получает
994 входных locator entries для двух контекстов и 1,242 для трёх; трёхконтекстный
`ALL` успевает потребить 1,232. Поэтому X1-PROJECTED сохраняет заметное
улучшение против H0, но требует `2.080-2.902x` времени X1 для collect и
`2.098-2.978x` для borrowed. В этом корпусе сохранённые списки не компенсируют
цену нескольких dense input streams.

## Steady members одного type

Время — `ns` на один scope; в ячейке показано `H0 / X1 / X1-PROJECTED`.
`Rows` — результат одного запроса. Это непосредственные `Property`/`Method`
members одного уже найденного type, отфильтрованные только по
`AvailabilityContext`.

| Selector | Anchor | Rows | Borrowed, ns | Collect, ns |
| --- | --- | ---: | ---: | ---: |
| `server_thick_client / ANY` | zero | 0 | 63 / 60 / 124 | 64 / 60 / 122 |
|  | median | 6 | 107 / 156 / 340 | 126 / 161 / 351 |
|  | p90 | 20 | 231 / 364 / 816 | 242 / 387 / 848 |
|  | p99 | 62 | 484 / 919 / 2,214 | 551 / 983 / 2,219 |
|  | max | 295 | 1,348 / 4,106 / 9,992 | 1,709 / 4,324 / 10,280 |
| `server_thick_client / ALL` | zero | 0 | 63 / 62 / 120 | 64 / 62 / 130 |
|  | median | 6 | 118 / 144 / 382 | 131 / 161 / 396 |
|  | p90 | 20 | 237 / 365 / 981 | 263 / 400 / 999 |
|  | p99 | 62 | 523 / 928 / 2,712 | 587 / 966 / 2,735 |
|  | max | 295 | 1,343 / 4,140 / 12,499 | 1,663 / 4,282 / 12,399 |
| `thin_web_thick_client / ANY` | zero | 0 | 63 / 61 / 131 | 68 / 64 / 132 |
|  | median | 6 | 110 / 145 / 402 | 127 / 157 / 413 |
|  | p90 | 20 | 340 / 383 / 791 | 360 / 390 / 806 |
|  | p99 | 62 | 813 / 922 / 2,212 | 823 / 958 / 2,198 |
|  | max | 295 | 1,343 / 4,060 / 12,738 | 1,654 / 4,423 / 12,917 |
| `thin_web_thick_client / ALL` | zero | 0 | 64 / 61 / 125 | 63 / 65 / 129 |
|  | median | 6 | 119 / 155 / 463 | 141 / 160 / 478 |
|  | p90 | 0 | 197 / 305 / 127 | 212 / 325 / 144 |
|  | p99 | 0 | 458 / 737 / 131 | 470 / 758 / 150 |
|  | max | 295 | 1,308 / 4,122 / 16,140 | 1,676 / 4,287 / 16,523 |

Для непустых scopes X1-PROJECTED требует `2.326-12.339x` времени H0 и
`2.065-3.916x` времени X1 на borrowed; collect — соответственно
`2.239-9.859x` и `2.067-3.854x`. Причина видна в physical inputs: для dense
двухконтекстных строк проекция читает `2 * owner_count`, для трёхконтекстных
median/max — `3 * owner_count`, тогда как H0/X1 читают owner range один раз.

Есть отдельный селективный случай, который нельзя смешивать с непустыми
scopes. Для `thin_web_thick_client / ALL` p90 и p99 одна из базовых проекций
пуста, intersection завершается без потребления locator entries. Здесь
X1-PROJECTED borrowed равен `0.645x` и `0.286x` H0 (`0.416x` и `0.178x` X1),
а collect — `0.679x` и `0.319x` H0 (`0.443x` и `0.198x` X1). Сохраняемые
проекции подтверждают сильный сигнал только для раннего пустого/очень
селективного `ALL`, но не для `ANY` или dense непустых scopes.

## Аллокации materialization

Borrowed steady paths всех трёх backend имеют ноль allocation/reallocation
calls и ноль allocated bytes. Collect не выполняет realloc и использует не
более одной allocation на запрос:

| Scope | H0 / X1 bytes per query | X1-PROJECTED bytes per query | Calls per non-zero-capacity query |
| --- | ---: | ---: | ---: |
| global | 2,404 | 2,404 | 1 |
| type zero | 0 | 0 | 0 |
| type median | 24 | 24 | 1 |
| type p90 | 92 | 80 | 1 |
| type p99 | 248 | 248 | 1 |
| type max | 1,180 | 1,180 | 1 |
| retained global + five type sets | 4,092 | 4,080 | 6 |

Эти bytes отражают зарезервированную capacity compact locator set, а не число
фактически возвращённых строк. Поэтому пустой результат p90/p99 в
трёхконтекстном `ALL` всё ещё может иметь allocation: все backend следуют
одной консервативной reserve policy. Времена и RSS/PSS allocation binary не
используются как timing evidence.

## Startup, first scopes и память

Ready/PSS/RSS — медиана всех 140 однородных timing resource observations на
backend. First-operation строки имеют по пять samples и публикуются отдельно.

| Backend | Entry-to-ready, ms | PSS, KiB | RSS, KiB | Artifact, bytes | Hot sections, bytes | Projected sections, bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `S83-H0` | 723.073 | 92,506 | 94,388 | 220,270,592 | 0 | 0 |
| `S83-X1` | 83.072 | 19,611 | 21,076 | 19,033,826 | 293,303 | 0 |
| `S83-X1-PROJECTED` | 82.979 | 19,919 | 21,384 | 19,446,247 | 705,580 | 412,317 |

Относительно X1 проекционный артефакт добавляет `2.167%`, retained PSS —
`308 KiB` (`1.571%`), RSS — `308 KiB` (`1.461%`). Разница ready time X1 и
X1-PROJECTED менее `0.1 ms` и не трактуется как отдельный выигрыш. Оба mapped
варианта имеют около `0.115x` entry-to-ready H0.

| Selector | First global, us H0 / X1 / PROJECTED | Retained scopes, us H0 / X1 / PROJECTED |
| --- | ---: | ---: |
| `server_thick_client / ANY` | 243.850 / 10.643 / 24.728 | 247.960 / 17.885 / 45.460 |
| `server_thick_client / ALL` | 252.066 / 11.074 / 23.463 | 258.621 / 18.542 / 46.161 |
| `thin_web_thick_client / ANY` | 237.677 / 10.360 / 31.531 | 242.690 / 17.629 / 54.085 |
| `thin_web_thick_client / ALL` | 250.666 / 11.427 / 25.284 | 257.580 / 18.699 / 47.738 |

Для навигации медиана first-type observations по четырём selectors равна:

| Anchor | H0, us | X1, us | X1-PROJECTED, us |
| --- | ---: | ---: | ---: |
| zero | 1.190 | 0.843 | 1.706 |
| median | 1.827 | 1.405 | 2.600 |
| p90 | 2.625 | 1.534 | 3.153 |
| p99 | 4.269 | 2.165 | 5.243 |
| max | 34.442 | 5.597 | 17.256 |

First-operation evidence описательное: `28/84` resource summary rows имеют
MAD/median выше `5%`. Steady time существенно стабильнее: noisy только
`8/144` строк.

## Lookup и полный payload

Type-name lookup и полный payload не зависят от `ANY`/`ALL`, поэтому AV6 их
не перезапускал. Отдельные результаты остаются в
[свидетельствах S83-AV5](hbk-s83-av5-evidence.md): X1 type-name lookup
`813 ns/query` против H0 `853 ns/query`; X1 full type/method/property payload
остаётся `2.11-2.49x` H0. X1-PROJECTED сохраняет X1 base layout, но AV6 не
выдаёт нового timing-утверждения для этих операций.

## Проверка гипотезы и граница решения

Гипотеза «девять базовых ordered locator projections окупят runtime
ANY/ALL merge» дала разделённый результат:

- exact behavior, zero-allocation borrowed path и небольшой дополнительный
  footprint подтверждены;
- global и все dense непустые type scopes не компенсируют несколько входных
  потоков и проигрывают простому сканированию X1;
- раннее завершение пустого трёхконтекстного `ALL` даёт сильный локальный
  сигнал для селективных intersections;
- mmap startup/PSS signal X1 сохраняется практически без изменения.

Следующая возможная гипотеза — гибридный путь: availability-word scan для
`ANY`/dense scopes и cardinality-aware раннее intersection либо компактный
bitset для селективного `ALL`. Это только вывод для последующего эксперимента,
не production-рекомендация и не выбор backend.

AV6 закрывает измерительный пункт 1.24, но не закрывает 1.15/T183. Ветки,
артефакты и evidence сохраняются до отдельного решения пользователя.
