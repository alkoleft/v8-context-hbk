use super::materialize::SnapshotBuilder;
use super::memory::{HbkFactSnapshotMemoryEntry, vec_heap_bytes, vec_payload_bytes};
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct IdLookup<T> {
    pub(super) key: StringId,
    pub(super) value: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct NameLookup<T> {
    pub(super) key: StringId,
    pub(super) value: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OwnerNameLookup<Owner, Value> {
    pub(super) owner: Owner,
    pub(super) key: StringId,
    pub(super) value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TypeTemplateLookup<T> {
    pub(super) family: StringId,
    pub(super) variant: StringId,
    pub(super) value: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MemberNameKindLookup {
    pub(super) owner: HbkPlatformTypeId,
    pub(super) key: StringId,
    pub(super) kind: Option<HbkTypeMemberKind>,
    pub(super) value: HbkTypeMemberId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct GlobalNameKindLookup {
    pub(super) domain: HbkLanguageDomain,
    pub(super) key: StringId,
    pub(super) kind: Option<HbkGlobalFactKind>,
    pub(super) value: HbkGlobalFactId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FactStringLookup {
    pub(super) fact: HbkFactRef,
    pub(super) value: StringId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FactSourceLookup {
    pub(super) fact: HbkFactRef,
    pub(super) source: HbkFactSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ModuleContextLookup {
    pub(super) domain: HbkLanguageDomain,
    pub(super) language_key: StringId,
    pub(super) module_kind: StringId,
    pub(super) value: HbkCallableId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct RelationLookupKey {
    pub(super) source: HbkFactRef,
    pub(super) kind: StringId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CsrIndex<K, V> {
    pub(super) keys: Vec<K>,
    pub(super) offsets: Vec<u32>,
    pub(super) values: Vec<V>,
}
impl<K, V> CsrIndex<K, V>
where
    K: Copy + Ord,
    V: Copy + Ord,
{
    pub(super) fn from_pairs(mut pairs: Vec<(K, V)>) -> Self {
        pairs.sort();
        pairs.dedup();
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

    pub(super) fn values(&self, key: K) -> &[V] {
        let Ok(index) = self.keys.binary_search(&key) else {
            return &[];
        };
        let start = self.offsets[index] as usize;
        let end = self.offsets[index + 1] as usize;
        &self.values[start..end]
    }

    pub(super) fn estimated_heap_bytes(&self) -> usize {
        vec_heap_bytes(&self.keys) + vec_heap_bytes(&self.offsets) + vec_heap_bytes(&self.values)
    }

    pub(super) fn memory_entry(&self) -> HbkFactSnapshotMemoryEntry {
        HbkFactSnapshotMemoryEntry {
            count: self.values.len(),
            bytes: self.estimated_heap_bytes(),
            payload_bytes: vec_payload_bytes(&self.keys)
                + vec_payload_bytes(&self.offsets)
                + vec_payload_bytes(&self.values),
        }
    }
}
pub(super) fn push_id_lookup<T: Copy>(
    output: &mut Vec<IdLookup<T>>,
    builder: &mut SnapshotBuilder,
    key: &str,
    value: T,
) {
    let key = builder.intern(key);
    output.push(IdLookup { key, value });
}

pub(super) fn push_name_lookups<T: Copy>(
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

pub(super) fn push_lookup<T: Copy>(
    output: &mut Vec<NameLookup<T>>,
    builder: &mut SnapshotBuilder,
    key: &str,
    value: T,
) {
    let key = builder.intern(key);
    output.push(NameLookup { key, value });
}

pub(super) fn push_owner_name_lookups<Owner: Copy, Value: Copy>(
    output: &mut Vec<OwnerNameLookup<Owner, Value>>,
    builder: &mut SnapshotBuilder,
    owner: Owner,
    name: &model::LocalizedName,
    value: Value,
) {
    push_owner_lookup(
        output,
        builder,
        owner,
        &normalize_lookup_key(&name.primary),
        value,
    );
    if let Some(alias) = &name.alias {
        push_owner_lookup(output, builder, owner, &normalize_lookup_key(alias), value);
    }
}

pub(super) fn push_member_name_kind_lookups(
    output: &mut Vec<MemberNameKindLookup>,
    builder: &mut SnapshotBuilder,
    owner: HbkPlatformTypeId,
    name: &model::LocalizedName,
    kind: HbkTypeMemberKind,
    value: HbkTypeMemberId,
) {
    push_member_name_kind_lookup(
        output,
        builder,
        owner,
        &normalize_lookup_key(&name.primary),
        kind,
        value,
    );
    if let Some(alias) = &name.alias {
        push_member_name_kind_lookup(
            output,
            builder,
            owner,
            &normalize_lookup_key(alias),
            kind,
            value,
        );
    }
}

pub(super) fn push_member_name_kind_lookup(
    output: &mut Vec<MemberNameKindLookup>,
    builder: &mut SnapshotBuilder,
    owner: HbkPlatformTypeId,
    key: &str,
    kind: HbkTypeMemberKind,
    value: HbkTypeMemberId,
) {
    let key = builder.intern(key);
    output.push(MemberNameKindLookup {
        owner,
        key,
        kind: Some(kind),
        value,
    });
}

pub(super) fn push_global_name_kind_lookups(
    output: &mut Vec<GlobalNameKindLookup>,
    builder: &mut SnapshotBuilder,
    domain: HbkLanguageDomain,
    name: &model::LocalizedName,
    kind: HbkGlobalFactKind,
    value: HbkGlobalFactId,
) {
    push_global_name_kind_lookup(
        output,
        builder,
        domain,
        &normalize_lookup_key(&name.primary),
        kind,
        value,
    );
    if let Some(alias) = &name.alias {
        push_global_name_kind_lookup(
            output,
            builder,
            domain,
            &normalize_lookup_key(alias),
            kind,
            value,
        );
    }
}

pub(super) fn push_global_name_kind_lookup(
    output: &mut Vec<GlobalNameKindLookup>,
    builder: &mut SnapshotBuilder,
    domain: HbkLanguageDomain,
    key: &str,
    kind: HbkGlobalFactKind,
    value: HbkGlobalFactId,
) {
    let key = builder.intern(key);
    output.push(GlobalNameKindLookup {
        domain,
        key,
        kind: Some(kind),
        value,
    });
}

pub(super) fn push_owner_lookup<Owner: Copy, Value: Copy>(
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

pub(super) fn sorted_id_lookup<T: Copy + Ord>(
    mut values: Vec<IdLookup<T>>,
    builder: &SnapshotBuilder,
) -> Vec<IdLookup<T>> {
    values.sort_by(|left, right| {
        builder
            .string(left.key)
            .cmp(builder.string(right.key))
            .then_with(|| left.value.cmp(&right.value))
    });
    values.dedup_by(|left, right| left.key == right.key && left.value == right.value);
    values
}

pub(super) fn sorted_name_lookup<T: Copy + Ord>(
    mut values: Vec<NameLookup<T>>,
    builder: &SnapshotBuilder,
) -> Vec<NameLookup<T>> {
    values.sort_by(|left, right| {
        builder
            .string(left.key)
            .cmp(builder.string(right.key))
            .then_with(|| left.value.cmp(&right.value))
    });
    values.dedup_by(|left, right| left.key == right.key && left.value == right.value);
    values
}

pub(super) fn sorted_type_template_lookup<T: Copy + Ord>(
    mut values: Vec<TypeTemplateLookup<T>>,
    builder: &SnapshotBuilder,
) -> Vec<TypeTemplateLookup<T>> {
    values.sort_by(|left, right| {
        builder
            .string(left.family)
            .cmp(builder.string(right.family))
            .then_with(|| {
                builder
                    .string(left.variant)
                    .cmp(builder.string(right.variant))
            })
            .then_with(|| left.value.cmp(&right.value))
    });
    values.dedup_by(|left, right| {
        left.family == right.family && left.variant == right.variant && left.value == right.value
    });
    values
}

pub(super) fn sorted_owner_name_lookup<Owner: Copy + Ord, Value: Copy + Ord>(
    mut values: Vec<OwnerNameLookup<Owner, Value>>,
    builder: &SnapshotBuilder,
) -> Vec<OwnerNameLookup<Owner, Value>> {
    values.sort_by(|left, right| {
        left.owner
            .cmp(&right.owner)
            .then_with(|| builder.string(left.key).cmp(builder.string(right.key)))
            .then_with(|| left.value.cmp(&right.value))
    });
    values.dedup_by(|left, right| {
        left.owner == right.owner && left.key == right.key && left.value == right.value
    });
    values
}

pub(super) fn sorted_member_name_kind_lookup(
    mut values: Vec<MemberNameKindLookup>,
    builder: &SnapshotBuilder,
) -> Vec<MemberNameKindLookup> {
    values.sort_by(|left, right| {
        left.owner
            .cmp(&right.owner)
            .then_with(|| builder.string(left.key).cmp(builder.string(right.key)))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.value.cmp(&right.value))
    });
    values.dedup_by(|left, right| {
        left.owner == right.owner
            && left.key == right.key
            && left.kind == right.kind
            && left.value == right.value
    });
    values
}

pub(super) fn sorted_global_name_kind_lookup(
    mut values: Vec<GlobalNameKindLookup>,
    builder: &SnapshotBuilder,
) -> Vec<GlobalNameKindLookup> {
    values.sort_by(|left, right| {
        left.domain
            .cmp(&right.domain)
            .then_with(|| builder.string(left.key).cmp(builder.string(right.key)))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.value.cmp(&right.value))
    });
    values.dedup_by(|left, right| {
        left.domain == right.domain
            && left.key == right.key
            && left.kind == right.kind
            && left.value == right.value
    });
    values
}

pub(super) fn sorted_string_owner_name_lookup<Value: Copy + Ord>(
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
    values.dedup_by(|left, right| {
        left.owner == right.owner && left.key == right.key && left.value == right.value
    });
    values
}

pub(super) fn sorted_module_context_lookup(
    mut values: Vec<ModuleContextLookup>,
    builder: &SnapshotBuilder,
) -> Vec<ModuleContextLookup> {
    values.sort_by(|left, right| {
        left.domain
            .cmp(&right.domain)
            .then_with(|| {
                builder
                    .string(left.language_key)
                    .cmp(builder.string(right.language_key))
            })
            .then_with(|| {
                builder
                    .string(left.module_kind)
                    .cmp(builder.string(right.module_kind))
            })
            .then_with(|| left.value.cmp(&right.value))
    });
    values.dedup_by(|left, right| {
        left.domain == right.domain
            && left.language_key == right.language_key
            && left.module_kind == right.module_kind
            && left.value == right.value
    });
    values
}

pub(super) fn lookup_id<T: Copy>(
    index: &[IdLookup<T>],
    snapshot: &HbkFactSnapshot,
    key: &str,
) -> Option<T> {
    index
        .binary_search_by(|candidate| snapshot.string(candidate.key).cmp(key))
        .ok()
        .map(|position| index[position].value)
}

pub(super) fn matching_range<T, F>(values: &[T], mut compare: F) -> std::ops::Range<usize>
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
