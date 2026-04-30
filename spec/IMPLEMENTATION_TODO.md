# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history:

- [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md)
- [archive/completed-tasks-t13-t17-t19-t24.md](archive/completed-tasks-t13-t17-t19-t24.md)

Current status: T32 is the first active unchecked task. T29 was explicitly reprioritized before T18
by the 2026-04-30 request to support previously out-of-scope Syntax Assistant event/table source
families and is now complete. T32 was explicitly reprioritized before the performance follow-up by
the 2026-04-30 request to make the consumer JSON output leaner and easier for downstream agents to
consume. Post-T29 measurements also found a release-profile `syntax-helper` runtime regression that
must be corrected before the query CLI work resumes. T30-T31 are reprioritized before T18 by the
2026-04-30 performance-regression review, after T32. T13-T17 and T19-T24 are
archived historical tasks; their durable performance conclusions live in `acceptance/baseline.md`,
`implementation/performance-baseline-t13.md` and `implementation/performance-variants.md`.
T25-T28 record export-completeness gaps found by the 2026-04-30 audit across Russian and
root/English Syntax Assistant exports. T25-T28 are closed after explicit export-completeness
reprioritization.

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

### [x] T29. Support Syntax Assistant global events and query/table metadata records

Depends on: T28.

Spec refs:

- FR-SH-001
- FR-SH-002
- FR-EXPORT-001
- NFR-DIAG-001
- UAT-SH-010
- UAT-SH-011
- `spec/source-evidence.md`, T28 diagnostic family classification
- `spec/implementation/components.md`

Scope:

- Promote `OUT_OF_SCOPE_GLOBAL_CONTEXT_EVENT`, `OUT_OF_SCOPE_TABLE_FIELD` and
  `OUT_OF_SCOPE_TABLE_PARAMETER` source families into typed extraction/export support.
- Add consumer record families for global context events, query/table fields and query/table
  parameters without mixing them into existing method/property record files.
- Bump the canonical consumer export schema version because the export file inventory changes.
- Preserve `UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE` diagnostics for direct global-context method-like
  TOC entries whose HTML is absent from `FileStorage`.
- Keep consumer records free of source HBK paths, TOC paths, HTML paths and page titles; parser
  provenance remains internal and in diagnostics.
- Do not implement the separate query CLI, semantic search, runtime 1C introspection or downstream
  compatibility DTOs in this task.

Expected artifacts:

- Model/extractor/export changes for `global-context-events.json`, `table-fields.json` and
  `table-parameters.json`.
- Fixture or unit tests covering at least one event, one table field and one table parameter.
- Updated README and acceptance baseline for schema version, file inventory, counts and remaining
  diagnostics.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- UAT-SH-010
- UAT-SH-011
- `git diff --check`

Completion notes:

- Completed on 2026-04-30.
- Added schema version 4 consumer record families:
  `global-context-events.json`, `table-fields.json` and `table-parameters.json`.
- Full CLI exports for `shcntx_ru.hbk` and `shcntx_root.hbk` now produce 33 global context events,
  588 query/table fields and 78 query/table parameters in each locale.
- Remaining diagnostics dropped from 703 to 4 in each locale. The remaining diagnostics are all
  `UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE`; `UNKNOWN_PAGE_CLASS`,
  `OUT_OF_SCOPE_GLOBAL_CONTEXT_EVENT`, `OUT_OF_SCOPE_TABLE_FIELD` and
  `OUT_OF_SCOPE_TABLE_PARAMETER` are absent.
- Consumer records still omit HBK provenance, TOC paths, HTML paths and page titles.
- Verified with `cargo fmt`, `cargo test --workspace`, UAT-SH-001, UAT-SH-002, UAT-SH-003,
  UAT-SH-010, UAT-SH-011 and `git diff --check`.

### [x] T32. Switch consumer JSON export to lean schema version 5

Depends on: T29.

Spec refs:

- FR-EXPORT-001
- FR-SH-002
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- UAT-SH-008
- UAT-SH-009
- UAT-SH-011
- UAT-SH-012

Scope:

- Bump the canonical consumer export schema to `schema_version: 5`.
- Keep record-family envelopes and parser diagnostics structurally valid, but omit `null` fields
  and empty arrays inside platform API consumer records.
- Simplify owned consumer records by serializing `owner` as the owner's primary-name string.
- Serialize `type_refs` and `return_types` as arrays of type-name strings wherever they appear,
  including signature parameters.
- Move recognized version facts from top-level `available_since` to `availability.since`; omit
  `available_since` from consumer records.
- Serialize `see_also` as an array of target primary-name strings.
- Normalize property `usage` to enum strings `Read`, `Write`, `ReadWrite` or `Unknown` in both
  `global-properties.json` and `type-properties.json`.
- Strip leading type-reference prose from property descriptions when that fact is already exposed
  through `type_refs`.
- Remove `signatures[].text` for methods, global context events and constructors.
- Move syntax-variant `title` and `description` directly onto signature records instead of nested
  `variant`.
- Merge enum values into owning records in `enums.json`; do not emit `enum-values.json`.
- Keep enum and enum value `name` as the localized-name object with `primary` and optional `alias`.
- For nested enum values, include `availability.since` only when it differs from the owning enum's
  `availability.since`.
- Do not implement the query CLI, semantic search, runtime 1C introspection or downstream
  compatibility DTOs in this task.

Expected artifacts:

- Model/export adapter changes for the schema version 5 consumer JSON shape.
- Fixture or export-level tests covering representative methods, events, constructors,
  properties, see-also links and merged enum values.
- Updated README and acceptance baseline for schema version, file inventory and record-family
  counts.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- UAT-SH-008
- UAT-SH-009
- UAT-SH-011
- UAT-SH-012
- Full CLI export for `shcntx_ru.hbk`
- Full CLI export for `shcntx_root.hbk`
- `git diff --check`

Completion notes:

- Completed on 2026-04-30.
- Raised canonical consumer export schema to `schema_version: 5`.
- Removed `enum-values.json` from the export inventory and nested all 3110 enum values under
  owning records in `enums.json` for both RU and root/English exports.
- Record-family counts remained stable: 500 global methods, 101 global properties, 33 global
  context events, 2533 platform types, 6702 type methods, 10732 type properties, 588 table fields,
  78 table parameters, 445 constructors, 713 enums and 4 diagnostics in each locale.
- Platform API consumer records now omit `null` fields and empty arrays, serialize `owner` as a
  primary-name string, serialize `type_refs` and `return_types` as arrays of type-name strings,
  move recognized version facts to `availability.since`, flatten `see_also` to target primary-name
  strings, normalize property `usage`, strip leading property type prose from descriptions, remove
  callable `signatures[].text` and flatten syntax-variant metadata onto signature records.
- Consumer records still omit HBK provenance, TOC paths, HTML paths and page titles; diagnostics
  remain provenance-rich for parser maintenance.
- Verified with `cargo fmt`, `cargo test --workspace`, UAT-SH-001, UAT-SH-002, UAT-SH-003,
  UAT-SH-008, UAT-SH-009, UAT-SH-011, UAT-SH-012, full CLI exports for `shcntx_ru.hbk` and
  `shcntx_root.hbk`, and `git diff --check`.

### [ ] T30. Remove post-T29 Syntax Assistant table-owner lookup regression

Depends on: T29 and T32.

Spec refs:

- NFR-PERF-001
- FR-SH-002
- FR-EXPORT-001
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- UAT-SH-010
- UAT-SH-011
- `spec/acceptance/baseline.md`, post-T29 runtime regression note
- `spec/implementation/performance-variants.md`

Scope:

- Replace the hot `query_table_owner` path that calls `Toc::find_by_html_path` for every
  `QueryTableField` and `QueryTableParameter`.
