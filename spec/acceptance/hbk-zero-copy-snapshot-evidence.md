# T183 HBK Zero-Copy Snapshot Evidence

Status: unranked experiment evidence; user decision pending.

This file records the first consolidated measurement pass over the committed
T183 hypotheses. It does not assign a score, rank, winner or first place.
Passing a numeric threshold means only that one metric is within its frozen
gate. It does not override an incomplete mandatory parity, safety or lifecycle
gate.

## Evidence Identity And Ancestry

All rows use the frozen harness
`051df7979e3cf5f6431b4d13829f436c98c47054`, workload
`hbk-snapshot-warm-lookups/v2`, platform `8.5.1.1150`, locale `ru`, provider
schema `16` and extraction schema `11`.

The real-corpus inputs are:

- HBK size/SHA-256: `41,361,963` bytes /
  `b8bc0d3a1ee8d00e2f113a800339731304428cc35ae395e5094a8b022773f8ed`;
- provider SQLite size/SHA-256: `206,753,792` bytes /
  `cc9b2b8aaf31f64c880b92cc3a02fd3166541f10f8d209faf8c7a7c22cac0d55`.

| ID | Branch | Measured commit | Parent/relationship |
| --- | --- | --- | --- |
| `H0` | `experiment/hbk-zero-copy-base` | `051df7979e3cf5f6431b4d13829f436c98c47054` | SQL-to-owned baseline |
| `C0` | `experiment/hbk-zero-copy-base` | `051df7979e3cf5f6431b4d13829f436c98c47054` | Current-cache-to-owned control |
| `H1` | `experiment/hbk-zero-copy-flat-h1` | `a2431254ee5d90a6e77c877e329bbb8d0ca50e84` | Parent `79e0b17083b12c2a778cea0237632746fcdfb396` |
| `H2` | `experiment/hbk-zero-copy-flat-typed-h2` | `826991395a508e36b7a684dc987ead218ef27184` | Exact parent is measured H1 commit `a2431254ee5d...` |
| `H3` | `experiment/hbk-zero-copy-rkyv-h3` | `497afa52344fb318a4f27c94762cc7eafa1126ca` | Two commits from `79e0b17083b12c2a778cea0237632746fcdfb396` |

The measured candidate artifacts are:

| ID | Bytes | SHA-256 |
| --- | ---: | --- |
| `H1` | 17,691,072 | `606a31140be1614b424abfc8f77283985420efa6ae15e8223112b1d40c5ba863` |
| `H2` | 11,445,079 | `d86cbe8ef7fe2c47f46a895007170674ee28a150106c6511412e4b5c9561fb78` |
| `H3` | 14,097,196 | `4a2a2154652ba41f58c16f853e4c621d90af3e78e6810be2c0246b8f65f03a10` |

Generated service evidence is under
`target/hbk-zero-copy-experiment/results/`:

- baseline `raw-v1.jsonl` and `summary-051df79.json`/`.md`;
- H1 `raw-h1-a243125.jsonl` and `summary-h1-a243125.json`/`.md`;
- H2 `raw-h2-8269913.jsonl` and `summary-h2-8269913.json`/`.md`;
- H3 `raw-h3-497afa.jsonl` and `summary-h3-497afa.json`/`.md`.

## Interpretation Limits

- H1's workload row is descriptive only. Three operations produce the wrong
  totals, and the reviewed path uses non-equivalent count/normalization
  behavior. H1 workload latency and per-operation latency are not admissible
  comparisons.
- H2 preserves all 25 frozen workload totals and uses the frozen mixed-case
  inputs, but its mapped oracle covers only counts, strings and a lookup smoke
  subset. Independent review also found that the `ModuleEventNames` validation
  comparator orders owner IDs numerically while the owned index is sorted by
  owner text, so valid owned order can be rejected before the module-event
  parity surface is proven.
- H3 preserves all 25 frozen workload totals. A complete owned-adapter oracle
  matches the frozen digests, but the full oracle is not traversed through the
  borrowed mapped view and reconstructs an owned graph in the parity process.
  Checked archive access is used, but validation still does not prove sorted
  order for every name/id lookup array consumed by binary search.
- The real corpus contains zero language facts. It cannot by itself prove
  language, ambiguity or unsupported-outcome parity; the mandatory
  real-derived fixtures remain required.
