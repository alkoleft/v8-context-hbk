## Context

HBK owns several related but distinct domains: documented platform API, BSL-language facts, query-language facts and documentation/source provenance. The current snapshot taxonomy was shaped by extraction and lookup requirements, so Rust records sometimes combine semantic concepts with materialization roles:

- `Hbk*Id` values are typed dense generation-local arena locators;
- record `id` fields are commonly `StringId` addresses of provider source keys;
- `HbkFactRef` is a mixed record reference used by relations;
- `Hbk*View` types abstract owned H0 and mapped X1 storage;
- indexes repeat locators for exact ID, primary/alias name, owner, kind and relations;
- some source entities appear in multiple semantic-looking record families.

Those mechanisms are useful, but they are not yet an explicit domain model. The deduplication change cannot safely select canonical owners until HBK distinguishes domain identity from record layout and lookup projections.

## Goals / Non-Goals

**Goals:**

- Establish an HBK ubiquitous language and bounded-context map.
- Classify all current concepts as entity, value, evidence/relation, projection/index, locator or infrastructure.
- Define identity, equality, lifetime and ownership independently from storage representation.
- Define aggregate/ownership boundaries and permitted cross-domain relations.
- Map current provider rows, H0/X1 types and public borrowed operations to the accepted concepts.
- Supply an explicit prerequisite for entity deduplication and future provider-trait work.

**Non-Goals:**

- Refactoring production Rust records, indexes, X1 layout or public APIs.
- Selecting a common identity representation for HBK, metadata and BSL.
- Creating a universal entity graph, registry, interner, repository abstraction or provider facade.
- Defining persistent/cross-session identity without a real persisted consumer.
- Resolving extension/inheritance/composition behavior owned by source formation or cross-source type work.
- Adding runtime 1C introspection.

## Decisions

### 1. The domain model is semantic and storage-independent

The accepted model describes concepts and invariants before mapping them to Rust. Existing record names do not automatically define entities, and a target concept does not automatically require a new struct or ID.

The primary deliverable is `domain-model.md` within this change. It contains the ubiquitous-language glossary, domain map, classification matrix, identity/lifecycle table, relation diagram, current-to-target mapping and resolved/unresolved decisions. OpenSpec requirements remain the normative guard; the document supplies the detailed evidence and rationale.

**Alternative: treat the snapshot structs as the domain model.** Rejected because record layout already reflects H0/X1 and lookup concerns and contains known parallel projections.

### 2. Use six non-overlapping modeling categories

Every inventoried shape receives exactly one primary category:

1. **Semantic entity** — independent identity matters and the entity participates in relations or follow-up lookup.
2. **Embedded value object** — equality is by value and lifecycle belongs to an owner.
3. **Source evidence/relation** — provenance or a typed relation about entities, not another entity copy.
4. **Lookup projection/index** — derives an access path to an owner without semantic ownership.
5. **Generation-local locator** — compact address into one immutable snapshot generation.
6. **Infrastructure/storage detail** — H0/X1 representation, ranges, offsets, dictionary addresses or validation metadata.

An entity may have value/evidence/projection representations, but a physical shape cannot use those roles interchangeably without an explicit mapping.

**Alternative: entity versus value only.** Rejected because it would misclassify indexes, locators and provenance as domain entities or values and would not explain the current duplicate problem.

### 3. Identity is described along independent axes

For each entity the model records:

- owning HBK domain and dataset/generation scope;
- canonical provider semantic key, if one exists;
- dense generation-local locator used for access;
- owner qualification and uniqueness rule;
- rename/change semantics;
- whether any real contract requires persistence;
- resolution from lookup key to locator and from locator to borrowed entity.

`StringId` is treated as a dictionary address; the string it addresses may be a source key, name, alias or evidence value. `Hbk*Id` is treated as a generation-local locator. Neither becomes cross-session identity by implication.

**Alternative: select one representation during modeling.** Rejected because identity semantics differ by HBK family and no current persistence contract justifies a universal representation.

### 4. Start from four bounded HBK domains

The initial domain map distinguishes:

- **Platform API catalog** — platform types, callables, properties, globals, enums/values, signatures and declared type relations.
- **BSL-language catalog** — language types, constructs, functions, operators, keywords, literals and module contexts.
- **Query-language catalog** — query tables, fields, parameters and query-language relations.
- **Documentation provenance** — source document/page/path/locale evidence linked to facts.

Snapshot materialization, H0/X1 storage and search indexes are infrastructure supporting these domains, not a fifth semantic catalog. The investigation may refine these boundaries, but it must document why.

### 5. Traits are semantic capabilities over owner records

Shared traits define reusable behavior and associated borrowed values; they do not define a provider-neutral entity aggregate. The model records which HBK concept implements each role and which provider-specific facet remains outside the common trait.

This keeps identity representation provider-owned while permitting common algorithms for names, callable arity, signatures, parameters, properties and type declarations.

