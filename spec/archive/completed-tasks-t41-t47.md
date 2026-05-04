# Completed Implementation Tasks T41-T47

This archive preserves completed task history moved out of the active implementation ledger.
It is evidence, not active implementation scope.

Raw command logs, generated exports, temporary probes and `target/...` paths are service data and
are intentionally omitted from this archive. Durable query-index, provider-direction and parser
conclusions live in `../acceptance/baseline.md`, `../requirements/functional.md`,
`../implementation/syntax-helper-query-cli.md` and `../implementation/syntax-bsl-provider-plan.md`.

For active task sequencing, use `../IMPLEMENTATION_TODO.md`.

## T41. Define query-index record identity and form-parameter classification

Depends on: T18 checkpoint `d990d8a`. Blocks further T18 continuation.

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- FR-EXPORT-001
- UAT-SH-004
- UAT-SH-006
- UAT-SH-015
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/source-evidence.md`

Scope:

- Define the `syntax-helper-search` document identity contract per record family before changing code.
  Document ids must not include HBK file paths, TOC paths, HTML paths, page titles or display strings
  such as `primary (alias)`.
- Reuse domain identifiers that already exist in the extraction/export model. Query table documents
  must use `QueryTable.identifier`, not display names such as `Основная таблица`; query table field and
  parameter documents must be owned by that table identity rather than only by the table page title.
  Accepted query table identity shape: use plain `QueryTable.identifier` when it is unique in the
  real source data, and append only the minimal semantic `owner_path`-derived variant when the same
  identifier appears in multiple table families, such as accounting-register tables with and without
  correspondence support. Query table field and parameter ids use that final table identity plus the
  field or parameter name.
- Classify Syntax Assistant pages under form and form-extension `Параметры формы` branches as form
  attributes/parameters owned by the form or extension type. They must not be emitted as
  `platform_type` records.
- Preserve semantic variants for same-primary form/interface platform types such as ordinary-form
  and managed-client-form `ЭлементыФормы`, and build type member ids from the final owner identity
  rather than `owner.primary` alone.
- Treat TOC duplicate-title markers such as `#&^@^%&*^#1` as parser service data, not semantic
  identity. After stripping the marker, duplicate source pages for the same final owner identity and
  primary name must not create a second search document or receive a source-path suffix; this applies
  across methods, properties, constructors, enums and enum values.
- Distinguish metadata-object property enums from ordinary system enums in enum document identity;
  enum value ids must be owned by the final enum identity.
- Treat same-name records as parser/model evidence first. Do not hide a source-family or
  classification defect by adding source-path-shaped suffixes to search ids.
- Preserve exact lookup by primary name and alias through lookup tables; aliases may participate in
  lookup keys but not in document identity.
- Rebuild a real Russian Syntax Assistant query index and verify that `documents.id`,
  `relations.source_id` and `relations.target_id` follow the accepted identity contract without
  SQLite uniqueness failures.

Expected artifacts:

- Updated implementation spec for record-family identity rules.
- Parser/model/search changes needed for query-table ids and form-parameter classification.
- Focused tests for query-table identity, relation endpoints and form-parameter classification.
- Updated UAT/baseline notes with the verified real-index result.

Completion notes:

- `syntax-helper-search` document ids now use semantic record-family identities rather than HBK,
  TOC, HTML or page-title provenance. Exact lookup keys remain in `document_names`.
- Query table ids use `QueryTable.identifier`, with `owner_path`-derived semantic variants only for
  duplicated real-source table identifiers. Query table field/parameter ids and relations use the
  final table identity.
- Managed-form `Параметры формы` pages are classified as type properties owned by the preceding
  form/form-extension type, including pages whose HTML path does not contain `/params/`.
- The rebuilt Russian index completed without uniqueness failures and produced 25,082 documents /
  65,455 relations. SQLite read-only checks found no `.html`, `/` source path or
  `#&^@^%&*^#` marker in document ids and no form-parameter `platform_type` records.

Verification:

- `cargo test -p syntax-helper-extract --lib classifies_form_parameters_as_type_properties`
- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- `cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax index
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/sh-search-ru.sqlite`
- read-only SQLite checks over `target/uat/sh-search-ru.sqlite` for id shape, query-table variants,
  relation endpoints and form-parameter classification.

## T42. Reduce Syntax Assistant query-index build memory and runtime

Depends on: T41 semantic query-index identity. Continues T18 after the index identity contract is
stable.

Spec refs:

- NFR-PERF-001
- NFR-QUERY-001
- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- UAT-SH-004
- UAT-SH-006
- `spec/implementation/performance-baseline-t13.md`
- `spec/implementation/performance-variants.md`
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/acceptance/baseline.md`

Problem:

- `syntax export` already uses `SyntaxHelperReader::extract_into()` and a streaming export sink,
  but `syntax index` still calls `SyntaxHelperReader::extract()` and materializes the full
  `PlatformContext` before SQLite index build.
- `syntax-helper-search::build_index()` then materializes search documents and relations in memory
  before inserting them into SQLite. The index build path can therefore retain the extraction model,
  search document graph, relation graph and SQL insertion temporaries at the same time.
- The current T41 debug index build for `shcntx_ru.hbk` completed successfully but took about
  `63.703 s` and produced a large local SQLite artifact. Interactive query commands meet
  NFR-QUERY-001 on the built index, so the first optimization target is index build, not query
  runtime.

Scope:

- Measure the current `syntax index` build path on `shcntx_ru.hbk` and `shcntx_root.hbk` with
  wall-clock time, peak RSS, exit status, document count, relation count and SQLite file size.
- Attribute index-build memory between extraction, search-document construction, relation
  construction and SQLite insertion before choosing the implementation shape.
- Replace the full `PlatformContext` index-build path with a bounded streaming or staged indexing
  path that does not keep `PlatformContext`, `Vec<SearchDocument>` and `Vec<Relation>` live
  together.
- Preserve the T41 semantic document identity contract: no HBK/TOC/HTML/page-title provenance in
  `documents.id`; query table, form-parameter, type-member, enum and enum-value ids must keep the
  accepted identity rules.
- Preserve atomic SQLite rebuild behavior: build a complete temporary database beside the target,
  validate it and atomically replace the target while keeping concurrent readers safe.
- Do not add broad pipeline frameworks, cache systems, tuning knobs, graph databases or external
  search services. Use the smallest measured change that reduces the build-path bottleneck.
- Treat interactive `syntax get/search/related` micro-optimizations as out of scope unless fresh
  measurements show they miss NFR-QUERY-001 after the build-path change.

Expected artifacts:

- Updated implementation notes describing the accepted index-build data flow and why broader
  storage/search changes were not needed.
- Code changes in `v8-context-hbk-cli`, `syntax-helper-extract` and/or `syntax-helper-search`
  needed for the bounded index-build path.
- Focused tests proving semantic document ids and relation endpoints stay stable when the index is
  built through the new path.
- Updated acceptance baseline with before/after measurements for both Syntax Assistant source
  books and representative query latency checks.

Completion notes:

- `syntax index` now consumes extraction through `SyntaxHelperReader::extract_into()` and a
  `syntax-helper-search::SearchIndexBuilder` instead of materializing a full `PlatformContext`.
- The builder stages search document drafts and the minimal identity inputs needed for T41 semantic
  ids, then the SQLite writer inserts documents and streams relation inserts from finalized
  documents without retaining a complete `Vec<Relation>`.
- Atomic rebuild behavior, writer locking, SQLite schema version `1`, read-only query commands and
  T41 document-id/relation identity rules are unchanged.
- Release-profile `shcntx_ru.hbk` index build improved from `20.25 s / 617872 KiB` to
  `18.80 s / 269612 KiB`; `shcntx_root.hbk` improved from `15.17 s / 443532 KiB` to
  `14.55 s / 239972 KiB`. Document, relation, `document_names` and SQLite file-size counts stayed
  stable for both source books.
- Representative release query checks against the rebuilt Russian index measured `0.00 s` exact,
  `0.04 s` keyword, `0.04 s` fuzzy and `0.01 s` related.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test -p syntax-helper-extract --lib`
- `cargo test --workspace`
- measured `syntax index` run for `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- measured `syntax index` run for `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`
- read-only SQLite checks for document count, relation count, id-shape invariants and relation
  endpoint integrity on the rebuilt Russian index
- measured `syntax get`, `syntax search --mode keywords`, `syntax search --mode fuzzy` and
  `syntax related` smoke commands against the rebuilt Russian index

## T43. Reduce SQLite insertion overhead in Syntax Assistant query-index build

Depends on: T42 staged index-build path.

Spec refs:

- NFR-PERF-001
- NFR-QUERY-001
- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/acceptance/baseline.md`

Problem:

- After T42, release `syntax export` for `shcntx_ru.hbk` measured `5.74 s / 197768 KiB`, while
  release `syntax index` measured `20.48 s / 269828 KiB` on the same source and binary.
- The rebuilt Russian SQLite index contains 25082 `documents`, 132646 `document_names`, 25082 FTS
  rows and 65455 `relations`. The current writer inserts those rows through per-row
  `Connection::execute()` calls, which prepares SQL repeatedly inside the same transaction.
- This is a narrower bottleneck than changing the index format, adding a cache, introducing
  concurrency or changing the query command contract.

Scope:

- Reuse prepared SQLite statements for document, name, FTS and relation insertion inside the
  existing temporary database transaction.
