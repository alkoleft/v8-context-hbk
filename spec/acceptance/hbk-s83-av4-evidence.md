# Неранжированные свидетельства S83-AV4

## Статус решения

Корректирующий workload S83-AV4 завершён. Он сравнивает фактические hot paths
основного потребителя `v8-context`: filtered platform global scope, lookup типа,
filtered scope одного найденного типа, материализацию компактного набора
locator и отдельное чтение полного payload.

Результат остаётся описательным:

- `selection = pending-user-decision`;
- ни один backend не назван победителем или приоритетным кандидатом;
- ни одна ветка кандидата не слита и не удалена;
- canonical runtime не изменён;
- `ModuleContextKind`, module events, precedence, `effective_members` и
  downstream resolve не входят в AV4.

## Воспроизводимость и происхождение

Измерен workload `s83-av4-consumer-scope-layout/v2` на платформе
`8.3.27.1859` и русском HBK:

- HBK: `40,744,845` bytes, SHA-256
  `5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48`;
- provider schema 16 / extraction schema 11: `220,270,592` bytes, SHA-256
  `f626207a93f99eecbb6c76fb13482058e32c4c4d404c84bb33b45abde45233bc`;
- query manifest: `16,428,376` bytes, SHA-256
  `15fbf865cd3d96d0c4df6fe23dee09a1d22bf08fd340d0c82663314187d5b394`;
- frozen harness commit:
  `97fa011d292ab0b243b484749e3f4ce5d22909e6`;
- common harness SHA-256:
  `7ac3b5861db2b05179564d0ef1317df459de3d70ddcb30c9cc2844f17a36394f`;
- driver SHA-256:
  `3d4cf973a1728f01651867e31027e2dab54e5f1f1c6a1ff08ad5f32e32444156`.

Последовательный чистый проход с полными per-context/per-anchor строками
находится в
`v8-context-hbk-wt-84-scope-harness/target/hbk-s83-av4-v2-final/run-final-sequential-20260802-clean-1/`.
Все performance-процессы и resource-процессы выполнялись последовательно.
Ранее остановленный contaminated run в эти результаты не включён.

| Артефакт evidence | Строки | SHA-256 |
| --- | ---: | --- |
| `raw.jsonl` | 17,793 | `e5c909b0b3eb179ccaa14d4574399e716fc1868a5ebfe28b306f4ba44b482f48` |
| `resource.jsonl` | 5,841 | `df262ad87f122bef1b7aaf91d5334b6b3281644b7d67996b17a81a09a1d36216` |
| `parity/parity.jsonl` | 74 | `bde8966296e5074edc116f8f77dfd6cf5bdb98a16868bdd92466ff5c4e4f74b1` |
| `summary.json` | 1,977 performance + 1,169 resource агрегатов | `1a263c9ce171052f337b4eeb7116df26e7105996b7d409f1cd120bcc43777d9e` |
| `summary.md` | описательная таблица | `e8a22acc4d90069e0ad4ae72b6fd24f2d854e89b3a85eb5bc7c31f1bf5fe5dec` |

Сводка запрещает поля `rank`, `score`, `winner`, `recommendation`,
`canonical` и их русские эквиваленты. Порог шума остаётся
`MAD / median > 5%`; samples не удалялись. Полная generated summary хранит
noise status каждой отдельной строки. Компактные таблицы ниже используют
медиану отдельных per-context медиан только как навигационное представление и
не являются score или новым критерием допуска. Пять type anchors никогда не
смешиваются в одно среднее.

## Поведенческая эквивалентность

Все 74 parity-записи имеют `parity_status = pass`:

- 72 точных context transcript: H0, C0, R1, R1-DIRECT и четыре R2 layout для
  каждого из девяти `AvailabilityContext`;
- две lookup-reference записи H0/I1;
- primary и alias возвращают ровно ожидаемого owner;
- miss возвращает ноль owners и пустой type scope;
- global/type locator order, owner, kind, logical ID, universal/explicit
  availability и normalized full payload совпадают с H0;
- smoke stderr всех девяти backend пуст.

Это доказывает эквивалентность именно AV4 query surface. Числовые locator/ID
остаются локальными для текущего run и не объявляются стабильными между
поколениями снапшота.

## Ветки и артефакты

