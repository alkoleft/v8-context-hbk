use super::indexes::*;
use super::*;

impl HbkFactSnapshot {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, SearchError> {
        let index = SearchIndex::open_read_only(path)?;
        Self::from_index(&index)
    }

    pub fn from_path_with_stage_timings(
        path: impl AsRef<Path>,
    ) -> Result<HbkFactSnapshotBuildReport, SearchError> {
        let total_start = Instant::now();
        let open_start = Instant::now();
        let index = SearchIndex::open_read_only(path)?;
        let open_index = open_start.elapsed();
        let mut report = Self::from_index_with_stage_timings(&index)?;
        report.timings.open_index = open_index;
        report.timings.total = total_start.elapsed();
        Ok(report)
    }

    pub fn from_index(index: &SearchIndex) -> Result<Self, SearchError> {
        SnapshotMaterializer::new(index).materialize()
    }

    pub fn from_index_with_stage_timings(
        index: &SearchIndex,
    ) -> Result<HbkFactSnapshotBuildReport, SearchError> {
        SnapshotMaterializer::new(index).materialize_with_stage_timings()
    }
}

struct DocumentRow {
    id: String,
    kind: SearchDocumentKind,
    name: model::LocalizedName,
    signature_text: String,
    availability_contexts: Vec<String>,
    available_since: Option<String>,
}

#[derive(Debug, Clone)]
struct SnapshotMetadataRow {
    owner_path: Vec<model::LocalizedName>,
    note: Option<String>,
    default_value: Option<String>,
    query_syntax: Option<model::LocalizedName>,
    query_identifier: Option<String>,
    query_table_role: Option<model::QueryTableRole>,
    template_parameters: Vec<String>,
}

#[derive(Debug, Clone)]
struct MemberRow {
    owner_type_id: String,
    member_kind: String,
    document_id: String,
}

#[derive(Debug, Clone)]
struct CallableRow {
    callable_id: String,
    document_id: String,
    callable_kind: String,
    owner_type_id: Option<String>,
}

#[derive(Debug, Clone)]
struct SignatureRow {
    signature_id: String,
    callable_id: String,
    ordinal: i64,
}

#[derive(Debug, Clone)]
struct ParameterRow {
    signature_id: String,
    ordinal: i64,
    name: String,
    required: bool,
}

#[derive(Debug, Clone)]
struct TypeRefRowSnapshot {
    source_document_id: String,
    ref_kind: String,
    source_signature_id: Option<String>,
    source_parameter_ordinal: Option<i64>,
    fact: SearchTypeRef,
}

#[derive(Default)]
pub(super) struct SnapshotBuilder {
    pub(super) strings: Vec<String>,
    pub(super) string_ids: BTreeMap<String, StringId>,
}

struct SnapshotMaterializer<'a> {
    index: &'a SearchIndex,
    builder: SnapshotBuilder,
}

impl<'a> SnapshotMaterializer<'a> {
    fn new(index: &'a SearchIndex) -> Self {
        Self {
            index,
            builder: SnapshotBuilder::default(),
        }
    }

    fn materialize(self) -> Result<HbkFactSnapshot, SearchError> {
        self.materialize_inner(None)
    }

    fn materialize_with_stage_timings(self) -> Result<HbkFactSnapshotBuildReport, SearchError> {
        let total_start = Instant::now();
        let mut timings = HbkFactSnapshotStageTimings::default();
        let cache_metadata =
            super::binary_cache::CacheMetadata::from_index(self.index.path(), self.index)?;
        let cache_index_path = self.index.path().to_path_buf();
        let snapshot = self.materialize_inner(Some(&mut timings))?;
        timings.total = total_start.elapsed();
        Ok(HbkFactSnapshotBuildReport {
            snapshot,
            timings,
            cache_index_path,
            cache_metadata,
        })
    }

