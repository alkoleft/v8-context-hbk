# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history:

- [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md)
- [archive/completed-tasks-t13-t17-t19-t24.md](archive/completed-tasks-t13-t17-t19-t24.md)
- [archive/completed-tasks-t25-t34.md](archive/completed-tasks-t25-t34.md)
- [archive/implementation-todo-2026-05-04.md](archive/implementation-todo-2026-05-04.md)

Current status: T35-T40 and the T18 first slice are archived historical tasks. Their durable
export, schema, data-quality, performance and query-search conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`implementation/components.md` and `implementation/syntax-helper-query-cli.md`.
There are no unchecked active tasks after T43. Further T18 continuation may add the next focused
query/search task after reconciling it with the durable T41 identity findings, T42 build-path
measurements and T43 SQLite-writer measurements below.

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

### [x] T41. Define query-index record identity and form-parameter classification

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

### [x] T42. Reduce Syntax Assistant query-index build memory and runtime

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

### [x] T43. Reduce SQLite insertion overhead in Syntax Assistant query-index build

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
