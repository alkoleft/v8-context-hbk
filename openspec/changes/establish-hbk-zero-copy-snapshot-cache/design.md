> **T183 BOUNDED EXPERIMENT — no production format selected.** The decisions
> below authorize isolated prototypes and measurements while preserving a
> separate user/durable-spec gate for format selection and production work.

## Context

`syntax-helper-search::HbkFactSnapshot` is the provider-owned immutable runtime
read model for documented platform, language and query facts. Typed BSL/SDBL
catalogs borrow its arenas and indexes; generic resolver adapters project only
at their compatibility boundary.

The existing derived binary cache preserves that model and loads much faster
than rebuilding it from SQLite, but it is not zero-copy:

1. the complete cache payload is read into a `Vec<u8>`;
2. every string and vector is allocated again;
3. nested callable/signature/parameter vectors become independent heap
   allocations;
4. the payload and the reconstructed snapshot overlap during loading.

The completed snapshot work deliberately deferred `fst`, `rkyv`/`zerovec` and
persisted zero-copy layouts until the owned arena snapshot had concrete
measurements. Those measurements and the deferred provider-startup H5
hypothesis now justify a separate design investigation, but do not select a
format.

The downstream unified semantic entity draft additionally needs compact names.
The current hypothesis is to map the HBK string dictionary first and let a
separate project/session symbol owner reuse it as an immutable base before
adding only BSL/metadata strings absent from HBK. That cross-source overlay is
not an HBK storage responsibility.

## Goals / Non-Goals

**Goals:**

- Measure whether a file-backed snapshot materially improves cold/warm
  provider open time, allocation count, retained heap and peak memory.
- Make accepted HBK facts, strings and physical indexes directly accessible
  through validated borrowed views without deserializing an entity graph.
- Preserve one provider fact owner and the existing catalog/read-handle
  semantics.
- Provide a compact base string dictionary with direct ID-to-text resolution
  and indexed text/canonical-name-to-ID lookup.
- Compare the current SQLite-to-owned, binary-cache-to-owned and zero-copy
  runtime paths on one explicit behavior and resource protocol.
- Define immutable publication and mapping safety for concurrent analyzer
  processes and provider rebuilds.
- Keep snapshot binary-layout, extraction-schema, source and platform-version
  metadata separate from semantic entity identity.
- Establish evidence that lets the downstream symbol work accept or reject the
  HBK-base/project-overlay hypothesis.
- Promote the zero-copy snapshot to the canonical HBK runtime context artifact
  only when the accepted behavior and resource gates pass.

**Non-Goals:**

- Implementing or migrating a production zero-copy cache in T183.
- Making the binary file a public analyzer or interchange format.
- Treating a candidate zero-copy snapshot as canonical before its evidence
  gate passes.
- Replacing the external HBK sources as the authoritative documentation input.
- Owning BSL/metadata strings, their project overlay or a process-global
  interner.
- Adding a persistent universal `SymbolId` or entity ID.
- Storing selected/effective analyzer context, derived analysis facts or
  analyzer caches in the HBK snapshot.
- Retaining heap and mapped provider fact models as production alternatives
  after an accepted cutover.
- Stabilizing an on-disk layout before format, safety and resource gates pass.

## Decisions

### 1. Preserve the provider boundary and gate the canonical runtime owner

The zero-copy snapshot remains private provider state owned by
`syntax-helper-search`. HBK files remain the authoritative external
documentation inputs. During discovery, the current SQLite provider index,
owned runtime snapshot and zero-copy candidate are comparison paths rather
than co-equal production owners.

If the zero-copy candidate passes the accepted behavior and resource gates, it
becomes the single canonical HBK runtime context artifact. SQLite may remain a
private rebuild/index-production input, but snapshot-backed runtime consumers
must not retain or query a parallel SQLite/owned fact model. If the candidate
fails, it is removed and the current accepted runtime path remains canonical.

Resolver/analyzer consumers receive the same typed catalogs and read
capabilities and do not receive file paths, offsets, archived implementation
types or snapshot validation policy.

Snapshot binary-layout version, extraction-schema version, source fingerprint,
locale and exact platform version remain artifact provenance and
compatibility metadata. They are not canonical entity key components. A
platform-version mismatch invalidates the artifact rather than attempting
cross-version reuse.

### 2. Replace the heap runtime model; do not layer over it

The accepted target flow is:

