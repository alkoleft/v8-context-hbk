## Why

The filtered `HbkPropertyView` preserves a borrowed member or global-property
source view but exposes only common property semantics. A real downstream
semantic oracle must preserve the provider-owned source identifier previously
observed from those concrete views; substituting display name changes the
accepted digest, while adding an analyzer mirror or a second dense-ID lookup
would violate the owner boundary.

## What Changes

- Expose the contained property record's provider-owned `StringId` through one
  allocation-free owner-local `HbkPropertyView::source_id()` accessor.
- Preserve exact owned/X1 and member/global parity with direct delegation to the
  existing concrete source views.
- Keep the neutral `PropertyView` contract, HBK storage/indexes, serialized X1
  format and public catalog lookup surface unchanged.
- Add a focused public-contract test and downstream real-X1 digest evidence.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `borrowed-semantic-role-capabilities`: The existing filtered HBK property
  view exposes its concrete provider source identifier without copying it into
  a common role or downstream mirror.

## Impact

- Affected owner crate: `crates/syntax-helper-search`.
- Public Rust API: one inherent getter on the existing `HbkPropertyView`.
- Downstream consumer: the private `v8-context` analyzer benchmark oracle.
- Storage, dependencies, cache/schema/serialization, indexes and CLI output:
  unchanged.
- Completion version: patch bump from `0.2.6` to `0.2.7`; no shipped
  user-facing functionality is added.
