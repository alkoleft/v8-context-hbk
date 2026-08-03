# Свидетельства X1-INT

Статус: `PASS`. OpenSpec 4.2–4.6 и все gates ADR-0012 пройдены без waiver или
aggregate score. Отдельный canonical cutover разрешён; сам cutover и cleanup
этим документом ещё не считаются выполненными.

## Frozen inputs

| Вход | Значение |
|---|---|
| Platform | `8.3.27.1859` |
| Locale / source locale | `ru` / `ru` |
| HBK | `/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk` |
| HBK bytes / SHA-256 | `40,744,845` / `5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48` |
| Provider | `/home/alko/develop/open-source/v8-maintain-projects/v8-context/.v8-context/platform-indexes/8.3.27.1859/shcntx_ru.sqlite` |
| Provider bytes / SHA-256 | `204,288,000` / `317f3cdd914e635c89b975bf9ebcf28238bdbabd54e455121a083558d4e05f5e` |
| Provider / extraction schema | `16` / `11` |

## OpenSpec 4.2 — full-corpus storage parity

Команда explicit acceptance probe:

```bash
env V8_CONTEXT_HBK_X1_INT_INDEX=/home/alko/develop/open-source/v8-maintain-projects/v8-context/.v8-context/platform-indexes/8.3.27.1859/shcntx_ru.sqlite \
  cargo test -p syntax-helper-search \
  snapshot::x1_format::tests::x1_full_corpus_forward_payload_matches_owned_snapshot \
  --lib -- --ignored --exact --nocapture
```

Результат: `PASS`, wall time запуска `71.73 s`. Это диагностическое время
build + publication + exhaustive comparison, не X1-INT performance sample и не
значение какого-либо gate.

Published X1 generation:

- bytes: `12,430,416`;
- SHA-256: `0f5843f95401ba9cb5421b2ecc58a101779e43b17a86909d484bd6123ce3ffd7`.

Полные counts:

| Family | Count |
|---|---:|
| strings | 71,073 |
| platform types | 1,749 |
| type members | 18,004 |
| callables | 8,299 |
| globals | 601 |
| query tables | 53 |
| query fields | 498 |
| query parameters | 56 |
| language facts | 0 |
| enums | 670 |
| enum values | 2,934 |

Comparator обошёл dictionary и все records каждого семейства, вложенные
signatures/parameters/type refs/template bindings, availability,
available-since и provenance. Fixture-path тем же comparator дополнительно
покрывает семейство language facts, отсутствующее в frozen S83 provider.
Сравнение numeric IDs допустимо только внутри одного build generation для
проверки сохранения layout; durable identity между sessions из результата не
следует.

Не закрыты этим результатом: catalog/resolver seam 4.4,
analyzer A/B 4.5 и применение всех gates 4.6. Raw stdout остаётся service
data и не хранится в `spec/`.

## OpenSpec 4.3 — full-corpus lookup parity

Команда explicit acceptance probe:

```bash
env V8_CONTEXT_HBK_X1_INT_INDEX=/home/alko/develop/open-source/v8-maintain-projects/v8-context/.v8-context/platform-indexes/8.3.27.1859/shcntx_ru.sqlite \
  cargo test -p syntax-helper-search \
  snapshot::x1_format::tests::x1_full_corpus_lookup_surface_matches_owned_snapshot \
  --lib -- --ignored --exact --nocapture
```

Результат: `PASS`.

- semantic call pairs: `280,317`;
- вызовы owned + mapped handles: `560,634`;
- ordered normalized transcript SHA-256:
  `ce7e5bf73e497703fba7c9000ac827ac07db1d3783d712eb4d7b656e45bd5847`;
- X1 bytes / SHA-256: `12,430,416` /
  `0f5843f95401ba9cb5421b2ecc58a101779e43b17a86909d484bd6123ce3ffd7`.

Probe обошёл каждый distinct persisted key, каждый exact ID и
owner range, включая empty ranges, и fixed miss для каждого method
family. Fixture дополнительно проверил duplicate multi-hit order,
ambiguity, optional kind, unsupported/unknown и language/module cases,
которых может не быть в frozen corpus. Numeric IDs сравнивались
только внутри одного build generation; durable identity не заявляется.

## OpenSpec 4.4 — unified catalog/resolver seam

Команды behavior и allocation probes:

```bash
cargo test -p context-resolver-search --all-features
cargo test -p context-resolver-search --features snapshot-experiment-alloc \
  tests::x1_mapped_borrowed_catalog_enumeration_allocates_nothing \
  -- --ignored --exact --test-threads=1
```

