# Solution Context Resolve Rust API

Status: design target accepted by ADR-0008. T67 implemented the first resolver core and
HBK-backed platform adapter slice.

## Purpose

The resolver API is the Rust integration boundary for a future application that builds a complete
1C solution context and uses it for validation, review, development assistance and diagnostics.
For the static-analysis integration path, the downstream application includes this boundary as
Cargo dependencies and calls it in process. HTTP, daemon, MCP and CLI-spawn transports are out of
scope for resolver hot-path lookup.

The application needs fast in-process access to facts from multiple sources:

- documented platform API facts extracted from Syntax Assistant HBK data;
- BSL language facts and the BSL type domain;
- query-language facts and the query-language type domain;
- configuration metadata-generated types;
- source-code declarations from modules, forms and other project sources.

This document defines the target API shape and composition rules. It does not implement
configuration parsing, BSL parsing, diagnostics or a service boundary.

## Product Requirements

Primary user: a Rust application that builds a complete solution context.

Primary jobs:

- resolve an identifier, stable id, owner/member pair or callable into a tool-readable fact;
- resolve a type in the correct source and language domain;
- list members for a resolved owner type;
- retrieve callable overloads with ordered parameters and return/result type references;
- merge candidates from platform, BSL, query-language, metadata and source-code providers without
  hiding ambiguity;
- preserve enough provenance and source identity for diagnostics and review explanations.

Non-goals:

- no BSL parser or query parser in this repository;
- no configuration metadata extractor in this repository;
- no runtime 1C introspection;
- no public SQLite table contract;
- no daemon, MCP server, HTTP API or network dependency;
- no CLI command execution as the normal Rust analyzer lookup path;
- no hidden winner selection when multiple domains contain the same display name.

## Dependency Integration Surface

The dependency-facing surface has two phases.

Analyzer lookup hot path:

- depend on `context-resolver-core` for `ContextResolver`, `ContextSource`, `CompositeResolver`,
  typed ids, fact DTOs and lookup response statuses;
- depend on concrete source adapters, currently `context-resolver-search`, for HBK-backed platform
  and language facts;
- open prebuilt source artifacts read-only and compose sources in process;
- call resolver methods directly from the analyzer without spawning `v8-context-hbk`, calling HTTP,
  reading SQLite tables or parsing HBK/HTML pages.

Provider setup or index-refresh phase:

- may use `hbk-book` and `syntax-helper-extract::SyntaxHelperReader` to read `shcntx_*` books;
- may stream extracted facts into `syntax-helper-search::SearchIndexBuilder` and write a local
  provider index;
- may rebuild source artifacts when platform HBK files or extractor versions change;
- must keep refresh failures separate from per-source analyzer diagnostics.

Minimal platform-provider wiring:

```rust
use context_resolver_core::{CompositeResolver, ContextResolver, ResolveContext, TypeLookup};
use context_resolver_search::PlatformSearchSource;

let platform = PlatformSearchSource::open_read_only("target/platform.sqlite")?;
let resolver = CompositeResolver::new(vec![Box::new(platform)]);
let response = resolver.resolve_type(
    TypeLookup::ExactName {
        source: None,
        domain: None,
        name: "HTTPСоединение",
    },
    &ResolveContext::all(),
)?;
```

Index build may be in-process too, but it is not part of the source-analysis hot path:

```rust
use hbk_book::HbkBook;
use syntax_helper_extract::SyntaxHelperReader;
use syntax_helper_search::{IndexMetadata, SearchIndexBuilder, build_index_from_builder};

let book = HbkBook::open("/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk")?;
let mut builder = SearchIndexBuilder::new();
SyntaxHelperReader::new(&book).extract_into(&mut builder)?;
build_index_from_builder("target/platform.sqlite", &IndexMetadata {
    locale: "ru".to_string(),
    source_locale: "ru".to_string(),
    source_hbk: book.path().display().to_string(),
    source_extraction_schema_version: 11,
}, builder)?;
```

The examples show ownership boundaries, not a stabilized copy-paste API contract. Exact helper
names may evolve while the public decision remains: lookup integrates by Cargo dependency through
resolver/source traits, not by transport or storage internals.

T129 implementation note: `context-resolver-search` now exposes adapter-level read-only open
constructors for the accepted source adapters. `syntax-helper-search` still owns the index schema
and build/open mechanics, but lookup-only analyzer code can depend on `context-resolver-search`
without importing `SearchIndex` directly just to open an existing provider database.

T130 verification note: the static-analysis dependency surface is covered by a consumer-style
integration smoke in `context-resolver-search`. The test keeps provider setup/index refresh and
analyzer lookup as separate modules: setup may build a deterministic index through
`syntax-helper-search`, while lookup opens that existing index only through
`context-resolver-search` read-only adapter constructors and composes sources through
`context-resolver-core`.