    fn materialize_inner(
        mut self,
        mut timings: Option<&mut HbkFactSnapshotStageTimings>,
    ) -> Result<HbkFactSnapshot, SearchError> {
        macro_rules! start_stage {
            () => {
                timings.as_ref().map(|_| Instant::now())
            };
        }
        macro_rules! finish_stage {
            ($started_at:expr, $field:ident) => {
                if let (Some(timings), Some(started_at)) = (&mut timings, $started_at) {
                    timings.$field = started_at.elapsed();
                }
            };
        }

        let stage_start = start_stage!();
        let index_metadata = self.index.metadata()?;
        let documents = self.documents()?;
        let metadata = self.metadata_rows()?;
        let type_identities = self.type_identities()?;
        let type_templates = self.type_templates()?;
        let members = self.members()?;
        let callables = self.callables()?;
        let signatures = self.signatures()?;
        let parameters = self.parameters()?;
        let type_refs = self.type_refs()?;
        let module_context_keys = self.module_context_keys()?;
        let query_owners = self.query_owner_edges()?;
        finish_stage!(stage_start, read_sql_rows);

        let stage_start = start_stage!();
        let documents_by_id = documents
            .iter()
            .map(|document| (document.id.as_str(), document))
            .collect::<BTreeMap<_, _>>();
        let metadata_by_id = metadata
            .iter()
            .map(|(id, row)| (id.as_str(), row))
            .collect::<BTreeMap<_, _>>();
        let type_id_by_document = type_identities
            .iter()
            .map(|(type_id, document_id)| (document_id.as_str(), type_id.as_str()))
            .collect::<BTreeMap<_, _>>();
        let type_template_by_document = type_templates
            .iter()
            .map(|(document_id, key)| (document_id.as_str(), key))
            .collect::<BTreeMap<_, _>>();
        finish_stage!(stage_start, build_lookup_maps);

        let stage_start = start_stage!();
        let mut platform_types = Vec::new();
        let mut platform_type_by_type_id = BTreeMap::<String, HbkPlatformTypeId>::new();
        let mut platform_type_ids = Vec::new();
        let mut platform_type_names = Vec::new();
        let mut platform_type_templates = Vec::new();
        for document in documents
            .iter()
            .filter(|document| document.kind == SearchDocumentKind::PlatformType)
        {
            let Some(type_id) = type_id_by_document.get(document.id.as_str()) else {
                continue;
            };
            let id = HbkPlatformTypeId(platform_types.len() as u32);
            let type_template_key =
                type_template_by_document
                    .get(document.id.as_str())
                    .map(|key| HbkPlatformTypeTemplateKey {
                        family: self.builder.intern(&key.family),
                        variant: self.builder.intern(&key.variant),
                    });
            platform_type_by_type_id.insert((*type_id).to_string(), id);
            platform_types.push(HbkPlatformType {
                id: self.builder.intern(type_id),
                name: self.builder.intern_name(&document.name),
                type_template_key,
                availability_contexts: self.builder.intern_many(&document.availability_contexts),
            });
            push_id_lookup(&mut platform_type_ids, &mut self.builder, type_id, id);
            push_name_lookups(
                &mut platform_type_names,
                &mut self.builder,
                &document.name,
                id,
            );
            if let Some(key) = type_template_key {
                platform_type_templates.push(TypeTemplateLookup {
                    family: key.family,
                    variant: key.variant,
                    value: id,
                });
            }
        }
        finish_stage!(stage_start, build_platform_types);

        let stage_start = start_stage!();
        let type_refs_by_document = group_document_type_refs(&mut self.builder, &type_refs);
        let return_refs_by_document = group_return_type_refs(&mut self.builder, &type_refs);
        let signature_refs = group_signature_return_type_refs(&mut self.builder, &type_refs);
        let parameter_refs = group_parameter_type_refs(&mut self.builder, &type_refs);
        finish_stage!(stage_start, group_type_refs);

        let stage_start = start_stage!();
        let parameters_by_signature = parameters_by_signature(parameters, parameter_refs);
        let signatures_by_callable = signatures_by_callable(
            &mut self.builder,
            &documents_by_id,
            signatures,
            parameters_by_signature,
            signature_refs,
        );
        finish_stage!(stage_start, build_signatures);

        let stage_start = start_stage!();
        let mut type_members = Vec::new();
        let mut member_ids = Vec::new();
        let mut member_owner_pairs = Vec::new();
        let mut members_by_owner_name = Vec::new();
        let mut members_by_owner_name_kind = Vec::new();
        for row in members {
            let Some(document) = documents_by_id.get(row.document_id.as_str()) else {
                continue;
            };
            let Some(owner) = platform_type_by_type_id
                .get(row.owner_type_id.as_str())
                .copied()
            else {
                continue;
            };
            let Some(kind) = member_kind_from_storage(&row.member_kind) else {
                continue;
            };
            let id = HbkTypeMemberId(type_members.len() as u32);
            type_members.push(HbkTypeMember {
                id: self.builder.intern(&document.id),
                owner,
                kind,
                name: self.builder.intern_name(&document.name),
                type_refs: type_refs_by_document
                    .get(&(
                        document.id.clone(),
                        document.kind.type_ref_kind().to_string(),
                    ))
                    .cloned()
                    .unwrap_or_default(),
                availability_contexts: self.builder.intern_many(&document.availability_contexts),
            });
            push_id_lookup(&mut member_ids, &mut self.builder, &document.id, id);
            push_owner_name_lookups(
                &mut members_by_owner_name,
                &mut self.builder,
                owner,
                &document.name,
                id,
            );
            push_member_name_kind_lookups(
                &mut members_by_owner_name_kind,
                &mut self.builder,
                owner,
                &document.name,
                kind,
                id,
            );
            member_owner_pairs.push((owner, id));
        }

        let mut callables_vec = Vec::new();
        let mut callable_ids = Vec::new();
        let mut callables_by_document = BTreeMap::<String, HbkCallableId>::new();
        let mut callable_owner_pairs = Vec::new();
        let mut callables_by_owner_name = Vec::new();
        let mut constructor_owner_pairs = Vec::new();
        for row in callables {
            let Some(document) = documents_by_id.get(row.document_id.as_str()) else {
                continue;
            };
            let Some(kind) = callable_kind_from_storage(&row.callable_kind) else {
                continue;
            };
            let owner = row
                .owner_type_id
                .as_deref()
                .and_then(|owner| platform_type_by_type_id.get(owner).copied());
            let id = HbkCallableId(callables_vec.len() as u32);
            callables_vec.push(HbkCallable {
                id: self.builder.intern(&row.callable_id),
                owner,
                kind,
                name: self.builder.intern_name(&document.name),
                signatures: signatures_by_callable
                    .get(row.callable_id.as_str())
                    .cloned()
                    .unwrap_or_default(),
                return_type_refs: return_refs_by_document
                    .get(document.id.as_str())
                    .cloned()
                    .unwrap_or_default(),
                availability_contexts: self.builder.intern_many(&document.availability_contexts),
            });
            callables_by_document.insert(document.id.clone(), id);
            push_id_lookup(&mut callable_ids, &mut self.builder, &row.callable_id, id);
            if let Some(owner) = owner {
                callable_owner_pairs.push((owner, id));
                if kind == HbkCallableKind::Constructor {
                    constructor_owner_pairs.push((owner, id));
                }
                push_owner_name_lookups(
                    &mut callables_by_owner_name,
                    &mut self.builder,
                    owner,
                    &document.name,
                    id,
                );
            }
        }

        let mut globals = Vec::new();
        let mut global_names = Vec::new();
        let mut globals_by_domain_name_kind = Vec::new();
        for document in documents.iter().filter(|document| {
            matches!(
                document.kind,
                SearchDocumentKind::GlobalMethod | SearchDocumentKind::GlobalProperty
            )
        }) {
            let id = HbkGlobalFactId(globals.len() as u32);
            globals.push(HbkGlobalFact {
                id: self.builder.intern(&document.id),
                kind: if document.kind == SearchDocumentKind::GlobalMethod {
                    HbkGlobalFactKind::Method
                } else {
                    HbkGlobalFactKind::Property
                },
                domain: HbkLanguageDomain::Bsl,
                name: self.builder.intern_name(&document.name),
                callable: callables_by_document.get(document.id.as_str()).copied(),
                type_refs: type_refs_by_document
                    .get(&(
                        document.id.clone(),
                        document.kind.type_ref_kind().to_string(),
                    ))
                    .cloned()
                    .unwrap_or_default(),
            });
            push_name_lookups(&mut global_names, &mut self.builder, &document.name, id);
            let kind = globals[id.0 as usize].kind;
            push_global_name_kind_lookups(
                &mut globals_by_domain_name_kind,
                &mut self.builder,
                HbkLanguageDomain::Bsl,
                &document.name,
                kind,
                id,
            );
        }

        let mut query_tables = Vec::new();
        let mut query_table_by_document = BTreeMap::<String, HbkQueryTableId>::new();
        let mut query_table_ids = Vec::new();
        let mut query_table_names = Vec::new();
        let mut query_table_syntax_names = Vec::new();
        let mut query_table_identifiers = Vec::new();
        for document in documents
            .iter()
            .filter(|document| document.kind == SearchDocumentKind::QueryTable)
        {
            let id = HbkQueryTableId(query_tables.len() as u32);
            let meta = metadata_by_id.get(document.id.as_str()).copied();
            query_table_by_document.insert(document.id.clone(), id);
            query_tables.push(HbkQueryTable {
                id: self.builder.intern(&document.id),
                name: self.builder.intern_name(&document.name),
                syntax: meta.and_then(|row| {
                    row.query_syntax
                        .as_ref()
                        .map(|name| self.builder.intern_name(name))
                }),
                identifier: meta
                    .and_then(|row| self.builder.intern_option(row.query_identifier.as_deref())),
                role: meta.and_then(|row| row.query_table_role),
                owner_path: meta
                    .map(|row| {
                        row.owner_path
                            .iter()
                            .map(|name| self.builder.intern_name(name))
                            .collect()
                    })
                    .unwrap_or_default(),
                template_parameters: meta
                    .map(|row| self.builder.intern_many(&row.template_parameters))
                    .unwrap_or_default(),
            });
            push_id_lookup(&mut query_table_ids, &mut self.builder, &document.id, id);
            push_name_lookups(
                &mut query_table_names,
                &mut self.builder,
                &document.name,
                id,
            );
            if let Some(meta) = meta {
                if let Some(query_syntax) = &meta.query_syntax {
                    push_name_lookups(
                        &mut query_table_syntax_names,
                        &mut self.builder,
                        query_syntax,
                        id,
                    );
                }
                if let Some(identifier) = &meta.query_identifier {
                    push_lookup(
                        &mut query_table_identifiers,
                        &mut self.builder,
                        &normalize_lookup_key(identifier),
                        id,
                    );
                }
            }
        }

        let mut query_fields = Vec::new();
        let mut query_field_owner_pairs = Vec::new();
        let mut query_fields_by_table_name = Vec::new();
        let mut query_parameters = Vec::new();
        let mut query_parameter_owner_pairs = Vec::new();
        let mut query_parameters_by_table_name = Vec::new();
        for (target_id, source_id) in &query_owners {
            let Some(document) = documents_by_id.get(target_id.as_str()) else {
                continue;
            };
            let Some(owner) = query_table_by_document.get(source_id.as_str()).copied() else {
                continue;
            };
            let meta = metadata_by_id.get(document.id.as_str()).copied();
            match document.kind {
                SearchDocumentKind::QueryTableField => {
                    let id = HbkQueryFieldId(query_fields.len() as u32);
                    query_fields.push(HbkQueryField {
                        id: self.builder.intern(&document.id),
                        owner,
                        name: self.builder.intern_name(&document.name),
                        type_refs: type_refs_by_document
                            .get(&(
                                document.id.clone(),
                                document.kind.type_ref_kind().to_string(),
                            ))
                            .cloned()
                            .unwrap_or_default(),
                        note: meta.and_then(|row| self.builder.intern_option(row.note.as_deref())),
                    });
                    query_field_owner_pairs.push((owner, id));
                    push_owner_name_lookups(
                        &mut query_fields_by_table_name,
                        &mut self.builder,
                        owner,
                        &document.name,
                        id,
                    );
                }
                SearchDocumentKind::QueryTableParameter => {
                    let id = HbkQueryParameterId(query_parameters.len() as u32);
                    query_parameters.push(HbkQueryParameter {
                        id: self.builder.intern(&document.id),
                        owner,
                        name: self.builder.intern_name(&document.name),
                        type_refs: type_refs_by_document
                            .get(&(
                                document.id.clone(),
                                document.kind.type_ref_kind().to_string(),
                            ))
                            .cloned()
                            .unwrap_or_default(),
                        default_value: meta.and_then(|row| {
                            self.builder.intern_option(row.default_value.as_deref())
                        }),
                    });
                    query_parameter_owner_pairs.push((owner, id));
                    push_owner_name_lookups(
                        &mut query_parameters_by_table_name,
                        &mut self.builder,
                        owner,
                        &document.name,
                        id,
                    );
                }
                _ => {}
            }
        }

        let mut language_facts = Vec::new();
        let mut language_ids = Vec::new();
        let mut language_names = Vec::new();
        for document in documents
            .iter()
            .filter(|document| document.kind.is_language())
        {
            let id = HbkLanguageFactId(language_facts.len() as u32);
            language_facts.push(HbkLanguageFact {
                id: self.builder.intern(&document.id),
                kind: document.kind,
                domain: language_domain_from_document_id(&document.id),
                name: self.builder.intern_name(&document.name),
                signatures: signatures_by_callable
                    .get(document.id.as_str())
                    .cloned()
                    .unwrap_or_default(),
                type_refs: type_refs_by_document
                    .get(&(
                        document.id.clone(),
                        document.kind.type_ref_kind().to_string(),
                    ))
                    .cloned()
                    .unwrap_or_default(),
                return_type_refs: return_refs_by_document
                    .get(document.id.as_str())
                    .cloned()
                    .unwrap_or_default(),
            });
            push_id_lookup(&mut language_ids, &mut self.builder, &document.id, id);
            push_name_lookups(&mut language_names, &mut self.builder, &document.name, id);
        }

        let mut enums = Vec::new();
        let mut enum_by_document = BTreeMap::<String, HbkEnumId>::new();
        let mut enum_ids = Vec::new();
        let mut enum_names = Vec::new();
        for document in documents
            .iter()
            .filter(|document| document.kind == SearchDocumentKind::Enum)
        {
            let id = HbkEnumId(enums.len() as u32);
            enum_by_document.insert(document.id.clone(), id);
            enums.push(HbkEnum {
                id: self.builder.intern(&document.id),
                name: self.builder.intern_name(&document.name),
            });
            push_id_lookup(&mut enum_ids, &mut self.builder, &document.id, id);
            push_name_lookups(&mut enum_names, &mut self.builder, &document.name, id);
        }

        let mut enum_values = Vec::new();
        let mut enum_value_ids = Vec::new();
        let mut enum_value_owner_pairs = Vec::new();
        let mut enum_values_by_enum_name = Vec::new();
        for (target_id, source_id) in &query_owners {
            let Some(document) = documents_by_id.get(target_id.as_str()) else {
                continue;
            };
            if document.kind != SearchDocumentKind::EnumValue {
                continue;
            }
            let Some(owner) = enum_by_document.get(source_id.as_str()).copied() else {
                continue;
            };
            let id = HbkEnumValueId(enum_values.len() as u32);
            enum_values.push(HbkEnumValue {
                id: self.builder.intern(&document.id),
                owner,
                name: self.builder.intern_name(&document.name),
            });
            push_id_lookup(&mut enum_value_ids, &mut self.builder, &document.id, id);
            enum_value_owner_pairs.push((owner, id));
            push_owner_name_lookups(
                &mut enum_values_by_enum_name,
                &mut self.builder,
                owner,
                &document.name,
                id,
            );
        }
        drop(query_owners);

        let mut module_event_names = Vec::new();
        let mut module_contexts_by_domain_language_kind = Vec::new();
        for (document_id, context_key) in module_context_keys {
            let Some(callable) = callables_by_document.get(document_id.as_str()).copied() else {
                continue;
            };
            let normalized_context_key = normalize_lookup_key(&context_key);
            push_lookup_with_owner_key(
                &mut module_event_names,
                &mut self.builder,
                &normalized_context_key,
                &normalized_context_key,
                callable,
            );
            let module_kind = normalized_context_key
                .strip_prefix("module_context:")
                .unwrap_or(normalized_context_key.as_str());
            module_contexts_by_domain_language_kind.push(ModuleContextLookup {
                domain: HbkLanguageDomain::Bsl,
                language_key: self.builder.intern("bsl"),
                module_kind: self.builder.intern(module_kind),
                value: callable,
            });
        }
        finish_stage!(stage_start, build_fact_arenas);

        let stage_start = start_stage!();
        let mut fact_ids = Vec::new();
        let mut fact_by_id = BTreeMap::<StringId, Vec<HbkFactRef>>::new();
        collect_fact_ids(
            &mut fact_ids,
            &mut fact_by_id,
            &self.builder,
            FactIdSources {
                platform_types: &platform_types,
                type_members: &type_members,
                callables: &callables_vec,
                globals: &globals,
                query_tables: &query_tables,
                query_fields: &query_fields,
                query_parameters: &query_parameters,
                language_facts: &language_facts,
                enums: &enums,
                enum_values: &enum_values,
            },
        );
        let relation_pairs = relation_pairs(self.index, &mut self.builder, &fact_by_id)?;
        let (availability_pairs, availability_since_by_fact) =
            availability_pairs(&mut self.builder, &fact_by_id, &documents);
        finish_stage!(stage_start, build_fact_ids_relations_availability);

        let stage_start = start_stage!();
        let fact_ids = sorted_id_lookup(fact_ids, &self.builder);
        let platform_type_ids = sorted_id_lookup(platform_type_ids, &self.builder);
        let platform_type_names = sorted_name_lookup(platform_type_names, &self.builder);
        let platform_type_templates =
            sorted_type_template_lookup(platform_type_templates, &self.builder);
        let member_ids = sorted_id_lookup(member_ids, &self.builder);
        let members_by_owner_name = sorted_owner_name_lookup(members_by_owner_name, &self.builder);
        let members_by_owner_name_kind =
            sorted_member_name_kind_lookup(members_by_owner_name_kind, &self.builder);
        let callable_ids = sorted_id_lookup(callable_ids, &self.builder);
        let callables_by_owner_name =
            sorted_owner_name_lookup(callables_by_owner_name, &self.builder);
        let global_names = sorted_name_lookup(global_names, &self.builder);
        let globals_by_domain_name_kind =
            sorted_global_name_kind_lookup(globals_by_domain_name_kind, &self.builder);
        let module_event_names = sorted_string_owner_name_lookup(module_event_names, &self.builder);
        let module_contexts_by_domain_language_kind =
            sorted_module_context_lookup(module_contexts_by_domain_language_kind, &self.builder);
        let query_table_ids = sorted_id_lookup(query_table_ids, &self.builder);
        let query_table_names = sorted_name_lookup(query_table_names, &self.builder);
        let query_table_syntax_names = sorted_name_lookup(query_table_syntax_names, &self.builder);
        let query_table_identifiers = sorted_name_lookup(query_table_identifiers, &self.builder);
        let query_fields_by_table_name =
            sorted_owner_name_lookup(query_fields_by_table_name, &self.builder);
        let query_parameters_by_table_name =
            sorted_owner_name_lookup(query_parameters_by_table_name, &self.builder);
        let language_ids = sorted_id_lookup(language_ids, &self.builder);
        let language_names = sorted_name_lookup(language_names, &self.builder);
        let enum_ids = sorted_id_lookup(enum_ids, &self.builder);
        let enum_names = sorted_name_lookup(enum_names, &self.builder);
        let enum_value_ids = sorted_id_lookup(enum_value_ids, &self.builder);
        let enum_values_by_enum_name =
            sorted_owner_name_lookup(enum_values_by_enum_name, &self.builder);
        finish_stage!(stage_start, sort_secondary_indexes);

        let stage_start = start_stage!();
        let source_locale = Some(self.builder.intern(&index_metadata.source_locale));
        let snapshot = HbkFactSnapshot {
            strings: self.builder.strings,
            source_locale,
            platform_types,
            type_members,
            callables: callables_vec,
            globals,
            query_tables,
            query_fields,
            query_parameters,
            language_facts,
            enums,
            enum_values,
            fact_ids,
            platform_type_ids,
            platform_type_names,
            platform_type_templates,
            member_ids,
            members_by_owner: CsrIndex::from_pairs(member_owner_pairs),
            members_by_owner_name,
            members_by_owner_name_kind,
            callable_ids,
            callables_by_owner: CsrIndex::from_pairs(callable_owner_pairs),
            callables_by_owner_name,
            constructors_by_type: CsrIndex::from_pairs(constructor_owner_pairs),
            global_names,
            globals_by_domain_name_kind,
            module_event_names,
            module_contexts_by_domain_language_kind,
            query_table_ids,
            query_table_names,
            query_table_syntax_names,
            query_table_identifiers,
            query_fields_by_table: CsrIndex::from_pairs(query_field_owner_pairs),
            query_fields_by_table_name,
            query_parameters_by_table: CsrIndex::from_pairs(query_parameter_owner_pairs),
            query_parameters_by_table_name,
            language_ids,
            language_names,
            enum_ids,
            enum_names,
            enum_value_ids,
            enum_values_by_enum: CsrIndex::from_pairs(enum_value_owner_pairs),
            enum_values_by_enum_name,
            availability_by_fact: CsrIndex::from_pairs(availability_pairs),
            availability_since_by_fact,
            relations_by_source_kind: CsrIndex::from_pairs(relation_pairs),
        };
        finish_stage!(stage_start, assemble_snapshot);
        Ok(snapshot)
    }

