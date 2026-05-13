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
- retrieve BSL and SDBL/query-language global context scopes for analyzer code that needs all
  globally visible methods, properties and language facts available at a call site;
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
- may use `hbk-book`, `syntax-helper-language::extract_language_facts` and
  `syntax-helper-search::SearchIndexBuilder::add_language_fact` for selected `shlang_*`,
  `shquery_*` and `dcsui_*` language pages;
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

Minimal BSL-language primitive lookup:

```rust
use context_resolver_core::{
    CompositeResolver, ContextResolver, LanguageDomain, ResolveContext, SourceId, TypeLookup,
};
use context_resolver_search::LanguageSearchSource;

let shlang = SourceId::new("shlang");
let language = LanguageSearchSource::open_shlang_read_only("target/language.sqlite")?;
let resolver = CompositeResolver::new(vec![Box::new(language)]);
let response = resolver.resolve_type(
    TypeLookup::ExactName {
        source: Some(&shlang),
        domain: Some(LanguageDomain::BslLanguage),
        name: "Число",
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

T131 implementation note: platform type resolution now includes `TypeLookup::ExactAlias` so
downstream analyzers can resolve stable English aliases through the resolver boundary instead of
carrying analyzer-owned localized-name tables. `TypeInfo.metadata_template` exposes the
Syntax Assistant-owned metadata-template facts for platform types: metadata kind and template
parameters. The SQLite storage used to preserve those facts remains internal to
`syntax-helper-search`; resolver consumers receive only the public DTOs from
`context-resolver-core`.

Platform type template ownership remains in this repository. Consumers must not recognize
localized names such as `СправочникСсылка` or aliases such as `CatalogRef` to discover generated
metadata-object template types. The resolver DTOs must expose open HBK-owned type template
families and generated variants instead of a closed metadata-object-kind / generated-role enum.

Type template family derivation is data-driven:

- start from `alias_base` when a type template has an alias, otherwise use the root-locale
  `primary_base`;
- derive family roots from template bases that end with `Manager`;
- assign templates to the longest matching manager root, so longer roots such as
  `DocumentJournal` are tested before shorter roots such as `Document`;
- do not create fallback-prefix families for unassigned templates;
- for unassigned templates, score direct type-template type-reference links between that template and
  already derived families; assign only when exactly one family has direct references;
- leave templates with no direct-reference family or several candidate families unclassified with a
  diagnostic rather than guessing.

Template parameter names remain source parameter slots and binding evidence only. They must not be
used as family or variant semantics.

Members and callables keep their owner through the existing resolved `TypeId` / `owner_type_id`
relationship. They do not repeat owner template kind on every member fact. Instead, member,
callable return and parameter `TypeRef` values may carry a type template instance binding when a
source-backed type reference points from one type template to another. For example,
`DocumentObject<T>.Ссылка` is represented as a reference to the document-reference template with
the result template argument bound to the corresponding owner template parameter slot.

The storage/index implementation may persist type template family, generated-variant,
classification evidence, diagnostics and template bindings in its SQLite artifact, but SQLite columns
remain private rebuildable provider state. The public contract is the resolver/search Rust API:
lookup by open family/variant key and typed template binding DTOs on returned type references.

T141 verification note: `context-resolver-search` preserves template owner-parameter bindings for
callable parameter and overload return type references when the HBK-backed search index contains
source-backed type-template links. This is an adapter/DTO guarantee, not a downstream analyzer
implementation or a public SQLite table contract.

T146 implementation note: `context-resolver-core` now exposes first-class `global_context` lookup
through `GlobalContextQuery`, `GlobalContextLanguage` and `ResolvedGlobalContext`. The composite
resolver merges source-specific scopes for one requested language without treating source order as a
semantic winner rule. The HBK-backed platform adapter contributes `shcntx_*` global methods and
global properties only to the BSL global context; global properties are returned under
`ResolvedGlobalContext.properties` with no fake owner `TypeId`. The language adapter contributes
`shlang_*` facts to the BSL global context and `shquery_*` / `dcsui_*` facts to the SDBL/query
global context, preserving source/domain-qualified identities.

Type boundary correction after the downstream `v8-context` platform-facts review: HBK remains the
owner of source-backed platform type-template evidence, not the owner of generated configuration
type composition. `PlatformTypeTemplateKey`, `TypeTemplateBinding`, template parameter slots and
unresolved/ambiguous type-reference diagnostics remain HBK/provider evidence because they come from
`shcntx_*`. A future shared type/typegen layer may reuse provider-neutral DTOs or compose
`metadata object + platform template evidence -> Configuration type`, but it must not move HBK
extraction, Syntax Assistant classification, search-index ownership or resolver source adapters out
of this repository without a separate ADR.

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
variants only when a provider needs them. The enum values are semantic domains, not source-file
families; each provider still has its own `SourceId` inside the selected domain.

Examples:

- `platform_type:Структура` is a platform API type fact from the HBK provider.
- a BSL language `Строка` type is a BSL-language type fact.
- a query-language `Строка` type is a query-language type fact.
- a generated catalog object type is a configuration-domain type fact.
- an exported function in a common module is a source-code callable fact.

Same display names across these domains are not equivalent. Equivalence, conversion, construction,
return and ownership are explicit relations.

Domain ownership rules:

- `shcntx_*` platform context facts are `PlatformApi` facts. Platform types, members, callables,
  constructors, enums and platform-owned type-template facts may be exposed by the platform adapter.
- `shlang_*` facts are `BslLanguage` facts. BSL value types, language constructs, keywords,
  operators and language functions must come from a BSL-language provider, not from a same-name
  platform type.
- `shquery_*` facts are `QueryLanguage` facts. Query clauses, query value types, functions,
  operators, keywords and literals must remain query-domain facts even when their display names
  match BSL or platform names.
- `dcsui_*` facts also participate in `QueryLanguage`, but through a distinct source identity so
  data-composition expression and query-extension facts do not overwrite base query-language facts.
- Configuration metadata types are `Configuration` facts owned by a downstream metadata provider.
  They may reference or augment platform facts only through explicit relations such as `generated_from`
  or `augments`.
- Concrete generated configuration types are composed outside this repository from downstream
  metadata object identities plus HBK template evidence. HBK must not synthesize
  `Configuration`-domain fact ids by substituting metadata names into platform template names.
- Source-code declarations are `SourceCode` facts owned by a downstream source provider. They may
  shadow, override or call platform/configuration/language facts only through explicit source-backed
  relations; they do not replace another provider's identity.

Existing HBK-backed facts that remain platform-provider facts:

- platform types/objects, global methods/properties, module/type events, type members,
  constructors, enums and enum values extracted from `shcntx_*`;
- source-owned platform type-template families, variants, classification diagnostics and
  owner-parameter template bindings extracted from `shcntx_*`;
- explicit platform API type-reference, return, construct and member ownership edges that the
  platform index can prove from extracted `shcntx_*` facts.

Facts that must wait for another provider or explicit mapping task:

- BSL language types and language constructs from `shlang_*`;
- query-language clauses, query value types, functions, operators, keywords and literals from
  `shquery_*`;
- data-composition expression/query-extension language facts from `dcsui_*`;
- configuration metadata-generated object/manager/reference/value types;
- source-code declarations from common modules, object modules, forms and other project files;
- `shcntx_*` `query_table`, `query_table_field` and `query_table_parameter` provider documents
  until a language-domain task defines their resolver mapping or relation shape.

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
    ModuleContext(ModuleContextInfo),
    Enum,
    EnumValue,
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

pub struct ResolvedGlobalContext {
    pub id: FactId,
    pub language: GlobalContextLanguage,
    pub sources: Vec<SourceId>,
    pub methods: Vec<ResolvedCallable>,
    pub properties: Vec<ContextFact>,
    pub facts: Vec<ContextFact>,
}

pub enum GlobalContextLanguage {
    Bsl,
    Sdbl,
}

pub enum ModuleContextKind {
    Common,
    Object,
    Manager,
    Form,
    Command,
    RecordSet,
    Session,
    OrdinaryApplication,
    ManagedApplication,
    ExternalConnection,
    WebService,
    HttpService,
    Unknown,
    Unsupported,
}

pub struct ModuleContextInfo {
    pub language: GlobalContextLanguage,
    pub domain: LanguageDomain,
    pub kind: ModuleContextKind,
}

pub struct ModuleContextQuery<'a> {
    pub language: GlobalContextLanguage,
    pub domain: LanguageDomain,
    pub kind: ModuleContextKind,
    pub sources: &'a [SourceId],
}

pub struct ResolvedModuleContext {
    pub id: FactId,
    pub language: GlobalContextLanguage,
    pub domain: LanguageDomain,
    pub kind: ModuleContextKind,
    pub sources: Vec<SourceId>,
    pub self_member: Option<ContextFact>,
    pub properties: Vec<ContextFact>,
    pub methods: Vec<ResolvedCallable>,
    pub events: Vec<ResolvedCallable>,
    pub facts: Vec<ContextFact>,
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

    fn global_context(
        &self,
        query: GlobalContextQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedGlobalContext>, ResolveError>;

    fn module_context(
        &self,
        query: ModuleContextQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedModuleContext>, ResolveError>;

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

pub enum GlobalContextQuery<'a> {
    Language {
        language: GlobalContextLanguage,
        sources: &'a [SourceId],
    },
}
```

