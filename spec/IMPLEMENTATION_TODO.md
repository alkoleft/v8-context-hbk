# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history:

- [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md)
- [archive/completed-tasks-t13-t17-t19-t24.md](archive/completed-tasks-t13-t17-t19-t24.md)
- [archive/completed-tasks-t25-t34.md](archive/completed-tasks-t25-t34.md)
- [archive/implementation-todo-2026-05-04.md](archive/implementation-todo-2026-05-04.md)
- [archive/implementation-todo-2026-05-05.md](archive/implementation-todo-2026-05-05.md)
- [archive/completed-tasks-t41-t47.md](archive/completed-tasks-t41-t47.md)
- [archive/completed-tasks-t48-t56.md](archive/completed-tasks-t48-t56.md)
- [archive/completed-tasks-t57-t65-t68-t85.md](archive/completed-tasks-t57-t65-t68-t85.md)
- [archive/completed-tasks-t66-t67-t86-t90.md](archive/completed-tasks-t66-t67-t86-t90.md)
- [archive/completed-tasks-t91-t110.md](archive/completed-tasks-t91-t110.md)

Current status: T35-T110 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design,
language-domain, cleanup, book-content export and documentation-site conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`requirements/non-functional.md`, `implementation/components.md`,
`implementation/documentation-site.md`, `implementation/syntax-helper-query-cli.md`,
`implementation/syntax-bsl-provider-plan.md`, `implementation/solution-context-resolve.md` and
`decisions/ADR-0010-documentation-site-generator-and-web-app.md`.

Current first unchecked task: none.
T111-T113 are user-requested documentation site generator/web-app slices that continue the
book-content export direction from T99-T109.

## Loop Rule

- Take the first unchecked task.
- If there is no unchecked task, add one before implementing new scope.
- Every new task must reference the relevant requirement, UAT, acceptance, implementation spec or
  ADR IDs from `spec/`.
- Keep scope limited to that task and its direct verification.
- Mark a task complete only after its listed verification passes.
- If a task cannot be completed, leave it unchecked and record the blocker in the task notes or final
  response.
- Do not start the next task in the same run unless the prompt explicitly asks for multiple tasks.
- Before committing, stage only files changed for the current task and verify
  `git diff --cached --name-only`.
- Do not create empty commits.

## Active Tasks

### [x] T111. Add `hbk-doc-site` generator crate boundary and global TOC merge model

Spec refs:

- UC-HBK-004
- FR-HBK-005
- NFR-SITE-001
- ADR-0010
- `spec/implementation/documentation-site.md`
- `spec/implementation/components.md`

Scope:

- Add a separate `hbk-doc-site` crate with typed generator request, result, error, book id, page id
  and TOC node models for documentation-site data generation.
- Implement source book discovery from an explicit list or source directory include filter.
- Implement deterministic locale grouping and global TOC merge for small multi-book fixtures.
- Preserve source book identity for page-bearing nodes and avoid silently collapsing same-title
  page-bearing nodes.
- Write the first `manifest.json`, `toc-root.json` and chunked child-section JSON artifacts for a
  fixture corpus.
- Keep page Markdown writing, web bundle generation, search, README command documentation and
  performance optimization out of this task.

Verification:

- Focused `hbk-doc-site` tests for deterministic merge, duplicate titles, source book provenance
  and fixture-level manifest/TOC JSON shape.
- `cargo test -p hbk-doc-site` passed on 2026-05-07.
- `cargo test --workspace` passed on 2026-05-07.

Completion notes:

- Added the `hbk-doc-site` crate with typed generator request/result/error, book id, page id and
  TOC node id models.
- Implemented deterministic explicit-file and source-directory discovery, safe locale artifact path
  validation, locale grouping, global TOC section merge and generated `data/manifest.json`,
  `toc-root.json` and `toc-sections/*.json` artifacts for fixture corpora.
- Page Markdown writing, CLI `site generate` wiring, real-corpus measurements and web app behavior
  remain T112/T113 scope.

### [x] T112. Generate page data and wire `site generate` CLI

Spec refs:

- UC-HBK-004
- FR-HBK-005
- FR-HBK-004
- FR-CLI-001
- NFR-SITE-001
- ADR-0010
- UAT-HBK-014
- `spec/acceptance/baseline.md`
- `spec/implementation/documentation-site.md`

Scope:

- Extend `hbk-doc-site` to write page Markdown files for page-bearing global TOC nodes.
- Reuse accepted single-book Markdown conversion behavior where possible, while keeping global
  page ids and cross-book link planning inside the site component.
- Wire `v8-context-hbk site generate <source-dir> --output <data-dir>` through the CLI with stable
  readable diagnostics for missing source directory, empty corpus and unsupported input.