    fn documents(&self) -> Result<Vec<DocumentRow>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT id, kind, name_primary, name_alias, owner_primary, owner_alias,
                        signature_text, availability_contexts, available_since
                 FROM documents
                 ORDER BY kind_priority, id",
            )
            .map_err(|source| self.index.sqlite(source))?;
        let rows = statement
            .query_map([], |row| {
                let kind_value: String = row.get(1)?;
                let kind = SearchDocumentKind::from_storage(&kind_value)
                    .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
                Ok(DocumentRow {
                    id: row.get(0)?,
                    kind,
                    name: model::LocalizedName {
                        primary: row.get(2)?,
                        alias: row.get(3)?,
                    },
                    signature_text: row.get(6)?,
                    availability_contexts: split_lines(row.get(7)?),
                    available_since: row.get(8)?,
                })
            })
            .map_err(|source| self.index.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.index.sqlite(source))
    }

    fn metadata_rows(&self) -> Result<Vec<(String, SnapshotMetadataRow)>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT document_id, owner_path, note, default_value, query_syntax_primary,
                        query_syntax_alias, query_identifier, query_table_role,
                        template_parameters
                 FROM document_metadata
                 ORDER BY document_id",
            )
            .map_err(|source| self.index.sqlite(source))?;
        let rows = statement
            .query_map([], |row| {
                let role: Option<String> = row.get(7)?;
                Ok((
                    row.get(0)?,
                    SnapshotMetadataRow {
                        owner_path: split_localized_names(row.get::<_, String>(1)?),
                        note: row.get(2)?,
                        default_value: row.get(3)?,
                        query_syntax: optional_localized_name(row.get(4)?, row.get(5)?),
                        query_identifier: row.get(6)?,
                        query_table_role: role.as_deref().and_then(query_table_role_from_code),
                        template_parameters: split_lines(row.get(8)?),
                    },
                ))
            })
            .map_err(|source| self.index.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.index.sqlite(source))
    }

    fn type_identities(&self) -> Result<Vec<(String, String)>, SearchError> {
        collect_pairs(
            self.index,
            "SELECT type_id, document_id FROM type_identities ORDER BY type_id",
        )
    }

    fn type_templates(&self) -> Result<Vec<(String, model::PlatformTypeTemplateKey)>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT document_id, template_family, template_variant
                 FROM type_templates
                 WHERE template_family IS NOT NULL
                   AND template_variant IS NOT NULL
                 ORDER BY template_family, template_variant, document_id",
            )
            .map_err(|source| self.index.sqlite(source))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    model::PlatformTypeTemplateKey::new(
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ),
                ))
            })
            .map_err(|source| self.index.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.index.sqlite(source))
    }

    fn members(&self) -> Result<Vec<MemberRow>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT owner_type_id, member_kind, document_id
                 FROM members
                 ORDER BY owner_type_id, member_kind, name_primary, document_id",
            )
            .map_err(|source| self.index.sqlite(source))?;
        let rows = statement
            .query_map([], |row| {
                Ok(MemberRow {
                    owner_type_id: row.get(0)?,
                    member_kind: row.get(1)?,
                    document_id: row.get(2)?,
                })
            })
            .map_err(|source| self.index.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.index.sqlite(source))
    }

    fn callables(&self) -> Result<Vec<CallableRow>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT callable_id, document_id, callable_kind, owner_type_id
                 FROM callables
                 ORDER BY callable_kind, document_id",
            )
            .map_err(|source| self.index.sqlite(source))?;
        let rows = statement
            .query_map([], |row| {
                Ok(CallableRow {
                    callable_id: row.get(0)?,
                    document_id: row.get(1)?,
                    callable_kind: row.get(2)?,
                    owner_type_id: row.get(3)?,
                })
            })
            .map_err(|source| self.index.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.index.sqlite(source))
    }

    fn signatures(&self) -> Result<Vec<SignatureRow>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT signature_id, callable_id, ordinal
                 FROM signatures
                 ORDER BY callable_id, ordinal",
            )
            .map_err(|source| self.index.sqlite(source))?;
        let rows = statement
            .query_map([], |row| {
                Ok(SignatureRow {
                    signature_id: row.get(0)?,
                    callable_id: row.get(1)?,
                    ordinal: row.get(2)?,
                })
            })
            .map_err(|source| self.index.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.index.sqlite(source))
    }

    fn parameters(&self) -> Result<Vec<ParameterRow>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT signature_id, ordinal, name, required
                 FROM parameters
                 ORDER BY signature_id, ordinal",
            )
            .map_err(|source| self.index.sqlite(source))?;
        let rows = statement
            .query_map([], |row| {
                Ok(ParameterRow {
                    signature_id: row.get(0)?,
                    ordinal: row.get(1)?,
                    name: row.get(2)?,
                    required: row.get(3)?,
                })
            })
            .map_err(|source| self.index.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.index.sqlite(source))
    }

    fn type_refs(&self) -> Result<Vec<TypeRefRowSnapshot>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT source_document_id, ref_kind, source_signature_id,
                        source_parameter_ordinal, target_type_name, target_type_id,
                        target_resolution_status, target_candidate_type_ids,
                        type_template_family, type_template_variant, template_binding_kind,
                        template_binding_owner_parameter_index,
                        template_binding_target_parameter_index, template_binding_arguments
                 FROM type_refs
                 ORDER BY source_document_id, ref_kind, source_signature_ordinal,
                          source_parameter_ordinal, ordinal, target_type_name",
            )
            .map_err(|source| self.index.sqlite(source))?;
        let rows = statement
            .query_map([], |row| {
                Ok(TypeRefRowSnapshot {
                    source_document_id: row.get(0)?,
                    ref_kind: row.get(1)?,
                    source_signature_id: row.get(2)?,
                    source_parameter_ordinal: row.get(3)?,
                    fact: snapshot_type_ref_from_row(row)?,
                })
            })
            .map_err(|source| self.index.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.index.sqlite(source))
    }

    fn module_context_keys(&self) -> Result<Vec<(String, String)>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT document_id, key
                 FROM document_names
                 WHERE key LIKE 'module_context:%'
                 ORDER BY key, document_id",
            )
            .map_err(|source| self.index.sqlite(source))?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|source| self.index.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.index.sqlite(source))
    }

    fn query_owner_edges(&self) -> Result<Vec<(String, String)>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT target_id, source_id
                 FROM relations
                 WHERE edge_kind = 'owns'
                 ORDER BY source_id, target_id",
            )
            .map_err(|source| self.index.sqlite(source))?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|source| self.index.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.index.sqlite(source))
    }
}

