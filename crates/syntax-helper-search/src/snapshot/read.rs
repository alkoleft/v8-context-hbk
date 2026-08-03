use super::indexes::*;
use super::x1_format::{
    X1FilteredGlobalIter, X1FilteredMemberIter, X1LookupValueIter, X1MemberLookupIter,
};
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

struct OwnedLookupIter<'a, Record, Value> {
    records: std::slice::Iter<'a, Record>,
    value: fn(&Record) -> Value,
}

impl<Record, Value> Iterator for OwnedLookupIter<'_, Record, Value> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.records.next().map(self.value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<Record, Value> ExactSizeIterator for OwnedLookupIter<'_, Record, Value> {
    fn len(&self) -> usize {
        self.records.len()
    }
}

enum LookupIter<'a, Record, Value> {
    Owned(OwnedLookupIter<'a, Record, Value>),
    Mapped(X1LookupValueIter<'a, Record, Value>),
}

impl<Record: super::codec::BinaryValue, Value> Iterator for LookupIter<'_, Record, Value> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Owned(values) => values.next(),
            Self::Mapped(values) => values.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<Record: super::codec::BinaryValue, Value> ExactSizeIterator for LookupIter<'_, Record, Value> {
    fn len(&self) -> usize {
        match self {
            Self::Owned(values) => values.len(),
            Self::Mapped(values) => values.len(),
        }
    }
}

fn owned_lookup<'a, Record, Value>(
    records: &'a [Record],
    range: std::ops::Range<usize>,
    value: fn(&Record) -> Value,
) -> LookupIter<'a, Record, Value> {
    LookupIter::Owned(OwnedLookupIter {
        records: records[range].iter(),
        value,
    })
}

pub struct MemberLookupIter<'a> {
    inner: MemberLookupIterInner<'a>,
}

enum MemberLookupIterInner<'a> {
    Name(OwnedLookupIter<'a, OwnerNameLookup<HbkPlatformTypeId, HbkTypeMemberId>, HbkTypeMemberId>),
    NameKind(OwnedLookupIter<'a, MemberNameKindLookup, HbkTypeMemberId>),
    Mapped(X1MemberLookupIter<'a>),
}

impl Iterator for MemberLookupIter<'_> {
    type Item = HbkTypeMemberId;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            MemberLookupIterInner::Name(values) => values.next(),
            MemberLookupIterInner::NameKind(values) => values.next(),
            MemberLookupIterInner::Mapped(values) => values.next(),
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
            MemberLookupIterInner::Name(values) => values.len(),
            MemberLookupIterInner::NameKind(values) => values.len(),
            MemberLookupIterInner::Mapped(values) => values.len(),
        }
    }
}

pub struct HbkFilteredGlobalIdIter<'a> {
    inner: HbkFilteredGlobalIdIterInner<'a>,
}

enum HbkFilteredGlobalIdIterInner<'a> {
    Owned {
        snapshot: &'a HbkFactSnapshot,
        index: usize,
        filter: HbkAvailabilityFilter,
        kind: Option<HbkGlobalFactKind>,
    },
    Mapped(X1FilteredGlobalIter<'a>),
}

impl Iterator for HbkFilteredGlobalIdIter<'_> {
    type Item = HbkGlobalFactId;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            HbkFilteredGlobalIdIterInner::Mapped(values) => values.next(),
            HbkFilteredGlobalIdIterInner::Owned {
                snapshot,
                index,
                filter,
                kind,
            } => {
                while *index < snapshot.globals.len() {
                    let id = HbkGlobalFactId(*index as u32);
                    *index += 1;
                    let global = &snapshot.globals[id.0 as usize];
                    if kind.is_some_and(|kind| kind != global.kind) {
                        continue;
                    }
                    if owned_availability_matches(snapshot, *filter, HbkFactRef::Global(id)) {
                        return Some(id);
                    }
                }
                None
            }
        }
    }
}

