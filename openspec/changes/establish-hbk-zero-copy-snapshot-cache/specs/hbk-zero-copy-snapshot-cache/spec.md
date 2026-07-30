> **T183 BOUNDED EXPERIMENT — the following requirements authorize isolated
> comparison prototypes only.** Production adoption remains blocked until the
> user selects an outcome and the durable HBK specification or ADR accepts it.

## ADDED Requirements

### Requirement: Zero-copy format selection is evidence-gated

The provider SHALL NOT select or implement a production zero-copy snapshot
format until the current owned cache, a custom flat mapped candidate and an
archive candidate with checked validation have been compared on the same provider and
downstream workloads.

#### Scenario: Comparison protocol is frozen before candidate acceptance

- **WHEN** the discovery phase prepares a candidate comparison
- **THEN** it SHALL record the exact HBK/provider artifact identity, platform
  version, locale, extraction-schema version, build profile, host/OS, commands,
  run count, warm-up policy, cold-cache method, summary statistic, raw-result
  format and measurement-tool fallbacks
- **AND** it SHALL define task-local material-benefit and regression thresholds
  before candidate results are used to select the production format.

#### Scenario: Candidate formats are compared

- **WHEN** the discovery phase evaluates zero-copy snapshot storage
- **THEN** it SHALL report the SQLite-to-owned, current-cache-to-owned and
  zero-copy paths separately
- **AND** it SHALL separately record snapshot production/rebuild, cold
  ready-for-query startup from process start through validation/mapping, warm
  ready-for-query startup under the same boundary, first-lookup latency,
  batched warm-lookup latency and post-workload steady state
- **AND** it SHALL record allocation count and bytes, retained private heap
  after open and after the workload, peak and steady-state RSS, PSS when
  available, aggregate multi-process PSS, file size, page faults and bytes
  touched when available
- **AND** behavior, candidate ordering and lookup results SHALL match the
  current snapshot contract.

#### Scenario: Candidate fails its resource gate

- **WHEN** a candidate retains a parallel provider model, has no material
  resource benefit or exceeds the accepted CPU/memory thresholds
- **THEN** the candidate SHALL be deleted rather than retained as an alternate
  production cache path.

### Requirement: Canonical runtime ownership is evidence-gated

HBK files SHALL remain authoritative external documentation inputs. The
zero-copy snapshot SHALL become the single canonical HBK runtime context
artifact only when its behavior and resource gates pass and that decision is
accepted in the durable HBK specification or ADR.

#### Scenario: Candidate has not passed the gate

- **WHEN** discovery or prototype work is still in progress
- **THEN** the current accepted runtime path SHALL remain canonical
- **AND** the candidate SHALL NOT become an alternate production path.

#### Scenario: Candidate passes the gate

- **WHEN** the accepted comparison proves full behavior parity and satisfies
  the startup, lookup and resource gates
- **THEN** the zero-copy snapshot SHALL become the single runtime owner for
  migrated HBK context consumers
- **AND** SQLite MAY remain only as a private rebuild/index-production input
- **AND** runtime consumers SHALL NOT retain or query a parallel SQLite or
  owned HBK fact model.

#### Scenario: Snapshot is missing or invalid

- **WHEN** the snapshot is missing, stale, incompatible, truncated or invalid
- **THEN** the provider SHALL use the rebuild path accepted by the canonical
  runtime decision
- **AND** snapshot-backed resolver/catalog consumers SHALL NOT introduce a
  hidden SQLite/HBK fallback or understand snapshot layout.

#### Scenario: Storage metadata is checked

- **WHEN** the provider opens a file-backed snapshot
- **THEN** it SHALL validate magic, snapshot binary-layout version,
  extraction-schema version, source identity, locale, exact platform version
  and the accepted structural/integrity metadata
- **AND** a platform-version mismatch SHALL invalidate the snapshot rather
  than reuse it across platform versions
- **AND** that storage metadata SHALL NOT become a semantic entity identity or
  canonical-name key.

