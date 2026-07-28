## ADDED Requirements

### Requirement: HbkBslContextCatalog is snapshot-owned

HBK SHALL expose one immutable `HbkBslContextCatalog` over the existing
snapshot/read-handle arenas. Public `HbkBslContextCatalog` methods SHALL return
arena-backed typed IDs, borrowed records and borrowed iterators with stable
lifetimes tied to the snapshot/read handle. The catalog SHALL preserve
deterministic provider order, source identity, source locale and provenance,
and SHALL be shareable across workers as `Send + Sync`. Existing snapshot
facts SHALL be sufficient for the catalog; implementation MUST NOT require a
new HBK fact family, storage arena, index, DTO mirror or SQLite fallback.

#### Scenario: Catalog returns borrowed arena-backed records

- **WHEN** a caller opens `HbkBslContextCatalog` for an HBK snapshot
- **THEN** point methods return optional borrowed records with arena-backed
  typed IDs
- **AND** enumeration methods return borrowed iterators over arena-backed typed
  IDs and records
- **AND** it creates no second arena, duplicate storage, duplicate index,
  flattened `ContextFact` collection, `Resolved*` materialization or
  public `HbkFactRef` surface for the catalog

#### Scenario: Catalog can be shared by analyzer workers

- **WHEN** multiple workers read the same borrowed BSL context catalog
- **THEN** the catalog can be shared immutably as `Send + Sync`
- **AND** reads preserve deterministic provider order, source locale and
  provenance without per-worker storage copies

### Requirement: HbkBslContextCatalog lifetime and read-handle API stay borrowed

`HbkBslContextCatalog` SHALL own the shared `Arc` snapshot handle needed to keep
snapshot storage alive, while returned records and iterators SHALL borrow from
the snapshot/read handle. Relevant read-handle APIs for catalog-covered BSL
facts SHALL NOT force consumers to collect into owned vectors or materialize
generic resolver DTOs before using catalog records.

#### Scenario: Catalog owns the snapshot handle and lends records

- **WHEN** a caller obtains BSL records through `HbkBslContextCatalog`
- **THEN** the catalog keeps the underlying snapshot alive through its owned
  `Arc`
- **AND** returned records and iterators borrow from the snapshot/read handle
  instead of owning copied DTO records

#### Scenario: Read-handle APIs do not force collection

- **WHEN** `HbkBslContextCatalog` enumerates catalog-covered BSL facts
- **THEN** the relevant read-handle API supports borrowed iteration
- **AND** callers are not forced to collect records or construct `ContextFact`,
  `Resolved*` or other owned DTOs before observing the catalog contract

### Requirement: HbkBslContextCatalog preserves point and enumeration parity

`HbkBslContextCatalog` SHALL expose point and enumeration access for global
properties, global methods, complete generated-self type lookup,
generated-self owner members, generated-self owner callables, metadata module
context members as the catalog's global properties/methods plus
`ModuleContextKind`-scoped module events. For the same source, owner,
canonical BSL name and member kind, point lookup and enumeration SHALL expose
the same HBK-owned identity, borrowed record, source identity, source locale
and provenance.

#### Scenario: Global point result appears in global enumeration

- **WHEN** a global property or global method is visible in the catalog
- **THEN** a matching point lookup and the corresponding enumeration expose the
  same arena-backed ID, borrowed record, member kind, source identity, source
  locale and provenance
- **AND** HBK does not materialize a generic `ContextFact` or `Resolved*`
  answer to prove parity

#### Scenario: Generated-self type lookup is complete

- **WHEN** generated-self type templates are visible in the HBK snapshot
- **THEN** `HbkBslContextCatalog` exposes complete generated-self type lookup
  for the catalog-covered generated-self selectors
- **AND** the lookup uses existing snapshot facts and does not require an
  analyzer-side generated-self mapping table

#### Scenario: Generated-self owner members and callables retain owner scope

- **WHEN** generated-self owner members or generated-self owner callables are
  visible in the catalog
- **THEN** point lookup and enumeration are scoped by the arena-backed
  generated owner ID and expose the same borrowed member or callable record
- **AND** the caller does not reconstruct owner scope from analyzer mappings,
  spelling heuristics or SQLite fallback data

#### Scenario: Metadata module context and events retain selector scope

- **WHEN** metadata module context members or metadata module events are visible
  for a `ModuleContextKind`
- **THEN** global properties/methods and `ModuleContextKind`-scoped module
  events compose the same module-context membership as the generic adapter
- **AND** point lookup and enumeration expose the same arena-backed ID and
  borrowed record
- **AND** source identity, source locale and typed record identity remain
  available as provenance inputs without a catalog-specific provenance DTO

### Requirement: BSL availability and module-role translation ownership stay upstream

HBK SHALL own typed BSL availability methods used by
`HbkBslContextCatalog` for platform types, members, callables and globals.
These methods SHALL return typed availability answers directly from existing
snapshot facts without exposing public `HbkFactRef`. Raw
`metadata.module-role` selector translation remains single-owned by
`context-resolver-core`; `HbkBslContextCatalog` SHALL consume the translated
`ModuleContextKind` and MUST NOT duplicate raw selector mapping. Analyzer
consumers MUST NOT maintain a parallel availability model, selector mapping
table, private provider read, SQLite fallback path, DTO or enum mirror, or
generic adapter behavior owner for these facts.

#### Scenario: Typed availability methods come from HBK

- **WHEN** the catalog evaluates BSL availability for a platform type, member,
  callable or global
- **THEN** it returns the HBK-owned typed availability answer from the existing
  snapshot
- **AND** the API does not expose `ContextFact`, `Resolved*` or public
  `HbkFactRef`

#### Scenario: Raw metadata.module-role translation is not duplicated

- **WHEN** the catalog evaluates metadata module context members or events
- **THEN** callers pass the already translated `ModuleContextKind` owned by
  `context-resolver-core`
- **AND** `HbkBslContextCatalog` does not translate raw `metadata.module-role`
  selectors or carry a duplicate selector mapping table
- **AND** analyzer-side mappings, compatibility shims, SQLite fallback queries
  and duplicated generic adapter logic are not part of the contract

#### Scenario: Generic resolver delegates to the BSL catalog for shared behavior

- **WHEN** a generic adapter or `ContextResolver` path needs BSL context facts
  covered by the borrowed catalog
- **THEN** it delegates to the same `HbkBslContextCatalog` behavior and
  preserves parity with the borrowed API
- **AND** it does not maintain a second behavior owner, duplicate storage,
  duplicate index or alternate selector access path
