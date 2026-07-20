## Why

`context-provider` must resolve BSL module context without mapping metadata
kinds/module kinds to HBK `ModuleContextKind`. The existing generated-self
selector resolves a platform type template only; it deliberately says nothing
about module context.

## What Changes

- Accept one borrowed opaque metadata module-role selector in an HBK public,
  source/domain-qualified module-context lookup operation.
- Interpret that selector only inside HBK into existing module-context facts.
- Preserve `NotFound`, `Unsupported`, `Ambiguous` and `ResolveError` without
  exact-name, alias or cross-source fallback.

## Non-Goals

- No metadata crate dependency or copy of its enums.
- No template-key API change, generated configuration type composition,
  analyzer DTO, cache/index mirror or SQLite contract.
- No analyzer-side mapping of metadata kinds or module kinds.
- No metadata-member selector lookup until an accepted HBK role-to-BSL
  result/status matrix and a real consumer require it.