| Backend | Роль | Commit | Runtime-артефакт, bytes | SHA-256 |
| --- | --- | --- | ---: | --- |
| `S83-H0` | SQL baseline | `97fa011d292ab0b243b484749e3f4ce5d22909e6` | 220,270,592 | `f626207a93f99eecbb6c76fb13482058e32c4c4d404c84bb33b45abde45233bc` |
| `S83-C0` | owned cache control | `97fa011d292ab0b243b484749e3f4ce5d22909e6` | 17,864,353 | `ea342bd539e6f171cf7be1661e07f2ee11f776403fc56c6984658eeb23a5cf05` |
| `S83-I1` | lookup reference | `a7faf33518d59905e0f6a557ab9841cd0fff1910` | 33,712,262 | `ec078da5c6215cf308633de424d67124c8a46b2ad2b625143cfe29baa7d7907a` |
| `S83-R1` | parent control | `b106caaec18a22dba12d2f02a1ab99846c136908` | 18,740,312 | `811606fd77539b3a68c2baf631be8361d5195e2733788e0cce5dfed8adc53309` |
| `S83-R1-DIRECT` | causal type-range control | `a62a8419842b44fc0e9f9e83c77a5dfc32f4b4e4` | 18,754,325 | `9918f59cbcfb84f4855c30c83dfe4afecf75939fe9e46e35a6de8dd476d6dd68` |
| `S83-R2-AOS` | AoS mask/layout | `1ab6ef419771c824f6377daa7f2a8aec56d2bc21` | 18,903,223 | `fe8c4c75592db4763afa8d0d266a3ef9e674a0cf6890fe430d23d7c69fdc95f3` |
| `S83-R2-SOA` | SoA columns/layout | `dc46c4b54d137cd5ac67d4781477462e9f817cf3` | 18,870,701 | `6c7a8a9760e72de2423055fba7f3060ec493347d552ef53be43b6f23d8fadadd` |
| `S83-R2-BITSET` | dense context bitmaps | `1111a715e45947ea73385067c10e576609658040` | 18,763,784 | `39d5ca0dd70de12e9adfce1fe7ec7c0a0f47b01727f56959d4b6c448783485ad` |
| `S83-R2-CSR` | direct context rows | `416d81a30bf4a9e21b2d7cff14c0a3adb1cbabce` | 19,161,431 | `72a6283a2c42770fae93ceb20e65d9bd6aedf5528c804140d54be74ffeec90f4` |

Все измеренные бинарные артефакты I1, R1, R1-DIRECT и R2 имеют mode `0444`.
Контрольные H0 provider и C0 owned cache не являются кандидатами snapshot
format; их файловые permissions не используются как evidence неизменяемой
публикации кандидата.

## Steady filtered global scope

Значения — микросекунды на один запрос. Каждая исходная строка содержит 1,000
повторов; в таблице показана медиана девяти per-context медиан. H0 называется
SQL baseline по происхождению/startup, но его steady path работает над уже
материализованным owned snapshot, а не выполняет SQL на каждом запросе.

| Backend | Borrowed, us | Collect compact set, us | Collect / H0 |
| --- | ---: | ---: | ---: |
| `S83-H0` | 43.642 | 43.821 | 1.000x |
| `S83-C0` | 43.117 | 43.692 | 0.997x |
| `S83-R1` | 194.799 | 196.598 | 4.486x |
| `S83-R1-DIRECT` | 196.717 | 196.912 | 4.493x |
| `S83-R2-AOS` | 10.229 | 10.681 | 0.244x |
| `S83-R2-SOA` | 8.176 | 8.636 | 0.197x |
| `S83-R2-BITSET` | 13.403 | 13.753 | 0.314x |
| `S83-R2-CSR` | 12.068 | 12.041 | 0.275x |

Все четыре R2 layout дают отдельный измеренный выигрыш filtered global scope
относительно H0/C0. Это не переносится автоматически на остальные операции и
не определяет итоговый выбор формата.

## Steady filtered scope одного типа

В ячейке указано `borrowed / collect`, ns на один запрос, как медиана девяти
per-context медиан. Anchors остаются отдельными: `zero`, `median`, `p90`,
`p99`, `max`.

