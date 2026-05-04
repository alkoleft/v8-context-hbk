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
There is one unchecked active task: T41. It blocks further T18 continuation because query-index
record identity must be settled against real Syntax Assistant data before expanding query/search
behavior.

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

### [ ] T41. Define query-index record identity and form-parameter classification

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
- Classify Syntax Assistant pages under form and form-extension `Параметры формы` branches as form
  attributes/parameters owned by the form or extension type. They must not be emitted as
  `platform_type` records.
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
