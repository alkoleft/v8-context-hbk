# ADR-0004: Add a Separate Syntax Assistant Query CLI on a Prebuilt Search Index

Date: 2026-04-30.

Status: Proposed.

## Context

The repository currently has one installed CLI, `v8-context-hbk`, whose commands are oriented around
HBK verification and batch extraction:

- `inspect`
- `toc`
- `page`
- `syntax-helper --output`

The `syntax-helper --output` command is intentionally a batch export path. On the current
`shcntx_ru.hbk` source it reads a 40 MiB HBK file and takes about 19 seconds in a debug build. That
is acceptable for extraction and UAT, but not for interactive API lookup.

The new requirement is a separate CLI interface for Syntax Assistant workflows:

- search by exact API name;
- search by purpose and keywords;
- fuzzy search;
- eventual semantic search;
- relationship search, for example discovering the API facts involved in creating an SKD
  data-composition filter.

The current lean consumer export is useful but not sufficient for every relationship query. It
contains names, aliases, owners, signatures, type references, return types and descriptions. It
intentionally omits per-record source provenance and navigation/link scaffolding from consumer
files. Structured "see also" links are not yet exported; in the current output they are flattened
into description text.

## Decision Proposal

Add a separate Syntax Assistant query CLI surface backed by a prebuilt local search index.

The proposed installed binary name is `v8-sh`.

The existing `v8-context-hbk` binary remains the HBK inspection, navigation and batch extraction
CLI. It must not become a broad interactive query shell.

The query CLI should use this flow:

1. Build or receive canonical Syntax Assistant export data.
2. Build a local search index from that export, with optional richer link/provenance input later.
3. Run exact lookup, keyword/fuzzy search and relationship queries against the prebuilt index.

Interactive query commands must not open or parse `shcntx_*.hbk` files.

## Consequences

- Fast query behavior is separated from expensive HBK extraction.
- `FR-EXPORT-001` can stay lean; search-only fields do not need to pollute consumer export files.
- A new `syntax-helper-search` library crate can own indexing, ranking and relationship traversal.
- A new `syntax-helper-cli` binary crate or CLI target can own the `v8-sh` command surface.
- Semantic search remains an additive extension after deterministic exact/keyword/relation search
  is useful and measured.

## Alternatives Considered

### Add more subcommands under `v8-context-hbk syntax-helper`

Rejected for the proposal.

The existing binary is a verification/extraction CLI. Adding interactive search modes there would
mix expensive source-book operations with fast query operations and make command intent less clear.

### Query the canonical JSON export directly every time

Rejected as the target architecture, but acceptable for a narrow prototype.

The current export is only about 21 MiB for `shcntx_ru.hbk`, so direct JSON loading can validate
query semantics. It does not give a durable latency contract, and it cannot recover structured links
that were intentionally omitted from consumer files.

### Parse HBK on every query

Rejected.

This violates the fast query requirement and duplicates the extraction path in an interactive
command.

### Start with semantic search

Rejected for the first slice.

Semantic search may be useful for purpose-oriented questions, but exact lookup and relationship
queries need deterministic behavior, offline availability and clear acceptance tests first.

## Implementation Plan

1. Keep T17 focused on the selected streaming extraction optimization unless the task ledger is
   explicitly reprioritized.
2. Add or refine a follow-up task for the query CLI after T17.
3. Implement `syntax-helper-search`:
   - load canonical export directories;
   - build `SearchDocument` records;
   - build exact primary/alias and owner/member indexes;
   - build relationship edges from owner/member and type-reference facts;
   - implement deterministic exact, keyword and fuzzy search.
4. Implement the separate query CLI surface:
   - `v8-sh index <syntax-helper-output-dir> --output <index-dir>`;
   - `v8-sh get --index <index-dir> --name <name> --format text|json`;
   - `v8-sh get --index <index-dir> --owner <type> --member <member> --format text|json`;
   - `v8-sh search --index <index-dir> --query <text> --mode keywords|fuzzy --format text|json`;
   - `v8-sh related --index <index-dir> --name <name> --format text|json`.
5. Add a follow-up extraction/index task for structured Syntax Assistant links:
   - preserve section member links where they help relationship queries;
   - extract "see also" links as structured relationships;
   - keep those fields in a search/maintenance artifact, not in lean consumer export files.
6. Add semantic search only after the deterministic local search path is accepted and measured.

## Verification

- [ ] `spec/requirements/functional.md` contains Syntax Assistant query/search requirements.
- [ ] `spec/requirements/non-functional.md` contains query latency expectations.
- [ ] `spec/acceptance/uat-test-cases.md` contains black-box UAT cases for index build, exact lookup
      and relationship search.
- [ ] Exact lookup for `ОтборКомпоновкиДанных` and `DataCompositionFilter` returns the same
      platform type.
- [ ] Owner/member lookup for `НастройкиКомпоновкиДанных.Отбор` returns a property with type
      reference `ОтборКомпоновкиДанных`.
- [ ] Relationship output for `ОтборКомпоновкиДанных` exposes constructor, `Элементы`,
      collection-item creation and filter item fields.
- [ ] Query commands meet or explicitly measure against NFR-QUERY-001.