Global context is a first-class resolver concept, not a synthetic platform type. There is no single
universal global context for all languages: the analyzer-facing scopes are language-specific. The
first required scopes are:

- BSL global context: BSL-language facts plus platform global methods/properties from `shcntx_*` that
  are visible to BSL code, composed through explicit source/domain rules;
- SDBL/query-language global context: query-language facts from `shquery_*` and `dcsui_*` that are
  visible to query/expression analysis.

Platform global methods and properties must be reachable through the BSL `global_context` for
analyzer setup. Platform global methods are additionally reachable through ownerless callable-name
lookup for point queries; platform global properties stay in `global_context.properties` until a
separate task defines an explicit global-property point lookup. Resolver internals must not force
global facts under a fake `TypeId`. SDBL functions must remain in the query-language global context
and must not be folded into the BSL/platform scope by matching display names.

Module context is a separate resolver concept for platform-owned facts visible in a concrete 1C
module kind. The HBK-backed provider owns only source-backed platform module context facts:
platform global methods/properties, module events, event signatures and their availability when
Syntax Assistant/index evidence contains them. Metadata-owned facts stay outside this repository:
concrete forms, form attributes, form elements, module ownership and generated configuration types
belong to the metadata/source providers and are composed downstream by `v8-context`.

`ModuleContextKind` is a provider-neutral key. Localized names and aliases such as
`ЭтотОбъект` / `ThisObject` are returned as facts only when the provider has indexed evidence for
that member. Until HBK extraction/indexing stores dedicated predefined module members, the
HBK-backed adapter must report `NotFound` or `Unsupported` with diagnostics instead of fabricating
an analyzer-side fallback list.

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
- never choose a platform, configuration, source-code, BSL-language or query-language fact only
  because it appears earlier in source order, shares the same display name or has a more familiar
  source family.

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
- `shlang_*`, `shquery_*`, `dcsui_*`, configuration and source-code facts must not be folded into
  platform API identities without an explicit source-backed relation and a task that owns the
  mapping.

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
- semantic type-template lookup backs `resolve_type` for provider-owned generated platform
  template types without exposing localized platform type names to consumers;