- Defer ordinary B-tree index creation until after bulk row insertion.
- Use fixed temp-rebuild-only SQLite settings that are safe for a disposable replacement database
  validated before atomic rename.
- Preserve T41/T42 behavior: same SQLite schema, same document ids, same relation endpoints, same
  writer lock and atomic replacement flow, same query commands.
- Do not add user-facing tuning knobs, external search services, alternate storage engines,
  parallel writers or new index artifacts in this slice.

Expected artifacts:

- Narrow `syntax-helper-search` writer changes.
- Focused tests continue to cover the staged builder-to-SQLite path and replacement/lock behavior.
- Updated acceptance baseline with before/after index-build timing for `shcntx_ru.hbk`.

Completion notes:

- `syntax-helper-search` now prepares document, lookup-name, FTS and relation insert statements once
  per temporary database transaction instead of preparing SQL for each row.
- Ordinary B-tree indexes for lookup and relation tables are created after bulk insertion. FTS
  remains populated through its existing virtual table.
- The temporary replacement database uses rebuild-only SQLite settings (`journal_mode=OFF`,
  `synchronous=OFF`, `locking_mode=EXCLUSIVE`, `temp_store=MEMORY`). The active target database is
  still replaced only after the temp database is built, indexed and validated.
- Release `shcntx_ru.hbk` index build improved from the post-T42 measured `20.48 s / 269828 KiB`
  triage run to `16.30 s / 269632 KiB`; `shcntx_root.hbk` improved from the T42 baseline class
  `14.55 s / 239972 KiB` to `12.79 s / 243992 KiB`.
- Counts stayed stable: Russian index `25082 documents / 132646 document_names / 25082 FTS rows /
  65455 relations`; root index `25062 / 47001 / 25062 / 68670`.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- measured release `syntax index` run for `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- read-only SQLite checks for document count, relation count and relation endpoint integrity on the
  rebuilt Russian index

## T44. Build Syntax Assistant FTS index in bulk mode

Depends on: T43 SQLite writer optimization.

Spec refs:

- NFR-PERF-001
- NFR-QUERY-001
- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- `spec/implementation/syntax-helper-query-cli.md`
- `spec/acceptance/baseline.md`

Problem:

- After T43, release `syntax index` for `shcntx_ru.hbk` still measured `16.30 s / 269632 KiB`,
  while release `syntax export` measured `5.74 s / 197768 KiB` on the same source and binary.
- The index artifact remains much larger than the canonical JSON export (`139M` SQLite vs `19M`
  JSON directory for Russian Syntax Assistant), and FTS construction/storage is the main remaining
  suspect after ordinary SQLite insert overhead was reduced.
- Continuing to tune per-row insertion is unlikely to produce a cardinal improvement. The next
  measured slice should avoid treating the FTS virtual table as a row-by-row sink when a bulk FTS
  rebuild can preserve the same query contract.

Scope:

- Change FTS population to a bulk-build path: load searchable rows into an ordinary content table,
  then build the FTS index with SQLite FTS rebuild semantics.
- Preserve query behavior: keyword search results, deterministic ordering, exact lookup, fuzzy
  lookup and relationship traversal must stay in the accepted behavior class.
- Preserve atomic rebuild behavior and writer serialization from T18/T42/T43.
- Prefer SQLite-native FTS variants before introducing a separate search engine. Evaluate
  external-content/contentless FTS only as measured variants inside the same query-index artifact
  boundary; do not add Tantivy, parallel writers or cache/reuse behavior in this slice.
- Do not split the index into mandatory and heavy/optional artifacts. T44 keeps one SQLite index
  built by one `syntax index` command.
- Bump or document the search-index schema metadata if the SQLite table layout changes.

Expected artifacts:

- `syntax-helper-search` schema/writer/query changes needed for the selected bulk FTS mode.
- Focused tests covering keyword search and staged builder-to-SQLite behavior through the new FTS
  layout.
- Updated implementation notes and acceptance baseline with before/after timing, SQLite size and
  query smoke results.

Completion notes:

- `syntax-helper-search` now writes searchable rows into the ordinary `document_search` content
  table and builds `document_fts` with SQLite FTS5 external-content rebuild semantics instead of
  inserting every row directly into the FTS virtual table.
- The search index remains one SQLite artifact produced by one `syntax index` command. Exact lookup,
  keyword search, fuzzy lookup, relationship traversal, writer locking and atomic replacement
  behavior stay in the existing contract.
- Search-index schema metadata is now version `2` because the SQLite layout gained
  `document_search` and external-content FTS. The canonical consumer JSON export schema remains
  unchanged.
- The measured contentless FTS variant reduced the Russian SQLite file to `126M`, but was slower
  than the selected external-content rebuild path (`15.94 s` vs `15.82-15.93 s`) and changed more
  query plumbing. It was not selected.
- Final release-profile external-content rebuild measurements: `shcntx_ru.hbk` `15.93 s /
  269696 KiB / 139M`, `shcntx_root.hbk` `12.52 s / 243860 KiB / 62M`. Counts stayed stable:
  Russian index `25082 documents / 132646 document_names / 25082 document_search / 25082 FTS rows /
  65455 relations`; root index `25062 / 47001 / 25062 / 25062 / 68670`.
- A representative release keyword search for `отбор скд` against the rebuilt Russian index
  measured `0.03 s` and returned the accepted result class.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- measured release `syntax index` runs for `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
  `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`
