#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkPlatformTypeId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkTypeMemberId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkCallableId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkGlobalFactId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkQueryTableId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkQueryFieldId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkQueryParameterId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkLanguageFactId(u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkName {
    pub primary: StringId,
    pub alias: Option<StringId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HbkTypeMemberKind {
    Property,
    Method,
    Event,
    EnumValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HbkCallableKind {
    Method,
    Constructor,
    GlobalMethod,
    Event,
    LanguageFunction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HbkGlobalFactKind {
    Method,
    Property,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HbkLanguageDomain {
    Bsl,
    Query,
    DataComposition,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkPlatformType {
    pub id: StringId,
    pub name: HbkName,
    pub availability_contexts: Vec<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkTypeMember {
    pub id: StringId,
    pub owner: HbkPlatformTypeId,
    pub kind: HbkTypeMemberKind,
    pub name: HbkName,
    pub type_refs: Vec<HbkTypeRef>,
    pub availability_contexts: Vec<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkCallable {
    pub id: StringId,
    pub owner: Option<HbkPlatformTypeId>,
    pub kind: HbkCallableKind,
    pub name: HbkName,
    pub signatures: Vec<HbkSignature>,
    pub return_type_refs: Vec<HbkTypeRef>,
    pub availability_contexts: Vec<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkSignature {
    pub text: StringId,
    pub parameters: Vec<HbkParameter>,
    pub return_type_refs: Vec<HbkTypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkParameter {
    pub name: StringId,
    pub required: bool,
    pub type_refs: Vec<HbkTypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkGlobalFact {
    pub id: StringId,
    pub kind: HbkGlobalFactKind,
    pub name: HbkName,
    pub callable: Option<HbkCallableId>,
    pub type_refs: Vec<HbkTypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkQueryTable {
    pub id: StringId,
    pub name: HbkName,
    pub syntax: Option<HbkName>,
    pub identifier: Option<StringId>,
    pub role: Option<model::QueryTableRole>,
    pub owner_path: Vec<HbkName>,
    pub template_parameters: Vec<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkQueryField {
    pub id: StringId,
    pub owner: HbkQueryTableId,
    pub name: HbkName,
    pub type_refs: Vec<HbkTypeRef>,
    pub note: Option<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkQueryParameter {
    pub id: StringId,
    pub owner: HbkQueryTableId,
    pub name: HbkName,
    pub type_refs: Vec<HbkTypeRef>,
    pub default_value: Option<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkLanguageFact {
    pub id: StringId,
    pub kind: SearchDocumentKind,
    pub domain: HbkLanguageDomain,
    pub name: HbkName,
    pub signatures: Vec<HbkSignature>,
    pub type_refs: Vec<HbkTypeRef>,
    pub return_type_refs: Vec<HbkTypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkTypeRef {
    pub name: StringId,
    pub target: HbkTypeRefTarget,
    pub type_template_key: Option<HbkPlatformTypeTemplateKey>,
    pub template_binding: Option<model::TypeTemplateBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HbkTypeRefTarget {
    Ok(StringId),
    Unresolved,
    Ambiguous(Vec<StringId>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkPlatformTypeTemplateKey {
    pub family: StringId,
    pub variant: StringId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IdLookup<T> {
    key: StringId,
    value: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NameLookup<T> {
    key: StringId,
    value: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OwnerNameLookup<Owner, Value> {
    owner: Owner,
    key: StringId,
    value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CsrIndex<K, V> {
    keys: Vec<K>,
    offsets: Vec<u32>,
    values: Vec<V>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkFactSnapshot {
    strings: Vec<String>,
    platform_types: Vec<HbkPlatformType>,
    type_members: Vec<HbkTypeMember>,
    callables: Vec<HbkCallable>,
    globals: Vec<HbkGlobalFact>,
    query_tables: Vec<HbkQueryTable>,
    query_fields: Vec<HbkQueryField>,
    query_parameters: Vec<HbkQueryParameter>,
    language_facts: Vec<HbkLanguageFact>,
    platform_type_ids: Vec<IdLookup<HbkPlatformTypeId>>,
    platform_type_names: Vec<NameLookup<HbkPlatformTypeId>>,
    member_ids: Vec<IdLookup<HbkTypeMemberId>>,
    members_by_owner: CsrIndex<HbkPlatformTypeId, HbkTypeMemberId>,
    members_by_owner_name: Vec<OwnerNameLookup<HbkPlatformTypeId, HbkTypeMemberId>>,
    callable_ids: Vec<IdLookup<HbkCallableId>>,
    callables_by_owner: CsrIndex<HbkPlatformTypeId, HbkCallableId>,
    callables_by_owner_name: Vec<OwnerNameLookup<HbkPlatformTypeId, HbkCallableId>>,
    global_names: Vec<NameLookup<HbkGlobalFactId>>,
    module_event_names: Vec<OwnerNameLookup<StringId, HbkCallableId>>,
    query_table_ids: Vec<IdLookup<HbkQueryTableId>>,
    query_table_names: Vec<NameLookup<HbkQueryTableId>>,
    query_fields_by_table: CsrIndex<HbkQueryTableId, HbkQueryFieldId>,
    query_parameters_by_table: CsrIndex<HbkQueryTableId, HbkQueryParameterId>,
    language_ids: Vec<IdLookup<HbkLanguageFactId>>,
    language_names: Vec<NameLookup<HbkLanguageFactId>>,
}

#[derive(Debug, Clone, Copy)]
pub struct HbkFactReadHandle<'a> {
    snapshot: &'a HbkFactSnapshot,
}

#[derive(Debug, Clone)]
struct DocumentRow {
    id: String,
    kind: SearchDocumentKind,
    name: model::LocalizedName,
    signature_text: String,
    availability_contexts: Vec<String>,
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
struct SnapshotBuilder {
    strings: Vec<String>,
    string_ids: BTreeMap<String, StringId>,
}

impl HbkFactSnapshot {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, SearchError> {
        let index = SearchIndex::open_read_only(path)?;
        Self::from_index(&index)
    }

    pub fn from_index(index: &SearchIndex) -> Result<Self, SearchError> {
        SnapshotMaterializer::new(index).materialize()
    }

    pub fn worker_handle(&self) -> HbkFactReadHandle<'_> {
        HbkFactReadHandle { snapshot: self }
    }

    pub fn string(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }

    pub fn platform_type(&self, id: HbkPlatformTypeId) -> &HbkPlatformType {
        &self.platform_types[id.0 as usize]
    }

    pub fn type_member(&self, id: HbkTypeMemberId) -> &HbkTypeMember {
        &self.type_members[id.0 as usize]
    }

    pub fn callable(&self, id: HbkCallableId) -> &HbkCallable {
        &self.callables[id.0 as usize]
    }

    pub fn global_fact(&self, id: HbkGlobalFactId) -> &HbkGlobalFact {
        &self.globals[id.0 as usize]
    }

    pub fn query_table(&self, id: HbkQueryTableId) -> &HbkQueryTable {
        &self.query_tables[id.0 as usize]
    }

    pub fn query_field(&self, id: HbkQueryFieldId) -> &HbkQueryField {
        &self.query_fields[id.0 as usize]
    }

    pub fn query_parameter(&self, id: HbkQueryParameterId) -> &HbkQueryParameter {
        &self.query_parameters[id.0 as usize]
    }

    pub fn language_fact(&self, id: HbkLanguageFactId) -> &HbkLanguageFact {
        &self.language_facts[id.0 as usize]
    }

    pub fn counts(&self) -> HbkFactSnapshotCounts {
        HbkFactSnapshotCounts {
            strings: self.strings.len(),
            platform_types: self.platform_types.len(),
            type_members: self.type_members.len(),
            callables: self.callables.len(),
            globals: self.globals.len(),
            query_tables: self.query_tables.len(),
            query_fields: self.query_fields.len(),
            query_parameters: self.query_parameters.len(),
            language_facts: self.language_facts.len(),
        }
    }

    pub fn estimated_heap_bytes(&self) -> usize {
        let mut total = vec_heap_bytes(&self.strings);
        total += self
            .strings
            .iter()
            .map(|value| value.capacity())
            .sum::<usize>();
        total += vec_heap_bytes(&self.platform_types);
        total += vec_heap_bytes(&self.type_members);
        total += vec_heap_bytes(&self.callables);
        total += vec_heap_bytes(&self.globals);
        total += vec_heap_bytes(&self.query_tables);
        total += vec_heap_bytes(&self.query_fields);
        total += vec_heap_bytes(&self.query_parameters);
        total += vec_heap_bytes(&self.language_facts);
        total += self
            .type_members
            .iter()
            .map(|member| {
                vec_heap_bytes(&member.type_refs) + vec_heap_bytes(&member.availability_contexts)
                    + type_refs_heap_bytes(&member.type_refs)
            })
            .sum::<usize>();
        total += self
            .callables
            .iter()
            .map(|callable| {
                vec_heap_bytes(&callable.signatures)
                    + callable
                        .signatures
                        .iter()
                        .map(|signature| {
                            vec_heap_bytes(&signature.parameters)
                                + vec_heap_bytes(&signature.return_type_refs)
                                + type_refs_heap_bytes(&signature.return_type_refs)
                                + signature
                                    .parameters
                                    .iter()
                                    .map(|parameter| {
                                        vec_heap_bytes(&parameter.type_refs)
                                            + type_refs_heap_bytes(&parameter.type_refs)
                                    })
                                    .sum::<usize>()
                        })
                        .sum::<usize>()
                    + vec_heap_bytes(&callable.return_type_refs)
                    + type_refs_heap_bytes(&callable.return_type_refs)
                    + vec_heap_bytes(&callable.availability_contexts)
            })
            .sum::<usize>();
        total += self
            .globals
            .iter()
            .map(|global| vec_heap_bytes(&global.type_refs) + type_refs_heap_bytes(&global.type_refs))
            .sum::<usize>();
        total += self
            .query_tables
            .iter()
            .map(|table| {
                vec_heap_bytes(&table.owner_path) + vec_heap_bytes(&table.template_parameters)
            })
            .sum::<usize>();
        total += self
            .query_fields
            .iter()
            .map(|field| vec_heap_bytes(&field.type_refs) + type_refs_heap_bytes(&field.type_refs))
            .sum::<usize>();
        total += self
            .query_parameters
            .iter()
            .map(|parameter| {
                vec_heap_bytes(&parameter.type_refs) + type_refs_heap_bytes(&parameter.type_refs)
            })
            .sum::<usize>();
        total += self
            .language_facts
            .iter()
            .map(|fact| {
                vec_heap_bytes(&fact.signatures)
                    + fact
                        .signatures
                        .iter()
                        .map(|signature| {
                            vec_heap_bytes(&signature.parameters)
                                + vec_heap_bytes(&signature.return_type_refs)
                                + type_refs_heap_bytes(&signature.return_type_refs)
                                + signature
                                    .parameters
                                    .iter()
                                    .map(|parameter| {
                                        vec_heap_bytes(&parameter.type_refs)
                                            + type_refs_heap_bytes(&parameter.type_refs)
                                    })
                                    .sum::<usize>()
                        })
                        .sum::<usize>()
                    + vec_heap_bytes(&fact.type_refs)
                    + type_refs_heap_bytes(&fact.type_refs)
                    + vec_heap_bytes(&fact.return_type_refs)
                    + type_refs_heap_bytes(&fact.return_type_refs)
            })
            .sum::<usize>();
        total += vec_heap_bytes(&self.platform_type_ids);
        total += vec_heap_bytes(&self.platform_type_names);
        total += vec_heap_bytes(&self.member_ids);
        total += self.members_by_owner.estimated_heap_bytes();
        total += vec_heap_bytes(&self.members_by_owner_name);
        total += vec_heap_bytes(&self.callable_ids);
        total += self.callables_by_owner.estimated_heap_bytes();
        total += vec_heap_bytes(&self.callables_by_owner_name);
        total += vec_heap_bytes(&self.global_names);
        total += vec_heap_bytes(&self.module_event_names);
        total += vec_heap_bytes(&self.query_table_ids);
        total += vec_heap_bytes(&self.query_table_names);
        total += self.query_fields_by_table.estimated_heap_bytes();
        total += self.query_parameters_by_table.estimated_heap_bytes();
        total += vec_heap_bytes(&self.language_ids);
        total += vec_heap_bytes(&self.language_names);
        total
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HbkFactSnapshotCounts {
    pub strings: usize,
    pub platform_types: usize,
    pub type_members: usize,
    pub callables: usize,
    pub globals: usize,
    pub query_tables: usize,
    pub query_fields: usize,
    pub query_parameters: usize,
    pub language_facts: usize,
}

impl<'a> HbkFactReadHandle<'a> {
    pub fn platform_type_by_id(&self, id: &str) -> Option<HbkPlatformTypeId> {
        lookup_id(&self.snapshot.platform_type_ids, self.snapshot, id)
    }

    pub fn platform_types_by_name(&self, name: &str) -> Vec<HbkPlatformTypeId> {
        lookup_name(&self.snapshot.platform_type_names, self.snapshot, name)
    }

    pub fn members_of_type(&self, owner: HbkPlatformTypeId) -> &[HbkTypeMemberId] {
        self.snapshot.members_by_owner.values(owner)
    }

    pub fn member_by_owner_name(
        &self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> Vec<HbkTypeMemberId> {
        lookup_owner_name(
            &self.snapshot.members_by_owner_name,
            self.snapshot,
            owner,
            name,
        )
    }

    pub fn callables_of_type(&self, owner: HbkPlatformTypeId) -> &[HbkCallableId] {
        self.snapshot.callables_by_owner.values(owner)
    }

    pub fn callable_by_owner_name(
        &self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> Vec<HbkCallableId> {
        lookup_owner_name(
            &self.snapshot.callables_by_owner_name,
            self.snapshot,
            owner,
            name,
        )
    }

    pub fn globals_by_name(&self, name: &str) -> Vec<HbkGlobalFactId> {
        lookup_name(&self.snapshot.global_names, self.snapshot, name)
    }

    pub fn module_events(&self, module_context_key: &str) -> Vec<HbkCallableId> {
        let key = normalize_lookup_key(module_context_key);
        lookup_owner_name_by_key(&self.snapshot.module_event_names, self.snapshot, &key)
    }

    pub fn query_table_by_id(&self, id: &str) -> Option<HbkQueryTableId> {
        lookup_id(&self.snapshot.query_table_ids, self.snapshot, id)
    }

    pub fn query_tables_by_name(&self, name: &str) -> Vec<HbkQueryTableId> {
        lookup_name(&self.snapshot.query_table_names, self.snapshot, name)
    }

    pub fn query_fields(&self, table: HbkQueryTableId) -> &[HbkQueryFieldId] {
        self.snapshot.query_fields_by_table.values(table)
    }

    pub fn query_parameters(&self, table: HbkQueryTableId) -> &[HbkQueryParameterId] {
        self.snapshot.query_parameters_by_table.values(table)
    }

    pub fn language_fact_by_id(&self, id: &str) -> Option<HbkLanguageFactId> {
        lookup_id(&self.snapshot.language_ids, self.snapshot, id)
    }

    pub fn language_facts_by_name(&self, name: &str) -> Vec<HbkLanguageFactId> {
        lookup_name(&self.snapshot.language_names, self.snapshot, name)
    }
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

    fn materialize(mut self) -> Result<HbkFactSnapshot, SearchError> {
        let documents = self.documents()?;
        let metadata = self.metadata_rows()?;
        let type_identities = self.type_identities()?;
        let members = self.members()?;
        let callables = self.callables()?;
        let signatures = self.signatures()?;
        let parameters = self.parameters()?;
        let type_refs = self.type_refs()?;
        let module_context_keys = self.module_context_keys()?;
        let query_owners = self.query_owner_edges()?;

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

        let mut platform_types = Vec::new();
        let mut platform_type_by_type_id = BTreeMap::<String, HbkPlatformTypeId>::new();
        let mut platform_type_ids = Vec::new();
        let mut platform_type_names = Vec::new();
        for document in documents
            .iter()
            .filter(|document| document.kind == SearchDocumentKind::PlatformType)
        {
            let Some(type_id) = type_id_by_document.get(document.id.as_str()) else {
                continue;
            };
            let id = HbkPlatformTypeId(platform_types.len() as u32);
            platform_type_by_type_id.insert((*type_id).to_string(), id);
            platform_types.push(HbkPlatformType {
                id: self.builder.intern(type_id),
                name: self.builder.intern_name(&document.name),
                availability_contexts: self.builder.intern_many(&document.availability_contexts),
            });
            push_id_lookup(&mut platform_type_ids, &mut self.builder, type_id, id);
            push_name_lookups(
                &mut platform_type_names,
                &mut self.builder,
                &document.name,
                id,
            );
        }

        let type_refs_by_document = group_document_type_refs(&mut self.builder, &type_refs);
        let return_refs_by_document = group_return_type_refs(&mut self.builder, &type_refs);
        let signature_refs = group_signature_return_type_refs(&mut self.builder, &type_refs);
        let parameter_refs = group_parameter_type_refs(&mut self.builder, &type_refs);
        let parameters_by_signature = parameters_by_signature(parameters, parameter_refs);
        let signatures_by_callable = signatures_by_callable(
            &mut self.builder,
            &documents_by_id,
            signatures,
            parameters_by_signature,
            signature_refs,
        );

        let mut type_members = Vec::new();
        let mut member_ids = Vec::new();
        let mut member_owner_pairs = Vec::new();
        let mut members_by_owner_name = Vec::new();
        for row in members {
            let Some(document) = documents_by_id.get(row.document_id.as_str()) else {
                continue;
            };
            let Some(owner) = platform_type_by_type_id.get(row.owner_type_id.as_str()).copied()
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
                    .get(&(document.id.clone(), document.kind.type_ref_kind().to_string()))
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
            member_owner_pairs.push((owner, id));
        }

        let mut callables_vec = Vec::new();
        let mut callable_ids = Vec::new();
        let mut callables_by_document = BTreeMap::<String, HbkCallableId>::new();
        let mut callable_owner_pairs = Vec::new();
        let mut callables_by_owner_name = Vec::new();
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
                name: self.builder.intern_name(&document.name),
                callable: callables_by_document.get(document.id.as_str()).copied(),
                type_refs: type_refs_by_document
                    .get(&(document.id.clone(), document.kind.type_ref_kind().to_string()))
                    .cloned()
                    .unwrap_or_default(),
            });
            push_name_lookups(&mut global_names, &mut self.builder, &document.name, id);
        }

        let mut query_tables = Vec::new();
        let mut query_table_by_document = BTreeMap::<String, HbkQueryTableId>::new();
        let mut query_table_ids = Vec::new();
        let mut query_table_names = Vec::new();
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
                identifier: meta.and_then(|row| {
                    self.builder
                        .intern_option(row.query_identifier.as_deref())
                }),
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
            push_name_lookups(&mut query_table_names, &mut self.builder, &document.name, id);
            if let Some(meta) = meta {
                if let Some(query_syntax) = &meta.query_syntax {
                    push_name_lookups(
                        &mut query_table_names,
                        &mut self.builder,
                        query_syntax,
                        id,
                    );
                }
                if let Some(identifier) = &meta.query_identifier {
                    push_lookup(
                        &mut query_table_names,
                        &mut self.builder,
                        &normalize_lookup_key(identifier),
                        id,
                    );
                }
            }
        }

        let mut query_fields = Vec::new();
        let mut query_field_owner_pairs = Vec::new();
        let mut query_parameters = Vec::new();
        let mut query_parameter_owner_pairs = Vec::new();
        for (target_id, source_id) in query_owners {
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
                            .get(&(document.id.clone(), document.kind.type_ref_kind().to_string()))
                            .cloned()
                            .unwrap_or_default(),
                        note: meta.and_then(|row| self.builder.intern_option(row.note.as_deref())),
                    });
                    query_field_owner_pairs.push((owner, id));
                }
                SearchDocumentKind::QueryTableParameter => {
                    let id = HbkQueryParameterId(query_parameters.len() as u32);
                    query_parameters.push(HbkQueryParameter {
                        id: self.builder.intern(&document.id),
                        owner,
                        name: self.builder.intern_name(&document.name),
                        type_refs: type_refs_by_document
                            .get(&(document.id.clone(), document.kind.type_ref_kind().to_string()))
                            .cloned()
                            .unwrap_or_default(),
                        default_value: meta
                            .and_then(|row| self.builder.intern_option(row.default_value.as_deref())),
                    });
                    query_parameter_owner_pairs.push((owner, id));
                }
                _ => {}
            }
        }

        let mut language_facts = Vec::new();
        let mut language_ids = Vec::new();
        let mut language_names = Vec::new();
        for document in documents.iter().filter(|document| document.kind.is_language()) {
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
                    .get(&(document.id.clone(), document.kind.type_ref_kind().to_string()))
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

        let mut module_event_names = Vec::new();
        for (document_id, context_key) in module_context_keys {
            let Some(callable) = callables_by_document.get(document_id.as_str()).copied() else {
                continue;
            };
            push_lookup_with_owner_key(
                &mut module_event_names,
                &mut self.builder,
                &normalize_lookup_key(&context_key),
                &normalize_lookup_key(&context_key),
                callable,
            );
        }

        let platform_type_ids = sorted_id_lookup(platform_type_ids, &self.builder);
        let platform_type_names = sorted_name_lookup(platform_type_names, &self.builder);
        let member_ids = sorted_id_lookup(member_ids, &self.builder);
        let members_by_owner_name = sorted_owner_name_lookup(members_by_owner_name, &self.builder);
        let callable_ids = sorted_id_lookup(callable_ids, &self.builder);
        let callables_by_owner_name =
            sorted_owner_name_lookup(callables_by_owner_name, &self.builder);
        let global_names = sorted_name_lookup(global_names, &self.builder);
        let module_event_names =
            sorted_string_owner_name_lookup(module_event_names, &self.builder);
        let query_table_ids = sorted_id_lookup(query_table_ids, &self.builder);
        let query_table_names = sorted_name_lookup(query_table_names, &self.builder);
        let language_ids = sorted_id_lookup(language_ids, &self.builder);
        let language_names = sorted_name_lookup(language_names, &self.builder);

        let snapshot = HbkFactSnapshot {
            strings: self.builder.strings,
            platform_types,
            type_members,
            callables: callables_vec,
            globals,
            query_tables,
            query_fields,
            query_parameters,
            language_facts,
            platform_type_ids,
            platform_type_names,
            member_ids,
            members_by_owner: CsrIndex::from_pairs(member_owner_pairs),
            members_by_owner_name,
            callable_ids,
            callables_by_owner: CsrIndex::from_pairs(callable_owner_pairs),
            callables_by_owner_name,
            global_names,
            module_event_names,
            query_table_ids,
            query_table_names,
            query_fields_by_table: CsrIndex::from_pairs(query_field_owner_pairs),
            query_parameters_by_table: CsrIndex::from_pairs(query_parameter_owner_pairs),
            language_ids,
            language_names,
        };
        Ok(snapshot)
    }

    fn documents(&self) -> Result<Vec<DocumentRow>, SearchError> {
        let mut statement = self
            .index
            .connection
            .prepare(
                "SELECT id, kind, name_primary, name_alias, owner_primary, owner_alias,
                        signature_text, availability_contexts
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
    fn intern(&mut self, value: &str) -> StringId {
        if let Some(id) = self.string_ids.get(value).copied() {
            return id;
        }
        let id = StringId(self.strings.len() as u32);
        self.strings.push(value.to_string());
        self.string_ids.insert(value.to_string(), id);
        id
    }

    fn intern_option(&mut self, value: Option<&str>) -> Option<StringId> {
        value.map(|value| self.intern(value))
    }

    fn intern_many(&mut self, values: &[String]) -> Vec<StringId> {
        values.iter().map(|value| self.intern(value)).collect()
    }

    fn intern_name(&mut self, name: &model::LocalizedName) -> HbkName {
        HbkName {
            primary: self.intern(&name.primary),
            alias: name.alias.as_deref().map(|alias| self.intern(alias)),
        }
    }

    fn string(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }
}

impl<K, V> CsrIndex<K, V>
where
    K: Copy + Ord,
    V: Copy + Ord,
{
    fn from_pairs(mut pairs: Vec<(K, V)>) -> Self {
        pairs.sort();
        let mut keys = Vec::new();
        let mut offsets = vec![0];
        let mut values = Vec::with_capacity(pairs.len());
        let mut current_key = None;
        for (key, value) in pairs {
            if current_key != Some(key) {
                if current_key.is_some() {
                    offsets.push(values.len() as u32);
                }
                keys.push(key);
                current_key = Some(key);
            }
            values.push(value);
        }
        offsets.push(values.len() as u32);
        Self {
            keys,
            offsets,
            values,
        }
    }

    fn values(&self, key: K) -> &[V] {
        let Ok(index) = self.keys.binary_search(&key) else {
            return &[];
        };
        let start = self.offsets[index] as usize;
        let end = self.offsets[index + 1] as usize;
        &self.values[start..end]
    }

    fn estimated_heap_bytes(&self) -> usize {
        vec_heap_bytes(&self.keys) + vec_heap_bytes(&self.offsets) + vec_heap_bytes(&self.values)
    }
}

fn vec_heap_bytes<T>(values: &Vec<T>) -> usize {
    values.capacity() * std::mem::size_of::<T>()
}

fn type_refs_heap_bytes(values: &[HbkTypeRef]) -> usize {
    values
        .iter()
        .map(|type_ref| {
            let target = match &type_ref.target {
                HbkTypeRefTarget::Ok(_) | HbkTypeRefTarget::Unresolved => 0,
                HbkTypeRefTarget::Ambiguous(candidates) => vec_heap_bytes(candidates),
            };
            let binding = type_ref
                .template_binding
                .as_ref()
                .map(|binding| vec_heap_bytes(&binding.arguments))
                .unwrap_or(0);
            target + binding
        })
        .sum()
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
        if let (Some(signature_id), Some(ordinal)) =
            (row.source_signature_id.as_deref(), row.source_parameter_ordinal)
        {
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
            .and_then(|document| split_lines(document.signature_text.clone()).get(signature.ordinal as usize).cloned())
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
        template_binding: value.template_binding.clone(),
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

fn push_id_lookup<T: Copy>(
    output: &mut Vec<IdLookup<T>>,
    builder: &mut SnapshotBuilder,
    key: &str,
    value: T,
) {
    let key = builder.intern(key);
    output.push(IdLookup { key, value });
}

fn push_name_lookups<T: Copy>(
    output: &mut Vec<NameLookup<T>>,
    builder: &mut SnapshotBuilder,
    name: &model::LocalizedName,
    value: T,
) {
    push_lookup(output, builder, &normalize_lookup_key(&name.primary), value);
    if let Some(alias) = &name.alias {
        push_lookup(output, builder, &normalize_lookup_key(alias), value);
    }
}

fn push_lookup<T: Copy>(
    output: &mut Vec<NameLookup<T>>,
    builder: &mut SnapshotBuilder,
    key: &str,
    value: T,
) {
    let key = builder.intern(key);
    output.push(NameLookup { key, value });
}

fn push_owner_name_lookups<Owner: Copy, Value: Copy>(
    output: &mut Vec<OwnerNameLookup<Owner, Value>>,
    builder: &mut SnapshotBuilder,
    owner: Owner,
    name: &model::LocalizedName,
    value: Value,
) {
    push_owner_lookup(output, builder, owner, &normalize_lookup_key(&name.primary), value);
    if let Some(alias) = &name.alias {
        push_owner_lookup(output, builder, owner, &normalize_lookup_key(alias), value);
    }
}

fn push_owner_lookup<Owner: Copy, Value: Copy>(
    output: &mut Vec<OwnerNameLookup<Owner, Value>>,
    builder: &mut SnapshotBuilder,
    owner: Owner,
    key: &str,
    value: Value,
) {
    output.push(OwnerNameLookup {
        owner,
        key: builder.intern(key),
        value,
    });
}

fn push_lookup_with_owner_key<Value: Copy>(
    output: &mut Vec<OwnerNameLookup<StringId, Value>>,
    builder: &mut SnapshotBuilder,
    owner: &str,
    key: &str,
    value: Value,
) {
    let owner = builder.intern(owner);
    let key = builder.intern(key);
    output.push(OwnerNameLookup { owner, key, value });
}

fn sorted_id_lookup<T: Copy + Ord>(
    mut values: Vec<IdLookup<T>>,
    builder: &SnapshotBuilder,
) -> Vec<IdLookup<T>> {
    values.sort_by(|left, right| {
        builder
            .string(left.key)
            .cmp(builder.string(right.key))
            .then_with(|| left.value.cmp(&right.value))
    });
    values
}

fn sorted_name_lookup<T: Copy + Ord>(
    mut values: Vec<NameLookup<T>>,
    builder: &SnapshotBuilder,
) -> Vec<NameLookup<T>> {
    values.sort_by(|left, right| {
        builder
            .string(left.key)
            .cmp(builder.string(right.key))
            .then_with(|| left.value.cmp(&right.value))
    });
    values
}

fn sorted_owner_name_lookup<Owner: Copy + Ord, Value: Copy + Ord>(
    mut values: Vec<OwnerNameLookup<Owner, Value>>,
    builder: &SnapshotBuilder,
) -> Vec<OwnerNameLookup<Owner, Value>> {
    values.sort_by(|left, right| {
        left.owner
            .cmp(&right.owner)
            .then_with(|| builder.string(left.key).cmp(builder.string(right.key)))
            .then_with(|| left.value.cmp(&right.value))
    });
    values
}

fn sorted_string_owner_name_lookup<Value: Copy + Ord>(
    mut values: Vec<OwnerNameLookup<StringId, Value>>,
    builder: &SnapshotBuilder,
) -> Vec<OwnerNameLookup<StringId, Value>> {
    values.sort_by(|left, right| {
        builder
            .string(left.owner)
            .cmp(builder.string(right.owner))
            .then_with(|| builder.string(left.key).cmp(builder.string(right.key)))
            .then_with(|| left.value.cmp(&right.value))
    });
    values
}

fn lookup_id<T: Copy>(index: &[IdLookup<T>], snapshot: &HbkFactSnapshot, key: &str) -> Option<T> {
    index
        .binary_search_by(|candidate| snapshot.string(candidate.key).cmp(key))
        .ok()
        .map(|position| index[position].value)
}

fn lookup_name<T: Copy>(
    index: &[NameLookup<T>],
    snapshot: &HbkFactSnapshot,
    name: &str,
) -> Vec<T> {
    let key = normalize_lookup_key(name);
    let range = matching_range(index, |candidate| snapshot.string(candidate.key).cmp(&key));
    index[range].iter().map(|candidate| candidate.value).collect()
}

fn lookup_owner_name<Owner: Copy + Ord, Value: Copy>(
    index: &[OwnerNameLookup<Owner, Value>],
    snapshot: &HbkFactSnapshot,
    owner: Owner,
    name: &str,
) -> Vec<Value> {
    let key = normalize_lookup_key(name);
    let range = matching_range(index, |candidate| {
        candidate
            .owner
            .cmp(&owner)
            .then_with(|| snapshot.string(candidate.key).cmp(&key))
    });
    index[range].iter().map(|candidate| candidate.value).collect()
}

fn lookup_owner_name_by_key<Value: Copy>(
    index: &[OwnerNameLookup<StringId, Value>],
    snapshot: &HbkFactSnapshot,
    key: &str,
) -> Vec<Value> {
    let range = matching_range(index, |candidate| {
        snapshot
            .string(candidate.owner)
            .cmp(key)
            .then_with(|| snapshot.string(candidate.key).cmp(key))
    });
    index[range].iter().map(|candidate| candidate.value).collect()
}

fn matching_range<T, F>(values: &[T], mut compare: F) -> std::ops::Range<usize>
where
    F: FnMut(&T) -> std::cmp::Ordering,
{
    let Ok(mut start) = values.binary_search_by(&mut compare) else {
        return 0..0;
    };
    let mut end = start + 1;
    while start > 0 && compare(&values[start - 1]).is_eq() {
        start -= 1;
    }
    while end < values.len() && compare(&values[end]).is_eq() {
        end += 1;
    }
    start..end
}
