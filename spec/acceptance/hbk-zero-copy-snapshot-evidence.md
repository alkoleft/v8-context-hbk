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

## S83 F0/A0 Reference Evidence

The S83 reference pass uses platform `8.3.27.1859`, HBK SHA-256
`5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48`,
provider SQLite SHA-256
`55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab`,
provider schema `16`, extraction schema `11` and frozen harness
`28f29b5a262db362b6b58c8109e6df6c2afbbc44`.

Generated service evidence is under
`target/hbk-zero-copy-experiment-8.3.27.1859/results/`:

- numeric resource raw:
  `raw-S83-F0-A0-complete-360cbd9.jsonl`;
- auxiliary numeric summary:
  `summary-S83-F0-A0-complete-360cbd9.json` / `.md`;
- diagnostic pre-merge resource raw:
  `raw-S83-F0-A0-resource-061a242.jsonl`;
- storage parity:
  `raw-S83-F0-5eac531-parity-rerun-6aadd9b.jsonl`,
  `raw-S83-A0-2a14ed6-parity-6aadd9b.jsonl`;
- semantic parity:
  `raw-semantic-s83-f0-semantic-a9a98a1.jsonl`,
  `raw-semantic-s83-a0-semantic-36a41aa.jsonl`.

All 72 records in `raw-S83-F0-A0-complete-360cbd9.jsonl` are successful:
18 runtime timing rows, nine production timing rows, three runtime allocation
profiles, three production allocation profiles and three aggregate four-reader
rows for each of F0 and A0. The measured artifacts remain immutable files:

| ID | Branch | Semantic commit | Artifact | Bytes | SHA-256 | Mode |
| --- | --- | --- | --- | ---: | --- | --- |
| `S83-F0` | `experiment/hbk-zero-copy-83-flat-f0-semantic` | `a9a98a18ed2af21ba16573a00719c13edddac97b` | `s83-f0.5eac531.h2` | 11,304,567 | `20bc6ff8bf922b129233cafdcb4abbec51496697ee086788aacaf1eb00bd74b2` | `0444` |
| `S83-A0` | `experiment/hbk-zero-copy-83-archive-a0-semantic` | `36a41aa74a9c6898576706f34a9a403918d452e4` | `s83-a0.2a14ed6.a0` | 13,936,492 | `6fbd33ab0d58c2197e324b0b61193d873bc777def0087ae42b178cd8b53e00d1` | `0444` |

### S83 Startup, Lookup And Runtime

Values are median ± MAD over nine release processes.

| Metric | S83-F0 | S83-A0 |
| --- | ---: | ---: |
| Warm ready, ms | 54.358 ± 0.618 | 39.174 ± 0.445 |
| Cold-best-effort ready, ms | 67.899 ± 0.673 | 50.703 ± 0.121 |
| Warm first lookup, µs | 3.902 ± 0.074 | 2.872 ± 0.165 |
| Cold first lookup, µs | 4.149 ± 0.520 | 2.823 ± 0.174 |
| Warm anchor resolution, µs | 14.007 ± 0.438 | 11.915 ± 0.381 |
| Cold anchor resolution, µs | 14.750 ± 0.506 | 11.965 ± 0.412 |
| Warm workload, ms | 358.243 ± 2.417 | 4,638.727 ± 14.339 |
| Cold workload, ms | 356.781 ± 2.173 | 4,644.303 ± 13.769 |
| Peak RSS warm/cold, KiB | 13,184 / 13,184 | 15,744 / 15,616 |
| Warm post-workload PSS/private, KiB | 11,770 / 11,756 | 14,358 / 14,344 |
| Cold post-workload PSS/private, KiB | 11,770 / 11,756 | 14,354 / 14,340 |
| Open minor faults warm/cold | 182 / 182 | 216 / 216 |
| Open major faults warm/cold | 0 / 1 | 0 / 1 |
| Cold file-resident growth, bytes | 11,304,960 | 13,938,688 |

