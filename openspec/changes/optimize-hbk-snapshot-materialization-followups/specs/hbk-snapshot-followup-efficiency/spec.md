## ADDED Requirements

### Requirement: Owner-local snapshot follow-up optimization
The provider SHALL evaluate each remaining snapshot-materialization hypothesis
at its existing owner and SHALL implement it only when its evidence, behavior
oracle and acceptance metric are recorded independently. The change SHALL not
alter provider facts, SQLite schema/indexes, cache layout/version, serialized
snapshot fields or downstream resolver/analyzer contracts without a separate
accepted decision.

#### Scenario: Candidate has independent owner evidence
- **WHEN** a candidate has a measured allocation/lifetime source and a
  single existing provider owner
- **THEN** its task records the owner, semantic oracle, resource metric and
  reintroduction guard before implementation

#### Scenario: Candidate lacks capacity or lifecycle evidence
- **WHEN** a capacity or cache-startup candidate lacks a measured owner-local
  source or accepted lifecycle contract
- **THEN** the provider SHALL record it as deferred or rejected without
  retaining speculative production scaffolding

### Requirement: Selected signature extraction preserves snapshot behavior
The snapshot materializer SHALL select an already-owned document signature line
through a borrowed `&str` and pass it directly to the existing snapshot
builder. It SHALL preserve the existing non-empty-line predicate and ordinal
selection without materializing every signature line in a temporary
`Vec<String>` or cloning the selected line before interning.

#### Scenario: Multiple signature lines
- **WHEN** a document signature contains multiple non-empty lines and empty
  separators
- **THEN** the materialized callable signatures retain the same non-empty lines
  in the same order without an all-lines temporary vector or selected-line
  clone in the materializer path

### Requirement: Type-reference contexts remain semantically distinct
The provider SHALL keep document, document-return, signature-return and
parameter type-reference groups distinct even when their names match. An
optimization SHALL not merge or deduplicate these groups based only on type
text.

#### Scenario: Equal type names occur in distinct contexts
- **WHEN** a document, callable return, signature return and parameter use the
  same textual type name
- **THEN** each context retains its separately addressable type-reference fact
  and original group order