## Source And Language Domains

Resolution is domain-aware. A name alone is not identity.

```rust
pub enum LanguageDomain {
    PlatformApi,
    BslLanguage,
    QueryLanguage,
    Configuration,
    SourceCode,
}
```

The first API should keep this enum small and explicit. Future extension/project layering can add
variants only when a provider needs them.

Examples:

- `platform_type:Структура` is a platform API type fact from the HBK provider.
- a BSL language `Строка` type is a BSL-language type fact.
- a query-language `Строка` type is a query-language type fact.
- a generated catalog object type is a configuration-domain type fact.
- an exported function in a common module is a source-code callable fact.

Same display names across these domains are not equivalent. Equivalence, conversion, construction,
return and ownership are explicit relations.

Current source evidence for non-platform HBK syntax domains lives in
[`source-evidence.md`](../source-evidence.md). The `shlang_*`, `shquery_*` and `dcsui_*` books must
be analyzed before BSL-language or query-language providers are implemented. The first resolver core
must stay source-neutral enough to accept those providers without reshaping everything around the
current `shcntx_*` platform API index.

## Fact Model

The generic API resolves context facts, not only types.

```rust
pub enum FactKind {
    Type,
    Member,
    Callable,
    Constructor,
    Global,
    Enum,
    EnumValue,
    QueryTable,
    QueryField,
    QueryParameter,
    Keyword,
    Operator,
}
```

Identities are source-qualified typed wrappers:

```rust
pub struct SourceId(String);

pub struct FactId {
    pub source: SourceId,
    pub domain: LanguageDomain,
    pub kind: FactKind,
    pub local_id: String,
}

pub struct TypeId(FactId);
pub struct MemberId(FactId);
pub struct CallableId(FactId);
```

The current platform provider can keep using existing provider document ids as `local_id`, for
example `platform_type:ОтборКомпоновкиДанных`. The resolver boundary adds source and domain around
that local id so other providers can use their own identity schemes without collisions.

Facts should be owned DTOs so callers do not borrow SQLite rows, parser buffers or provider internals:

```rust
pub struct ContextFact {
    pub id: FactId,
    pub name: Name,
    pub owner: Option<FactId>,
    pub details: FactDetails,
    pub relations: Vec<FactRelation>,
}

pub enum FactDetails {
    Type(TypeInfo),
    Member(MemberInfo),
    Callable(CallableInfo),
    QueryTable(QueryTableInfo),
    Language(LanguageInfo),
}
```

The implementation may internally use `Arc` or provider-specific caches later, but the public first
slice should keep result ownership simple.

Typed convenience results must keep identity and source/domain context. Do not return naked
`TypeInfo`, `MemberInfo` or `CallableInfo` values from resolver convenience methods.

```rust
pub struct ResolvedType {
    pub id: TypeId,
    pub fact: ContextFact,
    pub info: TypeInfo,
}

pub struct ResolvedMember {
    pub id: MemberId,
    pub owner: TypeId,
    pub fact: ContextFact,
    pub info: MemberInfo,
}

pub struct ResolvedCallable {
    pub id: CallableId,
    pub owner: Option<TypeId>,
    pub fact: ContextFact,
    pub info: CallableInfo,
}
```

## Resolver Traits

The API has a source-neutral resolver and source-specific providers.

```rust
pub trait ContextResolver {
    fn resolve(
        &self,
        query: ResolveQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError>;

    fn resolve_type(
        &self,
        query: TypeQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedType>, ResolveError>;

    fn members(
        &self,
        owner: &TypeId,
        query: MemberQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedMember>, ResolveError>;

    fn callable(
        &self,
        query: CallableQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedCallable>, ResolveError>;
}

pub trait ContextSource {
    fn descriptor(&self) -> SourceDescriptor;
    fn capabilities(&self) -> SourceCapabilities;

    fn resolve_local(
        &self,
        query: ResolveQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError>;
}
```

`ContextSource` implementations may use SQLite, in-memory indexes or generated tables internally,
but those details stay behind the source boundary.

The first API should stay synchronous. The supported sources are local indexes and in-memory
snapshots; adding async traits or a runtime requirement would create a service boundary before it is
needed.

## Query And Response Shape

Queries use borrowed inputs:

```rust
pub struct ResolveContext<'a> {
    pub active_sources: &'a [SourceId],
    pub domain: Option<LanguageDomain>,
    pub scope: Option<&'a ScopeId>,
}

pub enum ResolveQuery<'a> {
    Id(&'a FactId),
    ExactName {
        domain: Option<LanguageDomain>,
        kind: Option<FactKind>,
        name: &'a str,
    },
    OwnerMember {
        owner: &'a TypeId,
        member: &'a str,
        kind: Option<MemberKind>,
    },
    Members {
        owner: &'a TypeId,
        kind: Option<MemberKind>,
    },
    Callable {
        owner: Option<&'a TypeId>,
        name: &'a str,
    },
}
```