Both candidates preserve all 25 S83-C0 workload observed totals in warm and
cold-best-effort stances. Applying the frozen per-operation ceiling formula,
F0 exceeds 20 of 25 operation ceilings in each stance. A0 exceeds 20 of 25
warm ceilings and 19 of 25 cold ceilings.

### S83 Memory, Sharing And Allocations

| Metric | S83-F0 | S83-A0 |
| --- | ---: | ---: |
| Runtime allocation calls to ready | 66,266 | 151,855 |
| Runtime allocated bytes to ready | 4,633,385 | 6,791,316 |
| Final / peak live allocator bytes | 2,978 / 8,194 | 2,385 / 3,921 |
| Aggregate four-reader PSS/private, KiB | 12,342 / 664 | 15,180 / 1,044 |
| Aggregate four-reader RSS/shared/anonymous, KiB | 53,008 / 52,348 / 644 | 63,284 / 62,240 / 1,044 |

### S83 Artifact Production

| Metric | S83-F0 | S83-A0 |
| --- | ---: | ---: |
| Total local rebuild, ms | 3,272.623 ± 36.147 | 1,994.852 ± 23.196 |
| Materialize, ms | 584.768 ± 10.213 | 586.432 ± 5.495 |
| Write/publish, ms | 244.055 ± 32.800 | 258.073 ± 30.467 |
| Production peak RSS, KiB | 100,900 | 100,996 |
| Artifact bytes | 11,304,567 | 13,936,492 |
| Production allocation calls | 2,291,021 | 1,582,041 |
| Production allocated bytes | 281,173,378 | 203,711,633 |
| Production peak live bytes | 63,694,157 | 63,018,740 |

### S83 Behavioral And Safety Gates

| Gate | S83-F0 | S83-A0 |
| --- | --- | --- |
| Storage content and lookup parity | Pass: content `5f66d205...`, lookup `9b17c710...`, sequential plus four-reader readers, sources hidden before open |
| Semantic catalog/resolver parity | Pass: five transcripts, 742,872 records / 769,824,709 bytes, SHA-256 `1fe7f166...`, sources hidden before process and through replay |
| Exact platform/source/schema/header identity | Pass: platform `8.3.27.1859`, provider schema `16`, extraction schema `11`, exact HBK/provider hashes |
| No SQLite/HBK fallback after supplied-artifact open | Pass in storage and semantic gates with source-hidden probes |
| No complete owned runtime mirror | Pass for measured runtime and semantic adapter; F0 still decodes variable fact records on access |
| Immutable artifact and lock evidence | Pass: artifact mode `0444`, adjacent shared lock file |
| Workload observed totals | Pass: 25 of 25 in both stances |

### S83 Frozen Numeric Gate Matrix

`Pass` below means only that the recorded numeric value is within that one
frozen threshold. `Fail` means it is outside.

