use std::path::Path;

use context_resolver_core::{
    AvailabilityContext, AvailabilityFact, AvailabilityInfo, CallableId, CallableInfo,
    CallableKind, CallableLookup, ContextFact, ContextSource, FactDetails, FactId, FactKind,
    FactRelation, LanguageDomain, MemberId, MemberInfo, MemberKind, MemberQuery, MemberQueryKind,
    MetadataTemplateInfo, Name, Parameter, PlatformTypeTemplateKey, RelationKind, ResolveContext,
    ResolveError, ResolveResponse, ResolvedCallable, ResolvedMember, ResolvedType, Signature,
    SourceCapabilities, SourceDescriptor, SourceId, TemplateParameterBinding, TypeId, TypeInfo,
    TypeLookup, TypeRef, TypeRefTarget, TypeTemplateBinding,
};
use syntax_helper_search::{
    RelatedHit, SearchDocument, SearchDocumentKind, SearchError, SearchHit, SearchIndex,
    SearchTypeRef, SearchTypeRefTarget,
};

const DEFAULT_SOURCE_ID: &str = "shcntx-platform";

pub struct PlatformSearchSource {
    source_id: SourceId,
    index: SearchIndex,
}

pub struct LanguageSearchSource {
    source_id: SourceId,
    domain: LanguageDomain,
    index: SearchIndex,
}

fn search_source_failure(source_id: &SourceId, source: SearchError) -> ResolveError {
    ResolveError::SourceFailure {
        source_id: source_id.clone(),
        message: source.to_string(),
    }
}

impl PlatformSearchSource {
    pub fn new(index: SearchIndex) -> Self {
        Self {
            source_id: SourceId::new(DEFAULT_SOURCE_ID),
            index,
        }
    }

    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self, ResolveError> {
        Self::open_read_only_with_source_id(path, SourceId::new(DEFAULT_SOURCE_ID))
    }

    pub fn with_source_id(index: SearchIndex, source_id: SourceId) -> Self {
        Self { source_id, index }
    }

    pub fn open_read_only_with_source_id(
        path: impl AsRef<Path>,
        source_id: SourceId,
    ) -> Result<Self, ResolveError> {
        SearchIndex::open_read_only(path)
            .map(|index| Self::with_source_id(index, source_id.clone()))
            .map_err(|source| search_source_failure(&source_id, source))
    }

    fn source_failure(&self, source: SearchError) -> ResolveError {
        search_source_failure(&self.source_id, source)
    }

    fn fact_id(&self, kind: FactKind, local_id: impl Into<String>) -> FactId {
        FactId::new(
            self.source_id.clone(),
            LanguageDomain::PlatformApi,
            kind,
            local_id,
        )
    }

    fn type_id(&self, local_id: impl Into<String>) -> TypeId {
        TypeId(self.fact_id(FactKind::Type, local_id))
    }

    fn owner_id_for_document(
        &self,
        document: &SearchDocument,
    ) -> Result<Option<TypeId>, ResolveError> {
        self.index
            .owner_type_id_for_document(&document.id)
            .map(|owner| owner.map(|owner| self.type_id(owner)))
            .map_err(|source| self.source_failure(source))
    }

    fn type_ref_fact(&self, type_ref: &SearchTypeRef) -> TypeRef {
        TypeRef {
            name: type_ref.name.clone(),
            target: self.type_ref_target(&type_ref.target),
            template_binding: type_ref.template_binding.as_ref().map(map_template_binding),
        }
    }

    fn type_ref_target(&self, target: &SearchTypeRefTarget) -> TypeRefTarget {
        match target {
            SearchTypeRefTarget::Ok(type_id) => TypeRefTarget::Ok(self.type_id(type_id.clone())),
            SearchTypeRefTarget::Unresolved => TypeRefTarget::Unresolved,
            SearchTypeRefTarget::Ambiguous(candidates) => TypeRefTarget::Ambiguous(
                candidates
                    .iter()
                    .map(|type_id| self.type_id(type_id.clone()))
                    .collect(),
            ),
        }
    }

    fn type_ref_facts(&self, type_refs: &[SearchTypeRef]) -> Vec<TypeRef> {
        type_refs
            .iter()
            .map(|type_ref| self.type_ref_fact(type_ref))
            .collect()
    }

    fn edge_refs(&self, document_id: &str, edge: &str) -> Result<Vec<TypeRef>, ResolveError> {
        let hits = self
            .index
            .related_by_id_and_edge(document_id, edge, 20)
            .map_err(|source| self.source_failure(source))?;
        hits.into_iter()
            .map(|hit| {
                Ok(TypeRef {
                    name: hit.document.name.primary,
                    target: TypeRefTarget::Ok(self.type_id(hit.document.id)),
                    template_binding: None,
                })
            })
            .collect()
    }

    fn map_type(&self, hit: SearchHit) -> ResolvedType {
        let info = type_info(&hit.document);
        let id = self.type_id(hit.document.id.clone());
        let fact = ContextFact {
            id: id.0.clone(),
            name: map_name(&hit.document),
            owner: None,
            details: FactDetails::Type(info.clone()),
            relations: Vec::new(),
        };
        ResolvedType { id, fact, info }
    }

    fn map_context_fact(&self, hit: SearchHit) -> Result<Option<ContextFact>, ResolveError> {
        match hit.document.kind {
            SearchDocumentKind::PlatformType => Ok(Some(self.map_type(hit).fact)),
            SearchDocumentKind::TypeProperty
            | SearchDocumentKind::GlobalProperty
            | SearchDocumentKind::EnumValue => Ok(self.map_member(hit)?.map(|member| member.fact)),
            SearchDocumentKind::TypeMethod
            | SearchDocumentKind::GlobalMethod
            | SearchDocumentKind::Constructor
            | SearchDocumentKind::TypeEvent
            | SearchDocumentKind::ModuleEvent => {
                Ok(self.map_callable(hit)?.map(|callable| callable.fact))
            }
            SearchDocumentKind::Enum => Ok(Some(ContextFact {
                id: self.fact_id(FactKind::Enum, hit.document.id.clone()),
                name: map_name(&hit.document),
                owner: None,
                details: FactDetails::Enum,
                relations: Vec::new(),
            })),
            SearchDocumentKind::UnknownEvent
            | SearchDocumentKind::QueryTable
            | SearchDocumentKind::QueryTableField
            | SearchDocumentKind::QueryTableParameter
            | SearchDocumentKind::LanguageType
            | SearchDocumentKind::LanguageConstruct
            | SearchDocumentKind::LanguageFunction
            | SearchDocumentKind::LanguageOperator
            | SearchDocumentKind::LanguageKeyword
            | SearchDocumentKind::LanguageLiteral => Ok(None),
        }
    }

    fn id_kind_matches_document(&self, requested: FactKind, document: &SearchDocument) -> bool {
        fact_kind_for_document(document).is_some_and(|actual| actual == requested)
    }

    fn map_member(&self, hit: SearchHit) -> Result<Option<ResolvedMember>, ResolveError> {
        let Some(owner) = self.owner_id_for_document(&hit.document)? else {
            return Ok(None);
        };
        let kind = match hit.document.kind {
            SearchDocumentKind::TypeProperty | SearchDocumentKind::GlobalProperty => {
                MemberKind::Property
            }
            SearchDocumentKind::TypeMethod => MemberKind::Method,
            SearchDocumentKind::TypeEvent | SearchDocumentKind::ModuleEvent => MemberKind::Event,
            SearchDocumentKind::EnumValue => MemberKind::EnumValue,
            SearchDocumentKind::PlatformType
            | SearchDocumentKind::Constructor
            | SearchDocumentKind::GlobalMethod
            | SearchDocumentKind::UnknownEvent
            | SearchDocumentKind::QueryTable
            | SearchDocumentKind::QueryTableField
            | SearchDocumentKind::QueryTableParameter
            | SearchDocumentKind::LanguageType
            | SearchDocumentKind::LanguageConstruct
            | SearchDocumentKind::LanguageFunction
            | SearchDocumentKind::LanguageOperator
            | SearchDocumentKind::LanguageKeyword
            | SearchDocumentKind::LanguageLiteral
            | SearchDocumentKind::Enum => return Ok(None),
        };
        let info = MemberInfo {
            kind,
            types: self.type_ref_facts(&hit.document.type_ref_facts),
            description: hit.document.description.clone(),
        };
        let id = MemberId(self.fact_id(FactKind::Member, hit.document.id.clone()));
        let fact = ContextFact {
            id: id.0.clone(),
            name: map_name(&hit.document),
            owner: Some(owner.0.clone()),
            details: FactDetails::Member(info.clone()),
            relations: Vec::new(),
        };
        Ok(Some(ResolvedMember {
            id,
            owner,
            fact,
            info,
        }))
    }

