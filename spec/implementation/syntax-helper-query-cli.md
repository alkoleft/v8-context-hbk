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
4. Lean consumer export in `hbk-syntax-export` for downstream data ingestion.
5. Optional semantic vectors as a later index extension.

Do not make the lean consumer export carry every search/debug field. If the query commands need
structured links or page provenance, store those fields in the search index or a search-specific
service artifact rather than weakening FR-EXPORT-001.

## Proposed Components

### `syntax-helper-search`

Implemented library crate.

Responsibilities:

- build on-disk search indexes from streamed extracted Syntax Assistant API facts;
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
v8-context-hbk syntax search --index <index.sqlite> --query "Структура" --mode keywords --limit 3 --format json
v8-context-hbk syntax search --index <index.sqlite> --query "DataCompositionFilter" --mode fuzzy --format json
v8-context-hbk syntax related --index <index.sqlite> --name "ОтборКомпоновкиДанных" --format json
v8-context-hbk syntax related --index <index.sqlite> --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --format json
v8-context-hbk syntax related --index <index.sqlite> --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --edge member_of --format json
v8-context-hbk syntax related --index <index.sqlite> --id "type_property:platform_type:Символы:ПС" --limit 5 --compact --format json
v8-context-hbk syntax related --index <index.sqlite> --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json
v8-context-hbk syntax related --index <index.sqlite> --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --graph --format json
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

ADR-0008 adds a separate Rust solution-context resolver boundary for an in-process full-context
application. That boundary composes platform, BSL-language, query-language, configuration and
source-code providers; it does not change the CLI JSON contract or make the SQLite index public.
When a downstream static-analysis project can include this workspace as Cargo dependencies, its
hot-path integration should use ADR-0008's resolver/source traits and adapters instead of spawning
these CLI commands or introducing HTTP/MCP transport. The CLI JSON provider remains the
language-agnostic tool boundary and UAT-friendly compatibility surface.

### Provider JSON Assembly Boundary

T148 keeps the smallest provider JSON assembly boundary inside `v8-context-hbk-cli`. Command
handlers may parse CLI arguments, resolve the effective index path, execute `SearchIndex` queries
and select text versus JSON presentation, but provider envelope/result shaping belongs to the
private provider-JSON helper layer in the CLI crate.

That helper layer owns:

- the provider response envelope for `syntax get`, `syntax constructors`, `syntax search`,
  `syntax related` and `syntax related --graph`;
- export-compatible shared fact objects under `results[].fact`, including `signatures`,
  `signatures[].parameters[]`, `types`, `return`, `name`, `owner`, `id` and `kind`;
- query-only metadata under `results[].meta`, including search rank/score, relationship paths,
  owner/type resolution aids and graph type-reference resolution data;
- provider diagnostics for unsupported, missing, ambiguous, unresolved and ambiguous graph
  type-reference outcomes.

The boundary deliberately does not move JSON shape ownership into `syntax-helper-search`: that crate
owns index storage, lookup and graph traversal DTOs, while the CLI provider layer converts those
DTOs into the public provisional provider JSON contract. The CLI provider layer must not serialize
internal search/model DTOs wholesale with `json!(...)` when those DTOs carry fields outside the
provider contract. Graph type-reference metadata renders target status and template-binding data
explicitly:

- `status` is one of `ok`, `unresolved` or `ambiguous`;
- `target_type_id` is emitted only for resolved references;
- `candidate_type_ids` is emitted only for ambiguous references;
- `template_binding` uses an explicit object with `template_key.family`,
  `template_key.variant` and provider-owned binding argument objects.

This is a boundary hardening task only. It preserves provider `schema_version: 1`, the accepted
envelope shape and the canonical `syntax export` consumer schema.

T131 implementation note: schema version 8 stores platform metadata-template facts for resolver
consumers: metadata kind and template parameters for `PlatformTypeKind::MetadataTemplate` records.
The data is exposed through `context-resolver-core::TypeInfo`, not as a public SQLite schema
contract or CLI JSON expansion.

T132 implementation note: schema version 9 stored semantic platform type template facts and
template owner-parameter bindings for type references.