| Gate | S83-F0 | S83-A0 |
| --- | --- | --- |
| Warm ready ≤ 33,991,352 ns | Fail: 54,357,920 | Fail: 39,173,528 |
| Cold ready ≤ 59,020,968 ns | Fail: 67,899,017 | Pass: 50,703,490 |
| Runtime allocation calls ≤ 68,018 | Pass: 66,266 | Fail: 151,855 |
| Runtime allocated bytes ≤ 14,471,464 | Pass: 4,633,385 | Pass: 6,791,316 |
| Peak RSS ≤ 29,593 KiB | Pass: 13,184 | Pass: 15,744 |
| Warm PSS/private ≤ 17,712 / 17,696 KiB | Pass: 11,770 / 11,756 | Pass: 14,358 / 14,344 |
| Cold PSS/private ≤ 17,681 / 17,664 KiB | Pass: 11,770 / 11,756 | Pass: 14,354 / 14,340 |
| Four-reader PSS ≤ 64,913 KiB | Pass: 12,342 | Pass: 15,180 |
| Reverse dictionary hit ≤ 458 ns | Fail: 961 / 975 | Fail: 2,245 / 2,213 |
| Reverse dictionary miss ≤ 24,048 ns | Pass: 524 / 520 | Fail: 113,905 / 114,151 |
| First lookup ≤ 25,000 ns, both stances | Pass: 3,902 / 4,149 | Pass: 2,872 / 2,823 |
| Anchor resolution ≤ 25,000 ns, both stances | Pass: 14,007 / 14,750 | Pass: 11,915 / 11,965 |
| Warm/cold workload ceiling | Pass: 358.243 / 356.781 ms | Fail: 4,638.727 / 4,644.303 ms |
| Every per-operation ceiling | Fail: 20 of 25 in both stances | Fail: 20 warm / 19 cold of 25 |
| Forward dictionary absolute ≤ 10 ns | Fail: 23 / 23 | Pass: 3 / 3 |
| Open major faults remain zero | Fail: 0 / 1 | Fail: 0 / 1 |
| Open minor faults ≤ 9,525 | Pass: 182 | Pass: 216 |
| Cold file-resident growth ≤ 14,074,880 B | Pass: 11,304,960 | Pass: 13,938,688 |
| Artifact ≤ 13,982,571 B | Pass: 11,304,567 | Pass: 13,936,492 |
| Total local rebuild ≤ 803,548,621 ns | Fail: 3,272,622,772 | Fail: 1,994,852,136 |
| Production peak RSS ≤ 100,975 KiB | Pass: 100,900 | Fail: 100,996 |
| Production allocation calls ≤ 1,597,946 | Fail: 2,291,021 | Pass: 1,582,041 |
| Production allocated bytes ≤ 229,823,958 | Fail: 281,173,378 | Pass: 203,711,633 |
| Production peak live bytes ≤ 78,772,998 | Pass: 63,694,157 | Pass: 63,018,740 |

No S83-F0 or S83-A0 row passes every frozen numeric gate. The table is not a
ranking and does not select a candidate.

## S83 Consolidated Unranked Evidence

The derived-candidate resource raw
`target/hbk-zero-copy-experiment-8.3.27.1859/results/raw-S83-derived-resource-0219685.jsonl`
contains 180 successful records and has SHA-256
`fe9e800f32d129c3c82b7281a3f9be9bc5b607493ba692e53654a85e99d91351`.
For each of L1/I1/D1/P1/R1 it contains exactly nine warm runtime, nine
cold-best-effort runtime, nine production, three runtime-allocation, three
producer-allocation and three aggregate-four-reader records. Performance
processes were serialized; candidate order was rotated between runtime
samples. The complete machine-readable summary is
`summary-S83-all-candidates-0219685.json` and its rendered evidence table is
`summary-S83-all-candidates-0219685.md`. Both explicitly record
`ranked: false` and `selection: pending-user-decision`.

The five derived artifacts are deterministic, immutable files with adjacent
slot lock files:

| ID | Branch | Commit | Artifact bytes | SHA-256 | Mode |
| --- | --- | --- | ---: | --- | --- |
| `S83-L1` | `experiment/hbk-zero-copy-83-layout-l1` | `98f8b3bfadeeb40585fb4792aacfcc2f83b52bfc` | 11,304,567 | `cd0bfd19ae7592232f0eafb300a3f61c356ebdadaa600573245ff2144f14bc73` | `0444` |
| `S83-I1` | `experiment/hbk-zero-copy-83-index-i1` | `b7a674806a086ba40aaa617eca461238e23615dc` | 23,694,119 | `991b9e056c09defb8e12632cd83a709df5873b4383dbaea284c5f5dc64438c85` | `0444` |
| `S83-D1` | `experiment/hbk-zero-copy-83-dynamic-d1` | `a7ae5304b702759de92ed82847bf8be1f64eac22` | 11,304,567 | `20bc6ff8bf922b129233cafdcb4abbec51496697ee086788aacaf1eb00bd74b2` | `0444` |
| `S83-P1` | `experiment/hbk-zero-copy-83-produce-p1` | `b0d2523d06268f27eb36fcdfb601d0444bd372fc` | 11,304,567 | `20bc6ff8bf922b129233cafdcb4abbec51496697ee086788aacaf1eb00bd74b2` | `0444` |
| `S83-R1` | `experiment/hbk-zero-copy-83-record-r1` | `ffcb990cbd3c4e6c3e95e31ebb6b35cf716ad625` | 12,061,887 | `7bd06fd9bd0388b1d157c3fd38374c93654084cef7193b9f637abfb3cf8702d9` | `0444` |