- Support repeated `--include <file-name>` filters for deterministic small-corpus site UAT runs.
- Do not emit a web app bundle in this task; the separate web app belongs to T113.
- Record command summary data: source book count, generated page count, output size, build time and
  peak RSS or equivalent.
- Update `spec/acceptance/baseline.md` with the first real site generation measurements or a concrete
  skip reason if the local HBK corpus is unavailable.
- Keep search and semantic indexing out of this task.

Verification:

- Focused `hbk-doc-site` tests for page artifact planning and error diagnostics.
- Focused `v8-context-hbk-cli` tests for command parsing and diagnostics.
- UAT-HBK-014 passed on 2026-05-07 against the local 8.5.1.1150 corpus.
- `spec/acceptance/baseline.md` records the first site generation measurement.
- `cargo test -p hbk-book-export` passed on 2026-05-07.
- `cargo test -p hbk-doc-site` passed on 2026-05-07.
- `cargo test -p v8-context-hbk-cli` passed on 2026-05-07.
- `cargo test --workspace` passed on 2026-05-07.

Completion notes:

- `hbk-doc-site` now writes page Markdown files under
  `data/locales/<locale>/pages/<page-id>.md` for page-bearing global TOC nodes.
- Page generation reuses the accepted single-book Markdown conversion path from
  `hbk-book-export`, while `hbk-doc-site` owns global page ids, page file planning and artifact
  layout, including generated page-id link targets for internal Markdown links and fragments.
- Added `v8-context-hbk site generate <source-dir> --output <data-dir> [--include <file-name>]...`
  with readable source-directory/corpus diagnostics and summary data: source books, locales, TOC
  nodes, pages, files, output bytes, elapsed milliseconds and peak RSS when available.
- UAT-HBK-014 generated 4 source books, 1 locale, 267 TOC nodes, 254 page Markdown files,
  302 generated files, 931369 bytes, 3281 ms and 11632 KiB peak RSS for the representative local
  corpus.
- The separate documentation web app remains T113 scope.

### [x] T113. Add minimal documentation web app

Spec refs:

- UC-HBK-004
- FR-HBK-005
- NFR-SITE-001
- ADR-0010
- UAT-HBK-015
- `spec/implementation/documentation-site.md`

Scope:

- Add a minimal separate web application that serves/loads generated `data/` artifacts.
- Render locale selection, root global TOC, lazy child-section loading and page Markdown viewing.
- Ensure the initial web bundle does not embed generated page Markdown.
- Keep search, Syntax Assistant API, indexing status, `hbk-reader` route compatibility and frontend
  redesign beyond the first usable viewer out of scope.
- Verify desktop and mobile viewport basics for navigation and page readability.

Verification:

- Frontend unit tests for manifest/TOC/page data loading where the chosen web stack supports them.
- Production web build command for the chosen web app passed on 2026-05-07.
- UAT-HBK-015 passed on 2026-05-07 against the representative T112 generated data.
- Browser smoke over a generated fixture site verified manifest, TOC section and page assets are
  requested separately.

Completion notes:

- Added `web/docs-viewer`, a dependency-free Node/static documentation web app that serves generated
  `data/` artifacts and renders locale selection, root global TOC, lazy child sections and page
  Markdown.
- The server validates `--data`, confines `/data/*` paths to the generated data root and never
  parses HBK files or runs extraction in request paths.
- The initial app bundle contains viewer code only; generated Markdown pages are loaded on demand.
- Focused Node tests cover CLI argument parsing, invalid data roots, static/data 404 behavior, path
  traversal confinement and safe Markdown rendering for HTML-like input.

### [x] T114. Add visible progress for long-running documentation site generation

Spec refs:

- FR-HBK-005
- NFR-SITE-001
- UAT-HBK-014
- `spec/implementation/documentation-site.md`

Scope:

- Add a narrow progress-reporting path for `v8-context-hbk site generate` so users can see long
  source discovery, book loading, site planning and artifact writing stages.
- Keep progress on `stderr`; keep the final summary on `stdout` with the existing T112 keys.
- Keep progress coarse and deterministic. Do not add worker pools, caches, tuning knobs, search
  endpoints or broad performance refactors in this task.

Verification:

- Focused `hbk-doc-site` test captures progress events for a small fixture generation.
- Focused `v8-context-hbk-cli` test covers coarse page progress throttling.
- UAT-HBK-014 progress check passed on 2026-05-07 against the representative local 8.5.1.1150
  corpus.
- `cargo test -p hbk-doc-site` passed on 2026-05-07.
- `cargo test -p v8-context-hbk-cli` passed on 2026-05-07.
- `cargo test --workspace` passed on 2026-05-07.

Completion notes:

- Added `SiteGenerationProgress`, `GeneratedSiteFileKind` and
  `DocSiteGenerator::generate_with_progress` while preserving the no-progress
  `DocSiteGenerator::generate` wrapper for library callers.
