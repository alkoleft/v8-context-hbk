## ADDED Requirements

### Requirement: HBK classifies opaque metadata member selectors

HBK SHALL expose a source-neutral language classifier for an opaque metadata
member selector and SHALL return an existing `MemberQueryKind` only for the
documented initial corpus.

#### Scenario: Certified property source roles are classified

- **WHEN** a caller supplies `metadata.form-member.attribute` or
  `metadata.generated-member.property` or
  `metadata.generated-self-alias.property`
- **THEN** HBK returns `MemberQueryKind::Property`
- **AND** the caller does not map a metadata role to a BSL kind

#### Scenario: Selector is not in the accepted corpus

- **WHEN** a caller supplies an unknown selector, a form command/element/event
  selector or an unsupported generated selector
- **THEN** HBK returns normal absence
- **AND** it does not infer a kind from spelling, source data or a template

#### Scenario: No source query is involved

- **WHEN** the classifier is called
- **THEN** it does not route a source/domain, inspect capabilities, call an
  adapter or fabricate a source-backed member fact