D1 changes validation timing only and P1 changes formation only, so both
intentionally reproduce the exact F0 bytes. L1 changes only physical section
order. I1 adds one mapped-hash section. R1 changes record and nested-arena
representation. L1/I1/D1/P1/R1 are isolated-variable experiments against F0,
not standalone production preferences; their measurements establish the cost
and effect of the registered variable only.

### Consolidated Startup And Lookup

The following cells are `median ± MAD`; ready/workload use milliseconds and
first/anchor use microseconds. H0 is the SQL-to-owned baseline. C0 is the
current-cache-to-owned control. Registry row order is presentation order, not
a rank.

| ID | Warm ready, ms | Cold ready, ms | Warm first, µs | Cold first, µs | Warm anchor, µs | Cold anchor, µs | Warm workload, ms | Cold workload, ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `S83-H0` | 588.217 ± 4.898 | 1,631.261 ± 7.871 | 2.614 ± 0.187 | 2.994 ± 0.220 | 15.236 ± 0.597 | 15.341 ± 0.310 | 2,179.371 ± 6.546 | 2,173.222 ± 7.407 |
| `S83-C0` | 42.489 ± 0.388 | 73.776 ± 1.699 | 2.721 ± 0.307 | 3.461 ± 0.380 | 14.275 ± 0.160 | 15.108 ± 0.438 | 2,071.101 ± 9.079 | 2,091.371 ± 3.571 |
| `S83-F0` | 54.358 ± 0.618 | 67.899 ± 0.673 | 3.902 ± 0.074 | 4.149 ± 0.520 | 14.007 ± 0.438 | 14.750 ± 0.506 | 358.243 ± 2.417 | 356.781 ± 2.173 |
| `S83-A0` | 39.174 ± 0.445 | 50.703 ± 0.121 | 2.872 ± 0.165 | 2.823 ± 0.174 | 11.915 ± 0.381 | 11.965 ± 0.412 | 4,638.727 ± 14.339 | 4,644.303 ± 13.769 |
| `S83-L1` | 54.781 ± 1.201 | 65.887 ± 0.772 | 4.770 ± 0.459 | 4.024 ± 0.426 | 15.320 ± 0.269 | 13.918 ± 0.764 | 360.233 ± 1.534 | 359.028 ± 1.144 |
| `S83-I1` | 267.971 ± 6.043 | 287.740 ± 4.727 | 1.654 ± 0.040 | 1.802 ± 0.128 | 6.560 ± 0.241 | 6.855 ± 0.405 | 135.916 ± 1.907 | 134.760 ± 1.558 |
| `S83-D1` | 15.468 ± 0.930 | 28.296 ± 0.359 | 12,818.287 ± 246.196 | 11,800.780 ± 391.194 | 6,052.949 ± 85.231 | 6,143.532 ± 489.988 | 356.764 ± 0.896 | 357.536 ± 2.492 |
| `S83-P1` | 54.986 ± 1.266 | 64.242 ± 0.291 | 4.052 ± 0.222 | 4.249 ± 0.426 | 14.806 ± 0.940 | 15.878 ± 1.665 | 353.585 ± 1.452 | 356.805 ± 2.990 |
| `S83-R1` | 53.317 ± 1.049 | 63.633 ± 0.364 | 3.573 ± 0.065 | 3.497 ± 0.345 | 13.473 ± 0.648 | 13.454 ± 0.312 | 359.604 ± 1.508 | 361.455 ± 1.051 |