```text
SQLite or provider build input
        -> immutable snapshot file
        -> validated read-only mapping
        -> typed borrowed HBK catalogs
```

The mapped representation must become the single live HBK fact model for the
accepted snapshot-backed consumers. A prototype may compare the current and
candidate models in an isolated benchmark process, but production must not
retain both.

Rebuild construction may use bounded temporary state. Discovery must compare
the current heap-to-file writer with a direct/streaming file builder and decide
whether the former's transient peak is acceptable. Temporary construction data
must be dropped before the mapped snapshot is published to consumers.

### 3. Use an immutable HBK base dictionary with a separate downstream overlay

The mapped file should contain:

- a contiguous UTF-8 byte store;
- compact offsets/lengths indexed by dense HBK string ID;
- a reverse lookup from accepted text or canonical BSL name to base ID;
- explicit distinction between source/display spelling and canonical lookup
  keys where they have different semantics.

The downstream symbol hypothesis is:

```text
base IDs:     0 .. HBK_STRING_COUNT
overlay IDs:  HBK_STRING_COUNT .. PROJECT_STRING_COUNT
```

When BSL/metadata contributes an accepted semantic name, its owner first checks
the immutable HBK reverse index. A hit reuses the base ID; a miss is stored once
in the project/session overlay. The common ID is generation-scoped and cannot
be persisted or compared across unrelated base snapshots/project generations.

HBK exposes only the narrow base-dictionary capability needed for this
composition. It does not depend on BSL/metadata, own the overlay or introduce a
cross-source entity registry.

**Alternative: BSL-local interning only.** Retains independent numeric
namespaces and does not enable direct cross-source equality.

**Alternative: copy HBK strings into a new common table.** Rejected because it
retains a second live string store.

### 4. Compare storage formats as falsifiable hypotheses

At least these bounded candidates must use the same snapshot subset and
workload:

1. SQL-to-owned runtime materialization, which is the baseline rather than a
   zero-copy candidate;
2. current owned binary cache, which is the existing startup-control path;
3. custom flat little-endian sections with offset/range records and validated
   typed slices;
4. the same custom artifact with validated typed fixed sections via `zerocopy`
   only if the flat prototype shows a real lookup decoding cost worth isolating;
5. a validated archived representation such as `rkyv`/`bytecheck`.

`zerocopy` is a candidate implementation aid for fixed-layout sections, not a
complete format decision. An mmap-capable reverse string index such as `fst`
is a candidate only if it improves total file size/lookup/resource behavior
over a simpler sorted/hash index.

Each approach is a hypothesis, not an implementation commitment:

| ID | Hypothesis | Expected win | Rejection rule |
| --- | --- | --- | --- |
| H0 | SQLite provider index materializes the current owned snapshot. | Establishes the canonical SQL baseline for correctness, startup, lookup and memory. | Not a candidate; used as the denominator and behavioral oracle input. |
| C0 | Current binary cache loads an owned snapshot. | Quantifies the existing cache control and the cost that remains after avoiding SQLite materialization. | Rejected as the future runtime owner if retained heap/allocations remain materially worse than a passing mapped candidate. |
| H1 | Custom flat sectioned little-endian snapshot with checked offsets, ranges, arenas, sorted indexes and an interned string dictionary. | Lowest dependency and most explicit mmap safety model; should reduce retained private heap and allocations while preserving lookup latency. | Reject if validation/open cost, page-fault behavior, lookup decoding or file size fails the gates, or if it requires a parallel owned model. |
| H1a | H1 with validated `zerocopy` fixed-record sections where records are alignment-safe and byte-order-safe. | Isolates whether typed fixed sections remove enough decoding overhead to justify the dependency. | Do not prototype if H1 lookup decoding is not a bottleneck; reject if dependency/safety cost has no measured benefit. |
| H2 | Validated private archived mirror using `rkyv`/`bytecheck` without unchecked access. | Tests whether an archive crate shortens implementation while still reducing startup allocations and preserving safety. | Reject if validation touches too much data, file size grows materially, layout compatibility becomes public, or unsafe access cannot be proven unnecessary. |
| H3 | Reverse-index variants over the mapped base dictionary: sorted binary search first, then mapped open-address hash, then FST only if justified. | Determines the smallest immutable name/canonical lookup index that satisfies lookup latency and file-size gates. | Reject heavier variants unless they beat the simpler index on total measured startup, lookup, memory and file-size behavior. |

