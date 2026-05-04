# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history:

- [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md)
- [archive/completed-tasks-t13-t17-t19-t24.md](archive/completed-tasks-t13-t17-t19-t24.md)
- [archive/completed-tasks-t25-t34.md](archive/completed-tasks-t25-t34.md)

Current status: T25-T34 are archived historical tasks. Their durable export, schema,
data-quality and performance conclusions live in `acceptance/baseline.md`,
`source-evidence.md`, `requirements/functional.md` and `implementation/components.md`.
T35 was explicitly reprioritized before T18 by the 2026-05-01 review of TOC-aware Syntax Assistant
reading gaps. T36 is now the first unchecked task before T18 because the query/search CLI must build
on the accepted schema v8 query-table export shape rather than the temporary schema v7
`table-fields.json` / `table-parameters.json` split. T37, T38 and T39 follow T36 before T18 to
remove the historical global-context event filename, classify event owners without introducing
cross-cutting semantic IDs, keep type-event `owner_path` out of consumer JSON and derive query table
role/identifier from Syntax Assistant syntax. T18 is complete. There is no unchecked active task
after T18; add the next scoped task before implementing new behavior. T13-T17 and T19-T24 are
archived historical tasks; their durable
performance conclusions live in `acceptance/baseline.md`, `implementation/performance-baseline-t13.md`
and `implementation/performance-variants.md`.

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


### [x] T35. Make Syntax Assistant reading TOC-aware for ambiguous source families

Depends on: T34. Explicitly reprioritized before T18 by the 2026-05-01 `/tmp/shcntx/` review.

Spec refs:

- FR-SH-003
- FR-SH-002
- FR-EXPORT-001
- NFR-COMPAT-001
- NFR-DIAG-001
- UAT-SH-011
- UAT-SH-013
- ADR-0005
- `spec/source-evidence.md`, Syntax Assistant TOC-aware reading findings
- `spec/implementation/components.md`

Scope:

- Treat duplicate-looking consumer facts as a Syntax Assistant reading/classification defect first.
- Derive source family, semantic owner and branch context from the TOC ancestor chain before page
  parsing emits typed domain records.
- Implement TOC classification as two layers: branch kind and record family. Branch categories such
  as Automation/external API guide context but do not become record families by themselves.
- Replace query table field/parameter ownership that relies only on stripped HTML paths with
  TOC-aware query table context, including nested `Работа с запросами.Таблицы запросов` branches.
- Classify module-event groups as `module_event` facts, including events currently found under
  global context event groups and metadata/form/service module branches.
- Add platform type classification for at least `regular`, `extension`, `primitive` and
  `metadata_template`.
- Derive extension base relationships only when TOC/HTML/link evidence proves the base type or
  role.
- Treat `Примитивные типы` as a shallow branch: direct children are primitive types; nested literal
  pages such as `Булево > Истина` and `Булево > Ложь` are not platform types.
- Preserve distinct semantic contexts for same-name module events and event-like platform
  type/object pages unless an explicit source-family merge rule is documented and tested.
- Keep placeholder-like records distinguishable by semantic owner/context.
- Add fixture coverage from real Syntax Assistant TOC/page structures for the reported ambiguous
  families.
- Keep raw HBK provenance (`source_hbk`, `toc_path`, `html_path`, `page_title`) out of consumer
  records unless a separate export-contract task changes FR-EXPORT-001. If a consumer-visible
  semantic discriminator is needed for exact lookup, define it as a platform fact derived from
  reading context, not as parser provenance.
- Preserve deterministic record order, diagnostics order and current parser-maintenance diagnostic
  provenance.
- Do not implement query commands, semantic search, runtime 1C introspection or broad parser
  rewrites in this task.

Expected artifacts:

- Model/extractor changes for TOC-aware semantic reading context.
- Domain model changes for branch kind, module events and platform type kind.
- Narrow export/index adapter changes only if needed to expose the derived semantic fact identity
  required by UAT-SH-013.
- Unit or fixture tests covering the reported duplicate event/table/platform-type/placeholder
  families, primitive shallow traversal, extension types and metadata-template types.
