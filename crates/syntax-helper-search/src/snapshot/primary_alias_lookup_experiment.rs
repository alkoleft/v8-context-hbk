use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::hint::black_box;
use std::mem::size_of;
#[cfg(feature = "snapshot-experiment-alloc")]
use std::path::PathBuf;
use std::time::Instant;

use super::indexes::matching_range;
use super::materialize::SnapshotBuilder;
use super::*;

const CORPUS_ENV: &str = "V8_CONTEXT_HBK_PRIMARY_ALIAS_INDEX";
const FROZEN_PLATFORM_VERSION: &str = "8.3.27.1859";
const FROZEN_LOCALE: &str = "ru";
const FROZEN_EXTRACTION_SCHEMA: u32 = 11;
const GLOBAL_OWNER_BITS: u32 = u32::MAX;
const WARMUP_SAMPLES: usize = 2;
const MEASURED_SAMPLES: usize = 9;
const LOOKUP_PASSES: usize = 64;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TypeId(u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CallableNameId(u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PropertyNameId(u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct OwnerId(u32);

impl OwnerId {
    const GLOBAL: Self = Self(GLOBAL_OWNER_BITS);

    fn type_owner(id: TypeId) -> Self {
        assert_ne!(id.0, GLOBAL_OWNER_BITS, "TypeId collides with global owner");
        Self(id.0)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct CallableId {
    owner: OwnerId,
    name: CallableNameId,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PropertyId {
    owner: OwnerId,
    name: PropertyNameId,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LegacyTypeId(u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LegacyCallableId(u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LegacyPropertyId(u32);

trait InternToken: Copy + Debug + Eq + Ord {
    fn from_index(index: usize) -> Self;
}

trait CandidateIdentity: Copy + Debug + Eq + Hash + Ord {
    type Owner: Copy + Debug + Ord;
    type Token: InternToken + Hash;

    fn compose(owner: Self::Owner, token: Self::Token) -> Self;
    fn owner(self) -> Self::Owner;
    fn checksum(self) -> u64;
}

trait LegacyIdentity: Copy + Debug + Eq + Ord {
    fn from_entity(entity: u32) -> Self;
    fn entity(self) -> u32;
}

macro_rules! intern_token {
    ($token:ident) => {
        impl InternToken for $token {
            fn from_index(index: usize) -> Self {
                let value = u32::try_from(index).expect("experimental name token overflowed u32");
                assert_ne!(
                    value, GLOBAL_OWNER_BITS,
                    "name token reached reserved owner value"
                );
                Self(value)
            }
        }
    };
}

intern_token!(TypeId);
intern_token!(CallableNameId);
intern_token!(PropertyNameId);

macro_rules! legacy_identity {
    ($id:ident) => {
        impl LegacyIdentity for $id {
            fn from_entity(entity: u32) -> Self {
                Self(entity)
            }

            fn entity(self) -> u32 {
                self.0
            }
        }
    };
}

legacy_identity!(LegacyTypeId);
legacy_identity!(LegacyCallableId);
legacy_identity!(LegacyPropertyId);

impl CandidateIdentity for TypeId {
    type Owner = ();
    type Token = TypeId;

    fn compose((): (), token: TypeId) -> Self {
        token
    }

    fn owner(self) {}

    fn checksum(self) -> u64 {
        u64::from(self.0) + 1
    }
}

impl CandidateIdentity for CallableId {
    type Owner = OwnerId;
    type Token = CallableNameId;

    fn compose(owner: OwnerId, name: CallableNameId) -> Self {
        Self { owner, name }
    }

    fn owner(self) -> OwnerId {
        self.owner
    }

    fn checksum(self) -> u64 {
        (u64::from(self.owner.0) << 32 | u64::from(self.name.0)).wrapping_add(1)
    }
}

impl CandidateIdentity for PropertyId {
    type Owner = OwnerId;
    type Token = PropertyNameId;

    fn compose(owner: OwnerId, name: PropertyNameId) -> Self {
        Self { owner, name }
    }

    fn owner(self) -> OwnerId {
        self.owner
    }

    fn checksum(self) -> u64 {
        (u64::from(self.owner.0) << 32 | u64::from(self.name.0)).wrapping_add(1)
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceTypeRow {
    primary: StringId,
    alias: Option<StringId>,
}

#[derive(Debug, Clone, Copy)]
struct SourceMemberRow {
    owner_source: Option<usize>,
    primary: StringId,
    alias: Option<StringId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CanonicalRow<Scope> {
    scope: Scope,
    primary: StringId,
    alias: Option<StringId>,
    entity: u32,
}

struct CanonicalFamily<Scope> {
    rows: Vec<CanonicalRow<Scope>>,
    source_rows: usize,
    duplicate_primaries: usize,
    supplied_aliases: usize,
    redundant_aliases: usize,
}

impl<Scope> CanonicalFamily<Scope> {
    fn retained_aliases(&self) -> usize {
        self.supplied_aliases - self.redundant_aliases
    }
}

fn canonicalize_types(source: &[SourceTypeRow]) -> (CanonicalFamily<()>, Vec<u32>) {
    let mut canonical_by_primary = HashMap::<StringId, u32>::new();
    let mut rows = Vec::with_capacity(source.len());
    let mut source_to_type = Vec::with_capacity(source.len());
    let mut duplicate_primaries = 0;
    let mut supplied_aliases = 0;
    let mut redundant_aliases = 0;

    for row in source {
        // TEMPORARY: the HBK formation/extension composition owner must make
        // primaries unique before a production identity cutover. Both compared
        // layouts receive the same stable first-row projection.
        if let Some(entity) = canonical_by_primary.get(&row.primary).copied() {
            duplicate_primaries += 1;
            source_to_type.push(entity);
            continue;
        }
        let entity = u32::try_from(rows.len()).expect("canonical type count overflowed u32");
        canonical_by_primary.insert(row.primary, entity);
        source_to_type.push(entity);
        supplied_aliases += usize::from(row.alias.is_some());
        redundant_aliases += usize::from(row.alias == Some(row.primary));
        rows.push(CanonicalRow {
            scope: (),
            primary: row.primary,
            alias: row.alias,
            entity,
        });
    }

    (
        CanonicalFamily {
            rows,
            source_rows: source.len(),
            duplicate_primaries,
            supplied_aliases,
            redundant_aliases,
        },
        source_to_type,
    )
}

fn canonicalize_members(
    source: &[SourceMemberRow],
    source_to_type: &[u32],
) -> CanonicalFamily<Option<u32>> {
    let mut canonical_by_primary = HashMap::<(Option<u32>, StringId), u32>::new();
    let mut rows = Vec::with_capacity(source.len());
    let mut duplicate_primaries = 0;
    let mut supplied_aliases = 0;
    let mut redundant_aliases = 0;

    for row in source {
        let scope = row.owner_source.map(|owner| {
            source_to_type
                .get(owner)
                .copied()
                .expect("snapshot member owner must reference a projected type")
        });
        // TEMPORARY: uniqueness is scoped by the retained type (or the global
        // context), not by the source ordinal that happened to declare it.
        if canonical_by_primary.contains_key(&(scope, row.primary)) {
            duplicate_primaries += 1;
            continue;
        }
        let entity = u32::try_from(rows.len()).expect("canonical member count overflowed u32");
        canonical_by_primary.insert((scope, row.primary), entity);
        supplied_aliases += usize::from(row.alias.is_some());
        redundant_aliases += usize::from(row.alias == Some(row.primary));
        rows.push(CanonicalRow {
            scope,
            primary: row.primary,
            alias: row.alias,
            entity,
        });
    }

    CanonicalFamily {
        rows,
        source_rows: source.len(),
        duplicate_primaries,
        supplied_aliases,
        redundant_aliases,
    }
}

fn merged_name_lookup_from_rows<Scope, Id>(
    rows: &[CanonicalRow<Scope>],
    id: impl Fn(u32) -> Id,
) -> Vec<OwnerNameLookup<Scope, Id>>
where
    Scope: Copy + Ord,
    Id: Copy + Ord,
{
    let mut entries = Vec::with_capacity(
        rows.len()
            + rows
                .iter()
                .filter(|row| row.alias != Some(row.primary))
                .count(),
    );
    for row in rows {
        let id = id(row.entity);
        entries.push(OwnerNameLookup {
            owner: row.scope,
            key: row.primary,
            value: id,
        });
        if let Some(alias) = row.alias.filter(|alias| *alias != row.primary) {
            entries.push(OwnerNameLookup {
                owner: row.scope,
                key: alias,
                value: id,
            });
        }
    }
    entries.sort_unstable_by_key(|entry| (entry.owner, entry.key, entry.value));
    entries.dedup_by_key(|entry| (entry.owner, entry.key, entry.value));
    entries
}

fn merged_name_lookup<Scope, Id>(
    entries: &[OwnerNameLookup<Scope, Id>],
    scope: Scope,
    key: StringId,
) -> &[OwnerNameLookup<Scope, Id>]
where
    Scope: Copy + Ord,
    Id: Copy + Ord,
{
    let range = matching_range(entries, |entry| (entry.owner, entry.key).cmp(&(scope, key)));
    &entries[range]
}

/// The one primary-first/alias-fallback mechanism used by all three candidate
/// families. State and name tokens remain family-local.
struct PrimaryAliasLookup<Id> {
    primaries: Vec<NameLookup<Id>>,
    aliases: Vec<NameLookup<Id>>,
}

impl<Id> PrimaryAliasLookup<Id>
where
    Id: CandidateIdentity,
{
    fn from_rows(rows: &[CanonicalRow<Id::Owner>]) -> Self {
        let mut tokens = HashMap::<StringId, Id::Token>::new();
        let mut primaries = Vec::with_capacity(rows.len());
        let mut aliases = Vec::with_capacity(
            rows.iter()
                .filter(|row| row.alias != Some(row.primary))
                .count(),
        );
        for row in rows {
            let next_index = tokens.len();
            let token = *tokens
                .entry(row.primary)
                .or_insert_with(|| <Id::Token as InternToken>::from_index(next_index));
            let id = Id::compose(row.scope, token);
            primaries.push(NameLookup {
                key: row.primary,
                value: id,
            });
            if let Some(alias) = row.alias.filter(|alias| *alias != row.primary) {
                aliases.push(NameLookup {
                    key: alias,
                    value: id,
                });
            }
        }
        primaries.sort_unstable_by_key(|entry| (entry.value.owner(), entry.key, entry.value));
        primaries.dedup_by_key(|entry| (entry.value.owner(), entry.key, entry.value));
        aliases.sort_unstable_by_key(|entry| (entry.value.owner(), entry.key, entry.value));
        aliases.dedup_by_key(|entry| (entry.value.owner(), entry.key, entry.value));
        assert_eq!(primaries.len(), rows.len(), "candidate IDs must be unique");
        Self { primaries, aliases }
    }

    fn lookup(&self, owner: Id::Owner, key: StringId) -> CandidateMatch<'_, Id> {
        let primaries = self.matching(&self.primaries, owner, key);
        if let [primary] = primaries {
            return CandidateMatch::Primary(primary.value);
        }
        let aliases = self.matching(&self.aliases, owner, key);
        if aliases.is_empty() {
            CandidateMatch::Missing
        } else {
            CandidateMatch::Aliases(aliases)
        }
    }

    fn matching<'a>(
        &self,
        entries: &'a [NameLookup<Id>],
        owner: Id::Owner,
        key: StringId,
    ) -> &'a [NameLookup<Id>] {
        let range = matching_range(entries, |entry| {
            (entry.value.owner(), entry.key).cmp(&(owner, key))
        });
        &entries[range]
    }

    fn retained_bytes(&self) -> usize {
        self.primaries.capacity() * size_of::<NameLookup<Id>>()
            + self.aliases.capacity() * size_of::<NameLookup<Id>>()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateMatch<'a, Id> {
    Primary(Id),
    Aliases(&'a [NameLookup<Id>]),
    Missing,
}

fn candidate_ids<Id: Copy>(matched: CandidateMatch<'_, Id>) -> Vec<Id> {
    match matched {
        CandidateMatch::Primary(id) => vec![id],
        CandidateMatch::Aliases(entries) => entries.iter().map(|entry| entry.value).collect(),
        CandidateMatch::Missing => Vec::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Query<Scope> {
    scope: Scope,
    key: StringId,
}

struct QuerySets<Scope> {
    primary: Vec<Query<Scope>>,
    alias: Vec<Query<Scope>>,
    missing: Vec<Query<Scope>>,
    collision: Vec<Query<Scope>>,
    owner_isolation: Vec<Query<Scope>>,
}

fn query_sets<Scope>(rows: &[CanonicalRow<Scope>], missing_key: StringId) -> QuerySets<Scope>
where
    Scope: Copy + Eq + Hash + Ord,
{
    let primaries = rows
        .iter()
        .map(|row| Query {
            scope: row.scope,
            key: row.primary,
        })
        .collect::<BTreeSet<_>>();
    let aliases = rows
        .iter()
        .filter_map(|row| {
            row.alias
                .filter(|alias| *alias != row.primary)
                .map(|key| Query {
                    scope: row.scope,
                    key,
                })
        })
        .collect::<BTreeSet<_>>();
    let collision = primaries
        .intersection(&aliases)
        .copied()
        .collect::<Vec<_>>();
    let collision_set = collision.iter().copied().collect::<BTreeSet<_>>();
    let primary = primaries.difference(&aliases).copied().collect();
    let alias = aliases.difference(&primaries).copied().collect();
    let missing = rows
        .iter()
        .map(|row| row.scope)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|scope| Query {
            scope,
            key: missing_key,
        })
        .collect();

    let mut scopes_by_key = BTreeMap::<StringId, BTreeSet<Scope>>::new();
    for query in primaries.iter().chain(aliases.iter()) {
        scopes_by_key
            .entry(query.key)
            .or_default()
            .insert(query.scope);
    }
    let owner_isolation = scopes_by_key
        .into_iter()
        .filter(|(_, scopes)| scopes.len() > 1)
        .flat_map(|(key, scopes)| scopes.into_iter().map(move |scope| Query { scope, key }))
        .filter(|query| !collision_set.contains(query))
        .collect();

    QuerySets {
        primary,
        alias,
        missing,
        collision,
        owner_isolation,
    }
}

fn map_member_scope(
    scope: Option<u32>,
    type_rows: &[CanonicalRow<()>],
    types: &PrimaryAliasLookup<TypeId>,
) -> OwnerId {
    scope.map_or(OwnerId::GLOBAL, |entity| {
        let row = type_rows
            .get(usize::try_from(entity).expect("canonical type owner must fit usize"))
            .expect("member owner must reference a canonical type");
        let CandidateMatch::Primary(id) = types.lookup((), row.primary) else {
            panic!("canonical member owner type must resolve through the type lookup");
        };
        OwnerId::type_owner(id)
    })
}

fn map_rows<From: Copy, To>(
    rows: &[CanonicalRow<From>],
    mut scope: impl FnMut(From) -> To,
) -> Vec<CanonicalRow<To>> {
    rows.iter()
        .map(|row| CanonicalRow {
            scope: scope(row.scope),
            primary: row.primary,
            alias: row.alias,
            entity: row.entity,
        })
        .collect()
}

fn map_queries<From: Copy, To>(
    queries: &[Query<From>],
    mut scope: impl FnMut(From) -> To,
) -> Vec<Query<To>> {
    queries
        .iter()
        .map(|query| Query {
            scope: scope(query.scope),
            key: query.key,
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct AllocationObservation {
    calls: u64,
    allocated_bytes: u64,
    live_bytes_growth: u64,
    peak_live_bytes_growth: u64,
}

#[derive(Debug, Clone, Copy)]
struct ConstructionObservation {
    median_ns: u128,
    retained_bytes: usize,
    allocation: AllocationObservation,
}

fn median(mut samples: Vec<u128>) -> u128 {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn observe_construction<T>(
    mut build: impl FnMut() -> T,
    retained_bytes: impl Fn(&T) -> usize,
) -> (T, ConstructionObservation) {
    for _ in 0..WARMUP_SAMPLES {
        black_box(build());
    }
    let mut elapsed = Vec::with_capacity(MEASURED_SAMPLES);
    let mut allocations = Vec::with_capacity(MEASURED_SAMPLES);
    let mut retained = 0;
    for _ in 0..MEASURED_SAMPLES {
        let before = experiment_allocation_snapshot();
        let started = Instant::now();
        let value = build();
        elapsed.push(started.elapsed().as_nanos());
        let after = experiment_allocation_snapshot();
        retained = retained_bytes(&value);
        black_box(retained);
        allocations.push(after.delta_since(before));
        drop(value);
    }
    let value = build();
    let allocation = AllocationObservation {
        calls: median(
            allocations
                .iter()
                .map(|sample| u128::from(sample.allocation_calls))
                .collect(),
        ) as u64,
        allocated_bytes: median(
            allocations
                .iter()
                .map(|sample| u128::from(sample.allocated_bytes))
                .collect(),
        ) as u64,
        live_bytes_growth: median(
            allocations
                .iter()
                .map(|sample| {
                    u128::from(
                        sample
                            .live_bytes_after
                            .saturating_sub(sample.live_bytes_before),
                    )
                })
                .collect(),
        ) as u64,
        peak_live_bytes_growth: allocations
            .iter()
            .map(|sample| sample.peak_live_bytes_growth)
            .max()
            .unwrap_or_default(),
    };
    (
        value,
        ConstructionObservation {
            median_ns: median(elapsed),
            retained_bytes: retained,
            allocation,
        },
    )
}

#[derive(Debug, Clone, Copy)]
struct LookupObservation {
    query_count: usize,
    median_ns_per_query: u128,
    checksum: u64,
}

fn observe_lookup<Scope: Copy>(
    queries: &[Query<Scope>],
    mut lookup: impl FnMut(Scope, StringId) -> u64,
) -> LookupObservation {
    if queries.is_empty() {
        return LookupObservation {
            query_count: 0,
            median_ns_per_query: 0,
            checksum: 0,
        };
    }
    let run = |lookup: &mut dyn FnMut(Scope, StringId) -> u64| {
        let mut checksum = 0_u64;
        for _ in 0..LOOKUP_PASSES {
            for query in queries {
                checksum = checksum.wrapping_add(black_box(lookup(query.scope, query.key)));
            }
        }
        black_box(checksum)
    };
    for _ in 0..WARMUP_SAMPLES {
        run(&mut lookup);
    }
    let mut samples = Vec::with_capacity(MEASURED_SAMPLES);
    let mut checksum = 0;
    for _ in 0..MEASURED_SAMPLES {
        let started = Instant::now();
        checksum = run(&mut lookup);
        samples.push(started.elapsed().as_nanos());
    }
    LookupObservation {
        query_count: queries.len(),
        median_ns_per_query: median(samples) / (queries.len() * LOOKUP_PASSES) as u128,
        checksum,
    }
}

fn checksum_legacy<Scope, Id: LegacyIdentity>(entries: &[OwnerNameLookup<Scope, Id>]) -> u64 {
    entries.iter().fold(0xcbf2_9ce4_8422_2325, |hash, entry| {
        hash.wrapping_mul(0x0000_0100_0000_01b3)
            .wrapping_add(u64::from(entry.value.entity()) + 1)
    })
}

fn checksum_candidate<Id>(matched: CandidateMatch<'_, Id>) -> u64
where
    Id: CandidateIdentity,
{
    match matched {
        CandidateMatch::Primary(id) => id.checksum(),
        CandidateMatch::Aliases(entries) => {
            entries.iter().fold(0xcbf2_9ce4_8422_2325, |hash, entry| {
                hash.wrapping_mul(0x0000_0100_0000_01b3)
                    .wrapping_add(entry.value.checksum())
            })
        }
        CandidateMatch::Missing => 0,
    }
}

fn differential_entity_map<Id>(
    new: &PrimaryAliasLookup<Id>,
    rows: &[CanonicalRow<Id::Owner>],
) -> HashMap<Id, u32>
where
    Id: CandidateIdentity,
{
    rows.iter()
        .map(|row| {
            let CandidateMatch::Primary(id) = new.lookup(row.scope, row.primary) else {
                panic!("candidate primary row must resolve during differential setup");
            };
            (id, row.entity)
        })
        .collect()
}

fn assert_semantic_equivalence<OldScope, NewScope, OldId, NewId>(
    old: &[OwnerNameLookup<OldScope, OldId>],
    new: &PrimaryAliasLookup<NewId>,
    old_queries: &[Query<OldScope>],
    new_queries: &[Query<NewScope>],
    new_entity: &HashMap<NewId, u32>,
) where
    OldScope: Copy + Debug + Ord,
    NewScope: Copy + Debug + Ord,
    OldId: LegacyIdentity,
    NewId: CandidateIdentity<Owner = NewScope>,
{
    assert_eq!(old_queries.len(), new_queries.len());
    for (old_query, new_query) in old_queries.iter().zip(new_queries) {
        let mut expected = merged_name_lookup(old, old_query.scope, old_query.key)
            .iter()
            .map(|entry| entry.value.entity())
            .collect::<Vec<_>>();
        let mut actual = candidate_ids(new.lookup(new_query.scope, new_query.key))
            .into_iter()
            .map(|id| new_entity[&id])
            .collect::<Vec<_>>();
        expected.sort_unstable();
        actual.sort_unstable();
        assert_eq!(expected, actual, "non-colliding query changed entity set");
    }
}

fn print_construction(family: &str, variant: &str, observed: ConstructionObservation) {
    println!(
        "construction family={family} variant={variant} median_ns={} retained_bytes={} allocation_calls={} allocated_bytes={} live_bytes_growth={} peak_live_bytes_growth={}",
        observed.median_ns,
        observed.retained_bytes,
        observed.allocation.calls,
        observed.allocation.allocated_bytes,
        observed.allocation.live_bytes_growth,
        observed.allocation.peak_live_bytes_growth,
    );
}

fn print_lookup(family: &str, class: &str, variant: &str, observed: LookupObservation) {
    println!(
        "lookup family={family} class={class} variant={variant} queries={} median_ns_per_query={} checksum={}",
        observed.query_count, observed.median_ns_per_query, observed.checksum,
    );
}

fn run_family<CommonScope, OldScope, NewScope, OldId, NewId>(
    family: &str,
    rows: &[CanonicalRow<CommonScope>],
    missing_key: StringId,
    old_scope: impl Fn(CommonScope) -> OldScope + Copy,
    new_scope: impl Fn(CommonScope) -> NewScope + Copy,
) -> PrimaryAliasLookup<NewId>
where
    CommonScope: Copy + Eq + Hash + Ord,
    OldScope: Copy + Debug + Ord,
    NewScope: Copy + Debug + Ord,
    OldId: LegacyIdentity,
    NewId: CandidateIdentity<Owner = NewScope>,
{
    let query_sets = query_sets(rows, missing_key);
    let old_rows = map_rows(rows, old_scope);
    let new_rows = map_rows(rows, new_scope);
    let (old, old_build) = observe_construction(
        || merged_name_lookup_from_rows(&old_rows, OldId::from_entity),
        |entries| entries.capacity() * size_of::<OwnerNameLookup<OldScope, OldId>>(),
    );
    let (new_state, new_build) = observe_construction(
        || PrimaryAliasLookup::<NewId>::from_rows(&new_rows),
        PrimaryAliasLookup::retained_bytes,
    );
    let new = new_state;
    print_construction(family, "dense_merged_prepared_string_id", old_build);
    print_construction(family, "direct_composite_primary_alias", new_build);

    let classes = [
        ("primary", &query_sets.primary),
        ("alias", &query_sets.alias),
        ("missing", &query_sets.missing),
        ("collision", &query_sets.collision),
        ("owner_isolation", &query_sets.owner_isolation),
    ];
    {
        let new_entity = differential_entity_map::<NewId>(&new, &new_rows);
        for (class, common_queries) in classes {
            let old_queries = map_queries(common_queries, old_scope);
            let new_queries = map_queries(common_queries, new_scope);
            if class != "collision" {
                assert_semantic_equivalence(&old, &new, &old_queries, &new_queries, &new_entity);
            } else {
                for (old_query, new_query) in old_queries.iter().zip(&new_queries) {
                    let old_entities = merged_name_lookup(&old, old_query.scope, old_query.key)
                        .iter()
                        .map(|entry| entry.value.entity())
                        .collect::<Vec<_>>();
                    let CandidateMatch::Primary(id) = new.lookup(new_query.scope, new_query.key)
                    else {
                        panic!("candidate collision lookup must prefer primary");
                    };
                    assert!(
                        old_entities.len() > 1,
                        "merged collision must expose aliases"
                    );
                    assert!(old_entities.contains(&new_entity[&id]));
                }
            }
        }
    }
    for (class, common_queries) in classes {
        let old_queries = map_queries(common_queries, old_scope);
        let new_queries = map_queries(common_queries, new_scope);
        let old_observed = observe_lookup(&old_queries, |scope, key| {
            checksum_legacy(merged_name_lookup(&old, scope, key))
        });
        let new_observed = observe_lookup(&new_queries, |scope, key| {
            checksum_candidate::<NewId>(new.lookup(scope, key))
        });
        print_lookup(
            family,
            class,
            "dense_merged_prepared_string_id",
            old_observed,
        );
        print_lookup(
            family,
            class,
            "direct_composite_primary_alias",
            new_observed,
        );
    }
    new
}

fn fixture_key(builder: &mut SnapshotBuilder, value: &str) -> StringId {
    builder.intern(&normalize_lookup_key(value))
}

#[test]
fn composite_ids_reuse_family_name_tokens_but_not_owner_identity() {
    assert_eq!(size_of::<TypeId>(), 4);
    assert_eq!(size_of::<OwnerId>(), 4);
    assert_eq!(size_of::<CallableId>(), 8);
    assert_eq!(size_of::<PropertyId>(), 8);

    let mut builder = SnapshotBuilder::default();
    let type_a = fixture_key(&mut builder, "Array");
    let type_b = fixture_key(&mut builder, "ValueTable");
    let same_name = fixture_key(&mut builder, "Добавить");
    let (types, _) = canonicalize_types(&[
        SourceTypeRow {
            primary: type_a,
            alias: None,
        },
        SourceTypeRow {
            primary: type_b,
            alias: None,
        },
    ]);
    let type_lookup = PrimaryAliasLookup::<TypeId>::from_rows(&types.rows);
    let CandidateMatch::Primary(type_a_id) = type_lookup.lookup((), type_a) else {
        panic!("type A must resolve");
    };
    let CandidateMatch::Primary(type_b_id) = type_lookup.lookup((), type_b) else {
        panic!("type B must resolve");
    };
    let members = vec![
        CanonicalRow {
            scope: OwnerId::type_owner(type_a_id),
            primary: same_name,
            alias: None,
            entity: 0,
        },
        CanonicalRow {
            scope: OwnerId::type_owner(type_b_id),
            primary: same_name,
            alias: None,
            entity: 1,
        },
        CanonicalRow {
            scope: OwnerId::GLOBAL,
            primary: same_name,
            alias: None,
            entity: 2,
        },
    ];
    let callables = PrimaryAliasLookup::<CallableId>::from_rows(&members);
    let properties = PrimaryAliasLookup::<PropertyId>::from_rows(&members);
    let callable_ids = candidate_ids(callables.lookup(OwnerId::type_owner(type_a_id), same_name))
        .into_iter()
        .chain(candidate_ids(
            callables.lookup(OwnerId::type_owner(type_b_id), same_name),
        ))
        .chain(candidate_ids(callables.lookup(OwnerId::GLOBAL, same_name)))
        .collect::<Vec<_>>();
    let property_ids = candidate_ids(properties.lookup(OwnerId::type_owner(type_a_id), same_name))
        .into_iter()
        .chain(candidate_ids(
            properties.lookup(OwnerId::type_owner(type_b_id), same_name),
        ))
        .chain(candidate_ids(properties.lookup(OwnerId::GLOBAL, same_name)))
        .collect::<Vec<_>>();

    assert_eq!(callable_ids[0].name, callable_ids[1].name);
    assert_eq!(callable_ids[0].name, callable_ids[2].name);
    assert_ne!(callable_ids[0], callable_ids[1]);
    assert_ne!(callable_ids[0], callable_ids[2]);
    assert_eq!(property_ids[0].name, property_ids[1].name);
    assert_eq!(property_ids[0].name, property_ids[2].name);
    assert_ne!(property_ids[0], property_ids[1]);
    assert_ne!(property_ids[0], property_ids[2]);
    assert_eq!(
        candidate_ids(callables.lookup(OwnerId::GLOBAL, same_name)),
        vec![callable_ids[2]]
    );
    assert_eq!(
        candidate_ids(properties.lookup(OwnerId::type_owner(type_a_id), same_name)),
        vec![property_ids[0]]
    );
}

#[test]
fn primary_precedes_alias_and_alias_ambiguity_is_preserved() {
    let mut builder = SnapshotBuilder::default();
    let alpha = fixture_key(&mut builder, "Alpha");
    let beta = fixture_key(&mut builder, "Beta");
    let gamma = fixture_key(&mut builder, "Gamma");
    let delta = fixture_key(&mut builder, "Delta");
    let shared = fixture_key(&mut builder, "Shared");
    let missing = fixture_key(&mut builder, "Missing");
    let rows = vec![
        CanonicalRow {
            scope: (),
            primary: alpha,
            alias: Some(shared),
            entity: 0,
        },
        CanonicalRow {
            scope: (),
            primary: beta,
            alias: Some(shared),
            entity: 1,
        },
        CanonicalRow {
            scope: (),
            primary: gamma,
            alias: Some(alpha),
            entity: 2,
        },
        CanonicalRow {
            scope: (),
            primary: delta,
            alias: None,
            entity: 3,
        },
    ];
    let old = merged_name_lookup_from_rows(&rows, LegacyTypeId::from_entity);
    let new = PrimaryAliasLookup::<TypeId>::from_rows(&rows);
    let new_entity = differential_entity_map::<TypeId>(&new, &rows);
    let alpha_id = *new_entity
        .iter()
        .find(|(_, entity)| **entity == 0)
        .unwrap()
        .0;
    let delta_id = *new_entity
        .iter()
        .find(|(_, entity)| **entity == 3)
        .unwrap()
        .0;

    assert_eq!(candidate_ids(new.lookup((), alpha)), vec![alpha_id]);
    assert_eq!(merged_name_lookup(&old, (), alpha).len(), 2);
    assert_eq!(candidate_ids(new.lookup((), shared)).len(), 2);
    assert_eq!(candidate_ids(new.lookup((), delta)), vec![delta_id]);
    assert!(new.primaries.iter().all(|entry| entry.key != shared));
    assert!(matches!(new.lookup((), missing), CandidateMatch::Missing));

    let queries = query_sets(&rows, missing);
    assert_semantic_equivalence(&old, &new, &queries.alias, &queries.alias, &new_entity);
    assert_semantic_equivalence(&old, &new, &queries.missing, &queries.missing, &new_entity);
}

#[test]
fn temporary_duplicate_projection_reuses_canonical_owner() {
    let mut builder = SnapshotBuilder::default();
    let duplicate_type = fixture_key(&mut builder, "Duplicate type");
    let member = fixture_key(&mut builder, "Member");
    let first_alias = fixture_key(&mut builder, "First alias");
    let dropped_alias = fixture_key(&mut builder, "Dropped alias");
    let (types, source_to_type) = canonicalize_types(&[
        SourceTypeRow {
            primary: duplicate_type,
            alias: Some(first_alias),
        },
        SourceTypeRow {
            primary: duplicate_type,
            alias: Some(dropped_alias),
        },
    ]);
    assert_eq!(types.rows.len(), 1);
    assert_eq!(types.duplicate_primaries, 1);
    assert_eq!(source_to_type, vec![0, 0]);

    let members = canonicalize_members(
        &[
            SourceMemberRow {
                owner_source: Some(0),
                primary: member,
                alias: None,
            },
            SourceMemberRow {
                owner_source: Some(1),
                primary: member,
                alias: Some(dropped_alias),
            },
        ],
        &source_to_type,
    );
    assert_eq!(members.rows.len(), 1);
    assert_eq!(members.duplicate_primaries, 1);
    assert_eq!(members.rows[0].scope, Some(0));
}

#[test]
fn optimized_candidate_does_not_retain_removed_lookup_mirrors() {
    let source = include_str!("primary_alias_lookup_experiment.rs");
    let prohibited = [
        ["struct Key", "Id"].concat(),
        ["struct Key", "Pool"].concat(),
        ["struct Lookup", "Entry"].concat(),
        ["struct PrimaryName", "Interner"].concat(),
        ["struct MergedName", "Lookup"].concat(),
        ["primary", "_ids"].concat(),
        ["entity", "_ids"].concat(),
        ["matching", "_entries"].concat(),
        ["LegacyOwner", "Id"].concat(),
        ["String", "Id("].concat(),
    ];
    for pattern in prohibited {
        assert!(
            !source.contains(&pattern),
            "removed benchmark mirror reappeared: {pattern}"
        );
    }

    let lookup = source
        .split("struct PrimaryAliasLookup")
        .nth(1)
        .and_then(|tail| tail.split("}\n\nimpl").next())
        .expect("candidate lookup declaration must remain inspectable");
    assert_eq!(lookup.matches("Vec<NameLookup<Id>>").count(), 2);
    assert!(!lookup.contains("HashMap"));

    let corpus = source
        .rsplit("struct ProjectedCorpus")
        .next()
        .and_then(|tail| tail.split("}\n\nfn").next())
        .expect("projected corpus declaration must remain inspectable");
    assert!(!corpus.contains("HashMap"));
    assert!(!corpus.contains("by_text"));
    assert!(!corpus.contains("by_string"));
}

struct ProjectedCorpus {
    missing_key: StringId,
    prepared_key_count: usize,
    prepared_key_bytes: usize,
    types: CanonicalFamily<()>,
    callables: CanonicalFamily<Option<u32>>,
    properties: CanonicalFamily<Option<u32>>,
}

fn snapshot_prepared_keys(snapshot: &HbkFactSnapshot) -> HashMap<&str, StringId> {
    fn insert<'a>(
        keys: &mut HashMap<&'a str, StringId>,
        snapshot: &'a HbkFactSnapshot,
        id: StringId,
    ) {
        let value = snapshot.string(id);
        if let Some(previous) = keys.insert(value, id) {
            assert_eq!(
                previous, id,
                "one snapshot string must have one snapshot-owned StringId"
            );
        }
    }

    let mut keys = HashMap::new();
    for entry in &snapshot.platform_type_names {
        insert(&mut keys, snapshot, entry.key);
    }
    for entry in &snapshot.callables_by_owner_name {
        insert(&mut keys, snapshot, entry.key);
    }
    for entry in &snapshot.global_names {
        insert(&mut keys, snapshot, entry.key);
    }
    for entry in &snapshot.members_by_owner_name {
        insert(&mut keys, snapshot, entry.key);
    }
    for entry in &snapshot.enum_names {
        insert(&mut keys, snapshot, entry.key);
    }
    for entry in &snapshot.enum_values_by_enum_name {
        insert(&mut keys, snapshot, entry.key);
    }
    keys
}

fn project_key(
    keys: &HashMap<&str, StringId>,
    handle: HbkFactReadHandle<'_>,
    source: StringId,
) -> StringId {
    let normalized = normalize_lookup_key(handle.string(source));
    keys.get(normalized.as_str()).copied().unwrap_or_else(|| {
        panic!("normalized projected name is absent from snapshot-owned name indexes: {normalized}")
    })
}

fn project_name(
    keys: &HashMap<&str, StringId>,
    handle: HbkFactReadHandle<'_>,
    name: HbkNameView<'_>,
) -> (StringId, Option<StringId>) {
    let primary = project_key(keys, handle, name.primary());
    let alias = name.alias().map(|alias| project_key(keys, handle, alias));
    (primary, alias)
}

fn missing_prepared_key(
    snapshot: &HbkFactSnapshot,
    types: &CanonicalFamily<()>,
    callables: &CanonicalFamily<Option<u32>>,
    properties: &CanonicalFamily<Option<u32>>,
) -> StringId {
    let used = types
        .rows
        .iter()
        .flat_map(|row| std::iter::once(row.primary).chain(row.alias))
        .chain(
            callables
                .rows
                .iter()
                .flat_map(|row| std::iter::once(row.primary).chain(row.alias)),
        )
        .chain(
            properties
                .rows
                .iter()
                .flat_map(|row| std::iter::once(row.primary).chain(row.alias)),
        )
        .collect::<BTreeSet<_>>();

    snapshot
        .fact_ids
        .iter()
        .map(|entry| entry.key)
        .chain(snapshot.query_table_names.iter().map(|entry| entry.key))
        .chain(snapshot.language_names.iter().map(|entry| entry.key))
        .find(|candidate| !used.contains(candidate))
        .expect("snapshot must contain an interned string absent from experiment name indexes")
}

fn project_corpus(snapshot: &HbkFactSnapshot) -> ProjectedCorpus {
    let handle = snapshot.worker_handle();
    let counts = snapshot.counts();
    let keys = snapshot_prepared_keys(snapshot);
    let prepared_key_count = keys.len();
    let prepared_key_bytes = keys.keys().map(|key| key.len()).sum();
    let mut source_types = Vec::with_capacity(counts.platform_types + counts.enums);
    for ordinal in 0..counts.platform_types {
        let view = handle.platform_type(HbkPlatformTypeId(
            u32::try_from(ordinal).expect("platform type count overflowed u32"),
        ));
        let (primary, alias) = project_name(&keys, handle, view.name());
        source_types.push(SourceTypeRow { primary, alias });
    }
    for ordinal in 0..counts.enums {
        let view = handle.enum_fact(HbkEnumId(
            u32::try_from(ordinal).expect("enum count overflowed u32"),
        ));
        let (primary, alias) = project_name(&keys, handle, view.name());
        source_types.push(SourceTypeRow { primary, alias });
    }
    let (types, source_to_type) = canonicalize_types(&source_types);

    let mut source_callables = Vec::with_capacity(counts.callables);
    for ordinal in 0..counts.callables {
        let view = handle.callable(HbkCallableId(
            u32::try_from(ordinal).expect("callable count overflowed u32"),
        ));
        let include = matches!(
            view.kind(),
            HbkCallableKind::Constructor | HbkCallableKind::GlobalMethod | HbkCallableKind::Method
        ) || matches!(view.kind(), HbkCallableKind::Event) && view.owner().is_some();
        if !include {
            continue;
        }
        let (primary, alias) = project_name(&keys, handle, view.name());
        source_callables.push(SourceMemberRow {
            owner_source: view.owner().map(|owner| owner.0 as usize),
            primary,
            alias,
        });
    }
    let callables = canonicalize_members(&source_callables, &source_to_type);

    let mut source_properties = Vec::with_capacity(counts.type_members + counts.enum_values);
    for ordinal in 0..counts.type_members {
        let view = handle.type_member(HbkTypeMemberId(
            u32::try_from(ordinal).expect("type member count overflowed u32"),
        ));
        if view.kind() != HbkTypeMemberKind::Property {
            continue;
        }
        let (primary, alias) = project_name(&keys, handle, view.name());
        source_properties.push(SourceMemberRow {
            owner_source: Some(view.owner().0 as usize),
            primary,
            alias,
        });
    }
    for id in handle.global_fact_ids() {
        let view = handle.global_fact(id);
        if view.kind() != HbkGlobalFactKind::Property {
            continue;
        }
        let (primary, alias) = project_name(&keys, handle, view.name());
        source_properties.push(SourceMemberRow {
            owner_source: None,
            primary,
            alias,
        });
    }
    for ordinal in 0..counts.enum_values {
        let view = handle.enum_value(HbkEnumValueId(
            u32::try_from(ordinal).expect("enum value count overflowed u32"),
        ));
        let (primary, alias) = project_name(&keys, handle, view.name());
        source_properties.push(SourceMemberRow {
            owner_source: Some(counts.platform_types + view.owner().0 as usize),
            primary,
            alias,
        });
    }
    let properties = canonicalize_members(&source_properties, &source_to_type);
    let missing_key = missing_prepared_key(snapshot, &types, &callables, &properties);

    ProjectedCorpus {
        missing_key,
        prepared_key_count,
        prepared_key_bytes,
        types,
        callables,
        properties,
    }
}

fn validate_frozen_metadata(metadata: &StoredIndexMetadata) -> Result<(), String> {
    if metadata.locale != FROZEN_LOCALE {
        return Err(format!(
            "expected locale {FROZEN_LOCALE}, got {}",
            metadata.locale
        ));
    }
    if metadata.source_extraction_schema_version != FROZEN_EXTRACTION_SCHEMA {
        return Err(format!(
            "expected extraction schema {FROZEN_EXTRACTION_SCHEMA}, got {}",
            metadata.source_extraction_schema_version
        ));
    }
    if !metadata.source_hbk.contains(FROZEN_PLATFORM_VERSION) {
        return Err(format!(
            "expected source_hbk to identify {FROZEN_PLATFORM_VERSION}, got {}",
            metadata.source_hbk
        ));
    }
    Ok(())
}

fn print_family_corpus<Scope>(family: &str, corpus: &CanonicalFamily<Scope>) {
    println!(
        "family={family} source_rows={} canonical_rows={} duplicate_primaries={} supplied_aliases={} redundant_aliases={} retained_aliases={}",
        corpus.source_rows,
        corpus.rows.len(),
        corpus.duplicate_primaries,
        corpus.supplied_aliases,
        corpus.redundant_aliases,
        corpus.retained_aliases(),
    );
}

#[test]
#[ignore = "requires V8_CONTEXT_HBK_PRIMARY_ALIAS_INDEX with the frozen 8.3.27 provider index"]
fn primary_alias_lookup_real_corpus() {
    primary_alias_lookup_real_corpus_inner();
}

#[cfg(not(feature = "snapshot-experiment-alloc"))]
fn primary_alias_lookup_real_corpus_inner() {
    panic!("real corpus measurement requires snapshot-experiment-alloc");
}

#[cfg(feature = "snapshot-experiment-alloc")]
fn primary_alias_lookup_real_corpus_inner() {
    let path = std::env::var_os(CORPUS_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{CORPUS_ENV} must point to the frozen provider index"));
    let index = SearchIndex::open_read_only(&path).expect("frozen provider index must open");
    let metadata = index
        .metadata()
        .expect("frozen provider metadata must read");
    validate_frozen_metadata(&metadata).unwrap_or_else(|error| panic!("{error}"));
    let snapshot = HbkFactSnapshot::build_from_provider_index(&index)
        .expect("frozen provider snapshot must materialize");
    let corpus = project_corpus(&snapshot);

    println!(
        "corpus platform_version={} locale={} extraction_schema={} source_hbk={} snapshot_strings={} prepared_key_ids={} prepared_key_bytes={}",
        FROZEN_PLATFORM_VERSION,
        metadata.locale,
        metadata.source_extraction_schema_version,
        metadata.source_hbk,
        snapshot.counts().strings,
        corpus.prepared_key_count,
        corpus.prepared_key_bytes,
    );
    print_family_corpus("type", &corpus.types);
    print_family_corpus("callable", &corpus.callables);
    print_family_corpus("property", &corpus.properties);

    let types = run_family::<(), (), (), LegacyTypeId, TypeId>(
        "type",
        &corpus.types.rows,
        corpus.missing_key,
        |()| (),
        |()| (),
    );
    let member_scope = |scope| map_member_scope(scope, &corpus.types.rows, &types);
    run_family::<Option<u32>, OwnerId, OwnerId, LegacyCallableId, CallableId>(
        "callable",
        &corpus.callables.rows,
        corpus.missing_key,
        member_scope,
        member_scope,
    );
    run_family::<Option<u32>, OwnerId, OwnerId, LegacyPropertyId, PropertyId>(
        "property",
        &corpus.properties.rows,
        corpus.missing_key,
        member_scope,
        member_scope,
    );
}
