## Context

The provider-owned `HbkFactSnapshot` currently assigns dense `Hbk*Id`
ordinals independently of naming and stores normalized primary names and
aliases in the same sorted name indexes. Consequently a lookup does not
distinguish a canonical-name hit from an alias hit and can return several
candidates for either case.

The identity hypothesis under evaluation replaces the relevant snapshot-local
ordinals with three compact and separate identity families. `TypeId` is the
family-local interned canonical type-name token. `CallableId` and `PropertyId`
are composite `(OwnerId, family-local interned canonical-name token)` values.
A normalized scoped primary name establishes one identity; aliases are
secondary keys and never establish a second identity.
The production snapshot is immutable after construction, so lookup itself must
remain borrowed and non-inserting.

The experiment is owned by `crates/syntax-helper-search/src/snapshot`. That is
the current in-memory snapshot/index owner and avoids measuring SQLite. The
existing `SearchIndex::type_identities_by_name` and
`type_identities_by_alias` methods were inspected but rejected as the benchmark
seam: they are SQL search APIs, not the worker-safe snapshot lookup whose shape
would change.

## Goals / Non-Goals

**Goals:**

- Compare the current merged primary-and-alias lookup with one reusable
  primary-first/alias-fallback implementation over the same canonicalized HBK
  rows, normalized keys and query order.
- Instantiate that implementation independently for types, callables and
  properties, using a four-byte `TypeId` and distinct compact composite
  `CallableId` / `PropertyId` values.
- Cover platform types and enums as types; constructors, global methods, type
  methods and events as callables; and global properties, type properties and
  enum values as properties.
- Preserve owner scope by using no owner for type names and
  `OwnerId::Global` versus `OwnerId::Type(TypeId)` for callable/property
  identities and lookup keys.
- Record behavior, construction and lookup cost on the frozen 8.3.27 provider
  corpus.

**Non-Goals:**

- Changing production `HbkPlatformTypeId`, `HbkCallableId`, member IDs,
  snapshot fields, X1 sections, serializers or public read APIs.
- Introducing a shared identity registry, provider discriminator, persisted
  identity, generation/revision bits, signature IDs or parameter IDs.
- Changing HBK formation/extension composition. The experiment only makes the
  target uniqueness premise explicit and reports source duplicates.
- Reusing `context-resolver-core::TypeId` or `CallableId`. Those are
  source-qualified `FactId` projections owned by the resolver contract, not
  compact snapshot-local identities; using them would measure a different
  model and reverse the intended ownership direction.

## Decisions

### Use one generic mechanism with independent family state

The benchmark SHALL define one private
`PrimaryAliasLookup<Scope, Id>` implementation. It owns a primary index and an
alias index and performs exactly this operation:

1. search the primary index;
2. return its single typed ID when present;
3. otherwise search the alias index and return its zero, one or several typed
   IDs.

The type, callable and property lanes each own a separate instance. The generic
code is shared; string/index storage and identity allocation are not combined
across families. This separates reusable lookup behavior from provider/domain
identity ownership.

“Generic” is bounded to the three evaluated lanes inside the private
experiment module. This change does not propose a workspace utility or assert
that the abstraction belongs in production; only a later production change may
make that decision from the recorded evidence.

Alternatives rejected:

- three copied family-specific lookup implementations, because their ordering
  and collision behavior would drift;
- one common `StringId` or entity registry, because it would erase which
  identity family/provider owns the entity;
- a shallow production facade over the current indexes, because the task is an
  experiment and must not widen the snapshot surface.

### Make member identity explicitly owner-composite

The candidate identity layout SHALL be:

```text
TypeId       = interned normalized type primary
CallableId   = (OwnerId, CallableNameId)
PropertyId   = (OwnerId, PropertyNameId)
OwnerId      = Global | Type(TypeId)
```

