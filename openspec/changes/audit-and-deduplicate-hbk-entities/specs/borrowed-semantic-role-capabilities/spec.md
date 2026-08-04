## ADDED Requirements

### Requirement: Borrowed semantic roles reuse canonical HBK identity

An HBK borrowed semantic role SHALL observe the canonical identity and payload owned by the selected HBK entity. A role adapter MUST NOT manufacture a second identity, require a caller to retain paired record locators, or copy entity-shaped data to reconcile parallel snapshot projections.

#### Scenario: A callable is selected through a member lookup

- **WHEN** a type-member, global-context or event lookup selects an entity whose canonical HBK owner is callable-shaped
- **THEN** the borrowed operation consumes the canonical callable identity and view
- **AND** any member/global lookup evidence remains a non-owning source facet or control value
- **AND** paired member-plus-callable or global-plus-callable identity is not required outside the operation

#### Scenario: A role projection has distinct source evidence

- **WHEN** a borrowed role needs owner, availability, documentation or provenance evidence not stored on its canonical semantic record
- **THEN** it borrows that evidence from the existing HBK owner or source index
- **AND** it does not introduce a mirrored entity record, reverse identity registry or compatibility adapter

### Requirement: Consumers migrate directly to the canonical provider flow

When duplicate HBK identities are removed, every in-repository and identified external consumer SHALL migrate to the canonical provider lookup/view flow in the same accepted slice. The change MUST NOT retain deprecated identity aliases, parallel DTOs, fallback lookup paths or conversions that reconstruct the deleted pairing.

#### Scenario: A public HBK locator pair is replaced

- **WHEN** the audit proves that a public pair of concrete HBK locators identifies one semantic entity
- **THEN** the owning HBK API and all inventoried consumers use the accepted canonical locator or borrowed operation
- **AND** compilation or focused structural verification fails if the removed pair or its compatibility reconstruction returns
