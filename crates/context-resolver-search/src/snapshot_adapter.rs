impl PlatformSnapshotSource {
    pub fn new(snapshot: Arc<HbkFactSnapshot>) -> Self {
        Self::with_source_id(snapshot, SourceId::new(DEFAULT_SOURCE_ID))
    }

    pub fn with_source_id(snapshot: Arc<HbkFactSnapshot>, source_id: SourceId) -> Self {
        Self {
            source_id,
            snapshot,
        }
    }

    pub fn from_index(index: &SearchIndex, source_id: SourceId) -> Result<Self, ResolveError> {
        HbkFactSnapshot::from_index(index)
            .map(|snapshot| Self::with_source_id(Arc::new(snapshot), source_id.clone()))
            .map_err(|source| search_source_failure(&source_id, source))
    }

    pub fn open_read_only_with_source_id(
        path: impl AsRef<Path>,
        source_id: SourceId,
    ) -> Result<Self, ResolveError> {
        HbkFactSnapshot::from_path(path)
            .map(|snapshot| Self::with_source_id(Arc::new(snapshot), source_id.clone()))
            .map_err(|source| search_source_failure(&source_id, source))
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

    fn source_matches(&self, source: Option<&SourceId>) -> bool {
        source.is_none_or(|source| source == &self.source_id)
    }

    fn domain_matches(&self, domain: Option<LanguageDomain>) -> bool {
        domain.is_none_or(|domain| domain == LanguageDomain::PlatformApi)
    }

    fn platform_type_from_id(&self, id: HbkPlatformTypeId) -> ResolvedType {
        let fact = self.map_platform_type(id);
        let info = match &fact.details {
            FactDetails::Type(info) => info.clone(),
            _ => unreachable!("platform type maps to type details"),
        };
        ResolvedType {
            id: TypeId(fact.id.clone()),
            fact,
            info,
        }
    }

    fn map_platform_type(&self, id: HbkPlatformTypeId) -> ContextFact {
        let fact = self.snapshot.platform_type(id);
        let local_id = self.snapshot.string(fact.id).to_string();
        let info = TypeInfo {
            description: None,
            metadata_template: fact.metadata_template.as_ref().map(|template| {
                MetadataTemplateInfo {
                    metadata_kind: self.snapshot.string(template.metadata_kind).to_string(),
                    parameters: template
                        .template_parameters
                        .iter()
                        .map(|parameter| self.snapshot.string(*parameter).to_string())
                        .collect(),
                }
            }),
            type_template_key: fact
                .type_template_key
                .map(|key| self.map_type_template_key(key)),
        };
        ContextFact {
            id: self.fact_id(FactKind::Type, local_id),
            name: self.map_name(&fact.name),
            owner: None,
            details: FactDetails::Type(info),
            relations: Vec::new(),
        }
    }

    fn map_member(&self, id: HbkTypeMemberId) -> ResolvedMember {
        let member = self.snapshot.type_member(id);
        let owner = self.type_id(
            self.snapshot
                .string(self.snapshot.platform_type(member.owner).id),
        );
        let info = MemberInfo {
            kind: member_kind_from_snapshot(member.kind),
            types: self.map_type_refs(&member.type_refs),
            description: None,
        };
        let member_id = MemberId(self.fact_id(FactKind::Member, self.snapshot.string(member.id)));
        let fact = ContextFact {
            id: member_id.0.clone(),
            name: self.map_name(&member.name),
            owner: Some(owner.0.clone()),
            details: FactDetails::Member(info.clone()),
            relations: Vec::new(),
        };
        ResolvedMember {
            id: member_id,
            owner,
            fact,
            info,
        }
    }

    fn map_callable(&self, id: HbkCallableId) -> ResolvedCallable {
        let callable = self.snapshot.callable(id);
        let kind = callable_kind_from_snapshot(callable.kind);
        let owner = callable
            .owner
            .map(|owner| self.type_id(self.snapshot.string(self.snapshot.platform_type(owner).id)));
        let info = CallableInfo {
            kind,
            signatures: self.map_signatures(&callable.signatures),
            return_types: self.map_type_refs(&callable.return_type_refs),
            description: None,
        };
        let fact_kind = if matches!(kind, CallableKind::Constructor) {
            FactKind::Constructor
        } else {
            FactKind::Callable
        };
        let id = CallableId(self.fact_id(fact_kind, self.snapshot.string(callable.id)));
        let fact = ContextFact {
            id: id.0.clone(),
            name: self.map_name(&callable.name),
            owner: owner.as_ref().map(|owner| owner.0.clone()),
            details: FactDetails::Callable(info.clone()),
            relations: Vec::new(),
        };
        ResolvedCallable {
            id,
            owner,
            fact,
            info,
        }
    }

    fn map_global_property(&self, id: HbkGlobalFactId) -> ContextFact {
        let global = self.snapshot.global_fact(id);
        let info = MemberInfo {
            kind: MemberKind::Property,
            types: self.map_type_refs(&global.type_refs),
            description: None,
        };
        ContextFact {
            id: self.fact_id(FactKind::Global, self.snapshot.string(global.id)),
            name: self.map_name(&global.name),
            owner: None,
            details: FactDetails::Member(info),
            relations: Vec::new(),
        }
    }

    fn map_fact_ref_for_relation(
        &self,
        fact: HbkFactRef,
        relation: RelationKind,
    ) -> Option<ContextFact> {
        match fact {
            HbkFactRef::PlatformType(id) => Some(self.map_platform_type(id)),
            HbkFactRef::Enum(id)
                if matches!(
                    relation,
                    RelationKind::HasType | RelationKind::Returns | RelationKind::Constructs
                ) =>
            {
                Some(self.map_enum_as_type(id))
            }
            HbkFactRef::Enum(id) => Some(self.map_enum(id)),
            HbkFactRef::EnumValue(id) => Some(self.map_enum_value(id)),
            HbkFactRef::TypeMember(id) => Some(self.map_member(id).fact),
            HbkFactRef::Callable(id) => Some(self.map_callable(id).fact),
            HbkFactRef::Global(id) => Some(self.map_global_property(id)),
            _ => None,
        }
    }

    fn map_context_fact_for_requested(
        &self,
        fact: HbkFactRef,
        requested: FactKind,
    ) -> Option<ContextFact> {
        match (fact, requested) {
            (HbkFactRef::PlatformType(id), FactKind::Type) => Some(self.map_platform_type(id)),
            (HbkFactRef::Enum(id), FactKind::Type) => Some(self.map_enum_as_type(id)),
            (HbkFactRef::Enum(id), FactKind::Enum) => Some(self.map_enum(id)),
            (HbkFactRef::EnumValue(id), FactKind::EnumValue) => Some(self.map_enum_value(id)),
            (HbkFactRef::TypeMember(id), FactKind::Member) => Some(self.map_member(id).fact),
            (HbkFactRef::Global(id), FactKind::Global) => Some(self.map_global_property(id)),
            (HbkFactRef::Callable(id), FactKind::Constructor)
                if self.snapshot.callable(id).kind == HbkCallableKind::Constructor =>
            {
                Some(self.map_callable(id).fact)
            }
            (HbkFactRef::Callable(id), FactKind::Callable)
                if self.snapshot.callable(id).kind != HbkCallableKind::Constructor =>
            {
                Some(self.map_callable(id).fact)
            }
            (HbkFactRef::Callable(id), FactKind::Member)
                if self.snapshot.callable(id).kind == HbkCallableKind::Event =>
            {
                Some(self.map_event_as_member(id).fact)
            }
            _ => None,
        }
    }

    fn map_event_as_member(&self, id: HbkCallableId) -> ResolvedMember {
        let callable = self.snapshot.callable(id);
        let owner = callable
            .owner
            .expect("type event snapshot callable must have an owner");
        let owner = self.type_id(self.snapshot.string(self.snapshot.platform_type(owner).id));
        let info = MemberInfo {
            kind: MemberKind::Event,
            types: Vec::new(),
            description: None,
        };
        let member_id = MemberId(self.fact_id(FactKind::Member, self.snapshot.string(callable.id)));
        let fact = ContextFact {
            id: member_id.0.clone(),
            name: self.map_name(&callable.name),
            owner: Some(owner.0.clone()),
            details: FactDetails::Member(info.clone()),
            relations: Vec::new(),
        };
        ResolvedMember {
            id: member_id,
            owner,
            fact,
            info,
        }
    }

    fn map_enum_as_type(&self, id: HbkEnumId) -> ContextFact {
        let fact = self.snapshot.enum_fact(id);
        ContextFact {
            id: self.fact_id(FactKind::Type, self.snapshot.string(fact.id)),
            name: self.map_name(&fact.name),
            owner: None,
            details: FactDetails::Type(TypeInfo {
                description: None,
                metadata_template: None,
                type_template_key: None,
            }),
            relations: Vec::new(),
        }
    }

    fn map_enum(&self, id: HbkEnumId) -> ContextFact {
        let fact = self.snapshot.enum_fact(id);
        ContextFact {
            id: self.fact_id(FactKind::Enum, self.snapshot.string(fact.id)),
            name: self.map_name(&fact.name),
            owner: None,
            details: FactDetails::Enum,
            relations: Vec::new(),
        }
    }

    fn map_enum_value(&self, id: HbkEnumValueId) -> ContextFact {
        let fact = self.snapshot.enum_value(id);
        let owner = self.snapshot.enum_fact(fact.owner);
        ContextFact {
            id: self.fact_id(FactKind::EnumValue, self.snapshot.string(fact.id)),
            name: self.map_name(&fact.name),
            owner: Some(self.fact_id(FactKind::Enum, self.snapshot.string(owner.id))),
            details: FactDetails::EnumValue,
            relations: Vec::new(),
        }
    }

    fn map_name(&self, name: &HbkName) -> Name {
        Name::new(
            self.snapshot.string(name.primary).to_string(),
            name.alias
                .map(|alias| self.snapshot.string(alias).to_string()),
        )
    }

    fn map_type_refs(&self, refs: &[HbkTypeRef]) -> Vec<TypeRef> {
        refs.iter()
            .map(|type_ref| self.map_type_ref(type_ref))
            .collect()
    }

    fn map_type_ref(&self, type_ref: &HbkTypeRef) -> TypeRef {
        TypeRef {
            name: self.snapshot.string(type_ref.name).to_string(),
            target: self.map_type_ref_target(&type_ref.target),
            template_binding: type_ref.template_binding.as_ref().map(|binding| {
                TypeTemplateBinding {
                    template_key: self.map_type_template_key(binding.template_key),
                    arguments: binding
                        .arguments
                        .iter()
                        .map(|argument| {
                            match argument {
                            syntax_helper_search::model::TemplateParameterBinding::OwnerParameter {
                                owner_parameter_index,
                                target_parameter_index,
                            } => TemplateParameterBinding::OwnerParameter {
                                owner_parameter_index: *owner_parameter_index,
                                target_parameter_index: *target_parameter_index,
                            },
                        }
                        })
                        .collect(),
                }
            }),
        }
    }

    fn map_type_ref_target(&self, target: &HbkTypeRefTarget) -> TypeRefTarget {
        match target {
            HbkTypeRefTarget::Ok(id) => TypeRefTarget::Ok(self.type_id(self.snapshot.string(*id))),
            HbkTypeRefTarget::Unresolved => TypeRefTarget::Unresolved,
            HbkTypeRefTarget::Ambiguous(candidates) => TypeRefTarget::Ambiguous(
                candidates
                    .iter()
                    .map(|id| self.type_id(self.snapshot.string(*id)))
                    .collect(),
            ),
        }
    }

    fn map_type_template_key(
        &self,
        key: syntax_helper_search::HbkPlatformTypeTemplateKey,
    ) -> PlatformTypeTemplateKey {
        PlatformTypeTemplateKey::new(
            self.snapshot.string(key.family).to_string(),
            self.snapshot.string(key.variant).to_string(),
        )
    }

    fn map_signatures(&self, signatures: &[syntax_helper_search::HbkSignature]) -> Vec<Signature> {
        signatures
            .iter()
            .map(|signature| Signature {
                parameters: signature
                    .parameters
                    .iter()
                    .map(|parameter| Parameter {
                        name: self.snapshot.string(parameter.name).to_string(),
                        required: parameter.required,
                        types: self.map_type_refs(&parameter.type_refs),
                        description: None,
                    })
                    .collect(),
                return_types: self.map_type_refs(&signature.return_type_refs),
                title: Some(self.snapshot.string(signature.text).to_string()),
                description: None,
            })
            .collect()
    }

    fn map_availability(&self, id: &FactId, fact: HbkFactRef) -> AvailabilityFact {
        AvailabilityFact {
            id: id.clone(),
            availability: AvailabilityInfo {
                contexts: self
                    .snapshot
                    .worker_handle()
                    .availability_contexts(fact)
                    .iter()
                    .filter_map(|context| {
                        availability_context_from_code(self.snapshot.string(*context))
                    })
                    .collect(),
                since: self
                    .snapshot
                    .worker_handle()
                    .available_since(fact)
                    .map(|since| self.snapshot.string(since).to_string()),
            },
        }
    }
}

impl ContextSource for PlatformSnapshotSource {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            id: self.source_id.clone(),
            domain: LanguageDomain::PlatformApi,
            label: "Syntax Assistant platform fact snapshot".to_string(),
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
            module_context: true,
        }
    }

    fn resolve(
        &self,
        query: context_resolver_core::ResolveQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found(
                "platform snapshot source is not active",
            ));
        }
        let handle = self.snapshot.worker_handle();
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
                    let response = self.module_context(
                        ModuleContextQuery {
                            language: GlobalContextLanguage::Bsl,
                            domain: LanguageDomain::PlatformApi,
                            kind,
                            sources: &[],
                        },
                        context,
                    )?;
                    return if response.status == ResolveStatus::Ok {
                        Ok(ResolveResponse::ok(vec![self.module_context_fact(kind)]))
                    } else {
                        Ok(ResolveResponse {
                            status: response.status,
                            facts: Vec::new(),
                            candidates: response.candidates,
                            diagnostics: response.diagnostics,
                        })
                    };
                }
                let facts = handle
                    .facts_by_id(&id.local_id)
                    .into_iter()
                    .filter_map(|fact| self.map_context_fact_for_requested(fact, id.kind))
                    .collect::<Vec<_>>();
                Ok(response_from_facts(facts, "platform fact not found"))
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
                        "platform snapshot adapter supports exact-name resolver lookup for types only",
                    ));
                }
                let facts = handle
                    .platform_types_by_name(name)
                    .into_iter()
                    .map(|id| self.map_platform_type(id))
                    .chain(
                        handle
                            .enums_by_name(name)
                            .into_iter()
                            .map(|id| self.map_enum_as_type(id)),
                    )
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
            return Ok(ResolveResponse::not_found(
                "platform snapshot source is not active",
            ));
        }
        let handle = self.snapshot.worker_handle();
        let facts = match query {
            TypeLookup::Id(id) => {
                if id.0.source != self.source_id || id.0.domain != LanguageDomain::PlatformApi {
                    Vec::new()
                } else {
                    let mut facts = Vec::new();
                    if let Some(id) = handle.platform_type_by_id(&id.0.local_id) {
                        facts.push(self.platform_type_from_id(id));
                    } else if let Some(id) = handle.enum_by_id(&id.0.local_id) {
                        let fact = self.map_enum_as_type(id);
                        let FactDetails::Type(info) = fact.details.clone() else {
                            unreachable!("enum-as-type maps to type info");
                        };
                        facts.push(ResolvedType {
                            id: TypeId(fact.id.clone()),
                            fact,
                            info,
                        });
                    }
                    facts
                }
            }
            TypeLookup::ExactName {
                source,
                domain,
                name,
            }
            | TypeLookup::ExactAlias {
                source,
                domain,
                alias: name,
            } => {
                if !self.source_matches(source) || !self.domain_matches(domain) {
                    Vec::new()
                } else {
                    handle
                        .platform_types_by_name(name)
                        .into_iter()
                        .map(|id| self.platform_type_from_id(id))
                        .chain(handle.enums_by_name(name).into_iter().map(|id| {
                            let fact = self.map_enum_as_type(id);
                            let FactDetails::Type(info) = fact.details.clone() else {
                                unreachable!("enum-as-type maps to type info");
                            };
                            ResolvedType {
                                id: TypeId(fact.id.clone()),
                                fact,
                                info,
                            }
                        }))
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
                    handle
                        .platform_types_by_template_key(&key.family, &key.variant)
                        .into_iter()
                        .map(|id| self.platform_type_from_id(id))
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
        let handle = self.snapshot.worker_handle();
        let Some(owner_id) = handle.platform_type_by_id(&owner.0.local_id) else {
            return Ok(ResolveResponse::not_found("platform owner type not found"));
        };
        let member_kind = query.kind.map(member_query_kind_to_snapshot);
        let ids = match query.name {
            Some(name) => handle
                .member_by_owner_name_kind(owner_id, name, member_kind)
                .collect::<Vec<_>>(),
            None => handle.members_of_type(owner_id).to_vec(),
        };
        let facts = ids
            .into_iter()
            .filter(|id| member_kind.is_none_or(|kind| self.snapshot.type_member(*id).kind == kind))
            .map(|id| self.map_member(id))
            .collect::<Vec<_>>();
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
            return Ok(ResolveResponse::not_found(
                "platform snapshot source is not active",
            ));
        }
        let handle = self.snapshot.worker_handle();
        let facts = match query {
            CallableLookup::Id(id) => {
                if id.0.source != self.source_id || id.0.domain != LanguageDomain::PlatformApi {
                    Vec::new()
                } else {
                    handle
                        .facts_by_id(&id.0.local_id)
                        .into_iter()
                        .filter_map(|fact| match fact {
                            HbkFactRef::Callable(id) => Some(self.map_callable(id)),
                            _ => None,
                        })
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
                        let Some(owner_id) = handle.platform_type_by_id(&owner.0.local_id) else {
                            return Ok(ResolveResponse::not_found("platform callable not found"));
                        };
                        let mut ids = handle
                            .callable_by_owner_name(owner_id, name)
                            .collect::<Vec<_>>();
                        ids.extend(
                            handle
                                .constructors_of_type(owner_id)
                                .iter()
                                .copied()
                                .filter(|id| {
                                    name_matches(
                                        &self.map_name(&self.snapshot.callable(*id).name),
                                        name,
                                    )
                                }),
                        );
                        ids.sort_unstable();
                        ids.dedup();
                        ids.into_iter().map(|id| self.map_callable(id)).collect()
                    }
                } else {
                    handle
                        .globals_by_domain_name_kind(
                            HbkLanguageDomain::Bsl,
                            name,
                            Some(HbkGlobalFactKind::Method),
                        )
                        .into_iter()
                        .filter_map(|id| self.snapshot.global_fact(id).callable)
                        .map(|id| self.map_callable(id))
                        .collect()
                }
            }
        };
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
            return Ok(ResolveResponse::not_found(
                "platform snapshot source is not active",
            ));
        }
        let GlobalContextQuery::Language { language, sources } = query;
        if language != GlobalContextLanguage::Bsl
            || (!sources.is_empty() && !sources.iter().any(|source| source == &self.source_id))
        {
            return Ok(ResolveResponse::not_found(
                "platform source does not expose requested global context",
            ));
        }
        let handle = self.snapshot.worker_handle();
        let mut methods = Vec::new();
        let mut properties = Vec::new();
        for id in handle.global_fact_ids() {
            let global = self.snapshot.global_fact(id);
            match global.kind {
                HbkGlobalFactKind::Method => {
                    if let Some(callable) = global.callable {
                        methods.push(self.map_callable(callable));
                    }
                }
                HbkGlobalFactKind::Property => properties.push(self.map_global_property(id)),
            }
        }
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
            return Ok(ResolveResponse::not_found(
                "platform snapshot source is not active",
            ));
        }
        if query.language != GlobalContextLanguage::Bsl
            || query.domain != LanguageDomain::PlatformApi
            || (!query.sources.is_empty()
                && !query.sources.iter().any(|source| source == &self.source_id))
        {
            return Ok(ResolveResponse::not_found(
                "platform source does not expose requested module context",
            ));
        }
        let Some(search_key) = search_module_context_relation_key(query.kind) else {
            return Ok(ResolveResponse::unsupported(format!(
                "platform module context `{}` is not provider-backed by the HBK search index",
                query.kind.as_str()
            )));
        };
        let handle = self.snapshot.worker_handle();
        let context_id = self.module_context_id(query.kind);
        let events = handle
            .module_context_events(
                HbkLanguageDomain::Bsl,
                "bsl",
                search_key.trim_start_matches("module_context:"),
            )
            .into_iter()
            .map(|id| {
                let mut callable = self.map_callable(id);
                callable.fact.owner = Some(context_id.clone());
                callable
            })
            .collect::<Vec<_>>();
        let globals = self.global_context(
            GlobalContextQuery::Language {
                language: GlobalContextLanguage::Bsl,
                sources: &[],
            },
            context,
        )?;
        let Some(global) = globals.facts.into_iter().next() else {
            return Ok(ResolveResponse::not_found(
                "platform module context facts not found",
            ));
        };
        if global.methods.is_empty() && global.properties.is_empty() && events.is_empty() {
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
            properties: global.properties,
            methods: global.methods,
            events,
            facts: vec![self.module_context_fact(query.kind)],
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
            || source.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found("platform source fact not found"));
        }
        let Some(edge) = edge_from_relation_kind(kind) else {
            return Ok(ResolveResponse::unsupported(
                "platform adapter supports has_type, returns, constructs and member_of",
            ));
        };
        let handle = self.snapshot.worker_handle();
        let Some(source_ref) = handle
            .facts_by_id(&source.local_id)
            .into_iter()
            .find(|fact| {
                self.map_context_fact_for_requested(*fact, source.kind)
                    .is_some()
            })
        else {
            return Ok(ResolveResponse::not_found("platform source fact not found"));
        };
        let facts = handle
            .relations_by_source_kind(source_ref, edge)
            .iter()
            .filter_map(|target| {
                let mut fact = self.map_fact_ref_for_relation(*target, kind)?;
                fact.relations.push(FactRelation {
                    kind,
                    target: fact.id.clone(),
                    evidence: None,
                });
                Some(fact)
            })
            .collect();
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
        let handle = self.snapshot.worker_handle();
        let Some(fact) = handle
            .facts_by_id(&source.local_id)
            .into_iter()
            .find(|fact| {
                self.map_context_fact_for_requested(*fact, source.kind)
                    .is_some()
            })
        else {
            return Ok(ResolveResponse::not_found("platform fact not found"));
        };
        Ok(ResolveResponse::ok(vec![
            self.map_availability(source, fact)
        ]))
    }
}

