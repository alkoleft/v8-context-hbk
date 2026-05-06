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

Current status: T35-T90 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design,
language-domain and cleanup conclusions live in `acceptance/baseline.md`,
`source-evidence.md`, `requirements/functional.md`, `requirements/non-functional.md`,
`implementation/components.md`, `implementation/syntax-helper-query-cli.md`,
`implementation/syntax-bsl-provider-plan.md` and `implementation/solution-context-resolve.md`.

The active open tasks are T99-T103. T98 is complete; T99 is now the first unchecked planned task.
T99-T103 are user-requested book-content export slices handled outside the first-unchecked
cleanup/provider sequence.

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

### [x] T91. Collapse duplicated localized display-name presentation helper

Spec refs:

- T87
- `spec/implementation/components.md`

Problem:

- T87 classified most duplicate-looking mechanisms as accepted boundary separation or already
  covered by archived cleanup evidence.
- The remaining stale duplication is a small localized-name `display_name` helper duplicated in
  `syntax-helper-search` and `v8-context-hbk-cli`.
- The helper is presentation logic only, but keeping the same rule in two crates makes later
  presentation changes easy to drift.

Scope:

- Move the localized-name display rule to one narrow shared helper in an existing crate that both
  `syntax-helper-search` and `v8-context-hbk-cli` already depend on.
- Replace the duplicated local helpers with calls to the shared helper.
- Preserve current human text output, search document text, relation labels and provider JSON.
- Do not change lookup keys, search ranking, provider envelopes, export JSON, resolver facts,
  SQLite schema or public identity rules.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

Completion notes:

- Added the single localized-name presentation helper to `syntax-helper-model::LocalizedName`.
- Replaced the duplicated local helpers in `syntax-helper-search` and `v8-context-hbk-cli` with
  calls to the shared model helper.
- Preserved current human text output, search document text, relation labels and provider JSON.
- Verification passed:
  - `cargo test -p syntax-helper-search --lib`
  - `cargo test -p v8-context-hbk-cli`
  - `cargo test --workspace`

### [x] T92. Remove hidden resolver edge and constructor return fallbacks

Spec refs:

- ADR-0008
- FR-CTX-RESOLVE-001
- `spec/implementation/solution-context-resolve.md`

Problem:

- The platform resolver adapter still has fallback behavior after the first source-backed edge
  implementation:
  - `edge_refs()` falls back from edge-specific traversal to generic `related_by_id()` filtering;
  - `related()` repeats the same generic traversal fallback;
  - constructor callable mapping synthesizes a return type from the owner when explicit
    `constructs`/return data is absent.
- ADR-0008 and the resolver implementation notes require identity-preserving, source-backed
  relation traversal. Hidden fallback edges make it harder to distinguish missing source evidence
  from real resolver facts.

Scope:

- Remove the generic traversal fallback from platform adapter edge-specific relation lookup.
- Remove constructor return-type synthesis from owner name unless a spec update explicitly records
  it as a source-backed rule.
- Adjust tests so missing `returns`/`constructs` evidence is observable as missing relation/return
  data, not as a synthesized fact.
- Preserve current source/domain identity, query-table exclusion, resolver public types and
  `syntax-helper-search` relation storage.

Verification:

- `cargo test -p context-resolver-search`
- `cargo test --workspace`

Completion notes:

- Removed the platform adapter generic `related_by_id()` fallback from callable edge refs and
  public `related()` traversal.
- Removed constructor return-type synthesis from the owner name in resolver callable mapping.
- Added focused coverage where constructor `constructs` evidence is removed from the fixture index;
  resolver callable lookup now exposes no return/result type and `constructs` traversal returns an
  empty relation set.
- Preserved resolver public types, query-table exclusion, SQLite schema and `syntax-helper-search`
  relation storage.
- Verification passed:
  - `cargo test -p context-resolver-search`
  - `cargo test --workspace`

### [x] T93. Keep provider JSON assembly out of `syntax-helper-search` structs

Spec refs:

- ADR-0007
- `spec/implementation/components.md`
- `spec/implementation/syntax-helper-query-cli.md`