D1 makes ready time measure only eager validation and deliberately moves
typed section validation into first use; its first-lookup and anchor cells show
that shifted cost. I1 validates and maps 12,389,536 additional hash-index bytes
at open; its ready and memory cells include that cost.

All candidates preserve the exact 25 observed totals in both stances. Warm
per-operation medians in nanoseconds are:

| Operation | C0 | F0 | A0 | L1 | I1 | D1 | P1 | R1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `availability_by_fact` | 111 | 250 | 105 | 248 | 246 | 253 | 247 | 256 |
| `callable_by_owner_name` | 323 | 596 | 499 | 589 | 359 | 594 | 606 | 595 |
| `constructors_by_type` | 15 | 51 | 15 | 52 | 51 | 51 | 52 | 52 |
| `dictionary_by_id` | 0 | 23 | 3 | 24 | 23 | 24 | 23 | 23 |
| `dictionary_by_value` | 917 | 961 | 2,245 | 974 | 65 | 942 | 924 | 955 |
| `dictionary_by_value_miss` | 48,096 | 524 | 113,905 | 514 | 47 | 524 | 532 | 530 |
| `enum_by_name` | 208 | 504 | 404 | 527 | 202 | 508 | 507 | 504 |
| `enum_by_name_miss` | 345 | 652 | 602 | 666 | 349 | 635 | 639 | 637 |
| `exact_fact_id` | 99 | 1,294 | 394 | 1,288 | 108 | 1,274 | 1,231 | 1,255 |
| `exact_fact_id_miss` | 72 | 1,948 | 433 | 1,990 | 53 | 2,003 | 1,973 | 1,987 |
| `global_by_domain_name_kind` | 315 | 938 | 552 | 923 | 337 | 930 | 945 | 944 |
| `language_by_name` | 183 | 194 | 229 | 192 | 215 | 191 | 191 | 191 |
| `language_by_name_miss` | 349 | 363 | 393 | 360 | 414 | 359 | 361 | 361 |
| `member_by_owner_name_kind` | 232 | 597 | 358 | 613 | 241 | 599 | 592 | 763 |
| `members_by_owner` | 19 | 94 | 19 | 95 | 94 | 95 | 95 | 96 |
| `module_context_by_kind` | 577 | 1,774 | 476 | 1,796 | 424 | 1,747 | 1,750 | 1,760 |
| `query_field_by_table_name` | 222 | 444 | 348 | 449 | 256 | 442 | 449 | 438 |
| `query_param_by_table_name` | 335 | 500 | 432 | 503 | 407 | 509 | 504 | 497 |
| `query_table_by_identifier` | 329 | 782 | 489 | 842 | 394 | 803 | 788 | 780 |
| `query_table_by_name` | 523 | 1,081 | 730 | 1,085 | 616 | 1,099 | 1,048 | 1,054 |
| `query_table_by_syntax` | 682 | 1,279 | 919 | 1,282 | 783 | 1,281 | 1,285 | 1,297 |
| `relation_by_source_kind` | 48,965 | 858 | 106,148 | 847 | 290 | 857 | 841 | 853 |
| `type_by_name` | 246 | 975 | 494 | 963 | 248 | 965 | 970 | 1,000 |
| `type_by_name_miss` | 349 | 702 | 650 | 681 | 349 | 695 | 707 | 694 |
| `type_template_by_key` | 109 | 377 | 223 | 380 | 102 | 388 | 379 | 383 |

Cold-best-effort operation medians and every operation-specific
`median + max(25%, 3 × C0 MAD, 3 × candidate MAD)` ceiling are retained in
the generated JSON/Markdown summary. No operation is omitted from that proof.

