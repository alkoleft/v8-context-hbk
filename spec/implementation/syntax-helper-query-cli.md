# Syntax Assistant Query Command Architecture

Status: implemented first slice for FR-SH-SEARCH-001 and FR-SH-SEARCH-002. The component split is
governed by accepted ADR-0004.

## Problem

The previous `syntax-helper --output` command was an extraction/export command. T18 moved that
batch path to `syntax export`; it still reads large `shcntx_*.hbk` files and produces canonical
consumer JSON. That is the right boundary for batch export, but it is not an interactive search
interface.

The query command surface must retrieve API facts quickly and repeatedly. Index build commands read
Syntax Assistant HBK sources once through the normal extraction pipeline and write a local index.
Lookup, search and relationship commands search that prebuilt index and do not parse HBK books per
query.

## Data Layers

Keep these layers separate:

1. Provenance-rich extraction model in `syntax-helper-model`.
2. Search document model for CLI lookup/ranking.
3. Relationship graph model for owner/member/type/link traversal.
4. Lean consumer export in `hbk-export` for downstream data ingestion.
5. Optional semantic vectors as a later index extension.

Do not make the lean consumer export carry every search/debug field. If the query commands need
structured links or page provenance, store those fields in the search index or a search-specific
service artifact rather than weakening FR-EXPORT-001.

## Proposed Components

### `syntax-helper-search`

Implemented library crate.

Responsibilities:

- build in-memory or on-disk search indexes from extracted Syntax Assistant API facts;
- define `SearchDocument`, `SearchIndex`, `SearchQuery`, `SearchHit` and `RelationshipGraph`;
- own the SQLite schema, migrations for provisional schema versions and FTS5 query construction;
- normalize names, aliases and owner/member identifiers;
- implement exact, keyword and fuzzy search without HBK access;
- expose relationship traversal over owner, member, type-reference, return-type, constructor,
  enum-value and extracted-link edges;
- keep output order deterministic.

Non-responsibilities:

- HBK container reading;
- Syntax Assistant HTML parsing;
- reading or importing canonical consumer export directories in T18, including secondary or
  development-only modes;
- CLI argument parsing and presentation;
- embedding-provider integration in the first slice.

### `v8-context-hbk` `syntax` command group

Planned command group inside the existing `v8-context-hbk` binary for Syntax Assistant export,
index and query workflows.

Responsibilities:

- provide query-focused commands and readable errors;
- present text and JSON outputs;
- resolve the effective search-index path;
- load an existing search index for query commands;
- connect HBK extraction to `syntax-helper-search` for index build commands;
- avoid opening HBK files in query commands.

The existing `v8-context-hbk` binary remains the single installed CLI. `inspect`, `toc` and `page`
stay HBK inspection/navigation commands. `syntax export` is the Syntax Assistant batch export path,
while `syntax index`, `syntax get`, `syntax search` and `syntax related` are the local index/query
path.

## Command Shape

Initial command shape:

```bash
v8-context-hbk syntax export <shcntx.hbk> --output <dir>
v8-context-hbk syntax index <shcntx.hbk> --output <index.sqlite>
v8-context-hbk syntax get --index <index.sqlite> --name "ОтборКомпоновкиДанных" --format json
v8-context-hbk syntax get --index <index.sqlite> --owner "НастройкиКомпоновкиДанных" --member "Отбор"
v8-context-hbk syntax search --index <index.sqlite> --query "отбор скд" --mode keywords --format text
v8-context-hbk syntax search --index <index.sqlite> --query "DataCompositionFilter" --mode fuzzy --format json
v8-context-hbk syntax related --index <index.sqlite> --name "ОтборКомпоновкиДанных" --format json
```

For repeated interactive use, query commands may omit `--index` and use the resolved default index
path:

```bash
v8-context-hbk syntax index <shcntx.hbk>
v8-context-hbk syntax get --name "ОтборКомпоновкиДанных"
v8-context-hbk syntax search --query "отбор скд"
v8-context-hbk syntax related --name "ОтборКомпоновкиДанных"
```

Later command shape after the deterministic graph proves useful:

```bash
v8-context-hbk syntax search --query "как создается отбор скд" --mode semantic
v8-context-hbk syntax explain --query "как создается отбор скд" --format text
```

Semantic or explain-style commands must be additive. They must not replace exact lookup or graph
queries.

## Index Artifact

The first on-disk index is a rebuildable SQLite database:

- `index.sqlite`: primary index artifact.
- default path: `.v8-context-hbk/syntax/index.sqlite` relative to the current working directory;
- Optional debug exports under `target/` may dump `documents.jsonl` or `relations.jsonl`, but those
  files are not the query contract.

This is a derived artifact. The default path is suitable for repeated local interactive use. UAT and
development measurements may use explicit paths under `target/`. Index files are service data and
must not be committed unless a future task explicitly adds small committed fixtures.

### Index Path Resolution

`syntax index`, `syntax get`, `syntax search` and `syntax related` resolve one effective index path.
The first slice does not merge multiple index files.

Resolution order:

1. `--index <index.sqlite>` for query commands or `--output <index.sqlite>` for index build.
2. `V8_CONTEXT_HBK_SYNTAX_INDEX`.
3. `.v8-context-hbk/syntax/index.sqlite` under the current working directory.

This overlay is path selection only. Multi-index overlays, where a project-specific index augments a
base platform index, require a separate ranking and ambiguity contract before implementation.

