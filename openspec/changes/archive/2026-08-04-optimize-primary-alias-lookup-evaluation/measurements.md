# Optimized Primary / Alias Lookup Comparison

## Scope and provenance

Measured 2026-08-04 with the feature-gated snapshot experiment only. The
provider corpus was the frozen Russian 8.3.27.1859 index:

- `source_hbk`: `/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk`
- `locale`: `ru`
- source extraction schema: `11`
- snapshot strings: `71,073`
- prepared snapshot-owned key IDs: `19,724`
- prepared normalized key payload: `645,786` bytes; common corpus preparation,
  excluded from both measured variants

Command:

```text
V8_CONTEXT_HBK_PRIMARY_ALIAS_INDEX=/home/alko/develop/open-source/v8-maintain-projects/v8-context/.v8-context/platform-indexes/8.3.27.1859/shcntx_ru.sqlite \
cargo test -p syntax-helper-search --release \
  --features snapshot-experiment-alloc \
  snapshot::primary_alias_lookup_experiment::primary_alias_lookup_real_corpus \
  -- --ignored --exact --nocapture
```

Environment:

- `rustc 1.95.0 (59807616e 2026-04-14)`
- `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`
- Linux 6.8.0-111-generic, x86_64
- Intel Core i7-4770, 4 cores / 8 threads, 3.40 GHz, 8 MiB L3
- release profile; each benchmark run ran alone
- two warm-up samples and nine measured samples; every lookup sample traversed
  the fixed query list 64 times

The control was captured immediately before the implementation from commit
`0cb8ad0`. The optimized experiment was run twice after implementation. Lookup
medians were stable to within 0-3 ns except for the single type-miss query;
tables below use the second optimized run. Cross-run comparisons are labelled
and should not be read as paired microbenchmarks.

The dense lane is a deliberately independent merged reference over prepared
snapshot `StringId` keys. It is not current public production lookup latency,
because public lookup also prepares text and compares through the snapshot
string table.

## Corpus and invariant evidence

| Family | Source rows | Canonical rows | Temporarily dropped duplicate primaries | Supplied aliases | Redundant primary-equal aliases | Retained aliases |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Type | 2,419 | 2,416 | 3 | 2,414 | 2 | 2,412 |
| Callable | 8,253 | 8,248 | 5 | 7,807 | 132 | 7,675 |
| Property | 13,727 | 13,726 | 1 | 13,726 | 507 | 13,219 |

The counters are identical to the control. The target primary-name uniqueness
invariant is still false in the frozen source for all three families. Stable
first-row canonicalization remains temporary experiment scaffolding. Production
cutover remains blocked until HBK formation and extension composition establish
scoped primary uniqueness.

## Optimized construction and retained storage

Medians are nanoseconds. `peak_live_bytes_growth` was zero for every row
because the materialized snapshot had already established a higher
process-global peak; retained bytes and `live_bytes_growth` are the meaningful
per-construction observations.

| Family | Variant | Median build, ns | Retained bytes | Allocation calls | Allocated bytes | Live-byte growth |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Type | dense merged prepared-`StringId` reference | 119,272 | 38,640 | 1 | 38,640 | 38,640 |
| Type | direct composite primary/alias | 208,691 | 38,640 | 13 | 112,508 | 38,640 |
| Callable | dense merged prepared-`StringId` reference | 660,308 | 196,368 | 1 | 196,368 | 196,368 |
| Callable | direct composite primary/alias | 839,623 | 196,368 | 13 | 270,236 | 196,368 |
| Property | dense merged prepared-`StringId` reference | 1,148,986 | 323,340 | 1 | 323,340 | 323,340 |
| Property | direct composite primary/alias | 1,530,881 | 323,340 | 14 | 470,952 | 323,340 |

Same-run direct-candidate impact relative to the dense reference:

| Family | Build time | Retained bytes | Allocated bytes | Live-byte growth |
| --- | ---: | ---: | ---: | ---: |
| Type | +75.0% | 0.0% | +191.2% | 0.0% |
| Callable | +27.2% | 0.0% | +37.6% | 0.0% |
| Property | +33.2% | 0.0% | +45.7% | 0.0% |

The candidate's retained state is now exactly two direct vectors of existing
`NameLookup<typed ID>` records. Its construction-only family token map accounts
for the remaining transient allocations and is dropped before lookup.

## Fresh-control to optimized construction comparison

This cross-run table isolates the effect of removing the over-modelled
candidate state. The control candidate retained a benchmark key pool, a name
interner, a primary membership vector and an entity-ID vector in addition to
its lookup entries.

