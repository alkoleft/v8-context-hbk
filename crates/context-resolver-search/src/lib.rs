use context_resolver_core::{
    CallableId, CallableInfo, CallableKind, CallableLookup, ContextFact, ContextSource,
    FactDetails, FactId, FactKind, FactRelation, LanguageDomain, MemberId, MemberInfo, MemberKind,
    MemberQuery, MemberQueryKind, Name, Parameter, RelationKind, ResolveContext, ResolveError,
    ResolveResponse, ResolvedCallable, ResolvedMember, ResolvedType, Signature, SourceCapabilities,
    SourceDescriptor, SourceId, TypeId, TypeInfo, TypeLookup, TypeRef,
};
use syntax_helper_search::{RelatedHit, SearchDocument, SearchError, SearchHit, SearchIndex};

const DEFAULT_SOURCE_ID: &str = "shcntx-platform";

pub struct PlatformSearchSource {
    source_id: SourceId,
    index: SearchIndex,
}

impl PlatformSearchSource {
    pub fn new(index: SearchIndex) -> Self {
        Self {
            source_id: SourceId::new(DEFAULT_SOURCE_ID),
            index,
        }
    }

    pub fn with_source_id(index: SearchIndex, source_id: SourceId) -> Self {
        Self { source_id, index }
    }

