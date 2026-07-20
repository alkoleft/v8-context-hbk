## Why

The direct form-attribute BSL tier needs an exact semantic kind without making
the metadata provider expose BSL kinds or making the analyzer map metadata
roles. A form attribute's selector is a stable metadata-owned source-role
literal; only HBK may classify that literal as a BSL property.

## What Changes

- Add one source-neutral HBK core classifier from an opaque metadata member
  selector to the existing `MemberQueryKind`.
- Accept only `metadata.form-member.attribute` in the initial corpus and map it
  to `Property`.
- Return normal absence for unknown, direct non-attribute and every generated
  member selector.

## Non-Goals

- No `ContextResolver` method, `SourceId`/domain routing, source capability,
  platform fact, resolver status, template key, adapter mapping or index.
- No generated-member selector: generated self types retain the existing
  source-qualified template/member path.
- No metadata dependency or copied metadata enum.