- Exact per-section, dictionary and reverse-index byte footprints were not
  emitted by the candidate reports. Artifact sizes are exact, but those
  internal footprint fields remain unavailable and are an evidence gap.
- Candidate producer allocation profiles were not instrumented. The
  production allocation gates remain unevaluated for H1, H2 and H3.

## Startup, Lookup And Runtime

Values are median ± MAD over nine release processes. `Startup + first lookup`
is the median of the per-sample sum, not a sum of independently rounded table
medians.

| Metric | H0 SQL owned | C0 cache owned | H1 custom flat | H2 typed flat | H3 rkyv archive |
| --- | ---: | ---: | ---: | ---: | ---: |
| Warm ready, ms | 599.568 ± 7.419 | 41.764 ± 0.839 | 23.288 ± 0.214 | 55.195 ± 2.044 | 35.167 ± 1.089 |
| Cold-best-effort ready, ms | 1,707.429 ± 10.461 | 73.475 ± 2.774 | 44.448 ± 0.745 | 66.979 ± 0.474 | 48.908 ± 1.338 |
| Warm startup + first lookup, ms | 599.571 ± 7.419 | 41.767 ± 0.839 | 23.294 ± 0.216 | 55.199 ± 2.045 | 35.171 ± 1.087 |
| Cold startup + first lookup, ms | 1,707.431 ± 10.462 | 73.478 ± 2.774 | 44.456 ± 0.747 | 66.984 ± 0.475 | 48.914 ± 1.339 |
| Warm first lookup, µs | 2.703 ± 0.214 | 2.902 ± 0.113 | 7.154 ± 0.717 | 4.608 ± 0.307 | 4.173 ± 0.485 |
| Cold first lookup, µs | 2.751 ± 0.171 | 2.997 ± 0.125 | 7.874 ± 1.861 | 4.801 ± 0.348 | 3.814 ± 0.194 |
| Warm anchor resolution, µs | 15.458 ± 0.559 | 15.066 ± 0.325 | 27.077 ± 2.221 | 16.866 ± 1.568 | 15.583 ± 0.511 |
| Cold anchor resolution, µs | 15.420 ± 0.230 | 15.252 ± 0.289 | 26.613 ± 3.333 | 16.519 ± 0.670 | 14.572 ± 0.470 |
| Warm workload, ms | 2,226.218 ± 8.489 | 2,125.346 ± 11.649 | 248.780 ± 4.680 † | 370.613 ± 3.750 | 4,729.164 ± 18.520 |
| Cold workload, ms | 2,222.610 ± 6.898 | 2,136.306 ± 12.465 | 243.069 ± 3.080 † | 370.430 ± 3.709 | 4,754.210 ± 15.430 |
| Peak RSS warm/cold, KiB | 75,500 / 75,156 | 35,200 / 35,200 | 19,328 / 19,328 | 13,312 / 13,312 | 15,872 / 15,744 |
| Open minor faults warm/cold | 18,584 / 18,585 | 7,708 / 7,708 | 271 / 271 | 183 / 184 | 219 / 218 |
| Open major faults warm/cold | 0 / 0 | 0 / 0 | 0 / 1 | 0 / 1 | 0 / 1 |
| Cold file-resident growth, bytes | 117,473,280 | 11,403,264 | 17,694,720 | 11,448,320 | 14,098,432 |

† H1 workload values are not behaviorally comparable.

The single-shot candidate first-lookups have `MAD / median > 5%`, which is
consistent with the timer/scheduler noise anticipated by the frozen absolute
budget. Every observed candidate first lookup remained below 25 µs; the
candidate maxima were 17.007 µs for H1, 6.452 µs for H2 and 5.401 µs for H3.
This explains the noisy field without changing the predeclared threshold.

## Steady Memory, Sharing And Allocations

`smaps_rollup` private includes private-clean file-backed pages and is not
equivalent to heap. Anonymous memory is therefore reported separately. Exact
file-backed attribution was not available from the frozen `smaps_rollup`
record.

