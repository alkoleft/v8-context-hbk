#[derive(Debug, Clone)]
struct Relation {
    source_id: String,
    target_id: String,
    edge_kind: &'static str,
    label: String,
    evidence: &'static str,
    weight: i64,
}

#[derive(Debug)]
struct DocumentDraft {
    document: SearchDocument,
    identity: DraftIdentity,
}

struct DocumentsBuild {
    documents: Vec<SearchDocument>,
    warnings: Vec<IndexBuildWarning>,
}

impl DocumentDraft {
    fn new(document: SearchDocument, identity: DraftIdentity) -> Self {
        Self { document, identity }
    }

    fn into_document(
        mut self,
        identities: &DocumentIdentities,
    ) -> Result<SearchDocument, SearchError> {
        match self.identity {
            DraftIdentity::Immediate(id) => {
                self.document.id = id;
            }
            DraftIdentity::PlatformType {
                name_primary,
                semantic,
            } => {
                self.document.id = identities.platform_type_identity_by(&name_primary, &semantic);
                self.document
                    .relation_keys
                    .push(identity_relation_key(&self.document.id));
            }
            DraftIdentity::TypeOwned { owner_identity } => {
                let owner_identity = missing_owner_identity_error(&self.document, owner_identity)?;
                self.document.id = owned_document_identity(
                    self.document.kind.as_str(),
                    &owner_identity,
                    &self.document.name.primary,
                );
                self.document.owner_relation_key = Some(identity_relation_key(&owner_identity));
            }
            DraftIdentity::QueryTable {
                name_primary,
                identifier,
                semantic,
            } => {
                self.document.id = identities.query_table_identity_by(
                    &name_primary,
                    identifier.as_deref(),
                    &semantic,
                );
                self.document
                    .relation_keys
                    .push(semantic_relation_key(&semantic, &name_primary));
                self.document
                    .relation_keys
                    .push(identity_relation_key(&self.document.id));
            }
            DraftIdentity::QueryMember { owner_identity } => {
                let owner_identity = missing_owner_identity_error(&self.document, owner_identity)?;
                self.document.id = owned_document_identity(
                    self.document.kind.as_str(),
                    &owner_identity,
                    &self.document.name.primary,
                );
                self.document.owner_relation_key = Some(identity_relation_key(&owner_identity));
            }
            DraftIdentity::Enum {
                name_primary,
                name_alias,
                source_html_path,
            } => {
                self.document.id = identities.enum_identity_by(
                    &name_primary,
                    name_alias.as_deref(),
                    &source_html_path,
                );
                self.document
                    .relation_keys
                    .push(identity_relation_key(&self.document.id));
            }
            DraftIdentity::EnumValue { owner_identity } => {
                let owner_identity = missing_owner_identity_error(&self.document, owner_identity)?;
                self.document.id = owned_document_identity(
                    SearchDocumentKind::EnumValue.as_str(),
                    &owner_identity,
                    &self.document.name.primary,
                );
                self.document.owner_relation_key = Some(identity_relation_key(&owner_identity));
            }
        }
        Ok(self.document)
    }
}

fn missing_owner_identity_error(
    document: &SearchDocument,
    owner_identity: Option<String>,
) -> Result<String, SearchError> {
    owner_identity.ok_or_else(|| SearchError::MissingParentIdentity {
        kind: document.kind.as_str().to_string(),
        name: document.name.primary.clone(),
        owner: document
            .owner
            .as_ref()
            .map(model::LocalizedName::display_name)
            .unwrap_or_default(),
    })
}

#[derive(Debug)]
enum DraftIdentity {
    Immediate(String),
    PlatformType {
        name_primary: String,
        semantic: model::SemanticContext,
    },
    TypeOwned {
        owner_identity: Option<String>,
    },
    QueryTable {
        name_primary: String,
        identifier: Option<String>,
        semantic: model::SemanticContext,
    },
    QueryMember {
        owner_identity: Option<String>,
    },
    Enum {
        name_primary: String,
        name_alias: Option<String>,
        source_html_path: String,
    },
    EnumValue {
        owner_identity: Option<String>,
    },
}

#[derive(Debug)]
struct PlatformTypeIdentityInput {
    identity: Option<String>,
    name_primary: String,
    semantic: model::SemanticContext,
}

#[derive(Debug)]
struct QueryTableIdentityInput {
    identity: Option<String>,
    name_primary: String,
    identifier: Option<String>,
    semantic: model::SemanticContext,
}

