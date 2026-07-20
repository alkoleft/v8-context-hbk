## 1. Contract

- [x] 1.1 Record and consume the first companion metadata module-selector
  corpus: `common`, `command`, `object`, `manager`, `form`, `record_set`.
  Record normal absence for all other module roles and defer metadata-member
  selectors until an accepted HBK member result/status matrix exists.
- [x] 1.2 Add source/domain-qualified resolver query variants without exposing
  `ModuleContextKind` to this bridge's caller. The new public trait method
  has a default `Unsupported` implementation; composite dispatch must follow
  the recorded source/domain/capability status matrix.

## 2. Implementation

- [x] 2.1 Implement the private selector-to-existing-`ModuleContextKind`
  dispatch only in `context-resolver-core` composite resolution. Platform
  search and snapshot adapters remain selector-blind and are exercised only
  through their existing `module_context` query; add no selector literals,
  index or mapping there.
- [x] 2.2 Preserve explicit `NotFound`, `Unsupported`, `Ambiguous` and
  `ResolveError` outcomes with no fallback path.

## 3. Verification

- [x] 3.1 Add public resolver tests for corpus, source/domain isolation,
  unknown, unsupported, ambiguity and provider failure for the module selector.
- [x] 3.2 Run focused tests, formatting and strict OpenSpec validation.
