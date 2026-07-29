# ADR-0008: Add a Rust Context Resolver API for In-Process Solution Analysis

Date: 2026-05-05.

Status: Accepted as a specification and implementation direction.

## Context

ADR-0001 keeps `v8-context-hbk` standalone and uses file-level Syntax Assistant export as the first
future `v8-context` ingestion boundary. ADR-0006 orients the `syntax` scope toward BSL development
and code-analysis assistance. ADR-0007 keeps local CLI JSON over a prebuilt `syntax` index as the
first analyzer-facing provider boundary and explicitly defers Rust library APIs until a concrete
consumer proves the need.

The concrete next consumer is now a Rust application that forms a complete solution context:

- documented platform API facts from HBK / Syntax Assistant data;
- metadata-generated configuration types;
- source-code declarations from configuration modules and forms;
- BSL language facts and type system facts;
- query-language facts and type system facts.

The integration mode for that consumer is dependency-based: the downstream static-analysis project
will include this workspace's library crates as Cargo dependencies or workspace members. It will not
call an HTTP API, daemon, MCP server or similar transport for normal resolver queries.

The installed platform HBK set already contains separate source books for some of these domains:

- `shlang_ru.hbk` / `shlang_root.hbk` for BSL language syntax and language-level type facts;
- `shquery_ru.hbk` / `shquery_root.hbk` for query-language syntax;
- `dcsui_ru.hbk` / `dcsui_root.hbk` for data composition system expression language and query
  extension syntax.

The application goal is validation and review of 1C solutions, development assistance and future
diagnostics. Its hot path needs fast type lookup, member lookup and callable lookup, but the
resolution boundary cannot be platform-only. The same user-facing identifier can exist in different
source and language domains. BSL language types and query-language types must remain separate from
platform API types unless an explicit relation maps them.

## Decision

Add an in-process Rust context resolution boundary for complete-solution analysis.

The Rust API is a second provider boundary beside the accepted CLI JSON boundary, not a replacement
for it. CLI JSON remains the language-agnostic local integration surface. The Rust API exists for
Rust applications that need repeated low-latency lookups and a typed source-neutral interface across
platform, configuration, BSL-language and query-language providers.

For Rust static-analysis integration, the selected surface is direct library dependency on:

- `context-resolver-core` for source-neutral traits, typed identities, facts and lookup responses;
- `context-resolver-search` for HBK-backed platform/language source adapters over a prebuilt local
  search index;
- `syntax-helper-search` only as the local index open/build primitive behind those adapters.

Downstream analysis code should compose these providers in process. It must not depend on CLI
command wiring, SQLite table names, HBK parser internals or a transport/service boundary for normal
lookup calls.

The API must be centered on context facts, not only platform types:

- `ContextResolver` resolves source-qualified facts and provides convenience methods for type,
  member and callable lookups.
- `ContextSource` is the interface implemented by each indexed source provider: platform HBK,
  configuration metadata, source-code declarations, BSL language and query language.
- `LanguageDomain` distinguishes `PlatformApi`, `BslLanguage`, `QueryLanguage`, `Configuration`,
  `SourceCode` and future extension/project domains.
- `FactKind` distinguishes facts such as `Type`, `Member`, `Callable`, `Constructor`, `Global`,
  `Enum`, `EnumValue`, `QueryTable`, `QueryField`, `QueryParameter`, `Keyword` and `Operator`.
- `FactId`, `TypeId`, `MemberId` and `CallableId` are source-qualified typed wrappers. A display
  name alone is never a stable identity.
- `ResolveResponse<T>` reports `ok`, `not_found`, `ambiguous` or `unsupported` as data. Recoverable
  absence and ambiguity are not Rust errors. `ResolveError` is reserved for infrastructure failures
  such as unreadable indexes, unsupported schema versions or invalid source routing.

The composite resolver applies deterministic source ordering only to candidate order. It must not
silently select a hidden winner across platform, BSL, query-language, metadata or source-code
domains. Callers that want a narrower result must pass a source, language domain, fact kind, owner
identity or scope.

The first implementation direction is:

1. Add a source-neutral resolver core crate, with no HBK, SQLite, CLI or parser dependencies.
2. Analyze `shlang_*`, `shquery_*` and `dcsui_*` as separate HBK-backed language/domain sources
   before treating BSL-language or query-language facts as implementation-ready resolver providers.
3. Implement a platform API source adapter over `syntax-helper-search::SearchIndex`.
4. Leave configuration metadata extraction, BSL parser/source indexing, diagnostics and source-code
   declarations outside this repository until a concrete task assigns those providers.

