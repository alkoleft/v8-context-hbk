# Legacy Documentation and Evidence Index

OpenSpec is the primary source of truth for `v8-context-hbk`. Canonical
capability requirements live under `../openspec/specs/`; proposed and active
change scope, design, deltas and task state live under
`../openspec/changes/`.

The `spec/` directory is supporting legacy documentation, research, acceptance
evidence, ADR rationale and history. It does not own new normative requirements
or active task state. A legacy-only contract remains a binding baseline until
the smallest task-relevant portion is imported into an OpenSpec delta before
implementation edits that area.

## Relationship To OpenSpec

1. Canonical OpenSpec capability specs own current imported requirements.
2. An applicable active OpenSpec change owns proposed scope, design, deltas and
   task state; archived changes own completed change history.
3. Accepted ADRs in `decisions/` preserve rationale for hard-to-reverse
   decisions but do not override current OpenSpec contracts.
4. Requirements, use cases, implementation notes and acceptance artifacts in
   this directory remain binding legacy baseline or supporting evidence until
   the relevant contract is imported.
5. `archive/` contains historical task evidence, not active scope.

When OpenSpec conflicts with supporting material, OpenSpec takes precedence and
the supporting material must be reconciled.

## Specification Files

- `source-evidence.md`: current source observations, platform files and external reference anchors.
- `requirements/functional.md`: functional requirements and non-goals.
- `requirements/non-functional.md`: reliability, performance, diagnostics, compatibility and testability requirements.
- `use-cases.md`: users, jobs and externally observable use cases.
- `implementation/components.md`: crate boundaries, dependency rules and provisional implementation contracts.
- `implementation/performance-baseline-t13.md`: measured T13 performance/resource baseline,
  post-baseline performance updates and current implementation direction.
- `implementation/performance-variants.md`: saved performance/resource optimization variants and
  selection rules.
- `implementation/hbk-zero-copy-snapshot-experiment.md`: реестр гипотез zero-copy-снапшота T183,
  воспроизводимый протокол сравнения, изоляция ветвей/worktree и граница принятия решения.
- `implementation/hbk-zero-copy-x1-integration.md`: принятое решение X1, production-source/API
  ledger, точный X1-INT протокол и завершённые этапы canonical cutover/cleanup.
- `implementation/hbk-zero-copy-x1-cutover-inventory.md`: обязательный точный
  path/symbol ledger canonical cutover и последующей inventory-driven уборки,
  включая replacement owners, проверки и preserve-list SQLite contracts.
- `implementation/hbk-member-availability-layout-research.md`: первичные источники, оценки
  footprint и опровергаемые гипотезы AV3/AV4/AV5 для hot-layout global/type scopes,
  `AvailabilityContext` и составного X1-кандидата.
- `acceptance/hbk-zero-copy-snapshot-evidence.md`: неранжированные измерения кандидатов T183,
  полный разбор поведенческой эквивалентности, результаты отдельных операций и итоги
  зафиксированных критериев.
- `acceptance/hbk-x1-int-evidence.md`: принятые full-corpus, catalog/resolver и
  analyzer свидетельства полного X1-INT pass, на основании которых последующий
  reviewed canonical cutover завершён в T183.
- `acceptance/hbk-s83-av1-evidence.md`: дополнительный неранжированный проход T183 для
  полной enumeration глобальных BSL-методов с фильтром только по `AvailabilityContext`.
- `acceptance/hbk-s83-av2-evidence.md`: дополнительный неранжированный проход T183 для
  lookup, borrowed iteration, compact member set и full-payload access по форме результата A.
- `acceptance/hbk-s83-av4-evidence.md`: корректирующий неранжированный проход T183 по
  фактическим hot paths `v8-context`: filtered global scope, scope одного типа,
  end-to-end type lookup, payload, startup, retained memory и hot-layout evidence.
- `acceptance/hbk-s83-av5-evidence.md`: отдельный неранжированный проход T183 для
  составной гипотезы X1: global SoA, специализированный type-name hash,
  owner-contiguous member range, payload, startup и resource evidence.
- `acceptance/hbk-s83-av6-evidence.md`: отдельный неранжированный проход T183 для
  составной фильтрации `AvailabilityContext` в режимах `ANY`/`ALL`, включая
  H0, неизменённый X1 и X1-PROJECTED с сохраняемыми базовыми проекциями.
- `implementation/syntax-helper-query-cli.md`: draft architecture for the separate Syntax
  Assistant query/search CLI and its index/relationship model.
