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

## Exact Initial Corpus

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

## Mandatory Behavioral Oracle

Parity is an independent mandatory gate. Performance values do not count for a
candidate until its parity status is `pass`.

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
