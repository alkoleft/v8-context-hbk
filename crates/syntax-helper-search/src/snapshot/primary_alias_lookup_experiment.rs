use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::hint::black_box;
use std::mem::size_of;
#[cfg(feature = "snapshot-experiment-alloc")]
use std::path::PathBuf;
use std::time::Instant;

use super::indexes::matching_range;
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
    fn index(self) -> usize;
}

trait CandidateIdentity: Copy + Debug + Eq + Hash + Ord {
    const DIRECT_NAME_PRIMARY: bool;

    type Owner: Copy + Debug + Ord;
    type Token: InternToken + Hash;

    fn compose(owner: Self::Owner, token: Self::Token) -> Self;
    fn owner(self) -> Self::Owner;
    fn token(self) -> Self::Token;
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

            fn index(self) -> usize {
                usize::try_from(self.0).expect("experimental name token must fit usize")
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
    const DIRECT_NAME_PRIMARY: bool = true;

    type Owner = ();
    type Token = TypeId;

    fn compose((): (), token: TypeId) -> Self {
        token
    }

    fn owner(self) {}

    fn token(self) -> TypeId {
        self
    }

    fn checksum(self) -> u64 {
        u64::from(self.0) + 1
    }
}

impl CandidateIdentity for CallableId {
    const DIRECT_NAME_PRIMARY: bool = false;

    type Owner = OwnerId;
    type Token = CallableNameId;

    fn compose(owner: OwnerId, name: CallableNameId) -> Self {
        Self { owner, name }
    }

    fn owner(self) -> OwnerId {
        self.owner
    }

    fn token(self) -> CallableNameId {
        self.name
    }

    fn checksum(self) -> u64 {
        (u64::from(self.owner.0) << 32 | u64::from(self.name.0)).wrapping_add(1)
    }
}

impl CandidateIdentity for PropertyId {
    const DIRECT_NAME_PRIMARY: bool = false;

    type Owner = OwnerId;
    type Token = PropertyNameId;

    fn compose(owner: OwnerId, name: PropertyNameId) -> Self {
        Self { owner, name }
    }

    fn owner(self) -> OwnerId {
        self.owner
    }

    fn token(self) -> PropertyNameId {
        self.name
    }

