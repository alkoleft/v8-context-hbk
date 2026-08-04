use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::hint::black_box;
use std::mem::size_of;
use std::path::PathBuf;
use std::time::Instant;

use super::*;

const CORPUS_ENV: &str = "V8_CONTEXT_HBK_PRIMARY_ALIAS_INDEX";
const FROZEN_PLATFORM_VERSION: &str = "8.3.27.1859";
const FROZEN_LOCALE: &str = "ru";
const FROZEN_EXTRACTION_SCHEMA: u32 = 11;
const GLOBAL_OWNER_BITS: u32 = u32::MAX;
const WARMUP_SAMPLES: usize = 2;
const MEASURED_SAMPLES: usize = 9;
const LOOKUP_PASSES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct KeyId(u32);

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

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LegacyOwnerId(u32);

impl LegacyOwnerId {
    const GLOBAL: Self = Self(GLOBAL_OWNER_BITS);

    fn type_owner(id: LegacyTypeId) -> Self {
        assert_ne!(
            id.0, GLOBAL_OWNER_BITS,
            "legacy type ID collides with global owner"
        );
        Self(id.0)
    }
}

trait InternToken: Copy + Debug + Eq + Ord {
    fn from_index(index: usize) -> Self;
}

trait CandidateIdentity<Owner, Token>: Copy + Debug + Eq + Hash + Ord {
    fn compose(owner: Owner, token: Token) -> Self;
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

impl CandidateIdentity<(), TypeId> for TypeId {
    fn compose((): (), token: TypeId) -> Self {
        token
    }

    fn checksum(self) -> u64 {
        u64::from(self.0) + 1
    }
}

impl CandidateIdentity<OwnerId, CallableNameId> for CallableId {
    fn compose(owner: OwnerId, name: CallableNameId) -> Self {
        Self { owner, name }
    }

    fn checksum(self) -> u64 {
        (u64::from(self.owner.0) << 32 | u64::from(self.name.0)).wrapping_add(1)
    }
}

impl CandidateIdentity<OwnerId, PropertyNameId> for PropertyId {
    fn compose(owner: OwnerId, name: PropertyNameId) -> Self {
        Self { owner, name }
    }