impl SnapshotBuilder {
    pub(super) fn intern(&mut self, value: &str) -> StringId {
        if let Some(id) = self.string_ids.get(value).copied() {
            return id;
        }
        let id = StringId(self.strings.len() as u32);
        self.strings.push(value.to_string());
        self.string_ids.insert(value.to_string(), id);
        id
    }

    pub(super) fn intern_option(&mut self, value: Option<&str>) -> Option<StringId> {
        value.map(|value| self.intern(value))
    }

    pub(super) fn intern_many(&mut self, values: &[String]) -> Vec<StringId> {
        values.iter().map(|value| self.intern(value)).collect()
    }

    pub(super) fn intern_name(&mut self, name: &model::LocalizedName) -> HbkName {
        HbkName {
            primary: self.intern(&name.primary),
            alias: name.alias.as_deref().map(|alias| self.intern(alias)),
        }
    }

    pub(super) fn string(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }
}
fn collect_pairs(index: &SearchIndex, query: &str) -> Result<Vec<(String, String)>, SearchError> {
    let mut statement = index
        .connection
        .prepare(query)
        .map_err(|source| index.sqlite(source))?;
    let rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|source| index.sqlite(source))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|source| index.sqlite(source))
}

struct FactIdSources<'a> {
    platform_types: &'a [HbkPlatformType],
    type_members: &'a [HbkTypeMember],
    callables: &'a [HbkCallable],
    globals: &'a [HbkGlobalFact],
    query_tables: &'a [HbkQueryTable],
    query_fields: &'a [HbkQueryField],
    query_parameters: &'a [HbkQueryParameter],
    language_facts: &'a [HbkLanguageFact],
    enums: &'a [HbkEnum],
    enum_values: &'a [HbkEnumValue],
}