- `implementation/syntax-bsl-provider-plan.md`: ADR-0006 gap analysis, BSL/code-analysis use-case
  mapping and sequenced `syntax` provider improvement plan.
- `implementation/solution-context-resolve.md`: ADR-0008 Rust API design for resolving platform,
  BSL-language, query-language, configuration and source-code context facts through one
  source-neutral interface.
- `implementation/documentation-site.md`: ADR-0010 documentation-site generator and separate web
  app plan, global TOC merge contract and first implementation slices.
- `acceptance/baseline.md`: acceptance gates, commands, durable T9/T10 conclusions and success metrics.
- `acceptance/test-case-requirements.md`: rules for UAT and black-box test case specifications.
- `acceptance/uat-test-cases.md`: current UAT test case catalog.
- `decisions/`: ADRs and accepted decision records.
- `archive/`: completed milestones and historical task records
  (`completed-tasks-t0-t12.md`, `completed-tasks-t13-t17-t19-t24.md`,
  `completed-tasks-t25-t34.md`, `completed-tasks-t41-t47.md`,
  `completed-tasks-t48-t56.md`, `completed-tasks-t57-t65-t68-t85.md`,
  `completed-tasks-t66-t67-t86-t90.md`, `completed-tasks-t91-t110.md`,
  `completed-tasks-t111-t134.md`, `completed-tasks-t135-t142.md`,
  `completed-tasks-t143-t151.md`, `completed-tasks-t152-t164.md`,
  `completed-tasks-t165-t182.md`,
  `implementation-todo-2026-05-04.md`, `implementation-todo-2026-05-05.md`).

## External Files

- `../README.md`: end-user CLI documentation. Keep usage instructions there.
- `../openspec/config.yaml`: repository context and generation rules for new OpenSpec artifacts.
- `../openspec/specs/`: canonical imported capability requirements.
- `../openspec/changes/`: proposed, active and archived change artifacts and task state.
- `../AGENTS.md`: repository workflow rules for agents.
- `../scripts/infr/impl-prompt.md`: helper prompt for applying the selected OpenSpec change. It must not define contracts independently.

## Working Rules

- Add or change normative requirements through an OpenSpec change before the
  implementation tasks that depend on them.
- Before implementation edits an area governed only here, import the smallest
  task-relevant legacy contract into the active OpenSpec delta, including
  preservation scenarios for unchanged behavior.
- Add an ADR alongside the OpenSpec design when changing architecture,
  source-of-truth policy, public contract stability, integration strategy or a
  long-lived process. The ADR records rationale; OpenSpec owns current scope and
  contract state.
- Add or update supporting UAT cases when behavior must be validated through
  CLI/file-level workflows.
- After implementation, update affected supporting evidence, baselines,
  implementation notes and ADR rationale when durable conclusions changed.
- Keep historical task archives unchanged unless correcting historical evidence.

## Service Data Policy

Intermediate command reports, generated exports and one-off acceptance logs are service data. Do not
keep them as durable documentation unless their conclusions are promoted into a requirement,
OpenSpec change, acceptance baseline or ADR.

## Current Syntax Direction

The `syntax` command scope is oriented toward successful help during BSL code development and
analysis. Treat Syntax Assistant extraction, query commands, relationship traversal and JSON output
as a local platform-API provider for human developers, coding agents and a future BSL analyzer.
ADR-0006 owns this direction.

When changing `syntax` behavior, prefer precise callable facts, structured parameters, type
references, owner/member relationships, deterministic local queries and unambiguous machine-readable
output over generic documentation-search breadth.

## Current Export Contract

The current provisional Syntax Assistant consumer JSON contract is `schema_version: 11`.
`FR-EXPORT-001` owns the exact record-family shape; `acceptance/baseline.md` records the latest
validated counts, schema-changing task conclusions and query-index baselines. An applicable OpenSpec
capability and active change own future contract changes and task state.

ADR-0011 owns cross-consumer Syntax Assistant fact identity. Parent fact identity is a
`syntax-helper-model` / `syntax-helper-extract` responsibility and must be filled during reading;
search and export consume that domain identity instead of reimplementing parent-owner rules.

## Current Documentation Site Direction

ADR-0010 owns the documentation web-view direction. Do not route the full HBK documentation corpus
through MkDocs, Docusaurus or another page-per-route static-site generator as the first solution.
Split the work into a generator utility and a separate web app: generated manifest, merged global
TOC and page data first; search, Syntax Assistant index artifacts and web API endpoints later.
