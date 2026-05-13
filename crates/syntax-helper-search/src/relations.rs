fn insert_relations_from_documents(
    connection: &Connection,
    path: &Path,
    documents: &[SearchDocument],
) -> Result<(), SearchError> {
    let mut statement = connection
        .prepare(
            "INSERT INTO relations(source_id, target_id, edge_kind, label, evidence, weight)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    visit_relations_from_documents(documents, |relation| {
        statement
            .execute(params![
                relation.source_id,
                relation.target_id,
                relation.edge_kind,
                relation.label,
                relation.evidence,
                relation.weight,
            ])
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
        Ok(())
    })
}

fn visit_relations_from_documents<E>(
    documents: &[SearchDocument],
    mut visit: impl FnMut(Relation) -> Result<(), E>,
) -> Result<(), E> {
    let by_name = relation_lookup(documents);
    let type_ref_targets_by_key = type_ref_target_lookup(documents);
    let document_ids = documents
        .iter()
        .map(|document| document.id.as_str())
        .collect::<HashSet<_>>();
    let mut emitted = HashSet::new();
    for document in documents {
        visit_document_relations(
            document,
            &by_name,
            &type_ref_targets_by_key,
            &document_ids,
            |relation| {
                if emitted.insert((
                    relation.source_id.clone(),
                    relation.target_id.clone(),
                    relation.edge_kind,
                )) {
                    visit(relation)?;
                }
                Ok(())
            },
        )?;
    }
    Ok(())
}

fn visit_document_relations<E>(
    document: &SearchDocument,
    by_name: &BTreeMap<String, (&SearchDocument, String)>,
    type_ref_targets_by_key: &BTreeMap<String, BTreeSet<String>>,
    document_ids: &HashSet<&str>,
    mut visit: impl FnMut(Relation) -> Result<(), E>,
) -> Result<(), E> {
    visit_owner_relations(document, by_name, &mut visit)?;
    visit_constructor_relation(document, by_name, &mut visit)?;
    visit_type_reference_relations(document, type_ref_targets_by_key, document_ids, &mut visit)
}

fn visit_owner_relations<E>(
    document: &SearchDocument,
    by_name: &BTreeMap<String, (&SearchDocument, String)>,
    visit: &mut impl FnMut(Relation) -> Result<(), E>,
) -> Result<(), E> {
    let Some(owner) = &document.owner else {
        return Ok(());
    };
    let owner_key = document
        .owner_relation_key
        .as_deref()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| normalize_lookup_key(&owner.primary));
    let Some((_, owner_id)) = by_name.get(&owner_key) else {
        return Ok(());
    };
    visit(Relation {
        source_id: owner_id.clone(),
        target_id: document.id.clone(),
        edge_kind: "owns",
        label: format!("{} owns {}", owner.display_name(), document.name.primary),
        evidence: "owner",
        weight: 10,
    })?;
    visit(Relation {
        source_id: document.id.clone(),
        target_id: owner_id.clone(),
        edge_kind: "member_of",
        label: format!(
            "{} member of {}",
            document.name.primary,
            owner.display_name()
        ),
        evidence: "owner",
        weight: 20,
    })
}

fn visit_constructor_relation<E>(
    document: &SearchDocument,
    by_name: &BTreeMap<String, (&SearchDocument, String)>,
    visit: &mut impl FnMut(Relation) -> Result<(), E>,
) -> Result<(), E> {
    if document.kind != SearchDocumentKind::Constructor {
        return Ok(());
    }
    let (Some(owner), Some(owner_key)) = (&document.owner, document.owner_relation_key.as_deref())
    else {
        return Ok(());
    };
    let Some((_, owner_id)) = by_name.get(owner_key) else {
        return Ok(());
    };
    visit(Relation {
        source_id: document.id.clone(),
        target_id: owner_id.clone(),
        edge_kind: "constructs",
        label: format!("constructs {}", owner.display_name()),
        evidence: "structured",
        weight: 15,
    })
}

fn visit_type_reference_relations<E>(
    document: &SearchDocument,
    type_ref_targets_by_key: &BTreeMap<String, BTreeSet<String>>,
    document_ids: &HashSet<&str>,
    visit: &mut impl FnMut(Relation) -> Result<(), E>,
) -> Result<(), E> {
    for (ordinal, type_name) in document.type_refs.iter().enumerate() {
        let Some(target_id) = explicit_or_unique_type_ref_target_id(
            document,
            &document.explicit_type_ref_ids,
            ordinal,
            type_name,
            type_ref_targets_by_key,
            document_ids,
        ) else {
            continue;
        };
        visit(Relation {
            source_id: document.id.clone(),
            target_id,
            edge_kind: "has_type",
            label: type_name.clone(),
            evidence: "type_ref",
            weight: TYPE_REFERENCE_RELATION_WEIGHT,
        })?;
    }
    for (ordinal, type_name) in document.return_types.iter().enumerate() {
        let Some(target_id) = explicit_or_unique_type_ref_target_id(
            document,
            &document.explicit_return_type_ref_ids,
            ordinal,
            type_name,
            type_ref_targets_by_key,
            document_ids,
        ) else {
            continue;
        };
        visit(Relation {
            source_id: document.id.clone(),
            target_id,
            edge_kind: "returns",
            label: type_name.clone(),
            evidence: "type_ref",
            weight: TYPE_REFERENCE_RELATION_WEIGHT,
        })?;
    }
    for signature in &document.signatures {
        for type_name in &signature.return_types {
            let Some(target_ids) = type_ref_targets_by_key.get(&normalize_lookup_key(type_name))
            else {
                continue;
            };
            if target_ids.len() != 1 {
                continue;
            }
            let target_id = target_ids.iter().next().cloned().unwrap_or_default();
            if !document_ids.contains(target_id.as_str()) {
                continue;
            }
            visit(Relation {
                source_id: document.id.clone(),
                target_id,
                edge_kind: "returns",
                label: type_name.clone(),
                evidence: "type_ref",
                weight: TYPE_REFERENCE_RELATION_WEIGHT,
            })?;
        }
    }
    Ok(())
}

fn explicit_or_unique_type_ref_target_id(
    document: &SearchDocument,
    explicit_ids: &[Option<String>],
    ordinal: usize,
    type_name: &str,
    type_ref_targets_by_key: &BTreeMap<String, BTreeSet<String>>,
    document_ids: &HashSet<&str>,
) -> Option<String> {
    if let Some(Some(id)) = explicit_ids.get(ordinal) {
        return document_ids.contains(id.as_str()).then(|| id.clone());
    }
    if document.kind.is_language() {
        return None;
    }
    let key = normalize_lookup_key(type_name);
    let ids = type_ref_targets_by_key.get(&key)?;
    if ids.len() == 1 {
        ids.iter().next().cloned()
    } else {
        None
    }
}

fn validate_index(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    let count: i64 = connection
        .query_row("SELECT count(*) FROM documents", [], |row| row.get(0))
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    if count == 0 {
        return Err(SearchError::Sqlite {
            path: path.to_path_buf(),
            source: rusqlite::Error::QueryReturnedNoRows,
        });
    }
    Ok(())
}
