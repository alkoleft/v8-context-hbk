# ADR-0011: Compute Syntax Fact Identity in the Domain Model During Reading

Date: 2026-05-08.

Status: Accepted.

Decision maker: project maintainer.

## Context

`v8-context-hbk` reads Syntax Assistant HBK pages through a TOC-aware extraction pipeline and then
feeds the extracted facts into several consumers:

- lean consumer JSON export in `hbk-syntax-export`;
- local SQLite/FTS search index in `syntax-helper-search`;
- future resolver/provider layers above the indexed facts.

Recent query-table and type-event regressions showed that identity rules were split across
consumers:

- query-table fields could be indexed under a generic parent such as
  `query_table:Основная таблица`;
- type events could collapse under the generic event-group owner
  `type_event:owner:События:ОбработкаВыбора`;
- `hbk-syntax-export` already had a more correct type-event owner projection, while
  `syntax-helper-search` reimplemented a similar rule locally.

ADR-0005 already establishes that Syntax Assistant reading must derive semantic ownership from the
TOC hierarchy before typed facts are emitted, and that the query index must not repair ambiguous
facts after extraction. ADR-0004 establishes the search index as a query artifact, not as the
semantic source of truth.

The missing architectural rule is where stable fact identity is computed and when it becomes part
of the extracted domain record.

## Decision

`syntax-helper-model` owns the shared identity mechanics for Syntax Assistant domain facts.

`syntax-helper-extract` must compute and fill identity for owning parent facts while reading data,
before records are passed to any sink. The first owning parent fact families are:

- `platform_type`;
- `query_table`;
- `system_enum` / metadata-property enum records.

Member and child records must reference or resolve through the already computed parent identity
instead of rebuilding owner identity from localized names, generic TOC labels or local HTML paths.
This applies to:

- type methods, type properties and constructors through their owning platform type identity;
- query table fields and parameters through their owning query-table identity;
- enum values through their owning enum identity;
- type events through the shared JSON-compatible semantic owner projection.

`hbk-syntax-export` and `syntax-helper-search` may wrap or present identity differently for their
own contracts, but they must not own source-reading identity rules. In particular:

- consumer JSON may keep omitting a public `id` field where FR-EXPORT-001 says no `id` is exposed;
- search documents may keep search-specific string ids such as
  `type_property:<owner_identity>:<member>` or `query_table_field:<query_table_identity>:<field>`;
- search/index code may still detect and report duplicate final document ids after identity
  normalization;
- search/index code must not independently decide which TOC labels count as parent owners.

Identity must remain semantic. It must not contain raw HBK paths, TOC index paths, HTML paths,
page-title duplicate markers or source-path suffixes.

## Consequences

- Reader output becomes the single source for parent fact identity.
- Search and export stop drifting when the same owner projection rule changes.
- Query-table and type-event bugs are fixed at the domain boundary instead of by adding
  search-only special cases.
- Streaming extraction may keep a narrow parent-identity prepass or equivalent read-phase
  normalization when final parent identities require duplicate-aware disambiguation across sibling
  source records.
- Final search document id strings remain an index contract. They are built from domain identities,
  but they are not the global domain model itself.

## Alternatives Considered

### Keep Identity Construction in `syntax-helper-search`

Rejected.

This repeats source-reading rules in the search layer. It already caused drift from
`hbk-syntax-export` for type-event owners and query-table member owners.

### Use Consumer JSON as the Source of Search Identity

Rejected as a direct dependency.

The JSON shape is useful evidence for the correct semantic projection, especially for type-event
owners, but FR-EXPORT-001 intentionally keeps consumer JSON lean and does not expose a stable `id`
field for every record family. Search should consume typed domain facts, not parse the consumer
adapter output.

### Put Raw Source Paths into Identities

Rejected.

Raw paths would make collisions disappear mechanically while leaking parser provenance into
semantic identity. ADR-0005 requires semantic TOC-derived identity instead.

### Compute Parent Identity Lazily in Each Consumer

Rejected.

Lazy consumer-specific computation keeps the current failure mode: every consumer must remember the
same owner rules and duplicate-disambiguation rules.

## Implementation Plan

1. Extend `syntax-helper-model` with explicit identity fields or typed identity accessors for
   owning parent facts.
2. Move reusable identity helpers into `syntax-helper-model`, including:
   - TOC duplicate-marker stripping;
   - semantic owner-path key construction;
   - query-table parent identity construction;
   - platform-type parent identity construction;
   - enum parent identity construction;
   - type-event semantic owner projection.
3. Update `syntax-helper-extract` so parent identities are filled before records are sent to a
   `SyntaxHelperSink`.
   - If duplicate-aware final identity needs all parent records, perform that normalization in the
     read phase, not in `syntax-helper-search`.
   - Keep raw provenance only in `SyntaxHelperSource` and diagnostics.
4. Update `syntax-helper-search` to prefer precomputed parent identity from records and to use
   model-owned fallback helpers only for synthetic test records or legacy in-memory fixtures that do
   not carry identity.
5. Update `hbk-syntax-export` to consume model-owned semantic projections instead of local
   duplicated owner logic.
6. Preserve FR-EXPORT-001 JSON shape unless a separate schema task explicitly adds public ids.

## Verification

- [x] `syntax-helper-model` tests cover parent identity helpers and type-event owner projection.
- [x] `syntax-helper-extract` tests prove parent facts receive identity during reading.
- [x] `syntax-helper-search` tests prove member document ids use precomputed parent identity.
- [x] `hbk-syntax-export` tests prove type-event owner output still matches schema version 11.
- [x] Full `shcntx_ru.hbk` index rebuild has no duplicate
      `type_event:owner:События:ОбработкаВыбора` document id and no generic
      `query_table:Основная таблица` member-owner ids.