### Concurrent Access

Query commands open the resolved SQLite index as read-only connections. Multiple `syntax get`,
`syntax search` and `syntax related` processes may read the same index concurrently.

`syntax index` must build the replacement database in a temporary file beside the target index,
validate the completed database, then atomically rename it over the target path. It must serialize
concurrent writers with a lock. Readers must observe either the previous complete index or the next
complete index and must not observe a partially written database.

### Why SQLite First

SQLite with FTS5 is the first storage choice because it keeps the query path local and zero-service
while covering three index shapes in one file:

- exact lookup through indexed relational tables;
- full-text search through FTS5;
- relationship search through an edge table and bounded joins/recursive CTEs.

Do not add a graph database or standalone full-text engine in the first slice. Revisit those choices
only after SQLite-backed UAT and NFR-QUERY-001 measurements identify a concrete limitation.

## SQLite Schema Draft

The schema is provisional and may change while ADR-0004 remains proposed, but it should preserve
these concepts.

### `meta`

Key/value metadata:

- `schema_version`;
- `locale`;
- `source_locale`;
- `source_hbk`;
- `source_extraction_schema_version`;
- `built_at`;
- `builder_version`.

### `documents`

One row per searchable API fact:

- `id` stable text key, for example `platform_type:ОтборКомпоновкиДанных`;
- `kind`;
- `name_primary`;
- `name_alias`;
- `owner_primary`;
- `owner_alias`;
- `signature_text`;
- `type_names`;
- `return_names`;
- `description`;
- `preview`;
- optional source fields when the index was built from a provenance-rich artifact.

Indexes:

- `documents(kind)`;
- `documents(name_primary)`;
- `documents(name_alias)`;
- `documents(owner_primary, name_primary)`;
- `documents(owner_alias, name_alias)`;

### `document_names`

Normalized lookup keys:

- `key`;
- `key_kind`: `primary`, `alias`, `owner_member_primary`, `owner_member_alias`;
- `document_id`;

Index:

- unique or non-unique `document_names(key, key_kind, document_id)` depending on ambiguity handling.

The lookup layer must report ambiguous exact matches instead of picking a hidden winner.

### `document_fts`

FTS5 virtual table over:

- `name_primary`;
- `name_alias`;
- `owner`;
- `signatures`;
- `parameters`;
- `type_names`;
- `return_names`;
- `description`.

The first ranking may use FTS5 `bm25()` plus deterministic tie breakers:

1. exact primary or alias match;
2. owner/member match;
3. name/alias token match;
4. signature/type-reference match;
5. description match;
6. `kind` priority;
7. stable `id`.

### `relations`

Directed relationship edges:

- `source_id`;
- `target_id`;
- `edge_kind`;
- `label`;
- `evidence`: `structured`, `type_ref`, `return_type`, `owner`, `description`, `see_also`;
- `weight`;
- optional source/provenance fields when available.

Indexes:

- `relations(source_id, edge_kind, target_id)`;
- `relations(target_id, edge_kind, source_id)`;
- `relations(edge_kind)`.

Supported first-slice edge kinds:

- `owns`;
- `member_of`;
- `has_type`;
- `returns`;
- `constructs`;
- `enum_value_of`;
- `mentions`.

Future edge kind:

- `see_also`, after structured Syntax Assistant link extraction is implemented.

## Graph Query Rules

The first relationship search is not a general graph database.

Rules:

- traverse only bounded depth, default `5`, maximum `5` for the first SKD-filter path accepted by
  UAT-SH-006;
- traverse directed outgoing edges from the selected root fact;
- prefer structured owner/type-reference/return edges over future `mentions`;
- emit deterministic path order by edge weight, edge kind and stable document id;
- include enough edge labels to explain why a fact is related;
- do not use graph algorithms that require an external graph engine in the first slice.

External graph storage may be reconsidered only if a measured use case needs deeper path search,
centrality/community algorithms or multi-source graph analytics that are awkward or slow in SQLite.

## Full-Text and Fuzzy Search Rules

Full-text search:

- tokenize Russian and English names/descriptions with the SQLite tokenizer available in the chosen
  Rust SQLite binding;
- normalize case and punctuation in an application-side `search_text` field if tokenizer behavior is
  insufficient for 1C identifiers;
- keep ranking deterministic with stable tie breakers.

Fuzzy search:

- start with normalized name/alias candidates only;
- use edit distance or trigram-like matching in Rust or SQL only after exact and FTS candidates are
  insufficient;
- do not fuzzy-match full descriptions in the first slice.

Semantic search:

- add only after deterministic search is accepted;
- keep semantic vectors as optional sidecar data in SQLite or a separate rebuildable artifact;
- never make exact lookup or relationship traversal depend on an embedding provider.

## First Implementation Slice

Before adding semantic search:

1. Build a deterministic SQLite/FTS5 index from a Syntax Assistant HBK through the extraction
   pipeline.
2. Implement exact name and owner/member lookup.
3. Implement keyword search over normalized names, aliases, signatures, type refs and descriptions.
4. Implement relationship traversal over owner/member and type-reference edges stored in
   `relations`.
5. Add a follow-up task for structured "see also" extraction if relationship quality is insufficient.

Semantic search is intentionally deferred until the local deterministic search path is measured and
useful.

T18 implementation note: owner/member and type-reference edges were sufficient for UAT-SH-006 on
`shcntx_ru.hbk`; no immediate structured "see also" extraction follow-up was required.