## Boundary Contract

The Rust resolver boundary promises:

- local deterministic resolution over prebuilt source indexes or in-memory source snapshots;
- no HBK parsing, configuration parsing or source parsing in the lookup hot path;
- source-qualified identities for every returned fact;
- separate BSL language and query-language type domains;
- explicit ambiguity, missing-result and unsupported-query diagnostics;
- direct operations for resolving facts, resolving types, listing members, resolving one member,
  retrieving callable signatures and following type-reference or ownership edges;
- enough provenance for diagnostics at the source-provider boundary without leaking raw HBK paths or
  storage rows into generic context facts.

The boundary does not promise:

- a BSL parser, query parser, linter, diagnostic engine or code-action engine in this repository;
- configuration metadata extraction in this repository;
- runtime 1C introspection;
- public SQLite table, row or FTS-token contracts;
- a daemon, MCP server, HTTP API, network service or async runtime requirement;
- hidden compatibility with older provisional CLI/provider JSON shapes.

## Domain Separation Rules

- BSL language types and query-language types are distinct domains. For example, a BSL value type
  and a query-language value type with the same display name are separate `TypeId`s.
- BSL language and query-language facts must be sourced from their domain books or another explicit
  source-domain provider. They must not be inferred from `shcntx_*` platform API facts just because
  a same-name platform type or query table exists.
- Platform API types are not BSL language types by identity. If a platform API type participates in
  BSL expression typing, the relation is explicit, for example `maps_to`, `constructs`,
  `returns`, `has_type`, `member_of` or a future source-backed relation.
- Query tables, query fields and query parameters are query-language facts. They may reference BSL
  or platform value types through explicit type-reference edges, but they are not owned members of a
  platform API type unless a source-backed rule says so.
- Existing `query_table`, `query_table_field` and `query_table_parameter` facts from the current
  `shcntx_*` index are not part of the platform adapter and must not become `PlatformApi` facts.
  T166 supersedes the temporary T66 deferral by exposing them through an explicit
  `LanguageDomain::QueryLanguage` query-table source with a defined template/family-level mapping,
  ownership relation shape and type-reference relation shape.
- Configuration metadata and source-code declarations may generate or augment types. That
  generation or augmentation must be represented by explicit source-qualified identities and
  relations, not by replacing same-name platform facts.
- Exact id lookups route to one source. Exact name lookups query the requested source/domain/kind;
  if those constraints are omitted and more than one candidate remains, the result is ambiguous.
- Member lookup uses a resolved owner identity. Owner-name plus member-name lookup is a convenience
  that first resolves the owner and reports owner ambiguity before filtering members.

## Consequences

- ADR-0007 remains valid for CLI JSON consumers. This ADR adds a new boundary because the
  downstream consumer is now concrete and Rust-only enough to justify in-process lookup by Cargo
  dependency instead of command or network transport.
- `syntax-helper-search` may expose a platform-source adapter, but it must not become the
  source-neutral resolver model. The resolver core must not depend on SQLite or HBK crates.
- `syntax-helper-model` remains the HBK/Syntax Assistant extraction domain model. The resolver core
  may reuse compatible field concepts such as localized names, signatures and parameters, but it
  must not force configuration, BSL-language or query-language facts into Syntax Assistant record
  families.
- The full application can compose platform, metadata and source-code providers without making this
  repository parse configuration source or implement diagnostics.

## Alternatives Considered

### Keep CLI JSON as the Only Provider Boundary

Rejected for the new consumer.

CLI JSON remains useful and language-agnostic, but the full Rust application needs repeated
low-latency lookups, typed identifiers and a shared trait that non-HBK providers can implement
without spawning a process and serializing JSON for every type/member query.

### Make `syntax-helper-search` the Public Resolver API

Rejected.

`syntax-helper-search` is platform-HBK storage and query implementation. Making it the shared API
would force configuration and language providers to depend on platform index concepts, SQLite-shaped
queries or Syntax Assistant record families.

### Publish the SQLite Index Schema as the Integration Contract

Rejected.

The SQLite index remains a rebuildable internal artifact. It can support the platform adapter, but
future configuration, BSL-language and query-language providers may use different storage.

### Flatten All Sources Into One Name Map

Rejected.

Flattening loses source provenance and hides legitimate ambiguity between platform API, BSL
language, query language, metadata-generated and source-code facts.

## Implementation Plan