- Updated UAT-SH-013 with deterministic `jq` checks for the accepted semantic identity fields.
- Updated acceptance baseline and source evidence with the resolved reading behavior and any
  remaining ambiguous diagnostics.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-001
- UAT-SH-003
- UAT-SH-011
- UAT-SH-012
- UAT-SH-013
- Full CLI export for `shcntx_ru.hbk`
- Full CLI export for `shcntx_root.hbk`
- Targeted checks for `Метод дополнения периодов`, repeated global context event names,
  `ПередЗаписью` / `BeforeWrite` and placeholder-like records
- Targeted checks for module-event classification, primitive shallow traversal, one extension type
  and one metadata-template type
- `git diff --check`

Completion notes:

- Implemented in schema version 7 without adding raw HBK/TOC/HTML/page-title provenance to
  consumer records.
- `global-context-events.json` remains the required FR-EXPORT-001 adapter filename and now carries
  `record_kind=module_event` / `record_family=module_event` records.
- Fresh RU/root exports produced 697 module events and 1869 platform types per locale, while table
  field/parameter counts and parser diagnostics stayed at 588, 78 and 4 per locale.
- The target source did not expose primitive type records in `platform-types.json`; the T35 guard
  verifies that nested primitive literal pages are not emitted and any future `primitive_types`
  branch records are typed as `primitive`.
- Follow-up review tightened root/English guards for `Client application form...` module events and
  `Information` suffix branch classification.

### [x] T36. Replace flat query table files with schema v8 `query-tables.json`

Depends on: T35. Explicitly reprioritized before T18 by the schema v8 export-contract review.

Spec refs:

- FR-EXPORT-001
- FR-SH-002
- FR-SH-003
- NFR-COMPAT-001
- NFR-DIAG-001
- UAT-SH-011
- UAT-SH-012
- UAT-SH-013
- `spec/source-evidence.md`, Syntax Assistant TOC-aware reading findings
- `spec/implementation/components.md`

Scope:

- Raise the canonical consumer JSON export to `schema_version: 8`.
- Replace `table-fields.json` and `table-parameters.json` with `query-tables.json` in
  `metadata.json.files`.
- Do not delete stale files from older schema versions in reused output directories. Remove the
  current exporter mechanism that deletes files such as `enum-values.json`; the file inventory in
  `metadata.json` is the current export contract.
- Add a typed `QueryTable` domain record and route query table field/parameter extraction through
  table ownership derived from the TOC ancestor chain.
- Emit one `query-tables.json` record per real query-language/SDBL table page, including generic
  "Основная таблица" / "Main table" pages and additional table pages under the same owner family.
- Preserve the table family context on `query_table.owner_path`; do not repeat `owner_path` on nested
  fields or parameters.
- Add `table_role` with at least `primary`, `additional` and `unknown`. Treat "Основная таблица" /
  "Main table" as `primary`; treat other table pages under the same owner family as `additional`
  unless source evidence proves a more precise role.
- Use string names for query tables, nested fields and nested parameters. Do not use
  `{ primary, alias }` for this source family unless real source evidence proves aliases.
- Remove query table parameter `required` from the consumer JSON and from the internal query-table
  parameter model unless a reliable source contract is found.
- Keep `owner_path` out of derivative consumer records whose `owner` already identifies the owning
  semantic type: `type-methods.json`, `type-properties.json` and `constructors.json`.
- Preserve current module-event, platform-type, enum, availability, example and see-also behavior
  except for the schema version bump and file inventory change.
- Update README usage only after the implemented command output exists.

Expected artifacts:

- Model/extractor changes for `QueryTable`, table roles and string query table names.
- Export adapter changes for `query-tables.json`, schema version 8 and no stale-file deletion.
- Updated unit or fixture tests for query table grouping, primary/additional table roles, string
  child names, missing parameter `required` and derivative-record `owner_path` omission.
- Updated UAT-SH-011, UAT-SH-012 and UAT-SH-013 checks.
- Updated acceptance baseline and source evidence with schema v8 counts and any durable source
  findings about query table descriptions or aliases.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-011