impl QueryTableSnapshotSource {
    pub fn new(snapshot: Arc<HbkFactSnapshot>) -> Self {
        Self::with_source_ids(
            snapshot,
            SourceId::new("shcntx-query"),
            SourceId::new(DEFAULT_SOURCE_ID),
        )
    }

    pub fn with_source_ids(
        snapshot: Arc<HbkFactSnapshot>,
        source_id: SourceId,
        platform_source_id: SourceId,
    ) -> Self {
        Self {
            source_id,
            platform_source_id,
            snapshot,
        }
    }

    pub fn from_index(
        index: &SearchIndex,
        source_id: SourceId,
        platform_source_id: SourceId,
    ) -> Result<Self, ResolveError> {
        HbkFactSnapshot::from_index(index)
            .map(|snapshot| {
                Self::with_source_ids(Arc::new(snapshot), source_id.clone(), platform_source_id)
            })
            .map_err(|source| search_source_failure(&source_id, source))
    }

    pub fn query_fields_by_name(
        &self,
        table: &FactId,
        name: &str,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        if !context.is_source_active(&self.source_id)
            || table.source != self.source_id
            || table.domain != LanguageDomain::QueryLanguage
            || table.kind != FactKind::QueryTable
        {
            return Ok(ResolveResponse::not_found("query table fact not found"));
        }
        let handle = self.snapshot.worker_handle();
        let Some(table_id) = handle.query_table_by_id(&table.local_id) else {
            return Ok(ResolveResponse::not_found("query table fact not found"));
        };
        let facts = handle
            .query_fields_by_name(table_id, name)
            .into_iter()
            .map(|id| self.map_query_field(id))
            .collect::<Vec<_>>();
        Ok(response_from_facts(facts, "query field fact not found"))
    }

