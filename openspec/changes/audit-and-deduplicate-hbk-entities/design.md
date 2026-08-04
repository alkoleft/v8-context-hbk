## Context

HBK already uses compact typed `u32` locators such as `HbkPlatformTypeId`, `HbkTypeMemberId`, `HbkCallableId` and `HbkGlobalFactId`. Those locators efficiently address record arenas, while record `id()` accessors currently expose generation-local `StringId` values for provider source keys.

The record taxonomy can nevertheless give one logical source entity more than one typed record locator. Read-only SQL over the frozen Russian 8.3.27.1859 provider index found 6,670 type methods and 642 type events shared by member and callable source rows, plus 500 global methods shared by global and callable rows. Their source/callable keys are equal. Snapshot materialization currently creates the corresponding member/global and callable records independently, and at least one downstream boundary retains both locators for a selected global method.

Repeated index values are not automatically duplicates: owner/name/alias/kind indexes, `HbkFactRef` relation endpoints, H0/X1 storage views and source-provenance tables may legitimately provide different access paths to one record. The audit must distinguish those references from parallel semantic ownership before deletion.

## Goals / Non-Goals

**Goals:**

- Inventory every semantic identity and record projection owned by the HBK snapshot.
- Prove which repeated representations are true duplicates and which have distinct invariants.
- Select one canonical HBK owner for every proven duplicate family.
- Migrate lookups, relations, views and consumers directly to that owner and delete the duplicate flow.
- Preserve provider behavior, source evidence, owned/mapped parity and compact generation-local access.
- Add a durable reintroduction guard and measure resource effects.

**Non-Goals:**

- A common `TypeId`, `CallableId`, `PropertyId`, provider slot or registry shared with metadata, BSL or analyzer crates.
- Replacing HBK dense locators with primary-name-derived IDs or a new string interner.
- Cross-session/persistent identity, dataset versioning or serialization of public entity IDs.
- Defining extension, inheritance or composition semantics for duplicate source declarations.
- Removing a secondary index or source facet merely because it references an already known entity.
- Runtime 1C introspection.

## Decisions

### 1. Audit semantic ownership before changing record layout

The first deliverable is a deterministic ledger, not code deletion. For each platform type, type member, callable, global fact, enum/value, query fact and language fact it records:

- canonical provider source key and semantic kind;
- all dense locators and arenas that retain it;
- payload fields and source/provenance ownership;
- primary, alias, owner, kind, relation and exact-ID indexes;
- owned H0 and mapped X1 records/views;
- in-repository and external consumers;
- whether the repetition is an entity owner, non-owning projection, index entry or distinct entity.

The audit compares canonical source identity and semantic role evidence. It MUST NOT merge by display name, normalized name, source order or similar field shape.

**Alternative: immediately remove known member/callable pairs.** Rejected because type-member lookup may retain owner/kind/provenance behavior that must be migrated deliberately, and because the same physical pattern is not proof for every record family.

### 2. Keep dense generation-local locators

Existing typed ordinal locators remain the preferred HBK storage mechanism. Deduplication selects which arena owns a semantic entity; it does not replace ordinals with names, hashes, provider prefixes or shared IDs.

`StringId` remains a snapshot dictionary address unless a later accepted contract proves a different role. This change does not promote it to persistent identity or introduce family-specific interners.

**Alternative: use the primary-name table row as identity.** Rejected by the completed HBK experiment: lookup gains were small while separate candidate text increased retained index bytes and construction allocation materially, and duplicate/extension semantics remained unresolved.

### 3. Prefer an existing deep owner over a new canonical arena

For each duplicate family the implementation reuses the arena that already owns the complete semantic payload and the widest valid lookup behavior. For the known method/event shape, `HbkCallable` is the initial owner candidate because it owns kind, owner, names, signatures, return references and availability. The audit must still prove that member/global-only evidence has a valid owner before this is accepted.

A new semantic arena or identity type is allowed only if no existing owner can preserve the required invariant and the updated `Structure impact` plus repeated skeptic/codebase-design gates accept it. Implementation convenience is not sufficient.

**Alternative: retain both records and add a cross-map.** Rejected because it formalizes the duplicate, adds another index and leaves consumers responsible for paired identities.

### 4. Migrate one proven family at a time

Each implementation slice contains its owner selection, consumer migration, record/index deletion, H0/X1 layout update, differential tests, absence guard and measurements. A slice does not begin until its ledger row and preservation fixtures are accepted.

If X1 physical layout changes, the owning format version and cache validation change together. H0 and X1 remain parity-tested; no compatibility reader or dual-write layout is retained unless a concrete external artifact contract is discovered and accepted first.

### 5. Structural absence is part of correctness

Behavioral parity alone cannot detect a second record owner that happens to return identical data. The change therefore adds narrow structural checks for each removed duplicate path, such as prohibiting a method/event from being materialized into both the accepted callable owner and a named former semantic owner. Checks may not freeze unrelated private decomposition.

## Structure Impact

This change is expected to delete semantic structures and data-flow paths, not add a provider-neutral model.