- `v8-context-hbk site generate` writes progress to `stderr`: source discovery, source-book
  loading, loaded book count, planned locale/TOC/page counts and artifact writing milestones.
- The final T112 stdout summary shape is unchanged. Artifact progress is throttled to coarse
  milestones so full-corpus generation does not print one line per page or TOC section.

### [x] T115. Remove avoidable repeated work from documentation site page generation

Spec refs:

- FR-HBK-005
- NFR-SITE-001
- UAT-HBK-014
- `spec/acceptance/baseline.md`
- `spec/implementation/documentation-site.md`

Scope:

- Precompute locale-level Markdown link targets once per generated locale instead of rebuilding
  them for every generated page.
- Reuse a per-book Markdown page loader while writing site pages so `site generate` does not reopen
  the same book `FileStorage`/ZIP reader for every page.
- Keep generated data shape, page ids, Markdown link rewriting behavior and final stdout summary
  keys unchanged.
- Keep the change sequential and deterministic. Do not add worker pools, caches, tuning knobs,
  search endpoints or broad parser refactors.
- Keep progress output coarse for both TOC section and page artifact writing.

Verification:

- Focused `hbk-doc-site` tests cover same-book and cross-book site Markdown links after link target
  precomputation.
- Focused `v8-context-hbk-cli` tests cover coarse progress throttling for repeated artifact kinds.
- Representative release-profile UAT-HBK-014 measurement is updated in
  `spec/acceptance/baseline.md`.
- `cargo test -p hbk-docs` passed on 2026-05-07.
- `cargo test -p hbk-book-export` passed on 2026-05-07.
- `cargo test -p hbk-doc-site` passed on 2026-05-07.
- `cargo test -p v8-context-hbk-cli` passed on 2026-05-07.

Completion notes:

- `hbk-doc-site` now precomputes locale-level Markdown link targets once per locale and applies
  current-book scoping for unprefixed page links.
- `site generate` reuses one Markdown page loader per source book while writing page Markdown, so
  the same book `FileStorage`/ZIP reader is not reopened for every page.
- `hbk-docs` now builds a TOC HTML-path index once per `DocumentationPageLoader`, avoiding repeated
  `flat_pages()` expansion for every page and link in large books.
- CLI artifact progress remains deterministic but is throttled as bounded sparse milestones over the
  total generated file count.

### [x] T116. Simplify documentation site generation progress output

Spec refs:

- FR-HBK-005
- NFR-SITE-001
- UAT-HBK-014
- `spec/acceptance/baseline.md`
- `spec/implementation/documentation-site.md`

Scope:

- Reduce noisy `v8-context-hbk site generate` progress output while keeping progress on `stderr`
  and the final T112 `stdout` summary unchanged.
- Keep generator progress events available for library callers; simplify only the CLI rendering.
- Do not add quiet/verbose flags, worker pools, tuning knobs or generated data shape changes.

Verification:

- Focused `v8-context-hbk-cli` test covers sparse artifact progress milestones.

Completion notes:

- CLI progress no longer prints source HBK paths, generated artifact paths or artifact-kind labels.
- Interactive terminal progress updates one `stderr` line in place, shows the latest
  source/artifact file name and throttles file-level redraws to avoid terminal flicker.
- Redirected source loading is rendered as bounded sparse `<current>/<total>` milestones instead of
  one visible line per source book.
- Redirected artifact writing is rendered as bounded sparse `<current>/<total>` milestones over the
  total generated file count so large corpora still update regularly after the first artifact.

### [x] T117. Reduce repeated I/O and parsing in documentation site generation

Spec refs:

- FR-HBK-005
- NFR-SITE-001
- UAT-HBK-014
- `spec/acceptance/baseline.md`
- `spec/implementation/documentation-site.md`

Scope:

- Add a site-generation Markdown path that reads raw page HTML through the existing per-book page
  loader and avoids building full documentation `PageContent`/link diagnostics for every generated
  page.
- Avoid reading `FileStorage` during `HbkBook::open` when `PackBlock` already provides TOC data.
- Avoid a filesystem metadata call after every generated JSON/Markdown file write when the byte
  count is already known.
- Preserve generated data shape, page ids, Markdown link rewriting behavior, heading-only fallback
  behavior and final stdout summary keys.
- Keep the change sequential and deterministic. Do not add worker pools, caches, tuning knobs,
  search endpoints or broad parser refactors.
- Re-measure representative and full-corpus release-profile `site generate` runs after the change.

Verification:

- Focused `hbk-book-export` tests cover raw site Markdown link rewriting and missing-page fallback.
- Focused `hbk-doc-site` tests continue to cover same-book and cross-book generated Markdown links.
- Representative and full-corpus release-profile UAT-HBK-014 measurements are updated in
  `spec/acceptance/baseline.md`.
