# Completed Implementation Tasks T111-T134

This archive preserves completed task history moved out of the active implementation ledger.
It is evidence, not active implementation scope.

Raw command logs, generated exports, temporary probes and `target/...` paths are service data and
are intentionally omitted from this archive. Durable documentation-site, resolver integration,
Syntax Assistant identity, type-template and provider-boundary conclusions live in
`../acceptance/baseline.md`, `../requirements/functional.md`,
`../requirements/non-functional.md`, `../implementation/components.md`,
`../implementation/documentation-site.md`, `../implementation/syntax-helper-query-cli.md`,
`../implementation/solution-context-resolve.md` and ADRs in `../decisions/`.

For active task sequencing, use `../IMPLEMENTATION_TODO.md`.

## Archived Task Range

- T111. Add `hbk-doc-site` generator crate boundary and global TOC merge model.
- T112. Generate page data and wire `site generate` CLI.
- T113. Add minimal documentation web app.
- T114. Add visible progress for long-running documentation site generation.
- T115. Remove avoidable repeated work from documentation site page generation.
- T116. Simplify documentation site generation progress output.
- T117. Reduce repeated I/O and parsing in documentation site generation.
- T118. Merge same-address site TOC nodes.
- T119. Handle generated section links in the documentation web viewer.
- T120. Resolve placeholder page branches to concrete site page targets.
- T121. Keep documentation-site blockquotes and tables readable.
- T122. Recover from Syntax Assistant index identity collisions found in 8.5.1.
- T123. Fix query table member ids to use parent table identity.
- T124. Fix type event ids to use semantic event owner identity.
- T125. Centralize type-event owner identity projection in the domain model.
- T126. Compute Syntax Assistant parent identities during read phase.
- T127. Remove remaining consumer-side parent identity repair.
- T128. Specify dependency-based static-analysis integration surface.
- T129. Add adapter-level read-only index open constructors.
- T130. Prove the static-analysis dependency surface with a consumer-style smoke.
- T131. Expose platform template aliases and metadata-template facts through the resolver.
- T132. Expose semantic generic platform template kinds and template bindings.
- T133. Replace type template semantic enums with data-driven families.
- T134. Normalize platform type template terminology.

## Durable Conclusions

- Documentation site generation is split into a generator utility and a separate static web app.
  The generator owns source discovery, locale grouping, global TOC merge, page artifact planning,
  Markdown page writing, coarse progress reporting and deterministic generated data artifacts.
- The documentation web viewer loads generated `data/` artifacts on demand, supports locale
  selection, lazy TOC section loading, page Markdown viewing, generated service anchors,
  internal page-fragment navigation, blockquotes and GFM tables without embedding generated page
  Markdown in the initial bundle.
- Site generation performance was improved by precomputing locale link targets, reusing per-book
  page loaders, avoiding unnecessary full `PageContent` construction and avoiding metadata calls
  after generated file writes, while preserving generated data shape and final summary keys.
- Documentation-site TOC merging now handles same-address page nodes and unambiguous placeholder
  branches without merging distinct same-title pages with different addresses.
- Syntax Assistant index identity work moved parent identity responsibility into the read/domain
  phase. Search/export consumers now consume domain-owned identities for platform type children,
  query table children, enum values and type events instead of repairing owners from localized
  names, broad scans or generic TOC labels.
- Static-analysis integration is a Rust dependency/workspace boundary over
  `context-resolver-core`, `context-resolver-search` and provider index open/build primitives, not
  an HTTP, daemon, MCP, CLI-spawn or JSON-over-process hot path.
- Resolver/search APIs expose read-only provider-index constructors and a consumer-style smoke
  proves analyzer lookup can use resolver adapters without importing `SearchIndex`, reading SQLite
  tables directly, spawning the CLI or parsing HBK/HTML.
- Platform type-template support moved from closed generic-template enums to open HBK-owned
  family/variant keys: `PlatformTypeTemplateKey`, `TypeLookup::PlatformTypeTemplate`,
  `type_template_by_key`, `TypeTemplateBinding`, `TemplateParameterBinding` and
  `template_binding`.
- Template classification and binding data remain HBK-owned provider facts. SQLite schema layout is
  private rebuildable provider state, while Rust resolver/search APIs are the accepted integration
  surface for downstream analyzer consumers.

## Verification Summary

- T111-T121 were verified with focused `hbk-doc-site`, `hbk-book-export`,
  `v8-context-hbk-cli` and `web/docs-viewer` tests, production web build checks, representative
  and full-corpus UAT-HBK-014/UAT-HBK-015 measurements where applicable, and workspace tests
  recorded in the active ledger before archival.
- T122-T127 were verified with focused `syntax-helper-model`, `syntax-helper-extract`,
  `syntax-helper-search`, `hbk-syntax-export`, `context-resolver-search` and CLI/indexing
  regressions, plus local 8.5.1.1150 corpus indexing evidence recorded in
  `../acceptance/baseline.md`.
- T128 was spec-only and changed no Rust code, CLI behavior, JSON schema, SQLite schema or public
  export shape.
- T129-T130 were verified with focused `context-resolver-search` tests and a consumer-style
  dependency-boundary smoke.
- T131-T134 were verified with focused model/search/resolver/extract/export/CLI tests,
  `cargo fmt --all --check`, workspace tests, clippy where recorded, and representative
  `shcntx_ru.hbk` / `shcntx_root.hbk` type-template corpus evidence recorded in
  `../acceptance/baseline.md`.