- **Existing owners searched:** snapshot record arenas and typed locators in `snapshot/types.rs`; materialization in `snapshot/materialize.rs`; exact-ID/name/owner/kind/relation indexes; borrowed views and semantic roles; H0 memory reporting; X1 sections, codec, validation and parity; `context-resolver-search`; tests, fixtures, probes and external concrete-HBK consumers.
- **Search evidence:** `HbkPlatformTypeId`, `HbkTypeMemberId`, `HbkCallableId`, `HbkGlobalFactId`, `HbkEnumId`, `HbkEnumValueId`, `HbkFactRef`, `StringId`, `member_kind`, `callable_kind`, `callable`, `facts_by_id`, `members_by_owner`, `callables_by_owner`, `globals_by_name`, source key equality and paired view/ID fields.
- **Inputs/outputs:** provider SQLite rows and persisted X1 records enter HBK-owned materialization/read paths; outputs are existing lookup results, borrowed views, provenance and relations.
- **Readers/parsers/normalizers:** no new source reader, parser, name normalizer or fallback SQL path is allowed. Audit SQL/probes are independent read-only evidence and do not become production behavior.
- **Registries/caches/serializers:** no new registry, interner, reverse map, cache DTO or compatibility serializer is allowed. X1 sections may be deleted or reshaped only with format/version validation.
- **Public re-exports/conversions:** every removed locator/view pairing migrates directly; no alias, wrapper or conversion recreates it.

If implementation discovers a required unlisted semantic owner, adapter, registry, mapping or serialized shape, it stops before adding it, updates this ledger and affected specifications, and repeats the skeptic and codebase-design gates.

## Reintroduction Guard

- **Root cause:** one source document is materialized independently for each access role, so record taxonomy becomes semantic identity taxonomy and consumers retain paired locators.
- **Single allowed flow:** one canonical HBK semantic owner per source entity; secondary indexes and borrowed role/source facets reference that owner without owning another identity or equivalent payload.
- **Verification:** deterministic real-corpus inventory plus focused structural-absence tests reject every named removed duplicate family, and consumer searches reject paired locator reconstruction, compatibility adapters and cross-maps.

## Risks / Trade-offs

- **[Risk] Distinct source facts are merged because they share a name or payload.** → Require canonical provider source identity and semantic-role evidence; unresolved candidates remain separate.
- **[Risk] Removing a projection loses owner, kind, availability or provenance evidence.** → Add preservation fixtures before implementation and relocate evidence only to an existing valid owner/index.
- **[Risk] X1 compatibility or mmap layout regresses.** → Version and validate the format, run full owned/mapped parity, and compare artifact size and lookup latency.
- **[Risk] A new canonical wrapper merely renames the duplicate.** → Default to an existing deep owner; treat any new arena/type as a blocking structure-ledger change requiring repeated review.
- **[Risk] External consumers depend on concrete paired IDs.** → Inventory them before owner selection and migrate them directly in the same slice; do not add a compatibility seam.
- **[Trade-off] Family-by-family migration takes longer than a taxonomy rewrite.** → It bounds semantic and resource risk and permits stopping when a candidate is not proven duplicate.

## Migration Plan

1. Freeze deterministic source and snapshot duplicate counts for the accepted HBK corpus and create the complete identity/projection ledger.
2. Inventory real consumers and preservation behavior; resolve each candidate as duplicate, legitimate projection/index or distinct entity.
3. Capture H0/X1 build, memory, artifact-size and lookup baselines.
4. Run the required skeptic-review and pre-implementation codebase-design pass for the first accepted family.
5. Migrate one family atomically: canonical owner, indexes/relations/views, consumers, deletion, format handling, tests and measurements.
6. Repeat only for proven families, then run workspace verification and actual-diff reviews.
7. Apply the required patch version bump, validate OpenSpec and commit the scoped change.

Rollback is a normal commit revert before publishing a new X1 format. After a new persisted format is published, rollback requires restoring the matching format version and cache invalidation behavior together; dual layouts are not retained by default.

## Codebase-Design Review Record

### Pre-implementation pass

- **Status:** PENDING; implementation is blocked until the selected family, interfaces, owner, consumer migration and `Structure impact` are reviewed.
- **Required focus:** existing deep owner, useful public operation, H0/X1 locality, absence of pass-through adapters, and proof that the slice deletes rather than renames duplicate ownership.

### Actual-diff pass

- **Status:** PENDING; completion is blocked until the actual diff is reviewed.
- **Required focus:** promised deletions, no parallel shape/conversion/index, direct consumers, preserved behavior, structural guard and measured resource impact.

## Open Questions

- Which member/global fields are genuinely source facets versus duplicated callable payload?
- Can method/event member and global lookups return or borrow the canonical callable directly without weakening exact-kind and provenance behavior?
- Do any other entity families contain true duplicate semantic ownership, especially language facts or enum/type role projections?
- Which concrete HBK IDs are used outside a borrowed provider operation, and does any real consumer require durable follow-up lookup?
- Which X1 sections and format-version changes are necessary for each accepted removal?

## Version Classification

Completion requires a patch version bump: the change removes duplicate provider representation and may break provisional Rust contracts, but adds no shipped user-facing functionality.
