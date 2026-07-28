## ADDED Requirements

### Requirement: HbkSdblQueryCatalog is snapshot-owned

HBK SHALL expose one immutable `HbkSdblQueryCatalog` over the existing
snapshot/read-handle arenas. Public `HbkSdblQueryCatalog` methods SHALL return
arena-backed typed IDs, borrowed table/field/parameter records and borrowed
iterators with stable lifetimes tied to the snapshot/read handle. The catalog
SHALL expose query identifiers, syntax, type references, deterministic
provider order, source identity, source locale and provenance, and SHALL be
shareable across workers as `Send + Sync`. Existing snapshot facts SHALL be
sufficient for the catalog; implementation MUST NOT require a new HBK fact
family, storage arena, index, DTO mirror or SQLite fallback.

#### Scenario: Catalog returns borrowed arena-backed records

- **WHEN** a caller opens `HbkSdblQueryCatalog` for an HBK snapshot
- **THEN** point methods return optional borrowed records with arena-backed
  typed IDs
- **AND** enumeration methods return borrowed iterators over arena-backed typed
  IDs and records
- **AND** it creates no flattened `Vec<ContextFact>`, second arena, duplicate
  index, DTO or enum mirror, SQLite fallback path or analyzer selector mapping

#### Scenario: Catalog can be shared by analyzer workers

- **WHEN** multiple workers read the same borrowed SDBL query catalog
- **THEN** the catalog can be shared immutably as `Send + Sync`
- **AND** reads preserve deterministic provider order without per-worker
  materialization of SDBL context facts

### Requirement: HbkSdblQueryCatalog lifetime and read-handle API stay borrowed

`HbkSdblQueryCatalog` SHALL own the shared `Arc` snapshot handle needed to keep
snapshot storage alive, while returned records and iterators SHALL borrow from
the snapshot/read handle. Relevant read-handle APIs for catalog-covered SDBL
facts SHALL NOT force consumers to collect into owned vectors or materialize
generic resolver DTOs before using catalog records.

#### Scenario: Catalog owns the snapshot handle and lends records

- **WHEN** a caller obtains SDBL records through `HbkSdblQueryCatalog`
- **THEN** the catalog keeps the underlying snapshot alive through its owned
  `Arc`
- **AND** returned records and iterators borrow from the snapshot/read handle
  instead of owning copied DTO records

#### Scenario: Read-handle APIs do not force collection

- **WHEN** `HbkSdblQueryCatalog` enumerates catalog-covered SDBL facts
- **THEN** the relevant read-handle API supports borrowed iteration
- **AND** callers are not forced to collect records or construct `ContextFact`,
  `Resolved*` or other owned DTOs before observing the catalog contract

### Requirement: HbkSdblQueryCatalog preserves table and member parity

`HbkSdblQueryCatalog` SHALL expose table point lookup and table
enumeration, and SHALL expose owner-scoped fields and parameters for resolved
tables. For the same source, table identity, member name and member kind, point
lookup and enumeration SHALL expose the same HBK-owned identity, borrowed
  record, type references and provenance inputs.

#### Scenario: Table point result appears in table enumeration

- **WHEN** a query table is visible in the catalog
- **THEN** a matching table point lookup and table enumeration expose the same
  arena-backed ID, borrowed record, source identity, source locale, query
  identifier, syntax, type references and provenance inputs
- **AND** HBK does not materialize a flattened `Vec<ContextFact>` to prove
  parity

#### Scenario: Owner fields and parameters retain table scope

- **WHEN** fields or parameters belong to a resolved query table
- **THEN** owner-scoped point lookup and enumeration expose the same borrowed
  field or parameter record with its arena-backed owner table ID, name,
  aliases, type references, defaults where present, source locale and
  source identity, source locale and typed record identity as provenance inputs
- **AND** the caller does not reconstruct owner scope from analyzer mappings,
  SQLite fallback data or global context scans

### Requirement: SDBL query-source selectors stay upstream

HBK SHALL own query-source selector mapping for metadata-backed SDBL sources.
For source records with `source_locale=ru`, the borrowed SDBL query catalog
SHALL return one of the six exact opaque `metadata.sdbl.query-source.*`
selector values consumed directly by the metadata provider boundary, or normal
absence (`None`) for every other identifier or locale. The catalog SHALL NOT
introduce an analyzer-side selector mapping, enum or wrapper mirror, private
provider read or SQLite fallback path.

#### Scenario: Russian HBK query-source identifiers map to exact selectors

- **WHEN** a borrowed SDBL query table record has `source_locale=ru` and one of
  these query identifiers: `Справочник`, `Документ`, `РегистрСведений`,
  `РегистрНакопления`, `РегистрБухгалтерии` or `РегистрРасчета`
- **THEN** HBK returns the exact opaque selector value
  `metadata.sdbl.query-source.catalog`,
  `metadata.sdbl.query-source.document`,
  `metadata.sdbl.query-source.information-register`,
  `metadata.sdbl.query-source.accumulation-register`,
  `metadata.sdbl.query-source.accounting-register` or
  `metadata.sdbl.query-source.calculation-register` respectively
- **AND** analyzer consumers do not map source selectors to query-domain roles
- **AND** the metadata provider boundary consumes the returned selector value
  directly

#### Scenario: Unknown query-source selector is normal absence

- **WHEN** a query source identifier is outside the six accepted Russian HBK
  identifiers or the source locale is not `ru`
- **THEN** HBK returns normal absence (`None`)
- **AND** the analyzer semantic layer remains responsible for forming any typed
  unknown reason
- **AND** it does not infer a selector from spelling, analyzer context,
  SQLite fallback data or generic `ContextFact` shape

#### Scenario: Generic resolver delegates to the SDBL catalog for shared behavior

- **WHEN** a generic adapter or `ContextResolver` path needs SDBL query facts
  covered by the borrowed catalog
- **THEN** it delegates to the same `HbkSdblQueryCatalog` behavior and
  preserves parity with the borrowed API
- **AND** it does not maintain a second behavior owner, duplicate storage,
  duplicate index or alternate query-source selector path