fn collect_fact_ids(
    output: &mut Vec<IdLookup<HbkFactRef>>,
    by_id: &mut BTreeMap<StringId, Vec<HbkFactRef>>,
    builder: &SnapshotBuilder,
    sources: FactIdSources<'_>,
) {
    collect_fact_family_ids(
        output,
        by_id,
        builder,
        sources
            .platform_types
            .iter()
            .enumerate()
            .map(|(index, fact)| {
                (
                    fact.id,
                    HbkFactRef::PlatformType(HbkPlatformTypeId(index as u32)),
                )
            }),
    );
    collect_fact_family_ids(
        output,
        by_id,
        builder,
        sources
            .type_members
            .iter()
            .enumerate()
            .map(|(index, fact)| {
                (
                    fact.id,
                    HbkFactRef::TypeMember(HbkTypeMemberId(index as u32)),
                )
            }),
    );
    collect_fact_family_ids(
        output,
        by_id,
        builder,
        sources
            .callables
            .iter()
            .enumerate()
            .map(|(index, fact)| (fact.id, HbkFactRef::Callable(HbkCallableId(index as u32)))),
    );
    collect_fact_family_ids(
        output,
        by_id,
        builder,
        sources
            .globals
            .iter()
            .enumerate()
            .map(|(index, fact)| (fact.id, HbkFactRef::Global(HbkGlobalFactId(index as u32)))),
    );
    collect_fact_family_ids(
        output,
        by_id,
        builder,
        sources
            .query_tables
            .iter()
            .enumerate()
            .map(|(index, fact)| {
                (
                    fact.id,
                    HbkFactRef::QueryTable(HbkQueryTableId(index as u32)),
                )
            }),
    );
    collect_fact_family_ids(
        output,
        by_id,
        builder,
        sources
            .query_fields
            .iter()
            .enumerate()
            .map(|(index, fact)| {
                (
                    fact.id,
                    HbkFactRef::QueryField(HbkQueryFieldId(index as u32)),
                )
            }),
    );
    collect_fact_family_ids(
        output,
        by_id,
        builder,
        sources
            .query_parameters
            .iter()
            .enumerate()
            .map(|(index, fact)| {
                (
                    fact.id,
                    HbkFactRef::QueryParameter(HbkQueryParameterId(index as u32)),
                )
            }),
    );
    collect_fact_family_ids(
        output,
        by_id,
        builder,
        sources
            .language_facts
            .iter()
            .enumerate()
            .map(|(index, fact)| {
                (
                    fact.id,
                    HbkFactRef::LanguageFact(HbkLanguageFactId(index as u32)),
                )
            }),
    );
    collect_fact_family_ids(
        output,
        by_id,
        builder,
        sources
            .enums
            .iter()
            .enumerate()
            .map(|(index, fact)| (fact.id, HbkFactRef::Enum(HbkEnumId(index as u32)))),
    );
    collect_fact_family_ids(
        output,
        by_id,
        builder,
        sources
            .enum_values
            .iter()
            .enumerate()
            .map(|(index, fact)| (fact.id, HbkFactRef::EnumValue(HbkEnumValueId(index as u32)))),
    );
}

