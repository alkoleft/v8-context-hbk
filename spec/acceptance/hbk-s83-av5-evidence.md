# Неранжированные свидетельства S83-AV5

## Статус

S83-AV5 завершён как отдельная проверка составной гипотезы `S83-X1`:

- global scope использует узкие SoA columns;
- platform type lookup использует специализированный mapped open-address hash;
- members одного типа читаются через direct `member_start/member_count` и
  owner-contiguous AoS range;
- полный payload остаётся R1-derived cold representation.

Результат не выбирает backend, не присваивает первое место, не меняет
`active_candidate_shortlist = [A0, I1, P1, R1]`, не разрешает merge и не
назначает канонический runtime. Состояние решения остаётся
`selection = pending-user-decision`.

## Воспроизводимость

Чистый последовательный run имеет идентичность
`s83-av5-final-sequential-2026-08-02-x1-fixed-identity`. Performance начался
только после parity и preflight всей registry; процессы выполнялись
последовательно round-robin.

| Объект | Значение |
| --- | --- |
| workload | `s83-av5-composite-scope-cache/v1` |
| harness commit | `73abb871fdac91f5395f43289f1d23431365ebe1` |
| HBK | `40,744,845` bytes; `5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48` |
| platform / locale | `8.3.27.1859` / `ru` |
| provider | schema `16`, extraction `11`, `220,270,592` bytes; `f626207a93f99eecbb6c76fb13482058e32c4c4d404c84bb33b45abde45233bc` |
| AV5 manifest | `16,428,376` bytes; `9136f28ba131bbbea0e9e1e74ea1d4c236d1e33c5d06a7fa418767d2dae8a880` |
| orchestration | `7,428` bytes; `6d7fef9aafab93b1a875058dff799b1b8408ae8a3e39443bcb0bc98520b3c2e4` |
| raw | `11,124/11,124` строк `ok`; `f8a796d9b01016df068e378771b8468e1a3df9122a53e4a7c2558fd934145af0` |
| resource | `3,655/3,655` строк `ok`; `5f26035946da2efed0411e812ac5989b7f44394ec78550ecf081b6b1dcc0eb0b` |
| parity | `47/47` записей `pass`; `708dad526d4fb4ed8cf4eed77ad675a3498939962ece10227d79e56cd2d170f6` |
| summary JSON | `377cf05b8c70ee9cb96d5c1787d5baf085e58cad9ea9c13548088cc82f949b6c` |
| summary Markdown | `0c03ba58c3db70869770cc0483d9df642a1c4c7e9e6ec9a03448096ed98c31b3` |

Финальные raw/parity/summary находятся в
`../v8-context-hbk-wt-85-composite-harness/target/hbk-s83-av5-frozen/run-3/`,
manifest и orchestration — на один уровень выше. Все эти файлы имеют mode
`0444`.
JSONL использует только schema `hbk-s83-av5-raw/v1`,
`hbk-s83-av5-resource-raw/v1` и `hbk-s83-av5-parity/v1`. Поля aggregate
score, rank, winner, recommendation и canonical отсутствуют.

## Registry и происхождение артефактов

| Backend | Роль | Commit | Artifact bytes | Artifact SHA-256 |
| --- | --- | --- | ---: | --- |
| `S83-H0` | SQL baseline | `73abb871fdac91f5395f43289f1d23431365ebe1` | 220,270,592 | `f626207a93f99eecbb6c76fb13482058e32c4c4d404c84bb33b45abde45233bc` |
| `S83-C0` | owned-cache control | `73abb871fdac91f5395f43289f1d23431365ebe1` | 220,270,592 max declared | provider SHA выше |
| `S83-I1` | type-hash reference | `24267cf231d2f87684e956c705ab5f14a2fb8424` | 33,712,262 | `ec078da5c6215cf308633de424d67124c8a46b2ad2b625143cfe29baa7d7907a` |
| `S83-R2-AOS` | member-AoS reference | `811ed4e8badfcac786b01b9624088488da75f84f` | 18,903,223 | `fe8c4c75592db4763afa8d0d266a3ef9e674a0cf6890fe430d23d7c69fdc95f3` |
| `S83-R2-SOA` | global-SoA reference | `1b3aae2c37ba54eea29ef38b834e5268b6ec2659` | 18,870,701 | `6c7a8a9760e72de2423055fba7f3060ec493347d552ef53be43b6f23d8fadadd` |
| `S83-X1` | composite hypothesis | `a84750fb08d3e8906f3d5b85e8ae20a6549c85d2` | 19,033,826 | `933a5cf4649200dea15f7b15160d4bc50eedf98de242b7ba164e7c4410acdf17` |

