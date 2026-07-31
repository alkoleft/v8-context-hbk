# T183 HBK Zero-Copy Snapshot Experiment

Status: active bounded experiment. This document authorizes comparison
prototypes only. It does not select a production format, accept a dependency,
change the canonical runtime owner or authorize merging a candidate into
`master`.

## Question And Decision Boundary

The experiment asks whether a validated file-backed snapshot is materially
better than the current runtime paths while preserving all observable snapshot,
catalog and adapter behavior.

`H0` SQLite-to-owned is the baseline. `C0` current-binary-cache-to-owned is a
control. Candidate measurements are evidence rows, not a ranking. Passing the
predeclared gates means only that a candidate remains eligible for a later
decision. The user must choose whether any candidate is preferred; no branch is
promoted or merged into `master` merely because it has the best measured value.

HBK remains the authoritative documentation input. A zero-copy artifact may
become the canonical runtime context only after a separate accepted
specification or ADR decision.

## Comparison Set S85

The mandatory real-corpus row is:

- platform HBK:
  `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`;
- platform version: `8.5.1.1150`;
- locale: `ru`;
- HBK size: `41,361,963` bytes;
- HBK SHA-256:
  `b8bc0d3a1ee8d00e2f113a800339731304428cc35ae395e5094a8b022773f8ed`;
- provider SQLite:
  `target/snapshot-materialization/shcntx_ru.schema16.release.sqlite`;
- SQLite size: `206,753,792` bytes;
- SQLite SHA-256:
  `cc9b2b8aaf31f64c880b92cc3a02fd3166541f10f8d209faf8c7a7c22cac0d55`;
- provider schema: `16`;
- extraction schema: `11`.

The SQLite file is a prebuilt, reused, read-only provider input. SQL benchmark
processes do not rebuild it and do not share a `rusqlite::Connection`. Each
process starts with a fresh SQLite connection and the repository's existing
read settings.

This corpus currently contains no language facts. Full semantic parity
therefore also uses deterministic real-derived fixtures that contain language
facts, ambiguity and unsupported outcomes. Those fixtures are parity evidence,
not substitutes for real-corpus performance measurements. Any additional real
corpus must be recorded with the same identity fields and reported as a
separate dataset rather than pooled silently with this row.

## Comparison Set S83

The user requested an additional, independent comparison set over:

- dataset ID: `shcntx_ru-8.3.27.1859-schema16-extraction11`;
- platform HBK:
  `/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk`;
- platform version: `8.3.27.1859`;
- locale/source locale: `ru`;
- HBK size: `40,744,845` bytes;
- HBK SHA-256:
  `5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48`;
- provider SQLite:
  `target/snapshot-materialization/shcntx_ru.8.3.27.1859.schema16.release.sqlite`;
- SQLite size: `204,288,000` bytes;
- SQLite SHA-256:
  `55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab`;
- provider schema: `16`;
- extraction schema: `11`;
- service-data root:
  `target/hbk-zero-copy-experiment-8.3.27.1859/`.

S83 does not change the first supported production platform baseline. It tests
whether the hypotheses survive an older, smaller real corpus and requires the
artifact to reject every platform version other than `8.3.27.1859` for this
set. S83 raw/parity/prepared files and numerical gates are independent from
S85. No value or threshold is pooled or copied between the sets.

The release provider-index build completed in `18.19 s` at `286,912 KiB` peak
RSS and produced `25,052` documents. The initial owned snapshot contains
`69,695` strings, `1,749` platform types, `18,004` type members, `8,299`
callables, `601` globals, `53` query tables, `498` query fields, `56` query
parameters, zero language facts, `670` enums and `2,934` enum values. These
counts establish corpus identity; candidate parity still uses the full
canonical content and lookup files plus deterministic language/ambiguity/
unsupported fixtures.

## Hypothesis Registry And Branch Ancestry

