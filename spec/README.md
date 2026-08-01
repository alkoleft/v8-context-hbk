# Specification Index

The `spec/` directory is the durable source of truth for `v8-context-hbk`.

User-facing documentation, agent rules, code comments and chat decisions may explain or implement
the project, but they do not override this directory. If behavior, scope or contract intent changes,
update the relevant specification or ADR first, then update tasks and implementation.

## Truth Layers

Authoritative layers, from most specific to most operational:

1. Accepted ADRs in `decisions/` for architectural and process decisions.
2. Requirements, use cases, acceptance and implementation specifications in this directory.
3. `IMPLEMENTATION_TODO.md` for active task sequencing only.
4. `archive/` for completed historical task records. Archive files are evidence, not active scope.

When two source-of-truth files conflict, prefer the accepted ADR if it directly covers the decision.
Otherwise reconcile the specification files before implementing.

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
- `acceptance/hbk-zero-copy-snapshot-evidence.md`: неранжированные измерения кандидатов T183,
  полный разбор поведенческой эквивалентности, результаты отдельных операций и итоги
  зафиксированных критериев.
- `acceptance/hbk-s83-av1-evidence.md`: дополнительный неранжированный проход T183 для
  полной enumeration глобальных BSL-методов с фильтром только по `AvailabilityContext`.
- `acceptance/hbk-s83-av2-evidence.md`: дополнительный неранжированный проход T183 для
  lookup, borrowed iteration, compact member set и full-payload access по форме результата A.
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
- `IMPLEMENTATION_TODO.md`: first-unchecked-task ledger for implementation work.
- `archive/`: completed milestones and task history moved out of the active ledger
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
- `../AGENTS.md`: repository rules for agents. It must point agents back to this index before work.
- `../scripts/infr/impl-prompt.md`: helper prompt for the task loop. It must not define contracts independently.

## Working Rules

- Add or change requirements before adding implementation tasks that depend on them.
- Add an ADR before changing architecture, source-of-truth policy, public contract stability, integration strategy or long-lived process.
- Add or update UAT cases when a behavior must be validated through CLI/file-level user workflows.
- Keep `IMPLEMENTATION_TODO.md` short: active tasks, dependencies, spec references and verification only.
- After development, актуализируй `spec/`: update affected requirements, acceptance baselines,
  implementation notes, ADRs and the active task ledger before treating the work as complete.
- Move completed task detail to `archive/` when it stops being needed for the first unchecked task.

## Service Data Policy

Intermediate command reports, generated exports and one-off acceptance logs are service data. Do not
keep them as durable documentation unless their conclusions are promoted into a requirement,
acceptance baseline, task ledger entry or ADR.

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
validated counts, schema-changing task conclusions and query-index baselines. The active task ledger
owns current unchecked work.

ADR-0011 owns cross-consumer Syntax Assistant fact identity. Parent fact identity is a
`syntax-helper-model` / `syntax-helper-extract` responsibility and must be filled during reading;
search and export consume that domain identity instead of reimplementing parent-owner rules.

## Current Documentation Site Direction

ADR-0010 owns the documentation web-view direction. Do not route the full HBK documentation corpus
through MkDocs, Docusaurus or another page-per-route static-site generator as the first solution.
Split the work into a generator utility and a separate web app: generated manifest, merged global
TOC and page data first; search, Syntax Assistant index artifacts and web API endpoints later.