`CallableNameId` and `PropertyNameId` are separate family-local interned
primary-name tokens. Therefore `Array.Add` and `ValueTable.Add` reuse the same
callable-name token but have different `CallableId` values because their
`OwnerId` differs. A global callable/property uses `OwnerId::Global` and cannot
collide with an equally named member of a type. The benchmark SHALL use a
compact representation with a reserved global owner encoding and SHALL verify
that each composite ID occupies eight bytes; it SHALL not introduce provider
slots, persistence or generation/revision bits.

The family interner sees primary names only. Aliases never create a
`TypeId`, `CallableNameId` or `PropertyNameId`. Repeated primary text under a
different member owner reuses the family-local name token; repeated
`(OwnerId, primary token)` is the scoped duplicate handled by the temporary
canonicalization rule.

Alternatives rejected:

- one dense callable/property ordinal, because it does not encode the owner
  component required to distinguish equal member names;
- interning `(owner, text)` as one opaque value, because it hides the requested
  composite structure and prevents reuse of the same family-local name token;
- one common member-name interner, because callable/property identity families
  remain distinct.

### Keep lookup mechanics comparable

Both variants SHALL use the same canonical rows, normalized experiment keys,
owner semantics and deterministic query order. The old reference assigns the
current independent dense four-byte family ordinals, stores primary and alias
entries together and performs one range lookup. The candidate interns primary
names into the layouts above, stores them in separate primary and alias ranges
and performs primary-first fallback. Common normalization/key preparation is
measured separately from both construction paths; old/new ID formation is part
of the respective construction measurement.

Differential checks compare the canonical source-row identity behind returned
old/new IDs. They do not add a result-remapping step to either timed lookup.

The old variant is a deliberately independent reference implementation for
differential testing. It reproduces the current merged-name behavior without
calling SQL or copying provider facts into production state.

Alternatives rejected:

- comparing the candidate with `SearchIndex`, because SQLite and hydration
  would dominate the result;
- using a hash table for only one variant, because that would conflate the
  primary/alias decision with a storage-backend decision;
- invoking production lookup and then remapping its `Hbk*Id` results, because
  the remap would be charged only to the baseline.

### Canonicalize the corpus before both variants

The target invariant is one normalized primary key per family scope. The
benchmark SHALL therefore make one deterministic, stable, first-seen
canonicalization pass over types, callables and properties before constructing
either lookup. Duplicate `(scope, normalized_primary)` rows SHALL be omitted
from both variants and counted by family.

This drop is explicitly temporary experiment scaffolding. It is not an
accepted production conflict policy: the HBK formation/extension algorithm
must establish the invariant before any production cutover.

For type duplicates, every source owner ordinal SHALL map to the retained
canonical type row; the old variant maps it to its dense type ordinal and the
candidate maps it to the retained `TypeId`. Callable/property duplicates do not
allocate another old ordinal or composite ID. Missing aliases and aliases equal
to the primary remain represented as source facts.

### Make collision semantics observable

Primary uniqueness means a primary lookup returns exactly one typed ID. Alias
keys remain one-to-many because two entities can share an alias. When the same
scoped key is both a primary and an alias of another entity, the candidate
returns the primary identity and does not mix alias candidates into the result.
The old merged index returns the merged candidate set.

Differential equivalence SHALL therefore be required for primary hits, alias
hits and misses whose key has no primary/alias collision. Collision queries
SHALL instead verify and report the intentional semantic difference. Alias to
alias collisions SHALL preserve every distinct typed candidate in deterministic
ID order.

### Use the provider snapshot as the corpus owner

The ignored real-corpus experiment SHALL build `HbkFactSnapshot` through
`HbkFactSnapshot::build_from_provider_path` using an explicitly supplied frozen
8.3.27 provider-index path. Rows are projected only through snapshot-owned
views and normalized with the provider-owned normalization function. The
experiment SHALL not read downstream analyzer internals or parse HBK/SQLite
schema privately.

A small deterministic fixture SHALL cover invariants and collision behavior in
ordinary tests. The frozen corpus run supplies cardinality and performance
evidence, not the only correctness oracle.

