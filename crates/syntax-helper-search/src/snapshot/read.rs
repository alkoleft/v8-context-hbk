use super::indexes::*;
use super::*;

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
    pub enums: usize,
    pub enum_values: usize,
}

pub struct MemberLookupIter<'a> {
    inner: MemberLookupIterInner<'a>,
}

enum MemberLookupIterInner<'a> {
    Name(std::slice::Iter<'a, OwnerNameLookup<HbkPlatformTypeId, HbkTypeMemberId>>),
    NameKind(std::slice::Iter<'a, MemberNameKindLookup>),
}

impl Iterator for MemberLookupIter<'_> {
    type Item = HbkTypeMemberId;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            MemberLookupIterInner::Name(iter) => iter.next().map(|candidate| candidate.value),
            MemberLookupIterInner::NameKind(iter) => iter.next().map(|candidate| candidate.value),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for MemberLookupIter<'_> {
    fn len(&self) -> usize {
        match &self.inner {
            MemberLookupIterInner::Name(iter) => iter.len(),
            MemberLookupIterInner::NameKind(iter) => iter.len(),
        }
    }
}

impl HbkFactSnapshot {
    pub fn worker_handle(&self) -> HbkFactReadHandle<'_> {
        HbkFactReadHandle { snapshot: self }
    }

    pub fn string(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }

    pub fn source_locale(&self) -> Option<&str> {
        self.source_locale.map(|id| self.string(id))
    }

    fn string_id(&self, value: &str) -> Option<StringId> {
        self.strings
            .iter()
            .position(|candidate| candidate == value)
            .map(|index| StringId(index as u32))
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

    pub fn enum_fact(&self, id: HbkEnumId) -> &HbkEnum {
        &self.enums[id.0 as usize]
    }

    pub fn enum_value(&self, id: HbkEnumValueId) -> &HbkEnumValue {
        &self.enum_values[id.0 as usize]
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
            enums: self.enums.len(),
            enum_values: self.enum_values.len(),
        }
    }
}