- `members_by_type_id` backs `members`;
- `member_by_owner_type_id` backs owner/member resolution;
- `callable_by_id`, `callable_by_owner_type_id` and `constructors_by_type_id` back callable lookup;
- source-backed global method and global property facts participate in the BSL `global_context` and
  global methods back ownerless callable-name lookup for BSL-visible point queries;
- source-backed module-event facts participate in `module_context` when the search index preserves
  their provider-neutral module context kind; event signatures and availability are exposed through
  the same callable and availability DTOs used by other platform callables;
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

T146 implementation notes:

- `SearchIndex::documents_by_kind` backs global method/property enumeration from a prebuilt index;
  resolver lookup still does not parse HBK or expose SQLite row/table contracts.
- Ownerless platform callable lookup is intentionally limited to `global_method` documents. Owned
  method/constructor lookup still requires a resolved owner `TypeId`.
- Exact named member lookup with no matching member returns `NotFound`. Broad member listing for an
  owner with zero members remains an `Ok` empty set.
- Type-event documents that are listed as members keep `FactKind::Member` for exact id round trips.
  Their search document ids are built from read-phase `owner_identity`, matching ADR-0011's
  child/member boundary. Callable/event handling remains available through callable mapping where
  the caller uses the callable fact shape.

T152 implementation notes:

- `context-resolver-core` exposes first-class `module_context` lookup through
  `ModuleContextQuery`, `ModuleContextKind` and `ResolvedModuleContext`.
- `context-resolver-search` preserves `ModuleEventContext.kind` from indexed `module_event`
  documents as private provider index state and maps it to `ModuleContextKind` at the Rust resolver
  boundary. SQLite table names and storage columns remain private rebuildable provider state.
- The HBK-backed adapter currently provider-backs platform global methods/properties, module events,
  event signatures and availability for returned facts. Dedicated predefined self members and
  module-specific platform properties/methods remain explicit absence until extraction/indexing
  stores source evidence for them.
- The module context `FactId` is a provider-owned resolver handle. When a module context resolves,
  the same handle must round-trip through exact id lookup as a `ModuleContext` fact; unsupported or
  absent contexts must not synthesize that fact.
- `Command`, `RecordSet` and `Common` module contexts are not synthesized from localized names or
  analyzer compatibility lists by this slice.

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
- direct `shlang_*` primitive type pages map to `language_type` facts for `Null`,
  `Неопределено` / `Undefined`, `Число` / `Number`, `Строка` / `String`, `Дата` / `Date`,
  `Булево` / `Boolean` and `Тип` / `Type`; nested primitive literal pages such as
  `def_BooleanTrue` and `def_BooleanFalse` are not type facts;
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
- BSL global-context lookup returns known platform global methods/properties without requiring a fake
  owner type, SDBL global-context lookup returns query-language facts separately, and ownerless
  callable lookup can resolve a known BSL-visible global platform method by name;
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
