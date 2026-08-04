# Primary / Alias Lookup Comparison

## Scope and command

Measured 2026-08-04 with the feature-gated snapshot experiment only. The
provider corpus was the frozen Russian 8.3.27.1859 index:

- `source_hbk`: `/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk`
- `locale`: `ru`
- source extraction schema: `11`
- normalized experiment keys: `19,725`
- normalized key payload: `645,822` bytes, shared input preparation and not
  charged to either lookup variant

Command:

```text
V8_CONTEXT_HBK_PRIMARY_ALIAS_INDEX=/home/alko/develop/open-source/v8-maintain-projects/v8-context/.v8-context/platform-indexes/8.3.27.1859/shcntx_ru.sqlite \
cargo test -p syntax-helper-search --release \
  --features snapshot-experiment-alloc \
  snapshot::primary_alias_lookup_experiment::primary_alias_lookup_real_corpus \
  -- --ignored --exact --nocapture
```

Environment:

- `rustc 1.95.0 (59807616e 2026-04-14)`, LLVM 22.1.2
- `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`
- Linux 6.8.0-111-generic, x86_64
- Intel Core i7-4770, 4 cores / 8 threads, 3.40 GHz, 8 MiB L3
- release profile; the test ran alone
- two warm-up samples and nine measured samples; every lookup sample traversed
  the fixed query list 64 times

The dense baseline models current independent four-byte ordinals plus one
merged primary/alias index. The candidate models `TypeId` as the interned type
primary and `CallableId` / `PropertyId` as `(OwnerId, family-name-token)` plus
one generic primary-first/alias-fallback lookup. Construction numbers are the
full model costs, not the isolated cost of splitting indexes.

## Corpus and uniqueness evidence

| Family | Source rows | Canonical rows | Temporarily dropped duplicate primaries | Supplied aliases | Redundant primary-equal aliases | Retained alias entries |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Type | 2,419 | 2,416 | 3 | 2,414 | 2 | 2,412 |
| Callable | 8,253 | 8,248 | 5 | 7,807 | 132 | 7,675 |
| Property | 13,727 | 13,726 | 1 | 13,726 | 507 | 13,219 |

The target uniqueness invariant is false in the current frozen source for all
three families. The same stable first-row canonicalization was applied before
both variants. This is experiment scaffolding only. A production cutover is
blocked until HBK formation/extension composition establishes scoped primary
uniqueness.

## Construction and retained storage

Medians are nanoseconds. Allocation observations come from the existing single
snapshot experiment allocator. `peak_live_bytes_growth` was `0` for every row
because the already-live materialized snapshot had established a higher
process-global peak; the per-construction `live_bytes_growth` is the meaningful
retained allocation observation here.

| Family | Variant | Median build, ns | Retained identity + lookup bytes | Allocation calls | Allocated bytes | Live-byte growth |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Type | dense merged | 114,596 | 38,640 | 1 | 38,640 | 38,640 |
| Type | composite primary/alias | 201,222 | 57,968 | 15 | 131,836 | 57,968 |
| Callable | dense merged | 619,672 | 196,368 | 1 | 196,368 | 196,368 |
| Callable | composite primary/alias | 849,114 | 285,600 | 15 | 359,468 | 285,600 |
| Property | dense merged | 1,056,729 | 323,340 | 1 | 323,340 | 323,340 |
| Property | composite primary/alias | 1,454,544 | 475,056 | 16 | 622,668 | 475,056 |

Relative candidate impact:

| Family | Build time | Retained index bytes | Allocated bytes | Live-byte growth |
| --- | ---: | ---: | ---: | ---: |
| Type | +75.6% | +50.0% | +241.2% | +50.0% |
| Callable | +37.0% | +45.4% | +83.1% | +45.4% |
| Property | +37.6% | +46.9% | +92.6% | +46.9% |

The retained-byte increase includes the candidate's per-entity identity table,
family-local primary-name tokens and alias entries. The transient construction
allocation increase is larger because each candidate family builds its private
primary-name interner. There is no shared member-name or identity registry.

## Lookup latency

Values are median nanoseconds per pre-normalized query. Common normalization
and query-key preparation are outside both timed paths. Checksums consume the
native old/new ID layouts and therefore differ; exact differential assertions
map both results to the canonical source entity outside timed paths.

| Family | Query class | Queries | Dense merged, ns | Composite primary/alias, ns | Candidate impact |
| --- | --- | ---: | ---: | ---: | ---: |
| Type | primary | 2,416 | 51 | 49 | -3.9% |
| Type | alias-only | 2,410 | 54 | 75 | +38.9% |
| Type | missing | 1 | 19 | 47 | +147.4% |
| Callable | primary | 8,248 | 100 | 84 | -16.0% |
| Callable | alias-only | 7,675 | 100 | 110 | +10.0% |
| Callable | missing / owner scoped | 1,140 | 112 | 115 | +2.7% |
| Callable | owner isolation | 11,722 | 118 | 114 | -3.4% |
| Property | primary | 13,721 | 107 | 88 | -17.8% |
| Property | alias-only | 13,206 | 107 | 122 | +14.0% |
| Property | missing / owner scoped | 1,963 | 123 | 119 | -3.3% |
| Property | primary/alias collision | 5 | 100 | 83 | -17.0% |
| Property | owner isolation | 19,522 | 133 | 129 | -3.0% |

There were no real-corpus primary/alias collisions for types or callables and
five for properties. Deterministic fixtures cover all collision semantics,
including alias-to-alias ambiguity. For the five property collisions, the old
merged index returns the combined set while the candidate intentionally returns
the primary composite ID only.

The single-query type-miss result is too small to generalize. The higher-volume
result is consistent: primary lookup is roughly neutral for types and 16–18%
faster for callables/properties; alias fallback costs about 10–14% for member
families and 39% for types because it performs the primary check before the
alias range.

## Conclusion

The composite identity scenario is mechanically viable:

- the same callable/property primary text reuses its family-local name token;
- `OwnerId::Global` and distinct `OwnerId::Type(TypeId)` values keep global,
  array, table and other same-name members distinct;
- aliases allocate no identity token and preserve one-to-many ambiguity;
- one private generic primary-first/alias-fallback mechanism serves all three
  independent family states;
- the lookup hot path allocates nothing.

The hypothesis is not ready for production cutover. Formation-time duplicate
primaries remain the blocking semantic issue. If that invariant is established
in `establish-cross-source-type-system`, a follow-up production design can
weigh roughly 45–47% additional retained member identity-plus-lookup bytes and
higher build allocation against faster primary member lookup and the explicit
collision semantics. This experiment does not justify changing
snapshot/X1/public IDs by itself.