I1, R2-AOS, R2-SOA и X1 artifacts имеют mode `0444`. X1 использует отдельную
identity `HBKFX1`, layout `1`; его reader не совместим с R2/I1 identity.

## Поведенческая эквивалентность

Все 47 записей прошли exact parity:

- 45 context transcripts: H0, C0, R2-AOS, R2-SOA и X1 по всем девяти
  `AvailabilityContext`;
- две lookup-reference записи H0/I1;
- ordered global/type locator streams, owner/kind/logical identity,
  universal/explicit availability, provenance и full payload;
- primary/alias дают ожидаемого owner, miss даёт ноль owners и пустой scope.

Таким образом, performance-сравнение допущено поведенческим gate. Числовые ID
остаются session-local; parity сравнивает нормализованное логическое содержание.

## Правила чтения таблиц

Steady-таблицы используют медиану девяти samples. Для context-sensitive
операций показана медиана девяти per-context медиан; диапазоны и frozen `5%`
проверяются по отдельным context/anchor строкам. Это навигационная агрегация,
не score. `borrowed` не материализует набор; `collect` материализует компактный
набор locator. Время lookup приводится как `ns/query`, а не как время полного
manifest batch.

## Filtered global scope

Время — `us` на одно полное filtered enumeration глобального scope.

| Backend | Borrowed, us | Borrowed/H0 | Collect, us | Collect/H0 |
| --- | ---: | ---: | ---: | ---: |
| `S83-H0` | 43.268 | 1.000x | 44.058 | 1.000x |
| `S83-C0` | 43.144 | 1.001x | 43.655 | 0.993x |
| `S83-R2-AOS` | 10.179 | 0.236x | 10.589 | 0.241x |
| `S83-R2-SOA` | 8.340 | 0.194x | 8.742 | 0.206x |
| `S83-X1` | 10.809 | 0.249x | 11.220 | 0.257x |

X1 сохраняет сильный общий global-hot-layout signal относительно H0: collect
занимает `0.244-0.275x` H0 в зависимости от context. Однако точный перенос
R2-SOA component cost не подтверждён: X1/R2-SOA равен `1.049-1.411x` для
borrowed и `1.045-1.422x` для collect; frozen граница `5%` выполнена только в
одном из девяти contexts для каждой операции.

## Platform type lookup

| Backend | ns/query | H0 ratio |
| --- | ---: | ---: |
| `S83-H0` | 853 | 1.000x |
| `S83-C0` | 834 | 0.978x |
| `S83-I1` | 780 | 0.914x |
| `S83-R2-AOS` | 1,505 | 1.764x |
| `S83-R2-SOA` | 1,494 | 1.751x |
| `S83-X1` | 813 | 0.953x |

X1/I1 равен `1.042x`, поэтому специализированный X1 hash воспроизводит I1
steady signal в зафиксированной границе `5%`. X1 одновременно на `4.7%`
быстрее H0 для этого workload. Все строки выполняют одинаковую boundary-
нормализацию query: `349,500` allocation calls / `40,413,600` bytes за
steady sample, поэтому это не скрытая привилегия X1. Это отдельный lookup без
последующего scope, но не изолированный pre-normalized hash probe: цена
публичной нормализации имени входит в измерение.

## Filtered members одного типа

Время — `ns` на scope одного anchor; таблица показывает медиану девяти
per-context медиан. `X1/H0` и `X1/R2-AOS` — медианы девяти сопоставленных
context ratios.