The projection is fixed as follows; no other storage or SQL fields are an
allowed source:

| HBK fact family | Snapshot-owned source | Included predicate | Benchmark lane / scope |
| --- | --- | --- | --- |
| platform type | `counts().platform_types`, `HbkFactReadHandle::platform_type(HbkPlatformTypeId)`, `HbkPlatformTypeView::name` | every row | type / `()` |
| enum | `counts().enums`, `HbkFactReadHandle::enum_fact(HbkEnumId)`, `HbkEnumView::name` | every row | type / `()` |
| constructor, global method, type method, type event | `counts().callables`, `HbkFactReadHandle::callable(HbkCallableId)`, `HbkCallableView::{kind, owner, name}` | `Constructor`; `GlobalMethod`; `Method`; or `Event` with `Some(owner)`; ownerless module events and `LanguageFunction` are excluded | callable / `OwnerId::Global` or mapped `OwnerId::Type(TypeId)` |
| type property | `counts().type_members`, `HbkFactReadHandle::type_member(HbkTypeMemberId)`, `HbkTypeMemberView::{kind, owner, name}` | `HbkTypeMemberKind::Property` only; method/event/member enum-value projections are excluded | property / mapped `OwnerId::Type(TypeId)` |
| global property | `HbkFactReadHandle::global_fact_ids`, `HbkFactReadHandle::global_fact`, `HbkGlobalFactView::{kind, name}` | `HbkGlobalFactKind::Property` only; global methods use their callable record | property / `OwnerId::Global` |
| enum value | `counts().enum_values`, `HbkFactReadHandle::enum_value(HbkEnumValueId)`, `HbkEnumValueView::{owner, name}` | every row; the enum owner is mapped through its canonical type row | property / mapped `OwnerId::Type(TypeId)` |

Names are resolved through `HbkFactReadHandle::string` from the views'
`HbkNameView::{primary, alias}` IDs and normalized once with the provider-owned
normalizer before either variant is built.

### Measure construction, retained storage and lookup separately

For every family and variant the report SHALL include:

- canonical row count, primary-entry count, alias-entry count and temporarily
  dropped duplicate-primary count;
- construction wall time, allocation calls/bytes and peak live-byte growth
  when `snapshot-experiment-alloc` is enabled;
- retained identity/key/index payload and capacity bytes, including the
  candidate's per-entity composite IDs rather than hiding them in allocation
  totals;
- release-mode primary-hit, alias-fallback, miss, primary/alias-collision and
  owner-isolation timings over a fixed deterministic query order;
- a checksum and exact behavior assertions so optimization cannot remove work.

Construction results SHALL be labelled as the full “dense ordinal baseline”
versus “composite interned identity candidate” cost. They SHALL NOT be
presented as the isolated cost of splitting primary and alias indexes, because
candidate construction intentionally includes three family-local primary-name
interners while the old baseline assigns dense ordinals. Lookup hot-path
timings remain a separate comparison after both representations are built.

At least seven measured samples SHALL be recorded after warm-up. The durable
conclusion SHALL report medians and enough environment/corpus provenance to
reproduce the run. No universal pass/fail latency threshold is imposed by this
experiment; the purpose is to expose the cost and semantic trade-off before a
production decision.

`snapshot/x1_format.rs` remains the single test-binary owner of
`#[global_allocator]` under `snapshot-experiment-alloc`. The new module SHALL
only call `experiment_allocation_snapshot`; it SHALL NOT declare another global
allocator or add allocator/storage accounting outside the experiment. Retained
bytes are limited to experiment key bytes and `Vec` payload/capacity bytes that
can be computed locally.

The acceptance run is:

```text
V8_CONTEXT_HBK_PRIMARY_ALIAS_INDEX=/absolute/path/to/8.3.27.1859/shcntx_ru.sqlite \
cargo test -p syntax-helper-search --release \
  --features snapshot-experiment-alloc \
  snapshot::primary_alias_lookup_experiment::primary_alias_lookup_real_corpus \
  -- --ignored --exact --nocapture
```