The accepted layout should flatten nested variable-length data:

```text
Header / section table
String offsets / UTF-8 bytes / reverse lookup
Platform types / members / callables
Signatures / parameters / type references
Globals / language / query facts
CSR and sorted lookup indexes
```

Entity rows contain typed IDs and checked ranges into other sections rather
than native pointers or owned `Vec` fields.

An additional S83 comparison set separates the organization dimensions that
the initial format comparison left coupled:

| ID | Changed dimension | Fixed dimensions |
| --- | --- | --- |
| `S83-L1` | hot/cold page-clustered section order | F0 records, indexes, validation and writer semantics |
| `S83-I1` | mapped open-address reverse/name indexes | F0 records, section order, validation timing and writer |
| `S83-D1` | eager header/directory plus lazy safe section validation/access | F0 records, section order and index algorithms |
| `S83-P1` | two-pass/direct formation without a monolithic output buffer | F0 runtime layout, reader and lookup algorithms |

F0 typed-flat and A0 checked archive remain format/lifecycle references. A0
does not satisfy the requirement to test the four organizational dimensions.
Each derived hypothesis has its own branch/worktree and one primary variable.
Implementation may run in parallel, while measurements on the shared host run
serially with recorded load/memory state and cache stance.

### 5. Treat file immutability and change locking as safety invariants

Mapped bytes must never be modified in place. The writer creates a new
uniquely named or content-addressed file, flushes and validates it, then
publishes it atomically through provider-owned discovery metadata.

Readers take a shared lock through a stable provider-owned lock target and
retain that lock for the complete `HbkFactSnapshot` session. Any operation that
would replace or republish the active snapshot must take an exclusive lock.
Concurrent readers are allowed. A writer that cannot acquire the exclusive
lock because a live reader exists fails immediately with a typed
snapshot-in-use error; it does not wait, truncate, overwrite or replace the
active artifact. The lock protects modification by cooperating provider
processes; the design does not claim that an unrelated program ignoring
advisory locks can be prevented from writing on every supported OS.

The lock is scoped to the provider-owned logical snapshot slot that owns the
discovery metadata/current pointer, not to one content-addressed artifact
inode. It remains stable across artifact generations for that slot and protects
both active-artifact publication and current-pointer changes. Discovery must
define the exact slot key so unrelated platform/locale provider slots can be
updated independently without allowing a new source fingerprint or layout
generation to bypass the active reader lock.

The loader opens a read-only file and validates at least:

- magic and snapshot binary-layout version;
- extraction-schema version;
- source identity, locale and exact platform version;
- file and section bounds;
- alignment and byte order;
- count/range overflow;
- UTF-8 validity for the string store;
- enum/tag validity or safe access-time decoding;
- the accepted integrity/checksum contract.

Unchecked typed access is forbidden before the accepted validation proof. The
design must decide whether full validation/checksum cost defeats lazy paging,
whether section-level validation is sufficient, and how locally generated
trusted artifacts differ from externally supplied files.

No comparison of numeric local IDs across sessions is required. IDs remain
valid only while their owning snapshot session is alive. Opening a replacement
HBK source produces a new snapshot and a new local ID space without migration,
serialization or stability validation of the previous numbering.

### 6. Keep typed borrowed views as the semantic runtime surface

A zero-copy record cannot contain Rust `Vec` or process pointers. The provider
may therefore expose small borrowed values such as a snapshot reference plus a
typed local handle/range. Such a value is the native zero-copy entity reference,
not a DTO or materialized wrapper.

Existing BSL/SDBL catalogs, ordering, ambiguity, availability and typed ID
semantics must remain behaviorally equivalent. Generic `context-resolver-core`
DTOs remain a compatibility projection and must not be stored in the mapped
snapshot.

### 7. Separate true first-run and later-run claims

Mapping a snapshot that already exists may improve the analyzer's first open in
a process. Building the snapshot locally from SQLite cannot improve the
first-ever run that performs that build.

Discovery must choose one lifecycle:

- a snapshot produced by the HBK build/distribution pipeline;
- a locally derived cache that improves only later runs;
- a hybrid that prefers a supplied compatible snapshot and otherwise rebuilds
  provider-owned derived state.

No "first startup" claim is accepted until the artifact's production point and
cold-cache measurement protocol are explicit.

### 8. Gate acceptance on one explicit comparison protocol