### Requirement: The mapped snapshot is the single live HBK fact model

An accepted zero-copy snapshot SHALL replace the heap-materialized runtime
snapshot for migrated snapshot-backed consumers and SHALL NOT be retained
beside an equivalent owned provider fact graph.

#### Scenario: Snapshot open succeeds

- **WHEN** a valid zero-copy snapshot is opened
- **THEN** provider facts, strings and physical indexes SHALL be read through
  borrowed mapped views
- **AND** the loader SHALL NOT deserialize the complete payload into owned
  strings, nested vectors or a second fact arena.

#### Scenario: Snapshot is rebuilt

- **WHEN** the provider creates a replacement snapshot
- **THEN** construction-only state SHALL be released before mapped facts are
  published to consumers
- **AND** the final runtime SHALL retain only the mapped provider fact owner.

### Requirement: Mapped files are immutable and validated before typed access

The provider SHALL map only read-only immutable snapshot files and SHALL prove
the accepted file-layout safety invariants before exposing typed borrowed data.

#### Scenario: Reader opens a snapshot session

- **WHEN** a process opens a compatible snapshot
- **THEN** it SHALL acquire a shared modification lock for the provider-owned
  logical snapshot slot that selects the active artifact and retain it for the
  complete lifetime of that snapshot session
- **AND** that lock target SHALL remain stable across content-addressed or
  uniquely named artifact generations in the same slot
- **AND** other readers MAY acquire the same shared lock concurrently.

#### Scenario: Writer attempts to change an active snapshot

- **WHEN** a writer cannot acquire the required exclusive modification lock
  because one or more readers are active
- **THEN** it SHALL fail immediately with a typed snapshot-in-use error
- **AND** it SHALL NOT wait, truncate, overwrite, rename over or republish the
  active snapshot.

#### Scenario: Snapshot is published

- **WHEN** a writer completes a new snapshot
- **THEN** it SHALL hold the exclusive modification lock for the same logical
  snapshot slot while publishing both a newly named or content-addressed
  immutable file and the discovery metadata/current pointer atomically
- **AND** it SHALL NOT truncate, rewrite or mutate a file that may already be
  mapped by another reader.

#### Scenario: Typed data is accessed

- **WHEN** a reader creates typed views over mapped bytes
- **THEN** the provider SHALL first enforce the accepted magic/version, bounds,
  alignment, byte-order, range-overflow, UTF-8, enum/tag and integrity checks
- **AND** malformed bytes SHALL produce provider cache invalidation rather than
  unchecked dereference or undefined behavior.

#### Scenario: HBK source is replaced between sessions

- **WHEN** a new snapshot is produced for a replacement HBK source
- **THEN** it SHALL create a new session-local ID space
- **AND** the provider SHALL NOT compare, migrate, serialize or validate
  numeric local-ID stability against the previous snapshot.

### Requirement: HBK string storage can serve as an immutable base dictionary

The zero-copy snapshot SHALL expose a narrow provider-owned base dictionary
capability with dense generation-scoped IDs, borrowed ID-to-text resolution and
indexed accepted text/canonical-name-to-ID lookup.

#### Scenario: Downstream name exists in HBK

- **WHEN** an authorized downstream symbol composer looks up an accepted
  canonical name already present in the HBK base dictionary
- **THEN** the provider capability SHALL return the existing base ID without
  copying or re-interning the HBK string.

#### Scenario: Downstream name is absent from HBK

- **WHEN** a BSL/metadata semantic name is absent from the HBK base dictionary
- **THEN** HBK SHALL report a normal miss
- **AND** it SHALL NOT store the name, own a project overlay or create a
  cross-source entity registry.

#### Scenario: Base ID crosses generations

- **WHEN** callers compare or persist IDs from unrelated snapshot generations
- **THEN** the contract SHALL treat that operation as invalid
- **AND** no base string ID SHALL be advertised as a persistent universal
  identity.

### Requirement: Borrowed semantic catalog behavior is preserved

