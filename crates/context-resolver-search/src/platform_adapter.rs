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

    fn module_context_id(&self, kind: ModuleContextKind) -> FactId {
        self.fact_id(
            FactKind::ModuleContext,
            format!("module_context:{}", kind.as_str()),
        )
    }

    fn module_context_fact(&self, kind: ModuleContextKind) -> ContextFact {
        ContextFact {
            id: self.module_context_id(kind),
            name: Name::new(format!("{} module context", kind.as_str()), None::<String>),
            owner: None,
            details: FactDetails::ModuleContext(ModuleContextInfo {
                language: GlobalContextLanguage::Bsl,
                domain: LanguageDomain::PlatformApi,
                kind,
            }),
            relations: Vec::new(),
        }
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

    fn map_context_fact_for_kind(
        &self,
        hit: SearchHit,
        requested: Option<FactKind>,
    ) -> Result<Option<ContextFact>, ResolveError> {
        match hit.document.kind {
            SearchDocumentKind::PlatformType => Ok(Some(self.map_type(hit).fact)),
            SearchDocumentKind::TypeProperty | SearchDocumentKind::EnumValue => {
                Ok(self.map_member(hit)?.map(|member| member.fact))
            }
            SearchDocumentKind::GlobalProperty => Ok(Some(self.map_global_property(hit.document))),
            SearchDocumentKind::TypeMethod
            | SearchDocumentKind::GlobalMethod
            | SearchDocumentKind::Constructor => {
                Ok(self.map_callable(hit)?.map(|callable| callable.fact))
            }
            SearchDocumentKind::TypeEvent | SearchDocumentKind::ModuleEvent
                if requested == Some(FactKind::Member) =>
            {
                Ok(self.map_member(hit)?.map(|member| member.fact))
            }
            SearchDocumentKind::TypeEvent | SearchDocumentKind::ModuleEvent => {
                Ok(self.map_callable(hit)?.map(|callable| callable.fact))
            }
            SearchDocumentKind::Enum if requested == Some(FactKind::Type) => {
                Ok(Some(self.map_type(hit).fact))
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

    fn map_global_property(&self, document: SearchDocument) -> ContextFact {
        let info = MemberInfo {
            kind: MemberKind::Property,
            types: self.type_ref_facts(&document.type_ref_facts),
            description: document.description.clone(),
        };
        ContextFact {
            id: self.fact_id(FactKind::Global, document.id.clone()),
            name: map_name(&document),
            owner: None,
            details: FactDetails::Member(info),
            relations: Vec::new(),
        }
    }

    fn map_global_methods(&self) -> Result<Vec<ResolvedCallable>, ResolveError> {
        self.index
            .documents_by_kind(SearchDocumentKind::GlobalMethod)
            .map_err(|source| self.source_failure(source))?
            .into_iter()
            .filter_map(|hit| self.map_callable(hit).transpose())
            .collect::<Result<Vec<_>, ResolveError>>()
    }

    fn map_global_properties(&self) -> Result<Vec<ContextFact>, ResolveError> {
        Ok(self
            .index
            .documents_by_kind(SearchDocumentKind::GlobalProperty)
            .map_err(|source| self.source_failure(source))?
            .into_iter()
            .map(|hit| self.map_global_property(hit.document))
            .collect())
    }

    fn map_module_event(
        &self,
        hit: SearchHit,
        context_id: &FactId,
    ) -> Result<Option<ResolvedCallable>, ResolveError> {
        let Some(mut callable) = self.map_callable(hit)? else {
            return Ok(None);
        };
        callable.fact.owner = Some(context_id.clone());
        Ok(Some(callable))
    }

    fn id_kind_matches_document(&self, requested: FactKind, document: &SearchDocument) -> bool {
        fact_kind_for_document(document).is_some_and(|actual| actual == requested)
            || matches!(
                (requested, document.kind),
                (FactKind::Type, SearchDocumentKind::Enum)
                    |
                (
                    FactKind::Member,
                    SearchDocumentKind::TypeEvent | SearchDocumentKind::ModuleEvent
                )
            )
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
        let kind = if hit.document.kind.is_type_ref_target()
            && hit.via.iter().any(|step| {
                matches!(
                    step.edge_kind.as_str(),
                    "has_type" | "returns" | "constructs"
                )
            }) {
            FactKind::Type
        } else {
            fact_kind_for_document(&hit.document)?
        };
        let details = match kind {
            FactKind::Type => FactDetails::Type(type_info(&hit.document)),
            FactKind::Enum => FactDetails::Enum,
            _ => return None,
        };
        Some(ContextFact {
            id: self.fact_id(kind, hit.document.id.clone()),
            name: map_name(&hit.document),
            owner: None,
            details,
            relations: hit
                .via
                .into_iter()
                .filter_map(|step| {
                    Some(FactRelation {
                        kind: relation_kind_from_edge(&step.edge_kind)?,
                        target: self.fact_id(kind, step.to),
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