- UAT-SH-012
- UAT-SH-013
- Full CLI export for `shcntx_ru.hbk`
- Full CLI export for `shcntx_root.hbk`
- Targeted checks for "Основная таблица", additional query tables, nested fields, nested parameters,
  absence of `required` in query table parameters and absence of `owner_path` in derivative records
- `git diff --check`

Completion notes:

- Implemented schema v8 `query-tables.json` for both context and streaming exporters.
- Added typed query table records, TOC-derived table-family `owner_path`, `table_role`, nested
  fields and nested parameters.
- Removed query table parameter `required` from the internal query-table parameter model and
  consumer JSON.
- Preserved stale older-schema files in reused output directories; `metadata.json.files` is the
  current inventory contract.
- Verified with `cargo fmt`, `cargo test --workspace`, full debug CLI exports for
  `shcntx_ru.hbk` and `shcntx_root.hbk`, UAT-SH-011/UAT-SH-012/UAT-SH-013 jq checks and
  `git diff --check`.

### [x] T37. Split event export files and remove the historical global-context event adapter name

Depends on: T36. Explicitly reprioritized before T18 by the 2026-05-04 JSON output planning review.

Spec refs:

- FR-EXPORT-001
- FR-SH-003
- NFR-COMPAT-001
- NFR-DIAG-001
- UAT-SH-014
- `spec/implementation/components.md`

Scope:

- Raise the canonical consumer JSON export to the next schema version after T36.
- Replace `global-context-events.json` in `metadata.json.files` with:
  - `module-events.json`
  - `type-events.json`
  - `unknown-events.json`
- Route current module-level/global-context event records into `module-events.json`.
- Include object/manager module handlers in `module-events.json` when TOC context identifies them as
  module handlers rather than type/object event pages.
- Route remaining type/form/extension/object event-like records into `type-events.json`.
- Emit `unknown-events.json` only for recoverable event records whose TOC/HTML evidence is
  insufficient for safe module/type classification.
- Do not add a cross-cutting semantic `id`, `owner_ref` or global identity model in this task.
- Preserve schema version 8 `owner_path` narrowing: do not reintroduce `owner_path` on derivative
  type methods, type properties, constructors or nested query table records.
- Do not make event splitting depend on `owner_path` fields that T36 removes. If event records need
  owner disambiguation, define it within the event record contract without weakening the T36
  omission rule.
- Keep raw HBK, TOC, HTML and page-title provenance out of consumer event records; diagnostics
  remain provenance-rich.
- Preserve current availability, signatures, examples, see-also and deterministic ordering unless
  the event file split requires a documented schema adjustment.

Expected artifacts:

- Domain/model and extractor changes for event-family classification.
- Export adapter changes for the three event files and schema version bump.
- Fixture tests covering at least one module-level/global event, one object-module event, one
  type/form event and one unknown-event fallback if source evidence produces one.
- Updated README only after the implemented command output exists.
- Updated UAT-SH-011, UAT-SH-012, UAT-SH-013 and UAT-SH-014 checks.
- Updated acceptance baseline and source evidence with event-file counts and any remaining
  classification gaps.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-011
- UAT-SH-012
- UAT-SH-013
- UAT-SH-014
- Full CLI export for `shcntx_ru.hbk`
- Full CLI export for `shcntx_root.hbk`
- Targeted checks that `global-context-events.json` is absent from `metadata.json.files`, module
  events are in `module-events.json`, type/form events are in `type-events.json` and unknown events
  are empty or diagnostic-backed.
- `git diff --check`

Completion notes:

- Implemented schema version 9 for both context and streaming exporters.
- Replaced the historical `global-context-events.json` inventory entry with `module-events.json`,
  `type-events.json` and `unknown-events.json`.
- Routed global context and explicit module TOC branches to `module-events.json`; routed
  type/object/form/form-extension/control event pages without explicit module TOC context to
  `type-events.json`.