| ID | Path | Falsifiable claim | Branch relationship |
| --- | --- | --- | --- |
| `H0` | SQLite to current owned snapshot | Accepted semantic and performance baseline | common benchmark base |
| `C0` | Current binary cache to owned snapshot | Controls for avoiding SQLite while retaining full deserialization | common benchmark base |
| `H1` | Custom flat mapped sections with checked offsets and lazy `from_le_bytes` decoding | Avoids graph deserialization and private heap without unacceptable lookup cost | branch from frozen benchmark-base commit |
| `H2` | H1 artifact/layout with typed fixed-section views | Isolates typed-access cost from H1 layout/writer cost | branch from the exact measured H1 commit; reported as “H1 layout + typed reader”, not an independent format |
| `H3` | `rkyv` archive candidate | A validated archive may provide simpler borrowed traversal with competitive validation/open and lookup cost | branch from frozen benchmark-base commit |

`H3` is called an archive candidate until it proves its safe-access boundary,
format/schema/endianness compatibility, dependency acceptability and parity.
No unchecked archived access is allowed in domain or catalog code.

Reverse string lookup is a measured sub-hypothesis. Start with sorted
dictionary/remapped dense IDs and binary search; compare a mapped
open-addressed index only if exact/canonical lookup remains limiting. Do not
add FST unless both simpler variants leave measured lookup or size evidence.

The common harness and protocol are committed before candidate branches. Every
result records the harness commit and candidate commit. Candidate branches
must use the exact frozen harness; a harness change requires an explicit new
benchmark-base commit and rerunning all affected baseline and candidate rows.

The expected isolation is:

```text
experiment/hbk-zero-copy-base
├── experiment/hbk-zero-copy-flat-h1
├── experiment/hbk-zero-copy-rkyv-h3
└── experiment/hbk-zero-copy-flat-typed-h2  (from measured H1 commit)
```

H1 and H3 may be implemented in parallel worktrees. H2 starts only after H1
has a committed artifact/layout and measures the subtraction from H1.

### S83 organization registry

S83 keeps format references separate from the organization hypotheses:

| ID | Role / isolated variable | Falsifiable claim |
| --- | --- | --- |
| `S83-H0` | SQLite-to-owned baseline | Establishes S83 semantic, startup, lookup and resource denominators. |
| `S83-C0` | Current-cache-to-owned control | Establishes the cost left after avoiding SQLite while retaining full deserialization. |
| `S83-F0` | Corrected typed-flat format/lifecycle reference | A fully validated typed flat mapping can supply the common reference layout and complete mapped oracle without an owned runtime mirror. |
| `S83-A0` | Checked archive format/lifecycle reference | A checked archive provides a second format/safety reference; it is not a substitute for the organization hypotheses below. |
| `S83-L1` | F0 with only hot/cold page-clustered section order changed | Co-locating the sections touched together by the frozen workload reduces first-touch faults/latency or resident pages without changing lookup algorithms. |
| `S83-I1` | F0 with only mapped open-address reverse/name indexes changed | A stable checked hash/probe layout reduces limiting exact/reverse lookups enough to justify its artifact and validation overhead. |
| `S83-D1` | F0 with only lazy safe per-section validation/access changed | Header/directory validation plus checked first-use section validation reduces ready time/page touches without returning corrupt data or shifting unacceptable cost into first lookup. |
| `S83-P1` | F0 with only two-pass/direct formation changed | Writing the same runtime semantics without a monolithic output buffer reduces formation allocations/peak RSS or write amplification. |

Every candidate has its own branch and worktree. `S83-L1`, `S83-I1`,
`S83-D1` and `S83-P1` branch from the exact measured `S83-F0` commit.
Implementation work may run in parallel. Performance commands are serialized
by the coordinator; agents must not benchmark concurrently on the shared host.

S83 candidate timing is inadmissible until the candidate records the exact S83
HBK/SQLite/harness/query-manifest/oracle identities and its complete mapped
content and lookup files compare byte-for-byte with `S83-H0`/`S83-C0`.

## Frozen Measurement Boundary

