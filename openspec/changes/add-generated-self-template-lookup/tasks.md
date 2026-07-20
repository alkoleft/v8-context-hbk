## 1. Provider Contract

- [x] 1.1 Record the metadata selector corpus consumed by HBK and the explicit
  companion metadata-provider prerequisite; do not infer selector spelling
  from display/debug text.
- [x] 1.2 Add the borrowed source/domain-qualified generated-self template
  lookup variant to `context-resolver-core`, preserving existing resolver
  response and error contracts.

## 2. HBK Resolver Implementation

- [x] 2.1 Add provider-owned selector-to-classified-template resolution to the
  platform snapshot/search adapters using existing template evidence/indexes,
  without a metadata dependency, cache, public template key or analyzer model.
- [x] 2.2 Return `NotFound`, `Unsupported`, `Ambiguous` and `ResolveError` by
  the specified status matrix without exact-name/alias or cross-source fallback.

## 3. Contract Verification

- [x] 3.1 Add public resolver tests for every certified selector, identity and
  template evidence, requested source/domain isolation, unknown selector,
  unsupported source, ambiguity and provider-error propagation.
- [x] 3.2 Run focused resolver tests, formatting, strict OpenSpec validation
  and diff checks; update the HBK spec/task evidence.