The zero-copy storage change SHALL preserve the existing typed BSL/SDBL catalog
and read-handle results, ordering, ambiguity, availability and source identity
behavior.

#### Scenario: Full-corpus logical parity is checked

- **WHEN** the current and candidate snapshots are built from the same source
- **THEN** the parity oracle SHALL compare logical fact counts and sets for
  platform types, members, callables, constructors, overloads, signatures,
  parameters, globals, module contexts/events, language facts, enums/values and
  SDBL tables/fields/parameters
- **AND** it SHALL compare every field currently observable through the
  read-handle, borrowed BSL/SDBL catalogs and snapshot-backed adapters
- **AND** it SHALL normalize session-local numeric IDs through stable provider
  fact identity and text before comparison.

#### Scenario: Existing catalog query runs over mapped storage

- **WHEN** a consumer performs an exact identity, type name/alias,
  template-key, owner/member/kind, callable/constructor, global,
  module-context/event, language, enum/value, query-table/field/parameter,
  availability or relation lookup
- **THEN** it SHALL observe behavior equivalent to the current provider-owned
  snapshot
- **AND** normal hits, misses/empty results, multiple candidates, ambiguity and
  unsupported outcomes SHALL be preserved at the layer that owns them
- **AND** the provider SHALL not construct generic resolver DTOs, selected
  context records or owned signature/parameter projections in the storage
  layer.

#### Scenario: Ordering and concurrency parity are checked

- **WHEN** identical lookups run repeatedly, sequentially and through
  concurrent read-only consumers
- **THEN** candidate, overload, parameter, member and relation ordering SHALL
  remain deterministic
- **AND** concurrent reads SHALL produce the same logical results as
  sequential reads.

#### Scenario: Hidden runtime fallback is excluded

- **WHEN** a mapped snapshot and its borrowed catalogs/adapters have opened
  successfully and the source SQLite/HBK artifacts are made unavailable to the
  running parity probe
- **THEN** all covered lookups SHALL continue to produce the same results
- **AND** no covered runtime path SHALL fall back to SQLite or HBK parsing.

#### Scenario: Documentation parity scope is evaluated

- **WHEN** discovery defines the canonical snapshot payload
- **THEN** it SHALL explicitly decide whether any full documentation text is
  included
- **AND** until that decision is accepted, documentation parity SHALL cover
  only fields observable through current snapshot/catalog contracts rather
  than silently adding HTML, long descriptions or search/export payloads.

#### Scenario: Variable-length children are traversed

- **WHEN** a callable, signature, parameter, type reference or owner/member
  relation is traversed
- **THEN** the provider SHALL use validated ranges/slices into mapped sections
- **AND** traversal SHALL NOT allocate one owned vector per parent entity.

### Requirement: First-run claims identify snapshot production lifecycle

The change SHALL distinguish mapping an existing snapshot from creating the
snapshot for the first time.

#### Scenario: Ready snapshot is supplied

- **WHEN** the accepted HBK build/distribution pipeline supplies a compatible
  snapshot before analyzer startup
- **THEN** cold-start measurements MAY include that mapped snapshot as the
  first analyzer open
- **AND** the artifact producer, source identity and validation work SHALL be
  recorded.

#### Scenario: Snapshot is built locally on first use

- **WHEN** no compatible snapshot exists and the provider builds it from
  SQLite
- **THEN** the build cost SHALL be included in first-ever-run measurements
- **AND** the change SHALL NOT claim that cache mapping removed that initial
  build cost.

### Requirement: Dependency and durable-contract decisions remain explicit

No new zero-copy, mapping or on-disk index dependency SHALL be adopted until a
measured candidate and durable HBK ADR/spec decision own it.

#### Scenario: External crate is proposed

- **WHEN** implementation proposes `memmap2`, `rkyv`, `zerocopy`, `fst` or an
  equivalent dependency
- **THEN** the decision SHALL name the measured bottleneck, safety model,
  lifecycle owner and rejected alternatives
- **AND** durable HBK `spec/` and the affected acceptance baseline SHALL be
  updated before production implementation.
