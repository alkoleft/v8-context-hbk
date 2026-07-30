## ADDED Requirements

### Requirement: Worker-Safe HBK Fact Snapshot

`v8-context-hbk` SHALL provide a provider-owned immutable HBK fact snapshot that is safe to share across Rust worker threads.

#### Scenario: Snapshot Is Send And Sync

- **WHEN** a snapshot is constructed from a provider index
- **THEN** the snapshot type SHALL satisfy `Send + Sync`
- **AND** callers SHALL be able to share it as `Arc<_>` across worker threads
- **AND** worker-local handles SHALL not require shared mutable resolver state.

#### Scenario: Concurrent Reads Are Deterministic

- **GIVEN** one shared snapshot
- **WHEN** multiple threads perform the same representative lookups
- **THEN** each thread SHALL receive equivalent results
- **AND** no shared SQLite connection SHALL be used during those reads.

### Requirement: Explicit Resolver Backends

`context-resolver-search` SHALL expose snapshot-backed resolver sources separately from SQL/SearchIndex-backed resolver sources.

#### Scenario: Snapshot Backend Is Selected Explicitly

- **GIVEN** a provider-owned `HbkFactSnapshot` or public snapshot read handle
- **WHEN** analyzer composition needs worker-safe documented HBK fact lookup
- **THEN** callers SHALL compose snapshot-backed sources such as `PlatformSnapshotSource` or `QueryTableSnapshotSource`
- **AND** a broader `LanguageSnapshotSource` SHALL be added or used only when the migrated slice covers non-query-table language facts
- **AND** those sources SHALL implement `context_resolver_core::ContextSource`.

#### Scenario: SQL SearchIndex Backend Remains Explicit

- **GIVEN** an existing provider SQLite/SearchIndex artifact
- **WHEN** CLI, debug, index inspection or sequential local resolver scenarios need the existing search-backed behavior
- **THEN** callers SHALL compose `PlatformSearchSource` or `LanguageSearchSource`
- **AND** those source names SHALL remain SQL/SearchIndex-backed rather than silently switching to snapshot behavior
- **AND** downstream analyzer hot paths SHALL NOT use this backend as their worker-safe path.

#### Scenario: Snapshot Backend Has No Hidden SQL Fallback

- **GIVEN** a snapshot-backed resolver source
- **WHEN** a migrated hot-path lookup is executed
- **THEN** the lookup SHALL use `HbkFactReadHandle` or equivalent provider-owned snapshot read APIs
- **AND** it SHALL NOT open or read SQLite internally
- **AND** it SHALL NOT fall back to SQL/SearchIndex-backed source methods inside the resolver adapter
- **AND** missing snapshot coverage SHALL return the documented unsupported or empty result instead of switching storage backends.

#### Scenario: Adapter Boundary Is Worker Safe

- **GIVEN** a snapshot-backed source composed into a resolver boundary that may be passed to downstream workers
- **WHEN** the boundary is compiled or tested
- **THEN** the source/resolver composition SHALL satisfy `Send + Sync`
- **OR** the API SHALL expose and verify an explicit scoped-worker borrow contract
- **AND** checking only `HbkFactSnapshot: Send + Sync` SHALL NOT satisfy this scenario
- **AND** broad `Arc<Mutex<_>>` / `Arc<RwLock<_>>` wrappers around resolver/search state, SQLite connections or mutable adapter internals SHALL NOT satisfy this scenario.

#### Scenario: Identity Is Preserved For Migrated Snapshot Sources

- **GIVEN** facts with the same display name in source families migrated to snapshot-backed sources
- **WHEN** callers resolve through the snapshot-backed resolver composition
- **THEN** the result SHALL preserve source and domain identity
- **AND** ambiguity SHALL NOT be hidden by source ordering.

#### Scenario: Non-Migrated BSL-Language Facts Do Not Fall Back To SQL

- **GIVEN** T171 does not migrate non-query-table BSL-language facts into `LanguageSnapshotSource`
- **WHEN** a snapshot-backed resolver lookup requests such a fact
- **THEN** the lookup SHALL return the documented unsupported or empty result
- **AND** it SHALL NOT consult `LanguageSearchSource`, `PlatformSearchSource` or raw SQL/SearchIndex.

#### Scenario: Query Table Snapshot Lookup Uses Snapshot Keys