- read-only SQLite checks for document count, FTS row count, relation count, schema metadata and
  relation endpoint integrity on the rebuilt Russian index
- measured release `syntax search --mode keywords` smoke against the rebuilt Russian index

## T45. Add direct Syntax Assistant constructor lookup command

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- UAT-SH-006
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Add a direct CLI path for retrieving constructor signatures by type name from the existing local
  search index.
- Preserve the current index schema and relationship graph; this is a convenience query over existing
  constructor documents, not a new export or storage contract.
- Text output should print constructor signatures directly so users do not need `syntax related |
  jq` for the common "show constructor signatures" workflow.
- JSON output should remain deterministic and machine-readable.

Expected artifacts:

- `syntax-helper-search` query helper for constructor lookup by type name.
- `v8-context-hbk syntax constructors <TYPE>` CLI command.
- Focused tests and UAT/readme updates for the direct constructor workflow.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- `./target/release/v8-context-hbk syntax constructors "HTTPСоединение"`

Completion notes:

- Added `syntax-helper-search::SearchIndex::constructors_by_name()` as a read-only convenience query
  over existing owner-to-constructor relations.
- Added `v8-context-hbk syntax constructors <TYPE>`, with text output printing constructor
  signatures directly and JSON output preserving full deterministic hit records.
- No SQLite schema, export schema or relationship graph changes were required.
- Verified the release binary against the default local Syntax Assistant index for
  `HTTPСоединение`.

## T46. Add detailed text output for constructor lookup

Spec refs:

- FR-SH-SEARCH-001
- UAT-SH-006
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Add an opt-in detailed text mode to `syntax constructors <TYPE>`.
- Keep signature-only text output as the default for fast argument-order lookup.
- Include available owner and description context in detailed text output.
- Do not change JSON output, SQLite schema, export schema or constructor parsing.

Expected artifacts:

- `--details` CLI flag for `v8-context-hbk syntax constructors <TYPE>`.
- README/UAT/baseline updates for the detailed constructor workflow.

Verification:

- `cargo test -p syntax-helper-search --lib`
- `cargo test --workspace`
- `./target/release/v8-context-hbk syntax constructors "HTTPСоединение" --details`

Completion notes:

- Added `--details` to constructor lookup. Text mode now remains compact by default, while detailed
  mode prints each constructor signature with owner and description when available.
- JSON output remains the full search-hit records and is not changed by `--details`.

## T47. Preserve constructor parameters when parameter bodies contain inline notes

Spec refs:

- FR-SH-002
- FR-EXPORT-001
- UAT-SH-013
- `spec/implementation/components.md`

Scope:

- Fix Syntax Assistant HTML section extraction so `V8SH_chapter` section starts and ends are resolved
  from structural chapter markers, not from plain text labels inside section bodies.
- Preserve parameter parsing when a constructor parameter description contains inline text such as
  `Примечание:`.
- Keep text-section extraction unchanged unless a separate source-backed case requires changing it.
- Do not change consumer export schema or search-index schema.

Expected artifacts:

- Focused parser regression for `HTTPСоединение`-shaped constructor HTML where `<Сервер>` contains
  an inline `Примечание:` before later parameters.
- Parser fix in `syntax-helper-extract` only.
- Updated spec/UAT/baseline notes.

Verification:

- `cargo test -p syntax-helper-extract --lib constructor_parameters_keep_inline_notes_inside_parameter_section`
- `cargo test -p syntax-helper-extract --lib`
- `cargo test --workspace`

Completion notes:

- `section_html()` now treats sections headed by `<p class="V8SH_chapter">` structurally: it prefers
  real chapter markers when locating the requested section and ends the HTML slice at the next
  `V8SH_chapter` marker or HTML footer rather than at any label-like text in the section body.
- The new regression covers `HTTPСоединение`-style constructor parameters where an inline
  `Примечание:` appears inside the first parameter body before the remaining overload parameters.
- Consumer JSON and search-index schemas remain unchanged; rebuilt data will expose the recovered
  constructor parameters through the existing signature parameter shape.