    fn map_callable(&self, hit: SearchHit) -> Result<Option<ResolvedCallable>, ResolveError> {
        let kind = match hit.document.kind {
            SearchDocumentKind::Constructor => CallableKind::Constructor,
            SearchDocumentKind::TypeMethod => CallableKind::Method,
            SearchDocumentKind::GlobalMethod => CallableKind::GlobalMethod,
            SearchDocumentKind::TypeEvent | SearchDocumentKind::ModuleEvent => CallableKind::Event,
            SearchDocumentKind::PlatformType
            | SearchDocumentKind::TypeProperty
            | SearchDocumentKind::GlobalProperty
            | SearchDocumentKind::UnknownEvent
            | SearchDocumentKind::QueryTable
            | SearchDocumentKind::QueryTableField
            | SearchDocumentKind::QueryTableParameter
            | SearchDocumentKind::LanguageType
            | SearchDocumentKind::LanguageConstruct
            | SearchDocumentKind::LanguageFunction
            | SearchDocumentKind::LanguageOperator
            | SearchDocumentKind::LanguageKeyword
            | SearchDocumentKind::LanguageLiteral
            | SearchDocumentKind::Enum
            | SearchDocumentKind::EnumValue => return Ok(None),
        };
        let owner = self.owner_id_for_document(&hit.document)?;
        let signatures = hit
            .document
            .signatures
            .iter()
            .map(|signature| {
                Ok(Signature {
                    parameters: signature
                        .parameters
                        .iter()
                        .map(|parameter| {
                            Ok(Parameter {
                                name: parameter.name.clone(),
                                required: parameter.required,
                                types: self.type_ref_facts(&parameter.type_ref_facts),
                                description: parameter.description.clone(),
                            })
                        })
                        .collect::<Result<Vec<_>, ResolveError>>()?,
                    return_types: self.type_ref_facts(&signature.return_type_facts),
                    title: signature.title.clone(),
                    description: signature.description.clone(),
                })
            })
            .collect::<Result<Vec<_>, ResolveError>>()?;
        let mut return_types = self.type_ref_facts(&hit.document.return_type_facts);
        if return_types.is_empty()
            && !signatures
                .iter()
                .any(|signature| !signature.return_types.is_empty())
        {
            let edge = if matches!(kind, CallableKind::Constructor) {
                "constructs"
            } else {
                "returns"
            };
            return_types = self.edge_refs(&hit.document.id, edge)?;
        }
        let info = CallableInfo {
            kind,
            signatures,
            return_types,
            description: hit.document.description.clone(),
        };
        let id = CallableId(self.fact_id(
            if matches!(kind, CallableKind::Constructor) {
                FactKind::Constructor
            } else {
                FactKind::Callable
            },
            hit.document.id.clone(),
        ));
        let fact = ContextFact {
            id: id.0.clone(),
            name: map_name(&hit.document),
            owner: owner.as_ref().map(|owner| owner.0.clone()),
            details: FactDetails::Callable(info.clone()),
            relations: Vec::new(),
        };
        Ok(Some(ResolvedCallable {
            id,
            owner,
            fact,
            info,
        }))
    }

    fn map_related(&self, hit: RelatedHit) -> Option<ContextFact> {
        if !is_platform_document_kind(hit.document.kind) {
            return None;
        }
        let kind = fact_kind_for_document(&hit.document)?;
        Some(ContextFact {
            id: self.fact_id(kind, hit.document.id.clone()),
            name: map_name(&hit.document),
            owner: None,
            details: FactDetails::Type(type_info(&hit.document)),
            relations: hit
                .via
                .into_iter()
                .filter_map(|step| {
                    Some(FactRelation {
                        kind: relation_kind_from_edge(&step.edge_kind)?,
                        target: self.fact_id(FactKind::Type, step.to),
                        evidence: Some(step.evidence),
                    })
                })
                .collect(),
        })
    }

    fn source_matches(&self, source: Option<&SourceId>) -> bool {
        source.is_none_or(|source| source == &self.source_id)
    }

    fn domain_matches(&self, domain: Option<LanguageDomain>) -> bool {
        domain.is_none_or(|domain| domain == LanguageDomain::PlatformApi)
    }
}

impl LanguageSearchSource {
    pub fn shlang(index: SearchIndex) -> Self {
        Self::new("shlang", LanguageDomain::BslLanguage, index)
    }

    pub fn open_shlang_read_only(path: impl AsRef<Path>) -> Result<Self, ResolveError> {
        Self::open_read_only(path, "shlang", LanguageDomain::BslLanguage)
    }

    pub fn shquery(index: SearchIndex) -> Self {
        Self::new("shquery", LanguageDomain::QueryLanguage, index)
    }

    pub fn open_shquery_read_only(path: impl AsRef<Path>) -> Result<Self, ResolveError> {
        Self::open_read_only(path, "shquery", LanguageDomain::QueryLanguage)
    }

    pub fn dcsui(index: SearchIndex) -> Self {
        Self::new("dcsui", LanguageDomain::QueryLanguage, index)
    }

    pub fn open_dcsui_read_only(path: impl AsRef<Path>) -> Result<Self, ResolveError> {
        Self::open_read_only(path, "dcsui", LanguageDomain::QueryLanguage)
    }

    pub fn new(source_id: impl Into<String>, domain: LanguageDomain, index: SearchIndex) -> Self {
        Self {
            source_id: SourceId::new(source_id),
            domain,
            index,
        }
    }

    pub fn open_read_only(
        path: impl AsRef<Path>,
        source_id: impl Into<String>,
        domain: LanguageDomain,
    ) -> Result<Self, ResolveError> {
        let source_id = SourceId::new(source_id);
        SearchIndex::open_read_only(path)
            .map(|index| Self {
                source_id: source_id.clone(),
                domain,
                index,
            })
            .map_err(|source| search_source_failure(&source_id, source))
    }

    fn source_failure(&self, source: SearchError) -> ResolveError {
        search_source_failure(&self.source_id, source)
    }

    fn fact_id(&self, kind: FactKind, local_id: impl Into<String>) -> FactId {
        FactId::new(self.source_id.clone(), self.domain, kind, local_id)
    }

    fn type_id(&self, local_id: impl Into<String>) -> TypeId {
        TypeId(self.fact_id(FactKind::Type, local_id))
    }

    fn callable_id(&self, local_id: impl Into<String>) -> CallableId {
        CallableId(self.fact_id(FactKind::Callable, local_id))
    }

    fn local_id(&self, document: &SearchDocument) -> String {
        document
            .id
            .strip_prefix(self.source_id.as_str())
            .and_then(|tail| tail.strip_prefix(':'))
            .unwrap_or(&document.id)
            .to_string()
    }

    fn storage_id(&self, local_id: &str) -> String {
        if local_id
            .strip_prefix(self.source_id.as_str())
            .is_some_and(|tail| tail.starts_with(':'))
        {
            local_id.to_string()
        } else {
            format!("{}:{local_id}", self.source_id)
        }
    }

    fn source_matches(&self, source: Option<&SourceId>) -> bool {
        source.is_none_or(|source| source == &self.source_id)
    }

    fn domain_matches(&self, domain: Option<LanguageDomain>) -> bool {
        domain.is_none_or(|domain| domain == self.domain)
    }

    fn document_belongs_to_source(&self, document: &SearchDocument) -> bool {
        document
            .id
            .strip_prefix(self.source_id.as_str())
            .is_some_and(|tail| tail.starts_with(':'))
            && document.kind.is_language()
    }

    fn map_context_fact(&self, hit: SearchHit) -> Option<ContextFact> {
        let kind = language_fact_kind_for_document(&hit.document)?;
        let info = if kind == FactKind::Type {
            FactDetails::Type(TypeInfo {
                description: hit.document.description.clone(),
                metadata_template: None,
                type_template_key: None,
            })
        } else if kind == FactKind::Callable {
            FactDetails::Callable(self.callable_info(&hit.document))
        } else {
            FactDetails::Language
        };
        Some(ContextFact {
            id: self.fact_id(kind, self.local_id(&hit.document)),
            name: map_name(&hit.document),
            owner: None,
            details: info,
            relations: Vec::new(),
        })
    }

    fn map_type(&self, hit: SearchHit) -> Option<ResolvedType> {
        if !matches!(
            hit.document.kind,
            SearchDocumentKind::LanguageType | SearchDocumentKind::LanguageLiteral
        ) {
            return None;
        }
        let info = TypeInfo {
            description: hit.document.description.clone(),
            metadata_template: None,
            type_template_key: None,
        };
        let id = self.type_id(self.local_id(&hit.document));
        let fact = ContextFact {
            id: id.0.clone(),
            name: map_name(&hit.document),
            owner: None,
            details: FactDetails::Type(info.clone()),
            relations: Vec::new(),
        };
        Some(ResolvedType { id, fact, info })
    }

    fn map_callable(&self, hit: SearchHit) -> Option<ResolvedCallable> {
        if hit.document.kind != SearchDocumentKind::LanguageFunction {
            return None;
        }
        let info = self.callable_info(&hit.document);
        let id = self.callable_id(self.local_id(&hit.document));
        let fact = ContextFact {
            id: id.0.clone(),
            name: map_name(&hit.document),
            owner: None,
            details: FactDetails::Callable(info.clone()),
            relations: Vec::new(),
        };
        Some(ResolvedCallable {
            id,
            owner: None,
            fact,
            info,
        })
    }

    fn map_related(&self, hit: RelatedHit) -> Option<ContextFact> {
        let id = language_fact_id_for_document(&hit.document)?;
        let details = if id.kind == FactKind::Type {
            FactDetails::Type(TypeInfo {
                description: hit.document.description.clone(),
                metadata_template: None,
                type_template_key: None,
            })
        } else if id.kind == FactKind::Callable {
            FactDetails::Callable(self.callable_info(&hit.document))
        } else {
            FactDetails::Language
        };
        Some(ContextFact {
            id,
            name: map_name(&hit.document),
            owner: None,
            details,
            relations: hit
                .via
                .into_iter()
                .filter_map(|step| {
                    Some(FactRelation {
                        kind: relation_kind_from_edge(&step.edge_kind)?,
                        target: language_fact_id_from_storage_id(&step.to)?,
                        evidence: Some(step.evidence),
                    })
                })
                .collect(),
        })
    }

