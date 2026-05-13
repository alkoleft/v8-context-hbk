fn relation_lookup(documents: &[SearchDocument]) -> BTreeMap<String, (&SearchDocument, String)> {
    let mut by_name = BTreeMap::<String, (&SearchDocument, String)>::new();
    for document in documents {
        for key in document_lookup_keys(document) {
            match by_name.get(&key) {
                Some((existing, _))
                    if kind_priority(existing.kind) <= kind_priority(document.kind) => {}
                _ => {
                    by_name.insert(key, (document, document.id.clone()));
                }
            }
        }
    }
    by_name
}

fn type_ref_target_lookup(documents: &[SearchDocument]) -> BTreeMap<String, BTreeSet<String>> {
    let mut by_key = BTreeMap::<String, BTreeSet<String>>::new();
    for document in documents
        .iter()
        .filter(|document| document.kind.is_type_ref_target())
    {
        by_key
            .entry(normalize_lookup_key(&document.name.primary))
            .or_default()
            .insert(document.id.clone());
        if let Some(alias) = &document.name.alias {
            by_key
                .entry(normalize_lookup_key(alias))
                .or_default()
                .insert(document.id.clone());
        }
        if document.kind == SearchDocumentKind::PlatformType
            && let Some(metadata_kind) = &document.metadata_kind
        {
            by_key
                .entry(normalize_lookup_key(metadata_kind))
                .or_default()
                .insert(document.id.clone());
        }
    }
    by_key
}

fn document_lookup_keys(document: &SearchDocument) -> Vec<String> {
    let mut keys = vec![normalize_lookup_key(&document.name.primary)];
    if let Some(alias) = &document.name.alias {
        keys.push(normalize_lookup_key(alias));
    }
    keys.extend(document.relation_keys.iter().cloned());
    keys
}

fn document_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchDocument> {
    let description: Option<String> = row.get(7)?;
    let kind_text: String = row.get(1)?;
    let kind = SearchDocumentKind::from_storage(&kind_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            Box::new(UnknownSearchDocumentKind(kind_text)),
        )
    })?;
    Ok(SearchDocument {
        id: row.get(0)?,
        kind,
        name: model::LocalizedName {
            primary: row.get(2)?,
            alias: row.get(3)?,
        },
        owner: optional_localized_name(row.get(4)?, row.get(5)?),
        signatures: split_lines(row.get(6)?)
            .into_iter()
            .map(|text| SearchSignature {
                text,
                parameters: Vec::new(),
                return_types: Vec::new(),
                return_type_facts: Vec::new(),
                title: None,
                description: None,
            })
            .collect(),
        parameter_terms: Vec::new(),
        type_refs: Vec::new(),
        return_types: Vec::new(),
        type_ref_facts: Vec::new(),
        return_type_facts: Vec::new(),
        preview: description
            .as_deref()
            .map(|value| value.chars().take(180).collect())
            .unwrap_or_default(),
        description,
        relation_keys: Vec::new(),
        owner_relation_key: None,
        explicit_type_ref_ids: Vec::new(),
        explicit_return_type_ref_ids: Vec::new(),
        availability_contexts: split_lines(row.get(8)?),
        available_since: row.get(9)?,
        metadata_kind: None,
        template_parameters: Vec::new(),
        type_template_key: None,
        type_template_classification_diagnostic: None,
    })
}