The runner SHALL fail with a clear missing-variable error when the environment
variable is absent. It SHALL validate `locale = ru`, source extraction schema
`11`, and a `source_hbk` identifying `8.3.27.1859`; absence or mismatch blocks
completion because no substitute synthetic performance result satisfies the
frozen-corpus requirement.

## Structure impact

Searched owners and consumers:

- snapshot storage/build/read: `snapshot/mod.rs`, `materialize.rs`,
  `indexes.rs`, `read.rs`, `types.rs`, `views.rs`, `x1_format.rs`;
- normalization and current SQL identity lookup:
  `normalize_lookup_key`, `SearchIndex::type_identities_by_name`,
  `SearchIndex::type_identities_by_alias`, `document_names`;
- current and proposed identity names: `StringId`, `HbkPlatformTypeId`,
  `HbkTypeMemberId`, `HbkCallableId`, `TypeId`, `MemberId`, `CallableId`,
  `PropertyId` across `syntax-helper-search` and `context-resolver-core`;
- benchmark/memory support: `snapshot-experiment`,
  `snapshot-experiment-alloc`, `experiment_allocator`, existing ignored frozen
  corpus measurements and fixtures;
- provider consumers and contracts: snapshot read adapters/tests, X1 reader,
  cache/serializer specs, resolver projections, examples, scripts and the
  workspace Cargo feature/version surface. No frontend or external serialized
  output exists in this repository for this experiment.

Reused: provider-owned snapshot construction/views, normalization semantics,
the existing experiment allocation observer, current four-byte dense baseline
ID layout and sorted range-lookup behavior.

Added: one test-only feature-gated benchmark module; private experimental
`TypeId`, `OwnerId`, `CallableNameId`, `PropertyNameId`, composite `CallableId`
and composite `PropertyId`; three family-local primary-name interners; one generic
`PrimaryAliasLookup<Scope, Id>`; one deliberate merged reference; deterministic
fixture/query/report helpers; and OpenSpec measurement evidence.

Deleted or changed in production: none. No reader, parser, normalizer, loader,
serializer, cache key, registry, schema, mapping table, conversion path, public
re-export or provider fact is added or changed. The only mappings are the
benchmark-local projection from existing snapshot views into the two compared
layouts and the temporary canonical-owner mapping required by the stated
experiment invariant.

## Reintroduction guard

Root cause: primary/alias behavior can be copied per entity family, while a
misguided deduplication can move identity ownership into one common string or
entity registry.

Single allowed flow: snapshot views -> one benchmark canonicalization -> three
independent typed family instances of the same private
`PrimaryAliasLookup<Scope, Id>`. Production continues to use its existing
snapshot fields and read handle.

Within the candidate, the only allowed identity flow is type primary ->
`TypeId`, callable primary -> `CallableNameId` -> `(OwnerId, CallableNameId)`,
and property primary -> `PropertyNameId` -> `(OwnerId, PropertyNameId)`.
Aliases may reference those completed IDs but may not allocate tokens.

Verification SHALL fail if the experiment introduces a common `EntityId` or
identity registry, if more than one primary/alias algorithm appears, if a
family uses another family's ID type/state, if the benchmark is compiled
without the existing experiment/test gate, or if production snapshot/X1/public
fields change. Final diff review SHALL repeat searches for those prohibited
shapes and compare the actual diff with this section.

This is an explicit structural/manual diff guard, enforced with these recorded
commands rather than a broad source-shape test coupled to private
decomposition:

```text
rg -n '#\[global_allocator\]|struct (EntityId|IdentityRegistry)|struct PrimaryAliasLookup' \
  crates/syntax-helper-search/src
git diff -- crates/syntax-helper-search/src/snapshot/types.rs \
  crates/syntax-helper-search/src/snapshot/indexes.rs \
  crates/syntax-helper-search/src/snapshot/materialize.rs \
  crates/syntax-helper-search/src/snapshot/read.rs \
  crates/syntax-helper-search/src/snapshot/x1_format.rs
cargo check -p syntax-helper-search --lib
```

