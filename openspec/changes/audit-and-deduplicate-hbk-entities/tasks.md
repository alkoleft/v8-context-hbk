## 1. Identity And Projection Audit

- [ ] 1.1 Freeze the accepted HBK corpus, snapshot/cache format versions and deterministic read-only evidence commands; reproduce the known baseline of 6,670 type methods, 642 type events and 500 global methods with equal source keys across their parallel source rows.
- [ ] 1.2 Inventory every platform type, type member, callable, global fact, enum/value, query fact and language fact across provider rows, H0 arenas, X1 sections, typed locators, `StringId` source keys, `HbkFactRef` relations, exact/name/owner/kind indexes and borrowed views.
- [ ] 1.3 Inventory all in-repository and external consumers of concrete `Hbk*Id`, `HbkFactRef`, `StringId`, paired HBK views and lookup-to-view conversions; record which values escape a borrowed operation or require follow-up lookup.
- [ ] 1.4 Publish the candidate ledger classifying each repeated shape as a true duplicate semantic owner, legitimate index/view projection, or distinct entity with a named invariant; do not merge unresolved candidates by name, source order or field similarity.
- [ ] 1.5 For every proven duplicate family, select the narrowest existing canonical HBK owner and enumerate the exact records, indexes, conversions, X1 sections and consumer fields to delete; update proposal/design/spec deltas before implementation if the selected contract differs from this change.
- [ ] 1.6 Add deterministic preservation fixtures for every accepted family covering exact-ID, primary/alias, owner/kind, ordering, relations, availability, provenance, semantic roles and H0/X1 parity before deleting production paths.

## 2. Resource Baselines And Implementation Gate

- [ ] 2.1 Capture release-mode H0 materialization time, retained/payload bytes, allocation volume, representative lookup latency, X1 build/open latency and mapped artifact size for the accepted corpus and focused fixtures.
- [ ] 2.2 Complete the task-local implementation plan and reconcile its `Structure impact` with the full audit, including every reused/deleted structure, data-acquisition path, mapping, conversion, serializer and public re-export.
- [ ] 2.3 Send the task-local plan, `Structure impact` and `Reintroduction guard` through the required skeptic-review subagent and pre-implementation `mattpocock-skills:codebase-design` pass; record resolutions and PASS in `design.md` before editing implementation files.

## 3. Known Callable Duplicate Slice

- [ ] 3.1 Prove or reject `HbkCallable`/`HbkCallableId` as the existing canonical owner for type methods, type events and global methods, including every owner/kind/provenance fact currently supplied by member/global projections; stop and revise the design rather than adding a new arena if the proof fails.
- [ ] 3.2 For the accepted callable families, migrate H0 materialization, exact/name/owner/kind indexes, relations and borrowed views to the canonical owner and delete the parallel semantic record/identity path without adding a cross-map or compatibility locator.
- [ ] 3.3 Update X1 sections, codec, validation, cache identity/versioning and mapped views for the accepted removal, deleting obsolete physical records while preserving owned/mapped behavior and safe cache invalidation.
- [ ] 3.4 Migrate every inventoried in-repository and external consumer directly to the canonical HBK lookup/view operation; remove paired member-plus-callable and global-plus-callable fields, conversions and fallback reconstruction in the same slice.
- [ ] 3.5 Add the named structural-absence guard for each removed callable duplicate path and run the focused preservation, malformed-input, H0/X1 parity and external-boundary tests.
- [ ] 3.6 Repeat the release measurements from task 2.1, explain every material regression and retain the slice only when behavior, ownership and resource results satisfy the accepted design.

## 4. Remaining Proven Duplicate Families

- [ ] 4.1 Convert every additional proven candidate from the ledger into a named bounded implementation slice with its own preservation fixtures, task-local `Structure impact`, skeptic approval and codebase-design PASS; leave legitimate and unresolved projections unchanged.
- [ ] 4.2 For each accepted additional slice, migrate the canonical owner, H0/X1 storage and indexes, relations/views and all consumers atomically; delete its duplicate identity/payload/conversion path and add a family-specific reintroduction guard.
- [ ] 4.3 Record focused and release resource evidence for each additional slice and update the candidate ledger with the final retained/deleted decision.

## 5. Completion And Archival

- [ ] 5.1 Run format, focused tests, full workspace tests, strict Clippy, X1 validation/parity, deterministic corpus audit and release measurement commands on the final repository state.
- [ ] 5.2 Inspect the complete production/test/fixture/schema/probe diff for repeated semantic shapes, mapping chains, readers, indexes and compatibility surfaces; reconcile every addition/deletion against `Structure impact` and the candidate ledger.
- [ ] 5.3 Run a fresh reviewer subagent and the actual-diff `mattpocock-skills:codebase-design` pass, resolve actionable findings, record PASS in `design.md` and rerun affected verification.
- [ ] 5.4 Update architecture documentation if responsibilities, snapshot layout or public provider boundaries changed; otherwise record the evidence-backed no-update decision, then apply the required patch workspace version bump.
- [ ] 5.5 Complete measurement/design/task evidence, run strict change and canonical OpenSpec validation, archive and synchronize the capability specs, inspect staged scope and create the required task-scoped Conventional Commit.