1. Add `spec/implementation/solution-context-resolve.md` with the API sketch, domain rules,
   source composition rules and verification plan.
2. Update requirements, use cases and non-functional requirements with the complete-solution
   context resolver direction.
3. Add a spec-only implementation task for this ADR.
4. Add a required analysis task for `shlang_*`, `shquery_*` and `dcsui_*` before the first resolver
   implementation slice. The analysis must define source-domain boundaries, first extractable fact
   families and follow-up implementation tasks for language/domain extraction or indexing.
5. In the implementation slice, add a new source-neutral resolver core crate. The crate should use
   borrowed query inputs (`&str`, `&FactId`) and owned result DTOs with typed id wrappers. Avoid
   async traits, service lifecycles, global caches and generic plugin systems in the first slice.
6. Add a platform source adapter over `syntax-helper-search::SearchIndex` that implements the core
   source trait by calling the existing normalized lookup methods.
7. Add focused tests with fake platform/configuration/BSL/query providers to prove ambiguity,
   domain separation and member lookup behavior before adding real configuration-source providers.
8. Keep CLI JSON and existing UAT behavior unchanged unless a later task deliberately aligns both
   surfaces.
9. Document the dependency-based static-analysis wiring in the implementation spec: hot-path
   consumers depend on resolver crates and open prebuilt indexes; setup/index-refresh code may use
   HBK extraction crates, but analyzer lookup code must not parse HBK files per query.

## Verification

- [x] This ADR records why the Rust API is now justified despite ADR-0007's earlier deferral.
- [x] The API direction is source-neutral and not platform-only.
- [x] BSL language types and query-language types are explicitly separate domains.
- [x] `shlang_*`, `shquery_*` and `dcsui_*` are called out as required HBK evidence sources before
      BSL-language or query-language providers are implemented.
- [x] The implementation plan avoids configuration parsing, BSL parsing, diagnostics and runtime
      introspection in this repository.
- [x] A follow-up task can implement the first slice without depending on public SQLite tables or
      query-time HBK parsing.

## More Information

### 2026-05-08: Dependency-Based Static-Analysis Integration

The downstream integration target is a Rust static-analysis project that will include the resolver
surface as a library dependency. This confirms the in-process boundary selected above and narrows
the integration surface away from HTTP, daemon, MCP or other transport-oriented APIs.

The intended dependency layers are:

- analysis hot path: `context-resolver-core` plus concrete source adapters such as
  `context-resolver-search`;
- HBK-backed platform provider state: a prebuilt `syntax-helper-search` index opened read-only by
  the adapter;
- setup or index-refresh phase only: `hbk-book`, `syntax-helper-extract` and
  `syntax-helper-search::SearchIndexBuilder` when the embedding application chooses to rebuild the
  provider index in process.

Do not make `v8-context-hbk-cli`, the SQLite schema, Syntax Assistant HTML parsing functions,
ordinary book export, documentation-site generation or web-app code part of the static-analysis
library contract. A future facade crate is allowed only if real downstream wiring shows that the
current `core + adapter + index` dependency set is too leaky.

### 2026-05-26: Worker-Safe Provider Snapshot

The downstream `v8-context` module-analysis worker model needs HBK-backed platform and language
facts to be shared across Rust worker threads. Sharing `syntax-helper-search::SearchIndex` directly
is not the selected contract because it owns a `rusqlite::Connection` and keeps lookup state tied to
the rebuildable SQLite provider artifact.

The selected follow-up direction is a provider-owned immutable `HbkFactSnapshot` materialized by
`v8-context-hbk` from existing `syntax-helper-search` SQLite provider indexes through coarse
provider-owned bulk reads. SQLite remains a private build/materialization input: the snapshot may
open a read-only connection while constructing owned nodes and indexes, but worker lookup must use
`Arc<HbkFactSnapshot>` plus worker-local handles and must not share SQLite connections, raw table
readers, broad resolver locks or analyzer-owned mirror tables.

The snapshot is a provider read model, not the source-neutral resolver DTO model. It owns compact
platform, BSL language and query-language facts with provider identities, ownership edges,
availability, type references and provenance required for lookup. `context-resolver-search` may
project snapshot nodes into `context-resolver-core` DTOs at adapter boundaries. Downstream analyzer
code must not depend on SQLite table names or re-own documented HBK facts as analyzer storage.