T133 implementation note: schema version 11 superseded the closed semantic-kind shape. The index
stores open type template families, generated variants, classification evidence diagnostics and
template owner-parameter binding arguments derived from HBK facts, while keeping SQLite tables and
columns private rebuildable provider state. Consumers must use the Rust search/resolver APIs instead
of depending on SQLite table or column names. This does not change the CLI JSON provider envelope or
the canonical `syntax export` consumer schema.

T134 implementation note: schema version 12 keeps the same classification and binding semantics but
renames private SQLite layout identifiers to type-template terminology (`template_family`,
`template_variant`, `template_classification_diagnostic`, `type_template_family`,
`type_template_variant` and `template_binding_*`). The CLI provider JSON envelope remains
`schema_version: 1`, and the canonical `syntax export` consumer JSON remains `schema_version: 11`.

T139 implementation note: schema version 13 stores type-reference target resolution on each
normalized `type_refs` row as private rebuildable provider state: raw `target_type_name`,
`target_resolution_status`, optional resolved `target_type_id` and optional deterministic ambiguous
candidate ids. This keeps `types` / `return` public fact fields export-compatible source-name
arrays while giving search/resolver adapters one owner for resolved, unresolved and ambiguous
target outcomes.

T152 implementation note: schema version 14 stores module-context relation keys for module-event
documents as private rebuildable provider state. The Rust resolver adapter uses those keys to expose
provider-backed module contexts without adding a public SQLite table or changing provider JSON,
canonical `syntax export` JSON or query-time HBK access.

T135 implementation note: `syntax type-ref-gaps` is a report command over an existing prebuilt
SQLite index. It counts `type_refs` rows by source role and resolution status, reports template
bindings as an additional subset counter, and lists top unresolved/ambiguous target names with
source fact examples and ambiguous candidate type ids. It is not a provider JSON command and does
not change the provider envelope, canonical export schema or query-time HBK access boundary.

T141 verification note: platform type-template strengthening did not require a new schema version
or provider JSON change. The current index builder keeps the T133 classification rules, stores
callable parameter and overload return type-reference bindings as private normalized `type_refs`
data, and resolver/search adapters expose those bindings through Rust DTOs without making SQLite
columns a public contract.

T133 type template classification rule:

- use each template's `alias_base` or fallback root-locale `primary_base`; non-root localized
  primary names without aliases are left unclassified with diagnostics instead of becoming families;
- derive family roots from `*Manager` templates;
- assign templates by longest manager-root prefix;
- do not create fallback-prefix families for unassigned templates;
- classify remaining templates only through direct type-template type-reference scores against already
  derived families;
- assign only when exactly one family has direct references, otherwise keep the template
  unclassified with a diagnostic.

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

ADR-0011 owns the cross-consumer identity boundary for these facts. Parent fact identities are
computed by `syntax-helper-model` / `syntax-helper-extract` during reading, before records reach the
search-index builder. The search index may wrap those identities in search-specific document id
strings, but it must not reinterpret TOC ownership locally.

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

Constructor documents are owned by the final type identity and constructor signature text used by
the query index. If real source documentation emits duplicate constructor pages with the same final
owner identity and signature text, the index keeps the last extracted document for that id and
reports a build warning instead of aborting the rebuild. This is a documentation-defect recovery
rule applied after semantic identity normalization: unresolved duplicate final document ids keep the
last extracted document and emit a build warning so the source defect remains visible.

System enum documents normally use the primary enum name as their identity. If real source data
contains distinct system enums with the same primary name and different aliases, the index appends
the alias as the minimal semantic variant and owns enum-value document ids through that final enum
identity. Duplicate enum ids that cannot be distinguished by alias fall under the same
last-document warning rule after identity normalization.

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

`target_type_name` is the source-backed reference spelling and is preserved independently from
target resolution. Index build is the single owner of target resolution for these rows:

- `ok`: the name maps to exactly one canonical platform type identity and stores that
  `target_type_id`;
- `unresolved`: the name maps to no known platform type identity and stores no candidates;
- `ambiguous`: the name maps to multiple platform type identities and stores deterministic
  candidate type ids.