Результат: `PASS`.

- normalized full catalog/resolver transcript: `40` строк;
- transcript SHA-256:
  `35cb2cff4ba1777e200298a677725fe858b1bcdca5c0497c2a29c115237097c0`;
- sequential mapped repeats: `8/8`;
- concurrent mapped repeats: `4` workers × `8`, `32/32`;
- borrowed provider traversal: `0` allocation calls, `0` reallocations,
  `0` allocated bytes.

Transcript охватывает direct BSL/SDBL catalogs, полный view payload и
`PlatformSnapshotSource`/`QueryTableSnapshotSource`. Fixture задаёт непустые
`AvailabilityContext` для проверки `ANY`/`ALL`, публикует stable X1 slot,
открывает mapped owner и удаляет исходные HBK/SQLite до всех сравнений и
повторов. Compatibility response-DTO allocations не входят в allocation gate;
borrowed probe отдельно обходит filtered type members, global methods/
properties и query tables/fields/parameters.

Повторный frozen corpus regression сохранил:

- X1 bytes / SHA-256: `12,430,416` /
  `0f5843f95401ba9cb5421b2ecc58a101779e43b17a86909d484bd6123ce3ffd7`;
- lookup pairs/calls: `280,317` / `560,634`;
- lookup transcript SHA-256:
  `ce7e5bf73e497703fba7c9000ac827ac07db1d3783d712eb4d7b656e45bd5847`.

Task 4.4 не меняет canonical source и не разрешает cleanup. Открыты real
`v8-context` A/B 4.5 и применение frozen gates 4.6.

## OpenSpec 4.5 — real `v8-context` X1-INT

Дата: `2026-08-03`. Среда: `alko-home`, Intel Core i7-4770 3.40 GHz,
Linux `6.8.0-111-generic`, Rust/Cargo `1.95.0`, release,
`RAYON_NUM_THREADS=1`, один test thread. Измеренные revisions:

- `v8-context-hbk`: `464437a644d8249bc1dc76be24d909f2493d9d14`;
- `v8-context`: `b1cd76d48b8823cbb9a9792375fbf12f0c3fafbf`;
- project: `b7e627f02fe10028e27bfec99dbc1afa7fd8324d`.

Все 24 процесса выполнены в точном порядке `H0-A, X1-A, X1-B, H0-B` для
каждой пары scenario/mode. OS page cache не очищался. Raw logs, order,
postprocessed results и gates находятся в service data
`v8-context/target/x1-int/b1cd76d4/`; manifest содержит `30` SHA-256 строк и
сам имеет SHA-256
`86c7b34dedba5c8fe20b33b310f826a26fe4bc7dec18de84564bbfe086d33368`.
Existing benchmark checkpoint schema не изменён: он не представляет
same-revision четырёхпозиционную H0/X1 matrix.

### Первое производство X1

Wall и `dhat` использовали разные свежие stable slots. Граница начиналась до
SQLite-to-owned materialization и завершалась после encode/full validation,
fsync/publication/current, drop owned graph и validated mapped reopen.

| Метрика | Значение |
|---|---:|
| build-to-ready wall | `4761.942604 ms` |
| user / system CPU | `4360.000000 / 200.000000 ms` |
| process peak RSS | `108.609536 MB` |
| snapshot materialization substage | `631.928766 ms` |
| total allocations | `2,867,770` blocks / `314.552758 MB` |
| retained after operation | `5` blocks / `0.000353 MB` |
| peak heap | `208,640` blocks / `78.978466 MB` |
| X1 artifact | `12,430,416` bytes |
| X1 artifact SHA-256 | `0f5843f95401ba9cb5421b2ecc58a101779e43b17a86909d484bd6123ce3ffd7` |

Artifact platform/source/provider identity точно совпала с frozen inputs;
generation не была переиспользована.

### Wall-time, CPU и RSS

В ячейках time и RSS отношение равно `X1/H0`. CPU показан как
`user + system`, в ms.

