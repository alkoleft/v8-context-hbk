## MODIFIED Requirements

### Requirement: One primary-first lookup implementation remains family-scoped

The experiment SHALL implement first-character-routed primary/alias lookup once
as a generic mechanism and SHALL instantiate independent state for `TypeId`,
`CallableId` and `PropertyId`. `TypeId` SHALL be the four-byte ordinal of its
unique normalized primary string in the type primary table. `CallableId` and
`PropertyId` SHALL be distinct eight-byte composites of `OwnerId` and their own
family-local interned canonical-name ordinal. A common string identity, entity
identity, member-name token family or identity registry SHALL NOT be introduced.
The mechanism SHALL remain private to the three experiment lanes.

The type candidate SHALL retain its normalized primary-name table plus an alias
index. Each scoped callable/property candidate SHALL retain a family-specific
normalized primary-name table, one primary vector of completed IDs ordered by
`(OwnerId, primary text through ID.name)`, and an alias index whose values are
completed IDs. These are the only permitted candidate identity/search
structures. The candidate SHALL NOT retain `StringId`, a duplicate key pool,
text-to-token reverse map, per-entity ID mirror or repeated owner field.

#### Scenario: Same mechanics serve all three families

- **WHEN** equivalent eligible primary, alias and missing raw-name queries are
  executed for type, callable and property collections
- **THEN** all three normalize the raw query and use the same routing mechanism
- **AND** each result contains only that collection's typed IDs
- **AND** resolving one collection does not mutate another collection.

#### Scenario: Type primary table ordinal is identity

- **WHEN** raw primary name `Массив` normalizes to a row in the lexically sorted
  unique type primary-name table
- **THEN** the candidate searches that table directly
- **AND** returns the found row index as `TypeId`
- **AND** performs no intermediate string-ID or second ID lookup.

#### Scenario: Global and owned members remain isolated

- **WHEN** the same callable or property name occurs globally and under one or
  more type owners
- **THEN** its family name ordinal is reused
- **AND** each completed ID contains the appropriate owner
- **AND** primary search reads owner scope from the completed ID without a
  repeated owner field
- **AND** it does not return another owner's candidate.

#### Scenario: Name present under another owner remains missing

- **WHEN** owner A declares a callable or property primary name and owner B does
  not declare that name
- **THEN** lookup under owner B returns missing even though the family name
  table contains the name ordinal for owner A.

### Requirement: Primary identity is distinct from alias search

The experiment SHALL assign typed identity only from unique normalized primary
names. A type primary row ordinal SHALL be its `TypeId`; callable/property name
ordinals SHALL be combined with owner and present in the scoped primary vector.
Alias keys SHALL reference completed identities and SHALL NOT allocate identity.

The routed candidate SHALL select aliases when the first normalized character
is ASCII-Latin and primaries otherwise. It SHALL search exactly the selected
structure and SHALL NOT fall back to a second search. Conclusions SHALL exclude
English-only primary names and non-ASCII aliases while reporting their counts.

#### Scenario: Russian primary name routes directly

- **WHEN** a non-ASCII-Latin raw primary name is queried
- **THEN** only the family primary structure is searched
- **AND** the canonical typed ID is returned.

#### Scenario: English alias routes directly

- **WHEN** an eligible ASCII-Latin raw alias is queried
- **THEN** only the family alias index is searched
- **AND** all matching completed typed IDs are returned in deterministic order.

#### Scenario: Missing alias does not add an index row

- **WHEN** an entity has no alias
- **THEN** it still has one primary identity/presence entry
- **AND** no alias entry is synthesized.

#### Scenario: English-only primary is outside the hypothesis

- **WHEN** a primary name begins with an ASCII-Latin character and has no
  non-ASCII primary counterpart in the evaluated convention
- **THEN** its query is excluded from correctness and timing sets
- **AND** the excluded count is reported rather than classified as a mismatch.

