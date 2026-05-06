# Documentation Site Generator and Web App Plan

Status: draft implementation plan accepted by ADR-0010.

## Problem

The current `markdown/toc` export writes readable Markdown for one HBK book. A full documentation
site needs two separate shapes:

- users need one global documentation navigation tree, not one disconnected tree per book;
- the site must remain usable on the full 1C documentation corpus;
- general-purpose static-site generators are a poor first fit because they attempt to generate or
  bundle page-level route state for every Markdown file;
- the desired viewer replaces the web/view part of `hbk-reader`, not the HBK reader/parsers already
  implemented in Rust.
- future Syntax Assistant search/API behavior should reuse generated/indexed artifacts, not force
  the documentation web app to parse HBK files in request paths.

## Users

- Documentation consumer: browses installed 1C help books through a local documentation web app.
- Parser maintainer: inspects exported pages and TOC structure without opening HBK files manually.
- Future syntax consumer: queries generated Syntax Assistant/search artifacts through the web app
  after the documentation-site slice is accepted.

## Product Requirements

- Build generated documentation-site data artifacts from a directory or explicit list of HBK books.
- Generate one global TOC per locale by merging the individual book TOC trees.
- Preserve source book identity and page provenance in generated manifest data.
- Render a documentation page from static page data by stable page id.
- Load TOC sections and page content lazily through the web app from generated files.
- Avoid embedding all page Markdown into the web bundle.
- Keep output deterministic for the same source book set.
- Keep the first slice independent from Syntax Assistant fact export, resolver APIs, search APIs and
  future `v8-context` integration.
- Leave room for a later generated Syntax Assistant index and web API, but do not implement it in
  the first site slice.

## Generated Data Artifact Shape

The generator should create a data output directory with this logical structure:

```text
generated/
  data/
    manifest.json
    locales/
      ru/
        toc-root.json
        toc-sections/
          <section-id>.json
        pages/
          <page-id>.md
```

The separate web application build/runtime owns `index.html`, assets, routing and serving. The
versioned generated data contract is the `data/` subtree.

### `manifest.json`

Required fields:

- artifact schema version;
- deterministic build id, or generation timestamp only when the caller accepts non-deterministic
  metadata;
- generator version;
- available locales;
- source book inventory per locale with book id, file name, title/name from metadata when available
  and file size or modified timestamp for diagnostics;
- root data paths for TOC and pages.

### TOC Nodes

TOC node fields:

- `id`: stable site node id;
- `title`: display title;
- `book_id`: source book id when the node has page content;
- `page_id`: page content id when the node has page content;
- `has_children`;
- `children_path` for lazy child loading when children are not inlined;
- optional source diagnostics fields such as source book name and normalized HTML path for generated
  data, not for user-facing labels.

The first implementation may inline a bounded root depth and use `children_path` for deeper
sections. It must not require loading the whole global tree before showing the first screen.

### Page Data

Page content files are Markdown in the first documentation slice, using the existing accepted
Markdown conversion rules from `FR-HBK-004`. The web app is responsible for rendering Markdown
safely. Page content paths are addressed by `page_id`, not by raw HBK HTML path.

### Syntax Index Data

Generated Syntax Assistant/search index artifacts are a later slice. The planned generator may
produce them from accepted Syntax Assistant export/search pipelines later, but the first
documentation-site slice must not require them.

## Global TOC Merge

The merge input is a list of `(book_id, locale, Toc)` values. The output is a locale-level
navigation tree.

Rules:

- Merge only books with the same export locale into one tree.
- Sort source books deterministically before merge.
- Preserve the source book id on every page-bearing node.
- Merge same-level section nodes by normalized display title only when both nodes are section-like
  or their content relationship is proven equivalent.
- Do not merge two page-bearing nodes solely because they have the same title.
- When duplicate output ids are needed, disambiguate by deterministic source book id and local TOC
  path data inside the generated id, not in the visible title.
- Record merge diagnostics for conflicts such as same title with different page content.

This deliberately tightens the `hbk-reader` reference behavior: title-based branch merge is useful,
but public identity must not depend on mutable duplicate counters.

## Implementation Boundaries

Preferred component split:

- `hbk-doc-site`: Rust library for generator request/result/error types, source book discovery,
  global TOC merge, generated site data artifact planning and page data writing.
- `v8-context-hbk-cli`: command wiring and readable diagnostics, for example
  `v8-context-hbk site generate <source-dir> --output <data-dir>`.
- `web/docs-viewer` or equivalent: separate web application that serves the documentation UI and
  generated `data/` artifacts. Later it may own search and Syntax Assistant API endpoints backed by
  generated/indexed data.