    pub fn query_parameters_by_name(
        &self,
        table: &FactId,
        name: &str,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        if !context.is_source_active(&self.source_id)
            || table.source != self.source_id
            || table.domain != LanguageDomain::QueryLanguage
            || table.kind != FactKind::QueryTable
        {
            return Ok(ResolveResponse::not_found("query table fact not found"));
        }
        let handle = self.snapshot.worker_handle();
        let Some(table_id) = handle.query_table_by_id(&table.local_id) else {
            return Ok(ResolveResponse::not_found("query table fact not found"));
        };
        let facts = handle
            .query_parameters_by_name(table_id, name)
            .into_iter()
            .map(|id| self.map_query_parameter(id))
            .collect::<Vec<_>>();
        Ok(response_from_facts(facts, "query parameter fact not found"))
    }

    fn fact_id(&self, kind: FactKind, local_id: impl Into<String>) -> FactId {
        FactId::new(
            self.source_id.clone(),
            LanguageDomain::QueryLanguage,
            kind,
            local_id,
        )
    }

    fn platform_type_id(&self, local_id: impl Into<String>) -> TypeId {
        TypeId(FactId::new(
            self.platform_source_id.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            local_id,
        ))
    }

    fn source_matches(&self, source: Option<&SourceId>) -> bool {
        source.is_none_or(|source| source == &self.source_id)
    }