All timed paths run as separate release-profile processes. A parent timestamp
is captured immediately before `/usr/bin/time` executes the child and the
child reports parent-launch-to-ready plus earliest-`main`-entry-to-ready. The
first lookup includes read-handle creation. Workload anchor resolution is a
separate timed/fault/allocation phase before the reported warm-up and batched
workload. The harness exposes
separate commands for:

- `prepare-cache`: produce a C0 artifact outside any measured open process;
- `sql-owned`: process start to a ready current owned snapshot from the reused
  read-only SQLite artifact;
- `cache-owned`: process start to a ready owned snapshot from an already
  prepared compatible cache and fail the sample unless the loader reports
  `Loaded`;
- candidate `produce`: current owned snapshot input to candidate
  encode/validate/write, reported separately from SQL materialization and also
  as total local rebuild;
- candidate `open`: process start through compatibility validation and
  read-only mapping until ready for a query;
- `first-lookup`: the first representative lookup after open;
- `warm-lookup`: the versioned batched query manifest;
- `hold`: post-open and post-workload steady-state memory observation;
- `parity`: canonical logical oracle generation in a separate process, never
  mixed into startup or memory timing.

`prepare-cache` creates the current cache once from the exact SQLite input.
Measured C0 runs use a per-run artifact copy inside the experiment output
root, verify its checksum before the run and reject any sample whose status is
not `Loaded`. Candidate produce runs also use per-run outputs. Cleanup is
limited to paths created below `target/hbk-zero-copy-experiment/`; source HBK,
provider SQLite, prepared evidence and unrelated `target/` data are never
deleted.

Warm and cold-best-effort results are separate:

- warm runs use two unreported warm-ups followed by nine recorded samples;
- cold-best-effort runs use nine recorded fresh processes and a documented
  per-file Linux page-cache eviction request for every input artifact before
  each run;
- global `/proc/sys/vm/drop_caches` is not used;
- an advisory eviction that cannot be verified is labelled
  `cold-best-effort`, never “true cold”;
- all implementations use the same cache stance and interleaved run order.

Timing samples compile the experiment allocator as a direct `System`
delegation with counters removed. Allocation profiling is a separate release
binary with compile-time counting enabled; it reports calls, cumulative
allocated/deallocated bytes, live bytes and peak live bytes by phase.
`heaptrack` is an external cross-check rather than the source of timing
evidence. `/usr/bin/time` records process elapsed, maximum RSS and minor/major
faults. The held process exposes
`/proc/<pid>/smaps_rollup` evidence for RSS, PSS, private/shared and anonymous
memory. The four-reader scenario reports aggregate PSS. If a tool is
unavailable or denied, the field is `unavailable` with the exact reason; it is
not replaced by an incomparable estimate.

Raw results use versioned JSONL below
`target/hbk-zero-copy-experiment/results/`. Each record includes corpus,
backend/hypothesis, lifecycle scenario, cache stance, harness commit,
candidate commit, build profile, host/kernel, Rust/Cargo versions, command,
sample index, status and measured fields. Summary values are median and MAD.
Samples are never silently trimmed. A process/tool failure or invalid backend
status is recorded and invalidates the comparison group, which is rerun in
full. `MAD / median > 5%` marks a metric noisy and prevents a gate conclusion
until the noise is explained or the group is rerun.

Task-local numeric material-benefit and non-regression gates are derived from
the H0/C0 noise runs and written into this document before any candidate code
is implemented. Candidate results may not be inspected to choose or adjust
those thresholds.

## Frozen H0/C0 Evidence

The final common benchmark base is commit
`051df7979e3cf5f6431b4d13829f436c98c47054`. Its workload is
`hbk-snapshot-warm-lookups/v2`, including forward string resolution, reverse
string hit and reverse string miss. The raw service evidence contains 61
records for this harness commit: 60 successful measurements and one successful
parity record, with no failed record.

The H0/C0 timing and memory medians are:

| Backend/scenario | N | Ready median ± MAD | Workload median ± MAD | Peak RSS | Post-workload PSS/private |
| --- | ---: | ---: | ---: | ---: | ---: |
| H0 SQL owned, warm | 9 | 599.568 ± 7.419 ms | 2,226.218 ± 8.489 ms | 75,500 KiB | 68,428 / 68,408 KiB |
| H0 SQL owned, cold-best-effort | 9 | 1,707.429 ± 10.461 ms | 2,222.610 ± 6.898 ms | 75,156 KiB | 68,422 / 68,400 KiB |
| C0 cache owned, warm | 9 | 41.764 ± 0.839 ms | 2,125.346 ± 11.649 ms | 35,200 KiB | 22,312 / 22,292 KiB |
| C0 cache owned, cold-best-effort | 9 | 73.475 ± 2.774 ms | 2,136.306 ± 12.465 ms | 35,200 KiB | 22,312 / 22,292 KiB |

C0 first lookup is 2.902 ± 0.113 microseconds warm and
2.997 ± 0.125 microseconds cold-best-effort. Its warm anchor resolution is
15.066 ± 0.325 microseconds. The warm dictionary medians are 0 ns for
`dictionary_by_id` after integer averaging, 946 ± 23 ns for reverse hit and
49,871 ± 503 ns for reverse miss. The zero average is reported as timer
resolution evidence, not as a literal claim that forward lookup has no cost.

C0 runtime open performs 137,633 allocation calls and allocates 29,278,447
bytes before ready. Its final and peak live allocation-accounted values are
17,942,214 and 29,274,971 bytes. The four-reader C0 median is 82,021 KiB PSS
and 79,396 KiB private. H0 is 265,794 KiB PSS and 262,988 KiB private.

C0 artifact production, including SQL materialization, has a total local
rebuild median of 675.933 ± 10.180 ms. SQL materialization is
594.571 ± 2.660 ms, artifact write is 73.492 ± 20.710 ms, peak RSS is
81,356 KiB and artifact size is 11,325,758 bytes. The write phase is marked
noisy and remains descriptive; the non-noisy combined local-rebuild value is
the lifecycle gate. The production allocation profile is 1,291,568 calls,
182,653,725 allocated bytes and 63,498,929 peak live bytes.

Cold-best-effort C0 brings 11,403,264 file bytes resident according to
`fincore`; H0 brings 117,473,280 SQLite bytes resident. These are page-cache
residency deltas, not exact CPU byte-read counters.

The H0/C0 canonical content and lookup transcript digests are respectively
`000c78a733b286b1bf926ba5dec6e2168593ed14028ca4df02179fc8eedc6ba6`
and
`76b7ae21c8a70c10ca5d623de9d64309036f3219c7728a861217751b90874219`.
The full files are byte-identical, including four concurrent-reader
transcripts.

## Candidate Evidence Collected

The committed H1, H2 and H3 branches have been measured, but no candidate
completes the mandatory mapped parity and integrated rebuild-before-map
lifecycle gates. The full unranked runtime, startup-plus-first-lookup,
operation, allocation, memory, production, relative-value and gate tables are
in
[T183 HBK Zero-Copy Snapshot Evidence](../acceptance/hbk-zero-copy-snapshot-evidence.md).

That evidence also freezes the provisional downstream handoff: immutable
provider-owned HBK strings may be exposed through generation-scoped,
session-local IDs, while the project/request overlay remains outside HBK.
The physical snapshot representation remains unselected. The downstream
unified semantic entity change must not treat a memory-mapped base dictionary
or compact materialization as accepted production architecture before the
user's decision.

## Current Owned-Cache Inventory

The current writer first materializes the complete owned snapshot, serializes
it into a second `Vec<u8>` payload and writes that payload. The current reader
reads the complete payload into a `Vec<u8>`, validates its checksum and then
allocates a second complete owned graph. The payload buffer and partially
decoded graph overlap during load.

