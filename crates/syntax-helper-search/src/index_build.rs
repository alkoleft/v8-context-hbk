pub fn build_index_from_builder(
    path: impl AsRef<Path>,
    metadata: &IndexMetadata,
    builder: SearchIndexBuilder,
) -> Result<(), SearchError> {
    build_index_from_builder_with_report(path, metadata, builder).map(|_| ())
}

pub fn build_index_from_builder_with_report(
    path: impl AsRef<Path>,
    metadata: &IndexMetadata,
    builder: SearchIndexBuilder,
) -> Result<IndexBuildReport, SearchError> {
    let build = builder.into_documents(&metadata.source_locale)?;
    build_index_from_documents(path, metadata, build.documents)?;
    Ok(IndexBuildReport {
        warnings: build.warnings,
    })
}

fn build_index_from_documents(
    path: impl AsRef<Path>,
    metadata: &IndexMetadata,
    documents: Vec<SearchDocument>,
) -> Result<(), SearchError> {
    validate_document_id_collisions(&documents)?;
    let path = path.as_ref();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| SearchError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let _lock = WriterLock::acquire(path)?;
    let temp_path = temp_index_path(path);
    remove_sqlite_artifacts(&temp_path)?;

    let result = build_index_file(&temp_path, metadata, documents).and_then(|()| {
        remove_sqlite_sidecars(path)?;
        fs::rename(&temp_path, path).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })
    });
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn build_index_file(
    path: &Path,
    metadata: &IndexMetadata,
    documents: Vec<SearchDocument>,
) -> Result<(), SearchError> {
    let mut connection = Connection::open(path).map_err(|source| SearchError::Sqlite {
        path: path.to_path_buf(),
        source,
    })?;
    create_schema(&connection, path)?;
    write_metadata(&connection, path, metadata)?;
    let transaction = connection
        .transaction()
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    insert_documents(&transaction, path, &documents)?;
    insert_normalized_facts(&transaction, path, &documents)?;
    rebuild_document_fts(&transaction, path)?;
    insert_relations_from_documents(&transaction, path, &documents)?;
    create_lookup_indexes(&transaction, path)?;
    transaction.commit().map_err(|source| SearchError::Sqlite {
        path: path.to_path_buf(),
        source,
    })?;
    validate_index(&connection, path)
}

fn deduplicate_documents(documents: &mut Vec<SearchDocument>) -> Vec<IndexBuildWarning> {
    let mut counts = BTreeMap::<String, usize>::new();
    for document in documents.iter() {
        *counts.entry(document.id.clone()).or_default() += 1;
    }
    let warnings = counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(id, count)| IndexBuildWarning {
            code: "DUPLICATE_DOCUMENT_ID",
            message: format!(
                "duplicate Syntax Assistant search document id '{id}': {count} documents; kept the last document"
            ),
        })
        .collect::<Vec<_>>();
    if warnings.is_empty() {
        return warnings;
    }

    let mut remaining = counts;
    documents.retain(|document| {
        let Some(count) = remaining.get_mut(&document.id) else {
            return true;
        };
        if *count <= 1 {
            return true;
        }
        *count -= 1;
        false
    });
    warnings
}