Responses keep recoverable lookup outcomes as data:

```rust
pub enum ResolveStatus {
    Ok,
    NotFound,
    Ambiguous,
    Unsupported,
}

pub struct ResolveResponse<T> {
    pub status: ResolveStatus,
    pub facts: Vec<T>,
    pub candidates: Vec<ResolveCandidate>,
    pub diagnostics: Vec<ResolveDiagnostic>,
}
```

Rules:

- `Ok` may contain one or more facts when the query naturally returns a set, such as member listing.
- `NotFound` contains no facts and a diagnostic with the source/domain/kind that was searched.
- `Ambiguous` contains no selected fact and includes deterministic candidate summaries.
- `Unsupported` means the provider understood the source but not the query shape.
- `ResolveError` is reserved for infrastructure failures, not ordinary lookup outcomes.

## Composition Rules

The composite resolver queries active sources in deterministic order, but source order is not a
semantic override rule.

Exact id:

- route to the source encoded in the id;
- return `NotFound` if the source is active but does not contain the id;
- return `Unsupported` or `ResolveError` if the source id cannot be routed.

Exact name:

- query only the requested `domain` and `kind` when they are supplied;
- query all active compatible domains when they are omitted;
- return `Ambiguous` when more than one candidate remains;
- never choose a platform, configuration, BSL or query-language fact only because it appears earlier
  in source order.

Owner/member:

- require a resolved owner identity for analyzer-preferred calls;
- a convenience owner-name lookup must resolve the owner first and report owner ambiguity before
  filtering members;
- member listing returns direct members of the resolved owner unless a later requirement adds
  explicit inherited/effective-member expansion.

Cross-source relations:

- generated types, augmentations, inheritance, conversions, constructor result types and return
  types must be explicit `FactRelation`s;
- query-language facts may reference BSL or platform types, but they remain query-language facts;
- configuration/source providers may augment platform facts only through a declared relation such as
  `augments`, not by replacing source-qualified identities.

## Language Domain Analysis Gate

Before implementing BSL-language or query-language resolver providers, inspect representative TOC
and page shapes from:

- `shlang_ru.hbk` / `shlang_root.hbk` for BSL language constructs and type-domain facts;
- `shquery_ru.hbk` / `shquery_root.hbk` for query-language clauses, keywords, functions, operators
  and type/value facts;
- `dcsui_ru.hbk` / `dcsui_root.hbk` for data composition expression language and query-language
  extension constructs.

That analysis must choose domain-specific fact families and identity rules before any facts are
merged into the resolver. T66 completed this non-platform HBK domain-analysis gate before T67, so
T67 can implement the resolver core and first platform adapter slice without treating BSL/query
providers as implementation-ready. Real BSL/query providers still need the source-backed
fact-family decisions selected below.

T66 selected a minimal shared language-fact model for the first non-platform HBK slice. The current
source evidence does not justify platform-style model crates or export families for `shlang_*`,
`shquery_*` and `dcsui_*`; their pages are language reference pages, not platform API object/member
pages. The selected shared fact families are:

- `language_construct` for declarations, statements and grammar clauses;
- `language_type` for BSL primitive types and query/SKD value type facts;
- `language_function` for query and SKD expression functions;
- `language_operator` for BSL/query/SKD operators;
- `language_keyword` for keyword and keyword-modifier facts;
- `language_literal` for constants and literal forms.

T89 adds `syntax-helper-language` as the first small source-domain model/parser crate for those
language facts and wires them into `syntax-helper-search` as language document kinds. It does not
extend `syntax-helper-model` platform record families with non-platform language records. Consumer
export JSON for existing `syntax export` platform facts remains unchanged until a task defines a
language export surface.

Resolver identity rules for this slice:

- `LanguageDomain::BslLanguage` owns `shlang_*` facts.
- `LanguageDomain::QueryLanguage` owns `shquery_*` facts.
- `dcsui_*` facts use the resolver `QueryLanguage` domain with a distinct source family, because
  data composition expression/query-extension syntax participates in query-analysis workflows but
  must remain distinguishable from base query-language pages.
- Display names are never identities. `Строка` from `shlang:def_String`, `Строка`/`STRING` from
  query conversion and string-literal pages, and `Строка` parameter/return facts in SKD expression
  functions remain separate facts unless explicit relations connect them.
- Existing `shcntx_*` `query_table`, `query_table_field` and `query_table_parameter` index
  documents stay outside the first platform adapter and outside the first language resolver source.
  They may be related to future language facts only through explicit follow-up work.