`hbk-doc-site` may reuse or extract shared Markdown conversion behavior from `hbk-book-export`, but
global site identity, merged TOC and generated data artifact planning belong to the generator
component. The web app must not parse HBK files or build the corpus in request handlers.

## First Slice

The first implementation slice should stop at a usable documentation site without syntax/search API:

1. Add `hbk-doc-site` request/result/error model and small fixture-backed merge tests. Completed in
   T111 with deterministic build id, source book size inventory and safe locale artifact path
   validation.
2. Generate `data/manifest.json`, root TOC and section TOC JSON for a small multi-book fixture.
   Completed in T111.
3. Write page Markdown files for page-bearing nodes. Completed in T112 with generated
   `data/locales/<locale>/pages/<page-id>.md` files and page ids owned by `hbk-doc-site`.
4. Add CLI wiring for `site generate`. Completed in T112 as
   `v8-context-hbk site generate <source-dir> --output <data-dir> [--include <file-name>]...`
   with readable diagnostics and summary measurements.
5. Add a minimal separate web app that can serve/load the generated manifest, locale root TOC, lazy
   section children and page Markdown. Completed in T113 as the dependency-free
   `web/docs-viewer` Node/static app.
6. Add coarse progress reporting for long-running `site generate` runs. Completed in T114 with a
   generator progress callback and CLI `stderr` progress for source discovery, book loading, site
   planning and artifact writing. The final `stdout` summary remains unchanged.
7. Remove avoidable repeated work in page generation. Completed in T115 with locale-level link
   target precomputation, per-book Markdown page loader reuse and per-loader TOC HTML-path indexes.

Search and Syntax Assistant API behavior are intentionally later slices. When added, they should use
generated local index artifacts or existing accepted local index contracts, not HBK parsing in web
request paths.

## Verification

- Unit tests cover deterministic global TOC merge and duplicate ids.
- CLI tests cover typed diagnostics for missing source directory and empty corpus.
- UAT-HBK-014 validates generated site data artifacts on representative real books.
- UAT-HBK-015 validates the separate web app serving/loading generated data and lazy navigation.
- Build measurements record source book count, generated page count, output size, build time and
  peak RSS or equivalent before optimization work.

## T113 Web Viewer Notes

The first documentation web app lives under `web/docs-viewer`.

- `npm --prefix web/docs-viewer run build` copies the static app assets into
  `web/docs-viewer/dist`.
- `npm --prefix web/docs-viewer start -- --data <absolute-data-dir> --listen 127.0.0.1:4173`
  serves the built app and generated `data/` subtree.
- The server validates `--data`, confines `/data/*` requests to that directory and does not parse
  HBK files in request paths.
- The browser app loads `manifest.json`, locale `toc-root.json`, lazy `toc-sections/*.json` and
  page Markdown files through separate fetch requests.
- The initial bundle contains only the viewer code and no generated page Markdown payload.
- Markdown rendering is intentionally small and safe for the first slice: headings, paragraphs,
  lists, fenced code blocks, strong text and links are rendered with DOM nodes rather than raw HTML
  injection.

Search, Syntax Assistant API endpoints, indexing status and `hbk-reader` route compatibility remain
outside this slice.

## T114 Site Generation Progress Notes

`hbk-doc-site` exposes `DocSiteGenerator::generate_with_progress` for callers that need
observability during long generation runs. `DocSiteGenerator::generate` remains the no-progress
library wrapper for existing callers.

The CLI uses the progress callback in `site generate` and writes progress lines to `stderr`.
Progress covers:

- discovered source book count;
- currently loading source book;
- loaded book count;
- planned locale, TOC-node and page counts;
- artifact writing progress for manifest, TOC roots, TOC sections and coarse page milestones.

The progress stream is intentionally coarse. Page artifact messages are throttled so a full corpus
does not produce one terminal line per generated page. The final summary stays on `stdout` with the
same keys as T112.

## T115 Site Generation Performance Notes

The page-generation hot path remains sequential and deterministic, but it no longer repeats
corpus-scale work for every page:

- locale Markdown link targets are computed once per generated locale;
- unprefixed links are resolved against the current source book, while explicit
  `v8help://<book>/...` links use the precomputed locale target map;
- one Markdown page loader is reused per source book during site generation, so `FileStorage`/ZIP
  readers are not reopened for every page;
- each documentation page loader builds a TOC HTML-path index once and reuses it for page provenance
  and link resolution;
- CLI artifact progress is throttled for both TOC section and page artifact families.

This is intentionally not a worker-pool or tuning-knob change. Later parallelization still requires
separate measurements and must preserve deterministic artifact order and diagnostics.