fn classify_platform_type_templates(
    documents: &mut [SearchDocument],
    source_locale: &str,
) -> Vec<IndexBuildWarning> {
    let allow_primary_fallback = source_locale == "root";
    let by_name = relation_lookup(documents);
    let type_id_by_normalized_id = documents
        .iter()
        .filter(|document| document.kind == SearchDocumentKind::PlatformType)
        .map(|document| (normalize_lookup_key(&document.id), document.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let template_infos = documents
        .iter()
        .filter(|document| {
            document.kind == SearchDocumentKind::PlatformType && document.metadata_kind.is_some()
        })
        .filter_map(|document| {
            Some(PlatformTypeTemplateInfo {
                id: document.id.clone(),
                base: model::platform_type_template_base_for_source(
                    &document.name,
                    allow_primary_fallback,
                ),
                metadata_kind: document.metadata_kind.clone()?,
            })
        })
        .collect::<Vec<_>>();
    let mut family_roots = template_infos
        .iter()
        .filter_map(|template| {
            template
                .base
                .as_deref()
                .and_then(manager_family_root)
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    family_roots.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    family_roots.dedup();

    let mut classified = BTreeMap::<String, model::PlatformTypeTemplateKey>::new();
    let mut diagnostics = BTreeMap::<String, String>::new();
    let mut unassigned = BTreeSet::<String>::new();
    for template in &template_infos {
        let Some(base) = template.base.as_deref() else {
            unassigned.insert(template.id.clone());
            continue;
        };
        if let Some(root) = family_roots
            .iter()
            .find(|root| base.starts_with(root.as_str()))
        {
            let key = model::PlatformTypeTemplateKey::new(
                root.clone(),
                type_template_variant(root, base),
            );
            diagnostics.insert(
                template.id.clone(),
                format!("manager_root family={} variant={}", key.family, key.variant),
            );
            classified.insert(template.id.clone(), key);
        } else {
            unassigned.insert(template.id.clone());
        }
    }

    let template_id_by_metadata_kind = unique_lookup(template_infos.iter().map(|template| {
        (
            normalize_lookup_key(&template.metadata_kind),
            template.id.clone(),
        )
    }));
    let mut direct_scores = BTreeMap::<String, BTreeMap<String, usize>>::new();
    for document in documents.iter() {
        let owner_type_id = owner_type_id(document, &by_name, &type_id_by_normalized_id);
        let owner_key = owner_type_id
            .as_deref()
            .and_then(|type_id| classified.get(type_id));
        let owner_unassigned_id = owner_type_id
            .as_deref()
            .filter(|type_id| unassigned.contains(*type_id));
        for target_name in document_type_template_ref_names(document) {
            let Some(target_type_id) = template_id_by_metadata_kind
                .get(&normalize_lookup_key(target_name))
                .and_then(|type_id| type_id.as_deref())
            else {
                continue;
            };
            if unassigned.contains(target_type_id) {
                if let Some(owner_key) = owner_key {
                    *direct_scores
                        .entry(target_type_id.to_string())
                        .or_default()
                        .entry(owner_key.family.clone())
                        .or_default() += 1;
                }
            } else if let Some(owner_unassigned_id) = owner_unassigned_id
                && let Some(target_key) = classified.get(target_type_id)
            {
                *direct_scores
                    .entry(owner_unassigned_id.to_string())
                    .or_default()
                    .entry(target_key.family.clone())
                    .or_default() += 1;
            }
        }
    }

    let mut warnings = Vec::new();
    for template in template_infos
        .iter()
        .filter(|template| unassigned.contains(template.id.as_str()))
    {
        let Some(base) = template.base.as_deref() else {
            let message = format!(
                "type template '{}' has no alias base and primary fallback is only allowed for root source locale",
                template.id
            );
            diagnostics.insert(template.id.clone(), message.clone());
            warnings.push(IndexBuildWarning {
                code: "TYPE_TEMPLATE_UNCLASSIFIED",
                message,
            });
            continue;
        };
        match direct_scores.get(&template.id) {
            Some(scores) if scores.len() == 1 => {
                let (family, score) = scores.iter().next().expect("one score must exist");
                let key = model::PlatformTypeTemplateKey::new(
                    family.clone(),
                    type_template_variant(family, base),
                );
                diagnostics.insert(
                    template.id.clone(),
                    format!(
                        "direct_type_ref family={} variant={} score={}",
                        key.family, key.variant, score
                    ),
                );
                classified.insert(template.id.clone(), key);
            }
            Some(scores) if !scores.is_empty() => {
                let message = format!(
                    "type template '{}' has multiple direct type-template family candidates: {}",
                    template.id,
                    scores
                        .keys()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                diagnostics.insert(template.id.clone(), message.clone());
                warnings.push(IndexBuildWarning {
                    code: "TYPE_TEMPLATE_AMBIGUOUS_FAMILY",
                    message,
                });
            }
            _ => {
                let message = format!(
                    "type template '{}' has no manager-root or direct type-template type-reference family evidence",
                    template.id
                );
                diagnostics.insert(template.id.clone(), message.clone());
                warnings.push(IndexBuildWarning {
                    code: "TYPE_TEMPLATE_UNCLASSIFIED",
                    message,
                });
            }
        }
    }

    for document in documents {
        if let Some(key) = classified.get(&document.id) {
            document.type_template_key = Some(key.clone());
        }
        if let Some(diagnostic) = diagnostics.get(&document.id) {
            document.type_template_classification_diagnostic = Some(diagnostic.clone());
        }
    }
    warnings
}

#[derive(Debug)]
struct PlatformTypeTemplateInfo {
    id: String,
    base: Option<String>,
    metadata_kind: String,
}

fn manager_family_root(base: &str) -> Option<&str> {
    base.strip_suffix("Manager")
        .or_else(|| base.strip_suffix("Менеджер"))
        .filter(|root| !root.is_empty())
}

fn type_template_variant(family: &str, base: &str) -> String {
    base.strip_prefix(family)
        .filter(|suffix| !suffix.is_empty())
        .unwrap_or(base)
        .to_string()
}

fn unique_lookup(
    items: impl Iterator<Item = (String, String)>,
) -> BTreeMap<String, Option<String>> {
    let mut lookup = BTreeMap::<String, Option<String>>::new();
    for (key, value) in items {
        match lookup.get_mut(&key) {
            Some(existing) if existing.as_deref() == Some(value.as_str()) => {}
            Some(existing) => *existing = None,
            None => {
                lookup.insert(key, Some(value));
            }
        }
    }
    lookup
}

fn document_type_template_ref_names(document: &SearchDocument) -> impl Iterator<Item = &str> {
    document
        .type_refs
        .iter()
        .chain(document.return_types.iter())
        .map(String::as_str)
        .chain(
            document
                .signatures
                .iter()
                .flat_map(|signature| signature.parameters.iter())
                .flat_map(|parameter| parameter.type_refs.iter().map(String::as_str)),
        )
        .chain(
            document
                .signatures
                .iter()
                .flat_map(|signature| signature.return_types.iter().map(String::as_str)),
        )
}

fn validate_document_id_collisions(documents: &[SearchDocument]) -> Result<(), SearchError> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for document in documents {
        *counts.entry(document.id.as_str()).or_default() += 1;
    }
    if let Some((id, count)) = counts.into_iter().find(|(_, count)| *count > 1) {
        return Err(SearchError::DuplicateDocumentId {
            id: id.to_string(),
            count,
        });
    }
    Ok(())
}
