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

    pub fn query_tables(index: SearchIndex) -> Self {
        Self::new_query_tables(
            "shcntx-query",
            SourceId::new(DEFAULT_SOURCE_ID),
            index,
        )
    }

    pub fn open_query_tables_read_only(path: impl AsRef<Path>) -> Result<Self, ResolveError> {
        Self::open_query_tables_read_only_with_source_ids(
            path,
            "shcntx-query",
            SourceId::new(DEFAULT_SOURCE_ID),
        )
    }

    pub fn open_query_tables_read_only_with_source_ids(
        path: impl AsRef<Path>,
        source_id: impl Into<String>,
        platform_source_id: SourceId,
    ) -> Result<Self, ResolveError> {
        let source_id = SourceId::new(source_id);
        SearchIndex::open_read_only(path)
            .map(|index| Self::new_query_tables(source_id.as_str(), platform_source_id, index))
            .map_err(|source| search_source_failure(&source_id, source))
    }

    pub fn new(source_id: impl Into<String>, domain: LanguageDomain, index: SearchIndex) -> Self {
        Self {
            source_id: SourceId::new(source_id),
            domain,
            index,
            query_table_templates: false,
            platform_source_id: SourceId::new(DEFAULT_SOURCE_ID),
        }
    }

    pub fn new_query_tables(
        source_id: impl Into<String>,
        platform_source_id: SourceId,
        index: SearchIndex,
    ) -> Self {
        Self {
            source_id: SourceId::new(source_id),
            domain: LanguageDomain::QueryLanguage,
            index,
            query_table_templates: true,
            platform_source_id,
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
                query_table_templates: false,
                platform_source_id: SourceId::new(DEFAULT_SOURCE_ID),
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
        if self.query_table_templates {
            return local_id.to_string();
        }
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
        if self.query_table_templates {
            return matches!(
                document.kind,
                SearchDocumentKind::QueryTable
                    | SearchDocumentKind::QueryTableField
                    | SearchDocumentKind::QueryTableParameter
            );
        }
        document
            .id
            .strip_prefix(self.source_id.as_str())
            .is_some_and(|tail| tail.starts_with(':'))
            && document.kind.is_language()
    }

    fn map_context_fact(&self, hit: SearchHit) -> Option<ContextFact> {
        let kind = language_fact_kind_for_document(&hit.document)?;
        let info = if kind == FactKind::QueryTable {
            FactDetails::QueryTable(self.query_table_info(&hit.document))
        } else if kind == FactKind::QueryField {
            FactDetails::QueryField(self.query_field_info(&hit.document)?)
        } else if kind == FactKind::QueryParameter {
            FactDetails::QueryParameter(self.query_parameter_info(&hit.document)?)
        } else if kind == FactKind::Type {
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
        let owner = if matches!(kind, FactKind::QueryField | FactKind::QueryParameter) {
            self.owner_fact_id(&hit.document)
        } else {
            None
        };
        Some(ContextFact {
            id: self.fact_id(kind, self.local_id(&hit.document)),
            name: map_name(&hit.document),
            owner,
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
        let id = self.fact_id_from_storage_id(&hit.document.id)?;
        let details = if id.kind == FactKind::Type {
            FactDetails::Type(TypeInfo {
                description: hit.document.description.clone(),
                metadata_template: None,
                type_template_key: None,
            })
        } else if id.kind == FactKind::Callable {
            FactDetails::Callable(self.callable_info(&hit.document))
        } else if id.kind == FactKind::QueryTable {
            FactDetails::QueryTable(self.query_table_info(&hit.document))
        } else if id.kind == FactKind::QueryField {
            FactDetails::QueryField(self.query_field_info(&hit.document)?)
        } else if id.kind == FactKind::QueryParameter {
            FactDetails::QueryParameter(self.query_parameter_info(&hit.document)?)
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
                        target: self.fact_id_from_storage_id(&step.to)?,
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
                    variadic: signature_text_is_variadic(&signature.text),
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

    fn search_type_refs(&self, refs: &[SearchTypeRef]) -> Vec<TypeRef> {
        refs.iter().map(|type_ref| self.search_type_ref(type_ref)).collect()
    }

    fn search_type_ref(&self, type_ref: &SearchTypeRef) -> TypeRef {
        TypeRef {
            name: type_ref.name.clone(),
            target: self.search_type_ref_target(&type_ref.target),
            template_binding: type_ref.template_binding.as_ref().map(map_template_binding),
        }
    }

    fn search_type_ref_target(&self, target: &SearchTypeRefTarget) -> TypeRefTarget {
        match target {
            SearchTypeRefTarget::Ok(id) => self
                .type_id_from_storage_id(id)
                .map(TypeRefTarget::Ok)
                .unwrap_or(TypeRefTarget::Unresolved),
            SearchTypeRefTarget::Unresolved => TypeRefTarget::Unresolved,
            SearchTypeRefTarget::Ambiguous(candidates) => {
                let candidates = candidates
                    .iter()
                    .filter_map(|id| self.type_id_from_storage_id(id))
                    .collect::<Vec<_>>();
                if candidates.is_empty() {
                    TypeRefTarget::Unresolved
                } else {
                    TypeRefTarget::Ambiguous(candidates)
                }
            }
        }
    }

    fn type_id_from_storage_id(&self, storage_id: &str) -> Option<TypeId> {
        let fact_id = self.fact_id_from_storage_id(storage_id)?;
        (fact_id.kind == FactKind::Type).then_some(TypeId(fact_id))
    }

    fn fact_id_from_storage_id(&self, storage_id: &str) -> Option<FactId> {
        if let Some((source, domain, kind, local_id)) = language_source_domain_kind_and_local_id(storage_id)
        {
            return Some(FactId::new(source, domain, kind, local_id));
        }
        let kind = match storage_id.split_once(':')?.0 {
            "platform_type" | "enum" => FactKind::Type,
            "query_table" => FactKind::QueryTable,
            "query_table_field" => FactKind::QueryField,
            "query_table_parameter" => FactKind::QueryParameter,
            _ => return None,
        };
        let (source, domain) = if matches!(kind, FactKind::QueryTable | FactKind::QueryField | FactKind::QueryParameter) {
            (self.source_id.clone(), LanguageDomain::QueryLanguage)
        } else {
            (self.platform_source_id.clone(), LanguageDomain::PlatformApi)
        };
        Some(FactId::new(source, domain, kind, storage_id.to_string()))
    }

    fn query_table_info(&self, document: &SearchDocument) -> QueryTableInfo {
        QueryTableInfo {
            syntax: document.query_syntax.as_ref().map(map_model_name),
            identifier: document.query_identifier.clone(),
            sdbl_metadata_source_selector: sdbl_metadata_source_selector(
                document.query_identifier.as_deref(),
            ),
            table_role: query_table_role(document.query_table_role),
            owner_path: document.owner_path.iter().map(map_model_name).collect(),
            template_parameters: document.template_parameters.clone(),
            description: document.description.clone(),
            source: document
                .source
                .as_ref()
                .map(|source| fact_provenance(&self.source_id, &document.id, source)),
        }
    }

    fn query_field_info(&self, document: &SearchDocument) -> Option<QueryFieldInfo> {
        Some(QueryFieldInfo {
            owner: self.owner_fact_id(document)?,
            types: self.search_type_refs(&document.type_ref_facts),
            description: document.description.clone(),
            note: document.note.clone(),
            source: document
                .source
                .as_ref()
                .map(|source| fact_provenance(&self.source_id, &document.id, source)),
        })
    }

    fn query_parameter_info(&self, document: &SearchDocument) -> Option<QueryParameterInfo> {
        Some(QueryParameterInfo {
            owner: self.owner_fact_id(document)?,
            types: self.search_type_refs(&document.type_ref_facts),
            description: document.description.clone(),
            default_value: document.default_value.clone(),
            source: document
                .source
                .as_ref()
                .map(|source| fact_provenance(&self.source_id, &document.id, source)),
        })
    }

    fn owner_fact_id(&self, document: &SearchDocument) -> Option<FactId> {
        let prefix = format!("{}:", document.kind.as_str());
        let rest = document.id.strip_prefix(&prefix)?;
        let (owner, _) = rest.rsplit_once(':')?;
        Some(FactId::new(
            self.source_id.clone(),
            LanguageDomain::QueryLanguage,
            FactKind::QueryTable,
            owner.to_string(),
        ))
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

    fn source_id(&self) -> Option<&SourceId> {
        Some(&self.source_id)
    }

    fn capabilities(&self) -> SourceCapabilities {
        if self.query_table_templates {
            return SourceCapabilities {
                exact_lookup: true,
                type_lookup: false,
                members: false,
                callables: false,
                relations: true,
                global_context: true,
                module_context: false,
            };
        }
        SourceCapabilities {
            exact_lookup: true,
            type_lookup: true,
            members: false,
            callables: true,
            relations: true,
            global_context: true,
            module_context: false,
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
        if self.query_table_templates {
            return Ok(ResolveResponse::unsupported(
                "query table source does not expose type lookup",
            ));
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
            TypeLookup::PlatformTypeTemplate { .. }
            | TypeLookup::GeneratedSelfTemplate { .. } => {
                return Ok(ResolveResponse::unsupported(
                    "language source does not expose platform generated-self templates",
                ));
            }
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
        if self.query_table_templates {
            return Ok(ResolveResponse::unsupported(
                "query table source does not expose callable lookup",
            ));
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

    fn global_context(
        &self,
        query: GlobalContextQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedGlobalContext>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found("language source is not active"));
        }
        if self.query_table_templates {
            let GlobalContextQuery::Language { language, sources } = query;
            if language != GlobalContextLanguage::Sdbl
                || (!sources.is_empty() && !sources.iter().any(|source| source == &self.source_id))
            {
                return Ok(ResolveResponse::not_found(
                    "query table source does not expose requested global context",
                ));
            }

            let mut facts = Vec::new();
            for kind in [
                SearchDocumentKind::QueryTable,
                SearchDocumentKind::QueryTableField,
                SearchDocumentKind::QueryTableParameter,
            ] {
                facts.extend(
                    self.index
                        .documents_by_kind(kind)
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .filter(|hit| self.document_belongs_to_source(&hit.document))
                        .filter_map(|hit| self.map_context_fact(hit)),
                );
            }

            return Ok(ResolveResponse::ok(vec![ResolvedGlobalContext {
                id: self.fact_id(FactKind::Global, "global_context:sdbl:query_tables"),
                language,
                sources: vec![self.source_id.clone()],
                methods: Vec::new(),
                properties: Vec::new(),
                facts,
            }]));
        }
        let GlobalContextQuery::Language { language, sources } = query;
        let expected_domain = match language {
            GlobalContextLanguage::Bsl => LanguageDomain::BslLanguage,
            GlobalContextLanguage::Sdbl => LanguageDomain::QueryLanguage,
        };
        if self.domain != expected_domain
            || (!sources.is_empty() && !sources.iter().any(|source| source == &self.source_id))
        {
            return Ok(ResolveResponse::not_found(
                "language source does not expose requested global context",
            ));
        }

        let mut methods = Vec::new();
        let mut facts = Vec::new();
        for kind in [
            SearchDocumentKind::LanguageType,
            SearchDocumentKind::LanguageConstruct,
            SearchDocumentKind::LanguageFunction,
            SearchDocumentKind::LanguageOperator,
            SearchDocumentKind::LanguageKeyword,
            SearchDocumentKind::LanguageLiteral,
        ] {
            for hit in self
                .index
                .documents_by_kind(kind)
                .map_err(|source| self.source_failure(source))?
                .into_iter()
                .filter(|hit| self.document_belongs_to_source(&hit.document))
            {
                if let Some(callable) = self.map_callable(hit.clone()) {
                    methods.push(callable);
                } else if let Some(fact) = self.map_context_fact(hit) {
                    facts.push(fact);
                }
            }
        }

        Ok(ResolveResponse::ok(vec![ResolvedGlobalContext {
            id: self.fact_id(
                FactKind::Global,
                match language {
                    GlobalContextLanguage::Bsl => "global_context:bsl",
                    GlobalContextLanguage::Sdbl => "global_context:sdbl",
                },
            ),
            language,
            sources: vec![self.source_id.clone()],
            methods,
            properties: Vec::new(),
            facts,
        }]))
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
            RelationKind::MemberOf => "member_of",
            _ => {
                return Ok(ResolveResponse::unsupported(
                    "language adapter supports has_type, returns and member_of",
                ));
            }
        };
        let storage_id = self.storage_id(&source.local_id);
        let Some(source_hit) = self
            .index
            .get_by_id(&storage_id)
            .map_err(|source| self.source_failure(source))?
        else {
            return Ok(ResolveResponse::not_found("language source fact not found"));
        };
        if !self.document_belongs_to_source(&source_hit.document) {
            return Ok(ResolveResponse::not_found("language source fact not found"));
        }
        if language_fact_kind_for_document(&source_hit.document) != Some(source.kind) {
            return Ok(ResolveResponse::not_found(
                "fact kind does not match indexed language document",
            ));
        }
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
