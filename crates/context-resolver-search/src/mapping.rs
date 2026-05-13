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
        SearchDocumentKind::TypeProperty => Some(FactKind::Member),
        SearchDocumentKind::GlobalProperty => Some(FactKind::Global),
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

fn name_matches(name: &Name, value: &str) -> bool {
    name.primary == value || name.alias.as_deref() == Some(value)
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
        SearchDocumentKind::QueryTable => Some(FactKind::QueryTable),
        SearchDocumentKind::QueryTableField => Some(FactKind::QueryField),
        SearchDocumentKind::QueryTableParameter => Some(FactKind::QueryParameter),
        SearchDocumentKind::PlatformType
        | SearchDocumentKind::TypeProperty
        | SearchDocumentKind::TypeMethod
        | SearchDocumentKind::Constructor
        | SearchDocumentKind::GlobalMethod
        | SearchDocumentKind::GlobalProperty
        | SearchDocumentKind::ModuleEvent
        | SearchDocumentKind::TypeEvent
        | SearchDocumentKind::UnknownEvent
        | SearchDocumentKind::Enum
        | SearchDocumentKind::EnumValue => None,
    }
}

fn language_source_domain_kind_and_local_id(
    storage_id: &str,
) -> Option<(SourceId, LanguageDomain, FactKind, &str)> {
    let (source, local_id) = storage_id.split_once(':')?;
    let (domain, kind) = match source {
        "shlang" => (LanguageDomain::BslLanguage, FactKind::Type),
        "shquery" | "dcsui" => (LanguageDomain::QueryLanguage, FactKind::Type),
        _ => return None,
    };
    Some((SourceId::new(source), domain, kind, local_id))
}

fn map_model_name(name: &syntax_helper_search::model::LocalizedName) -> Name {
    Name::new(name.primary.clone(), name.alias.clone())
}

fn query_table_role(role: Option<syntax_helper_search::model::QueryTableRole>) -> QueryTableRole {
    match role.unwrap_or(syntax_helper_search::model::QueryTableRole::Unknown) {
        syntax_helper_search::model::QueryTableRole::Primary => QueryTableRole::Primary,
        syntax_helper_search::model::QueryTableRole::Additional => QueryTableRole::Additional,
        syntax_helper_search::model::QueryTableRole::Unknown => QueryTableRole::Unknown,
    }
}

fn fact_provenance(
    source_id: &SourceId,
    document_id: &str,
    source: &syntax_helper_search::model::SyntaxHelperSource,
) -> FactProvenance {
    FactProvenance {
        source: source_id.clone(),
        evidence_id: document_id.to_string(),
        locale: Some(source.locale.clone()),
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

fn search_module_context_relation_key(kind: ModuleContextKind) -> Option<&'static str> {
    match kind {
        ModuleContextKind::Session => Some("module_context:session"),
        ModuleContextKind::OrdinaryApplication => Some("module_context:ordinary_application"),
        ModuleContextKind::ManagedApplication => Some("module_context:managed_application"),
        ModuleContextKind::ExternalConnection => Some("module_context:external_connection"),
        ModuleContextKind::Object => Some("module_context:object"),
        ModuleContextKind::Manager => Some("module_context:manager"),
        ModuleContextKind::Form => Some("module_context:form"),
        ModuleContextKind::WebService => Some("module_context:web_service"),
        ModuleContextKind::HttpService => Some("module_context:http_service"),
        ModuleContextKind::Unknown => Some("module_context:unknown"),
        ModuleContextKind::Common
        | ModuleContextKind::Command
        | ModuleContextKind::RecordSet
        | ModuleContextKind::Unsupported => None,
    }
}

fn module_context_kind_from_local_id(local_id: &str) -> Option<ModuleContextKind> {
    match local_id.strip_prefix("module_context:")? {
        "session" => Some(ModuleContextKind::Session),
        "ordinary_application" => Some(ModuleContextKind::OrdinaryApplication),
        "managed_application" => Some(ModuleContextKind::ManagedApplication),
        "external_connection" => Some(ModuleContextKind::ExternalConnection),
        "object" => Some(ModuleContextKind::Object),
        "manager" => Some(ModuleContextKind::Manager),
        "form" => Some(ModuleContextKind::Form),
        "web_service" => Some(ModuleContextKind::WebService),
        "http_service" => Some(ModuleContextKind::HttpService),
        "unknown" => Some(ModuleContextKind::Unknown),
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
