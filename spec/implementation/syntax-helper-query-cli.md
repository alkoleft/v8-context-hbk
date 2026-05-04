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

## Product Direction

ADR-0006 owns the product direction for this scope. The `syntax` scope exists to help with BSL code
development and analysis. Treat Syntax Assistant lookup, search, relationship traversal and JSON
output as a local platform-API provider for a human developer, coding agent and future BSL analyzer.

This direction is the decision filter for new `syntax` work:

- Prefer precise callable facts over broad prose search: signatures, constructor overloads,
  parameter names, parameter types, return types, owner/member relationships and related platform
  objects.
- Keep machine-readable query output typed and unambiguous enough for tools. Search-only tokens,
  ranking aids and presentation shortcuts must not leak as misleading public JSON fields.
- Use `syntax export` consumer JSON as the compatibility anchor for shared fact shapes. Query JSON
  may add query metadata, scores or relationship paths, but callable signatures, parameters,
  `types`, `return`, names and owners should match export conventions where applicable.
- Preserve deterministic local behavior: analyzers must be able to query a prebuilt index without
  opening HBK books, depending on network services or receiving nondeterministic result ordering.
- Keep the BSL analyzer itself out of this repository until a separate plan changes scope. This
  repository provides extracted platform facts and query/index contracts.
- Evaluate future search/index/storage changes by whether they improve BSL development and
  code-analysis workflows, not only generic documentation search quality.

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
v8-context-hbk syntax get --index <index.sqlite> --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --format json
v8-context-hbk syntax get --index <index.sqlite> --name "ОтборКомпоновкиДанных" --format json
v8-context-hbk syntax get --index <index.sqlite> --owner "НастройкиКомпоновкиДанных" --member "Отбор"
v8-context-hbk syntax constructors --index <index.sqlite> "HTTPСоединение"
v8-context-hbk syntax constructors --index <index.sqlite> "HTTPСоединение" --details
v8-context-hbk syntax search --index <index.sqlite> --query "отбор скд" --mode keywords --format text
v8-context-hbk syntax search --index <index.sqlite> --query "DataCompositionFilter" --mode fuzzy --format json
v8-context-hbk syntax related --index <index.sqlite> --name "ОтборКомпоновкиДанных" --format json
v8-context-hbk syntax related --index <index.sqlite> --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --format json
v8-context-hbk syntax related --index <index.sqlite> --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json
```

For repeated interactive use, query commands may omit `--index` and use the resolved default index
path:

```bash
v8-context-hbk syntax index <shcntx.hbk>
v8-context-hbk syntax get --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"
v8-context-hbk syntax get --name "ОтборКомпоновкиДанных"
v8-context-hbk syntax constructors "HTTPСоединение"
v8-context-hbk syntax constructors "HTTPСоединение" --details
v8-context-hbk syntax search --query "отбор скд"
v8-context-hbk syntax related --name "ОтборКомпоновкиДанных"
v8-context-hbk syntax related --owner "НастройкиКомпоновкиДанных" --member "Отбор"
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

### Analyzer Provider Boundary

ADR-0007 selects local CLI JSON over a prebuilt `syntax` index as the first downstream
analyzer-facing provider boundary.

The SQLite index remains an internal derived artifact for the provider implementation. Downstream
analyzer callers should not depend on SQLite table names, row layouts, FTS columns or index schema
versions as their integration contract. Query commands own the public provider response envelope:
`syntax get`, `syntax constructors`, `syntax search` and `syntax related` return versioned,
deterministic JSON with shared platform facts under `results[].fact` and query-only metadata under
`results[].meta`.

T56 may normalize analyzer-critical storage tables so the provider can answer type/member inference
questions without parsing JSON blobs, but that storage revision does not by itself create a Rust API
contract, file artifact contract, service boundary or analyzer implementation. Any new boundary
requires a separate ADR or task with a concrete consumer and verification path.

### Index Path Resolution

`syntax index`, `syntax get`, `syntax constructors`, `syntax search` and `syntax related` resolve
one effective index path. The first slice does not merge multiple index files.

Resolution order:

1. `--index <index.sqlite>` for query commands or `--output <index.sqlite>` for index build.
2. `V8_CONTEXT_HBK_SYNTAX_INDEX`.
3. `.v8-context-hbk/syntax/index.sqlite` under the current working directory.

