## Why

The public resolver exposes HBK template facts by caller-supplied template key,
but a downstream project context resolver receives only a metadata-provider
certified generated-self role. The downstream analyzer currently has a legacy
role-to-template table; reusing or copying it would create a second owner for
HBK platform-template semantics.

## What Changes

- Add a source/domain-qualified public resolver lookup from a stable opaque
  generated-self role selector to an existing HBK platform-template type.
- Keep the selector owned and documented by the metadata provider; HBK owns the
  selector-to-template interpretation and all returned template/member facts.
- Preserve normal resolver response semantics: unknown selector/template is
  `NotFound`, sources without this capability are `Unsupported`, and provider
  failures remain `ResolveError`.
- Do not expose SQLite/schema internals, add a metadata dependency, create a
  shared analyzer DTO, or publish a template key to the downstream context
  resolver for this operation.

## Capabilities

### New Capabilities

- `generated-self-template-lookup`: Resolves a metadata-provider-certified
  generated-self role selector to source-backed HBK platform template facts.

### Modified Capabilities

- None.

## Impact

- `context-resolver-core`: one source-neutral borrowed type-lookup variant.
- `context-resolver-search`: platform source adapters and their existing
  provider-owned template indexes.
- No new runtime dependency, storage format, cache, analyzer mapping or
  metadata-provider dependency.
