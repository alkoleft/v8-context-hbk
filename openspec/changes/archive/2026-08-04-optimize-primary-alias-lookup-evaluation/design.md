## Context

The archived `compare-primary-alias-lookup` experiment proved the composite
identity semantics, but its candidate retained four overlapping projections:
a common `KeyPool`, a family `PrimaryNameInterner`, a sorted primary-ID
membership vector and a per-entity ID vector. It also copied the provider's
name-lookup record and matching-range behavior into `LookupEntry` and
`matching_entries`. The resulting extra searches explain part of the alias and
miss latency, while the retained copies inflate the reported memory cost.

This change corrects only the experiment. The frozen provider snapshot remains
the corpus owner, and the composite identity contract remains:

```text
TypeId       = family-local interned normalized primary
CallableId   = (OwnerId, CallableNameId)
PropertyId   = (OwnerId, PropertyNameId)
OwnerId      = Global | Type(TypeId)
```

Applicable architecture invariants are AIR-001 through AIR-003. The benchmark
retains compact IDs and lookup control state only; it does not copy semantic
entities, move identity storage into `v8-context-semantic-entities`, or add a
universal registry/interner.

## Goals / Non-Goals

**Goals:**

- Measure the smallest direct primary/alias representation compatible with
  separate typed identity families, owner-composite member IDs and immutable
  primary-first lookup.
- Reuse provider-owned normalized `StringId` values, `NameLookup`,
  `OwnerNameLookup` and `matching_range` instead of benchmark mirrors.
- Retain one primary lookup entry and zero or one alias lookup entry per
  canonical entity, with the completed typed ID as the value.
- Compare the optimized candidate against the same-run dense merged reference
  and the freshly recorded control run of the over-modelled candidate.

**Non-Goals:**

- Changing production IDs, snapshot records/indexes, X1/cache schemas, public
  read interfaces or provider formation.
- Adding identities or storage to the shared semantic-role crate; AIR-003 keeps
  source identities and indexes provider-owned.
- Solving the known source duplicate primaries or choosing a production hash,
  perfect-hash, radix or incremental index backend.
- Claiming a global optimum across all data structures. “Optimized” here means
  the minimal sorted-vector representation under the already selected
  primary-index-then-alias-index contract.

## Decisions

### Reuse the snapshot string owner

The real-corpus projection SHALL resolve every normalized primary and alias to
the existing `StringId` already interned by `HbkFactSnapshot`. A transient
borrowed reverse map from `&str` to `StringId` may be built during corpus
projection and SHALL be dropped before either construction or lookup is
measured. The deterministic fixture SHALL allocate keys with the existing
`SnapshotBuilder::intern` operation.

This removes the benchmark-local `KeyId` and `KeyPool` without changing the
production snapshot. It also keeps normalization and string interning as common
prepared input rather than charging them to one variant.

Alternative rejected: wrapping benchmark-local ordinals in `StringId`, because
that would create values not owned by the snapshot string table.

### Retain only direct primary and alias entries

The candidate SHALL retain two sorted vectors of the existing
`NameLookup<Id>` record:

```text
primaries: normalized StringId -> completed typed ID
aliases:   normalized StringId -> completed typed ID
```

For `CallableId` and `PropertyId`, `OwnerId` is read from the ID value itself;
it SHALL NOT be repeated in an `OwnerNameLookup` record. The generic lookup
orders and searches entries by `(id.owner(), key, id)` using the existing
`matching_range` behavior. Type IDs use the unit owner.

A family-local `HashMap<StringId, NameToken>` allocates/reuses primary-name
tokens only while the two vectors are constructed. It is dropped when build
returns. The completed ID appears in the primary entry and alias entries; the
candidate SHALL NOT retain a primary-name map, primary membership vector or a
second entity-ID vector.

Alternatives rejected:

- retaining the name-token map, because the direct primary index already owns
  string-to-ID lookup for the immutable HBK snapshot;
- storing owner both in the record and in the composite ID, because it repeats
  identity state and increases retained bytes;
- merging primary and aliases with a marker, because the accepted contract
  explicitly evaluates separate primary and alias indexes;
- adding a hash lookup backend, because that would conflate representation
  deduplication with a different storage algorithm.

### Reuse the current merged-reference mechanics

The dense reference SHALL store its benchmark rows in the existing
`OwnerNameLookup<Owner, LegacyId>` record and call `matching_range`. It remains
a deliberately independent reference because both variants must receive the
same temporarily canonicalized rows, but it SHALL NOT declare another record
or range-search algorithm.

Because both sides receive prepared numeric `StringId` keys, the report SHALL
name this lane “dense merged reference over prepared snapshot StringId keys”.
It SHALL NOT present the lane as current production lookup latency, whose
public operation also normalizes text and compares through the snapshot string
table.

The optimized candidate's differential map from completed ID to canonical
entity SHALL be derived transiently by executing primary lookup for each
canonical row. It SHALL be dropped before timed lookup and excluded from
retained-byte reporting. This keeps test oracle state out of the candidate.