Problem:

- T87 classifies provider JSON DTO shaping as a CLI boundary, but `SearchSignature` and
  `SearchParameter` still derive serde traits and `v8-context-hbk-cli` serializes
  `SearchDocument.signatures` directly into provider JSON.
- A `syntax-helper-search` test still describes structured signatures as public JSON. That keeps a
  stale provider DTO concern in the search/index crate after T72/T86 cleanup.

Scope:

- Remove serde derives and serde field attributes from search result nested structs when no longer
  needed by `syntax-helper-search`.
- Move provider signature/parameter JSON assembly into `v8-context-hbk-cli`, preserving the current
  provider envelope and export-compatible fact field names.
- Update tests so provider JSON shape is asserted at the CLI/provider boundary, while
  `syntax-helper-search` tests assert Rust query structs and normalized storage behavior.
- Do not change provider response schema version, SQLite schema, search ranking, lookup behavior or
  export JSON.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

Completion notes:

- Removed serde derives and serde field attributes from `SearchSignature` and `SearchParameter`.
- Removed the `syntax-helper-search` serde/serde_json dependency edge that existed only for direct
  provider DTO serialization.
- Replaced the stale search-crate provider-JSON assertion with Rust struct and normalized storage
  assertions.
- Moved signature/parameter provider JSON assembly into `v8-context-hbk-cli`, preserving
  export-compatible `signatures[].parameters[]`, `types`, optional `description` and omission of
  signature `text` from provider facts.
- Provider response schema version, SQLite schema, search ranking, lookup behavior and export JSON
  are unchanged.
- Verification passed:
  - `cargo test -p syntax-helper-search --lib`
  - `cargo test -p v8-context-hbk-cli`
  - `cargo test --workspace`

### [x] T94. Deduplicate search relation graph construction

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-PROVIDER-001
- `spec/implementation/components.md`

Problem:

- `syntax-helper-search` keeps production relation insertion and the test-only
  `relations_from_documents()` helper as two separate implementations of the same relation graph
  rules.
- Tests that call the duplicate helper can drift from the SQLite insertion path and accidentally
  verify a copied algorithm instead of the actual indexed relation output.

Scope:

- Extract one narrow relation-building function or iterator that produces typed relation rows from
  search documents.
- Use that shared builder for SQLite relation insertion and focused tests.
- Preserve current relation edge kinds, labels, evidence, weights and deduplication semantics.
- Do not change SQLite schema, query result ordering, provider JSON or resolver facts.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test -p context-resolver-search`
- `cargo test --workspace`

Completion notes:

- Extracted one shared streaming relation-row visitor in `syntax-helper-search` for owner/member,
  constructor, type-reference and return-reference edges.
- SQLite relation insertion and focused relation tests now use the same builder and the same
  `(source_id, target_id, edge_kind)` deduplication key.
- Added a focused parity test that compares stored SQLite `relations` rows with the shared builder
  output for the fixture document set.
- Preserved current edge kinds, labels, evidence, weights, SQLite schema, query ordering, provider
  JSON and resolver facts.
- Verification passed:
  - `cargo test -p syntax-helper-search --lib`
  - `cargo test -p context-resolver-search`
  - `cargo test --workspace`

### [x] T95. Replace stringly search document kinds with a typed internal kind

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-PROVIDER-001
- FR-CTX-RESOLVE-001
- `spec/implementation/components.md`

Problem:

- Search document kinds such as `platform_type`, `type_property`, `language_function` and
  `query_table_field` are repeated as raw strings across search indexing, kind priority,
  language-fact mapping, CLI filtering and resolver adapter mapping.
- This duplicates the same closed set of internal index document families and makes new fact
  families easy to add inconsistently.

Scope:

- Introduce an internal typed document-kind model in `syntax-helper-search` with explicit string
  conversion at SQLite/provider boundaries.
- Replace local string matches in search/index logic and resolver adapter mapping where the typed
  kind can cross the crate boundary without expanding public JSON contracts.
- Preserve existing stored string values, provider `kind` values, search ordering, resolver fact
  mapping and export JSON.
- Do not turn this into a generic cross-domain ontology or public stable enum unless the spec is
  updated first.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test -p context-resolver-search`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

