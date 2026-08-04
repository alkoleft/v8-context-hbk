## ADDED Requirements

### Requirement: HBK maintains one explicit ubiquitous language

The HBK domain model SHALL define each provider concept once with its meaning, owner, lifecycle and allowed relations. Every current fact family SHALL be classified as a semantic entity, embedded value object, source evidence/relation, lookup projection, generation-local locator or storage detail.

#### Scenario: A current snapshot type is classified

- **WHEN** a maintainer evaluates an H0 record, X1 record, borrowed view or public locator
- **THEN** the domain model identifies the domain concept it represents or marks it as infrastructure-only
- **AND** names its single semantic/storage owner
- **AND** records any mismatch between the concept and the current Rust type name or record family

#### Scenario: A concept has no proven independent identity

- **WHEN** signatures, parameters, names, aliases, type references, availability entries, template bindings or similar contained data have no independent addressability contract
- **THEN** the domain model classifies them as values or evidence owned by another entity
- **AND** it does not invent a standalone entity ID for implementation convenience

### Requirement: Identity roles remain distinct

The domain model SHALL distinguish provider semantic identity, dataset/generation identity, dense record locator, dictionary/string address, lookup key and durable external reference. Equality, uniqueness, lifetime and resolution rules SHALL be stated independently for every identity role used by HBK.

#### Scenario: A dense HBK locator is compared across generations

- **WHEN** equal `Hbk*Id` ordinal values originate from different snapshot generations
- **THEN** the model treats them as unrelated generation-local locators
- **AND** it does not claim cross-session semantic equality from the ordinal alone

#### Scenario: A StringId points to a source key

- **WHEN** a record stores its provider source key through `StringId`
- **THEN** the model classifies `StringId` as a generation-local dictionary address
- **AND** distinguishes the addressed source key from the storage address
- **AND** does not promote that address to a durable or cross-provider identity

#### Scenario: A durable HBK reference is requested

- **WHEN** a real external or persisted consumer requires reopening the same HBK entity across sessions or datasets
- **THEN** a separate accepted contract defines dataset scope, canonical key, compatibility, migration and failure behavior
- **AND** the domain model does not silently reuse a generation-local locator or string-table address

### Requirement: Semantic entities are independent of storage projections

One semantic entity MAY participate in several lookup roles, source facets and serialized projections, but the domain model SHALL identify one canonical semantic owner. Multiple record families are distinct entities only when they preserve different identity or invariant evidence.

#### Scenario: One method is represented as member and callable records

- **WHEN** member and callable records have the same provider source key and describe the same method/event declaration
- **THEN** the model determines whether they are one entity with multiple projections or distinct entities with documented invariants
- **AND** an unresolved classification blocks deduplication rather than being decided from record layout alone

#### Scenario: A secondary index references an entity

- **WHEN** primary-name, alias, exact-ID, owner, kind, relation or CSR storage repeats an entity locator
- **THEN** the model classifies that storage as a lookup projection
- **AND** it does not treat the repeated reference as another semantic entity

### Requirement: HBK domains and relations are explicit

The domain model SHALL describe the boundaries and permitted relations among platform API facts, BSL-language facts, query-language facts and documentation/source provenance. Platform types, callables, properties, globals, enums/values, language constructs and query tables/fields/parameters SHALL retain provider-specific distinctions unless an accepted semantic rule proves equivalence.

#### Scenario: Equal-looking facts belong to different HBK domains

- **WHEN** platform, BSL-language or query-language facts share a display or normalized name
- **THEN** the model preserves their distinct domain ownership
- **AND** it does not merge them through name equality or a generic entity family

#### Scenario: Extension or composition semantics are unknown

- **WHEN** multiple source rows may represent extension, inheritance or composition rather than accidental duplication
- **THEN** the model records the relation as unresolved or source-specific evidence
- **AND** no canonical merge is inferred until the owning formation/composition contract is accepted

### Requirement: Shared semantic traits express roles, not ownership

Source-neutral semantic traits SHALL describe allocation-free behavior over borrowed HBK entities without owning HBK records, locators, registries or indexes. Associated values MAY remain provider-specific where their identity or evidence semantics differ.

#### Scenario: A common callable algorithm consumes HBK data

- **WHEN** an algorithm uses a shared callable/signature/parameter role over an HBK view
- **THEN** HBK retains entity identity, storage and lookup ownership
- **AND** the trait does not require a common cross-provider ID or copied neutral entity record

### Requirement: Domain decisions are traceable to current data and consumers

Every accepted entity boundary and identity rule SHALL cite representative provider rows, H0/X1 structures, borrowed operations and real consumers. Unresolved questions and current-model contradictions SHALL remain explicit follow-up items rather than being hidden by terminology.

#### Scenario: The target model differs from the current snapshot taxonomy

- **WHEN** the model concludes that a current record family combines concepts or duplicates an owner
- **THEN** the domain document records the current-to-target mapping and affected consumers
- **AND** production changes are delegated to a separate accepted implementation change with preservation and resource evidence