    fn source_failure(&self, source: SearchError) -> ResolveError {
        ResolveError::SourceFailure {
            source_id: self.source_id.clone(),
            message: source.to_string(),
        }
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

    fn type_ref(&self, name: &str) -> Result<TypeRef, ResolveError> {
        let candidates = self
            .index
            .type_identities_by_name(name)
            .map_err(|source| self.source_failure(source))?;
        let id = match candidates.as_slice() {
            [hit] => Some(self.type_id(hit.document.id.clone())),
            _ => None,
        };
        Ok(TypeRef {
            name: name.to_string(),
            id,
        })
    }

    fn typed_refs(&self, names: &[String]) -> Result<Vec<TypeRef>, ResolveError> {
        names.iter().map(|name| self.type_ref(name)).collect()
    }

    fn edge_refs(&self, document_id: &str, edge: &str) -> Result<Vec<TypeRef>, ResolveError> {
        let mut hits = self
            .index
            .related_by_id_and_edge(document_id, edge, 20)
            .map_err(|source| self.source_failure(source))?;
        if hits.is_empty() {
            hits = self
                .index
                .related_by_id(document_id, 1, 20)
                .map_err(|source| self.source_failure(source))?
                .into_iter()
                .filter(|hit| hit.via.iter().any(|step| step.edge_kind == edge))
                .collect();
        }
        hits.into_iter()
            .map(|hit| {
                Ok(TypeRef {
                    name: hit.document.name.primary,
                    id: Some(self.type_id(hit.document.id)),
                })
            })
            .collect()
    }

    fn map_type(&self, hit: SearchHit) -> ResolvedType {
        let info = TypeInfo {
            description: hit.document.description.clone(),
        };
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
        match hit.document.kind.as_str() {
            "platform_type" => Ok(Some(self.map_type(hit).fact)),
            "type_property" | "global_property" | "enum_value" => {
                Ok(self.map_member(hit)?.map(|member| member.fact))
            }
            "type_method" | "global_method" | "constructor" | "type_event" | "module_event" => {
                Ok(self.map_callable(hit)?.map(|callable| callable.fact))
            }
            "enum" => Ok(Some(ContextFact {
                id: self.fact_id(FactKind::Enum, hit.document.id.clone()),
                name: map_name(&hit.document),
                owner: None,
                details: FactDetails::Enum,
                relations: Vec::new(),
            })),
            _ => Ok(None),
        }
    }

    fn id_kind_matches_document(&self, requested: FactKind, document: &SearchDocument) -> bool {
        fact_kind_for_document(document).is_some_and(|actual| actual == requested)
    }

    fn map_member(&self, hit: SearchHit) -> Result<Option<ResolvedMember>, ResolveError> {
        let Some(owner) = self.owner_id_for_document(&hit.document)? else {
            return Ok(None);
        };
        let kind = match hit.document.kind.as_str() {
            "type_property" | "global_property" => MemberKind::Property,
            "type_method" => MemberKind::Method,
            "type_event" | "module_event" => MemberKind::Event,
            "enum_value" => MemberKind::EnumValue,
            _ => return Ok(None),
        };
        let info = MemberInfo {
            kind,
            types: self.typed_refs(&hit.document.type_refs)?,
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
        let kind = match hit.document.kind.as_str() {
            "constructor" => CallableKind::Constructor,
            "type_method" => CallableKind::Method,
            "global_method" => CallableKind::GlobalMethod,
            "type_event" | "module_event" => CallableKind::Event,
            _ => return Ok(None),
        };
        let owner = self.owner_id_for_document(&hit.document)?;
        let mut return_types = self.typed_refs(&hit.document.return_types)?;
        if return_types.is_empty() {
            let edge = if matches!(kind, CallableKind::Constructor) {
                "constructs"
            } else {
                "returns"
            };
            return_types = self.edge_refs(&hit.document.id, edge)?;
        }
        if return_types.is_empty()
            && matches!(kind, CallableKind::Constructor)
            && let (Some(owner), Some(owner_name)) = (&owner, hit.document.owner.as_ref())
        {
            return_types.push(TypeRef {
                name: owner_name.primary.clone(),
                id: Some(owner.clone()),
            });
        }
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
                                types: self.typed_refs(&parameter.type_refs)?,
                                description: parameter.description.clone(),
                            })
                        })
                        .collect::<Result<Vec<_>, ResolveError>>()?,
                    title: signature.title.clone(),
                    description: signature.description.clone(),
                })
            })
            .collect::<Result<Vec<_>, ResolveError>>()?;
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
        if !is_platform_document_kind(&hit.document.kind) {
            return None;
        }
        let kind = fact_kind_for_document(&hit.document)?;
        Some(ContextFact {
            id: self.fact_id(kind, hit.document.id.clone()),
            name: map_name(&hit.document),
            owner: None,
            details: FactDetails::Type(TypeInfo {
                description: hit.document.description,
            }),
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
                if !is_platform_document_kind(&hit.document.kind) {
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
        if !facts.is_empty() {
            return Ok(ResolveResponse::ok(facts));
        }
        let facts = self
            .index
            .related_by_id(&source.local_id, 1, 20)
            .map_err(|source| self.source_failure(source))?
            .into_iter()
            .filter(|hit| hit.via.iter().any(|step| step.edge_kind == edge))
            .filter_map(|hit| self.map_related(hit))
            .collect::<Vec<_>>();
        Ok(ResolveResponse::ok(facts))
    }
}

fn map_name(document: &SearchDocument) -> Name {
    Name::new(document.name.primary.clone(), document.name.alias.clone())
}

fn is_platform_document_kind(kind: &str) -> bool {
    matches!(
        kind,
        "platform_type"
            | "type_property"
            | "type_method"
            | "constructor"
            | "global_method"
            | "global_property"
            | "module_event"
            | "type_event"
            | "enum"
            | "enum_value"
    )
}

fn fact_kind_for_document(document: &SearchDocument) -> Option<FactKind> {
    match document.kind.as_str() {
        "platform_type" => Some(FactKind::Type),
        "type_property" | "global_property" => Some(FactKind::Member),
        "type_method" | "global_method" | "module_event" | "type_event" => Some(FactKind::Callable),
        "constructor" => Some(FactKind::Constructor),
        "enum" => Some(FactKind::Enum),
        "enum_value" => Some(FactKind::EnumValue),
        _ => None,
    }
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
        ContextSource, MemberQuery, RelationKind, ResolveContext, ResolveStatus, TypeLookup,
    };
    use syntax_helper_model as model;
    use syntax_helper_model::SyntaxHelperSink;
    use syntax_helper_search::{IndexMetadata, SearchIndexBuilder, build_index_from_builder};

    use super::*;

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
            callable.facts[0].info.return_types[0]
                .id
                .as_ref()
                .expect("return type id must be preserved")
                .0
                .local_id,
            "platform_type:ЭлементОтбораКомпоновкиДанных"
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
            constructor.facts[0].info.return_types[0]
                .id
                .as_ref()
                .expect("constructor type id must be preserved")
                .0
                .local_id,
            "platform_type:ОтборКомпоновкиДанных"
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
    fn platform_adapter_does_not_expose_query_table_documents() {
        let source = fixture_source();
        let index = fixture_index("platform-adapter-query-table-hidden.sqlite");
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());
        let query_table = FactId::new(
            source,
            LanguageDomain::PlatformApi,
            FactKind::QueryTable,
            "query_table:ОсновнаяТаблица",
        );

        let response = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&query_table),
                &ResolveContext::all(),
            )
            .expect("query_table id lookup must not fail");

        assert_eq!(response.status, ResolveStatus::NotFound);
    }

    fn fixture_source() -> SourceId {
        SourceId::new("test-platform")
    }

    fn fixture_index(file_name: &str) -> SearchIndex {
        let path = temp_path(file_name);
        let mut builder = SearchIndexBuilder::new();
        for record in [
            platform_type("НастройкиКомпоновкиДанных", None),
            platform_type("ОтборКомпоновкиДанных", Some("DataCompositionFilter")),
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
            .constructor(model::Constructor {
                owner: name("ОтборКомпоновкиДанных", None),
                name: name("Новый ОтборКомпоновкиДанных()", None),
                semantic: model::SemanticContext::default(),
                signatures: vec![model::Signature {
                    text: "Новый ОтборКомпоновкиДанных()".to_string(),
                    parameters: Vec::new(),
                    variant: None,
                }],
                description: Some("Создает фильтр.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("filter-constructor"),
            })
            .expect("constructor must sink");
        builder
            .query_table(model::QueryTable {
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
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");
        SearchIndex::open_read_only(path).expect("index must open")
    }

    fn platform_type(primary: &str, alias: Option<&str>) -> model::PlatformType {
        model::PlatformType {
            name: name(primary, alias),
            semantic: model::SemanticContext::default(),
            type_kind: model::PlatformTypeKind::Regular,
            object_kind: None,
            extends: Vec::new(),
            metadata_kind: None,
            template_parameters: Vec::new(),
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: Some(format!("{primary} description.")),
            facts: model::SectionFacts::default(),
            source: source_ref(primary),
        }
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