**Alternative: add common entity records and IDs beside traits.** Rejected because that duplicates provider entities and makes borrowing/storage convenience define the domain.

### 6. Domain decisions require evidence triangulation

Each classification uses three evidence classes:

- source/provider schema rows and stable keys;
- H0/X1 records, indexes, relations and views;
- real operations and consumers that address or compare the concept.

Two evidence classes are insufficient when they disagree. Ambiguous candidates remain unresolved and block dependent production changes.

### 7. Deduplication consumes this model but remains separate

`audit-and-deduplicate-hbk-entities` supplies detailed duplicate evidence and performs production migration. This change establishes the terms and canonical-owner criteria. The deduplication change MUST NOT begin record-deletion slices until the relevant domain classifications and identity rules are accepted here.

The domain-model change itself does not edit production code, X1 schemas or external repositories.

## Structure Impact

- **Production semantic structures/conversions/mappings:** none; this change is documentation and architecture analysis only.
- **Areas inventoried:** provider SQLite schema and extraction model; all snapshot record/ID/view/index/relation families; H0/X1 materialization, serialization and validation; semantic-role traits; in-repository and external consumers; fixtures, probes and existing OpenSpec decisions.
- **Search evidence:** `Hbk*Id`, `StringId`, `HbkFactRef`, `Hbk*View`, record `id`/`owner`/`callable` fields, name/alias/exact-ID indexes, source/provenance, signatures, parameters, type refs, availability, templates, module contexts, query facts, language facts and paired identities.
- **New documentation shape:** one `domain-model.md` detailed model owned by this OpenSpec change. It does not become a parallel implementation ledger or generated schema.
- **Readers/parsers/registries/caches/serializers:** no production path is added or changed. Read-only queries/probes may gather evidence but remain task artifacts.

Any production structure proposed while resolving the model is moved to a separate implementation change with its own `Structure impact`, review and verification.

## Reintroduction Guard

Future HBK changes that add or change an entity, ID, record, view, index, relation or public trait mapping must name its domain category, single owner, identity/lifecycle rule and current-to-target mapping. Review rejects a new shape that cannot be classified or that recreates an existing semantic owner under a storage/lookup name.

## Risks / Trade-offs

- **[Risk] The model merely renames current Rust structs.** → Require source, storage and consumer evidence plus explicit current-to-target mismatches.
- **[Risk] Modeling becomes a speculative universal ontology.** → Limit it to current HBK facts and retained operations; unresolved future concepts stay out of scope.
- **[Risk] Similar names cause cross-domain merging.** → Preserve bounded-domain ownership and prohibit name-only equivalence.
- **[Risk] Every contained value receives an ID.** → Require independent addressability and lifecycle evidence before classifying a value as entity.
- **[Risk] The model blocks useful implementation indefinitely.** → Resolve one bounded family at a time and publish explicit unresolved decisions with the precise missing evidence.
- **[Trade-off] The detailed mapping duplicates some code vocabulary in documentation.** → The document records semantic roles and mismatches, not field-for-field record schemas; code remains the storage source of truth.

## Migration Plan

1. Inventory current source, storage and consumer vocabularies without changing production code.
2. Draft the glossary, category matrix, bounded-context map and identity/lifecycle table.
3. Resolve high-impact cases first: type/member/callable/property/global/enum boundaries and record-versus-semantic identity.
4. Validate the model against representative platform, BSL-language, query and provenance scenarios plus known duplicate evidence.
5. Review and accept the model; update dependent deduplication tasks with the accepted canonical-owner criteria.
6. Synchronize the new canonical capability spec, record the architecture navigation/update decision, bump the required patch version and archive the documentation change.

Rollback is a documentation revert. No runtime artifact or cache migration is performed by this change.

## Codebase-Design Review Record

### Pre-document acceptance pass

- **Status:** PENDING.
- **Required focus:** domain boundaries reflect real variation; terminology maps to deep existing owners; traits remain roles; no new shallow facade, DTO, registry or speculative identity is proposed.

### Actual-diff pass

- **Status:** PENDING.
- **Required focus:** complete current-to-target mapping, one owner per concept, explicit unresolved cases, no production changes and no parallel task/source-of-truth document.

## Open Questions

- Is a platform enum a platform type entity, a distinct entity family, or a type role over an enum owner?
- Are type/global methods and events one callable entity with member/global projections in every case?
- What is the canonical property entity across type members, global facts and enum values, if a single family is justified at all?
- Which BSL-language and query-language facts require independent identity versus value/evidence classification?
- Which provider source keys have stable semantics across HBK datasets, and does any real consumer require that stability?
- Which current `HbkFactRef` variants represent semantic entities versus record/provenance endpoints?
- Where do extension/inheritance/composition rows enter formation, and which change owns their resolution?

## Version Classification

Completion requires a patch version bump because the change establishes architecture/domain documentation without shipped user-facing functionality.
