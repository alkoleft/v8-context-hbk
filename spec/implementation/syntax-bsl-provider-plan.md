# Syntax Scope BSL Provider Plan

Status: active planning note for ADR-0006.

## Goal

The `syntax` scope must help with BSL code development and analysis. It should let a human
developer, coding agent or future BSL analyzer query documented platform API facts from a prebuilt
local artifact without opening HBK books in the query path.

The provider target is not a BSL parser or linter inside this repository. The target is a precise
platform-API fact provider that another analyzer can call.

## Current Capabilities

- `syntax export` extracts canonical Syntax Assistant record-family JSON from `shcntx_*.hbk`.
- `syntax index` builds a local SQLite/FTS5 index from the normal Syntax Assistant extraction
  pipeline.
- `syntax get` supports exact name lookup and owner/member lookup.
- `syntax constructors` returns constructor signatures by type name, with compact text and optional
  details text.
- `syntax search` supports deterministic keyword and fuzzy search over the prebuilt index.
- `syntax related` performs bounded relationship traversal over owner/member/type-reference edges
  and accepts name, document-id and owner/member roots.
- Query commands resolve a default local index path and do not parse HBK files per query.

These capabilities are a useful first slice for BSL assistance, but the current JSON surface is
still a provisional CLI provider contract rather than a stabilized downstream analyzer boundary.

The provider contract should not invent a second shape for facts already normalized by
`syntax export`. Query/provider JSON should reuse the accepted export field names and nested shapes
where applicable. The current query JSON is provisional and may change; compatibility with its
older `SearchHit<SearchDocument>` serialization is not a goal when it conflicts with
export-compatible typed facts or the provider envelope implemented by T52.

## Gap Matrix

| Area | Current State | Gap for ADR-0006 Goal | Planned Resolution |
| --- | --- | --- | --- |
| Public query JSON | T52 implements the T50 provider response envelope for query commands, and T53 validates it through accepted BSL task scenarios. ADR-0007 selects local CLI JSON over a prebuilt `syntax` index as the first downstream analyzer-provider boundary. | The provider contract remains provisional and not yet a stabilized external protocol. Additional boundaries still need concrete consumer evidence. | Keep CLI JSON as the current provider boundary; add batch/library/service contracts only through a future ADR or task. |
| Callable details | T48/T51 preserve structured signatures in the query index through schema version `3` `signature_json`; signature text remains presentation/FTS data. T53 validates `HTTPСоединение` constructor-call parameters through UAT-SH-017. | Additional callable gaps should now come from failed accepted BSL scenarios, not isolated DTO review. | T54 adds only scenario-driven relationship or parser improvements. |
| Exact identity | T52 adds document-id lookup and relationship roots by document id and owner/member. | Additional analyzer-oriented batch APIs are not defined yet. | Use exact ids and owner/member roots through the CLI JSON provider boundary selected by ADR-0007. |
| Relationship traversal | Graph covers owner/member/type-reference/return/constructor edges and accepted SKD flow. | Code-facing workflows need reliable paths for creation/configuration tasks, not only nearby docs. | Add UAT scenarios and edge coverage for selected BSL development tasks before adding new graph features. |
| Provider contract | T50 defines and T52 implements a provisional provider schema/envelope for CLI JSON outputs. ADR-0007 keeps CLI JSON as the first analyzer-facing boundary. | Rust/library/file/service boundaries are intentionally not selected without a concrete downstream consumer. | Treat the CLI JSON envelope as the current provider boundary while storage and query capabilities continue to evolve. |
| Evidence from real BSL | T53 adds UAT-SH-017 for source-backed BSL development scenarios: constructor call, SKD owner/member access and accounting-register query-table discovery. | The scenario set is intentionally small and should expand only when real workflow gaps are found. | Use UAT-SH-017 as the acceptance corpus for T49/T54 before broad search/storage changes. |
| Storage evaluation | T49 plans Tantivy comparison. | Storage speed alone does not prove analyzer usefulness. | Run T49 against UAT-SH-017 plus the existing exact, constructor, provider JSON and relationship workflows. |
| Analyzer storage shape | Schema version `3` stores documents, lookup names, FTS content, relations and `signature_json`. | Type inference, expression-chain evaluation and member completion need relational facts for type identities, members, signatures, parameters and type references, not JSON blobs or FTS terms. | T56 designs and implements an analyzer-oriented SQLite schema revision without JSON columns for inference-critical facts and audits redundant presentation fields such as `preview`. |

## Target Use Cases and Solution Shape

### UC-SH-005A: Resolve Constructor Call

Input shape:

- code-facing question: `Новый HTTPСоединение(...)`;
- provider query: constructor lookup by type name or exact constructor document id.

Expected provider answer:

- all documented constructor overloads;
- ordered parameters per overload;
- each parameter's name, requiredness, type references and description when available;
- export-compatible parameter shape: `name`, `required`, `types`, optional `description`;
- stable owner/type identity;
- deterministic JSON and compact text for humans.

Resolution path:

- T48 fixes the immediate parameter JSON ambiguity.
- Follow-up provider DTO work preserves structured `Signature`/`Parameter` data in query output.
- UAT must assert `HTTPСоединение` exposes `Таймаут`, `ЗащищенноеСоединение` and
  `ИспользоватьАутентификациюОС` as parameter facts, not as interleaved strings.

