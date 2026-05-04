# ADR-0006: Orient Syntax Scope Toward BSL Development and Analysis

Date: 2026-05-04.

Status: Accepted.

## Context

`v8-context-hbk` started as an independently testable extractor for 1C HBK help books and Syntax
Assistant facts. ADR-0001 keeps this repository standalone and uses file-level export as the first
integration boundary. ADR-0004 adds a `syntax` command group backed by a prebuilt local search
index, and ADR-0005 makes Syntax Assistant reading TOC-aware.

The current `syntax` surface is already more than batch extraction:

- `syntax export` writes canonical Syntax Assistant JSON records;
- `syntax index` builds a local query artifact;
- `syntax get`, `syntax constructors`, `syntax search` and `syntax related` retrieve API facts from
  that artifact.

Recent work exposed a contract problem: query JSON contained a `parameters` array that mixed
parameter names and parameter type names. That shape can be useful as internal search text, but it
is misleading for a developer, coding agent or future code analyzer trying to reason about BSL API
usage.

The project needs a durable direction for the `syntax` scope so future changes are not evaluated as
generic documentation search features only.

## Decision

The `syntax` scope is oriented toward successful help during BSL code development and analysis.

Treat Syntax Assistant extraction, query commands, relationship traversal and machine-readable JSON
as a local platform-API provider for:

- human BSL developers;
- coding agents assisting with BSL;
- a future BSL analyzer that needs documented platform API facts.

This repository does not become the BSL analyzer. It provides extracted platform facts and local
query/index contracts that can later serve such an analyzer without opening or parsing HBK books in
analyzer query paths.

Future `syntax` work must use this decision filter:

- Prefer precise callable facts over broad prose search: method/constructor signatures, parameter
  names, parameter types, requiredness, descriptions, return types, owner/member relationships and
  related platform objects.
- Keep public machine-readable query output typed and unambiguous. Search-only tokens, ranking
  features and presentation shortcuts must not appear as misleading JSON contract fields.
- Align provider/query JSON with the accepted `syntax export` consumer JSON shapes wherever both
  surfaces expose the same platform fact. For example, callable parameters should use the export
  shape with `name`, `required`, `types` and optional `description`, not a query-specific
  reinvention.
- Preserve deterministic local behavior: a developer tool or analyzer should query a prebuilt local
  artifact without network services, runtime 1C introspection or nondeterministic result ordering.
- Keep Syntax Assistant source truth behind the extraction/index boundary. Query commands and future
  analyzer-provider paths should not parse `shcntx_*.hbk` per query.
- Evaluate index/storage/search changes by whether they improve BSL development and code-analysis
  workflows, not only generic documentation search quality.

## Consequences

- FR-SH-SEARCH-001 and related query UAT cases should prioritize code-facing API facts, especially
  signatures, constructors, parameters, type references and relationship chains.
- Query JSON contracts need more rigor than text presentation. If a field is exposed to JSON, its
  shape should be meaningful for tools, not just convenient for FTS.
- The accepted `syntax export` consumer JSON is the preferred compatibility anchor for shared fact
  shapes. Current query JSON is provisional and may change to match export-compatible typed shapes.
- T48 should fix the ambiguous `SearchDocument.parameters` output by separating public structured
  parameter facts from internal searchable text.
- T49's Tantivy comparison must measure the current accepted workflows, including exact lookup,
  constructor lookup, deterministic JSON and relationship traversal. A faster full-text engine is
  not sufficient if it weakens the provider role.
- README may remain user-facing CLI documentation, but durable direction and acceptance criteria
  belong in `spec/`.

Implementation status after T48-T56:

- T48/T51 removed the ambiguous public `parameters` array and preserved callable parameters as
  structured `signatures[].parameters[]` facts.
- T52 implemented the provider response envelope with explicit `status`, `diagnostics`,
  `results[].fact` and query-only `results[].meta` fields.
- T53 added UAT-SH-017 as the source-backed BSL task scenario corpus for constructor lookup,
  owner/member lookup, relationship traversal and task-oriented query-table discovery.
- T54 improved relationship traversal for the accepted SKD chain without adding prose heuristics,
  parser scope or a new graph engine.
- T55 accepted ADR-0007: local CLI JSON over a prebuilt `syntax` index is the first downstream
  analyzer-provider boundary.
- T56 normalized analyzer-critical SQLite storage into relational tables for type identities,
  members, callables, signatures, parameters and type references. JSON fields are no longer the
  source of truth for inference-critical facts.

## Alternatives Considered

### Treat `syntax` as a generic documentation search CLI

Rejected.

Generic search is useful, but it does not provide enough guidance for output contracts. It would
allow shapes such as mixed parameter/type token arrays that are searchable but weak for code
analysis.

### Move BSL analysis into this repository now

Rejected.

The repository boundary remains HBK/Syntax Assistant extraction and local query/index artifacts.
BSL parsing, linting and analyzer diagnostics belong in a separate component unless a future ADR
changes the project scope.

### Make downstream `v8-context` integration the only guiding consumer

Rejected as the sole direction.

ADR-0001 keeps the first downstream integration boundary explicit, but `syntax` also has direct
local CLI and tooling value. The immediate product direction is practical BSL development and
analysis assistance, with future analyzer-provider compatibility as a design constraint.

## Implementation Plan

1. Update `spec/requirements/functional.md`:
   - state the `syntax` scope goal as BSL development and code-analysis assistance;
   - record future BSL analyzer integration as an intended consumer direction;
   - keep implementing a full BSL analyzer as a non-goal for this repository.
2. Update `spec/use-cases.md`:
   - add a BSL developer / code-analysis tool user;
   - add a use case for resolving code-facing platform API facts.
3. Update `spec/implementation/syntax-helper-query-cli.md`:
   - add the product direction and decision filter for future `syntax` work;
   - require typed, unambiguous machine-readable output for tool-facing JSON.
4. Update `spec/IMPLEMENTATION_TODO.md`:
   - make T48 explicitly follow this ADR when fixing parameter JSON;
   - keep queued search/index tasks aligned with code-analysis workflows.
5. When query/provider JSON exposes the same facts as `syntax export`, reuse the export field names
   and shapes unless there is source-backed reason to add query-specific wrapper metadata.
6. Do not change CLI behavior or index schema as part of this ADR by itself. Behavior changes must
   be done through the active task ledger and verified by tests/UAT.
7. For analyzer-facing storage, keep inference-critical facts in relational rows rather than JSON
   blobs or FTS text fields. Presentation/search projections may remain internal as long as provider
   facts are assembled from typed data.

## Verification

- [x] `spec/README.md` points agents to the current `syntax` direction.
- [x] `spec/requirements/functional.md` records BSL development/code-analysis assistance as the
      `syntax` scope goal.
- [x] `spec/use-cases.md` includes a BSL developer or code-analysis tool use case.
- [x] `spec/implementation/syntax-helper-query-cli.md` contains a decision filter for future
      query/search/index changes.
- [x] T48 references the BSL analyzer provider direction when fixing public query JSON.
- [x] The implementation plan records `syntax export` consumer JSON as the compatibility anchor for
      shared provider/query fact shapes.
- [x] T53 records source-backed BSL task scenarios in UAT.
- [x] T55 records the first downstream analyzer-provider boundary in ADR-0007.
- [x] T56 stores analyzer-critical type/member/callable facts in normalized relational tables
      instead of JSON fields.