    fn callable_info(&self, document: &SearchDocument) -> CallableInfo {
        CallableInfo {
            kind: CallableKind::GlobalMethod,
            signatures: document
                .signatures
                .iter()
                .map(|signature| Signature {
                    parameters: signature
                        .parameters
                        .iter()
                        .map(|parameter| Parameter {
                            name: parameter.name.clone(),
                            required: parameter.required,
                            types: self.type_refs(&parameter.type_refs),
                            description: parameter.description.clone(),
                        })
                        .collect(),
                    return_types: self.type_refs(&signature.return_types),
                    title: signature.title.clone(),
                    description: signature.description.clone(),
                })
                .collect(),
            return_types: self.type_refs(&document.return_types),
            description: document.description.clone(),
        }
    }

    fn type_refs(&self, names: &[String]) -> Vec<TypeRef> {
        names
            .iter()
            .map(|name| TypeRef {
                name: name.clone(),
                target: TypeRefTarget::Unresolved,
                template_binding: None,
            })
            .collect()
    }

    fn facts_by_name(
        &self,
        name: &str,
        kind: Option<FactKind>,
    ) -> Result<Vec<ContextFact>, ResolveError> {
        let facts = self
            .index
            .get_by_name(name)
            .map_err(|source| self.source_failure(source))?
            .into_iter()
            .filter(|hit| self.document_belongs_to_source(&hit.document))
            .filter(|hit| {
                kind.is_none_or(|kind| language_fact_kind_for_document(&hit.document) == Some(kind))
            })
            .filter_map(|hit| self.map_context_fact(hit))
            .collect::<Vec<_>>();
        Ok(facts)
    }
}

impl ContextSource for LanguageSearchSource {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            id: self.source_id.clone(),
            domain: self.domain,
            label: format!("Syntax Assistant language source {}", self.source_id),
        }
    }

    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities {
            exact_lookup: true,
            type_lookup: true,
            members: false,
            callables: true,
            relations: true,
        }
    }

    fn resolve(
        &self,
        query: context_resolver_core::ResolveQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found("language source is not active"));
        }
        match query {
            context_resolver_core::ResolveQuery::Id(id) => {
                if id.source != self.source_id || id.domain != self.domain {
                    return Ok(ResolveResponse::not_found(
                        "fact source or domain does not match",
                    ));
                }
                let storage_id = self.storage_id(&id.local_id);
                let Some(hit) = self
                    .index
                    .get_by_id(&storage_id)
                    .map_err(|source| self.source_failure(source))?
                else {
                    return Ok(ResolveResponse::not_found("language fact not found"));
                };
                if !self.document_belongs_to_source(&hit.document) {
                    return Ok(ResolveResponse::not_found("language fact not found"));
                }
                if language_fact_kind_for_document(&hit.document) != Some(id.kind) {
                    return Ok(ResolveResponse::not_found(
                        "fact kind does not match indexed language document",
                    ));
                }
                let Some(fact) = self.map_context_fact(hit) else {
                    return Ok(ResolveResponse::not_found("language fact not found"));
                };
                Ok(ResolveResponse::ok(vec![fact]))
            }
            context_resolver_core::ResolveQuery::ExactName {
                source,
                domain,
                kind,
                name,
            } => {
                if !self.source_matches(source) || !self.domain_matches(domain) {
                    return Ok(ResolveResponse::not_found(
                        "language source or domain does not match",
                    ));
                }
                let facts = self.facts_by_name(name, kind)?;
                Ok(response_from_facts(facts, "language fact not found"))
            }
        }
    }

    fn resolve_type(
        &self,
        query: TypeLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedType>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found("language source is not active"));
        }
        let facts = match query {
            TypeLookup::Id(id) => {
                if id.0.source != self.source_id || id.0.domain != self.domain {
                    Vec::new()
                } else {
                    self.index
                        .get_by_id(&self.storage_id(&id.0.local_id))
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .filter(|hit| self.document_belongs_to_source(&hit.document))
                        .filter_map(|hit| self.map_type(hit))
                        .collect()
                }
            }
            TypeLookup::ExactName {
                source,
                domain,
                name,
            } => {
                if !self.source_matches(source) || !self.domain_matches(domain) {
                    Vec::new()
                } else {
                    self.index
                        .get_by_name(name)
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .filter(|hit| self.document_belongs_to_source(&hit.document))
                        .filter_map(|hit| self.map_type(hit))
                        .collect()
                }
            }
            TypeLookup::ExactAlias {
                source,
                domain,
                alias,
            } => {
                if !self.source_matches(source) || !self.domain_matches(domain) {
                    Vec::new()
                } else {
                    self.index
                        .get_by_name(alias)
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .filter(|hit| self.document_belongs_to_source(&hit.document))
                        .filter(|hit| hit.document.name.alias.as_deref() == Some(alias))
                        .filter_map(|hit| self.map_type(hit))
                        .collect()
                }
            }
            TypeLookup::PlatformTypeTemplate { .. } => Vec::new(),
        };
        Ok(response_from_resolved_types(
            facts,
            "language type not found",
        ))
    }

    fn members(
        &self,
        _owner: &TypeId,
        _query: MemberQuery<'_>,
        _context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedMember>, ResolveError> {
        Ok(ResolveResponse::unsupported(
            "language source does not expose members in this slice",
        ))
    }

    fn callable(
        &self,
        query: CallableLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedCallable>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found("language source is not active"));
        }
        let facts = match query {
            CallableLookup::Id(id) => {
                if id.0.source != self.source_id || id.0.domain != self.domain {
                    Vec::new()
                } else {
                    self.index
                        .get_by_id(&self.storage_id(&id.0.local_id))
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .filter(|hit| self.document_belongs_to_source(&hit.document))
                        .filter_map(|hit| self.map_callable(hit))
                        .collect()
                }
            }
            CallableLookup::OwnerName { owner, name } => {
                if owner.is_some() {
                    return Ok(ResolveResponse::unsupported(
                        "language callable lookup does not support owner in this slice",
                    ));
                }
                self.index
                    .get_by_name(name)
                    .map_err(|source| self.source_failure(source))?
                    .into_iter()
                    .filter(|hit| self.document_belongs_to_source(&hit.document))
                    .filter_map(|hit| self.map_callable(hit))
                    .collect()
            }
        };
        Ok(response_from_resolved_callables(
            facts,
            "language callable not found",
        ))
    }

    fn related(
        &self,
        source: &FactId,
        kind: RelationKind,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        if !context.is_source_active(&self.source_id)
            || source.source != self.source_id
            || source.domain != self.domain
        {
            return Ok(ResolveResponse::not_found("language source fact not found"));
        }
        let edge = match kind {
            RelationKind::HasType => "has_type",
            RelationKind::Returns => "returns",
            _ => {
                return Ok(ResolveResponse::unsupported(
                    "language adapter supports has_type and returns",
                ));
            }
        };
        let storage_id = self.storage_id(&source.local_id);
        let facts = self
            .index
            .related_by_id_and_edge(&storage_id, edge, 20)
            .map_err(|source| self.source_failure(source))?
            .into_iter()
            .filter_map(|hit| self.map_related(hit))
            .collect::<Vec<_>>();
        Ok(ResolveResponse::ok(facts))
    }
}