| Family | Metric | Control candidate | Optimized direct | Change |
| --- | --- | ---: | ---: | ---: |
| Type | Build, ns | 200,116 | 208,691 | +4.3% |
| Type | Retained bytes | 57,968 | 38,640 | -33.3% |
| Type | Allocated bytes | 131,836 | 112,508 | -14.7% |
| Type | Allocation calls | 15 | 13 | -13.3% |
| Callable | Build, ns | 775,693 | 839,623 | +8.2% |
| Callable | Retained bytes | 285,600 | 196,368 | -31.2% |
| Callable | Allocated bytes | 359,468 | 270,236 | -24.8% |
| Callable | Allocation calls | 15 | 13 | -13.3% |
| Property | Build, ns | 1,392,558 | 1,530,881 | +9.9% |
| Property | Retained bytes | 475,056 | 323,340 | -31.9% |
| Property | Allocated bytes | 622,668 | 470,952 | -24.4% |
| Property | Allocation calls | 16 | 14 | -12.5% |

The retained-memory result is decisive: removing duplicated representation
saves 31-33% versus the old candidate and leaves no retained-byte premium over
the dense reference. Cross-run build medians are 4-10% higher, while even the
dense reference drifted by 1-5%; construction is therefore not an optimization
win. The direct split still sorts two vectors and builds a transient token map.

## Optimized lookup latency

Values are median nanoseconds per pre-normalized query. Common normalization
and query-key preparation are outside both timed paths.

| Family | Query class | Queries | Dense reference, ns | Optimized direct, ns | Same-run impact |
| --- | --- | ---: | ---: | ---: | ---: |
| Type | Primary | 2,416 | 34 | 28 | -17.6% |
| Type | Alias-only | 2,410 | 35 | 51 | +45.7% |
| Type | Missing | 1 | 30 | 56 | +86.7% |
| Callable | Primary | 8,248 | 69 | 65 | -5.8% |
| Callable | Alias-only | 7,675 | 66 | 116 | +75.8% |
| Callable | Missing / owner-scoped | 1,140 | 60 | 113 | +88.3% |
| Callable | Owner isolation | 11,722 | 70 | 98 | +40.0% |
| Property | Primary | 13,721 | 71 | 66 | -7.0% |
| Property | Alias-only | 13,206 | 71 | 122 | +71.8% |
| Property | Missing / owner-scoped | 1,963 | 67 | 118 | +76.1% |
| Property | Primary/alias collision | 5 | 67 | 61 | -9.0% |
| Property | Owner isolation | 19,522 | 78 | 104 | +33.3% |

The primary path benefits from searching the smaller primary vector. Alias and
miss paths necessarily search both indexes under the selected primary-first
contract; the 72-88% member-family penalty against a one-search merged
reference is therefore structural, not evidence of a retained mirror. The
single type-miss observation is too small to generalize. Five property
collisions intentionally return the primary ID in the direct candidate while
the dense reference returns the combined range.

## Cross-run lookup comparison with the old candidate

| Family | Query class | Old candidate, ns | Optimized direct, ns | Change |
| --- | --- | ---: | ---: | ---: |
| Type | Primary | 51 | 28 | -45.1% |
| Type | Alias-only | 65 | 51 | -21.5% |
| Type | Missing | 34 | 56 | +64.7% |
| Callable | Primary | 86 | 65 | -24.4% |
| Callable | Alias-only | 109 | 116 | +6.4% |
| Callable | Missing / owner-scoped | 115 | 113 | -1.7% |
| Callable | Owner isolation | 111 | 98 | -11.7% |
| Property | Primary | 87 | 66 | -24.1% |
| Property | Alias-only | 122 | 122 | 0.0% |
| Property | Missing / owner-scoped | 122 | 118 | -3.3% |
| Property | Primary/alias collision | 88 | 61 | -30.7% |
| Property | Owner isolation | 130 | 104 | -20.0% |

The optimized primary path is 24-45% faster than the old candidate. Member
alias lookup is effectively unchanged to 6% slower, while misses and owner
isolation are mostly improved. These are cross-run comparisons and the dense
reference implementation also moved from copied range logic to the production
`matching_range`; the same-run table is the stronger statement about the
primary/alias split itself.

## Conclusion

The direct design is the optimal result within the tested sorted-vector,
primary-then-alias hypothesis:

- `TypeId` remains four bytes; `CallableId` and `PropertyId` remain eight-byte
  `(OwnerId, name token)` values;
- snapshot-owned normalized `StringId` values are used directly, without a
  second string interner or fabricated IDs;
- the only retained candidate state is one primary vector and one alias vector;
- retained memory equals the dense reference and is 31-33% below the previous
  candidate;
- primary lookup improves, but alias fallback keeps the intrinsic cost of two
  searches;
- lookup allocates nothing, and construction-only maps are dropped.

This does not prove a global optimum against hash tables, perfect hashing or a
different primary/alias contract. It also does not authorize a production
snapshot/API change. The remaining production blocker is the 3 type, 5
callable and 1 property duplicate primaries that HBK formation currently has to
discard temporarily.
