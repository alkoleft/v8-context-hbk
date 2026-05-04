# Completed Implementation Tasks T48-T56

This archive preserves completed task history moved out of the active implementation ledger.
It is evidence, not active implementation scope.

Raw command logs, generated exports, temporary probes and `target/...` paths are service data and
are intentionally omitted from this archive. Durable provider, storage, query-index and BSL-task
conclusions live in `../acceptance/baseline.md`, `../implementation/syntax-helper-query-cli.md` and
`../implementation/syntax-bsl-provider-plan.md`.

For active task sequencing, use `../IMPLEMENTATION_TODO.md`.

## T48. Separate structured parameters from search terms in query JSON

Spec refs:

- FR-SH-SEARCH-001
- UAT-SH-006
- ADR-0006
- `spec/implementation/syntax-helper-query-cli.md`

Prompt:

```text
Investigate and fix `syntax constructors <TYPE> --format json` output where
`document.parameters` is a flat list that mixes parameter names with type names, for example:
`["Сервер", "Строка", "Порт", "Число", ...]`.

Start from `spec/README.md`, `spec/requirements/functional.md`,
`spec/implementation/syntax-helper-query-cli.md`, `spec/acceptance/uat-test-cases.md` and the active
task ledger. Treat the `syntax` goal as practical BSL development/code-analysis assistance and a
future typed local provider for a BSL analyzer. Keep the fix scoped to the search/query layer unless
source evidence shows a parser problem.

Current evidence:
- The parser/domain model is structured: `Signature.parameters: Vec<Parameter>` and each
  `Parameter` has `name`, `required`, `type_refs`, and `description`.
- `syntax-helper-search::SearchDocument.parameters` is currently `Vec<String>`.
- `syntax-helper-search::document()` builds that list by chaining each parameter name with all of
  that parameter's type names. This is useful as internal searchable text, but misleading as public
  JSON.

Expected outcome:
- Public JSON must not expose a field named `parameters` that mixes parameter names and type names.
- For shared callable facts, prefer the existing `syntax export` shape over a new query-only shape:
  `signatures[].parameters[]` should expose `name`, `required`, `types` and optional `description`
  when those facts are available.
- Keep raw search terms internal and do not serialize them as the public parameter contract.
- Preserve existing text output behavior for `syntax constructors <TYPE>`.
- Preserve SQLite rebuild determinism. If the SQLite schema changes, document it in spec/baseline
  and update tests.
- Add focused tests that cover `HTTPСоединение` constructor JSON and assert names and types are not
  interleaved in one ambiguous array.

Verification:
- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- rebuild a real index from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `v8-context-hbk syntax constructors --index <rebuilt.sqlite> "HTTPСоединение" --format json`
```

Scope:

- Fix the public query JSON contract for callable parameters in search documents.
- Align shared callable fact shapes with `syntax export` JSON rather than preserving the current
  provisional query JSON shape.
- Keep search-only tokenization internal to the index/search implementation.
- Do not revisit the T47 HTML section-boundary parser fix unless a regression test proves it is
  still involved.

Result:

- Public query JSON no longer exposes mixed `document.parameters` for callable facts.
- `syntax-helper-search` schema version `3` stores structured `signature_json` and keeps raw
  parameter/type search terms internal to `parameter_text` / `document_search.parameters`.
- `syntax constructors "HTTPСоединение" --format json` exposes structured
  `signatures[].parameters[]` with `name`, `required`, `types` and optional `description`; compact
  and detailed text output still prints signature text.