| Operation | Anchor | H0 | R2-AOS | R2-SOA | X1 | X1/H0 | X1/R2-AOS |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| borrowed | zero | 57 | 58 | 115 | 58 | 1.018x | 1.000x |
| borrowed | median | 155 | 176 | 242 | 177 | 1.154x | 1.006x |
| borrowed | p90 | 355 | 460 | 487 | 457 | 1.351x | 0.987x |
| borrowed | p99 | 493 | 884 | 703 | 894 | 1.799x | 1.010x |
| borrowed | max | 4,705 | 5,309 | 5,776 | 5,305 | 1.128x | 0.999x |
| collect | zero | 60 | 61 | 117 | 61 | 1.000x | 1.000x |
| collect | median | 201 | 224 | 288 | 225 | 1.169x | 1.000x |
| collect | p90 | 413 | 521 | 561 | 526 | 1.281x | 0.998x |
| collect | p99 | 539 | 956 | 721 | 944 | 1.732x | 1.005x |
| collect | max | 4,786 | 5,470 | 6,038 | 5,457 | 1.140x | 0.992x |

X1 воспроизводит R2-AOS member-range cost в 86 из 90 точных
operation/anchor/context сравнений. Четыре строки выходят за `5%`:

- borrowed/web_client/zero: `61` против `58 ns` (`1.052x`);
- collect/mobile_application_client/zero: `61` против `66 ns` (`0.924x`);
- collect/mobile_application_server/zero: `63` против `59 ns` (`1.068x`);
- collect/mobile_standalone_server/p90: `526` против `555 ns` (`0.948x`).

В трёх из четырёх сравнений хотя бы одна строка отмечена summary как noisy;
даже единственный формально стабильный выход равен `3 ns`. По frozen правилу
ожидание «каждый anchor/context в пределах 5%» всё равно не выполнено полностью.
Главное ограничение не меняется: на непустых anchors X1 обычно медленнее H0,
поэтому owner-contiguous range подтверждён как воспроизведённая структура
R2-AOS, но не как превосходящая SQL baseline операция.

## Lookup плюс filtered scope

Медианы по девяти contexts, `us` на end-to-end lookup и compact scope.

| Anchor | Form | H0, us | X1, us | X1/H0 range |
| --- | --- | ---: | ---: | ---: |
| zero | primary / alias / miss | 0.404 / 0.295 / 0.445 | 0.455 / 0.364 / 0.439 | 1.108-1.139 / 1.200-1.286 / 0.973-0.993x |
| median | primary / alias / miss | 1.761 / 0.839 / 0.446 | 2.195 / 1.034 / 0.438 | 1.171-1.261 / 1.218-1.258 / 0.973-1.003x |
| p90 | primary / alias / miss | 1.822 / 0.869 / 0.447 | 2.382 / 1.108 / 0.439 | 1.192-1.321 / 1.204-1.426 / 0.971-1.007x |
| p99 | primary / alias / miss | 1.606 / 1.178 / 0.447 | 2.355 / 1.736 / 0.437 | 1.164-1.512 / 1.182-1.482 / 0.945-0.987x |
| max | primary / alias / miss | 5.605 / 5.066 / 0.468 | 6.279 / 5.781 / 0.437 | 1.106-1.160 / 1.096-1.166 / 0.914-0.966x |

Hash заметно сокращает цену относительно R2 lookup+scope, но успешный
primary/alias end-to-end путь всё ещё медленнее H0. Miss обычно быстрее H0,
поскольку после отрицательного hash lookup scope остаётся пустым.

## Full payload

| Operation | H0, us | R2-AOS, us | R2-SOA, us | X1, us | X1/H0 | X1 allocations |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| type payload | 3.871 | 9.617 | 9.448 | 9.622 | 2.486x | 500 calls / 12,000 bytes |
| method payload | 54.661 | 114.818 | 115.054 | 115.343 | 2.110x | 2,500 calls / 47,600 bytes |
| property payload | 277.981 | 594.496 | 587.237 | 602.707 | 2.168x | 0 / 0 |

X1 находится в `1.001-1.014x` R2-AOS и тем самым действительно сохраняет
неизменённую R1-derived payload цену. Эта цена остаётся `2.11-2.49x` H0 и не
скрывается общей оценкой.

## Startup, first operations и память

`Entry -> ready` и PSS — медианы resource rows backend. I1 имеет только одну
lookup-reference строку. First-operation строки публикуются отдельно и не
сворачиваются в score.

