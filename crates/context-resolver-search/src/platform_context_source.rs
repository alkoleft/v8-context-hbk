impl ContextSource for PlatformSearchSource {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            id: self.source_id.clone(),
            domain: LanguageDomain::PlatformApi,
            label: "Syntax Assistant platform search index".to_string(),
        }
    }

    fn source_id(&self) -> Option<&SourceId> {
        Some(&self.source_id)
    }

    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities {
            exact_lookup: true,
            type_lookup: true,
            members: true,
            callables: true,
            relations: true,
            global_context: true,
            module_context: true,
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
                if id.kind == FactKind::ModuleContext {
                    let Some(kind) = module_context_kind_from_local_id(&id.local_id) else {
                        return Ok(ResolveResponse::not_found(
                            "module context local id is not recognized",
                        ));
                    };
                    let context = self.module_context(
                        ModuleContextQuery {
                            language: GlobalContextLanguage::Bsl,
                            domain: LanguageDomain::PlatformApi,
                            kind,
                            sources: &[],
                        },
                        context,
                    )?;
                    if context.status == ResolveStatus::Ok {
                        return Ok(ResolveResponse::ok(vec![self.module_context_fact(kind)]));
                    }
                    return Ok(ResolveResponse {
                        status: context.status,
                        facts: Vec::new(),
                        candidates: context.candidates,
                        diagnostics: context.diagnostics,
                    });
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
                let Some(fact) = self.map_context_fact_for_kind(hit, Some(id.kind))? else {
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
            TypeLookup::GeneratedSelfTemplate {
                source,
                domain,
                generated_self_role,
            } => {
                if !self.source_matches(source) || !self.domain_matches(domain) {
                    Vec::new()
                } else if let Some(key) = template_key_for_generated_self_role(generated_self_role)
                {
                    let kind = syntax_helper_search::model::PlatformTypeTemplateKey::new(
                        key.family,
                        key.variant,
                    );
                    self.index
                        .type_template_by_key(&kind)
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .map(|hit| self.map_type(hit))
                        .collect()
                } else {
                    Vec::new()
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
        if hits.is_empty() {
            if query.name.is_some() {
                return Ok(ResolveResponse::not_found("platform member not found"));
            }
            let Some(owner_hit) = self
                .index
                .get_by_id(&owner.0.local_id)
                .map_err(|source| self.source_failure(source))?
            else {
                return Ok(ResolveResponse::not_found("platform owner type not found"));
            };
            match owner_hit.document.kind {
                SearchDocumentKind::PlatformType => return Ok(ResolveResponse::ok(Vec::new())),
                SearchDocumentKind::Enum => {
                    if query
                        .kind
                        .is_some_and(|kind| kind != MemberQueryKind::EnumValue)
                    {
                        return Ok(ResolveResponse::ok(Vec::new()));
                    }
                    let owned_values = self
                        .index
                        .related_by_id_and_edge(&owner.0.local_id, "owns", usize::MAX)
                        .map_err(|source| self.source_failure(source))?
                        .into_iter()
                        .filter(|hit| hit.document.kind == SearchDocumentKind::EnumValue)
                        .map(|hit| SearchHit {
                            document: hit.document,
                            score: 0,
                        })
                        .collect::<Vec<_>>();
                    let facts = owned_values
                        .into_iter()
                        .map(|hit| self.map_enum_value_member(owner, hit))
                        .collect::<Vec<_>>();
                    return Ok(ResolveResponse::ok(facts));
                }
                _ => return Ok(ResolveResponse::not_found("platform owner type not found")),
            }
        }
        let facts = hits
            .into_iter()
            .filter_map(|hit| self.map_member(hit).transpose())
            .filter_map(|member| match member {
                Ok(member) if query.kind.is_none_or(|kind| member.info.kind.query_kind() == kind) => {
                    Some(Ok(member))
                }
                Ok(_) => None,
                Err(source) => Some(Err(source)),
            })
            .collect::<Result<Vec<_>, ResolveError>>()?;
        if query.name.is_some() && facts.is_empty() {
            return Ok(ResolveResponse::not_found("platform member not found"));
        }
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
                if let Some(owner) = owner {
                    if owner.0.source != self.source_id
                        || owner.0.domain != LanguageDomain::PlatformApi
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
                } else {
                    let response = self.global_context(
                        GlobalContextQuery::Language {
                            language: GlobalContextLanguage::Bsl,
                            sources: &[],
                        },
                        context,
                    )?;
                    return Ok(response_from_resolved_callables(
                        response
                            .facts
                            .into_iter()
                            .flat_map(|scope| scope.methods)
                            .filter(|method| name_matches(&method.fact.name, name))
                            .collect(),
                        "platform callable not found",
                    ));
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

    fn global_context(
        &self,
        query: GlobalContextQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedGlobalContext>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found("platform source is not active"));
        }
        let GlobalContextQuery::Language { language, sources } = query;
        if language != GlobalContextLanguage::Bsl
            || (!sources.is_empty() && !sources.iter().any(|source| source == &self.source_id))
        {
            return Ok(ResolveResponse::not_found(
                "platform source does not expose requested global context",
            ));
        }

        let methods = self.map_global_methods()?;
        let properties = self.map_global_properties()?;

        Ok(ResolveResponse::ok(vec![ResolvedGlobalContext {
            id: self.fact_id(FactKind::Global, "global_context:bsl"),
            language,
            sources: vec![self.source_id.clone()],
            methods,
            properties,
            facts: Vec::new(),
        }]))
    }

    fn module_context(
        &self,
        query: ModuleContextQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedModuleContext>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found("platform source is not active"));
        }
        if query.language != GlobalContextLanguage::Bsl
            || query.domain != LanguageDomain::PlatformApi
            || (!query.sources.is_empty() && !query.sources.iter().any(|source| source == &self.source_id))
        {
            return Ok(ResolveResponse::not_found(
                "platform source does not expose requested module context",
            ));
        }
        let Some(search_key) = crate::hbk_catalogs::bsl::bsl_module_context_key(query.kind) else {
            return Ok(ResolveResponse::unsupported(format!(
                "platform module context `{}` is not provider-backed by the HBK search index",
                query.kind.as_str()
            )));
        };

        let context_id = self.module_context_id(query.kind);
        let methods = self.map_global_methods()?;
        let properties = self.map_global_properties()?;
        let events = self
            .index
            .get_by_name(search_key)
            .map_err(|source| self.source_failure(source))?
            .into_iter()
            .filter(|hit| hit.document.kind == SearchDocumentKind::ModuleEvent)
            .filter_map(|hit| self.map_module_event(hit, &context_id).transpose())
            .collect::<Result<Vec<_>, ResolveError>>()?;

        if methods.is_empty() && properties.is_empty() && events.is_empty() {
            return Ok(ResolveResponse::not_found(
                "platform module context facts not found",
            ));
        }

        Ok(ResolveResponse::ok(vec![ResolvedModuleContext {
            id: context_id,
            language: query.language,
            domain: query.domain,
            kind: query.kind,
            sources: vec![self.source_id.clone()],
            self_member: None,
            properties,
            methods,
            events,
            facts: vec![self.module_context_fact(query.kind)],
        }]))
    }

    fn module_context_member(
        &self,
        query: ModuleContextMemberLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedBslContextMember>, ResolveError> {
        if !context.is_source_active(&self.source_id)
            || query.language != GlobalContextLanguage::Bsl
            || query.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found(
                "platform source does not expose requested module member",
            ));
        }
        let facts = match query.kind {
            MemberQueryKind::Property => self
                .index
                .get_by_name(query.name)
                .map_err(|source| self.source_failure(source))?
                .into_iter()
                .filter(|hit| hit.document.kind == SearchDocumentKind::GlobalProperty)
                .map(|hit| ResolvedBslContextMember::Property(self.map_global_property(hit.document)))
                .collect(),
            MemberQueryKind::Method => self
                .index
                .get_by_name(query.name)
                .map_err(|source| self.source_failure(source))?
                .into_iter()
                .filter(|hit| hit.document.kind == SearchDocumentKind::GlobalMethod)
                .filter_map(|hit| self.map_callable(hit).transpose())
                .collect::<Result<Vec<_>, ResolveError>>()?
                .into_iter()
                .map(ResolvedBslContextMember::Callable)
                .collect(),
            MemberQueryKind::Event => {
                let Some(search_key) =
                    crate::hbk_catalogs::bsl::bsl_module_context_key(query.module_kind)
                else {
                    return Ok(ResolveResponse::unsupported(format!(
                        "platform module context `{}` is not provider-backed by the HBK search index",
                        query.module_kind.as_str()
                    )));
                };
                self.index
                    .module_event_by_context_name(search_key, query.name)
                    .map_err(|source| self.source_failure(source))?
                    .into_iter()
                    .filter(|hit| hit.document.kind == SearchDocumentKind::ModuleEvent)
                    .filter_map(|hit| {
                        self.map_module_event(hit, &self.module_context_id(query.module_kind))
                            .transpose()
                    })
                    .collect::<Result<Vec<_>, ResolveError>>()?
                    .into_iter()
                    .map(ResolvedBslContextMember::Callable)
                    .collect()
            }
            MemberQueryKind::EnumValue => {
                return Ok(ResolveResponse::unsupported(
                    "platform module context does not expose enum-value members",
                ));
            }
        };
        Ok(response_from_bsl_context_members(
            facts,
            "platform module member not found",
        ))
    }

    fn module_context_members(
        &self,
        query: ModuleContextMembersLookup,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedBslContextMember>, ResolveError> {
        if !context.is_source_active(&self.source_id)
            || query.language != GlobalContextLanguage::Bsl
            || query.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found(
                "platform source does not expose requested module members",
            ));
        }
        let Some(search_key) =
            crate::hbk_catalogs::bsl::bsl_module_context_key(query.module_kind)
        else {
            return Ok(ResolveResponse::unsupported(format!(
                "platform module context `{}` is not provider-backed by the HBK search index",
                query.module_kind.as_str()
            )));
        };

        let mut facts = Vec::new();
        facts.extend(
            self.index
                .documents_by_kind(SearchDocumentKind::GlobalProperty)
                .map_err(|source| self.source_failure(source))?
                .into_iter()
                .map(|hit| ResolvedBslContextMember::Property(self.map_global_property(hit.document))),
        );
        facts.extend(
            self.index
                .documents_by_kind(SearchDocumentKind::GlobalMethod)
                .map_err(|source| self.source_failure(source))?
                .into_iter()
                .filter_map(|hit| self.map_callable(hit).transpose())
                .collect::<Result<Vec<_>, ResolveError>>()?
                .into_iter()
                .map(ResolvedBslContextMember::Callable),
        );
        facts.extend(
            self.index
                .get_by_name(search_key)
                .map_err(|source| self.source_failure(source))?
                .into_iter()
                .filter(|hit| hit.document.kind == SearchDocumentKind::ModuleEvent)
                .filter_map(|hit| {
                    self.map_module_event(hit, &self.module_context_id(query.module_kind))
                        .transpose()
                })
                .collect::<Result<Vec<_>, ResolveError>>()?
                .into_iter()
                .map(ResolvedBslContextMember::Callable),
        );
        if facts.is_empty() {
            return Ok(ResolveResponse::not_found(
                "platform module members not found",
            ));
        }
        Ok(ResolveResponse::ok(facts))
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