fn collect_fact_family_ids(
    output: &mut Vec<IdLookup<HbkFactRef>>,
    by_id: &mut BTreeMap<StringId, Vec<HbkFactRef>>,
    _builder: &SnapshotBuilder,
    facts: impl IntoIterator<Item = (StringId, HbkFactRef)>,
) {
    for (id, fact_ref) in facts {
        output.push(IdLookup {
            key: id,
            value: fact_ref,
        });
        by_id.entry(id).or_default().push(fact_ref);
    }
}

fn availability_pairs(
    builder: &mut SnapshotBuilder,
    fact_by_id: &BTreeMap<StringId, Vec<HbkFactRef>>,
    documents: &[DocumentRow],
) -> (Vec<(HbkFactRef, StringId)>, Vec<FactStringLookup>) {
    let mut contexts = Vec::new();
    let mut available_since = Vec::new();
    for document in documents {
        let Some(document_id) = builder.string_ids.get(&document.id).copied() else {
            continue;
        };
        let Some(fact_refs) = fact_by_id.get(&document_id) else {
            continue;
        };
        for fact_ref in fact_refs {
            for context in &document.availability_contexts {
                let context = builder.intern(context);
                contexts.push((*fact_ref, context));
            }
            if let Some(since) = document.available_since.as_deref() {
                available_since.push(FactStringLookup {
                    fact: *fact_ref,
                    value: builder.intern(since),
                });
            }
        }
    }
    available_since.sort_by_key(|entry| entry.fact);
    available_since.dedup_by_key(|entry| entry.fact);
    (contexts, available_since)
}