This overlay is path selection only. Multi-index overlays, where a project-specific index augments a
base platform index, require a separate ranking and ambiguity contract before implementation.

### Concurrent Access

Query commands open the resolved SQLite index as read-only connections. Multiple `syntax get`,
`syntax constructors`, `syntax search` and `syntax related` processes may read the same index
concurrently.

`syntax index` must build the replacement database in a temporary file beside the target index,
clean stale temporary database artifacts before opening that replacement file, validate the
completed database, then atomically rename it over the target path. It must serialize concurrent
writers with a lock. Readers must observe either the previous complete index or the next complete
index and must not observe a missing or partially written target database.

T42 implementation note: index build consumes Syntax Assistant extraction through
`SyntaxHelperReader::extract_into()` into a search-index builder rather than through the full
`PlatformContext` convenience path. The builder stages search document drafts and the minimal
identity inputs needed for T41 semantic ids, then writes documents to the temporary SQLite database
and inserts relations from the finalized document set without materializing a complete
`Vec<Relation>`. This preserves the SQLite artifact, query commands and atomic rebuild behavior; no
cache layer, graph database, external search service or tuning knob was added.

T43 implementation note: the SQLite writer keeps the same schema and artifact contract but treats
index build as a bulk load into a disposable replacement database. It prepares insert statements once
per transaction, creates ordinary lookup/relation B-tree indexes after row insertion and uses fixed
temp-rebuild settings for the replacement database. These settings do not create a user-facing
tuning surface: a failed build can leave only a stale temp artifact, which the next build removes
before creating a new temp database, while readers continue using the previous complete index until
validated atomic rename.

T44 implementation note: the search index is still one SQLite artifact, but FTS population is no
longer a row-by-row write into the virtual table. The writer stores searchable text in the ordinary
`document_search` content table, then runs SQLite FTS5 external-content rebuild for `document_fts`.
Exact lookup, fuzzy lookup and relationships remain in relational tables. The measured contentless
FTS variant reduced file size but was slower on the accepted real Russian HBK benchmark, so the
selected schema uses external-content FTS rather than a separate search artifact, contentless rowid
mapping, Tantivy, cache reuse or parallel SQLite writers.

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

- `id` stable search-index document identity, for example `platform_type:ОтборКомпоновкиДанных`;
- `kind`;
- `name_primary`;
- `name_alias`;
- `owner_primary`;
- `owner_alias`;
- `signature_text`: compact presentation text for human constructor/method output and FTS input;
- `description`;
- optional source fields when the index was built from a provenance-rich artifact.

Schema version `4` removes `documents.signature_json`, `documents.parameter_text`,
`documents.type_names`, `documents.return_names` and `documents.preview`. Callable structure,
parameter facts and type references are normalized in the analyzer-oriented tables below. Compact
preview text, when needed by CLI text presentation, is generated from `description` after reading
the row rather than stored as a SQLite column.

Document ids are search-index identities, not human-facing display labels, parser provenance or
general API presentation keys. They must not include HBK file paths, TOC paths, HTML paths, page
titles, alias display strings such as `primary (alias)` or fallback source-path suffixes. Exact
primary-name, alias and owner/member lookup belongs in `document_names`, not in `documents.id`.

Query table documents use `QueryTable.identifier` as the base identity. If real source data contains
more than one query table with the same `QueryTable.identifier`, the index must append the minimal
semantic table-family variant needed to disambiguate the duplicate, derived from the semantic
`owner_path` labels rather than from raw source paths. For example, the two accounting-register
families with and without correspondence support share table identifiers and require a
correspondence-support semantic variant.

Query table field and parameter documents are owned by the final query table identity. Their ids use
the accepted table identity plus the field or parameter name:
`query_table_field:<query_table_identity>:<field.name>` and
`query_table_parameter:<query_table_identity>:<parameter.name>`. Here `query_table_identity` is the
plain `QueryTable.identifier` for unique tables and `QueryTable.identifier` plus the accepted
semantic variant for duplicated table identities.

Platform type documents use the primary name as the base identity, but same-primary facts from
different semantic type families must keep a minimal semantic variant. Form-related examples include
ordinary-form and managed-client-form `ЭлементыФормы` / `Controls` / `FormItems` records, which are
different source-backed types rather than duplicate pages. Type member documents must therefore be
owned by the final owner identity, not only by `owner.primary`:
`type_method:<owner_identity>:<method.name>` and
`type_property:<owner_identity>:<property.name>`.

