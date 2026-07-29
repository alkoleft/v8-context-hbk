impl PlatformSnapshotSource {
    pub fn new(snapshot: Arc<HbkFactSnapshot>) -> Self {
        Self::with_source_id(snapshot, SourceId::new(DEFAULT_SOURCE_ID))
    }

    pub fn with_source_id(snapshot: Arc<HbkFactSnapshot>, source_id: SourceId) -> Self {
        Self {
            catalog: HbkBslContextCatalog::with_source_id(snapshot, source_id),
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
        project_hbk_fact_id(&self.catalog, kind, local_id)
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
        source.is_none_or(|source| source == self.catalog.source_id())
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
        let snapshot = self.catalog.snapshot();
        let fact = snapshot.platform_type(id);
        let local_id = snapshot.string(fact.id).to_string();
        ContextFact {
            id: self.fact_id(FactKind::Type, local_id),
            name: self.map_name(&fact.name),
            owner: None,
            details: FactDetails::Type(snapshot_platform_type_info(snapshot, id)),
            relations: Vec::new(),
        }
    }

    fn map_member(&self, id: HbkTypeMemberId) -> ResolvedMember {
        let snapshot = self.catalog.snapshot();
        let member = snapshot.type_member(id);
        let owner = self.type_id(snapshot.string(snapshot.platform_type(member.owner).id));
        let info = MemberInfo {
            kind: member_kind_from_snapshot(member.kind),
            types: self.map_type_refs(&member.type_refs),
            description: None,
        };
        let member_id = MemberId(self.fact_id(FactKind::Member, snapshot.string(member.id)));
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

    fn map_enum_value_member(&self, id: HbkEnumValueId) -> ResolvedMember {
        let snapshot = self.catalog.snapshot();
        let member = snapshot.enum_value(id);
        let owner = self.type_id(snapshot.string(snapshot.enum_fact(member.owner).id));
        let info = MemberInfo {
            kind: MemberKind::EnumValue,
            types: Vec::new(),
            description: None,
        };
        let member_id = MemberId(self.fact_id(FactKind::Member, snapshot.string(member.id)));
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

    fn enum_value_members(
        &self,
        owner_id: HbkEnumId,
        query: MemberQuery<'_>,
    ) -> Vec<ResolvedMember> {
        if query
            .kind
            .is_some_and(|kind| kind != MemberQueryKind::EnumValue)
            || query.name.is_some()
        {
            return Vec::new();
        }
        self.catalog
            .snapshot()
            .worker_handle()
            .enum_values(owner_id)
            .iter()
            .copied()
            .map(|id| self.map_enum_value_member(id))
            .collect()
    }

    fn map_callable(&self, id: HbkCallableId) -> ResolvedCallable {
        let snapshot = self.catalog.snapshot();
        let callable = snapshot.callable(id);
        let kind = callable_kind_from_snapshot(callable.kind);
        let owner = callable
            .owner
            .map(|owner| self.type_id(snapshot.string(snapshot.platform_type(owner).id)));
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
        let id = CallableId(self.fact_id(fact_kind, snapshot.string(callable.id)));
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
        let snapshot = self.catalog.snapshot();
        let global = snapshot.global_fact(id);
        let info = MemberInfo {
            kind: MemberKind::Property,
            types: self.map_type_refs(&global.type_refs),
            description: None,
        };
        ContextFact {
            id: self.fact_id(FactKind::Global, snapshot.string(global.id)),
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
                if self.catalog.snapshot().callable(id).kind == HbkCallableKind::Constructor =>
            {
                Some(self.map_callable(id).fact)
            }
            (HbkFactRef::Callable(id), FactKind::Callable)
                if self.catalog.snapshot().callable(id).kind != HbkCallableKind::Constructor =>
            {
                Some(self.map_callable(id).fact)
            }
            (HbkFactRef::Callable(id), FactKind::Member)
                if self.catalog.snapshot().callable(id).kind == HbkCallableKind::Event =>
            {
                Some(self.map_event_as_member(id).fact)
            }
            _ => None,
        }
    }

    fn map_catalog_fact_for_requested(
        &self,
        local_id: &str,
        requested: FactKind,
    ) -> Option<ContextFact> {
        match requested {
            FactKind::Type => self
                .catalog
                .platform_type_by_id(local_id)
                .map(|(id, _)| self.map_platform_type(id))
                .or_else(|| {
                    let id = self
                        .catalog
                        .snapshot()
                        .worker_handle()
                        .enum_by_id(local_id)?;
                    Some(self.map_enum_as_type(id))
                }),
            FactKind::Member => self
                .catalog
                .member_by_id(local_id)
                .map(|(id, _)| self.map_member(id).fact)
                .or_else(|| {
                    let (id, callable) = self.catalog.callable_by_id(local_id)?;
                    (callable.kind == HbkCallableKind::Event)
                        .then(|| self.map_event_as_member(id).fact)
                }),
            FactKind::Global => self
                .catalog
                .global_by_id(local_id)
                .map(|(id, _)| self.map_global_property(id)),
            FactKind::Callable => {
                let (id, callable) = self.catalog.callable_by_id(local_id)?;
                (callable.kind != HbkCallableKind::Constructor).then(|| self.map_callable(id).fact)
            }
            FactKind::Constructor => {
                let (id, callable) = self.catalog.callable_by_id(local_id)?;
                (callable.kind == HbkCallableKind::Constructor).then(|| self.map_callable(id).fact)
            }
            FactKind::Enum => {
                let id = self
                    .catalog
                    .snapshot()
                    .worker_handle()
                    .enum_by_id(local_id)?;
                Some(self.map_enum(id))
            }
            FactKind::EnumValue => {
                let id = self
                    .catalog
                    .snapshot()
                    .worker_handle()
                    .enum_value_by_id(local_id)?;
                Some(self.map_enum_value(id))
            }
            _ => None,
        }
    }

    fn map_event_as_member(&self, id: HbkCallableId) -> ResolvedMember {
        let snapshot = self.catalog.snapshot();
        let callable = snapshot.callable(id);
        let owner = callable
            .owner
            .expect("type event snapshot callable must have an owner");
        let owner = self.type_id(snapshot.string(snapshot.platform_type(owner).id));
        let info = MemberInfo {
            kind: MemberKind::Event,
            types: Vec::new(),
            description: None,
        };
        let member_id = MemberId(self.fact_id(FactKind::Member, snapshot.string(callable.id)));
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
        let snapshot = self.catalog.snapshot();
        let fact = snapshot.enum_fact(id);
        ContextFact {
            id: self.fact_id(FactKind::Type, snapshot.string(fact.id)),
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
        let snapshot = self.catalog.snapshot();
        let fact = snapshot.enum_fact(id);
        ContextFact {
            id: self.fact_id(FactKind::Enum, snapshot.string(fact.id)),
            name: self.map_name(&fact.name),
            owner: None,
            details: FactDetails::Enum,
            relations: Vec::new(),
        }
    }

    fn map_enum_value(&self, id: HbkEnumValueId) -> ContextFact {
        let snapshot = self.catalog.snapshot();
        let fact = snapshot.enum_value(id);
        let owner = snapshot.enum_fact(fact.owner);
        ContextFact {
            id: self.fact_id(FactKind::EnumValue, snapshot.string(fact.id)),
            name: self.map_name(&fact.name),
            owner: Some(self.fact_id(FactKind::Enum, snapshot.string(owner.id))),
            details: FactDetails::EnumValue,
            relations: Vec::new(),
        }
    }

    fn map_name(&self, name: &HbkName) -> Name {
        let snapshot = self.catalog.snapshot();
        Name::new(
            snapshot.string(name.primary).to_string(),
            name.alias.map(|alias| snapshot.string(alias).to_string()),
        )
    }

    fn map_type_refs(&self, refs: &[HbkTypeRef]) -> Vec<TypeRef> {
        refs.iter()
            .map(|type_ref| project_hbk_type_ref(&self.catalog, type_ref))
            .collect()
    }

    fn map_signatures(&self, signatures: &[syntax_helper_search::HbkSignature]) -> Vec<Signature> {
        signatures
            .iter()
            .map(|signature| project_hbk_signature(&self.catalog, signature))
            .collect()
    }

    fn catalog_availability(&self, id: &FactId) -> Option<AvailabilityFact> {
        let fact = match id.kind {
            FactKind::Type => {
                if let Some((typed_id, _)) = self.catalog.platform_type_by_id(&id.local_id) {
                    HbkFactRef::PlatformType(typed_id)
                } else {
                    HbkFactRef::Enum(
                        self.catalog
                            .snapshot()
                            .worker_handle()
                            .enum_by_id(&id.local_id)?,
                    )
                }
            }
            FactKind::Member => {
                if let Some((typed_id, _)) = self.catalog.member_by_id(&id.local_id) {
                    HbkFactRef::TypeMember(typed_id)
                } else {
                    let (typed_id, callable) = self.catalog.callable_by_id(&id.local_id)?;
                    if callable.kind != HbkCallableKind::Event {
                        return None;
                    }
                    HbkFactRef::Callable(typed_id)
                }
            }
            FactKind::Callable | FactKind::Constructor => {
                let (typed_id, callable) = self.catalog.callable_by_id(&id.local_id)?;
                let matches_kind = match id.kind {
                    FactKind::Constructor => callable.kind == HbkCallableKind::Constructor,
                    _ => callable.kind != HbkCallableKind::Constructor,
                };
                if !matches_kind {
                    return None;
                }
                HbkFactRef::Callable(typed_id)
            }
            FactKind::Global => {
                let (typed_id, _) = self.catalog.global_by_id(&id.local_id)?;
                HbkFactRef::Global(typed_id)
            }
            FactKind::Enum => HbkFactRef::Enum(
                self.catalog
                    .snapshot()
                    .worker_handle()
                    .enum_by_id(&id.local_id)?,
            ),
            FactKind::EnumValue => HbkFactRef::EnumValue(
                self.catalog
                    .snapshot()
                    .worker_handle()
                    .enum_value_by_id(&id.local_id)?,
            ),
            _ => return None,
        };
        let (contexts, since) = self.catalog.availability(fact);
        Some(AvailabilityFact {
            id: id.clone(),
            availability: AvailabilityInfo {
                contexts: contexts.collect(),
                since: since.map(str::to_string),
            },
        })
    }
}

fn snapshot_platform_type_info(
    snapshot: &HbkFactSnapshot,
    id: HbkPlatformTypeId,
) -> TypeInfo {
    let fact = snapshot.platform_type(id);
    TypeInfo {
        description: None,
        metadata_template: fact.metadata_template.as_ref().map(|template| {
            MetadataTemplateInfo {
                metadata_kind: snapshot.string(template.metadata_kind).to_string(),
                parameters: template
                    .template_parameters
                    .iter()
                    .map(|parameter| snapshot.string(*parameter).to_string())
                    .collect(),
            }
        }),
        type_template_key: fact
            .type_template_key
            .map(|key| snapshot_type_template_key(snapshot, key)),
    }
}

fn snapshot_type_template_key(
    snapshot: &HbkFactSnapshot,
    key: syntax_helper_search::HbkPlatformTypeTemplateKey,
) -> PlatformTypeTemplateKey {
    PlatformTypeTemplateKey::new(
        snapshot.string(key.family).to_string(),
        snapshot.string(key.variant).to_string(),
    )
}

impl ContextSource for PlatformSnapshotSource {
    fn descriptor(&self) -> SourceDescriptor {
        SourceDescriptor {
            id: self.catalog.source_id().clone(),
            domain: LanguageDomain::PlatformApi,
            label: "Syntax Assistant platform fact snapshot".to_string(),
        }
    }

    fn source_id(&self) -> Option<&SourceId> {
        Some(self.catalog.source_id())
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
        if !context.is_source_active(self.catalog.source_id()) {
            return Ok(ResolveResponse::not_found(
                "platform snapshot source is not active",
            ));
        }
        match query {
            context_resolver_core::ResolveQuery::Id(id) => {
                if &id.source != self.catalog.source_id()
                    || id.domain != LanguageDomain::PlatformApi
                {
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
                let facts = self
                    .map_catalog_fact_for_requested(&id.local_id, id.kind)
                    .into_iter()
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
                let facts = self
                    .catalog
                    .platform_types_by_name(name)
                    .map(|(id, _)| self.map_platform_type(id))
                    .chain(
                        self.catalog
                            .snapshot()
                            .worker_handle()
                            .enums_by_name(name)
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
        if !context.is_source_active(self.catalog.source_id()) {
            return Ok(ResolveResponse::not_found(
                "platform snapshot source is not active",
            ));
        }
        let facts = match query {
            TypeLookup::Id(id) => {
                if &id.0.source != self.catalog.source_id()
                    || id.0.domain != LanguageDomain::PlatformApi
                {
                    Vec::new()
                } else {
                    let mut facts = Vec::new();
                    if let Some((id, _)) = self.catalog.platform_type_by_id(&id.0.local_id) {
                        facts.push(self.platform_type_from_id(id));
                    } else if let Some(id) = self
                        .catalog
                        .snapshot()
                        .worker_handle()
                        .enum_by_id(&id.0.local_id)
                    {
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
                    self.catalog
                        .platform_types_by_name(name)
                        .map(|(id, _)| self.platform_type_from_id(id))
                        .chain(self.catalog.snapshot().worker_handle().enums_by_name(name).map(|id| {
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
                    self.catalog
                        .platform_types_by_template_key(&key.family, &key.variant)
                        .map(|(id, _)| self.platform_type_from_id(id))
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
                } else {
                    self.catalog
                        .generated_self_types(generated_self_role)
                        .map(|(id, _)| self.platform_type_from_id(id))
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
        if !context.is_source_active(self.catalog.source_id())
            || &owner.0.source != self.catalog.source_id()
            || owner.0.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found("platform owner type not found"));
        }
        let facts = if let Some((owner_id, _)) = self.catalog.platform_type_by_id(&owner.0.local_id)
        {
            let member_kind = query.kind.map(member_query_kind_to_snapshot);
            match query.name {
                Some(name) => self
                    .catalog
                    .member_by_name_kind(owner_id, name, member_kind)
                    .map(|(id, _)| id)
                    .collect::<Vec<_>>(),
                None => self
                    .catalog
                    .members(owner_id)
                    .map(|(id, _)| id)
                    .collect::<Vec<_>>(),
            }
            .into_iter()
            .filter(|id| {
                member_kind.is_none_or(|kind| self.catalog.snapshot().type_member(*id).kind == kind)
            })
            .map(|id| self.map_member(id))
            .collect::<Vec<_>>()
        } else if let Some(owner_id) = self
            .catalog
            .snapshot()
            .worker_handle()
            .enum_by_id(&owner.0.local_id)
        {
            if query.name.is_some() {
                return Ok(ResolveResponse::not_found("platform member not found"));
            }
            self.enum_value_members(owner_id, query)
        } else {
            return Ok(ResolveResponse::not_found("platform owner type not found"));
        };
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
        if !context.is_source_active(self.catalog.source_id()) {
            return Ok(ResolveResponse::not_found(
                "platform snapshot source is not active",
            ));
        }
        let facts = match query {
            CallableLookup::Id(id) => {
                if &id.0.source != self.catalog.source_id()
                    || id.0.domain != LanguageDomain::PlatformApi
                {
                    Vec::new()
                } else {
                    self.catalog
                        .callable_by_id(&id.0.local_id)
                        .map(|(id, _)| self.map_callable(id))
                        .into_iter()
                        .collect()
                }
            }
            CallableLookup::OwnerName { owner, name } => {
                if let Some(owner) = owner {
                    if &owner.0.source != self.catalog.source_id()
                        || owner.0.domain != LanguageDomain::PlatformApi
                    {
                        Vec::new()
                    } else {
                        let Some((owner_id, _)) = self.catalog.platform_type_by_id(&owner.0.local_id) else {
                            return Ok(ResolveResponse::not_found("platform callable not found"));
                        };
                        let mut ids = self
                            .catalog
                            .callable_by_name(owner_id, name)
                            .map(|(id, _)| id)
                            .collect::<Vec<_>>();
                        ids.extend(
                            self.catalog
                                .constructors(owner_id)
                                .filter(|id| {
                                    name_matches(
                                        &self.map_name(&id.1.name),
                                        name,
                                    )
                                })
                                .map(|(id, _)| id),
                        );
                        ids.sort_unstable();
                        ids.dedup();
                        ids.into_iter().map(|id| self.map_callable(id)).collect()
                    }
                } else {
                    self.catalog
                        .global_method_by_name(name)
                        .map(|(_, _, id, _)| id)
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
        if !context.is_source_active(self.catalog.source_id()) {
            return Ok(ResolveResponse::not_found(
                "platform snapshot source is not active",
            ));
        }
        let GlobalContextQuery::Language { language, sources } = query;
        if language != GlobalContextLanguage::Bsl
            || (!sources.is_empty()
                && !sources
                    .iter()
                    .any(|source| source == self.catalog.source_id()))
        {
            return Ok(ResolveResponse::not_found(
                "platform source does not expose requested global context",
            ));
        }
        let methods = self
            .catalog
            .global_methods()
            .map(|(_, _, id, _)| self.map_callable(id))
            .collect();
        let properties = self
            .catalog
            .global_properties()
            .map(|(id, _)| self.map_global_property(id))
            .collect();
        Ok(ResolveResponse::ok(vec![ResolvedGlobalContext {
            id: self.fact_id(FactKind::Global, "global_context:bsl"),
            language,
            sources: vec![self.catalog.source_id().clone()],
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
        if !context.is_source_active(self.catalog.source_id()) {
            return Ok(ResolveResponse::not_found(
                "platform snapshot source is not active",
            ));
        }
        if query.language != GlobalContextLanguage::Bsl
            || query.domain != LanguageDomain::PlatformApi
            || (!query.sources.is_empty()
                && !query
                    .sources
                    .iter()
                    .any(|source| source == self.catalog.source_id()))
        {
            return Ok(ResolveResponse::not_found(
                "platform source does not expose requested module context",
            ));
        }
        if crate::hbk_catalogs::bsl::bsl_module_context_key(query.kind).is_none() {
            return Ok(ResolveResponse::unsupported(format!(
                "platform module context `{}` is not provider-backed by the HBK search index",
                query.kind.as_str()
            )));
        }
        let context_id = self.module_context_id(query.kind);
        let events = self
            .catalog
            .module_context_events(query.kind)
            .map(|(id, _)| id)
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
            sources: vec![self.catalog.source_id().clone()],
            self_member: None,
            properties: global.properties,
            methods: global.methods,
            events,
            facts: vec![self.module_context_fact(query.kind)],
        }]))
    }

    fn module_context_member(
        &self,
        query: ModuleContextMemberLookup<'_>,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ResolvedBslContextMember>, ResolveError> {
        if !context.is_source_active(self.catalog.source_id())
            || query.language != GlobalContextLanguage::Bsl
            || query.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found(
                "platform snapshot source does not expose requested module member",
            ));
        }
        let facts = match query.kind {
            MemberQueryKind::Property => self
                .catalog
                .global_property_by_name(query.name)
                .map(|(id, _)| ResolvedBslContextMember::Property(self.map_global_property(id)))
                .collect(),
            MemberQueryKind::Method => self
                .catalog
                .global_method_by_name(query.name)
                .map(|(_, _, id, _)| id)
                .map(|id| ResolvedBslContextMember::Callable(self.map_callable(id)))
                .collect(),
            MemberQueryKind::Event => {
                if crate::hbk_catalogs::bsl::bsl_module_context_key(query.module_kind).is_none() {
                    return Ok(ResolveResponse::unsupported(format!(
                        "platform module context `{}` is not provider-backed by the HBK search index",
                        query.module_kind.as_str()
                    )));
                }
                self.catalog
                    .module_context_event_by_name(query.module_kind, query.name)
                    .map(|(id, _)| {
                        let mut callable = self.map_callable(id);
                        callable.fact.owner = Some(self.module_context_id(query.module_kind));
                        ResolvedBslContextMember::Callable(callable)
                    })
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
        if !context.is_source_active(self.catalog.source_id())
            || query.language != GlobalContextLanguage::Bsl
            || query.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found(
                "platform snapshot source does not expose requested module members",
            ));
        }
        if crate::hbk_catalogs::bsl::bsl_module_context_key(query.module_kind).is_none() {
            return Ok(ResolveResponse::unsupported(format!(
                "platform module context `{}` is not provider-backed by the HBK search index",
                query.module_kind.as_str()
            )));
        }

        let context_id = self.module_context_id(query.module_kind);
        let mut facts = self
            .catalog
            .global_properties()
            .map(|(id, _)| ResolvedBslContextMember::Property(self.map_global_property(id)))
            .collect::<Vec<_>>();
        facts.extend(
            self.catalog
                .global_methods()
                .map(|(_, _, id, _)| ResolvedBslContextMember::Callable(self.map_callable(id))),
        );
        facts.extend(
            self.catalog
                .module_context_events(query.module_kind)
                .map(|(id, _)| {
                    let mut callable = self.map_callable(id);
                    callable.fact.owner = Some(context_id.clone());
                    ResolvedBslContextMember::Callable(callable)
                }),
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
        if !context.is_source_active(self.catalog.source_id())
            || &source.source != self.catalog.source_id()
            || source.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found("platform source fact not found"));
        }
        let Some(edge) = edge_from_relation_kind(kind) else {
            return Ok(ResolveResponse::unsupported(
                "platform adapter supports has_type, returns, constructs and member_of",
            ));
        };
        let handle = self.catalog.snapshot().worker_handle();
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
        if !context.is_source_active(self.catalog.source_id())
            || &source.source != self.catalog.source_id()
            || source.domain != LanguageDomain::PlatformApi
        {
            return Ok(ResolveResponse::not_found("platform source fact not found"));
        }
        let Some(availability) = self.catalog_availability(source) else {
            return Ok(ResolveResponse::not_found("platform fact not found"));
        };
        Ok(ResolveResponse::ok(vec![availability]))
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
            catalog: HbkSdblQueryCatalog::with_source_ids(snapshot, source_id, platform_source_id),
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

    fn query_table_id(
        &self,
        table: &FactId,
        context: &ResolveContext<'_>,
    ) -> Option<HbkQueryTableId> {
        if !context.is_source_active(self.catalog.source_id())
            || table.source != *self.catalog.source_id()
            || table.domain != LanguageDomain::QueryLanguage
            || table.kind != FactKind::QueryTable
        {
            return None;
        }
        self.catalog
            .query_table_by_id(&table.local_id)
            .map(|(id, _)| id)
    }

    pub fn query_fields(
        &self,
        table: &FactId,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        let Some(table_id) = self.query_table_id(table, context) else {
            return Ok(ResolveResponse::not_found("query table fact not found"));
        };
        Ok(ResolveResponse::ok(
            self.catalog
                .query_fields(table_id)
                .map(|(id, _)| self.map_query_field(id))
                .collect(),
        ))
    }

    pub fn query_parameters(
        &self,
        table: &FactId,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        let Some(table_id) = self.query_table_id(table, context) else {
            return Ok(ResolveResponse::not_found("query table fact not found"));
        };
        Ok(ResolveResponse::ok(
            self.catalog
                .query_parameters(table_id)
                .map(|(id, _)| self.map_query_parameter(id))
                .collect(),
        ))
    }

    pub fn query_fields_by_name(
        &self,
        table: &FactId,
        name: &str,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        let Some(table_id) = self.query_table_id(table, context) else {
            return Ok(ResolveResponse::not_found("query table fact not found"));
        };
        let facts = self
            .catalog
            .query_field_by_name(table_id, name)
            .map(|(id, _)| self.map_query_field(id))
            .collect::<Vec<_>>();
        Ok(response_from_facts(facts, "query field fact not found"))
    }

    pub fn query_parameters_by_name(
        &self,
        table: &FactId,
        name: &str,
        context: &ResolveContext<'_>,
    ) -> Result<ResolveResponse<ContextFact>, ResolveError> {
        let Some(table_id) = self.query_table_id(table, context) else {
            return Ok(ResolveResponse::not_found("query table fact not found"));
        };
        let facts = self
            .catalog
            .query_parameter_by_name(table_id, name)
            .map(|(id, _)| self.map_query_parameter(id))
            .collect::<Vec<_>>();
        Ok(response_from_facts(facts, "query parameter fact not found"))
    }

    fn fact_id(&self, kind: FactKind, local_id: impl Into<String>) -> FactId {
        FactId::new(
            self.catalog.source_id().clone(),
            LanguageDomain::QueryLanguage,
            kind,
            local_id,
        )
    }

    fn platform_type_id(&self, local_id: impl Into<String>) -> TypeId {
        TypeId(FactId::new(
            self.catalog.platform_source_id().clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            local_id,
        ))
    }

    fn source_matches(&self, source: Option<&SourceId>) -> bool {
        source.is_none_or(|source| source == self.catalog.source_id())
    }

    fn domain_matches(&self, domain: Option<LanguageDomain>) -> bool {
        domain.is_none_or(|domain| domain == LanguageDomain::QueryLanguage)
    }

    fn map_query_table(&self, id: HbkQueryTableId) -> ContextFact {
        let snapshot = self.catalog.snapshot();
        let table = snapshot.query_table(id);
        ContextFact {
            id: self.fact_id(FactKind::QueryTable, snapshot.string(table.id)),
            name: self.map_name(&table.name),
            owner: None,
            details: FactDetails::QueryTable(QueryTableInfo {
                syntax: table.syntax.as_ref().map(|name| self.map_name(name)),
                identifier: table.identifier.map(|id| snapshot.string(id).to_string()),
                sdbl_metadata_source_selector: self
                    .catalog
                    .metadata_source_selector(id)
                    .map(str::to_string),
                table_role: query_table_role(table.role),
                owner_path: table
                    .owner_path
                    .iter()
                    .map(|name| self.map_name(name))
                    .collect(),
                template_parameters: table
                    .template_parameters
                    .iter()
                    .map(|id| snapshot.string(*id).to_string())
                    .collect(),
                description: None,
                source: self.catalog.source_locale().map(|locale| FactProvenance {
                    source: self.catalog.source_id().clone(),
                    evidence_id: snapshot.string(table.id).to_string(),
                    locale: Some(locale.to_string()),
                }),
            }),
            relations: Vec::new(),
        }
    }

    fn map_query_field(&self, id: HbkQueryFieldId) -> ContextFact {
        let snapshot = self.catalog.snapshot();
        let field = snapshot.query_field(id);
        let owner = self.fact_id(
            FactKind::QueryTable,
            snapshot.string(snapshot.query_table(field.owner).id),
        );
        ContextFact {
            id: self.fact_id(FactKind::QueryField, snapshot.string(field.id)),
            name: self.map_name(&field.name),
            owner: Some(owner.clone()),
            details: FactDetails::QueryField(QueryFieldInfo {
                owner,
                types: self.map_type_refs(&field.type_refs),
                description: None,
                note: field
                    .note
                    .map(|note| snapshot.string(note).to_string()),
                source: self.catalog.source_locale().map(|locale| FactProvenance {
                    source: self.catalog.source_id().clone(),
                    evidence_id: snapshot.string(field.id).to_string(),
                    locale: Some(locale.to_string()),
                }),
            }),
            relations: Vec::new(),
        }
    }

    fn map_query_parameter(&self, id: HbkQueryParameterId) -> ContextFact {
        let snapshot = self.catalog.snapshot();
        let parameter = snapshot.query_parameter(id);
        let owner = self.fact_id(
            FactKind::QueryTable,
            snapshot.string(snapshot.query_table(parameter.owner).id),
        );
        ContextFact {
            id: self.fact_id(FactKind::QueryParameter, snapshot.string(parameter.id)),
            name: self.map_name(&parameter.name),
            owner: Some(owner.clone()),
            details: FactDetails::QueryParameter(QueryParameterInfo {
                owner,
                types: self.map_type_refs(&parameter.type_refs),
                description: None,
                default_value: parameter
                    .default_value
                    .map(|value| snapshot.string(value).to_string()),
                source: self.catalog.source_locale().map(|locale| FactProvenance {
                    source: self.catalog.source_id().clone(),
                    evidence_id: snapshot.string(parameter.id).to_string(),
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
                let snapshot = self.catalog.snapshot();
                let fact = snapshot.enum_fact(id);
                Some(ContextFact {
                    id: self.platform_type_id(snapshot.string(fact.id)).0,
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
        let snapshot = self.catalog.snapshot();
        let fact = snapshot.platform_type(id);
        ContextFact {
            id: self.platform_type_id(snapshot.string(fact.id)).0,
            name: self.map_name(&fact.name),
            owner: None,
            details: FactDetails::Type(snapshot_platform_type_info(snapshot, id)),
            relations: Vec::new(),
        }
    }

    fn map_name(&self, name: &HbkName) -> Name {
        Name::new(
            self.catalog.snapshot().string(name.primary).to_string(),
            name.alias
                .map(|alias| self.catalog.snapshot().string(alias).to_string()),
        )
    }

    fn map_type_template_key(
        &self,
        key: syntax_helper_search::HbkPlatformTypeTemplateKey,
    ) -> PlatformTypeTemplateKey {
        snapshot_type_template_key(self.catalog.snapshot(), key)
    }

    fn map_type_refs(&self, refs: &[HbkTypeRef]) -> Vec<TypeRef> {
        refs.iter()
            .map(|type_ref| self.map_type_ref(type_ref))
            .collect()
    }

    fn map_type_ref(&self, type_ref: &HbkTypeRef) -> TypeRef {
        TypeRef {
            name: self.catalog.snapshot().string(type_ref.name).to_string(),
            target: match &type_ref.target {
                HbkTypeRefTarget::Ok(id) => {
                    TypeRefTarget::Ok(self.platform_type_id(self.catalog.snapshot().string(*id)))
                }
                HbkTypeRefTarget::Unresolved => TypeRefTarget::Unresolved,
                HbkTypeRefTarget::Ambiguous(candidates) => TypeRefTarget::Ambiguous(
                    candidates
                        .iter()
                        .map(|id| self.platform_type_id(self.catalog.snapshot().string(*id)))
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
            id: self.catalog.source_id().clone(),
            domain: LanguageDomain::QueryLanguage,
            label: format!(
                "Syntax Assistant query table snapshot {}",
                self.catalog.source_id()
            ),
        }
    }

    fn source_id(&self) -> Option<&SourceId> {
        Some(self.catalog.source_id())
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
        if !context.is_source_active(self.catalog.source_id()) {
            return Ok(ResolveResponse::not_found(
                "query table snapshot source is not active",
            ));
        }
        match query {
            context_resolver_core::ResolveQuery::Id(id) => {
                if id.source != *self.catalog.source_id()
                    || id.domain != LanguageDomain::QueryLanguage
                {
                    return Ok(ResolveResponse::not_found(
                        "fact source or domain does not match",
                    ));
                }
                let facts = match id.kind {
                    FactKind::QueryTable => self
                        .catalog
                        .query_table_by_id(&id.local_id)
                        .map(|(id, _)| self.map_query_table(id))
                        .into_iter()
                        .collect(),
                    FactKind::QueryField => self
                        .catalog
                        .query_field_by_id(&id.local_id)
                        .map(|(id, _)| self.map_query_field(id))
                        .into_iter()
                        .collect(),
                    FactKind::QueryParameter => self
                        .catalog
                        .query_parameter_by_id(&id.local_id)
                        .map(|(id, _)| self.map_query_parameter(id))
                        .into_iter()
                        .collect(),
                    _ => Vec::new(),
                };
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
                let mut ids = self
                    .catalog
                    .query_tables_by_name(name)
                    .map(|(id, _)| id)
                    .collect::<Vec<_>>();
                ids.extend(self.catalog.query_tables_by_identifier(name).map(|(id, _)| id));
                ids.extend(self.catalog.query_tables_by_syntax(name).map(|(id, _)| id));
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
        if !context.is_source_active(self.catalog.source_id()) {
            return Ok(ResolveResponse::not_found(
                "query table snapshot source is not active",
            ));
        }
        let GlobalContextQuery::Language { language, sources } = query;
        if language != GlobalContextLanguage::Sdbl
            || (!sources.is_empty()
                && !sources
                    .iter()
                    .any(|source| source == self.catalog.source_id()))
        {
            return Ok(ResolveResponse::not_found(
                "query table source does not expose requested global context",
            ));
        }
        let facts = self
            .catalog
            .query_tables()
            .map(|(id, _)| self.map_query_table(id))
            .chain(self.catalog.query_tables().flat_map(|(table, _)| {
                self.catalog
                    .query_fields(table)
                    .map(|(id, _)| self.map_query_field(id))
            }))
            .chain(self.catalog.query_tables().flat_map(|(table, _)| {
                self.catalog
                    .query_parameters(table)
                    .map(|(id, _)| self.map_query_parameter(id))
            }))
            .collect::<Vec<_>>();
        Ok(ResolveResponse::ok(vec![ResolvedGlobalContext {
            id: self.fact_id(FactKind::Global, "global_context:sdbl:query_tables"),
            language,
            sources: vec![self.catalog.source_id().clone()],
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
        if !context.is_source_active(self.catalog.source_id())
            || source.source != *self.catalog.source_id()
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
        let handle = self.catalog.snapshot().worker_handle();
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