The real corpus owned graph contains a `Vec<String>` with 70,860 individually
allocated strings and eleven top-level fact arenas: 1,754 platform types,
18,167 members, 8,337 callables, 601 globals, 53 query tables, 498 fields,
56 parameters, zero language facts in this corpus, 711 enums and 3,087 enum
values. Records additionally own nested vectors for metadata-template
parameters, type references, signatures, signature parameters and returns,
query owner paths/template parameters and related ordered values.

The snapshot also materializes 34 lookup/index representations:

- generic fact IDs; type IDs, names and templates;
- member IDs, owner CSR, owner/name and owner/name/kind;
- callable IDs, owner CSR, owner/name and constructors;
- global names and domain/name/kind;
- module event names and domain/language/module-kind contexts;
- query table IDs, names, syntax names and identifiers;
- query field owner CSR and owner/name;
- query parameter owner CSR and owner/name;
- language IDs and names;
- enum IDs and names; enum-value IDs, owner CSR and owner/name;
- availability owner CSR and available-since;
- relation source/kind CSR.

For the selected corpus the serialized C0 artifact is 11,325,758 bytes while
the ready owned graph accounts for 17,908,362 logical/heap bytes and
22,288 KiB process-private memory after the workload. A candidate must avoid
reconstructing this graph; retaining both a complete mapped representation and
a complete owned mirror is a structural failure regardless of timings.

## Prototype Production Lifecycle

T183 uses a release/installation artifact with a first-use rebuild fallback:

1. a supplied compatible immutable generation is the preferred runtime input;
2. a missing, corrupt, wrong-layout, wrong-extraction-schema, wrong-source,
   wrong-locale or wrong-platform-version artifact is rebuilt before any
   mapping is exposed;
3. rebuild produces a new temporary immutable generation and publishes it
   atomically; it never overwrites or truncates the mapped file;
4. discovery/publication requires the stable logical slot's exclusive lock;
   opening and the complete mapping lifetime hold its shared lock;
5. an update attempt while a reader holds the shared lock fails immediately
   with a typed snapshot-in-use error;
6. after publication a new session opens the new generation and receives a new
   session-local numeric ID space.

Measurements keep supplied-artifact open and local rebuild as separate rows.
This lifecycle contract does not decide that any candidate is canonical; that
decision still requires the user's explicit selection.

## Predeclared Candidate Gates

These gates are frozen before H1/H3 code. They compare runtime candidates to
C0, because C0 is the current no-SQL runtime control, while every table also
shows H0-relative values. A gate marked noisy (`MAD / median > 5%`) is rerun or
explained before a conclusion. Thresholds are not relaxed after inspecting a
candidate.

Mandatory correctness and safety:

- canonical content and lookup files are byte-identical to H0/C0 and both
  digests match the frozen values above;
- sequential and four-reader transcripts match;
- no SQLite/HBK fallback occurs after candidate open;
- header compatibility, structural validation, platform-version rejection,
  immutable-generation publication, mapping lifetime and fail-fast lock tests
  pass;
- no complete owned snapshot mirror exists beside the mapped artifact.

Mandatory material benefit against C0:

Fractional ceilings are rounded down to the nearest whole measured unit so
that rounding cannot weaken the stated minimum reduction.

| Metric | Required candidate median |
| --- | ---: |
| warm process-start-to-ready | at most 33,410,942 ns (20% reduction) |
| cold-best-effort process-start-to-ready | at most 58,780,152 ns (20% reduction) |
| runtime allocation calls to ready | at most 68,816 (50% reduction) |
| runtime allocated bytes to ready | at most 14,639,223 (50% reduction) |
| peak runtime RSS | at most 29,920 KiB (15% reduction) |
| warm post-workload PSS | at most 17,849 KiB (20% reduction) |
| warm post-workload private | at most 17,833 KiB (20% reduction) |
| cold post-workload PSS | at most 17,849 KiB (20% reduction) |
| cold post-workload private | at most 17,833 KiB (20% reduction) |
| aggregate four-reader PSS | at most 65,616 KiB (20% reduction) |
| reverse dictionary hit | at most 473 ns (50% reduction) |
| reverse dictionary miss | at most 24,935 ns (50% reduction) |