Expected evidence: one existing allocator declaration in `x1_format.rs`, one
private generic lookup definition in the experiment, no common identity type,
an empty production snapshot-file diff, and a feature-off library build.
The same review SHALL reject an owner interner or a shared callable/property
member-name interner; exactly the type, callable-name and property-name
interners are allowed.

## Codebase-Design Review Record

### Pre-implementation pass — 2026-08-04 — PASS

- Owner: the experiment stays beside `HbkFactSnapshot`, the current owner of
  the compared lookup and corpus views.
- Interface depth: one generic operation expresses the real variation
  (`Scope`, `Id`) and hides primary/alias range mechanics; the three instances
  retain independent state.
- Seams/adapters: the only projection is at the benchmark boundary; SQL and
  resolver DTOs are not introduced into the hot path.
- Findings resolved: rejected the existing SQL split lookup as the wrong
  performance seam; rejected importing resolver `FactId` wrappers; required a
  single temporary canonicalization shared by both variants; required the
  merged baseline to be named as a differential reference rather than a second
  production owner; replaced dense callable/property candidate ordinals with
  the required owner-plus-family-name composite identities.

### Actual-diff pass — 2026-08-04 — PASS

- The fresh reviewer approved correctness, performance fairness, identity
  ownership, alias semantics and the codebase-design shape without blocking
  findings.
- The actual production diff is limited to the feature-gated module
  declaration. All identity layouts, projections, canonicalization, reference
  lookup and measurements remain private to one test-only module.
- The module deliberately remains one cohesive experiment: its fixture,
  reference, candidate and measurement code share one private contract, while
  splitting it would add shallow navigation without creating a real boundary.
- The diff accounts for exactly the structures and mappings listed in
  `Structure impact`. It adds no provider registry, common entity/string ID,
  owner interner, public re-export, production snapshot field or serialized
  representation.
- The structural guard found one existing global allocator, one generic
  `PrimaryAliasLookup`, no prohibited common identity type, an empty diff for
  production snapshot storage/read/X1 files and a successful feature-off
  library build.
- Focused and full feature tests, the allocation-enabled release corpus run,
  formatting, clippy and strict OpenSpec validation passed. The reviewer also
  confirmed that aliases allocate no identity tokens and that differential
  remapping stays outside timed lookup.

## Risks / Trade-offs

- [The frozen corpus still contains formation-time duplicate primaries] ->
  Canonicalize identically for both variants, report every drop and prohibit
  interpreting first-seen retention as production policy. A non-zero duplicate
  count in any family explicitly blocks a production cutover until
  formation/extension composition establishes the invariant.
- [Separating indexes makes alias hits perform two searches] -> Measure primary
  and alias workloads independently and report the query mix, not one blended
  number.
- [Primary-first semantics differs from the merged index on collisions] ->
  Test collision classes explicitly and compare equivalence only where the
  contracts overlap.
- [A microbenchmark can overstate lookup cost relative to normalization] ->
  Report pre-normalized hot-path and common key-preparation costs separately.
- [Process-global allocation counters can include concurrent test activity] ->
  Run the ignored measurement test alone and record that command.

## Migration Plan

1. Land and validate the OpenSpec design/spec/tasks.
2. Add the isolated benchmark and deterministic behavior tests under the
   existing snapshot experiment gate.
3. Run release measurements against the frozen corpus and record the result.
4. Remove all benchmark-only state by reverting the isolated module if the
   hypothesis is rejected; production lookup remains untouched either way.
5. Complete/archive this internal experiment with a patch workspace version
   bump. Any production identity or snapshot migration requires a separate
   accepted change.

## Open Questions

None for the experiment. Production treatment of formation/extension records
and any cross-provider identity owner remain decisions for a later change.
