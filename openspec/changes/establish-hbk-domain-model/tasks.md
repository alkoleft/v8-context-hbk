## 1. Current Domain Evidence

- [ ] 1.1 Inventory existing canonical OpenSpec requirements, archived design decisions and task-relevant legacy documentation for HBK entities, identity, snapshot ownership, borrowed roles, provenance and H0/X1 storage; record conflicts rather than choosing silently.
- [ ] 1.2 Inventory provider schema/source families and representative real-corpus rows for platform API, BSL-language, query-language and documentation provenance concepts, including canonical source keys and owner relations.
- [ ] 1.3 Inventory every H0/X1 record, typed locator, `StringId` use, `HbkFactRef` variant, borrowed view, index, relation, serializer section and validation rule; map each to its source input and exposed operation.
- [ ] 1.4 Inventory all in-repository and external operations that address, compare, retain, serialize or resolve HBK concepts, with special attention to paired locators and values escaping borrowed callbacks.

## 2. Ubiquitous Language And Classification

- [ ] 2.1 Create `domain-model.md` with one glossary for dataset/generation, semantic entity, value object, evidence/relation, projection/index, locator, dictionary address, lookup key, source key and durable reference.
- [ ] 2.2 Classify every inventoried source/snapshot/view concept into exactly one primary modeling category and name its semantic or infrastructure owner; attach source, H0/X1 and consumer evidence to each classification.
- [ ] 2.3 Define the identity/lifecycle table for every accepted entity: domain owner, dataset/generation scope, canonical provider key, dense locator, owner qualification, uniqueness, equality, rename/change semantics, persistence status and lookup/resolution flow.
- [ ] 2.4 Define the bounded-context and relation diagrams for platform API, BSL-language, query-language and documentation provenance, keeping snapshot storage/indexing outside the semantic domain map.
- [ ] 2.5 Record the complete current-to-target mapping for Rust records, views, IDs and indexes, naming every duplicate owner, combined concept, misleading term, missing invariant and legitimate storage projection.
- [ ] 2.6 Map shared semantic traits to their HBK entity/value roles and provider-specific facets; verify that traits own neither HBK identity nor entity storage and do not require a common cross-provider ID.

## 3. High-Impact Domain Decisions

- [ ] 3.1 Resolve and document platform type versus enum identity/role boundaries, including type references, templates and name/alias lookup semantics.
- [ ] 3.2 Resolve and document callable, type-member, global-method, event, constructor and module-context boundaries, including whether current parallel records are entities or projections.
- [ ] 3.3 Resolve and document property, global-property and enum-value boundaries, including owner scoping and whether a single property identity family is justified.
- [ ] 3.4 Resolve and document which BSL-language and query-language facts require independent entity identity and which are values, evidence or catalog projections.
- [ ] 3.5 Identify extension/inheritance/composition and same-name source cases; assign each unresolved rule to its owning formation/cross-source change and prohibit deduplication until that rule is accepted.
- [ ] 3.6 Decide which current provider source keys have only generation-local meaning and whether any proven consumer requires a separate durable HBK reference contract; do not design persistence without such a consumer.

## 4. Validation And Dependent Change Handoff

- [ ] 4.1 Validate the model against deterministic representative scenarios for each bounded domain, equal-looking unrelated facts, duplicate projections, embedded signatures/parameters, aliases, provenance and cross-generation locator reuse.
- [ ] 4.2 Reconcile every decision with `hbk-domain-model` requirements and current HBK capability specs; update affected proposal/design/spec content rather than leaving contradictions in narrative evidence.
- [ ] 4.3 Update `audit-and-deduplicate-hbk-entities` with the accepted classifications, canonical-owner criteria and unresolved blockers before any production record-deletion task starts.
- [ ] 4.4 Run the pre-acceptance and actual-document `mattpocock-skills:codebase-design` passes plus a fresh reviewer, resolve findings and record PASS in `design.md`; verify no production structure, adapter, registry or schema entered the diff.
- [ ] 4.5 Update architecture documentation/navigation if the accepted domain boundaries require it; otherwise record the evidence-backed no-update decision, then apply the required patch workspace version bump.
- [ ] 4.6 Complete supporting evidence and task status, run strict change and canonical OpenSpec validation, archive and synchronize the new capability spec, inspect staged scope and create the required task-scoped Conventional Commit.
