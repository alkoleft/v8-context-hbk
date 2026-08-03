# Свидетельства X1-INT

Статус: выполняется. X1 остаётся non-canonical; этот документ не разрешает
cutover и cleanup до полного pass OpenSpec 4.2–4.6 и всех gates ADR-0012.

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

Не закрыты этим результатом: полный lookup transcript 4.3, catalog/resolver
seam 4.4, analyzer A/B 4.5 и применение всех gates 4.6. Raw stdout остаётся
service data и не хранится в `spec/`.