The Syntax Assistant TOC may disambiguate duplicate same-title children by appending an internal
marker such as `#&^@^%&*^#1` to the title. This marker is parser service data, not semantic identity.
Search-index document ids and lookup names must ignore the marker. If a marker-stripped fact with
the same final owner identity and primary name has already been indexed, the marked source page must
not create a second document or receive a source-path suffix. This rule applies across document
families, including methods, properties, constructors, enums and enum values.

Constructor documents are owned by the final type identity and constructor primary name.

Enum documents use the primary name as the base identity, but metadata-object property enums are a
separate enum kind from ordinary system enums. Enum value documents are owned by the final enum
identity: `enum_value:<enum_identity>:<value.name>`.

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

### `document_search`

Ordinary FTS content table. It stores one searchable row per `documents` row and owns the stable
`rowid` used by the external-content FTS table:

- `rowid`;
- `document_id`;
- `name_primary`;
- `name_alias`;
- `owner`;
- `signatures`;
- `parameters`: searchable parameter names and type-reference terms for FTS only, not a public JSON
  contract field;
- `type_names`;
- `return_names`;
- `description`.

### `document_fts`

FTS5 virtual table over:

- `document_id` as an unindexed column;
- `name_primary`;
- `name_alias`;
- `owner`;
- `signatures`;
- `parameters`;
- `type_names`;
- `return_names`;
- `description`.

Schema version `2` uses `document_search` as the external content table and populates
`document_fts` with SQLite FTS5 rebuild semantics after content rows are loaded.

Schema version `3` keeps the same SQLite/FTS5 artifact shape but stores structured callable
signatures in `documents.signature_json`. Public query JSON for callable facts uses
`signatures[].parameters[]` with `name`, `required`, `types` and optional `description`; raw
parameter/type search terms remain in `documents.parameter_text` and `document_search.parameters`.

Schema version `4` keeps `document_search` / `document_fts` as the lexical search projection, but
the structured provider facts are assembled from relational rows instead of `signature_json`.
`document_search.parameters`, `type_names` and `return_names` remain FTS/presentation input only and
are generated during index build from normalized signatures, parameters and type-reference rows.

### Analyzer-Oriented Storage Normalization

Schema version `4` is the first analyzer-oriented storage normalization slice. Analyzer-facing
storage must answer structured questions without parsing JSON blobs or FTS text fields:

- resolve a constructor expression such as `Новый HTTPСоединение(...)` to possible platform types
  and overloads;
- resolve owner/member access such as `НастройкиКомпоновкиДанных.Отбор` to a member fact and its
  type references;
- list members for a known platform type identity;
- follow method/constructor return types and parameter type references across a chain of
  expressions;
- report ambiguous, missing or unsupported type/member resolution deterministically.

The normalized schema uses ordinary relational tables:

- `type_identities`: canonical platform type identities, primary names and aliases used by type
  inference and member lookup;
- `members`: owned platform type members with `owner_type_id`, member kind, primary/alias names and
  the backing document id;
- `callables`: methods, constructors and callable events that own signatures;
- `signatures`: overload rows with stable callable id and ordinal;
- `parameters`: ordered signature parameters with name, requiredness and optional description;
- `type_refs`: normalized type-reference rows for property/query-field/query-parameter types,
  parameter types, return types, constructor result types and extension/base references where source
  evidence exists.

`documents` remains the provider/search fact projection, and `relations` remains the graph/query
edge table. Analyzer-critical facts should read the normalized rows first. Provider JSON is
assembled from those relational rows and existing Rust domain structs; the database schema does not
introduce a new JSON cache as the source of truth for typed facts.

When a type-reference name matches exactly one canonical platform type identity, `type_refs` may
store both `target_type_name` and `target_type_id`. When the name matches multiple semantic type
identities, the row must keep `target_type_name` and leave `target_type_id` unset instead of
choosing a hidden winner. Disambiguating those cases requires an explicit owner/semantic rule in a
future task.

`documents.description` and `document_search.description` intentionally serve different roles:
provider fact text vs normalized FTS content. `documents.signature_text` remains only for compact
human text output and FTS input. Search-only text fields are confined to `document_search`.