- Build or reuse a single deterministic TOC lookup/index during `SyntaxHelperReader::extract_into`
  and pass it through the extraction path instead of repeatedly flattening the whole TOC.
- Keep table owner names locale-aware and keep the accepted consumer record-family JSON shape
  unchanged (`schema_version: 5` after T32).
- Preserve deterministic record order, diagnostics order and streaming export behavior.
- Do not add broad caches, generic pipeline abstractions, parallel parsing, query CLI work or
  downstream compatibility DTOs in this task.

Expected artifacts:

- Extractor changes that make table-owner lookup O(1) or otherwise bounded by one prebuilt TOC
  pass per `syntax-helper` run.
- Unit or fixture coverage proving table field/parameter owner names still resolve for RU and
  root/English examples.
- Release-profile measurements for `shcntx_ru.hbk` and `shcntx_root.hbk` comparing current HEAD
  against the fixed code.
- Updated acceptance baseline and task completion notes with elapsed time, peak RSS, record counts
  and remaining diagnostics.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- UAT-SH-010
- UAT-SH-011
- Release-profile `syntax-helper` measurement for `shcntx_ru.hbk`
- Release-profile `syntax-helper` measurement for `shcntx_root.hbk`
- `git diff --check`

Regression evidence:

- T24 release baseline measured `shcntx_ru.hbk` at `3.38s / 151136 KiB` and
  `shcntx_root.hbk` at `2.57s / 119936 KiB`.
- A 2026-04-30 commit-by-commit release measurement of `shcntx_root.hbk` showed
  `c3ff0df` at `3.53s / 119808 KiB` and `8da6a7c` at `11.04s / 119808 KiB`.
- The same review measured RU at `c3ff0df 4.80s / 132352 KiB` and
  `8da6a7c 12.70s / 132352 KiB`.
- The main suspected cause is the post-T29 table-owner resolution path:
  `query_table_owner` calls `Toc::find_by_html_path`, and `find_by_html_path` rebuilds
  `flat_pages()` on every call.

### [ ] T31. Re-measure and optimize residual Syntax Assistant parser overhead after T30

Depends on: T30.

Spec refs:

- NFR-PERF-001
- FR-SH-002
- FR-EXPORT-001
- `spec/acceptance/baseline.md`, post-T29 runtime regression note
- `spec/implementation/performance-variants.md`

Scope:

- Re-run release-profile measurements after T30 before changing parser code.
- If elapsed time remains materially above the T24/T28 class, attribute the remaining overhead to
  concrete parser helpers or export steps before implementation.
- Candidate areas to measure first:
  - repeated `section_text` / `section_html` scans over expanded section-boundary labels;
  - `section_facts` extracting availability, examples, see-also and version facts on most pages;
  - `parse_variant_signatures` detection before ordinary signature parsing;
  - rubric-parameter parsing before plain text fallback.
- Apply only the smallest measured parser/export optimization that preserves T25-T29 structured
  facts and the accepted schema version 5 output from T32.
- Do not introduce parallel parsing, generic HTML pipelines, persistent caches or query CLI work in
  this task.

Expected artifacts:

- Measurement notes that distinguish fixed T30 lookup cost from residual parser cost.
- Parser/export changes only if measurements identify a concrete bottleneck.
- Fixture or unit tests for any parser behavior touched.
- Updated acceptance baseline and completion notes with before/after release-profile measurements.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- Relevant Syntax Assistant UAT cases for any touched record families
- Release-profile `syntax-helper` measurement for `shcntx_ru.hbk`
- Release-profile `syntax-helper` measurement for `shcntx_root.hbk`
- Deterministic export comparison for at least one Syntax Assistant book when parser output is
  expected to stay semantically unchanged
- `git diff --check`

### [ ] T18. Design and implement the separate Syntax Assistant query CLI first slice