## First Platform Adapter

The first platform adapter is implemented in `context-resolver-search` over
`syntax-helper-search::SearchIndex`.

Mapping:

- `type_identity_by_id`, `type_identities_by_name` and `type_identities_by_alias` back
  `resolve_type` for `LanguageDomain::PlatformApi`;
- `members_by_type_id` backs `members`;
- `member_by_owner_type_id` backs owner/member resolution;
- `callable_by_id`, `callable_by_owner_type_id` and `constructors_by_type_id` back callable lookup;
- `related_by_id_and_edge` backs explicit relation traversal for `has_type`, `returns`,
  `constructs` and `member_of`.

The adapter must not expose SQLite table names, rowids, FTS tokens, HBK paths, TOC paths, HTML paths
or page titles through generic facts.

Current `SearchIndex` also contains `query_table`, `query_table_field` and `query_table_parameter`
documents extracted from `shcntx_*`. T67 must not expose them through the first platform adapter as
generic platform facts. T66 selected the current decision: those facts remain CLI/provider facts for
now and are not the first `QueryLanguage` resolver provider. A later language-domain task must
define an explicit mapping or relation shape before exposing them through the source-neutral
resolver.

T67 implementation notes:

- `context-resolver-core` owns the source-neutral model, synchronous traits and composite resolver.
- `context-resolver-search` owns the platform adapter translation from `SearchIndex` hits and
  relation traversal into resolver facts.
- The platform adapter uses `type_identity_by_id`, `type_identities_by_name`,
  `members_by_type_id`, `member_by_owner_type_id`, `callable_by_id`,
  `callable_by_owner_type_id`, `constructors_by_type_id` and `related_by_id_and_edge`.
- Exact-name generic resolver lookup is intentionally limited to platform type identity lookup in
  this first adapter slice; broader name search remains the CLI/search-provider concern.
- Query-table provider documents stay hidden from the platform adapter.

T92 implementation notes:

- Platform adapter callable return/result mapping uses explicit `return_types` or edge-specific
  `related_by_id_and_edge` evidence only. Constructors no longer synthesize a return type from the
  owner name when `constructs` evidence is absent.
- Platform adapter relation traversal no longer falls back from edge-specific lookup to generic
  `related_by_id` filtering. Missing edge-specific evidence is returned as an empty relation set so
  resolver clients can distinguish absent evidence from real facts.

## First Language Adapter

T90 implements the first language-domain resolver adapter slice in `context-resolver-search` over
the T89 language-fact index shape.

Mapping:

- `shlang` source facts map to `LanguageDomain::BslLanguage`;
- `shquery` and `dcsui` source facts map to `LanguageDomain::QueryLanguage`, with distinct source
  ids so SKD expression/query-extension facts do not overwrite base query-language facts;
- `language_type` and the current `language_literal` facts back `resolve_type`;
- `language_function` backs callable lookup with ordered signatures, parameters and return/type
  names where T89 extracted them;
- `language_keyword`, `language_operator` and `language_construct` remain general context facts;
- relation traversal is backed by explicit extracted type-reference edges in the prebuilt index,
  not by same-name merging during resolver lookup.

The language adapter does not parse HBK pages in lookup calls, does not expose query-table provider
documents and does not add a public language export JSON contract.

## Verification Plan

The first implementation task should prove the API with behavior tests:

- a fake platform source and fake configuration source with the same type name return `Ambiguous`
  for unconstrained exact-name lookup;
- the same candidates resolve uniquely when the query includes a source id, domain or exact id;
- BSL `Строка` and query-language `Строка` are separate `TypeId`s;
- member listing by resolved owner id does not mix members from another source/domain with the same
  owner display name;
- callable lookup preserves callable identity, ordered parameters and return or constructor type
  references;
- platform adapter relation traversal preserves source-backed `has_type`, `returns`, `constructs`
  and `member_of` edges;
- a fake query table field can reference a BSL/query/platform type through an explicit relation;
- the platform adapter resolves `platform_type:ОтборКомпоновкиДанных`, lists its members and
  resolves at least one source-backed callable using a test index built through existing
  `syntax-helper-search` fixtures;
- the platform adapter traverses `НастройкиКомпоновкиДанных.Отбор` ->
  `ОтборКомпоновкиДанных` through `has_type` and verifies one source-backed callable `returns` or
  `constructs` edge when the selected fixture exposes it;
- exact type resolution, member listing, callable lookup and relation traversal are measured after
  source open against the NFR-RESOLVE-001 provisional `100 ms` target, or the implementation records
  measured misses and follow-up tasks.

Full real-index UAT should be added only when the platform adapter is implemented. Configuration,
BSL parser and query parser UAT belongs to the future provider that owns those sources.