impl ContextSource for PlatformSearchSource {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            id: self.source_id.clone(),
            domain: LanguageDomain::PlatformApi,
            label: "Syntax Assistant platform search index".to_string(),
        }
    }

    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities {
            exact_lookup: true,
            type_lookup: true,
            members: true,
            callables: true,
            relations: true,
        }
    }

    fn resolve(
        &self,
        query: context_resolver_core::ResolveQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found("platform source is not active"));
        }
        match query {
            context_resolver_core::ResolveQuery::Id(id) => {
                if id.source != self.source_id || id.domain != LanguageDomain::PlatformApi {
                    return Ok(ResolveResponse::not_found(
                        "fact source or domain does not match",
                    ));
                }
                let Some(hit) = self
                    .index
                    .get_by_id(&id.local_id)
                    .map_err(|source| self.source_failure(source))?
                else {
                    return Ok(ResolveResponse::not_found("platform fact not found"));
                };
                if !is_platform_document_kind(hit.document.kind) {
                    return Ok(ResolveResponse::not_found(
                        "non-platform document is hidden",
                    ));
                }
                if !self.id_kind_matches_document(id.kind, &hit.document) {
                    return Ok(ResolveResponse::not_found(
                        "fact kind does not match indexed platform document",
                    ));
                }
                let Some(fact) = self.map_context_fact(hit)? else {
                    return Ok(ResolveResponse::not_found("platform fact not found"));
                };
                Ok(ResolveResponse::ok(vec![fact]))
            }
            context_resolver_core::ResolveQuery::ExactName {
                source,
                domain,
                kind,
                name,
            } => {
                if !self.source_matches(source) || !self.domain_matches(domain) {
                    return Ok(ResolveResponse::not_found(
                        "platform source or domain does not match",
                    ));
                }
                if !matches!(kind, None | Some(FactKind::Type)) {
                    return Ok(ResolveResponse::unsupported(
                        "platform adapter supports exact-name resolver lookup for types only",
                    ));
                }
                let facts = self
                    .index
                    .type_identities_by_name(name)
                    .map_err(|source| self.source_failure(source))?
                    .into_iter()
                    .map(|hit| self.map_type(hit).fact)
                    .collect::<Vec<_>>();
                Ok(response_from_facts(facts, "platform type not found"))
            }
        }
    }

    fn resolve_type(
        &self,
        query: TypeLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedType>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found("platform source is not active"));
        }
        let facts = match query {
            TypeLookup::Id(id) => {
                if id.0.source != self.source_id || id.0.domain != LanguageDomain::PlatformApi {
                    Vec::new()
                } else {
                    self.index
                        .type_identity_by_id(&id.0.local_id)
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .map(|hit| self.map_type(hit))
                        .collect()
                }
            }
            TypeLookup::ExactName {
                source,
                domain,
                name,
            } => {
                if !self.source_matches(source) || !self.domain_matches(domain) {
                    Vec::new()
                } else {
                    self.index
                        .type_identities_by_name(name)
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .map(|hit| self.map_type(hit))
                        .collect()
                }
            }
            TypeLookup::ExactAlias {
                source,
                domain,
                alias,
            } => {
                if !self.source_matches(source) || !self.domain_matches(domain) {
                    Vec::new()
                } else {
                    self.index
                        .type_identities_by_alias(alias)
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .map(|hit| self.map_type(hit))
                        .collect()
                }
            }
            TypeLookup::PlatformTypeTemplate {
                source,
                domain,
                key,
            } => {
                if !self.source_matches(source) || !self.domain_matches(domain) {
                    Vec::new()
                } else {
                    let kind = syntax_helper_search::model::PlatformTypeTemplateKey::new(
                        key.family.clone(),
                        key.variant.clone(),
                    );
                    self.index
                        .type_template_by_key(&kind)
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .map(|hit| self.map_type(hit))
                        .collect()
                }
            }
        };
        Ok(response_from_resolved_types(
            facts,
            "platform type not found",
        ))
    }

    fn members(
        &self,
        owner: &TypeId,
        query: MemberQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedMember>, ResolveError> {
        if !context.is_source_active(&self.source_id)
            || owner.0.source != self.source_id
            || owner.0.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found("platform owner type not found"));
        }
        let hits = match query.name {
            Some(name) => self
                .index
                .member_by_owner_type_id(&owner.0.local_id, name)
                .map_err(|source| self.source_failure(source))?,
            None => self
                .index
                .members_by_type_id(&owner.0.local_id)
                .map_err(|source| self.source_failure(source))?,
        };
        let facts = hits
            .into_iter()
            .filter_map(|hit| self.map_member(hit).transpose())
            .filter_map(|member| match member {
                Ok(member)
                    if query
                        .kind
                        .is_none_or(|kind| member_query_matches(kind, member.info.kind)) =>
                {
                    Some(Ok(member))
                }
                Ok(_) => None,
                Err(source) => Some(Err(source)),
            })
            .collect::<Result<Vec<_>, ResolveError>>()?;
        Ok(ResolveResponse::ok(facts))
    }

    fn callable(
        &self,
        query: CallableLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedCallable>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found("platform source is not active"));
        }
        let hits = match query {
            CallableLookup::Id(id) => {
                if id.0.source != self.source_id || id.0.domain != LanguageDomain::PlatformApi {
                    Vec::new()
                } else {
                    self.index
                        .callable_by_id(&id.0.local_id)
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .collect()
                }
            }
            CallableLookup::OwnerName { owner, name } => {
                let Some(owner) = owner else {
                    return Ok(ResolveResponse::unsupported(
                        "platform callable lookup requires a resolved owner id",
                    ));
                };
                if owner.0.source != self.source_id || owner.0.domain != LanguageDomain::PlatformApi
                {
                    Vec::new()
                } else {
                    let mut hits = self
                        .index
                        .callable_by_owner_type_id(&owner.0.local_id, name)
                        .map_err(|source| self.source_failure(source))?;
                    hits.extend(
                        self.index
                            .constructors_by_type_id(&owner.0.local_id)
                            .map_err(|source| self.source_failure(source))?
                            .into_iter()
                            .filter(|hit| hit.document.name.primary == name),
                    );
                    hits
                }
            }
        };
        let mut seen = std::collections::BTreeSet::new();
        let facts = hits
            .into_iter()
            .filter(|hit| seen.insert(hit.document.id.clone()))
            .filter_map(|hit| self.map_callable(hit).transpose())
            .collect::<Result<Vec<_>, ResolveError>>()?;
        Ok(response_from_resolved_callables(
            facts,
            "platform callable not found",
        ))
    }

    fn related(
        &self,
        source: &FactId,
        kind: RelationKind,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        if !context.is_source_active(&self.source_id)
            || source.source != self.source_id
            || source.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found("platform source fact not found"));
        }
        let Some(edge) = edge_from_relation_kind(kind) else {
            return Ok(ResolveResponse::unsupported(
                "platform adapter supports has_type, returns, constructs and member_of",
            ));
        };
        let facts = self
            .index
            .related_by_id_and_edge(&source.local_id, edge, 20)
            .map_err(|source| self.source_failure(source))?
            .into_iter()
            .filter_map(|hit| self.map_related(hit))
            .collect::<Vec<_>>();
        Ok(ResolveResponse::ok(facts))
    }

    fn availability(
        &self,
        source: &FactId,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<AvailabilityFact>, ResolveError> {
        if !context.is_source_active(&self.source_id)
            || source.source != self.source_id
            || source.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found("platform source fact not found"));
        }
        let Some(hit) = self
            .index
            .get_by_id(&source.local_id)
            .map_err(|source| self.source_failure(source))?
        else {
            return Ok(ResolveResponse::not_found("platform fact not found"));
        };
        if !is_platform_document_kind(hit.document.kind) {
            return Ok(ResolveResponse::not_found(
                "non-platform document is hidden",
            ));
        }
        Ok(ResolveResponse::ok(vec![AvailabilityFact {
            id: source.clone(),
            availability: AvailabilityInfo {
                contexts: hit
                    .document
                    .availability_contexts
                    .iter()
                    .filter_map(|context| availability_context_from_code(context))
                    .collect(),
                since: hit.document.available_since,
            },
        }]))
    }
}

fn map_name(document: &SearchDocument) -> Name {
    Name::new(document.name.primary.clone(), document.name.alias.clone())
}

fn type_info(document: &SearchDocument) -> TypeInfo {
    TypeInfo {
        description: document.description.clone(),
        metadata_template: document.metadata_kind.as_ref().map(|metadata_kind| {
            MetadataTemplateInfo {
                metadata_kind: metadata_kind.clone(),
                parameters: document.template_parameters.clone(),
            }
        }),
        type_template_key: document
            .type_template_key
            .as_ref()
            .map(map_type_template_key),
    }
}

fn map_template_binding(
    binding: &syntax_helper_search::model::TypeTemplateBinding,
) -> TypeTemplateBinding {
    TypeTemplateBinding {
        template_key: map_type_template_key(&binding.template_key),
        arguments: binding
            .arguments
            .iter()
            .map(|argument| match argument {
                syntax_helper_search::model::TemplateParameterBinding::OwnerParameter {
                    owner_parameter_index,
                    target_parameter_index,
                } => TemplateParameterBinding::OwnerParameter {
                    owner_parameter_index: *owner_parameter_index,
                    target_parameter_index: *target_parameter_index,
                },
            })
            .collect(),
    }
}

fn map_type_template_key(
    kind: &syntax_helper_search::model::PlatformTypeTemplateKey,
) -> PlatformTypeTemplateKey {
    PlatformTypeTemplateKey::new(kind.family.clone(), kind.variant.clone())
}

fn is_platform_document_kind(kind: SearchDocumentKind) -> bool {
    matches!(
        kind,
        SearchDocumentKind::PlatformType
            | SearchDocumentKind::TypeProperty
            | SearchDocumentKind::TypeMethod
            | SearchDocumentKind::Constructor
            | SearchDocumentKind::GlobalMethod
            | SearchDocumentKind::GlobalProperty
            | SearchDocumentKind::ModuleEvent
            | SearchDocumentKind::TypeEvent
            | SearchDocumentKind::Enum
            | SearchDocumentKind::EnumValue
    )
}

fn fact_kind_for_document(document: &SearchDocument) -> Option<FactKind> {
    match document.kind {
        SearchDocumentKind::PlatformType => Some(FactKind::Type),
        SearchDocumentKind::TypeProperty | SearchDocumentKind::GlobalProperty => {
            Some(FactKind::Member)
        }
        SearchDocumentKind::TypeMethod
        | SearchDocumentKind::GlobalMethod
        | SearchDocumentKind::ModuleEvent
        | SearchDocumentKind::TypeEvent => Some(FactKind::Callable),
        SearchDocumentKind::Constructor => Some(FactKind::Constructor),
        SearchDocumentKind::Enum => Some(FactKind::Enum),
        SearchDocumentKind::EnumValue => Some(FactKind::EnumValue),
        SearchDocumentKind::UnknownEvent
        | SearchDocumentKind::QueryTable
        | SearchDocumentKind::QueryTableField
        | SearchDocumentKind::QueryTableParameter
        | SearchDocumentKind::LanguageType
        | SearchDocumentKind::LanguageConstruct
        | SearchDocumentKind::LanguageFunction
        | SearchDocumentKind::LanguageOperator
        | SearchDocumentKind::LanguageKeyword
        | SearchDocumentKind::LanguageLiteral => None,
    }
}

fn language_fact_kind_for_document(document: &SearchDocument) -> Option<FactKind> {
    match document.kind {
        SearchDocumentKind::LanguageType | SearchDocumentKind::LanguageLiteral => {
            Some(FactKind::Type)
        }
        SearchDocumentKind::LanguageFunction => Some(FactKind::Callable),
        SearchDocumentKind::LanguageKeyword => Some(FactKind::Keyword),
        SearchDocumentKind::LanguageOperator => Some(FactKind::Operator),
        SearchDocumentKind::LanguageConstruct => Some(FactKind::Global),
        SearchDocumentKind::PlatformType
        | SearchDocumentKind::TypeProperty
        | SearchDocumentKind::TypeMethod
        | SearchDocumentKind::Constructor
        | SearchDocumentKind::GlobalMethod
        | SearchDocumentKind::GlobalProperty
        | SearchDocumentKind::ModuleEvent
        | SearchDocumentKind::TypeEvent
        | SearchDocumentKind::UnknownEvent
        | SearchDocumentKind::QueryTable
        | SearchDocumentKind::QueryTableField
        | SearchDocumentKind::QueryTableParameter
        | SearchDocumentKind::Enum
        | SearchDocumentKind::EnumValue => None,
    }
}