fn relation_pairs(
    index: &SearchIndex,
    builder: &mut SnapshotBuilder,
    fact_by_id: &BTreeMap<StringId, Vec<HbkFactRef>>,
) -> Result<Vec<(RelationLookupKey, HbkFactRef)>, SearchError> {
    let mut output = Vec::new();
    let mut statement = index
        .connection
        .prepare(
            "SELECT source_id, target_id, edge_kind
             FROM relations
             ORDER BY source_id, edge_kind, target_id",
        )
        .map_err(|source| index.sqlite(source))?;
    let mut rows = statement.query([]).map_err(|source| index.sqlite(source))?;
    while let Some(row) = rows.next().map_err(|source| index.sqlite(source))? {
        let source_id: String = row.get(0).map_err(|source| index.sqlite(source))?;
        let target_id: String = row.get(1).map_err(|source| index.sqlite(source))?;
        let kind: String = row.get(2).map_err(|source| index.sqlite(source))?;
        let (Some(source_id), Some(target_id)) = (
            builder.string_ids.get(&source_id).copied(),
            builder.string_ids.get(&target_id).copied(),
        ) else {
            continue;
        };
        let (Some(source_refs), Some(target_refs)) =
            (fact_by_id.get(&source_id), fact_by_id.get(&target_id))
        else {
            continue;
        };
        let kind = builder.intern(&normalize_lookup_key(&kind));
        for source in source_refs {
            for target in target_refs {
                output.push((
                    RelationLookupKey {
                        source: *source,
                        kind,
                    },
                    *target,
                ));
            }
        }
    }
    Ok(output)
}

