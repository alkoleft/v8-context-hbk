## ADDED Requirements

### Requirement: One primary-first lookup implementation remains family-scoped

The experiment SHALL implement primary-name lookup followed by alias fallback
once as a generic mechanism and SHALL instantiate it with independent state for
`TypeId`, `CallableId` and `PropertyId`. `TypeId` SHALL be a distinct four-byte
interned canonical type-name token. `CallableId` and `PropertyId` SHALL be
distinct eight-byte composites of `OwnerId` and their own family-local interned
canonical-name token. A common entity identity, member-name token family or
identity registry SHALL NOT be introduced. The generic mechanism SHALL remain
private to the three evaluated experiment lanes and SHALL NOT become a
production/workspace abstraction in this change.

#### Scenario: Same mechanics serve all three families

- **WHEN** equivalent primary, alias and missing-name queries are executed for
  the type, callable and property collections
- **THEN** all three collections use the same primary-first/alias-fallback
  implementation
- **AND** each result contains only that collection's typed IDs
- **AND** inserting or resolving one collection does not mutate either other
  collection.

#### Scenario: Global and owned members remain isolated

- **WHEN** the same callable or property name occurs in the global context and
  under one or more `TypeId` owners
- **THEN** the interned callable/property name token is reused within its own
  family
- **AND** each resulting ID is composed with `OwnerId::Global` or the requested
  `OwnerId::Type(TypeId)`
- **AND** it does not return candidates from another owner scope.

#### Scenario: Same callable name occurs on two types

- **WHEN** two types both declare callable `Добавить`
- **THEN** their `CallableNameId` component is equal
- **AND** their composite `CallableId` values differ because
  their `OwnerId::Type` components differ.

#### Scenario: Same property name occurs on two types

- **WHEN** two types both declare the same property primary name
- **THEN** their `PropertyNameId` component is equal
- **AND** their composite `PropertyId` values differ because their
  `OwnerId::Type` components differ.

### Requirement: Primary identity is distinct from alias search

The experiment SHALL assign one typed identity to each unique normalized
primary key in its family scope. Type primaries SHALL allocate `TypeId` tokens;
callable/property primaries SHALL allocate or reuse their family-local name
token and compose it with `OwnerId`. It SHALL search primary keys first and
SHALL search aliases only when no primary identity exists for the scoped key.
Alias keys SHALL reference existing identities and SHALL NOT allocate name
tokens or identities.

#### Scenario: Primary hit wins over an alias collision

- **WHEN** a scoped normalized key is the primary name of one entity and an
  alias of another entity
- **THEN** the candidate lookup returns only the primary entity ID
- **AND** the merged reference exposes its current combined candidate set
- **AND** the report classifies the difference as an intentional collision
  semantic rather than an equivalence failure.

#### Scenario: Alias fallback preserves ambiguity

- **WHEN** no primary matches a scoped key and two entities share that alias
- **THEN** alias fallback returns both distinct typed IDs in deterministic ID
  order
- **AND** it does not silently choose the first alias candidate.

#### Scenario: Missing alias does not add an index row

- **WHEN** an entity has no alias
- **THEN** it still has one identity and one primary lookup entry
- **AND** no alias entry is synthesized.

### Requirement: Target primary uniqueness is evaluated explicitly

The experiment SHALL canonicalize the input presented to both compared
variants to one row per `(family scope, normalized primary)` before assigning
the compared typed IDs. Until HBK formation/extension composition establishes
that invariant at its owner, the canonicalization SHALL be marked temporary,
SHALL retain the first row in stable provider order and SHALL report every
discarded row by family.

#### Scenario: Duplicate type primaries are present in source data

- **WHEN** two source type rows have the same normalized primary name
- **THEN** both old and new variants receive the same first canonical type row
- **AND** both source owner ordinals map to its one `TypeId`
- **AND** the report increments the temporary type-duplicate count.

#### Scenario: Duplicate scoped callable or property primaries are present

- **WHEN** two callable or property rows have the same owner scope and
  normalized primary name
- **THEN** both variants receive only the stable first row
- **AND** no second `CallableId` or `PropertyId` is assigned
- **AND** the corresponding temporary duplicate count is reported.

### Requirement: Old and new lookup comparison is controlled

The experiment SHALL compare a deliberate reference for the current merged
primary-plus-alias behavior with the generic primary-first/alias-fallback
candidate using identical canonical rows, normalized keys, owner semantics and
deterministic query order. The reference SHALL use independent dense four-byte
family ordinals; the candidate SHALL use the specified interned/composite IDs.
It SHALL not use SQL lookup or charge a result-ID remapping to either timed
variant.

#### Scenario: Non-colliding behavior is equivalent

- **WHEN** the fixed query corpus contains primary hits, alias-only hits and
  misses without a primary/alias collision
- **THEN** the merged reference and candidate resolve the same canonical
  entity candidates
- **AND** an exact differential assertion verifies the equivalence.

#### Scenario: Lookup mechanics are measured independently

- **WHEN** release-mode measurements are executed
- **THEN** common snapshot loading, normalization and experiment-key
  preparation are outside both compared lookup timings or are reported as a
  shared phase
- **AND** the old and new lookup loops use the same query sequence and sample
  count
- **AND** a checksum proves that every result was consumed.

### Requirement: Resource evidence is reproducible

The experiment SHALL record frozen-corpus provenance, build profile, command,
sample count and environment together with per-family/per-variant construction
time, allocation observations, retained identity/index/key bytes and lookup
timing for primary hits, alias fallback hits, misses, primary/alias collisions
and owner isolation. Candidate retained bytes SHALL include the per-entity
composite ID table. At least seven post-warm-up samples SHALL contribute to
every reported median.

Construction evidence SHALL label the complete dense-ordinal baseline and the
complete composite-interned candidate costs; it SHALL NOT attribute their
difference solely to primary/alias index splitting. Lookup timing SHALL be
reported separately after construction.

If any scoped family reports a non-zero duplicate-primary count, the recorded
conclusion SHALL state that a production cutover is blocked until the owning
formation/extension composition establishes the uniqueness invariant.

#### Scenario: Frozen corpus measurement is recorded

- **WHEN** the ignored real-corpus experiment runs against the frozen 8.3.27
  provider index in release mode
- **THEN** it validates the corpus identity/version expectation
- **AND** records canonical/alias entry counts and temporary duplicate counts
- **AND** records the required resource/timing measurements in the change
  evidence.

#### Scenario: Allocation evidence is collected in isolation

- **WHEN** construction allocation counters are enabled
- **THEN** the measurement test runs alone under the existing snapshot
  experiment allocator
- **AND** reports allocation calls, allocated bytes and peak live-byte growth
  for each constructed variant
- **AND** the experiment does not declare a second global allocator.

### Requirement: Experiment does not alter production contracts

The comparison SHALL compile only in test code under the existing snapshot
experiment gate and SHALL NOT modify production snapshot fields, X1/cache
layout, public read interfaces, provider facts, SQL schema, resolver DTOs or
serialized output.

#### Scenario: Experiment feature is disabled

- **WHEN** `syntax-helper-search` is built and tested without snapshot
  experiment features
- **THEN** no `PrimaryAliasLookup`, experimental typed ID or benchmark corpus
  state is present in the production build
- **AND** existing snapshot lookup behavior and public types remain unchanged.