    fn domain_matches(&self, domain: Option<LanguageDomain>) -> bool {
        domain.is_none_or(|domain| domain == LanguageDomain::QueryLanguage)
    }

    fn map_query_table(&self, id: HbkQueryTableId) -> ContextFact {
        let table = self.snapshot.query_table(id);
        ContextFact {
            id: self.fact_id(FactKind::QueryTable, self.snapshot.string(table.id)),
            name: self.map_name(&table.name),
            owner: None,
            details: FactDetails::QueryTable(QueryTableInfo {
                syntax: table.syntax.as_ref().map(|name| self.map_name(name)),
                identifier: table
                    .identifier
                    .map(|id| self.snapshot.string(id).to_string()),
                table_role: query_table_role(table.role),
                owner_path: table
                    .owner_path
                    .iter()
                    .map(|name| self.map_name(name))
                    .collect(),
                template_parameters: table
                    .template_parameters
                    .iter()
                    .map(|id| self.snapshot.string(*id).to_string())
                    .collect(),
                description: None,
                source: self.snapshot.source_locale().map(|locale| FactProvenance {
                    source: self.source_id.clone(),
                    evidence_id: self.snapshot.string(table.id).to_string(),
                    locale: Some(locale.to_string()),
                }),
            }),
            relations: Vec::new(),
        }
    }

