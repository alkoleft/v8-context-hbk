#[allow(clippy::too_many_arguments)]
fn document(
    kind: SearchDocumentKind,
    owner: Option<&model::LocalizedName>,
    name: &model::LocalizedName,
    signatures: &[model::Signature],
    return_types: &[model::TypeRef],
    type_refs: &[model::TypeRef],
    description: Option<&str>,
    id: String,
) -> SearchDocument {
    let parameter_terms = signatures
        .iter()
        .flat_map(|signature| signature.parameters.iter())
        .flat_map(|parameter| {
            std::iter::once(parameter.name.clone()).chain(
                parameter
                    .type_refs
                    .iter()
                    .map(|type_ref| type_ref.name.clone()),
            )
        })
        .collect::<Vec<_>>();
    let signatures = signatures
        .iter()
        .map(SearchSignature::from)
        .collect::<Vec<_>>();
    let return_types = return_types
        .iter()
        .map(|type_ref| type_ref.name.clone())
        .collect::<Vec<_>>();
    let type_refs = type_refs
        .iter()
        .map(|type_ref| type_ref.name.clone())
        .collect::<Vec<_>>();
    SearchDocument {
        id,
        kind,
        name: name.clone(),
        owner: owner.cloned(),
        signatures,
        type_refs,
        return_types,
        type_ref_facts: Vec::new(),
        return_type_facts: Vec::new(),
        description: description.map(ToOwned::to_owned),
        preview: description
            .map(|value| value.chars().take(180).collect())
            .unwrap_or_default(),
        parameter_terms,
        relation_keys: Vec::new(),
        owner_relation_key: None,
        explicit_type_ref_ids: Vec::new(),
        explicit_return_type_ref_ids: Vec::new(),
        availability_contexts: Vec::new(),
        available_since: None,
        metadata_kind: None,
        template_parameters: Vec::new(),
        type_template_key: None,
        type_template_classification_diagnostic: None,
    }
}

impl From<&model::Signature> for SearchSignature {
    fn from(signature: &model::Signature) -> Self {
        Self {
            text: signature.text.clone(),
            parameters: signature
                .parameters
                .iter()
                .map(SearchParameter::from)
                .collect(),
            return_types: signature
                .return_types
                .iter()
                .map(|type_ref| type_ref.name.clone())
                .collect(),
            return_type_facts: Vec::new(),
            title: signature
                .variant
                .as_ref()
                .map(|variant| variant.title.clone())
                .filter(|title| !title.is_empty()),
            description: signature
                .variant
                .as_ref()
                .and_then(|variant| variant.description.clone()),
        }
    }
}

impl From<&model::Parameter> for SearchParameter {
    fn from(parameter: &model::Parameter) -> Self {
        Self {
            name: parameter.name.clone(),
            required: parameter.required,
            type_refs: parameter
                .type_refs
                .iter()
                .map(|type_ref| type_ref.name.clone())
                .collect(),
            type_ref_facts: Vec::new(),
            description: parameter.description.clone(),
        }
    }
}

fn language_document(fact: &language::LanguageFact) -> SearchDocument {
    let mut signatures = fact
        .signatures
        .iter()
        .map(|signature| SearchSignature {
            text: signature.text.clone(),
            parameters: signature
                .parameters
                .iter()
                .map(|parameter| SearchParameter {
                    name: parameter.name.clone(),
                    required: parameter.required,
                    type_refs: parameter.type_refs.clone(),
                    type_ref_facts: Vec::new(),
                    description: parameter.description.clone(),
                })
                .collect(),
            return_types: Vec::new(),
            return_type_facts: Vec::new(),
            title: None,
            description: None,
        })
        .collect::<Vec<_>>();
    if signatures.is_empty()
        && let Some(syntax) = &fact.syntax
        && !syntax.is_empty()
    {
        signatures.push(SearchSignature {
            text: syntax.clone(),
            parameters: Vec::new(),
            return_types: Vec::new(),
            return_type_facts: Vec::new(),
            title: None,
            description: None,
        });
    }
    let parameter_terms = fact
        .signatures
        .iter()
        .flat_map(|signature| signature.parameters.iter())
        .flat_map(|parameter| {
            std::iter::once(parameter.name.clone()).chain(parameter.type_refs.iter().cloned())
        })
        .chain(fact.type_refs.iter().cloned())
        .chain(fact.return_types.iter().cloned())
        .collect::<Vec<_>>();
    let type_refs = fact
        .type_refs
        .iter()
        .cloned()
        .chain(
            fact.signatures
                .iter()
                .flat_map(|signature| signature.parameters.iter())
                .flat_map(|parameter| parameter.type_refs.iter().cloned()),
        )
        .collect::<Vec<_>>();
    let explicit_type_ref_ids = explicit_language_type_ref_ids(fact, &type_refs);
    let explicit_return_type_ref_ids = explicit_language_type_ref_ids(fact, &fact.return_types);
    SearchDocument {
        id: fact.id.clone(),
        kind: language_document_kind(fact.family),
        name: fact.name.clone(),
        owner: None,
        signatures,
        type_refs,
        return_types: fact.return_types.clone(),
        type_ref_facts: Vec::new(),
        return_type_facts: Vec::new(),
        description: fact.description.clone(),
        preview: fact
            .description
            .as_deref()
            .map(|value| value.chars().take(180).collect())
            .unwrap_or_default(),
        parameter_terms,
        relation_keys: vec![fact.id.clone()],
        owner_relation_key: None,
        explicit_type_ref_ids,
        explicit_return_type_ref_ids,
        availability_contexts: Vec::new(),
        available_since: None,
        metadata_kind: None,
        template_parameters: Vec::new(),
        type_template_key: None,
        type_template_classification_diagnostic: None,
    }
}

