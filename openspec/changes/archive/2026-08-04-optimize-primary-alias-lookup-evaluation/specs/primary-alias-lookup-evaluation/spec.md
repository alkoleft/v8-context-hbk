## MODIFIED Requirements

### Requirement: One primary-first lookup implementation remains family-scoped

The experiment SHALL implement direct primary-name lookup followed by alias
fallback once as a generic mechanism and SHALL instantiate it with independent
state for `TypeId`, `CallableId` and `PropertyId`. `TypeId` SHALL be a distinct
four-byte interned canonical type-name token. `CallableId` and `PropertyId`
SHALL be distinct eight-byte composites of `OwnerId` and their own family-local
interned canonical-name token. A common entity identity, member-name token
family or identity registry SHALL NOT be introduced. The generic mechanism
SHALL remain private to the three evaluated experiment lanes and SHALL NOT
become a production/workspace abstraction in this change.

The retained candidate state SHALL consist only of separate direct primary and
alias lookup entries whose values are completed typed IDs. Family-local token
allocation SHALL be construction-only. The candidate SHALL NOT retain a
second key pool, primary-name map, primary-ID membership collection,
per-entity ID mirror or repeated owner field.

#### Scenario: Same mechanics serve all three families

- **WHEN** equivalent primary, alias and missing-name queries are executed for
  the type, callable and property collections
- **THEN** all three collections use the same direct
  primary-first/alias-fallback implementation
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
- **AND** the lookup reads that scope from the completed ID without retaining a
  second owner field
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

### Requirement: Old and new lookup comparison is controlled

The experiment SHALL compare a deliberate reference for the current merged
primary-plus-alias behavior with the generic primary-first/alias-fallback
candidate using identical canonical rows, snapshot-owned normalized string
IDs, owner semantics and deterministic query order. The reference SHALL use
independent dense four-byte family ordinals; the candidate SHALL use the
specified interned/composite IDs. Both variants SHALL reuse the snapshot-owned
lookup record and matching-range behavior. The experiment SHALL not use SQL
lookup, copy those lookup primitives, or charge a result-ID remapping to either
timed variant. Because the measured operation receives prepared numeric
`StringId` keys, the dense lane SHALL be described as a reference over prepared
snapshot keys and SHALL NOT be described as current public production lookup
latency.

#### Scenario: Non-colliding behavior is equivalent

- **WHEN** the fixed query corpus contains primary hits, alias-only hits and
  misses without a primary/alias collision
- **THEN** the merged reference and candidate resolve the same canonical
  entity candidates
- **AND** an exact differential assertion verifies the equivalence
- **AND** transient differential maps are dropped before timed measurement.

#### Scenario: Lookup mechanics are measured independently

- **WHEN** release-mode measurements are executed
- **THEN** common snapshot loading, normalization and snapshot-string-ID
  preparation are outside both compared lookup timings or are reported as a
  shared phase
- **AND** no direct construction of a `StringId` or retained reverse-string map
  substitutes another string-table owner
- **AND** the old and new lookup loops use the same query sequence and sample
  count
- **AND** a checksum proves that every result was consumed.

### Requirement: Resource evidence is reproducible

The experiment SHALL record frozen-corpus provenance, build profile, command,
sample count and environment together with per-family/per-variant construction
time, allocation observations, retained direct primary/alias bytes and lookup
timing for primary hits, alias fallback hits, misses, primary/alias collisions
and owner isolation. Candidate retained bytes SHALL include every direct lookup
entry and SHALL exclude only construction-token and differential-oracle state
that is destroyed before measurement. At least seven post-warm-up samples
SHALL contribute to every reported median.

Construction evidence SHALL label the complete dense-ordinal baseline and the
complete direct composite-interned candidate costs; it SHALL NOT attribute
their difference solely to primary/alias index splitting. Lookup timing SHALL
be reported separately after construction.

The durable report SHALL compare the optimized candidate with the same-run
dense merged baseline and with a control run of the replaced over-modelled
candidate on the same frozen corpus. Cross-run comparisons SHALL be labelled,
and baseline drift SHALL be reported. If any scoped family reports a non-zero
duplicate-primary count, the conclusion SHALL state that a production cutover
is blocked until the owning formation/extension composition establishes the
uniqueness invariant.

#### Scenario: Frozen corpus measurement is recorded

- **WHEN** the ignored real-corpus experiment runs against the frozen 8.3.27
  provider index in release mode
- **THEN** it validates the corpus identity/version expectation
- **AND** records canonical/alias entry counts and temporary duplicate counts
- **AND** records the required optimized resource/timing measurements and
  before/after comparison in the change evidence.

#### Scenario: Allocation evidence is collected in isolation

- **WHEN** construction allocation counters are enabled
- **THEN** the measurement test runs alone under the existing snapshot
  experiment allocator
- **AND** reports allocation calls, allocated bytes and peak live-byte growth
  for each constructed variant
- **AND** the experiment does not declare a second global allocator.
