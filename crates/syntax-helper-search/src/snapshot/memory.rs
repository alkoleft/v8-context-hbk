use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkFactSnapshotMemory {
    pub string_store: HbkFactSnapshotMemoryEntry,
    pub node_arenas: HbkFactSnapshotMemoryEntry,
    pub indexes: HbkFactSnapshotIndexMemory,
}

impl HbkFactSnapshotMemory {
    pub fn total_bytes(&self) -> usize {
        self.string_store.bytes + self.node_arenas.bytes + self.indexes.total_bytes()
    }

    pub fn total_payload_bytes(&self) -> usize {
        self.string_store.payload_bytes
            + self.node_arenas.payload_bytes
            + self.indexes.total_payload_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HbkFactSnapshotMemoryEntry {
    pub count: usize,
    pub bytes: usize,
    pub payload_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkFactSnapshotIndexMemory {
    pub fact_ids: HbkFactSnapshotMemoryEntry,
    pub platform_type_ids: HbkFactSnapshotMemoryEntry,
    pub platform_type_names: HbkFactSnapshotMemoryEntry,
    pub platform_type_templates: HbkFactSnapshotMemoryEntry,
    pub member_ids: HbkFactSnapshotMemoryEntry,
    pub members_by_owner: HbkFactSnapshotMemoryEntry,
    pub members_by_owner_name: HbkFactSnapshotMemoryEntry,
    pub members_by_owner_name_kind: HbkFactSnapshotMemoryEntry,
    pub callable_ids: HbkFactSnapshotMemoryEntry,
    pub callables_by_owner: HbkFactSnapshotMemoryEntry,
    pub callables_by_owner_name: HbkFactSnapshotMemoryEntry,
    pub constructors_by_type: HbkFactSnapshotMemoryEntry,
    pub global_names: HbkFactSnapshotMemoryEntry,
    pub globals_by_domain_name_kind: HbkFactSnapshotMemoryEntry,
    pub module_event_names: HbkFactSnapshotMemoryEntry,
    pub module_contexts_by_domain_language_kind: HbkFactSnapshotMemoryEntry,
    pub query_table_ids: HbkFactSnapshotMemoryEntry,
    pub query_table_names: HbkFactSnapshotMemoryEntry,
    pub query_table_syntax_names: HbkFactSnapshotMemoryEntry,
    pub query_table_identifiers: HbkFactSnapshotMemoryEntry,
    pub query_fields_by_table: HbkFactSnapshotMemoryEntry,
    pub query_fields_by_table_name: HbkFactSnapshotMemoryEntry,
    pub query_parameters_by_table: HbkFactSnapshotMemoryEntry,
    pub query_parameters_by_table_name: HbkFactSnapshotMemoryEntry,
    pub language_ids: HbkFactSnapshotMemoryEntry,
    pub language_names: HbkFactSnapshotMemoryEntry,
    pub enum_ids: HbkFactSnapshotMemoryEntry,
    pub enum_names: HbkFactSnapshotMemoryEntry,
    pub enum_value_ids: HbkFactSnapshotMemoryEntry,
    pub enum_values_by_enum: HbkFactSnapshotMemoryEntry,
    pub enum_values_by_enum_name: HbkFactSnapshotMemoryEntry,
    pub availability_by_fact: HbkFactSnapshotMemoryEntry,
    pub availability_since_by_fact: HbkFactSnapshotMemoryEntry,
    pub source_by_fact: HbkFactSnapshotMemoryEntry,
    pub relations_by_source_kind: HbkFactSnapshotMemoryEntry,
}

impl HbkFactSnapshotIndexMemory {
    pub fn total_bytes(&self) -> usize {
        self.fact_ids.bytes
            + self.platform_type_ids.bytes
            + self.platform_type_names.bytes
            + self.platform_type_templates.bytes
            + self.member_ids.bytes
            + self.members_by_owner.bytes
            + self.members_by_owner_name.bytes
            + self.members_by_owner_name_kind.bytes
            + self.callable_ids.bytes
            + self.callables_by_owner.bytes
            + self.callables_by_owner_name.bytes
            + self.constructors_by_type.bytes
            + self.global_names.bytes
            + self.globals_by_domain_name_kind.bytes
            + self.module_event_names.bytes
            + self.module_contexts_by_domain_language_kind.bytes
            + self.query_table_ids.bytes
            + self.query_table_names.bytes
            + self.query_table_syntax_names.bytes
            + self.query_table_identifiers.bytes
            + self.query_fields_by_table.bytes
            + self.query_fields_by_table_name.bytes
            + self.query_parameters_by_table.bytes
            + self.query_parameters_by_table_name.bytes
            + self.language_ids.bytes
            + self.language_names.bytes
            + self.enum_ids.bytes
            + self.enum_names.bytes
            + self.enum_value_ids.bytes
            + self.enum_values_by_enum.bytes
            + self.enum_values_by_enum_name.bytes
            + self.availability_by_fact.bytes
            + self.availability_since_by_fact.bytes
            + self.source_by_fact.bytes
            + self.relations_by_source_kind.bytes
    }

    pub fn total_payload_bytes(&self) -> usize {
        self.fact_ids.payload_bytes
            + self.platform_type_ids.payload_bytes
            + self.platform_type_names.payload_bytes
            + self.platform_type_templates.payload_bytes
            + self.member_ids.payload_bytes
            + self.members_by_owner.payload_bytes
            + self.members_by_owner_name.payload_bytes
            + self.members_by_owner_name_kind.payload_bytes
            + self.callable_ids.payload_bytes
            + self.callables_by_owner.payload_bytes
            + self.callables_by_owner_name.payload_bytes
            + self.constructors_by_type.payload_bytes
            + self.global_names.payload_bytes
            + self.globals_by_domain_name_kind.payload_bytes
            + self.module_event_names.payload_bytes
            + self.module_contexts_by_domain_language_kind.payload_bytes
            + self.query_table_ids.payload_bytes
            + self.query_table_names.payload_bytes
            + self.query_table_syntax_names.payload_bytes
            + self.query_table_identifiers.payload_bytes
            + self.query_fields_by_table.payload_bytes
            + self.query_fields_by_table_name.payload_bytes
            + self.query_parameters_by_table.payload_bytes
            + self.query_parameters_by_table_name.payload_bytes
            + self.language_ids.payload_bytes
            + self.language_names.payload_bytes
            + self.enum_ids.payload_bytes
            + self.enum_names.payload_bytes
            + self.enum_value_ids.payload_bytes
            + self.enum_values_by_enum.payload_bytes
            + self.enum_values_by_enum_name.payload_bytes
            + self.availability_by_fact.payload_bytes
            + self.availability_since_by_fact.payload_bytes
            + self.source_by_fact.payload_bytes
            + self.relations_by_source_kind.payload_bytes
    }
}

impl HbkFactSnapshot {
    /// Estimates heap retained by an owned build/oracle snapshot.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Mapped artifact accounting is a
    /// separate concern and is intentionally not approximated as heap usage.
    pub fn estimated_heap_bytes(&self) -> usize {
        self.memory_accounting().total_bytes()
    }

    /// Describes heap retained by an owned build/oracle snapshot.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Mapped artifact accounting is a
    /// separate concern and is intentionally not approximated as heap usage.
    pub fn memory_accounting(&self) -> HbkFactSnapshotMemory {
        self.assert_owned();
        let string_store_bytes = vec_heap_bytes(&self.strings)
            + self
                .strings
                .iter()
                .map(|value| value.capacity())
                .sum::<usize>();
        let string_store_payload_bytes =
            vec_payload_bytes(&self.strings) + self.strings.iter().map(String::len).sum::<usize>();
        let mut node_arenas_bytes = vec_heap_bytes(&self.platform_types);
        let mut node_arenas_payload_bytes = vec_payload_bytes(&self.platform_types);
        node_arenas_bytes += vec_heap_bytes(&self.type_members);
        node_arenas_payload_bytes += vec_payload_bytes(&self.type_members);
        node_arenas_bytes += vec_heap_bytes(&self.callables);
        node_arenas_payload_bytes += vec_payload_bytes(&self.callables);
        node_arenas_bytes += vec_heap_bytes(&self.globals);
        node_arenas_payload_bytes += vec_payload_bytes(&self.globals);
        node_arenas_bytes += vec_heap_bytes(&self.query_tables);
        node_arenas_payload_bytes += vec_payload_bytes(&self.query_tables);
        node_arenas_bytes += vec_heap_bytes(&self.query_fields);
        node_arenas_payload_bytes += vec_payload_bytes(&self.query_fields);
        node_arenas_bytes += vec_heap_bytes(&self.query_parameters);
        node_arenas_payload_bytes += vec_payload_bytes(&self.query_parameters);
        node_arenas_bytes += vec_heap_bytes(&self.language_facts);
        node_arenas_payload_bytes += vec_payload_bytes(&self.language_facts);
        node_arenas_bytes += vec_heap_bytes(&self.enums);
        node_arenas_payload_bytes += vec_payload_bytes(&self.enums);
        node_arenas_bytes += vec_heap_bytes(&self.enum_values);
        node_arenas_payload_bytes += vec_payload_bytes(&self.enum_values);
        node_arenas_bytes += self
            .type_members
            .iter()
            .map(|member| {
                vec_heap_bytes(&member.type_refs)
                    + vec_heap_bytes(&member.availability_contexts)
                    + type_refs_heap_bytes(&member.type_refs)
            })
            .sum::<usize>();
        node_arenas_payload_bytes += self
            .type_members
            .iter()
            .map(|member| {
                vec_payload_bytes(&member.type_refs)
                    + vec_payload_bytes(&member.availability_contexts)
                    + type_refs_payload_bytes(&member.type_refs)
            })
            .sum::<usize>();
        node_arenas_bytes += self
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
        node_arenas_payload_bytes += self
            .callables
            .iter()
            .map(|callable| {
                vec_payload_bytes(&callable.signatures)
                    + callable
                        .signatures
                        .iter()
                        .map(|signature| {
                            vec_payload_bytes(&signature.parameters)
                                + vec_payload_bytes(&signature.return_type_refs)
                                + type_refs_payload_bytes(&signature.return_type_refs)
                                + signature
                                    .parameters
                                    .iter()
                                    .map(|parameter| {
                                        vec_payload_bytes(&parameter.type_refs)
                                            + type_refs_payload_bytes(&parameter.type_refs)
                                    })
                                    .sum::<usize>()
                        })
                        .sum::<usize>()
                    + vec_payload_bytes(&callable.return_type_refs)
                    + type_refs_payload_bytes(&callable.return_type_refs)
                    + vec_payload_bytes(&callable.availability_contexts)
            })
            .sum::<usize>();
        node_arenas_bytes += self
            .globals
            .iter()
            .map(|global| {
                vec_heap_bytes(&global.type_refs) + type_refs_heap_bytes(&global.type_refs)
            })
            .sum::<usize>();
        node_arenas_payload_bytes += self
            .globals
            .iter()
            .map(|global| {
                vec_payload_bytes(&global.type_refs) + type_refs_payload_bytes(&global.type_refs)
            })
            .sum::<usize>();
        node_arenas_bytes += self
            .platform_types
            .iter()
            .map(|platform_type| {
                platform_type
                    .metadata_template
                    .as_ref()
                    .map(|template| vec_heap_bytes(&template.template_parameters))
                    .unwrap_or_default()
            })
            .sum::<usize>();
        node_arenas_payload_bytes += self
            .platform_types
            .iter()
            .map(|platform_type| {
                platform_type
                    .metadata_template
                    .as_ref()
                    .map(|template| vec_payload_bytes(&template.template_parameters))
                    .unwrap_or_default()
            })
            .sum::<usize>();
        node_arenas_bytes += self
            .query_tables
            .iter()
            .map(|table| {
                vec_heap_bytes(&table.owner_path) + vec_heap_bytes(&table.template_parameters)
            })
            .sum::<usize>();
        node_arenas_payload_bytes += self
            .query_tables
            .iter()
            .map(|table| {
                vec_payload_bytes(&table.owner_path) + vec_payload_bytes(&table.template_parameters)
            })
            .sum::<usize>();
        node_arenas_bytes += self
            .query_fields
            .iter()
            .map(|field| vec_heap_bytes(&field.type_refs) + type_refs_heap_bytes(&field.type_refs))
            .sum::<usize>();
        node_arenas_payload_bytes += self
            .query_fields
            .iter()
            .map(|field| {
                vec_payload_bytes(&field.type_refs) + type_refs_payload_bytes(&field.type_refs)
            })
            .sum::<usize>();
        node_arenas_bytes += self
            .query_parameters
            .iter()
            .map(|parameter| {
                vec_heap_bytes(&parameter.type_refs) + type_refs_heap_bytes(&parameter.type_refs)
            })
            .sum::<usize>();
        node_arenas_payload_bytes += self
            .query_parameters
            .iter()
            .map(|parameter| {
                vec_payload_bytes(&parameter.type_refs)
                    + type_refs_payload_bytes(&parameter.type_refs)
            })
            .sum::<usize>();
        node_arenas_bytes += self
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
        node_arenas_payload_bytes += self
            .language_facts
            .iter()
            .map(|fact| {
                vec_payload_bytes(&fact.signatures)
                    + fact
                        .signatures
                        .iter()
                        .map(|signature| {
                            vec_payload_bytes(&signature.parameters)
                                + vec_payload_bytes(&signature.return_type_refs)
                                + type_refs_payload_bytes(&signature.return_type_refs)
                                + signature
                                    .parameters
                                    .iter()
                                    .map(|parameter| {
                                        vec_payload_bytes(&parameter.type_refs)
                                            + type_refs_payload_bytes(&parameter.type_refs)
                                    })
                                    .sum::<usize>()
                        })
                        .sum::<usize>()
                    + vec_payload_bytes(&fact.type_refs)
                    + type_refs_payload_bytes(&fact.type_refs)
                    + vec_payload_bytes(&fact.return_type_refs)
                    + type_refs_payload_bytes(&fact.return_type_refs)
            })
            .sum::<usize>();
        HbkFactSnapshotMemory {
            string_store: HbkFactSnapshotMemoryEntry {
                count: self.strings.len(),
                bytes: string_store_bytes,
                payload_bytes: string_store_payload_bytes,
            },
            node_arenas: HbkFactSnapshotMemoryEntry {
                count: self.platform_types.len()
                    + self.type_members.len()
                    + self.callables.len()
                    + self.globals.len()
                    + self.query_tables.len()
                    + self.query_fields.len()
                    + self.query_parameters.len()
                    + self.language_facts.len()
                    + self.enums.len()
                    + self.enum_values.len(),
                bytes: node_arenas_bytes,
                payload_bytes: node_arenas_payload_bytes,
            },
            indexes: HbkFactSnapshotIndexMemory {
                fact_ids: vec_memory_entry(&self.fact_ids),
                platform_type_ids: vec_memory_entry(&self.platform_type_ids),
                platform_type_names: vec_memory_entry(&self.platform_type_names),
                platform_type_templates: vec_memory_entry(&self.platform_type_templates),
                member_ids: vec_memory_entry(&self.member_ids),
                members_by_owner: self.members_by_owner.memory_entry(),
                members_by_owner_name: vec_memory_entry(&self.members_by_owner_name),
                members_by_owner_name_kind: vec_memory_entry(&self.members_by_owner_name_kind),
                callable_ids: vec_memory_entry(&self.callable_ids),
                callables_by_owner: self.callables_by_owner.memory_entry(),
                callables_by_owner_name: vec_memory_entry(&self.callables_by_owner_name),
                constructors_by_type: self.constructors_by_type.memory_entry(),
                global_names: vec_memory_entry(&self.global_names),
                globals_by_domain_name_kind: vec_memory_entry(&self.globals_by_domain_name_kind),
                module_event_names: vec_memory_entry(&self.module_event_names),
                module_contexts_by_domain_language_kind: vec_memory_entry(
                    &self.module_contexts_by_domain_language_kind,
                ),
                query_table_ids: vec_memory_entry(&self.query_table_ids),
                query_table_names: vec_memory_entry(&self.query_table_names),
                query_table_syntax_names: vec_memory_entry(&self.query_table_syntax_names),
                query_table_identifiers: vec_memory_entry(&self.query_table_identifiers),
                query_fields_by_table: self.query_fields_by_table.memory_entry(),
                query_fields_by_table_name: vec_memory_entry(&self.query_fields_by_table_name),
                query_parameters_by_table: self.query_parameters_by_table.memory_entry(),
                query_parameters_by_table_name: vec_memory_entry(
                    &self.query_parameters_by_table_name,
                ),
                language_ids: vec_memory_entry(&self.language_ids),
                language_names: vec_memory_entry(&self.language_names),
                enum_ids: vec_memory_entry(&self.enum_ids),
                enum_names: vec_memory_entry(&self.enum_names),
                enum_value_ids: vec_memory_entry(&self.enum_value_ids),
                enum_values_by_enum: self.enum_values_by_enum.memory_entry(),
                enum_values_by_enum_name: vec_memory_entry(&self.enum_values_by_enum_name),
                availability_by_fact: self.availability_by_fact.memory_entry(),
                availability_since_by_fact: vec_memory_entry(&self.availability_since_by_fact),
                source_by_fact: vec_memory_entry(&self.source_by_fact),
                relations_by_source_kind: self.relations_by_source_kind.memory_entry(),
            },
        }
    }
}

pub(super) fn vec_heap_bytes<T>(values: &Vec<T>) -> usize {
    values.capacity() * std::mem::size_of::<T>()
}

pub(super) fn vec_payload_bytes<T>(values: &[T]) -> usize {
    std::mem::size_of_val(values)
}

pub(super) fn vec_memory_entry<T>(values: &Vec<T>) -> HbkFactSnapshotMemoryEntry {
    HbkFactSnapshotMemoryEntry {
        count: values.len(),
        bytes: vec_heap_bytes(values),
        payload_bytes: vec_payload_bytes(values),
    }
}

pub(super) fn type_refs_heap_bytes(values: &[HbkTypeRef]) -> usize {
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

pub(super) fn type_refs_payload_bytes(values: &[HbkTypeRef]) -> usize {
    values
        .iter()
        .map(|type_ref| {
            let target = match &type_ref.target {
                HbkTypeRefTarget::Ok(_) | HbkTypeRefTarget::Unresolved => 0,
                HbkTypeRefTarget::Ambiguous(candidates) => vec_payload_bytes(candidates),
            };
            let binding = type_ref
                .template_binding
                .as_ref()
                .map(|binding| vec_payload_bytes(&binding.arguments))
                .unwrap_or(0);
            target + binding
        })
        .sum()
}