#[derive(Debug)]
struct EnumIdentityInput {
    identity: Option<String>,
    name_primary: String,
    name_alias: Option<String>,
    source_html_path: String,
}

fn create_schema(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = OFF;
             PRAGMA synchronous = OFF;
             PRAGMA locking_mode = EXCLUSIVE;
             PRAGMA temp_store = MEMORY;
             CREATE TABLE meta (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE documents (
                 id TEXT PRIMARY KEY,
                 kind TEXT NOT NULL,
                 kind_priority INTEGER NOT NULL,
                 name_primary TEXT NOT NULL,
                 name_alias TEXT,
                 owner_primary TEXT,
                 owner_alias TEXT,
                 signature_text TEXT NOT NULL,
                 description TEXT,
                 availability_contexts TEXT NOT NULL,
                 available_since TEXT
             );
             CREATE TABLE type_identities (
                 type_id TEXT PRIMARY KEY,
                 document_id TEXT NOT NULL REFERENCES documents(id),
                 name_primary TEXT NOT NULL,
                 name_alias TEXT
             );
             CREATE TABLE type_templates (
                 document_id TEXT PRIMARY KEY REFERENCES documents(id),
                 metadata_kind TEXT NOT NULL,
                 template_parameters TEXT NOT NULL,
                 template_family TEXT,
                 template_variant TEXT,
                 template_classification_diagnostic TEXT
             );
             CREATE TABLE members (
                 member_id TEXT PRIMARY KEY,
                 owner_type_id TEXT NOT NULL REFERENCES type_identities(type_id),
                 member_kind TEXT NOT NULL,
                 name_primary TEXT NOT NULL,
                 name_alias TEXT,
                 document_id TEXT NOT NULL REFERENCES documents(id)
             );
             CREATE TABLE callables (
                 callable_id TEXT PRIMARY KEY,
                 document_id TEXT NOT NULL REFERENCES documents(id),
                 callable_kind TEXT NOT NULL,
                 owner_type_id TEXT REFERENCES type_identities(type_id)
             );
             CREATE TABLE signatures (
                 signature_id TEXT PRIMARY KEY,
                 callable_id TEXT NOT NULL REFERENCES callables(callable_id),
                 ordinal INTEGER NOT NULL,
                 title TEXT,
                 description TEXT
             );
             CREATE TABLE parameters (
                 parameter_id TEXT PRIMARY KEY,
                 signature_id TEXT NOT NULL REFERENCES signatures(signature_id),
                 ordinal INTEGER NOT NULL,
                 name TEXT NOT NULL,
                 required INTEGER NOT NULL,
                 description TEXT
             );
             CREATE TABLE type_refs (
                 source_document_id TEXT NOT NULL REFERENCES documents(id),
                 ref_kind TEXT NOT NULL,
                 ordinal INTEGER NOT NULL,
                 source_signature_id TEXT REFERENCES signatures(signature_id),
                 source_signature_ordinal INTEGER,
                 source_parameter_ordinal INTEGER,
                 target_type_name TEXT NOT NULL,
                 target_type_id TEXT REFERENCES type_identities(type_id),
                 target_resolution_status TEXT NOT NULL,
                 target_candidate_type_ids TEXT,
                 type_template_family TEXT,
                 type_template_variant TEXT,
                 template_binding_kind TEXT,
                 template_binding_owner_parameter_index INTEGER,
                 template_binding_target_parameter_index INTEGER,
                 template_binding_arguments TEXT
             );
             CREATE TABLE document_names (
                 key TEXT NOT NULL,
                 key_kind TEXT NOT NULL,
                 document_id TEXT NOT NULL REFERENCES documents(id)
             );
             CREATE TABLE document_search (
                 rowid INTEGER PRIMARY KEY,
                 document_id TEXT NOT NULL REFERENCES documents(id),
                 name_primary TEXT NOT NULL,
                 name_alias TEXT,
                 owner TEXT,
                 signatures TEXT NOT NULL,
                 parameters TEXT NOT NULL,
                 type_names TEXT NOT NULL,
                 return_names TEXT NOT NULL,
                 description TEXT
             );
             CREATE VIRTUAL TABLE document_fts USING fts5(
                 document_id UNINDEXED,
                 name_primary,
                 name_alias,
                 owner,
                 signatures,
                 parameters,
                 type_names,
                 return_names,
                 description,
                 content = 'document_search',
                 content_rowid = 'rowid',
                 tokenize = 'unicode61 remove_diacritics 0'
             );
             CREATE TABLE relations (
                 source_id TEXT NOT NULL REFERENCES documents(id),
                 target_id TEXT NOT NULL REFERENCES documents(id),
                 edge_kind TEXT NOT NULL,
                 label TEXT NOT NULL,
                 evidence TEXT NOT NULL,
                 weight INTEGER NOT NULL
             );",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })
}

fn create_lookup_indexes(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    connection
        .execute_batch(
            "CREATE INDEX document_names_key_idx ON document_names(key, key_kind, document_id);
             CREATE INDEX documents_owner_member_idx ON documents(owner_primary, name_primary);
             CREATE INDEX type_identities_name_idx ON type_identities(name_primary, name_alias, type_id);
             CREATE INDEX type_identities_document_idx ON type_identities(document_id);
             CREATE INDEX type_templates_template_key_idx ON type_templates(
                template_family, template_variant, document_id
             );
             CREATE INDEX members_owner_idx ON members(owner_type_id, member_kind, name_primary, document_id);
             CREATE INDEX members_document_owner_idx ON members(document_id, owner_type_id);
             CREATE INDEX callables_document_idx ON callables(document_id, callable_kind);
             CREATE INDEX callables_document_owner_idx ON callables(document_id, owner_type_id);
             CREATE INDEX signatures_callable_idx ON signatures(callable_id, ordinal);
             CREATE INDEX parameters_signature_idx ON parameters(signature_id, ordinal);
             CREATE INDEX type_refs_source_idx ON type_refs(source_document_id, ref_kind, ordinal);
             CREATE INDEX type_refs_signature_idx ON type_refs(source_signature_id, source_parameter_ordinal, ordinal);
             CREATE INDEX type_refs_target_idx ON type_refs(target_type_id, ref_kind, source_document_id);
             CREATE INDEX relations_source_idx ON relations(source_id, edge_kind, target_id);
             CREATE INDEX relations_target_idx ON relations(target_id, edge_kind, source_id);",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })
}

fn write_metadata(
    connection: &Connection,
    path: &Path,
    metadata: &IndexMetadata,
) -> Result<(), SearchError> {
    let built_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());
    for (key, value) in [
        ("schema_version", INDEX_SCHEMA_VERSION.to_string()),
        ("locale", metadata.locale.clone()),
        ("source_locale", metadata.source_locale.clone()),
        ("source_hbk", metadata.source_hbk.clone()),
        (
            "source_extraction_schema_version",
            metadata.source_extraction_schema_version.to_string(),
        ),
        ("built_at", built_at),
        ("builder_version", env!("CARGO_PKG_VERSION").to_string()),
    ] {
        connection
            .execute(
                "INSERT INTO meta(key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}

fn insert_documents(
    connection: &Connection,
    path: &Path,
    documents: &[SearchDocument],
) -> Result<(), SearchError> {
    let mut document_statement = connection
        .prepare(
            "INSERT INTO documents(
                id, kind, kind_priority, name_primary, name_alias, owner_primary, owner_alias,
                signature_text, description, availability_contexts, available_since
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut name_statement = connection
        .prepare("INSERT INTO document_names(key, key_kind, document_id) VALUES (?1, ?2, ?3)")
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut fts_statement = connection
        .prepare(
            "INSERT INTO document_search(
                rowid,
                document_id, name_primary, name_alias, owner, signatures, parameters,
                type_names, return_names, description
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;

    for (index, document) in documents.iter().enumerate() {
        let rowid = (index + 1) as i64;
        let signatures = document.signature_text_lines().join("\n");
        let parameters = document.parameter_terms.join("\n");
        let type_names = document.type_refs.join("\n");
        let return_names = document
            .return_types
            .iter()
            .chain(
                document
                    .signatures
                    .iter()
                    .flat_map(|signature| signature.return_types.iter()),
            )
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let owner_primary = document.owner.as_ref().map(|owner| owner.primary.as_str());
        let owner_alias = document
            .owner
            .as_ref()
            .and_then(|owner| owner.alias.as_deref());
        document_statement
            .execute(params![
                document.id,
                document.kind.as_str(),
                kind_priority(document.kind),
                document.name.primary,
                document.name.alias,
                owner_primary,
                owner_alias,
                signatures,
                document.description,
                document.availability_contexts.join("\n"),
                document.available_since,
            ])
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
        insert_name_keys(&mut name_statement, path, document)?;
        fts_statement
            .execute(params![
                rowid,
                document.id,
                searchable_name(&document.name.primary),
                document.name.alias.as_deref().map(searchable_name),
                document
                    .owner
                    .as_ref()
                    .map(|owner| searchable_name(&owner.display_name())),
                searchable_text(&signatures),
                searchable_text(&parameters),
                searchable_text(&type_names),
                searchable_text(&return_names),
                document.description.as_deref().map(searchable_text),
            ])
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}

fn insert_normalized_facts(
    connection: &Connection,
    path: &Path,
    documents: &[SearchDocument],
) -> Result<(), SearchError> {
    let by_name = relation_lookup(documents);
    let mut type_ids_by_key = BTreeMap::new();
    let mut type_ids_by_exact_key = BTreeMap::new();
    let mut type_id_by_normalized_id = BTreeMap::new();
    let mut type_ids_by_metadata_kind = BTreeMap::new();
    let mut type_template_by_type_id = BTreeMap::new();
    for document in documents
        .iter()
        .filter(|document| document.kind.is_type_ref_target())
    {
        insert_type_lookup_key(
            &mut type_ids_by_key,
            normalize_lookup_key(&document.name.primary),
            &document.id,
        );
        insert_type_lookup_key(
            &mut type_ids_by_exact_key,
            exact_type_ref_lookup_key(&document.name.primary),
            &document.id,
        );
        if let Some(alias) = &document.name.alias {
            insert_type_lookup_key(
                &mut type_ids_by_key,
                normalize_lookup_key(alias),
                &document.id,
            );
            insert_type_lookup_key(
                &mut type_ids_by_exact_key,
                exact_type_ref_lookup_key(alias),
                &document.id,
            );
        }
        type_id_by_normalized_id.insert(normalize_lookup_key(&document.id), document.id.clone());
        if document.kind == SearchDocumentKind::PlatformType
            && let Some(metadata_kind) = &document.metadata_kind
        {
            insert_type_lookup_key(
                &mut type_ids_by_metadata_kind,
                normalize_lookup_key(metadata_kind),
                &document.id,
            );
        }
        if document.kind == SearchDocumentKind::PlatformType
            && let Some(kind) = &document.type_template_key
        {
            type_template_by_type_id.insert(
                document.id.clone(),
                TypeTemplateFact {
                    key: kind.clone(),
                    parameters: document.template_parameters.clone(),
                },
            );
        }
    }

    let mut type_statement = connection
        .prepare(
            "INSERT INTO type_identities(type_id, document_id, name_primary, name_alias)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut type_template_statement = connection
        .prepare(
            "INSERT INTO type_templates(
                document_id, metadata_kind, template_parameters, template_family,
                template_variant, template_classification_diagnostic
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut member_statement = connection
        .prepare(
            "INSERT INTO members(member_id, owner_type_id, member_kind, name_primary, name_alias, document_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut callable_statement = connection
        .prepare(
            "INSERT INTO callables(callable_id, document_id, callable_kind, owner_type_id)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut signature_statement = connection
        .prepare(
            "INSERT INTO signatures(signature_id, callable_id, ordinal, title, description)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut parameter_statement = connection
        .prepare(
            "INSERT INTO parameters(parameter_id, signature_id, ordinal, name, required, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut type_ref_statement = connection
        .prepare(
            "INSERT INTO type_refs(
                source_document_id, ref_kind, ordinal, source_signature_id,
                source_signature_ordinal, source_parameter_ordinal, target_type_name,
                target_type_id, target_resolution_status, target_candidate_type_ids,
                type_template_family, type_template_variant, template_binding_kind,
                template_binding_owner_parameter_index, template_binding_target_parameter_index,
                template_binding_arguments
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;

    for document in documents
        .iter()
        .filter(|document| document.kind.is_type_ref_target())
    {
        type_statement
            .execute(params![
                document.id,
                document.id,
                document.name.primary,
                document.name.alias,
            ])
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
        if document.kind == SearchDocumentKind::PlatformType
            && let Some(metadata_kind) = &document.metadata_kind
        {
            let type_template_key = document.type_template_key.as_ref();
            type_template_statement
                .execute(params![
                    document.id,
                    metadata_kind,
                    document.template_parameters.join("\n"),
                    type_template_key.map(|kind| kind.family.as_str()),
                    type_template_key.map(|kind| kind.variant.as_str()),
                    document.type_template_classification_diagnostic.as_deref(),
                ])
                .map_err(|source| SearchError::Sqlite {
                    path: path.to_path_buf(),
                    source,
                })?;
        }
    }

    for document in documents {
        let owner_type_id = owner_type_id(document, &by_name, &type_id_by_normalized_id);

        if let Some(owner_type_id) = owner_type_id.as_deref()
            && matches!(
                document.kind,
                SearchDocumentKind::TypeMethod
                    | SearchDocumentKind::TypeProperty
                    | SearchDocumentKind::Constructor
                    | SearchDocumentKind::TypeEvent
            )
        {
            member_statement
                .execute(params![
                    document.id,
                    owner_type_id,
                    document.kind.as_str(),
                    document.name.primary,
                    document.name.alias,
                    document.id,
                ])
                .map_err(|source| SearchError::Sqlite {
                    path: path.to_path_buf(),
                    source,
                })?;
        }

        if document.kind.is_callable() {
            callable_statement
                .execute(params![
                    document.id,
                    document.id,
                    document.kind.as_str(),
                    owner_type_id
                ])
                .map_err(|source| SearchError::Sqlite {
                    path: path.to_path_buf(),
                    source,
                })?;
            for (signature_ordinal, signature) in document.signatures.iter().enumerate() {
                let signature_id = signature_id(&document.id, signature_ordinal);
                signature_statement
                    .execute(params![
                        signature_id,
                        document.id,
                        signature_ordinal as i64,
                        signature.title,
                        signature.description,
                    ])
                    .map_err(|source| SearchError::Sqlite {
                        path: path.to_path_buf(),
                        source,
                    })?;
                for (parameter_ordinal, parameter) in signature.parameters.iter().enumerate() {
                    let parameter_id = parameter_id(&signature_id, parameter_ordinal);
                    parameter_statement
                        .execute(params![
                            parameter_id,
                            signature_id,
                            parameter_ordinal as i64,
                            parameter.name,
                            parameter.required,
                            parameter.description,
                        ])
                        .map_err(|source| SearchError::Sqlite {
                            path: path.to_path_buf(),
                            source,
                        })?;
                    for (type_ordinal, type_name) in parameter.type_refs.iter().enumerate() {
                        insert_type_ref(
                            &mut type_ref_statement,
                            path,
                            &type_ids_by_key,
                            &type_ids_by_exact_key,
                            &type_ids_by_metadata_kind,
                            &type_template_by_type_id,
                            TypeRefRow {
                                source_document_id: &document.id,
                                ref_kind: "parameter_type",
                                ordinal: type_ordinal,
                                owner_type_id: owner_type_id.as_deref(),
                                source_signature_id: Some(&signature_id),
                                source_signature_ordinal: Some(signature_ordinal),
                                source_parameter_ordinal: Some(parameter_ordinal),
                                target_type_name: type_name,
                            },
                        )?;
                    }
                }
                for (type_ordinal, type_name) in signature.return_types.iter().enumerate() {
                    insert_type_ref(
                        &mut type_ref_statement,
                        path,
                        &type_ids_by_key,
                        &type_ids_by_exact_key,
                        &type_ids_by_metadata_kind,
                        &type_template_by_type_id,
                        TypeRefRow {
                            source_document_id: &document.id,
                            ref_kind: "return_type",
                            ordinal: type_ordinal,
                            owner_type_id: owner_type_id.as_deref(),
                            source_signature_id: Some(&signature_id),
                            source_signature_ordinal: Some(signature_ordinal),
                            source_parameter_ordinal: None,
                            target_type_name: type_name,
                        },
                    )?;
                }
            }
        }

        for (ordinal, type_name) in document.type_refs.iter().enumerate() {
            insert_type_ref(
                &mut type_ref_statement,
                path,
                &type_ids_by_key,
                &type_ids_by_exact_key,
                &type_ids_by_metadata_kind,
                &type_template_by_type_id,
                TypeRefRow {
                    source_document_id: &document.id,
                    ref_kind: document.kind.type_ref_kind(),
                    ordinal,
                    owner_type_id: owner_type_id.as_deref(),
                    source_signature_id: None,
                    source_signature_ordinal: None,
                    source_parameter_ordinal: None,
                    target_type_name: type_name,
                },
            )?;
        }
        for (ordinal, type_name) in document.return_types.iter().enumerate() {
            insert_type_ref(
                &mut type_ref_statement,
                path,
                &type_ids_by_key,
                &type_ids_by_exact_key,
                &type_ids_by_metadata_kind,
                &type_template_by_type_id,
                TypeRefRow {
                    source_document_id: &document.id,
                    ref_kind: "return_type",
                    ordinal,
                    owner_type_id: owner_type_id.as_deref(),
                    source_signature_id: None,
                    source_signature_ordinal: None,
                    source_parameter_ordinal: None,
                    target_type_name: type_name,
                },
            )?;
        }
        if document.kind == SearchDocumentKind::Constructor
            && let (Some(owner), Some(owner_type_id)) = (&document.owner, owner_type_id.as_deref())
        {
            insert_type_ref(
                &mut type_ref_statement,
                path,
                &type_ids_by_key,
                &type_ids_by_exact_key,
                &type_ids_by_metadata_kind,
                &type_template_by_type_id,
                TypeRefRow {
                    source_document_id: &document.id,
                    ref_kind: "constructor_result",
                    ordinal: 0,
                    owner_type_id: Some(owner_type_id),
                    source_signature_id: None,
                    source_signature_ordinal: None,
                    source_parameter_ordinal: None,
                    target_type_name: &owner.primary,
                },
            )?;
            connection
                .execute(
                    "UPDATE type_refs
                     SET target_type_id = ?1,
                         target_resolution_status = 'ok',
                         target_candidate_type_ids = NULL
                     WHERE source_document_id = ?2 AND ref_kind = 'constructor_result'",
                    params![owner_type_id, document.id],
                )
                .map_err(|source| SearchError::Sqlite {
                    path: path.to_path_buf(),
                    source,
                })?;
        }
    }
    Ok(())
}

struct TypeRefRow<'a> {
    source_document_id: &'a str,
    ref_kind: &'a str,
    ordinal: usize,
    owner_type_id: Option<&'a str>,
    source_signature_id: Option<&'a str>,
    source_signature_ordinal: Option<usize>,
    source_parameter_ordinal: Option<usize>,
    target_type_name: &'a str,
}

fn insert_type_ref(
    statement: &mut Statement<'_>,
    path: &Path,
    type_ids_by_key: &BTreeMap<String, BTreeSet<String>>,
    type_ids_by_exact_key: &BTreeMap<String, BTreeSet<String>>,
    type_ids_by_metadata_kind: &BTreeMap<String, BTreeSet<String>>,
    type_template_by_type_id: &BTreeMap<String, TypeTemplateFact>,
    row: TypeRefRow<'_>,
) -> Result<(), SearchError> {
    let target = resolve_type_ref_target(
        row.target_type_name,
        type_ids_by_key,
        type_ids_by_exact_key,
        type_ids_by_metadata_kind,
    );
    let target_type_id = target.target_type_id();
    let candidate_type_ids = target.candidate_type_ids().join("\n");
    let type_template_key =
        target_type_id.and_then(|type_id| type_template_by_type_id.get(type_id));
    let template_binding =
        type_template_binding(row.owner_type_id, target_type_id, type_template_by_type_id);
    statement
        .execute(params![
            row.source_document_id,
            row.ref_kind,
            row.ordinal as i64,
            row.source_signature_id,
            row.source_signature_ordinal.map(|value| value as i64),
            row.source_parameter_ordinal.map(|value| value as i64),
            row.target_type_name,
            target_type_id,
            target_resolution_status(&target),
            (!candidate_type_ids.is_empty()).then_some(candidate_type_ids),
            type_template_key.map(|fact| fact.key.family.as_str()),
            type_template_key.map(|fact| fact.key.variant.as_str()),
            template_binding.as_ref().map(|_| "owner_parameter"),
            template_binding
                .as_ref()
                .and_then(|binding| binding.arguments.first())
                .map(|argument| argument.owner_parameter_index as i64),
            template_binding
                .as_ref()
                .and_then(|binding| binding.arguments.first())
                .map(|argument| argument.target_parameter_index as i64),
            template_binding.as_ref().map(binding_arguments_to_storage),
        ])
        .map(|_| ())
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })
}

#[derive(Debug, Clone)]
struct OwnerParameterBinding {
    arguments: Vec<OwnerParameterBindingArgument>,
}

#[derive(Debug, Clone, Copy)]
struct OwnerParameterBindingArgument {
    owner_parameter_index: usize,
    target_parameter_index: usize,
}

fn type_template_binding(
    owner_type_id: Option<&str>,
    target_type_id: Option<&str>,
    type_template_by_type_id: &BTreeMap<String, TypeTemplateFact>,
) -> Option<OwnerParameterBinding> {
    let owner = type_template_by_type_id.get(owner_type_id?)?;
    let target = type_template_by_type_id.get(target_type_id?)?;
    if owner.key.family != target.key.family {
        return None;
    }
    let mut owner_parameters = BTreeMap::<&str, usize>::new();
    for (index, parameter) in owner.parameters.iter().enumerate() {
        if owner_parameters.insert(parameter.as_str(), index).is_some() {
            return None;
        }
    }
    let mut arguments = Vec::new();
    for (target_parameter_index, parameter) in target.parameters.iter().enumerate() {
        let owner_parameter_index = *owner_parameters.get(parameter.as_str())?;
        arguments.push(OwnerParameterBindingArgument {
            owner_parameter_index,
            target_parameter_index,
        });
    }
    (!arguments.is_empty()).then_some(OwnerParameterBinding { arguments })
}

fn binding_arguments_to_storage(binding: &OwnerParameterBinding) -> String {
    binding
        .arguments
        .iter()
        .map(|argument| {
            format!(
                "{}:{}",
                argument.owner_parameter_index, argument.target_parameter_index
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn insert_type_lookup_key(
    type_ids_by_key: &mut BTreeMap<String, BTreeSet<String>>,
    key: String,
    type_id: &str,
) {
    type_ids_by_key
        .entry(key)
        .or_default()
        .insert(type_id.to_string());
}

fn resolve_type_ref_target(
    target_type_name: &str,
    type_ids_by_key: &BTreeMap<String, BTreeSet<String>>,
    type_ids_by_exact_key: &BTreeMap<String, BTreeSet<String>>,
    type_ids_by_metadata_kind: &BTreeMap<String, BTreeSet<String>>,
) -> SearchTypeRefTarget {
    let key = normalize_lookup_key(target_type_name);
    let mut candidates = BTreeSet::new();
    let mut has_metadata_candidates = false;
    if let Some(ids) = type_ids_by_key.get(&key) {
        candidates.extend(ids.iter().cloned());
    }
    if let Some(ids) = type_ids_by_metadata_kind.get(&key) {
        has_metadata_candidates = !ids.is_empty();
        candidates.extend(ids.iter().cloned());
    }
    let candidates = candidates.into_iter().collect::<Vec<_>>();
    if !has_metadata_candidates
        && candidates.len() > 1
        && let Some(exact_candidates) =
            exact_type_ref_candidates(target_type_name, type_ids_by_exact_key, &candidates)
    {
        return exact_candidates;
    }
    match candidates.as_slice() {
        [] => SearchTypeRefTarget::Unresolved,
        [type_id] => SearchTypeRefTarget::Ok(type_id.clone()),
        _ => SearchTypeRefTarget::Ambiguous(candidates),
    }
}

fn exact_type_ref_candidates(
    target_type_name: &str,
    type_ids_by_exact_key: &BTreeMap<String, BTreeSet<String>>,
    normalized_candidates: &[String],
) -> Option<SearchTypeRefTarget> {
    let exact = type_ids_by_exact_key.get(&exact_type_ref_lookup_key(target_type_name))?;
    let exact = exact
        .iter()
        .filter(|candidate| normalized_candidates.contains(candidate))
        .cloned()
        .collect::<Vec<_>>();
    match exact.as_slice() {
        [type_id] => Some(SearchTypeRefTarget::Ok(type_id.clone())),
        [] => None,
        _ => Some(SearchTypeRefTarget::Ambiguous(exact)),
    }
}

fn target_resolution_status(target: &SearchTypeRefTarget) -> &'static str {
    match target {
        SearchTypeRefTarget::Ok(_) => "ok",
        SearchTypeRefTarget::Unresolved => "unresolved",
        SearchTypeRefTarget::Ambiguous(_) => "ambiguous",
    }
}

fn owner_type_id(
    document: &SearchDocument,
    by_name: &BTreeMap<String, (&SearchDocument, String)>,
    type_id_by_normalized_id: &BTreeMap<String, String>,
) -> Option<String> {
    document
        .owner_relation_key
        .as_deref()
        .and_then(|key| by_name.get(key))
        .map(|(_, id)| id)
        .and_then(|id| type_id_by_normalized_id.get(&normalize_lookup_key(id)))
        .cloned()
}

fn edge_ref_kind(edge_kind: &str) -> &'static str {
    match edge_kind {
        "returns" => "return_type",
        "constructs" => "constructor_result",
        _ => "property_type",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeReferenceStatus {
    Resolved,
    Unresolved,
    Ambiguous,
}

#[derive(Debug, Clone)]
struct TypeReferenceMeasurementRow {
    source_document_id: String,
    role: String,
    target_type_name: String,
    status: TypeReferenceStatus,
    candidate_type_ids: Vec<String>,
    has_template_binding: bool,
    source_kind: SearchDocumentKind,
    source_name: model::LocalizedName,
    source_owner: Option<model::LocalizedName>,
}

#[derive(Debug, Default)]
struct TypeReferenceCounts {
    total: usize,
    resolved: usize,
    unresolved: usize,
    ambiguous: usize,
    template_bindings: usize,
}

impl TypeReferenceCounts {
    fn add(&mut self, status: TypeReferenceStatus, has_template_binding: bool) {
        self.total += 1;
        match status {
            TypeReferenceStatus::Resolved => self.resolved += 1,
            TypeReferenceStatus::Unresolved => self.unresolved += 1,
            TypeReferenceStatus::Ambiguous => self.ambiguous += 1,
        }
        if has_template_binding {
            self.template_bindings += 1;
        }
    }
}

#[derive(Debug, Default)]
struct GapAccumulator {
    count: usize,
    examples: BTreeMap<String, TypeReferenceGapExample>,
    candidate_type_ids: Vec<String>,
}

fn accumulate_gap(
    gaps: &mut BTreeMap<(String, String), GapAccumulator>,
    row: TypeReferenceMeasurementRow,
) {
    let key = (row.role.clone(), row.target_type_name.clone());
    let gap = gaps.entry(key).or_default();
    gap.count += 1;
    if gap.candidate_type_ids.is_empty() {
        gap.candidate_type_ids = row.candidate_type_ids.clone();
    }
    gap.examples
        .entry(row.source_document_id.clone())
        .or_insert(TypeReferenceGapExample {
            source_document_id: row.source_document_id,
            source_kind: row.source_kind,
            source_name: row.source_name,
            source_owner: row.source_owner,
        });
}

fn type_reference_status_from_storage(value: String) -> rusqlite::Result<TypeReferenceStatus> {
    match value.as_str() {
        "ok" => Ok(TypeReferenceStatus::Resolved),
        "unresolved" => Ok(TypeReferenceStatus::Unresolved),
        "ambiguous" => Ok(TypeReferenceStatus::Ambiguous),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn top_gaps(
    gaps: BTreeMap<(String, String), GapAccumulator>,
    limit: usize,
) -> Vec<TypeReferenceGap> {
    let mut gaps = gaps
        .into_iter()
        .map(|((role, target_type_name), gap)| TypeReferenceGap {
            role,
            target_type_name,
            count: gap.count,
            examples: gap.examples.into_values().take(3).collect(),
            candidate_type_ids: gap.candidate_type_ids,
        })
        .collect::<Vec<_>>();
    gaps.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.role.cmp(&right.role))
            .then_with(|| left.target_type_name.cmp(&right.target_type_name))
    });
    gaps.truncate(limit);
    gaps
}

fn signature_id(document_id: &str, ordinal: usize) -> String {
    format!("{document_id}:signature:{ordinal}")
}

fn parameter_id(signature_id: &str, ordinal: usize) -> String {
    format!("{signature_id}:parameter:{ordinal}")
}

fn rebuild_document_fts(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    connection
        .execute(
            "INSERT INTO document_fts(document_fts) VALUES ('rebuild')",
            [],
        )
        .map(|_| ())
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })
}

fn insert_name_keys(
    statement: &mut Statement<'_>,
    path: &Path,
    document: &SearchDocument,
) -> Result<(), SearchError> {
    let mut keys = BTreeSet::new();
    keys.insert((normalize_lookup_key(&document.name.primary), "primary"));
    if let Some(alias) = &document.name.alias {
        keys.insert((normalize_lookup_key(alias), "alias"));
    }
    if let Some(owner) = &document.owner {
        keys.insert((
            owner_member_key(&owner.primary, &document.name.primary),
            "owner_member_primary",
        ));
        if let Some(owner_alias) = &owner.alias {
            keys.insert((
                owner_member_key(owner_alias, &document.name.primary),
                "owner_member_alias",
            ));
        }
        if let Some(name_alias) = &document.name.alias {
            keys.insert((
                owner_member_key(&owner.primary, name_alias),
                "owner_member_alias",
            ));
            if let Some(owner_alias) = &owner.alias {
                keys.insert((
                    owner_member_key(owner_alias, name_alias),
                    "owner_member_alias",
                ));
            }
        }
    }
    for key in document
        .relation_keys
        .iter()
        .filter(|key| key.starts_with("module_context:"))
    {
        keys.insert((key.clone(), "module_context"));
    }
    for (key, kind) in keys {
        statement
            .execute(params![key, kind, document.id])
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}
