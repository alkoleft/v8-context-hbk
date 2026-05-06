# ADR-0010: Split Documentation Site Generation and Web Application

Date: 2026-05-06.

Status: Accepted as a specification and implementation direction.

## Context

`v8-context-hbk` already exports ordinary HBK book content as Markdown/TOC files. That export is
useful for review and indexing, but it is not enough for a complete documentation web site:

- the installed 1C documentation corpus contains enough pages that MkDocs and Docusaurus are not a
  reliable first implementation path;
- those generators try to turn every Markdown page into build-time site state, which makes the
  build sensitive to page count, disk usage and route generation overhead;
- the desired web view is closer to the existing `/home/alko/develop/open-source/hbk-reader` viewer:
  a global TOC formed by merging book TOC trees, lazy navigation and direct page viewing.

The project should replace `hbk-reader` only in the web/view responsibility. It must not reproduce
the Kotlin/Spring application shape, parser APIs or fallback-heavy runtime behavior.

## Decision

Add a custom documentation-site generation path and a separate web application instead of routing
the full corpus through MkDocs, Docusaurus or another general-purpose static-site generator.

The site architecture is:

1. A Rust generator utility scans a directory or explicit list of HBK books.
2. The generator groups books by locale and builds a global TOC per locale by merging the individual
   book TOC trees.
3. The generator writes a data artifact directory containing:
   - a small manifest with locales, source books and artifact versions;
   - a global TOC manifest, preferably chunked by section for lazy loading;
   - page content files referenced by stable page ids;
   - optional Syntax Assistant/search index artifacts after the documentation-site slice is accepted.
4. A separate web application serves the documentation UI, loads generated TOC/page data lazily,
   performs search when that slice exists and later exposes API endpoints for Syntax Assistant data.

The first implementation slice is only the documentation site. It must not implement Syntax
Assistant API/search behavior yet, but the generator and web-app boundaries must leave that path
open.

## Boundary Contract

The new generator and web app are separate concerns from ordinary single-book Markdown export:

- `hbk-book-export` continues to own single-book raw and Markdown/TOC export behavior.
- A new generator component owns corpus discovery, global TOC merge, stable site ids, generated
  artifact layout and the web-app data contract.
- The web application is a separate application boundary. It owns serving the UI, lazy loading,
  client/server presentation concerns and later search/API endpoints. It must not parse HBK files,
  know `FileStorage` internals or perform Syntax Assistant extraction in request paths.
- Syntax Assistant JSON export, search/index and resolver crates remain separate from the
  documentation generator unless a later task explicitly defines generated syntax index artifacts or
  a web API adapter.

## Global TOC Merge Contract

The first global TOC merge should use `hbk-reader` as a reference, not as a contract to copy
blindly:

- merge book TOC roots within the same locale into one locale-level tree;
- preserve the source book identity for every page that can be opened;
- keep deterministic ordering across runs;
- merge section nodes by normalized display title only when they represent the same navigation
  branch at the same level;
- do not use raw TOC indexes, raw HTML paths or raw HBK paths as public navigation labels;
- assign stable page ids for duplicate titles and duplicate HTML paths without relying on mutable
  `locationIndex`-style state as the public identity.

The first implementation may keep a conservative merge policy. If title-based merging causes a
wrong branch merge on real HBK data, the task must record the evidence and tighten the identity
rule before broadening the web UI.

## Performance Contract

The generator and web application must avoid the failure mode seen with general-purpose SSG tools:

- do not load all generated page Markdown into the web bundle;
- do not create one JavaScript route/module per page;
- write page content as generated data files;
- build TOC/search artifacts in bounded memory and deterministic order;
- measure file count, total output size, build time and peak RSS or equivalent on the representative
  local HBK corpus before optimizing.

## Non-Goals

- Reimplement MkDocs, Docusaurus or their plugin ecosystems.
- Put HBK parsing, Syntax Assistant extraction or corpus generation into web request handlers.
- Copy the `hbk-reader` frontend stack, routes or Kotlin/Spring service layout wholesale.
- Add search, Syntax Assistant API, network-hosted search, embeddings or semantic ranking in the
  first site slice.
- Stabilize public site URLs beyond the first accepted artifact contract.

## Implementation Plan

1. Specify the generated site data artifact contract and global TOC merge model.
2. Add a Rust generator component with typed request/result/error models.
3. Generate a global TOC manifest for a small multi-book fixture and representative real
   books.
4. Write page content into stable page data files, reusing the accepted Markdown conversion
   behavior where it remains valid for a global documentation site.
5. Add a minimal separate web application that serves the documentation UI and renders locale
   selection, lazy global TOC navigation and page content from generated artifacts.
6. Add generated Syntax Assistant/search index artifacts and web API/search behavior only after
   documentation navigation and page viewing are accepted and measured.

## Verification

- UAT covers generated site data artifacts for representative 8.5.1.1150 HBK books.
- UAT proves the web application can serve or load generated site data without invoking HBK parsing
  in request paths.
- UAT proves the web bundle or server response does not embed all page Markdown and the TOC/page data
  are loaded lazily.
- Focused tests cover global TOC merge determinism, duplicate page ids and source book provenance.