The comparison must use the same real HBK corpus, exact platform version,
locale, extraction-schema version and lookup workload. It must record the
input artifact identity/checksum, build profile, host/OS, commands, raw
results, run count, warm-up policy, summary statistic and task-local
regression/material-benefit thresholds before candidate results are used for a
decision.

Three path families must be reported separately, with SQL-to-owned as the
baseline row in the final table:

1. SQLite provider index to the current owned runtime snapshot;
2. current binary cache to the owned runtime snapshot;
3. candidate zero-copy snapshot production and mapped runtime access.

The committed hypothesis registry is H1 custom flat checked decoding, H2 the
same H1 artifact/layout with a typed fixed-section reader, and H3 an archive
candidate. H1 and H3 branch from the frozen benchmark-base commit and may run
in parallel worktrees. H2 branches from the exact measured H1 commit, so its
row is explicitly dependent and measures the subtraction from H1 rather than
claiming an independent format result.

The common harness commit is immutable for a comparison set. A harness change
requires a new benchmark-base commit and rerunning all affected H0, C0 and
candidate rows. Candidate performance evidence is ignored until the
independent versioned parity oracle passes.

Candidate commands that already emit the frozen measurement envelope use the
immutable harness `run-command` and `record-parity` entry points directly.
Candidate allocation and four-reader collection use a thin outer
orchestration driver because the frozen baseline helpers hard-code the H0/C0
binaries. That driver may substitute only the candidate executable and
artifact path, must emit the same raw JSON identities, machine-state and
resource fields, and must record both the frozen harness commit and exact
candidate commit. The driver is candidate-side orchestration rather than a
change to the timing/parity harness; it must not transform measured values or
change cache-stance rules.

The following lifecycle scenarios are distinct measurements:

- production/rebuild time, peak RSS and allocations;
- cold compatible open from process start through validation/mapping until the
  provider is ready to accept a query;
- warm compatible open under the same definition;
- first representative lookup after open, including demand page faults;
- batched warm lookup after the source is open;
- post-workload steady state;
- sequential and concurrent multi-process startup/use;
- incompatible platform/layout/extraction version handling and locked update.

Every candidate report must include:

- production/write and validation/open wall-clock time;
- first-lookup and batched warm-lookup latency;
- minor/major page faults and bytes touched where available;
- allocation count/bytes and retained private heap after open and after the
  representative workload;
- peak RSS plus steady-state RSS and PSS where available, because clean mapped
  pages differ from private heap;
- aggregate PSS for the accepted concurrent-reader workload;
- snapshot file size;
- per-index/section footprint where available;
- base-dictionary exact and canonical reverse lookup;
- the complete BSL/SDBL catalog and read-handle parity oracle defined below.

The format is rejected if it merely moves retained bytes from measured heap
into an unaccounted mapped duplicate, retains a fourth provider model, or
regresses an accepted startup/lookup/resource gate without an explicitly
accepted trade-off.

The durable result summary must use one comparison table for all measured
paths. At minimum, each row records the path/hypothesis ID, artifact kind,
behavior oracle status, production/rebuild median and MAD, cold and warm
ready-for-query startup median and MAD, first lookup, batched warm lookup,
allocation count/bytes, peak RSS, steady RSS/PSS/private heap/file-backed
memory, aggregate multi-process PSS, page faults/bytes touched, file and
section/index sizes, relative result versus SQL baseline and the accepted
decision. Raw per-run measurements remain service data under `target/` unless
their conclusions are promoted into `spec/`.

The table is evidence, not a scorecard: it contains no automatic ranking,
winner or first-place marker. Passing gates only keeps a candidate eligible.
The user selects the outcome before any candidate is promoted or merged into
the production branch.

### 9. Define complete behavioral equivalence by observable contracts

Parity compares logical facts and observable provider results, not byte
layout, offsets, memory addresses, allocation order or numeric session-local
IDs. Candidate-local IDs must be normalized through stable provider fact
identity and text before baseline/candidate results are compared.

The full-corpus oracle must compare counts and logical fact sets for:

- platform types, type members, callables and constructors;
- overloads, signatures, parameters, return/type references and ownership
  ranges;
- globals, module contexts and module events;
- language facts;
- enums and enum values;
- SDBL query tables, fields and parameters.

For every fact family it must compare the fields currently observable through
`HbkFactReadHandle`, `HbkBslContextCatalog`, `HbkSdblQueryCatalog`,
`PlatformSnapshotSource` and `QueryTableSnapshotSource`, including names,
aliases/canonical keys, kinds, domains, owners, availability,
available-since, locale, source identity and supported relations.

