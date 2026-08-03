# borrowed-semantic-role-capabilities Specification

## Purpose
Define shared borrowed semantic roles over provider-owned HBK views without a
second semantic store, index or downstream re-export.
## Requirements
### Requirement: HBK exposes platform declarations through shared borrowed roles

HBK SHALL implement the solution-owned callable, signature, parameter,
property and type-declaration capabilities over its provider-owned borrowed
views. It SHALL NOT construct a second owned semantic record family, common
catalog, shared index or compatibility re-export.

#### Scenario: A platform callable has overloads

- **WHEN** a consumer traverses an `HbkCallableView` through `CallableView`
- **THEN** every existing `HbkSignatureView` is yielded in source order
- **AND** every existing parameter and declared result type-reference view is
  lent without collecting or cloning

#### Scenario: Owned and X1 sources expose the same declaration

- **WHEN** equivalent owned and canonical X1 snapshots expose a callable,
  property or platform type
- **THEN** the common role observations are identical
- **AND** both paths preserve their concrete provider IDs and source ordering

### Requirement: Role classification uses only HBK source evidence

HBK SHALL map its local callable and member kinds to independent common role
enums without inferring availability, type equality or unsupported BSL
semantics.

#### Scenario: HBK declares a global method

- **WHEN** `HbkCallableKind::GlobalMethod` is exposed through `CallableView`
- **THEN** the role reports platform origin, global-context ownership and
  common method kind
- **AND** it does not guess BSL procedure or function semantics

#### Scenario: HBK declares a language function

- **WHEN** `HbkCallableKind::LanguageFunction` is exposed through
  `CallableView`
- **THEN** it reports the source-proved common function kind
- **AND** it retains an empty concrete platform owner

#### Scenario: Passing mode is absent from HBK

- **WHEN** an `HbkParameterView` is exposed through `ParameterView`
- **THEN** its required flag maps to required or optional
- **AND** its passing mode is `SourceUnspecified`

### Requirement: Property roles reject non-property members

The raw `HbkTypeMemberView` and `HbkGlobalFactView` SHALL remain the complete
source views. HBK SHALL expose `PropertyView` only through one borrowed,
kind/domain-checked property-role view for type members whose kinds prove
property or enum-value semantics and BSL global facts whose kind proves
property semantics.

#### Scenario: Member is a property or enum value

- **WHEN** a consumer requests the property role for such a member
- **THEN** HBK returns a borrowed property view with the existing platform-type
  owner, name and declared type-reference iteration

#### Scenario: Member is a method or event

- **WHEN** a consumer requests the property role for such a member
- **THEN** HBK returns absence
- **AND** it does not coerce the member into a property kind

#### Scenario: Global fact is a BSL property

- **WHEN** a consumer requests the property role for a BSL
  `HbkGlobalFactKind::Property`
- **THEN** HBK returns the same filtered property view with global-context
  ownership, the existing name and declared type-reference iteration

#### Scenario: Global fact has another kind or language domain

- **WHEN** a global fact is a method or is not in the BSL domain
- **THEN** HBK returns absence
- **AND** it does not infer a common property from record shape alone

### Requirement: Platform type roles preserve exact HBK name and identity

`TypeDeclarationView` for `HbkPlatformTypeView` SHALL use
`HbkNameView<'a>` as its associated name value and the record's existing
`StringId` source ID as its associated owner/source value. It SHALL report
platform origin and platform-type ownership without allocating a display name
or inventing a common ID.

#### Scenario: A platform type is observed through the common role

- **WHEN** a consumer reads its name and owner/source value
- **THEN** the name retains the existing primary/alias HBK IDs
- **AND** the owner/source value equals `HbkPlatformTypeView::id()`

### Requirement: Name resolution stays source-backed and allocation-free

Common callable, parameter and property name getters SHALL borrow `str` from
the existing HBK snapshot dictionary. The owned view branch MAY retain a
borrowed snapshot reference needed for resolution; the mapped branch SHALL use
its existing X1 generation handle. Neither branch SHALL allocate an owned name.

#### Scenario: A common name is read repeatedly

- **WHEN** a consumer calls a common name getter on the same view
- **THEN** each result points into the same source-owned dictionary generation
- **AND** no `String`, name arena or reverse lookup registry is created

### Requirement: HBK retains sole storage and lookup ownership

The allowed flow SHALL remain
`HbkFactSnapshot -> Hbk*View -> shared capability/algorithm -> caller outcome`.
HBK SHALL retain its typed generation-local IDs and H0/X1 source indexes, and
this capability SHALL NOT add another entity locator table, cache shape,
serializer or public neutral facade.

#### Scenario: Common argument-count behavior is used

- **WHEN** a consumer checks an HBK signature's argument count
- **THEN** it uses the neutral allocation-free arity behavior over the borrowed
  parameter iterator
- **AND** HBK does not retain a second manual arity predicate

### Requirement: Filtered property views preserve provider source identity

`HbkPropertyView` SHALL expose the provider-owned `StringId` of its contained
source property through an allocation-free owner-local accessor. This source
identity MUST remain generation-local HBK evidence and MUST NOT be added to the
source-neutral `PropertyView` capability, copied into a downstream selected
record or recovered through a second catalog index.

#### Scenario: Type member property identity is observed

- **WHEN** a consumer reads source identity from an `HbkPropertyView` created
  from an `HbkTypeMemberView`
- **THEN** it equals the contained member view's existing `id()` value
- **AND** owned and mapped/X1 views report the same source identifier

#### Scenario: BSL global property identity is observed

- **WHEN** a consumer reads source identity from an `HbkPropertyView` created
  from a BSL `HbkGlobalFactView`
- **THEN** it equals the contained global view's existing `id()` value
- **AND** no display name, dense ordinal or copied identifier substitutes for
  that provider source evidence