fn language_document_kind(family: language::LanguageFactFamily) -> SearchDocumentKind {
    match family {
        language::LanguageFactFamily::Construct => SearchDocumentKind::LanguageConstruct,
        language::LanguageFactFamily::Type => SearchDocumentKind::LanguageType,
        language::LanguageFactFamily::Function => SearchDocumentKind::LanguageFunction,
        language::LanguageFactFamily::Operator => SearchDocumentKind::LanguageOperator,
        language::LanguageFactFamily::Keyword => SearchDocumentKind::LanguageKeyword,
        language::LanguageFactFamily::Literal => SearchDocumentKind::LanguageLiteral,
    }
}

fn explicit_language_type_ref_ids(
    fact: &language::LanguageFact,
    names: &[String],
) -> Vec<Option<String>> {
    names
        .iter()
        .map(|name| explicit_language_type_ref_id(fact.source_family, name))
        .collect()
}

fn explicit_language_type_ref_id(
    source_family: language::LanguageSourceFamily,
    name: &str,
) -> Option<String> {
    let normalized = normalize_lookup_key(name);
    match source_family {
        language::LanguageSourceFamily::Shlang => None,
        language::LanguageSourceFamily::Shquery
            if matches!(normalized.as_str(), "строка" | "string") =>
        {
            Some("shquery:LitString".to_string())
        }
        language::LanguageSourceFamily::Dcsui
            if matches!(normalized.as_str(), "строка" | "string") =>
        {
            Some("shlang:def_String".to_string())
        }
        _ => None,
    }
}

fn type_ref_from_name(name: &model::LocalizedName) -> model::TypeRef {
    model::TypeRef {
        name: name.display_name(),
    }
}

fn event_owner(event: &model::GlobalContextEvent) -> Option<model::LocalizedName> {
    if let Some(owner) = event.module.owner_path.last().cloned() {
        return Some(owner);
    }
    None
}

fn module_context_relation_key(kind: model::ModuleKind) -> String {
    normalize_lookup_key(&format!(
        "module_context:{}",
        match kind {
            model::ModuleKind::Session => "session",
            model::ModuleKind::OrdinaryApplication => "ordinary_application",
            model::ModuleKind::ManagedApplication => "managed_application",
            model::ModuleKind::ExternalConnection => "external_connection",
            model::ModuleKind::Object => "object",
            model::ModuleKind::Manager => "manager",
            model::ModuleKind::Form => "form",
            model::ModuleKind::WebService => "web_service",
            model::ModuleKind::HttpService => "http_service",
            model::ModuleKind::Unknown => "unknown",
        }
    ))
}

fn semantic_relation_key(semantic: &model::SemanticContext, fallback: &str) -> String {
    let mut parts = semantic
        .owner_path
        .iter()
        .map(|name| name.primary.as_str())
        .collect::<Vec<_>>();
    if parts
        .last()
        .is_none_or(|last| normalize_lookup_key(last) != normalize_lookup_key(fallback))
    {
        parts.push(fallback);
    }
    normalize_lookup_key(&parts.join("."))
}