The query oracle must cover:

- exact fact/type/language/enum/query-table identity;
- platform type name/alias and template family/variant;
- member by owner/name/optional kind;
- callable and constructor by owner/name;
- global by domain/name/optional kind;
- module context and module event;
- query table by name, syntax and identifier;
- query field and parameter by table/name;
- enum value by owner/name;
- availability and relation traversal.

Each query compares normal hits, misses/empty results, multiple candidates,
ambiguity and unsupported outcomes at the layer that owns those outcomes.
Candidate ordering, overload/parameter/member ordering and relation traversal
must be deterministic for identical inputs. Parallel reads must produce the
same logical results as sequential reads.

Parity must also prove that mapped catalogs and downstream adapters continue
to work after the SQLite provider index and source HBK files are made
unavailable to the running probe. This detects hidden runtime fallback.

The exact documentation payload covered by the snapshot remains a discovery
decision. Until that decision is accepted, “documentation parity” means only
fields already observable through the current snapshot/catalog contracts; it
does not silently expand the runtime artifact to full HTML, long descriptions
or search/export payloads.

## Risks / Trade-offs

- [Mapped file is modified or truncated] → hold a shared reader lock for the
  snapshot session, require an exclusive fail-fast writer lock, publish an
  immutable new file and never rewrite a mapped inode/path target in place.
- [Validation touches the entire file and erases cold-start benefit] → compare
  full archive validation, bounded section validation and trusted locally
  generated artifacts; accept only a documented safety model.
- [Random mmap access causes page-fault latency] → group hot dictionary and
  lookup sections, measure cold/warm paths and use advisory prefetch only when
  it produces a repeatable benefit.
- [Archived layout becomes a public compatibility promise] → keep it private
  and versioned; invalidate/rebuild rather than add compatibility readers.
- [HBK base dictionary becomes a cross-source owner] → expose only base
  lookup/resolution; keep project overlay lifecycle and BSL/metadata semantics
  in their separate neutral/downstream owner.
- [Raw and canonical names are conflated] → inventory lookup/display consumers
  and retain distinct typed meanings or mappings.
- [Overlay IDs depend on project load order] → keep IDs generation-local and
  prohibit serialized or ordered-output semantics based on numeric value;
  separately decide whether deterministic assignment is required.
- [New dependency adds more cost than it removes] → compare custom flat,
  archived and current formats before accepting `rkyv`, `zerocopy`, `fst` or
  another crate.

## Migration Plan

1. Capture the exact current cache/open, heap and downstream analyzer baselines.
2. Inventory every snapshot field/index and classify fixed records,
   variable-length ranges, strings, reverse lookups and non-hot payload.
3. Freeze the behavior oracle, measurement protocol and numerical gates before
   using candidate measurements for acceptance.
4. Build isolated throwaway candidates without changing production selection.
5. Accept or reject the format, validation model, canonical runtime owner and
   first-run artifact lifecycle through durable HBK spec/ADR updates.
6. Implement one provider-owned loader/builder path and prove full catalog and
   downstream-adapter parity.
7. On acceptance, cut snapshot-backed consumers over to the mapped owner and
   remove the replaced heap runtime model in the same migration. SQLite may
   remain only in the accepted private rebuild/index-production role.
8. If any acceptance gate fails, delete the candidate path and retain the
   current owned runtime without a compatibility layer.

## Open Questions

- Is the snapshot supplied by HBK distribution, built locally, or hybrid?
- Which format wins the measured custom-flat versus validated-archive
  comparison?
- Can validation remain sound without touching every data page at every open?
- Which strings belong in the reusable base: all HBK strings, only semantic
  names, or separate raw/canonical tables?
- What reverse lookup gives the best total memory/file/lookup result?
- Must project overlay ID assignment be deterministic, or is generation-local
  equality sufficient?
- Which neutral component owns the eventual common symbol type and project
  overlay?
- How are standalone BSL analysis and metadata-only workflows seeded when no
  HBK base is available?
- Which cross-platform shared/exclusive lock mechanism implements fail-fast
  modification protection for all provider-owned writers?
- Which exact resource thresholds define success?
- Does the canonical analyzer-context snapshot include any documentation
  payload beyond the fields already exposed by current snapshot/catalog
  contracts?