    fn map_query_field(&self, id: HbkQueryFieldId) -> ContextFact {
        let field = self.snapshot.query_field(id);
        let owner = self.fact_id(
            FactKind::QueryTable,
            self.snapshot
                .string(self.snapshot.query_table(field.owner).id),
        );
        ContextFact {
            id: self.fact_id(FactKind::QueryField, self.snapshot.string(field.id)),
            name: self.map_name(&field.name),
            owner: Some(owner.clone()),
            details: FactDetails::QueryField(QueryFieldInfo {
                owner,
                types: self.map_type_refs(&field.type_refs),
                description: None,
                note: field
                    .note
                    .map(|note| self.snapshot.string(note).to_string()),
                source: self.snapshot.source_locale().map(|locale| FactProvenance {
                    source: self.source_id.clone(),
                    evidence_id: self.snapshot.string(field.id).to_string(),
                    locale: Some(locale.to_string()),
                }),
            }),
            relations: Vec::new(),
        }
    }

    fn map_query_parameter(&self, id: HbkQueryParameterId) -> ContextFact {
        let parameter = self.snapshot.query_parameter(id);
        let owner = self.fact_id(
            FactKind::QueryTable,
            self.snapshot
                .string(self.snapshot.query_table(parameter.owner).id),
        );
        ContextFact {
            id: self.fact_id(FactKind::QueryParameter, self.snapshot.string(parameter.id)),
            name: self.map_name(&parameter.name),
            owner: Some(owner.clone()),
            details: FactDetails::QueryParameter(QueryParameterInfo {
                owner,
                types: self.map_type_refs(&parameter.type_refs),
                description: None,
                default_value: parameter
                    .default_value
                    .map(|value| self.snapshot.string(value).to_string()),
                source: self.snapshot.source_locale().map(|locale| FactProvenance {
                    source: self.source_id.clone(),
                    evidence_id: self.snapshot.string(parameter.id).to_string(),
                    locale: Some(locale.to_string()),
                }),
            }),
            relations: Vec::new(),
        }
    }