Mandatory non-regression and resource ceilings:

- first lookup median is at most 25,000 ns in each cache stance; this absolute
  budget is used because a few-microsecond C0 single-shot baseline is
  timer/scheduler noisy;
- anchor resolution median is at most 25,000 ns in each cache stance;
- total warm workload is at most 2,444,147,515 ns and cold-best-effort workload
  at most 2,456,751,978 ns (15% regression ceiling);
- every individual batched operation must preserve observed totals and its
  median must be no greater than `C0 median + max(25% of C0 median,
  3 × C0 MAD, 3 × candidate MAD)`; forward dictionary lookup additionally has
  an absolute 10 ns average ceiling;
- open major faults remain zero and open minor faults are at most 9,635;
- cold-best-effort file-resident growth is at most 14,254,080 bytes;
- artifact size is at most 14,157,197 bytes;
- total local rebuild is at most 844,916,105 ns, production peak RSS at most
  101,695 KiB, production allocation calls at most 1,614,460, production
  allocated bytes at most 228,317,156 and production peak live bytes at most
  79,373,661;
- write time, every section size, private/shared/anonymous memory and
  H0-relative results are still reported even where the gate applies only to
  the combined lifecycle or C0-relative value.

Passing all gates means “eligible for user consideration”, not first place.
Failing a gate remains an evidence row and does not authorize deleting its
branch or selecting another candidate without the user's decision.

## Frozen S83 H0/C0 Evidence And Candidate Gates

S83 uses harness commit
`28f29b5a262db362b6b58c8109e6df6c2afbbc44` and workload
`hbk-snapshot-warm-lookups/v2`. Its service evidence is
`target/hbk-zero-copy-experiment-8.3.27.1859/results/raw-28f29b5.jsonl`.
The file contains 61 records: 45 runtime/formation timing samples, nine
allocation profiles, six aggregate four-reader samples and one parity record.
Every record is successful and carries the same harness, corpus and provider
identity.

The complete H0/C0 canonical files compare byte-for-byte. The content oracle
contains 176,793 records / 57,486,556 bytes with SHA-256
`5f66d20509877ac29a83ede2d5178368ed3fd78d7dab0ffbc12df506acc3b1fd`.
The lookup transcript contains 276,415 records / 88,520,585 bytes with SHA-256
`9b17c7100cd368fe0880e679d66ab8eb7d8505ee617d9fc80b1a9a9d8aa5c5c8`.
Four concurrent C0 oracle processes reproduce both files exactly.

The timing and memory medians are:

| Backend/scenario | N | Ready median ± MAD | Workload median ± MAD | Peak RSS | Post-workload PSS/private |
| --- | ---: | ---: | ---: | ---: | ---: |
| S83-H0 SQL-owned warm | 9 | 588.217 ± 4.898 ms | 2,179.371 ± 6.546 ms | 74,948 KiB | 67,992 / 67,972 KiB |
| S83-H0 SQL-owned cold-best-effort | 9 | 1,631.261 ± 7.871 ms | 2,173.222 ± 7.407 ms | 74,664 KiB | 67,989 / 67,968 KiB |
| S83-C0 cache-owned warm | 9 | 42.489 ± 0.388 ms | 2,071.101 ± 9.079 ms | 34,816 KiB | 22,141 / 22,120 KiB |
| S83-C0 cache-owned cold-best-effort | 9 | 73.776 ± 1.699 ms | 2,091.371 ± 3.571 ms | 34,816 KiB | 22,102 / 22,080 KiB |

S83-C0 local production takes 642.839 ± 19.106 ms total, including a
582.161 ± 1.910 ms owned-snapshot open/materialize phase and
56.912 ± 11.701 ms artifact writing.
Its peak RSS is 80,780 KiB and its artifact is 11,186,057 bytes. The
write-only phase is noisy and remains reported evidence; the non-regression
gate applies to the combined local-production time.