When index-time type-reference target resolution uses the source spelling from an extracted
Syntax Assistant type reference, it may use exact primary/alias spelling as a disambiguator after
the broader whitespace-insensitive lookup has found multiple candidates. This is deliberately
narrower than public plain-name lookup: `syntax get --name`, owner-name/member lookup and graph root
resolution must still report `ambiguous` when the caller provides an ambiguous display name.
`ЭлементыФормы` remains ambiguous in the current RU baseline because both platform type identities
share the same exact primary spelling; resolving those rows requires preserving source link target
identity or an equivalent parser-owned target fact.

Provider and resolver adapters must consume that stored resolution data instead of recomputing
ambiguous candidates or unresolved status from type names in each layer. Export-compatible public
fact fields continue to expose source names through `types` and `return`; resolution aids belong in
provider metadata or resolver DTOs and must not turn SQLite columns into a public contract.

Callable return references preserve their source scope. Page-level/shared return sections are
stored as callable/document-level `return_type` rows without `source_signature_id` and remain
exposed as fact-level `return`. Source-proven overload-specific returns are stored as `return_type`
rows with `source_signature_id` and `source_signature_ordinal` and may be exposed as
`signatures[].return`. Provider response `schema_version: 1` is unchanged because the envelope and
command semantics stay the same; the optional signature-level field uses the existing
export-compatible `return` name for a more precise callable fact. Query commands still read only the
prebuilt index and must not inspect HBK/HTML pages to decide return scope.

`documents.description` and `document_search.description` intentionally serve different roles:
provider fact text vs normalized FTS content. `documents.signature_text` remains only for compact
human text output and FTS input. Search-only text fields are confined to `document_search`.

## Provider JSON Response Contract

Status: provisional provider contract implemented after T50-T52 and narrowed by T86. Query result
DTOs such as `SearchHit`, `SearchDocument`, `RelatedHit` and `RelationStep` are Rust adapter structs,
not a public JSON shape. `v8-context-hbk-cli` assembles provider JSON explicitly; compatibility with
any older direct DTO serialization is not a goal when it conflicts with the provider contract below.
T93 enforced this boundary for nested callable facts: `SearchSignature` and `SearchParameter` no
longer carry serde/provider attributes, and the CLI assembles `signatures[].parameters[]` JSON
explicitly with export-compatible field names.

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
  Overload-specific return facts, when source evidence proves them, use `signatures[].return` while
  shared/page-level callable returns stay on fact-level `return`.
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
  `results[].meta.rank`. Ranking metadata is not part of the fact. It accepts `--limit <N>` to
  bound the deterministic result array. When omitted, the default remains the first implementation's
  `20` search results.
- `syntax related` returns related facts with relationship traversal metadata in `results[].meta`.
  Relationship roots may be a plain name for human use, a document id, or an owner/member pair.
  Plain-name and owner/member roots must report `status: "ambiguous"` when they do not resolve to
  exactly one root. Missing roots must report `status: "not_found"`. It accepts `--limit <N>` to
  bound the deterministic result array. When omitted, the default remains the first implementation's
  `200` relationship results. `syntax related --compact` is an explicit review/triage output mode:
  it keeps stable fact identity fields (`id`, `kind`, `name` and optional `owner`) and relationship
  explanation under `results[].meta.depth` / `results[].meta.path`, while omitting bulky fact fields
  such as descriptions, signatures, `types` and `return`. Full provider JSON remains the default
  when `--compact` is omitted.

The first ranking may use FTS5 `bm25()` plus deterministic tie breakers:

1. exact primary or alias match;
2. owner/member match;
3. name/alias token match;
4. signature/type-reference match;
5. description match;
6. `kind` priority;
7. stable `id`.

## Analyzer Query Primitives

Status: implemented by T58 over the schema-v4 normalized provider index.

ADR-0007 keeps CLI JSON over the existing `syntax` query command group as the provider boundary.
The analyzer-oriented primitives below are therefore provider query kinds inside the same resolved
prebuilt-index envelope, not a Rust API, SQLite table contract, daemon or analyzer implementation.
They read the normalized schema-v4 facts added by T56, but table names and row shapes stay
internal.

For the separate in-process resolver API accepted later, see
[`solution-context-resolve.md`](solution-context-resolve.md) and ADR-0008.

Selected command shape:

```bash
v8-context-hbk syntax get --index <index.sqlite> --kind platform_type --id "platform_type:ОтборКомпоновкиДанных" --format json
v8-context-hbk syntax get --index <index.sqlite> --kind platform_type --name "ОтборКомпоновкиДанных" --format json
v8-context-hbk syntax get --index <index.sqlite> --kind platform_type --alias "DataCompositionFilter" --format json
v8-context-hbk syntax get --index <index.sqlite> --members-of "platform_type:ОтборКомпоновкиДанных" --format json
v8-context-hbk syntax get --index <index.sqlite> --owner-type-id "platform_type:НастройкиКомпоновкиДанных" --member "Отбор" --format json
v8-context-hbk syntax get --index <index.sqlite> --owner "НастройкиКомпоновкиДанных" --member "Отбор" --format json
v8-context-hbk syntax constructors --index <index.sqlite> "HTTPСоединение" --format json
v8-context-hbk syntax get --index <index.sqlite> --callable-id "type_method:platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных:Добавить" --format json
v8-context-hbk syntax get --index <index.sqlite> --owner-type-id "platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных" --callable "Добавить" --format json
v8-context-hbk syntax related --index <index.sqlite> --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --edge has_type --format json
```

The first implementation should extend `get`, `constructors` and `related` rather than add new
top-level commands such as `syntax type`, `syntax members` or `syntax callable`. This keeps the
public CLI surface aligned with ADR-0007 while allowing the normalized `query.kind` values to be
more analyzer-specific.

Primitive behavior:

- type identity resolution uses `syntax get --kind platform_type` with exact `--id`, `--name` or
  `--alias`. Unique matches return `status: "ok"` and exactly one `platform_type` fact. Duplicate
  primary or alias matches return `status: "ambiguous"` with deterministic candidate summaries; no
  source page order, FTS rank or first row may select a hidden winner. Missing matches return
  `status: "not_found"`.
- member listing uses `syntax get --members-of <TYPE_ID>`. It requires an exact type id in the first
  implementation. The result is a deterministic array ordered by member kind priority, primary name
  and stable id. Unsupported plain owner names use `status: "unsupported"` until type resolution is
  explicitly chained by the caller.
- member resolution uses `syntax get --owner-type-id --member` or the existing
  `--owner --member`. The owner-type-id path is analyzer-preferred. The owner-name path first applies
  the same exact type resolution rules as type identity resolution; if the owner name is ambiguous,
  the whole command returns `status: "ambiguous"` for the owner. If multiple members with the same
  name remain under one resolved owner, the command returns `status: "ambiguous"` with member
  candidates.
- callable overload retrieval uses `syntax constructors <TYPE>` for constructors and
  `syntax get --callable-id <ID>` or owner identity plus callable name for methods and callable
  events. It returns ordered
  `signatures[]`, `signatures[].parameters[]`, optional `signatures[].return` and fact-level
  `return` / constructor result `types` using the export-compatible field names already used by the
  provider envelope. Fact-level `return` means shared callable return evidence; signature-level
  `return` means source-proven overload-specific return evidence.
- type-reference traversal uses `syntax related --id <FACT_ID> --edge has_type|returns|constructs`
  or the typed fields already present on exact `get` facts. Property and field facts return their
  `types`; callable facts return parameter `types` and `return` facts; constructor callables return
  constructor result `types`. When a reference name maps to exactly one known type identity, the
  result may include the resolved type id in `meta.target_type_ids`; unresolved or duplicate targets
  keep the source-backed type name and avoid hidden disambiguation. Resolver-facing type references
  expose the same distinction as data: source name plus target resolution `ok`, `unresolved` or
  `ambiguous` with candidate ids for ambiguous rows.

All primitive JSON responses use provider `schema_version: 1` until the envelope itself changes.
The `command` field remains `get`, `constructors` or `related`. The `query.kind` field records the
normalized primitive, for example `type_identity`, `member_list`, `owner_type_member`,
`callable_overloads` or `type_references`. Example:

```json
{
  "schema_version": 1,
  "command": "get",
  "status": "ok",
  "query": {
    "kind": "owner_type_member",
    "owner_type_id": "platform_type:НастройкиКомпоновкиДанных",
    "name": "Отбор"
  },
  "results": [
    {
      "fact": {
        "id": "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор",
        "kind": "type_property",
        "name": { "primary": "Отбор" },
        "owner": "НастройкиКомпоновкиДанных",
        "types": ["ОтборКомпоновкиДанных"]
      },
      "meta": {
        "owner_type_id": "platform_type:НастройкиКомпоновкиДанных",
        "target_type_ids": ["platform_type:ОтборКомпоновкиДанных"]
      }
    }
  ],
  "diagnostics": []
}
```

Fact shape rules:

- Shared platform facts stay under `results[].fact` and reuse export-compatible names:
  `id`, `kind`, `name`, `owner`, `signatures`, `signatures[].parameters[]`, `types`, `return` and
  optional `description`.
- Analyzer-only resolution aids belong in `results[].meta`, for example `owner_type_id`,
  `target_type_ids`, `signature_ordinal` or an edge `source`.
- Public JSON must not expose SQLite table names, rowids, FTS text fields, ranking tokens, HBK
  paths, TOC paths, HTML paths or page titles.

Diagnostics:

- `NOT_FOUND`: no exact type, member, callable or type-reference root matches the normalized query.
- `AMBIGUOUS`: exact input matches more than one type/member/callable. Diagnostics include stable
  candidate summaries with id, kind, primary name, optional alias and owner when available.
- `UNSUPPORTED_QUERY`: an input combination is outside the primitive contract, such as
  `syntax get --members-of <NAME>` in the first implementation, or a command mixing mutually
  exclusive roots.

Non-goals:

- no BSL parsing, expression parser, linter, diagnostics or code actions;
- no Rust public analyzer API through this CLI provider primitive, daemon, MCP service, network
  search or storage selector;
- no compatibility with older provisional query JSON when it conflicts with this provider shape;
- no public SQLite table, column or index schema contract.

T58 implementation note: the CLI extends the existing command group only. `syntax get` accepts the
selected type identity, member-list, owner-type/member and callable roots; `syntax constructors`
continues to own constructor overload lookup; `syntax related --id --edge` provides direct
edge-filtered traversal for `has_type`, `returns` and `constructs` style edges. The implementation
reads normalized schema-v4 rows inside `syntax-helper-search`, but the public JSON remains the
provider envelope with stable fact fields under `results[].fact` and analyzer-only resolution
metadata under `results[].meta`.

T60 implementation note: exact-name provider lookup no longer collapses mixed ownerless/owned
matches to the ownerless fact. `syntax get --name` and `syntax related --name` now surface the full
deterministic candidate set as `status: "ambiguous"` when more than one exact fact matches. For
owner-name/member roots, the provider first resolves the owner as a platform type identity; if that
owner name is ambiguous, `syntax get --owner --member` and `syntax related --owner --member` return
the owner type candidates instead of filtering down to whichever duplicate happens to have the
requested member. `syntax constructors <TYPE>` uses the same type identity resolution and returns a
provider `ambiguous` envelope for duplicate type names.

T64 implementation note: `member_of` is a public provider edge filter, not storage-only service
data. `syntax related --id <owned-fact-id> --edge member_of` traverses the existing directed
inverse owner edge from an owned fact to its owning fact and returns the normal provider envelope.
Type-reference edge filters keep `query.kind: "type_references"`; `member_of` remains
`query.kind: "related"` because it explains graph ownership rather than a property, return or
constructor type reference. The first public edge-filter surface remains bounded to exact `--id`
roots and is not a general graph-query language.

### Type Graph Query Primitive

Status: specified by T142 as a provider primitive under the existing `syntax related` command
family.

The type graph primitive is a compact graph-oriented provider view rooted at one exact provider id:

```bash
v8-context-hbk syntax related --index <index.sqlite> \
  --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" \
  --graph --format json
```

The primitive does not add a top-level command, transport, service, storage selector, BSL parser,
analyzer implementation or public SQLite table contract. It reads only the resolved prebuilt local
index and reuses the provider response envelope:

- `schema_version` remains `1`;
- `command` remains `related`;
- `query.kind` is `type_graph`;
- `query.root.id` records the exact root id;
- `query.depth` uses the existing bounded traversal depth and is capped at `5`;
- `query.limit`, when present, bounds the total `results[]` array including the root fact;
- `results[0]` is the root fact when the root exists and `limit >= 1`;
- subsequent results are deterministic related facts reachable through the existing bounded
  relationship graph, deduplicated by provider id.