T167 measured this direction on a schema-16 `shcntx_ru` provider index. The release HBK-to-SQLite
index build took `14.50s` with `284360 KiB` peak RSS for `25415` documents. A temporary compact
SQLite-to-snapshot probe materialized the same index in `474 ms`, with `49112 KiB` peak RSS and
`34935365` estimated heap bytes. This accepts SQLite bulk materialization as the first
implementation source for the worker-safe snapshot. Direct HBK reading remains setup/index-refresh
input and comparison baseline, not the worker hot path.

### 2026-07-28: Borrowed Domain Catalog Refinement

The downstream context-formation measurements identified two hot-path domains
where the source-neutral owned resolver DTO contract is broader than the
consumer needs:

- BSL platform context, generated-self, metadata module members/events and
  typed availability;
- SDBL query tables with owner-scoped fields/parameters and metadata
  query-source classification.

ADR-0008 remains accepted. Its source-neutral `ContextResolver` boundary
continues to own cross-provider composition, generic identities, ambiguity
status and compatibility DTOs for `analyze-project`, platform-adapter and
explicit SQL/`SearchIndex` consumers.

For these two measured analyzer hot paths, `context-resolver-search` additionally
owns `HbkBslContextCatalog` and `HbkSdblQueryCatalog`. The catalogs hold the
existing worker-safe `Arc<HbkFactSnapshot>` and lend typed IDs and records from
the provider-owned arenas without first materializing generic `ContextFact` or
`Resolved*` answers. Snapshot-backed generic adapters delegate catalog-covered
acquisition and project once at the resolver boundary.

This refinement does not create another storage/read model. `HbkFactSnapshot`
and `HbkFactReadHandle` remain the only HBK snapshot storage/read owners.
`PlatformSearchSource` and `LanguageSearchSource` remain explicit
SQL/`SearchIndex` adapters for CLI, debug, index inspection and local sequential
flows; snapshot catalogs never fall back to them. Raw
`metadata.module-role.*` translation remains in `context-resolver-core`, and
catalog-covered selector behavior must not be copied into analyzer code.

The direct BSL analyzer handoff refines the same accepted boundary without
changing this decision. Catalog availability must expose existing typed
`AvailabilityContext` values plus borrowed available-since text rather than
raw snapshot string IDs. Stable fact-id, type-reference and signature
projection is shared from one upstream behavior owner between the retained
generic adapter and concrete downstream owned-output boundaries.
`HbkBslContextCatalog` itself still does not expose generic resolver DTOs, and
no projection holder or second adapter/storage layer is introduced.

The same projection owner also provides the existing exhaustive HBK
member-kind and callable-kind conversions. `PlatformSnapshotSource` and direct
analyzer owned-output boundaries reuse those functions, including the
established `LanguageFunction -> GlobalMethod` compatibility meaning. A
downstream kind table, analyzer-specific exception or separate query-kind
mapping is not an accepted seam.

The final direct handoff keeps inverse provider lookup and callable identity on
that same owner: `MemberQueryKind` is projected to `HbkTypeMemberKind` once,
and callable `FactId` construction classifies constructors separately while
all other HBK callables remain callable facts. Direct consumers also call the
existing `context-resolver-core` metadata module-role translator instead of
deriving HBK module context from analyzer metadata kinds. The six opaque
selectors and unknown-as-absence behavior remain unchanged.

The accepted implementation contract and consumer inventory are recorded by
OpenSpec change `expose-borrowed-hbk-domain-catalogs`.

The first BSL catalog slice implemented this refinement on 2026-07-28.
`PlatformSnapshotSource` now stores one `HbkBslContextCatalog`, catalog-covered
acquisition returns existing typed snapshot IDs/records, and generic resolver
DTOs are projected only by the compatibility adapter. The identical
preimplementation/after compatibility probe preserved every observable result
count and the 0.10 s warm command wall time; the direct catalog and snapshot
adapter both completed after removal of the source SQLite file. The exact
probe and measurements are retained under OpenSpec change
`expose-borrowed-hbk-domain-catalogs`.

The SDBL catalog slice completed the second domain gate on 2026-07-28.
`QueryTableSnapshotSource` now stores one `HbkSdblQueryCatalog`, delegates
table/field/parameter acquisition and retains only generic projection plus
private relation glue. The catalog and explicit SQL adapter share one
locale-aware selector owner; non-Russian and unknown identifiers return normal
absence. The identical `ff70367`/after compatibility probe preserved every
observable result count and the 0.11 s warm command wall time, and both direct
and compatibility snapshot paths completed after removal of the SQLite file.
Downstream analyzer handoff remains separately gated by the active OpenSpec
change.