    fn map_context_fact_for_requested(
        &self,
        fact: HbkFactRef,
        requested: FactKind,
    ) -> Option<ContextFact> {
        match (fact, requested) {
            (HbkFactRef::QueryTable(id), FactKind::QueryTable) => Some(self.map_query_table(id)),
            (HbkFactRef::QueryField(id), FactKind::QueryField) => Some(self.map_query_field(id)),
            (HbkFactRef::QueryParameter(id), FactKind::QueryParameter) => {
                Some(self.map_query_parameter(id))
            }
            _ => None,
        }
    }

    fn map_fact_ref_for_relation(&self, fact: HbkFactRef) -> Option<ContextFact> {
        match fact {
            HbkFactRef::QueryTable(id) => Some(self.map_query_table(id)),
            HbkFactRef::QueryField(id) => Some(self.map_query_field(id)),
            HbkFactRef::QueryParameter(id) => Some(self.map_query_parameter(id)),
            HbkFactRef::PlatformType(id) => Some(self.map_platform_type_for_relation(id)),
            HbkFactRef::Enum(id) => {
                let fact = self.snapshot.enum_fact(id);
                Some(ContextFact {
                    id: self.platform_type_id(self.snapshot.string(fact.id)).0,
                    name: self.map_name(&fact.name),
                    owner: None,
                    details: FactDetails::Type(TypeInfo {
                        description: None,
                        metadata_template: None,
                        type_template_key: None,
                    }),
                    relations: Vec::new(),
                })
            }
            _ => None,
        }
    }