- `cargo test -p hbk-book -p hbk-book-export -p hbk-doc-site` passed on 2026-05-07.
- `cargo test --workspace` passed on 2026-05-07.

Completion notes:

- `BookMarkdownPageLoader::linked_markdown_toc_page` now uses raw page HTML for the site-generation
  path, preserving link rewriting, heading-only fallback and HTML title behavior without building
  full `PageContent` diagnostics for every page.
- `HbkBook::open` no longer reads the full `FileStorage` entity when `PackBlock` is available; the
  storage fallback remains for books without a `PackBlock` body.
- Generated JSON/text writers now use known serialized/text byte lengths instead of `fs::metadata`
  after each file write.
- Release-profile UAT-HBK-014 on the representative four-book corpus produced 4 source books,
  1 locale, 267 TOC nodes, 254 pages, 302 files, 931369 bytes, 122 ms and 7252 KiB peak RSS.
- The diagnostic full-corpus release run against all 116 local 8.5.1.1150 HBK files produced
  3 locales, 60686 TOC nodes, 54849 pages, 66730 files, 82233487 bytes, 18293 ms and
  222896 KiB peak RSS.

### [x] T118. Merge same-address site TOC nodes

Spec refs:

- FR-HBK-005
- UAT-HBK-014
- ADR-0010
- `spec/implementation/documentation-site.md`

Scope:

- Merge same-level page-bearing documentation-site TOC nodes when they point to the same normalized
  page address instead of leaving duplicate entries solely because they came from different TOC
  nodes or source books.
- Write one generated page data file for a merged same-address page and keep Markdown link aliases
  from all duplicate source books resolving to that page file.
- Keep same-title pages with different addresses distinct.
- Keep the merge independent from HTML page titles because some source pages expose generic or
  unreliable titles; visible navigation labels stay TOC-derived.
- Do not add search, route compatibility, web-app link handling, new web dependencies or broad
  frontend redesign.

Verification:

- Focused `hbk-doc-site` tests cover same-address page-bearing TOC merge and unique generated page
  output.
- Full-corpus UAT-HBK-014 diagnostic generation confirms real duplicate root entries merge by
  address.
- `cargo fmt --all --check` passed on 2026-05-07.
- `cargo test -p hbk-doc-site` passed on 2026-05-07.
- `cargo test --workspace` passed on 2026-05-07.

Completion notes:

- `hbk-doc-site` now merges same-level page-bearing TOC nodes by normalized page address and writes
  one page data file for the merged target.
- Source-book aliases from duplicate page targets are registered for Markdown link resolution.
- Page ids are opaque locale/address ids and do not depend on TOC path or title text.
- A full-corpus check against all 116 local 8.5.1.1150 HBK files confirmed that duplicate
  `form_plannerdimensionsdlg` root entries merge while distinct `1С:Предприятие` root targets
  remain separate.

### [x] T119. Handle generated section links in the documentation web viewer

Spec refs:

- FR-HBK-005
- UAT-HBK-015
- ADR-0010
- `spec/implementation/documentation-site.md`

Scope:

- Render generated service anchors such as `<a id="..."></a>` as invisible DOM anchors in the web
  app instead of visible raw text.
- Intercept internal generated page links such as `<page-id>.md#fragment` so page and section links
  open inside the web app.
- Do not add search, route compatibility, new web dependencies or broad frontend redesign.

Verification:

- Focused `web/docs-viewer` tests cover generated service anchors and internal page-fragment link
  handling.
- `npm --prefix web/docs-viewer test` passed on 2026-05-07.
- `npm --prefix web/docs-viewer run build` passed on 2026-05-07.

Completion notes:

- The viewer renders generated `<a id="..."></a>` anchors as invisible DOM anchors.
- Internal generated page links such as `<page-id>.md#fragment` are parsed and opened through the
  viewer instead of navigating to raw Markdown files.
- The Markdown renderer preserves generated `page-*.md` hrefs so the viewer click handler can route
  page-to-page links; this mirrors the `hbk-reader` approach where content-area clicks are
  intercepted and resolved by application navigation instead of browser file navigation.

### [x] T120. Resolve placeholder page branches to concrete site page targets

Spec refs:

- FR-HBK-005
- UAT-HBK-014
- ADR-0010
- `spec/implementation/documentation-site.md`

Scope:

- Detect same-level TOC branches where one source book uses a `_CONTENTS_NODE_*` placeholder page
  address and exactly one equivalent branch from another source book has a concrete page address.
- Merge the placeholder branch into the concrete generated page target instead of leaving an empty
  placeholder page next to the real page.
- Register the placeholder `source book + html path` pair as a Markdown link alias for the concrete
  generated page.
- Keep ambiguous placeholder branches with multiple concrete candidates on the existing
  placeholder-address behavior; do not choose hidden winners.