## Provider JSON Response Contract

Status: provisional target contract for T50. Existing `SearchHit<SearchDocument>` JSON is a
temporary implementation shape. Compatibility with that shape is not a goal when it conflicts with
the provider contract below.

`syntax get`, `syntax constructors`, `syntax search` and `syntax related` JSON output should use one
response envelope:

```json
{
  "schema_version": 1,
  "command": "constructors",
  "status": "ok",
  "query": {
    "kind": "constructor",
    "name": "HTTPСоединение"
  },
  "results": [
    {
      "fact": {
        "id": "constructor:platform_type:HTTPСоединение:...",
        "kind": "constructor",
        "name": {
          "primary": "Новый HTTPСоединение(<Сервер>, ...)"
        },
        "owner": "HTTPСоединение",
        "signatures": [
          {
            "parameters": [
              {
                "name": "Сервер",
                "required": true,
                "types": ["Строка"],
                "description": "..."
              }
            ]
          }
        ]
      },
      "meta": {
        "rank": 1
      }
    }
  ],
  "diagnostics": []
}
```

Envelope fields:

- `schema_version`: provider response schema version. It is separate from `syntax export` consumer
  `schema_version` and from SQLite index `schema_version`.
- `command`: one of `get`, `constructors`, `search` or `related`.
- `status`: `ok`, `not_found`, `ambiguous` or `unsupported`.
- `query`: normalized user query shape, such as `{ "kind": "document_id", "id": "..." }`,
  `{ "kind": "exact_name", "name": "..." }`,
  `{ "kind": "owner_member", "owner": "...", "member": "..." }`,
  `{ "kind": "constructor", "name": "..." }`, `{ "kind": "search", "mode": "keywords",
  "text": "..." }` or `{ "kind": "related", "root": { "name": "..." }, "depth": 2 }`.
- `results`: deterministic array of result objects. It is empty for `not_found`.
- `diagnostics`: deterministic array of provider diagnostics. It is empty when `status` is `ok`.

Result fields:

- `fact`: the platform fact. Shared fact shapes use `syntax export` field names wherever both
  surfaces expose the same data: `id`, `kind`, `name`, `owner`, `signatures`, `types`, `return`,
  `description`, availability, examples and see-also when present. Callable signatures use
  `signatures[].parameters[]` with `name`, `required`, `types` and optional `description`.
- `meta`: query-only metadata. `search` may put `score`, `rank` and matched mode here. `related`
  may put `depth` and `path` here. `get` and `constructors` may omit scores when the lookup is
  exact and deterministic. Query metadata may include richer owner identity, such as owner alias or
  owner document id, when the fact's export-compatible `owner` field is intentionally only the
  owner's primary string.

Diagnostic fields:

- `code`: stable machine code such as `NOT_FOUND`, `AMBIGUOUS` or `UNSUPPORTED_QUERY`.
- `message`: concise human-readable explanation.
- `query`: optional normalized query fragment that caused the diagnostic.
- `candidates`: optional stable candidate summaries for ambiguity diagnostics.

Query-only metadata rules:

- FTS/search token fields, raw `parameter_text`, `document_search.parameters`, internal rowids,
  SQLite scores and debug ranking inputs are not platform facts and must not appear under `fact`.
- Search scores and ranks belong in `results[].meta`.
- Relationship traversal metadata belongs in `results[].meta.depth` and `results[].meta.path`.
  Path steps may contain document ids, `edge_kind`, `label` and `evidence`, but they do not redefine
  the target fact.
- Ambiguity and missing-result behavior belongs in `status` and `diagnostics`, not in a hidden
  winner selection.
- Provider JSON must not expose HBK file paths, TOC paths, HTML paths or page titles in consumer
  facts. The `id` field is the semantic search-index identity accepted by this spec, not source
  provenance.

Command-specific mapping:

- `syntax get --id`, `syntax get --name` and `syntax get --owner --member` return exact lookup
  facts. Unique matches use `status: "ok"`. Multiple exact matches use `status: "ambiguous"` and
  include candidate facts or candidate summaries in deterministic order. Missing matches use
  `status: "not_found"`.
- Unsupported root combinations, such as passing both `--id` and `--name`, use
  `status: "unsupported"` with an `UNSUPPORTED_QUERY` diagnostic in JSON mode.
