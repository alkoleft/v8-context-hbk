## Why

`context-provider` needs to resolve one metadata-module BSL member without
accepting a whole module context collection. The current metadata-selector
resolver operation returns a materialized vector, so filtering its output would
recreate a downstream effective-context path and hide equal-rank ambiguity.

## What Changes

- Add a source/domain-qualified exact point query for one metadata-module BSL
  member by canonical name and existing `MemberQueryKind`.
- Return a single HBK-owned member answer that preserves kind, callable
  signatures, fact identity and source evidence, or existing resolver
  `NotFound`, `Ambiguous`, `Unsupported` and provider-error outcomes.
- Keep opaque metadata module-role selector interpretation in HBK; do not
  expose `ModuleContextKind`, platform template keys, storage internals or an
  analyzer-specific DTO.
- Add source-indexed search and snapshot adapter support rather than deriving
  point results by building or filtering `ResolvedModuleContext` collections.

## Capabilities

### New Capabilities

- `exact-bsl-context-member-lookup`: Defines exact source-owned metadata-module
  BSL member resolution and its status/evidence contract.

### Modified Capabilities

- None.

## Impact

`context-resolver-core` gains provisional resolver query and answer types;
`context-resolver-search` exposes the required indexed platform source
operations. The change is an upstream prerequisite for the analyzer's active
`add-context-provider-boundary` change and does not introduce a metadata or
analyzer dependency.
