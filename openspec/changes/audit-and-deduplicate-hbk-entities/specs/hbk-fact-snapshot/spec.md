## ADDED Requirements

### Requirement: Each HBK semantic entity has one canonical identity owner

The HBK snapshot SHALL assign one canonical provider-owned identity to each source-backed semantic entity within a generation. Multiple indexes, role projections or storage views MAY address that entity, but they MUST NOT create an independent semantic identity or repeat equivalent entity payload without a distinct documented invariant.

#### Scenario: One method appears in member and callable source rows

- **WHEN** snapshot materialization encounters member and callable rows with the same provider source key that describe one method or event
- **THEN** the retained snapshot exposes one canonical semantic identity for that entity
- **AND** member, owner, global-context and callable lookup paths reference that identity or a documented non-owning projection
- **AND** no consumer must pair two independent IDs to identify the method or event

#### Scenario: Similar records are distinct entities

- **WHEN** two records share a name or overlapping payload but have different provider source identity or a proven distinct semantic invariant
- **THEN** the audit retains them as distinct entities
- **AND** it records the evidence that prevents their accidental merge

#### Scenario: Secondary index repeats a locator

- **WHEN** an ID is repeated only as the value of owner, name, alias, kind, relation or CSR indexes
- **THEN** the repetition is classified as a legitimate index reference
- **AND** the index is not removed merely because it references the same canonical entity from another lookup path

### Requirement: Duplicate removal preserves snapshot behavior and provenance

Removing a duplicate identity or payload owner SHALL preserve every retained provider lookup result, ordering rule, owner relation, source provenance record, availability fact and explicit ambiguity outcome unless a separately accepted requirement changes that behavior. Owned H0 and mapped X1 snapshots SHALL remain behaviorally equivalent.

#### Scenario: A duplicate family is migrated

- **WHEN** one duplicate record family is replaced by references to its canonical owner
- **THEN** focused differential fixtures prove equivalent ID, name, owner, kind, relation, provenance and semantic-role outcomes
- **AND** H0/X1 parity covers the migrated lookup paths
- **AND** any serialized layout change increments and validates the owning format version

### Requirement: Deduplication is evidence-driven and resource-measured

Before deleting or merging a representation, the change SHALL inventory its source rows, retained records, indexes, conversions, public views and real consumers. Each implementation slice SHALL record before/after build time, retained memory, allocation volume, mapped artifact size and representative lookup latency.

#### Scenario: A candidate duplicate has no proven single owner

- **WHEN** the inventory cannot prove that two representations describe the same semantic entity or cannot select a canonical owner without losing behavior
- **THEN** implementation stops for that candidate
- **AND** the candidate remains explicitly unresolved rather than being merged by name, source order or field similarity

#### Scenario: A duplicate is removed

- **WHEN** inventory and fixtures prove one canonical owner and the implementation removes the parallel owner
- **THEN** a structural-absence regression detects recreation of the prohibited identity or payload path
- **AND** the recorded measurements make any resource regression explicit