The allocation and concurrent-reader baselines are:

| Backend/scenario | N | Allocations to ready | Allocated bytes to ready | Final / peak live bytes |
| --- | ---: | ---: | ---: | ---: |
| S83-H0 SQL-owned allocation profile | 3 | 1,278,346 | 156,058,238 | 22,262,899 / 63,018,253 |
| S83-C0 cache-owned allocation profile | 3 | 136,036 | 28,942,929 | 17,746,497 / 28,939,398 |
| S83-C0 local-production allocation profile | 3 | 1,278,357 | 183,859,167 | 22,261,991 / 63,018,399 |
| S83-H0 aggregate four-reader | 3 | — | — | 263,954 KiB PSS / 261,196 KiB private |
| S83-C0 aggregate four-reader | 3 | — | — | 81,142 KiB PSS / 78,548 KiB private |

Across the 126 captured before/hold/after machine-state snapshots, maximum
one-minute load was 0.205 per logical CPU, minimum available memory was
11,924,664 KiB and the maximum instantaneous runnable-task count was 3
during the explicit concurrent-reader checks. Candidate timing remains
serialized and must record the same fields.

S83 first-lookup medians are single-shot 2.614–3.461 microsecond observations
and exceed the five-percent MAD ratio in every runtime group. The same
relative-noise effect applies to a few 100–250 ns batched operations, while
their absolute MAD remains 10–16 ns. These cases use the already declared
absolute first-lookup budget and the per-operation noise envelope below rather
than a fractional speed comparison. No candidate threshold is derived from a
candidate result.

The following S83 gates are frozen before F0/A0/L1/I1/D1/P1 code. They use
S83-C0 only and are not copied from S85. Fractional ceilings are rounded down.

Mandatory correctness and safety:

- content and lookup files are byte-identical to the S83 digests and full files
  above, including sequential and four-reader transcripts;
- exact HBK/provider identity, platform `8.3.27.1859`, locale, provider schema,
  extraction schema, layout and section structure are validated before access;
- supplied-artifact open performs no SQLite/HBK fallback and keeps no complete
  owned snapshot mirror;
- immutable-generation publication, rebuild-before-map, session-long shared
  lock, fail-fast writer lock and mapping lifetime tests pass;
- every candidate records section/dictionary/index sizes and producer
  allocation evidence.

Mandatory material benefit against S83-C0:

| Metric | Required candidate median |
| --- | ---: |
| warm process-start-to-ready | at most 33,991,352 ns (20% reduction) |
| cold-best-effort process-start-to-ready | at most 59,020,968 ns (20% reduction) |
| runtime allocation calls to ready | at most 68,018 (50% reduction) |
| runtime allocated bytes to ready | at most 14,471,464 (50% reduction) |
| peak runtime RSS in either stance | at most 29,593 KiB (at least 15% reduction in both stances) |
| warm post-workload PSS | at most 17,712 KiB (20% reduction) |
| warm post-workload private | at most 17,696 KiB (20% reduction) |
| cold post-workload PSS | at most 17,681 KiB (20% reduction) |
| cold post-workload private | at most 17,664 KiB (20% reduction) |
| aggregate four-reader PSS | at most 64,913 KiB (20% reduction) |
| reverse dictionary hit | at most 458 ns (50% reduction) |
| reverse dictionary miss | at most 24,048 ns (50% reduction) |

Mandatory non-regression and resource ceilings:

- first lookup and anchor resolution medians are each at most 25,000 ns in
  each cache stance;
- total warm workload is at most 2,381,766,522 ns and cold-best-effort
  workload at most 2,405,076,745 ns;
- every individual batched operation preserves observed totals and its median
  is no greater than `S83-C0 median + max(25% of S83-C0 median,
  3 × S83-C0 MAD, 3 × candidate MAD)`; forward dictionary lookup additionally
  has an absolute 10 ns average ceiling;
