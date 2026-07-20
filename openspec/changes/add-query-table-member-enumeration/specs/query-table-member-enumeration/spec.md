## ADDED Requirements

### Requirement: Query table member enumeration is provider-owned and scoped

`QueryTableSnapshotSource` SHALL enumerate fields and parameters for one
resolved `QueryTable` fact through the provider-owned immutable snapshot. It
MUST validate the active source and table identity before accessing the
owner-scoped snapshot index, and it MUST NOT require consumers to scan global
SDBL context facts or read `HbkFactReadHandle` directly.

#### Scenario: Fields of a resolved table are enumerated

- **WHEN** an active query-table source receives a resolved query-table fact
- **THEN** it returns exactly that table's field facts in provider deterministic
  order with the existing owner, name, type references, alias and provenance
  mapping

#### Scenario: Parameters of a resolved table are enumerated

- **WHEN** an active query-table source receives a resolved query-table fact
- **THEN** it returns exactly that table's parameter facts in provider
  deterministic order with the existing owner, name, type references, default
  value, alias and provenance mapping

#### Scenario: Table identity is absent or invalid

- **WHEN** the table fact is unknown, belongs to another source, has another
  domain or kind, or the query source is inactive
- **THEN** enumeration returns the existing normal `NotFound` response and
  reads no unrelated table members

#### Scenario: A known table has no members

- **WHEN** an active resolved table has no fields or no parameters
- **THEN** enumeration returns an `Ok` response with an empty fact collection

### Requirement: Point and enumeration query-table member semantics agree

The provider SHALL preserve point-query semantics while exposing enumeration.
For a matching owner/name, the existing field and parameter point queries MUST
return the same mapped fact as owner-scoped enumeration; a nonmatching name
MUST remain normal `NotFound`.

#### Scenario: A point result is contained in enumeration

- **WHEN** a named field or parameter belongs to an active resolved table
- **THEN** the point query and the corresponding owner-scoped enumeration
  expose equal fact identity and mapped evidence