### Requirement: Old and new lookup comparison is controlled

The experiment SHALL compare the current raw-name lookup mechanics with the
typed-table candidate using identical canonical source entities, raw `&str`
query text, owner semantics, query order and sample count. Both variants SHALL
execute `normalize_lookup_key` inside the timed operation. The baseline SHALL
use the current snapshot representation and text comparisons; the candidate
SHALL contain no `StringId` after the source adapter boundary.

The source adapter SHALL resolve legacy `HbkNameView` handles immediately into
borrowed raw `&str` before projected source/query rows are formed. Candidate
tables, IDs, aliases, projected rows, queries and timed closures SHALL not
contain, construct or compare `StringId`. The experiment SHALL not use SQL,
copy production parsing/normalization behavior or retain a reverse string map.
A baseline-only projection SHALL retain the existing normalized snapshot
`StringId` keys and current lookup SHALL compare them through the snapshot
string table. Any transient borrowed text-to-existing-key preparation map SHALL
be dropped before construction and lookup measurements.

#### Scenario: Non-colliding eligible behavior is equivalent

- **WHEN** the fixed query corpus contains eligible primary hits, alias hits,
  misses and owner-isolation cases
- **THEN** current and candidate variants resolve the same canonical source
  entities
- **AND** exact differential assertions run before timing
- **AND** transient oracle maps are dropped before timing.

#### Scenario: Both routing branches include misses

- **WHEN** absent ASCII-Latin and non-ASCII-Latin raw names are queried
- **THEN** the candidate searches aliases and primaries respectively exactly
  once
- **AND** both variants return missing.

#### Scenario: Equal raw work is measured

- **WHEN** release-mode lookup measurements run
- **THEN** both lanes receive the identical raw query sequence and sample count
- **AND** both normalize inside the observed call
- **AND** pre-timing differential verification maps both result streams to
  canonical source entities and reports zero mismatches
- **AND** timed loops black-box/checksum native result IDs without charging
  either lane a benchmark-only identity remapping lookup
- **AND** the report labels the baseline as current raw lookup.

### Requirement: Resource evidence is reproducible

The experiment SHALL record frozen-corpus provenance, build profile, command,
sample count and environment together with per-family/per-variant construction
time, allocation observations, retained bytes and raw lookup timing for eligible
primary hits, alias hits, both miss routes and owner isolation. Candidate
retained bytes SHALL include every owned normalized string payload, type table,
family name table, scoped completed-ID primary vector and alias entry. At least
seven post-warm-up samples SHALL contribute to every median.

The report SHALL publish source/canonical rows, discarded duplicate primaries,
eligible/excluded query counts and semantic mismatch count. If any scoped family
has non-zero duplicate primaries, the conclusion SHALL state that production
cutover remains blocked until formation/extension composition establishes the
uniqueness invariant. Earlier prepared-`StringId` measurements SHALL remain
separately labelled historical evidence.

#### Scenario: Duplicate type primaries are temporarily discarded

- **WHEN** source formation yields repeated normalized type primary names
- **THEN** the experiment retains the stable first row before assigning
  lexically sorted `TypeId` ordinals
- **AND** reports the discarded count as temporary formation evidence.

#### Scenario: Frozen corpus measurement is recorded

- **WHEN** the ignored release experiment runs against frozen Russian
  8.3.27.1859 provider data
- **THEN** it validates corpus identity/version
- **AND** records coverage, exclusions, duplicates and semantic mismatches
- **AND** records same-run current and candidate construction allocations,
  complete retained bytes and raw-name median latency
- **AND** bounds conclusions to the measured bilingual corpus.

#### Scenario: Allocation evidence is collected in isolation

- **WHEN** construction allocation counters are enabled
- **THEN** the measurement test runs alone under the existing experiment
  allocator
- **AND** reports allocation calls, allocated bytes and peak live-byte growth
  for each compared construction
- **AND** introduces no second global allocator.