| Metric, KiB unless noted | H0 | C0 | H1 | H2 | H3 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Warm after-open RSS/PSS/private | 70,308 / 68,428 / 68,408 | 24,284 / 22,312 / 22,292 | 19,352 / 17,886 / 17,872 | 13,364 / 11,898 / 11,884 | 15,820 / 14,354 / 14,340 |
| Warm post-workload RSS/PSS/private | 70,308 / 68,428 / 68,408 | 24,284 / 22,312 / 22,292 | 19,384 / 17,918 / 17,904 | 13,372 / 11,906 / 11,892 | 15,944 / 14,478 / 14,464 |
| Warm post-workload shared/anonymous | 1,896 / 65,744 | 1,972 / 19,824 | 1,480 / 144 | 1,480 / 160 | 1,480 / 260 |
| Cold post-workload RSS/PSS/private | 70,308 / 68,422 / 68,400 | 24,264 / 22,312 / 22,292 | 19,376 / 17,910 / 17,896 | 13,396 / 11,930 / 11,916 | 15,944 / 14,478 / 14,464 |
| Four-reader aggregate PSS/private | 265,794 / 262,988 | 82,021 / 79,396 | 18,386 / 568 | 12,848 / 1,036 | 15,300 / 1,040 |
| Runtime allocation calls to ready | 1,291,557 | 137,633 | 8 | 66,698 | 153,416 |
| Runtime allocated bytes to ready | 154,852,833 | 29,278,447 | 4,484 | 4,662,246 | 6,856,044 |
| Final/peak live allocator bytes | 22,409,823 / 63,498,795 | 17,942,214 / 29,274,971 | 2,964 / 4,500 | 2,965 / 8,181 | 2,316 / 3,852 |

## Artifact Production

| Row | Total local rebuild | Materialize/encode | Serialize | Write/publish | Validation | Peak RSS | Artifact |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| C0 | 675.933 ± 10.180 ms | 594.571 ± 2.660 ms | included | 73.492 ± 20.710 ms | included | 81,356 KiB | 11,325,758 B |
| H1 | 1,064.891 ± 8.750 ms | unavailable | unavailable | unavailable | unavailable | 115,436 KiB | 17,691,072 B |
| H2 | 2,188.519 ± 29.555 ms | 1,923.008 ± 12.309 ms encode | included | 204.104 ± 14.488 ms residual | 55.958 ± 3.694 ms | 101,716 ± 132 KiB | 11,445,079 B |
| H3 | 1,985.567 ± 15.508 ms | 601.640 ± 3.766 ms | 22.982 ± 0.214 ms | 230.377 ± 15.446 ms | unavailable | 87,992 ± 4 KiB | 14,097,196 B |

H2 does not time write directly. Its residual is external process time minus
candidate-reported encode and validation and therefore also includes process
startup/exit, lock and reporting overhead. H3's internal total includes work
outside its separately reported materialize/serialize/write phases.

## Dictionary And Batched Operations

The dictionary medians are average nanoseconds per operation:

| Row | Forward by ID warm/cold | Reverse hit warm/cold | Reverse miss warm/cold |
| --- | ---: | ---: | ---: |
| C0 | 0 / 1 | 946 / 962 | 49,871 / 49,217 |
| H1 | 19 / 20 † | 763 / 738 † | 567 / 552 † |
| H2 | 23 / 23 | 834 / 830 | 600 / 610 |
| H3 | 3 / 3 | 2,189 / 2,185 | 115,513 / 116,035 |

The complete warm/cold operation table is:

