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
    Enum,
    EnumValue,
    Language,
    QueryTable,
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

    fn active_sources<'a>(&'a self, context: &ResolveContext<'_>) -> Vec<&'a dyn ContextSource> {
        self.sources
            .iter()
            .map(Box::as_ref)
            .filter(|source| context.is_source_active(&source.descriptor().id))
            .collect()
    }
}

impl ContextResolver for CompositeResolver {
    fn resolve(
        &self,
        query: ResolveQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        if let ResolveQuery::Id(id) = query {
            let Some(source) = self
                .active_sources(context)
                .into_iter()
                .find(|source| source.descriptor().id == id.source)
            else {
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
        for source in self.active_sources(context) {
            if let ResolveQuery::ExactName {
                source: Some(query_source),
                ..
            } = query
                && source.descriptor().id != *query_source
            {
                continue;
            }
            let response = source.resolve(query, context)?;
            match response.status {
                ResolveStatus::Ok => facts.extend(response.facts),
                ResolveStatus::Ambiguous => candidates.extend(response.candidates),
                ResolveStatus::Unsupported => unsupported.extend(response.diagnostics),
                ResolveStatus::NotFound => {}
            }
        }
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
            let Some(source) = self
                .active_sources(context)
                .into_iter()
                .find(|source| source.descriptor().id == id.0.source)
            else {
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
        for source in self.active_sources(context) {
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
            } = query
                && source.descriptor().id != *query_source
            {
                continue;
            }
            let response = source.resolve_type(query, context)?;
            match response.status {
                ResolveStatus::Ok => facts.extend(response.facts),
                ResolveStatus::Ambiguous => candidates.extend(response.candidates),
                ResolveStatus::Unsupported => unsupported.extend(response.diagnostics),
                ResolveStatus::NotFound => {}
            }
        }
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
        let Some(source) = self
            .active_sources(context)
            .into_iter()
            .find(|source| source.descriptor().id == owner.0.source)
        else {
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
                let Some(source) = self
                    .active_sources(context)
                    .into_iter()
                    .find(|source| source.descriptor().id == id.0.source)
                else {
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
                let Some(source) = self
                    .active_sources(context)
                    .into_iter()
                    .find(|source| source.descriptor().id == owner.0.source)
                else {
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
                for source in self.active_sources(context) {
                    let response = source.callable(query, context)?;
                    match response.status {
                        ResolveStatus::Ok => facts.extend(response.facts),
                        ResolveStatus::Ambiguous => candidates.extend(response.candidates),
                        ResolveStatus::Unsupported => unsupported.extend(response.diagnostics),
                        ResolveStatus::NotFound => {}
                    }
                }
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

        for source in self.active_sources(context) {
            let source_id = source.descriptor().id;
            if !sources.is_empty() && !sources.iter().any(|id| id == &source_id) {
                continue;
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
                ResolveStatus::Unsupported => unsupported.extend(response.diagnostics),
                ResolveStatus::NotFound | ResolveStatus::Ambiguous => {}
            }
        }

        let has_facts = !merged.methods.is_empty()
            || !merged.properties.is_empty()
            || !merged.facts.is_empty()
            || !merged.sources.is_empty();
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

    fn related(
        &self,
        source_id: &FactId,
        kind: RelationKind,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        let Some(source) = self
            .active_sources(context)
            .into_iter()
            .find(|source| source.descriptor().id == source_id.source)
        else {
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
        let Some(source) = self
            .active_sources(context)
            .into_iter()
            .find(|source| source.descriptor().id == source_id.source)
        else {
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
        resolve_type_status: Option<ResolveStatus>,
        callable_status: Option<ResolveStatus>,
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
                resolve_type_status: None,
                callable_status: None,
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

        fn with_resolve_type_status(mut self, status: ResolveStatus) -> Self {
            self.resolve_type_status = Some(status);
            self
        }

        fn with_callable_status(mut self, status: ResolveStatus) -> Self {
            self.callable_status = Some(status);
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

        fn capabilities(&self) -> SourceCapabilities {
            SourceCapabilities {
                exact_lookup: true,
                type_lookup: true,
                members: true,
                callables: true,
                relations: true,
                global_context: true,
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
        let bsl_scope = ResolvedGlobalContext {
            id: FactId::new(
                bsl_source.clone(),
                LanguageDomain::BslLanguage,
                FactKind::Global,
                "global_context:bsl",
            ),
            language: GlobalContextLanguage::Bsl,
            sources: vec![bsl_source.clone()],
            methods: Vec::new(),
            properties: Vec::new(),
            facts: vec![ContextFact {
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
        };
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
}
