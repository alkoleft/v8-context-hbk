## ADDED Requirements

### Requirement: HBK interprets opaque metadata BSL-context selectors

The system SHALL resolve metadata-owned opaque module-role selectors only
through source/domain-qualified HBK lookup and return existing module-context
facts.

#### Scenario: Selector resolves one source-backed relation

- **WHEN** an active platform source receives a certified selector with matching
  source/domain filters
- **THEN** HBK returns the matching existing module-context fact
- **AND** the caller neither supplies nor observes an HBK module kind key

#### Scenario: Selector cannot be resolved

- **WHEN** a selector is unknown, unsupported, ambiguous or the provider fails
- **THEN** the corresponding explicit resolver outcome is returned
- **AND** no name, alias or cross-source fallback is attempted

#### Scenario: Source and capability selection precede selector relation lookup

- **WHEN** a module-role selector supplies source/domain filters
- **THEN** HBK considers only matching active sources
- **AND** no matching source is `NotFound`, while any matching source without
  module-context capability is `Unsupported`
- **AND** an eligible `object`, `manager` or `form` selector preserves the
  existing module-context resolver outcome
- **AND** eligible `common`, `command` and `record_set` selectors are
  `NotFound` until HBK facts are accepted.