| Operation, average ns | C0 warm/cold | H1† warm/cold | H2 warm/cold | H3 warm/cold |
| --- | ---: | ---: | ---: | ---: |
| availability_by_fact | 111 / 110 | 281 / 270 | 268 / 265 | 107 / 107 |
| callable_by_owner_name | 324 / 320 | 337 / 347 | 630 / 645 | 480 / 483 |
| constructors_by_type | 15 / 15 | 43 / 42 | 49 / 50 | 15 / 15 |
| dictionary_by_id | 0 / 1 | 19 / 20 | 23 / 23 | 3 / 3 |
| dictionary_by_value | 946 / 962 | 763 / 738 | 834 / 830 | 2,189 / 2,185 |
| dictionary_by_value_miss | 49,871 / 49,217 | 567 / 552 | 600 / 610 | 115,513 / 116,035 |
| enum_by_name | 198 / 198 | 349 / 355 | 534 / 531 | 440 / 444 |
| enum_by_name_miss | 347 / 348 | 355 / 334 | 678 / 664 | 628 / 627 |
| exact_fact_id | 91 / 102 | 1,297 / 1,265 | 1,389 / 1,398 | 395 / 403 |
| exact_fact_id_miss | 71 / 72 | 2,006 / 1,979 | 2,058 / 2,041 | 406 / 405 |
| global_by_domain_name_kind | 314 / 317 | 742 / 724 | 940 / 968 | 551 / 558 |
| language_by_name | 183 / 181 | 6 / 5 | 194 / 196 | 228 / 237 |
| language_by_name_miss | 357 / 352 | 5 / 5 | 372 / 369 | 414 / 423 |
| member_by_owner_name_kind | 231 / 228 | 372 / 358 | 553 / 566 | 325 / 327 |
| members_by_owner | 19 / 19 | 72 / 68 | 100 / 94 | 19 / 19 |
| module_context_by_kind | 595 / 615 | 1,253 / 1,199 | 2,011 / 2,026 | 482 / 493 |
| query_field_by_table_name | 224 / 220 | 237 / 224 | 438 / 446 | 360 / 361 |
| query_param_by_table_name | 335 / 336 | 156 / 152 | 501 / 501 | 432 / 440 |
| query_table_by_identifier | 334 / 337 | 461 / 443 | 806 / 800 | 479 / 496 |
| query_table_by_name | 547 / 533 | 366 / 364 | 1,065 / 1,062 | 741 / 761 |
| query_table_by_syntax | 716 / 670 | 546 / 516 | 1,304 / 1,308 | 901 / 951 |
| relation_by_source_kind | 50,051 / 50,498 | 755 / 733 | 869 / 875 | 110,003 / 110,521 |
| type_by_name | 245 / 245 | 765 / 752 | 999 / 1,017 | 486 / 493 |
| type_by_name_miss | 355 / 350 | 397 / 375 | 725 / 719 | 636 / 665 |
| type_template_by_key | 112 / 109 | 204 / 195 | 390 / 386 | 211 / 221 |

H1 produces zero rather than 20,000 for `query_table_by_name`,
`query_table_by_syntax` and `type_template_by_key` in both stances. H2 and H3
match every C0 observed total. Applying the frozen per-operation ceiling, H2
exceeds 20 of 25 ceilings in each stance. H3 exceeds 19 of 25 warm ceilings
and 20 of 25 cold ceilings. Total workload time does not override these
individual non-regression failures.

## Relative Values

Each cell is relative to C0 / H0. Artifact and rebuild have no H0 value.

| Metric | H1 | H2 | H3 |
| --- | ---: | ---: | ---: |
| Warm ready | -44.2% / -96.1% | +32.2% / -90.8% | -15.8% / -94.1% |
| Cold ready | -39.5% / -97.4% | -8.8% / -96.1% | -33.4% / -97.1% |
| Warm first lookup | +146.5% / +164.7% | +58.8% / +70.5% | +43.8% / +54.4% |
| Warm workload | not comparable | -82.6% / -83.4% | +122.5% / +112.4% |
| Runtime allocations to ready | -100.0% / -100.0% | -51.5% / -94.8% | +11.5% / -88.1% |
| Runtime allocated bytes | -100.0% / -100.0% | -84.1% / -97.0% | -76.6% / -95.6% |
| Peak RSS | -45.1% / -74.4% | -62.2% / -82.4% | -54.9% / -79.0% |
| Warm post-workload PSS | -19.7% / -73.8% | -46.6% / -82.6% | -35.1% / -78.8% |
| Four-reader PSS | -77.6% / -93.1% | -84.3% / -95.2% | -81.3% / -94.2% |
| Artifact bytes | +56.2% / n/a | +1.1% / n/a | +24.5% / n/a |
| Total local rebuild | +57.5% / n/a | +223.8% / n/a | +193.8% / n/a |

## Mandatory Correctness And Lifecycle Gates

