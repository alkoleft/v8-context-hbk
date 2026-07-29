## Why

The analyzer hot path needs direct access to HBK BSL-context and SDBL query-domain facts without materializing generic owned DTOs. Existing `syntax-helper-search` snapshot arenas are sufficient storage for these facts, but the current public generic owned DTO contract is too broad and allocation-heavy for analyzer-owned consumers that only need stable `Arc` catalog handles returning borrowed records over an existing snapshot/read handle.

The ownership boundary belongs upstream in HBK: BSL availability/module-selector facts and SDBL query-source classification are documented-domain facts, not analyzer-side compatibility data. Keeping those facts in HBK avoids analyzer shims, private provider reads, duplicated caches, and SQLite fallback paths. The public catalog owner is `context-resolver-search`, layered over the existing `syntax-helper-search` arenas/read handle.

## What Changes

- Add `HbkBslContextCatalog` and `HbkSdblQueryCatalog` from `context-resolver-search` over the existing HBK snapshot/read handle.
- Return `Arc` catalog handles whose methods return borrowed HBK records tied to the underlying snapshot/read-handle lifetime.
- Expose complete BSL-context capability through `HbkBslContextCatalog`: generated-self facts, owner members/callables, module members/events, globals, and typed availability.
- Complete the downstream BSL handoff by returning existing typed
  `AvailabilityContext` values and borrowed available-since text instead of raw
  snapshot string IDs, and by sharing the existing stable identity,
  type-reference and signature projection behavior between the generic adapter
  and the analyzer's concrete owned-output boundary.
- Expose SDBL query-source classification through `HbkSdblQueryCatalog`.
- Make only the narrow lifetime receiver refinement needed in the `syntax-helper-search` read handle; do not add a second API or storage path there.
- Retain the generic `ContextResolver` contract for distinct consumers that still need source-neutral resolver semantics, including generic context resolution, diagnostics/debug inspection, and non-hot-path integration tests.
- Retain the existing SQL-backed and `SearchIndex` paths for full-text/search use cases; this change is not a storage or search-index replacement.
- Retain raw `metadata.module-role.*` selector translation in `context-resolver-core`; borrowed catalogs consume the existing translated domain records rather than owning that mapping.
- Deliver BSL and SDBL catalog exposure as independently gated implementation commits under the same accepted change, so one domain can be verified without silently broadening or blocking the other.
- Reconcile ADR-0008 and related documentation so the borrowed catalog boundary and the retained generic resolver boundary describe distinct consumers and responsibilities.

## Capabilities

### New Capabilities

- `borrowed-bsl-context-catalog`: `context-resolver-search` exposes `HbkBslContextCatalog`, an `Arc` handle over the existing HBK snapshot/read handle returning borrowed records for generated self, owner members/callables, module members/events, globals, and typed availability.
- `borrowed-sdbl-query-catalog`: `context-resolver-search` exposes `HbkSdblQueryCatalog`, an `Arc` handle over the existing HBK snapshot/read handle returning borrowed query-source classification records.

### Modified Capabilities

None.

## Impact

- Affected upstream provider: `v8-context-hbk`.
- Affected analyzer integration: analyzer consumers can use `HbkBslContextCatalog` and `HbkSdblQueryCatalog` instead of generic owned DTO materialization on hot paths.
- Public boundary impact: `context-resolver-search` becomes the catalog API owner over `syntax-helper-search` snapshot arenas/read handles; `syntax-helper-search` receives only the narrow lifetime receiver refinement required for borrowed records.
- Storage impact: reuse existing HBK snapshot arenas; add no materialization change, new arena, cache, DTO mirror, analyzer shim, or SQLite fallback.
- Materialization impact: existing snapshot materialization is sufficient for the complete BSL capability and SDBL classification capability in this proposal.
- Search impact: keep SQL-backed search and direct `SearchIndex` flows explicit and separate from borrowed catalogs.
- Generic resolver impact: keep `ContextResolver` for source-neutral and diagnostic/debug consumers with a distinct contract from analyzer hot-path catalog reads.
- Mapping impact: keep raw `metadata.module-role.*` selector translation in `context-resolver-core`; no catalog-local mapping table is added.
- Projection impact: keep generic DTOs at concrete consumer boundaries, but
  give the existing stable identity/type-reference/signature conversion one
  reusable upstream owner so analyzer consumers do not copy it.
- Boundary impact: BSL generated-self, owner member/callable, module member/event, global, availability facts and SDBL query-source classification remain upstream HBK responsibilities.
- Delivery impact: BSL and SDBL exposure are independently gated commits under this change.
- Documentation impact: ADR-0008 and related docs must be reconciled with the borrowed catalog API and the retained `ContextResolver` use case.
