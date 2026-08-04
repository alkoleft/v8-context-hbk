## Why

HBK storage currently mixes semantic entities, embedded values, source evidence, lookup projections and generation-local record locators in one snapshot taxonomy. Before deduplicating records or expanding provider traits, the project needs an explicit HBK-owned domain model that defines what each concept is, what gives it identity, which data is only a value/projection, and which invariants provider operations must preserve.

## What Changes

- Establish a shared HBK ubiquitous language for datasets/generations, source documents, semantic entities, embedded values, locators, source keys, lookup keys, indexes, projections, relations and provenance.
- Classify every current HBK fact family as entity, value object, relation/evidence, lookup projection or storage detail, with its identity scope, owner and lifecycle.
- Separate provider semantic identity from dense generation-local locators, `StringId` dictionary addresses, display names, aliases and persistent/external references.
- Define domain boundaries and relations for platform types, callables, properties, globals, enums/values, BSL-language facts, query facts, signatures, parameters, type references, templates, availability and documentation provenance.
- Map current H0/X1 records, views, indexes and public operations to the domain model and record every mismatch, duplicate owner, ambiguous term and missing invariant as a follow-up candidate.
- Define how source-neutral semantic traits consume borrowed HBK entities without owning HBK identity or imposing a cross-provider representation.
- Produce an accepted domain-model document, diagrams, decision tables and conformance fixtures/review checks before identity deduplication selects canonical record owners.
- Keep production record/layout refactoring outside this change; implementation belongs to explicit follow-up changes such as `audit-and-deduplicate-hbk-entities` after they consume the accepted model.

## Capabilities

### New Capabilities

- `hbk-domain-model`: Defines HBK domain terminology, semantic ownership, entity/value/projection classification, identity scopes and conformance rules for provider records and borrowed operations.

### Modified Capabilities

None.

## Impact

- Documentation and decision scope: all HBK snapshot fact families, H0/X1 storage, extraction/source evidence, search indexes, relations, public borrowed views and semantic-role implementations.
- Direct dependency: `audit-and-deduplicate-hbk-entities` must consume the accepted classification and identity-owner decisions before production deletion slices begin.
- External provider consumers are evidence inputs only; no `v8-context`, metadata or BSL production file is modified by this change.
- No new runtime dependency, registry, interner, DTO, cache, serialized layout or public identity type is introduced.
- Completion uses a patch version bump because this is architecture/domain clarification without new shipped user-facing functionality.