| Gate | H1 | H2 | H3 |
| --- | --- | --- | --- |
| Full mapped canonical content and lookup files | Incomplete: smoke only | Incomplete: smoke only | Incomplete: matching digests are from owned adapter, not mapped view |
| Canonical digests | Not produced | Not produced | Owned adapter matches `000c78a7...` / `76b7ae21...`; mapped gate still incomplete |
| Sequential and four-reader canonical transcripts | Incomplete | Incomplete | Incomplete |
| No SQLite/HBK fallback after mapped open | Incomplete: no full probe | Incomplete: no full probe | Incomplete: no full mapped probe |
| 25 workload observed totals | Fail: three mismatches | Pass | Pass |
| Exact HBK/provider/platform/schema identity | Fail: exact provider SQLite identity missing | Pass | Pass |
| Structural validation before typed access | Fail: shallow record/reference/tag validation | Incomplete: strong section/reference/order checks, but `ModuleEventNames` owner-order validation does not match the owned text-order contract | Incomplete: source-locale and several indexes are checked, but not every binary-searched name/id array is proven sorted |
| Read-only mapping and owned mapping lifetime | Pass for tested mapped path | Pass | Pass through checked `rkyv::access` |
| Fail-fast typed writer lock | Partial: helper typed, producer path generic | Partial: typed producer, but self-validation runs after lock release | Pass for separate slot producer/open APIs |
| Integrated rebuild-before-map lifecycle | Not implemented | Not implemented | Not implemented |
| No complete owned runtime mirror | Pass for measured runtime | Pass for measured runtime | Pass for measured runtime; parity tool separately constructs an owned mirror |
| Language/ambiguity/unsupported fixture parity | Incomplete | Incomplete | Incomplete |

No candidate completes every mandatory correctness and lifecycle gate.

## Frozen Numeric Gate Matrix

`Pass` below means only that the recorded numeric value is within that one
frozen threshold. `Fail` means it is outside. `Not evaluated` means the
required comparable or instrumented evidence is absent.

### Material-Benefit Gates

| Gate | H1 | H2 | H3 |
| --- | --- | --- | --- |
| Warm ready ≤ 33,410,942 ns | Pass: 23,287,938 | Fail: 55,194,658 | Fail: 35,167,009 |
| Cold ready ≤ 58,780,152 ns | Pass: 44,447,777 | Fail: 66,978,928 | Pass: 48,908,203 |
| Runtime allocation calls ≤ 68,816 | Pass: 8 | Pass: 66,698 | Fail: 153,416 |
| Runtime allocated bytes ≤ 14,639,223 | Pass: 4,484 | Pass: 4,662,246 | Pass: 6,856,044 |
| Peak RSS ≤ 29,920 KiB | Pass: 19,328 | Pass: 13,312 | Pass: 15,872 |
| Warm PSS ≤ 17,849 KiB | Fail: 17,918 | Pass: 11,906 | Pass: 14,478 |
| Warm private ≤ 17,833 KiB | Fail: 17,904 | Pass: 11,892 | Pass: 14,464 |
| Cold PSS ≤ 17,849 KiB | Fail: 17,910 | Pass: 11,930 | Pass: 14,478 |
| Cold private ≤ 17,833 KiB | Fail: 17,896 | Pass: 11,916 | Pass: 14,464 |
| Four-reader PSS ≤ 65,616 KiB | Pass: 18,386 | Pass: 12,848 | Pass: 15,300 |
| Reverse dictionary hit ≤ 473 ns | Fail: 763 | Fail: 834 | Fail: 2,189 |
| Reverse dictionary miss ≤ 24,935 ns | Pass: 567 | Pass: 600 | Fail: 115,513 |

### Non-Regression And Resource Gates

