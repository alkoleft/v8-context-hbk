# Completed Implementation Tasks T135-T142

This archive preserves completed task history moved out of the active implementation ledger.
It is evidence, not active implementation scope.

Raw command logs, generated indexes, temporary probes and `target/...` paths are service data and
are intentionally omitted from this archive. Durable type-reference, template, provider graph and
resolver-domain conclusions live in `../acceptance/baseline.md`,
`../requirements/functional.md`, `../implementation/components.md`,
`../implementation/syntax-helper-query-cli.md`, `../implementation/solution-context-resolve.md`
and ADRs in `../decisions/`.

For active task sequencing, use `../IMPLEMENTATION_TODO.md`.

## Archived Task Range

- T135. Measure Syntax Assistant type-reference gaps.
- T136. Add type quality gates to the acceptance baseline.
- T137. Specify explicit type domain separation.
- T138. Decide the separate type crate boundary.
- T139. Split raw type references from resolved type targets.
- T140. Move return types toward overload-level facts.
- T141. Strengthen platform type template resolution.
- T142. Add a type graph query primitive.

## Durable Conclusions

- Type-reference quality is measured through deterministic `syntax type-ref-gaps` runs against
  prebuilt local indexes. Resolved, unresolved and ambiguous references are tracked separately, and
  template bindings are counted as a separate subset.
- T136 introduced acceptance gates for unresolved type-reference count, ambiguous type-reference
  count, classified metadata-template count, unclassified template diagnostics, template binding
  count and expression-chain provider scenarios. Gate updates require fresh corpus measurement and a
  task-owned rationale.
- Resolver and provider contracts are domain-aware. Same display names across platform API, BSL
  language, query language, configuration metadata and source-code domains do not imply identity
  without an explicit source-backed relation.
- A separate workspace crate for type identities, type-reference resolution DTOs and type-template
  binding DTOs is deferred. The current smallest ownership boundaries remain `syntax-helper-model`,
  `syntax-helper-search`, `context-resolver-core` and `context-resolver-search`.
- Private search-index schema version `13` stores source-backed type-reference spelling separately
  from target resolution outcome: `ok`, `unresolved` or `ambiguous`. Provider JSON keeps
  export-compatible `types` / `return` name arrays, while Rust resolver DTOs expose target outcome as
  data.
- Callable return facts now distinguish shared page-level returns from source-proven
  overload/signature-level returns. Provider JSON schema remains stable for existing consumers.
- Platform type-template resolution preserves owner-parameter bindings on member, callable return and
  parameter type references when HBK exposes template-to-template references.
- The first compact type-graph provider primitive is implemented as
  `syntax related --id <exact-provider-id> --graph`; it keeps graph metadata under `results[].meta`
  and reports unresolved/ambiguous type-reference diagnostics without changing shared fact fields.

## Verification Summary

- T135-T142 were verified with focused model/search/resolver/export/CLI tests, `cargo fmt --all
  --check`, `cargo test --workspace`, and representative `shcntx_ru.hbk` / `shcntx_root.hbk`
  measurement or UAT evidence recorded in `../acceptance/baseline.md`.
- T142 UAT-SH-024 passed on a fresh `shcntx_ru.hbk` index with `25415` documents, and the accepted
  graph query completed within the recorded NFR target.
