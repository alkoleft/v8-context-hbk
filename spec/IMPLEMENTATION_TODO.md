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