| Gate | H1 | H2 | H3 |
| --- | --- | --- | --- |
| First lookup median ≤ 25,000 ns, both stances | Pass: 7,154 / 7,874 | Pass: 4,608 / 4,801 | Pass: 4,173 / 3,814 |
| Anchor median ≤ 25,000 ns, both stances | Fail: 27,077 / 26,613 | Pass: 16,866 / 16,519 | Pass: 15,583 / 14,572 |
| Warm/cold total workload ceiling | Not evaluated: workload mismatch | Pass: 370.613 / 370.430 ms | Fail: 4,729.164 / 4,754.210 ms |
| All observed totals match | Fail: 3 of 25 differ | Pass | Pass |
| Every per-operation ceiling | Not evaluated | Fail: 20 of 25 in both stances | Fail: 19 warm / 20 cold of 25 |
| Forward dictionary absolute ≤ 10 ns | Fail: 19 / 20 | Fail: 23 / 23 | Pass: 3 / 3 |
| Open major faults remain zero | Fail: 0 / 1 | Fail: 0 / 1 | Fail: 0 / 1 |
| Open minor faults ≤ 9,635 | Pass: 271 | Pass: 184 | Pass: 219 |
| Cold file-resident growth ≤ 14,254,080 B | Fail: 17,694,720 | Pass: 11,448,320 | Pass: 14,098,432 |
| Artifact ≤ 14,157,197 B | Fail: 17,691,072 | Pass: 11,445,079 | Pass: 14,097,196 |
| Total local rebuild ≤ 844,916,105 ns | Fail: 1,064,890,508 | Fail: 2,188,519,416 | Fail: 1,985,567,453 |
| Production peak RSS ≤ 101,695 KiB | Fail: 115,436 | Fail: 101,716 | Pass: 87,992 |
| Production allocation calls/bytes/peak | Not evaluated | Not evaluated | Not evaluated |

## Full Behavioral Equivalence Breakdown

Full equivalence is not a single count check. It requires all of the following:

1. Storage identity: exact source HBK, provider SQLite, locale, platform,
   provider schema and extraction schema are checked before typed access.
2. Logical content: every observable field in platform types, members,
   callables, signatures, parameters, globals, module contexts/events,
   language facts, enums/values and query tables/fields/parameters is
   normalized from local IDs to text/logical fact identity and compared.
3. Ordered children: signatures, parameters, type references, overloads,
   members, constructors, enum values, query fields/parameters, availability
   and relations preserve their contract-visible order.
4. Lookup behavior: exact/name/alias/template/owner/kind/global/module/
   language/enum/query/availability/relation hits and misses preserve
   cardinality, ordering and typed results.
5. Resolver behavior: ambiguity, unsupported and not-found outcomes match
   through `HbkFactReadHandle`, borrowed BSL/SDBL catalogs,
   `PlatformSnapshotSource` and `QueryTableSnapshotSource`.
6. Concurrency and fallback: sequential and concurrent transcripts are
   byte-identical, and lookups still pass after SQLite/HBK are made
   unavailable to the already-open probe.
7. Ownership: the measured runtime keeps only mapped provider state, not a
   second complete owned graph.

H1 proves none of the complete logical/adapter layers and also fails the
25-operation smoke. H2 proves the 25-operation manifest and broad mapped-file
validation, but not the full content/catalog/adapter/concurrency oracle, and
one reviewed module-event ordering check does not match the owned text-order
contract. H3
proves the complete normalized files only through a reconstructed owned
adapter, not through the mapped read surface, and does not yet validate sorted
order for every lookup array used by binary search. Therefore no candidate has
established full behavioral equivalence.

Numeric string and fact IDs are deliberately absent from equivalence. They are
generation-scoped and session-local. Replacing the HBK source creates new IDs;
the experiment neither persists nor migrates them.

## Downstream Handoff While The Decision Is Pending

The downstream change
`v8-context/openspec/changes/establish-unified-semantic-entity-model` may rely
only on this provisional boundary:

- `hbk_dependency_state`: provisional;
- `base_dictionary_scope`: provider-owned immutable HBK strings exposed
  through borrowed generation-scoped IDs; physical snapshot format unselected;
- `string_identity_scope`: generation-scoped and session-local;
- `identity_policy`: no persistent universal IDs and no cross-generation ID
  reuse;
- `overlay_owner`: outside HBK, owned by the downstream BSL/metadata side;
- `promotion_state`: no winner selected and no canonical zero-copy artifact;
- `compatibility_gate`: wrong source, locale or platform version requires
  rebuild before any mapping is exposed.

Downstream design text must not imply that a memory-mapped base dictionary,
borrowed handoff implementation or compact materialization has already been
accepted as the production architecture. Those remain candidate properties
until the user makes a selection and a durable HBK decision accepts it.

## Decision State

No candidate is currently eligible under all frozen gates. No candidate branch
is merged, deleted, promoted or marked first. The current owned snapshot/cache
path remains canonical only because the experiment has not produced an
accepted replacement and the user has not selected an outcome.