- Verified with `cargo test -p syntax-helper-search --lib`, `cargo test --workspace`, real RU index
  rebuild from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`, JSON `jq` assertions and read-only
  schema/count inspection (`schema_version=3`, `documents=document_search=document_fts=25082`).

## T50. Define export-compatible provider response contract

Spec refs:

- FR-SH-PROVIDER-001
- ADR-0006
- UC-SH-005D
- `spec/implementation/syntax-bsl-provider-plan.md`

Scope:

- Define the provisional query/provider JSON response envelope for `syntax get`, `syntax
  constructors`, `syntax search` and `syntax related`.
- Use `syntax export` field names and shapes for shared platform facts wherever applicable.
- Define where query-only metadata belongs: score, depth, relationship path, ambiguity and missing
  result diagnostics.
- State that compatibility with the current provisional `SearchHit<SearchDocument>` JSON is not a
  goal when it conflicts with export-compatible provider facts.

Verification:

- Updated `spec/requirements/functional.md`, `spec/implementation/syntax-helper-query-cli.md`,
  `spec/implementation/syntax-bsl-provider-plan.md` and UAT cases.
- Review sample JSON for `HTTPСоединение` and `НастройкиКомпоновкиДанных.Отбор` against the
  proposed contract before implementation.

Result:

- Defined the provisional provider response envelope for `syntax get`, `syntax constructors`,
  `syntax search` and `syntax related` in `spec/implementation/syntax-helper-query-cli.md`.
- Provider JSON target uses provider `schema_version: 1`, `command`, `status`, normalized `query`,
  deterministic `results[]` and `diagnostics[]`.
- Shared platform facts live under `results[].fact` and use export-compatible field names such as
  `owner`, `signatures`, `signatures[].parameters[]`, `types` and `return`; query-only metadata
  such as search score/rank, relationship depth/path and richer owner identity lives under
  `results[].meta`.
- Missing and ambiguous exact lookups are represented through `status` and diagnostics instead of a
  hidden winner.
- Reviewed current sample JSON for `HTTPСоединение` constructors and
  `НастройкиКомпоновкиДанных.Отбор`; those samples are still the pre-envelope
  `SearchHit<SearchDocument>` implementation shape, so implementation remains for a follow-up task.

## T51. Preserve structured callable facts in the query index

Spec refs:

- FR-SH-PROVIDER-001
- UC-SH-005A
- `spec/implementation/syntax-bsl-provider-plan.md`

Scope:

- Preserve or reconstruct export-compatible structured signatures in query output for constructors,
  methods and events.
- Keep internal FTS/search tokens separate from public JSON.
- Keep text output behavior stable unless the task explicitly updates CLI text UX.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- Rebuilt real RU index exposes `HTTPСоединение` constructor parameters as structured facts with
  `name`, `required`, `types` and optional `description`.

Result:

- Completed by the T48 schema-v3 query-index change. `SearchDocument.signatures` now stores
  structured signatures for methods, constructors and events; `documents.signature_json` preserves
  those facts across SQLite rebuild/read-only query; `signature_text` remains presentation/FTS data.
- Raw parameter/type search terms remain internal to `parameter_text` / `document_search.parameters`
  and are not serialized as public `document.parameters`.
- Verified by the T48 focused SQLite round-trip test, full workspace tests and real RU constructor
  JSON assertions for `HTTPСоединение`.

## T52. Add analyzer-safe identity query roots

Spec refs:

- FR-SH-PROVIDER-001
- FR-SH-SEARCH-002
- UC-SH-005B
- UC-SH-005D

Scope:

- Add query entry points needed to resolve ambiguous facts deterministically, such as document-id
  lookup and owner/member roots for relationship traversal.
- Preserve current simple name-based CLI UX for human use.
- Define explicit ambiguity and missing-result JSON behavior.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- UAT covers relationship traversal from `НастройкиКомпоновкиДанных.Отбор` without relying on an
  ambiguous plain-name root.

Result:

- `syntax-helper-search` now exposes document-id lookup plus relationship traversal from document id
  and owner/member roots.
- `v8-context-hbk syntax related` accepts `--id`, `--name` or `--owner --member`; the existing
  plain-name human UX remains, while analyzer workflows can use exact roots.
- JSON output for `syntax get`, `syntax constructors`, `syntax search` and `syntax related` now uses
  provider `schema_version: 1` with `command`, `status`, `query`, `results[]` and `diagnostics[]`,
  including `UNSUPPORTED_QUERY` diagnostics for invalid JSON root combinations.
- Shared platform facts are emitted under `results[].fact` with export-compatible names such as
  `types`, `return`, `signatures` and owner string; rank, score, relationship depth and path are
  emitted under `results[].meta`.
- Missing and ambiguous lookups are represented by `status: "not_found"` / `"ambiguous"` and
  diagnostics, not by empty bare arrays or hidden winner selection.
- Verified with focused search crate tests, full workspace tests, real RU index rebuild from
  `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`, and provider JSON `jq` assertions for constructor,
  get-by-id, owner/member, search, related-by-name, related-by-id, related-by-owner-member,
  missing, ambiguous and unsupported lookup paths.

## T53. Add BSL task scenario UAT

Spec refs:

- UC-SH-005A
- UC-SH-005B
- UC-SH-005C
- ADR-0006

Scope:

- Add source-backed BSL development scenarios that validate the utility against real code-analysis
  questions.
- Start with constructor lookup for `HTTPСоединение`, owner/member lookup for SKD filter access and
  task-oriented search/relationship discovery for one query-table or register-table scenario.
- Keep raw scenario run outputs under `target/`; promote only stable commands, assertions and
  conclusions into UAT/baseline.

Verification:

- Updated `spec/acceptance/uat-test-cases.md`.
- UAT commands are reproducible with a rebuilt local Syntax Assistant index.

Result:

- Added UAT-SH-017 as the accepted BSL task scenario corpus for ADR-0006 provider workflows.
- The scenario rebuilds a real RU index under `target/uat/`, then verifies constructor-call
  assistance for `HTTPСоединение`, exact owner/member lookup and relationship traversal for
  `НастройкиКомпоновкиДанных.Отбор`, and task-oriented query-table discovery for
  `таблица регистра бухгалтерии`.
- Promoted the durable T53 conclusion into `spec/acceptance/baseline.md`; raw JSON and SQLite
  artifacts remain service data under `target/uat/`.
- Verified against `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`: rebuilt index produced `25082`
  documents in `54105 ms`, and the UAT-SH-017 `jq` assertions passed.

## T49. Evaluate Tantivy against the current SQLite/FTS5 query index

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- NFR-QUERY-001
- NFR-PERF-001
- UAT-SH-004
- UAT-SH-006
- UAT-SH-015
- UAT-SH-017
- ADR-0006
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/acceptance/baseline.md`