fn snapshot_type_ref_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchTypeRef> {
    let type_template_key = type_template_key_from_codes(
        row.get::<_, Option<String>>(8)?,
        row.get::<_, Option<String>>(9)?,
    );
    let binding_kind: Option<String> = row.get(10)?;
    let owner_parameter_index: Option<i64> = row.get(11)?;
    let target_parameter_index: Option<i64> = row.get(12)?;
    let binding_arguments: Option<String> = row.get(13)?;
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
    let target_type_id: Option<String> = row.get(5)?;
    let target = match row.get::<_, String>(6)?.as_str() {
        "ok" => {
            SearchTypeRefTarget::Ok(target_type_id.ok_or_else(|| rusqlite::Error::InvalidQuery)?)
        }
        "unresolved" => SearchTypeRefTarget::Unresolved,
        "ambiguous" => {
            let candidates = row
                .get::<_, Option<String>>(7)?
                .map(|value| value.lines().map(str::to_string).collect())
                .unwrap_or_default();
            SearchTypeRefTarget::Ambiguous(candidates)
        }
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(SearchTypeRef {
        name: row.get(4)?,
        target,
        type_template_key,
        template_binding,
    })
}

fn group_document_type_refs(
    builder: &mut SnapshotBuilder,
    rows: &[TypeRefRowSnapshot],
) -> BTreeMap<(String, String), Vec<HbkTypeRef>> {
    let mut groups = BTreeMap::<(String, String), Vec<HbkTypeRef>>::new();
    for row in rows
        .iter()
        .filter(|row| row.source_signature_id.is_none() && row.ref_kind != "return_type")
    {
        groups
            .entry((row.source_document_id.clone(), row.ref_kind.clone()))
            .or_default()
            .push(map_type_ref(builder, &row.fact));
    }
    groups
}

fn group_return_type_refs(
    builder: &mut SnapshotBuilder,
    rows: &[TypeRefRowSnapshot],
) -> BTreeMap<String, Vec<HbkTypeRef>> {
    let mut groups = BTreeMap::<String, Vec<HbkTypeRef>>::new();
    for row in rows
        .iter()
        .filter(|row| row.source_signature_id.is_none() && row.ref_kind == "return_type")
    {
        groups
            .entry(row.source_document_id.clone())
            .or_default()
            .push(map_type_ref(builder, &row.fact));
    }
    groups
}

fn group_signature_return_type_refs(
    builder: &mut SnapshotBuilder,
    rows: &[TypeRefRowSnapshot],
) -> BTreeMap<String, Vec<HbkTypeRef>> {
    let mut groups = BTreeMap::<String, Vec<HbkTypeRef>>::new();
    for row in rows.iter().filter(|row| {
        row.source_signature_id.is_some()
            && row.source_parameter_ordinal.is_none()
            && row.ref_kind == "return_type"
    }) {
        groups
            .entry(row.source_signature_id.clone().unwrap_or_default())
            .or_default()
            .push(map_type_ref(builder, &row.fact));
    }
    groups
}

fn group_parameter_type_refs(
    builder: &mut SnapshotBuilder,
    rows: &[TypeRefRowSnapshot],
) -> BTreeMap<(String, i64), Vec<HbkTypeRef>> {
    let mut groups = BTreeMap::<(String, i64), Vec<HbkTypeRef>>::new();
    for row in rows.iter().filter(|row| row.ref_kind == "parameter_type") {
        if let (Some(signature_id), Some(ordinal)) = (
            row.source_signature_id.as_deref(),
            row.source_parameter_ordinal,
        ) {
            groups
                .entry((signature_id.to_string(), ordinal))
                .or_default()
                .push(map_type_ref(builder, &row.fact));
        }
    }
    groups
}

fn parameters_by_signature(
    parameters: Vec<ParameterRow>,
    parameter_refs: BTreeMap<(String, i64), Vec<HbkTypeRef>>,
) -> BTreeMap<String, Vec<HbkParameterDraft>> {
    let mut output = BTreeMap::<String, Vec<HbkParameterDraft>>::new();
    for parameter in parameters {
        let type_refs = parameter_refs
            .get(&(parameter.signature_id.clone(), parameter.ordinal))
            .cloned()
            .unwrap_or_default();
        output
            .entry(parameter.signature_id)
            .or_default()
            .push(HbkParameterDraft {
                name: parameter.name,
                required: parameter.required,
                type_refs,
            });
    }
    output
}

#[derive(Debug, Clone)]
struct HbkParameterDraft {
    name: String,
    required: bool,
    type_refs: Vec<HbkTypeRef>,
}

fn signatures_by_callable(
    builder: &mut SnapshotBuilder,
    documents: &BTreeMap<&str, &DocumentRow>,
    signatures: Vec<SignatureRow>,
    mut parameters: BTreeMap<String, Vec<HbkParameterDraft>>,
    signature_refs: BTreeMap<String, Vec<HbkTypeRef>>,
) -> BTreeMap<String, Vec<HbkSignature>> {
    let mut output = BTreeMap::<String, Vec<HbkSignature>>::new();
    for signature in signatures {
        let signature_text = documents
            .get(signature.callable_id.as_str())
            .and_then(|document| {
                split_lines(document.signature_text.clone())
                    .get(signature.ordinal as usize)
                    .cloned()
            })
            .unwrap_or_default();
        let params = parameters
            .remove(&signature.signature_id)
            .unwrap_or_default()
            .into_iter()
            .map(|parameter| HbkParameter {
                name: builder.intern(&parameter.name),
                required: parameter.required,
                type_refs: parameter.type_refs,
            })
            .collect();
        output
            .entry(signature.callable_id)
            .or_default()
            .push(HbkSignature {
                text: builder.intern(&signature_text),
                parameters: params,
                return_type_refs: signature_refs
                    .get(signature.signature_id.as_str())
                    .cloned()
                    .unwrap_or_default(),
            });
    }
    for signatures in output.values_mut() {
        signatures.sort_by_key(|signature| signature.text);
    }
    output
}

fn map_type_ref(builder: &mut SnapshotBuilder, value: &SearchTypeRef) -> HbkTypeRef {
    HbkTypeRef {
        name: builder.intern(&value.name),
        target: match &value.target {
            SearchTypeRefTarget::Ok(type_id) => HbkTypeRefTarget::Ok(builder.intern(type_id)),
            SearchTypeRefTarget::Unresolved => HbkTypeRefTarget::Unresolved,
            SearchTypeRefTarget::Ambiguous(candidates) => HbkTypeRefTarget::Ambiguous(
                candidates
                    .iter()
                    .map(|candidate| builder.intern(candidate))
                    .collect(),
            ),
        },
        type_template_key: value
            .type_template_key
            .as_ref()
            .map(|key| HbkPlatformTypeTemplateKey {
                family: builder.intern(&key.family),
                variant: builder.intern(&key.variant),
            }),
        template_binding: value
            .template_binding
            .as_ref()
            .map(|binding| HbkTypeTemplateBinding {
                template_key: HbkPlatformTypeTemplateKey {
                    family: builder.intern(&binding.template_key.family),
                    variant: builder.intern(&binding.template_key.variant),
                },
                arguments: binding.arguments.clone(),
            }),
    }
}

fn member_kind_from_storage(value: &str) -> Option<HbkTypeMemberKind> {
    match SearchDocumentKind::from_storage(value)? {
        SearchDocumentKind::TypeProperty => Some(HbkTypeMemberKind::Property),
        SearchDocumentKind::TypeMethod => Some(HbkTypeMemberKind::Method),
        SearchDocumentKind::TypeEvent => Some(HbkTypeMemberKind::Event),
        SearchDocumentKind::EnumValue => Some(HbkTypeMemberKind::EnumValue),
        SearchDocumentKind::PlatformType
        | SearchDocumentKind::Constructor
        | SearchDocumentKind::GlobalMethod
        | SearchDocumentKind::GlobalProperty
        | SearchDocumentKind::ModuleEvent
        | SearchDocumentKind::UnknownEvent
        | SearchDocumentKind::QueryTable
        | SearchDocumentKind::QueryTableField
        | SearchDocumentKind::QueryTableParameter
        | SearchDocumentKind::LanguageType
        | SearchDocumentKind::LanguageConstruct
        | SearchDocumentKind::LanguageFunction
        | SearchDocumentKind::LanguageOperator
        | SearchDocumentKind::LanguageKeyword
        | SearchDocumentKind::LanguageLiteral
        | SearchDocumentKind::Enum => None,
    }
}

fn callable_kind_from_storage(value: &str) -> Option<HbkCallableKind> {
    match SearchDocumentKind::from_storage(value)? {
        SearchDocumentKind::TypeMethod => Some(HbkCallableKind::Method),
        SearchDocumentKind::Constructor => Some(HbkCallableKind::Constructor),
        SearchDocumentKind::GlobalMethod => Some(HbkCallableKind::GlobalMethod),
        SearchDocumentKind::ModuleEvent
        | SearchDocumentKind::TypeEvent
        | SearchDocumentKind::UnknownEvent => Some(HbkCallableKind::Event),
        SearchDocumentKind::LanguageFunction => Some(HbkCallableKind::LanguageFunction),
        SearchDocumentKind::PlatformType
        | SearchDocumentKind::TypeProperty
        | SearchDocumentKind::GlobalProperty
        | SearchDocumentKind::QueryTable
        | SearchDocumentKind::QueryTableField
        | SearchDocumentKind::QueryTableParameter
        | SearchDocumentKind::LanguageType
        | SearchDocumentKind::LanguageConstruct
        | SearchDocumentKind::LanguageOperator
        | SearchDocumentKind::LanguageKeyword
        | SearchDocumentKind::LanguageLiteral
        | SearchDocumentKind::Enum
        | SearchDocumentKind::EnumValue => None,
    }
}

fn language_domain_from_document_id(id: &str) -> HbkLanguageDomain {
    if id.starts_with("shlang:") {
        HbkLanguageDomain::Bsl
    } else if id.starts_with("shquery:") {
        HbkLanguageDomain::Query
    } else if id.starts_with("dcsui:") {
        HbkLanguageDomain::DataComposition
    } else {
        HbkLanguageDomain::Unknown
    }
}
