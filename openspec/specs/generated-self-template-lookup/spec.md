# generated-self-template-lookup Specification

## Purpose
TBD - created by archiving change add-generated-self-template-lookup. Update Purpose after archive.
## Requirements
### Requirement: Generated-self selector resolves through the public HBK resolver

The system SHALL resolve a documented opaque generated-self role selector to
an existing platform template using a source/domain-qualified public resolver
lookup. The returned type, its members and all template evidence SHALL remain
HBK-owned facts.

#### Scenario: Certified role resolves one template

- **WHEN** an active platform source receives a metadata-provider-certified
  generated-self role selector and matching source/domain filters
- **THEN** it returns the unique `ResolvedType` for its classified platform
  template through the existing resolver response shape
- **AND** the result preserves source-qualified identity and template evidence
- **AND** no caller supplies or receives a `PlatformTypeTemplateKey` for this
  operation

#### Scenario: Source and domain restrict the lookup

- **WHEN** two active sources or domains expose conflicting template facts for
  a selector
- **THEN** only the source/domain requested by the lookup participates
- **AND** an incompatible requested source or domain returns `NotFound`

### Requirement: Generated-self selector failure semantics are explicit

The system SHALL distinguish ordinary unknown selector/template, unsupported
source capability, ambiguity and provider failure without a fallback lookup.

#### Scenario: Selector has no classified template

- **WHEN** the active HBK source has no template classified for a selector
- **THEN** the resolver returns `NotFound`
- **AND** it does not try exact-name, alias or another source implicitly

#### Scenario: Source does not implement the capability

- **WHEN** a non-platform or legacy source receives the generated-self lookup
- **THEN** it returns `Unsupported`
- **AND** it does not report an empty successful template list

#### Scenario: Several templates match one selector

- **WHEN** provider classification produces more than one template for one
  active selector/source/domain query
- **THEN** the resolver returns `Ambiguous`
- **AND** no result is selected by insertion or storage order

#### Scenario: Provider lookup fails

- **WHEN** the provider-owned search or snapshot lookup fails
- **THEN** the resolver returns its existing typed `ResolveError`
- **AND** it does not collapse the failure to `NotFound` or retry another
  resolver path