Problem:

- After T42-T44, the current single-file SQLite index keeps exact lookup, relations and FTS in one
  local artifact, but `syntax index` remains materially slower than `syntax export`.
- SQLite FTS5 is single-writer and convenient, while Tantivy may offer faster/fuller full-text
  indexing and search-specific ranking primitives. This needs source-backed measurement, not a
  speculative storage rewrite.
- Choosing Tantivy only for speed is not enough: the accepted query workflows depend on exact
  lookup, aliases, owner/member lookup, constructor lookup, deterministic JSON, relationship
  traversal, local rebuildability and simple artifact operations.

Scope:

- Build a measured prototype that indexes the same `SearchDocument` facts with Tantivy and compares
  it with the current SQLite/FTS5 implementation.
- Keep the current SQLite index path as the control baseline. Do not replace the production query
  index until the comparison proves a better choice against the criteria below.
- Evaluate at least two integration shapes:
  - Tantivy only for keyword/fuzzy full-text search while SQLite keeps exact lookup and relations;
  - Tantivy as the primary query artifact only if exact lookup, constructor lookup and relations can
    be preserved without worse complexity or behavior.
- Do not commit large generated Tantivy artifacts. Store raw run outputs under `target/` and promote
  only conclusions, commands and metrics into `spec/acceptance/baseline.md`.
- Do not add a user-facing storage selection knob until a winner is chosen. If no option clearly
  wins, record SQLite/FTS5 as retained and close the task with the measured reason.

Comparison cases:

- Build both indexes from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
  `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`.
- Exact lookup: `ОтборКомпоновкиДанных`, `DataCompositionFilter`, owner/member
  `НастройкиКомпоновкиДанных.Отбор`.
- Constructor lookup: `HTTPСоединение`, including compact text, detailed text and deterministic JSON
  output after T48.
- Keyword search: `отбор скд`, `HTTP соединение`, `таблица регистра бухгалтерии`.
- Fuzzy search: `ОтборКомпоновкиДаных`, plus one English typo from the root source.
- Relationship traversal: `ОтборКомпоновкиДанных` and one constructor/type relationship case.
- BSL task scenario UAT: UAT-SH-017 constructor, owner/member, relationship and
  accounting-register query-table assertions.
- Ambiguity behavior: same-name owner/member or enum cases already covered by search-index tests.

Decision criteria:

- Build elapsed time, peak RSS and artifact size for RU and root sources.
- Query latency for exact, constructor, keyword, fuzzy and relationship workflows.
- Result quality compared with the accepted SQLite/FTS5 output class: top hits, deterministic order,
  ambiguity handling and no loss of aliases/owners/type refs.
- Artifact and operational complexity: one file vs directory/segments, atomic rebuild story,
  cleanup behavior, read-only concurrent queries, packaging and future migration burden.