impl<'a> HbkFactReadHandle<'a> {
    pub fn global_fact_ids(&self) -> impl Iterator<Item = HbkGlobalFactId> + '_ {
        (0..self.snapshot.globals.len()).map(|index| HbkGlobalFactId(index as u32))
    }

    pub fn query_table_ids(&self) -> impl Iterator<Item = HbkQueryTableId> + '_ {
        (0..self.snapshot.query_tables.len()).map(|index| HbkQueryTableId(index as u32))
    }

    pub fn query_field_ids(&self) -> impl Iterator<Item = HbkQueryFieldId> + '_ {
        (0..self.snapshot.query_fields.len()).map(|index| HbkQueryFieldId(index as u32))
    }

    pub fn query_parameter_ids(&self) -> impl Iterator<Item = HbkQueryParameterId> + '_ {
        (0..self.snapshot.query_parameters.len()).map(|index| HbkQueryParameterId(index as u32))
    }

    pub fn facts_by_id(&self, id: &str) -> impl ExactSizeIterator<Item = HbkFactRef> + '_ {
        lookup_id_all(&self.snapshot.fact_ids, self.snapshot, id)
    }

    pub fn platform_type_by_id(&self, id: &str) -> Option<HbkPlatformTypeId> {
        lookup_id(&self.snapshot.platform_type_ids, self.snapshot, id)
    }

    pub fn platform_types_by_name(
        &self,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkPlatformTypeId> + '_ {
        lookup_name(&self.snapshot.platform_type_names, self.snapshot, name)
    }

    pub fn platform_types_by_template_key(
        &self,
        family: &str,
        variant: &str,
    ) -> impl ExactSizeIterator<Item = HbkPlatformTypeId> + '_ {
        lookup_type_template(
            &self.snapshot.platform_type_templates,
            self.snapshot,
            family,
            variant,
        )
    }

    pub fn members_of_type(&self, owner: HbkPlatformTypeId) -> &[HbkTypeMemberId] {
        self.snapshot.members_by_owner.values(owner)
    }

    pub fn member_by_owner_name(
        &self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkTypeMemberId> + '_ {
        lookup_owner_name(
            &self.snapshot.members_by_owner_name,
            self.snapshot,
            owner,
            name,
        )
    }

    pub fn member_by_owner_name_kind(
        &self,
        owner: HbkPlatformTypeId,
        name: &str,
        kind: Option<HbkTypeMemberKind>,
    ) -> MemberLookupIter<'_> {
        let Some(kind) = kind else {
            let key = normalize_lookup_key(name);
            let range = matching_range(&self.snapshot.members_by_owner_name, |candidate| {
                candidate
                    .owner
                    .cmp(&owner)
                    .then_with(|| self.snapshot.string(candidate.key).cmp(&key))
            });
            return MemberLookupIter {
                inner: MemberLookupIterInner::Name(
                    self.snapshot.members_by_owner_name[range].iter(),
                ),
            };
        };
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.members_by_owner_name_kind, |candidate| {
            candidate
                .owner
                .cmp(&owner)
                .then_with(|| self.snapshot.string(candidate.key).cmp(&key))
                .then_with(|| candidate.kind.cmp(&Some(kind)))
        });
        MemberLookupIter {
            inner: MemberLookupIterInner::NameKind(
                self.snapshot.members_by_owner_name_kind[range].iter(),
            ),
        }
    }

    pub fn callables_of_type(&self, owner: HbkPlatformTypeId) -> &[HbkCallableId] {
        self.snapshot.callables_by_owner.values(owner)
    }

    pub fn callable_by_owner_name(
        &self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkCallableId> + '_ {
        lookup_owner_name(
            &self.snapshot.callables_by_owner_name,
            self.snapshot,
            owner,
            name,
        )
    }

    pub fn constructors_of_type(&self, owner: HbkPlatformTypeId) -> &[HbkCallableId] {
        self.snapshot.constructors_by_type.values(owner)
    }

    pub fn globals_by_name(
        &self,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkGlobalFactId> + '_ {
        lookup_name(&self.snapshot.global_names, self.snapshot, name)
    }

    pub fn globals_by_domain_name_kind(
        &self,
        domain: HbkLanguageDomain,
        name: &str,
        kind: Option<HbkGlobalFactKind>,
    ) -> impl ExactSizeIterator<Item = HbkGlobalFactId> + '_ {
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.globals_by_domain_name_kind, |candidate| {
            let ordering = candidate
                .domain
                .cmp(&domain)
                .then_with(|| self.snapshot.string(candidate.key).cmp(&key));
            if let Some(kind) = kind {
                ordering.then_with(|| candidate.kind.cmp(&Some(kind)))
            } else {
                ordering
            }
        });
        self.snapshot.globals_by_domain_name_kind[range]
            .iter()
            .map(|candidate| candidate.value)
    }

    pub fn module_events(
        &self,
        module_context_key: &str,
    ) -> impl ExactSizeIterator<Item = HbkCallableId> + '_ {
        let key = normalize_lookup_key(module_context_key);
        lookup_owner_name_by_key(&self.snapshot.module_event_names, self.snapshot, &key)
    }

    pub fn module_event_by_context_name(
        &self,
        module_context_key: &str,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkCallableId> + '_ {
        let owner = normalize_lookup_key(module_context_key);
        lookup_owner_name_by_key_and_name(
            &self.snapshot.module_event_names,
            self.snapshot,
            &owner,
            name,
        )
    }

    pub fn module_context_events(
        &self,
        domain: HbkLanguageDomain,
        language_key: &str,
        module_kind: &str,
    ) -> impl ExactSizeIterator<Item = HbkCallableId> + '_ {
        let language_key = normalize_lookup_key(language_key);
        let module_kind = normalize_lookup_key(module_kind);
        let range = matching_range(
            &self.snapshot.module_contexts_by_domain_language_kind,
            |candidate| {
                candidate
                    .domain
                    .cmp(&domain)
                    .then_with(|| {
                        self.snapshot
                            .string(candidate.language_key)
                            .cmp(&language_key)
                    })
                    .then_with(|| {
                        self.snapshot
                            .string(candidate.module_kind)
                            .cmp(&module_kind)
                    })
            },
        );
        self.snapshot.module_contexts_by_domain_language_kind[range]
            .iter()
            .map(|candidate| candidate.value)
    }

    pub fn query_table_by_id(&self, id: &str) -> Option<HbkQueryTableId> {
        lookup_id(&self.snapshot.query_table_ids, self.snapshot, id)
    }

    pub fn query_tables_by_name(
        &self,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkQueryTableId> + '_ {
        lookup_name(&self.snapshot.query_table_names, self.snapshot, name)
    }

    pub fn query_tables_by_syntax(
        &self,
        syntax: &str,
    ) -> impl ExactSizeIterator<Item = HbkQueryTableId> + '_ {
        lookup_name(
            &self.snapshot.query_table_syntax_names,
            self.snapshot,
            syntax,
        )
    }

    pub fn query_tables_by_identifier(
        &self,
        identifier: &str,
    ) -> impl ExactSizeIterator<Item = HbkQueryTableId> + '_ {
        lookup_name(
            &self.snapshot.query_table_identifiers,
            self.snapshot,
            identifier,
        )
    }

    pub fn query_fields(&self, table: HbkQueryTableId) -> &[HbkQueryFieldId] {
        self.snapshot.query_fields_by_table.values(table)
    }

    pub fn query_fields_by_name(
        &self,
        table: HbkQueryTableId,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkQueryFieldId> + '_ {
        lookup_owner_name(
            &self.snapshot.query_fields_by_table_name,
            self.snapshot,
            table,
            name,
        )
    }

    pub fn query_parameters(&self, table: HbkQueryTableId) -> &[HbkQueryParameterId] {
        self.snapshot.query_parameters_by_table.values(table)
    }

    pub fn query_parameters_by_name(
        &self,
        table: HbkQueryTableId,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkQueryParameterId> + '_ {
        lookup_owner_name(
            &self.snapshot.query_parameters_by_table_name,
            self.snapshot,
            table,
            name,
        )
    }

    pub fn language_fact_by_id(&self, id: &str) -> Option<HbkLanguageFactId> {
        lookup_id(&self.snapshot.language_ids, self.snapshot, id)
    }

    pub fn language_facts_by_name(
        &self,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkLanguageFactId> + '_ {
        lookup_name(&self.snapshot.language_names, self.snapshot, name)
    }

    pub fn enum_by_id(&self, id: &str) -> Option<HbkEnumId> {
        lookup_id(&self.snapshot.enum_ids, self.snapshot, id)
    }

    pub fn enums_by_name(&self, name: &str) -> impl ExactSizeIterator<Item = HbkEnumId> + '_ {
        lookup_name(&self.snapshot.enum_names, self.snapshot, name)
    }

    pub fn enum_value_by_id(&self, id: &str) -> Option<HbkEnumValueId> {
        lookup_id(&self.snapshot.enum_value_ids, self.snapshot, id)
    }

    pub fn enum_values(&self, owner: HbkEnumId) -> &[HbkEnumValueId] {
        self.snapshot.enum_values_by_enum.values(owner)
    }

    pub fn enum_values_by_name(
        &self,
        owner: HbkEnumId,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkEnumValueId> + '_ {
        lookup_owner_name(
            &self.snapshot.enum_values_by_enum_name,
            self.snapshot,
            owner,
            name,
        )
    }

    pub fn availability_contexts(&self, fact: HbkFactRef) -> &[StringId] {
        self.snapshot.availability_by_fact.values(fact)
    }

    pub fn available_since(&self, fact: HbkFactRef) -> Option<StringId> {
        self.snapshot
            .availability_since_by_fact
            .binary_search_by(|candidate| candidate.fact.cmp(&fact))
            .ok()
            .map(|position| self.snapshot.availability_since_by_fact[position].value)
    }

    pub fn relations_by_source_kind(&self, source: HbkFactRef, kind: &str) -> &[HbkFactRef] {
        let kind = normalize_lookup_key(kind);
        let Some(kind) = self.snapshot.string_id(&kind) else {
            return &[];
        };
        self.snapshot
            .relations_by_source_kind
            .values(RelationLookupKey { source, kind })
    }
}
