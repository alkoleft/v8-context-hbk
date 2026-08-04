# Raw-name typed primary-table lookup comparison

## Scope and provenance

Measured 2026-08-05 against the frozen Russian 8.3.27.1859 provider index:

- `source_hbk`: `/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk`
- locale: `ru`
- source extraction schema: `11`
- snapshot strings: `71,073`
- raw projected primary/alias names: `48,355`
- raw projected name payload: `1,159,115` bytes

Command, executed twice after the final `Box<str>` optimization:

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
- release profile; each run executed alone
- two warm-up samples and nine measured samples per reported median
- each lookup sample traversed its fixed raw query list 64 times

Both lanes receive the same raw `&str`, execute `normalize_lookup_key` inside
the timed call and consume native result IDs. Pre-timing differential checks
map both representations to canonical source entities. They passed for every
eligible query; semantic mismatch count is zero.

The baseline is the current in-memory lookup mechanics: one merged vector of
normalized snapshot `StringId` entries, with comparisons through
`snapshot.string(entry.key)`. The candidate contains no `StringId`: type
primary names are a sorted `Vec<Box<str>>` whose row index is `TypeId`; scoped
callable/property primaries are completed IDs compared through their separate
family name table; aliases are separate owned-text entries.

## Corpus boundary and temporary uniqueness evidence

| Family | Source rows | Canonical rows | Temporarily dropped primary duplicates | Retained aliases | Eligible primary queries | Eligible alias queries | Excluded ASCII primaries | Excluded non-ASCII aliases |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Type | 2,419 | 2,416 | 3 | 2,412 | 2,380 | 2,410 | 36 | 0 |
| Callable | 8,253 | 8,248 | 5 | 7,675 | 8,101 | 7,668 | 147 | 7 |
| Property | 13,727 | 13,726 | 1 | 13,219 | 13,147 | 12,971 | 574 | 235 |

The experiment also executes two missing routes for every owner scope. The
missing raw strings come from existing snapshot facts and their normalized
text was absent from all retained primary and alias names. Five property
primary/alias collisions are outside the routed bilingual query set and are
reported as coverage evidence.

The non-zero duplicate counts mean production cutover remains blocked until
HBK formation and extension composition establish scoped primary uniqueness.
The experiment temporarily retains the first source row before assigning the
new typed IDs.

## Construction and retained storage

The table uses the second final run. Retained bytes are deterministic across
both runs. Candidate bytes include every owned `Box<str>` payload and header,
scoped primary ID and alias entry. Current bytes exclude the already-owned
snapshot string table, matching the incremental current index cost.

| Family | Variant | Median build, ns | Retained bytes | Allocation calls | Allocated bytes | Live-byte growth |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Type | current raw-name lookup | 650,090 | 38,640 | 1 | 38,640 | 38,640 |
| Type | routed typed table | 1,260,125 | 303,269 | 4,831 | 405,685 | 303,269 |
| Callable | current raw-name lookup | 1,675,698 | 196,368 | 1 | 196,368 | 196,368 |
| Callable | routed typed table | 3,741,140 | 550,243 | 10,651 | 784,627 | 550,243 |
| Property | current raw-name lookup | 3,287,247 | 323,340 | 1 | 323,340 | 323,340 |
| Property | routed typed table | 6,942,451 | 856,017 | 18,715 | 1,280,449 | 856,017 |

Same-run candidate impact:

| Family | Build time | Retained bytes | Allocated bytes |
| --- | ---: | ---: | ---: |
| Type | +93.8% | +684.9% | +949.9% |
| Callable | +123.3% | +180.2% | +299.6% |
| Property | +111.2% | +164.7% | +296.0% |

Construction time was the least stable metric between the two isolated runs:
same-run candidate overhead ranged from +93.8% to +106.0% for types, +110.9%
to +123.3% for callables and +111.2% to +128.8% for properties. The direction
is nevertheless unambiguous: constructing and owning the candidate strings is
materially more expensive than building the current compact entry vector.

The final immutable-string and exact-capacity pass was material: compared with
the preliminary `Vec<String>` candidate, it reduced retained bytes by 11.3%
for types, 15.3% for callables and 14.9% for properties. Avoiding clone-per-row
during name-table formation reduced allocated bytes by 35.0%, 42.7% and 42.2%
respectively. It does not change the conclusion relative to current storage.

## End-to-end raw-name lookup latency

Values are median nanoseconds per query from the second final run. Negative
impact means the routed typed-table candidate was faster. Normalization and its
allocation are deliberately included in both lanes.

| Family | Query class | Queries | Current, ns | Candidate, ns | Candidate impact |
| --- | --- | ---: | ---: | ---: | ---: |
| Type | Primary | 2,380 | 1,329 | 1,275 | -4.1% |
| Type | Alias | 2,410 | 711 | 626 | -12.0% |
| Type | Missing | 2 | 1,127 | 1,099 | -2.5% |
| Callable | Primary | 8,101 | 920 | 905 | -1.6% |
| Callable | Alias | 7,668 | 351 | 339 | -3.4% |
| Callable | Missing / owner-scoped | 2,280 | 1,166 | 1,189 | +2.0% |
| Callable | Owner isolation | 11,674 | 519 | 504 | -2.9% |
| Property | Primary | 13,147 | 745 | 742 | -0.4% |
| Property | Alias | 12,971 | 348 | 330 | -5.2% |
| Property | Missing / owner-scoped | 3,926 | 1,185 | 1,135 | -4.2% |
| Property | Owner isolation | 19,043 | 546 | 506 | -7.3% |

Repeat stability bounds the claim:

- type primary hits were 4.1-5.4% faster and type alias hits were 4.6-12.0%
  faster in both final runs;
- callable/property primary hits were 0.4-5.3% faster and alias hits were
  1.5-5.2% faster in both runs;
- callable owner-isolation was 2.6-2.9% faster and property owner-isolation was
  7.3-10.4% faster in both runs;
- callable misses were 0.5-2.0% slower, property misses changed sign and type
  misses had only two queries, so no missing-query win is claimed;
- normalization dominates absolute latency, so the one-search routing gain is
  modest rather than structural.

## Conclusion

The identity hypothesis is mechanically viable:

- type primary-table position is the complete `TypeId`, with no `StringId`,
  no `Vec<TypeId>` membership allocation and no second lookup;
- callable/property IDs remain `(OwnerId, family name ordinal)` and their one
  scoped primary vector prevents false positives under another owner;
- primary and alias routes each execute one binary search from the same raw
  input and preserve eligible current semantics.

It is not an overall resource improvement for the current HBK snapshot. The
candidate buys small, sometimes noisy end-to-end lookup gains at the cost of
2.6-7.8 times the incremental retained lookup bytes and roughly 4-10.5 times
the construction allocation volume. The strongest repeatable latency results
are 5-12% on type aliases and property owner-isolation; most other classes
improve by only 0-5% or are neutral after including normalization.

Therefore this experiment supports the typed identity semantics but does not
support replacing the current shared snapshot string ownership with separately
owned primary/alias text in production as measured. A later production design
would need either shared immutable text storage without a generic public
`StringId`, or evidence that these name bytes replace rather than duplicate an
already required provider-owned name representation. That storage decision is
outside this hypothesis check.