- Keep ordinary same-address page merge and generated section-link handling unchanged.

Verification:

- Focused `hbk-doc-site` tests cover placeholder-to-concrete target resolution and link aliasing.
- Focused `hbk-doc-site` tests cover ambiguous placeholder branches without choosing a hidden
  concrete target.
- Full-corpus UAT-HBK-014 diagnostic generation confirms real root placeholder/concrete branches
  merge where the concrete target is unambiguous.
- `cargo fmt --all --check` passed on 2026-05-07.
- `cargo test -p hbk-doc-site` passed on 2026-05-07.
- `cargo test --workspace` passed on 2026-05-07.

Completion notes:

- `hbk-doc-site` now precomputes unambiguous concrete page targets by TOC label path before merging
  locale TOC nodes.
- `_CONTENTS_NODE_*` placeholder branches merge into a concrete page target only when there is
  exactly one equivalent concrete candidate.
- Placeholder source-book addresses are registered as link aliases for the concrete generated page.
- A full-corpus check against all 116 local 8.5.1.1150 HBK files confirmed that the Russian root
  `1С:Предприятие` branches reduce from three nodes to two after placeholder/concrete resolution,
  while `form_plannerdimensionsdlg` remains merged as one same-address node.

### [x] T121. Keep documentation-site blockquotes and tables readable

Spec refs:

- FR-HBK-004
- FR-HBK-005
- UAT-HBK-015
- ADR-0010
- `spec/implementation/documentation-site.md`

Scope:

- Ensure layout-only HTML blockquote/table diagrams from ordinary HBK pages export as readable
  quoted prose lines, not as raw quoted Markdown table scaffolding.
- Render generated Markdown blockquotes and GFM tables in `web/docs-viewer` as DOM nodes so page
  content does not show raw `> | ... |` markup.
- Preserve the existing safe renderer boundary: no raw HTML injection, no new web dependencies and
  no broad frontend redesign.
- Do not add search, route compatibility or Syntax Assistant API behavior.

Verification:

- Focused `hbk-book-export` regression covers the `1cv8_ru.hbk` launch-flow blockquote/table shape
  from `Запуск 1С:Предприятие 8 и параметры запуска`.
- Focused `web/docs-viewer` tests cover Markdown blockquotes, GFM tables and quoted GFM tables.
- Regenerate a representative site data slice and inspect that the reported launch page no longer
  contains raw `> |` quoted table markup.
- `cargo fmt --all --check` passed on 2026-05-07.
- `cargo test -p hbk-book-export` passed on 2026-05-07.
- `cargo test --workspace` passed on 2026-05-07.
- `npm --prefix web/docs-viewer test` passed on 2026-05-07.
- `npm --prefix web/docs-viewer run build` passed on 2026-05-07.

Completion notes:

- `hbk-book-export` now rewrites non-code blockquote/table launch-flow diagrams into quoted prose
  lines before `quick_html2md`, while leaving Courier code/query examples and linked blockquotes on
  their existing paths.
- The `1cv8_ru.hbk` `ZIF` page now exports the launch-flow diagram as:
  `> Программа запуска - 1CEStart`,
  `> Интерактивная программа запуска - 1Cv8s` and `> Клиентское приложение`.
- The representative `site generate --include 1cv8_ru.hbk` run produced
  1 source book, 1 locale, 397 TOC nodes, 365 pages, 410 files, 1127587 bytes, 1761 ms and
  14208 KiB peak RSS. The generated page `page-ru-c5a12eeae852efad.md` no longer contains raw
  quoted table scaffolding for the reported launch-flow block.

### [x] T122. Recover from Syntax Assistant index identity collisions found in 8.5.1

Spec refs:

- FR-SH-SEARCH-001
- NFR-QUERY-001
- UAT-SH-004
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Treat duplicate constructor source pages with the same final owner identity and signature-text
  document id as a documentation defect, not an index rebuild blocker.
- Keep one deterministic document for unresolved duplicate final search document ids, choosing the
  last extracted source record.
- Report each duplicate during `syntax index` rebuild on `stderr`.
- Preserve distinct same-primary system enums by using alias as the minimal semantic variant when
  aliases differ.
- Do not change SQLite schema, query command shapes or provider JSON schema.

Verification:

- Focused `syntax-helper-search` regressions cover duplicate final document ids, last-record
  selection and build warning.
- Focused `syntax-helper-search` regression covers same-primary system enum identities
  distinguished by alias and enum-value ownership through the final enum identity.
- `v8-context-hbk syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output
  target/repro-index.sqlite` completes and reports the duplicate constructor warning.
- SQLite check confirms the reported `МенеджерКриптографии` constructor id exists once and both
  `ИспользованиеТекущейСтроки` system enum aliases exist as separate documents.

### [x] T123. Fix query table member ids to use parent table identity

Spec refs:

- FR-SH-003
- FR-SH-SEARCH-001
- UAT-SH-004
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Fix query table field/parameter semantic owner paths so TOC traversal does not duplicate the
  table node when the member page is a descendant of its table page.
- Ensure search-index query table field/parameter document ids use the resolved final parent
  query-table id instead of falling back to generic titles such as `query_table:Основная таблица`.
- Keep fallback identities semantic-owner-path based when a parent query table record is missing.
- Do not change SQLite schema, query command shapes, provider JSON schema or duplicate-document
  warning behavior.

Verification:

- Focused `syntax-helper-extract` regression covers TOC-descended query table member owner paths.
- Focused `syntax-helper-search` regressions covered parent-table id use and the then-current
  semantic fallback for missing parent table records.
- `cargo test -p syntax-helper-extract -p syntax-helper-search` passed on 2026-05-08.
- `v8-context-hbk syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output
  target/query-table-id-regression.sqlite` completed on 2026-05-08.
- SQLite checks confirmed no query table member id uses `query_table:Основная таблица`, the
  reported `<Имя общего реквизита>` field is absent under that generic owner id, and
  `query_table:Задача` owns `query_table_field:query_table:Задача:<Имя общего реквизита>`.
- Superseded by T127 for missing child parent identity behavior: search/export no longer keep a
  consumer-side semantic fallback for child/member facts without `owner_identity`.

### [x] T124. Fix type event ids to use semantic event owner identity

Spec refs:

- FR-SH-003
- FR-SH-SEARCH-001
- UAT-SH-004
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Ensure search-index `type_event` document ids use the composed TOC-derived semantic owner instead
  of the generic event-group label such as `События` / `Events`.
- Keep the schema version 10 consumer export rule unchanged: `type-events.json` still exposes one
  composed `owner` string and no `owner_path`.
- Do not change SQLite schema, query command shapes, provider JSON schema or duplicate-document
  warning behavior.

Verification:

- Focused `syntax-helper-search` regression covers same-name type events under different semantic
  owners and rejects `type_event:owner:События:*` identity collapse.
- `cargo test -p syntax-helper-extract -p syntax-helper-search` passed on 2026-05-08.
- `cargo run -p v8-context-hbk-cli -- syntax index
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output
  target/type-event-owner-regression.sqlite` completed on 2026-05-08.
- SQLite checks confirmed `type_event:owner:События:ОбработкаВыбора` is absent, same-name
  `ОбработкаВыбора` type-event documents use composed semantic owners, and there are zero duplicate
  final `documents.id` groups.

### [x] T125. Centralize type-event owner identity projection in the domain model

Spec refs:

- FR-EXPORT-001
- FR-SH-SEARCH-001
- UAT-SH-004
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Move the accepted JSON-compatible type-event owner projection out of `hbk-syntax-export` and
  `syntax-helper-search` into `syntax-helper-model`.
- Keep `hbk-syntax-export` and `syntax-helper-search` on the same shared owner projection.
- Preserve the existing consumer JSON contract: no `owner_path`, `id`, `owner_ref` or owner-kind
  fields on type events.
- Keep final search document ids search-specific; only the semantic owner projection is shared in
  this task.
- Do not change SQLite schema, query command shapes, provider JSON schema or duplicate-document
  warning behavior.

Verification:

- Focused `syntax-helper-model` regression covers the JSON-compatible type-event owner projection.
- Focused `hbk-syntax-export` and `syntax-helper-search` regressions continue to pass using the
  shared projection.
- `cargo test -p syntax-helper-model -p hbk-syntax-export -p syntax-helper-search` passed on
  2026-05-08.

### [x] T126. Compute Syntax Assistant parent identities during read phase

Spec refs:

- ADR-0011
- FR-SH-003
- FR-SH-SEARCH-001
- FR-EXPORT-001
- UAT-SH-004
- UAT-SH-013
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Keep shared parent identity mechanics in `syntax-helper-model` for platform types, query tables,
  enums, TOC duplicate-marker cleanup and type-event owner projection.
- Fill `PlatformType.identity`, `QueryTable.identity` and `EnumDefinition.identity` in
  `syntax-helper-extract` during TOC-aware reading before records reach `SyntaxHelperSink`.
- Perform duplicate-aware parent identity normalization as a read-phase prepass over parent facts,
  not inside `syntax-helper-search`.
- At T126 time, update `syntax-helper-search` so member document ids prefer precomputed parent
  identity while still allowing synthetic/legacy fallback records without identity.
- Keep `hbk-syntax-export` on model-owned type-event owner projection and preserve schema version
  11 JSON shape.
- Superseded by T127 for child/member facts: missing child `owner_identity` is now an index-build
  error, not a search/export fallback.

Verification:

- Focused `syntax-helper-model` tests cover parent identity helpers and type-event owner
  projection.
