## Task-local implementation plan (T173)

Implement the exact metadata-module BSL member operation in three direct steps:

1. `context-resolver-core` adds one required-source request and one HBK-owned answer union, a
   default unsupported source operation, and composite selector routing.
2. `PlatformSearchSource` adds a direct SQL intersection over existing
   canonical-name and module-context keys; `PlatformSnapshotSource` materializes
   the same `(module-context, canonical-name)` index family. Both use the
   existing global exact path for property/method, neither may call or filter
   `module_context`.
3. RED-first tests cover property/method/event, source/domain isolation and
   terminal outcomes. Run both touched package suites, formatting and strict
   OpenSpec validation.

### Structure impact

Searched owners and consumers: `context-resolver-core::{ContextSource,
ContextResolver, MetadataModuleContextLookup, ModuleContextQuery,
MemberQueryKind, ContextFact, ResolvedCallable, ResolveResponse}`, composite
routing and selector mapping; `context-resolver-search::{PlatformSearchSource,
PlatformSnapshotSource}`; `syntax-helper-search` SQL owner/name and
snapshot `globals_by_domain_name_kind`/module-event indexes; existing resolver
tests, downstream `context-provider`, provider specs and serialized contracts.
Search terms: `metadata_module_context`, `module_context`, `MemberQueryKind`,
`ResolvedModuleContext`, `ResolvedCallable`, `GlobalMethod`, `GlobalProperty`,
`ModuleEvent`, `globals_by_domain_name_kind`, `module_context_events` and
`module_event_names`, `SNAPSHOT_LAYOUT_VERSION` and binary-cache metadata.

Reused: metadata selector strings stay in core; existing `SourceId`, domain,
kind, fact identity, `ContextFact`, `ResolvedCallable`, status/error and direct
SQL/snapshot indexes. Added: one source-neutral exact request, one answer enum,
one direct source trait operation and the provider-owned direct module-event
primary-name lookup in existing SQL/snapshot index families. The existing derived
snapshot cache remains its sole owner; its layout version increments so an old
owner-only event index is rebuilt instead of being read as an exact index. No
field mirror, kind-to-result mapping outside the answer enum, second cache,
collection, reader/parser, serializer, new SQLite schema/table, dependency,
public re-export facade or metadata/analyzer coupling is added. Real consumers
are future context-provider BSL lookup and existing resolver callers; the
provisional external contract preserves one source-owned selected answer and
terminal resolver statuses.

### Reintroduction guard

Root cause: the only prior metadata-selector operation returns
`ResolvedModuleContext`, inviting every consumer to scan a materialized vector.
The sole permitted flow is `opaque metadata selector -> composite module-kind
dispatch -> source-owned indexed exact lookup -> existing property/callable
evidence`. Tests and structural review must reject exact-path calls to
`metadata_module_context`/`module_context`, `ResolvedModuleContext` traversal,
multi-source aggregation, analyzer/metadata imports, new cache/index families
outside the provider's SQL/snapshot event lookup, reuse of an earlier snapshot
layout version after the event-index key semantics change, and source/name
fallback.
