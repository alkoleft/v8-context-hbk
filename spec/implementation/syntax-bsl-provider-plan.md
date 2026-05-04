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
- `syntax related` performs bounded relationship traversal over owner/member/type-reference edges.
- Query commands resolve a default local index path and do not parse HBK files per query.

These capabilities are a useful first slice for BSL assistance, but the current JSON surface is
still a search-result DTO rather than an analyzer-safe provider contract.

The provider contract should not invent a second shape for facts already normalized by
`syntax export`. Query/provider JSON should reuse the accepted export field names and nested shapes
where applicable. The current query JSON is provisional and may change; compatibility with its
existing `SearchHit<SearchDocument>` serialization is not a goal when it conflicts with
export-compatible typed facts.

## Gap Matrix

| Area | Current State | Gap for ADR-0006 Goal | Planned Resolution |
| --- | --- | --- | --- |
| Public query JSON | CLI serializes `SearchHit<SearchDocument>` directly. | Internal search fields leak into public JSON; `parameters` mixes names and type names. | T48 returns export-compatible structured parameter facts and keeps search terms internal. |
| Callable details | `signatures` are text strings in `SearchDocument`; structured domain `Signature` is not preserved in query JSON. | Analyzer cannot reliably inspect parameter requiredness, per-parameter types or descriptions from query output. | Add provider DTOs for structured signatures using the `syntax export` signature shape where applicable. |
| Exact identity | JSON includes document ids, but command inputs mostly use names. `related` accepts only `--name`. | Analyzer workflows need stable disambiguation by id and owner/member, especially for same-name facts. | Add id-based and owner/member query entry points where ambiguity matters. |
| Relationship traversal | Graph covers owner/member/type-reference/return/constructor edges and accepted SKD flow. | Code-facing workflows need reliable paths for creation/configuration tasks, not only nearby docs. | Add UAT scenarios and edge coverage for selected BSL development tasks before adding new graph features. |
| Provider contract | No separate provider schema/envelope for query outputs. | Future analyzer cannot depend on response version, field semantics or field compatibility with export. | Define a provisional provider JSON contract anchored to `syntax export` shapes for shared facts. |
| Evidence from real BSL | Utility has been manually tried on RAT modules, but no durable scenario set exists. | Development may optimize for isolated API lookups rather than real code-analysis questions. | Add a small real-BSL scenario corpus under service data or fixture policy and promote only conclusions to spec. |
| Storage evaluation | T49 plans Tantivy comparison. | Storage speed alone does not prove analyzer usefulness. | Run T49 only against accepted provider workflows and deterministic JSON requirements. |

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
- Provider JSON needs a stable response envelope and typed fields for tools.
- Relationship traversal should accept an exact document id or owner/member input to avoid ambiguous
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
2. **T49: Measure storage/search alternatives against provider workflows.** Tantivy is evaluated
   only against exact lookup, constructor lookup, deterministic JSON and relationship workflows.
3. **T50: Define provider response contract.** Specify response envelope, schema/version fields,
   typed callable details, ambiguity/missing-result shape and export-compatibility rules for JSON
   output.
4. **T51: Preserve structured callable facts in the query index.** Store or reconstruct structured
   signatures/parameters for methods, constructors and events instead of returning only signature
   strings.
5. **T52: Add analyzer-safe identity queries.** Allow document-id and owner/member relationship
   roots where names are ambiguous; keep text UX simple but make JSON stable for tools.
6. **T53: Add BSL task scenario UAT.** Use real or source-backed BSL examples to validate
   constructor lookup, owner/member lookup and task-oriented relationship discovery.
7. **T54: Improve relationship coverage from accepted scenarios.** Add only the edges or parser
   facts needed by failed BSL task scenarios.
8. **T55: Decide provider boundary for downstream analyzers.** Choose whether the provider is CLI
   JSON only, a Rust library API, a file artifact contract, or a combination. Capture a new ADR if
   this changes integration architecture.

## Non-Goals for This Plan

- Implement a BSL parser, linter or diagnostics engine in this repository.
- Add network-hosted semantic search.
- Replace SQLite/FTS5 before T49 produces measured evidence.
- Stabilize all public contracts before the real BSL task scenarios are accepted.