### Consolidated Memory And Sharing

Medians are shown below; allocation MAD is zero for every listed candidate
sample.

| ID | Peak RSS warm/cold, KiB | PSS warm/cold, KiB | Private warm/cold, KiB | Minor faults warm/cold | Major faults warm/cold | Cold resident growth, B | Four-reader PSS, KiB | Runtime alloc calls / bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `S83-H0` | 74,948 / 74,664 | 67,992 / 67,989 | 67,972 / 67,968 | 18,452 / 18,452 | 0 / 0 | 116,178,944 | 263,954 | 1,278,346 / 156,058,238 |
| `S83-C0` | 34,816 / 34,816 | 22,141 / 22,102 | 22,120 / 22,080 | 7,620 / 7,620 | 0 / 0 | 11,259,904 | 81,142 | 136,036 / 28,942,929 |
| `S83-F0` | 13,184 / 13,184 | 11,770 / 11,770 | 11,756 / 11,756 | 182 / 182 | 0 / 1 | 11,304,960 | 12,342 | 66,266 / 4,633,385 |
| `S83-A0` | 15,744 / 15,616 | 14,358 / 14,354 | 14,344 / 14,340 | 216 / 216 | 0 / 1 | 13,938,688 | 15,180 | 151,855 / 6,791,316 |
| `S83-L1` | 13,312 / 13,312 | 11,894 / 11,894 | 11,880 / 11,880 | 181 / 182 | 0 / 1 | 11,304,960 | 12,729 | 66,266 / 4,633,364 |
| `S83-I1` | 25,472 / 25,344 | 24,098 / 24,098 | 24,084 / 24,084 | 372 / 372 | 0 / 1 | 23,695,360 | 24,915 | 66,266 / 4,633,374 |
| `S83-D1` | 13,184 / 13,184 | 11,798 / 11,766 | 11,784 / 11,752 | 177 / 177 | 0 / 1 | 11,304,960 | 12,347 | 10 / 4,552 |
| `S83-P1` | 13,184 / 13,184 | 11,770 / 11,802 | 11,756 / 11,788 | 182 / 181 | 0 / 1 | 11,304,960 | 12,346 | 66,266 / 4,633,365 |
| `S83-R1` | 13,952 / 13,952 | 12,506 / 12,514 | 12,492 / 12,500 | 188 / 187 | 0 / 1 | 12,062,720 | 12,993 | 10 / 4,676 |

The frozen major-fault gate remains zero. H0/C0 recorded zero in both stances;
every mapped candidate recorded a warm median of zero and cold-best-effort
median of one, so that gate fails for every candidate. The result is retained
rather than normalized away.

### Consolidated Production And Footprint

Cells are `median ± MAD`. Time is milliseconds; bytes and KiB retain their
native units. H0 has no separate cache-production row because SQL
materialization is its declared runtime baseline.

