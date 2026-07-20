## ADDED Requirements

### Requirement: HBK classifies the opaque form-attribute selector

HBK SHALL expose a source-neutral language classifier for an opaque metadata
member selector and SHALL return an existing `MemberQueryKind` only for the
documented initial corpus.

#### Scenario: Certified form attribute is classified

- **WHEN** a caller supplies `metadata.form-member.attribute`
- **THEN** HBK returns `MemberQueryKind::Property`
- **AND** the caller does not map a metadata role to a BSL kind

#### Scenario: Selector is not in the accepted corpus

- **WHEN** a caller supplies an unknown selector, a form command/element/event
  selector or a generated-member selector
- **THEN** HBK returns normal absence
- **AND** it does not infer a kind from spelling, source data or a template

#### Scenario: No source query is involved

- **WHEN** the classifier is called
- **THEN** it does not route a source/domain, inspect capabilities, call an
  adapter or fabricate a source-backed member fact