pub struct HbkFilteredMemberIdIter<'a> {
    inner: HbkFilteredMemberIdIterInner<'a>,
}

enum HbkFilteredMemberIdIterInner<'a> {
    Owned {
        snapshot: &'a HbkFactSnapshot,
        ids: std::slice::Iter<'a, HbkTypeMemberId>,
        filter: HbkAvailabilityFilter,
        kind: Option<HbkTypeMemberKind>,
    },
    Mapped(X1FilteredMemberIter<'a>),
}

impl Iterator for HbkFilteredMemberIdIter<'_> {
    type Item = HbkTypeMemberId;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            HbkFilteredMemberIdIterInner::Mapped(values) => values.next(),
            HbkFilteredMemberIdIterInner::Owned {
                snapshot,
                ids,
                filter,
                kind,
            } => {
                for id in ids.by_ref().copied() {
                    let member = &snapshot.type_members[id.0 as usize];
                    if kind.is_some_and(|kind| kind != member.kind) {
                        continue;
                    }
                    if owned_availability_matches(snapshot, *filter, HbkFactRef::TypeMember(id)) {
                        return Some(id);
                    }
                }
                None
            }
        }
    }
}

fn owned_availability_matches(
    snapshot: &HbkFactSnapshot,
    filter: HbkAvailabilityFilter,
    fact: HbkFactRef,
) -> bool {
    let contexts = snapshot.availability_by_fact.values(fact);
    if contexts.is_empty() {
        return true;
    }
    let mut available_mask = 0_u16;
    for context in contexts {
        let Some(bit) = availability_context_code_bit(snapshot.string(*context)) else {
            return false;
        };
        available_mask |= bit;
    }
    filter.includes_mask(available_mask, true)
}