| ID | Total, ms | Materialize, ms | Serialize/formation, ms | Validate, ms | Write/publish, ms | Artifact bytes | Peak RSS, KiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `S83-C0` | 642.839 ± 19.106 | 582.161 ± 18.629 | n/a | n/a | 56.912 ± 5.536 | 11,186,057 ± 0 | 80,780 ± 0 |
| `S83-F0` | 3,272.623 ± 36.147 | 584.768 ± 10.213 | n/a | 96.983 ± 0.296 | 244.055 ± 32.800 | 11,304,567 ± 0 | 100,900 ± 52 |
| `S83-A0` | 1,994.852 ± 23.196 | 586.432 ± 5.495 | 17.462 ± 0.078 | 18.026 ± 0.269 | 258.073 ± 30.467 | 13,936,492 ± 0 | 100,996 ± 4 |
| `S83-L1` | 3,251.808 ± 13.793 | 597.935 ± 6.658 | n/a | 48.461 ± 0.431 | 256.748 ± 21.375 | 11,304,567 ± 0 | 100,908 ± 44 |
| `S83-I1` | 4,573.844 ± 49.268 | 604.127 ± 12.648 | n/a | 506.733 ± 4.500 | 683.438 ± 28.552 | 23,694,119 ± 0 | 137,332 ± 4 |
| `S83-D1` | 3,296.207 ± 38.794 | 589.986 ± 15.162 | n/a | 97.229 ± 1.123 | 260.224 ± 22.228 | 11,304,567 ± 0 | 100,912 ± 104 |
| `S83-P1` | 3,101.216 ± 20.611 | 584.968 ± 4.049 | 70.593 ± 1.966 | 79.676 ± 2.628 | 210.733 ± 8.070 | 11,304,567 ± 0 | 89,588 ± 112 |
| `S83-R1` | 3,315.699 ± 15.749 | 599.949 ± 15.847 | n/a | 95.561 ± 0.267 | 283.741 ± 19.060 | 12,061,887 ± 0 | 102,456 ± 0 |

| ID | Producer alloc calls | Producer allocated bytes | Producer peak-live bytes | Section bytes | Dictionary bytes | Index bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `S83-C0` | 1,278,357 | 183,859,167 | 63,018,399 | n/a | n/a | n/a |
| `S83-F0` | 2,291,021 | 281,173,378 | 63,694,157 | 11,303,343 | 4,913,114 | 3,558,899 |
| `S83-A0` | 1,582,041 | 203,711,633 | 63,018,740 | n/a | 4,039,645 text | 2,238,408 estimated fixed |
| `S83-L1` | 2,224,762 | 276,546,222 | 63,695,496 | 11,303,343 | 4,913,114 | 3,558,899 |
| `S83-I1` | 2,291,073 | 367,802,030 | 107,009,264 | 23,692,879 | 4,913,114 | 15,948,435 |
| `S83-D1` | 2,291,021 | 281,173,406 | 63,694,164 | 11,303,343 | 4,913,114 | 3,558,899 |
| `S83-P1` | 2,025,983 | 218,911,208 | 63,018,773 | 11,303,343 | 4,913,114 | 3,558,899 |
| `S83-R1` | 2,146,274 | 275,969,710 | 70,567,980 | 12,060,397 | 4,913,164 | 3,924,922 |

I1's index total includes 12,389,536 mapped-hash bytes:
12,388,896 bucket bytes, 26 tables, 226,354 groups, 774,306 buckets and
maximum observed probe 23. R1 adds 1,078,791 record-head bytes and 2,143,520
nested-arena bytes. P1 retains no monolithic artifact buffer, retains at most
one completed section buffer, writes 11,304,567 logical bytes with measured
userspace write-amplification ratio 1.0, and reports 4,633,993 peak section
buffer bytes / 9,763,449 peak tracked working-buffer bytes.

### Consolidated Behavioral And Gate State

Every F0/A0/L1/I1/D1/P1/R1 row passes all of the following independent
behavioral proofs:

- storage content: 176,793 canonical records / 57,486,556 bytes /
  SHA-256 `5f66d20509877ac29a83ede2d5178368ed3fd78d7dab0ffbc12df506acc3b1fd`;
- storage lookup behavior: 276,415 canonical records / 88,520,585 bytes /
  SHA-256 `9b17c7100cd368fe0880e679d66ab8eb7d8505ee617d9fc80b1a9a9d8aa5c5c8`;
- catalog/resolver semantics: 742,872 records / 769,824,709 bytes /
  SHA-256 `1fe7f166caad8e8573b809a97f7104caf85301370f1d34017376bc82ee893a29`;
- one sequential plus four concurrent byte-identical storage replays and the
  corresponding five semantic transcripts;
- sources hidden before candidate process start and throughout replay;
- exact platform `8.3.27.1859`, HBK/provider identity, provider schema `16`,
  extraction schema `11`, registered artifact layout and immutable mode.

