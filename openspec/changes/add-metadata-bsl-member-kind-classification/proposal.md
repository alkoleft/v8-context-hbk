## Why

Metadata source facts need exact semantic kinds without making the metadata
provider expose BSL kinds or making the analyzer map metadata roles. A
provider-certified form attribute, generated member or generated self-alias
selector is a stable metadata-owned source-role literal; only HBK may classify
that literal as a BSL property.

## What Changes

- Add one source-neutral HBK core classifier from an opaque metadata member
  selector to the existing `MemberQueryKind`.
- Accept `metadata.form-member.attribute`,
  `metadata.generated-member.property` and
  `metadata.generated-self-alias.property` in the corpus and map each to the
  existing `Property`.
- Return normal absence for unknown selectors and unsupported direct form or
  generated selectors.

## Non-Goals

- No `ContextResolver` method, `SourceId`/domain routing, source capability,
  platform fact, resolver status, template key, adapter mapping or index.
- No generated-member or generated-self-alias query or answer: the existing
  metadata point/enumeration fact keeps identity and evidence; this classifier
  supplies only the existing BSL `Property` kind.
- No metadata dependency or copied metadata enum.