Completion notes:

- Added `syntax-helper-search::SearchDocumentKind` as the typed in-workspace document-kind model.
- Converted Syntax Assistant and language fact builders, SQLite row hydration, normalized fact
  insertion, relation construction, ranking priority and resolver adapter mapping to use the typed
  kind internally.
- Kept SQLite `documents.kind` values and provider JSON `kind` values as the existing strings at
  explicit boundary conversions.
- Added focused kind round-trip/priority coverage and retained resolver coverage proving
  `query_table*` provider facts stay hidden from the platform resolver adapter.
- Preserved search ordering, resolver fact mapping, provider JSON, SQLite schema and export JSON.
- Verification passed:
  - `cargo test -p syntax-helper-search --lib`
  - `cargo test -p context-resolver-search`
  - `cargo test -p v8-context-hbk-cli`
  - `cargo test --workspace`

### [x] T96. Narrow or retire `HbkBook::read_pages` test-support API

Spec refs:

- FR-HBK-002
- FR-HBK-003
- `spec/implementation/components.md`

Problem:

- The supported ordinary page/file surface is `read_file`, `read_page` and `FileStorageReader`.
  `HbkBook::read_pages` is currently gated behind `test-utils`, and the only in-repo use is a
  deterministic unit test.
- Keeping this as a public feature API makes a non-contract convenience method look like a supported
  book-reading surface.

Scope:

- Either move `read_pages` into test-only support or remove it in favor of direct
  `FileStorageReader` usage in tests.
- Preserve production `HbkBook::open`, `read_file`, `read_page` and `FileStorageReader` behavior.
- Do not reintroduce retained `FileStorage` bytes or broader in-memory book state.

Verification:

- `cargo test -p hbk-book`
- `cargo test --workspace`

Completion notes:

- Removed the gated `HbkBook::read_pages` test/support convenience API.
- Preserved production `HbkBook::open`, `read_file`, `read_page` and `FileStorageReader` behavior.
- Kept deterministic repeated-page coverage on the supported `FileStorageReader` surface.
- Verification passed:
  - `cargo test -p hbk-book`
  - `cargo test --workspace`

### [x] T97. Deduplicate first language callable parsing paths

Spec refs:

- ADR-0008
- FR-CTX-RESOLVE-001
- `spec/implementation/solution-context-resolve.md`

Problem:

- `syntax-helper-language` now has two similar callable extraction paths:
  `query_function_fact()` for `shquery_*` functions and `extract_dcsui_string_functions()` for the
  first SKD string function fixture.
- Both produce `LanguageFactFamily::Function` facts with syntax, signatures, parameters and return
  or type references, but the DCSUI path uses ad hoc body slicing.

Scope:

- Extract a small shared helper for language callable fact assembly where the source shapes already
  match.
- Keep source-family-specific page discovery, page-key matching and fixture expectations separate.
- Preserve current source-qualified ids, language domains, fact families, signatures, parameter
  names, return/type refs and provenance.
- Do not expand parser coverage beyond the existing fixture-backed pages in this task.

Verification:

- `cargo test -p syntax-helper-language`
- `cargo test -p syntax-helper-search --lib`
- `cargo test -p context-resolver-search`
- `cargo test --workspace`

Completion notes:

- Extracted one shared `syntax-helper-language` callable fact assembly helper for language function
  facts.
- Kept `shquery_*` and `dcsui_*` page discovery, page-key matching, syntax extraction and fixture
  expectations source-family-specific.
- Preserved current source-qualified ids, language domains, fact families, signatures, parameter
  names, return/type refs and provenance, including the `SKD_Functions_Strings#StringLength`
  anchor.
- Added focused fixture assertions for structured query-function and SKD string-function
  signatures/parameters.
- Verification passed:
  - `cargo test -p syntax-helper-language`
  - `cargo test -p syntax-helper-search --lib`
  - `cargo test -p context-resolver-search`
  - `cargo test --workspace`