fn language_fact_id_for_document(document: &SearchDocument) -> Option<FactId> {
    let kind = language_fact_kind_for_document(document)?;
    let (source, domain, local_id) = language_source_domain_and_local_id(&document.id)?;
    Some(FactId::new(source, domain, kind, local_id))
}

fn language_fact_id_from_storage_id(storage_id: &str) -> Option<FactId> {
    let (source, domain, local_id) = language_source_domain_and_local_id(storage_id)?;
    Some(FactId::new(source, domain, FactKind::Type, local_id))
}

fn language_source_domain_and_local_id(
    storage_id: &str,
) -> Option<(SourceId, LanguageDomain, &str)> {
    let (source, local_id) = storage_id.split_once(':')?;
    let domain = match source {
        "shlang" => LanguageDomain::BslLanguage,
        "shquery" | "dcsui" => LanguageDomain::QueryLanguage,
        _ => return None,
    };
    Some((SourceId::new(source), domain, local_id))
}

fn member_query_matches(query: MemberQueryKind, kind: MemberKind) -> bool {
    matches!(
        (query, kind),
        (MemberQueryKind::Property, MemberKind::Property)
            | (MemberQueryKind::Method, MemberKind::Method)
            | (MemberQueryKind::Event, MemberKind::Event)
            | (MemberQueryKind::EnumValue, MemberKind::EnumValue)
    )
}

fn availability_context_from_code(value: &str) -> Option<AvailabilityContext> {
    match value {
        "thin_client" => Some(AvailabilityContext::ThinClient),
        "web_client" => Some(AvailabilityContext::WebClient),
        "mobile_client" => Some(AvailabilityContext::MobileClient),
        "server" => Some(AvailabilityContext::Server),
        "thick_client" => Some(AvailabilityContext::ThickClient),
        "external_connection" => Some(AvailabilityContext::ExternalConnection),
        "mobile_application_client" => Some(AvailabilityContext::MobileApplicationClient),
        "mobile_application_server" => Some(AvailabilityContext::MobileApplicationServer),
        "mobile_standalone_server" => Some(AvailabilityContext::MobileStandaloneServer),
        _ => None,
    }
}

fn edge_from_relation_kind(kind: RelationKind) -> Option<&'static str> {
    match kind {
        RelationKind::HasType => Some("has_type"),
        RelationKind::Returns => Some("returns"),
        RelationKind::Constructs => Some("constructs"),
        RelationKind::MemberOf => Some("member_of"),
        _ => None,
    }
}

fn relation_kind_from_edge(edge: &str) -> Option<RelationKind> {
    match edge {
        "has_type" => Some(RelationKind::HasType),
        "returns" => Some(RelationKind::Returns),
        "constructs" => Some(RelationKind::Constructs),
        "member_of" => Some(RelationKind::MemberOf),
        _ => None,
    }
}

fn response_from_facts(
    facts: Vec<ContextFact>,
    not_found: &'static str,
) -> ResolveResponse<ContextFact> {
    match facts.len() {
        0 => ResolveResponse::not_found(not_found),
        1 => ResolveResponse::ok(facts),
        _ => ResolveResponse::ambiguous(
            facts
                .iter()
                .map(|fact| context_resolver_core::ResolveCandidate {
                    id: fact.id.clone(),
                    name: fact.name.clone(),
                })
                .collect(),
        ),
    }
}

fn response_from_resolved_types(
    facts: Vec<ResolvedType>,
    not_found: &'static str,
) -> ResolveResponse<ResolvedType> {
    match facts.len() {
        0 => ResolveResponse::not_found(not_found),
        1 => ResolveResponse::ok(facts),
        _ => ResolveResponse::ambiguous(
            facts
                .iter()
                .map(|fact| context_resolver_core::ResolveCandidate {
                    id: fact.id.0.clone(),
                    name: fact.fact.name.clone(),
                })
                .collect(),
        ),
    }
}

