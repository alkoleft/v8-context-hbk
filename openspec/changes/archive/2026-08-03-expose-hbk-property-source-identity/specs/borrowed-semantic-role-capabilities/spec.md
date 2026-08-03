## ADDED Requirements

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
