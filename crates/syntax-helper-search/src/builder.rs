#[derive(Debug, Default)]
pub struct SearchIndexBuilder {
    drafts: Vec<DocumentDraft>,
    platform_types: Vec<PlatformTypeIdentityInput>,
    query_tables: Vec<QueryTableIdentityInput>,
    enums: Vec<EnumIdentityInput>,
}

impl SearchIndexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_language_fact(&mut self, fact: language::LanguageFact) {
        self.drafts.push(DocumentDraft::new(
            language_document(&fact),
            DraftIdentity::Immediate(fact.id),
        ));
    }

    fn into_documents(self, source_locale: &str) -> Result<DocumentsBuild, SearchError> {
        let identities =
            DocumentIdentities::from_inputs(&self.platform_types, &self.query_tables, &self.enums);
        let mut documents = self
            .drafts
            .into_iter()
            .map(|draft| draft.into_document(&identities))
            .collect::<Result<Vec<_>, _>>()?;
        documents.sort_by(|left, right| {
            kind_priority(left.kind)
                .cmp(&kind_priority(right.kind))
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut warnings = deduplicate_documents(&mut documents);
        warnings.extend(classify_platform_type_templates(
            &mut documents,
            source_locale,
        ));
        validate_document_id_collisions(&documents)?;
        Ok(DocumentsBuild {
            documents,
            warnings,
        })
    }
}

impl model::SyntaxHelperSink for SearchIndexBuilder {
    type Error = Infallible;

    fn global_context(&mut self, _record: model::GlobalContext) -> Result<(), Self::Error> {
        Ok(())
    }

    fn global_method(&mut self, record: model::GlobalMethod) -> Result<(), Self::Error> {
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::GlobalMethod,
                None,
                &record.name,
                &record.signatures,
                &record.return_types,
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::Immediate(document_identity(
                SearchDocumentKind::GlobalMethod.as_str(),
                None,
                &record.name,
            )),
        ));
        Ok(())
    }

    fn global_property(&mut self, record: model::GlobalProperty) -> Result<(), Self::Error> {
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::GlobalProperty,
                None,
                &record.name,
                &[],
                &[],
                &record.type_refs,
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::Immediate(document_identity(
                SearchDocumentKind::GlobalProperty.as_str(),
                None,
                &record.name,
            )),
        ));
        Ok(())
    }

    fn global_context_event(
        &mut self,
        record: model::GlobalContextEvent,
    ) -> Result<(), Self::Error> {
        let kind = match record.semantic.record_family {
            model::RecordFamily::ModuleEvent => SearchDocumentKind::ModuleEvent,
            model::RecordFamily::TypeEvent => SearchDocumentKind::TypeEvent,
            _ => SearchDocumentKind::UnknownEvent,
        };
        let owner = match kind {
            SearchDocumentKind::TypeEvent => None,
            _ => event_owner(&record),
        };
        let document = document(
            kind,
            owner.as_ref(),
            &record.name,
            &record.signatures,
            &[],
            &[],
            record.description.as_deref(),
            String::new(),
        )
        .with_section_facts(&record.facts);
        let identity = if kind == SearchDocumentKind::TypeEvent {
            DraftIdentity::TypeOwned {
                owner_identity: record.owner_identity,
            }
        } else {
            DraftIdentity::Immediate(document_identity(
                kind.as_str(),
                owner.as_ref(),
                &record.name,
            ))
        };
        self.drafts.push(DocumentDraft::new(document, identity));
        Ok(())
    }

    fn platform_type(&mut self, record: model::PlatformType) -> Result<(), Self::Error> {
        self.platform_types.push(PlatformTypeIdentityInput {
            identity: record.identity.clone(),
            name_primary: record.name.primary.clone(),
            semantic: record.semantic.clone(),
        });
        let mut document = document(
            SearchDocumentKind::PlatformType,
            None,
            &record.name,
            &[],
            &[],
            &record
                .extends
                .iter()
                .map(type_ref_from_name)
                .collect::<Vec<_>>(),
            record.description.as_deref(),
            String::new(),
        )
        .with_section_facts(&record.facts);
        if record.type_kind == model::PlatformTypeKind::MetadataTemplate
            && let Some(metadata_kind) = record.metadata_kind
        {
            document.metadata_kind = Some(metadata_kind);
            document.template_parameters = record.template_parameters;
            document.type_template_key = record.type_template_key;
        }
        self.drafts.push(DocumentDraft::new(
            document,
            DraftIdentity::PlatformType {
                name_primary: record.name.primary,
                semantic: record.semantic,
            },
        ));
        Ok(())
    }

    fn query_table(&mut self, record: model::QueryTable) -> Result<(), Self::Error> {
        let name = model::LocalizedName {
            primary: record.name.clone(),
            alias: record
                .syntax
                .as_ref()
                .and_then(|syntax| syntax.alias.clone()),
        };
        self.query_tables.push(QueryTableIdentityInput {
            identity: record.identity.clone(),
            name_primary: record.name.clone(),
            identifier: record.identifier.clone(),
            semantic: record.semantic.clone(),
        });
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::QueryTable,
                None,
                &name,
                &[],
                &[],
                &[],
                record.description.as_deref(),
                String::new(),
            ),
            DraftIdentity::QueryTable {
                name_primary: record.name,
                identifier: record.identifier,
                semantic: record.semantic,
            },
        ));
        Ok(())
    }

    fn type_method(&mut self, record: model::PlatformMethod) -> Result<(), Self::Error> {
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::TypeMethod,
                Some(&record.owner),
                &record.name,
                &record.signatures,
                &record.return_types,
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::TypeOwned {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn type_property(&mut self, record: model::PlatformProperty) -> Result<(), Self::Error> {
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::TypeProperty,
                Some(&record.owner),
                &record.name,
                &[],
                &[],
                &record.type_refs,
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::TypeOwned {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn table_field(&mut self, record: model::QueryTableField) -> Result<(), Self::Error> {
        let name = model::LocalizedName {
            primary: record.name,
            alias: None,
        };
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::QueryTableField,
                Some(&record.owner),
                &name,
                &[],
                &[],
                &record.type_refs,
                record.description.as_deref(),
                String::new(),
            ),
            DraftIdentity::QueryMember {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn table_parameter(&mut self, record: model::QueryTableParameter) -> Result<(), Self::Error> {
        let name = model::LocalizedName {
            primary: record.name,
            alias: None,
        };
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::QueryTableParameter,
                Some(&record.owner),
                &name,
                &[],
                &[],
                &record.type_refs,
                record.description.as_deref(),
                String::new(),
            ),
            DraftIdentity::QueryMember {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn constructor(&mut self, record: model::Constructor) -> Result<(), Self::Error> {
        let name = model::LocalizedName {
            primary: record
                .signatures
                .first()
                .map(|signature| signature.text.clone())
                .unwrap_or_else(|| format!("Новый {}", record.owner.primary)),
            alias: record.name.alias,
        };
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::Constructor,
                Some(&record.owner),
                &name,
                &record.signatures,
                &[],
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::TypeOwned {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn enum_definition(&mut self, record: model::EnumDefinition) -> Result<(), Self::Error> {
        self.enums.push(EnumIdentityInput {
            identity: record.identity.clone(),
            name_primary: record.name.primary.clone(),
            name_alias: record.name.alias.clone(),
            source_html_path: record.source.html_path.clone(),
        });
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::Enum,
                None,
                &record.name,
                &[],
                &[],
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::Enum {
                name_primary: record.name.primary,
                name_alias: record.name.alias,
                source_html_path: record.source.html_path,
            },
        ));
        Ok(())
    }

    fn enum_value(&mut self, record: model::EnumValue) -> Result<(), Self::Error> {
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::EnumValue,
                Some(&record.owner),
                &record.name,
                &[],
                &[],
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::EnumValue {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn diagnostic(&mut self, _record: model::SyntaxHelperDiagnostic) -> Result<(), Self::Error> {
        Ok(())
    }
}