    fn map_platform_type_for_relation(&self, id: HbkPlatformTypeId) -> ContextFact {
        let fact = self.snapshot.platform_type(id);
        ContextFact {
            id: self.platform_type_id(self.snapshot.string(fact.id)).0,
            name: self.map_name(&fact.name),
            owner: None,
            details: FactDetails::Type(TypeInfo {
                description: None,
                metadata_template: fact.metadata_template.as_ref().map(|template| {
                    MetadataTemplateInfo {
                        metadata_kind: self.snapshot.string(template.metadata_kind).to_string(),
                        parameters: template
                            .template_parameters
                            .iter()
                            .map(|parameter| self.snapshot.string(*parameter).to_string())
                            .collect(),
                    }
                }),
                type_template_key: fact
                    .type_template_key
                    .map(|key| self.map_type_template_key(key)),
            }),
            relations: Vec::new(),
        }
    }

    fn map_name(&self, name: &HbkName) -> Name {
        Name::new(
            self.snapshot.string(name.primary).to_string(),
            name.alias
                .map(|alias| self.snapshot.string(alias).to_string()),
        )
    }

    fn map_type_template_key(
        &self,
        key: syntax_helper_search::HbkPlatformTypeTemplateKey,
    ) -> PlatformTypeTemplateKey {
        PlatformTypeTemplateKey::new(
            self.snapshot.string(key.family).to_string(),
            self.snapshot.string(key.variant).to_string(),
        )
    }

    fn map_type_refs(&self, refs: &[HbkTypeRef]) -> Vec<TypeRef> {
        refs.iter()
            .map(|type_ref| self.map_type_ref(type_ref))
            .collect()
    }

    fn map_type_ref(&self, type_ref: &HbkTypeRef) -> TypeRef {
        TypeRef {
            name: self.snapshot.string(type_ref.name).to_string(),
            target: match &type_ref.target {
                HbkTypeRefTarget::Ok(id) => {
                    TypeRefTarget::Ok(self.platform_type_id(self.snapshot.string(*id)))
                }
                HbkTypeRefTarget::Unresolved => TypeRefTarget::Unresolved,
                HbkTypeRefTarget::Ambiguous(candidates) => TypeRefTarget::Ambiguous(
                    candidates
                        .iter()
                        .map(|id| self.platform_type_id(self.snapshot.string(*id)))
                        .collect(),
                ),
            },
            template_binding: type_ref.template_binding.as_ref().map(|binding| {
                TypeTemplateBinding {
                    template_key: self.map_type_template_key(binding.template_key),
                    arguments: binding
                        .arguments
                        .iter()
                        .map(|argument| {
                            match argument {
                            syntax_helper_search::model::TemplateParameterBinding::OwnerParameter {
                                owner_parameter_index,
                                target_parameter_index,
                            } => TemplateParameterBinding::OwnerParameter {
                                owner_parameter_index: *owner_parameter_index,
                                target_parameter_index: *target_parameter_index,
                            },
                        }
                        })
                        .collect(),
                }
            }),
        }
    }
}

