## Why

Metadata source facts need exact semantic kinds without making the metadata
provider expose BSL kinds or making the analyzer map metadata roles. A
provider-certified form attribute or generated member selector is a stable
metadata-owned source-role literal; only HBK may classify that literal as a BSL
property.

## What Changes

- Add one source-neutral HBK core classifier from an opaque metadata member
  selector to the existing `MemberQueryKind`.
- Accept `metadata.form-member.attribute` and
  `metadata.generated-member.property` in the corpus and map each to
  `Property`.
- Return normal absence for unknown selectors and unsupported direct form or
  generated-member roles.

## Non-Goals

- No `ContextResolver` method, `SourceId`/domain routing, source capability,
  platform fact, resolver status, template key, adapter mapping or index.
- No generated-member query or answer: the existing metadata point/enumeration
  fact remains evidence-bearing; this classifier supplies only its BSL kind.
- No metadata dependency or copied metadata enum.