The F0 and A0 storage proofs were rerun at the exact measured semantic/resource
commits in `raw-S83-F0-a9a98a1-parity-exact-0219685.jsonl` and
`raw-S83-A0-36a41aa-parity-exact-0219685.jsonl`. The final summary rejects an
ancestor-only storage proof.

Thus full equivalence for T183 means identical logical content, ordering,
lookup statuses/results and public catalog/resolver behavior, not stable
numeric IDs or equal physical bytes. Numeric string/fact IDs remain valid only
inside the current mapped generation.

No row passes every frozen numeric gate, so
`eligibility_state = no-candidate-passes-all-frozen-gates`. No waiver is
recorded. A noisy gate is `inconclusive-noisy` rather than pass/fail unless it
uses the predeclared first-lookup absolute-budget or per-operation MAD-envelope
exception.

| ID | Failed frozen gates | Inconclusive noisy gates |
| --- | --- | --- |
| `S83-F0` | warm/cold ready; cold major fault; total rebuild; producer allocation calls/bytes; warm/cold forward dictionary; warm/cold reverse hit; warm/cold per-operation ceiling | none |
| `S83-A0` | warm ready; runtime allocation calls; warm/cold workload; cold major fault; total rebuild; production peak RSS; warm/cold reverse hit/miss; warm/cold per-operation ceiling | none |
| `S83-L1` | warm/cold ready; cold major fault; total rebuild; producer allocation calls/bytes; warm/cold forward dictionary; warm/cold reverse hit; warm/cold per-operation ceiling | cold anchor |
| `S83-I1` | warm/cold ready; warm/cold PSS/private; cold major fault; cold resident growth; artifact size; total rebuild; production peak RSS; producer allocation calls/bytes/peak; warm/cold forward dictionary; warm/cold per-operation ceiling | cold anchor |
| `S83-D1` | warm/cold first lookup; warm anchor; cold major fault; total rebuild; producer allocation calls/bytes; warm/cold forward dictionary; warm/cold reverse hit; warm/cold per-operation ceiling | warm ready; cold anchor |
| `S83-P1` | warm/cold ready; cold major fault; total rebuild; producer allocation calls; warm/cold forward dictionary; warm/cold reverse hit; warm/cold per-operation ceiling | warm/cold anchor |
| `S83-R1` | warm/cold ready; cold major fault; total rebuild; production peak RSS; producer allocation calls/bytes; warm/cold forward dictionary; warm/cold reverse hit; warm/cold per-operation ceiling | none |

This is a threshold matrix, not a score. It neither orders the rows nor
chooses an acceptable trade-off. Under the frozen contract, the user can ask
for a new experiment/rerun, explicitly waive named gates with a durable
owner/date/rationale, or reject/stop production adoption; the evidence alone
does not authorize selecting an ineligible row.

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
contract. H3 proves the complete normalized files only through a reconstructed
owned adapter, not through the mapped read surface, and does not yet validate
sorted order for every lookup array used by binary search. Those earlier S85
rows therefore remain behaviorally incomplete.

The independent S83 F0/A0/L1/I1/D1/P1/R1 rows all pass the complete storage
and catalog/resolver equivalence protocol above. This establishes behavioral
equivalence for the current snapshot/catalog scope; it does not establish that
every row satisfies the independent performance/resource gates, and it does
not extend the scope to full HTML, long descriptions or search/export
payloads.

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

No candidate satisfies every frozen numeric gate, and no waiver exists;
therefore no S83 row is currently eligible under the frozen contract. All
seven S83 rows pass the mandatory behavioral-equivalence gates, so parity is
not the blocker. No candidate branch is merged, deleted, promoted,
recommended or marked first. The current owned snapshot/cache path remains
canonical because the user has not selected an outcome and no durable
production decision has accepted a replacement.
