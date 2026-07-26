## ADDED Requirements

### Requirement: Bounded Type-Reference Materialization

The provider MUST materialize an `HbkFactSnapshot` from SQLite without
retaining a complete owned collection of type-reference source rows after each
row is projected into the existing snapshot type-reference groups.

The provider MUST preserve deterministic per-group order, error propagation and
existing read-handle type/member/callable/query results. The published snapshot
remains the sole provider-owned fact representation.

#### Scenario: Ordered source rows populate all snapshot groups

- **WHEN** the source index contains document, document-return,
  signature-return and parameter type references
- **THEN** the snapshot exposes the same facts and type-reference values as
  before materialization changed
- **AND** each group retains its SQL-defined order

#### Scenario: Invalid source rows remain typed errors before filtering

- **WHEN** SQLite returns a type-reference row whose encoded status or target
  contradicts existing model rules, including a row no snapshot group consumes
- **THEN** construction returns the existing typed search error
- **AND** no partial snapshot is published

#### Scenario: Existing snapshot caches remain readable

- **WHEN** a binary cache was produced by the unchanged snapshot layout
- **THEN** its existing metadata and payload read through the cache path
- **AND** no cache layout version change is required