- **GIVEN** query-table facts are migrated to a snapshot-backed source
- **WHEN** callers resolve query tables by name, syntax or identifier
- **THEN** the lookup SHALL use provider-owned snapshot keys
- **AND** it SHALL NOT rely on SQL table lookup.

#### Scenario: Enum Fact Coverage Is Explicit

- **GIVEN** enum and enum-value facts in the provider-owned snapshot
- **WHEN** exact-id or relation lookup is migrated to a snapshot-backed resolver source
- **THEN** enum and enum-value facts SHALL participate in the migrated lookup
- **OR** the migrated slice SHALL document and test that those facts are excluded and return the documented unsupported or empty result.

### Requirement: Provider-Owned Fact Coverage

The snapshot SHALL model documented HBK provider facts, not analyzer/project facts.

#### Scenario: Platform Facts Are Nested By Owner

- **WHEN** platform type facts are materialized
- **THEN** platform type nodes SHALL own references to their constructors, members, callables and events
- **AND** callable signatures, parameters, return types, availability and provenance SHALL remain available from provider facts.

#### Scenario: Global And Module Contexts Are Provider Facts

- **WHEN** documented global and module context facts are materialized
- **THEN** global methods/properties and module context events SHALL be accessible without analyzer-owned fallback readers
- **AND** module context facts SHALL remain documented HBK facts, not effective project module context.

#### Scenario: Query Tables Own Fields And Parameters

- **WHEN** query table facts are materialized
- **THEN** query table nodes SHALL own references to documented fields and parameters
- **AND** field/parameter type references and provenance SHALL remain available.

#### Scenario: Language Facts Are Domain-Separated

- **WHEN** BSL or query language facts are materialized
- **THEN** BSL language facts SHALL remain `BslLanguage`
- **AND** SDBL/query facts SHALL remain `QueryLanguage`
- **AND** same-name platform, BSL and query facts SHALL not be merged by display name.

### Requirement: SQLite Bulk Materialization

The first snapshot materializer SHALL build from existing provider SQLite indexes through provider-owned bulk reads.

#### Scenario: Build Phase Uses SQLite Without Worker Sharing

- **WHEN** the snapshot is built from SQLite
- **THEN** SQLite connections MAY be opened only during build/materialization
- **AND** the resulting snapshot SHALL not store or share a `rusqlite::Connection`.

#### Scenario: Materialization Avoids Public N+1 Lookup APIs

- **WHEN** the snapshot is built from SQLite
- **THEN** construction SHALL use coarse table-family reads over provider-owned schema
- **AND** construction SHALL not use lookup-oriented loops such as per-type member lookup or per-document hydration as the primary path.

### Requirement: Compact Owned Read Model

The snapshot storage SHALL use owned nested nodes and derived indexes.

#### Scenario: Snapshot Excludes Non-Contract Data

- **WHEN** the snapshot is materialized from a provider SQLite index
- **THEN** it SHALL select only columns required by documented fact lookup contracts
- **AND** it SHALL exclude search/export/index-maintenance payloads such as FTS rows, preview text, raw descriptions, raw HBK paths, raw TOC paths, raw HTML paths, relation weights and parser diagnostics unless a specific snapshot lookup requires them.

#### Scenario: Secondary Indexes Are Not Sources Of Truth

- **WHEN** exact id, name, owner or context indexes are built
- **THEN** indexes SHALL reference owned snapshot nodes
- **AND** indexes SHALL not duplicate full facts as independent mutable state.

#### Scenario: Resolver DTOs Are Projection DTOs

- **WHEN** existing `context-resolver-core` DTOs are returned
- **THEN** they SHALL be projections from snapshot nodes
- **AND** they SHALL not define the physical snapshot storage model.

### Requirement: Measurements Are Recorded

Snapshot work SHALL record measurements before accepting SQLite-first materialization as the implementation direction.

#### Scenario: Snapshot Measurements Exist

- **WHEN** the first snapshot materialization slice is completed
- **THEN** design/tasks or acceptance baseline SHALL record build time, RSS delta or peak RSS, estimated heap when practical, node counts, lookup surface coverage and representative lookup latency.

#### Scenario: Materialization Source Is Compared

- **WHEN** SQLite materialization is evaluated
- **THEN** it SHALL be compared with the current SQLite `SearchIndex` lookup path and/or HBK extraction/index build path
- **AND** conclusions SHALL be recorded before broad implementation proceeds.