- Focused `syntax-helper-extract` tests prove parent identities are filled by reading for platform
  types, query tables and enums.
- Focused `syntax-helper-search` regression proves member ids use precomputed parent identity.
- Existing `hbk-syntax-export` regressions prove type-events JSON still emits the same `owner`
  shape and no `owner_path`.
- `cargo fmt --all --check` passed on 2026-05-08.
- `cargo check -p syntax-helper-model -p syntax-helper-extract -p hbk-syntax-export -p
  syntax-helper-search` passed on 2026-05-08.
- `cargo test -p syntax-helper-model -p syntax-helper-extract -p hbk-syntax-export -p
  syntax-helper-search` passed on 2026-05-08.
- `cargo run -p v8-context-hbk-cli -- syntax index
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/adr-0011-identity.sqlite`
  completed on 2026-05-08.
- SQLite checks confirmed no `type_event:owner:События:ОбработкаВыбора`, no
  `query_table_field` / `query_table_parameter` document under `query_table:Основная таблица`, and
  zero duplicate final `documents.id` groups.

### [x] T127. Remove remaining consumer-side parent identity repair

Spec refs:

- ADR-0011
- FR-SH-003
- FR-SH-SEARCH-001
- FR-EXPORT-001
- UAT-SH-004
- UAT-SH-013
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Extend child domain records with precomputed parent identity where ADR-0011 requires it:
  platform type members/constructors, query table fields/parameters and enum values.
- Fill child parent identities in `syntax-helper-extract` during the TOC-aware read phase before
  records reach `SyntaxHelperSink`.
- Update `syntax-helper-search` and `hbk-syntax-export` to consume child parent identities instead
  of reconstructing owner identity from localized names, broad owner scans or generic TOC labels.
- Fix constructor relation construction to use the resolved parent identity, not `owner.primary`.
- Remove avoidable query-table page double parsing introduced by the parent-identity prepass.
- Preserve schema version 11 JSON shape, search-specific final document ids, duplicate final
  document-id warning behavior, CLI command shapes and SQLite schema version.

Verification:

- Focused `syntax-helper-model`, `syntax-helper-extract`, `hbk-syntax-export` and
  `syntax-helper-search` regressions cover child parent identity propagation and grouping.
- Focused extraction regression proves query-table pages are not loaded twice for identity prepass
  plus record emission.
- `cargo fmt --all` passed on 2026-05-08.
- `cargo check --workspace` passed on 2026-05-08.
- `cargo test --workspace` passed on 2026-05-08.
- `cargo run -p v8-context-hbk-cli -- syntax index
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t127-parent-identity.sqlite`
  completed on 2026-05-08 with existing `DUPLICATE_DOCUMENT_ID` warnings.
- `cargo run -p v8-context-hbk-cli -- syntax index
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output
  target/t127-parent-identity-root.sqlite` completed on 2026-05-08 with existing
  `DUPLICATE_DOCUMENT_ID` warnings.
- SQLite assertions over both T127 indexes confirmed zero final duplicate `documents.id` groups,
  zero query-table field/parameter documents under generic `query_table:Основная таблица` /
  `query_table:Main table`, zero constructor callables without `owner_type_id`, and zero
  constructor result type references without `target_type_id`.

### [x] T128. Specify dependency-based static-analysis integration surface

Spec refs:

- ADR-0001
- ADR-0007
- ADR-0008
- FR-SH-PROVIDER-001
- FR-CTX-RESOLVE-001
- UC-CTX-002
- NFR-RESOLVE-001
- `spec/implementation/components.md`
- `spec/implementation/solution-context-resolve.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Record that the static-analysis integration mode is Cargo dependency or workspace membership, not
  HTTP, daemon, MCP, CLI-spawn or JSON-over-process transport in the analyzer hot path.
- Keep ADR-0007 valid for language-agnostic CLI JSON provider consumers, while pointing Rust
  static-analysis consumers to ADR-0008.
- Define the dependency layers: `context-resolver-core` for source-neutral traits/DTOs,
  `context-resolver-search` for HBK-backed source adapters, `syntax-helper-search` for local index
  open/build primitives and HBK extraction crates only for setup/index-refresh.
- Keep SQLite tables, HBK paths, Syntax Assistant HTML parser internals, CLI wiring, export crates,
  documentation-site code and web-app code out of the static-analysis library contract.
- Do not add a new facade crate, transport, code implementation or compatibility layer in this
  spec-only task.

Verification:

- ADR-0008 records dependency-based static-analysis integration and rejects transport-oriented
  resolver APIs for the hot path.
- ADR-0001 and ADR-0007 clarify their relationship to the ADR-0008 Rust dependency boundary.
- `functional.md`, `non-functional.md`, `use-cases.md`, `components.md`,
  `solution-context-resolve.md` and `syntax-helper-query-cli.md` describe the selected integration
  surface consistently.
- No Rust code, CLI behavior, JSON schema, SQLite schema or public export shape changed in this
  task.

### [x] T129. Add adapter-level read-only index open constructors

Spec refs:

- ADR-0008
- FR-CTX-RESOLVE-001
- NFR-RESOLVE-001
- UC-CTX-002
- `spec/implementation/components.md`
- `spec/implementation/solution-context-resolve.md`

Scope:

- Add `context-resolver-search` constructors that open an existing read-only
  `syntax-helper-search` provider index and return the relevant source adapter.
- Cover platform source with default and explicit source id constructors.
- Cover language sources with generic and source-family constructors for `shlang`, `shquery` and
  `dcsui`.
- Keep `syntax-helper-search` as the owner of SQLite schema, index build/open mechanics and lower
  level `SearchIndex` APIs.
- Do not change SQLite schema, CLI behavior, provider JSON, export JSON or resolver fact shapes.

Verification:

- Focused `context-resolver-search` tests cover opening platform and language adapters from an
  index path and resolving a fact through the opened adapter.
- `cargo fmt --all --check` passes.
- `cargo test -p context-resolver-search` passes.

### [x] T130. Prove the static-analysis dependency surface with a consumer-style smoke

Spec refs:

- ADR-0008
- FR-CTX-RESOLVE-001
- UC-CTX-002
- NFR-RESOLVE-001
- `spec/implementation/components.md`
- `spec/implementation/solution-context-resolve.md`

Scope:

- Add a focused consumer-style integration smoke that models two explicit phases:
  provider setup/index refresh and analyzer lookup.
- In the setup phase, build a small deterministic provider index through the owning crates
  (`syntax-helper-search` plus existing fixture/domain builders; real HBK input is not required for
  this task).
- In the lookup phase, compose `context-resolver-core` and `context-resolver-search` only, opening
  the existing index through adapter-level constructors from T129.
- Assert the lookup phase resolves at least one platform type, one member/callable path and one
  language fact without importing `SearchIndex`, reading SQLite tables directly, spawning
  `v8-context-hbk` or parsing HBK/HTML.
- Keep the smoke inside the existing workspace test structure unless a concrete downstream repo
  integration target is provided.
- Do not add a facade crate, transport, CLI command, SQLite schema change, provider JSON change or
  export JSON change.

Verification:

- Completed with `crates/context-resolver-search/tests/static_analysis_consumer_smoke.rs`.
- The smoke separates provider setup and analyzer lookup modules. Setup builds a deterministic
  provider index through `syntax-helper-search`; lookup composes only `context-resolver-core` and
  `context-resolver-search` adapter constructors and does not import
  `syntax-helper-search::SearchIndex`.
- `cargo fmt --all --check` passed.
- `cargo test -p context-resolver-search` passed.
- If the implementation adds a separate smoke crate or example package, its package-specific test
  command is recorded here when T130 is completed. No separate smoke crate or example package was
  added.

### [x] T131. Expose platform template aliases and metadata-template facts through the resolver

Spec refs:

- ADR-0008
- ADR-0011
- FR-CTX-RESOLVE-001
- UC-CTX-002
- NFR-RESOLVE-001
- `spec/implementation/solution-context-resolve.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Add a public resolver type lookup path for exact platform type aliases so downstream analyzer
  adapters can use English aliases without analyzer-owned localized name tables.
- Persist and expose platform metadata-template information already owned by Syntax Assistant
  extraction: metadata kind and template parameters for `PlatformTypeKind::MetadataTemplate`
  records.
- Keep SQLite table details internal to `syntax-helper-search`; expose the data through
  `context-resolver-core` / `context-resolver-search` DTOs.
- Do not implement downstream analyzer generic refinement, generated configuration type mapping,
  CLI behavior, transport layers, compatibility fallbacks or public SQLite schema commitments.

Verification:

- Focused `syntax-helper-search` test proves metadata-template information survives index build and
  read-only open.
- Focused `context-resolver-search` test proves exact alias type lookup and resolver DTO template
  metadata for a catalog metadata-template type.
- `cargo fmt --all --check` passes.
- `cargo test -p syntax-helper-search` passes.
- `cargo test -p context-resolver-search` passes.

Completed 2026-05-10:

- Added `TypeLookup::ExactAlias` and `TypeInfo.metadata_template` to the resolver core boundary.
- Persisted metadata-template facts in search index schema version 8 and hydrated them through
  resolver search DTOs.
- Added focused index/resolver tests plus CLI fixture updates for the expanded `SearchDocument`
  shape.
- Verified with `cargo fmt --all --check`, `cargo test -p syntax-helper-search`,
  `cargo test -p context-resolver-core`, `cargo test -p context-resolver-search` and
  `cargo test -p v8-context-hbk-cli`.