- Implementation complexity and dependency risk inside the Rust workspace.

Expected artifacts:

- A short implementation/measurement note in `spec/implementation/syntax-helper-query-cli.md`
  describing the compared shapes and selected direction.
- Updated `spec/acceptance/baseline.md` with the comparison table and winner/retention decision.
- Focused prototype code or benchmark harness if needed, kept scoped and either promoted into the
  chosen implementation or removed before task completion.
- A clear final decision: keep SQLite/FTS5, adopt Tantivy for FTS only, adopt Tantivy more broadly,
  or defer because the measured delta does not justify the added complexity.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- UAT-SH-017 provider workflow assertions against the rebuilt RU index
- measured RU/root build comparison for current SQLite/FTS5 and Tantivy prototype
- measured query comparison for all comparison cases above

Result:

- Used a measurement-only Tantivy sidecar prototype during T49. It read accepted SQLite
  `documents` / `document_search` rows, wrote a Tantivy directory under `target/`, and reported
  keyword/fuzzy measurements as JSON without changing production CLI behavior. The prototype code
  and dependency were removed before task completion because Tantivy was not selected.
- Release-profile SQLite rebuilds from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
  `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` produced the accepted schema-v3 row counts
  (`25082`/`25062` documents, `65455`/`68670` relations). The Tantivy sidecars were much smaller
  and fast to build from already materialized SQLite rows, but they did not cover exact lookup,
  owner/member lookup, constructor lookup, deterministic provider JSON or relationship traversal
  without SQLite.
- Query measurements stayed within NFR-QUERY-001 for SQLite/FTS5. UAT-SH-017 assertions passed for
  `HTTPСоединение` constructor parameters, `НастройкиКомпоновкиДанных.Отбор` traversal,
  accounting-register query-table discovery, root English fuzzy typo lookup and a constructor/type
  relationship case from `HTTPСоединение`.
- Retained the single SQLite/FTS5 query artifact. Tantivy is not adopted for primary storage or
  FTS-only sidecar in this task because the prototype lost accepted workflow quality: fuzzy
  `ОтборКомпоновкиДаных` returned no hits and `таблица регистра бухгалтерии` ranked generic
  accounting-register table variants above the accepted UAT-SH-017 top hit.

## T54. Improve relationship coverage from accepted BSL scenarios

Spec refs:

- FR-SH-SEARCH-002
- FR-SH-PROVIDER-001
- UC-SH-005C

Scope:

- Add only the parser facts or graph edges required by failed accepted BSL task scenarios.
- Prefer structured Syntax Assistant facts and links over prose-only heuristics.
- Keep relationship traversal deterministic and local.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test -p syntax-helper-extract --lib` when parser facts change
- `cargo test --workspace`
- UAT task scenarios pass without broad ranking regressions.

Result:

- Prioritized structured type-reference and return-type relation edges ahead of the reverse
  `member_of` edge during SQLite relation traversal.
- This keeps `syntax related --owner "НастройкиКомпоновкиДанных" --member "Отбор"` moving forward
  through the BSL type chain before expanding the owning settings object, so the accepted SKD
  scenario reaches `ОтборКомпоновкиДанных.Элементы`,
  `КоллекцияЭлементовОтбораКомпоновкиДанных.Добавить` and `ЭлементОтбораКомпоновкиДанных` fields
  inside the existing bounded graph query.
- No parser facts, SQLite schema, public provider JSON shape, storage engine or search sidecar were
  added.
- Verified with `cargo test -p syntax-helper-search --lib`, `cargo test --workspace`, real RU index
  rebuild from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and stricter UAT-SH-017 `jq`
  assertions.

## T55. Decide downstream provider boundary for BSL analyzers

Spec refs:

- ADR-0001
- ADR-0006
- FR-SH-PROVIDER-001

Scope:

- Decide whether the analyzer-facing provider is CLI JSON only, a Rust library API, a file artifact
  contract, or a combination.
- Capture a new ADR if this changes integration architecture or creates a long-lived public API
  boundary.
- Keep the decision compatible with the accepted `syntax export` shapes for shared facts.

Verification:

- ADR or implementation spec records the selected boundary, non-goals and verification path.
- No BSL parser/analyzer implementation is added as part of the boundary decision.

Result:

- Accepted ADR-0007 and selected local CLI JSON over a prebuilt `syntax` index as the first
  downstream analyzer-provider boundary.
- The public boundary is the provider JSON envelope returned by `syntax get`, `syntax
  constructors`, `syntax search` and `syntax related`; the SQLite index remains a rebuildable
  internal provider artifact, not a table-level integration contract.
- Rust library APIs, analyzer-specific file artifacts, daemon/MCP/service boundaries and bulk APIs
  remain future decisions that require a concrete consumer and separate ADR/task.
- No BSL parser, analyzer diagnostics, runtime 1C introspection, new storage selector or provider
  JSON shape change was added.

## T56. Normalize query-index storage for analyzer type inference

Spec refs:

- ADR-0006
- UC-SH-005A
- UC-SH-005B
- UC-SH-005D
- FR-SH-PROVIDER-001
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/implementation/syntax-bsl-provider-plan.md`