| Backend | zero | median | p90 | p99 | max |
| --- | ---: | ---: | ---: | ---: | ---: |
| `S83-H0` | 57 / 60 | 155 / 197 | 354 / 401 | 477 / 546 | 4,673 / 4,775 |
| `S83-C0` | 57 / 60 | 151 / 196 | 355 / 405 | 460 / 523 | 4,692 / 4,782 |
| `S83-R1` | 154 / 157 | 1,643 / 1,696 | 4,902 / 4,973 | 11,707 / 11,756 | 19,087 / 19,418 |
| `S83-R1-DIRECT` | 81 / 83 | 1,553 / 1,601 | 4,832 / 4,905 | 11,631 / 11,689 | 19,275 / 19,454 |
| `S83-R2-AOS` | 58 / 60 | 179 / 224 | 462 / 521 | 883 / 940 | 5,304 / 5,442 |
| `S83-R2-SOA` | 116 / 117 | 245 / 291 | 483 / 564 | 713 / 723 | 5,732 / 6,011 |
| `S83-R2-BITSET` | 121 / 121 | 388 / 438 | 896 / 929 | 622 / 684 | 12,135 / 12,416 |
| `S83-R2-CSR` | 61 / 61 | 280 / 313 | 814 / 857 | 509 / 551 | 10,558 / 10,617 |

Ни один R2 layout не даёт общего выигрыша над H0/C0 на small isolated type
scope. AOS ближе остальных layout к owned controls. Один только
`member_start/count` уменьшает zero-anchor overhead R1, но не устраняет
отставание R1/R1-DIRECT на непустых anchors.

## Lookup и end-to-end lookup + type scope

`type_by_name` измеряет полный frozen manifest lookup за один steady query;
`ns/query` нормализует время по числу запросов, а `ns/object` — по числу
возвращённых owner objects. Остальные колонки —
`type_lookup_scope_collect` для `median` anchor, агрегированный только по девяти
context. Composite-единицы — микросекунды.

| Backend | Type manifest, us | ns/query | ns/object | Primary + scope | Alias + scope | Miss + empty scope |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `S83-H0` | 2,972.338 | 850 | 847 | 1.817 | 0.798 | 0.470 |
| `S83-C0` | 2,915.971 | 834 | 831 | 1.833 | 0.806 | 0.468 |
| `S83-I1` | 2,750.175 | 786 | 784 | n/a | n/a | n/a |
| `S83-R1` | 5,131.894 | 1,468 | 1,463 | 4.547 | 2.977 | 0.948 |
| `S83-R1-DIRECT` | 5,179.928 | 1,482 | 1,477 | 4.341 | 3.044 | 0.970 |
| `S83-R2-AOS` | 5,276.507 | 1,509 | 1,504 | 2.972 | 1.389 | 0.940 |
| `S83-R2-SOA` | 5,225.661 | 1,495 | 1,490 | 3.066 | 1.440 | 0.815 |
| `S83-R2-BITSET` | 5,109.177 | 1,461 | 1,457 | 3.231 | 1.709 | 0.807 |
| `S83-R2-CSR` | 5,168.154 | 1,478 | 1,474 | 3.048 | 1.418 | 0.941 |

I1 сохраняет отдельный steady type-index signal (`0.925x` H0 для полного
manifest), но не участвует в scope/payload suites. На end-to-end primary/alias
scope все R1/R2 строки медленнее H0/C0; miss также не выигрывает у owned
controls. Эти локальные результаты не отменяют выигрыш R2 на global scope.

## Полный payload

Lookup и фильтрация подготовлены до timed interval. Значения — ns на один
полностью прочитанный объект; полный batch total остаётся в generated summary.

| Backend | Type | Method | Property |
| --- | ---: | ---: | ---: |
| `S83-H0` | 772 | 1,100 | 790 |
| `S83-C0` | 786 | 1,088 | 776 |
| `S83-R1` | 1,912 | 2,341 | 1,673 |
| `S83-R1-DIRECT` | 1,898 | 2,327 | 1,648 |
| `S83-R2-AOS` | 1,907 | 2,343 | 1,649 |
| `S83-R2-SOA` | 1,904 | 2,318 | 1,672 |
| `S83-R2-BITSET` | 1,864 | 2,332 | 1,682 |
| `S83-R2-CSR` | 1,908 | 2,360 | 1,695 |

Все mapped R1/R2 представления в этом проходе медленнее H0/C0 на полном
payload. AV4 не скрывает эту регрессию за более быстрым global hot path.

## Startup, первая операция, retained memory и размер

