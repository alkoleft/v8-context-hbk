# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history:

- [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md)
- [archive/completed-tasks-t13-t17-t19-t24.md](archive/completed-tasks-t13-t17-t19-t24.md)
- [archive/completed-tasks-t25-t34.md](archive/completed-tasks-t25-t34.md)
- [archive/implementation-todo-2026-05-04.md](archive/implementation-todo-2026-05-04.md)
- [archive/completed-tasks-t41-t47.md](archive/completed-tasks-t41-t47.md)

Current status: T35-T47 and the T18 first slice are archived historical tasks. Their durable
export, schema, data-quality, performance, parser and query-search conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`implementation/components.md`, `implementation/syntax-helper-query-cli.md` and
`implementation/syntax-bsl-provider-plan.md`.
Next active unchecked task is T53. T50 defines the provider response contract, T51 was completed by
the same schema-v3 structured callable fact mechanism as T48, and T52 added analyzer-safe identity
roots for query/provider traversal. T49 is intentionally parked until BSL task scenario UAT is fixed
by T53. The queued roadmap comes from
`implementation/syntax-bsl-provider-plan.md`. All `syntax` scope work is oriented toward successful
help during BSL development and code analysis, and toward a future typed local provider role for a
BSL analyzer.

### [x] T48. Separate structured parameters from search terms in query JSON

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

### [x] T50. Define export-compatible provider response contract

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

### [x] T51. Preserve structured callable facts in the query index

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

### [x] T52. Add analyzer-safe identity query roots

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

### [ ] T53. Add BSL task scenario UAT

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

### [ ] T49. Evaluate Tantivy against the current SQLite/FTS5 query index

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- NFR-QUERY-001
- NFR-PERF-001
- UAT-SH-004
- UAT-SH-006
- UAT-SH-015
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
- measured RU/root build comparison for current SQLite/FTS5 and Tantivy prototype
- measured query comparison for all comparison cases above

### [ ] T54. Improve relationship coverage from accepted BSL scenarios

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

### [ ] T55. Decide downstream provider boundary for BSL analyzers

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