- Fresh RU/root exports produced 47 module events, 650 type events and 0 unknown events per locale.
- Preserved T36 query-table shape and derivative-record `owner_path` omission; no event records
  expose raw HBK/TOC/HTML/page-title provenance, `id`, `owner_ref` or event-local `owner_kind`.
- Verified with `cargo fmt`, `cargo test --workspace`, full CLI exports for `shcntx_ru.hbk` and
  `shcntx_root.hbk`, UAT-SH-011/UAT-SH-012/UAT-SH-013/UAT-SH-014 jq checks and
  `git diff --check`.

### [x] T38. Move owner/object classification to platform type/object records for event consumers

Depends on: T37.

Spec refs:

- FR-EXPORT-001
- FR-SH-003
- NFR-COMPAT-001
- NFR-DIAG-001
- UAT-SH-014
- `spec/implementation/components.md`

Scope:

- Add a source-backed owner/object classification field to platform type/object records only when
  TOC evidence proves the classification.
- Use this owner/object classification to support event consumers without adding an event-local
  `owner.kind` taxonomy.
- Keep event records focused on event family, owner name/context, signatures and section facts.
- Do not add cross-cutting semantic IDs or `owner_ref` links.
- Do not reintroduce `owner_path` on derivative type methods, type properties, constructors or
  nested query table records. This task must build on the schema version 8 omission rule rather
  than weakening it.
- If source evidence is insufficient for a proposed owner/object kind, leave the field omitted and
  record the classification gap in source evidence or diagnostics.

Expected artifacts:

- Model/extractor/export changes for the owner/object classification field.
- Fixture tests for at least form/form-extension, metadata object/object module and regular
  platform type owner classifications when source evidence supports them.
- Updated UAT-SH-014 checks for owner/object classification on the owner records, not on an
  event-only `owner.kind` field.
- Updated acceptance baseline and source evidence with durable classification counts or gaps.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-014
- Full CLI export for `shcntx_ru.hbk`
- Full CLI export for `shcntx_root.hbk`
- Targeted checks that event records do not carry an event-only `owner.kind` field and derivative
  records still omit `owner_path`.
- `git diff --check`

Completion notes:

- Kept the canonical consumer JSON schema at version 9 and added optional `object_kind` only to
  `platform-types.json`.
- Derived `object_kind` from TOC-backed platform type context after `branch_kind` and `type_kind`:
  `regular_platform_type`, `managed_form`, `form_extension` and `metadata_object`.
- Event files still do not expose event-local owner/object taxonomy (`owner_kind` / `object_kind`),
  `id`, `owner_ref` or raw parser provenance.
- Full debug CLI exports for `shcntx_ru.hbk` and `shcntx_root.hbk` preserved T37 record-family
  counts and produced source-backed platform owner classification counts recorded in
  `spec/source-evidence.md` and `spec/acceptance/baseline.md`.
- Verification passed with `cargo fmt`, `cargo test --workspace`, full RU/root UAT exports,
  UAT-SH-014 jq checks and `git diff --check`.

### [x] T39. Keep type-event `owner_path` out of consumer JSON

Depends on: T38. Explicitly reprioritized before T18 by the 2026-05-04 regression report that
`owner_path` still appeared in JSON after the schema v8 narrowing.

Spec refs:

- FR-EXPORT-001
- FR-SH-003
- UAT-SH-011
- UAT-SH-013
- UAT-SH-014

Scope:

- Raise the canonical consumer JSON export to `schema_version: 10`.
- Keep `owner_path` only on owning semantic records: `platform-types.json`, module event context
  and query table records.
- Remove `owner_path` from `type-events.json` records; keep type-event owner context as one
  composed `owner` string that remains sufficient for exact owner/event lookup.
- Preserve `owner_path` omission on type methods, type properties, constructors and nested query
  table fields/parameters.
- Update README and spec truth so the event split does not weaken the schema v8 omission rule.

Expected artifacts:

- Export adapter change removing serialized `owner_path` from type events.
- Regression tests for context and streaming exporters.
- Updated UAT checks and acceptance/source evidence.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- Full CLI export for `shcntx_ru.hbk`
- Targeted jq checks for `schema_version: 10` and `type-events.json` owner_path absence
- `git diff --check`