Accepted roots are exact provider ids for platform types, owned member facts and callable facts.
Plain names, owner/member roots and callable-name roots are intentionally not graph roots in this
slice because they can be ambiguous. Callers must resolve those inputs first through the existing
`syntax get` primitives and pass the resulting provider id. Missing roots return
`status: "not_found"` with a `NOT_FOUND` diagnostic. Graph mode with multiple roots, `--edge` or
`--compact` returns `status: "unsupported"` with an `UNSUPPORTED_QUERY` diagnostic. This restriction
applies only to the new `--graph` mode; accepted non-graph behavior such as
`syntax related --compact` and `syntax related --id ... --edge member_of` remains unchanged.

Fact fields stay export-compatible and live under `results[].fact`: `id`, `kind`, `name`, `owner`,
`signatures`, `signatures[].parameters[]`, `types`, `return` and optional `description`. Graph and
resolution details live only under `results[].meta`, including:

- `root`: `true` for the root result and omitted or `false` for related results;
- `depth` and `path` relationship metadata using the same path step shape as `syntax related`;
- `owner_type_id` where the fact has a resolved owner type;
- `target_type_ids` for resolved type-reference targets;
- `type_references`, a deterministic graph metadata array covering fact-level type refs,
  callable-level returns, signature-level returns and signature parameter type refs.

Each `meta.type_references[]` item records the source-backed type-reference spelling and resolution
state without changing export-compatible fact fields:

- `role`: `type`, `return`, `signature_return` or `parameter_type`;
- `name`: source-backed type-reference spelling;
- `status`: `ok`, `unresolved` or `ambiguous`;
- `target_type_id`: present only for `ok`;
- `candidate_type_ids`: present for `ambiguous`;
- `signature_ordinal` and `parameter_ordinal` where the reference belongs to a concrete signature
  or parameter;
- `parameter_name` for parameter type refs;
- `template_binding` when HBK-backed template binding evidence exists.

For ordinary provider commands, diagnostics remain empty when `status` is `ok`. The graph primitive
adds a T142-specific recoverable diagnostic exception: if the root exists but graph type-reference
metadata contains unresolved or ambiguous targets, the response still has `status: "ok"` and may
include deterministic `UNRESOLVED_TYPE_REFERENCE` and `AMBIGUOUS_TYPE_REFERENCE` diagnostics. These
diagnostics are graph-quality metadata, not lookup failure. They include the source fact id, role,
type-reference name and candidate ids for ambiguous references. They must not expose HBK paths, TOC
paths, HTML paths, page titles, SQLite rowids, FTS terms or storage table names.

The first accepted UAT root is the SKD filter property
`type_property:platform_type:НастройкиКомпоновкиДанных:Отбор`. One graph response must expose the
property itself, the referenced `ОтборКомпоновкиДанных` type, its `Элементы` property, the
`КоллекцияЭлементовОтбораКомпоновкиДанных.Добавить` callable and the documented
`ЭлементОтбораКомпоновкиДанных` fields needed by the expression-chain provider scenario.

T145 broadened the consumer UAT roots without changing the graph contract or storage shape. The
additional accepted roots are:

- `type_method:platform_type:Запрос:Выполнить`, which must let an analyzer-style consumer traverse
  from query execution to `РезультатЗапроса`, `РезультатЗапроса.Выбрать`,
  `ВыборкаИзРезультатаЗапроса`, iteration and field-access facts in one bounded graph call.
- `type_method:platform_type:HTTPСоединение:Получить`, which must traverse from an HTTP request call
  to `HTTPОтвет`, status/header properties and body-access methods in one bounded graph call.
- `type_method:platform_type:ДвоичныеДанные:ОткрытьПотокДляЧтения`, which must traverse from binary
  data stream opening to `Поток`, read/close methods and readable-stream capability facts in one
  bounded graph call.

These scenarios are acceptance coverage for externally observable provider JSON only. They do not
stabilize SQLite tables, traversal internals or a downstream analyzer API, and they preserve the
T142 unsupported combinations for graph mode.

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