- open major faults remain zero and open minor faults are at most 9,525;
- cold-best-effort file-resident growth is at most 14,074,880 bytes;
- artifact size is at most 13,982,571 bytes;
- total local production is at most 803,548,621 ns, production peak RSS at
  most 100,975 KiB, production allocation calls at most 1,597,946,
  production allocated bytes at most 229,823,958 and production peak live
  bytes at most 78,772,998;
- write time, every section/dictionary/index size, private/shared/anonymous
  memory, machine pressure and H0-relative results remain reported even where
  the gate applies only to a combined or C0-relative value.

These gates determine only whether an S83 row is eligible for the user's
consideration. They do not rank candidates, select first place, authorize a
merge or make a snapshot canonical.

## Mandatory Behavioral Oracle

Parity is an independent mandatory gate. Performance values remain recorded
evidence, but they cannot support accepting a candidate until its parity status
is `pass`.

The versioned canonical JSONL oracle compares logical content, not layout or
session-local numeric IDs. Every string ID is resolved to text and every typed
record link is normalized to `(fact-family, logical-id-text)`. Nested
signatures, parameters and type references retain source order.

The content oracle covers all observable fields, counts, ordered memberships,
availability and relations for platform types, members, callables,
constructors, signatures, parameters, globals, module contexts/events,
language facts, enums/values and query tables/fields/parameters. The lookup
transcript covers exhaustive corpus hits plus fixed misses, wrong
owner/domain/kind, ambiguity and unsupported outcomes through
`HbkFactReadHandle`, borrowed BSL/SDBL catalogs,
`PlatformSnapshotSource` and `QueryTableSnapshotSource`.

Sequential and concurrent transcripts must be byte-identical. The full
canonical files are compared byte-for-byte; SHA-256 is only a compact table
field. After a candidate opens, the parity probe makes SQLite and HBK
unavailable to that running probe and repeats covered lookups to exclude a
hidden fallback.

Documentation parity includes only fields already observable through the
current snapshot/catalog contracts. Full HTML, long descriptions and
search/export payloads are outside T183.

## Compatibility And Safety Prototype Contract

Every candidate header carries and validates its own binary-layout version,
extraction-schema version, source identity/checksum, locale and exact platform
version. Layout version is not cache-format terminology for the SQLite
extraction schema; the values are independent compatibility dimensions.
Platform-version mismatch makes an artifact incompatible and requires a new
artifact.

Numeric string and fact IDs are valid only for the current mapped session.
Replacing the source creates a new ID space; no stability comparison,
migration or persistence across sessions is required.

Mapped artifacts are immutable. A reader holds a shared lock for the stable
logical snapshot slot for the entire mapping lifetime. A writer must acquire a
fail-fast exclusive lock before changing discovery metadata or publishing a
new immutable generation; if readers exist, it returns a typed
snapshot-in-use error and does not wait, truncate, overwrite or rename over an
active mapping.

Before typed access, a candidate validates magic and versions, section bounds,
alignment, range overflow, byte order, UTF-8, enum/tag values and integrity
metadata. The mapping owner and lock outlive every borrowed view. `unsafe`
remains inside a documented safe abstraction and is not used to escape the
ownership model.

## Result Table Contract

The final durable table has one row per dataset/backend/variant and contains:

- branch ancestry, harness commit and candidate commit;
- parity status and canonical content/behavior digests;
- production/write/validation and total local rebuild median/MAD;
- cold-best-effort and warm process-start-to-ready median/MAD;
- first lookup and each batched warm operation;
- allocation count/bytes;
- peak RSS, steady RSS/PSS/private/anonymous/file-backed memory;
- aggregate four-reader PSS;
- minor/major faults and bytes touched when available;
- artifact, section, dictionary and reverse-index sizes;
- relative values against H0 and C0;
- each predeclared gate status.

The table contains no automatic score, rank, “winner” or first-place marker.
The user receives the evidence and makes the selection decision.

For S83, `production` in result fields means release-profile snapshot
formation/rebuild measurement only. It does not mean deployment, production
adoption or canonical-runtime promotion.