- `syntax constructors <TYPE>` returns constructor facts owned by the resolved type. Constructor
  facts must expose structured `signatures` and must not expose mixed parameter/type token arrays.
- `syntax search --query <TEXT>` returns ranked facts with `results[].meta.score` and
  `results[].meta.rank`. Ranking metadata is not part of the fact.
- `syntax related` returns related facts with relationship traversal metadata in `results[].meta`.
  Relationship roots may be a plain name for human use, a document id, or an owner/member pair.
  Plain-name and owner/member roots must report `status: "ambiguous"` when they do not resolve to
  exactly one root. Missing roots must report `status: "not_found"`.

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

T52 implementation note: query JSON now uses the provider envelope from this spec for `syntax get`,
`syntax constructors`, `syntax search` and `syntax related`. The search library exposes document-id
lookup plus relationship traversal from document id and owner/member roots. The human-friendly
plain-name `syntax related --name` path remains, but analyzer workflows can avoid ambiguous
same-name roots by using `--id` or `--owner --member`.

T49 implementation note: a temporary Tantivy sidecar prototype was measured against the accepted
SQLite/FTS5 index after the provider-envelope and BSL-task-scenario work, then removed before task
completion because it was not selected. The prototype indexed the same `documents` /
`document_search` rows as text fields and measured keyword/fuzzy search as a full-text sidecar only;
exact lookup, constructor lookup, provider JSON assembly and relationship traversal still required
the SQLite relational tables. The measured sidecar was fast and small, but it did not preserve
accepted workflow quality by itself: Russian fuzzy lookup for `ОтборКомпоновкиДаных` returned no
hits, and task search for `таблица регистра бухгалтерии` ranked generic accounting-register table
variants ahead of the UAT-SH-017 accepted top hit
`query_table:РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии`. Root/English fuzzy lookup for
`DataCompositionFiltter` did find `DataCompositionFilter`, so the conclusion is not that Tantivy is
unusable; it is that a simple FTS-only sidecar does not improve the accepted BSL/provider workflows
enough to justify a second artifact and ranking path. SQLite/FTS5 therefore remains the selected
query artifact. Tantivy remains a possible future FTS-only sidecar only if a new source-backed BSL
scenario shows a search-quality gap that SQLite cannot address without worse complexity.

T49 MyStem follow-up: measurement-only harnesses lemmatized RU `document_search` text with MyStem
3.1 before Tantivy indexing and compared morphology, identifier splitting, word-to-identifier query
compounding, domain query expansion and simple provider-aware reranking. MyStem lemmas fixed
inflected accounting-register wording, and `mystem -d` improved the plural form
`таблицы регистров бухгалтерии` from rank 2 to rank 1, but it cost roughly 4x more lemmatization
time. Identifier splitting helped compact API names: `ОтборКомпоновкиДаных` reached rank 10 with
split terms and rank 1 after adding known compounded query terms. Domain query expansion improved
`отбор скд`, but generic lexical `Отбор` facts still outranked provider-target facts until a
domain reranker lifted `НастройкиКомпоновкиДанных.Отбор`.

The practical direction is not "turn on MyStem" by itself. If a future source-backed BSL scenario
requires better Russian task search, the likely shape is: keep SQLite for exact/provider/related
facts; optionally add an FTS-only sidecar that indexes original identifiers, split identifier terms
and MyStem lemmas; apply a controlled synonym map such as `СКД -> компоновка данных`; and rank with
provider-aware kind/owner boosts. Do not introduce MyStem as a required indexing dependency without
a new ADR covering the external binary/process boundary and without proving the sidecar improves
accepted workflows beyond what SQLite/tokenization/reranking can do in one artifact.

T54 implementation note: relationship traversal now prioritizes structured type-reference and
return-type edges before the reverse `member_of` owner edge. This keeps analyzer-style roots such as
`НастройкиКомпоновкиДанных.Отбор` moving forward along the BSL type chain before expanding the
owning settings object, so the accepted SKD scenario reaches `ОтборКомпоновкиДанных.Элементы`,
`КоллекцияЭлементовОтбораКомпоновкиДанных.Добавить` and `ЭлементОтбораКомпоновкиДанных` fields
within the existing bounded local traversal. The graph still uses the same SQLite relation table,
edge kinds and maximum depth; no parser facts, storage engine, network service or semantic-search
sidecar were added.