### [x] T98. Rename Syntax Assistant export crate to `hbk-syntax-export`

Spec refs:

- ADR-0009
- FR-EXPORT-001
- `spec/implementation/components.md`

Scope:

- Rename the existing Syntax Assistant JSON export crate from `hbk-export` to
  `hbk-syntax-export`.
- Update workspace membership, dependency aliases, package names and Rust imports.
- Preserve `v8-context-hbk syntax export` behavior and `schema_version` unchanged.
- Do not add ordinary book export behavior in this task.

Verification:

- `cargo test -p hbk-syntax-export`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

Completion notes:

- Renamed the Syntax Assistant JSON export crate directory and package from `hbk-export` to
  `hbk-syntax-export`.
- Updated workspace membership, workspace dependency alias, CLI dependency alias, Rust import path
  and Cargo.lock package/dependency entries.
- Kept `v8-context-hbk syntax export` command shape and consumer JSON `schema_version` unchanged.
- Added no ordinary book-content export behavior to `hbk-syntax-export`; T99 remains the first
  unchecked ordinary book export crate-boundary task.
- Verification passed:
  - `cargo test -p hbk-syntax-export`
  - `cargo test -p v8-context-hbk-cli`
  - `cargo test --workspace`

### [x] T99. Add `hbk-book-export` crate boundary and request model

Spec refs:

- UC-HBK-003
- FR-HBK-004
- ADR-0009
- `spec/implementation/components.md`

Scope:

- Add a separate `hbk-book-export` crate for ordinary book-content export responsibility.
- Define typed request, format, hierarchy, result and error types.
- Implement safe output-root validation and unsupported format/hierarchy diagnostics.
- Keep the crate dependent only on `hbk-book`, `hbk-docs` and narrow utility dependencies needed by
  export behavior.
- Do not wire the CLI command and do not implement actual file export in this task.

Verification:

- Focused tests for request validation, output-root safety and unsupported combinations.
- Dependency-boundary check proves `hbk-book-export` depends only on `hbk-book`, `hbk-docs` and
  narrow utility dependencies, and does not depend on Syntax Assistant extraction,
  `hbk-syntax-export`, search/index or resolver crates (`cargo tree -p hbk-book-export` or
  equivalent `cargo metadata` query).
- `cargo test -p hbk-book-export`
- `cargo test --workspace`

Completion notes:

- Added `crates/hbk-book-export` and wired it into the workspace plus `Cargo.lock`.
- Defined the typed ordinary book export request/format/hierarchy/exporter/result/error boundary.
- Implemented output-root validation at the public request boundary: roots must contain at least one
  directory name and must not contain `..` segments.
- Implemented typed unsupported-combination diagnostics for `raw/toc` and `markdown/raw`; request
  validation recognizes only future-supported `raw/raw` and `markdown/toc` combinations.
- Did not wire the CLI command and did not implement actual raw unpack, Markdown conversion or file
  writes.
- Verification passed:
  - `cargo fmt --all --check`
  - `cargo test -p hbk-book-export`
  - `cargo tree -p hbk-book-export --depth 1`
  - `cargo test --workspace`

### [ ] T100. Implement raw/raw book unpack in `hbk-book-export`

Spec refs:

- UC-HBK-003
- FR-HBK-004
- FR-HBK-002
- ADR-0009
- `spec/implementation/components.md`

Scope:

- Implement `format=raw` with `hierarchy=raw` as normalized `FileStorage` unpacking.
- Preserve original stored bytes for exported entries.
- Reject unsafe or escaping storage paths before writing.
- Keep TOC traversal and Markdown conversion out of this task.

Verification:

- Focused tests for raw/raw export behavior on a small deterministic HBK fixture or small real HBK
  source.
- `cargo test -p hbk-book`
- `cargo test -p hbk-book-export`
- `cargo test --workspace`

### [ ] T101. Wire top-level raw export CLI and unsupported matrix diagnostics

Spec refs:

- UC-HBK-003
- FR-HBK-004
- FR-CLI-001
- ADR-0009