    fn checksum(self) -> u64 {
        (u64::from(self.owner.0) << 32 | u64::from(self.name.0)).wrapping_add(1)
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceTypeRow<'a> {
    primary_raw: &'a str,
    alias_raw: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
struct SourceMemberRow<'a> {
    owner_source: Option<usize>,
    primary_raw: &'a str,
    alias_raw: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AliasName<'a> {
    raw: &'a str,
    key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalRow<'a, Scope> {
    scope: Scope,
    primary_raw: &'a str,
    primary: String,
    alias: Option<AliasName<'a>>,
    entity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LegacyCanonicalRow<Scope> {
    scope: Scope,
    primary: StringId,
    alias: Option<StringId>,
    entity: u32,
}

struct CanonicalFamily<'a, Scope> {
    rows: Vec<CanonicalRow<'a, Scope>>,
    source_rows: usize,
    duplicate_primaries: usize,
    supplied_aliases: usize,
    redundant_aliases: usize,
}

impl<Scope> CanonicalFamily<'_, Scope> {
    fn retained_aliases(&self) -> usize {
        self.supplied_aliases - self.redundant_aliases
    }
}

fn canonicalize_types<'a>(source: &[SourceTypeRow<'a>]) -> (CanonicalFamily<'a, ()>, Vec<u32>) {
    let mut canonical_by_primary = HashMap::<String, u32>::new();
    let mut rows = Vec::with_capacity(source.len());
    let mut source_to_type = Vec::with_capacity(source.len());
    let mut duplicate_primaries = 0;
    let mut supplied_aliases = 0;
    let mut redundant_aliases = 0;

    for row in source {
        let primary = normalize_lookup_key(row.primary_raw);
        let alias = row.alias_raw.map(|raw| AliasName {
            raw,
            key: normalize_lookup_key(raw),
        });
        // TEMPORARY: the HBK formation/extension composition owner must make
        // primaries unique before a production identity cutover. Both compared
        // layouts receive the same stable first-row projection.
        if let Some(entity) = canonical_by_primary.get(&primary).copied() {
            duplicate_primaries += 1;
            source_to_type.push(entity);
            continue;
        }
        let entity = u32::try_from(rows.len()).expect("canonical type count overflowed u32");
        canonical_by_primary.insert(primary.clone(), entity);
        source_to_type.push(entity);
        supplied_aliases += usize::from(alias.is_some());
        redundant_aliases += usize::from(alias.as_ref().is_some_and(|alias| alias.key == primary));
        rows.push(CanonicalRow {
            scope: (),
            primary_raw: row.primary_raw,
            primary: primary.clone(),
            alias: alias.clone(),
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

fn canonicalize_members<'a>(
    source: &[SourceMemberRow<'a>],
    source_to_type: &[u32],
) -> CanonicalFamily<'a, Option<u32>> {
    let mut canonical_by_primary = HashMap::<(Option<u32>, String), u32>::new();
    let mut rows = Vec::with_capacity(source.len());
    let mut duplicate_primaries = 0;
    let mut supplied_aliases = 0;
    let mut redundant_aliases = 0;

    for row in source {
        let primary = normalize_lookup_key(row.primary_raw);
        let alias = row.alias_raw.map(|raw| AliasName {
            raw,
            key: normalize_lookup_key(raw),
        });
        let scope = row.owner_source.map(|owner| {
            source_to_type
                .get(owner)
                .copied()
                .expect("snapshot member owner must reference a projected type")
        });
        // TEMPORARY: uniqueness is scoped by the retained type (or the global
        // context), not by the source ordinal that happened to declare it.
        if canonical_by_primary.contains_key(&(scope, primary.clone())) {
            duplicate_primaries += 1;
            continue;
        }
        let entity = u32::try_from(rows.len()).expect("canonical member count overflowed u32");
        canonical_by_primary.insert((scope, primary.clone()), entity);
        supplied_aliases += usize::from(alias.is_some());
        redundant_aliases += usize::from(alias.as_ref().is_some_and(|alias| alias.key == primary));
        rows.push(CanonicalRow {
            scope,
            primary_raw: row.primary_raw,
            primary: primary.clone(),
            alias: alias.clone(),
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
    rows: &[LegacyCanonicalRow<Scope>],
    id: impl Fn(u32) -> Id,
    snapshot: &HbkFactSnapshot,
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
    entries.sort_unstable_by(|left, right| {
        (left.owner, snapshot.string(left.key), left.value).cmp(&(
            right.owner,
            snapshot.string(right.key),
            right.value,
        ))
    });
    entries.dedup_by_key(|entry| (entry.owner, entry.key, entry.value));
    entries
}

fn merged_name_lookup_raw<'a, Scope, Id>(
    entries: &'a [OwnerNameLookup<Scope, Id>],
    scope: Scope,
    raw: &str,
    snapshot: &HbkFactSnapshot,
) -> &'a [OwnerNameLookup<Scope, Id>]
where
    Scope: Copy + Ord,
    Id: Copy + Ord,
{
    let normalized = normalize_lookup_key(raw);
    let range = matching_range(entries, |entry| {
        entry
            .owner
            .cmp(&scope)
            .then_with(|| snapshot.string(entry.key).cmp(normalized.as_str()))
    });
    &entries[range]
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AliasLookup<Id> {
    key: Box<str>,
    value: Id,
}

/// The one routed primary/alias mechanism used by all three candidate
/// families. Candidate state does not retain snapshot string handles.
struct PrimaryAliasLookup<Id> {
    names: Vec<Box<str>>,
    primaries: Vec<Id>,
    aliases: Vec<AliasLookup<Id>>,
}

impl<Id> PrimaryAliasLookup<Id>
where
    Id: CandidateIdentity,
{
    fn from_rows(rows: &[CanonicalRow<Id::Owner>]) -> Self {
        let mut name_texts = rows
            .iter()
            .map(|row| row.primary.as_str())
            .collect::<Vec<_>>();
        name_texts.sort_unstable();
        name_texts.dedup();
        let mut names = name_texts
            .into_iter()
            .map(Box::<str>::from)
            .collect::<Vec<_>>();
        let tokens = names
            .iter()
            .enumerate()
            .map(|(index, name)| (name.as_ref(), Id::Token::from_index(index)))
            .collect::<HashMap<_, _>>();
        let mut primaries = if Id::DIRECT_NAME_PRIMARY {
            Vec::new()
        } else {
            Vec::with_capacity(rows.len())
        };
        let mut aliases = Vec::with_capacity(
            rows.iter()
                .filter(|row| {
                    row.alias
                        .as_ref()
                        .is_some_and(|alias| alias.key != row.primary)
                })
                .count(),
        );
        for row in rows {
            let token = tokens[row.primary.as_str()];
            let id = Id::compose(row.scope, token);
            if !Id::DIRECT_NAME_PRIMARY {
                primaries.push(id);
            }
            if let Some(alias) = row.alias.as_ref().filter(|alias| alias.key != row.primary) {
                aliases.push(AliasLookup {
                    key: Box::<str>::from(alias.key.as_str()),
                    value: id,
                });
            }
        }
        if Id::DIRECT_NAME_PRIMARY {
            assert_eq!(
                names.len(),
                rows.len(),
                "direct-name primary IDs require unique primary names"
            );
        } else {
            primaries.sort_unstable_by(|left, right| {
                (left.owner(), &*names[left.token().index()], *left).cmp(&(
                    right.owner(),
                    &*names[right.token().index()],
                    *right,
                ))
            });
            primaries.dedup_by(|left, right| {
                left.owner() == right.owner()
                    && names[left.token().index()] == names[right.token().index()]
            });
            assert_eq!(primaries.len(), rows.len(), "candidate IDs must be unique");
        }
        aliases.sort_unstable_by(|left, right| {
            (left.value.owner(), left.key.as_ref(), left.value).cmp(&(
                right.value.owner(),
                right.key.as_ref(),
                right.value,
            ))
        });
        aliases.dedup();
        names.shrink_to_fit();
        Self {
            names,
            primaries,
            aliases,
        }
    }

    fn lookup_raw(&self, owner: Id::Owner, raw: &str) -> CandidateMatch<'_, Id> {
        self.lookup_raw_counted(owner, raw, &mut |_| {})
    }

    fn lookup_raw_counted(
        &self,
        owner: Id::Owner,
        raw: &str,
        on_range: &mut impl FnMut(&'static str),
    ) -> CandidateMatch<'_, Id> {
        let normalized = normalize_lookup_key(raw);
        if routes_to_alias(&normalized) {
            on_range("alias");
            let aliases = self.matching_aliases(owner, &normalized);
            if aliases.is_empty() {
                CandidateMatch::Missing
            } else {
                CandidateMatch::Aliases(aliases)
            }
        } else {
            on_range("primary");
            self.matching_primary(owner, &normalized)
        }
    }

    fn lookup_primary_key(&self, owner: Id::Owner, key: &str) -> CandidateMatch<'_, Id> {
        self.matching_primary(owner, key)
    }

    fn matching_primary(&self, owner: Id::Owner, key: &str) -> CandidateMatch<'_, Id> {
        if Id::DIRECT_NAME_PRIMARY {
            let Ok(index) = self.names.binary_search_by(|name| name.as_ref().cmp(key)) else {
                return CandidateMatch::Missing;
            };
            return CandidateMatch::Primary(Id::compose(owner, Id::Token::from_index(index)));
        }
        let range = matching_range(&self.primaries, |id| {
            id.owner()
                .cmp(&owner)
                .then_with(|| self.names[id.token().index()].as_ref().cmp(key))
        });
        match &self.primaries[range] {
            [] => CandidateMatch::Missing,
            [id] => CandidateMatch::Primary(*id),
            _ => panic!("candidate primary table contains repeated owner/text"),
        }
    }

    fn matching_aliases(&self, owner: Id::Owner, key: &str) -> &[AliasLookup<Id>] {
        let range = matching_range(&self.aliases, |entry| {
            entry
                .value
                .owner()
                .cmp(&owner)
                .then_with(|| entry.key.as_ref().cmp(key))
        });
        &self.aliases[range]
    }

    fn retained_bytes(&self) -> usize {
        self.names.capacity() * size_of::<Box<str>>()
            + self.names.iter().map(|name| name.len()).sum::<usize>()
            + self.primaries.capacity() * size_of::<Id>()
            + self.aliases.capacity() * size_of::<AliasLookup<Id>>()
            + self
                .aliases
                .iter()
                .map(|entry| entry.key.len())
                .sum::<usize>()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CandidateMatch<'a, Id> {
    Primary(Id),
    Aliases(&'a [AliasLookup<Id>]),
    Missing,
}

fn candidate_ids<Id: Copy>(matched: CandidateMatch<'_, Id>) -> Vec<Id> {
    match matched {
        CandidateMatch::Primary(id) => vec![id],
        CandidateMatch::Aliases(entries) => entries.iter().map(|entry| entry.value).collect(),
        CandidateMatch::Missing => Vec::new(),
    }
}

fn routes_to_alias(normalized: &str) -> bool {
    normalized
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Query<'a, Scope> {
    scope: Scope,
    raw: &'a str,
    key: &'a str,
}

struct QuerySets<'a, Scope> {
    primary: Vec<Query<'a, Scope>>,
    alias: Vec<Query<'a, Scope>>,
    missing: Vec<Query<'a, Scope>>,
    owner_isolation: Vec<Query<'a, Scope>>,
    excluded_ascii_primary: usize,
    excluded_non_ascii_alias: usize,
    raw_collisions: usize,
}

fn query_sets<'a, Scope>(
    rows: &'a [CanonicalRow<'a, Scope>],
    ascii_missing_raw: &'a str,
    non_ascii_missing_raw: &'a str,
) -> QuerySets<'a, Scope>
where
    Scope: Copy + Eq + Hash + Ord,
{
    let primaries = rows
        .iter()
        .map(|row| Query {
            scope: row.scope,
            raw: row.primary_raw,
            key: row.primary.as_str(),
        })
        .collect::<BTreeSet<_>>();
    let aliases = rows
        .iter()
        .filter_map(|row| {
            row.alias
                .as_ref()
                .filter(|alias| alias.key != row.primary)
                .map(|alias| Query {
                    scope: row.scope,
                    raw: alias.raw,
                    key: alias.key.as_str(),
                })
        })
        .collect::<BTreeSet<_>>();
    let primary_keys = primaries
        .iter()
        .map(|query| (query.scope, query.key))
        .collect::<BTreeSet<_>>();
    let alias_keys = aliases
        .iter()
        .map(|query| (query.scope, query.key))
        .collect::<BTreeSet<_>>();
    let collision = primary_keys
        .intersection(&alias_keys)
        .copied()
        .collect::<Vec<_>>();
    let collision_set = collision.iter().copied().collect::<BTreeSet<_>>();
    let primary_all = primaries
        .iter()
        .filter(|query| !alias_keys.contains(&(query.scope, query.key)))
        .cloned()
        .collect::<Vec<_>>();
    let alias_all = aliases
        .iter()
        .filter(|query| !primary_keys.contains(&(query.scope, query.key)))
        .cloned()
        .collect::<Vec<_>>();
    let primary = primary_all
        .iter()
        .filter(|query| !routes_to_alias(query.key))
        .cloned()
        .collect::<Vec<_>>();
    let alias = alias_all
        .iter()
        .filter(|query| routes_to_alias(query.key))
        .cloned()
        .collect::<Vec<_>>();
    let excluded_ascii_primary = primary_all.len() - primary.len();
    let excluded_non_ascii_alias = alias_all.len() - alias.len();
    let missing = rows
        .iter()
        .map(|row| row.scope)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .flat_map(|scope| {
            [
                Query {
                    scope,
                    raw: ascii_missing_raw,
                    key: ascii_missing_raw,
                },
                Query {
                    scope,
                    raw: non_ascii_missing_raw,
                    key: non_ascii_missing_raw,
                },
            ]
        })
        .collect();

    let mut scopes_by_key = BTreeMap::<&str, BTreeSet<Scope>>::new();
    for query in primaries.iter().chain(aliases.iter()) {
        scopes_by_key
            .entry(query.key)
            .or_default()
            .insert(query.scope);
    }
    let owner_isolation = scopes_by_key
        .into_iter()
        .filter(|(_, scopes)| scopes.len() > 1)
        .flat_map(|(key, scopes)| {
            let raw = primaries
                .iter()
                .chain(&aliases)
                .find(|query| query.key == key)
                .expect("owner-isolation key must come from query corpus")
                .raw;
            scopes
                .into_iter()
                .map(move |scope| Query { scope, raw, key })
        })
        .filter(|query| !collision_set.contains(&(query.scope, query.key)))
        .filter(|query| {
            (primary_keys.contains(&(query.scope, query.key)) && !routes_to_alias(query.key))
                || (alias_keys.contains(&(query.scope, query.key)) && routes_to_alias(query.key))
        })
        .collect();

    QuerySets {
        primary,
        alias,
        missing,
        owner_isolation,
        excluded_ascii_primary,
        excluded_non_ascii_alias,
        raw_collisions: collision.len(),
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
        let CandidateMatch::Primary(id) = types.lookup_primary_key((), &row.primary) else {
            panic!("canonical member owner type must resolve through the type lookup");
        };
        OwnerId::type_owner(id)
    })
}

fn map_rows<'a, From: Copy, To>(
    rows: &[CanonicalRow<'a, From>],
    mut scope: impl FnMut(From) -> To,
) -> Vec<CanonicalRow<'a, To>> {
    rows.iter()
        .map(|row| CanonicalRow {
            scope: scope(row.scope),
            primary_raw: row.primary_raw,
            primary: row.primary.clone(),
            alias: row.alias.clone(),
            entity: row.entity,
        })
        .collect()
}