impl ContextSource for QueryTableSnapshotSource {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            id: self.source_id.clone(),
            domain: LanguageDomain::QueryLanguage,
            label: format!("Syntax Assistant query table snapshot {}", self.source_id),
        }
    }

    fn capabilities(&self) -> SourceCapabilities {
        SourceCapabilities {
            exact_lookup: true,
            type_lookup: false,
            members: false,
            callables: false,
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
            return Ok(ResolveResponse::not_found(
                "query table snapshot source is not active",
            ));
        }
        let handle = self.snapshot.worker_handle();
        match query {
            context_resolver_core::ResolveQuery::Id(id) => {
                if id.source != self.source_id || id.domain != LanguageDomain::QueryLanguage {
                    return Ok(ResolveResponse::not_found(
                        "fact source or domain does not match",
                    ));
                }
                let facts = handle
                    .facts_by_id(&id.local_id)
                    .into_iter()
                    .filter_map(|fact| self.map_context_fact_for_requested(fact, id.kind))
                    .collect::<Vec<_>>();
                Ok(response_from_facts(facts, "query table fact not found"))
            }
            context_resolver_core::ResolveQuery::ExactName {
                source,
                domain,
                kind,
                name,
            } => {
                if !self.source_matches(source) || !self.domain_matches(domain) {
                    return Ok(ResolveResponse::not_found(
                        "query table source or domain does not match",
                    ));
                }
                if !matches!(kind, None | Some(FactKind::QueryTable)) {
                    return Ok(ResolveResponse::not_found("query table fact not found"));
                }
                let mut ids = handle.query_tables_by_name(name).collect::<Vec<_>>();
                ids.extend(handle.query_tables_by_identifier(name));
                ids.extend(handle.query_tables_by_syntax(name));
                ids.sort_unstable();
                ids.dedup();
                Ok(response_from_facts(
                    ids.into_iter().map(|id| self.map_query_table(id)).collect(),
                    "query table fact not found",
                ))
            }
        }
    }

    fn resolve_type(
        &self,
        _query: TypeLookup<'_>,
        _context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedType>, ResolveError> {
        Ok(ResolveResponse::unsupported(
            "query table source does not expose type lookup",
        ))
    }

    fn members(
        &self,
        _owner: &TypeId,
        _query: MemberQuery<'_>,
        _context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedMember>, ResolveError> {
        Ok(ResolveResponse::unsupported(
            "query table source does not expose members in this slice",
        ))
    }

    fn callable(
        &self,
        _query: CallableLookup<'_>,
        _context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedCallable>, ResolveError> {
        Ok(ResolveResponse::unsupported(
            "query table source does not expose callable lookup",
        ))
    }

    fn global_context(
        &self,
        query: GlobalContextQuery<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedGlobalContext>, ResolveError> {
        if !context.is_source_active(&self.source_id) {
            return Ok(ResolveResponse::not_found(
                "query table snapshot source is not active",
            ));
        }
        let GlobalContextQuery::Language { language, sources } = query;
        if language != GlobalContextLanguage::Sdbl
            || (!sources.is_empty() && !sources.iter().any(|source| source == &self.source_id))
        {
            return Ok(ResolveResponse::not_found(
                "query table source does not expose requested global context",
            ));
        }
        let handle = self.snapshot.worker_handle();
        let facts = handle
            .query_table_ids()
            .map(|id| self.map_query_table(id))
            .chain(handle.query_field_ids().map(|id| self.map_query_field(id)))
            .chain(
                handle
                    .query_parameter_ids()
                    .map(|id| self.map_query_parameter(id)),
            )
            .collect::<Vec<_>>();
        Ok(ResolveResponse::ok(vec![ResolvedGlobalContext {
            id: self.fact_id(FactKind::Global, "global_context:sdbl:query_tables"),
            language,
            sources: vec![self.source_id.clone()],
            methods: Vec::new(),
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
            || source.domain != LanguageDomain::QueryLanguage
        {
            return Ok(ResolveResponse::not_found(
                "query table source fact not found",
            ));
        }
        let edge = match kind {
            RelationKind::HasType => "has_type",
            RelationKind::MemberOf => "member_of",
            _ => {
                return Ok(ResolveResponse::unsupported(
                    "query table snapshot adapter supports has_type and member_of",
                ));
            }
        };
        let handle = self.snapshot.worker_handle();
        let Some(source_ref) = handle
            .facts_by_id(&source.local_id)
            .into_iter()
            .find(|fact| {
                self.map_context_fact_for_requested(*fact, source.kind)
                    .is_some()
            })
        else {
            return Ok(ResolveResponse::not_found(
                "query table source fact not found",
            ));
        };
        let facts = handle
            .relations_by_source_kind(source_ref, edge)
            .iter()
            .filter_map(|target| {
                let mut fact = self.map_fact_ref_for_relation(*target)?;
                fact.relations.push(FactRelation {
                    kind,
                    target: fact.id.clone(),
                    evidence: None,
                });
                Some(fact)
            })
            .collect();
        Ok(ResolveResponse::ok(facts))
    }
}

fn member_kind_from_snapshot(kind: HbkTypeMemberKind) -> MemberKind {
    match kind {
        HbkTypeMemberKind::Property => MemberKind::Property,
        HbkTypeMemberKind::Method => MemberKind::Method,
        HbkTypeMemberKind::Event => MemberKind::Event,
        HbkTypeMemberKind::EnumValue => MemberKind::EnumValue,
    }
}

fn member_query_kind_to_snapshot(kind: MemberQueryKind) -> HbkTypeMemberKind {
    match kind {
        MemberQueryKind::Property => HbkTypeMemberKind::Property,
        MemberQueryKind::Method => HbkTypeMemberKind::Method,
        MemberQueryKind::Event => HbkTypeMemberKind::Event,
        MemberQueryKind::EnumValue => HbkTypeMemberKind::EnumValue,
    }
}

fn callable_kind_from_snapshot(kind: HbkCallableKind) -> CallableKind {
    match kind {
        HbkCallableKind::Method => CallableKind::Method,
        HbkCallableKind::Constructor => CallableKind::Constructor,
        HbkCallableKind::GlobalMethod => CallableKind::GlobalMethod,
        HbkCallableKind::Event => CallableKind::Event,
        HbkCallableKind::LanguageFunction => CallableKind::GlobalMethod,
    }
}
