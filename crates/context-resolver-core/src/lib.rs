use std::fmt;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceId(String);

impl SourceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LanguageDomain {
    PlatformApi,
    BslLanguage,
    QueryLanguage,
    Configuration,
    SourceCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FactKind {
    Type,
    Member,
    Callable,
    Constructor,
    Global,
    ModuleContext,
    Enum,
    EnumValue,
    QueryTable,
    QueryField,
    QueryParameter,
    Keyword,
    Operator,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactId {
    pub source: SourceId,
    pub domain: LanguageDomain,
    pub kind: FactKind,
    pub local_id: String,
}

impl FactId {
    pub fn new(
        source: SourceId,
        domain: LanguageDomain,
        kind: FactKind,
        local_id: impl Into<String>,
    ) -> Self {
        Self {
            source,
            domain,
            kind,
            local_id: local_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeId(pub FactId);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemberId(pub FactId);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallableId(pub FactId);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub primary: String,
    pub alias: Option<String>,
}

impl Name {
    pub fn new(primary: impl Into<String>, alias: Option<impl Into<String>>) -> Self {
        Self {
            primary: primary.into(),
            alias: alias.map(Into::into),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    HasType,
    Returns,
    Constructs,
    MemberOf,
    MapsTo,
    Augments,
    GeneratedFrom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactRelation {
    pub kind: RelationKind,
    pub target: FactId,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    pub name: String,
    pub target: TypeRefTarget,
    pub template_binding: Option<TypeTemplateBinding>,
}

impl TypeRef {
    pub fn resolved_id(&self) -> Option<&TypeId> {
        match &self.target {
            TypeRefTarget::Ok(id) => Some(id),
            TypeRefTarget::Unresolved | TypeRefTarget::Ambiguous(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRefTarget {
    Ok(TypeId),
    Unresolved,
    Ambiguous(Vec<TypeId>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub required: bool,
    pub types: Vec<TypeRef>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub parameters: Vec<Parameter>,
    pub return_types: Vec<TypeRef>,
    pub variadic: bool,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInfo {
    pub description: Option<String>,
    pub metadata_template: Option<MetadataTemplateInfo>,
    pub type_template_key: Option<PlatformTypeTemplateKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataTemplateInfo {
    pub metadata_kind: String,
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryTableRole {
    Primary,
    Additional,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactProvenance {
    pub source: SourceId,
    pub evidence_id: String,
    pub locale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryTableInfo {
    pub syntax: Option<Name>,
    pub identifier: Option<String>,
    pub table_role: QueryTableRole,
    pub owner_path: Vec<Name>,
    pub template_parameters: Vec<String>,
    pub description: Option<String>,
    pub source: Option<FactProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryFieldInfo {
    pub owner: FactId,
    pub types: Vec<TypeRef>,
    pub description: Option<String>,
    pub note: Option<String>,
    pub source: Option<FactProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryParameterInfo {
    pub owner: FactId,
    pub types: Vec<TypeRef>,
    pub description: Option<String>,
    pub default_value: Option<String>,
    pub source: Option<FactProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlatformTypeTemplateKey {
    pub family: String,
    pub variant: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeTemplateBinding {
    pub template_key: PlatformTypeTemplateKey,
    pub arguments: Vec<TemplateParameterBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateParameterBinding {
    OwnerParameter {
        owner_parameter_index: usize,
        target_parameter_index: usize,
    },
}

impl PlatformTypeTemplateKey {
    pub fn new(family: impl Into<String>, variant: impl Into<String>) -> Self {
        Self {
            family: family.into(),
            variant: variant.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberKind {
    Property,
    Method,
    Event,
    EnumValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberInfo {
    pub kind: MemberKind,
    pub types: Vec<TypeRef>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallableKind {
    Method,
    Constructor,
    GlobalMethod,
    Event,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallableInfo {
    pub kind: CallableKind,
    pub signatures: Vec<Signature>,
    pub return_types: Vec<TypeRef>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactDetails {
    Type(TypeInfo),
    Member(MemberInfo),
    Callable(CallableInfo),
    ModuleContext(ModuleContextInfo),
    Enum,
    EnumValue,
    Language,
    QueryTable(QueryTableInfo),
    QueryField(QueryFieldInfo),
    QueryParameter(QueryParameterInfo),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextFact {
    pub id: FactId,
    pub name: Name,
    pub owner: Option<FactId>,
    pub details: FactDetails,
    pub relations: Vec<FactRelation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailabilityContext {
    ThinClient,
    WebClient,
    MobileClient,
    Server,
    ThickClient,
    ExternalConnection,
    MobileApplicationClient,
    MobileApplicationServer,
    MobileStandaloneServer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailabilityInfo {
    pub contexts: Vec<AvailabilityContext>,
    pub since: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailabilityFact {
    pub id: FactId,
    pub availability: AvailabilityInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedType {
    pub id: TypeId,
    pub fact: ContextFact,
    pub info: TypeInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMember {
    pub id: MemberId,
    pub owner: TypeId,
    pub fact: ContextFact,
    pub info: MemberInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCallable {
    pub id: CallableId,
    pub owner: Option<TypeId>,
    pub fact: ContextFact,
    pub info: CallableInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalContextLanguage {
    Bsl,
    Sdbl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedGlobalContext {
    pub id: FactId,
    pub language: GlobalContextLanguage,
    pub sources: Vec<SourceId>,
    pub methods: Vec<ResolvedCallable>,
    pub properties: Vec<ContextFact>,
    pub facts: Vec<ContextFact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

impl ModuleContextKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Common => "common",
            Self::Object => "object",
            Self::Manager => "manager",
            Self::Form => "form",
            Self::Command => "command",
            Self::RecordSet => "record_set",
            Self::Session => "session",
            Self::OrdinaryApplication => "ordinary_application",
            Self::ManagedApplication => "managed_application",
            Self::ExternalConnection => "external_connection",
            Self::WebService => "web_service",
            Self::HttpService => "http_service",
            Self::Unknown => "unknown",
            Self::Unsupported => "unsupported",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleContextInfo {
    pub language: GlobalContextLanguage,
    pub domain: LanguageDomain,
    pub kind: ModuleContextKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveStatus {
    Ok,
    NotFound,
    Ambiguous,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveCandidate {
    pub id: FactId,
    pub name: Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveDiagnostic {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveResponse<T> {
    pub status: ResolveStatus,
    pub facts: Vec<T>,
    pub candidates: Vec<ResolveCandidate>,
    pub diagnostics: Vec<ResolveDiagnostic>,
}

impl<T> ResolveResponse<T> {
    pub fn ok(facts: Vec<T>) -> Self {
        Self {
            status: ResolveStatus::Ok,
            facts,
            candidates: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: ResolveStatus::NotFound,
            facts: Vec::new(),
            candidates: Vec::new(),
            diagnostics: vec![ResolveDiagnostic {
                message: message.into(),
            }],
        }
    }

    pub fn ambiguous(candidates: Vec<ResolveCandidate>) -> Self {
        Self {
            status: ResolveStatus::Ambiguous,
            facts: Vec::new(),
            candidates,
            diagnostics: vec![ResolveDiagnostic {
                message: "ambiguous context fact lookup".to_string(),
            }],
        }
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self {
            status: ResolveStatus::Unsupported,
            facts: Vec::new(),
            candidates: Vec::new(),
            diagnostics: vec![ResolveDiagnostic {
                message: message.into(),
            }],
        }
    }
}

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("source `{source_id}` failed: {message}")]
    SourceFailure {
        source_id: SourceId,
        message: String,
    },
    #[error("source `{0}` is not active or registered")]
    InvalidSource(SourceId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDescriptor {
    pub id: SourceId,
    pub domain: LanguageDomain,
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceCapabilities {
    pub exact_lookup: bool,
    pub type_lookup: bool,
    pub members: bool,
    pub callables: bool,
    pub relations: bool,
    pub global_context: bool,
    pub module_context: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberQueryKind {
    Property,
    Method,
    Event,
    EnumValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeLookup<'a> {
    Id(&'a TypeId),
    GeneratedSelfTemplate {
        source: Option<&'a SourceId>,
        domain: Option<LanguageDomain>,
        generated_self_role: &'a str,
    },
    PlatformTypeTemplate {
        source: Option<&'a SourceId>,
        domain: Option<LanguageDomain>,
        key: &'a PlatformTypeTemplateKey,
    },
    ExactName {
        source: Option<&'a SourceId>,
        domain: Option<LanguageDomain>,
        name: &'a str,
    },
    ExactAlias {
        source: Option<&'a SourceId>,
        domain: Option<LanguageDomain>,
        alias: &'a str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemberQuery<'a> {
    pub name: Option<&'a str>,
    pub kind: Option<MemberQueryKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallableLookup<'a> {
    Id(&'a CallableId),
    OwnerName {
        owner: Option<&'a TypeId>,
        name: &'a str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalContextQuery<'a> {
    Language {
        language: GlobalContextLanguage,
        sources: &'a [SourceId],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleContextQuery<'a> {
    pub language: GlobalContextLanguage,
    pub domain: LanguageDomain,
    pub kind: ModuleContextKind,
    pub sources: &'a [SourceId],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetadataModuleContextLookup<'a> {
    pub source: Option<&'a SourceId>,
    pub domain: Option<LanguageDomain>,
    pub metadata_module_role: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveQuery<'a> {
    Id(&'a FactId),
    ExactName {
        source: Option<&'a SourceId>,
        domain: Option<LanguageDomain>,
        kind: Option<FactKind>,
        name: &'a str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolveContext<'a> {
    pub active_sources: &'a [SourceId],
    pub domain: Option<LanguageDomain>,
    pub scope: Option<&'a str>,
}

impl<'a> ResolveContext<'a> {
    pub fn all() -> Self {
        Self {
            active_sources: &[],
            domain: None,
            scope: None,
        }
    }

    pub fn is_source_active(&self, source: &SourceId) -> bool {
        self.active_sources.is_empty() || self.active_sources.iter().any(|id| id == source)
    }
}

pub trait ContextSource {
    fn descriptor(&self) -> SourceDescriptor;

    fn source_id(&self) -> Option<&SourceId> {
        None
    }

    fn capabilities(&self) -> SourceCapabilities;

    fn resolve(
        &self,
        query: ResolveQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError>;

    fn resolve_type(
        &self,
        query: TypeLookup<'_>,
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
        query: CallableLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedCallable>, ResolveError>;

    fn global_context(
        &self,
        query: GlobalContextQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedGlobalContext>, ResolveError> {
        let _ = (query, context);
        Ok(ResolveResponse::unsupported(
            "context source does not expose global context",
        ))
    }

    fn module_context(
        &self,
        _query: ModuleContextQuery<'_>,
        _context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedModuleContext>, ResolveError> {
        Ok(ResolveResponse::unsupported(
            "context source does not expose module context",
        ))
    }

    fn related(
        &self,
        source: &FactId,
        kind: RelationKind,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError>;

    fn availability(
        &self,
        _source: &FactId,
        _context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<AvailabilityFact>, ResolveError> {
        Ok(ResolveResponse::unsupported(
            "context source does not expose availability facts",
        ))
    }
}

pub trait ContextResolver {
    fn resolve(
        &self,
        query: ResolveQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError>;

    fn resolve_type(
        &self,
        query: TypeLookup<'_>,
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
        query: CallableLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedCallable>, ResolveError>;

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

    fn metadata_module_context(
        &self,
        _query: MetadataModuleContextLookup<'_>,
        _context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedModuleContext>, ResolveError> {
        Ok(ResolveResponse::unsupported(
            "context resolver does not expose metadata module context",
        ))
    }

    fn related(
        &self,
        source: &FactId,
        kind: RelationKind,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError>;

    fn availability(
        &self,
        source: &FactId,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<AvailabilityFact>, ResolveError>;
}

pub struct CompositeResolver {
    sources: Vec<Box<dyn ContextSource>>,
}

impl CompositeResolver {
    pub fn new(sources: Vec<Box<dyn ContextSource>>) -> Self {
        Self { sources }
    }
}

pub struct WorkerSafeCompositeResolver {
    sources: Vec<Box<dyn ContextSource + Send + Sync>>,
}

impl WorkerSafeCompositeResolver {
    pub fn new(sources: Vec<Box<dyn ContextSource + Send + Sync>>) -> Self {
        Self { sources }
    }
}

trait ActiveContextSources {
    fn find_active_source<'a>(
        &'a self,
        context: &ResolveContext<'_>,
        source_id: &SourceId,
    ) -> Option<&'a dyn ContextSource>;

    fn for_each_active_source<'a, E>(
        &'a self,
        context: &ResolveContext<'_>,
        visit: impl FnMut(&'a dyn ContextSource) -> Result<(), E>,
    ) -> Result<(), E>;
}

impl ActiveContextSources for CompositeResolver {
    fn find_active_source<'a>(
        &'a self,
        context: &ResolveContext<'_>,
        source_id: &SourceId,
    ) -> Option<&'a dyn ContextSource> {
        self.sources.iter().map(Box::as_ref).find(|source| {
            source_is_active(*source, context) && source_id_matches(*source, source_id)
        })
    }

    fn for_each_active_source<'a, E>(
        &'a self,
        context: &ResolveContext<'_>,
        mut visit: impl FnMut(&'a dyn ContextSource) -> Result<(), E>,
    ) -> Result<(), E> {
        for source in &self.sources {
            let source = source.as_ref();
            if source_is_active(source, context) {
                visit(source)?;
            }
        }
        Ok(())
    }
}

impl ActiveContextSources for WorkerSafeCompositeResolver {
    fn find_active_source<'a>(
        &'a self,
        context: &ResolveContext<'_>,
        source_id: &SourceId,
    ) -> Option<&'a dyn ContextSource> {
        self.sources
            .iter()
            .map(Box::as_ref)
            .map(|source| source as &dyn ContextSource)
            .find(|source| {
                source_is_active(*source, context) && source_id_matches(*source, source_id)
            })
    }

    fn for_each_active_source<'a, E>(
        &'a self,
        context: &ResolveContext<'_>,
        mut visit: impl FnMut(&'a dyn ContextSource) -> Result<(), E>,
    ) -> Result<(), E> {
        for source in &self.sources {
            let source = source.as_ref() as &dyn ContextSource;
            if source_is_active(source, context) {
                visit(source)?;
            }
        }
        Ok(())
    }
}

fn source_is_active(source: &dyn ContextSource, context: &ResolveContext<'_>) -> bool {
    context.active_sources.is_empty()
        || context
            .active_sources
            .iter()
            .any(|active| source_id_matches(source, active))
}

fn source_id_matches(source: &dyn ContextSource, expected: &SourceId) -> bool {
    match source.source_id() {
        Some(source_id) => source_id == expected,
        None => source.descriptor().id == *expected,
    }
}

fn metadata_module_context_kind(selector: &str) -> Option<ModuleContextKind> {
    match selector {
        "metadata.module-role.common" => Some(ModuleContextKind::Common),
        "metadata.module-role.command" => Some(ModuleContextKind::Command),
        "metadata.module-role.object" => Some(ModuleContextKind::Object),
        "metadata.module-role.manager" => Some(ModuleContextKind::Manager),
        "metadata.module-role.form" => Some(ModuleContextKind::Form),
        "metadata.module-role.record-set" => Some(ModuleContextKind::RecordSet),
        _ => None,
    }
}

impl<T: ActiveContextSources> ContextResolver for T {
    fn resolve(
        &self,
        query: ResolveQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        if let ResolveQuery::Id(id) = query {
            let Some(source) = self.find_active_source(context, &id.source) else {
                return Ok(ResolveResponse::not_found(format!(
                    "source `{}` is not active",
                    id.source
                )));
            };
            return source.resolve(query, context);
        }

        let mut facts = Vec::new();
        let mut candidates = Vec::new();
        let mut unsupported = Vec::new();
        self.for_each_active_source(context, |source| {
            if let ResolveQuery::ExactName {
                source: Some(query_source),
                ..
            } = query
                && !source_id_matches(source, query_source)
            {
                return Ok(());
            }
            let response = source.resolve(query, context)?;
            match response.status {
                ResolveStatus::Ok => facts.extend(response.facts),
                ResolveStatus::Ambiguous => candidates.extend(response.candidates),
                ResolveStatus::Unsupported => unsupported.extend(response.diagnostics),
                ResolveStatus::NotFound => {}
            }
            Ok(())
        })?;
        if !candidates.is_empty() {
            candidates.extend(facts.iter().map(CandidateView::candidate));
            return Ok(ResolveResponse::ambiguous(candidates));
        }
        if facts.is_empty() && !unsupported.is_empty() {
            return Ok(ResolveResponse {
                status: ResolveStatus::Unsupported,
                facts: Vec::new(),
                candidates: Vec::new(),
                diagnostics: unsupported,
            });
        }
        Ok(single_or_ambiguous(facts, "context fact not found"))
    }

    fn resolve_type(
        &self,
        query: TypeLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedType>, ResolveError> {
        if let TypeLookup::Id(id) = query {
            let Some(source) = self.find_active_source(context, &id.0.source) else {
                return Ok(ResolveResponse::not_found(format!(
                    "source `{}` is not active",
                    id.0.source
                )));
            };
            return source.resolve_type(query, context);
        }

        let mut facts = Vec::new();
        let mut candidates = Vec::new();
        let mut unsupported = Vec::new();
        self.for_each_active_source(context, |source| {
            if let TypeLookup::ExactName {
                source: Some(query_source),
                ..
            }
            | TypeLookup::ExactAlias {
                source: Some(query_source),
                ..
            }
            | TypeLookup::PlatformTypeTemplate {
                source: Some(query_source),
                ..
            }
            | TypeLookup::GeneratedSelfTemplate {
                source: Some(query_source),
                ..
            } = query
                && !source_id_matches(source, query_source)
            {
                return Ok(());
            }
            let response = source.resolve_type(query, context)?;
            match response.status {
                ResolveStatus::Ok => facts.extend(response.facts),
                ResolveStatus::Ambiguous => candidates.extend(response.candidates),
                ResolveStatus::Unsupported => unsupported.extend(response.diagnostics),
                ResolveStatus::NotFound => {}
            }
            Ok(())
        })?;
        if !candidates.is_empty() {
            candidates.extend(facts.iter().map(CandidateView::candidate));
            return Ok(ResolveResponse::ambiguous(candidates));
        }
        if facts.is_empty() && !unsupported.is_empty() {
            return Ok(ResolveResponse {
                status: ResolveStatus::Unsupported,
                facts: Vec::new(),
                candidates: Vec::new(),
                diagnostics: unsupported,
            });
        }
        Ok(single_or_ambiguous(facts, "type not found"))
    }

    fn members(
        &self,
        owner: &TypeId,
        query: MemberQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedMember>, ResolveError> {
        let Some(source) = self.find_active_source(context, &owner.0.source) else {
            return Ok(ResolveResponse::not_found(format!(
                "source `{}` is not active",
                owner.0.source
            )));
        };
        source.members(owner, query, context)
    }

    fn callable(
        &self,
        query: CallableLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedCallable>, ResolveError> {
        match query {
            CallableLookup::Id(id) => {
                let Some(source) = self.find_active_source(context, &id.0.source) else {
                    return Ok(ResolveResponse::not_found(format!(
                        "source `{}` is not active",
                        id.0.source
                    )));
                };
                source.callable(query, context)
            }
            CallableLookup::OwnerName {
                owner: Some(owner), ..
            } => {
                let Some(source) = self.find_active_source(context, &owner.0.source) else {
                    return Ok(ResolveResponse::not_found(format!(
                        "source `{}` is not active",
                        owner.0.source
                    )));
                };
                source.callable(query, context)
            }
            CallableLookup::OwnerName { owner: None, .. } => {
                let mut facts = Vec::new();
                let mut candidates = Vec::new();
                let mut unsupported = Vec::new();
                self.for_each_active_source(context, |source| {
                    let response = source.callable(query, context)?;
                    match response.status {
                        ResolveStatus::Ok => facts.extend(response.facts),
                        ResolveStatus::Ambiguous => candidates.extend(response.candidates),
                        ResolveStatus::Unsupported => unsupported.extend(response.diagnostics),
                        ResolveStatus::NotFound => {}
                    }
                    Ok(())
                })?;
                if !candidates.is_empty() {
                    candidates.extend(facts.iter().map(CandidateView::candidate));
                    return Ok(ResolveResponse::ambiguous(candidates));
                }
                if facts.is_empty() && !unsupported.is_empty() {
                    return Ok(ResolveResponse {
                        status: ResolveStatus::Unsupported,
                        facts: Vec::new(),
                        candidates: Vec::new(),
                        diagnostics: unsupported,
                    });
                }
                Ok(single_or_ambiguous(facts, "callable not found"))
            }
        }
    }

    fn global_context(
        &self,
        query: GlobalContextQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedGlobalContext>, ResolveError> {
        let GlobalContextQuery::Language { language, sources } = query;
        let mut merged = ResolvedGlobalContext {
            id: FactId::new(
                SourceId::new("composite"),
                match language {
                    GlobalContextLanguage::Bsl => LanguageDomain::BslLanguage,
                    GlobalContextLanguage::Sdbl => LanguageDomain::QueryLanguage,
                },
                FactKind::Global,
                match language {
                    GlobalContextLanguage::Bsl => "global_context:bsl",
                    GlobalContextLanguage::Sdbl => "global_context:sdbl",
                },
            ),
            language,
            sources: Vec::new(),
            methods: Vec::new(),
            properties: Vec::new(),
            facts: Vec::new(),
        };
        let mut unsupported = Vec::new();
        let mut candidates = Vec::new();

        self.for_each_active_source(context, |source| {
            if !sources.is_empty() && !sources.iter().any(|id| source_id_matches(source, id)) {
                return Ok(());
            }
            let response = source.global_context(query, context)?;
            match response.status {
                ResolveStatus::Ok => {
                    for scope in response.facts {
                        for source_id in scope.sources {
                            if !merged.sources.contains(&source_id) {
                                merged.sources.push(source_id);
                            }
                        }
                        merged.methods.extend(scope.methods);
                        merged.properties.extend(scope.properties);
                        merged.facts.extend(scope.facts);
                    }
                }
                ResolveStatus::Ambiguous => candidates.extend(response.candidates),
                ResolveStatus::Unsupported => unsupported.extend(response.diagnostics),
                ResolveStatus::NotFound => {}
            }
            Ok(())
        })?;

        let has_facts = !merged.methods.is_empty()
            || !merged.properties.is_empty()
            || !merged.facts.is_empty()
            || !merged.sources.is_empty();
        if !candidates.is_empty() {
            if has_facts {
                candidates.push(merged.candidate());
            }
            return Ok(ResolveResponse::ambiguous(candidates));
        }
        if has_facts {
            return Ok(ResolveResponse::ok(vec![merged]));
        }
        if !unsupported.is_empty() {
            return Ok(ResolveResponse {
                status: ResolveStatus::Unsupported,
                facts: Vec::new(),
                candidates: Vec::new(),
                diagnostics: unsupported,
            });
        }
        Ok(ResolveResponse::not_found("global context not found"))
    }

    fn module_context(
        &self,
        query: ModuleContextQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedModuleContext>, ResolveError> {
        let mut merged = ResolvedModuleContext {
            id: FactId::new(
                SourceId::new("composite"),
                query.domain,
                FactKind::ModuleContext,
                format!("module_context:{}", query.kind.as_str()),
            ),
            language: query.language,
            domain: query.domain,
            kind: query.kind,
            sources: Vec::new(),
            self_member: None,
            properties: Vec::new(),
            methods: Vec::new(),
            events: Vec::new(),
            facts: Vec::new(),
        };
        let mut unsupported = Vec::new();
        let mut candidates = Vec::new();
        let mut has_source_owned_id = false;

        self.for_each_active_source(context, |source| {
            if !query.sources.is_empty()
                && !query.sources.iter().any(|id| source_id_matches(source, id))
            {
                return Ok(());
            }
            let response = source.module_context(query, context)?;
            match response.status {
                ResolveStatus::Ok => {
                    for scope in response.facts {
                        if !has_source_owned_id {
                            merged.id = scope.id.clone();
                            has_source_owned_id = true;
                        }
                        for source_id in scope.sources {
                            if !merged.sources.contains(&source_id) {
                                merged.sources.push(source_id);
                            }
                        }
                        if merged.self_member.is_none() {
                            merged.self_member = scope.self_member;
                        } else if let Some(self_member) = scope.self_member {
                            merged.facts.push(self_member);
                        }
                        merged.properties.extend(scope.properties);
                        merged.methods.extend(scope.methods);
                        merged.events.extend(scope.events);
                        merged.facts.extend(scope.facts);
                    }
                }
                ResolveStatus::Ambiguous => candidates.extend(response.candidates),
                ResolveStatus::Unsupported => unsupported.extend(response.diagnostics),
                ResolveStatus::NotFound => {}
            }
            Ok(())
        })?;

        let has_facts = merged.self_member.is_some()
            || !merged.properties.is_empty()
            || !merged.methods.is_empty()
            || !merged.events.is_empty()
            || !merged.facts.is_empty()
            || !merged.sources.is_empty();
        if !candidates.is_empty() {
            if has_facts {
                candidates.push(merged.candidate());
            }
            return Ok(ResolveResponse::ambiguous(candidates));
        }
        if has_facts {
            return Ok(ResolveResponse::ok(vec![merged]));
        }
        if !unsupported.is_empty() {
            return Ok(ResolveResponse {
                status: ResolveStatus::Unsupported,
                facts: Vec::new(),
                candidates: Vec::new(),
                diagnostics: unsupported,
            });
        }
        Ok(ResolveResponse::not_found("module context not found"))
    }

    fn metadata_module_context(
        &self,
        query: MetadataModuleContextLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedModuleContext>, ResolveError> {
        let Some(kind) = metadata_module_context_kind(query.metadata_module_role) else {
            return Ok(ResolveResponse::not_found("metadata module role not found"));
        };
        let mut sources = Vec::new();
        let mut unsupported = false;
        self.for_each_active_source(context, |source| {
            let descriptor = source.descriptor();
            if descriptor.domain != LanguageDomain::PlatformApi
                || query
                    .domain
                    .is_some_and(|domain| domain != descriptor.domain)
                || query
                    .source
                    .is_some_and(|id| !source_id_matches(source, id))
            {
                return Ok(());
            }
            sources.push(descriptor.id);
            unsupported |= !source.capabilities().module_context;
            Ok::<_, ResolveError>(())
        })?;
        if sources.is_empty() {
            return Ok(ResolveResponse::not_found(
                "metadata module source not found",
            ));
        }
        if unsupported {
            return Ok(ResolveResponse::unsupported(
                "selected source does not expose module context",
            ));
        }
        if matches!(
            kind,
            ModuleContextKind::Common | ModuleContextKind::Command | ModuleContextKind::RecordSet
        ) {
            return Ok(ResolveResponse::not_found(
                "metadata module context not found",
            ));
        }
        self.module_context(
            ModuleContextQuery {
                language: GlobalContextLanguage::Bsl,
                domain: LanguageDomain::PlatformApi,
                kind,
                sources: &sources,
            },
            context,
        )
    }

    fn related(
        &self,
        source_id: &FactId,
        kind: RelationKind,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        let Some(source) = self.find_active_source(context, &source_id.source) else {
            return Ok(ResolveResponse::not_found(format!(
                "source `{}` is not active",
                source_id.source
            )));
        };
        source.related(source_id, kind, context)
    }

    fn availability(
        &self,
        source_id: &FactId,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<AvailabilityFact>, ResolveError> {
        let Some(source) = self.find_active_source(context, &source_id.source) else {
            return Ok(ResolveResponse::not_found(format!(
                "source `{}` is not active",
                source_id.source
            )));
        };
        source.availability(source_id, context)
    }
}

fn single_or_ambiguous<T: CandidateView>(
    facts: Vec<T>,
    not_found: &'static str,
) -> ResolveResponse<T> {
    match facts.len() {
        0 => ResolveResponse::not_found(not_found),
        1 => ResolveResponse::ok(facts),
        _ => ResolveResponse::ambiguous(facts.iter().map(CandidateView::candidate).collect()),
    }
}

trait CandidateView {
    fn candidate(&self) -> ResolveCandidate;
}

impl CandidateView for ContextFact {
    fn candidate(&self) -> ResolveCandidate {
        ResolveCandidate {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }
}

impl CandidateView for ResolvedType {
    fn candidate(&self) -> ResolveCandidate {
        self.fact.candidate()
    }
}

impl CandidateView for ResolvedCallable {
    fn candidate(&self) -> ResolveCandidate {
        self.fact.candidate()
    }
}

impl CandidateView for ResolvedGlobalContext {
    fn candidate(&self) -> ResolveCandidate {
        ResolveCandidate {
            id: self.id.clone(),
            name: Name::new(
                match self.language {
                    GlobalContextLanguage::Bsl => "BSL global context",
                    GlobalContextLanguage::Sdbl => "SDBL global context",
                },
                None::<String>,
            ),
        }
    }
}

impl CandidateView for ResolvedModuleContext {
    fn candidate(&self) -> ResolveCandidate {
        ResolveCandidate {
            id: self.id.clone(),
            name: Name::new(
                format!("{} module context", self.kind.as_str()),
                None::<String>,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct FakeSource {
        source: SourceId,
        domain: LanguageDomain,
        types: Vec<ResolvedType>,
        members: Vec<ResolvedMember>,
        callables: Vec<ResolvedCallable>,
        relations: Vec<(FactId, RelationKind, ContextFact)>,
        global_contexts: Vec<ResolvedGlobalContext>,
        module_contexts: Vec<ResolvedModuleContext>,
        resolve_type_status: Option<ResolveStatus>,
        callable_status: Option<ResolveStatus>,
        module_context_status: Option<ResolveStatus>,
        module_context_error: Option<&'static str>,
        module_context_capability: bool,
    }

    impl FakeSource {
        fn new(source: &str, domain: LanguageDomain) -> Self {
            Self {
                source: SourceId::new(source),
                domain,
                types: Vec::new(),
                members: Vec::new(),
                callables: Vec::new(),
                relations: Vec::new(),
                global_contexts: Vec::new(),
                module_contexts: Vec::new(),
                resolve_type_status: None,
                callable_status: None,
                module_context_status: None,
                module_context_error: None,
                module_context_capability: true,
            }
        }

        fn with_type(mut self, local_id: &str, name: &str) -> Self {
            let id = TypeId(FactId::new(
                self.source.clone(),
                self.domain,
                FactKind::Type,
                local_id,
            ));
            let info = TypeInfo {
                description: None,
                metadata_template: None,
                type_template_key: None,
            };
            let fact = ContextFact {
                id: id.0.clone(),
                name: Name::new(name, None::<String>),
                owner: None,
                details: FactDetails::Type(info.clone()),
                relations: Vec::new(),
            };
            self.types.push(ResolvedType { id, fact, info });
            self
        }

        fn with_resolved_type(mut self, resolved: ResolvedType) -> Self {
            self.types.push(resolved);
            self
        }

        fn with_member(
            mut self,
            owner: &TypeId,
            local_id: &str,
            name: &str,
            target: TypeRef,
        ) -> Self {
            let id = MemberId(FactId::new(
                self.source.clone(),
                self.domain,
                FactKind::Member,
                local_id,
            ));
            let info = MemberInfo {
                kind: MemberKind::Property,
                types: vec![target],
                description: None,
            };
            let fact = ContextFact {
                id: id.0.clone(),
                name: Name::new(name, None::<String>),
                owner: Some(owner.0.clone()),
                details: FactDetails::Member(info.clone()),
                relations: Vec::new(),
            };
            self.members.push(ResolvedMember {
                id,
                owner: owner.clone(),
                fact,
                info,
            });
            self
        }

        fn with_callable(
            mut self,
            owner: Option<&TypeId>,
            local_id: &str,
            name: &str,
            parameters: Vec<Parameter>,
            return_types: Vec<TypeRef>,
        ) -> Self {
            let id = CallableId(FactId::new(
                self.source.clone(),
                self.domain,
                FactKind::Callable,
                local_id,
            ));
            let info = CallableInfo {
                kind: CallableKind::Method,
                signatures: vec![Signature {
                    parameters,
                    return_types: Vec::new(),
                    variadic: false,
                    title: None,
                    description: None,
                }],
                return_types,
                description: None,
            };
            let fact = ContextFact {
                id: id.0.clone(),
                name: Name::new(name, None::<String>),
                owner: owner.map(|owner| owner.0.clone()),
                details: FactDetails::Callable(info.clone()),
                relations: Vec::new(),
            };
            self.callables.push(ResolvedCallable {
                id,
                owner: owner.cloned(),
                fact,
                info,
            });
            self
        }

        fn with_relation(
            mut self,
            source: &FactId,
            kind: RelationKind,
            target: ContextFact,
        ) -> Self {
            self.relations.push((source.clone(), kind, target));
            self
        }

        fn with_global_context(mut self, scope: ResolvedGlobalContext) -> Self {
            self.global_contexts.push(scope);
            self
        }

        fn with_module_context(mut self, scope: ResolvedModuleContext) -> Self {
            self.module_contexts.push(scope);
            self
        }

        fn with_resolve_type_status(mut self, status: ResolveStatus) -> Self {
            self.resolve_type_status = Some(status);
            self
        }

        fn with_callable_status(mut self, status: ResolveStatus) -> Self {
            self.callable_status = Some(status);
            self
        }

        fn without_module_context_capability(mut self) -> Self {
            self.module_context_capability = false;
            self
        }

        fn with_module_context_status(mut self, status: ResolveStatus) -> Self {
            self.module_context_status = Some(status);
            self
        }

        fn with_module_context_error(mut self, message: &'static str) -> Self {
            self.module_context_error = Some(message);
            self
        }
    }

    impl ContextSource for FakeSource {
        fn descriptor(&self) -> SourceDescriptor {
            SourceDescriptor {
                id: self.source.clone(),
                domain: self.domain,
                label: self.source.to_string(),
            }
        }

        fn source_id(&self) -> Option<&SourceId> {
            Some(&self.source)
        }

        fn capabilities(&self) -> SourceCapabilities {
            SourceCapabilities {
                exact_lookup: true,
                type_lookup: true,
                members: true,
                callables: true,
                relations: true,
                global_context: true,
                module_context: self.module_context_capability,
            }
        }

        fn resolve(
            &self,
            query: ResolveQuery<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
            let facts = match query {
                ResolveQuery::Id(id) => self
                    .types
                    .iter()
                    .map(|resolved| resolved.fact.clone())
                    .chain(self.module_contexts.iter().map(|scope| ContextFact {
                        id: scope.id.clone(),
                        name: Name::new(
                            format!("{} module context", scope.kind.as_str()),
                            None::<String>,
                        ),
                        owner: None,
                        details: FactDetails::ModuleContext(ModuleContextInfo {
                            language: scope.language,
                            domain: scope.domain,
                            kind: scope.kind,
                        }),
                        relations: Vec::new(),
                    }))
                    .find(|fact| &fact.id == id)
                    .into_iter()
                    .collect(),
                ResolveQuery::ExactName {
                    domain, kind, name, ..
                } => self
                    .types
                    .iter()
                    .filter(|resolved| domain.is_none_or(|domain| domain == resolved.id.0.domain))
                    .filter(|resolved| kind.is_none_or(|kind| kind == resolved.id.0.kind))
                    .filter(|resolved| resolved.fact.name.primary == name)
                    .map(|resolved| resolved.fact.clone())
                    .collect(),
            };
            Ok(single_or_ambiguous(facts, "context fact not found"))
        }

        fn resolve_type(
            &self,
            query: TypeLookup<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ResolvedType>, ResolveError> {
            if let Some(status) = self.resolve_type_status {
                return Ok(forced_response(status, self.types.clone()));
            }
            let facts = match query {
                TypeLookup::Id(id) => self
                    .types
                    .iter()
                    .find(|resolved| &resolved.id == id)
                    .cloned()
                    .into_iter()
                    .collect(),
                TypeLookup::ExactName {
                    source,
                    domain,
                    name,
                } => self
                    .types
                    .iter()
                    .filter(|resolved| source.is_none_or(|source| source == &resolved.id.0.source))
                    .filter(|resolved| domain.is_none_or(|domain| domain == resolved.id.0.domain))
                    .filter(|resolved| resolved.fact.name.primary == name)
                    .cloned()
                    .collect(),
                TypeLookup::ExactAlias {
                    source,
                    domain,
                    alias,
                } => self
                    .types
                    .iter()
                    .filter(|resolved| source.is_none_or(|source| source == &resolved.id.0.source))
                    .filter(|resolved| domain.is_none_or(|domain| domain == resolved.id.0.domain))
                    .filter(|resolved| resolved.fact.name.alias.as_deref() == Some(alias))
                    .cloned()
                    .collect(),
                TypeLookup::PlatformTypeTemplate {
                    source,
                    domain,
                    key,
                } => self
                    .types
                    .iter()
                    .filter(|resolved| source.is_none_or(|source| source == &resolved.id.0.source))
                    .filter(|resolved| domain.is_none_or(|domain| domain == resolved.id.0.domain))
                    .filter(|resolved| resolved.info.type_template_key.as_ref() == Some(key))
                    .cloned()
                    .collect(),
                TypeLookup::GeneratedSelfTemplate { .. } => Vec::new(),
            };
            Ok(single_or_ambiguous(facts, "type not found"))
        }

        fn members(
            &self,
            owner: &TypeId,
            query: MemberQuery<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ResolvedMember>, ResolveError> {
            let facts = self
                .members
                .iter()
                .filter(|member| &member.owner == owner)
                .filter(|member| {
                    query
                        .name
                        .is_none_or(|name| member.fact.name.primary == name)
                })
                .cloned()
                .collect();
            Ok(ResolveResponse::ok(facts))
        }

        fn callable(
            &self,
            query: CallableLookup<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ResolvedCallable>, ResolveError> {
            if let Some(status) = self.callable_status {
                return Ok(forced_response(status, self.callables.clone()));
            }
            let facts = match query {
                CallableLookup::Id(id) => self
                    .callables
                    .iter()
                    .filter(|callable| &callable.id == id)
                    .cloned()
                    .collect(),
                CallableLookup::OwnerName { owner, name } => self
                    .callables
                    .iter()
                    .filter(|callable| {
                        owner.is_none_or(|owner| callable.owner.as_ref() == Some(owner))
                    })
                    .filter(|callable| callable.fact.name.primary == name)
                    .cloned()
                    .collect(),
            };
            Ok(single_or_ambiguous(facts, "callable not found"))
        }

        fn global_context(
            &self,
            query: GlobalContextQuery<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ResolvedGlobalContext>, ResolveError> {
            let GlobalContextQuery::Language { language, sources } = query;
            if !sources.is_empty() && !sources.iter().any(|source| source == &self.source) {
                return Ok(ResolveResponse::not_found(
                    "global context source not active",
                ));
            }
            let facts = self
                .global_contexts
                .iter()
                .filter(|scope| scope.language == language)
                .cloned()
                .collect();
            Ok(single_or_ambiguous(facts, "global context not found"))
        }

        fn module_context(
            &self,
            query: ModuleContextQuery<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ResolvedModuleContext>, ResolveError> {
            if !query.sources.is_empty()
                && !query.sources.iter().any(|source| source == &self.source)
            {
                return Ok(ResolveResponse::not_found(
                    "module context source not active",
                ));
            }
            if let Some(message) = self.module_context_error {
                return Err(ResolveError::SourceFailure {
                    source_id: self.source.clone(),
                    message: message.to_string(),
                });
            }
            let facts = self
                .module_contexts
                .iter()
                .filter(|scope| scope.language == query.language)
                .filter(|scope| scope.domain == query.domain)
                .filter(|scope| scope.kind == query.kind)
                .cloned()
                .collect();
            if let Some(status) = self.module_context_status {
                return Ok(forced_response(status, facts));
            }
            Ok(single_or_ambiguous(facts, "module context not found"))
        }

        fn related(
            &self,
            source: &FactId,
            kind: RelationKind,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
            let facts = self
                .relations
                .iter()
                .filter(|(from, relation_kind, _)| from == source && *relation_kind == kind)
                .map(|(_, _, target)| target.clone())
                .collect();
            Ok(ResolveResponse::ok(facts))
        }
    }

    struct DefaultMetadataResolver;

    impl ContextResolver for DefaultMetadataResolver {
        fn resolve(
            &self,
            _query: ResolveQuery<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
            Ok(ResolveResponse::not_found("test resolver has no facts"))
        }

        fn resolve_type(
            &self,
            _query: TypeLookup<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ResolvedType>, ResolveError> {
            Ok(ResolveResponse::not_found("test resolver has no types"))
        }

        fn members(
            &self,
            _owner: &TypeId,
            _query: MemberQuery<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ResolvedMember>, ResolveError> {
            Ok(ResolveResponse::not_found("test resolver has no members"))
        }

        fn callable(
            &self,
            _query: CallableLookup<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ResolvedCallable>, ResolveError> {
            Ok(ResolveResponse::not_found("test resolver has no callables"))
        }

        fn global_context(
            &self,
            _query: GlobalContextQuery<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ResolvedGlobalContext>, ResolveError> {
            Ok(ResolveResponse::not_found("test resolver has no globals"))
        }

        fn module_context(
            &self,
            _query: ModuleContextQuery<'_>,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ResolvedModuleContext>, ResolveError> {
            Ok(ResolveResponse::not_found(
                "test resolver has no module context",
            ))
        }

        fn related(
            &self,
            _source: &FactId,
            _kind: RelationKind,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
            Ok(ResolveResponse::not_found("test resolver has no relations"))
        }

        fn availability(
            &self,
            _source: &FactId,
            _context: &ResolveContext<'_>,
        ) -> Result<ResolveResponse<AvailabilityFact>, ResolveError> {
            Ok(ResolveResponse::not_found(
                "test resolver has no availability facts",
            ))
        }
    }

    #[test]
    fn same_name_types_across_sources_are_ambiguous_without_constraints() {
        let resolver = CompositeResolver::new(vec![
            Box::new(
                FakeSource::new("platform", LanguageDomain::PlatformApi)
                    .with_type("platform_type:Строка", "Строка"),
            ),
            Box::new(
                FakeSource::new("configuration", LanguageDomain::Configuration)
                    .with_type("catalog:Строка", "Строка"),
            ),
        ]);

        let response = resolver
            .resolve_type(
                TypeLookup::ExactName {
                    source: None,
                    domain: None,
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ambiguous);
        assert_eq!(response.candidates.len(), 2);
    }

    #[test]
    fn bsl_and_query_string_are_separate_type_ids() {
        let bsl = SourceId::new("shlang");
        let query = SourceId::new("shquery");
        let bsl_string = TypeId(FactId::new(
            bsl,
            LanguageDomain::BslLanguage,
            FactKind::Type,
            "def_String",
        ));
        let query_string = TypeId(FactId::new(
            query,
            LanguageDomain::QueryLanguage,
            FactKind::Type,
            "STRING",
        ));

        assert_ne!(bsl_string, query_string);
        assert_eq!(bsl_string.0.domain, LanguageDomain::BslLanguage);
        assert_eq!(query_string.0.domain, LanguageDomain::QueryLanguage);
    }

    #[test]
    fn composite_merges_language_specific_global_context_scopes() {
        let bsl_source = SourceId::new("shlang");
        let platform_source = SourceId::new("platform");
        let bsl_scope = global_context_scope(
            bsl_source.clone(),
            LanguageDomain::BslLanguage,
            GlobalContextLanguage::Bsl,
            vec![ContextFact {
                id: FactId::new(
                    bsl_source,
                    LanguageDomain::BslLanguage,
                    FactKind::Type,
                    "def_String",
                ),
                name: Name::new("Строка", None::<String>),
                owner: None,
                details: FactDetails::Type(TypeInfo {
                    description: None,
                    metadata_template: None,
                    type_template_key: None,
                }),
                relations: Vec::new(),
            }],
        );
        let platform_scope = ResolvedGlobalContext {
            id: FactId::new(
                platform_source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Global,
                "global_context:bsl",
            ),
            language: GlobalContextLanguage::Bsl,
            sources: vec![platform_source.clone()],
            methods: vec![ResolvedCallable {
                id: CallableId(FactId::new(
                    platform_source,
                    LanguageDomain::PlatformApi,
                    FactKind::Callable,
                    "global_method:Сообщить",
                )),
                owner: None,
                fact: ContextFact {
                    id: FactId::new(
                        SourceId::new("platform"),
                        LanguageDomain::PlatformApi,
                        FactKind::Callable,
                        "global_method:Сообщить",
                    ),
                    name: Name::new("Сообщить", None::<String>),
                    owner: None,
                    details: FactDetails::Callable(CallableInfo {
                        kind: CallableKind::GlobalMethod,
                        signatures: Vec::new(),
                        return_types: Vec::new(),
                        description: None,
                    }),
                    relations: Vec::new(),
                },
                info: CallableInfo {
                    kind: CallableKind::GlobalMethod,
                    signatures: Vec::new(),
                    return_types: Vec::new(),
                    description: None,
                },
            }],
            properties: Vec::new(),
            facts: Vec::new(),
        };
        let resolver = CompositeResolver::new(vec![
            Box::new(
                FakeSource::new("shlang", LanguageDomain::BslLanguage)
                    .with_global_context(bsl_scope),
            ),
            Box::new(
                FakeSource::new("platform", LanguageDomain::PlatformApi)
                    .with_global_context(platform_scope),
            ),
        ]);

        let response = resolver
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Bsl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("global context lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        let scope = &response.facts[0];
        assert_eq!(scope.language, GlobalContextLanguage::Bsl);
        assert_eq!(scope.sources.len(), 2);
        assert_eq!(scope.methods[0].fact.name.primary, "Сообщить");
        assert_eq!(scope.facts[0].name.primary, "Строка");
    }

    #[test]
    fn composite_merges_module_context_scopes_without_fabricating_self_member() {
        let platform_source = SourceId::new("platform");
        let event_id = CallableId(FactId::new(
            platform_source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Callable,
            "module_event:form:ПриОткрытии",
        ));
        let event_info = CallableInfo {
            kind: CallableKind::Event,
            signatures: Vec::new(),
            return_types: Vec::new(),
            description: None,
        };
        let scope = ResolvedModuleContext {
            id: FactId::new(
                platform_source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::ModuleContext,
                "module_context:form",
            ),
            language: GlobalContextLanguage::Bsl,
            domain: LanguageDomain::PlatformApi,
            kind: ModuleContextKind::Form,
            sources: vec![platform_source.clone()],
            self_member: None,
            properties: Vec::new(),
            methods: Vec::new(),
            events: vec![ResolvedCallable {
                id: event_id.clone(),
                owner: None,
                fact: ContextFact {
                    id: event_id.0,
                    name: Name::new("ПриОткрытии", Some("OnOpen")),
                    owner: None,
                    details: FactDetails::Callable(event_info.clone()),
                    relations: Vec::new(),
                },
                info: event_info,
            }],
            facts: Vec::new(),
        };
        let resolver = CompositeResolver::new(vec![Box::new(
            FakeSource::new("platform", LanguageDomain::PlatformApi).with_module_context(scope),
        )]);

        let response = resolver
            .module_context(
                ModuleContextQuery {
                    language: GlobalContextLanguage::Bsl,
                    domain: LanguageDomain::PlatformApi,
                    kind: ModuleContextKind::Form,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("module context lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        let scope = response.facts.first().expect("module context must resolve");
        assert_eq!(&scope.id.source, &platform_source);
        assert_eq!(scope.sources, vec![platform_source.clone()]);
        assert_eq!(scope.self_member, None);
        assert_eq!(scope.events[0].fact.name.alias.as_deref(), Some("OnOpen"));

        let resolved = resolver
            .resolve(ResolveQuery::Id(&scope.id), &ResolveContext::all())
            .expect("module context id lookup must not fail");
        assert_eq!(resolved.status, ResolveStatus::Ok);
        assert!(matches!(
            resolved.facts[0].details,
            FactDetails::ModuleContext(_)
        ));
    }

    #[test]
    fn metadata_module_role_delegates_to_existing_module_context() {
        let source = SourceId::new("platform");
        let scope = module_context_scope(source.clone(), ModuleContextKind::Object);
        let resolver = CompositeResolver::new(vec![Box::new(
            FakeSource::new("platform", LanguageDomain::PlatformApi).with_module_context(scope),
        )]);
        let response = resolver
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect("metadata module lookup must not fail");
        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(response.facts[0].kind, ModuleContextKind::Object);

        for metadata_module_role in [
            "metadata.module-role.common",
            "metadata.module-role.command",
            "metadata.module-role.record-set",
            "metadata.module-role.unknown",
        ] {
            let response = resolver
                .metadata_module_context(
                    MetadataModuleContextLookup {
                        source: Some(&source),
                        domain: Some(LanguageDomain::PlatformApi),
                        metadata_module_role,
                    },
                    &ResolveContext::all(),
                )
                .expect("metadata module lookup must not fail");
            assert_eq!(response.status, ResolveStatus::NotFound);
        }
    }

    #[test]
    fn metadata_module_role_delegates_the_supported_corpus() {
        let source = SourceId::new("platform");
        let resolver = CompositeResolver::new(vec![Box::new(
            FakeSource::new("platform", LanguageDomain::PlatformApi)
                .with_module_context(module_context_scope(
                    source.clone(),
                    ModuleContextKind::Object,
                ))
                .with_module_context(module_context_scope(
                    source.clone(),
                    ModuleContextKind::Manager,
                ))
                .with_module_context(module_context_scope(
                    source.clone(),
                    ModuleContextKind::Form,
                )),
        )]);

        for (metadata_module_role, expected_kind) in [
            ("metadata.module-role.object", ModuleContextKind::Object),
            ("metadata.module-role.manager", ModuleContextKind::Manager),
            ("metadata.module-role.form", ModuleContextKind::Form),
        ] {
            let response = resolver
                .metadata_module_context(
                    MetadataModuleContextLookup {
                        source: Some(&source),
                        domain: Some(LanguageDomain::PlatformApi),
                        metadata_module_role,
                    },
                    &ResolveContext::all(),
                )
                .expect("supported metadata module role must not fail");
            assert_eq!(response.status, ResolveStatus::Ok);
            assert_eq!(response.facts[0].kind, expected_kind);
        }
    }

    #[test]
    fn metadata_module_role_restricts_sources_and_requires_capability() {
        let platform = SourceId::new("platform");
        let missing = SourceId::new("missing");
        let capable = SourceId::new("capable");
        let unavailable = SourceId::new("unavailable");
        let resolver = CompositeResolver::new(vec![
            Box::new(
                FakeSource::new("platform", LanguageDomain::PlatformApi).with_module_context(
                    module_context_scope(platform.clone(), ModuleContextKind::Object),
                ),
            ),
            Box::new(
                FakeSource::new("bsl", LanguageDomain::BslLanguage)
                    .with_module_context_error("non-platform source must not be selected"),
            ),
        ]);

        let source_absent = resolver
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: Some(&missing),
                    domain: Some(LanguageDomain::PlatformApi),
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect("absent source must not fail");
        assert_eq!(source_absent.status, ResolveStatus::NotFound);

        let domain_mismatch = resolver
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: Some(&platform),
                    domain: Some(LanguageDomain::BslLanguage),
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect("domain mismatch must not fail");
        assert_eq!(domain_mismatch.status, ResolveStatus::NotFound);

        let no_source_filter = resolver
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: None,
                    domain: None,
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect("non-platform source must stay outside dispatch");
        assert_eq!(no_source_filter.status, ResolveStatus::Ok);

        let capability_resolver = CompositeResolver::new(vec![
            Box::new(
                FakeSource::new("capable", LanguageDomain::PlatformApi).with_module_context(
                    module_context_scope(capable.clone(), ModuleContextKind::Object),
                ),
            ),
            Box::new(
                FakeSource::new("unavailable", LanguageDomain::PlatformApi)
                    .without_module_context_capability(),
            ),
        ]);
        let any_unavailable = capability_resolver
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: None,
                    domain: Some(LanguageDomain::PlatformApi),
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect("capability check must not fail");
        assert_eq!(any_unavailable.status, ResolveStatus::Unsupported);

        let source_isolated = capability_resolver
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: Some(&capable),
                    domain: Some(LanguageDomain::PlatformApi),
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect("selected capable source must not fail");
        assert_eq!(source_isolated.status, ResolveStatus::Ok);
        assert_eq!(source_isolated.facts[0].id.source, capable);
        let unavailable_source = capability_resolver
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: Some(&unavailable),
                    domain: Some(LanguageDomain::PlatformApi),
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect("unsupported source must not fail");
        assert_eq!(unavailable_source.status, ResolveStatus::Unsupported);
    }

    #[test]
    fn metadata_module_role_preserves_ambiguity_and_resolver_failure() {
        let source = SourceId::new("platform");
        let ambiguous = CompositeResolver::new(vec![Box::new(
            FakeSource::new("platform", LanguageDomain::PlatformApi)
                .with_module_context(module_context_scope(
                    source.clone(),
                    ModuleContextKind::Object,
                ))
                .with_module_context(module_context_scope(
                    source.clone(),
                    ModuleContextKind::Object,
                )),
        )]);
        let ambiguous_response = ambiguous
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect("ambiguous source answer must not become an error");
        assert_eq!(ambiguous_response.status, ResolveStatus::Ambiguous);

        let failed = CompositeResolver::new(vec![Box::new(
            FakeSource::new("platform", LanguageDomain::PlatformApi)
                .with_module_context_error("provider failed"),
        )]);
        let error = failed
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect_err("provider failure must not be converted to absence");
        assert!(matches!(
            error,
            ResolveError::SourceFailure { source_id, message }
                if source_id == source && message == "provider failed"
        ));

        let unsupported = CompositeResolver::new(vec![Box::new(
            FakeSource::new("platform", LanguageDomain::PlatformApi)
                .with_module_context_status(ResolveStatus::Unsupported),
        )]);
        let unsupported_response = unsupported
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect("provider unsupported outcome must not fail");
        assert_eq!(unsupported_response.status, ResolveStatus::Unsupported);
    }

    #[test]
    fn metadata_module_role_default_is_unsupported() {
        let resolver = DefaultMetadataResolver;
        let response = resolver
            .metadata_module_context(
                MetadataModuleContextLookup {
                    source: None,
                    domain: None,
                    metadata_module_role: "metadata.module-role.object",
                },
                &ResolveContext::all(),
            )
            .expect("default resolver response must not fail");
        assert_eq!(response.status, ResolveStatus::Unsupported);
    }

    #[test]
    fn composite_global_context_preserves_source_level_ambiguity() {
        let bsl_source = SourceId::new("bsl-ok");
        let ambiguous_source = SourceId::new("bsl-ambiguous");
        let ok_scope = global_context_scope(
            bsl_source.clone(),
            LanguageDomain::BslLanguage,
            GlobalContextLanguage::Bsl,
            vec![language_fact(&bsl_source, "def_String", "Строка")],
        );
        let left_scope = global_context_scope(
            ambiguous_source.clone(),
            LanguageDomain::BslLanguage,
            GlobalContextLanguage::Bsl,
            vec![language_fact(&ambiguous_source, "left", "Левый")],
        );
        let right_scope = global_context_scope(
            ambiguous_source.clone(),
            LanguageDomain::BslLanguage,
            GlobalContextLanguage::Bsl,
            vec![language_fact(&ambiguous_source, "right", "Правый")],
        );
        let resolver = CompositeResolver::new(vec![
            Box::new(
                FakeSource::new("bsl-ok", LanguageDomain::BslLanguage)
                    .with_global_context(ok_scope),
            ),
            Box::new(
                FakeSource::new("bsl-ambiguous", LanguageDomain::BslLanguage)
                    .with_global_context(left_scope)
                    .with_global_context(right_scope),
            ),
        ]);

        let response = resolver
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Bsl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("global context lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ambiguous);
        assert_eq!(response.facts.len(), 0);
        let candidate_sources = response
            .candidates
            .iter()
            .map(|candidate| candidate.id.source.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(candidate_sources.contains("bsl-ambiguous"));
        assert!(candidate_sources.contains("composite"));
    }

    #[test]
    fn composite_preserves_source_level_ambiguous_type_response() {
        let resolver = CompositeResolver::new(vec![Box::new(
            FakeSource::new("ambiguous", LanguageDomain::PlatformApi)
                .with_type("left", "Строка")
                .with_type("right", "Строка")
                .with_resolve_type_status(ResolveStatus::Ambiguous),
        )]);

        let response = resolver
            .resolve_type(
                TypeLookup::ExactName {
                    source: None,
                    domain: None,
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ambiguous);
        assert_eq!(response.candidates.len(), 2);
    }

    #[test]
    fn composite_ambiguity_keeps_ok_candidates_from_other_sources() {
        let resolver = CompositeResolver::new(vec![
            Box::new(
                FakeSource::new("bsl", LanguageDomain::BslLanguage)
                    .with_type("def_String", "Строка"),
            ),
            Box::new(
                FakeSource::new("query", LanguageDomain::QueryLanguage)
                    .with_type("STRING", "Строка")
                    .with_type("LitString", "Строка")
                    .with_resolve_type_status(ResolveStatus::Ambiguous),
            ),
        ]);

        let response = resolver
            .resolve_type(
                TypeLookup::ExactName {
                    source: None,
                    domain: None,
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ambiguous);
        let candidate_sources = response
            .candidates
            .iter()
            .map(|candidate| candidate.id.source.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(candidate_sources.contains("bsl"));
        assert!(candidate_sources.contains("query"));
    }

    #[test]
    fn composite_preserves_source_level_unsupported_callable_response() {
        let resolver = CompositeResolver::new(vec![Box::new(
            FakeSource::new("unsupported", LanguageDomain::PlatformApi)
                .with_callable_status(ResolveStatus::Unsupported),
        )]);

        let response = resolver
            .callable(
                CallableLookup::OwnerName {
                    owner: None,
                    name: "Найти",
                },
                &ResolveContext::all(),
            )
            .expect("lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Unsupported);
    }

    #[test]
    fn member_listing_uses_resolved_owner_id_not_display_name() {
        let source = SourceId::new("fake");
        let platform_owner = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform:Owner",
        ));
        let configuration_owner = TypeId(FactId::new(
            source,
            LanguageDomain::Configuration,
            FactKind::Type,
            "configuration:Owner",
        ));
        let target = TypeRef {
            name: "Строка".to_string(),
            target: TypeRefTarget::Unresolved,
            template_binding: None,
        };
        let fake = FakeSource::new("fake", LanguageDomain::PlatformApi)
            .with_member(
                &platform_owner,
                "platform:Owner.Field",
                "Поле",
                target.clone(),
            )
            .with_member(
                &configuration_owner,
                "configuration:Owner.Field",
                "Поле",
                target,
            );
        let resolver = CompositeResolver::new(vec![Box::new(fake)]);

        let response = resolver
            .members(
                &platform_owner,
                MemberQuery {
                    name: Some("Поле"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("member lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(response.facts.len(), 1);
        assert_eq!(response.facts[0].owner, platform_owner);
    }

    #[test]
    fn type_template_lookup_uses_open_family_variant_key() {
        let source = SourceId::new("platform");
        let kind = PlatformTypeTemplateKey::new("Document", "Ref");
        let id = TypeId(FactId::new(
            source,
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ДокументСсылка.<Имя документа>",
        ));
        let info = TypeInfo {
            description: None,
            metadata_template: Some(MetadataTemplateInfo {
                metadata_kind: "ДокументСсылка".to_string(),
                parameters: vec!["Имя документа".to_string()],
            }),
            type_template_key: Some(kind.clone()),
        };
        let fact = ContextFact {
            id: id.0.clone(),
            name: Name::new(
                "ДокументСсылка.<Имя документа>",
                Some("DocumentRef.<Document name>"),
            ),
            owner: None,
            details: FactDetails::Type(info.clone()),
            relations: Vec::new(),
        };
        let fake = FakeSource::new("platform", LanguageDomain::PlatformApi)
            .with_resolved_type(ResolvedType { id, fact, info });
        let resolver = CompositeResolver::new(vec![Box::new(fake)]);

        let response = resolver
            .resolve_type(
                TypeLookup::PlatformTypeTemplate {
                    source: None,
                    domain: Some(LanguageDomain::PlatformApi),
                    key: &kind,
                },
                &ResolveContext::all(),
            )
            .expect("type template lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(
            response.facts[0].info.type_template_key.as_ref(),
            Some(&kind)
        );
    }

    #[test]
    fn callable_lookup_preserves_identity_parameter_order_and_return_types() {
        let owner = TypeId(FactId::new(
            SourceId::new("platform"),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:HTTPСоединение",
        ));
        let bool_type = TypeId(FactId::new(
            SourceId::new("platform"),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:Булево",
        ));
        let parameters = vec![
            Parameter {
                name: "Сервер".to_string(),
                required: true,
                types: vec![TypeRef {
                    name: "Строка".to_string(),
                    target: TypeRefTarget::Unresolved,
                    template_binding: None,
                }],
                description: None,
            },
            Parameter {
                name: "ИспользоватьАутентификациюОС".to_string(),
                required: false,
                types: vec![TypeRef {
                    name: "Булево".to_string(),
                    target: TypeRefTarget::Ok(bool_type),
                    template_binding: None,
                }],
                description: None,
            },
        ];
        let fake = FakeSource::new("platform", LanguageDomain::PlatformApi).with_callable(
            Some(&owner),
            "constructor:platform_type:HTTPСоединение:default",
            "Новый HTTPСоединение",
            parameters,
            vec![TypeRef {
                name: "HTTPСоединение".to_string(),
                target: TypeRefTarget::Ok(owner.clone()),
                template_binding: None,
            }],
        );
        let resolver = CompositeResolver::new(vec![Box::new(fake)]);

        let response = resolver
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&owner),
                    name: "Новый HTTPСоединение",
                },
                &ResolveContext::all(),
            )
            .expect("callable lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        let callable = &response.facts[0];
        assert_eq!(
            callable.id.0.local_id,
            "constructor:platform_type:HTTPСоединение:default"
        );
        let parameter_names = callable.info.signatures[0]
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(parameter_names, ["Сервер", "ИспользоватьАутентификациюОС"]);
        assert_eq!(callable.info.return_types[0].resolved_id(), Some(&owner));
    }

    #[test]
    fn fake_query_field_can_reference_explicit_bsl_query_or_platform_type() {
        let query_field = FactId::new(
            SourceId::new("query-fixture"),
            LanguageDomain::QueryLanguage,
            FactKind::QueryField,
            "query_field:Ссылка",
        );
        let platform_type = TypeId(FactId::new(
            SourceId::new("platform"),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:СправочникСсылка",
        ));
        let platform_fact = ContextFact {
            id: platform_type.0.clone(),
            name: Name::new("СправочникСсылка", None::<String>),
            owner: None,
            details: FactDetails::Type(TypeInfo {
                description: None,
                metadata_template: None,
                type_template_key: None,
            }),
            relations: Vec::new(),
        };
        let fake = FakeSource::new("query-fixture", LanguageDomain::QueryLanguage).with_relation(
            &query_field,
            RelationKind::HasType,
            platform_fact,
        );
        let resolver = CompositeResolver::new(vec![Box::new(fake)]);

        let response = resolver
            .related(&query_field, RelationKind::HasType, &ResolveContext::all())
            .expect("relation traversal must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(response.facts[0].id, platform_type.0);
    }

    fn forced_response<T: CandidateView + Clone>(
        status: ResolveStatus,
        facts: Vec<T>,
    ) -> ResolveResponse<T> {
        match status {
            ResolveStatus::Ok => ResolveResponse::ok(facts),
            ResolveStatus::Ambiguous => {
                ResolveResponse::ambiguous(facts.iter().map(CandidateView::candidate).collect())
            }
            ResolveStatus::Unsupported => ResolveResponse::unsupported("forced unsupported"),
            ResolveStatus::NotFound => ResolveResponse::not_found("forced not found"),
        }
    }

    fn global_context_scope(
        source: SourceId,
        domain: LanguageDomain,
        language: GlobalContextLanguage,
        facts: Vec<ContextFact>,
    ) -> ResolvedGlobalContext {
        ResolvedGlobalContext {
            id: FactId::new(
                source.clone(),
                domain,
                FactKind::Global,
                "global_context:bsl",
            ),
            language,
            sources: vec![source],
            methods: Vec::new(),
            properties: Vec::new(),
            facts,
        }
    }

    fn module_context_scope(source: SourceId, kind: ModuleContextKind) -> ResolvedModuleContext {
        ResolvedModuleContext {
            id: FactId::new(
                source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::ModuleContext,
                format!("module_context:{}", kind.as_str()),
            ),
            language: GlobalContextLanguage::Bsl,
            domain: LanguageDomain::PlatformApi,
            kind,
            sources: vec![source],
            self_member: None,
            properties: Vec::new(),
            methods: Vec::new(),
            events: Vec::new(),
            facts: Vec::new(),
        }
    }

    fn language_fact(source: &SourceId, local_id: &str, name: &str) -> ContextFact {
        ContextFact {
            id: FactId::new(
                source.clone(),
                LanguageDomain::BslLanguage,
                FactKind::Type,
                local_id,
            ),
            name: Name::new(name, None::<String>),
            owner: None,
            details: FactDetails::Type(TypeInfo {
                description: None,
                metadata_template: None,
                type_template_key: None,
            }),
            relations: Vec::new(),
        }
    }
}
