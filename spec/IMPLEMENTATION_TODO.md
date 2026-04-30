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
T35 is explicitly reprioritized before T18 by the 2026-05-01 review of TOC-aware Syntax Assistant
reading gaps: duplicate-looking consumer facts are symptoms of insufficient TOC-derived reading
context, not a request to restore raw provenance to consumer JSON. T13-T17 and T19-T24 are archived
historical tasks; their durable performance conclusions live in `acceptance/baseline.md`,
`implementation/performance-baseline-t13.md` and `implementation/performance-variants.md`.

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


### [ ] T35. Make Syntax Assistant reading TOC-aware for ambiguous source families

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
- Do not implement the query CLI, semantic search, runtime 1C introspection or broad parser
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
- UAT-SH-013
- Full CLI export for `shcntx_ru.hbk`
- Targeted checks for `Метод дополнения периодов`, repeated global context event names,
  `ПередЗаписью` / `BeforeWrite` and placeholder-like records
- Targeted checks for module-event classification, primitive shallow traversal, one extension type
  and one metadata-template type
- `git diff --check`

### [ ] T18. Design and implement the separate Syntax Assistant query CLI first slice

Depends on: T17 and T35 unless this task is explicitly reprioritized.

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
