## Context

`HbkPropertyView` is the existing closed borrowed property seam over either an
`HbkTypeMemberView` or a BSL `HbkGlobalFactView`. Both concrete views already
return a provider-owned `StringId` from `id()`, but the filtered view exposes
only the source-neutral `PropertyView` observations. The analyzer's retained
semantic oracle previously hashed that source identifier; after the property
cutover it could only hash `PropertyView::name()`, changing the real-X1 digest
from the accepted value while preserving the same member count.

The neutral role must not absorb provider identity, and downstream code must
not add a mirrored field or a second dense-ID rehydration lookup. HBK therefore
owns the missing source-only observation.

## Goals / Non-Goals

**Goals:**

- Lend the exact existing member/global source identifier through
  `HbkPropertyView` without allocation, copying or lookup.
- Preserve the same result for owned and mapped/X1 source views.
- Restore the downstream semantic oracle's accepted source-identity bytes.

**Non-Goals:**

- Changing the neutral `PropertyView` capability or treating source identity as
  a universal semantic ID.
- Adding a catalog lookup, analyzer projection, storage/index, cache/schema,
  serialized format or CLI contract.
- Changing property name, owner, kind or declared-type behavior.

## Decisions

### Deepen the existing filtered property view

Add one inherent `source_id() -> StringId` method on `HbkPropertyView`. Its
private match delegates to `HbkTypeMemberView::id()` or
`HbkGlobalFactView::id()`. The method is infallible because the closed wrapper
can only contain one of those two validated source views.

This keeps the module deep: callers use the existing property seam, while the
owned/X1 representation and source branching remain private. Adding the value
to `PropertyView` is rejected because it is provider-only evidence. Adding a
field to `BslSelectedView` or a public dense-ID catalog lookup is rejected
because either would create a downstream mirror or a second access path.

### Verify both source families and storage representations

Extend the existing semantic-role test surface so a member property and a BSL
global property report the exact concrete-view `id()` value. The current
owned/X1 parity fixture must observe identical source IDs. The downstream
real-X1 effective-context scenario must recover its accepted count and digest.

### Structure impact

Searches covered `HbkPropertyView`, `HbkTypeMemberView::id`,
`HbkGlobalFactView::id`, `StringId`, owned/X1 view implementations, semantic
role tests, context selection, analyzer benchmark oracle, public re-exports,
storage/indexes, cache/schema/serialization and CLI/export surfaces.

Reuse the existing wrapper, private inner enum and two source accessors. Add one
reusable owner-local getter and focused assertions. Add no semantic structure,
field, enum, iterator, adapter, conversion, mapping, reader, loader, resolver,
serializer, cache key, registry, index, re-export or dependency. The only real
downstream consumer is the crate-private analyzer benchmark oracle.

### Reintroduction guard

The single allowed source-identity flow is
`HbkTypeMemberView/HbkGlobalFactView -> HbkPropertyView::source_id() ->
operation-local consumer`. Tests must fail if either source family or owned/X1
representation returns a display name, dense ordinal or copied value instead.
Review and source search reject a neutral source-ID field, analyzer selected
mirror, or second public catalog lookup for the same evidence.

## Risks / Trade-offs

- **[Risk]** Callers treat `StringId` as persistent or cross-generation
  identity. → The API returns the existing generation-local provider type and
  documentation names it source identity; no serialization or equality domain
  is added.
- **[Risk]** A later source variant lacks an identifier. → The private exhaustive
  match makes that source-owner decision a compile-time change.

## Migration Plan

1. Add the accessor and owned/global/mapped parity assertions.
2. Update the downstream oracle to resolve the returned `StringId` through its
   already borrowed catalog.
3. Run strict HBK and downstream verification, then bump HBK patch version to
   `0.2.7` and commit the HBK change before the downstream completion commit.

Rollback is the inverse two-repository commit pair; there is no persisted data
or schema migration.

## Open Questions

None.

## Codebase-Design Review Record

### Pre-implementation pass

**PASS (2026-08-03).** Reviewed `snapshot/semantic_roles.rs`, the concrete view
`id()` accessors, analyzer selection/oracle consumers and rejected alternatives.
The existing filtered property view is the narrow owner and the new method
deepens that module without a pass-through adapter. Direct delegation preserves
locality and lifetime safety; a neutral field, analyzer mirror or dense-ID
lookup would be shallower and duplicate the source path. No actionable finding
remains.

### Actual-diff pass

**PASS (2026-08-03).** Reviewed the complete HBK production/test/OpenSpec/
version diff and the downstream oracle call. `source_id()` is one exhaustive
match on the existing private inner enum and delegates directly to the concrete
view `id()` methods. It adds no state, conversion, lookup or alternate facade;
the returned `StringId` remains generation-local. The parity test covers member
and global sources through both owned and mapped views. The structural guard's
allocating-return check was narrowed from the ambiguous substring `-> String`
to `-> String {`, so it continues to reject an owned `String` return without
mistaking `StringId` for one. No shallow module, duplicated behavior or
unlisted Structure impact remains.

## Final Verification Record

The downstream real-X1 prepared module-context guard restored the accepted
1,798-member digest
`4006d1c39dd3f767f2d8f2f88917123df4215dd091b146c6d27b201fa628478f`.
HBK `cargo fmt --all -- --check`, strict Clippy for
`syntax-helper-search`, the full workspace test suite, strict validation of
this change and `openspec validate --all --strict` all passed. The downstream
analyzer's affected strict Clippy, full workspace tests, both active strict
OpenSpec validations and diff check also passed.

Fresh correctness and architecture/performance reviewers inspected the HBK
and downstream actual diffs. Both returned `NO FINDINGS`; their focused checks
covered exact member/global and owned/X1 source-ID parity, the filtered-role
structural guard, downstream generated-self callable selection, shared arity
behavior and the restored oracle flow. The change is ready to archive after its
scoped commit; it is intentionally not archived by the apply workflow.