### UC-SH-005B: Resolve Owner/Member Access

Input shape:

- code-facing question: `НастройкиКомпоновкиДанных.Отбор`;
- provider query: exact owner/member lookup.

Expected provider answer:

- member kind and stable id;
- owner identity;
- type references and return types;
- relationship edges to the referenced type and useful members.

Resolution path:

- Existing `syntax get --owner --member` covers the lookup.
- Provider JSON uses the versioned response envelope with typed fields for tools.
- Relationship traversal accepts an exact document id or owner/member input to avoid ambiguous
  same-name roots.

### UC-SH-005C: Find APIs for a BSL Task

Input shape:

- task-oriented query: `отбор скд`, `HTTP соединение`, `таблица регистра бухгалтерии`;
- provider query: keyword/fuzzy search followed by relationship traversal.

Expected provider answer:

- ranked candidate API facts;
- deterministic top hits;
- graph path explaining related constructors/properties/methods;
- enough edge evidence for a developer or agent to decide which API to use next.

Resolution path:

- Existing `syntax search` and `syntax related` provide the first slice.
- Add accepted BSL task scenarios before broad ranking changes.
- Relationship quality improvements should be driven by missing paths in those scenarios.

### UC-SH-005D: Analyzer-Safe Batch Lookup

Input shape:

- analyzer has one or more exact symbols from parsed BSL;
- provider query resolves names, owner/member pairs or ids against a prebuilt platform index.

Expected provider answer:

- versioned JSON response;
- no presentation-only or FTS-only fields;
- typed facts suitable for diagnostics, completions or code actions;
- explicit ambiguity and missing-result behavior.

Resolution path:

- Define provider JSON contract before adding analyzer-specific commands.
- Keep the contract local/offline and deterministic.
- Do not implement BSL parsing in this repository.

## Sequenced Plan

1. **T48: Fix public parameter JSON.** Separate internal search terms from typed public parameters.
   This removes the immediate blocker for constructor-call assistance.
2. **T50: Define provider response contract.** Specify response envelope, schema/version fields,
   typed callable details, ambiguity/missing-result shape and export-compatibility rules for JSON
   output. The accepted target envelope is recorded in
   `spec/implementation/syntax-helper-query-cli.md`: provider facts live under `results[].fact`,
   query-only metadata lives under `results[].meta`, and response-level ambiguity or missing-result
   behavior is expressed through `status` and `diagnostics`.
3. **T51: Preserve structured callable facts in the query index.** Completed with the T48
   schema-v3 structured callable fact change; future work should continue with the provider
   envelope/identity/scenario tasks instead of duplicating this storage fix.
4. **T52: Add analyzer-safe identity queries.** Completed: document-id lookup and relationship
   roots by document id and owner/member are available, plain-name UX remains, and provider JSON
   reports missing or ambiguous roots through `status` and diagnostics.
5. **T53: Add BSL task scenario UAT.** Completed: UAT-SH-017 validates constructor lookup,
   owner/member lookup and task-oriented query-table discovery against a rebuilt Russian Syntax
   Assistant index.
6. **T49: Measure storage/search alternatives against provider workflows.** Tantivy is evaluated
   only after the provider contract and BSL task scenarios are in place, and only against exact
   lookup, constructor lookup, deterministic JSON and relationship workflows.
7. **T54: Improve relationship coverage from accepted scenarios.** Add only the edges or parser
   facts needed by failed BSL task scenarios.
8. **T55: Decide provider boundary for downstream analyzers.** Completed by ADR-0007: local CLI
   JSON over a prebuilt `syntax` index is the first analyzer-facing provider boundary. Rust library
   APIs, analyzer-specific file artifacts, services and bulk APIs require a future ADR or task with
   a concrete consumer need.
9. **T56: Normalize analyzer storage.** Revise the SQLite index schema so type/member inference
   facts live in relational tables rather than JSON fields: type identities, owned members,
   callables, signatures, parameters and typed references. Remove or confine presentation-only
   duplicates where possible.
10. **T57: Define analyzer query primitives.** Specify provider operations for resolving type
    identities, listing members, resolving owner/member access, retrieving callable overloads and
    following type-reference edges without exposing SQLite tables as the public contract. Completed
    as a spec-only contract: the selected shape extends existing `syntax get`,
    `syntax constructors` and `syntax related` JSON query kinds instead of adding new top-level
    commands. Implementation remains T58.
11. **T58: Implement analyzer provider primitives.** Completed: the selected primitives are
    available through the CLI JSON boundary over normalized storage while preserving existing
    `get`, `constructors`, `search` and `related` workflows.
12. **T59: Add expression-chain provider UAT.** Validate type/member inference support as a sequence
    of provider calls over real Syntax Assistant data, without adding a BSL parser to this
    repository.
13. **T60: Harden ambiguity handling.** Ensure duplicate type/member cases return deterministic
    `ambiguous` diagnostics instead of hidden winner selection.
14. **T61: Evaluate analyzer batch lookup needs.** Decide from primitive/UAT evidence whether
    analyzer workflows need a batch CLI JSON command or another boundary decision.

## Non-Goals for This Plan

- Implement a BSL parser, linter or diagnostics engine in this repository.
- Add network-hosted semantic search.
- Replace SQLite/FTS5 before T49 produces measured evidence.
- Stabilize all public contracts before the real BSL task scenarios are accepted.