Problem:

- The current SQLite schema version `3` is sufficient for CLI lookup/search/provider JSON, but it
  still stores analyzer-critical callable details in `documents.signature_json` and stores
  parameter/type/return references as text fields for FTS or presentation.
- Future BSL analyzer use cases need deterministic type inference, expression-chain evaluation and
  member completion. Those workflows should query typed relational facts, not parse JSON blobs or
  infer meaning from `parameter_text`, `type_names`, `return_names` or FTS rows.
- The current `documents.preview` column duplicates `description` as a truncated 180-character
  presentation value and should be removed or generated at presentation time unless measurement
  proves it is needed in the SQLite artifact.

Scope:

- Design and implement the next SQLite schema revision for analyzer-oriented facts without JSON
  columns as the source of truth for inference-critical data.
- Add relational tables for:
  - canonical type identities and aliases;
  - owned members by owner type id and member kind;
  - callables, signatures and ordered parameters;
  - typed references for property types, parameter types, return types, constructor result types and
    other source-backed inference edges.
- Keep `documents`, `document_names`, `document_search`/`document_fts` and `relations` only where
  they still serve provider/search/graph workflows.
- Remove or confine redundant/presentation-only fields where possible:
  - remove `documents.preview` or generate it outside storage;
  - avoid keeping both document-level and FTS-level copies of searchable text unless each has a
    distinct query/provider role;
  - replace `documents.signature_json` for provider output with assembly from normalized
    signature/parameter/type-reference rows;
  - keep `signature_text`, `parameter_text`, `type_names` and `return_names` as FTS/presentation
    inputs only if they are generated from normalized rows and not treated as analyzer truth.
- Do not add a BSL parser, linter or diagnostics engine in this task.
- Do not add compatibility import/migration for old generated indexes; stale indexes may be rejected
  by schema version with a rebuild instruction.

Expected outcome:

- Analyzer-relevant questions can be served from relational tables:
  - resolve constructor overloads and parameter type references;
  - resolve owner/member access to a typed member fact;
  - list members for a platform type identity;
  - follow return/property/parameter type references across expression chains.
- Provider JSON remains export-compatible, but it is assembled from normalized storage rather than
  stored as JSON in SQLite.
- The implementation spec records the final schema shape and explains which fields remain for
  search/presentation only.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- rebuild a real RU index from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- UAT-SH-017 provider workflow assertions still pass against the rebuilt index
- targeted SQL or library assertions prove that constructor signatures, parameter types,
  owner/member facts and return/property type refs are present in normalized tables
- inspect the rebuilt schema to confirm analyzer-critical tables do not use JSON columns

Result:

- `syntax-helper-search` schema version `4` stores analyzer-critical facts in normalized
  `type_identities`, `members`, `callables`, `signatures`, `parameters` and `type_refs` tables.
- Removed `documents.signature_json` and `documents.preview`; provider output is assembled from
  relational signature/parameter/type-reference rows, while compact preview text is generated from
  `description` after read.
- Kept `documents`, `document_names`, `document_search` / `document_fts` and `relations` as the
  provider/search/graph projections. FTS text remains internal and is not exposed as public JSON.
- No old-index migration compatibility was added; stale generated indexes are rejected by schema
  version and must be rebuilt.
- Verified with `cargo test -p syntax-helper-search --lib`, `cargo test --workspace`, a real RU
  index rebuild from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` (`25082` documents in `55459 ms`),
  unchanged UAT-SH-017 provider assertions, read-only SQL inspection of normalized tables, and a
  regression check that duplicate type names do not receive a hidden `target_type_id` winner.
