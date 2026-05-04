# ADR-0004: Add Syntax Assistant Query Commands on a Prebuilt Search Index

Date: 2026-04-30.

Status: Accepted.

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

The new requirement is a separate command level inside the existing CLI for Syntax Assistant
workflows:

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

Add a `syntax` command group under the existing `v8-context-hbk` binary, backed by a prebuilt local
search index.

Use a single local SQLite database with FTS5 as the first index storage format:

- normal tables for documents, names, owners and deterministic relationship edges;
- an FTS5 virtual table for lexical full-text search over names, aliases, signatures, type
  references and descriptions;
- SQL queries for exact lookup, owner/member lookup and bounded relationship traversal;
- recursive CTEs only for small bounded graph walks;
- optional vector/semantic tables only after deterministic lexical and graph search are accepted and
  measured.

The existing `v8-context-hbk` binary remains the only installed CLI. `inspect`, `toc` and `page`
stay HBK inspection/navigation commands. `syntax export` owns Syntax Assistant batch export, while
`syntax index`, `syntax get`, `syntax search` and `syntax related` own local index/query workflows.

The query command group should use this flow:

1. Read a Syntax Assistant HBK through the normal extraction pipeline.
2. Build a local search index from extracted API facts, with optional richer link/provenance input
   later.
3. Run exact lookup, keyword/fuzzy search and relationship queries against the prebuilt index.

Interactive query commands must not open or parse `shcntx_*.hbk` files.

Query commands should support a default index path so repeated interactive commands can omit
`--index`. The first slice resolves one effective index path by command-line option, environment
variable or default path; it does not merge multiple index files.

## Consequences

- Fast query behavior is separated from expensive HBK extraction.
- `FR-EXPORT-001` can stay lean; search-only fields do not need to pollute consumer export files.
- A new `syntax-helper-search` library crate can own indexing, ranking and relationship traversal.
- The existing `v8-context-hbk-cli` crate owns the `syntax` command group and presentation.
- The first durable index is one rebuildable local database file, not a directory of ad hoc JSONL
  files.
- The first graph implementation is an edge table plus bounded SQL traversal, not an external graph
  database.
- Semantic search remains an additive extension after deterministic exact/keyword/relation search
  is useful and measured.

Implementation status after T18/T48-T56:

- The accepted CLI shape is implemented as `syntax export`, `syntax index`, `syntax get`,
  `syntax constructors`, `syntax search` and `syntax related`.
- The selected query artifact remains one local SQLite/FTS5 database. T49 measured a temporary
  Tantivy sidecar against the accepted provider workflows and retained SQLite/FTS5 because Tantivy
  did not preserve exact lookup, constructor lookup, deterministic provider JSON and relationship
  traversal by itself.
- Schema version `4` keeps the SQLite/FTS5 artifact but normalizes analyzer-critical facts into
  relational tables: type identities, members, callables, signatures, parameters and type
  references. Provider JSON is assembled from normalized rows, while FTS text remains a
  presentation/search projection.
- Relationship traversal stays bounded SQL over the local index. T54 improved accepted BSL scenario
  coverage by prioritizing structured type-reference and return-type edges before broad owner
  expansion.

## Alternatives Considered

### Add more subcommands under `v8-context-hbk syntax-helper`

Rejected in favor of the shorter `v8-context-hbk syntax` command group.

The `syntax-helper` name is too long for repeated interactive search commands. The shorter `syntax`
level keeps one installed CLI while still separating HBK inspection/navigation from Syntax Assistant
export/index/query workflows.

### Build the index from canonical JSON export directories

Rejected for the first slice.

The index should be built directly from HBK through the extraction pipeline, then passed to the
search library as typed API facts. Building from consumer JSON adds an unnecessary IO and
serialization round trip, makes search depend on the lean export adapter shape and cannot recover
structured links or provenance that were intentionally omitted from consumer files.

### Use JSONL index files plus in-memory maps

Rejected as the first durable index format.

JSONL files are simple and transparent, but they push exact lookup, full-text ranking, edge joins and
schema migration logic into custom Rust code. They remain useful as debugging exports from the index
builder, not as the primary query store.

### Use Tantivy or another embedded Rust full-text engine

Deferred.

An embedded search engine could provide stronger lexical ranking than SQLite FTS5. It does not
solve owner/member lookup and relationship traversal by itself, so the project would still need a
separate structured store. Use it only if SQLite FTS5 ranking or query latency is measured as
insufficient.

### Use an external graph database

Rejected for the first slice.

The current relationship graph is small enough to fit in a local index built from roughly tens of
thousands of Syntax Assistant facts. Requiring Neo4j, Kuzu, SurrealDB or another graph service would
add installation and operational complexity to a local CLI before graph queries prove they need it.
If later relationship queries need deeper graph algorithms, revisit this as a separate ADR.

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
2. Use T18 as the first query command implementation task after the prerequisite export/schema
   fixes.
3. Implement `syntax-helper-search`:
   - accept extracted Syntax Assistant API facts from the extraction pipeline;
   - build `SearchDocument` records;
   - write `index.sqlite` with schema metadata, document rows, exact-name rows, FTS5 rows and
     relationship-edge rows;
   - build exact primary/alias and owner/member indexes;
   - build relationship edges from owner/member and type-reference facts;
   - implement deterministic exact, keyword and fuzzy search over SQLite queries.
4. Implement the `v8-context-hbk syntax` command group:
   - `v8-context-hbk syntax export <shcntx.hbk> --output <dir>`;
   - `v8-context-hbk syntax index <shcntx.hbk> --output <index.sqlite>`;
   - `v8-context-hbk syntax get --index <index.sqlite> --name <name> --format text|json`;
   - `v8-context-hbk syntax get --index <index.sqlite> --owner <type> --member <member> --format text|json`;
   - `v8-context-hbk syntax search --index <index.sqlite> --query <text> --mode keywords|fuzzy --format text|json`;
   - `v8-context-hbk syntax related --index <index.sqlite> --name <name> --format text|json`.
5. Add default index path resolution:
   - explicit `--index` or `--output`;
   - `V8_CONTEXT_HBK_SYNTAX_INDEX`;
   - `.v8-context-hbk/syntax/index.sqlite` under the current working directory.
6. Add a follow-up extraction/index task for structured Syntax Assistant links:
   - preserve section member links where they help relationship queries;
   - extract "see also" links as structured relationships;
   - keep those fields in the search index or a search-specific service artifact, not in lean
     consumer export files.
7. Add semantic search only after the deterministic local search path is accepted and measured.

## Verification

- [x] `spec/requirements/functional.md` contains Syntax Assistant query/search requirements.
- [x] `spec/requirements/non-functional.md` contains query latency expectations.
- [x] `spec/acceptance/uat-test-cases.md` contains black-box UAT cases for index build, exact lookup
      and relationship search.
- [x] Exact lookup for `ОтборКомпоновкиДанных` and `DataCompositionFilter` returns the same
      platform type.
- [x] Owner/member lookup for `НастройкиКомпоновкиДанных.Отбор` returns a property with type
      reference `ОтборКомпоновкиДанных`.
- [x] Relationship output for `ОтборКомпоновкиДанных` exposes constructor, `Элементы`,
      collection-item creation and filter item fields.
- [x] Query commands meet or explicitly measure against NFR-QUERY-001.
- [x] The index artifact is a rebuildable SQLite database with FTS5 enabled or the implementation
      records a measured blocker and updates this ADR before choosing another store.