fn search_type_ref_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchTypeRef> {
    let type_template_key = type_template_key_from_codes(
        row.get::<_, Option<String>>(4)?,
        row.get::<_, Option<String>>(5)?,
    );
    let binding_kind: Option<String> = row.get(6)?;
    let owner_parameter_index: Option<i64> = row.get(7)?;
    let target_parameter_index: Option<i64> = row.get(8)?;
    let binding_arguments: Option<String> = row.get(9)?;
    let template_binding = match (
        type_template_key.clone(),
        binding_kind.as_deref(),
        owner_parameter_index,
        target_parameter_index,
        binding_arguments.as_deref(),
    ) {
        (Some(template_key), Some("owner_parameter"), _, _, Some(arguments)) => {
            Some(model::TypeTemplateBinding {
                template_key,
                arguments: parse_binding_arguments(arguments),
            })
        }
        (
            Some(template_key),
            Some("owner_parameter"),
            Some(owner_index),
            Some(target_index),
            None,
        ) => Some(model::TypeTemplateBinding {
            template_key,
            arguments: vec![model::TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: owner_index as usize,
                target_parameter_index: target_index as usize,
            }],
        }),
        _ => None,
    };
    let target_type_id: Option<String> = row.get(1)?;
    let target = match row.get::<_, String>(2)?.as_str() {
        "ok" => {
            SearchTypeRefTarget::Ok(target_type_id.ok_or_else(|| rusqlite::Error::InvalidQuery)?)
        }
        "unresolved" => SearchTypeRefTarget::Unresolved,
        "ambiguous" => {
            let candidates = row
                .get::<_, Option<String>>(3)?
                .map(|value| value.lines().map(str::to_string).collect())
                .unwrap_or_default();
            SearchTypeRefTarget::Ambiguous(candidates)
        }
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(SearchTypeRef {
        name: row.get(0)?,
        target,
        type_template_key,
        template_binding,
    })
}

fn parse_binding_arguments(value: &str) -> Vec<model::TemplateParameterBinding> {
    value
        .lines()
        .filter_map(|line| {
            let (owner, target) = line.split_once(':')?;
            Some(model::TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: owner.parse().ok()?,
                target_parameter_index: target.parse().ok()?,
            })
        })
        .collect()
}

fn availability_context_code(context: model::AvailabilityContext) -> &'static str {
    match context {
        model::AvailabilityContext::ThinClient => "thin_client",
        model::AvailabilityContext::WebClient => "web_client",
        model::AvailabilityContext::MobileClient => "mobile_client",
        model::AvailabilityContext::Server => "server",
        model::AvailabilityContext::ThickClient => "thick_client",
        model::AvailabilityContext::ExternalConnection => "external_connection",
        model::AvailabilityContext::MobileApplicationClient => "mobile_application_client",
        model::AvailabilityContext::MobileApplicationServer => "mobile_application_server",
        model::AvailabilityContext::MobileStandaloneServer => "mobile_standalone_server",
    }
}

fn type_template_key_from_codes(
    family: Option<String>,
    variant: Option<String>,
) -> Option<model::PlatformTypeTemplateKey> {
    Some(model::PlatformTypeTemplateKey::new(family?, variant?))
}

fn optional_localized_name(
    primary: Option<String>,
    alias: Option<String>,
) -> Option<model::LocalizedName> {
    primary.map(|primary| model::LocalizedName { primary, alias })
}