The retained type lookup itself SHALL resolve canonical type rows to `TypeId`
when callable/property owner scopes are projected. `run_family` returns that
lookup rather than an entity-ID vector, so downstream scope mapping reuses the
identity owner instead of creating a second ordinal-to-ID table.

### Compare a fresh control with the optimized run

Before implementation, the committed over-modelled candidate SHALL be run
alone against the frozen corpus and its medians recorded as the control. After
the replacement, the same release command, corpus, sample count, query order
and allocator shall produce the optimized results. The report SHALL show:

- same-run dense merged versus optimized candidate;
- control over-modelled candidate versus optimized candidate;
- absolute nanoseconds/bytes and relative changes;
- the unchanged duplicate/collision counters and the bounded conclusion.

Cross-run percentages SHALL be labelled as such. The same-run dense baseline
provides a stability check; material drift must be explained before accepting
the comparison.

## Structure impact

Searched owners and consumers:

- experiment structures and flows: `KeyId`, `KeyPool`, `CanonicalRow`,
  `LookupEntry`, `MergedNameLookup`, `PrimaryNameInterner`,
  `PrimaryAliasLookup`, `primary_ids`, `entity_ids`, `matching_entries`;
- production string/index owners: `StringId`, `SnapshotBuilder::intern`,
  `NameLookup`, `OwnerNameLookup`, `CsrIndex`, `matching_range`, snapshot
  materialization/read/X1 paths;
- identity owners: snapshot `Hbk*Id`, resolver `FactId` projections and
  `v8-context-semantic-entities` role-only contracts;
- consumers and evidence: snapshot experiment tests, frozen runner,
  allocation observer, archived measurements, Cargo version/features, public
  exports, schemas, examples and CLI. No frontend exists for this provider
  experiment.

Reused: provider snapshot views and normalized `StringId` storage,
`SnapshotBuilder::intern`, `NameLookup`, `OwnerNameLookup`, `matching_range`,
the existing allocation observer, canonicalization/query classification and
the typed experimental identity layouts.

Changed: the private `PrimaryAliasLookup` implementation becomes a direct pair
of existing lookup-record vectors; corpus preparation resolves existing
snapshot string IDs; differential mapping becomes transient.

Deleted: `KeyId`, `KeyPool`, `LookupEntry`, `matching_entries`, retained
`PrimaryNameInterner`, `primary_ids`, returned `entity_ids` and
`LegacyOwnerId`. No semantic entity, registry, DTO, reader, parser,
normalizer, serializer, cache shape, schema, conversion, public re-export or
production mapping is added.

Required contract: the OpenSpec experiment requires direct primary-first
lookup, separate alias fallback, owner-composite member identity and
reproducible resource evidence. That contract justifies the remaining private
experimental ID newtypes; no current production owner expresses this
hypothetical layout, and the neutral role crate is explicitly prohibited from
owning it by AIR-003.

## Reintroduction guard

Root cause: construction-only token allocation and differential-test mappings
were retained as runtime candidate indexes, while existing snapshot lookup
records/behavior were copied under benchmark-local names.

Single allowed flow:

```text
snapshot StringId + canonical compact rows
  -> construction-only family token allocator
  -> direct NameLookup<typed ID> primary/alias vectors
  -> one generic primary-first lookup using matching_range
```

The dense reference alone may independently reproduce merged semantics, but it
must use `OwnerNameLookup` and `matching_range`. Differential entity maps are
test-oracle state, must be scoped outside measurement and must not be fields of
either representation.

The final review SHALL run:

```text
rg -n 'struct (KeyId|KeyPool|LookupEntry|PrimaryNameInterner)|primary_ids|entity_ids|matching_entries|LegacyOwnerId' crates/syntax-helper-search/src/snapshot/primary_alias_lookup_experiment.rs
rg -n 'StringId\(' crates/syntax-helper-search/src/snapshot/primary_alias_lookup_experiment.rs
rg -n 'struct (ProjectedCorpus|PrimaryAliasLookup|MergedNameLookup)|reverse|by_text|by_string' crates/syntax-helper-search/src/snapshot/primary_alias_lookup_experiment.rs
rg -n 'NameLookup<|OwnerNameLookup<|matching_range' crates/syntax-helper-search/src/snapshot/primary_alias_lookup_experiment.rs
git diff -- crates/syntax-helper-search/src/snapshot/types.rs crates/syntax-helper-search/src/snapshot/indexes.rs crates/syntax-helper-search/src/snapshot/materialize.rs crates/syntax-helper-search/src/snapshot/read.rs crates/syntax-helper-search/src/snapshot/x1_format.rs
cargo check -p syntax-helper-search --lib
```

Expected evidence: the first two prohibited searches are empty; inspection of
the third search proves `ProjectedCorpus`, `PrimaryAliasLookup` and the dense
reference retain no reverse string map; the experiment reuses the three named
production owners; production snapshot-file diff is empty; and the feature-off
library builds. Any direct `StringId(...)` construction, differently named
retained name/reverse map, membership vector, entity-ID mirror, owner
repetition or copied range-search implementation blocks completion.