Completion notes:

- Removed serialized `owner_path` from `type-events.json` while preserving the same semantic context
  in a composed `owner` string.
- Updated consumer schema version to 10 and narrowed FR/UAT/README wording.
- Verified with `cargo fmt`, `cargo test --workspace`, a fresh full RU CLI export and targeted jq
  checks.

### [x] T40. Derive query table role and identifier from Syntax Assistant syntax

Depends on: T39. Explicitly reprioritized before T18 by the 2026-05-04 query table
syntax/identifier regression report.

Spec refs:

- FR-EXPORT-001
- FR-SH-002
- FR-SH-003
- UAT-SH-011
- UAT-SH-012
- UAT-SH-013
- `spec/source-evidence.md`, Query Table Export Shape Findings
- `spec/implementation/components.md`

Scope:

- Raise the canonical consumer JSON export to `schema_version: 11`.
- Extract the `Синтаксис` / `Syntax` section for each query table page and expose it as a localized
  `query-tables.json.records[].syntax` object with `primary` and optional `alias` when present.
- Add `query-tables.json.records[].identifier` as a query-table-local consumer key.
- Derive `table_role` from `syntax.primary` shape before falling back to generic page names:
  - `syntax.primary` with at most two dot-separated semantic segments is `primary`;
  - `syntax.primary` with more than two segments is `additional`;
  - empty syntax keeps the existing `unknown` / generic-name fallback behavior.
- Treat `Таблица бизнес-процессов` / `Business Process Table` and similar metadata table pages as
  primary when their syntax is `БизнесПроцесс.<Имя бизнес-процесса>` /
  `BusinessProcess.<Business process name>`.
- Derive primary identifiers from the first syntax segment, for example `БизнесПроцесс` or
  `BusinessProcess`.
- Derive additional identifiers from the primary identifier plus the table page `name` with
  CamelCase normalization; whitespace, hyphens and punctuation are separators and are not copied.
- Keep query table display `name`, `owner_path`, nested fields/parameters, event files and
  derivative-record `owner_path` omission otherwise unchanged.
- Do not introduce cross-file semantic IDs, query behavior, search indexes or broad TOC
  refactors in this task.

Expected artifacts:

- Model/extractor/export changes for query table syntax, identifier and role derivation.
- Regression tests for business-process primary table classification and at least one additional
  table syntax.
- Updated UAT-SH-011/UAT-SH-012/UAT-SH-013 checks.
- Updated README, acceptance baseline and source evidence after verification.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- Full CLI export for `shcntx_ru.hbk`
- Full CLI export for `shcntx_root.hbk`
- Targeted jq checks for schema version 11, business-process primary table identifiers, additional
  query table identifiers, localized syntax fields and unchanged nested field/parameter shape
- `git diff --check`

Completion notes:

- Raised the canonical consumer JSON schema to version 11.
- Added query table localized `syntax` and `identifier` to the model and `query-tables.json` export.
- Derived `table_role` from query table `syntax.primary` before page-name fallback.
  Business-process table pages are now primary when syntax has the two-segment shape
  `БизнесПроцесс.<Имя бизнес-процесса>` / `BusinessProcess.<Business process name>`.
- Additional table identifiers are derived from the primary syntax segment plus CamelCase-normalized
  table `name`; `.Точки` / `.Points` tables remain `additional`.
- Full debug CLI exports for `shcntx_ru.hbk` and `shcntx_root.hbk` preserved existing record-family
  counts: 59 query tables, 588 nested fields, 78 nested parameters, 47 module events, 650 type
  events and 4 parser diagnostics per locale.
- Follow-up review clarified that additional identifier suffixes use CamelCase normalization and
  remove punctuation such as hyphens, e.g. `Таблица изменений бизнес-процессов` becomes
  `ТаблицаИзмененийБизнесПроцессов`.