`Ready` — медиана `entry_to_ready` по resource rows backend. First-operation
значения получены в свежих процессах. `Retained PSS` — PSS процесса при
одновременном удержании compact global/type sets, а не только размер этих
векторов. Для C0 в колонке размера указан cache-файл; provider также объявлен
для проверки provenance.

Resource operation timings заметно шумнее steady: noise gate отмечает
392/1,169 resource aggregates против 29/1,977 steady aggregates. Поэтому
first-operation числа являются отдельным описательным сигналом и не могут
самостоятельно определять выбор. Для I1 resource evidence содержит только один
sample, как и было frozen до запуска.

| Backend | Ready, ms | First global, us | First type lookup, us | First primary lookup + median scope, us | Retained PSS, KiB | Runtime file, bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `S83-H0` | 764.276 | 57.694 | 2.977 | 5.000 | 92,798 | 220,270,592 |
| `S83-C0` | 66.672 | 54.204 | 4.707 | 4.799 | 71,216 | 17,864,353 |
| `S83-I1` | 386.611 | n/a | 1.173 | n/a | n/a | 33,712,262 |
| `S83-R1` | 80.695 | 209.257 | 2.840 | 8.093 | 43,604 | 18,740,312 |
| `S83-R1-DIRECT` | 77.995 | 216.695 | 2.646 | 7.818 | 43,648 | 18,754,325 |
| `S83-R2-AOS` | 79.870 | 13.336 | 2.711 | 5.919 | 43,780 | 18,903,223 |
| `S83-R2-SOA` | 78.815 | 13.114 | 2.690 | 6.181 | 43,780 | 18,870,701 |
| `S83-R2-BITSET` | 80.171 | 17.248 | 2.941 | 6.588 | 43,644 | 18,763,784 |
| `S83-R2-CSR` | 83.727 | 16.029 | 2.768 | 5.720 | 44,076 | 19,161,431 |

R1/R2 startup остаётся близок к C0 и существенно ниже H0; retained PSS R1/R2
также ниже обоих owned controls. I1 даёт минимальную first type lookup latency,
но имеет отдельную высокую ready-cost и не предоставляет scope/payload.

## Проверка физических гипотез layout

`Physical entries` и `returned` ниже относятся к 1,000 повторов global
borrowed для context с медианным count. Это проверяет, что bitmap/CSR результат
не получен скрытым row-wise scan.

| Backend | Hot sections, bytes | Physical entries | Returned locators |
| --- | ---: | ---: | ---: |
| `S83-R1` | 1,076,784 | 601,000 | 354,000 |
| `S83-R1-DIRECT` | 1,218,776 | 601,000 | 354,000 |
| `S83-R2-AOS` | 162,832 | 601,000 | 354,000 |
| `S83-R2-SOA` | 130,235 | 601,000 | 354,000 |
| `S83-R2-BITSET` | 23,360 | 10,000 bitmap words | 354,000 |
| `S83-R2-CSR` | 420,985 | 354,000 | 354,000 |

Измерения подтверждают разные причинные механизмы:

- AOS/SOA сохраняют scan 601 hot entries, но их компактная hot layout даёт
  выигрыш global scope;
- BITSET проходит bitmap words, а не выполняет per-row availability predicate;
- CSR читает уже отфильтрованный ordered row и физически проходит только
  возвращаемые locators;
- для маленького type scope уменьшение физической работы BITSET/CSR не
  компенсирует overhead представления; global и type scope требуют разных
  компромиссов.

## Итог без выбора

AV4 опроверг утверждение, что zero-copy layout в принципе не может обогнать H0
на filtered enumeration: все четыре R2 варианта быстрее H0/C0 на основном
filtered global scope. Одновременно AV4 показывает, что ни один измеренный
layout не доминирует по всем нужным операциям:

- R2 выигрывает global scope, startup, retained memory и размер относительно
  H0;
- isolated type scope остаётся ближе всего к H0/C0 у AOS, но не быстрее их;
- I1 сохраняет отдельный lookup signal, не являясь полным scope backend;
- mapped full payload и end-to-end type lookup + scope медленнее owned controls;
- `member_start/count` само по себе не объясняет выигрыш R2 global layout.

Следующий шаг — решение пользователя о допустимых компромиссах и о том, нужна
ли составная гипотеза. До такого решения этот evidence не назначает первое
место и не разрешает production-миграцию.