## Codebase-Design Review Record

### Pre-implementation pass — 2026-08-04 — PASS

- Scope reviewed: the private snapshot experiment, production string/index
  owners, deterministic fixture, frozen projection, differential oracle and
  measurement/report path.
- Module interface: `PrimaryAliasLookup<Id>` retains only direct primary and
  alias entries and exposes construction, lookup and retained-byte accounting;
  this hides token allocation, ordering and primary-first semantics behind one
  small test-only interface used by all three families.
- Seam and adapters: the seam remains inside the existing feature-gated
  snapshot experiment. No external seam or adapter is introduced; corpus
  projection directly reuses snapshot-owned IDs and records.
- Owners and locality: `HbkFactSnapshot` owns strings, `indexes` owns records
  and range behavior, each candidate family owns its token state, and the
  neutral semantic-role crate remains storage-free.
- Findings resolved: label the dense lane as a reference over prepared
  `StringId` keys rather than production latency; prohibit direct `StringId`
  construction and retained reverse maps; return/reuse the type lookup for
  member scope projection rather than retaining an ordinal-to-TypeId mirror.
- Deletion test: removing `PrimaryAliasLookup` would redistribute
  primary-first/collision behavior across three families, so the generic module
  earns its interface; the copied record/range and retained mapping modules do
  not, and are deleted.
- Skeptic review: APPROVED after the guard corrections above.

### Actual-diff pass — 2026-08-04 — PASS

- Scope reviewed: actual source diff is limited to the feature-gated
  `primary_alias_lookup_experiment` module plus this OpenSpec evidence; the
  production snapshot/read/materialize/X1 files named in the guard have empty
  diff.
- Module interface: `PrimaryAliasLookup<Id>` retained the planned small
  interface and exactly two `Vec<NameLookup<Id>>` fields; no wrapper around the
  dense reference remains, and the reference uses `OwnerNameLookup` plus
  `matching_range`.
- Structure reconciliation: the deleted mirrors (`KeyId`, `KeyPool`,
  `LookupEntry`, `PrimaryNameInterner`, `primary_ids`, `entity_ids`,
  `matching_entries`, `LegacyOwnerId`) are absent; direct `StringId(...)`
  construction is absent; `ProjectedCorpus` does not retain a reverse string
  map.
- Behavior reconciliation: deterministic tests cover typed ID sizes, owner
  isolation, primary-before-alias semantics, alias ambiguity, duplicate
  canonicalization and structural absence; the frozen release run validates the
  real 8.3.27 corpus and preserves duplicate counters.
- Verification: `cargo fmt --all -- --check`, focused experiment tests with
  `snapshot-experiment` and `snapshot-experiment-alloc`, full
  `cargo test -p syntax-helper-search --features snapshot-experiment`,
  `cargo check -p syntax-helper-search --lib`, `cargo clippy -p
  syntax-helper-search --all-targets --features snapshot-experiment-alloc -- -D
  warnings -A clippy::needless_return`, `git diff --check` and the exact
  guard searches passed.
- Fresh reviewer: APPROVED the current diff with no blocking findings and
  independently confirmed the measurement arithmetic and guard evidence.
- Architecture documentation: no update is required because the actual diff
  remains inside the existing private feature-gated experiment and changes no
  crate responsibility, dependency direction, provider boundary, public
  contract, cache/schema shape or production orchestration.

## Risks / Trade-offs

- [Direct entries discard the retained reverse token map] -> This is valid only
  for the immutable HBK experiment; future session-wide on-demand allocation
  requires a separate provider-owned design and measurement.
- [Alias fallback still executes two searches] -> Report it as the intrinsic
  cost of the accepted separate-index semantics; do not add a hidden negative
  cache or merged marker.
- [Using `StringId` can accidentally refer to another table] -> Real rows use
  IDs from the corpus snapshot, fixtures use `SnapshotBuilder::intern`, no
  direct `StringId(...)` construction is allowed, and the reverse map remains
  a projection-local borrowed value rather than a corpus field.
- [Cross-run control can contain noise] -> Compare the same-run dense baseline
  with the control baseline and label every over-modelled/optimized percentage
  as cross-run.
- [Source duplicate primaries remain] -> Keep the same temporary stable filter
  and continue to block production cutover.

## Migration Plan

1. Record the control run from commit `0cb8ad0`.
2. Replace only the private experiment representation and update behavior tests.
3. Run focused/full verification and the frozen release measurement.
4. Record the comparison and review the actual diff against the deletion guard.
5. Bump the workspace patch version, archive/synchronize OpenSpec and commit.

Rollback is the single task commit; production behavior is never switched.
This completed internal experiment is a patch-version change.

## Open Questions

None. A production identity/index owner and any on-demand BSL allocation remain
future OpenSpec work.