- The exported `syntax` field follows the same localized-name shape as other dual-language Syntax
  Assistant facts: Russian pages expose the Russian expression as `primary` and the parenthesized
  English expression as `alias`; root pages expose only English `primary` when no alias exists.
- Verification passed with `cargo fmt`, `cargo test --workspace`, full RU/root exports, targeted
  jq checks and `git diff --check`.

### [x] T18. Design and implement the Syntax Assistant query command first slice

Depends on: T17, T35, T36, T37, T38, T39 and T40 unless this task is explicitly reprioritized.

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- NFR-QUERY-001
- UC-SH-003
- UC-SH-004
- UAT-SH-004, including environment override and default index path resolution
- UAT-SH-005
- UAT-SH-006
- UAT-SH-015
- ADR-0004
- `spec/source-evidence.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Treat ADR-0004 as the accepted command/index decision before coding. If implementation discovers
  that the accepted command shape, crate split or index artifact is wrong, update or supersede
  ADR-0004 and the implementation spec before changing code.
- Implement the first deterministic local search slice before semantic search:
  - build a local SQLite/FTS5 index directly from a Syntax Assistant HBK through the extraction
    pipeline;
  - exact lookup by primary name and alias;
  - exact owner/member lookup;
  - keyword search over names, aliases, signatures, type references and descriptions;
  - relationship traversal over owner/member and type-reference edges stored in an edge table.
- Keep query commands on a prebuilt local index. Do not parse `shcntx_*.hbk` in query commands.
- Open query indexes read-only so separate CLI processes can run `syntax get`, `syntax search` and
  `syntax related` concurrently against the same index. Build replacement indexes into a temporary
  file and atomically replace the target path; serialize concurrent `syntax index` writers with a
  lock.
- Do not add any export-directory read/import path in this task, including secondary or
  development-only modes. Debug artifacts may be write-only service data.
- Add default index path resolution so repeated interactive commands can omit `--index`: explicit
  command option first, `V8_CONTEXT_HBK_SYNTAX_INDEX` second and
  `.v8-context-hbk/syntax/index.sqlite` under the current working directory by default.
- Treat the first-slice overlay as selecting one effective index path. Do not implement multi-index
  merging or project-over-platform index composition in this task.
- Keep the lean consumer export shape from FR-EXPORT-001. If search needs structured links or page
  provenance, add a search-specific index/maintenance artifact instead of putting those fields back
  into consumer record-family files.
- Do not implement semantic search, embedding providers, network calls, graph database integration,
  caches, server mode, MCP or UI in this first slice.
- Measure query latency against NFR-QUERY-001 on the Russian Syntax Assistant data set.

Expected artifacts:

- Search/index library code and `v8-context-hbk syntax` command surface.
- Rebuildable SQLite index artifact generated by UAT, not committed to source.
- README usage for the implemented query commands only after the commands exist.
- Completion notes with query measurements and any relationship-quality gaps.
- Structured "see also" link extraction follow-up was not required for the SKD-filter UAT path
  because owner/member and type-reference edges were sufficient.

Completion notes:

- Added `syntax-helper-search` with SQLite/FTS5 index schema version `1`.
- Added `v8-context-hbk syntax export/index/get/search/related`.
- `syntax index` builds through the extraction pipeline; query commands use only the prebuilt index.
- Default index path and environment override are covered by command UAT; read-only query opens,
  atomic replacement, stale SQLite sidecar cleanup and writer-lock serialization are covered by
  focused unit tests.
- `shcntx_ru.hbk` UAT index contained 25,594 searchable documents, including event facts.
- NFR-QUERY-001 debug measurements: exact lookup `0.00s`, keyword search `0.11s`, fuzzy search
  `0.44-0.58s`, relationship search `0.07-0.08s`.

Verification:

- `cargo fmt` passed.
- `cargo test --workspace` passed.
- UAT-SH-004 passed, including environment override and default index path resolution.
- UAT-SH-005 passed.
- UAT-SH-006 passed.
- UAT-SH-015 passed.
- NFR-QUERY-001 measurement notes recorded for exact lookup, keyword search, fuzzy search and
  relationship search.
- `git diff --check` passed.
