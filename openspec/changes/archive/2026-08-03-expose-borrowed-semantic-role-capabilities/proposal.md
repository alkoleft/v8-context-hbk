## Why

The solution-level semantic kernel now owns source-neutral borrowed roles for
callables, signatures, parameters, properties and type declarations. HBK is
the platform declaration owner, but its accepted H0/X1-neutral `Hbk*View`
family currently exposes dictionary `StringId` values separately from the
`HbkFactReadHandle` that resolves them. Downstream consumers therefore cannot
use the common role contracts directly without either copying provider facts or
adding a shallow adapter family.

HBK should deepen its existing storage-neutral views so they can lend their
existing names and children through the neutral roles while retaining the
provider snapshot as the sole storage, identity and index owner.

## What Changes

- Add a direct path dependency from `syntax-helper-search`, the owner of the
  public `Hbk*View` family, to `v8-context-semantic-entities`.
- Keep the existing public view types and make their owned branches retain the
  already-borrowed snapshot context needed to resolve names; mapped X1 branches
  continue to use their existing generation handle.
- Implement `CallableView`, `SignatureView`, `ParameterView` and
  `TypeDeclarationView` directly on existing HBK views.
- Add one filtered borrowed `HbkPropertyView` over either an existing
  `HbkTypeMemberView` or a BSL `HbkGlobalFactView`, because both raw families
  also represent non-property roles that cannot truthfully implement
  `PropertyView` directly.
- Preserve every overload, parameter order, required flag, owner ID and
  declared type-reference view; represent missing passing-mode evidence as
  `SourceUnspecified`.
- Add owned/X1 parity, common arity and structural boundary tests.

## Capabilities

### New Capabilities

- `borrowed-semantic-role-capabilities`: HBK platform declarations implement
  the solution-owned borrowed semantic role contracts without constructing a
  common provider record, index or compatibility facade.

### Modified Capabilities

None.

## Impact

- Affected owner crate: `crates/syntax-helper-search` only.
- Dependency direction: `syntax-helper-search` depends directly on the neutral
  `std`-only leaf; HBK does not depend on analyzer crates.
- Runtime storage: unchanged `HbkFactSnapshot` H0/X1 storage, typed IDs,
  dictionaries, indexes and publication lifecycle.
- Public HBK API: existing views gain borrowed semantic role behavior and one
  kind/domain-checked property-role view covering platform-type and BSL-global
  properties; no neutral-crate re-export is added.
- Serialization/cache/schema: unchanged.
- Downstream selection, availability, type equality, overload selection and
  BSL procedure/function inference remain outside HBK.