| Scenario | Pair | H0 median / MAD, ms | X1 median / MAD, ms | Time ratio | H0 CPU | X1 CPU | H0 RSS, MB | X1 RSS, MB | RSS ratio |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| prepared handle | A | 0.020060 / 0.000332 | 0.019788 / 0.000348 | 0.986441 | 10 + 0 | 0 + 10 | 84.983808 | 31.612928 | 0.371988 |
| prepared handle | B | 0.020256 / 0.000312 | 0.019005 / 0.000285 | 0.938241 | 10 + 0 | 10 + 0 | 85.188608 | 31.518720 | 0.369987 |
| cold handle | A | 848.151145 / 6.599076 | 358.077365 / 2.681058 | 0.422186 | 9250 + 570 | 5050 + 300 | 91.066368 | 31.678464 | 0.347861 |
| cold handle | B | 848.876683 / 8.123441 | 354.817525 / 8.932345 | 0.417985 | 9170 + 610 | 5040 + 310 | 90.419200 | 31.518720 | 0.348584 |
| prepared full resolution | A | 1297.956653 / 4.570690 | 1178.200720 / 1.358481 | 0.907735 | 11640 + 30 | 10590 + 20 | 84.606976 | 44.191744 | 0.522318 |
| prepared full resolution | B | 1271.109479 / 7.239661 | 1175.756587 / 10.809131 | 0.924985 | 11410 + 30 | 10630 + 20 | 84.426752 | 43.511808 | 0.515379 |

Каждая wall-строка содержит девять raw samples; они сохранены в manifest-
проверенных logs. Prepared handle и full resolution открывают provider до
таймера; поэтому их RSS, в отличие от heap operation profile, фиксирует эффект
замены owned graph на mapped X1 на уровне отдельного процесса.

### Heap и allocations

| Scenario | Pair | H0 total blocks / MB | X1 total blocks / MB | H0 peak blocks / MB | X1 peak blocks / MB |
|---|---|---:|---:|---:|---:|
| prepared handle | A | 1,900 / 0.173300 | 1,900 / 0.173300 | 3 / 0.001192 | 3 / 0.001192 |
| prepared handle | B | 1,900 / 0.173300 | 1,900 / 0.173300 | 3 / 0.001192 | 3 / 0.001192 |
| cold handle | A | 2,174,641 / 234.496727 | 699,366 / 81.159818 | 504,290 / 63.424453 | 15,908 / 3.078806 |
| cold handle | B | 2,174,663 / 234.497951 | 699,357 / 81.158242 | 504,290 / 63.424453 | 15,907 / 3.077702 |
| prepared full resolution | A | 5,434,572 / 370.934786 | 5,434,136 / 370.926591 | 90,409 / 15.706299 | 90,409 / 15.706299 |
| prepared full resolution | B | 5,434,572 / 370.934786 | 5,434,136 / 370.926591 | 90,409 / 15.706299 | 90,409 / 15.706299 |

Cold peak-heap ratios равны `0.048543` и `0.048525`. В prepared scenarios
snapshot уже открыт до запуска `dhat`, поэтому одинаковый operation-local peak
ожидаем и не используется как подмена process RSS gate.

### Поведенческая эквивалентность

Во всех 24 процессах:

- effective context: `1798`, SHA-256
  `4006d1c39dd3f767f2d8f2f88917123df4215dd091b146c6d27b201fa628478f`;
- full resolution: `2490 / 2286 / 204`, SHA-256
  `b37bd7885b01262821fb4f929a8ad576fc53de23c61264adfaeff82552bd3287`.

Downstream regression дополнительно публикует minimal X1, удаляет source HBK и
provider SQLite, затем открывает stable slot и выполняет реальный BSL catalog
traversal. Structural guards запрещают owned HBK entity records в production
`context-provider`, отдельный X1 reader и H0 fallback в X1 benchmark arm.

## OpenSpec 4.6 — применение gates

| Gate | Pair A | Pair B | Результат |
|---|---:|---:|---|
| cold median `<= 0.50x` H0 | 0.422186 | 0.417985 | PASS |
| prepared handle median `<= 1.10x` H0 | 0.986441 | 0.938241 | PASS |
| prepared full resolution median `<= 1.05x` H0 | 0.907735 | 0.924985 | PASS |
| prepared handle RSS `< H0` | 0.371988 | 0.369987 | PASS |
| cold handle RSS `< H0` | 0.347861 | 0.348584 | PASS |
| prepared full resolution RSS `< H0` | 0.522318 | 0.515379 | PASS |
| cold peak heap `< H0` | 0.048543 | 0.048525 | PASS |

Итог: все semantic, lifecycle, allocation, wall-time, process RSS и cold heap
gates прошли отдельно в A и B. Aggregate score и waiver не применялись.
Разрешён отдельный reviewed canonical cutover OpenSpec 5.1; до его завершения
H0 runtime code не удаляется.