fn split_lines(value: String) -> Vec<String> {
    value
        .lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

struct DocumentIdentities {
    platform_type_ids: BTreeMap<String, String>,
    query_table_ids: BTreeMap<String, String>,
    enum_ids: BTreeMap<String, String>,
}

impl DocumentIdentities {
    fn from_inputs(
        platform_types: &[PlatformTypeIdentityInput],
        query_tables: &[QueryTableIdentityInput],
        enums: &[EnumIdentityInput],
    ) -> Self {
        let platform_type_counts = model::count_identity_keys(
            platform_types
                .iter()
                .map(|record| model::platform_type_identity_key(&record.name_primary)),
        );
        let query_table_counts = model::count_identity_keys(query_tables.iter().map(|record| {
            model::query_table_identity_key(
                &record.name_primary,
                record.identifier.as_deref(),
                &record.semantic,
            )
        }));
        let enum_counts = model::count_identity_keys(enums.iter().map(|record| {
            model::enum_identity_key(&record.name_primary, &record.source_html_path)
        }));
        let platform_type_ids = platform_types
            .iter()
            .map(|record| {
                (
                    model::platform_type_semantic_key(&record.name_primary, &record.semantic),
                    record.identity.clone().unwrap_or_else(|| {
                        model::platform_type_identity(
                            &record.name_primary,
                            &record.semantic,
                            platform_type_counts
                                .get(&model::platform_type_identity_key(&record.name_primary))
                                .copied()
                                .unwrap_or_default(),
                        )
                    }),
                )
            })
            .collect();
        let query_table_ids = query_tables
            .iter()
            .map(|record| {
                (
                    model::query_table_semantic_key(&record.semantic, &record.name_primary),
                    record.identity.clone().unwrap_or_else(|| {
                        model::query_table_identity(
                            &record.name_primary,
                            record.identifier.as_deref(),
                            &record.semantic,
                            query_table_counts
                                .get(&model::query_table_identity_key(
                                    &record.name_primary,
                                    record.identifier.as_deref(),
                                    &record.semantic,
                                ))
                                .copied()
                                .unwrap_or_default(),
                        )
                    }),
                )
            })
            .collect();
        let enum_ids = enums
            .iter()
            .map(|record| {
                (
                    enum_record_key(&record.name_primary, &record.source_html_path),
                    record.identity.clone().unwrap_or_else(|| {
                        model::enum_identity(
                            &record.name_primary,
                            record.name_alias.as_deref(),
                            &record.source_html_path,
                            enum_counts
                                .get(&model::enum_identity_key(
                                    &record.name_primary,
                                    &record.source_html_path,
                                ))
                                .copied()
                                .unwrap_or_default(),
                        )
                    }),
                )
            })
            .collect();

        Self {
            platform_type_ids,
            query_table_ids,
            enum_ids,
        }
    }

    fn platform_type_identity_by(
        &self,
        name_primary: &str,
        semantic: &model::SemanticContext,
    ) -> String {
        self.platform_type_ids
            .get(&model::platform_type_semantic_key(name_primary, semantic))
            .cloned()
            .unwrap_or_else(|| {
                format!("platform_type:{}", model::clean_identity_part(name_primary))
            })
    }

    fn query_table_identity_by(
        &self,
        name_primary: &str,
        identifier: Option<&str>,
        semantic: &model::SemanticContext,
    ) -> String {
        self.query_table_ids
            .get(&model::query_table_semantic_key(semantic, name_primary))
            .cloned()
            .unwrap_or_else(|| model::query_table_identity(name_primary, identifier, semantic, 1))
    }

    fn enum_identity_by(
        &self,
        name_primary: &str,
        name_alias: Option<&str>,
        source_html_path: &str,
    ) -> String {
        self.enum_ids
            .get(&enum_record_key(name_primary, source_html_path))
            .cloned()
            .unwrap_or_else(|| model::enum_identity(name_primary, name_alias, source_html_path, 1))
    }
}

fn document_identity(
    kind: &str,
    owner: Option<&model::LocalizedName>,
    name: &model::LocalizedName,
) -> String {
    match owner {
        Some(owner) => owned_document_identity(
            kind,
            &format!("owner:{}", model::clean_identity_part(&owner.primary)),
            &name.primary,
        ),
        None => format!("{kind}:{}", model::clean_identity_part(&name.primary)),
    }
}

fn owned_document_identity(kind: &str, owner_identity: &str, name: &str) -> String {
    format!(
        "{kind}:{owner_identity}:{}",
        model::clean_identity_part(name)
    )
}

fn identity_relation_key(identity: &str) -> String {
    format!("id:{}", normalize_lookup_key(identity))
}

fn enum_record_key(name_primary: &str, source_html_path: &str) -> String {
    format!(
        "{}:{}",
        model::enum_identity_key(name_primary, source_html_path),
        source_html_path
    )
}