fn response_from_resolved_callables(
    facts: Vec<ResolvedCallable>,
    not_found: &'static str,
) -> ResolveResponse<ResolvedCallable> {
    match facts.len() {
        0 => ResolveResponse::not_found(not_found),
        1 => ResolveResponse::ok(facts),
        _ => ResolveResponse::ambiguous(
            facts
                .iter()
                .map(|fact| context_resolver_core::ResolveCandidate {
                    id: fact.id.0.clone(),
                    name: fact.fact.name.clone(),
                })
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Instant;

    use context_resolver_core::{
        CallableLookup, CompositeResolver, ContextResolver, ContextSource, MemberQuery,
        PlatformTypeTemplateKey, RelationKind, ResolveContext, ResolveStatus,
        TemplateParameterBinding, TypeLookup,
    };
    use syntax_helper_language::{LanguagePageInput, LanguageSourceFamily, extract_language_facts};
    use syntax_helper_model as model;
    use syntax_helper_model::SyntaxHelperSink;
    use syntax_helper_search::{IndexMetadata, SearchIndexBuilder, build_index_from_builder};

    use super::*;

    #[test]
    fn platform_adapter_opens_read_only_index_from_path() {
        let source = fixture_source();
        let path = fixture_index_path("platform-adapter-open-read-only.sqlite");
        let adapter = PlatformSearchSource::open_read_only_with_source_id(&path, source.clone())
            .expect("platform adapter must open index path");

        assert_eq!(adapter.descriptor().id, source.clone());

        let response = adapter
            .resolve_type(
                TypeLookup::ExactName {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    name: "ОтборКомпоновкиДанных",
                },
                &ResolveContext::all(),
            )
            .expect("type lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(response.facts.len(), 1);
    }

    #[test]
    fn platform_adapter_resolves_alias_and_metadata_template_info() {
        let source = fixture_source();
        let index = fixture_index("platform-adapter-template-alias.sqlite");
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());

        let response = adapter
            .resolve_type(
                TypeLookup::ExactAlias {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    alias: "CatalogManager.<Catalog name>",
                },
                &ResolveContext::all(),
            )
            .expect("alias type lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        let template = response.facts.first().expect("template must resolve");
        assert_eq!(
            template.id.0.local_id,
            "platform_type:СправочникМенеджер.<Имя справочника>"
        );
        assert_eq!(
            template
                .info
                .metadata_template
                .as_ref()
                .expect("template metadata must be exposed")
                .metadata_kind,
            "СправочникМенеджер"
        );
        assert_eq!(
            template.info.metadata_template.as_ref().unwrap().parameters,
            vec!["Имя справочника".to_string()]
        );
        assert_eq!(
            template.info.type_template_key,
            Some(PlatformTypeTemplateKey::new("Catalog", "Manager"))
        );

        let by_kind = adapter
            .resolve_type(
                TypeLookup::PlatformTypeTemplate {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    key: &PlatformTypeTemplateKey::new("Catalog", "Manager"),
                },
                &ResolveContext::all(),
            )
            .expect("semantic type template lookup must not fail");
        assert_eq!(by_kind.status, ResolveStatus::Ok);
        assert_eq!(by_kind.facts[0].id, template.id);
    }

    #[test]
    fn platform_adapter_exposes_template_owner_parameter_binding() {
        let source = fixture_source();
        let index = fixture_index("platform-adapter-generic-binding.sqlite");
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());
        let owner = adapter
            .resolve_type(
                TypeLookup::PlatformTypeTemplate {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    key: &PlatformTypeTemplateKey::new("Document", "Object"),
                },
                &ResolveContext::all(),
            )
            .expect("document object template lookup must not fail")
            .facts
            .into_iter()
            .next()
            .expect("document object template must resolve");

        let response = adapter
            .members(
                &owner.id,
                MemberQuery {
                    name: Some("Ссылка"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("type-template member lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        let property_type = response.facts[0]
            .info
            .types
            .first()
            .expect("type-template property type must be exposed");
        assert_eq!(
            property_type.resolved_id().map(|id| id.0.local_id.as_str()),
            Some("platform_type:ДокументСсылка.<Имя документа>")
        );
        let binding = property_type
            .template_binding
            .as_ref()
            .expect("template owner-parameter binding must be visible");
        assert_eq!(
            binding.template_key,
            PlatformTypeTemplateKey::new("Document", "Ref")
        );
        assert_eq!(
            binding.arguments,
            vec![TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: 0,
                target_parameter_index: 0,
            }]
        );
    }

    #[test]
    fn platform_adapter_exposes_template_constructor_result_binding() {
        let source = fixture_source();
        let index = fixture_index("platform-adapter-generic-constructor-binding.sqlite");
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());
        let owner = TypeId(FactId::new(
            source,
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ДокументОбъект.<Имя документа>",
        ));

        let constructor = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&owner),
                    name: "Новый ДокументОбъект.<Имя документа>()",
                },
                &ResolveContext::all(),
            )
            .expect("type-template constructor lookup must not fail");

        assert_eq!(constructor.status, ResolveStatus::Ok);
        let result_type = constructor.facts[0]
            .info
            .return_types
            .first()
            .expect("type-template constructor result type must be exposed");
        assert_eq!(
            result_type.resolved_id().map(|id| id.0.local_id.as_str()),
            Some("platform_type:ДокументОбъект.<Имя документа>")
        );
        let binding = result_type
            .template_binding
            .as_ref()
            .expect("type-template constructor result binding must be visible");
        assert_eq!(
            binding.template_key,
            PlatformTypeTemplateKey::new("Document", "Object")
        );
        assert_eq!(
            binding.arguments,
            vec![TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: 0,
                target_parameter_index: 0,
            }]
        );
    }

    #[test]
    fn platform_adapter_resolves_type_member_callable_and_relations() {
        let source = fixture_source();
        let filter = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ОтборКомпоновкиДанных",
        ));
        let settings = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:НастройкиКомпоновкиДанных",
        ));
        let index = fixture_index("platform-adapter.sqlite");
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());

        let started = Instant::now();
        let type_response = adapter
            .resolve_type(
                TypeLookup::Id(&filter),
                &ResolveContext {
                    active_sources: std::slice::from_ref(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    scope: None,
                },
            )
            .expect("type lookup must not fail");
        assert_eq!(type_response.status, ResolveStatus::Ok);
        assert_eq!(
            type_response.facts[0].fact.name.primary,
            "ОтборКомпоновкиДанных"
        );
        assert!(started.elapsed().as_millis() < 100);

        let started = Instant::now();
        let members = adapter
            .members(
                &filter,
                MemberQuery {
                    name: None,
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("member listing must not fail");
        assert!(
            members
                .facts
                .iter()
                .any(|member| member.fact.name.primary == "Элементы")
        );
        assert!(started.elapsed().as_millis() < 100);

        let filter_member = adapter
            .members(
                &settings,
                MemberQuery {
                    name: Some("Отбор"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("owner member lookup must not fail");
        assert_eq!(filter_member.facts.len(), 1);
        assert_eq!(filter_member.facts[0].owner, settings);

        let started = Instant::now();
        let has_type = adapter
            .related(
                &filter_member.facts[0].id.0,
                RelationKind::HasType,
                &ResolveContext::all(),
            )
            .expect("has_type traversal must not fail");
        let relation_elapsed = started.elapsed();
        assert_eq!(
            has_type.facts[0].id.local_id,
            "platform_type:ОтборКомпоновкиДанных"
        );
        assert!(relation_elapsed.as_millis() < 100);

        let started = Instant::now();
        let callable = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Найти",
                },
                &ResolveContext::all(),
            )
            .expect("callable lookup must not fail");
        let callable_elapsed = started.elapsed();
        assert_eq!(callable.status, ResolveStatus::Ok);
        assert_eq!(
            callable.facts[0].info.signatures[0].parameters[0].name,
            "Значение"
        );
        assert!(callable_elapsed.as_millis() < 100);
        assert_eq!(
            callable.facts[0].info.return_types[0].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:ЭлементОтбораКомпоновкиДанных",
            )))
        );

        let returns = adapter
            .related(
                &callable.facts[0].id.0,
                RelationKind::Returns,
                &ResolveContext::all(),
            )
            .expect("returns traversal must not fail");
        assert_eq!(
            returns.facts[0].id.local_id,
            "platform_type:ЭлементОтбораКомпоновкиДанных"
        );

        let constructor = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Новый ОтборКомпоновкиДанных()",
                },
                &ResolveContext::all(),
            )
            .expect("constructor lookup must not fail");
        assert_eq!(constructor.status, ResolveStatus::Ok);
        assert_eq!(constructor.facts.len(), 1);
        assert_eq!(
            constructor.facts[0].info.return_types[0].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:ОтборКомпоновкиДанных",
            )))
        );
        let constructs = adapter
            .related(
                &constructor.facts[0].id.0,
                RelationKind::Constructs,
                &ResolveContext::all(),
            )
            .expect("constructs traversal must not fail");
        assert_eq!(
            constructs.facts[0].id.local_id,
            "platform_type:ОтборКомпоновкиДанных"
        );

        let member_of = adapter
            .related(
                &filter_member.facts[0].id.0,
                RelationKind::MemberOf,
                &ResolveContext::all(),
            )
            .expect("member_of traversal must not fail");
        assert_eq!(
            member_of.facts[0].id.local_id,
            "platform_type:НастройкиКомпоновкиДанных"
        );
    }

    #[test]
    fn platform_adapter_keeps_signature_return_out_of_callable_return() {
        let source = fixture_source();
        let filter = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ОтборКомпоновкиДанных",
        ));
        let index = fixture_index_with_signature_only_return(
            "platform-adapter-signature-only-return.sqlite",
        );
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());

        let callable = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Найти",
                },
                &ResolveContext::all(),
            )
            .expect("callable lookup must not fail");

        assert_eq!(callable.status, ResolveStatus::Ok);
        assert!(
            callable.facts[0].info.return_types.is_empty(),
            "signature return types must not be folded back into callable return types"
        );
        let signature_return = callable.facts[0].info.signatures[0]
            .return_types
            .first()
            .expect("signature-level return type must be exposed");
        assert_eq!(
            signature_return
                .resolved_id()
                .map(|id| id.0.local_id.as_str()),
            Some("platform_type:ЭлементОтбораКомпоновкиДанных")
        );
    }

    #[test]
    fn platform_adapter_preserves_type_ref_resolution_status_and_template_binding() {
        let source = fixture_source();
        let path = temp_path("platform-adapter-type-ref-resolution.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for record in [
            platform_template_type(
                "ДокументМенеджер.<Имя документа>",
                "DocumentManager.<Document name>",
                "ДокументМенеджер",
                "Имя документа",
            ),
            platform_template_type(
                "ДокументОбъект.<Имя документа>",
                "DocumentObject.<Document name>",
                "ДокументОбъект",
                "Имя документа",
            ),
            platform_template_type(
                "ДокументСсылка.<Имя документа>",
                "DocumentRef.<Document name>",
                "ДокументСсылка",
                "Имя документа",
            ),
            platform_type("РазрешенныйТип", None),
            platform_type_with_owner_path("ДубльТип", "Первый"),
            platform_type_with_owner_path("ДубльТип", "Второй"),
        ] {
            builder
                .platform_type(record)
                .expect("platform type must sink");
        }
        builder
            .type_property(model::PlatformProperty {
                owner: name(
                    "ДокументОбъект.<Имя документа>",
                    Some("DocumentObject.<Document name>"),
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name("Поле", None),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![
                    model::TypeRef {
                        name: "ДокументСсылка".to_string(),
                    },
                    model::TypeRef {
                        name: "РазрешенныйТип".to_string(),
                    },
                    model::TypeRef {
                        name: "НесуществующийТип".to_string(),
                    },
                    model::TypeRef {
                        name: "ДубльТип".to_string(),
                    },
                ],
                description: None,
                facts: model::SectionFacts::default(),
                source: source_ref("owner-field"),
            })
            .expect("property must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");
        let adapter = PlatformSearchSource::with_source_id(open_index(&path), source.clone());
        let owner = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ДокументОбъект.<Имя документа>",
        ));

        let members = adapter
            .members(
                &owner,
                MemberQuery {
                    name: Some("Поле"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("member lookup must not fail");

        let types = &members.facts[0].info.types;
        assert_eq!(types.len(), 4);
        assert_eq!(
            types[0].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:ДокументСсылка.<Имя документа>",
            )))
        );
        assert_eq!(
            types[0].resolved_id().map(|id| id.0.local_id.as_str()),
            Some("platform_type:ДокументСсылка.<Имя документа>")
        );
        let binding = types[0]
            .template_binding
            .as_ref()
            .expect("resolved template target must carry owner-parameter binding");
        assert_eq!(
            binding.template_key,
            PlatformTypeTemplateKey::new("Document", "Ref")
        );
        assert_eq!(
            binding.arguments,
            vec![TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: 0,
                target_parameter_index: 0,
            }]
        );
        assert_eq!(
            types[1].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:РазрешенныйТип",
            )))
        );
        assert_eq!(types[1].template_binding, None);
        assert_eq!(types[2].target, TypeRefTarget::Unresolved);
        assert_eq!(types[2].template_binding, None);
        assert_eq!(
            types[3].target,
            TypeRefTarget::Ambiguous(vec![
                TypeId(FactId::new(
                    source.clone(),
                    LanguageDomain::PlatformApi,
                    FactKind::Type,
                    "platform_type:ДубльТип:Второй",
                )),
                TypeId(FactId::new(
                    source,
                    LanguageDomain::PlatformApi,
                    FactKind::Type,
                    "platform_type:ДубльТип:Первый",
                )),
            ])
        );
        assert_eq!(types[3].template_binding, None);
    }

    #[test]
    fn platform_adapter_does_not_expose_query_table_documents() {
        let source = fixture_source();
        let index = fixture_index("platform-adapter-query-table-hidden.sqlite");
        for id in [
            "query_table:ОсновнаяТаблица",
            "query_table_field:query_table:ОсновнаяТаблица:Период",
            "query_table_parameter:query_table:ОсновнаяТаблица:Дата",
        ] {
            assert!(
                index
                    .get_by_id(id)
                    .expect("search provider id lookup must not fail")
                    .is_some(),
                "{id} must stay available as a search/provider document"
            );
        }
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());

        for (kind, local_id) in [
            (FactKind::QueryTable, "query_table:ОсновнаяТаблица"),
            (
                FactKind::QueryField,
                "query_table_field:query_table:ОсновнаяТаблица:Период",
            ),
            (
                FactKind::QueryParameter,
                "query_table_parameter:query_table:ОсновнаяТаблица:Дата",
            ),
        ] {
            let fact = FactId::new(source.clone(), LanguageDomain::PlatformApi, kind, local_id);
            let response = adapter
                .resolve(
                    context_resolver_core::ResolveQuery::Id(&fact),
                    &ResolveContext::all(),
                )
                .expect("query_table* id lookup must not fail");

            assert_eq!(response.status, ResolveStatus::NotFound);
        }
    }

    #[test]
    fn platform_adapter_does_not_synthesize_constructor_return_from_owner() {
        let source = fixture_source();
        let filter = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ОтборКомпоновкиДанных",
        ));
        let index = fixture_index_without_constructor_result(
            "platform-adapter-missing-constructor-result.sqlite",
        );
        let adapter = PlatformSearchSource::with_source_id(index, source);

        let constructor = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Новый ОтборКомпоновкиДанных()",
                },
                &ResolveContext::all(),
            )
            .expect("constructor lookup must not fail");

        assert_eq!(constructor.status, ResolveStatus::Ok);
        assert_eq!(constructor.facts.len(), 1);
        assert!(
            constructor.facts[0].info.return_types.is_empty(),
            "constructor return type must require explicit return/constructs evidence"
        );

        let constructs = adapter
            .related(
                &constructor.facts[0].id.0,
                RelationKind::Constructs,
                &ResolveContext::all(),
            )
            .expect("constructs traversal must not fail");
        assert_eq!(constructs.status, ResolveStatus::Ok);
        assert!(
            constructs.facts.is_empty(),
            "constructs traversal must require edge-specific source evidence"
        );
    }

    #[test]
    fn language_adapter_opens_read_only_index_from_path() {
        let path = language_fixture_index("language-adapter-open-read-only.sqlite");
        let source = SourceId::new("shlang");
        let adapter = LanguageSearchSource::open_shlang_read_only(&path)
            .expect("language adapter must open index path");

        assert_eq!(adapter.descriptor().id, source.clone());

        let response = adapter
            .resolve_type(
                TypeLookup::ExactName {
                    source: Some(&source),
                    domain: Some(LanguageDomain::BslLanguage),
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("type lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(response.facts.len(), 1);
    }

    #[test]
    fn language_adapter_preserves_domain_identity_and_ambiguity() {
        let path = language_fixture_index("language-resolver-ambiguity.sqlite");
        let shlang = SourceId::new("shlang");
        let shquery = SourceId::new("shquery");
        let dcsui = SourceId::new("dcsui");
        let resolver = CompositeResolver::new(vec![
            Box::new(LanguageSearchSource::shlang(open_index(&path))),
            Box::new(LanguageSearchSource::shquery(open_index(&path))),
            Box::new(LanguageSearchSource::dcsui(open_index(&path))),
        ]);

        let ambiguous = resolver
            .resolve(
                context_resolver_core::ResolveQuery::ExactName {
                    source: None,
                    domain: None,
                    kind: None,
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("language fact lookup must not fail");
        assert_eq!(ambiguous.status, ResolveStatus::Ambiguous);
        let ambiguous_ids = ambiguous
            .candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.id.source.as_str().to_string(),
                    candidate.id.local_id.as_str().to_string(),
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(ambiguous_ids.contains(&("shlang".to_string(), "def_String".to_string())));
        assert!(ambiguous_ids.contains(&("shquery".to_string(), "STRING".to_string())));
        assert!(ambiguous_ids.contains(&("shquery".to_string(), "LitString".to_string())));

        let ambiguous_types = resolver
            .resolve_type(
                TypeLookup::ExactName {
                    source: None,
                    domain: None,
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("language type lookup must not fail");
        assert_eq!(ambiguous_types.status, ResolveStatus::Ambiguous);
        let ambiguous_type_ids = ambiguous_types
            .candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.id.source.as_str().to_string(),
                    candidate.id.local_id.as_str().to_string(),
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(ambiguous_type_ids.contains(&("shlang".to_string(), "def_String".to_string())));
        assert!(ambiguous_type_ids.contains(&("shquery".to_string(), "LitString".to_string())));

        let started = Instant::now();
        let bsl_string = resolver
            .resolve_type(
                TypeLookup::ExactName {
                    source: Some(&shlang),
                    domain: Some(LanguageDomain::BslLanguage),
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("constrained BSL lookup must not fail");
        assert_eq!(bsl_string.status, ResolveStatus::Ok);
        assert_eq!(bsl_string.facts[0].id.0.local_id, "def_String");
        assert_eq!(bsl_string.facts[0].id.0.domain, LanguageDomain::BslLanguage);
        assert!(started.elapsed().as_millis() < 100);

        let query_string = CallableId(FactId::new(
            shquery.clone(),
            LanguageDomain::QueryLanguage,
            FactKind::Callable,
            "STRING",
        ));
        let query_function = resolver
            .callable(CallableLookup::Id(&query_string), &ResolveContext::all())
            .expect("query function lookup must not fail");
        assert_eq!(query_function.status, ResolveStatus::Ok);
        assert_eq!(query_function.facts[0].id.0.source, shquery);
        assert_eq!(query_function.facts[0].fact.name.primary, "СТРОКА");

        let query_literal = TypeId(FactId::new(
            SourceId::new("shquery"),
            LanguageDomain::QueryLanguage,
            FactKind::Type,
            "LitString",
        ));
        let literal = resolver
            .resolve_type(TypeLookup::Id(&query_literal), &ResolveContext::all())
            .expect("query literal lookup must not fail");
        assert_eq!(literal.status, ResolveStatus::Ok);
        assert_eq!(literal.facts[0].id.0.local_id, "LitString");

        let skd_string_length = CallableId(FactId::new(
            dcsui,
            LanguageDomain::QueryLanguage,
            FactKind::Callable,
            "SKD_Functions_Strings#StringLength",
        ));
        let skd_function = resolver
            .callable(
                CallableLookup::Id(&skd_string_length),
                &ResolveContext::all(),
            )
            .expect("SKD function lookup must not fail");
        assert_eq!(skd_function.status, ResolveStatus::Ok);
        assert_eq!(skd_function.facts[0].fact.name.primary, "ДлинаСтроки");
        assert_eq!(
            skd_function.facts[0].info.signatures[0].parameters[0].name,
            "Строка"
        );
    }

    #[test]
    fn language_adapter_traverses_only_explicit_extracted_type_edges() {
        let path = language_fixture_index("language-resolver-relations.sqlite");
        let shquery = SourceId::new("shquery");
        let dcsui = SourceId::new("dcsui");
        let query_adapter = LanguageSearchSource::shquery(open_index(&path));
        let adapter = LanguageSearchSource::dcsui(open_index(&path));
        let query_string = FactId::new(
            shquery,
            LanguageDomain::QueryLanguage,
            FactKind::Callable,
            "STRING",
        );
        let string_length = FactId::new(
            dcsui,
            LanguageDomain::QueryLanguage,
            FactKind::Callable,
            "SKD_Functions_Strings#StringLength",
        );

        let query_return = query_adapter
            .related(&query_string, RelationKind::Returns, &ResolveContext::all())
            .expect("query return traversal must not fail");
        assert_eq!(query_return.status, ResolveStatus::Ok);
        assert!(
            query_return.facts.iter().any(|fact| {
                fact.id.source.as_str() == "shquery"
                    && fact.id.domain == LanguageDomain::QueryLanguage
                    && fact.id.local_id == "LitString"
            }),
            "query STRING return must use the explicit query-language string literal/type edge"
        );
        assert!(
            query_return
                .facts
                .iter()
                .all(|fact| fact.id.source.as_str() != "shlang"),
            "query STRING return must not choose the BSL string type by same-name lookup"
        );

        let started = Instant::now();
        let related = adapter
            .related(
                &string_length,
                RelationKind::HasType,
                &ResolveContext::all(),
            )
            .expect("language relation traversal must not fail");
        assert!(started.elapsed().as_millis() < 100);
        assert_eq!(related.status, ResolveStatus::Ok);
        assert!(
            related.facts.iter().any(|fact| {
                fact.id.source.as_str() == "shlang"
                    && fact.id.domain == LanguageDomain::BslLanguage
                    && fact.id.local_id == "def_String"
            }),
            "SKD parameter type must traverse to the explicit BSL string type edge"
        );
    }

    fn fixture_source() -> SourceId {
        SourceId::new("test-platform")
    }

    fn fixture_index(file_name: &str) -> SearchIndex {
        let path = fixture_index_path(file_name);
        SearchIndex::open_read_only(path).expect("index must open")
    }

    fn fixture_index_without_constructor_result(file_name: &str) -> SearchIndex {
        let path = fixture_index_path(file_name);
        let connection = rusqlite::Connection::open(&path).expect("index must open for mutation");
        let constructor_id =
            "constructor:platform_type:ОтборКомпоновкиДанных:Новый ОтборКомпоновкиДанных()";
        connection
            .execute(
                "DELETE FROM type_refs WHERE source_document_id = ?1 AND ref_kind = 'constructor_result'",
                [constructor_id],
            )
            .expect("constructor result type ref must be removable");
        connection
            .execute(
                "DELETE FROM relations WHERE source_id = ?1 AND edge_kind = 'constructs'",
                [constructor_id],
            )
            .expect("constructor relation must be removable");
        drop(connection);
        SearchIndex::open_read_only(path).expect("index must open")
    }

    fn fixture_index_with_signature_only_return(file_name: &str) -> SearchIndex {
        let path = fixture_index_path(file_name);
        let connection = rusqlite::Connection::open(&path).expect("index must open for mutation");
        let method_id = "type_method:platform_type:ОтборКомпоновкиДанных:Найти";
        let signature_id: String = connection
            .query_row(
                "SELECT signature_id FROM signatures WHERE callable_id = ?1 ORDER BY ordinal LIMIT 1",
                [method_id],
                |row| row.get(0),
            )
            .expect("fixture method signature must exist");
        connection
            .execute(
                "UPDATE type_refs
                 SET source_signature_id = ?1,
                     source_signature_ordinal = 0
                 WHERE source_document_id = ?2
                   AND ref_kind = 'return_type'",
                rusqlite::params![signature_id, method_id],
            )
            .expect("fixture return type must become signature-scoped");
        drop(connection);
        SearchIndex::open_read_only(path).expect("index must open")
    }

    fn fixture_index_path(file_name: &str) -> PathBuf {
        let path = temp_path(file_name);
        let mut builder = SearchIndexBuilder::new();
        for record in [
            platform_type("НастройкиКомпоновкиДанных", None),
            platform_type("ОтборКомпоновкиДанных", Some("DataCompositionFilter")),
            platform_template_type(
                "СправочникМенеджер.<Имя справочника>",
                "CatalogManager.<Catalog name>",
                "СправочникМенеджер",
                "Имя справочника",
            ),
            platform_template_type(
                "ДокументМенеджер.<Имя документа>",
                "DocumentManager.<Document name>",
                "ДокументМенеджер",
                "Имя документа",
            ),
            platform_template_type(
                "ДокументОбъект.<Имя документа>",
                "DocumentObject.<Document name>",
                "ДокументОбъект",
                "Имя документа",
            ),
            platform_template_type(
                "ДокументСсылка.<Имя документа>",
                "DocumentRef.<Document name>",
                "ДокументСсылка",
                "Имя документа",
            ),
            platform_type("КоллекцияЭлементовОтбораКомпоновкиДанных", None),
            platform_type("ЭлементОтбораКомпоновкиДанных", None),
        ] {
            builder
                .platform_type(record)
                .expect("platform type must sink");
        }
        builder
            .type_property(model::PlatformProperty {
                owner: name("НастройкиКомпоновкиДанных", None),
                owner_identity: Some("platform_type:НастройкиКомпоновкиДанных".to_string()),
                name: name("Отбор", None),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ОтборКомпоновкиДанных".to_string(),
                }],
                description: Some("Фильтр настроек.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("settings-filter"),
            })
            .expect("property must sink");
        builder
            .type_property(model::PlatformProperty {
                owner: name("ОтборКомпоновкиДанных", None),
                owner_identity: Some("platform_type:ОтборКомпоновкиДанных".to_string()),
                name: name("Элементы", None),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "КоллекцияЭлементовОтбораКомпоновкиДанных".to_string(),
                }],
                description: Some("Элементы фильтра.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("filter-items"),
            })
            .expect("property must sink");
        builder
            .type_method(model::PlatformMethod {
                owner: name("ОтборКомпоновкиДанных", None),
                owner_identity: Some("platform_type:ОтборКомпоновкиДанных".to_string()),
                name: name("Найти", None),
                semantic: model::SemanticContext::default(),
                signatures: vec![model::Signature {
                    text: "Найти(<Значение>)".to_string(),
                    parameters: vec![model::Parameter {
                        name: "Значение".to_string(),
                        required: true,
                        type_refs: vec![model::TypeRef {
                            name: "ОтборКомпоновкиДанных".to_string(),
                        }],
                        description: None,
                    }],
                    return_types: Vec::new(),
                    variant: None,
                }],
                return_types: vec![model::TypeRef {
                    name: "ЭлементОтбораКомпоновкиДанных".to_string(),
                }],
                description: Some("Ищет элемент.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("filter-find"),
            })
            .expect("method must sink");
        builder
            .type_property(model::PlatformProperty {
                owner: name(
                    "ДокументОбъект.<Имя документа>",
                    Some("DocumentObject.<Document name>"),
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name("Ссылка", Some("Ref")),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ДокументСсылка".to_string(),
                }],
                description: Some("Document reference.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("document-object-ref"),
            })
            .expect("type-template property must sink");
        builder
            .constructor(model::Constructor {
                owner: name("ОтборКомпоновкиДанных", None),
                owner_identity: Some("platform_type:ОтборКомпоновкиДанных".to_string()),
                name: name("Новый ОтборКомпоновкиДанных()", None),
                semantic: model::SemanticContext::default(),
                signatures: vec![model::Signature {
                    text: "Новый ОтборКомпоновкиДанных()".to_string(),
                    parameters: Vec::new(),
                    return_types: Vec::new(),
                    variant: None,
                }],
                description: Some("Создает фильтр.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("filter-constructor"),
            })
            .expect("constructor must sink");
        builder
            .constructor(model::Constructor {
                owner: name(
                    "ДокументОбъект.<Имя документа>",
                    Some("DocumentObject.<Document name>"),
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name(
                    "Новый ДокументОбъект.<Имя документа>()",
                    Some("New DocumentObject.<Document name>()"),
                ),
                semantic: model::SemanticContext::default(),
                signatures: vec![model::Signature {
                    text: "Новый ДокументОбъект.<Имя документа>()".to_string(),
                    parameters: Vec::new(),
                    return_types: Vec::new(),
                    variant: None,
                }],
                description: Some("Creates document object.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("document-object-constructor"),
            })
            .expect("type-template constructor must sink");
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:ОсновнаяТаблица".to_string()),
                name: "Основная таблица".to_string(),
                syntax: Some(name("ОсновнаяТаблица", None)),
                identifier: Some("ОсновнаяТаблица".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Primary,
                description: Some("Query provider fact.".to_string()),
                source: source_ref("query-table"),
            })
            .expect("query table must sink");
        builder
            .table_field(model::QueryTableField {
                owner: name("ОсновнаяТаблица", None),
                owner_identity: Some("query_table:ОсновнаяТаблица".to_string()),
                name: "Период".to_string(),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTableField,
                ),
                type_refs: vec![model::TypeRef {
                    name: "Дата".to_string(),
                }],
                description: Some("Query field provider fact.".to_string()),
                note: None,
                source: source_ref("query-table-field"),
            })
            .expect("query table field must sink");
        builder
            .table_parameter(model::QueryTableParameter {
                owner: name("ОсновнаяТаблица", None),
                owner_identity: Some("query_table:ОсновнаяТаблица".to_string()),
                name: "Дата".to_string(),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTableParameter,
                ),
                type_refs: vec![model::TypeRef {
                    name: "Дата".to_string(),
                }],
                description: Some("Query parameter provider fact.".to_string()),
                default_value: None,
                source: source_ref("query-table-parameter"),
            })
            .expect("query table parameter must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");
        path
    }

    fn platform_type(primary: &str, alias: Option<&str>) -> model::PlatformType {
        model::PlatformType {
            identity: None,
            name: name(primary, alias),
            semantic: model::SemanticContext::default(),
            type_kind: model::PlatformTypeKind::Regular,
            object_kind: None,
            extends: Vec::new(),
            metadata_kind: None,
            template_parameters: Vec::new(),
            type_template_key: None,
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: Some(format!("{primary} description.")),
            facts: model::SectionFacts::default(),
            source: source_ref(primary),
        }
    }

    fn platform_type_with_owner_path(primary: &str, owner_path: &str) -> model::PlatformType {
        let mut record = platform_type(primary, None);
        record.semantic = model::SemanticContext::new(
            model::BranchKind::PlatformObjects,
            model::RecordFamily::PlatformType,
        )
        .with_owner_path(vec![name(owner_path, None)]);
        record
    }

    fn platform_template_type(
        primary: &str,
        alias: &str,
        metadata_kind: &str,
        template_parameter: &str,
    ) -> model::PlatformType {
        let mut record = platform_type(primary, Some(alias));
        record.type_kind = model::PlatformTypeKind::MetadataTemplate;
        record.metadata_kind = Some(metadata_kind.to_string());
        record.template_parameters = vec![template_parameter.to_string()];
        record
    }

    fn name(primary: &str, alias: Option<&str>) -> model::LocalizedName {
        model::LocalizedName {
            primary: primary.to_string(),
            alias: alias.map(ToOwned::to_owned),
        }
    }

    fn source_ref(title: &str) -> model::SyntaxHelperSource {
        model::SyntaxHelperSource {
            hbk_path: PathBuf::from("/fixtures/shcntx_ru.hbk"),
            locale: "ru".to_string(),
            toc_path: Some(title.to_string()),
            html_path: format!("{title}.html"),
            page_title: title.to_string(),
        }
    }

    fn metadata() -> IndexMetadata {
        IndexMetadata {
            locale: "ru".to_string(),
            source_locale: "ru".to_string(),
            source_hbk: "/fixtures/shcntx_ru.hbk".to_string(),
            source_extraction_schema_version: 11,
        }
    }

    fn language_fixture_index(file_name: &str) -> PathBuf {
        let path = temp_path(file_name);
        let mut builder = SearchIndexBuilder::new();
        for fact in language_fixture_facts() {
            builder.add_language_fact(fact);
        }
        build_index_from_builder(&path, &metadata(), builder).expect("language index must build");
        path
    }

    fn language_fixture_facts() -> Vec<syntax_helper_language::LanguageFact> {
        [
            (
                LanguageSourceFamily::Shlang,
                "def_String",
                "shlang_def_string_ru.html",
            ),
            (
                LanguageSourceFamily::Shquery,
                "STRING",
                "shquery_string_ru.html",
            ),
            (
                LanguageSourceFamily::Shquery,
                "LitString",
                "shquery_lit_string_ru.html",
            ),
            (
                LanguageSourceFamily::Dcsui,
                "SKD_Functions_Strings",
                "dcsui_functions_strings_ru.html",
            ),
        ]
        .into_iter()
        .flat_map(|(source_family, html_path, fixture_name)| {
            let html = std::fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../tests/fixtures/syntax-helper-language")
                    .join(fixture_name),
            )
            .expect("language fixture must be readable");
            extract_language_facts(LanguagePageInput {
                source_hbk: "fixture.hbk",
                source_family,
                locale: "ru",
                html_path,
                html: &html,
            })
        })
        .collect()
    }

    fn open_index(path: &std::path::Path) -> SearchIndex {
        SearchIndex::open_read_only(path).expect("index must open")
    }

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "v8-context-hbk-context-resolver-search-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }
}