impl HbkFactSnapshot {
    pub fn worker_handle(&self) -> HbkFactReadHandle<'_> {
        HbkFactReadHandle { snapshot: self }
    }

    pub fn string(&self, id: StringId) -> &str {
        if let Some(mapped) = self.mapped_read_handle() {
            return mapped.string(id);
        }
        &self.strings[id.0 as usize]
    }

    pub fn source_locale(&self) -> Option<&str> {
        if let Some(mapped) = self.mapped_read_handle() {
            return Some(mapped.source_locale());
        }
        self.source_locale.map(|id| self.string(id))
    }

    fn string_id(&self, value: &str) -> Option<StringId> {
        if let Some(mapped) = self.mapped_read_handle() {
            return mapped.string_id(value);
        }
        self.strings
            .iter()
            .position(|candidate| candidate == value)
            .map(|index| StringId(index as u32))
    }

    /// Returns an owned build/oracle record.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Runtime readers must use
    /// [`HbkFactReadHandle::platform_type`] to obtain a storage-neutral view.
    pub fn platform_type(&self, id: HbkPlatformTypeId) -> &HbkPlatformType {
        self.assert_owned();
        &self.platform_types[id.0 as usize]
    }

    /// Returns an owned build/oracle record.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Runtime readers must use
    /// [`HbkFactReadHandle::type_member`] to obtain a storage-neutral view.
    pub fn type_member(&self, id: HbkTypeMemberId) -> &HbkTypeMember {
        self.assert_owned();
        &self.type_members[id.0 as usize]
    }

    /// Returns an owned build/oracle record.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Runtime readers must use
    /// [`HbkFactReadHandle::callable`] to obtain a storage-neutral view.
    pub fn callable(&self, id: HbkCallableId) -> &HbkCallable {
        self.assert_owned();
        &self.callables[id.0 as usize]
    }

    /// Returns an owned build/oracle record.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Runtime readers must use
    /// [`HbkFactReadHandle::global_fact`] to obtain a storage-neutral view.
    pub fn global_fact(&self, id: HbkGlobalFactId) -> &HbkGlobalFact {
        self.assert_owned();
        &self.globals[id.0 as usize]
    }

    /// Returns an owned build/oracle record.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Runtime readers must use
    /// [`HbkFactReadHandle::query_table`] to obtain a storage-neutral view.
    pub fn query_table(&self, id: HbkQueryTableId) -> &HbkQueryTable {
        self.assert_owned();
        &self.query_tables[id.0 as usize]
    }

    /// Returns an owned build/oracle record.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Runtime readers must use
    /// [`HbkFactReadHandle::query_field`] to obtain a storage-neutral view.
    pub fn query_field(&self, id: HbkQueryFieldId) -> &HbkQueryField {
        self.assert_owned();
        &self.query_fields[id.0 as usize]
    }

    /// Returns an owned build/oracle record.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Runtime readers must use
    /// [`HbkFactReadHandle::query_parameter`] to obtain a storage-neutral view.
    pub fn query_parameter(&self, id: HbkQueryParameterId) -> &HbkQueryParameter {
        self.assert_owned();
        &self.query_parameters[id.0 as usize]
    }

    /// Returns an owned build/oracle record.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Runtime readers must use
    /// [`HbkFactReadHandle::language_fact`] to obtain a storage-neutral view.
    pub fn language_fact(&self, id: HbkLanguageFactId) -> &HbkLanguageFact {
        self.assert_owned();
        &self.language_facts[id.0 as usize]
    }

    /// Returns an owned build/oracle record.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Runtime readers must use
    /// [`HbkFactReadHandle::enum_fact`] to obtain a storage-neutral view.
    pub fn enum_fact(&self, id: HbkEnumId) -> &HbkEnum {
        self.assert_owned();
        &self.enums[id.0 as usize]
    }

    /// Returns an owned build/oracle record.
    ///
    /// # Panics
    /// Panics for an X1-mapped snapshot. Runtime readers must use
    /// [`HbkFactReadHandle::enum_value`] to obtain a storage-neutral view.
    pub fn enum_value(&self, id: HbkEnumValueId) -> &HbkEnumValue {
        self.assert_owned();
        &self.enum_values[id.0 as usize]
    }

    pub fn counts(&self) -> HbkFactSnapshotCounts {
        if let Some(mapped) = self.mapped_generation.as_deref() {
            return mapped.counts();
        }
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
    fn mapped(self) -> Option<super::x1_format::X1MappedReadHandle<'a>> {
        self.snapshot.mapped_read_handle()
    }

    pub fn string(self, id: StringId) -> &'a str {
        self.snapshot.string(id)
    }

    pub fn source_locale(self) -> Option<&'a str> {
        self.snapshot.source_locale()
    }

    pub fn global_fact_ids(self) -> impl ExactSizeIterator<Item = HbkGlobalFactId> + 'a + use<'a> {
        let len = self.snapshot.counts().globals as u32;
        (0..len).map(HbkGlobalFactId)
    }

    pub fn query_table_ids(self) -> impl ExactSizeIterator<Item = HbkQueryTableId> + 'a + use<'a> {
        let len = self.snapshot.counts().query_tables as u32;
        (0..len).map(HbkQueryTableId)
    }

    pub fn query_field_ids(self) -> impl ExactSizeIterator<Item = HbkQueryFieldId> + 'a + use<'a> {
        let len = self.snapshot.counts().query_fields as u32;
        (0..len).map(HbkQueryFieldId)
    }

    pub fn query_parameter_ids(
        self,
    ) -> impl ExactSizeIterator<Item = HbkQueryParameterId> + 'a + use<'a> {
        let len = self.snapshot.counts().query_parameters as u32;
        (0..len).map(HbkQueryParameterId)
    }

    pub fn facts_by_id(self, id: &str) -> impl ExactSizeIterator<Item = HbkFactRef> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.facts_by_id(id));
        }
        let range = matching_range(&self.snapshot.fact_ids, |candidate| {
            self.snapshot.string(candidate.key).cmp(id)
        });
        owned_lookup(&self.snapshot.fact_ids, range, |candidate| candidate.value)
    }

    pub fn platform_type_by_id(self, id: &str) -> Option<HbkPlatformTypeId> {
        self.mapped().map_or_else(
            || lookup_id(&self.snapshot.platform_type_ids, self.snapshot, id),
            |mapped| mapped.platform_type_by_id(id),
        )
    }

    pub fn platform_types_by_name(
        self,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkPlatformTypeId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.platform_types_by_name(name));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.platform_type_names, |candidate| {
            self.snapshot.string(candidate.key).cmp(&key)
        });
        owned_lookup(&self.snapshot.platform_type_names, range, |candidate| {
            candidate.value
        })
    }

    pub fn platform_types_by_template_key(
        self,
        family: &str,
        variant: &str,
    ) -> impl ExactSizeIterator<Item = HbkPlatformTypeId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.platform_types_by_template_key(family, variant));
        }
        let range = matching_range(&self.snapshot.platform_type_templates, |candidate| {
            self.snapshot
                .string(candidate.family)
                .cmp(family)
                .then_with(|| self.snapshot.string(candidate.variant).cmp(variant))
        });
        owned_lookup(&self.snapshot.platform_type_templates, range, |candidate| {
            candidate.value
        })
    }

    pub fn members_of_type(self, owner: HbkPlatformTypeId) -> HbkTypeMemberIdIter<'a> {
        self.mapped().map_or_else(
            || HbkTypeMemberIdIter::owned(self.snapshot.members_by_owner.values(owner)),
            |mapped| HbkTypeMemberIdIter::mapped(mapped.members_of_type(owner)),
        )
    }

    pub fn member_by_owner_name(
        self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkTypeMemberId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.member_by_owner_name(owner, name));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.members_by_owner_name, |candidate| {
            candidate
                .owner
                .cmp(&owner)
                .then_with(|| self.snapshot.string(candidate.key).cmp(&key))
        });
        owned_lookup(&self.snapshot.members_by_owner_name, range, |candidate| {
            candidate.value
        })
    }

    pub fn member_by_owner_name_kind(
        self,
        owner: HbkPlatformTypeId,
        name: &str,
        kind: Option<HbkTypeMemberKind>,
    ) -> MemberLookupIter<'a> {
        if let Some(mapped) = self.mapped() {
            return MemberLookupIter {
                inner: MemberLookupIterInner::Mapped(
                    mapped.member_by_owner_name_kind(owner, name, kind),
                ),
            };
        }
        let key = normalize_lookup_key(name);
        if let Some(kind) = kind {
            let range = matching_range(&self.snapshot.members_by_owner_name_kind, |candidate| {
                candidate
                    .owner
                    .cmp(&owner)
                    .then_with(|| self.snapshot.string(candidate.key).cmp(&key))
                    .then_with(|| candidate.kind.cmp(&Some(kind)))
            });
            return MemberLookupIter {
                inner: MemberLookupIterInner::NameKind(OwnedLookupIter {
                    records: self.snapshot.members_by_owner_name_kind[range].iter(),
                    value: |candidate| candidate.value,
                }),
            };
        }
        let range = matching_range(&self.snapshot.members_by_owner_name, |candidate| {
            candidate
                .owner
                .cmp(&owner)
                .then_with(|| self.snapshot.string(candidate.key).cmp(&key))
        });
        MemberLookupIter {
            inner: MemberLookupIterInner::Name(OwnedLookupIter {
                records: self.snapshot.members_by_owner_name[range].iter(),
                value: |candidate| candidate.value,
            }),
        }
    }

    pub fn callables_of_type(self, owner: HbkPlatformTypeId) -> HbkCallableIdIter<'a> {
        self.mapped().map_or_else(
            || HbkCallableIdIter::owned(self.snapshot.callables_by_owner.values(owner)),
            |mapped| HbkCallableIdIter::mapped(mapped.callables_of_type(owner)),
        )
    }

    pub fn callable_by_owner_name(
        self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkCallableId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.callable_by_owner_name(owner, name));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.callables_by_owner_name, |candidate| {
            candidate
                .owner
                .cmp(&owner)
                .then_with(|| self.snapshot.string(candidate.key).cmp(&key))
        });
        owned_lookup(&self.snapshot.callables_by_owner_name, range, |candidate| {
            candidate.value
        })
    }

    pub fn constructors_of_type(self, owner: HbkPlatformTypeId) -> HbkCallableIdIter<'a> {
        self.mapped().map_or_else(
            || HbkCallableIdIter::owned(self.snapshot.constructors_by_type.values(owner)),
            |mapped| HbkCallableIdIter::mapped(mapped.constructors_of_type(owner)),
        )
    }

    pub fn globals_by_name(
        self,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkGlobalFactId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.globals_by_name(name));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.global_names, |candidate| {
            self.snapshot.string(candidate.key).cmp(&key)
        });
        owned_lookup(&self.snapshot.global_names, range, |candidate| {
            candidate.value
        })
    }

    pub fn filtered_global_ids(
        self,
        filter: HbkAvailabilityFilter,
        kind: Option<HbkGlobalFactKind>,
    ) -> HbkFilteredGlobalIdIter<'a> {
        if let Some(mapped) = self.mapped() {
            return HbkFilteredGlobalIdIter {
                inner: HbkFilteredGlobalIdIterInner::Mapped(
                    mapped.filtered_globals(filter.into(), kind),
                ),
            };
        }
        HbkFilteredGlobalIdIter {
            inner: HbkFilteredGlobalIdIterInner::Owned {
                snapshot: self.snapshot,
                index: 0,
                filter,
                kind,
            },
        }
    }

    pub fn filtered_members(
        self,
        owner: HbkPlatformTypeId,
        filter: HbkAvailabilityFilter,
        kind: Option<HbkTypeMemberKind>,
    ) -> HbkFilteredMemberIdIter<'a> {
        if let Some(mapped) = self.mapped() {
            return HbkFilteredMemberIdIter {
                inner: HbkFilteredMemberIdIterInner::Mapped(mapped.filtered_members(
                    owner,
                    filter.into(),
                    kind,
                )),
            };
        }
        HbkFilteredMemberIdIter {
            inner: HbkFilteredMemberIdIterInner::Owned {
                snapshot: self.snapshot,
                ids: self.snapshot.members_by_owner.values(owner).iter(),
                filter,
                kind,
            },
        }
    }

    pub fn globals_by_domain_name_kind(
        self,
        domain: HbkLanguageDomain,
        name: &str,
        kind: Option<HbkGlobalFactKind>,
    ) -> impl ExactSizeIterator<Item = HbkGlobalFactId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.globals_by_domain_name_kind(domain, name, kind));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.globals_by_domain_name_kind, |candidate| {
            let order = candidate
                .domain
                .cmp(&domain)
                .then_with(|| self.snapshot.string(candidate.key).cmp(&key));
            kind.map_or(order, |kind| {
                order.then_with(|| candidate.kind.cmp(&Some(kind)))
            })
        });
        owned_lookup(
            &self.snapshot.globals_by_domain_name_kind,
            range,
            |candidate| candidate.value,
        )
    }

    pub fn module_events(
        self,
        module_context_key: &str,
    ) -> impl ExactSizeIterator<Item = HbkCallableId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.module_events(module_context_key));
        }
        let key = normalize_lookup_key(module_context_key);
        let range = matching_range(&self.snapshot.module_event_names, |candidate| {
            self.snapshot.string(candidate.owner).cmp(&key)
        });
        owned_lookup(&self.snapshot.module_event_names, range, |candidate| {
            candidate.value
        })
    }

    pub fn module_event_by_context_name(
        self,
        module_context_key: &str,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkCallableId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(
                mapped.module_event_by_context_name(module_context_key, name),
            );
        }
        let owner = normalize_lookup_key(module_context_key);
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.module_event_names, |candidate| {
            self.snapshot
                .string(candidate.owner)
                .cmp(&owner)
                .then_with(|| self.snapshot.string(candidate.key).cmp(&key))
        });
        owned_lookup(&self.snapshot.module_event_names, range, |candidate| {
            candidate.value
        })
    }

    pub fn module_context_events(
        self,
        domain: HbkLanguageDomain,
        language_key: &str,
        module_kind: &str,
    ) -> impl ExactSizeIterator<Item = HbkCallableId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.module_context_events(
                domain,
                language_key,
                module_kind,
            ));
        }
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
        owned_lookup(
            &self.snapshot.module_contexts_by_domain_language_kind,
            range,
            |candidate| candidate.value,
        )
    }

    pub fn query_table_by_id(self, id: &str) -> Option<HbkQueryTableId> {
        self.mapped().map_or_else(
            || lookup_id(&self.snapshot.query_table_ids, self.snapshot, id),
            |mapped| mapped.query_table_by_id(id),
        )
    }

    pub fn query_tables_by_name(
        self,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkQueryTableId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.query_tables_by_name(name));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.query_table_names, |candidate| {
            self.snapshot.string(candidate.key).cmp(&key)
        });
        owned_lookup(&self.snapshot.query_table_names, range, |candidate| {
            candidate.value
        })
    }

    pub fn query_tables_by_syntax(
        self,
        syntax: &str,
    ) -> impl ExactSizeIterator<Item = HbkQueryTableId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.query_tables_by_syntax(syntax));
        }
        let key = normalize_lookup_key(syntax);
        let range = matching_range(&self.snapshot.query_table_syntax_names, |candidate| {
            self.snapshot.string(candidate.key).cmp(&key)
        });
        owned_lookup(
            &self.snapshot.query_table_syntax_names,
            range,
            |candidate| candidate.value,
        )
    }

    pub fn query_tables_by_identifier(
        self,
        identifier: &str,
    ) -> impl ExactSizeIterator<Item = HbkQueryTableId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.query_tables_by_identifier(identifier));
        }
        let key = normalize_lookup_key(identifier);
        let range = matching_range(&self.snapshot.query_table_identifiers, |candidate| {
            self.snapshot.string(candidate.key).cmp(&key)
        });
        owned_lookup(&self.snapshot.query_table_identifiers, range, |candidate| {
            candidate.value
        })
    }

    pub fn query_fields(self, table: HbkQueryTableId) -> HbkQueryFieldIdIter<'a> {
        self.mapped().map_or_else(
            || HbkQueryFieldIdIter::owned(self.snapshot.query_fields_by_table.values(table)),
            |mapped| HbkQueryFieldIdIter::mapped(mapped.query_fields(table)),
        )
    }

    pub fn query_fields_by_name(
        self,
        table: HbkQueryTableId,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkQueryFieldId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.query_fields_by_name(table, name));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.query_fields_by_table_name, |candidate| {
            candidate
                .owner
                .cmp(&table)
                .then_with(|| self.snapshot.string(candidate.key).cmp(&key))
        });
        owned_lookup(
            &self.snapshot.query_fields_by_table_name,
            range,
            |candidate| candidate.value,
        )
    }

    pub fn query_parameters(self, table: HbkQueryTableId) -> HbkQueryParameterIdIter<'a> {
        self.mapped().map_or_else(
            || {
                HbkQueryParameterIdIter::owned(
                    self.snapshot.query_parameters_by_table.values(table),
                )
            },
            |mapped| HbkQueryParameterIdIter::mapped(mapped.query_parameters(table)),
        )
    }

    pub fn query_parameters_by_name(
        self,
        table: HbkQueryTableId,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkQueryParameterId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.query_parameters_by_name(table, name));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.query_parameters_by_table_name, |candidate| {
            candidate
                .owner
                .cmp(&table)
                .then_with(|| self.snapshot.string(candidate.key).cmp(&key))
        });
        owned_lookup(
            &self.snapshot.query_parameters_by_table_name,
            range,
            |candidate| candidate.value,
        )
    }

    pub fn language_fact_by_id(self, id: &str) -> Option<HbkLanguageFactId> {
        self.mapped().map_or_else(
            || lookup_id(&self.snapshot.language_ids, self.snapshot, id),
            |mapped| mapped.language_fact_by_id(id),
        )
    }

    pub fn language_facts_by_name(
        self,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkLanguageFactId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.language_facts_by_name(name));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.language_names, |candidate| {
            self.snapshot.string(candidate.key).cmp(&key)
        });
        owned_lookup(&self.snapshot.language_names, range, |candidate| {
            candidate.value
        })
    }

    pub fn enum_by_id(self, id: &str) -> Option<HbkEnumId> {
        self.mapped().map_or_else(
            || lookup_id(&self.snapshot.enum_ids, self.snapshot, id),
            |mapped| mapped.enum_by_id(id),
        )
    }

    pub fn enums_by_name(
        self,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkEnumId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.enums_by_name(name));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.enum_names, |candidate| {
            self.snapshot.string(candidate.key).cmp(&key)
        });
        owned_lookup(&self.snapshot.enum_names, range, |candidate| {
            candidate.value
        })
    }

    pub fn enum_value_by_id(self, id: &str) -> Option<HbkEnumValueId> {
        self.mapped().map_or_else(
            || lookup_id(&self.snapshot.enum_value_ids, self.snapshot, id),
            |mapped| mapped.enum_value_by_id(id),
        )
    }

    pub fn enum_values(self, owner: HbkEnumId) -> HbkEnumValueIdIter<'a> {
        self.mapped().map_or_else(
            || HbkEnumValueIdIter::owned(self.snapshot.enum_values_by_enum.values(owner)),
            |mapped| HbkEnumValueIdIter::mapped(mapped.enum_values(owner)),
        )
    }

    pub fn enum_values_by_name(
        self,
        owner: HbkEnumId,
        name: &str,
    ) -> impl ExactSizeIterator<Item = HbkEnumValueId> + 'a + use<'a> {
        if let Some(mapped) = self.mapped() {
            return LookupIter::Mapped(mapped.enum_values_by_name(owner, name));
        }
        let key = normalize_lookup_key(name);
        let range = matching_range(&self.snapshot.enum_values_by_enum_name, |candidate| {
            candidate
                .owner
                .cmp(&owner)
                .then_with(|| self.snapshot.string(candidate.key).cmp(&key))
        });
        owned_lookup(
            &self.snapshot.enum_values_by_enum_name,
            range,
            |candidate| candidate.value,
        )
    }

    pub fn availability_contexts(self, fact: HbkFactRef) -> HbkStringIdIter<'a> {
        self.mapped().map_or_else(
            || HbkStringIdIter::owned(self.snapshot.availability_by_fact.values(fact)),
            |mapped| HbkStringIdIter::mapped(mapped.availability_contexts(fact)),
        )
    }

    pub fn available_since(self, fact: HbkFactRef) -> Option<StringId> {
        if let Some(mapped) = self.mapped() {
            return mapped.available_since(fact);
        }
        self.snapshot
            .availability_since_by_fact
            .binary_search_by(|candidate| candidate.fact.cmp(&fact))
            .ok()
            .map(|position| self.snapshot.availability_since_by_fact[position].value)
    }

    pub fn relations_by_source_kind(self, source: HbkFactRef, kind: &str) -> HbkFactRefIter<'a> {
        if let Some(mapped) = self.mapped() {
            return HbkFactRefIter::mapped(mapped.relations_by_source_kind(source, kind));
        }
        let kind = normalize_lookup_key(kind);
        let Some(kind) = self.snapshot.string_id(&kind) else {
            return HbkFactRefIter::owned(&[]);
        };
        HbkFactRefIter::owned(
            self.snapshot
                .relations_by_source_kind
                .values(RelationLookupKey { source, kind }),
        )
    }

    pub fn platform_type(self, id: HbkPlatformTypeId) -> HbkPlatformTypeView<'a> {
        self.mapped().map_or_else(
            || HbkPlatformTypeView::owned(&self.snapshot.platform_types[id.0 as usize]),
            |mapped| HbkPlatformTypeView::mapped(mapped.platform_type(id)),
        )
    }
    pub fn type_member(self, id: HbkTypeMemberId) -> HbkTypeMemberView<'a> {
        self.mapped().map_or_else(
            || HbkTypeMemberView::owned(self.snapshot, &self.snapshot.type_members[id.0 as usize]),
            |mapped| HbkTypeMemberView::mapped(mapped.type_member(id)),
        )
    }
    pub fn callable(self, id: HbkCallableId) -> HbkCallableView<'a> {
        self.mapped().map_or_else(
            || HbkCallableView::owned(self.snapshot, &self.snapshot.callables[id.0 as usize]),
            |mapped| HbkCallableView::mapped(mapped.callable(id)),
        )
    }
    pub fn global_fact(self, id: HbkGlobalFactId) -> HbkGlobalFactView<'a> {
        self.mapped().map_or_else(
            || HbkGlobalFactView::owned(self.snapshot, &self.snapshot.globals[id.0 as usize]),
            |mapped| HbkGlobalFactView::mapped(mapped.global(id)),
        )
    }
    pub fn query_table(self, id: HbkQueryTableId) -> HbkQueryTableView<'a> {
        self.mapped().map_or_else(
            || HbkQueryTableView::owned(&self.snapshot.query_tables[id.0 as usize]),
            |mapped| HbkQueryTableView::mapped(mapped.query_table(id)),
        )
    }
    pub fn query_field(self, id: HbkQueryFieldId) -> HbkQueryFieldView<'a> {
        self.mapped().map_or_else(
            || HbkQueryFieldView::owned(&self.snapshot.query_fields[id.0 as usize]),
            |mapped| HbkQueryFieldView::mapped(mapped.query_field(id)),
        )
    }
    pub fn query_parameter(self, id: HbkQueryParameterId) -> HbkQueryParameterView<'a> {
        self.mapped().map_or_else(
            || HbkQueryParameterView::owned(&self.snapshot.query_parameters[id.0 as usize]),
            |mapped| HbkQueryParameterView::mapped(mapped.query_parameter(id)),
        )
    }
    pub fn language_fact(self, id: HbkLanguageFactId) -> HbkLanguageFactView<'a> {
        self.mapped().map_or_else(
            || {
                HbkLanguageFactView::owned(
                    self.snapshot,
                    &self.snapshot.language_facts[id.0 as usize],
                )
            },
            |mapped| HbkLanguageFactView::mapped(mapped.language_fact(id)),
        )
    }
    pub fn enum_fact(self, id: HbkEnumId) -> HbkEnumView<'a> {
        self.mapped().map_or_else(
            || HbkEnumView::owned(&self.snapshot.enums[id.0 as usize]),
            |mapped| HbkEnumView::mapped(mapped.enum_fact(id)),
        )
    }
    pub fn enum_value(self, id: HbkEnumValueId) -> HbkEnumValueView<'a> {
        self.mapped().map_or_else(
            || HbkEnumValueView::owned(&self.snapshot.enum_values[id.0 as usize]),
            |mapped| HbkEnumValueView::mapped(mapped.enum_value(id)),
        )
    }
    pub fn source(self, fact: HbkFactRef) -> Option<HbkFactSourceView<'a>> {
        if let Some(mapped) = self.mapped() {
            return mapped.source(fact).map(HbkFactSourceView::mapped);
        }
        self.snapshot
            .source_by_fact
            .binary_search_by(|candidate| candidate.fact.cmp(&fact))
            .ok()
            .map(|index| HbkFactSourceView::owned(&self.snapshot.source_by_fact[index].source))
    }
}