Depends on: T17 and T31 unless this task is explicitly reprioritized.

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- NFR-QUERY-001
- UC-SH-003
- UC-SH-004
- UAT-SH-004
- UAT-SH-005
- UAT-SH-006
- ADR-0004
- `spec/source-evidence.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Confirm or revise ADR-0004 before coding. If the accepted binary name, crate split or index
  artifact differs from the draft, update ADR-0004 and the implementation spec first.
- Implement the first deterministic local search slice before semantic search:
  - build a local SQLite/FTS5 index from the current canonical Syntax Assistant export directory;
  - exact lookup by primary name and alias;
  - exact owner/member lookup;
  - keyword search over names, aliases, signatures, type references and descriptions;
  - relationship traversal over owner/member and type-reference edges stored in an edge table.
- Keep query commands on a prebuilt local export or index. Do not parse `shcntx_*.hbk` in query
  commands.
- Keep the lean consumer export shape from FR-EXPORT-001. If search needs structured links or page
  provenance, add a search-specific index/maintenance artifact instead of putting those fields back
  into consumer record-family files.
- Do not implement semantic search, embedding providers, network calls, graph database integration,
  caches, server mode, MCP or UI in this first slice.
- Measure query latency against NFR-QUERY-001 on the Russian Syntax Assistant data set.

Expected artifacts:

- Search/index library code and separate query CLI surface.
- Rebuildable SQLite index artifact generated by UAT, not committed to source.
- README usage for the implemented query CLI only after the command exists.
- Completion notes with query measurements and any relationship-quality gaps.
- Follow-up task for structured "see also" link extraction if deterministic relationships are not
  sufficient for the SKD-filter UAT path.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-004
- UAT-SH-005
- UAT-SH-006
- NFR-QUERY-001 measurement notes for exact lookup, keyword search and relationship search
- `git diff --check`

### [x] T25. Fix locale-aware Syntax Assistant section parsing and type references

Depends on: T24. Scheduled after T18 unless Syntax Assistant export completeness is explicitly
reprioritized.

Spec refs:

- FR-SH-002
- FR-EXPORT-001
- UAT-SH-007
- `spec/source-evidence.md`, Syntax Assistant Export Completeness Audit
- `spec/implementation/components.md`

Scope:

- Fix root/English parsing parity for `Type:` and `Returned value:` sections so return types,
  property type references and parameter type references are extracted from `shcntx_root.hbk`.
- Extend section boundary detection for both locales so descriptions, signatures and parameter
  descriptions stop swallowing later sections:
  - `Доступность:` / `Availability:`;
  - `Пример:` / `Example:`;
  - `См. также:` / `See also:`;
  - `Использование в версии:` / `Available since:`;
  - overload variant labels.
- Keep consumer record files free of HBK provenance and duplicate navigation-link catalogs.
- Do not introduce a generic HTML pipeline, caches, parallelism, query CLI changes or downstream
  compatibility DTOs in this task.
- If removing swallowed sections would otherwise drop facts needed by T26, preserve them in an
  internal section representation or implement the smallest shared extraction helper required by
  T26.

Expected artifacts:

- Parser changes and fixture tests covering representative Russian and root/English pages.
- Export regression checks showing `XMLСтрока` / `XMLString`, `Массив.Добавить` / `Array.Add` and
  `ОткрытьФорму` / `OpenForm` retain type facts in both locales.
- Completion notes with before/after counts for empty return/type reference arrays by
  record-family and locale.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- UAT-SH-007
- Deterministic export comparison for at least one Syntax Assistant book
- `git diff --check`

Completion notes:

- Completed on 2026-04-30.
- Implemented root/English `Type:` and `Returned value:` parsing plus RU/root section boundaries for
  availability, examples, see-also, available-since and overload variant labels.
- Verified with existing real-source audit fixtures and full CLI exports for `shcntx_ru.hbk` and
  `shcntx_root.hbk`; repeated `shcntx_ru.hbk` export was byte-identical by `diff -qr`.
- Empty type-reference counts are recorded in `spec/source-evidence.md` and
  `spec/acceptance/baseline.md`.
- No consumer JSON schema/version change; consumer record-family files still omit HBK provenance,
  TOC paths, HTML paths and page titles.

### [x] T26. Extract structured availability, examples, see-also and version facts

Depends on: T25.

Spec refs:

- FR-SH-002
- FR-EXPORT-001
- UAT-SH-008
- `spec/source-evidence.md`, Syntax Assistant Export Completeness Audit
- `spec/implementation/components.md`

Scope:

- Add typed domain/export representation for non-description Syntax Assistant facts that are
  currently flattened into `description`:
  - availability/application contexts;
  - examples/code blocks;
  - see-also relationships;
  - available-since/version text.
- Normalize availability contexts to stable values while preserving localized display text only
  where needed for diagnostics or examples.
- Cover at least global methods, global properties, platform types, type methods, type properties,
  constructors, enums and enum values when the source page contains those sections.
- Keep parser provenance in diagnostics and keep consumer record files free of HBK source paths,
  TOC paths and page titles.
- Update export schema version and README examples only if the consumer JSON shape changes.
- Do not solve query indexing, semantic search, runtime 1C introspection or broad relationship graph
  design in this task.

Expected artifacts:

- Model/export changes for structured section facts.
- Fixture tests and export-level assertions proving examples and availability are no longer embedded
  only in `description`.
- Updated FR-EXPORT-001 details if field names or schema version change during implementation.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- UAT-SH-008
- Deterministic export comparison for at least one Syntax Assistant book after accepting the new
  schema shape
- `git diff --check`

Completion notes:

- Completed on 2026-04-30.
- Added schema version 2 structured section fields to consumer record-family JSON:
  `availability`, `examples`, `see_also` and `available_since`.
- Normalized availability contexts to stable snake_case values and kept `see_also` consumer targets
  provenance-free by exposing target names without HBK/TOC/HTML paths or page titles.
- Verified real-source audit fixtures and full CLI exports for `shcntx_ru.hbk` and
  `shcntx_root.hbk`; record-family counts remained stable at the T25 baseline.
- UAT-SH-001, UAT-SH-002, UAT-SH-003 and UAT-SH-008 passed on schema version 2 exports, and a
  repeated `shcntx_ru.hbk` export was byte-identical by `diff -qr`.
- T27 overload/syntax-variant structure was intentionally pending at T26 completion; see completed
  T27 below for closure.

### [x] T27. Parse overload and syntax-variant pages structurally

Depends on: T25. Prefer running after T26 if variant descriptions or examples need the structured
section model introduced there.

Spec refs:

- FR-SH-002
- FR-EXPORT-001
- UAT-SH-009
- `spec/source-evidence.md`, Syntax Assistant Export Completeness Audit

Scope:

- Represent Syntax Assistant overloads/syntax variants without mixing labels or prose into
  `Signature.text`.
- Preserve variant title and variant description as metadata when source HTML contains
  `Вариант синтаксиса:` / `Syntax variant:` and `Описание варианта метода:` /
  `Description of method variant:`.
- Attach parameters to the correct variant instead of letting parameter descriptions absorb later
  variant text.
- Preserve return types for variant-heavy pages in both Russian and root/English exports.
- Cover `ДокументDOM.СоздатьРазыменовательПИ` / `DOMDocument.CreateNSResolver` as a regression
  fixture, plus at least one current English/root false multi-signature page where returned-value
  prose is being parsed as signatures.
- Do not change record-family counts or query CLI behavior unless the accepted export schema for
  overloads requires it.

Expected artifacts:

- Parser/model/export changes for structured overloads or an ADR/spec update if the current
  `Signature` model must be replaced.
- Fixture tests for representative Russian and root/English overload pages.
- Completion notes with before/after counts for signatures containing raw section labels.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-007
- UAT-SH-009
- Deterministic export comparison for at least one Syntax Assistant book after accepting the new
  schema shape
- `git diff --check`

Completion notes:

- Completed on 2026-04-30.
- Added schema version 3 `signatures[].variant` metadata with `title` and `description` for
  Syntax Assistant syntax-variant pages. Consumer record-family JSON still omits HBK provenance,
  TOC paths, HTML paths and page titles.
- Implemented locale-aware variant block parsing for Russian and root/English pages so
  `ДокументDOM.СоздатьРазыменовательПИ` / `DOMDocument.CreateNSResolver` exports four callable
  signatures in both locales, with parameters attached to the owning variant and return types
  preserved.
- Covered existing real-source audit fixtures for `DOMDocument.CreateNSResolver` and root
  `OpenForm`; no new HTML case was required because the manifest already registered the T27
  fixtures.
- Full RU/root CLI export record-family counts remained stable. Structured variant metadata is
  present on 266 records and 604 signatures in each locale; `global-methods.json` accounts for
  23 records / 60 signatures and `type-methods.json` for 243 records / 544 signatures.
- Signature text containing raw overload section labels or returned-value labels was zero in the
  post-T26 baseline and remained zero after T27 for both locales.
- Verified with `cargo fmt`, `cargo test --workspace`, UAT-SH-007, UAT-SH-009, repeated
  `shcntx_ru.hbk` export compared by `diff -qr`, `git diff --check` and an independent reviewer
  pass.

### [x] T28. Classify remaining Syntax Assistant diagnostics and extraction completeness

Depends on: T25.

Spec refs:

- FR-SH-001
- FR-SH-002
- NFR-DIAG-001
- UAT-SH-010
- `spec/source-evidence.md`, Syntax Assistant Export Completeness Audit
- `spec/acceptance/baseline.md`

Scope:

- Review the 703 `UNKNOWN_PAGE_CLASS` diagnostics in both locales and classify each source family as
  in scope, explicitly out of scope or follow-up scope.
- Pay special attention to:
  - direct `objects/Global context/*.html` pages that look like global context methods;
  - global-context event pages;
  - table field and parameter pages.
- Add extraction support or explicit diagnostics only for source families that FR-SH-002 makes
  in-scope. Do not silently drop unclassified pages.
- Update requirements/UAT if global events, table fields or other currently diagnostic families are
  promoted into scope.
- Preserve deterministic diagnostics and source provenance.

Expected artifacts:

- A checked-in completeness note or source-evidence update with diagnostic family counts and the
  in-scope/out-of-scope decision.
- Follow-up implementation tasks for any promoted source family that is too large to implement
  safely in this task.
- Updated acceptance baseline when diagnostic counts or meanings change.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- Export diagnostic summary for `shcntx_ru.hbk` and `shcntx_root.hbk`
- `git diff --check`

Done notes:

- Completed by explicit export-completeness reprioritization before T18.
- Replaced the audited generic `UNKNOWN_PAGE_CLASS` diagnostics with stable family-specific codes
  for both source locales:
  - `UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE`: 4 RU / 4 EN-root, in FR-SH-002 scope but not safely
    extractable from the current TOC-only direct `objects/Global context/*.html` entries.
  - `OUT_OF_SCOPE_GLOBAL_CONTEXT_EVENT`: 33 RU / 33 EN-root, explicitly out of T28 scope.
  - `OUT_OF_SCOPE_TABLE_FIELD`: 588 RU / 588 EN-root, explicitly out of T28 scope.
  - `OUT_OF_SCOPE_TABLE_PARAMETER`: 78 RU / 78 EN-root, explicitly out of T28 scope.
- T28 did not promote global events, table fields or table parameters into scope. T29 later promoted
  those families into typed export records.
- Consumer record-family JSON remained schema version 3 at T28 completion and still omitted HBK
  provenance, TOC paths, HTML paths and page titles.