    fn checksum(self) -> u64 {
        (u64::from(self.owner.0) << 32 | u64::from(self.name.0)).wrapping_add(1)
    }
}

#[derive(Default)]
struct KeyPool {
    by_text: HashMap<Box<str>, KeyId>,
    retained_key_bytes: usize,
}

impl KeyPool {
    fn intern(&mut self, value: &str) -> KeyId {
        let normalized = normalize_lookup_key(value);
        if let Some(id) = self.by_text.get(normalized.as_str()).copied() {
            return id;
        }
        let id =
            KeyId(u32::try_from(self.by_text.len()).expect("experimental key pool overflowed u32"));
        self.retained_key_bytes += normalized.len();
        self.by_text.insert(normalized.into_boxed_str(), id);
        id
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceTypeRow {
    primary: KeyId,
    alias: Option<KeyId>,
}

#[derive(Debug, Clone, Copy)]
struct SourceMemberRow {
    owner_source: Option<usize>,
    primary: KeyId,
    alias: Option<KeyId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CanonicalRow<Scope> {
    scope: Scope,
    primary: KeyId,
    alias: Option<KeyId>,
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
    let mut canonical_by_primary = HashMap::<KeyId, u32>::new();
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
    let mut canonical_by_primary = HashMap::<(Option<u32>, KeyId), u32>::new();
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct LookupEntry<Scope, Id> {
    scope: Scope,
    key: KeyId,
    id: Id,
}

struct MergedNameLookup<Scope, Id> {
    entries: Vec<LookupEntry<Scope, Id>>,
}

impl<Scope, Id> MergedNameLookup<Scope, Id>
where
    Scope: Copy + Ord,
    Id: Copy + Ord,
{
    fn from_rows(rows: &[CanonicalRow<Scope>], id: impl Fn(u32) -> Id) -> Self {
        let mut entries = Vec::with_capacity(
            rows.len()
                + rows
                    .iter()
                    .filter(|row| row.alias != Some(row.primary))
                    .count(),
        );
        for row in rows {
            let id = id(row.entity);
            entries.push(LookupEntry {
                scope: row.scope,
                key: row.primary,
                id,
            });
            if let Some(alias) = row.alias.filter(|alias| *alias != row.primary) {
                entries.push(LookupEntry {
                    scope: row.scope,
                    key: alias,
                    id,
                });
            }
        }
        entries.sort_unstable();
        entries.dedup();
        Self { entries }
    }

    fn lookup(&self, scope: Scope, key: KeyId) -> &[LookupEntry<Scope, Id>] {
        matching_entries(&self.entries, scope, key)
    }

    fn retained_bytes(&self) -> usize {
        self.entries.capacity() * size_of::<LookupEntry<Scope, Id>>()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PrimaryNameEntry<Token> {
    key: KeyId,
    token: Token,
}

struct PrimaryNameInterner<Token> {
    entries: Vec<PrimaryNameEntry<Token>>,
}

impl<Token: Copy + Ord> PrimaryNameInterner<Token> {
    fn get(&self, key: KeyId) -> Option<Token> {
        self.entries
            .binary_search_by_key(&key, |entry| entry.key)
            .ok()
            .map(|index| self.entries[index].token)
    }

    fn retained_bytes(&self) -> usize {
        self.entries.capacity() * size_of::<PrimaryNameEntry<Token>>()
    }
}

struct PrimaryNameInternerBuilder<Token> {
    by_key: HashMap<KeyId, Token>,
}

impl<Token: InternToken + Hash> PrimaryNameInternerBuilder<Token> {
    fn new() -> Self {
        Self {
            by_key: HashMap::new(),
        }
    }

    fn intern(&mut self, key: KeyId) -> Token {
        if let Some(token) = self.by_key.get(&key).copied() {
            return token;
        }
        let token = Token::from_index(self.by_key.len());
        self.by_key.insert(key, token);
        token
    }

    fn finish(self) -> PrimaryNameInterner<Token> {
        let mut entries = self
            .by_key
            .into_iter()
            .map(|(key, token)| PrimaryNameEntry { key, token })
            .collect::<Vec<_>>();
        entries.sort_unstable();
        PrimaryNameInterner { entries }
    }
}

/// The one primary-first/alias-fallback mechanism used by all three candidate
/// families. State and name tokens remain family-local.
struct PrimaryAliasLookup<Owner, Token, Id> {
    primary_names: PrimaryNameInterner<Token>,
    primary_ids: Vec<Id>,
    aliases: Vec<LookupEntry<Owner, Id>>,
}

impl<Owner, Token, Id> PrimaryAliasLookup<Owner, Token, Id>
where
    Owner: Copy + Ord,
    Token: InternToken + Hash,
    Id: CandidateIdentity<Owner, Token>,
{
    fn from_rows(rows: &[CanonicalRow<Owner>]) -> (Self, Vec<Id>) {
        let mut names = PrimaryNameInternerBuilder::<Token>::new();
        let mut primary_ids = Vec::with_capacity(rows.len());
        let mut aliases = Vec::with_capacity(
            rows.iter()
                .filter(|row| row.alias != Some(row.primary))
                .count(),
        );
        let mut entity_ids = Vec::with_capacity(rows.len());
        for row in rows {
            let id = Id::compose(row.scope, names.intern(row.primary));
            primary_ids.push(id);
            entity_ids.push(id);
            if let Some(alias) = row.alias.filter(|alias| *alias != row.primary) {
                aliases.push(LookupEntry {
                    scope: row.scope,
                    key: alias,
                    id,
                });
            }
        }
        primary_ids.sort_unstable();
        primary_ids.dedup();
        aliases.sort_unstable();
        aliases.dedup();
        assert_eq!(
            primary_ids.len(),
            rows.len(),
            "candidate IDs must be unique"
        );
        (
            Self {
                primary_names: names.finish(),
                primary_ids,
                aliases,
            },
            entity_ids,
        )
    }

    fn lookup(&self, owner: Owner, key: KeyId) -> CandidateMatch<'_, Owner, Id> {
        if let Some(token) = self.primary_names.get(key) {
            let id = Id::compose(owner, token);
            if self.primary_ids.binary_search(&id).is_ok() {
                return CandidateMatch::Primary(id);
            }
        }
        let aliases = matching_entries(&self.aliases, owner, key);
        if aliases.is_empty() {
            CandidateMatch::Missing
        } else {
            CandidateMatch::Aliases(aliases)
        }
    }

    fn retained_bytes(&self) -> usize {
        self.primary_names.retained_bytes()
            + self.primary_ids.capacity() * size_of::<Id>()
            + self.aliases.capacity() * size_of::<LookupEntry<Owner, Id>>()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateMatch<'a, Owner, Id> {
    Primary(Id),
    Aliases(&'a [LookupEntry<Owner, Id>]),
    Missing,
}

fn matching_entries<Scope, Id>(
    entries: &[LookupEntry<Scope, Id>],
    scope: Scope,
    key: KeyId,
) -> &[LookupEntry<Scope, Id>]
where
    Scope: Copy + Ord,
    Id: Copy + Ord,
{
    let start = entries.partition_point(|entry| (entry.scope, entry.key) < (scope, key));
    let end =
        entries[start..].partition_point(|entry| (entry.scope, entry.key) == (scope, key)) + start;
    &entries[start..end]
}

fn candidate_ids<Owner, Id: Copy>(matched: CandidateMatch<'_, Owner, Id>) -> Vec<Id> {
    match matched {
        CandidateMatch::Primary(id) => vec![id],
        CandidateMatch::Aliases(entries) => entries.iter().map(|entry| entry.id).collect(),
        CandidateMatch::Missing => Vec::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Query<Scope> {
    scope: Scope,
    key: KeyId,
}

struct QuerySets<Scope> {
    primary: Vec<Query<Scope>>,
    alias: Vec<Query<Scope>>,
    missing: Vec<Query<Scope>>,
    collision: Vec<Query<Scope>>,
    owner_isolation: Vec<Query<Scope>>,
}

fn query_sets<Scope>(rows: &[CanonicalRow<Scope>], missing_key: KeyId) -> QuerySets<Scope>
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

    let mut scopes_by_key = BTreeMap::<KeyId, BTreeSet<Scope>>::new();
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

fn map_member_scope_to_legacy(scope: Option<u32>) -> LegacyOwnerId {
    scope.map_or(LegacyOwnerId::GLOBAL, |entity| {
        LegacyOwnerId::type_owner(LegacyTypeId(entity))
    })
}

fn map_member_scope_to_candidate(scope: Option<u32>, types: &[TypeId]) -> OwnerId {
    scope.map_or(OwnerId::GLOBAL, |entity| {
        OwnerId::type_owner(types[entity as usize])
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
    mut lookup: impl FnMut(Scope, KeyId) -> u64,
) -> LookupObservation {
    if queries.is_empty() {
        return LookupObservation {
            query_count: 0,
            median_ns_per_query: 0,
            checksum: 0,
        };
    }
    let run = |lookup: &mut dyn FnMut(Scope, KeyId) -> u64| {
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

fn checksum_legacy<Scope, Id: LegacyIdentity>(entries: &[LookupEntry<Scope, Id>]) -> u64 {
    entries.iter().fold(0xcbf2_9ce4_8422_2325, |hash, entry| {
        hash.wrapping_mul(0x0000_0100_0000_01b3)
            .wrapping_add(u64::from(entry.id.entity()) + 1)
    })
}

fn checksum_candidate<Owner, Token, Id>(matched: CandidateMatch<'_, Owner, Id>) -> u64
where
    Id: CandidateIdentity<Owner, Token>,
{
    match matched {
        CandidateMatch::Primary(id) => id.checksum(),
        CandidateMatch::Aliases(entries) => {
            entries.iter().fold(0xcbf2_9ce4_8422_2325, |hash, entry| {
                hash.wrapping_mul(0x0000_0100_0000_01b3)
                    .wrapping_add(entry.id.checksum())
            })
        }
        CandidateMatch::Missing => 0,
    }
}

fn assert_semantic_equivalence<OldScope, NewScope, OldId, Token, NewId>(
    old: &MergedNameLookup<OldScope, OldId>,
    new: &PrimaryAliasLookup<NewScope, Token, NewId>,
    old_queries: &[Query<OldScope>],
    new_queries: &[Query<NewScope>],
    new_entity: &HashMap<NewId, u32>,
) where
    OldScope: Copy + Debug + Ord,
    NewScope: Copy + Debug + Ord,
    OldId: LegacyIdentity,
    Token: InternToken + Hash,
    NewId: CandidateIdentity<NewScope, Token>,
{
    assert_eq!(old_queries.len(), new_queries.len());
    for (old_query, new_query) in old_queries.iter().zip(new_queries) {
        let mut expected = old
            .lookup(old_query.scope, old_query.key)
            .iter()
            .map(|entry| entry.id.entity())
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

struct FamilyRun<NewId> {
    entity_ids: Vec<NewId>,
}

fn run_family<CommonScope, OldScope, NewScope, OldId, Token, NewId>(
    family: &str,
    rows: &[CanonicalRow<CommonScope>],
    missing_key: KeyId,
    old_scope: impl Fn(CommonScope) -> OldScope + Copy,
    new_scope: impl Fn(CommonScope) -> NewScope + Copy,
) -> FamilyRun<NewId>
where
    CommonScope: Copy + Eq + Hash + Ord,
    OldScope: Copy + Debug + Ord,
    NewScope: Copy + Debug + Ord,
    OldId: LegacyIdentity,
    Token: InternToken + Hash,
    NewId: CandidateIdentity<NewScope, Token>,
{
    let query_sets = query_sets(rows, missing_key);
    let old_rows = map_rows(rows, old_scope);
    let new_rows = map_rows(rows, new_scope);
    let (old, old_build) = observe_construction(
        || MergedNameLookup::from_rows(&old_rows, OldId::from_entity),
        MergedNameLookup::retained_bytes,
    );
    let (new_state, new_build) = observe_construction(
        || PrimaryAliasLookup::<NewScope, Token, NewId>::from_rows(&new_rows),
        |built| built.0.retained_bytes() + built.1.capacity() * size_of::<NewId>(),
    );
    let (new, entity_ids) = new_state;
    print_construction(family, "dense_merged", old_build);
    print_construction(family, "composite_primary_alias", new_build);

    let new_entity = entity_ids
        .iter()
        .copied()
        .enumerate()
        .map(|(entity, id)| (id, entity as u32))
        .collect::<HashMap<_, _>>();
    let classes = [
        ("primary", &query_sets.primary),
        ("alias", &query_sets.alias),
        ("missing", &query_sets.missing),
        ("collision", &query_sets.collision),
        ("owner_isolation", &query_sets.owner_isolation),
    ];
    for (class, common_queries) in classes {
        let old_queries = map_queries(common_queries, old_scope);
        let new_queries = map_queries(common_queries, new_scope);
        if class != "collision" {
            assert_semantic_equivalence(&old, &new, &old_queries, &new_queries, &new_entity);
        } else {
            for (old_query, new_query) in old_queries.iter().zip(&new_queries) {
                let old_entities = old
                    .lookup(old_query.scope, old_query.key)
                    .iter()
                    .map(|entry| entry.id.entity())
                    .collect::<Vec<_>>();
                let CandidateMatch::Primary(id) = new.lookup(new_query.scope, new_query.key) else {
                    panic!("candidate collision lookup must prefer primary");
                };
                assert!(
                    old_entities.len() > 1,
                    "merged collision must expose aliases"
                );
                assert!(old_entities.contains(&new_entity[&id]));
            }
        }
        let old_observed = observe_lookup(&old_queries, |scope, key| {
            checksum_legacy(old.lookup(scope, key))
        });
        let new_observed = observe_lookup(&new_queries, |scope, key| {
            checksum_candidate::<NewScope, Token, NewId>(new.lookup(scope, key))
        });
        print_lookup(family, class, "dense_merged", old_observed);
        print_lookup(family, class, "composite_primary_alias", new_observed);
    }
    FamilyRun { entity_ids }
}

#[test]
fn composite_ids_reuse_family_name_tokens_but_not_owner_identity() {
    assert_eq!(size_of::<TypeId>(), 4);
    assert_eq!(size_of::<OwnerId>(), 4);
    assert_eq!(size_of::<CallableId>(), 8);
    assert_eq!(size_of::<PropertyId>(), 8);

    let mut keys = KeyPool::default();
    let type_a = keys.intern("Array");
    let type_b = keys.intern("ValueTable");
    let same_name = keys.intern("Добавить");
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
    let (_, type_ids) = PrimaryAliasLookup::<(), TypeId, TypeId>::from_rows(&types.rows);
    let members = vec![
        CanonicalRow {
            scope: OwnerId::type_owner(type_ids[0]),
            primary: same_name,
            alias: None,
            entity: 0,
        },
        CanonicalRow {
            scope: OwnerId::type_owner(type_ids[1]),
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
    let (callables, callable_ids) =
        PrimaryAliasLookup::<OwnerId, CallableNameId, CallableId>::from_rows(&members);
    let (properties, property_ids) =
        PrimaryAliasLookup::<OwnerId, PropertyNameId, PropertyId>::from_rows(&members);

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
        candidate_ids(properties.lookup(OwnerId::type_owner(type_ids[0]), same_name)),
        vec![property_ids[0]]
    );
}

#[test]
fn primary_precedes_alias_and_alias_ambiguity_is_preserved() {
    let mut keys = KeyPool::default();
    let alpha = keys.intern("Alpha");
    let beta = keys.intern("Beta");
    let gamma = keys.intern("Gamma");
    let delta = keys.intern("Delta");
    let shared = keys.intern("Shared");
    let missing = keys.intern("Missing");
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
    let old = MergedNameLookup::from_rows(&rows, LegacyTypeId::from_entity);
    let (new, entity_ids) = PrimaryAliasLookup::<(), TypeId, TypeId>::from_rows(&rows);

    assert_eq!(candidate_ids(new.lookup((), alpha)), vec![entity_ids[0]]);
    assert_eq!(old.lookup((), alpha).len(), 2);
    assert_eq!(candidate_ids(new.lookup((), shared)).len(), 2);
    assert_eq!(candidate_ids(new.lookup((), delta)), vec![entity_ids[3]]);
    assert_eq!(new.primary_names.get(shared), None);
    assert!(matches!(new.lookup((), missing), CandidateMatch::Missing));

    let queries = query_sets(&rows, missing);
    let new_entity = entity_ids
        .iter()
        .copied()
        .enumerate()
        .map(|(entity, id)| (id, entity as u32))
        .collect::<HashMap<_, _>>();
    assert_semantic_equivalence(&old, &new, &queries.alias, &queries.alias, &new_entity);
    assert_semantic_equivalence(&old, &new, &queries.missing, &queries.missing, &new_entity);
}

#[test]
fn temporary_duplicate_projection_reuses_canonical_owner() {
    let mut keys = KeyPool::default();
    let duplicate_type = keys.intern("Duplicate type");
    let member = keys.intern("Member");
    let first_alias = keys.intern("First alias");
    let dropped_alias = keys.intern("Dropped alias");
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

struct ProjectedCorpus {
    keys: KeyPool,
    types: CanonicalFamily<()>,
    callables: CanonicalFamily<Option<u32>>,
    properties: CanonicalFamily<Option<u32>>,
}

fn project_name(
    keys: &mut KeyPool,
    handle: HbkFactReadHandle<'_>,
    name: HbkNameView<'_>,
) -> (KeyId, Option<KeyId>) {
    let primary = keys.intern(handle.string(name.primary()));
    let alias = name.alias().map(|alias| keys.intern(handle.string(alias)));
    (primary, alias)
}

fn project_corpus(snapshot: &HbkFactSnapshot) -> ProjectedCorpus {
    let handle = snapshot.worker_handle();
    let counts = snapshot.counts();
    let mut keys = KeyPool::default();
    let mut source_types = Vec::with_capacity(counts.platform_types + counts.enums);
    for ordinal in 0..counts.platform_types {
        let view = handle.platform_type(HbkPlatformTypeId(
            u32::try_from(ordinal).expect("platform type count overflowed u32"),
        ));
        let (primary, alias) = project_name(&mut keys, handle, view.name());
        source_types.push(SourceTypeRow { primary, alias });
    }
    for ordinal in 0..counts.enums {
        let view = handle.enum_fact(HbkEnumId(
            u32::try_from(ordinal).expect("enum count overflowed u32"),
        ));
        let (primary, alias) = project_name(&mut keys, handle, view.name());
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
        let (primary, alias) = project_name(&mut keys, handle, view.name());
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
        let (primary, alias) = project_name(&mut keys, handle, view.name());
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
        let (primary, alias) = project_name(&mut keys, handle, view.name());
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
        let (primary, alias) = project_name(&mut keys, handle, view.name());
        source_properties.push(SourceMemberRow {
            owner_source: Some(counts.platform_types + view.owner().0 as usize),
            primary,
            alias,
        });
    }
    let properties = canonicalize_members(&source_properties, &source_to_type);

    ProjectedCorpus {
        keys,
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
    let mut corpus = project_corpus(&snapshot);
    let missing_key = corpus.keys.intern("__v8_context_primary_alias_missing__");

    println!(
        "corpus platform_version={} locale={} extraction_schema={} source_hbk={} interned_keys={} retained_key_bytes={}",
        FROZEN_PLATFORM_VERSION,
        metadata.locale,
        metadata.source_extraction_schema_version,
        metadata.source_hbk,
        corpus.keys.by_text.len(),
        corpus.keys.retained_key_bytes,
    );
    print_family_corpus("type", &corpus.types);
    print_family_corpus("callable", &corpus.callables);
    print_family_corpus("property", &corpus.properties);

    let types = run_family::<(), (), (), LegacyTypeId, TypeId, TypeId>(
        "type",
        &corpus.types.rows,
        missing_key,
        |()| (),
        |()| (),
    );
    let type_ids = types.entity_ids;
    run_family::<Option<u32>, LegacyOwnerId, OwnerId, LegacyCallableId, CallableNameId, CallableId>(
        "callable",
        &corpus.callables.rows,
        missing_key,
        map_member_scope_to_legacy,
        |scope| map_member_scope_to_candidate(scope, &type_ids),
    );
    run_family::<Option<u32>, LegacyOwnerId, OwnerId, LegacyPropertyId, PropertyNameId, PropertyId>(
        "property",
        &corpus.properties.rows,
        missing_key,
        map_member_scope_to_legacy,
        |scope| map_member_scope_to_candidate(scope, &type_ids),
    );
}
