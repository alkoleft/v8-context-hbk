# hbk-fact-snapshot-cache Specification

## Purpose
TBD - created by archiving change stabilize-hbk-fact-snapshot-cache. Update Purpose after archive.
## Requirements
### Requirement: Provider-Owned Snapshot Cache

`v8-context-hbk` SHALL treat a persisted `HbkFactSnapshot` cache as provider-owned derived state over
the canonical SQLite provider index.

#### Scenario: SQLite Provider Index Remains Canonical

- **GIVEN** an HBK provider SQLite index and a persisted snapshot cache
- **WHEN** the snapshot is needed for resolver/analyzer lookup
- **THEN** the provider layer MAY load the snapshot from the cache
- **AND** it SHALL be able to rebuild the cache from the SQLite provider index
- **AND** downstream resolver adapters SHALL NOT depend on cache files or binary layout details.

#### Scenario: Cache Metadata Controls Invalidation

- **GIVEN** a persisted snapshot cache
- **WHEN** the provider layer validates it before loading
- **THEN** validation SHALL check cache format version, provider SQLite schema version, source index
  identity, platform version/locale when available, snapshot layout version or flags and an integrity
  guard
- **AND** mismatch, unsupported version or integrity failure SHALL invalidate the cache.

#### Scenario: Invalid Cache Rebuilds From SQLite

- **GIVEN** a missing, stale, unsupported or corrupted cache
- **WHEN** the provider layer needs an `HbkFactSnapshot`
- **THEN** it SHALL rebuild from the canonical SQLite provider index
- **AND** this rebuild SHALL be provider cache invalidation behavior, not a hidden SQL/SearchIndex
  fallback inside snapshot-backed resolver adapters.

### Requirement: Cache Format Decision Is Measured

The first accepted cache format SHALL be selected from post-T169 measurements rather than from a
speculative storage preference.

#### Scenario: Measurements Compare SQLite And Cache Paths

- **WHEN** the cache format decision is accepted
- **THEN** the acceptance baseline SHALL record SQLite materialization time, cache validation/load
  time, cache write time when applicable, process peak RSS, capacity-based heap bytes, logical
  payload bytes, cache file size and representative read-handle lookup timings.

#### Scenario: Dependency Requires Measured Justification

- **GIVEN** the no-dependency binary DTO path is already measured
- **WHEN** a new serialization, zero-copy or memory-mapping dependency is proposed
- **THEN** the task SHALL record the measured bottleneck it addresses
- **AND** an ADR or implementation spec update SHALL own the dependency decision before adoption.

### Requirement: Resolver Backend Split Remains Stable

Snapshot cache work SHALL preserve the T171 resolver backend split.

#### Scenario: Cache-Loaded Snapshot Uses Existing Snapshot Sources

- **GIVEN** a snapshot loaded from a provider-owned cache
- **WHEN** resolver/analyzer lookup is composed
- **THEN** callers SHALL use the existing snapshot-backed source boundary such as
  `PlatformSnapshotSource` and `QueryTableSnapshotSource`
- **AND** the cache path SHALL NOT require resolver adapters to know the cache file format.

#### Scenario: Non-Query BSL Language Snapshot Source Is Out Of Scope

- **GIVEN** T171 completed only platform and query-table snapshot-backed sources
- **WHEN** T170 stabilizes the cache path
- **THEN** it SHALL NOT add a non-query-table `LanguageSnapshotSource`
- **AND** any later migration of non-query-table `BslLanguage` facts SHALL be tracked as a separate
  task/change with its own identity and no-fallback coverage.