| Backend | Ready, ms | PSS, KiB | RSS, KiB | Artifact, bytes | Hot sections, bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| `S83-H0` | 760.207 | 92,787 | 94,664 | 220,270,592 | 0 |
| `S83-C0` | 65.984 | 71,192 | 73,140 | 220,270,592 max declared | 0 |
| `S83-I1` | 382.870 | 58,004 | 59,464 | 33,712,262 | 2,228,224 |
| `S83-R2-AOS` | 83.101 | 43,800 | 45,264 | 18,903,223 | 162,832 |
| `S83-R2-SOA` | 81.283 | 43,732 | 45,196 | 18,870,701 | 130,235 |
| `S83-X1` | 80.901 | 43,880 | 45,344 | 19,033,826 | 293,303 |

| Backend | First global collect, us | First type lookup, us | Median first lookup+scope, us | Retained scopes, us |
| --- | ---: | ---: | ---: | ---: |
| `S83-H0` | 56.899 | 2.507 | 4.343 | 491.027 |
| `S83-C0` | 53.841 | 2.273 | 3.816 | 483.359 |
| `S83-I1` | n/a | 1.125 | n/a | n/a |
| `S83-R2-AOS` | 18.330 | 2.621 | 4.396 | 177.468 |
| `S83-R2-SOA` | 12.069 | 2.885 | 4.658 | 172.689 |
| `S83-X1` | 13.528 | 1.721 | 3.513 | 180.237 |

Resource first-operation evidence содержит только пять samples и часто шумно:
353 resource summary rows имеют `noise_status = noisy`, из них 342 относятся
к `first_type_lookup_scope`. Поэтому first-operation цифры описательны;
устойчивый вывод по memory опирается прежде всего на ready/PSS/artifact.

Borrowed global/type scope выполняется без steady allocations. Compact global
collect выполняет `1,000` allocations / `2,404,000` bytes за sample у всех
полных backend. X1 type collect имеет одну materialization allocation на
итерацию: от `0` для zero до `10,000` calls / `11,800,000` bytes для max.

## Проверка составной гипотезы

| Frozen expectation | Результат |
| --- | --- |
| X1 global в пределах `5%` R2-SOA | не воспроизведено: вне границы 8/9 contexts для borrowed и 8/9 для collect |
| X1 type-name lookup в пределах `5%` I1 | воспроизведено: `1.042x` I1 |
| X1 type scope каждого anchor/context в пределах `5%` R2-AOS | частично: `86/90`; четыре точных строки вне границы |
| End-to-end lookup+scope рассматривается отдельно | выполнено; successful forms медленнее H0, miss около/быстрее H0 |
| Payload/startup/memory/artifact публикуются отдельно | выполнено |
| Exact behavior parity до performance | `47/47 pass` |

Составная гипотеза дала смешанный, причинно разложимый результат. Широкие
component signals подтверждаются повторно: компактный mapped global layout
намного быстрее H0, специализированный hash сохраняет I1 steady lookup,
fixed/mapped artifact резко сокращает ready time, PSS и размер относительно H0.
Но X1 не сохраняет точную стоимость R2-SOA global path и owner-contiguous
member range не обгоняет H0 на основных непустых type scopes. Поэтому X1 как
целое нельзя считать подтверждённой гипотезой только на основании сильных
компонентов.

## Почему global выигрывает, а type members проигрывают

Это разные профили доступа.

- Global scope последовательно проверяет 601 записей. Узкие mapped columns
  читают только locator, `u16 availability_word` и kind, отделяя cold payload;
  на длинном последовательном проходе меньший hot working set окупает bounds и
  decode.
- Type scope читает один уже найденный owner range. Большинство ranges короткие
  и нерегулярные; H0/C0 уже имеют прямой owned ID slice, поэтому постоянные
  затраты цикла, availability check и mapped-record decode доминируют. На
  p99/max большее число AoS записей усиливает разницу.
- Hash ускоряет нахождение type owner, но не устраняет последующий member scan.
  Поэтому X1 улучшает R2 end-to-end lookup+scope, однако successful
  primary/alias всё ещё платят цену owner-contiguous enumeration; miss сразу
  завершает hash lookup и не выполняет scan.

Первые два пункта — вывод из измеренной physical/layout формы; конкретное
распределение CPU-cache промахов не профилировалось и остаётся объясняющей
гипотезой, а не отдельным доказанным счётчиком.

## Граница решения

AV5 закрывает измерительный пункт 1.23, но не закрывает 1.15/T183. Ветки,
артефакты и evidence сохраняются. Пользовательский выбор backend, merge,
production dependency, канонический формат и возможное составление иного
runtime остаются отдельным последующим решением.