Scope:

- Add a top-level `v8-context-hbk export <book.hbk> --output <dir> --format <raw|markdown>
  --hierarchy <raw|toc>` command.
- Wire `format=raw --hierarchy=raw` through `hbk-book-export`.
- Return stable readable diagnostics for unsupported combinations until later tasks implement them.
- Keep `syntax export` unchanged; book-content export must not invoke Syntax Assistant extraction or
  change JSON schema contracts.

Verification:

- Focused CLI tests for raw/raw success and unsupported combination diagnostics.
- `cargo test -p hbk-book-export`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`

### [ ] T102. Implement Markdown conversion for TOC pages

Spec refs:

- UC-HBK-003
- FR-HBK-004
- FR-DOC-001
- ADR-0009
- UAT-HBK-005
- UAT-HBK-006
- UAT-HBK-007
- `spec/implementation/components.md`

Scope:

- Select and add the approved stable HTML-to-Markdown library candidate.
- Implement HTML-to-Markdown conversion for individual TOC pages inside `hbk-book-export`.
- Preserve readable page headings, body text, links, lists, tables and syntax placeholders.
- Ensure normal Markdown output does not leak raw HBK file paths, raw TOC indexes, raw HTML page
  paths or service HTML scaffolding.
- Keep output directory layout and CLI UAT wiring out of this task unless needed for focused tests.

Verification:

- Focused conversion tests based on representative real HBK page HTML from `dcsui_ru.hbk`,
  `shlang_ru.hbk`, `shquery_ru.hbk`, `fmtdui_ru.hbk`, `htmlui_ru.hbk` and `moxelui_ru.hbk`.
- Focused conversion tests assert normal Markdown contains no raw HBK file paths, raw TOC indexes,
  raw HTML page paths or service HTML scaffolding.
- Dependency-boundary check proves `hbk-book-export` does not depend on Syntax Assistant
  extraction, `hbk-syntax-export`, search/index or resolver crates (`cargo tree -p hbk-book-export`
  or equivalent `cargo metadata` query).
- `cargo test -p hbk-book-export`
- `cargo test --workspace`

### [ ] T103. Implement markdown/toc export layout and UAT corpus

Spec refs:

- UC-HBK-003
- FR-HBK-004
- FR-HBK-003
- FR-DOC-001
- FR-CLI-001
- ADR-0009
- UAT-HBK-004
- UAT-HBK-005
- UAT-HBK-006
- UAT-HBK-007
- `spec/implementation/components.md`

Scope:

- Implement `hierarchy=toc` as TOC-ordered page export under deterministic page directories.
- Implement `format=markdown --hierarchy=toc` as readable Markdown page files for TOC HTML content.
- Preserve the FR-HBK-004 Markdown invariant across full layout export: Markdown files must not
  contain raw HBK file paths, raw TOC indexes, raw HTML page paths or service HTML scaffolding.
- Run the first Markdown UAT corpus on representative 8.5.1.1150 pages from `fmtdui_ru.hbk`,
  `htmlui_ru.hbk`, `moxelui_ru.hbk`, `shlang_ru.hbk`, `shquery_ru.hbk` and `dcsui_ru.hbk`.
- Keep `format=raw --hierarchy=toc` and `format=markdown --hierarchy=raw` unsupported unless a
  later spec task defines them.

Verification:

- Focused tests for markdown/toc export layout and deterministic file naming.
- UAT-HBK-004 through UAT-HBK-007 pass or are skipped only for missing installed HBK books.
- UAT-HBK-004 includes a corpus-level negative search for raw HBK file paths, raw TOC indexes, raw
  HTML page paths and service HTML scaffolding in exported Markdown files.
- Dependency-boundary check proves `hbk-book-export` does not depend on Syntax Assistant
  extraction, `hbk-syntax-export`, search/index or resolver crates (`cargo tree -p hbk-book-export`
  or equivalent `cargo metadata` query).
- `cargo test -p hbk-book-export`
- `cargo test -p v8-context-hbk-cli`
- `cargo test --workspace`