fn map_queries<'a, From: Copy, To>(
    queries: &[Query<'a, From>],
    mut scope: impl FnMut(From) -> To,
) -> Vec<Query<'a, To>> {
    queries
        .iter()
        .map(|query| Query {
            scope: scope(query.scope),
            raw: query.raw,
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
    queries: &[Query<'_, Scope>],
    mut lookup: impl FnMut(Scope, &str) -> u64,
) -> LookupObservation {
    if queries.is_empty() {
        return LookupObservation {
            query_count: 0,
            median_ns_per_query: 0,
            checksum: 0,
        };
    }
    let run = |lookup: &mut dyn FnMut(Scope, &str) -> u64| {
        let mut checksum = 0_u64;
        for _ in 0..LOOKUP_PASSES {
            for query in queries {
                checksum = checksum.wrapping_add(black_box(lookup(query.scope, query.raw)));
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
            let CandidateMatch::Primary(id) = new.lookup_primary_key(row.scope, &row.primary)
            else {
                panic!("candidate primary row must resolve during differential setup");
            };
            (id, row.entity)
        })
        .collect()
}

fn assert_semantic_equivalence<OldScope, NewScope, OldId, NewId>(
    old: &[OwnerNameLookup<OldScope, OldId>],
    new: &PrimaryAliasLookup<NewId>,
    old_queries: &[Query<'_, OldScope>],
    new_queries: &[Query<'_, NewScope>],
    new_entity: &HashMap<NewId, u32>,
    snapshot: &HbkFactSnapshot,
) where
    OldScope: Copy + Debug + Ord,
    NewScope: Copy + Debug + Ord,
    OldId: LegacyIdentity,
    NewId: CandidateIdentity<Owner = NewScope>,
{
    assert_eq!(old_queries.len(), new_queries.len());
    for (old_query, new_query) in old_queries.iter().zip(new_queries) {
        let mut expected = merged_name_lookup_raw(old, old_query.scope, old_query.raw, snapshot)
            .iter()
            .map(|entry| entry.value.entity())
            .collect::<Vec<_>>();
        let mut actual = candidate_ids(new.lookup_raw(new_query.scope, new_query.raw))
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

fn print_query_corpus<Scope>(family: &str, queries: &QuerySets<Scope>) {
    println!(
        "query_corpus family={family} included_primary={} included_alias={} included_missing={} included_owner_isolation={} excluded_ascii_primary={} excluded_non_ascii_alias={} raw_collisions={}",
        queries.primary.len(),
        queries.alias.len(),
        queries.missing.len(),
        queries.owner_isolation.len(),
        queries.excluded_ascii_primary,
        queries.excluded_non_ascii_alias,
        queries.raw_collisions,
    );
}

fn run_family<CommonScope, OldScope, NewScope, OldId, NewId>(
    family: &str,
    rows: &[CanonicalRow<'_, CommonScope>],
    legacy_rows: &[LegacyCanonicalRow<CommonScope>],
    missing: &MissingNames,
    snapshot: &HbkFactSnapshot,
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
    let query_sets = query_sets(rows, &missing.ascii_raw, &missing.non_ascii_raw);
    print_query_corpus(family, &query_sets);
    let old_legacy_rows = legacy_rows
        .iter()
        .map(|row| LegacyCanonicalRow {
            scope: old_scope(row.scope),
            primary: row.primary,
            alias: row.alias,
            entity: row.entity,
        })
        .collect::<Vec<_>>();
    let new_rows = map_rows(rows, new_scope);
    let (old, old_build) = observe_construction(
        || merged_name_lookup_from_rows(&old_legacy_rows, OldId::from_entity, snapshot),
        |entries| entries.capacity() * size_of::<OwnerNameLookup<OldScope, OldId>>(),
    );
    let (new_state, new_build) = observe_construction(
        || PrimaryAliasLookup::<NewId>::from_rows(&new_rows),
        PrimaryAliasLookup::retained_bytes,
    );
    let new = new_state;
    print_construction(family, "current_raw_name", old_build);
    print_construction(family, "routed_primary_alias_raw_name", new_build);

    let classes = [
        ("primary", &query_sets.primary),
        ("alias", &query_sets.alias),
        ("missing", &query_sets.missing),
        ("owner_isolation", &query_sets.owner_isolation),
    ];
    {
        let new_entity = differential_entity_map::<NewId>(&new, &new_rows);
        for (_, common_queries) in classes {
            let old_queries = map_queries(common_queries, old_scope);
            let new_queries = map_queries(common_queries, new_scope);
            assert_semantic_equivalence(
                &old,
                &new,
                &old_queries,
                &new_queries,
                &new_entity,
                snapshot,
            );
        }
    }
    for (class, common_queries) in classes {
        let old_queries = map_queries(common_queries, old_scope);
        let new_queries = map_queries(common_queries, new_scope);
        let old_observed = observe_lookup(&old_queries, |scope, raw| {
            checksum_legacy(merged_name_lookup_raw(&old, scope, raw, snapshot))
        });
        let new_observed = observe_lookup(&new_queries, |scope, raw| {
            checksum_candidate::<NewId>(new.lookup_raw(scope, raw))
        });
        print_lookup(family, class, "current_raw_name", old_observed);
        print_lookup(family, class, "routed_primary_alias_raw_name", new_observed);
    }
    new
}

#[test]
fn composite_ids_reuse_family_name_tokens_but_not_owner_identity() {
    assert_eq!(size_of::<TypeId>(), 4);
    assert_eq!(size_of::<OwnerId>(), 4);
    assert_eq!(size_of::<CallableId>(), 8);
    assert_eq!(size_of::<PropertyId>(), 8);

    let (types, _) = canonicalize_types(&[
        SourceTypeRow {
            primary_raw: "Array",
            alias_raw: None,
        },
        SourceTypeRow {
            primary_raw: "ValueTable",
            alias_raw: None,
        },
    ]);
    let type_lookup = PrimaryAliasLookup::<TypeId>::from_rows(&types.rows);
    let CandidateMatch::Primary(type_a_id) = type_lookup.lookup_primary_key((), "array") else {
        panic!("type A must resolve");
    };
    let CandidateMatch::Primary(type_b_id) = type_lookup.lookup_primary_key((), "valuetable")
    else {
        panic!("type B must resolve");
    };
    let members = vec![
        CanonicalRow {
            scope: OwnerId::type_owner(type_a_id),
            primary_raw: "Добавить",
            primary: "добавить".to_string(),
            alias: None,
            entity: 0,
        },
        CanonicalRow {
            scope: OwnerId::type_owner(type_b_id),
            primary_raw: "Добавить",
            primary: "добавить".to_string(),
            alias: None,
            entity: 1,
        },
        CanonicalRow {
            scope: OwnerId::GLOBAL,
            primary_raw: "Добавить",
            primary: "добавить".to_string(),
            alias: None,
            entity: 2,
        },
    ];
    let callables = PrimaryAliasLookup::<CallableId>::from_rows(&members);
    let properties = PrimaryAliasLookup::<PropertyId>::from_rows(&members);
    let callable_ids =
        candidate_ids(callables.lookup_raw(OwnerId::type_owner(type_a_id), "Добавить"))
            .into_iter()
            .chain(candidate_ids(
                callables.lookup_raw(OwnerId::type_owner(type_b_id), "Добавить"),
            ))
            .chain(candidate_ids(
                callables.lookup_raw(OwnerId::GLOBAL, "Добавить"),
            ))
            .collect::<Vec<_>>();
    let property_ids =
        candidate_ids(properties.lookup_raw(OwnerId::type_owner(type_a_id), "Добавить"))
            .into_iter()
            .chain(candidate_ids(
                properties.lookup_raw(OwnerId::type_owner(type_b_id), "Добавить"),
            ))
            .chain(candidate_ids(
                properties.lookup_raw(OwnerId::GLOBAL, "Добавить"),
            ))
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
        candidate_ids(callables.lookup_raw(OwnerId::GLOBAL, "Добавить")),
        vec![callable_ids[2]]
    );
    assert_eq!(
        candidate_ids(properties.lookup_raw(OwnerId::type_owner(type_a_id), "Добавить")),
        vec![property_ids[0]]
    );
}

#[test]
fn primary_precedes_alias_and_alias_ambiguity_is_preserved() {
    let rows = vec![
        CanonicalRow {
            scope: (),
            primary_raw: "Массив",
            primary: "массив".to_string(),
            alias: Some(AliasName {
                raw: "Array",
                key: "array".to_string(),
            }),
            entity: 0,
        },
        CanonicalRow {
            scope: (),
            primary_raw: "ТаблицаЗначений",
            primary: "таблицазначений".to_string(),
            alias: Some(AliasName {
                raw: "Array",
                key: "array".to_string(),
            }),
            entity: 1,
        },
        CanonicalRow {
            scope: (),
            primary_raw: "Структура",
            primary: "структура".to_string(),
            alias: Some(AliasName {
                raw: "Массив",
                key: "массив".to_string(),
            }),
            entity: 2,
        },
        CanonicalRow {
            scope: (),
            primary_raw: "Дата",
            primary: "дата".to_string(),
            alias: None,
            entity: 3,
        },
    ];
    let new = PrimaryAliasLookup::<TypeId>::from_rows(&rows);
    assert_eq!(new.aliases.capacity(), new.aliases.len());
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

    assert_eq!(candidate_ids(new.lookup_raw((), "Массив")), vec![alpha_id]);
    assert_eq!(candidate_ids(new.lookup_raw((), "Array")).len(), 2);
    assert_eq!(candidate_ids(new.lookup_raw((), "Дата")), vec![delta_id]);
    assert!(new.aliases.iter().all(|entry| &*entry.key != "дата"));
    assert!(matches!(
        new.lookup_raw((), "Missing"),
        CandidateMatch::Missing
    ));

    let queries = query_sets(&rows, "Missing", "Отсутствует");
    assert_eq!(queries.alias.len(), 1);
    assert_eq!(queries.missing.len(), 2);
}

#[test]
fn russian_primary_single_row_index_is_type_id() {
    let (types, _) = canonicalize_types(&[SourceTypeRow {
        primary_raw: "Массив",
        alias_raw: Some("Array"),
    }]);
    let lookup = PrimaryAliasLookup::<TypeId>::from_rows(&types.rows);

    assert!(lookup.primaries.is_empty());
    assert_eq!(lookup.primaries.capacity(), 0);
    assert_eq!(
        lookup
            .names
            .iter()
            .map(|name| name.as_ref())
            .collect::<Vec<_>>(),
        vec!["массив"]
    );
    assert_eq!(
        candidate_ids(lookup.lookup_raw((), "Массив")),
        vec![TypeId(0)]
    );
    assert_eq!(
        candidate_ids(lookup.lookup_raw((), "Array")),
        vec![TypeId(0)]
    );
}

#[test]
fn same_name_under_other_owner_is_missing_without_fallback() {
    let owner_a = OwnerId(10);
    let owner_b = OwnerId(20);
    let rows = vec![CanonicalRow {
        scope: owner_a,
        primary_raw: "Добавить",
        primary: "добавить".to_string(),
        alias: Some(AliasName {
            raw: "Add",
            key: "add".to_string(),
        }),
        entity: 0,
    }];
    let lookup = PrimaryAliasLookup::<CallableId>::from_rows(&rows);

    assert!(matches!(
        lookup.lookup_raw(owner_b, "Добавить"),
        CandidateMatch::Missing
    ));
    assert!(matches!(
        lookup.lookup_raw(owner_b, "Add"),
        CandidateMatch::Missing
    ));
}

#[test]
fn routed_lookup_uses_equal_raw_input_and_one_vector() {
    let rows = vec![CanonicalRow {
        scope: (),
        primary_raw: "Массив",
        primary: "массив".to_string(),
        alias: Some(AliasName {
            raw: "Array",
            key: "array".to_string(),
        }),
        entity: 0,
    }];
    let new = PrimaryAliasLookup::<TypeId>::from_rows(&rows);

    let mut routes = Vec::new();
    let raw_alias = "Array";
    let actual = candidate_ids(new.lookup_raw_counted((), raw_alias, &mut |route| {
        routes.push(route);
    }))
    .into_iter()
    .map(|id| id.0)
    .collect::<Vec<_>>();
    assert_eq!(actual, vec![0]);
    assert_eq!(routes, vec!["alias"]);

    routes.clear();
    let raw_primary = "Массив";
    let actual = candidate_ids(new.lookup_raw_counted((), raw_primary, &mut |route| {
        routes.push(route);
    }))
    .into_iter()
    .map(|id| id.0)
    .collect::<Vec<_>>();
    assert_eq!(actual, vec![0]);
    assert_eq!(routes, vec!["primary"]);
}

#[test]
fn temporary_duplicate_projection_reuses_canonical_owner() {
    let (types, source_to_type) = canonicalize_types(&[
        SourceTypeRow {
            primary_raw: "Duplicate type",
            alias_raw: Some("First alias"),
        },
        SourceTypeRow {
            primary_raw: "duplicate type",
            alias_raw: Some("Dropped alias"),
        },
    ]);
    assert_eq!(types.rows.len(), 1);
    assert_eq!(types.duplicate_primaries, 1);
    assert_eq!(source_to_type, vec![0, 0]);

    let members = canonicalize_members(
        &[
            SourceMemberRow {
                owner_source: Some(0),
                primary_raw: "Member",
                alias_raw: None,
            },
            SourceMemberRow {
                owner_source: Some(1),
                primary_raw: "member",
                alias_raw: Some("Dropped alias"),
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
    assert!(lookup.contains("names: Vec<Box<str>>"));
    assert!(lookup.contains("primaries: Vec<Id>"));
    assert!(lookup.contains("aliases: Vec<AliasLookup<Id>>"));
    assert!(!lookup.contains("StringId"));
    assert!(!lookup.contains("NameLookup"));
    assert!(!lookup.contains("OwnerNameLookup"));
    assert!(!lookup.contains("HashMap"));

    let source_rows = source
        .split("struct SourceTypeRow")
        .nth(1)
        .and_then(|tail| tail.split("struct LegacyCanonicalRow").next())
        .expect("source/canonical rows must remain inspectable");
    assert!(!source_rows.contains("StringId"));
    assert!(!source_rows.contains("primary_raw: String"));
    assert!(!source_rows.contains("alias_raw: Option<String>"));

    let query = source
        .split("struct Query")
        .nth(1)
        .and_then(|tail| tail.split("struct QuerySets").next())
        .expect("query rows must remain inspectable");
    assert!(!query.contains("StringId"));
    assert!(!query.contains("String"));

    let raw_lookup = source
        .split("fn lookup_raw_counted")
        .nth(1)
        .and_then(|tail| tail.split("fn lookup_primary_key").next())
        .expect("raw routed lookup body must remain inspectable");
    assert!(raw_lookup.contains("if routes_to_alias(&normalized)"));
    assert_eq!(raw_lookup.matches("self.matching_aliases").count(), 1);
    assert_eq!(raw_lookup.matches("self.matching_primary").count(), 1);
    assert!(!raw_lookup.contains("lookup_raw("));

    let primary_match = source
        .split("fn matching_primary")
        .nth(1)
        .and_then(|tail| tail.split("fn matching_aliases").next())
        .expect("primary matcher body must remain inspectable");
    let alias_match = source
        .split("fn matching_aliases")
        .nth(1)
        .and_then(|tail| tail.split("fn retained_bytes").next())
        .expect("alias matcher body must remain inspectable");
    assert_eq!(primary_match.matches("matching_range").count(), 1);
    assert_eq!(alias_match.matches("matching_range").count(), 1);

    let corpus = source
        .rsplit("struct ProjectedCorpus")
        .next()
        .and_then(|tail| tail.split("}\n\nfn").next())
        .expect("projected corpus declaration must remain inspectable");
    assert!(!corpus.contains("HashMap"));
    assert!(!corpus.contains("by_text"));
    assert!(!corpus.contains("by_string"));
}

struct MissingNames {
    ascii_raw: String,
    non_ascii_raw: String,
    ascii_normalized: String,
    non_ascii_normalized: String,
}

struct ProjectedCorpus<'a> {
    missing: MissingNames,
    raw_name_count: usize,
    raw_name_bytes: usize,
    types: CanonicalFamily<'a, ()>,
    legacy_types: Vec<LegacyCanonicalRow<()>>,
    callables: CanonicalFamily<'a, Option<u32>>,
    legacy_callables: Vec<LegacyCanonicalRow<Option<u32>>>,
    properties: CanonicalFamily<'a, Option<u32>>,
    legacy_properties: Vec<LegacyCanonicalRow<Option<u32>>>,
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

fn project_key(keys: &HashMap<&str, StringId>, normalized: &str) -> StringId {
    keys.get(normalized).copied().unwrap_or_else(|| {
        panic!("normalized projected name is absent from snapshot-owned name indexes: {normalized}")
    })
}

fn project_name<'a>(
    handle: HbkFactReadHandle<'a>,
    name: HbkNameView<'a>,
) -> (&'a str, Option<&'a str>) {
    let primary_id = name.primary();
    let primary_raw = handle.string(primary_id);
    let alias_raw = name.alias().map(|alias| handle.string(alias));
    (primary_raw, alias_raw)
}

fn legacy_rows_from_canonical<Scope: Copy>(
    rows: &[CanonicalRow<'_, Scope>],
    keys: &HashMap<&str, StringId>,
) -> Vec<LegacyCanonicalRow<Scope>> {
    rows.iter()
        .map(|row| LegacyCanonicalRow {
            scope: row.scope,
            primary: project_key(keys, &row.primary),
            alias: row
                .alias
                .as_ref()
                .map(|alias| project_key(keys, &alias.key)),
            entity: row.entity,
        })
        .collect()
}

fn retained_normalized_names<'a>(
    types: &'a CanonicalFamily<()>,
    callables: &'a CanonicalFamily<Option<u32>>,
    properties: &'a CanonicalFamily<Option<u32>>,
) -> BTreeSet<&'a str> {
    types
        .rows
        .iter()
        .flat_map(|row| {
            std::iter::once(row.primary.as_str())
                .chain(row.alias.iter().map(|alias| alias.key.as_str()))
        })
        .chain(callables.rows.iter().flat_map(|row| {
            std::iter::once(row.primary.as_str())
                .chain(row.alias.iter().map(|alias| alias.key.as_str()))
        }))
        .chain(properties.rows.iter().flat_map(|row| {
            std::iter::once(row.primary.as_str())
                .chain(row.alias.iter().map(|alias| alias.key.as_str()))
        }))
        .collect()
}

fn missing_raw_names(
    snapshot: &HbkFactSnapshot,
    types: &CanonicalFamily<()>,
    callables: &CanonicalFamily<Option<u32>>,
    properties: &CanonicalFamily<Option<u32>>,
) -> MissingNames {
    let used = retained_normalized_names(types, callables, properties);
    let mut ascii = None;
    let mut non_ascii = None;

    for raw in snapshot
        .fact_ids
        .iter()
        .map(|entry| snapshot.string(entry.key))
        .chain(
            snapshot
                .query_table_names
                .iter()
                .map(|entry| snapshot.string(entry.key)),
        )
        .chain(
            snapshot
                .language_names
                .iter()
                .map(|entry| snapshot.string(entry.key)),
        )
    {
        let normalized = normalize_lookup_key(raw);
        if used.contains(normalized.as_str()) {
            continue;
        }
        if routes_to_alias(&normalized) {
            ascii.get_or_insert_with(|| (raw.to_string(), normalized));
        } else {
            non_ascii.get_or_insert_with(|| (raw.to_string(), normalized));
        }
        if ascii.is_some() && non_ascii.is_some() {
            break;
        }
    }

    let (ascii_raw, ascii_normalized) =
        ascii.expect("snapshot must contain an ASCII-routed raw missing name");
    let (non_ascii_raw, non_ascii_normalized) =
        non_ascii.expect("snapshot must contain a primary-routed raw missing name");
    MissingNames {
        ascii_raw,
        non_ascii_raw,
        ascii_normalized,
        non_ascii_normalized,
    }
}

fn project_corpus(snapshot: &HbkFactSnapshot) -> ProjectedCorpus<'_> {
    let handle = snapshot.worker_handle();
    let counts = snapshot.counts();
    let keys = snapshot_prepared_keys(snapshot);
    let mut raw_name_count = 0;
    let mut raw_name_bytes = 0;
    let mut source_types = Vec::with_capacity(counts.platform_types + counts.enums);
    for ordinal in 0..counts.platform_types {
        let view = handle.platform_type(HbkPlatformTypeId(
            u32::try_from(ordinal).expect("platform type count overflowed u32"),
        ));
        let (primary_raw, alias_raw) = project_name(handle, view.name());
        raw_name_count += 1 + usize::from(alias_raw.is_some());
        raw_name_bytes += primary_raw.len() + alias_raw.map_or(0, str::len);
        source_types.push(SourceTypeRow {
            primary_raw,
            alias_raw,
        });
    }
    for ordinal in 0..counts.enums {
        let view = handle.enum_fact(HbkEnumId(
            u32::try_from(ordinal).expect("enum count overflowed u32"),
        ));
        let (primary_raw, alias_raw) = project_name(handle, view.name());
        raw_name_count += 1 + usize::from(alias_raw.is_some());
        raw_name_bytes += primary_raw.len() + alias_raw.map_or(0, str::len);
        source_types.push(SourceTypeRow {
            primary_raw,
            alias_raw,
        });
    }
    let (types, source_to_type) = canonicalize_types(&source_types);
    let legacy_types = legacy_rows_from_canonical(&types.rows, &keys);

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
        let (primary_raw, alias_raw) = project_name(handle, view.name());
        raw_name_count += 1 + usize::from(alias_raw.is_some());
        raw_name_bytes += primary_raw.len() + alias_raw.map_or(0, str::len);
        source_callables.push(SourceMemberRow {
            owner_source: view.owner().map(|owner| owner.0 as usize),
            primary_raw,
            alias_raw,
        });
    }
    let callables = canonicalize_members(&source_callables, &source_to_type);
    let legacy_callables = legacy_rows_from_canonical(&callables.rows, &keys);

    let mut source_properties = Vec::with_capacity(counts.type_members + counts.enum_values);
    for ordinal in 0..counts.type_members {
        let view = handle.type_member(HbkTypeMemberId(
            u32::try_from(ordinal).expect("type member count overflowed u32"),
        ));
        if view.kind() != HbkTypeMemberKind::Property {
            continue;
        }
        let (primary_raw, alias_raw) = project_name(handle, view.name());
        raw_name_count += 1 + usize::from(alias_raw.is_some());
        raw_name_bytes += primary_raw.len() + alias_raw.map_or(0, str::len);
        source_properties.push(SourceMemberRow {
            owner_source: Some(view.owner().0 as usize),
            primary_raw,
            alias_raw,
        });
    }
    for id in handle.global_fact_ids() {
        let view = handle.global_fact(id);
        if view.kind() != HbkGlobalFactKind::Property {
            continue;
        }
        let (primary_raw, alias_raw) = project_name(handle, view.name());
        raw_name_count += 1 + usize::from(alias_raw.is_some());
        raw_name_bytes += primary_raw.len() + alias_raw.map_or(0, str::len);
        source_properties.push(SourceMemberRow {
            owner_source: None,
            primary_raw,
            alias_raw,
        });
    }
    for ordinal in 0..counts.enum_values {
        let view = handle.enum_value(HbkEnumValueId(
            u32::try_from(ordinal).expect("enum value count overflowed u32"),
        ));
        let (primary_raw, alias_raw) = project_name(handle, view.name());
        raw_name_count += 1 + usize::from(alias_raw.is_some());
        raw_name_bytes += primary_raw.len() + alias_raw.map_or(0, str::len);
        source_properties.push(SourceMemberRow {
            owner_source: Some(counts.platform_types + view.owner().0 as usize),
            primary_raw,
            alias_raw,
        });
    }
    let properties = canonicalize_members(&source_properties, &source_to_type);
    let legacy_properties = legacy_rows_from_canonical(&properties.rows, &keys);
    let missing = missing_raw_names(snapshot, &types, &callables, &properties);

    ProjectedCorpus {
        missing,
        raw_name_count,
        raw_name_bytes,
        types,
        legacy_types,
        callables,
        legacy_callables,
        properties,
        legacy_properties,
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
        "corpus platform_version={} locale={} extraction_schema={} source_hbk={} snapshot_strings={} raw_names={} raw_name_bytes={} missing_ascii_raw={:?} missing_ascii_normalized={:?} missing_ascii_absent_matches=0 missing_non_ascii_raw={:?} missing_non_ascii_normalized={:?} missing_non_ascii_absent_matches=0",
        FROZEN_PLATFORM_VERSION,
        metadata.locale,
        metadata.source_extraction_schema_version,
        metadata.source_hbk,
        snapshot.counts().strings,
        corpus.raw_name_count,
        corpus.raw_name_bytes,
        corpus.missing.ascii_raw,
        corpus.missing.ascii_normalized,
        corpus.missing.non_ascii_raw,
        corpus.missing.non_ascii_normalized,
    );
    print_family_corpus("type", &corpus.types);
    print_family_corpus("callable", &corpus.callables);
    print_family_corpus("property", &corpus.properties);

    let types = run_family::<(), (), (), LegacyTypeId, TypeId>(
        "type",
        &corpus.types.rows,
        &corpus.legacy_types,
        &corpus.missing,
        &snapshot,
        |()| (),
        |()| (),
    );
    let member_scope = |scope| map_member_scope(scope, &corpus.types.rows, &types);
    run_family::<Option<u32>, OwnerId, OwnerId, LegacyCallableId, CallableId>(
        "callable",
        &corpus.callables.rows,
        &corpus.legacy_callables,
        &corpus.missing,
        &snapshot,
        member_scope,
        member_scope,
    );
    run_family::<Option<u32>, OwnerId, OwnerId, LegacyPropertyId, PropertyId>(
        "property",
        &corpus.properties.rows,
        &corpus.legacy_properties,
        &corpus.missing,
        &snapshot,
        member_scope,
        member_scope,
    );
}
