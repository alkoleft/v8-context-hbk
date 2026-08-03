use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Cursor, Read, Write};
use std::ops::Range;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use memmap2::Mmap;
use sha2::{Digest, Sha256};

use super::binary_cache::{BinaryReader, BinaryValue, BinaryWriter};
use super::indexes::*;
use super::*;

const MAGIC: &[u8; 8] = b"HBKFX1\0\0";
const LAYOUT_VERSION: u32 = 1;
const LAYOUT_FLAGS: u32 = 1;
const SUPPORTED_PROVIDER_SCHEMA: u32 = INDEX_SCHEMA_VERSION;
const SUPPORTED_EXTRACTION_SCHEMA: u32 = 11;
const BACKEND_ID: &str = "x1-global-soa-type-hash-member-aos";
const RECORD_LAYOUT: &str = "fixed-head-range-x1-global-soa-type-hash-member-aos-provenance-v1";
const SECTION_COUNT: usize = 78;
const HEADER_LEN: usize = 216;
const DIRECTORY_ENTRY_LEN: usize = 16;
const SECTION_ALIGNMENT: usize = 8;
const MAX_ARTIFACT_BYTES: u64 = 256 * 1024 * 1024;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const X1_SLOT_LOCK_FILE: &str = "snapshot.lock";
const X1_SLOT_GENERATIONS_DIR: &str = "generations";
const X1_SLOT_CURRENT_FILE: &str = "current";
const X1_GENERATION_PREFIX: &str = "generation-";
const X1_GENERATION_SUFFIX: &str = ".x1";
const SHA256_HEX_LEN: usize = 64;
const X1_CURRENT_POINTER_LEN: usize =
    X1_GENERATION_PREFIX.len() + SHA256_HEX_LEN + X1_GENERATION_SUFFIX.len() + 1;

static X1_PUBLICATION_NONCE: AtomicU64 = AtomicU64::new(0);

fn x1_normalize_lookup_key(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    normalized.extend(
        value
            .chars()
            .filter(|character| !character.is_whitespace())
            .flat_map(char::to_lowercase),
    );
    normalized
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct X1ArtifactIdentity {
    source_path: String,
    source_bytes: u64,
    source_sha256: String,
    locale: String,
    source_locale: String,
    platform_version: String,
    provider_identity: String,
    provider_bytes: u64,
    provider_sha256: String,
    provider_schema: u32,
    extraction_schema: u32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct X1RuntimeExpectation {
    platform_version: String,
    locale: String,
    source_locale: String,
    source_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkFactSnapshotArtifactWriteReport {
    pub artifact_bytes: u64,
    pub platform_version: String,
    pub source_sha256: String,
    pub provider_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkFactSnapshotArtifactPublicationReport {
    pub artifact_bytes: u64,
    pub artifact_sha256: String,
    pub generation_file_name: String,
    pub platform_version: String,
    pub source_sha256: String,
    pub provider_sha256: String,
    pub reused_existing_generation: bool,
}

#[allow(dead_code)]
struct X1MappedGeneration {
    mmap: Mmap,
    _file: File,
    sections: Vec<Section>,
    counts: HbkFactSnapshotCounts,
    source_locale: StringId,
    identity: X1ArtifactIdentity,
}

#[allow(dead_code)]
impl X1MappedGeneration {
    /// # Safety
    ///
    /// The caller must guarantee that the explicit generation file cannot be
    /// modified or truncated for the returned owner's lifetime. The stable-slot
    /// owner upholds this with its shared reader lock; direct test callers own
    /// and keep their isolated read-only generation unchanged.
    unsafe fn open(path: &Path, expected: &X1RuntimeExpectation) -> Result<Self, SearchError> {
        let before = fs::symlink_metadata(path).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        validate_stable_regular_file_metadata(path, &before, "X1 generation")?;
        validate_generation_metadata(path, &before)?;

        let file = File::open(path).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let file_metadata = file.metadata().map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        validate_same_file(
            path,
            &before,
            &file_metadata,
            "X1 generation changed while opening",
        )?;
        validate_generation_metadata(path, &file_metadata)?;
        if file_metadata.len() != before.len() {
            return Err(snapshot_artifact_invalid(
                path,
                "X1 artifact size changed before mapping",
            ));
        }

        // SAFETY: The caller guarantees that this explicit generation cannot
        // be modified or truncated while the returned owner exists. The file
        // is opened read-only, no mutable mapping is created, and typed access
        // remains unavailable until the full byte validator succeeds.
        let mmap = unsafe { Mmap::map(&file) }.map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if mmap.len() as u64 != file_metadata.len() {
            return Err(snapshot_artifact_invalid(
                path,
                "X1 mapped artifact size does not match file metadata",
            ));
        }

        let (sections, counts, source_locale, identity) = validate_mmap_expected(&mmap, None)
            .map_err(|source| SearchError::SnapshotArtifact {
                path: path.to_path_buf(),
                source: artifact_error_from_io(source),
            })?;
        validate_runtime_expectation(path, expected, &identity)?;
        let source_locale = source_locale.ok_or_else(|| {
            snapshot_artifact_invalid(path, "X1 source locale dictionary reference is missing")
        })?;
        let after = file.metadata().map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        validate_generation_metadata(path, &after)?;
        validate_same_file(
            path,
            &file_metadata,
            &after,
            "X1 artifact changed while validating mapping",
        )?;
        let path_after = fs::symlink_metadata(path).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        validate_stable_regular_file_metadata(path, &path_after, "X1 generation")?;
        validate_same_file(
            path,
            &file_metadata,
            &path_after,
            "X1 generation path changed while opening",
        )?;

        Ok(Self {
            mmap,
            _file: file,
            sections,
            counts,
            source_locale,
            identity,
        })
    }

    fn artifact_len(&self) -> usize {
        self.mmap.len()
    }

    fn read_handle(&self) -> X1MappedReadHandle<'_> {
        X1MappedReadHandle { generation: self }
    }

    fn vector(&self, section: S) -> VectorView<'_> {
        VectorView::new(section_bytes(&self.mmap, &self.sections, section))
            .expect("X1 generation was fully validated before access")
    }

    fn record<T: BinaryValue>(&self, section: S, index: usize) -> T {
        self.vector(section)
            .get(index)
            .expect("X1 record was fully validated before access")
    }

    fn records<T: BinaryValue>(&self, section: S, range: X1Range) -> X1RecordIter<'_, T> {
        X1RecordIter::new(self.vector(section), range)
    }
}

/// Owns the shared slot lock after the mapped generation so Rust drops the
/// mapping before releasing the lock. This remains crate-private until the
/// catalog/runtime migration in OpenSpec task 4.4.
#[allow(dead_code)]
pub(super) struct X1StableSlotGeneration {
    generation: X1MappedGeneration,
    _shared_lock: File,
}

#[allow(dead_code)]
impl X1StableSlotGeneration {
    fn open(slot_path: &Path, expected: &X1RuntimeExpectation) -> Result<Self, SearchError> {
        reject_symlink_path_components(slot_path)?;
        validate_stable_directory(slot_path, "X1 snapshot slot")?;
        let generations_path = slot_path.join(X1_SLOT_GENERATIONS_DIR);
        validate_stable_directory(&generations_path, "X1 generations directory")?;

        let lock_path = slot_path.join(X1_SLOT_LOCK_FILE);
        let lock_file = open_stable_regular_file(&lock_path, false, "X1 slot lock")?;
        File::lock_shared(&lock_file).map_err(|source| SearchError::Io {
            path: lock_path.clone(),
            source,
        })?;
        validate_open_file_path(&lock_path, &lock_file, "X1 slot lock changed after locking")?;

        let current_path = slot_path.join(X1_SLOT_CURRENT_FILE);
        let generation_file_name = read_current_pointer(&current_path)?;
        let generation_path = generations_path.join(generation_file_name);
        validate_generation_content_address(&generation_path)?;

        // SAFETY: The stable slot is a trusted service-data boundary. This
        // owner holds the slot's shared lock until after `generation` is
        // dropped, all cooperating publishers require its exclusive lock,
        // generations are never modified in place, and `open` verifies the
        // generation's non-symlink stable inode before and after mapping.
        let generation = unsafe { X1MappedGeneration::open(&generation_path, expected) }?;

        Ok(Self {
            generation,
            _shared_lock: lock_file,
        })
    }

    fn read_handle(&self) -> X1MappedReadHandle<'_> {
        self.generation.read_handle()
    }

    fn artifact_len(&self) -> usize {
        self.generation.artifact_len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum X1PublicationFailurePoint {
    BeforeGeneration,
    GenerationPublished,
    CurrentPublished,
}

#[derive(Debug, Clone)]
struct X1PublicationOptions {
    nonce: String,
    fail_at: Option<X1PublicationFailurePoint>,
    #[cfg(test)]
    lock_hook: Option<X1PublicationLockHook>,
}

#[cfg(test)]
#[derive(Debug, Clone)]
struct X1PublicationLockHook {
    acquired: std::sync::Arc<std::sync::Barrier>,
    release: std::sync::Arc<std::sync::Barrier>,
}

impl X1PublicationOptions {
    fn production() -> Self {
        let sequence = X1_PUBLICATION_NONCE.fetch_add(1, AtomicOrdering::Relaxed);
        Self {
            nonce: format!("{}-{sequence}", std::process::id()),
            fail_at: None,
            #[cfg(test)]
            lock_hook: None,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum X1AvailabilityMode {
    Any,
    All,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct X1AvailabilityFilter {
    requested_mask: u16,
    mode: X1AvailabilityMode,
}

#[allow(dead_code)]
impl X1AvailabilityFilter {
    fn from_codes<'a>(
        codes: impl IntoIterator<Item = &'a str>,
        mode: X1AvailabilityMode,
    ) -> io::Result<Self> {
        let mut requested_mask = 0_u16;
        for code in codes {
            requested_mask |= x1_context_code_bit(code)
                .ok_or_else(|| invalid_data("unknown X1 availability context filter"))?;
        }
        Ok(Self {
            requested_mask,
            mode,
        })
    }

    #[inline]
    fn includes(self, availability_word: u16) -> bool {
        if availability_word & X1_HAS_EXPLICIT_DECLARATION == 0 {
            return true;
        }
        let available = availability_word & X1_CONTEXT_BITS;
        match self.mode {
            X1AvailabilityMode::Any => available & self.requested_mask != 0,
            X1AvailabilityMode::All => available & self.requested_mask == self.requested_mask,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct X1MappedReadHandle<'a> {
    generation: &'a X1MappedGeneration,
}

#[allow(dead_code)]
impl<'a> X1MappedReadHandle<'a> {
    fn string(self, id: StringId) -> &'a str {
        self.generation
            .vector(S::Strings)
            .get_str(id.0 as usize)
            .expect("X1 string was fully validated before access")
    }

    fn source_locale(self) -> &'a str {
        self.string(self.generation.source_locale)
    }

    fn string_id(self, value: &str) -> Option<StringId> {
        let order = self.generation.vector(S::StringOrder);
        binary_search_view(&order, |id: StringId| self.string(id).cmp(value))
            .ok()
            .map(|index| {
                order
                    .get::<StringId>(index)
                    .expect("X1 string order was fully validated before access")
            })
    }

    fn global_fact_ids(self) -> impl ExactSizeIterator<Item = HbkGlobalFactId> + 'a {
        (0..self.generation.counts.globals).map(|index| HbkGlobalFactId(index as u32))
    }

    fn query_table_ids(self) -> impl ExactSizeIterator<Item = HbkQueryTableId> + 'a {
        (0..self.generation.counts.query_tables).map(|index| HbkQueryTableId(index as u32))
    }

    fn query_field_ids(self) -> impl ExactSizeIterator<Item = HbkQueryFieldId> + 'a {
        (0..self.generation.counts.query_fields).map(|index| HbkQueryFieldId(index as u32))
    }

    fn query_parameter_ids(self) -> impl ExactSizeIterator<Item = HbkQueryParameterId> + 'a {
        (0..self.generation.counts.query_parameters).map(|index| HbkQueryParameterId(index as u32))
    }

    fn facts_by_id(self, id: &str) -> X1LookupValueIter<'a, IdLookup<HbkFactRef>, HbkFactRef> {
        self.lookup_id_values(S::FactIds, id, |candidate| candidate.value)
    }

    fn platform_type_by_id(self, id: &str) -> Option<HbkPlatformTypeId> {
        self.lookup_id_one(
            S::PlatformTypeIds,
            id,
            |candidate: IdLookup<HbkPlatformTypeId>| candidate.value,
        )
    }

    fn platform_types_by_name(
        self,
        name: &str,
    ) -> X1LookupValueIter<'a, NameLookup<HbkPlatformTypeId>, HbkPlatformTypeId> {
        let normalized = x1_normalize_lookup_key(name);
        self.platform_types_by_normalized_name(&normalized)
    }

    fn platform_types_by_normalized_name(
        self,
        normalized: &str,
    ) -> X1LookupValueIter<'a, NameLookup<HbkPlatformTypeId>, HbkPlatformTypeId> {
        let buckets = self.generation.vector(S::PlatformTypeNameHash);
        let names = self.generation.vector(S::PlatformTypeNames);
        let strings = self.generation.vector(S::Strings);
        let hash = x1_hash_key(normalized);
        let bucket_index = x1_probe_bucket(&buckets, &names, &strings, hash, normalized)
            .expect("X1 type-name hash was fully validated before access");
        let bucket = buckets
            .get::<X1TypeNameHashBucket>(bucket_index)
            .expect("X1 type-name hash was fully validated before access");
        let range = if bucket.count == 0 {
            0..0
        } else {
            let start = bucket.start as usize;
            start..start + bucket.count as usize
        };
        X1LookupValueIter::new(names, range, |candidate| candidate.value)
    }

    fn platform_types_by_template_key(
        self,
        family: &str,
        variant: &str,
    ) -> X1LookupValueIter<'a, TypeTemplateLookup<HbkPlatformTypeId>, HbkPlatformTypeId> {
        let index = self.generation.vector(S::PlatformTypeTemplates);
        let range = matching_range_view(
            &index,
            |candidate: TypeTemplateLookup<HbkPlatformTypeId>| {
                self.string(candidate.family)
                    .cmp(family)
                    .then_with(|| self.string(candidate.variant).cmp(variant))
            },
        );
        X1LookupValueIter::new(index, range, |candidate| candidate.value)
    }

    fn members_of_type(self, owner: HbkPlatformTypeId) -> X1RecordIter<'a, HbkTypeMemberId> {
        self.csr_values(
            S::MembersByOwnerKeys,
            S::MembersByOwnerOffsets,
            S::MembersByOwnerValues,
            owner,
        )
    }

    fn member_by_owner_name(
        self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> X1LookupValueIter<'a, OwnerNameLookup<HbkPlatformTypeId, HbkTypeMemberId>, HbkTypeMemberId>
    {
        let normalized = x1_normalize_lookup_key(name);
        self.member_by_owner_normalized_name(owner, &normalized)
    }

    fn member_by_owner_normalized_name(
        self,
        owner: HbkPlatformTypeId,
        normalized: &str,
    ) -> X1LookupValueIter<'a, OwnerNameLookup<HbkPlatformTypeId, HbkTypeMemberId>, HbkTypeMemberId>
    {
        self.lookup_owner_name_values(S::MembersByOwnerName, owner, normalized, |candidate| {
            candidate.value
        })
    }

    fn member_by_owner_name_kind(
        self,
        owner: HbkPlatformTypeId,
        name: &str,
        kind: Option<HbkTypeMemberKind>,
    ) -> X1MemberLookupIter<'a> {
        let normalized = x1_normalize_lookup_key(name);
        let Some(kind) = kind else {
            return X1MemberLookupIter::Name(
                self.member_by_owner_normalized_name(owner, &normalized),
            );
        };
        let index = self.generation.vector(S::MembersByOwnerNameKind);
        let range = matching_range_view(&index, |candidate: MemberNameKindLookup| {
            candidate
                .owner
                .cmp(&owner)
                .then_with(|| self.string(candidate.key).cmp(&normalized))
                .then_with(|| candidate.kind.cmp(&Some(kind)))
        });
        X1MemberLookupIter::NameKind(X1LookupValueIter::new(index, range, |candidate| {
            candidate.value
        }))
    }

    fn callables_of_type(self, owner: HbkPlatformTypeId) -> X1RecordIter<'a, HbkCallableId> {
        self.csr_values(
            S::CallablesByOwnerKeys,
            S::CallablesByOwnerOffsets,
            S::CallablesByOwnerValues,
            owner,
        )
    }

    fn callable_by_owner_name(
        self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> X1LookupValueIter<'a, OwnerNameLookup<HbkPlatformTypeId, HbkCallableId>, HbkCallableId>
    {
        let normalized = x1_normalize_lookup_key(name);
        self.lookup_owner_name_values(S::CallablesByOwnerName, owner, &normalized, |candidate| {
            candidate.value
        })
    }

    fn constructors_of_type(self, owner: HbkPlatformTypeId) -> X1RecordIter<'a, HbkCallableId> {
        self.csr_values(
            S::ConstructorsByTypeKeys,
            S::ConstructorsByTypeOffsets,
            S::ConstructorsByTypeValues,
            owner,
        )
    }

    fn globals_by_name(
        self,
        name: &str,
    ) -> X1LookupValueIter<'a, NameLookup<HbkGlobalFactId>, HbkGlobalFactId> {
        let normalized = x1_normalize_lookup_key(name);
        self.lookup_name_values(S::GlobalNames, &normalized, |candidate| candidate.value)
    }

    fn globals_by_domain_name_kind(
        self,
        domain: HbkLanguageDomain,
        name: &str,
        kind: Option<HbkGlobalFactKind>,
    ) -> X1LookupValueIter<'a, GlobalNameKindLookup, HbkGlobalFactId> {
        let normalized = x1_normalize_lookup_key(name);
        let index = self.generation.vector(S::GlobalsByDomainNameKind);
        let range = matching_range_view(&index, |candidate: GlobalNameKindLookup| {
            let ordering = candidate
                .domain
                .cmp(&domain)
                .then_with(|| self.string(candidate.key).cmp(&normalized));
            kind.map_or(ordering, |kind| {
                ordering.then_with(|| candidate.kind.cmp(&Some(kind)))
            })
        });
        X1LookupValueIter::new(index, range, |candidate| candidate.value)
    }

    fn module_events(
        self,
        module_context_key: &str,
    ) -> X1LookupValueIter<'a, OwnerNameLookup<StringId, HbkCallableId>, HbkCallableId> {
        let normalized = x1_normalize_lookup_key(module_context_key);
        let index = self.generation.vector(S::ModuleEventNames);
        let range = matching_range_view(
            &index,
            |candidate: OwnerNameLookup<StringId, HbkCallableId>| {
                self.string(candidate.owner).cmp(&normalized)
            },
        );
        X1LookupValueIter::new(index, range, |candidate| candidate.value)
    }

    fn module_event_by_context_name(
        self,
        module_context_key: &str,
        name: &str,
    ) -> X1LookupValueIter<'a, OwnerNameLookup<StringId, HbkCallableId>, HbkCallableId> {
        let owner = x1_normalize_lookup_key(module_context_key);
        let normalized = x1_normalize_lookup_key(name);
        let index = self.generation.vector(S::ModuleEventNames);
        let range = matching_range_view(
            &index,
            |candidate: OwnerNameLookup<StringId, HbkCallableId>| {
                self.string(candidate.owner)
                    .cmp(&owner)
                    .then_with(|| self.string(candidate.key).cmp(&normalized))
            },
        );
        X1LookupValueIter::new(index, range, |candidate| candidate.value)
    }

    fn module_context_events(
        self,
        domain: HbkLanguageDomain,
        language_key: &str,
        module_kind: &str,
    ) -> X1LookupValueIter<'a, ModuleContextLookup, HbkCallableId> {
        let language_key = x1_normalize_lookup_key(language_key);
        let module_kind = x1_normalize_lookup_key(module_kind);
        let index = self
            .generation
            .vector(S::ModuleContextsByDomainLanguageKind);
        let range = matching_range_view(&index, |candidate: ModuleContextLookup| {
            candidate
                .domain
                .cmp(&domain)
                .then_with(|| self.string(candidate.language_key).cmp(&language_key))
                .then_with(|| self.string(candidate.module_kind).cmp(&module_kind))
        });
        X1LookupValueIter::new(index, range, |candidate| candidate.value)
    }

    fn query_table_by_id(self, id: &str) -> Option<HbkQueryTableId> {
        self.lookup_id_one(
            S::QueryTableIds,
            id,
            |candidate: IdLookup<HbkQueryTableId>| candidate.value,
        )
    }

    fn query_tables_by_name(
        self,
        name: &str,
    ) -> X1LookupValueIter<'a, NameLookup<HbkQueryTableId>, HbkQueryTableId> {
        let normalized = x1_normalize_lookup_key(name);
        self.lookup_name_values(S::QueryTableNames, &normalized, |candidate| candidate.value)
    }

    fn query_tables_by_syntax(
        self,
        syntax: &str,
    ) -> X1LookupValueIter<'a, NameLookup<HbkQueryTableId>, HbkQueryTableId> {
        let normalized = x1_normalize_lookup_key(syntax);
        self.lookup_name_values(S::QueryTableSyntaxNames, &normalized, |candidate| {
            candidate.value
        })
    }

    fn query_tables_by_identifier(
        self,
        identifier: &str,
    ) -> X1LookupValueIter<'a, NameLookup<HbkQueryTableId>, HbkQueryTableId> {
        let normalized = x1_normalize_lookup_key(identifier);
        self.lookup_name_values(S::QueryTableIdentifiers, &normalized, |candidate| {
            candidate.value
        })
    }

    fn query_fields(self, table: HbkQueryTableId) -> X1RecordIter<'a, HbkQueryFieldId> {
        self.csr_values(
            S::QueryFieldsByTableKeys,
            S::QueryFieldsByTableOffsets,
            S::QueryFieldsByTableValues,
            table,
        )
    }

    fn query_fields_by_name(
        self,
        table: HbkQueryTableId,
        name: &str,
    ) -> X1LookupValueIter<'a, OwnerNameLookup<HbkQueryTableId, HbkQueryFieldId>, HbkQueryFieldId>
    {
        let normalized = x1_normalize_lookup_key(name);
        self.lookup_owner_name_values(S::QueryFieldsByTableName, table, &normalized, |candidate| {
            candidate.value
        })
    }

    fn query_parameters(self, table: HbkQueryTableId) -> X1RecordIter<'a, HbkQueryParameterId> {
        self.csr_values(
            S::QueryParametersByTableKeys,
            S::QueryParametersByTableOffsets,
            S::QueryParametersByTableValues,
            table,
        )
    }

    fn query_parameters_by_name(
        self,
        table: HbkQueryTableId,
        name: &str,
    ) -> X1LookupValueIter<
        'a,
        OwnerNameLookup<HbkQueryTableId, HbkQueryParameterId>,
        HbkQueryParameterId,
    > {
        let normalized = x1_normalize_lookup_key(name);
        self.lookup_owner_name_values(
            S::QueryParametersByTableName,
            table,
            &normalized,
            |candidate| candidate.value,
        )
    }

    fn language_fact_by_id(self, id: &str) -> Option<HbkLanguageFactId> {
        self.lookup_id_one(
            S::LanguageIds,
            id,
            |candidate: IdLookup<HbkLanguageFactId>| candidate.value,
        )
    }

    fn language_facts_by_name(
        self,
        name: &str,
    ) -> X1LookupValueIter<'a, NameLookup<HbkLanguageFactId>, HbkLanguageFactId> {
        let normalized = x1_normalize_lookup_key(name);
        self.lookup_name_values(S::LanguageNames, &normalized, |candidate| candidate.value)
    }

    fn enum_by_id(self, id: &str) -> Option<HbkEnumId> {
        self.lookup_id_one(S::EnumIds, id, |candidate: IdLookup<HbkEnumId>| {
            candidate.value
        })
    }

    fn enums_by_name(self, name: &str) -> X1LookupValueIter<'a, NameLookup<HbkEnumId>, HbkEnumId> {
        let normalized = x1_normalize_lookup_key(name);
        self.lookup_name_values(S::EnumNames, &normalized, |candidate| candidate.value)
    }

    fn enum_value_by_id(self, id: &str) -> Option<HbkEnumValueId> {
        self.lookup_id_one(
            S::EnumValueIds,
            id,
            |candidate: IdLookup<HbkEnumValueId>| candidate.value,
        )
    }

    fn enum_values(self, owner: HbkEnumId) -> X1RecordIter<'a, HbkEnumValueId> {
        self.csr_values(
            S::EnumValuesByEnumKeys,
            S::EnumValuesByEnumOffsets,
            S::EnumValuesByEnumValues,
            owner,
        )
    }

    fn enum_values_by_name(
        self,
        owner: HbkEnumId,
        name: &str,
    ) -> X1LookupValueIter<'a, OwnerNameLookup<HbkEnumId, HbkEnumValueId>, HbkEnumValueId> {
        let normalized = x1_normalize_lookup_key(name);
        self.lookup_owner_name_values(S::EnumValuesByEnumName, owner, &normalized, |candidate| {
            candidate.value
        })
    }

    fn availability_contexts(self, fact: HbkFactRef) -> X1RecordIter<'a, StringId> {
        self.csr_values(
            S::AvailabilityByFactKeys,
            S::AvailabilityByFactOffsets,
            S::AvailabilityByFactValues,
            fact,
        )
    }

    fn available_since(self, fact: HbkFactRef) -> Option<StringId> {
        let index = self.generation.vector(S::AvailabilitySinceByFact);
        binary_search_view(&index, |candidate: FactStringLookup| {
            candidate.fact.cmp(&fact)
        })
        .ok()
        .map(|position| {
            index
                .get::<FactStringLookup>(position)
                .expect("X1 available-since index was fully validated before access")
                .value
        })
    }

    fn relations_by_source_kind(
        self,
        source: HbkFactRef,
        kind: &str,
    ) -> X1RecordIter<'a, HbkFactRef> {
        let normalized = x1_normalize_lookup_key(kind);
        let Some(kind) = self.string_id(&normalized) else {
            return X1RecordIter::empty(self.generation.vector(S::RelationsBySourceKindValues));
        };
        self.csr_values(
            S::RelationsBySourceKindKeys,
            S::RelationsBySourceKindOffsets,
            S::RelationsBySourceKindValues,
            RelationLookupKey { source, kind },
        )
    }

    fn lookup_id_one<Record, Value>(
        self,
        section: S,
        key: &str,
        value: fn(Record) -> Value,
    ) -> Option<Value>
    where
        Record: BinaryValue + Copy + X1IdLookupRecord,
        Value: Copy,
    {
        let index = self.generation.vector(section);
        binary_search_view(&index, |candidate: Record| {
            self.string(candidate.key()).cmp(key)
        })
        .ok()
        .map(|position| {
            value(
                index
                    .get::<Record>(position)
                    .expect("X1 ID lookup was fully validated before access"),
            )
        })
    }

    fn lookup_id_values<Record, Value>(
        self,
        section: S,
        key: &str,
        value: fn(Record) -> Value,
    ) -> X1LookupValueIter<'a, Record, Value>
    where
        Record: BinaryValue + Copy + X1IdLookupRecord,
        Value: Copy,
    {
        let index = self.generation.vector(section);
        let range = matching_range_view(&index, |candidate: Record| {
            self.string(candidate.key()).cmp(key)
        });
        X1LookupValueIter::new(index, range, value)
    }

    fn lookup_name_values<Record, Value>(
        self,
        section: S,
        normalized: &str,
        value: fn(Record) -> Value,
    ) -> X1LookupValueIter<'a, Record, Value>
    where
        Record: BinaryValue + Copy + X1NameLookupRecord,
        Value: Copy,
    {
        let index = self.generation.vector(section);
        let range = matching_range_view(&index, |candidate: Record| {
            self.string(candidate.key()).cmp(normalized)
        });
        X1LookupValueIter::new(index, range, value)
    }

    fn lookup_owner_name_values<Record, Owner, Value>(
        self,
        section: S,
        owner: Owner,
        normalized: &str,
        value: fn(Record) -> Value,
    ) -> X1LookupValueIter<'a, Record, Value>
    where
        Record: BinaryValue + Copy + X1OwnerNameLookupRecord<Owner>,
        Owner: Copy + Ord,
        Value: Copy,
    {
        let index = self.generation.vector(section);
        let range = matching_range_view(&index, |candidate: Record| {
            candidate
                .owner()
                .cmp(&owner)
                .then_with(|| self.string(candidate.key()).cmp(normalized))
        });
        X1LookupValueIter::new(index, range, value)
    }

    fn csr_values<Key: BinaryValue + Copy + Ord, Value: BinaryValue + Copy>(
        self,
        keys_section: S,
        offsets_section: S,
        values_section: S,
        key: Key,
    ) -> X1RecordIter<'a, Value> {
        let keys = self.generation.vector(keys_section);
        let offsets = self.generation.vector(offsets_section);
        let values = self.generation.vector(values_section);
        let Ok(position) = binary_search_view(&keys, |candidate: Key| candidate.cmp(&key)) else {
            return X1RecordIter::empty(values);
        };
        let start = offsets
            .get::<u32>(position)
            .expect("X1 CSR offsets were fully validated before access")
            as usize;
        let end = offsets
            .get::<u32>(position + 1)
            .expect("X1 CSR offsets were fully validated before access") as usize;
        X1RecordIter::from_bounds(values, start, end)
    }

    fn platform_type(self, id: HbkPlatformTypeId) -> X1PlatformTypeView<'a> {
        X1PlatformTypeView {
            handle: self,
            head: self.generation.record(S::PlatformTypes, id.0 as usize),
        }
    }

    fn type_member(self, id: HbkTypeMemberId) -> X1TypeMemberView<'a> {
        X1TypeMemberView {
            handle: self,
            head: self.generation.record(S::TypeMembers, id.0 as usize),
        }
    }

    fn callable(self, id: HbkCallableId) -> X1CallableView<'a> {
        X1CallableView {
            handle: self,
            head: self.generation.record(S::Callables, id.0 as usize),
        }
    }

    fn global(self, id: HbkGlobalFactId) -> X1GlobalFactView<'a> {
        X1GlobalFactView {
            handle: self,
            head: self.generation.record(S::Globals, id.0 as usize),
        }
    }

    fn query_table(self, id: HbkQueryTableId) -> X1QueryTableView<'a> {
        X1QueryTableView {
            handle: self,
            head: self.generation.record(S::QueryTables, id.0 as usize),
        }
    }

    fn query_field(self, id: HbkQueryFieldId) -> X1QueryFieldView<'a> {
        X1QueryFieldView {
            handle: self,
            head: self.generation.record(S::QueryFields, id.0 as usize),
        }
    }

    fn query_parameter(self, id: HbkQueryParameterId) -> X1QueryParameterView<'a> {
        X1QueryParameterView {
            handle: self,
            head: self.generation.record(S::QueryParameters, id.0 as usize),
        }
    }

    fn language_fact(self, id: HbkLanguageFactId) -> X1LanguageFactView<'a> {
        X1LanguageFactView {
            handle: self,
            head: self.generation.record(S::LanguageFacts, id.0 as usize),
        }
    }

    fn enum_fact(self, id: HbkEnumId) -> X1EnumView {
        X1EnumView {
            head: self.generation.record(S::Enums, id.0 as usize),
        }
    }

    fn enum_value(self, id: HbkEnumValueId) -> X1EnumValueView {
        X1EnumValueView {
            head: self.generation.record(S::EnumValues, id.0 as usize),
        }
    }

    fn source(self, fact: HbkFactRef) -> Option<X1FactSourceView> {
        let sources = self.generation.vector(S::SourceByFact);
        binary_search_view(&sources, |candidate: FactSourceLookup| {
            candidate.fact.cmp(&fact)
        })
        .ok()
        .map(|index| X1FactSourceView {
            source: sources
                .get::<FactSourceLookup>(index)
                .expect("X1 source lookup was fully validated before access")
                .source,
        })
    }

    fn filtered_globals(
        self,
        filter: X1AvailabilityFilter,
        kind: Option<HbkGlobalFactKind>,
    ) -> X1FilteredGlobalIter<'a> {
        X1FilteredGlobalIter {
            locators: self.generation.vector(S::GlobalAvailabilityHot),
            masks: self.generation.vector(S::GlobalAvailabilityMasks),
            kinds: self.generation.vector(S::GlobalAvailabilityKinds),
            index: 0,
            filter,
            kind,
        }
    }

    fn filtered_members(
        self,
        owner: HbkPlatformTypeId,
        filter: X1AvailabilityFilter,
        kind: Option<HbkTypeMemberKind>,
    ) -> X1FilteredMemberIter<'a> {
        let range = self
            .generation
            .vector(S::TypeMemberRanges)
            .get::<X1TypeMemberRangeHot>(owner.0 as usize)
            .expect("X1 member owner range was fully validated before access");
        X1FilteredMemberIter {
            hot: self.generation.vector(S::MemberAvailabilityHot),
            index: range.member_start as usize,
            end: (range.member_start + range.member_count) as usize,
            filter,
            kind,
        }
    }
}

#[allow(dead_code)]
struct X1RecordIter<'a, T> {
    view: VectorView<'a>,
    index: usize,
    end: usize,
    marker: std::marker::PhantomData<T>,
}

impl<'a, T> X1RecordIter<'a, T> {
    fn new(view: VectorView<'a>, range: X1Range) -> Self {
        let range = range
            .as_usize()
            .expect("X1 range was fully validated before access");
        Self {
            view,
            index: range.start,
            end: range.end,
            marker: std::marker::PhantomData,
        }
    }

    fn from_bounds(view: VectorView<'a>, start: usize, end: usize) -> Self {
        debug_assert!(start <= end && end <= view.len());
        Self {
            view,
            index: start,
            end,
            marker: std::marker::PhantomData,
        }
    }

    fn empty(view: VectorView<'a>) -> Self {
        Self::from_bounds(view, 0, 0)
    }
}

impl<T: BinaryValue> Iterator for X1RecordIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == self.end {
            return None;
        }
        let index = self.index;
        self.index += 1;
        Some(
            self.view
                .get(index)
                .expect("X1 nested record was fully validated before access"),
        )
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<T: BinaryValue> ExactSizeIterator for X1RecordIter<'_, T> {
    fn len(&self) -> usize {
        self.end - self.index
    }
}

trait X1IdLookupRecord {
    fn key(self) -> StringId;
}

impl<T: Copy> X1IdLookupRecord for IdLookup<T> {
    fn key(self) -> StringId {
        self.key
    }
}

trait X1NameLookupRecord {
    fn key(self) -> StringId;
}

impl<T: Copy> X1NameLookupRecord for NameLookup<T> {
    fn key(self) -> StringId {
        self.key
    }
}

trait X1OwnerNameLookupRecord<Owner> {
    fn owner(self) -> Owner;
    fn key(self) -> StringId;
}

impl<Owner: Copy, Value: Copy> X1OwnerNameLookupRecord<Owner> for OwnerNameLookup<Owner, Value> {
    fn owner(self) -> Owner {
        self.owner
    }

    fn key(self) -> StringId {
        self.key
    }
}

#[allow(dead_code)]
struct X1LookupValueIter<'a, Record, Value> {
    records: X1RecordIter<'a, Record>,
    value: fn(Record) -> Value,
}

impl<'a, Record, Value> X1LookupValueIter<'a, Record, Value> {
    fn new(view: VectorView<'a>, range: Range<usize>, value: fn(Record) -> Value) -> Self {
        Self {
            records: X1RecordIter::from_bounds(view, range.start, range.end),
            value,
        }
    }
}

impl<Record: BinaryValue, Value> Iterator for X1LookupValueIter<'_, Record, Value> {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        self.records.next().map(self.value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<Record: BinaryValue, Value> ExactSizeIterator for X1LookupValueIter<'_, Record, Value> {
    fn len(&self) -> usize {
        self.records.len()
    }
}

#[allow(dead_code)]
enum X1MemberLookupIter<'a> {
    Name(
        X1LookupValueIter<'a, OwnerNameLookup<HbkPlatformTypeId, HbkTypeMemberId>, HbkTypeMemberId>,
    ),
    NameKind(X1LookupValueIter<'a, MemberNameKindLookup, HbkTypeMemberId>),
}

impl Iterator for X1MemberLookupIter<'_> {
    type Item = HbkTypeMemberId;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Name(iter) => iter.next(),
            Self::NameKind(iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for X1MemberLookupIter<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Name(iter) => iter.len(),
            Self::NameKind(iter) => iter.len(),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct X1FilteredGlobalIter<'a> {
    locators: VectorView<'a>,
    masks: VectorView<'a>,
    kinds: VectorView<'a>,
    index: usize,
    filter: X1AvailabilityFilter,
    kind: Option<HbkGlobalFactKind>,
}

impl<'a> Iterator for X1FilteredGlobalIter<'a> {
    type Item = HbkGlobalFactId;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.locators.len() {
            let index = self.index;
            self.index += 1;
            let word = self
                .masks
                .get::<u16>(index)
                .expect("X1 global mask was fully validated before access");
            if !self.filter.includes(word) {
                continue;
            }
            let actual_kind = x1_global_kind_from_tag(
                self.kinds
                    .get::<u8>(index)
                    .expect("X1 global kind was fully validated before access"),
            )
            .expect("X1 global kind was fully validated before access");
            if self.kind.is_some_and(|kind| kind != actual_kind) {
                continue;
            }
            let id = self
                .locators
                .get::<u32>(index)
                .expect("X1 global locator was fully validated before access");
            return Some(HbkGlobalFactId(id));
        }
        None
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
struct X1FilteredMemberIter<'a> {
    hot: VectorView<'a>,
    index: usize,
    end: usize,
    filter: X1AvailabilityFilter,
    kind: Option<HbkTypeMemberKind>,
}

impl<'a> Iterator for X1FilteredMemberIter<'a> {
    type Item = HbkTypeMemberId;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.end {
            let index = self.index;
            self.index += 1;
            let hot = self
                .hot
                .get_x1_available_member_hot(index)
                .expect("X1 member hot record was fully validated before access");
            if !self.filter.includes(hot.availability_word) {
                continue;
            }
            let actual_kind = x1_member_kind_from_tag(hot.kind)
                .expect("X1 member kind was fully validated before access");
            if self.kind.is_some_and(|kind| kind != actual_kind) {
                continue;
            }
            return Some(HbkTypeMemberId(hot.member_id));
        }
        None
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct X1NameView {
    head: X1NameHead,
}

#[allow(dead_code)]
impl X1NameView {
    fn primary(self) -> StringId {
        self.head.primary
    }

    fn alias(self) -> Option<StringId> {
        (self.head.alias != NONE_U32).then_some(StringId(self.head.alias))
    }
}

fn x1_template_key(head: X1TemplateKeyHead) -> Option<HbkPlatformTypeTemplateKey> {
    (head != X1TemplateKeyHead::NONE).then_some(HbkPlatformTypeTemplateKey {
        family: StringId(head.family),
        variant: StringId(head.variant),
    })
}

fn x1_optional_string(value: u32) -> Option<StringId> {
    (value != NONE_U32).then_some(StringId(value))
}

macro_rules! x1_mapped_view {
    ($name:ident, $head:ty) => {
        #[allow(dead_code)]
        #[derive(Clone, Copy)]
        struct $name<'a> {
            handle: X1MappedReadHandle<'a>,
            head: $head,
        }
    };
}

x1_mapped_view!(X1PlatformTypeView, X1PlatformTypeHead);
x1_mapped_view!(X1MetadataTemplateView, X1MetadataTemplateHead);
x1_mapped_view!(X1TypeMemberView, X1TypeMemberHead);
x1_mapped_view!(X1CallableView, X1CallableHead);
x1_mapped_view!(X1SignatureView, X1SignatureHead);
x1_mapped_view!(X1ParameterView, X1ParameterHead);
x1_mapped_view!(X1GlobalFactView, X1GlobalFactHead);
x1_mapped_view!(X1QueryTableView, X1QueryTableHead);
x1_mapped_view!(X1QueryFieldView, X1QueryFieldHead);
x1_mapped_view!(X1QueryParameterView, X1QueryParameterHead);
x1_mapped_view!(X1LanguageFactView, X1LanguageFactHead);
x1_mapped_view!(X1TypeRefView, X1TypeRefHead);
x1_mapped_view!(X1TemplateBindingView, X1TemplateBindingHead);

#[allow(dead_code)]
impl<'a> X1PlatformTypeView<'a> {
    fn id(self) -> StringId {
        self.head.id
    }

    fn name(self) -> X1NameView {
        X1NameView {
            head: self.head.name,
        }
    }

    fn metadata_template(self) -> Option<X1MetadataTemplateView<'a>> {
        (self.head.metadata_template != NONE_U32).then(|| X1MetadataTemplateView {
            handle: self.handle,
            head: self
                .handle
                .generation
                .record(S::MetadataTemplates, self.head.metadata_template as usize),
        })
    }

    fn type_template_key(self) -> Option<HbkPlatformTypeTemplateKey> {
        x1_template_key(self.head.type_template_key)
    }

    fn availability_contexts(self) -> X1RecordIter<'a, StringId> {
        self.handle
            .generation
            .records(S::StringIds, self.head.availability_contexts)
    }
}

#[allow(dead_code)]
impl<'a> X1MetadataTemplateView<'a> {
    fn metadata_kind(self) -> StringId {
        self.head.metadata_kind
    }

    fn template_parameters(self) -> X1RecordIter<'a, StringId> {
        self.handle
            .generation
            .records(S::StringIds, self.head.template_parameters)
    }
}

#[allow(dead_code)]
impl<'a> X1TypeMemberView<'a> {
    fn id(self) -> StringId {
        self.head.id
    }

    fn owner(self) -> HbkPlatformTypeId {
        self.head.owner
    }

    fn kind(self) -> HbkTypeMemberKind {
        self.head.kind
    }

    fn name(self) -> X1NameView {
        X1NameView {
            head: self.head.name,
        }
    }

    fn type_refs(self) -> impl ExactSizeIterator<Item = X1TypeRefView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::TypeRefs, self.head.type_refs)
            .map(move |head| X1TypeRefView { handle, head })
    }

    fn availability_contexts(self) -> X1RecordIter<'a, StringId> {
        self.handle
            .generation
            .records(S::StringIds, self.head.availability_contexts)
    }
}

#[allow(dead_code)]
impl<'a> X1CallableView<'a> {
    fn id(self) -> StringId {
        self.head.id
    }

    fn owner(self) -> Option<HbkPlatformTypeId> {
        (self.head.owner != NONE_U32).then_some(HbkPlatformTypeId(self.head.owner))
    }

    fn kind(self) -> HbkCallableKind {
        self.head.kind
    }

    fn name(self) -> X1NameView {
        X1NameView {
            head: self.head.name,
        }
    }

    fn signatures(self) -> impl ExactSizeIterator<Item = X1SignatureView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::Signatures, self.head.signatures)
            .map(move |head| X1SignatureView { handle, head })
    }

    fn return_type_refs(self) -> impl ExactSizeIterator<Item = X1TypeRefView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::TypeRefs, self.head.return_type_refs)
            .map(move |head| X1TypeRefView { handle, head })
    }

    fn availability_contexts(self) -> X1RecordIter<'a, StringId> {
        self.handle
            .generation
            .records(S::StringIds, self.head.availability_contexts)
    }
}

#[allow(dead_code)]
impl<'a> X1SignatureView<'a> {
    fn text(self) -> StringId {
        self.head.text
    }

    fn parameters(self) -> impl ExactSizeIterator<Item = X1ParameterView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::Parameters, self.head.parameters)
            .map(move |head| X1ParameterView { handle, head })
    }

    fn return_type_refs(self) -> impl ExactSizeIterator<Item = X1TypeRefView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::TypeRefs, self.head.return_type_refs)
            .map(move |head| X1TypeRefView { handle, head })
    }
}

#[allow(dead_code)]
impl<'a> X1ParameterView<'a> {
    fn name(self) -> StringId {
        self.head.name
    }

    fn required(self) -> bool {
        self.head.required
    }

    fn type_refs(self) -> impl ExactSizeIterator<Item = X1TypeRefView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::TypeRefs, self.head.type_refs)
            .map(move |head| X1TypeRefView { handle, head })
    }
}

#[allow(dead_code)]
impl<'a> X1GlobalFactView<'a> {
    fn id(self) -> StringId {
        self.head.id
    }

    fn kind(self) -> HbkGlobalFactKind {
        self.head.kind
    }

    fn domain(self) -> HbkLanguageDomain {
        self.head.domain
    }

    fn name(self) -> X1NameView {
        X1NameView {
            head: self.head.name,
        }
    }

    fn callable(self) -> Option<HbkCallableId> {
        (self.head.callable != NONE_U32).then_some(HbkCallableId(self.head.callable))
    }

    fn type_refs(self) -> impl ExactSizeIterator<Item = X1TypeRefView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::TypeRefs, self.head.type_refs)
            .map(move |head| X1TypeRefView { handle, head })
    }
}

#[allow(dead_code)]
impl<'a> X1QueryTableView<'a> {
    fn id(self) -> StringId {
        self.head.id
    }

    fn name(self) -> X1NameView {
        X1NameView {
            head: self.head.name,
        }
    }

    fn syntax(self) -> Option<X1NameView> {
        self.head.syntax_present.then_some(X1NameView {
            head: self.head.syntax,
        })
    }

    fn identifier(self) -> Option<StringId> {
        x1_optional_string(self.head.identifier)
    }

    fn role(self) -> Option<model::QueryTableRole> {
        match self.head.role {
            0 => None,
            1 => Some(model::QueryTableRole::Primary),
            2 => Some(model::QueryTableRole::Additional),
            3 => Some(model::QueryTableRole::Unknown),
            _ => unreachable!("X1 query-table role was fully validated before access"),
        }
    }

    fn owner_path(self) -> impl ExactSizeIterator<Item = X1NameView> + 'a {
        self.handle
            .generation
            .records(S::Names, self.head.owner_path)
            .map(|head| X1NameView { head })
    }

    fn template_parameters(self) -> X1RecordIter<'a, StringId> {
        self.handle
            .generation
            .records(S::StringIds, self.head.template_parameters)
    }
}

#[allow(dead_code)]
impl<'a> X1QueryFieldView<'a> {
    fn id(self) -> StringId {
        self.head.id
    }

    fn owner(self) -> HbkQueryTableId {
        self.head.owner
    }

    fn name(self) -> X1NameView {
        X1NameView {
            head: self.head.name,
        }
    }

    fn type_refs(self) -> impl ExactSizeIterator<Item = X1TypeRefView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::TypeRefs, self.head.type_refs)
            .map(move |head| X1TypeRefView { handle, head })
    }

    fn note(self) -> Option<StringId> {
        x1_optional_string(self.head.note)
    }
}

#[allow(dead_code)]
impl<'a> X1QueryParameterView<'a> {
    fn id(self) -> StringId {
        self.head.id
    }

    fn owner(self) -> HbkQueryTableId {
        self.head.owner
    }

    fn name(self) -> X1NameView {
        X1NameView {
            head: self.head.name,
        }
    }

    fn type_refs(self) -> impl ExactSizeIterator<Item = X1TypeRefView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::TypeRefs, self.head.type_refs)
            .map(move |head| X1TypeRefView { handle, head })
    }

    fn default_value(self) -> Option<StringId> {
        x1_optional_string(self.head.default_value)
    }
}

#[allow(dead_code)]
impl<'a> X1LanguageFactView<'a> {
    fn id(self) -> StringId {
        self.head.id
    }

    fn kind(self) -> SearchDocumentKind {
        self.head.kind
    }

    fn domain(self) -> HbkLanguageDomain {
        self.head.domain
    }

    fn name(self) -> X1NameView {
        X1NameView {
            head: self.head.name,
        }
    }

    fn signatures(self) -> impl ExactSizeIterator<Item = X1SignatureView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::Signatures, self.head.signatures)
            .map(move |head| X1SignatureView { handle, head })
    }

    fn type_refs(self) -> impl ExactSizeIterator<Item = X1TypeRefView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::TypeRefs, self.head.type_refs)
            .map(move |head| X1TypeRefView { handle, head })
    }

    fn return_type_refs(self) -> impl ExactSizeIterator<Item = X1TypeRefView<'a>> + 'a {
        let handle = self.handle;
        handle
            .generation
            .records(S::TypeRefs, self.head.return_type_refs)
            .map(move |head| X1TypeRefView { handle, head })
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct X1EnumView {
    head: X1EnumHead,
}

#[allow(dead_code)]
impl X1EnumView {
    fn id(self) -> StringId {
        self.head.id
    }

    fn name(self) -> X1NameView {
        X1NameView {
            head: self.head.name,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct X1EnumValueView {
    head: X1EnumValueHead,
}

#[allow(dead_code)]
impl X1EnumValueView {
    fn id(self) -> StringId {
        self.head.id
    }

    fn owner(self) -> HbkEnumId {
        self.head.owner
    }

    fn name(self) -> X1NameView {
        X1NameView {
            head: self.head.name,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum X1TypeRefTargetKind {
    Ok,
    Unresolved,
    Ambiguous,
}

#[allow(dead_code)]
impl<'a> X1TypeRefView<'a> {
    fn name(self) -> StringId {
        self.head.name
    }

    fn target_kind(self) -> X1TypeRefTargetKind {
        match self.head.target_tag {
            0 => X1TypeRefTargetKind::Ok,
            1 => X1TypeRefTargetKind::Unresolved,
            2 => X1TypeRefTargetKind::Ambiguous,
            _ => unreachable!("X1 type-ref target was fully validated before access"),
        }
    }

    fn target_ok(self) -> Option<StringId> {
        (self.head.target_tag == 0).then_some(StringId(self.head.target_ok))
    }

    fn ambiguous_targets(self) -> X1RecordIter<'a, StringId> {
        self.handle
            .generation
            .records(S::StringIds, self.head.ambiguous_targets)
    }

    fn type_template_key(self) -> Option<HbkPlatformTypeTemplateKey> {
        x1_template_key(self.head.type_template_key)
    }

    fn template_binding(self) -> Option<X1TemplateBindingView<'a>> {
        (self.head.template_binding != NONE_U32).then(|| X1TemplateBindingView {
            handle: self.handle,
            head: self
                .handle
                .generation
                .record(S::TemplateBindings, self.head.template_binding as usize),
        })
    }
}

#[allow(dead_code)]
impl<'a> X1TemplateBindingView<'a> {
    fn template_key(self) -> HbkPlatformTypeTemplateKey {
        x1_template_key(self.head.template_key)
            .expect("X1 template binding key was fully validated before access")
    }

    fn arguments(self) -> X1RecordIter<'a, model::TemplateParameterBinding> {
        self.handle
            .generation
            .records(S::TemplateArguments, self.head.arguments)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct X1FactSourceView {
    source: HbkFactSource,
}

#[allow(dead_code)]
impl X1FactSourceView {
    fn hbk_path(self) -> StringId {
        self.source.hbk_path
    }

    fn locale(self) -> StringId {
        self.source.locale
    }

    fn toc_path(self) -> Option<StringId> {
        self.source.toc_path
    }

    fn html_path(self) -> StringId {
        self.source.html_path
    }

    fn page_title(self) -> StringId {
        self.source.page_title
    }
}

#[derive(Debug, Clone, Copy)]
struct Section {
    offset: usize,
    len: usize,
}

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum S {
    Strings,
    StringOrder,
    PlatformTypes,
    TypeMembers,
    Callables,
    Globals,
    QueryTables,
    QueryFields,
    QueryParameters,
    LanguageFacts,
    Enums,
    EnumValues,
    MetadataTemplates,
    Signatures,
    Parameters,
    TypeRefs,
    TemplateBindings,
    TemplateArguments,
    Names,
    StringIds,
    FactIds,
    PlatformTypeIds,
    PlatformTypeNames,
    PlatformTypeTemplates,
    MemberIds,
    MembersByOwnerKeys,
    MembersByOwnerOffsets,
    MembersByOwnerValues,
    TypeMemberRanges,
    MemberAvailabilityHot,
    GlobalAvailabilityHot,
    GlobalAvailabilityMasks,
    GlobalAvailabilityKinds,
    MembersByOwnerName,
    MembersByOwnerNameKind,
    CallableIds,
    CallablesByOwnerKeys,
    CallablesByOwnerOffsets,
    CallablesByOwnerValues,
    CallablesByOwnerName,
    ConstructorsByTypeKeys,
    ConstructorsByTypeOffsets,
    ConstructorsByTypeValues,
    GlobalNames,
    GlobalsByDomainNameKind,
    ModuleEventNames,
    ModuleContextsByDomainLanguageKind,
    QueryTableIds,
    QueryTableNames,
    QueryTableSyntaxNames,
    QueryTableIdentifiers,
    QueryFieldsByTableKeys,
    QueryFieldsByTableOffsets,
    QueryFieldsByTableValues,
    QueryFieldsByTableName,
    QueryParametersByTableKeys,
    QueryParametersByTableOffsets,
    QueryParametersByTableValues,
    QueryParametersByTableName,
    LanguageIds,
    LanguageNames,
    EnumIds,
    EnumNames,
    EnumValueIds,
    EnumValuesByEnumKeys,
    EnumValuesByEnumOffsets,
    EnumValuesByEnumValues,
    EnumValuesByEnumName,
    AvailabilityByFactKeys,
    AvailabilityByFactOffsets,
    AvailabilityByFactValues,
    AvailabilitySinceByFact,
    SourceByFact,
    RelationsBySourceKindKeys,
    RelationsBySourceKindOffsets,
    RelationsBySourceKindValues,
    PlatformTypeNameHash,
    CompatibilityMetadata,
}

impl S {
    fn index(self) -> usize {
        self as usize
    }
}

const NONE_U32: u32 = u32::MAX;
const X1_CONTEXT_BITS: u16 = 0x01ff;
const X1_HAS_EXPLICIT_DECLARATION: u16 = 0x8000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct X1Range {
    start: u32,
    len: u32,
}

impl X1Range {
    const EMPTY: Self = Self { start: 0, len: 0 };

    fn from_bounds(start: usize, end: usize, what: &'static str) -> io::Result<Self> {
        let len = end.checked_sub(start).ok_or_else(|| invalid_data(what))?;
        let start = u32::try_from(start).map_err(|_| invalid_data(what))?;
        let len = u32::try_from(len).map_err(|_| invalid_data(what))?;
        Ok(Self { start, len })
    }

    fn as_usize(self) -> io::Result<Range<usize>> {
        let start = self.start as usize;
        let end = start
            .checked_add(self.len as usize)
            .ok_or_else(|| invalid_data("X1 arena range overflow"))?;
        Ok(start..end)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct X1NameHead {
    primary: StringId,
    alias: u32,
}

impl From<&HbkName> for X1NameHead {
    fn from(value: &HbkName) -> Self {
        Self {
            primary: value.primary,
            alias: value.alias.map_or(NONE_U32, |id| id.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct X1TemplateKeyHead {
    family: u32,
    variant: u32,
}

impl X1TemplateKeyHead {
    const NONE: Self = Self {
        family: NONE_U32,
        variant: NONE_U32,
    };

    fn from_option(value: Option<HbkPlatformTypeTemplateKey>) -> Self {
        value.map_or(Self::NONE, |value| Self {
            family: value.family.0,
            variant: value.variant.0,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct X1PlatformTypeHead {
    id: StringId,
    name: X1NameHead,
    metadata_template: u32,
    type_template_key: X1TemplateKeyHead,
    availability_contexts: X1Range,
}

#[derive(Debug, Clone, Copy)]
struct X1MetadataTemplateHead {
    metadata_kind: StringId,
    template_parameters: X1Range,
}

#[derive(Debug, Clone, Copy)]
struct X1TypeMemberHead {
    id: StringId,
    owner: HbkPlatformTypeId,
    kind: HbkTypeMemberKind,
    name: X1NameHead,
    type_refs: X1Range,
    availability_contexts: X1Range,
}

#[derive(Debug, Clone, Copy)]
struct X1TypeMemberRangeHot {
    member_start: u32,
    member_count: u32,
}

#[derive(Debug, Clone, Copy)]
struct X1AvailableMemberHot {
    member_id: u32,
    availability_word: u16,
    kind: u8,
    reserved: u8,
}

#[derive(Debug, Clone, Copy)]
struct X1TypeNameHashBucket {
    hash: u64,
    start: u32,
    count: u32,
}

#[derive(Debug, Clone, Copy)]
struct X1CallableHead {
    id: StringId,
    owner: u32,
    kind: HbkCallableKind,
    name: X1NameHead,
    signatures: X1Range,
    return_type_refs: X1Range,
    availability_contexts: X1Range,
}

#[derive(Debug, Clone, Copy)]
struct X1SignatureHead {
    text: StringId,
    parameters: X1Range,
    return_type_refs: X1Range,
}

#[derive(Debug, Clone, Copy)]
struct X1ParameterHead {
    name: StringId,
    required: bool,
    type_refs: X1Range,
}

#[derive(Debug, Clone, Copy)]
struct X1GlobalFactHead {
    id: StringId,
    kind: HbkGlobalFactKind,
    domain: HbkLanguageDomain,
    name: X1NameHead,
    callable: u32,
    type_refs: X1Range,
}

#[derive(Debug, Clone, Copy)]
struct X1QueryTableHead {
    id: StringId,
    name: X1NameHead,
    syntax_present: bool,
    syntax: X1NameHead,
    identifier: u32,
    role: u8,
    owner_path: X1Range,
    template_parameters: X1Range,
}

#[derive(Debug, Clone, Copy)]
struct X1QueryFieldHead {
    id: StringId,
    owner: HbkQueryTableId,
    name: X1NameHead,
    type_refs: X1Range,
    note: u32,
}

#[derive(Debug, Clone, Copy)]
struct X1QueryParameterHead {
    id: StringId,
    owner: HbkQueryTableId,
    name: X1NameHead,
    type_refs: X1Range,
    default_value: u32,
}

#[derive(Debug, Clone, Copy)]
struct X1LanguageFactHead {
    id: StringId,
    kind: SearchDocumentKind,
    domain: HbkLanguageDomain,
    name: X1NameHead,
    signatures: X1Range,
    type_refs: X1Range,
    return_type_refs: X1Range,
}

#[derive(Debug, Clone, Copy)]
struct X1EnumHead {
    id: StringId,
    name: X1NameHead,
}

#[derive(Debug, Clone, Copy)]
struct X1EnumValueHead {
    id: StringId,
    owner: HbkEnumId,
    name: X1NameHead,
}

#[derive(Debug, Clone, Copy)]
struct X1TypeRefHead {
    name: StringId,
    target_tag: u8,
    target_ok: u32,
    ambiguous_targets: X1Range,
    type_template_key: X1TemplateKeyHead,
    template_binding: u32,
}

#[derive(Debug, Clone, Copy)]
struct X1TemplateBindingHead {
    template_key: X1TemplateKeyHead,
    arguments: X1Range,
}

macro_rules! x1_binary_record {
    ($type:ty { $($field:ident: $field_type:ty),+ $(,)? }) => {
        impl BinaryValue for $type {
            fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
                $(self.$field.write_to(writer)?;)+
                Ok(())
            }

            fn read_from<R: std::io::Read>(
                reader: &mut BinaryReader<R>,
            ) -> io::Result<Self> {
                Ok(Self {
                    $($field: <$field_type>::read_from(reader)?,)+
                })
            }
        }
    };
}

impl BinaryValue for u8 {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_u8(*self)
    }

    fn read_from<R: std::io::Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        reader.read_u8()
    }
}

impl BinaryValue for u16 {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.inner.write_all(&self.to_le_bytes())
    }

    fn read_from<R: std::io::Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        let mut bytes = [0_u8; 2];
        reader.inner.read_exact(&mut bytes)?;
        Ok(Self::from_le_bytes(bytes))
    }
}

impl BinaryValue for u64 {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.inner.write_all(&self.to_le_bytes())
    }

    fn read_from<R: std::io::Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        let mut bytes = [0_u8; 8];
        reader.inner.read_exact(&mut bytes)?;
        Ok(Self::from_le_bytes(bytes))
    }
}

impl BinaryValue for bool {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_bool(*self)
    }

    fn read_from<R: std::io::Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        reader.read_bool()
    }
}

x1_binary_record!(X1Range {
    start: u32,
    len: u32
});
x1_binary_record!(X1NameHead {
    primary: StringId,
    alias: u32
});
x1_binary_record!(X1TemplateKeyHead {
    family: u32,
    variant: u32
});
x1_binary_record!(X1PlatformTypeHead {
    id: StringId,
    name: X1NameHead,
    metadata_template: u32,
    type_template_key: X1TemplateKeyHead,
    availability_contexts: X1Range,
});
x1_binary_record!(X1MetadataTemplateHead {
    metadata_kind: StringId,
    template_parameters: X1Range,
});
x1_binary_record!(X1TypeMemberHead {
    id: StringId,
    owner: HbkPlatformTypeId,
    kind: HbkTypeMemberKind,
    name: X1NameHead,
    type_refs: X1Range,
    availability_contexts: X1Range,
});
x1_binary_record!(X1TypeMemberRangeHot {
    member_start: u32,
    member_count: u32,
});
x1_binary_record!(X1AvailableMemberHot {
    member_id: u32,
    availability_word: u16,
    kind: u8,
    reserved: u8,
});
x1_binary_record!(X1TypeNameHashBucket {
    hash: u64,
    start: u32,
    count: u32,
});
x1_binary_record!(X1CallableHead {
    id: StringId,
    owner: u32,
    kind: HbkCallableKind,
    name: X1NameHead,
    signatures: X1Range,
    return_type_refs: X1Range,
    availability_contexts: X1Range,
});
x1_binary_record!(X1SignatureHead {
    text: StringId,
    parameters: X1Range,
    return_type_refs: X1Range,
});
x1_binary_record!(X1ParameterHead {
    name: StringId,
    required: bool,
    type_refs: X1Range,
});
x1_binary_record!(X1GlobalFactHead {
    id: StringId,
    kind: HbkGlobalFactKind,
    domain: HbkLanguageDomain,
    name: X1NameHead,
    callable: u32,
    type_refs: X1Range,
});
x1_binary_record!(X1QueryTableHead {
    id: StringId,
    name: X1NameHead,
    syntax_present: bool,
    syntax: X1NameHead,
    identifier: u32,
    role: u8,
    owner_path: X1Range,
    template_parameters: X1Range,
});
x1_binary_record!(X1QueryFieldHead {
    id: StringId,
    owner: HbkQueryTableId,
    name: X1NameHead,
    type_refs: X1Range,
    note: u32,
});
x1_binary_record!(X1QueryParameterHead {
    id: StringId,
    owner: HbkQueryTableId,
    name: X1NameHead,
    type_refs: X1Range,
    default_value: u32,
});
x1_binary_record!(X1LanguageFactHead {
    id: StringId,
    kind: SearchDocumentKind,
    domain: HbkLanguageDomain,
    name: X1NameHead,
    signatures: X1Range,
    type_refs: X1Range,
    return_type_refs: X1Range,
});
x1_binary_record!(X1EnumHead {
    id: StringId,
    name: X1NameHead
});
x1_binary_record!(X1EnumValueHead {
    id: StringId,
    owner: HbkEnumId,
    name: X1NameHead,
});
x1_binary_record!(X1TypeRefHead {
    name: StringId,
    target_tag: u8,
    target_ok: u32,
    ambiguous_targets: X1Range,
    type_template_key: X1TemplateKeyHead,
    template_binding: u32,
});
x1_binary_record!(X1TemplateBindingHead {
    template_key: X1TemplateKeyHead,
    arguments: X1Range,
});

impl HbkFactSnapshotBuildReport {
    /// Writes a validated immutable X1 generation without enabling it as a
    /// runtime source. The target must not already exist.
    pub fn write_x1_generation(
        &self,
        artifact_path: impl AsRef<Path>,
    ) -> Result<HbkFactSnapshotArtifactWriteReport, SearchError> {
        write_x1_generation(self, artifact_path.as_ref()).map_err(|source| SearchError::Io {
            path: artifact_path.as_ref().to_path_buf(),
            source,
        })
    }

    /// Publishes this snapshot into a stable content-addressed X1 slot.
    ///
    /// Publication is fail-fast while any mapped reader owns a shared slot
    /// lock. Existing immutable generations are reused only after their full
    /// content hash and artifact identity have been validated.
    pub fn publish_x1_generation(
        &self,
        slot_path: impl AsRef<Path>,
    ) -> Result<HbkFactSnapshotArtifactPublicationReport, SearchError> {
        self.publish_x1_generation_with_options(
            slot_path.as_ref(),
            X1PublicationOptions::production(),
        )
    }

    fn publish_x1_generation_with_options(
        &self,
        slot_path: &Path,
        options: X1PublicationOptions,
    ) -> Result<HbkFactSnapshotArtifactPublicationReport, SearchError> {
        reject_symlink_path_components(slot_path)?;
        ensure_stable_directory(slot_path, "X1 snapshot slot")?;
        let generations_path = slot_path.join(X1_SLOT_GENERATIONS_DIR);
        ensure_stable_directory(&generations_path, "X1 generations directory")?;

        let lock_path = slot_path.join(X1_SLOT_LOCK_FILE);
        let lock_file = open_or_create_stable_lock(&lock_path)?;
        match File::try_lock(&lock_file) {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(SearchError::SnapshotInUse {
                    path: slot_path.to_path_buf(),
                });
            }
            Err(TryLockError::Error(source)) => {
                return Err(SearchError::Io {
                    path: lock_path,
                    source,
                });
            }
        }
        validate_open_file_path(
            &slot_path.join(X1_SLOT_LOCK_FILE),
            &lock_file,
            "X1 slot lock changed after locking",
        )?;
        #[cfg(test)]
        if let Some(hook) = &options.lock_hook {
            hook.acquired.wait();
            hook.release.wait();
        }
        reject_symlink_path_components(slot_path)?;
        validate_stable_directory(slot_path, "X1 snapshot slot")?;
        validate_stable_directory(&generations_path, "X1 generations directory")?;
        validate_current_before_publication(slot_path)?;

        let identity = artifact_identity(self).map_err(|source| SearchError::Io {
            path: self.cache_index_path.clone(),
            source,
        })?;
        let stage_path = generations_path.join(format!(".generation-{}.tmp", options.nonce));
        let pointer_temp_path = slot_path.join(format!(".current-{}.tmp", options.nonce));
        reject_existing_temp_candidate(&stage_path)?;
        reject_existing_temp_candidate(&pointer_temp_path)?;

        let mut owns_stage = false;
        let mut owns_pointer_temp = false;
        let result = (|| {
            let written = self.write_x1_generation(&stage_path)?;
            owns_stage = true;
            let stage_bytes = read_stable_generation(&stage_path, "X1 generation temp")?;
            validate_mmap_expected(&stage_bytes, Some(&identity)).map_err(|source| {
                SearchError::SnapshotArtifact {
                    path: stage_path.clone(),
                    source: artifact_error_from_io(source),
                }
            })?;
            let artifact_sha256 = bytes_sha256(&stage_bytes);
            let generation_file_name = generation_file_name(&artifact_sha256)?;
            let generation_path = generations_path.join(&generation_file_name);

            inject_publication_failure(
                options.fail_at,
                X1PublicationFailurePoint::BeforeGeneration,
                &stage_path,
            )?;

            let reused_existing_generation = match fs::symlink_metadata(&generation_path) {
                Ok(metadata) => {
                    validate_stable_regular_file_metadata(
                        &generation_path,
                        &metadata,
                        "X1 generation target",
                    )?;
                    validate_generation_metadata(&generation_path, &metadata)?;
                    let existing =
                        read_stable_generation(&generation_path, "X1 generation target")?;
                    let existing_sha = bytes_sha256(&existing);
                    if existing_sha != artifact_sha256 {
                        return Err(snapshot_artifact_invalid(
                            &generation_path,
                            "existing X1 generation content does not match its content-addressed name",
                        ));
                    }
                    validate_mmap_expected(&existing, Some(&identity)).map_err(|source| {
                        SearchError::SnapshotArtifact {
                            path: generation_path.clone(),
                            source: artifact_error_from_io(source),
                        }
                    })?;
                    if existing != stage_bytes {
                        return Err(snapshot_artifact_invalid(
                            &generation_path,
                            "existing X1 generation bytes differ from staged generation",
                        ));
                    }
                    remove_owned_temp(&stage_path)?;
                    owns_stage = false;
                    true
                }
                Err(source) if source.kind() == io::ErrorKind::NotFound => {
                    fs::rename(&stage_path, &generation_path).map_err(|source| {
                        SearchError::Io {
                            path: generation_path.clone(),
                            source,
                        }
                    })?;
                    owns_stage = false;
                    validate_openable_stable_file(&generation_path, "X1 generation target")?;
                    sync_directory(&generations_path)?;
                    false
                }
                Err(source) => {
                    return Err(SearchError::Io {
                        path: generation_path,
                        source,
                    });
                }
            };

            inject_publication_failure(
                options.fail_at,
                X1PublicationFailurePoint::GenerationPublished,
                &generation_path,
            )?;

            let pointer = format!("{generation_file_name}\n");
            let mut pointer_temp = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&pointer_temp_path)
                .map_err(|source| SearchError::Io {
                    path: pointer_temp_path.clone(),
                    source,
                })?;
            owns_pointer_temp = true;
            pointer_temp
                .write_all(pointer.as_bytes())
                .and_then(|()| pointer_temp.sync_all())
                .map_err(|source| SearchError::Io {
                    path: pointer_temp_path.clone(),
                    source,
                })?;
            let mut permissions = pointer_temp
                .metadata()
                .map_err(|source| SearchError::Io {
                    path: pointer_temp_path.clone(),
                    source,
                })?
                .permissions();
            permissions.set_readonly(true);
            pointer_temp
                .set_permissions(permissions)
                .and_then(|()| pointer_temp.sync_all())
                .map_err(|source| SearchError::Io {
                    path: pointer_temp_path.clone(),
                    source,
                })?;
            drop(pointer_temp);
            validate_openable_stable_file(&pointer_temp_path, "X1 current temp")?;

            let current_path = slot_path.join(X1_SLOT_CURRENT_FILE);
            fs::rename(&pointer_temp_path, &current_path).map_err(|source| SearchError::Io {
                path: current_path.clone(),
                source,
            })?;
            owns_pointer_temp = false;
            assert_current_pointer(&current_path, &generation_file_name)?;

            inject_publication_failure(
                options.fail_at,
                X1PublicationFailurePoint::CurrentPublished,
                &current_path,
            )?;
            sync_directory(slot_path)?;

            Ok(HbkFactSnapshotArtifactPublicationReport {
                artifact_bytes: written.artifact_bytes,
                artifact_sha256,
                generation_file_name,
                platform_version: written.platform_version,
                source_sha256: written.source_sha256,
                provider_sha256: written.provider_sha256,
                reused_existing_generation,
            })
        })();

        if owns_stage {
            let _ = fs::remove_file(&stage_path);
        }
        if owns_pointer_temp {
            let _ = fs::remove_file(&pointer_temp_path);
        }
        result
    }
}

fn reject_symlink_path_components(path: &Path) -> Result<(), SearchError> {
    let mut current = if path.is_absolute() {
        PathBuf::new()
    } else {
        std::env::current_dir().map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?
    };
    let mut missing_component_seen = false;
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(component.as_os_str()),
            Component::CurDir => continue,
            Component::ParentDir => {
                return Err(snapshot_artifact_invalid(
                    path,
                    "X1 slot path must not contain parent-directory components",
                ));
            }
            Component::Normal(value) => current.push(value),
        }
        if missing_component_seen {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(snapshot_artifact_invalid(
                    &current,
                    "X1 slot path contains a symlink component",
                ));
            }
            Ok(_) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                missing_component_seen = true;
            }
            Err(source) => {
                return Err(SearchError::Io {
                    path: current,
                    source,
                });
            }
        }
    }
    Ok(())
}

fn ensure_stable_directory(path: &Path, description: &'static str) -> Result<(), SearchError> {
    match fs::create_dir_all(path) {
        Ok(()) => validate_stable_directory(path, description),
        Err(source) => Err(SearchError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn validate_stable_directory(path: &Path, description: &'static str) -> Result<(), SearchError> {
    let before = stable_path_metadata(path, description)?;
    if before.file_type().is_symlink() || !before.file_type().is_dir() {
        return Err(snapshot_artifact_invalid(path, description));
    }
    let directory = File::open(path).map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let opened = directory.metadata().map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let after = fs::symlink_metadata(path).map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if after.file_type().is_symlink() || !after.file_type().is_dir() {
        return Err(snapshot_artifact_invalid(path, description));
    }
    validate_same_file(path, &before, &opened, "X1 directory changed while opening")?;
    validate_same_file(
        path,
        &opened,
        &after,
        "X1 directory path changed while opening",
    )
}

fn open_or_create_stable_lock(path: &Path) -> Result<File, SearchError> {
    match OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(file) => {
            validate_open_file_path(path, &file, "X1 slot lock changed while creating")?;
            Ok(file)
        }
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            open_stable_regular_file(path, true, "X1 slot lock")
        }
        Err(source) => Err(SearchError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn open_stable_regular_file(
    path: &Path,
    writable: bool,
    description: &'static str,
) -> Result<File, SearchError> {
    let before = stable_path_metadata(path, description)?;
    validate_stable_regular_file_metadata(path, &before, description)?;
    let file = OpenOptions::new()
        .read(true)
        .write(writable)
        .open(path)
        .map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    validate_open_file_path_against(path, &file, &before, description)?;
    Ok(file)
}

fn stable_path_metadata(
    path: &Path,
    missing_message: &'static str,
) -> Result<fs::Metadata, SearchError> {
    fs::symlink_metadata(path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            snapshot_artifact_invalid(path, missing_message)
        } else {
            SearchError::Io {
                path: path.to_path_buf(),
                source,
            }
        }
    })
}

fn validate_openable_stable_file(
    path: &Path,
    description: &'static str,
) -> Result<(), SearchError> {
    open_stable_regular_file(path, false, description).map(drop)
}

fn validate_open_file_path(
    path: &Path,
    file: &File,
    description: &'static str,
) -> Result<(), SearchError> {
    let before = fs::symlink_metadata(path).map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    validate_stable_regular_file_metadata(path, &before, description)?;
    validate_open_file_path_against(path, file, &before, description)
}

fn validate_open_file_path_against(
    path: &Path,
    file: &File,
    before: &fs::Metadata,
    description: &'static str,
) -> Result<(), SearchError> {
    let opened = file.metadata().map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    validate_stable_regular_file_metadata(path, &opened, description)?;
    let after = fs::symlink_metadata(path).map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    validate_stable_regular_file_metadata(path, &after, description)?;
    validate_same_file(path, before, &opened, description)?;
    validate_same_file(path, &opened, &after, description)
}

fn validate_stable_regular_file_metadata(
    path: &Path,
    metadata: &fs::Metadata,
    message: &'static str,
) -> Result<(), SearchError> {
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(snapshot_artifact_invalid(path, message));
    }
    Ok(())
}

fn validate_same_file(
    path: &Path,
    before: &fs::Metadata,
    after: &fs::Metadata,
    message: &'static str,
) -> Result<(), SearchError> {
    #[cfg(unix)]
    let same_identity = before.dev() == after.dev() && before.ino() == after.ino();
    #[cfg(not(unix))]
    let same_identity = before.len() == after.len();
    if !same_identity || before.len() != after.len() {
        return Err(snapshot_artifact_invalid(path, message));
    }
    Ok(())
}

fn read_stable_generation(path: &Path, description: &'static str) -> Result<Vec<u8>, SearchError> {
    let mut file = open_stable_regular_file(path, false, description)?;
    let metadata = file.metadata().map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    validate_generation_metadata(path, &metadata)?;
    let len = usize::try_from(metadata.len()).map_err(|_| {
        snapshot_artifact_invalid(path, "X1 generation length does not fit address space")
    })?;
    let mut bytes = vec![0_u8; len];
    file.read_exact(&mut bytes)
        .map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    validate_open_file_path(path, &file, "X1 generation changed while reading")?;
    Ok(bytes)
}

fn hash_stable_generation(path: &Path, description: &'static str) -> Result<String, SearchError> {
    let mut file = open_stable_regular_file(path, false, description)?;
    let metadata = file.metadata().map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    validate_generation_metadata(path, &metadata)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut total = 0_u64;
    loop {
        let read = file.read(&mut buffer).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        total = total.saturating_add(read as u64);
        digest.update(&buffer[..read]);
    }
    if total != metadata.len() {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 generation length changed while hashing",
        ));
    }
    validate_open_file_path(path, &file, "X1 generation changed while hashing")?;
    Ok(format!("{:x}", digest.finalize()))
}

fn bytes_sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn generation_file_name(artifact_sha256: &str) -> Result<String, SearchError> {
    if artifact_sha256.len() != SHA256_HEX_LEN
        || !artifact_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(snapshot_artifact_invalid(
            Path::new(X1_SLOT_CURRENT_FILE),
            "X1 artifact SHA-256 is not lowercase hexadecimal",
        ));
    }
    Ok(format!(
        "{X1_GENERATION_PREFIX}{artifact_sha256}{X1_GENERATION_SUFFIX}"
    ))
}

fn parse_current_pointer(path: &Path, bytes: &[u8]) -> Result<String, SearchError> {
    if bytes.len() != X1_CURRENT_POINTER_LEN || bytes.last() != Some(&b'\n') {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 current pointer has invalid length or terminator",
        ));
    }
    let file_name = std::str::from_utf8(&bytes[..bytes.len() - 1])
        .map_err(|_| snapshot_artifact_invalid(path, "X1 current pointer is not valid UTF-8"))?;
    let hash = file_name
        .strip_prefix(X1_GENERATION_PREFIX)
        .and_then(|value| value.strip_suffix(X1_GENERATION_SUFFIX))
        .ok_or_else(|| {
            snapshot_artifact_invalid(path, "X1 current pointer has invalid generation name")
        })?;
    if hash.len() != SHA256_HEX_LEN
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 current pointer hash is not lowercase hexadecimal",
        ));
    }
    Ok(file_name.to_string())
}

fn read_current_pointer(path: &Path) -> Result<String, SearchError> {
    let mut file = open_stable_regular_file(path, false, "X1 current pointer")?;
    let metadata = file.metadata().map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.permissions().readonly() {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 current pointer is not read-only",
        ));
    }
    if metadata.len() != X1_CURRENT_POINTER_LEN as u64 {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 current pointer has invalid byte length",
        ));
    }
    let mut bytes = [0_u8; X1_CURRENT_POINTER_LEN];
    file.read_exact(&mut bytes)
        .map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    validate_open_file_path(path, &file, "X1 current pointer changed while reading")?;
    parse_current_pointer(path, &bytes)
}

fn assert_current_pointer(path: &Path, expected: &str) -> Result<(), SearchError> {
    let actual = read_current_pointer(path)?;
    if actual != expected {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 current pointer does not name the published generation",
        ));
    }
    Ok(())
}

fn validate_current_before_publication(slot_path: &Path) -> Result<(), SearchError> {
    let current_path = slot_path.join(X1_SLOT_CURRENT_FILE);
    match fs::symlink_metadata(&current_path) {
        Ok(_) => {
            let generation_name = read_current_pointer(&current_path)?;
            let generation_path = slot_path
                .join(X1_SLOT_GENERATIONS_DIR)
                .join(generation_name);
            let bytes = read_stable_generation(&generation_path, "X1 current generation")?;
            validate_generation_content_address_bytes(&generation_path, &bytes)?;
            validate_mmap_expected(&bytes, None).map_err(|source| {
                SearchError::SnapshotArtifact {
                    path: generation_path,
                    source: artifact_error_from_io(source),
                }
            })?;
            Ok(())
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(SearchError::Io {
            path: current_path,
            source,
        }),
    }
}

fn validate_generation_content_address(path: &Path) -> Result<(), SearchError> {
    let actual_hash = hash_stable_generation(path, "X1 current generation")?;
    validate_generation_content_address_hash(path, &actual_hash)
}

fn validate_generation_content_address_bytes(path: &Path, bytes: &[u8]) -> Result<(), SearchError> {
    validate_generation_content_address_hash(path, &bytes_sha256(bytes))
}

fn validate_generation_content_address_hash(
    path: &Path,
    actual_hash: &str,
) -> Result<(), SearchError> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            snapshot_artifact_invalid(path, "X1 generation file name is not valid UTF-8")
        })?;
    let expected_hash = file_name
        .strip_prefix(X1_GENERATION_PREFIX)
        .and_then(|value| value.strip_suffix(X1_GENERATION_SUFFIX))
        .ok_or_else(|| {
            snapshot_artifact_invalid(path, "X1 generation file name is not content-addressed")
        })?;
    if expected_hash.len() != SHA256_HEX_LEN
        || !expected_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 generation file name hash is invalid",
        ));
    }
    if actual_hash != expected_hash {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 generation content does not match current content address",
        ));
    }
    Ok(())
}

fn reject_existing_temp_candidate(path: &Path) -> Result<(), SearchError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(snapshot_artifact_invalid(
            path,
            "X1 publication temp candidate is a symlink",
        )),
        Ok(_) => Err(snapshot_artifact_invalid(
            path,
            "X1 publication temp candidate already exists",
        )),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(SearchError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn remove_owned_temp(path: &Path) -> Result<(), SearchError> {
    fs::remove_file(path).map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn sync_directory(path: &Path) -> Result<(), SearchError> {
    let directory = File::open(path).map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    directory.sync_all().map_err(|source| SearchError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn inject_publication_failure(
    configured: Option<X1PublicationFailurePoint>,
    current: X1PublicationFailurePoint,
    path: &Path,
) -> Result<(), SearchError> {
    if configured == Some(current) {
        return Err(SearchError::Io {
            path: path.to_path_buf(),
            source: io::Error::other(format!("injected X1 publication failure at {current:?}")),
        });
    }
    Ok(())
}

fn write_x1_generation(
    report: &HbkFactSnapshotBuildReport,
    artifact_path: &Path,
) -> io::Result<HbkFactSnapshotArtifactWriteReport> {
    let identity = artifact_identity(report)?;
    validate_snapshot_source_identity(&report.snapshot, &identity)?;
    let bytes = encode_snapshot_with_identity(&report.snapshot, &identity)?;
    validate_mmap_expected(&bytes, Some(&identity))?;

    if let Some(parent) = artifact_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(artifact_path)?;
    let write_result = (|| {
        output.write_all(&bytes)?;
        output.sync_all()?;
        let mut permissions = output.metadata()?.permissions();
        permissions.set_readonly(true);
        output.set_permissions(permissions)?;
        output.sync_all()?;
        drop(output);

        let persisted = fs::read(artifact_path)?;
        validate_mmap_expected(&persisted, Some(&identity))?;
        let after = artifact_identity(report)?;
        if after != identity {
            return Err(invalid_data("X1 inputs changed while writing generation"));
        }
        Ok(())
    })();
    if let Err(source) = write_result {
        let _ = fs::remove_file(artifact_path);
        return Err(source);
    }

    Ok(HbkFactSnapshotArtifactWriteReport {
        artifact_bytes: bytes.len() as u64,
        platform_version: identity.platform_version,
        source_sha256: identity.source_sha256,
        provider_sha256: identity.provider_sha256,
    })
}

fn artifact_identity(report: &HbkFactSnapshotBuildReport) -> io::Result<X1ArtifactIdentity> {
    let index = SearchIndex::open_read_only(&report.cache_index_path).map_err(search_as_io)?;
    let current_cache =
        super::binary_cache::CacheMetadata::from_index(&report.cache_index_path, &index)
            .map_err(search_as_io)?;
    if current_cache != report.cache_metadata {
        return Err(invalid_data(
            "X1 provider index changed after snapshot materialization",
        ));
    }
    if current_cache.provider_schema_version != SUPPORTED_PROVIDER_SCHEMA
        || current_cache.source_extraction_schema_version != SUPPORTED_EXTRACTION_SCHEMA
    {
        return Err(invalid_data("X1 input schema is unsupported"));
    }

    let stored = index.metadata().map_err(search_as_io)?;
    let source_path = fs::canonicalize(&stored.source_hbk)?;
    let platform_version = platform_version_from_source_path(&source_path)?;
    validate_fixed_field(&stored.locale, 8, "locale")?;
    validate_fixed_field(&stored.source_locale, 8, "source locale")?;
    validate_fixed_field(&platform_version, 16, "platform version")?;

    let source_metadata = fs::metadata(&source_path)?;
    let provider_metadata = fs::metadata(&report.cache_index_path)?;
    let source_sha256 = file_sha256(&source_path)?;
    let provider_sha256 = file_sha256(&report.cache_index_path)?;
    let provider_identity = stored
        .source_index_identity
        .unwrap_or_else(|| format!("sha256:{provider_sha256}"));
    let source_path = source_path
        .to_str()
        .ok_or_else(|| invalid_data("X1 source HBK path is not UTF-8"))?
        .to_string();

    Ok(X1ArtifactIdentity {
        source_path,
        source_bytes: source_metadata.len(),
        source_sha256,
        locale: stored.locale,
        source_locale: stored.source_locale,
        platform_version,
        provider_identity,
        provider_bytes: provider_metadata.len(),
        provider_sha256,
        provider_schema: current_cache.provider_schema_version,
        extraction_schema: current_cache.source_extraction_schema_version,
    })
}

fn search_as_io(error: SearchError) -> io::Error {
    io::Error::other(error)
}

fn validate_generation_metadata(path: &Path, metadata: &fs::Metadata) -> Result<(), SearchError> {
    if !metadata.file_type().is_file() {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 generation path is not a regular file",
        ));
    }
    if !metadata.permissions().readonly() {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 generation file is not read-only",
        ));
    }
    let len = metadata.len();
    if len < HEADER_LEN as u64 {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 artifact header is truncated",
        ));
    }
    if len > MAX_ARTIFACT_BYTES {
        return Err(snapshot_artifact_invalid(
            path,
            "X1 artifact exceeds maximum supported size",
        ));
    }
    Ok(())
}

fn validate_runtime_expectation(
    path: &Path,
    expected: &X1RuntimeExpectation,
    actual: &X1ArtifactIdentity,
) -> Result<(), SearchError> {
    validate_expected_field(
        path,
        "platform_version",
        &expected.platform_version,
        &actual.platform_version,
    )?;
    validate_expected_field(path, "locale", &expected.locale, &actual.locale)?;
    validate_expected_field(
        path,
        "source_locale",
        &expected.source_locale,
        &actual.source_locale,
    )?;
    validate_expected_field(
        path,
        "source_sha256",
        &expected.source_sha256,
        &actual.source_sha256,
    )
}

fn validate_expected_field(
    path: &Path,
    field: &'static str,
    expected: &str,
    actual: &str,
) -> Result<(), SearchError> {
    if expected == actual {
        return Ok(());
    }
    Err(SearchError::SnapshotArtifact {
        path: path.to_path_buf(),
        source: HbkFactSnapshotArtifactError::CompatibilityMismatch {
            field,
            expected: expected.to_string(),
            actual: actual.to_string(),
        },
    })
}

fn snapshot_artifact_invalid(path: &Path, message: &'static str) -> SearchError {
    SearchError::SnapshotArtifact {
        path: path.to_path_buf(),
        source: HbkFactSnapshotArtifactError::Invalid {
            message: message.to_string(),
        },
    }
}

fn artifact_error_from_io(source: io::Error) -> HbkFactSnapshotArtifactError {
    HbkFactSnapshotArtifactError::Invalid {
        message: source.to_string(),
    }
}

fn validate_snapshot_source_identity(
    snapshot: &HbkFactSnapshot,
    identity: &X1ArtifactIdentity,
) -> io::Result<()> {
    let canonical_source = Path::new(&identity.source_path);
    for entry in &snapshot.source_by_fact {
        let source_path = snapshot
            .strings
            .get(entry.source.hbk_path.0 as usize)
            .ok_or_else(|| invalid_data("X1 fact provenance HBK path is out of bounds"))?;
        if fs::canonicalize(source_path)? != canonical_source {
            return Err(invalid_data("X1 fact provenance HBK identity mismatch"));
        }
        let locale = snapshot
            .strings
            .get(entry.source.locale.0 as usize)
            .ok_or_else(|| invalid_data("X1 fact provenance locale is out of bounds"))?;
        if locale != &identity.locale && locale != &identity.source_locale {
            return Err(invalid_data("X1 fact provenance locale identity mismatch"));
        }
    }
    Ok(())
}

fn platform_version_from_source_path(path: &Path) -> io::Result<String> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| invalid_data("X1 source HBK file name is not UTF-8"))?;
    let locale = file_name
        .strip_prefix("shcntx_")
        .and_then(|value| value.strip_suffix(".hbk"))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid_data("X1 source file is not a shcntx_*.hbk help book"))?;
    if !locale
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(invalid_data("X1 source HBK locale suffix is invalid"));
    }
    let version = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| invalid_data("X1 source HBK has no platform version directory"))?;
    validate_platform_version(version)?;
    Ok(version.to_string())
}

#[derive(Default)]
struct X1RecordBuilder {
    platform_types: Vec<X1PlatformTypeHead>,
    type_members: Vec<X1TypeMemberHead>,
    callables: Vec<X1CallableHead>,
    globals: Vec<X1GlobalFactHead>,
    query_tables: Vec<X1QueryTableHead>,
    query_fields: Vec<X1QueryFieldHead>,
    query_parameters: Vec<X1QueryParameterHead>,
    language_facts: Vec<X1LanguageFactHead>,
    enums: Vec<X1EnumHead>,
    enum_values: Vec<X1EnumValueHead>,
    metadata_templates: Vec<X1MetadataTemplateHead>,
    signatures: Vec<X1SignatureHead>,
    parameters: Vec<X1ParameterHead>,
    type_refs: Vec<X1TypeRefHead>,
    template_bindings: Vec<X1TemplateBindingHead>,
    template_arguments: Vec<model::TemplateParameterBinding>,
    names: Vec<X1NameHead>,
    string_ids: Vec<StringId>,
    type_member_ranges: Vec<X1TypeMemberRangeHot>,
    member_availability_hot: Vec<X1AvailableMemberHot>,
    global_availability_locators: Vec<u32>,
    global_availability_masks: Vec<u16>,
    global_availability_kinds: Vec<u8>,
}

impl X1RecordBuilder {
    fn build(snapshot: &HbkFactSnapshot) -> io::Result<Self> {
        let mut output = Self::default();
        for value in &snapshot.platform_types {
            let metadata_template = value
                .metadata_template
                .as_ref()
                .map(|template| output.push_metadata_template(template))
                .transpose()?
                .unwrap_or(NONE_U32);
            let availability_contexts =
                output.push_string_ids(&value.availability_contexts, "X1 availability range")?;
            output.platform_types.push(X1PlatformTypeHead {
                id: value.id,
                name: X1NameHead::from(&value.name),
                metadata_template,
                type_template_key: X1TemplateKeyHead::from_option(value.type_template_key),
                availability_contexts,
            });
        }
        for value in &snapshot.type_members {
            let type_refs = output.push_type_refs(&value.type_refs)?;
            let availability_contexts =
                output.push_string_ids(&value.availability_contexts, "X1 availability range")?;
            output.type_members.push(X1TypeMemberHead {
                id: value.id,
                owner: value.owner,
                kind: value.kind,
                name: X1NameHead::from(&value.name),
                type_refs,
                availability_contexts,
            });
        }
        for value in &snapshot.callables {
            let signatures = output.push_signatures(&value.signatures)?;
            let return_type_refs = output.push_type_refs(&value.return_type_refs)?;
            let availability_contexts =
                output.push_string_ids(&value.availability_contexts, "X1 availability range")?;
            output.callables.push(X1CallableHead {
                id: value.id,
                owner: value.owner.map_or(NONE_U32, |id| id.0),
                kind: value.kind,
                name: X1NameHead::from(&value.name),
                signatures,
                return_type_refs,
                availability_contexts,
            });
        }
        for value in &snapshot.globals {
            let type_refs = output.push_type_refs(&value.type_refs)?;
            output.globals.push(X1GlobalFactHead {
                id: value.id,
                kind: value.kind,
                domain: value.domain,
                name: X1NameHead::from(&value.name),
                callable: value.callable.map_or(NONE_U32, |id| id.0),
                type_refs,
            });
        }
        for value in &snapshot.query_tables {
            let owner_path = output.push_names(&value.owner_path)?;
            let template_parameters = output
                .push_string_ids(&value.template_parameters, "X1 template-parameter range")?;
            let syntax = value
                .syntax
                .as_ref()
                .map(X1NameHead::from)
                .unwrap_or(X1NameHead {
                    primary: StringId(0),
                    alias: NONE_U32,
                });
            let role = match value.role {
                None => 0,
                Some(model::QueryTableRole::Primary) => 1,
                Some(model::QueryTableRole::Additional) => 2,
                Some(model::QueryTableRole::Unknown) => 3,
            };
            output.query_tables.push(X1QueryTableHead {
                id: value.id,
                name: X1NameHead::from(&value.name),
                syntax_present: value.syntax.is_some(),
                syntax,
                identifier: value.identifier.map_or(NONE_U32, |id| id.0),
                role,
                owner_path,
                template_parameters,
            });
        }
        for value in &snapshot.query_fields {
            let type_refs = output.push_type_refs(&value.type_refs)?;
            output.query_fields.push(X1QueryFieldHead {
                id: value.id,
                owner: value.owner,
                name: X1NameHead::from(&value.name),
                type_refs,
                note: value.note.map_or(NONE_U32, |id| id.0),
            });
        }
        for value in &snapshot.query_parameters {
            let type_refs = output.push_type_refs(&value.type_refs)?;
            output.query_parameters.push(X1QueryParameterHead {
                id: value.id,
                owner: value.owner,
                name: X1NameHead::from(&value.name),
                type_refs,
                default_value: value.default_value.map_or(NONE_U32, |id| id.0),
            });
        }
        for value in &snapshot.language_facts {
            let signatures = output.push_signatures(&value.signatures)?;
            let type_refs = output.push_type_refs(&value.type_refs)?;
            let return_type_refs = output.push_type_refs(&value.return_type_refs)?;
            output.language_facts.push(X1LanguageFactHead {
                id: value.id,
                kind: value.kind,
                domain: value.domain,
                name: X1NameHead::from(&value.name),
                signatures,
                type_refs,
                return_type_refs,
            });
        }
        output
            .enums
            .extend(snapshot.enums.iter().map(|value| X1EnumHead {
                id: value.id,
                name: X1NameHead::from(&value.name),
            }));
        output
            .enum_values
            .extend(snapshot.enum_values.iter().map(|value| X1EnumValueHead {
                id: value.id,
                owner: value.owner,
                name: X1NameHead::from(&value.name),
            }));
        output.type_member_ranges = build_x1_type_member_ranges(snapshot)?;
        output.member_availability_hot = build_x1_member_availability_hot(snapshot)?;
        let global_hot = build_x1_global_availability_hot(snapshot)?;
        output.global_availability_locators = global_hot.locators;
        output.global_availability_masks = global_hot.masks;
        output.global_availability_kinds = global_hot.kinds;
        Ok(output)
    }

    fn push_metadata_template(&mut self, value: &HbkMetadataTemplate) -> io::Result<u32> {
        let template_parameters =
            self.push_string_ids(&value.template_parameters, "X1 metadata-template range")?;
        let index = u32::try_from(self.metadata_templates.len())
            .map_err(|_| invalid_data("X1 metadata-template arena exceeds u32"))?;
        self.metadata_templates.push(X1MetadataTemplateHead {
            metadata_kind: value.metadata_kind,
            template_parameters,
        });
        Ok(index)
    }

    fn push_signatures(&mut self, values: &[HbkSignature]) -> io::Result<X1Range> {
        let start = self.signatures.len();
        for value in values {
            let parameters = self.push_parameters(&value.parameters)?;
            let return_type_refs = self.push_type_refs(&value.return_type_refs)?;
            self.signatures.push(X1SignatureHead {
                text: value.text,
                parameters,
                return_type_refs,
            });
        }
        X1Range::from_bounds(
            start,
            self.signatures.len(),
            "X1 signature range exceeds u32",
        )
    }

    fn push_parameters(&mut self, values: &[HbkParameter]) -> io::Result<X1Range> {
        let start = self.parameters.len();
        for value in values {
            let type_refs = self.push_type_refs(&value.type_refs)?;
            self.parameters.push(X1ParameterHead {
                name: value.name,
                required: value.required,
                type_refs,
            });
        }
        X1Range::from_bounds(
            start,
            self.parameters.len(),
            "X1 parameter range exceeds u32",
        )
    }

    fn push_type_refs(&mut self, values: &[HbkTypeRef]) -> io::Result<X1Range> {
        let start = self.type_refs.len();
        for value in values {
            let (target_tag, target_ok, ambiguous_targets) = match &value.target {
                HbkTypeRefTarget::Ok(id) => (0, id.0, X1Range::EMPTY),
                HbkTypeRefTarget::Unresolved => (1, NONE_U32, X1Range::EMPTY),
                HbkTypeRefTarget::Ambiguous(ids) => (
                    2,
                    NONE_U32,
                    self.push_string_ids(ids, "X1 ambiguous-target range")?,
                ),
            };
            let template_binding = value
                .template_binding
                .as_ref()
                .map(|binding| self.push_template_binding(binding))
                .transpose()?
                .unwrap_or(NONE_U32);
            self.type_refs.push(X1TypeRefHead {
                name: value.name,
                target_tag,
                target_ok,
                ambiguous_targets,
                type_template_key: X1TemplateKeyHead::from_option(value.type_template_key),
                template_binding,
            });
        }
        X1Range::from_bounds(start, self.type_refs.len(), "X1 type-ref range exceeds u32")
    }

    fn push_template_binding(&mut self, value: &HbkTypeTemplateBinding) -> io::Result<u32> {
        let start = self.template_arguments.len();
        self.template_arguments
            .extend(value.arguments.iter().cloned());
        let arguments = X1Range::from_bounds(
            start,
            self.template_arguments.len(),
            "X1 template-argument range exceeds u32",
        )?;
        let index = u32::try_from(self.template_bindings.len())
            .map_err(|_| invalid_data("X1 template-binding arena exceeds u32"))?;
        self.template_bindings.push(X1TemplateBindingHead {
            template_key: X1TemplateKeyHead::from_option(Some(value.template_key)),
            arguments,
        });
        Ok(index)
    }

    fn push_names(&mut self, values: &[HbkName]) -> io::Result<X1Range> {
        let start = self.names.len();
        self.names.extend(values.iter().map(X1NameHead::from));
        X1Range::from_bounds(start, self.names.len(), "X1 name range exceeds u32")
    }

    fn push_string_ids(&mut self, values: &[StringId], what: &'static str) -> io::Result<X1Range> {
        let start = self.string_ids.len();
        self.string_ids.extend_from_slice(values);
        X1Range::from_bounds(start, self.string_ids.len(), what)
    }
}

fn build_x1_type_member_ranges(
    snapshot: &HbkFactSnapshot,
) -> io::Result<Vec<X1TypeMemberRangeHot>> {
    let mut output = Vec::with_capacity(snapshot.platform_types.len());
    let mut key_index = 0usize;
    let mut cursor = 0usize;
    for owner_index in 0..snapshot.platform_types.len() {
        let owner = HbkPlatformTypeId(owner_index as u32);
        let (start, end) = if snapshot.members_by_owner.keys.get(key_index) == Some(&owner) {
            let start = snapshot.members_by_owner.offsets[key_index] as usize;
            let end = snapshot.members_by_owner.offsets[key_index + 1] as usize;
            key_index += 1;
            (start, end)
        } else {
            (cursor, cursor)
        };
        if start != cursor {
            return Err(invalid_data(
                "X1 owner-major member ranges are not contiguous",
            ));
        }
        output.push(X1TypeMemberRangeHot {
            member_start: u32::try_from(start)
                .map_err(|_| invalid_data("X1 member range start exceeds u32"))?,
            member_count: u32::try_from(end - start)
                .map_err(|_| invalid_data("X1 member range count exceeds u32"))?,
        });
        cursor = end;
    }
    if key_index != snapshot.members_by_owner.keys.len()
        || cursor != snapshot.members_by_owner.values.len()
    {
        return Err(invalid_data(
            "X1 type/member ranges do not cover owner-major members",
        ));
    }
    Ok(output)
}

fn build_x1_member_availability_hot(
    snapshot: &HbkFactSnapshot,
) -> io::Result<Vec<X1AvailableMemberHot>> {
    let mut output = Vec::with_capacity(snapshot.members_by_owner.values.len());
    for member_id in &snapshot.members_by_owner.values {
        let member = snapshot
            .type_members
            .get(member_id.0 as usize)
            .ok_or_else(|| invalid_data("X1 member hot source id out of bounds"))?;
        output.push(X1AvailableMemberHot {
            member_id: member_id.0,
            availability_word: x1_availability_word(snapshot, &member.availability_contexts)?,
            kind: x1_member_kind_tag(member.kind),
            reserved: 0,
        });
    }
    Ok(output)
}

struct X1GlobalAvailabilityHot {
    locators: Vec<u32>,
    masks: Vec<u16>,
    kinds: Vec<u8>,
}

fn build_x1_global_availability_hot(
    snapshot: &HbkFactSnapshot,
) -> io::Result<X1GlobalAvailabilityHot> {
    let mut locators = Vec::with_capacity(snapshot.globals.len());
    let mut masks = Vec::with_capacity(snapshot.globals.len());
    let mut kinds = Vec::with_capacity(snapshot.globals.len());
    for (index, global) in snapshot.globals.iter().enumerate() {
        let id = HbkGlobalFactId(index as u32);
        locators.push(id.0);
        masks.push(x1_availability_word(
            snapshot,
            snapshot.availability_by_fact.values(HbkFactRef::Global(id)),
        )?);
        kinds.push(x1_global_kind_tag(global.kind));
    }
    Ok(X1GlobalAvailabilityHot {
        locators,
        masks,
        kinds,
    })
}

fn x1_availability_word(snapshot: &HbkFactSnapshot, contexts: &[StringId]) -> io::Result<u16> {
    if contexts.is_empty() {
        return Ok(X1_CONTEXT_BITS);
    }
    let mut word = X1_HAS_EXPLICIT_DECLARATION;
    for context in contexts {
        let code = snapshot.string(*context);
        let Some(bit) = x1_context_code_bit(code) else {
            return Err(invalid_data("unknown X1 explicit availability context"));
        };
        word |= bit;
    }
    if word & X1_CONTEXT_BITS == 0 {
        return Err(invalid_data("empty X1 explicit availability context"));
    }
    Ok(word)
}

fn x1_context_code_bit(code: &str) -> Option<u16> {
    match code {
        "thin_client" => Some(1 << 0),
        "web_client" => Some(1 << 1),
        "mobile_client" => Some(1 << 2),
        "server" => Some(1 << 3),
        "thick_client" => Some(1 << 4),
        "external_connection" => Some(1 << 5),
        "mobile_application_client" => Some(1 << 6),
        "mobile_application_server" => Some(1 << 7),
        "mobile_standalone_server" => Some(1 << 8),
        _ => None,
    }
}

fn x1_member_kind_tag(kind: HbkTypeMemberKind) -> u8 {
    match kind {
        HbkTypeMemberKind::Property => 0,
        HbkTypeMemberKind::Method => 1,
        HbkTypeMemberKind::Event => 2,
        HbkTypeMemberKind::EnumValue => 3,
    }
}

fn x1_member_kind_from_tag(kind: u8) -> io::Result<HbkTypeMemberKind> {
    match kind {
        0 => Ok(HbkTypeMemberKind::Property),
        1 => Ok(HbkTypeMemberKind::Method),
        2 => Ok(HbkTypeMemberKind::Event),
        3 => Ok(HbkTypeMemberKind::EnumValue),
        _ => Err(invalid_data("invalid X1 member kind tag")),
    }
}

fn x1_global_kind_tag(kind: HbkGlobalFactKind) -> u8 {
    match kind {
        HbkGlobalFactKind::Method => 0,
        HbkGlobalFactKind::Property => 1,
    }
}

fn x1_global_kind_from_tag(kind: u8) -> io::Result<HbkGlobalFactKind> {
    match kind {
        0 => Ok(HbkGlobalFactKind::Method),
        1 => Ok(HbkGlobalFactKind::Property),
        _ => Err(invalid_data("invalid X1 global kind tag")),
    }
}

fn encode_snapshot_with_identity(
    snapshot: &HbkFactSnapshot,
    identity: &X1ArtifactIdentity,
) -> io::Result<Vec<u8>> {
    let records = X1RecordBuilder::build(snapshot)?;
    let strings = normalized_snapshot_strings(snapshot, identity)?;
    let mut sections = Vec::with_capacity(SECTION_COUNT);
    push_strings(&mut sections, &strings)?;
    let mut string_order: Vec<_> = (0..strings.len())
        .map(|index| StringId(index as u32))
        .collect();
    string_order
        .sort_unstable_by(|left, right| strings[left.0 as usize].cmp(&strings[right.0 as usize]));
    push_vec(&mut sections, &string_order)?;
    push_vec(&mut sections, &records.platform_types)?;
    push_vec(&mut sections, &records.type_members)?;
    push_vec(&mut sections, &records.callables)?;
    push_vec(&mut sections, &records.globals)?;
    push_vec(&mut sections, &records.query_tables)?;
    push_vec(&mut sections, &records.query_fields)?;
    push_vec(&mut sections, &records.query_parameters)?;
    push_vec(&mut sections, &records.language_facts)?;
    push_vec(&mut sections, &records.enums)?;
    push_vec(&mut sections, &records.enum_values)?;
    push_vec(&mut sections, &records.metadata_templates)?;
    push_vec(&mut sections, &records.signatures)?;
    push_vec(&mut sections, &records.parameters)?;
    push_vec(&mut sections, &records.type_refs)?;
    push_vec(&mut sections, &records.template_bindings)?;
    push_vec(&mut sections, &records.template_arguments)?;
    push_vec(&mut sections, &records.names)?;
    push_vec(&mut sections, &records.string_ids)?;
    push_vec(&mut sections, &snapshot.fact_ids)?;
    push_vec(&mut sections, &snapshot.platform_type_ids)?;
    push_vec(&mut sections, &snapshot.platform_type_names)?;
    push_vec(&mut sections, &snapshot.platform_type_templates)?;
    push_vec(&mut sections, &snapshot.member_ids)?;
    push_csr(&mut sections, &snapshot.members_by_owner)?;
    push_vec(&mut sections, &records.type_member_ranges)?;
    push_vec(&mut sections, &records.member_availability_hot)?;
    push_vec(&mut sections, &records.global_availability_locators)?;
    push_vec(&mut sections, &records.global_availability_masks)?;
    push_vec(&mut sections, &records.global_availability_kinds)?;
    push_vec(&mut sections, &snapshot.members_by_owner_name)?;
    push_vec(&mut sections, &snapshot.members_by_owner_name_kind)?;
    push_vec(&mut sections, &snapshot.callable_ids)?;
    push_csr(&mut sections, &snapshot.callables_by_owner)?;
    push_vec(&mut sections, &snapshot.callables_by_owner_name)?;
    push_csr(&mut sections, &snapshot.constructors_by_type)?;
    push_vec(&mut sections, &snapshot.global_names)?;
    push_vec(&mut sections, &snapshot.globals_by_domain_name_kind)?;
    push_vec(&mut sections, &snapshot.module_event_names)?;
    push_vec(
        &mut sections,
        &snapshot.module_contexts_by_domain_language_kind,
    )?;
    push_vec(&mut sections, &snapshot.query_table_ids)?;
    push_vec(&mut sections, &snapshot.query_table_names)?;
    push_vec(&mut sections, &snapshot.query_table_syntax_names)?;
    push_vec(&mut sections, &snapshot.query_table_identifiers)?;
    push_csr(&mut sections, &snapshot.query_fields_by_table)?;
    push_vec(&mut sections, &snapshot.query_fields_by_table_name)?;
    push_csr(&mut sections, &snapshot.query_parameters_by_table)?;
    push_vec(&mut sections, &snapshot.query_parameters_by_table_name)?;
    push_vec(&mut sections, &snapshot.language_ids)?;
    push_vec(&mut sections, &snapshot.language_names)?;
    push_vec(&mut sections, &snapshot.enum_ids)?;
    push_vec(&mut sections, &snapshot.enum_names)?;
    push_vec(&mut sections, &snapshot.enum_value_ids)?;
    push_csr(&mut sections, &snapshot.enum_values_by_enum)?;
    push_vec(&mut sections, &snapshot.enum_values_by_enum_name)?;
    push_csr(&mut sections, &snapshot.availability_by_fact)?;
    push_vec(&mut sections, &snapshot.availability_since_by_fact)?;
    push_vec(&mut sections, &snapshot.source_by_fact)?;
    push_csr(&mut sections, &snapshot.relations_by_source_kind)?;
    push_vec(
        &mut sections,
        &build_x1_platform_type_name_hash(&snapshot.platform_type_names, snapshot)?,
    )?;
    let metadata = vec![
        identity.source_path.clone(),
        identity.source_bytes.to_string(),
        identity.source_sha256.clone(),
        identity.locale.clone(),
        identity.source_locale.clone(),
        identity.platform_version.clone(),
        identity.provider_identity.clone(),
        identity.provider_bytes.to_string(),
        identity.provider_sha256.clone(),
        identity.provider_schema.to_string(),
        identity.extraction_schema.to_string(),
        BACKEND_ID.to_string(),
        RECORD_LAYOUT.to_string(),
    ];
    push_strings(&mut sections, &metadata)?;
    debug_assert_eq!(sections.len(), SECTION_COUNT);

    let directory_offset = HEADER_LEN;
    let payload_offset = directory_offset + SECTION_COUNT * DIRECTORY_ENTRY_LEN;
    let mut directory = Vec::with_capacity(SECTION_COUNT);
    let mut payload = Vec::new();
    for section in &sections {
        let absolute_offset = payload_offset + payload.len();
        let padding = (SECTION_ALIGNMENT - absolute_offset % SECTION_ALIGNMENT) % SECTION_ALIGNMENT;
        payload.resize(payload.len() + padding, 0);
        directory.push((
            payload_offset as u64 + payload.len() as u64,
            section.len() as u64,
        ));
        payload.extend_from_slice(section);
    }
    let checksum = fnv1a(&payload);
    let counts = snapshot.counts();
    let source_locale = snapshot
        .source_locale
        .map_or(u64::MAX, |id| u64::from(id.0));
    let artifact_len = payload_offset
        .checked_add(payload.len())
        .and_then(|len| u32::try_from(len).ok())
        .ok_or_else(|| invalid_data("X1 artifact length exceeds u32"))?;
    let mut output = Vec::with_capacity(artifact_len as usize);
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&LAYOUT_VERSION.to_le_bytes());
    output.extend_from_slice(&identity.extraction_schema.to_le_bytes());
    output.extend_from_slice(&identity.provider_schema.to_le_bytes());
    output.extend_from_slice(&LAYOUT_FLAGS.to_le_bytes());
    output.extend_from_slice(&(SECTION_COUNT as u32).to_le_bytes());
    output.extend_from_slice(&(directory_offset as u32).to_le_bytes());
    output.extend_from_slice(&checksum.to_le_bytes());
    output.extend_from_slice(&(source_locale as u32).to_le_bytes());
    output.extend_from_slice(&artifact_len.to_le_bytes());
    output.extend_from_slice(&identity.source_bytes.to_le_bytes());
    output.extend_from_slice(&identity.provider_bytes.to_le_bytes());
    output.extend_from_slice(identity.source_sha256.as_bytes());
    output.extend_from_slice(identity.provider_sha256.as_bytes());
    write_fixed_ascii(&mut output, &identity.locale, 8)?;
    write_fixed_ascii(&mut output, &identity.platform_version, 16)?;
    debug_assert_eq!(output.len(), HEADER_LEN);
    for (offset, len) in directory {
        output.extend_from_slice(&offset.to_le_bytes());
        output.extend_from_slice(&len.to_le_bytes());
    }
    output.extend_from_slice(&payload);
    validate_mmap_expected(&output, Some(identity))?;
    if counts != validate_counts(&output)? {
        return Err(invalid_data("X1 encoded counts mismatch"));
    }
    Ok(output)
}

fn normalized_snapshot_strings(
    snapshot: &HbkFactSnapshot,
    identity: &X1ArtifactIdentity,
) -> io::Result<Vec<String>> {
    let mut strings = snapshot.strings.clone();
    for entry in &snapshot.source_by_fact {
        let value = strings
            .get_mut(entry.source.hbk_path.0 as usize)
            .ok_or_else(|| invalid_data("X1 fact provenance HBK path is out of bounds"))?;
        *value = identity.source_path.clone();
    }
    Ok(strings)
}

#[cfg(test)]
fn encode_snapshot(snapshot: &HbkFactSnapshot) -> io::Result<Vec<u8>> {
    let identity = X1ArtifactIdentity {
        source_path: "/tmp/8.3.0.0/fixture.hbk".to_string(),
        source_bytes: 7,
        source_sha256: "0".repeat(64),
        locale: "ru".to_string(),
        source_locale: "ru".to_string(),
        platform_version: "8.3.0.0".to_string(),
        provider_identity: "fixture-provider".to_string(),
        provider_bytes: 11,
        provider_sha256: "1".repeat(64),
        provider_schema: SUPPORTED_PROVIDER_SCHEMA,
        extraction_schema: SUPPORTED_EXTRACTION_SCHEMA,
    };
    encode_snapshot_with_identity(snapshot, &identity)
}

fn push_vec<T: BinaryValue>(sections: &mut Vec<Vec<u8>>, values: &[T]) -> io::Result<()> {
    let mut payload = Vec::new();
    let mut offsets = Vec::with_capacity(values.len() + 1);
    offsets.push(0u32);
    let mut fixed_stride = None;
    for value in values {
        let mut writer = BinaryWriter::new(Vec::new());
        value.write_to(&mut writer)?;
        let encoded = writer.into_inner();
        fixed_stride = match fixed_stride {
            None => Some(encoded.len()),
            Some(stride) if stride == encoded.len() => Some(stride),
            Some(_) => Some(0),
        };
        payload.extend_from_slice(&encoded);
        offsets.push(
            u32::try_from(payload.len())
                .map_err(|_| invalid_data("X1 vector payload exceeds u32 range"))?,
        );
    }
    let count =
        u32::try_from(values.len()).map_err(|_| invalid_data("X1 vector count exceeds u32"))?;
    let stride = fixed_stride.filter(|stride| *stride > 0).unwrap_or(0);
    let stride = u32::try_from(stride).map_err(|_| invalid_data("X1 vector stride exceeds u32"))?;
    let offset_bytes = if stride == 0 { offsets.len() * 4 } else { 0 };
    let mut section = Vec::with_capacity(8 + offset_bytes + payload.len());
    section.extend_from_slice(&count.to_le_bytes());
    section.extend_from_slice(&stride.to_le_bytes());
    if stride == 0 {
        for offset in offsets {
            section.extend_from_slice(&offset.to_le_bytes());
        }
    }
    section.extend_from_slice(&payload);
    sections.push(section);
    Ok(())
}

fn push_strings(sections: &mut Vec<Vec<u8>>, values: &[String]) -> io::Result<()> {
    let count =
        u32::try_from(values.len()).map_err(|_| invalid_data("X1 string count exceeds u32"))?;
    let mut payload = Vec::new();
    let mut offsets = Vec::with_capacity(values.len() + 1);
    offsets.push(0_u32);
    for value in values {
        payload.extend_from_slice(value.as_bytes());
        offsets.push(
            u32::try_from(payload.len())
                .map_err(|_| invalid_data("X1 string payload exceeds u32 range"))?,
        );
    }
    let mut section = Vec::with_capacity(8 + offsets.len() * 4 + payload.len());
    section.extend_from_slice(&count.to_le_bytes());
    section.extend_from_slice(&0_u32.to_le_bytes());
    for offset in offsets {
        section.extend_from_slice(&offset.to_le_bytes());
    }
    section.extend_from_slice(&payload);
    sections.push(section);
    Ok(())
}

fn write_fixed_ascii(output: &mut Vec<u8>, value: &str, width: usize) -> io::Result<()> {
    if !value.is_ascii() || value.len() > width {
        return Err(invalid_data("X1 fixed ASCII value exceeds header field"));
    }
    output.extend_from_slice(value.as_bytes());
    output.resize(output.len() + width - value.len(), 0);
    Ok(())
}

fn push_csr<K: BinaryValue, V: BinaryValue>(
    sections: &mut Vec<Vec<u8>>,
    index: &CsrIndex<K, V>,
) -> io::Result<()> {
    push_vec(sections, &index.keys)?;
    push_vec(sections, &index.offsets)?;
    push_vec(sections, &index.values)
}

fn build_x1_platform_type_name_hash(
    names: &[NameLookup<HbkPlatformTypeId>],
    snapshot: &HbkFactSnapshot,
) -> io::Result<Vec<X1TypeNameHashBucket>> {
    build_x1_platform_type_name_hash_with_hasher(names, snapshot, x1_hash_key)
}

fn build_x1_platform_type_name_hash_with_hasher(
    names: &[NameLookup<HbkPlatformTypeId>],
    snapshot: &HbkFactSnapshot,
    hash_key: impl Fn(&str) -> u64,
) -> io::Result<Vec<X1TypeNameHashBucket>> {
    let key_count = names
        .iter()
        .fold((0usize, None), |(count, previous), record| {
            let key = snapshot.string(record.key);
            (count + usize::from(previous != Some(key)), Some(key))
        })
        .0;
    let capacity = x1_hash_capacity(key_count);
    let mut buckets = vec![
        X1TypeNameHashBucket {
            hash: 0,
            start: 0,
            count: 0,
        };
        capacity
    ];
    let mut start = 0usize;
    while start < names.len() {
        let key = snapshot.string(names[start].key);
        let mut end = start + 1;
        while end < names.len() && snapshot.string(names[end].key) == key {
            end += 1;
        }
        let hash = hash_key(key);
        let bucket = x1_insert_hash_bucket(&buckets, names, snapshot, hash, key)?;
        if buckets[bucket].count != 0 {
            return Err(invalid_data("X1 platform type-name hash duplicate key"));
        }
        buckets[bucket] = X1TypeNameHashBucket {
            hash,
            start: u32::try_from(start)
                .map_err(|_| invalid_data("X1 platform type-name hash start exceeds u32"))?,
            count: u32::try_from(end - start)
                .map_err(|_| invalid_data("X1 platform type-name hash count exceeds u32"))?,
        };
        start = end;
    }
    Ok(buckets)
}

fn x1_hash_capacity(key_count: usize) -> usize {
    let needed = key_count.saturating_mul(2).max(2);
    needed.next_power_of_two()
}

fn x1_insert_hash_bucket(
    buckets: &[X1TypeNameHashBucket],
    names: &[NameLookup<HbkPlatformTypeId>],
    snapshot: &HbkFactSnapshot,
    hash: u64,
    key: &str,
) -> io::Result<usize> {
    let mask = buckets.len() - 1;
    let mut index = (hash as usize) & mask;
    for _ in 0..64 {
        let bucket = buckets[index];
        if bucket.count == 0 {
            return Ok(index);
        }
        if bucket.hash == hash && x1_build_bucket_key_matches(names, snapshot, bucket, key)? {
            return Ok(index);
        }
        index = (index + 1) & mask;
    }
    Err(invalid_data(
        "X1 platform type-name hash max probe exceeded",
    ))
}

fn x1_build_bucket_key_matches(
    names: &[NameLookup<HbkPlatformTypeId>],
    snapshot: &HbkFactSnapshot,
    bucket: X1TypeNameHashBucket,
    key: &str,
) -> io::Result<bool> {
    let start = bucket.start as usize;
    let end = start
        .checked_add(bucket.count as usize)
        .ok_or_else(|| invalid_data("X1 platform type-name hash range overflow"))?;
    if start >= end || end > names.len() {
        return Err(invalid_data(
            "X1 platform type-name hash range out of bounds",
        ));
    }
    Ok(snapshot.string(names[start].key) == key)
}

fn x1_hash_key(key: &str) -> u64 {
    let hash = fnv1a(key.as_bytes());
    if hash == 0 { 1 } else { hash }
}

fn validate_mmap(
    bytes: &[u8],
) -> io::Result<(
    Vec<Section>,
    HbkFactSnapshotCounts,
    Option<StringId>,
    X1ArtifactIdentity,
)> {
    validate_mmap_expected(bytes, None)
}

fn validate_mmap_expected(
    bytes: &[u8],
    expected_identity: Option<&X1ArtifactIdentity>,
) -> io::Result<(
    Vec<Section>,
    HbkFactSnapshotCounts,
    Option<StringId>,
    X1ArtifactIdentity,
)> {
    if bytes.len() as u64 > MAX_ARTIFACT_BYTES {
        return Err(invalid_data("X1 artifact exceeds maximum supported size"));
    }
    if bytes.len() < HEADER_LEN {
        return Err(invalid_data("X1 artifact header is truncated"));
    }
    if &bytes[0..8] != MAGIC {
        return Err(invalid_data("invalid X1 magic"));
    }
    let layout = read_u32_at(bytes, 8)?;
    if layout != LAYOUT_VERSION {
        return Err(invalid_data("unsupported X1 layout version"));
    }
    let schema = read_u32_at(bytes, 12)?;
    if schema != SUPPORTED_EXTRACTION_SCHEMA {
        return Err(invalid_data("unsupported X1 extraction schema"));
    }
    let provider_schema = read_u32_at(bytes, 16)?;
    if provider_schema != SUPPORTED_PROVIDER_SCHEMA {
        return Err(invalid_data("unsupported X1 provider schema"));
    }
    if read_u32_at(bytes, 20)? != LAYOUT_FLAGS {
        return Err(invalid_data("unsupported X1 layout flags"));
    }
    let section_count = read_u32_at(bytes, 24)? as usize;
    if section_count != SECTION_COUNT {
        return Err(invalid_data("invalid X1 section count"));
    }
    let directory_offset = read_u32_at(bytes, 28)? as usize;
    let checksum = read_u64_at(bytes, 32)?;
    let source_locale = match read_u32_at(bytes, 40)? {
        u32::MAX => None,
        value => Some(StringId(value)),
    };
    if read_u32_at(bytes, 44)? as usize != bytes.len() {
        return Err(invalid_data("X1 artifact length mismatch"));
    }
    let source_bytes = read_u64_at(bytes, 48)?;
    let provider_bytes = read_u64_at(bytes, 56)?;
    let source_sha256 = read_ascii(bytes, 64, 64, "source SHA-256")?;
    let provider_sha256 = read_ascii(bytes, 128, 64, "provider SHA-256")?;
    validate_sha256(&source_sha256)?;
    validate_sha256(&provider_sha256)?;
    let locale = read_fixed_ascii(bytes, 192, 8, "locale")?;
    let platform_version = read_fixed_ascii(bytes, 200, 16, "platform version")?;
    validate_platform_version(&platform_version)?;
    let directory_len = section_count
        .checked_mul(DIRECTORY_ENTRY_LEN)
        .ok_or_else(|| invalid_data("X1 directory length overflow"))?;
    let directory_end = directory_offset
        .checked_add(directory_len)
        .ok_or_else(|| invalid_data("X1 directory end overflow"))?;
    if directory_offset != HEADER_LEN || directory_end > bytes.len() {
        return Err(invalid_data("invalid X1 directory bounds"));
    }
    let mut sections = Vec::with_capacity(section_count);
    let mut payload_hash = FNV_OFFSET_BASIS;
    let mut expected_offset = directory_end;
    for index in 0..section_count {
        let entry = directory_offset + index * DIRECTORY_ENTRY_LEN;
        let offset = usize::try_from(read_u64_at(bytes, entry)?)
            .map_err(|_| invalid_data("X1 section offset does not fit usize"))?;
        let len = usize::try_from(read_u64_at(bytes, entry + 8)?)
            .map_err(|_| invalid_data("X1 section length does not fit usize"))?;
        let end = offset
            .checked_add(len)
            .ok_or_else(|| invalid_data("X1 section end overflow"))?;
        if offset < expected_offset || end > bytes.len() {
            return Err(invalid_data("invalid X1 section bounds"));
        }
        if offset % SECTION_ALIGNMENT != 0 {
            return Err(invalid_data("invalid X1 section alignment"));
        }
        if bytes[expected_offset..offset].iter().any(|byte| *byte != 0) {
            return Err(invalid_data("non-zero X1 section alignment padding"));
        }
        payload_hash = fnv1a_update(payload_hash, &bytes[expected_offset..offset]);
        payload_hash = fnv1a_update(payload_hash, &bytes[offset..end]);
        sections.push(Section { offset, len });
        expected_offset = end;
    }
    if expected_offset != bytes.len() {
        return Err(invalid_data("X1 artifact has trailing bytes"));
    }
    if payload_hash != checksum {
        return Err(invalid_data("X1 payload checksum mismatch"));
    }
    for section in &sections {
        VectorView::new(&bytes[section.offset..section.offset + section.len])?.validate()?;
    }
    let metadata = VectorView::new(section_bytes(bytes, &sections, S::CompatibilityMetadata))?;
    if metadata.len() != 13 {
        return Err(invalid_data("X1 compatibility metadata mismatch"));
    }
    let actual_identity = X1ArtifactIdentity {
        source_path: metadata.get_str(0)?.to_string(),
        source_bytes: parse_metadata_u64(metadata.get_str(1)?)?,
        source_sha256: metadata.get_str(2)?.to_string(),
        locale: metadata.get_str(3)?.to_string(),
        source_locale: metadata.get_str(4)?.to_string(),
        platform_version: metadata.get_str(5)?.to_string(),
        provider_identity: metadata.get_str(6)?.to_string(),
        provider_bytes: parse_metadata_u64(metadata.get_str(7)?)?,
        provider_sha256: metadata.get_str(8)?.to_string(),
        provider_schema: parse_metadata_u32(metadata.get_str(9)?)?,
        extraction_schema: parse_metadata_u32(metadata.get_str(10)?)?,
    };
    validate_fixed_field(&actual_identity.source_locale, 8, "source locale")?;
    if metadata.get_str(11)? != BACKEND_ID || metadata.get_str(12)? != RECORD_LAYOUT {
        return Err(invalid_data("X1 compatibility metadata mismatch"));
    }
    if actual_identity.source_bytes != source_bytes
        || actual_identity.provider_bytes != provider_bytes
        || actual_identity.source_sha256 != source_sha256
        || actual_identity.provider_sha256 != provider_sha256
        || actual_identity.locale != locale
        || actual_identity.platform_version != platform_version
        || actual_identity.provider_schema != provider_schema
        || actual_identity.extraction_schema != schema
        || expected_identity.is_some_and(|expected| expected != &actual_identity)
    {
        return Err(invalid_data("X1 compatibility metadata mismatch"));
    }
    let counts = counts_from_sections(bytes, &sections)?;
    let strings = VectorView::new(section_bytes(bytes, &sections, S::Strings))?;
    let source_locale = source_locale
        .ok_or_else(|| invalid_data("X1 source locale dictionary reference is missing"))?;
    if strings.get_str(source_locale.0 as usize)? != actual_identity.source_locale {
        return Err(invalid_data("X1 source locale identity mismatch"));
    }
    validate_typed_sections(bytes, &sections, counts)?;
    validate_template_binding_semantics(bytes, &sections, counts)?;
    validate_fact_source_identity(bytes, &sections, &actual_identity)?;
    Ok((sections, counts, Some(source_locale), actual_identity))
}

fn validate_counts(bytes: &[u8]) -> io::Result<HbkFactSnapshotCounts> {
    let (sections, counts, _, _) = validate_mmap(bytes)?;
    let _ = sections;
    Ok(counts)
}

fn counts_from_sections(bytes: &[u8], sections: &[Section]) -> io::Result<HbkFactSnapshotCounts> {
    let len =
        |section| VectorView::new(section_bytes(bytes, sections, section)).map(|view| view.len());
    Ok(HbkFactSnapshotCounts {
        strings: len(S::Strings)?,
        platform_types: len(S::PlatformTypes)?,
        type_members: len(S::TypeMembers)?,
        callables: len(S::Callables)?,
        globals: len(S::Globals)?,
        query_tables: len(S::QueryTables)?,
        query_fields: len(S::QueryFields)?,
        query_parameters: len(S::QueryParameters)?,
        language_facts: len(S::LanguageFacts)?,
        enums: len(S::Enums)?,
        enum_values: len(S::EnumValues)?,
    })
}

fn validate_typed_sections(
    bytes: &[u8],
    sections: &[Section],
    counts: HbkFactSnapshotCounts,
) -> io::Result<()> {
    let strings = VectorView::new(section_bytes(bytes, sections, S::Strings))?;
    for index in 0..strings.len() {
        let _ = strings.get_str(index)?;
    }
    let string = |id: StringId| validate_index(id.0, counts.strings, "string id");
    let fact = |value: HbkFactRef| validate_fact_ref(value, counts);

    let string_order = VectorView::new(section_bytes(bytes, sections, S::StringOrder))?;
    if string_order.len() != strings.len() {
        return Err(invalid_data("X1 string order count mismatch"));
    }
    let mut previous = None;
    for index in 0..string_order.len() {
        let id = string_order.get::<StringId>(index)?;
        string(id)?;
        let text = strings.get_str(id.0 as usize)?;
        if previous.is_some_and(|previous| previous >= text) {
            return Err(invalid_data("X1 string order is not strictly sorted"));
        }
        previous = Some(text);
    }

    let arena_len =
        |section| VectorView::new(section_bytes(bytes, sections, section)).map(|view| view.len());
    let string_ids_len = arena_len(S::StringIds)?;
    let names_len = arena_len(S::Names)?;
    let metadata_templates_len = arena_len(S::MetadataTemplates)?;
    let signatures_len = arena_len(S::Signatures)?;
    let parameters_len = arena_len(S::Parameters)?;
    let type_refs_len = arena_len(S::TypeRefs)?;
    let template_bindings_len = arena_len(S::TemplateBindings)?;
    let template_arguments_len = arena_len(S::TemplateArguments)?;
    let validate_name_head = |value: X1NameHead| -> io::Result<()> {
        string(value.primary)?;
        validate_optional_string(value.alias, &string)
    };
    let validate_template_key_head = |value: X1TemplateKeyHead| -> io::Result<()> {
        if value.family == NONE_U32 || value.variant == NONE_U32 {
            if value != X1TemplateKeyHead::NONE {
                return Err(invalid_data("X1 partial optional template key"));
            }
            return Ok(());
        }
        string(StringId(value.family))?;
        string(StringId(value.variant))
    };
    let validate_string_range = |range: X1Range| -> io::Result<()> {
        validate_range(range, string_ids_len, "X1 string-id arena range")?;
        let view = VectorView::new(section_bytes(bytes, sections, S::StringIds))?;
        for index in range.as_usize()? {
            string(view.get(index)?)?;
        }
        Ok(())
    };
    let validate_name_range = |range: X1Range| -> io::Result<()> {
        validate_range(range, names_len, "X1 name arena range")?;
        let view = VectorView::new(section_bytes(bytes, sections, S::Names))?;
        for index in range.as_usize()? {
            validate_name_head(view.get(index)?)?;
        }
        Ok(())
    };

    validate_records::<X1MetadataTemplateHead>(bytes, sections, S::MetadataTemplates, |value| {
        string(value.metadata_kind)?;
        validate_string_range(value.template_parameters)
    })?;
    validate_records::<X1TemplateBindingHead>(bytes, sections, S::TemplateBindings, |value| {
        validate_template_key_head(value.template_key)?;
        if value.template_key == X1TemplateKeyHead::NONE {
            return Err(invalid_data("X1 template binding has no template key"));
        }
        validate_range(
            value.arguments,
            template_arguments_len,
            "X1 template-argument arena range",
        )
    })?;
    validate_records::<model::TemplateParameterBinding>(
        bytes,
        sections,
        S::TemplateArguments,
        |_| Ok(()),
    )?;
    validate_records::<X1TypeRefHead>(bytes, sections, S::TypeRefs, |value| {
        string(value.name)?;
        match value.target_tag {
            0 => {
                string(StringId(value.target_ok))?;
                if value.ambiguous_targets.len != 0 {
                    return Err(invalid_data("X1 ok type-ref has ambiguous targets"));
                }
            }
            1 => {
                if value.target_ok != NONE_U32 || value.ambiguous_targets.len != 0 {
                    return Err(invalid_data("X1 unresolved type-ref has target payload"));
                }
            }
            2 => {
                if value.target_ok != NONE_U32 {
                    return Err(invalid_data("X1 ambiguous type-ref has ok target"));
                }
                validate_string_range(value.ambiguous_targets)?;
            }
            _ => return Err(invalid_data("invalid X1 type-ref target tag")),
        }
        validate_template_key_head(value.type_template_key)?;
        validate_optional_index(
            value.template_binding,
            template_bindings_len,
            "X1 template-binding index",
        )
    })?;
    validate_records::<X1ParameterHead>(bytes, sections, S::Parameters, |value| {
        string(value.name)?;
        validate_range(
            value.type_refs,
            type_refs_len,
            "X1 parameter type-ref range",
        )
    })?;
    validate_records::<X1SignatureHead>(bytes, sections, S::Signatures, |value| {
        string(value.text)?;
        validate_range(
            value.parameters,
            parameters_len,
            "X1 signature parameter range",
        )?;
        validate_range(
            value.return_type_refs,
            type_refs_len,
            "X1 signature return-type range",
        )
    })?;
    validate_records::<X1NameHead>(bytes, sections, S::Names, |value| {
        validate_name_head(*value)
    })?;
    validate_records::<StringId>(bytes, sections, S::StringIds, |value| string(*value))?;

    validate_records::<X1PlatformTypeHead>(bytes, sections, S::PlatformTypes, |value| {
        string(value.id)?;
        validate_name_head(value.name)?;
        validate_optional_index(
            value.metadata_template,
            metadata_templates_len,
            "X1 metadata-template index",
        )?;
        validate_template_key_head(value.type_template_key)?;
        validate_string_range(value.availability_contexts)
    })?;
    validate_records::<X1TypeMemberHead>(bytes, sections, S::TypeMembers, |value| {
        string(value.id)?;
        validate_index(value.owner.0, counts.platform_types, "type-member owner")?;
        validate_name_head(value.name)?;
        validate_range(value.type_refs, type_refs_len, "X1 member type-ref range")?;
        validate_string_range(value.availability_contexts)
    })?;
    validate_records::<X1CallableHead>(bytes, sections, S::Callables, |value| {
        string(value.id)?;
        validate_optional_index(value.owner, counts.platform_types, "X1 callable owner")?;
        validate_name_head(value.name)?;
        validate_range(
            value.signatures,
            signatures_len,
            "X1 callable signature range",
        )?;
        validate_range(
            value.return_type_refs,
            type_refs_len,
            "X1 callable return-type range",
        )?;
        validate_string_range(value.availability_contexts)
    })?;
    validate_records::<X1GlobalFactHead>(bytes, sections, S::Globals, |value| {
        string(value.id)?;
        validate_name_head(value.name)?;
        validate_optional_index(value.callable, counts.callables, "X1 global callable")?;
        validate_range(value.type_refs, type_refs_len, "X1 global type-ref range")
    })?;
    validate_records::<X1QueryTableHead>(bytes, sections, S::QueryTables, |value| {
        string(value.id)?;
        validate_name_head(value.name)?;
        if value.syntax_present {
            validate_name_head(value.syntax)?;
        }
        validate_optional_string(value.identifier, &string)?;
        if value.role > 3 {
            return Err(invalid_data("invalid X1 query-table role tag"));
        }
        validate_name_range(value.owner_path)?;
        validate_string_range(value.template_parameters)
    })?;
    validate_records::<X1QueryFieldHead>(bytes, sections, S::QueryFields, |value| {
        string(value.id)?;
        validate_index(value.owner.0, counts.query_tables, "query-field owner")?;
        validate_name_head(value.name)?;
        validate_range(
            value.type_refs,
            type_refs_len,
            "X1 query-field type-ref range",
        )?;
        validate_optional_string(value.note, &string)
    })?;
    validate_records::<X1QueryParameterHead>(bytes, sections, S::QueryParameters, |value| {
        string(value.id)?;
        validate_index(value.owner.0, counts.query_tables, "query-parameter owner")?;
        validate_name_head(value.name)?;
        validate_range(
            value.type_refs,
            type_refs_len,
            "X1 query-parameter type-ref range",
        )?;
        validate_optional_string(value.default_value, &string)
    })?;
    validate_records::<X1LanguageFactHead>(bytes, sections, S::LanguageFacts, |value| {
        string(value.id)?;
        validate_name_head(value.name)?;
        validate_range(
            value.signatures,
            signatures_len,
            "X1 language signature range",
        )?;
        validate_range(value.type_refs, type_refs_len, "X1 language type-ref range")?;
        validate_range(
            value.return_type_refs,
            type_refs_len,
            "X1 language return-type range",
        )
    })?;
    validate_records::<X1EnumHead>(bytes, sections, S::Enums, |value| {
        string(value.id)?;
        validate_name_head(value.name)
    })?;
    validate_records::<X1EnumValueHead>(bytes, sections, S::EnumValues, |value| {
        string(value.id)?;
        validate_index(value.owner.0, counts.enums, "enum-value owner")?;
        validate_name_head(value.name)
    })?;

    validate_id_lookup::<HbkFactRef>(bytes, sections, S::FactIds, &string, &fact)?;
    validate_id_lookup::<HbkPlatformTypeId>(
        bytes,
        sections,
        S::PlatformTypeIds,
        &string,
        |value| validate_index(value.0, counts.platform_types, "platform-type id lookup"),
    )?;
    validate_name_lookup::<HbkPlatformTypeId>(
        bytes,
        sections,
        S::PlatformTypeNames,
        &string,
        |value| validate_index(value.0, counts.platform_types, "platform-type name lookup"),
    )?;
    validate_x1_platform_type_name_hash(bytes, sections, counts)?;
    validate_records::<TypeTemplateLookup<HbkPlatformTypeId>>(
        bytes,
        sections,
        S::PlatformTypeTemplates,
        |value| {
            string(value.family)?;
            string(value.variant)?;
            validate_index(
                value.value.0,
                counts.platform_types,
                "platform-type template lookup",
            )
        },
    )?;
    validate_id_lookup::<HbkTypeMemberId>(bytes, sections, S::MemberIds, &string, |value| {
        validate_index(value.0, counts.type_members, "member id lookup")
    })?;
    validate_csr::<HbkPlatformTypeId, HbkTypeMemberId>(
        bytes,
        sections,
        S::MembersByOwnerKeys,
        S::MembersByOwnerOffsets,
        S::MembersByOwnerValues,
        |key| validate_index(key.0, counts.platform_types, "members owner key"),
        |value| validate_index(value.0, counts.type_members, "members owner value"),
    )?;
    validate_x1_member_availability_hot(bytes, sections, counts)?;
    validate_x1_global_availability_hot(bytes, sections, counts)?;
    validate_owner_name_lookup::<HbkPlatformTypeId, HbkTypeMemberId>(
        bytes,
        sections,
        S::MembersByOwnerName,
        &string,
        |owner| validate_index(owner.0, counts.platform_types, "member-name owner"),
        |value| validate_index(value.0, counts.type_members, "member-name value"),
    )?;
    validate_records::<MemberNameKindLookup>(
        bytes,
        sections,
        S::MembersByOwnerNameKind,
        |value| {
            validate_index(value.owner.0, counts.platform_types, "member-kind owner")?;
            string(value.key)?;
            validate_index(value.value.0, counts.type_members, "member-kind value")
        },
    )?;
    validate_id_lookup::<HbkCallableId>(bytes, sections, S::CallableIds, &string, |value| {
        validate_index(value.0, counts.callables, "callable id lookup")
    })?;
    validate_csr::<HbkPlatformTypeId, HbkCallableId>(
        bytes,
        sections,
        S::CallablesByOwnerKeys,
        S::CallablesByOwnerOffsets,
        S::CallablesByOwnerValues,
        |key| validate_index(key.0, counts.platform_types, "callables owner key"),
        |value| validate_index(value.0, counts.callables, "callables owner value"),
    )?;
    validate_owner_name_lookup::<HbkPlatformTypeId, HbkCallableId>(
        bytes,
        sections,
        S::CallablesByOwnerName,
        &string,
        |owner| validate_index(owner.0, counts.platform_types, "callable-name owner"),
        |value| validate_index(value.0, counts.callables, "callable-name value"),
    )?;
    validate_csr::<HbkPlatformTypeId, HbkCallableId>(
        bytes,
        sections,
        S::ConstructorsByTypeKeys,
        S::ConstructorsByTypeOffsets,
        S::ConstructorsByTypeValues,
        |key| validate_index(key.0, counts.platform_types, "constructor owner"),
        |value| validate_index(value.0, counts.callables, "constructor value"),
    )?;
    validate_name_lookup::<HbkGlobalFactId>(bytes, sections, S::GlobalNames, &string, |value| {
        validate_index(value.0, counts.globals, "global name value")
    })?;
    validate_records::<GlobalNameKindLookup>(
        bytes,
        sections,
        S::GlobalsByDomainNameKind,
        |value| {
            string(value.key)?;
            validate_index(value.value.0, counts.globals, "global kind value")
        },
    )?;
    validate_owner_name_lookup::<StringId, HbkCallableId>(
        bytes,
        sections,
        S::ModuleEventNames,
        &string,
        &string,
        |value| validate_index(value.0, counts.callables, "module-event value"),
    )?;
    validate_records::<ModuleContextLookup>(
        bytes,
        sections,
        S::ModuleContextsByDomainLanguageKind,
        |value| {
            string(value.language_key)?;
            string(value.module_kind)?;
            validate_index(value.value.0, counts.callables, "module-context value")
        },
    )?;
    validate_id_lookup::<HbkQueryTableId>(bytes, sections, S::QueryTableIds, &string, |value| {
        validate_index(value.0, counts.query_tables, "query-table id lookup")
    })?;
    for section in [
        S::QueryTableNames,
        S::QueryTableSyntaxNames,
        S::QueryTableIdentifiers,
    ] {
        validate_name_lookup::<HbkQueryTableId>(bytes, sections, section, &string, |value| {
            validate_index(value.0, counts.query_tables, "query-table lookup")
        })?;
    }
    validate_csr::<HbkQueryTableId, HbkQueryFieldId>(
        bytes,
        sections,
        S::QueryFieldsByTableKeys,
        S::QueryFieldsByTableOffsets,
        S::QueryFieldsByTableValues,
        |key| validate_index(key.0, counts.query_tables, "query-field table key"),
        |value| validate_index(value.0, counts.query_fields, "query-field table value"),
    )?;
    validate_owner_name_lookup::<HbkQueryTableId, HbkQueryFieldId>(
        bytes,
        sections,
        S::QueryFieldsByTableName,
        &string,
        |owner| validate_index(owner.0, counts.query_tables, "query-field name owner"),
        |value| validate_index(value.0, counts.query_fields, "query-field name value"),
    )?;
    validate_csr::<HbkQueryTableId, HbkQueryParameterId>(
        bytes,
        sections,
        S::QueryParametersByTableKeys,
        S::QueryParametersByTableOffsets,
        S::QueryParametersByTableValues,
        |key| validate_index(key.0, counts.query_tables, "query-parameter table key"),
        |value| {
            validate_index(
                value.0,
                counts.query_parameters,
                "query-parameter table value",
            )
        },
    )?;
    validate_owner_name_lookup::<HbkQueryTableId, HbkQueryParameterId>(
        bytes,
        sections,
        S::QueryParametersByTableName,
        &string,
        |owner| validate_index(owner.0, counts.query_tables, "query-parameter name owner"),
        |value| {
            validate_index(
                value.0,
                counts.query_parameters,
                "query-parameter name value",
            )
        },
    )?;
    validate_id_lookup::<HbkLanguageFactId>(bytes, sections, S::LanguageIds, &string, |value| {
        validate_index(value.0, counts.language_facts, "language id lookup")
    })?;
    validate_name_lookup::<HbkLanguageFactId>(
        bytes,
        sections,
        S::LanguageNames,
        &string,
        |value| validate_index(value.0, counts.language_facts, "language name lookup"),
    )?;
    validate_id_lookup::<HbkEnumId>(bytes, sections, S::EnumIds, &string, |value| {
        validate_index(value.0, counts.enums, "enum id lookup")
    })?;
    validate_name_lookup::<HbkEnumId>(bytes, sections, S::EnumNames, &string, |value| {
        validate_index(value.0, counts.enums, "enum name lookup")
    })?;
    validate_id_lookup::<HbkEnumValueId>(bytes, sections, S::EnumValueIds, &string, |value| {
        validate_index(value.0, counts.enum_values, "enum-value id lookup")
    })?;
    validate_csr::<HbkEnumId, HbkEnumValueId>(
        bytes,
        sections,
        S::EnumValuesByEnumKeys,
        S::EnumValuesByEnumOffsets,
        S::EnumValuesByEnumValues,
        |key| validate_index(key.0, counts.enums, "enum-value owner key"),
        |value| validate_index(value.0, counts.enum_values, "enum-value owner value"),
    )?;
    validate_owner_name_lookup::<HbkEnumId, HbkEnumValueId>(
        bytes,
        sections,
        S::EnumValuesByEnumName,
        &string,
        |owner| validate_index(owner.0, counts.enums, "enum-value name owner"),
        |value| validate_index(value.0, counts.enum_values, "enum-value name value"),
    )?;
    validate_csr::<HbkFactRef, StringId>(
        bytes,
        sections,
        S::AvailabilityByFactKeys,
        S::AvailabilityByFactOffsets,
        S::AvailabilityByFactValues,
        &fact,
        &string,
    )?;
    validate_records::<FactStringLookup>(bytes, sections, S::AvailabilitySinceByFact, |value| {
        fact(value.fact)?;
        string(value.value)
    })?;
    validate_records::<FactSourceLookup>(bytes, sections, S::SourceByFact, |value| {
        fact(value.fact)?;
        validate_fact_source(value.source, &string)
    })?;
    validate_csr::<RelationLookupKey, HbkFactRef>(
        bytes,
        sections,
        S::RelationsBySourceKindKeys,
        S::RelationsBySourceKindOffsets,
        S::RelationsBySourceKindValues,
        |key| {
            fact(key.source)?;
            string(key.kind)
        },
        fact,
    )?;
    validate_index_ordering(bytes, sections)
}

fn validate_index_ordering(bytes: &[u8], sections: &[Section]) -> io::Result<()> {
    let strings = VectorView::new(section_bytes(bytes, sections, S::Strings))?;
    let text = |id: StringId| {
        strings
            .get_str(id.0 as usize)
            .expect("X1 string IDs were validated before index ordering")
    };
    macro_rules! id_order {
        ($section:expr, $value:ty) => {
            validate_sorted_records::<IdLookup<$value>>(
                bytes,
                sections,
                $section,
                |left, right| {
                    text(left.key)
                        .cmp(text(right.key))
                        .then_with(|| left.value.cmp(&right.value))
                },
            )?
        };
    }
    macro_rules! name_order {
        ($section:expr, $value:ty) => {
            validate_sorted_records::<NameLookup<$value>>(
                bytes,
                sections,
                $section,
                |left, right| {
                    text(left.key)
                        .cmp(text(right.key))
                        .then_with(|| left.value.cmp(&right.value))
                },
            )?
        };
    }
    macro_rules! owner_name_order {
        ($section:expr, $owner:ty, $value:ty) => {
            validate_sorted_records::<OwnerNameLookup<$owner, $value>>(
                bytes,
                sections,
                $section,
                |left, right| {
                    left.owner
                        .cmp(&right.owner)
                        .then_with(|| text(left.key).cmp(text(right.key)))
                        .then_with(|| left.value.cmp(&right.value))
                },
            )?
        };
    }
    macro_rules! string_owner_name_order {
        ($section:expr, $value:ty) => {
            validate_sorted_records::<OwnerNameLookup<StringId, $value>>(
                bytes,
                sections,
                $section,
                |left, right| {
                    text(left.owner)
                        .cmp(text(right.owner))
                        .then_with(|| text(left.key).cmp(text(right.key)))
                        .then_with(|| left.value.cmp(&right.value))
                },
            )?
        };
    }
    id_order!(S::FactIds, HbkFactRef);
    id_order!(S::PlatformTypeIds, HbkPlatformTypeId);
    name_order!(S::PlatformTypeNames, HbkPlatformTypeId);
    validate_sorted_records::<TypeTemplateLookup<HbkPlatformTypeId>>(
        bytes,
        sections,
        S::PlatformTypeTemplates,
        |left, right| {
            text(left.family)
                .cmp(text(right.family))
                .then_with(|| text(left.variant).cmp(text(right.variant)))
                .then_with(|| left.value.cmp(&right.value))
        },
    )?;
    id_order!(S::MemberIds, HbkTypeMemberId);
    owner_name_order!(S::MembersByOwnerName, HbkPlatformTypeId, HbkTypeMemberId);
    validate_sorted_records::<MemberNameKindLookup>(
        bytes,
        sections,
        S::MembersByOwnerNameKind,
        |left, right| {
            left.owner
                .cmp(&right.owner)
                .then_with(|| text(left.key).cmp(text(right.key)))
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.value.cmp(&right.value))
        },
    )?;
    id_order!(S::CallableIds, HbkCallableId);
    owner_name_order!(S::CallablesByOwnerName, HbkPlatformTypeId, HbkCallableId);
    name_order!(S::GlobalNames, HbkGlobalFactId);
    validate_sorted_records::<GlobalNameKindLookup>(
        bytes,
        sections,
        S::GlobalsByDomainNameKind,
        |left, right| {
            left.domain
                .cmp(&right.domain)
                .then_with(|| text(left.key).cmp(text(right.key)))
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.value.cmp(&right.value))
        },
    )?;
    string_owner_name_order!(S::ModuleEventNames, HbkCallableId);
    validate_sorted_records::<ModuleContextLookup>(
        bytes,
        sections,
        S::ModuleContextsByDomainLanguageKind,
        |left, right| {
            left.domain
                .cmp(&right.domain)
                .then_with(|| text(left.language_key).cmp(text(right.language_key)))
                .then_with(|| text(left.module_kind).cmp(text(right.module_kind)))
                .then_with(|| left.value.cmp(&right.value))
        },
    )?;
    id_order!(S::QueryTableIds, HbkQueryTableId);
    name_order!(S::QueryTableNames, HbkQueryTableId);
    name_order!(S::QueryTableSyntaxNames, HbkQueryTableId);
    name_order!(S::QueryTableIdentifiers, HbkQueryTableId);
    owner_name_order!(S::QueryFieldsByTableName, HbkQueryTableId, HbkQueryFieldId);
    owner_name_order!(
        S::QueryParametersByTableName,
        HbkQueryTableId,
        HbkQueryParameterId
    );
    id_order!(S::LanguageIds, HbkLanguageFactId);
    name_order!(S::LanguageNames, HbkLanguageFactId);
    id_order!(S::EnumIds, HbkEnumId);
    name_order!(S::EnumNames, HbkEnumId);
    id_order!(S::EnumValueIds, HbkEnumValueId);
    owner_name_order!(S::EnumValuesByEnumName, HbkEnumId, HbkEnumValueId);
    validate_sorted_records::<FactStringLookup>(
        bytes,
        sections,
        S::AvailabilitySinceByFact,
        |left, right| {
            left.fact
                .cmp(&right.fact)
                .then_with(|| left.value.cmp(&right.value))
        },
    )?;
    validate_sorted_records::<FactSourceLookup>(bytes, sections, S::SourceByFact, |left, right| {
        left.fact.cmp(&right.fact)
    })
}

fn validate_fact_source(
    value: HbkFactSource,
    validate_string: &impl Fn(StringId) -> io::Result<()>,
) -> io::Result<()> {
    validate_string(value.hbk_path)?;
    validate_string(value.locale)?;
    if let Some(toc_path) = value.toc_path {
        validate_string(toc_path)?;
    }
    validate_string(value.html_path)?;
    validate_string(value.page_title)
}

fn validate_fact_source_identity(
    bytes: &[u8],
    sections: &[Section],
    identity: &X1ArtifactIdentity,
) -> io::Result<()> {
    let strings = VectorView::new(section_bytes(bytes, sections, S::Strings))?;
    let sources = VectorView::new(section_bytes(bytes, sections, S::SourceByFact))?;
    for index in 0..sources.len() {
        let source = sources.get::<FactSourceLookup>(index)?.source;
        if strings.get_str(source.hbk_path.0 as usize)? != identity.source_path {
            return Err(invalid_data("X1 fact provenance HBK identity mismatch"));
        }
        let locale = strings.get_str(source.locale.0 as usize)?;
        if locale != identity.locale && locale != identity.source_locale {
            return Err(invalid_data("X1 fact provenance locale identity mismatch"));
        }
    }
    Ok(())
}

fn validate_template_binding_semantics(
    bytes: &[u8],
    sections: &[Section],
    counts: HbkFactSnapshotCounts,
) -> io::Result<()> {
    let platform_types = VectorView::new(section_bytes(bytes, sections, S::PlatformTypes))?;
    let metadata_templates = VectorView::new(section_bytes(bytes, sections, S::MetadataTemplates))?;
    let query_tables = VectorView::new(section_bytes(bytes, sections, S::QueryTables))?;
    let template_lookups =
        VectorView::new(section_bytes(bytes, sections, S::PlatformTypeTemplates))?;
    let type_refs = VectorView::new(section_bytes(bytes, sections, S::TypeRefs))?;
    let bindings = VectorView::new(section_bytes(bytes, sections, S::TemplateBindings))?;
    let arguments = VectorView::new(section_bytes(bytes, sections, S::TemplateArguments))?;
    let members = VectorView::new(section_bytes(bytes, sections, S::TypeMembers))?;
    let callables = VectorView::new(section_bytes(bytes, sections, S::Callables))?;
    let globals = VectorView::new(section_bytes(bytes, sections, S::Globals))?;
    let query_fields = VectorView::new(section_bytes(bytes, sections, S::QueryFields))?;
    let query_parameters = VectorView::new(section_bytes(bytes, sections, S::QueryParameters))?;
    let language_facts = VectorView::new(section_bytes(bytes, sections, S::LanguageFacts))?;
    let signatures = VectorView::new(section_bytes(bytes, sections, S::Signatures))?;
    let parameters = VectorView::new(section_bytes(bytes, sections, S::Parameters))?;

    let mut platform_parameter_counts = Vec::with_capacity(counts.platform_types);
    for index in 0..platform_types.len() {
        let head = platform_types.get::<X1PlatformTypeHead>(index)?;
        let count = if head.metadata_template == NONE_U32 {
            0
        } else {
            metadata_templates
                .get::<X1MetadataTemplateHead>(head.metadata_template as usize)?
                .template_parameters
                .len as usize
        };
        platform_parameter_counts.push(count);
    }
    let mut query_parameter_counts = Vec::with_capacity(counts.query_tables);
    for index in 0..query_tables.len() {
        query_parameter_counts.push(
            query_tables
                .get::<X1QueryTableHead>(index)?
                .template_parameters
                .len as usize,
        );
    }

    let mut target_parameter_counts = BTreeMap::<(u32, u32), usize>::new();
    for index in 0..template_lookups.len() {
        let lookup = template_lookups.get::<TypeTemplateLookup<HbkPlatformTypeId>>(index)?;
        let count = platform_parameter_counts[lookup.value.0 as usize];
        target_parameter_counts
            .entry((lookup.family.0, lookup.variant.0))
            .and_modify(|current| *current = (*current).max(count))
            .or_insert(count);
    }

    let mut type_ref_owners = vec![None; type_refs.len()];
    let mut signature_owners = vec![None; signatures.len()];
    let mut parameter_owners = vec![None; parameters.len()];

    for index in 0..members.len() {
        let head = members.get::<X1TypeMemberHead>(index)?;
        assign_owner_range(
            &mut type_ref_owners,
            head.type_refs,
            platform_parameter_counts[head.owner.0 as usize],
            "X1 member type-ref ownership overlaps",
        )?;
    }
    for index in 0..callables.len() {
        let head = callables.get::<X1CallableHead>(index)?;
        let owner_parameters = if head.owner == NONE_U32 {
            0
        } else {
            platform_parameter_counts[head.owner as usize]
        };
        assign_owner_range(
            &mut type_ref_owners,
            head.return_type_refs,
            owner_parameters,
            "X1 callable return type-ref ownership overlaps",
        )?;
        assign_owner_range(
            &mut signature_owners,
            head.signatures,
            owner_parameters,
            "X1 callable signature ownership overlaps",
        )?;
    }
    for index in 0..globals.len() {
        assign_owner_range(
            &mut type_ref_owners,
            globals.get::<X1GlobalFactHead>(index)?.type_refs,
            0,
            "X1 global type-ref ownership overlaps",
        )?;
    }
    for index in 0..query_fields.len() {
        let head = query_fields.get::<X1QueryFieldHead>(index)?;
        assign_owner_range(
            &mut type_ref_owners,
            head.type_refs,
            query_parameter_counts[head.owner.0 as usize],
            "X1 query-field type-ref ownership overlaps",
        )?;
    }
    for index in 0..query_parameters.len() {
        let head = query_parameters.get::<X1QueryParameterHead>(index)?;
        assign_owner_range(
            &mut type_ref_owners,
            head.type_refs,
            query_parameter_counts[head.owner.0 as usize],
            "X1 query-parameter type-ref ownership overlaps",
        )?;
    }
    for index in 0..language_facts.len() {
        let head = language_facts.get::<X1LanguageFactHead>(index)?;
        assign_owner_range(
            &mut type_ref_owners,
            head.type_refs,
            0,
            "X1 language type-ref ownership overlaps",
        )?;
        assign_owner_range(
            &mut type_ref_owners,
            head.return_type_refs,
            0,
            "X1 language return type-ref ownership overlaps",
        )?;
        assign_owner_range(
            &mut signature_owners,
            head.signatures,
            0,
            "X1 language signature ownership overlaps",
        )?;
    }
    for (index, owner_parameters) in signature_owners.iter().copied().enumerate() {
        let owner_parameters = owner_parameters
            .ok_or_else(|| invalid_data("X1 signature arena contains an orphan record"))?;
        let head = signatures.get::<X1SignatureHead>(index)?;
        assign_owner_range(
            &mut type_ref_owners,
            head.return_type_refs,
            owner_parameters,
            "X1 signature return type-ref ownership overlaps",
        )?;
        assign_owner_range(
            &mut parameter_owners,
            head.parameters,
            owner_parameters,
            "X1 parameter ownership overlaps",
        )?;
    }
    for (index, owner_parameters) in parameter_owners.iter().copied().enumerate() {
        let owner_parameters = owner_parameters
            .ok_or_else(|| invalid_data("X1 parameter arena contains an orphan record"))?;
        assign_owner_range(
            &mut type_ref_owners,
            parameters.get::<X1ParameterHead>(index)?.type_refs,
            owner_parameters,
            "X1 parameter type-ref ownership overlaps",
        )?;
    }

    let mut visited_bindings = vec![false; bindings.len()];
    let mut visited_arguments = vec![false; arguments.len()];
    for (index, owner_parameters) in type_ref_owners.iter().copied().enumerate() {
        let owner_parameters = owner_parameters
            .ok_or_else(|| invalid_data("X1 type-ref arena contains an orphan record"))?;
        let type_ref = type_refs.get::<X1TypeRefHead>(index)?;
        if type_ref.template_binding == NONE_U32 {
            continue;
        }
        let binding_index = type_ref.template_binding as usize;
        visited_bindings[binding_index] = true;
        let binding = bindings.get::<X1TemplateBindingHead>(binding_index)?;
        if type_ref.type_template_key != binding.template_key {
            return Err(invalid_data("X1 type-ref template binding key mismatch"));
        }
        let target_parameters = *target_parameter_counts
            .get(&(binding.template_key.family, binding.template_key.variant))
            .ok_or_else(|| invalid_data("X1 template binding target key is unknown"))?;
        for argument_index in binding.arguments.as_usize()? {
            visited_arguments[argument_index] = true;
            match arguments.get::<model::TemplateParameterBinding>(argument_index)? {
                model::TemplateParameterBinding::OwnerParameter {
                    owner_parameter_index,
                    target_parameter_index,
                } => {
                    if owner_parameter_index >= owner_parameters {
                        return Err(invalid_data(
                            "X1 template binding owner parameter is out of bounds",
                        ));
                    }
                    if target_parameter_index >= target_parameters {
                        return Err(invalid_data(
                            "X1 template binding target parameter is out of bounds",
                        ));
                    }
                }
            }
        }
    }
    if visited_bindings.iter().any(|visited| !visited) {
        return Err(invalid_data(
            "X1 template binding arena contains an orphan record",
        ));
    }
    if visited_arguments.iter().any(|visited| !visited) {
        return Err(invalid_data(
            "X1 template argument arena contains an orphan record",
        ));
    }
    Ok(())
}

fn assign_owner_range(
    owners: &mut [Option<usize>],
    range: X1Range,
    owner_parameters: usize,
    overlap_error: &'static str,
) -> io::Result<()> {
    for index in range.as_usize()? {
        let owner = owners
            .get_mut(index)
            .ok_or_else(|| invalid_data("X1 semantic owner range is out of bounds"))?;
        if owner.replace(owner_parameters).is_some() {
            return Err(invalid_data(overlap_error));
        }
    }
    Ok(())
}

fn validate_sorted_records<T: BinaryValue>(
    bytes: &[u8],
    sections: &[Section],
    section: S,
    compare: impl Fn(&T, &T) -> Ordering,
) -> io::Result<()> {
    let view = VectorView::new(section_bytes(bytes, sections, section))?;
    if view.len() < 2 {
        return Ok(());
    }
    let mut previous = view.get::<T>(0)?;
    for index in 1..view.len() {
        let current = view.get::<T>(index)?;
        if compare(&previous, &current) != Ordering::Less {
            return Err(invalid_data("X1 lookup index is not strictly sorted"));
        }
        previous = current;
    }
    Ok(())
}

fn validate_records<T: BinaryValue>(
    bytes: &[u8],
    sections: &[Section],
    section: S,
    mut validate: impl FnMut(&T) -> io::Result<()>,
) -> io::Result<()> {
    let view = VectorView::new(section_bytes(bytes, sections, section))?;
    for index in 0..view.len() {
        let value = view.get::<T>(index)?;
        validate(&value)?;
    }
    Ok(())
}

fn validate_x1_member_availability_hot(
    bytes: &[u8],
    sections: &[Section],
    counts: HbkFactSnapshotCounts,
) -> io::Result<()> {
    let type_ranges = VectorView::new(section_bytes(bytes, sections, S::TypeMemberRanges))?;
    let hot = VectorView::new(section_bytes(bytes, sections, S::MemberAvailabilityHot))?;
    let owner_keys = VectorView::new(section_bytes(bytes, sections, S::MembersByOwnerKeys))?;
    let owner_offsets = VectorView::new(section_bytes(bytes, sections, S::MembersByOwnerOffsets))?;
    let owner_values = VectorView::new(section_bytes(bytes, sections, S::MembersByOwnerValues))?;
    let members = VectorView::new(section_bytes(bytes, sections, S::TypeMembers))?;
    let strings = VectorView::new(section_bytes(bytes, sections, S::Strings))?;
    let string_ids = VectorView::new(section_bytes(bytes, sections, S::StringIds))?;
    if type_ranges.len() != counts.platform_types {
        return Err(invalid_data("X1 type/member range count mismatch"));
    }
    if hot.len() != owner_values.len() {
        return Err(invalid_data("X1 member hot count mismatch"));
    }
    for index in 0..hot.len() {
        let record = hot.get_x1_available_member_hot(index)?;
        if record.reserved != 0 {
            return Err(invalid_data("X1 member hot reserved byte is non-zero"));
        }
        validate_x1_availability_word(record.availability_word)?;
        validate_index(record.member_id, counts.type_members, "X1 member hot id")?;
        if owner_values.get::<HbkTypeMemberId>(index)?.0 != record.member_id {
            return Err(invalid_data("X1 member hot owner order mismatch"));
        }
        let member = members.get::<X1TypeMemberHead>(record.member_id as usize)?;
        if x1_member_kind_tag(member.kind) != record.kind {
            return Err(invalid_data("X1 member hot kind mismatch"));
        }
        let expected_word =
            x1_mapped_availability_word(&strings, &string_ids, member.availability_contexts)?;
        if record.availability_word != expected_word {
            return Err(invalid_data("X1 member hot availability word mismatch"));
        }
        x1_member_kind_from_tag(record.kind)?;
    }
    let mut csr_owner_index = 0usize;
    let mut cursor = 0usize;
    for owner_index in 0..counts.platform_types {
        let owner = HbkPlatformTypeId(owner_index as u32);
        let range = type_ranges.get::<X1TypeMemberRangeHot>(owner_index)?;
        let start = range.member_start as usize;
        let end = start
            .checked_add(range.member_count as usize)
            .ok_or_else(|| invalid_data("X1 type/member range overflow"))?;
        if start != cursor || end > hot.len() {
            return Err(invalid_data("X1 type/member ranges are not contiguous"));
        }
        if owner_keys.get::<HbkPlatformTypeId>(csr_owner_index).ok() == Some(owner) {
            let expected_start = owner_offsets.get::<u32>(csr_owner_index)? as usize;
            let expected_end = owner_offsets.get::<u32>(csr_owner_index + 1)? as usize;
            if (start, end) != (expected_start, expected_end) {
                return Err(invalid_data(
                    "X1 type/member range differs from owner index",
                ));
            }
            csr_owner_index += 1;
        } else if start != end {
            return Err(invalid_data("X1 empty owner has a non-empty member range"));
        }
        for index in start..end {
            let record = hot.get_x1_available_member_hot(index)?;
            let member = members.get::<X1TypeMemberHead>(record.member_id as usize)?;
            if member.owner != owner {
                return Err(invalid_data("X1 member hot owner range mismatch"));
            }
        }
        cursor = end;
    }
    if cursor != hot.len() || csr_owner_index != owner_keys.len() {
        return Err(invalid_data(
            "X1 type/member ranges do not cover the hot section",
        ));
    }
    Ok(())
}

fn validate_x1_global_availability_hot(
    bytes: &[u8],
    sections: &[Section],
    counts: HbkFactSnapshotCounts,
) -> io::Result<()> {
    let locators = VectorView::new(section_bytes(bytes, sections, S::GlobalAvailabilityHot))?;
    let masks = VectorView::new(section_bytes(bytes, sections, S::GlobalAvailabilityMasks))?;
    let kinds = VectorView::new(section_bytes(bytes, sections, S::GlobalAvailabilityKinds))?;
    let globals = VectorView::new(section_bytes(bytes, sections, S::Globals))?;
    if locators.len() != counts.globals
        || masks.len() != counts.globals
        || kinds.len() != counts.globals
        || globals.len() != counts.globals
    {
        return Err(invalid_data("X1 global hot count mismatch"));
    }
    for index in 0..locators.len() {
        let global_id = locators.get::<u32>(index)?;
        let availability_word = masks.get::<u16>(index)?;
        let kind = kinds.get::<u8>(index)?;
        if global_id as usize != index {
            return Err(invalid_data("X1 global hot locator mismatch"));
        }
        validate_x1_availability_word(availability_word)?;
        let global = globals.get::<X1GlobalFactHead>(index)?;
        if x1_global_kind_tag(global.kind) != kind {
            return Err(invalid_data("X1 global hot kind mismatch"));
        }
        x1_global_kind_from_tag(kind)?;
        let expected_word = x1_mapped_fact_availability_word(
            bytes,
            sections,
            HbkFactRef::Global(HbkGlobalFactId(index as u32)),
        )?;
        if availability_word != expected_word {
            return Err(invalid_data("X1 global hot availability word mismatch"));
        }
    }
    Ok(())
}

fn validate_x1_platform_type_name_hash(
    bytes: &[u8],
    sections: &[Section],
    counts: HbkFactSnapshotCounts,
) -> io::Result<()> {
    let buckets = VectorView::new(section_bytes(bytes, sections, S::PlatformTypeNameHash))?;
    let names = VectorView::new(section_bytes(bytes, sections, S::PlatformTypeNames))?;
    let strings = VectorView::new(section_bytes(bytes, sections, S::Strings))?;
    if buckets.len() == 0 || !buckets.len().is_power_of_two() {
        return Err(invalid_data(
            "X1 platform type-name hash capacity is invalid",
        ));
    }
    let mut occupied = 0usize;
    let mut covered = vec![false; names.len()];
    for bucket_index in 0..buckets.len() {
        let bucket = buckets.get::<X1TypeNameHashBucket>(bucket_index)?;
        if bucket.count == 0 {
            if bucket.hash != 0 || bucket.start != 0 {
                return Err(invalid_data(
                    "X1 platform type-name hash empty bucket is dirty",
                ));
            }
            continue;
        }
        occupied += 1;
        let start = bucket.start as usize;
        let end = start
            .checked_add(bucket.count as usize)
            .ok_or_else(|| invalid_data("X1 platform type-name hash range overflow"))?;
        if start >= end || end > names.len() {
            return Err(invalid_data(
                "X1 platform type-name hash range out of bounds",
            ));
        }
        let first = names.get::<NameLookup<HbkPlatformTypeId>>(start)?;
        validate_index(
            first.value.0,
            counts.platform_types,
            "X1 platform type-name hash value",
        )?;
        let key = strings.get_str(first.key.0 as usize)?;
        if bucket.hash != x1_hash_key(key) {
            return Err(invalid_data("X1 platform type-name hash value mismatch"));
        }
        let expected_bucket = x1_probe_bucket(&buckets, &names, &strings, bucket.hash, key)?;
        if expected_bucket != bucket_index {
            return Err(invalid_data(
                "X1 platform type-name hash probe chain mismatch",
            ));
        }
        for (index, covered_entry) in covered.iter_mut().enumerate().take(end).skip(start) {
            if std::mem::replace(covered_entry, true) {
                return Err(invalid_data("X1 platform type-name hash overlaps ranges"));
            }
            let record = names.get::<NameLookup<HbkPlatformTypeId>>(index)?;
            validate_index(
                record.value.0,
                counts.platform_types,
                "X1 platform type-name hash value",
            )?;
            if strings.get_str(record.key.0 as usize)? != key {
                return Err(invalid_data("X1 platform type-name hash mixed-key bucket"));
            }
        }
    }
    if occupied.saturating_mul(2) > buckets.len() {
        return Err(invalid_data("X1 platform type-name hash load exceeds 0.5"));
    }
    if covered.iter().any(|seen| !*seen) {
        return Err(invalid_data(
            "X1 platform type-name hash does not cover names",
        ));
    }
    Ok(())
}

fn x1_probe_bucket(
    buckets: &VectorView<'_>,
    names: &VectorView<'_>,
    strings: &VectorView<'_>,
    hash: u64,
    key: &str,
) -> io::Result<usize> {
    let mut index = (hash as usize) & (buckets.len() - 1);
    for _ in 0..64 {
        let candidate = buckets.get::<X1TypeNameHashBucket>(index)?;
        if candidate.count == 0 {
            return Ok(index);
        }
        if candidate.hash == hash
            && x1_validated_bucket_key_matches(names, strings, candidate, key)?
        {
            return Ok(index);
        }
        index = (index + 1) & (buckets.len() - 1);
    }
    Err(invalid_data(
        "X1 platform type-name hash max probe exceeded",
    ))
}

fn x1_validated_bucket_key_matches(
    names: &VectorView<'_>,
    strings: &VectorView<'_>,
    bucket: X1TypeNameHashBucket,
    key: &str,
) -> io::Result<bool> {
    let start = bucket.start as usize;
    let end = start
        .checked_add(bucket.count as usize)
        .ok_or_else(|| invalid_data("X1 platform type-name hash range overflow"))?;
    if start >= end || end > names.len() {
        return Err(invalid_data(
            "X1 platform type-name hash range out of bounds",
        ));
    }
    let first = names.get::<NameLookup<HbkPlatformTypeId>>(start)?;
    Ok(strings.get_str(first.key.0 as usize)? == key)
}

fn x1_mapped_fact_availability_word(
    bytes: &[u8],
    sections: &[Section],
    fact: HbkFactRef,
) -> io::Result<u16> {
    let keys = VectorView::new(section_bytes(bytes, sections, S::AvailabilityByFactKeys))?;
    let offsets = VectorView::new(section_bytes(bytes, sections, S::AvailabilityByFactOffsets))?;
    let values = VectorView::new(section_bytes(bytes, sections, S::AvailabilityByFactValues))?;
    let strings = VectorView::new(section_bytes(bytes, sections, S::Strings))?;
    let Some(position) =
        binary_search_view(&keys, |candidate: HbkFactRef| candidate.cmp(&fact)).ok()
    else {
        return Ok(X1_CONTEXT_BITS);
    };
    let start = offsets.get::<u32>(position)? as usize;
    let end = offsets.get::<u32>(position + 1)? as usize;
    if start == end {
        return Ok(X1_CONTEXT_BITS);
    }
    let mut word = X1_HAS_EXPLICIT_DECLARATION;
    for index in start..end {
        let id = values.get::<StringId>(index)?;
        let code = strings.get_str(id.0 as usize)?;
        let Some(bit) = x1_context_code_bit(code) else {
            return Err(invalid_data("unknown X1 explicit availability context"));
        };
        word |= bit;
    }
    validate_x1_availability_word(word)?;
    Ok(word)
}

fn x1_mapped_availability_word(
    strings: &VectorView<'_>,
    string_ids: &VectorView<'_>,
    range: X1Range,
) -> io::Result<u16> {
    if range.len == 0 {
        return Ok(X1_CONTEXT_BITS);
    }
    let mut word = X1_HAS_EXPLICIT_DECLARATION;
    for index in range.as_usize()? {
        let id = string_ids.get::<StringId>(index)?;
        let code = strings.get_str(id.0 as usize)?;
        let Some(bit) = x1_context_code_bit(code) else {
            return Err(invalid_data("unknown X1 explicit availability context"));
        };
        word |= bit;
    }
    if word & X1_CONTEXT_BITS == 0 {
        return Err(invalid_data("empty X1 explicit availability context"));
    }
    Ok(word)
}

fn validate_x1_availability_word(word: u16) -> io::Result<()> {
    let allowed = X1_CONTEXT_BITS | X1_HAS_EXPLICIT_DECLARATION;
    if word & !allowed != 0 {
        return Err(invalid_data("X1 availability word has unsupported bits"));
    }
    let context_bits = word & X1_CONTEXT_BITS;
    let explicit = word & X1_HAS_EXPLICIT_DECLARATION != 0;
    if explicit {
        if context_bits == 0 {
            return Err(invalid_data("X1 explicit availability word is empty"));
        }
    } else if context_bits != X1_CONTEXT_BITS {
        return Err(invalid_data("X1 universal availability word is invalid"));
    }
    Ok(())
}

fn validate_id_lookup<T: BinaryValue + Copy>(
    bytes: &[u8],
    sections: &[Section],
    section: S,
    validate_string: &impl Fn(StringId) -> io::Result<()>,
    mut validate_value: impl FnMut(T) -> io::Result<()>,
) -> io::Result<()> {
    validate_records::<IdLookup<T>>(bytes, sections, section, |value| {
        validate_string(value.key)?;
        validate_value(value.value)
    })
}

fn validate_name_lookup<T: BinaryValue + Copy>(
    bytes: &[u8],
    sections: &[Section],
    section: S,
    validate_string: &impl Fn(StringId) -> io::Result<()>,
    mut validate_value: impl FnMut(T) -> io::Result<()>,
) -> io::Result<()> {
    validate_records::<NameLookup<T>>(bytes, sections, section, |value| {
        validate_string(value.key)?;
        validate_value(value.value)
    })
}

fn validate_owner_name_lookup<Owner: BinaryValue + Copy, Value: BinaryValue + Copy>(
    bytes: &[u8],
    sections: &[Section],
    section: S,
    validate_string: &impl Fn(StringId) -> io::Result<()>,
    mut validate_owner: impl FnMut(Owner) -> io::Result<()>,
    mut validate_value: impl FnMut(Value) -> io::Result<()>,
) -> io::Result<()> {
    validate_records::<OwnerNameLookup<Owner, Value>>(bytes, sections, section, |value| {
        validate_owner(value.owner)?;
        validate_string(value.key)?;
        validate_value(value.value)
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_csr<K: BinaryValue + Copy + Ord, V: BinaryValue + Copy>(
    bytes: &[u8],
    sections: &[Section],
    keys_section: S,
    offsets_section: S,
    values_section: S,
    mut validate_key: impl FnMut(K) -> io::Result<()>,
    mut validate_value: impl FnMut(V) -> io::Result<()>,
) -> io::Result<()> {
    let keys = VectorView::new(section_bytes(bytes, sections, keys_section))?;
    let offsets = VectorView::new(section_bytes(bytes, sections, offsets_section))?;
    let values = VectorView::new(section_bytes(bytes, sections, values_section))?;
    let valid_offset_count = offsets.len() == keys.len() + 1
        || (keys.len() == 0 && offsets.len() == 2 && values.len() == 0);
    if !valid_offset_count {
        return Err(invalid_data("X1 CSR offsets count mismatch"));
    }
    let mut previous = 0_u32;
    for index in 0..offsets.len() {
        let offset = offsets.get::<u32>(index)?;
        if (index == 0 && offset != 0) || offset < previous || offset as usize > values.len() {
            return Err(invalid_data("invalid X1 CSR offset"));
        }
        previous = offset;
    }
    if previous as usize != values.len() {
        return Err(invalid_data("X1 CSR terminal offset mismatch"));
    }
    let mut previous_key = None;
    for index in 0..keys.len() {
        let key = keys.get::<K>(index)?;
        if previous_key.is_some_and(|previous| previous >= key) {
            return Err(invalid_data("X1 CSR keys are not strictly sorted"));
        }
        validate_key(key)?;
        previous_key = Some(key);
    }
    for index in 0..values.len() {
        validate_value(values.get::<V>(index)?)?;
    }
    Ok(())
}

fn validate_index(value: u32, count: usize, what: &'static str) -> io::Result<()> {
    if value as usize >= count {
        return Err(invalid_data(what));
    }
    Ok(())
}

fn validate_optional_index(value: u32, count: usize, what: &'static str) -> io::Result<()> {
    if value == NONE_U32 {
        return Ok(());
    }
    validate_index(value, count, what)
}

fn validate_optional_string(
    value: u32,
    validate_string: &impl Fn(StringId) -> io::Result<()>,
) -> io::Result<()> {
    if value == NONE_U32 {
        return Ok(());
    }
    validate_string(StringId(value))
}

fn validate_range(range: X1Range, count: usize, what: &'static str) -> io::Result<()> {
    let end = (range.start as usize)
        .checked_add(range.len as usize)
        .ok_or_else(|| invalid_data(what))?;
    if end > count {
        return Err(invalid_data(what));
    }
    Ok(())
}

fn validate_fact_ref(value: HbkFactRef, counts: HbkFactSnapshotCounts) -> io::Result<()> {
    match value {
        HbkFactRef::PlatformType(id) => {
            validate_index(id.0, counts.platform_types, "platform-type fact ref")
        }
        HbkFactRef::TypeMember(id) => {
            validate_index(id.0, counts.type_members, "type-member fact ref")
        }
        HbkFactRef::Callable(id) => validate_index(id.0, counts.callables, "callable fact ref"),
        HbkFactRef::Global(id) => validate_index(id.0, counts.globals, "global fact ref"),
        HbkFactRef::QueryTable(id) => {
            validate_index(id.0, counts.query_tables, "query-table fact ref")
        }
        HbkFactRef::QueryField(id) => {
            validate_index(id.0, counts.query_fields, "query-field fact ref")
        }
        HbkFactRef::QueryParameter(id) => {
            validate_index(id.0, counts.query_parameters, "query-parameter fact ref")
        }
        HbkFactRef::LanguageFact(id) => {
            validate_index(id.0, counts.language_facts, "language fact ref")
        }
        HbkFactRef::Enum(id) => validate_index(id.0, counts.enums, "enum fact ref"),
        HbkFactRef::EnumValue(id) => {
            validate_index(id.0, counts.enum_values, "enum-value fact ref")
        }
    }
}

fn section_bytes<'a>(bytes: &'a [u8], sections: &[Section], section: S) -> &'a [u8] {
    let section = sections[section.index()];
    &bytes[section.offset..section.offset + section.len]
}

fn binary_search_view<T: BinaryValue + Copy>(
    view: &VectorView<'_>,
    mut compare: impl FnMut(T) -> Ordering,
) -> Result<usize, usize> {
    let mut size = view.len();
    let mut base = 0usize;
    while size > 0 {
        let half = size / 2;
        let mid = base + half;
        let candidate = view.get::<T>(mid).map_err(|_| base)?;
        match compare(candidate) {
            Ordering::Less => {
                base = mid + 1;
                size -= half + 1;
            }
            Ordering::Equal => return Ok(mid),
            Ordering::Greater => size = half,
        }
    }
    Err(base)
}

fn matching_range_view<T: BinaryValue + Copy>(
    view: &VectorView<'_>,
    mut compare: impl FnMut(T) -> Ordering,
) -> Range<usize> {
    let Ok(mut start) = binary_search_view(view, &mut compare) else {
        return 0..0;
    };
    let mut end = start + 1;
    while start > 0 {
        let candidate = view
            .get::<T>(start - 1)
            .expect("X1 lookup index was fully validated before access");
        if !compare(candidate).is_eq() {
            break;
        }
        start -= 1;
    }
    while end < view.len() {
        let candidate = view
            .get::<T>(end)
            .expect("X1 lookup index was fully validated before access");
        if !compare(candidate).is_eq() {
            break;
        }
        end += 1;
    }
    start..end
}

#[derive(Clone, Copy)]
struct VectorView<'a> {
    bytes: &'a [u8],
    count: usize,
    stride: usize,
    payload_start: usize,
}

impl<'a> VectorView<'a> {
    fn new(bytes: &'a [u8]) -> io::Result<Self> {
        if bytes.len() < 8 {
            return Err(invalid_data("X1 vector section is truncated"));
        }
        let count = read_u32_at(bytes, 0)? as usize;
        let stride = read_u32_at(bytes, 4)? as usize;
        let payload_start = if stride == 0 {
            let offsets_len = count
                .checked_add(1)
                .and_then(|count| count.checked_mul(4))
                .ok_or_else(|| invalid_data("X1 vector offsets overflow"))?;
            8usize
                .checked_add(offsets_len)
                .ok_or_else(|| invalid_data("X1 vector payload offset overflow"))?
        } else {
            8
        };
        if payload_start > bytes.len() {
            return Err(invalid_data("X1 vector offsets exceed section"));
        }
        if stride != 0 {
            let expected_len = count
                .checked_mul(stride)
                .and_then(|payload| payload.checked_add(payload_start))
                .ok_or_else(|| invalid_data("X1 fixed vector length overflow"))?;
            if expected_len != bytes.len() {
                return Err(invalid_data("invalid X1 fixed vector length"));
            }
        }
        Ok(Self {
            bytes,
            count,
            stride,
            payload_start,
        })
    }

    fn len(&self) -> usize {
        self.count
    }

    fn validate(&self) -> io::Result<()> {
        if self.stride != 0 {
            return Ok(());
        }
        let payload_len = self.bytes.len() - self.payload_start;
        let mut previous = 0usize;
        for index in 0..=self.count {
            let offset = self.offset(index)?;
            if offset < previous || offset > payload_len {
                return Err(invalid_data("invalid X1 vector record offset"));
            }
            previous = offset;
        }
        if previous != payload_len {
            return Err(invalid_data("X1 variable vector has trailing bytes"));
        }
        Ok(())
    }

    fn get<T: BinaryValue>(&self, index: usize) -> io::Result<T> {
        let bytes = self.record(index)?;
        let mut reader = BinaryReader::new(Cursor::new(bytes));
        let value = T::read_from(&mut reader)?;
        if reader.inner.position() as usize != bytes.len() {
            return Err(invalid_data("X1 record has trailing bytes"));
        }
        Ok(value)
    }

    fn get_str(&self, index: usize) -> io::Result<&'a str> {
        if self.stride != 0 {
            return Err(invalid_data("X1 string vector must be variable-width"));
        }
        let bytes = self.record(index)?;
        std::str::from_utf8(bytes).map_err(|_| invalid_data("invalid X1 string UTF-8"))
    }

    fn get_x1_available_member_hot(&self, index: usize) -> io::Result<X1AvailableMemberHot> {
        if self.stride != 8 {
            return Err(invalid_data("X1 member hot vector must be fixed-width"));
        }
        let bytes = self.record(index)?;
        Ok(X1AvailableMemberHot {
            member_id: read_u32_at(bytes, 0)?,
            availability_word: read_u16_at(bytes, 4)?,
            kind: *bytes
                .get(6)
                .ok_or_else(|| invalid_data("truncated X1 member hot kind"))?,
            reserved: *bytes
                .get(7)
                .ok_or_else(|| invalid_data("truncated X1 member hot reserved byte"))?,
        })
    }

    fn record(&self, index: usize) -> io::Result<&'a [u8]> {
        if index >= self.count {
            return Err(invalid_data("X1 vector index out of bounds"));
        }
        let (start, end) = if self.stride == 0 {
            (self.offset(index)?, self.offset(index + 1)?)
        } else {
            let start = index
                .checked_mul(self.stride)
                .ok_or_else(|| invalid_data("X1 fixed record offset overflow"))?;
            (start, start + self.stride)
        };
        Ok(&self.bytes[self.payload_start + start..self.payload_start + end])
    }

    fn offset(&self, index: usize) -> io::Result<usize> {
        let offset_at = 8 + index * 4;
        Ok(read_u32_at(self.bytes, offset_at)? as usize)
    }
}
fn file_sha256(path: impl AsRef<Path>) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn read_ascii(
    bytes: &[u8],
    offset: usize,
    width: usize,
    _field: &'static str,
) -> io::Result<String> {
    let actual = bytes
        .get(offset..offset + width)
        .ok_or_else(|| invalid_data("truncated X1 fixed ASCII field"))?;
    if !actual.is_ascii() {
        return Err(invalid_data("X1 fixed ASCII header is not ASCII"));
    }
    std::str::from_utf8(actual)
        .map(str::to_string)
        .map_err(|_| invalid_data("X1 fixed ASCII header is not UTF-8"))
}

fn read_fixed_ascii(
    bytes: &[u8],
    offset: usize,
    width: usize,
    _field: &'static str,
) -> io::Result<String> {
    let value = read_ascii(bytes, offset, width, "fixed field")?;
    let value = value.trim_end_matches('\0');
    if value.is_empty() {
        return Err(invalid_data("X1 fixed ASCII header field is empty"));
    }
    Ok(value.to_string())
}

fn validate_fixed_field(value: &str, width: usize, _field: &'static str) -> io::Result<()> {
    if value.is_empty() || value.len() > width || !value.is_ascii() || value.as_bytes().contains(&0)
    {
        return Err(invalid_data("X1 fixed ASCII field is invalid"));
    }
    Ok(())
}

fn validate_sha256(value: &str) -> io::Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid_data("X1 SHA-256 is invalid"));
    }
    Ok(())
}

fn validate_platform_version(value: &str) -> io::Result<()> {
    if value.split('.').count() != 4
        || value
            .split('.')
            .any(|part| part.is_empty() || part.parse::<u32>().is_err())
    {
        return Err(invalid_data("X1 platform version is invalid"));
    }
    Ok(())
}

fn parse_metadata_u64(value: &str) -> io::Result<u64> {
    value
        .parse()
        .map_err(|_| invalid_data("X1 compatibility u64 is invalid"))
}

fn parse_metadata_u32(value: &str) -> io::Result<u32> {
    value
        .parse()
        .map_err(|_| invalid_data("X1 compatibility u32 is invalid"))
}

fn read_u32_at(bytes: &[u8], offset: usize) -> io::Result<u32> {
    let bytes = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| invalid_data("truncated X1 u32"))?;
    Ok(u32::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

fn read_u16_at(bytes: &[u8], offset: usize) -> io::Result<u16> {
    let bytes = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| invalid_data("truncated X1 u16"))?;
    Ok(u16::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

fn read_u64_at(bytes: &[u8], offset: usize) -> io::Result<u64> {
    let bytes = bytes
        .get(offset..offset + 8)
        .ok_or_else(|| invalid_data("truncated X1 u64"))?;
    Ok(u64::from_le_bytes(
        bytes.try_into().expect("slice length checked"),
    ))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    fnv1a_update(FNV_OFFSET_BASIS, bytes)
}

fn fnv1a_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[cfg(feature = "snapshot-experiment-alloc")]
    #[global_allocator]
    static X1_TEST_ALLOCATOR: HbkSnapshotExperimentAllocator = HbkSnapshotExperimentAllocator;

    #[test]
    fn x1_encoding_is_deterministic_and_fully_validated() {
        let snapshot = fixture_snapshot();
        let first = encode_snapshot(&snapshot).unwrap();
        let second = encode_snapshot(&snapshot).unwrap();

        assert_eq!(first, second);
        let (_, counts, locale, _) = validate_mmap(&first).unwrap();
        assert_eq!(counts, snapshot.counts());
        assert_eq!(locale, Some(StringId(0)));
    }

    #[test]
    fn x1_accepts_legacy_provider_without_optional_fact_provenance() {
        let mut snapshot = fixture_snapshot();
        snapshot.source_by_fact.clear();

        assert!(validate_mmap(&encode_snapshot(&snapshot).unwrap()).is_ok());
    }

    #[test]
    fn x1_validator_rejects_truncation_and_checksum_corruption() {
        let bytes = encode_snapshot(&fixture_snapshot()).unwrap();
        assert!(validate_mmap(&bytes[..bytes.len() - 1]).is_err());

        let mut corrupt = bytes;
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0x55;
        assert!(validate_mmap(&corrupt).is_err());
    }

    #[test]
    fn x1_validator_rejects_misaligned_and_overlapping_sections() {
        let bytes = encode_snapshot(&fixture_snapshot()).unwrap();

        let mut misaligned = bytes.clone();
        let first_offset = read_u64_at(&misaligned, HEADER_LEN).unwrap();
        misaligned[HEADER_LEN..HEADER_LEN + 8].copy_from_slice(&(first_offset + 1).to_le_bytes());
        assert!(validate_mmap(&misaligned).is_err());

        let mut overlapping = bytes;
        let second_entry = HEADER_LEN + DIRECTORY_ENTRY_LEN;
        overlapping[second_entry..second_entry + 8].copy_from_slice(&first_offset.to_le_bytes());
        assert!(validate_mmap(&overlapping).is_err());
    }

    #[test]
    fn x1_validator_rejects_invalid_utf8_range_and_tag() {
        let bytes = encode_snapshot(&fixture_snapshot()).unwrap();
        let (sections, _, _, _) = validate_mmap(&bytes).unwrap();

        let mut invalid_utf8 = bytes.clone();
        let strings = VectorView::new(section_bytes(&invalid_utf8, &sections, S::Strings)).unwrap();
        let at = strings.record(0).unwrap().as_ptr() as usize - invalid_utf8.as_ptr() as usize;
        invalid_utf8[at] = 0xff;
        rewrite_payload_checksum(&mut invalid_utf8);
        assert!(validate_mmap(&invalid_utf8).is_err());

        let mut invalid_range = bytes.clone();
        let types =
            VectorView::new(section_bytes(&invalid_range, &sections, S::PlatformTypes)).unwrap();
        let at = types.record(0).unwrap().as_ptr() as usize - invalid_range.as_ptr() as usize;
        invalid_range[at + 24..at + 28].copy_from_slice(&u32::MAX.to_le_bytes());
        rewrite_payload_checksum(&mut invalid_range);
        assert!(validate_mmap(&invalid_range).is_err());

        let mut invalid_tag = bytes;
        let fact_ids = VectorView::new(section_bytes(&invalid_tag, &sections, S::FactIds)).unwrap();
        let at = fact_ids.record(0).unwrap().as_ptr() as usize - invalid_tag.as_ptr() as usize;
        invalid_tag[at + 4] = u8::MAX;
        rewrite_payload_checksum(&mut invalid_tag);
        assert!(validate_mmap(&invalid_tag).is_err());
    }

    #[test]
    fn x1_validator_rejects_version_and_identity_mismatch() {
        let identity = test_identity();
        let bytes = encode_snapshot_with_identity(&fixture_snapshot(), &identity).unwrap();

        let mut wrong_layout = bytes.clone();
        wrong_layout[8..12].copy_from_slice(&(LAYOUT_VERSION + 1).to_le_bytes());
        assert!(validate_mmap(&wrong_layout).is_err());

        let mut wrong_source_locale = bytes.clone();
        wrong_source_locale[40..44].copy_from_slice(&1_u32.to_le_bytes());
        assert!(validate_mmap(&wrong_source_locale).is_err());

        let mut wrong_identity = identity;
        wrong_identity.platform_version = "8.3.0.1".to_string();
        assert!(validate_mmap_expected(&bytes, Some(&wrong_identity)).is_err());
    }

    #[test]
    fn x1_source_binding_requires_shcntx_help_book_in_version_directory() {
        assert_eq!(
            platform_version_from_source_path(Path::new("/fixture/8.3.27.1859/shcntx_ru.hbk"))
                .unwrap(),
            "8.3.27.1859"
        );
        assert!(
            platform_version_from_source_path(Path::new(
                "/fixture/8.3.27.1859/not-a-help-book.hbk"
            ))
            .is_err()
        );
        assert!(
            platform_version_from_source_path(Path::new("/fixture/not-a-version/shcntx_ru.hbk"))
                .is_err()
        );
    }

    #[test]
    fn x1_validator_rejects_fact_provenance_from_another_source() {
        let mut bytes = encode_snapshot(&fixture_snapshot()).unwrap();
        let (sections, _, _, _) = validate_mmap(&bytes).unwrap();
        let sources = VectorView::new(section_bytes(&bytes, &sections, S::SourceByFact)).unwrap();
        let at = sources.record(0).unwrap().as_ptr() as usize - bytes.as_ptr() as usize;
        bytes[at + 5..at + 9].copy_from_slice(&2_u32.to_le_bytes());
        rewrite_payload_checksum(&mut bytes);

        assert!(validate_mmap(&bytes).is_err());
    }

    #[test]
    fn x1_validator_rejects_template_binding_parameter_overflow() {
        let bytes = encode_snapshot(&template_binding_fixture_snapshot()).unwrap();
        let (sections, _, _, _) = validate_mmap(&bytes).unwrap();
        let arguments =
            VectorView::new(section_bytes(&bytes, &sections, S::TemplateArguments)).unwrap();
        let at = arguments.record(0).unwrap().as_ptr() as usize - bytes.as_ptr() as usize;

        let mut invalid_owner = bytes.clone();
        invalid_owner[at + 1..at + 9].copy_from_slice(&1_u64.to_le_bytes());
        rewrite_payload_checksum(&mut invalid_owner);
        assert!(validate_mmap(&invalid_owner).is_err());

        let mut invalid_target = bytes;
        invalid_target[at + 9..at + 17].copy_from_slice(&1_u64.to_le_bytes());
        rewrite_payload_checksum(&mut invalid_target);
        assert!(validate_mmap(&invalid_target).is_err());
    }

    #[test]
    fn x1_type_hash_handles_distinct_keys_with_the_same_initial_bucket() {
        let mut snapshot = fixture_snapshot();
        let capacity = 4usize;
        let mut by_bucket = std::collections::BTreeMap::<usize, Vec<String>>::new();
        let pair = (0..128)
            .find_map(|index| {
                let key = format!("collision-key-{index}");
                let bucket = (x1_hash_key(&key) as usize) & (capacity - 1);
                let values = by_bucket.entry(bucket).or_default();
                values.push(key);
                (values.len() == 2).then(|| values.clone())
            })
            .unwrap();
        let first = StringId(snapshot.strings.len() as u32);
        snapshot.strings.push(pair[0].clone());
        let second = StringId(snapshot.strings.len() as u32);
        snapshot.strings.push(pair[1].clone());
        let names = vec![
            NameLookup {
                key: first,
                value: HbkPlatformTypeId(0),
            },
            NameLookup {
                key: second,
                value: HbkPlatformTypeId(0),
            },
        ];

        let buckets = build_x1_platform_type_name_hash(&names, &snapshot).unwrap();
        let occupied = buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| bucket.count != 0)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        assert_eq!(buckets.len(), capacity);
        assert_eq!(occupied.len(), 2);
        let initial = (x1_hash_key(&pair[0]) as usize) & (capacity - 1);
        assert!(occupied.contains(&initial));
        assert!(occupied.contains(&((initial + 1) & (capacity - 1))));
    }

    #[test]
    fn x1_mapped_type_name_lookup_follows_a_collision_probe_chain() {
        let mut snapshot = fixture_snapshot();
        let capacity = 4usize;
        let mut by_bucket = std::collections::BTreeMap::<usize, Vec<String>>::new();
        let pair = (0..128)
            .find_map(|index| {
                let key = format!("collision-key-{index}");
                let bucket = (x1_hash_key(&key) as usize) & (capacity - 1);
                let values = by_bucket.entry(bucket).or_default();
                values.push(key);
                (values.len() == 2).then(|| values.clone())
            })
            .unwrap();
        let first = push_fixture_string(&mut snapshot, &pair[0]);
        let second = push_fixture_string(&mut snapshot, &pair[1]);
        snapshot.platform_type_names = vec![
            NameLookup {
                key: first,
                value: HbkPlatformTypeId(0),
            },
            NameLookup {
                key: second,
                value: HbkPlatformTypeId(0),
            },
        ];
        let strings = &snapshot.strings;
        snapshot.platform_type_names.sort_by(|left, right| {
            strings[left.key.0 as usize]
                .cmp(&strings[right.key.0 as usize])
                .then_with(|| left.value.cmp(&right.value))
        });

        let root = temp_path("x1-lookup-collision");
        fs::create_dir_all(&root).unwrap();
        let artifact = root.join("generation.x1");
        let identity = test_identity();
        let bytes = encode_snapshot_with_identity(&snapshot, &identity).unwrap();
        write_readonly_artifact(&artifact, &bytes);
        let mapped =
            open_controlled_generation(&artifact, &runtime_expectation(&identity)).unwrap();
        let read = mapped.read_handle();

        for key in &pair {
            assert_eq!(
                read.platform_types_by_name(key).collect::<Vec<_>>(),
                vec![HbkPlatformTypeId(0)]
            );
        }
        assert_eq!(read.platform_types_by_name("missing").len(), 0);

        drop(mapped);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_availability_mask_uses_the_frozen_nine_context_registry() {
        let contexts = [
            "thin_client",
            "web_client",
            "mobile_client",
            "server",
            "thick_client",
            "external_connection",
            "mobile_application_client",
            "mobile_application_server",
            "mobile_standalone_server",
        ];
        for (index, context) in contexts.into_iter().enumerate() {
            assert_eq!(x1_context_code_bit(context), Some(1 << index));
        }
        assert_eq!(x1_context_code_bit("module_context_kind"), None);
    }

    #[test]
    fn x1_writer_uses_real_version_directory_and_never_overwrites() {
        let root = temp_path("x1-writer");
        let source_dir = root.join("8.3.27.1859");
        fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("shcntx_ru.hbk");
        fs::write(&source, b"minimal hbk fixture").unwrap();
        let alias_dir = source_dir.join("alias");
        fs::create_dir(&alias_dir).unwrap();
        let source_alias = alias_dir.join("..").join("shcntx_ru.hbk");
        let index_path = root.join("provider.sqlite");
        let metadata = IndexMetadata {
            locale: "ru".to_string(),
            source_locale: "ru".to_string(),
            source_hbk: source_alias.to_string_lossy().into_owned(),
            source_extraction_schema_version: SUPPORTED_EXTRACTION_SCHEMA,
        };
        build_index_from_builder(&index_path, &metadata, fixture_index_builder(&source_alias))
            .unwrap();
        let report = HbkFactSnapshot::from_path_with_stage_timings(&index_path).unwrap();
        let artifact = root.join("generation.x1");
        let second_artifact = root.join("generation-second.x1");

        let written = report.write_x1_generation(&artifact).unwrap();
        report.write_x1_generation(&second_artifact).unwrap();
        let original = fs::read(&artifact).unwrap();
        assert_eq!(written.platform_version, "8.3.27.1859");
        assert_eq!(written.artifact_bytes, original.len() as u64);
        assert!(validate_mmap(&original).is_ok());
        assert_eq!(fs::read(&second_artifact).unwrap(), original);

        assert!(report.write_x1_generation(&artifact).is_err());
        assert_eq!(fs::read(&artifact).unwrap(), original);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_slot_publishes_reuses_and_opens_without_source_or_sql_provider() {
        let root = temp_path("x1-slot-publish");
        let input_root = root.join("input");
        let slot = root.join("slot");
        let (report, identity, source, index) =
            fixture_build_report(&input_root, b"first source generation");
        let expected_counts = report.snapshot.counts();

        let first = report.publish_x1_generation(&slot).unwrap();
        assert!(!first.reused_existing_generation);
        assert_eq!(first.artifact_sha256.len(), SHA256_HEX_LEN);
        assert_eq!(
            fs::read_to_string(slot.join(X1_SLOT_CURRENT_FILE)).unwrap(),
            format!("{}\n", first.generation_file_name)
        );
        let generation = slot
            .join(X1_SLOT_GENERATIONS_DIR)
            .join(&first.generation_file_name);
        assert_eq!(file_sha256(&generation).unwrap(), first.artifact_sha256);

        let second = report.publish_x1_generation(&slot).unwrap();
        assert!(second.reused_existing_generation);
        assert_eq!(second.generation_file_name, first.generation_file_name);
        assert_eq!(
            slot_generation_names(&slot),
            vec![first.generation_file_name]
        );

        fs::remove_file(source).unwrap();
        fs::remove_file(index).unwrap();
        let mapped = X1StableSlotGeneration::open(&slot, &runtime_expectation(&identity)).unwrap();
        assert_eq!(mapped.artifact_len() as u64, first.artifact_bytes);
        assert_eq!(mapped.generation.counts, expected_counts);

        drop(mapped);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_slot_shared_readers_block_publication_without_touching_slot() {
        let root = temp_path("x1-slot-lock");
        let slot = root.join("slot");
        let (report, identity, _, _) =
            fixture_build_report(&root.join("input"), b"locked source generation");
        report.publish_x1_generation(&slot).unwrap();
        let first = X1StableSlotGeneration::open(&slot, &runtime_expectation(&identity)).unwrap();
        let second = X1StableSlotGeneration::open(&slot, &runtime_expectation(&identity)).unwrap();
        let current_before = fs::read(slot.join(X1_SLOT_CURRENT_FILE)).unwrap();
        let entries_before = slot_tree_names(&slot);

        assert!(matches!(
            report.publish_x1_generation(&slot),
            Err(SearchError::SnapshotInUse { path }) if path == slot
        ));
        assert_eq!(
            fs::read(slot.join(X1_SLOT_CURRENT_FILE)).unwrap(),
            current_before
        );
        assert_eq!(slot_tree_names(&slot), entries_before);

        drop(first);
        assert!(matches!(
            report.publish_x1_generation(&slot),
            Err(SearchError::SnapshotInUse { .. })
        ));
        drop(second);
        assert!(
            report
                .publish_x1_generation(&slot)
                .unwrap()
                .reused_existing_generation
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_slot_concurrent_first_setup_uses_one_stable_lock_inode() {
        use std::sync::{Arc, Barrier};

        let root = temp_path("x1-slot-first-setup");
        let slot = root.join("slot");
        let (report, _, _, _) =
            fixture_build_report(&root.join("input"), b"concurrent first setup");
        let acquired = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let worker_report = report.clone();
        let worker_slot = slot.clone();
        let worker_acquired = Arc::clone(&acquired);
        let worker_release = Arc::clone(&release);
        let worker = std::thread::spawn(move || {
            worker_report.publish_x1_generation_with_options(
                &worker_slot,
                X1PublicationOptions {
                    nonce: "first-writer".to_string(),
                    fail_at: None,
                    lock_hook: Some(X1PublicationLockHook {
                        acquired: worker_acquired,
                        release: worker_release,
                    }),
                },
            )
        });

        acquired.wait();
        assert!(matches!(
            report.publish_x1_generation(&slot),
            Err(SearchError::SnapshotInUse { path }) if path == slot
        ));
        release.wait();
        assert!(worker.join().unwrap().is_ok());
        assert!(slot.join(X1_SLOT_LOCK_FILE).is_file());
        assert_eq!(slot_generation_names(&slot).len(), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_slot_pointer_parser_and_content_address_fail_closed() {
        let pointer_path = Path::new("current");
        let valid_name = format!(
            "{X1_GENERATION_PREFIX}{}{X1_GENERATION_SUFFIX}",
            "a".repeat(SHA256_HEX_LEN)
        );
        assert_eq!(
            parse_current_pointer(pointer_path, format!("{valid_name}\n").as_bytes()).unwrap(),
            valid_name
        );
        for invalid in [
            b"".as_slice(),
            b"../generation.x1\n".as_slice(),
            format!(
                "{X1_GENERATION_PREFIX}{}{X1_GENERATION_SUFFIX}\n",
                "A".repeat(SHA256_HEX_LEN)
            )
            .as_bytes(),
            format!(
                "{X1_GENERATION_PREFIX}{}{X1_GENERATION_SUFFIX}",
                "a".repeat(SHA256_HEX_LEN)
            )
            .as_bytes(),
        ] {
            assert!(parse_current_pointer(pointer_path, invalid).is_err());
        }

        let root = temp_path("x1-slot-content-address");
        let slot = root.join("slot");
        let (report, identity, _, _) =
            fixture_build_report(&root.join("input"), b"content-address source");
        let published = report.publish_x1_generation(&slot).unwrap();
        let original = slot
            .join(X1_SLOT_GENERATIONS_DIR)
            .join(&published.generation_file_name);
        let wrong_name = format!(
            "{X1_GENERATION_PREFIX}{}{X1_GENERATION_SUFFIX}",
            "0".repeat(SHA256_HEX_LEN)
        );
        let wrong = slot.join(X1_SLOT_GENERATIONS_DIR).join(&wrong_name);
        fs::copy(&original, &wrong).unwrap();
        make_readonly(&wrong);
        overwrite_readonly(
            &slot.join(X1_SLOT_CURRENT_FILE),
            format!("{wrong_name}\n").as_bytes(),
        );
        assert!(matches!(
            X1StableSlotGeneration::open(&slot, &runtime_expectation(&identity)),
            Err(SearchError::SnapshotArtifact { .. })
        ));
        assert!(matches!(
            report.publish_x1_generation(&slot),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_slot_rejects_corrupt_existing_generation_without_replacing_it() {
        let root = temp_path("x1-slot-corrupt-existing");
        let slot = root.join("slot");
        let (report, _, _, _) =
            fixture_build_report(&root.join("input"), b"corrupt existing source");
        let publication = report.publish_x1_generation(&slot).unwrap();
        let generation_path = slot
            .join(X1_SLOT_GENERATIONS_DIR)
            .join(publication.generation_file_name);
        fs::remove_file(slot.join(X1_SLOT_CURRENT_FILE)).unwrap();
        let mut corrupt = fs::read(&generation_path).unwrap();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0x5a;
        overwrite_readonly(&generation_path, &corrupt);

        assert!(matches!(
            report.publish_x1_generation(&slot),
            Err(SearchError::SnapshotArtifact { .. })
        ));
        assert_eq!(fs::read(&generation_path).unwrap(), corrupt);
        assert!(!slot.join(X1_SLOT_CURRENT_FILE).exists());
        assert_eq!(slot_generation_names(&slot).len(), 1);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_slot_failure_phases_leave_only_valid_recovery_states() {
        let root = temp_path("x1-slot-failures");
        let (old_report, old_identity, _, _) =
            fixture_build_report(&root.join("old-input"), b"old source generation");
        let (new_report, new_identity, _, _) =
            fixture_build_report(&root.join("new-input"), b"new source generation");
        let new_counts = new_report.snapshot.counts();

        let before_slot = root.join("before");
        let old_before = old_report.publish_x1_generation(&before_slot).unwrap();
        let old_before_pointer = fs::read(before_slot.join(X1_SLOT_CURRENT_FILE)).unwrap();
        let stale = before_slot.join("unrelated-stale.tmp");
        fs::write(&stale, b"owned by another operation").unwrap();
        assert!(
            new_report
                .publish_x1_generation_with_options(
                    &before_slot,
                    X1PublicationOptions {
                        nonce: "before".to_string(),
                        fail_at: Some(X1PublicationFailurePoint::BeforeGeneration),
                        lock_hook: None,
                    },
                )
                .is_err()
        );
        assert!(stale.exists());
        assert_eq!(
            fs::read(before_slot.join(X1_SLOT_CURRENT_FILE)).unwrap(),
            old_before_pointer
        );
        assert_eq!(slot_generation_names(&before_slot).len(), 1);
        assert_eq!(
            slot_generation_names(&before_slot),
            vec![old_before.generation_file_name]
        );
        assert!(
            !before_slot
                .join(X1_SLOT_GENERATIONS_DIR)
                .join(".generation-before.tmp")
                .exists()
        );
        let old_session =
            X1StableSlotGeneration::open(&before_slot, &runtime_expectation(&old_identity))
                .unwrap();
        drop(old_session);

        let between_slot = root.join("between");
        old_report.publish_x1_generation(&between_slot).unwrap();
        let old_between_pointer = fs::read(between_slot.join(X1_SLOT_CURRENT_FILE)).unwrap();
        assert!(
            new_report
                .publish_x1_generation_with_options(
                    &between_slot,
                    X1PublicationOptions {
                        nonce: "between".to_string(),
                        fail_at: Some(X1PublicationFailurePoint::GenerationPublished),
                        lock_hook: None,
                    },
                )
                .is_err()
        );
        assert_eq!(
            fs::read(between_slot.join(X1_SLOT_CURRENT_FILE)).unwrap(),
            old_between_pointer
        );
        assert_eq!(slot_generation_names(&between_slot).len(), 2);
        let old_session =
            X1StableSlotGeneration::open(&between_slot, &runtime_expectation(&old_identity))
                .unwrap();
        drop(old_session);

        let after_slot = root.join("after");
        old_report.publish_x1_generation(&after_slot).unwrap();
        assert!(
            new_report
                .publish_x1_generation_with_options(
                    &after_slot,
                    X1PublicationOptions {
                        nonce: "after".to_string(),
                        fail_at: Some(X1PublicationFailurePoint::CurrentPublished),
                        lock_hook: None,
                    },
                )
                .is_err()
        );
        let mapped =
            X1StableSlotGeneration::open(&after_slot, &runtime_expectation(&new_identity)).unwrap();
        assert_eq!(mapped.generation.counts, new_counts);

        drop(mapped);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_slot_source_replacement_isolated_by_exclusive_publication() {
        let root = temp_path("x1-slot-source-replacement");
        let slot = root.join("slot");
        let (first_report, first_identity, _, _) =
            fixture_build_report(&root.join("first-input"), b"first logical source");
        let (second_report, second_identity, _, _) =
            fixture_build_report(&root.join("second-input"), b"second logical source");
        let first_publication = first_report.publish_x1_generation(&slot).unwrap();
        let first_session =
            X1StableSlotGeneration::open(&slot, &runtime_expectation(&first_identity)).unwrap();

        assert!(matches!(
            second_report.publish_x1_generation(&slot),
            Err(SearchError::SnapshotInUse { .. })
        ));
        assert_eq!(first_session.generation.identity, first_identity);
        drop(first_session);

        let second_publication = second_report.publish_x1_generation(&slot).unwrap();
        assert_ne!(
            first_publication.generation_file_name,
            second_publication.generation_file_name
        );
        let second_session =
            X1StableSlotGeneration::open(&slot, &runtime_expectation(&second_identity)).unwrap();
        assert_eq!(second_session.generation.identity, second_identity);
        assert_eq!(slot_generation_names(&slot).len(), 2);

        drop(second_session);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_slot_reports_each_runtime_compatibility_mismatch_without_fallback() {
        let root = temp_path("x1-slot-compatibility");
        let slot = root.join("slot");
        let (report, identity, _, _) =
            fixture_build_report(&root.join("input"), b"compatibility source");
        report.publish_x1_generation(&slot).unwrap();

        let mut wrong_platform = runtime_expectation(&identity);
        wrong_platform.platform_version = "8.3.27.1860".to_string();
        let mut wrong_locale = runtime_expectation(&identity);
        wrong_locale.locale = "en".to_string();
        let mut wrong_source_locale = runtime_expectation(&identity);
        wrong_source_locale.source_locale = "en".to_string();
        let mut wrong_source = runtime_expectation(&identity);
        wrong_source.source_sha256 = "2".repeat(SHA256_HEX_LEN);

        for (field, expectation) in [
            ("platform_version", wrong_platform),
            ("locale", wrong_locale),
            ("source_locale", wrong_source_locale),
            ("source_sha256", wrong_source),
        ] {
            let Err(error) = X1StableSlotGeneration::open(&slot, &expectation) else {
                panic!("incompatible stable slot must fail closed");
            };
            assert!(matches!(
                error,
                SearchError::SnapshotArtifact {
                    source: HbkFactSnapshotArtifactError::CompatibilityMismatch {
                        field: actual,
                        ..
                    },
                    ..
                } if actual == field
            ));
        }

        let first = X1StableSlotGeneration::open(&slot, &runtime_expectation(&identity)).unwrap();
        let second = X1StableSlotGeneration::open(&slot, &runtime_expectation(&identity)).unwrap();
        assert_eq!(first.generation.identity, identity);
        assert_eq!(second.generation.identity, identity);

        drop(first);
        drop(second);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_slot_routes_format_corruption_through_the_full_byte_validator() {
        let root = temp_path("x1-slot-validator-matrix");
        let identity = test_identity();
        let bytes = encode_snapshot_with_identity(&fixture_snapshot(), &identity).unwrap();
        let mut corruptions = Vec::new();

        let mut magic = bytes.clone();
        magic[0] = b'X';
        corruptions.push(("magic", magic));

        let mut layout = bytes.clone();
        layout[8..12].copy_from_slice(&(LAYOUT_VERSION + 1).to_le_bytes());
        corruptions.push(("layout", layout));

        let mut extraction_schema = bytes.clone();
        extraction_schema[12..16].copy_from_slice(&(SUPPORTED_EXTRACTION_SCHEMA + 1).to_le_bytes());
        corruptions.push(("extraction-schema", extraction_schema));

        let mut provider_schema = bytes.clone();
        provider_schema[16..20].copy_from_slice(&(SUPPORTED_PROVIDER_SCHEMA + 1).to_le_bytes());
        corruptions.push(("provider-schema", provider_schema));

        corruptions.push(("truncated", bytes[..bytes.len() - 1].to_vec()));

        let mut checksum = bytes.clone();
        let last = checksum.len() - 1;
        checksum[last] ^= 0x55;
        corruptions.push(("checksum", checksum));

        let mut section = bytes;
        let first_offset = read_u64_at(&section, HEADER_LEN).unwrap();
        section[HEADER_LEN..HEADER_LEN + 8].copy_from_slice(&(first_offset + 1).to_le_bytes());
        corruptions.push(("section", section));

        for (name, corrupt) in corruptions {
            let slot = root.join(name);
            let generation = write_content_addressed_test_slot(&slot, &corrupt);
            validate_generation_content_address(&generation).unwrap();
            assert_eq!(
                read_current_pointer(&slot.join(X1_SLOT_CURRENT_FILE)).unwrap(),
                generation.file_name().unwrap().to_string_lossy()
            );
            assert!(matches!(
                X1StableSlotGeneration::open(&slot, &runtime_expectation(&identity)),
                Err(SearchError::SnapshotArtifact {
                    source: HbkFactSnapshotArtifactError::Invalid { .. },
                    ..
                })
            ));
        }

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_slot_reader_rejects_missing_non_regular_and_corrupt_components() {
        let root = temp_path("x1-slot-invalid-components");
        let slot = root.join("slot");
        let (report, identity, _, _) =
            fixture_build_report(&root.join("input"), b"invalid component source");
        let expectation = runtime_expectation(&identity);
        let publication = report.publish_x1_generation(&slot).unwrap();

        let lock_path = slot.join(X1_SLOT_LOCK_FILE);
        fs::remove_file(&lock_path).unwrap();
        assert!(matches!(
            X1StableSlotGeneration::open(&slot, &expectation),
            Err(SearchError::SnapshotArtifact { .. })
        ));
        fs::create_dir(&lock_path).unwrap();
        assert!(matches!(
            X1StableSlotGeneration::open(&slot, &expectation),
            Err(SearchError::SnapshotArtifact { .. })
        ));
        fs::remove_dir(&lock_path).unwrap();
        fs::write(&lock_path, b"").unwrap();

        let current_path = slot.join(X1_SLOT_CURRENT_FILE);
        fs::remove_file(&current_path).unwrap();
        assert!(matches!(
            X1StableSlotGeneration::open(&slot, &expectation),
            Err(SearchError::SnapshotArtifact { .. })
        ));
        fs::write(&current_path, b"../not-a-generation\n").unwrap();
        make_readonly(&current_path);
        assert!(matches!(
            X1StableSlotGeneration::open(&slot, &expectation),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        make_writable(&current_path);
        let huge_current = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&current_path)
            .unwrap();
        huge_current.set_len(MAX_ARTIFACT_BYTES + 1).unwrap();
        drop(huge_current);
        make_readonly(&current_path);
        assert!(matches!(
            X1StableSlotGeneration::open(&slot, &expectation),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        overwrite_readonly(
            &current_path,
            format!("{}\n", publication.generation_file_name).as_bytes(),
        );
        let generation_path = slot
            .join(X1_SLOT_GENERATIONS_DIR)
            .join(publication.generation_file_name);
        fs::remove_file(&generation_path).unwrap();
        assert!(matches!(
            X1StableSlotGeneration::open(&slot, &expectation),
            Err(SearchError::SnapshotArtifact { .. })
        ));
        let huge_generation = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&generation_path)
            .unwrap();
        huge_generation.set_len(MAX_ARTIFACT_BYTES + 1).unwrap();
        drop(huge_generation);
        make_readonly(&generation_path);
        assert!(matches!(
            X1StableSlotGeneration::open(&slot, &expectation),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn x1_slot_rejects_symlinked_layout_and_temp_components() {
        let root = temp_path("x1-slot-symlinks");
        fs::create_dir_all(&root).unwrap();
        let (report, identity, _, _) =
            fixture_build_report(&root.join("input"), b"symlink source generation");

        let actual_slot = root.join("actual-slot");
        fs::create_dir(&actual_slot).unwrap();
        let linked_slot = root.join("linked-slot");
        symlink(&actual_slot, &linked_slot).unwrap();
        assert!(matches!(
            report.publish_x1_generation(&linked_slot),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        let ancestor_target = root.join("ancestor-target");
        fs::create_dir(&ancestor_target).unwrap();
        let ancestor_link = root.join("ancestor-link");
        symlink(&ancestor_target, &ancestor_link).unwrap();
        assert!(matches!(
            report.publish_x1_generation(ancestor_link.join("nested-slot")),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        let generations_slot = root.join("generations-slot");
        fs::create_dir(&generations_slot).unwrap();
        let actual_generations = root.join("actual-generations");
        fs::create_dir(&actual_generations).unwrap();
        symlink(
            &actual_generations,
            generations_slot.join(X1_SLOT_GENERATIONS_DIR),
        )
        .unwrap();
        assert!(matches!(
            report.publish_x1_generation(&generations_slot),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        let lock_slot = root.join("lock-slot");
        fs::create_dir_all(lock_slot.join(X1_SLOT_GENERATIONS_DIR)).unwrap();
        let lock_target = root.join("lock-target");
        fs::write(&lock_target, b"").unwrap();
        symlink(&lock_target, lock_slot.join(X1_SLOT_LOCK_FILE)).unwrap();
        assert!(matches!(
            report.publish_x1_generation(&lock_slot),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        let published_slot = root.join("published-slot");
        let publication = report.publish_x1_generation(&published_slot).unwrap();
        let reader_ancestor_link = root.join("reader-ancestor-link");
        symlink(&root, &reader_ancestor_link).unwrap();
        assert!(matches!(
            X1StableSlotGeneration::open(
                &reader_ancestor_link.join("published-slot"),
                &runtime_expectation(&identity),
            ),
            Err(SearchError::SnapshotArtifact { .. })
        ));
        let current_path = published_slot.join(X1_SLOT_CURRENT_FILE);
        let current_target = root.join("current-target");
        fs::write(&current_target, fs::read(&current_path).unwrap()).unwrap();
        fs::remove_file(&current_path).unwrap();
        symlink(&current_target, &current_path).unwrap();
        assert!(matches!(
            X1StableSlotGeneration::open(&published_slot, &runtime_expectation(&identity)),
            Err(SearchError::SnapshotArtifact { .. })
        ));
        assert!(matches!(
            report.publish_x1_generation(&published_slot),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        fs::remove_file(&current_path).unwrap();
        fs::copy(&current_target, &current_path).unwrap();
        make_readonly(&current_path);
        let generation_path = published_slot
            .join(X1_SLOT_GENERATIONS_DIR)
            .join(publication.generation_file_name);
        let generation_target = root.join("generation-target");
        fs::rename(&generation_path, &generation_target).unwrap();
        symlink(&generation_target, &generation_path).unwrap();
        assert!(matches!(
            X1StableSlotGeneration::open(&published_slot, &runtime_expectation(&identity)),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        let temp_slot = root.join("temp-slot");
        fs::create_dir_all(temp_slot.join(X1_SLOT_GENERATIONS_DIR)).unwrap();
        let temp_target = root.join("temp-target");
        fs::write(&temp_target, b"").unwrap();
        symlink(
            &temp_target,
            temp_slot
                .join(X1_SLOT_GENERATIONS_DIR)
                .join(".generation-fixed.tmp"),
        )
        .unwrap();
        assert!(matches!(
            report.publish_x1_generation_with_options(
                &temp_slot,
                X1PublicationOptions {
                    nonce: "fixed".to_string(),
                    fail_at: None,
                    lock_hook: None,
                },
            ),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        let pointer_temp_slot = root.join("pointer-temp-slot");
        fs::create_dir_all(pointer_temp_slot.join(X1_SLOT_GENERATIONS_DIR)).unwrap();
        symlink(&temp_target, pointer_temp_slot.join(".current-pointer.tmp")).unwrap();
        assert!(matches!(
            report.publish_x1_generation_with_options(
                &pointer_temp_slot,
                X1PublicationOptions {
                    nonce: "pointer".to_string(),
                    fail_at: None,
                    lock_hook: None,
                },
            ),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_writer_rejects_source_without_version_directory() {
        let root = temp_path("x1-invalid-version");
        fs::create_dir_all(&root).unwrap();
        let source = root.join("shcntx_ru.hbk");
        fs::write(&source, b"minimal hbk fixture").unwrap();
        let index_path = root.join("provider.sqlite");
        let metadata = IndexMetadata {
            locale: "ru".to_string(),
            source_locale: "ru".to_string(),
            source_hbk: source.to_string_lossy().into_owned(),
            source_extraction_schema_version: SUPPORTED_EXTRACTION_SCHEMA,
        };
        build_index_from_builder(&index_path, &metadata, fixture_index_builder(&source)).unwrap();
        let report = HbkFactSnapshot::from_path_with_stage_timings(&index_path).unwrap();

        assert!(
            report
                .write_x1_generation(root.join("generation.x1"))
                .is_err()
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_mapped_generation_opens_without_hbk_or_sql_inputs() {
        let root = temp_path("x1-mmap-open");
        fs::create_dir_all(&root).unwrap();
        let artifact = root.join("generation.x1");
        let identity = test_identity();
        let bytes = encode_snapshot_with_identity(&fixture_snapshot(), &identity).unwrap();
        write_readonly_artifact(&artifact, &bytes);

        let mapped =
            open_controlled_generation(&artifact, &runtime_expectation(&identity)).unwrap();

        assert_eq!(mapped.artifact_len(), bytes.len());
        assert_eq!(mapped.counts, fixture_snapshot().counts());
        assert_eq!(mapped.source_locale, StringId(0));
        assert_eq!(mapped.identity, identity);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_mapped_generation_rejects_writable_non_regular_and_mismatched_inputs() {
        let root = temp_path("x1-mmap-rejects");
        fs::create_dir_all(&root).unwrap();
        let identity = test_identity();
        let bytes = encode_snapshot_with_identity(&fixture_snapshot(), &identity).unwrap();
        let expectation = runtime_expectation(&identity);

        let writable = root.join("writable.x1");
        fs::write(&writable, &bytes).unwrap();
        assert!(matches!(
            open_controlled_generation(&writable, &expectation),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        assert!(matches!(
            open_controlled_generation(&root, &expectation),
            Err(SearchError::SnapshotArtifact { .. })
        ));

        let readonly = root.join("readonly.x1");
        write_readonly_artifact(&readonly, &bytes);
        let mut wrong_platform = expectation.clone();
        wrong_platform.platform_version = "8.3.0.1".to_string();
        let mut wrong_locale = expectation.clone();
        wrong_locale.locale = "en".to_string();
        let mut wrong_source_locale = expectation.clone();
        wrong_source_locale.source_locale = "en".to_string();
        let mut wrong_source = expectation;
        wrong_source.source_sha256 = "2".repeat(64);
        for (field, wrong) in [
            ("platform_version", wrong_platform),
            ("locale", wrong_locale),
            ("source_locale", wrong_source_locale),
            ("source_sha256", wrong_source),
        ] {
            assert_compatibility_mismatch(&readonly, &wrong, field);
        }

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_mapped_generation_uses_full_validator_before_returning_owner() {
        let root = temp_path("x1-mmap-full-validator");
        fs::create_dir_all(&root).unwrap();
        let identity = test_identity();
        let bytes = encode_snapshot_with_identity(&fixture_snapshot(), &identity).unwrap();
        let mut corruptions = Vec::new();
        let mut magic = bytes.clone();
        magic[0] = b'X';
        corruptions.push(("magic", magic));
        let mut layout = bytes.clone();
        layout[8..12].copy_from_slice(&(LAYOUT_VERSION + 1).to_le_bytes());
        corruptions.push(("layout", layout));
        let mut extraction_schema = bytes.clone();
        extraction_schema[12..16].copy_from_slice(&(SUPPORTED_EXTRACTION_SCHEMA + 1).to_le_bytes());
        corruptions.push(("extraction-schema", extraction_schema));
        let mut provider_schema = bytes;
        provider_schema[16..20].copy_from_slice(&(SUPPORTED_PROVIDER_SCHEMA + 1).to_le_bytes());
        corruptions.push(("provider-schema", provider_schema));

        for (name, corrupt) in corruptions {
            let artifact = root.join(format!("corrupt-{name}.x1"));
            write_readonly_artifact(&artifact, &corrupt);
            assert!(matches!(
                open_controlled_generation(&artifact, &runtime_expectation(&identity)),
                Err(SearchError::SnapshotArtifact {
                    source: HbkFactSnapshotArtifactError::Invalid { .. },
                    ..
                })
            ));
        }

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_mapped_generation_rejects_truncation_checksum_and_section_corruption() {
        let root = temp_path("x1-mmap-corruption");
        fs::create_dir_all(&root).unwrap();
        let identity = test_identity();
        let bytes = encode_snapshot_with_identity(&fixture_snapshot(), &identity).unwrap();
        let expectation = runtime_expectation(&identity);

        let truncated = root.join("truncated.x1");
        write_readonly_artifact(&truncated, &bytes[..bytes.len() - 1]);
        assert!(open_controlled_generation(&truncated, &expectation).is_err());

        let mut checksum = bytes.clone();
        let last = checksum.len() - 1;
        checksum[last] ^= 0x55;
        let checksum_path = root.join("checksum.x1");
        write_readonly_artifact(&checksum_path, &checksum);
        assert!(open_controlled_generation(&checksum_path, &expectation).is_err());

        let mut section = bytes;
        let first_offset = read_u64_at(&section, HEADER_LEN).unwrap();
        section[HEADER_LEN..HEADER_LEN + 8].copy_from_slice(&(first_offset + 1).to_le_bytes());
        let section_path = root.join("section.x1");
        write_readonly_artifact(&section_path, &section);
        assert!(open_controlled_generation(&section_path, &expectation).is_err());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_mapped_forward_payload_matches_owned_fixture() {
        let root = temp_path("x1-forward-payload");
        fs::create_dir_all(&root).unwrap();
        let artifact = root.join("generation.x1");
        let identity = test_identity();
        let snapshot = forward_payload_fixture_snapshot();
        let bytes = encode_snapshot_with_identity(&snapshot, &identity).unwrap();
        write_readonly_artifact(&artifact, &bytes);
        let mapped =
            open_controlled_generation(&artifact, &runtime_expectation(&identity)).unwrap();
        let read = mapped.read_handle();

        assert_eq!(read.source_locale(), "ru");
        assert_eq!(read.string(StringId(1)), snapshot.string(StringId(1)));

        let platform_type = read.platform_type(HbkPlatformTypeId(0));
        let owned_type = snapshot.platform_type(HbkPlatformTypeId(0));
        assert_eq!(platform_type.id(), owned_type.id);
        assert_name(platform_type.name(), &owned_type.name);
        assert_eq!(
            platform_type.type_template_key(),
            owned_type.type_template_key
        );
        assert_eq!(
            platform_type.availability_contexts().collect::<Vec<_>>(),
            owned_type.availability_contexts
        );
        let metadata = platform_type.metadata_template().unwrap();
        let owned_metadata = owned_type.metadata_template.as_ref().unwrap();
        assert_eq!(metadata.metadata_kind(), owned_metadata.metadata_kind);
        assert_eq!(
            metadata.template_parameters().collect::<Vec<_>>(),
            owned_metadata.template_parameters
        );

        for (index, owned) in snapshot.type_members.iter().enumerate() {
            let view = read.type_member(HbkTypeMemberId(index as u32));
            assert_eq!(view.id(), owned.id);
            assert_eq!(view.owner(), owned.owner);
            assert_eq!(view.kind(), owned.kind);
            assert_name(view.name(), &owned.name);
            assert_type_refs(view.type_refs(), &owned.type_refs);
            assert_eq!(
                view.availability_contexts().collect::<Vec<_>>(),
                owned.availability_contexts
            );
        }

        for (index, owned) in snapshot.callables.iter().enumerate() {
            let view = read.callable(HbkCallableId(index as u32));
            assert_eq!(view.id(), owned.id);
            assert_eq!(view.owner(), owned.owner);
            assert_eq!(view.kind(), owned.kind);
            assert_name(view.name(), &owned.name);
            let signatures = view.signatures().collect::<Vec<_>>();
            assert_eq!(signatures.len(), owned.signatures.len());
            for (signature, owned_signature) in signatures.into_iter().zip(&owned.signatures) {
                assert_eq!(signature.text(), owned_signature.text);
                let parameters = signature.parameters().collect::<Vec<_>>();
                assert_eq!(parameters.len(), owned_signature.parameters.len());
                for (parameter, owned_parameter) in
                    parameters.into_iter().zip(&owned_signature.parameters)
                {
                    assert_eq!(parameter.name(), owned_parameter.name);
                    assert_eq!(parameter.required(), owned_parameter.required);
                    assert_type_refs(parameter.type_refs(), &owned_parameter.type_refs);
                }
                assert_type_refs(
                    signature.return_type_refs(),
                    &owned_signature.return_type_refs,
                );
            }
            assert_type_refs(view.return_type_refs(), &owned.return_type_refs);
            assert_eq!(
                view.availability_contexts().collect::<Vec<_>>(),
                owned.availability_contexts
            );
        }

        for (index, owned) in snapshot.globals.iter().enumerate() {
            let view = read.global(HbkGlobalFactId(index as u32));
            assert_eq!(view.id(), owned.id);
            assert_eq!(view.kind(), owned.kind);
            assert_eq!(view.domain(), owned.domain);
            assert_name(view.name(), &owned.name);
            assert_eq!(view.callable(), owned.callable);
            assert_type_refs(view.type_refs(), &owned.type_refs);
        }

        let table = read.query_table(HbkQueryTableId(0));
        let owned_table = snapshot.query_table(HbkQueryTableId(0));
        assert_eq!(table.id(), owned_table.id);
        assert_name(table.name(), &owned_table.name);
        assert_name(
            table.syntax().unwrap(),
            owned_table.syntax.as_ref().unwrap(),
        );
        assert_eq!(table.identifier(), owned_table.identifier);
        assert_eq!(table.role(), owned_table.role);
        assert_eq!(
            table
                .owner_path()
                .map(|name| (name.primary(), name.alias()))
                .collect::<Vec<_>>(),
            owned_table
                .owner_path
                .iter()
                .map(|name| (name.primary, name.alias))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            table.template_parameters().collect::<Vec<_>>(),
            owned_table.template_parameters
        );

        let field = read.query_field(HbkQueryFieldId(0));
        let owned_field = snapshot.query_field(HbkQueryFieldId(0));
        assert_eq!(field.id(), owned_field.id);
        assert_eq!(field.owner(), owned_field.owner);
        assert_name(field.name(), &owned_field.name);
        assert_type_refs(field.type_refs(), &owned_field.type_refs);
        assert_eq!(field.note(), owned_field.note);

        let parameter = read.query_parameter(HbkQueryParameterId(0));
        let owned_parameter = snapshot.query_parameter(HbkQueryParameterId(0));
        assert_eq!(parameter.id(), owned_parameter.id);
        assert_eq!(parameter.owner(), owned_parameter.owner);
        assert_name(parameter.name(), &owned_parameter.name);
        assert_type_refs(parameter.type_refs(), &owned_parameter.type_refs);
        assert_eq!(parameter.default_value(), owned_parameter.default_value);

        let language = read.language_fact(HbkLanguageFactId(0));
        let owned_language = snapshot.language_fact(HbkLanguageFactId(0));
        assert_eq!(language.id(), owned_language.id);
        assert_eq!(language.kind(), owned_language.kind);
        assert_eq!(language.domain(), owned_language.domain);
        assert_name(language.name(), &owned_language.name);
        assert_eq!(language.signatures().len(), owned_language.signatures.len());
        for (signature, owned_signature) in language.signatures().zip(&owned_language.signatures) {
            assert_eq!(signature.text(), owned_signature.text);
            assert_eq!(
                signature.parameters().len(),
                owned_signature.parameters.len()
            );
            for (parameter, owned_parameter) in
                signature.parameters().zip(&owned_signature.parameters)
            {
                assert_eq!(parameter.name(), owned_parameter.name);
                assert_eq!(parameter.required(), owned_parameter.required);
                assert_type_refs(parameter.type_refs(), &owned_parameter.type_refs);
            }
            assert_type_refs(
                signature.return_type_refs(),
                &owned_signature.return_type_refs,
            );
        }
        assert_type_refs(language.type_refs(), &owned_language.type_refs);
        assert_type_refs(
            language.return_type_refs(),
            &owned_language.return_type_refs,
        );

        let enum_fact = read.enum_fact(HbkEnumId(0));
        assert_eq!(enum_fact.id(), snapshot.enum_fact(HbkEnumId(0)).id);
        assert_name(enum_fact.name(), &snapshot.enum_fact(HbkEnumId(0)).name);
        let enum_value = read.enum_value(HbkEnumValueId(0));
        assert_eq!(enum_value.id(), snapshot.enum_value(HbkEnumValueId(0)).id);
        assert_eq!(
            enum_value.owner(),
            snapshot.enum_value(HbkEnumValueId(0)).owner
        );
        assert_name(
            enum_value.name(),
            &snapshot.enum_value(HbkEnumValueId(0)).name,
        );

        for owned in &snapshot.source_by_fact {
            let source = read.source(owned.fact).unwrap();
            assert_eq!(source.hbk_path(), owned.source.hbk_path);
            assert_eq!(source.locale(), owned.source.locale);
            assert_eq!(source.toc_path(), owned.source.toc_path);
            assert_eq!(source.html_path(), owned.source.html_path);
            assert_eq!(source.page_title(), owned.source.page_title);
        }

        drop(mapped);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_mapped_filters_globals_and_known_owner_members_with_any_all_semantics() {
        let root = temp_path("x1-forward-filter");
        fs::create_dir_all(&root).unwrap();
        let artifact = root.join("generation.x1");
        let identity = test_identity();
        let snapshot = forward_payload_fixture_snapshot();
        let bytes = encode_snapshot_with_identity(&snapshot, &identity).unwrap();
        write_readonly_artifact(&artifact, &bytes);
        let mapped =
            open_controlled_generation(&artifact, &runtime_expectation(&identity)).unwrap();
        let read = mapped.read_handle();

        let any_server =
            X1AvailabilityFilter::from_codes(["server"], X1AvailabilityMode::Any).unwrap();
        let all_server_thin =
            X1AvailabilityFilter::from_codes(["server", "thin_client"], X1AvailabilityMode::All)
                .unwrap();
        let any_empty =
            X1AvailabilityFilter::from_codes(std::iter::empty(), X1AvailabilityMode::Any).unwrap();
        let all_empty =
            X1AvailabilityFilter::from_codes(std::iter::empty(), X1AvailabilityMode::All).unwrap();

        assert_eq!(
            read.filtered_members(HbkPlatformTypeId(0), any_server, None)
                .collect::<Vec<_>>(),
            vec![HbkTypeMemberId(0), HbkTypeMemberId(1)]
        );
        assert_eq!(
            read.filtered_members(HbkPlatformTypeId(0), all_server_thin, None)
                .collect::<Vec<_>>(),
            vec![HbkTypeMemberId(0)]
        );
        assert_eq!(
            read.filtered_members(
                HbkPlatformTypeId(0),
                all_empty,
                Some(HbkTypeMemberKind::Method),
            )
            .collect::<Vec<_>>(),
            vec![HbkTypeMemberId(1), HbkTypeMemberId(2)]
        );
        assert_eq!(
            read.filtered_members(HbkPlatformTypeId(1), any_server, None)
                .collect::<Vec<_>>(),
            vec![HbkTypeMemberId(3)]
        );
        assert_eq!(
            read.filtered_globals(any_empty, None).collect::<Vec<_>>(),
            vec![HbkGlobalFactId(0)]
        );
        assert_eq!(
            read.filtered_globals(any_server, None).collect::<Vec<_>>(),
            vec![HbkGlobalFactId(0), HbkGlobalFactId(1)]
        );
        assert_eq!(
            read.filtered_globals(all_server_thin, None)
                .collect::<Vec<_>>(),
            vec![HbkGlobalFactId(0), HbkGlobalFactId(1)]
        );
        assert_eq!(
            read.filtered_globals(all_empty, Some(HbkGlobalFactKind::Method))
                .collect::<Vec<_>>(),
            vec![HbkGlobalFactId(0), HbkGlobalFactId(1)]
        );
        assert!(
            X1AvailabilityFilter::from_codes(["module_context_kind"], X1AvailabilityMode::Any,)
                .is_err()
        );

        drop(mapped);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_mapped_lookup_surface_matches_owned_handle() {
        let root = temp_path("x1-lookup-surface");
        fs::create_dir_all(&root).unwrap();
        let artifact = root.join("generation.x1");
        let identity = test_identity();
        let snapshot = lookup_fixture_snapshot();
        let bytes = encode_snapshot_with_identity(&snapshot, &identity).unwrap();
        write_readonly_artifact(&artifact, &bytes);
        let mapped =
            open_controlled_generation(&artifact, &runtime_expectation(&identity)).unwrap();
        let actual = mapped.read_handle();
        let expected = snapshot.worker_handle();

        assert_eq!(
            actual.string_id("Shared Type"),
            snapshot
                .strings
                .iter()
                .position(|candidate| candidate == "Shared Type")
                .map(|index| StringId(index as u32))
        );
        assert_eq!(
            actual.string_id("Запрос"),
            snapshot
                .strings
                .iter()
                .position(|candidate| candidate == "Запрос")
                .map(|index| StringId(index as u32))
        );
        assert_eq!(actual.string_id("missing"), None);
        assert_eq!(
            actual.global_fact_ids().collect::<Vec<_>>(),
            expected.global_fact_ids().collect::<Vec<_>>()
        );
        assert_eq!(
            actual.query_table_ids().collect::<Vec<_>>(),
            expected.query_table_ids().collect::<Vec<_>>()
        );
        assert_eq!(
            actual.query_field_ids().collect::<Vec<_>>(),
            expected.query_field_ids().collect::<Vec<_>>()
        );
        assert_eq!(
            actual.query_parameter_ids().collect::<Vec<_>>(),
            expected.query_parameter_ids().collect::<Vec<_>>()
        );

        assert_lookup_eq(
            actual.facts_by_id("duplicate-fact-id"),
            expected.facts_by_id("duplicate-fact-id"),
        );
        assert_lookup_eq(
            actual.facts_by_id("missing"),
            expected.facts_by_id("missing"),
        );
        assert_eq!(
            actual.platform_type_by_id("platform_type:Запрос"),
            expected.platform_type_by_id("platform_type:Запрос")
        );
        assert_eq!(actual.platform_type_by_id("missing"), None);
        assert_lookup_eq(
            actual.platform_types_by_name("Shared Type"),
            expected.platform_types_by_name("Shared Type"),
        );
        assert_lookup_eq(
            actual.platform_types_by_name("missing"),
            expected.platform_types_by_name("missing"),
        );
        let template = snapshot.platform_types[0].type_template_key.unwrap();
        assert_lookup_eq(
            actual.platform_types_by_template_key(
                snapshot.string(template.family),
                snapshot.string(template.variant),
            ),
            expected.platform_types_by_template_key(
                snapshot.string(template.family),
                snapshot.string(template.variant),
            ),
        );
        assert_eq!(
            actual
                .platform_types_by_template_key("missing", "missing")
                .len(),
            0
        );

        assert_lookup_eq(
            actual.members_of_type(HbkPlatformTypeId(0)),
            expected
                .members_of_type(HbkPlatformTypeId(0))
                .iter()
                .copied(),
        );
        assert_lookup_eq(
            actual.members_of_type(HbkPlatformTypeId(9)),
            std::iter::empty(),
        );
        assert_lookup_eq(
            actual.member_by_owner_name(HbkPlatformTypeId(0), "Shared Member"),
            expected.member_by_owner_name(HbkPlatformTypeId(0), "Shared Member"),
        );
        assert_eq!(
            actual
                .member_by_owner_name(HbkPlatformTypeId(0), "missing")
                .len(),
            0
        );
        assert_lookup_eq(
            actual.member_by_owner_name_kind(HbkPlatformTypeId(0), "Shared Member", None),
            expected.member_by_owner_name_kind(HbkPlatformTypeId(0), "Shared Member", None),
        );
        assert_lookup_eq(
            actual.member_by_owner_name_kind(
                HbkPlatformTypeId(0),
                "Shared Member",
                Some(HbkTypeMemberKind::Method),
            ),
            expected.member_by_owner_name_kind(
                HbkPlatformTypeId(0),
                "Shared Member",
                Some(HbkTypeMemberKind::Method),
            ),
        );
        assert_eq!(
            actual
                .member_by_owner_name_kind(
                    HbkPlatformTypeId(0),
                    "Shared Member",
                    Some(HbkTypeMemberKind::Property),
                )
                .len(),
            0
        );
        assert_lookup_eq(
            actual.callables_of_type(HbkPlatformTypeId(0)),
            expected
                .callables_of_type(HbkPlatformTypeId(0))
                .iter()
                .copied(),
        );
        assert_lookup_eq(
            actual.callable_by_owner_name(HbkPlatformTypeId(0), "Server Method Callable"),
            expected.callable_by_owner_name(HbkPlatformTypeId(0), "Server Method Callable"),
        );
        assert_eq!(
            actual
                .callable_by_owner_name(HbkPlatformTypeId(0), "missing")
                .len(),
            0
        );
        assert_lookup_eq(
            actual.constructors_of_type(HbkPlatformTypeId(0)),
            expected
                .constructors_of_type(HbkPlatformTypeId(0))
                .iter()
                .copied(),
        );

        assert_lookup_eq(
            actual.globals_by_name("Shared Global"),
            expected.globals_by_name("Shared Global"),
        );
        assert_eq!(actual.globals_by_name("missing").len(), 0);
        assert_lookup_eq(
            actual.globals_by_domain_name_kind(HbkLanguageDomain::Bsl, "Shared Global", None),
            expected.globals_by_domain_name_kind(HbkLanguageDomain::Bsl, "Shared Global", None),
        );
        assert_lookup_eq(
            actual.globals_by_domain_name_kind(
                HbkLanguageDomain::Bsl,
                "Shared Global",
                Some(HbkGlobalFactKind::Method),
            ),
            expected.globals_by_domain_name_kind(
                HbkLanguageDomain::Bsl,
                "Shared Global",
                Some(HbkGlobalFactKind::Method),
            ),
        );
        assert_lookup_eq(
            actual.module_events("Common Module"),
            expected.module_events("Common Module"),
        );
        assert_lookup_eq(
            actual.module_event_by_context_name("Common Module", "Event Name"),
            expected.module_event_by_context_name("Common Module", "Event Name"),
        );
        assert_lookup_eq(
            actual.module_context_events(HbkLanguageDomain::Bsl, "BSL", "Common Module"),
            expected.module_context_events(HbkLanguageDomain::Bsl, "BSL", "Common Module"),
        );
        assert_eq!(actual.module_events("missing").len(), 0);
        assert_eq!(
            actual
                .module_context_events(HbkLanguageDomain::Bsl, "missing", "missing")
                .len(),
            0
        );

        assert_eq!(
            actual.query_table_by_id("query_table:Sales"),
            expected.query_table_by_id("query_table:Sales")
        );
        assert_eq!(actual.query_table_by_id("missing"), None);
        assert_lookup_eq(
            actual.query_tables_by_name("Sales"),
            expected.query_tables_by_name("Sales"),
        );
        assert_lookup_eq(
            actual.query_tables_by_syntax("Sales Syntax"),
            expected.query_tables_by_syntax("Sales Syntax"),
        );
        assert_lookup_eq(
            actual.query_tables_by_identifier("SALES ID"),
            expected.query_tables_by_identifier("SALES ID"),
        );
        assert_eq!(actual.query_tables_by_name("missing").len(), 0);
        assert_lookup_eq(
            actual.query_fields(HbkQueryTableId(0)),
            expected.query_fields(HbkQueryTableId(0)).iter().copied(),
        );
        assert_lookup_eq(
            actual.query_fields_by_name(HbkQueryTableId(0), "Shared Field"),
            expected.query_fields_by_name(HbkQueryTableId(0), "Shared Field"),
        );
        assert_eq!(
            actual
                .query_fields_by_name(HbkQueryTableId(0), "missing")
                .len(),
            0
        );
        assert_lookup_eq(
            actual.query_parameters(HbkQueryTableId(0)),
            expected
                .query_parameters(HbkQueryTableId(0))
                .iter()
                .copied(),
        );
        assert_lookup_eq(
            actual.query_parameters_by_name(HbkQueryTableId(0), "Shared Parameter"),
            expected.query_parameters_by_name(HbkQueryTableId(0), "Shared Parameter"),
        );
        assert_eq!(
            actual
                .query_parameters_by_name(HbkQueryTableId(0), "missing")
                .len(),
            0
        );

        assert_eq!(
            actual.language_fact_by_id("language:Function"),
            expected.language_fact_by_id("language:Function")
        );
        assert_eq!(actual.language_fact_by_id("missing"), None);
        assert_lookup_eq(
            actual.language_facts_by_name("Language Function"),
            expected.language_facts_by_name("Language Function"),
        );
        assert_eq!(actual.language_facts_by_name("missing").len(), 0);
        assert_eq!(
            actual.enum_by_id("enum:Color"),
            expected.enum_by_id("enum:Color")
        );
        assert_eq!(actual.enum_by_id("missing"), None);
        assert_lookup_eq(
            actual.enums_by_name("Color"),
            expected.enums_by_name("Color"),
        );
        assert_eq!(actual.enums_by_name("missing").len(), 0);
        assert_eq!(
            actual.enum_value_by_id("enum_value:Red"),
            expected.enum_value_by_id("enum_value:Red")
        );
        assert_eq!(actual.enum_value_by_id("missing"), None);
        assert_lookup_eq(
            actual.enum_values(HbkEnumId(0)),
            expected.enum_values(HbkEnumId(0)).iter().copied(),
        );
        assert_lookup_eq(
            actual.enum_values_by_name(HbkEnumId(0), "Shared Value"),
            expected.enum_values_by_name(HbkEnumId(0), "Shared Value"),
        );
        assert_eq!(actual.enum_values_by_name(HbkEnumId(0), "missing").len(), 0);

        let available_fact = HbkFactRef::Global(HbkGlobalFactId(1));
        assert_lookup_eq(
            actual.availability_contexts(available_fact),
            expected
                .availability_contexts(available_fact)
                .iter()
                .copied(),
        );
        assert_eq!(
            actual.available_since(available_fact),
            expected.available_since(available_fact)
        );
        assert_lookup_eq(
            actual.relations_by_source_kind(available_fact, "Type Reference"),
            expected
                .relations_by_source_kind(available_fact, "Type Reference")
                .iter()
                .copied(),
        );
        assert_lookup_eq(
            actual.relations_by_source_kind(available_fact, "missing"),
            std::iter::empty(),
        );

        drop(mapped);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn x1_mapped_handles_and_views_carry_the_generation_lifetime() {
        fn handle<'a>(generation: &'a X1MappedGeneration) -> X1MappedReadHandle<'a> {
            generation.read_handle()
        }
        fn platform_type<'a>(handle: X1MappedReadHandle<'a>) -> X1PlatformTypeView<'a> {
            handle.platform_type(HbkPlatformTypeId(0))
        }

        let _: for<'a> fn(&'a X1MappedGeneration) -> X1MappedReadHandle<'a> = handle;
        let _: for<'a> fn(X1MappedReadHandle<'a>) -> X1PlatformTypeView<'a> = platform_type;
    }

    #[cfg(feature = "snapshot-experiment-alloc")]
    #[test]
    #[ignore = "run alone to isolate process-global allocation counters"]
    fn x1_mapped_steady_filtered_and_nested_traversal_allocates_nothing() {
        let root = temp_path("x1-forward-allocation");
        fs::create_dir_all(&root).unwrap();
        let artifact = root.join("generation.x1");
        let identity = test_identity();
        let snapshot = forward_payload_fixture_snapshot();
        let bytes = encode_snapshot_with_identity(&snapshot, &identity).unwrap();
        write_readonly_artifact(&artifact, &bytes);
        let mapped =
            open_controlled_generation(&artifact, &runtime_expectation(&identity)).unwrap();
        let read = mapped.read_handle();
        let filter = X1AvailabilityFilter::from_codes(["server"], X1AvailabilityMode::Any).unwrap();

        for _ in 0..8 {
            traverse_steady_payload(read, filter);
        }
        let before = experiment_allocation_snapshot();
        for _ in 0..128 {
            traverse_steady_payload(read, filter);
        }
        let delta = experiment_allocation_snapshot().delta_since(before);

        assert_eq!(delta.allocation_calls, 0);
        assert_eq!(delta.reallocation_calls, 0);
        assert_eq!(delta.allocated_bytes, 0);

        drop(mapped);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(feature = "snapshot-experiment-alloc")]
    #[test]
    #[ignore = "run alone to isolate process-global allocation counters"]
    fn x1_mapped_pre_normalized_lookup_and_ranges_allocate_nothing() {
        let root = temp_path("x1-lookup-allocation");
        fs::create_dir_all(&root).unwrap();
        let artifact = root.join("generation.x1");
        let identity = test_identity();
        let snapshot = lookup_fixture_snapshot();
        let bytes = encode_snapshot_with_identity(&snapshot, &identity).unwrap();
        write_readonly_artifact(&artifact, &bytes);
        let mapped =
            open_controlled_generation(&artifact, &runtime_expectation(&identity)).unwrap();
        let read = mapped.read_handle();

        for _ in 0..8 {
            traverse_steady_lookup(read);
        }
        let before = experiment_allocation_snapshot();
        for _ in 0..128 {
            traverse_steady_lookup(read);
        }
        let delta = experiment_allocation_snapshot().delta_since(before);
        assert_eq!(delta.allocation_calls, 0);
        assert_eq!(delta.reallocation_calls, 0);
        assert_eq!(delta.allocated_bytes, 0);

        drop(mapped);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(feature = "snapshot-experiment-alloc")]
    #[test]
    #[ignore = "run alone to isolate process-global allocation counters"]
    fn x1_mapped_raw_lookup_allocates_only_its_normalized_key() {
        let root = temp_path("x1-raw-lookup-allocation");
        fs::create_dir_all(&root).unwrap();
        let artifact = root.join("generation.x1");
        let identity = test_identity();
        let snapshot = lookup_fixture_snapshot();
        let bytes = encode_snapshot_with_identity(&snapshot, &identity).unwrap();
        write_readonly_artifact(&artifact, &bytes);
        let mapped =
            open_controlled_generation(&artifact, &runtime_expectation(&identity)).unwrap();
        let read = mapped.read_handle();

        std::hint::black_box(read.platform_types_by_name("Shared Type").count());
        let before = experiment_allocation_snapshot();
        std::hint::black_box(read.platform_types_by_name("Shared Type").count());
        let delta = experiment_allocation_snapshot().delta_since(before);
        assert_eq!(delta.allocation_calls, 1);
        assert_eq!(delta.reallocation_calls, 0);

        drop(mapped);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(feature = "snapshot-experiment-alloc")]
    fn traverse_steady_lookup(read: X1MappedReadHandle<'_>) {
        std::hint::black_box(read.string_id("Shared Type"));
        for id in read.platform_types_by_normalized_name("sharedtype") {
            std::hint::black_box(id);
        }
        for id in read.member_by_owner_normalized_name(HbkPlatformTypeId(0), "sharedmember") {
            std::hint::black_box(id);
        }
        for id in read.members_of_type(HbkPlatformTypeId(0)) {
            std::hint::black_box(id);
        }
        for id in read.callables_of_type(HbkPlatformTypeId(0)) {
            std::hint::black_box(id);
        }
        for id in read.query_fields(HbkQueryTableId(0)) {
            std::hint::black_box(id);
        }
        for id in read.query_parameters(HbkQueryTableId(0)) {
            std::hint::black_box(id);
        }
        for id in read.enum_values(HbkEnumId(0)) {
            std::hint::black_box(id);
        }
        for id in read.availability_contexts(HbkFactRef::Global(HbkGlobalFactId(1))) {
            std::hint::black_box(id);
        }
    }

    #[cfg(feature = "snapshot-experiment-alloc")]
    fn traverse_steady_payload(read: X1MappedReadHandle<'_>, filter: X1AvailabilityFilter) {
        std::hint::black_box(read.source_locale());
        for id in [HbkPlatformTypeId(0), HbkPlatformTypeId(1)] {
            let platform_type = read.platform_type(id);
            std::hint::black_box(platform_type.id());
            black_box_name(read, platform_type.name());
            std::hint::black_box(platform_type.type_template_key());
            for availability in platform_type.availability_contexts() {
                std::hint::black_box(read.string(availability));
            }
            if let Some(metadata) = platform_type.metadata_template() {
                std::hint::black_box(read.string(metadata.metadata_kind()));
                for parameter in metadata.template_parameters() {
                    std::hint::black_box(read.string(parameter));
                }
            }
        }
        for id in read.filtered_globals(filter, None) {
            let global = read.global(id);
            std::hint::black_box(global.id());
            std::hint::black_box(global.kind());
            std::hint::black_box(global.domain());
            black_box_name(read, global.name());
            std::hint::black_box(global.callable());
            black_box_type_refs(read, global.type_refs());
        }
        for id in read.filtered_members(HbkPlatformTypeId(0), filter, None) {
            let member = read.type_member(id);
            std::hint::black_box(member.id());
            std::hint::black_box(member.owner());
            std::hint::black_box(member.kind());
            black_box_name(read, member.name());
            black_box_type_refs(read, member.type_refs());
            for availability in member.availability_contexts() {
                std::hint::black_box(read.string(availability));
            }
        }
        for id in [HbkCallableId(0), HbkCallableId(1)] {
            let callable = read.callable(id);
            std::hint::black_box(callable.id());
            std::hint::black_box(callable.owner());
            std::hint::black_box(callable.kind());
            black_box_name(read, callable.name());
            for signature in callable.signatures() {
                black_box_signature(read, signature);
            }
            black_box_type_refs(read, callable.return_type_refs());
            for availability in callable.availability_contexts() {
                std::hint::black_box(read.string(availability));
            }
        }

        let table = read.query_table(HbkQueryTableId(0));
        std::hint::black_box(table.id());
        black_box_name(read, table.name());
        if let Some(syntax) = table.syntax() {
            black_box_name(read, syntax);
        }
        std::hint::black_box(table.identifier());
        std::hint::black_box(table.role());
        for owner in table.owner_path() {
            black_box_name(read, owner);
        }
        for parameter in table.template_parameters() {
            std::hint::black_box(read.string(parameter));
        }

        let field = read.query_field(HbkQueryFieldId(0));
        std::hint::black_box(field.id());
        std::hint::black_box(field.owner());
        black_box_name(read, field.name());
        black_box_type_refs(read, field.type_refs());
        std::hint::black_box(field.note());

        let parameter = read.query_parameter(HbkQueryParameterId(0));
        std::hint::black_box(parameter.id());
        std::hint::black_box(parameter.owner());
        black_box_name(read, parameter.name());
        black_box_type_refs(read, parameter.type_refs());
        std::hint::black_box(parameter.default_value());

        let language = read.language_fact(HbkLanguageFactId(0));
        std::hint::black_box(language.id());
        std::hint::black_box(language.kind());
        std::hint::black_box(language.domain());
        black_box_name(read, language.name());
        for signature in language.signatures() {
            black_box_signature(read, signature);
        }
        black_box_type_refs(read, language.type_refs());
        black_box_type_refs(read, language.return_type_refs());

        let enum_fact = read.enum_fact(HbkEnumId(0));
        std::hint::black_box(enum_fact.id());
        black_box_name(read, enum_fact.name());
        let enum_value = read.enum_value(HbkEnumValueId(0));
        std::hint::black_box(enum_value.id());
        std::hint::black_box(enum_value.owner());
        black_box_name(read, enum_value.name());

        for fact in [
            HbkFactRef::PlatformType(HbkPlatformTypeId(0)),
            HbkFactRef::TypeMember(HbkTypeMemberId(0)),
            HbkFactRef::Callable(HbkCallableId(0)),
            HbkFactRef::Global(HbkGlobalFactId(0)),
            HbkFactRef::QueryTable(HbkQueryTableId(0)),
            HbkFactRef::QueryField(HbkQueryFieldId(0)),
            HbkFactRef::QueryParameter(HbkQueryParameterId(0)),
            HbkFactRef::LanguageFact(HbkLanguageFactId(0)),
            HbkFactRef::Enum(HbkEnumId(0)),
            HbkFactRef::EnumValue(HbkEnumValueId(0)),
        ] {
            let source = read.source(fact).unwrap();
            std::hint::black_box(source.hbk_path());
            std::hint::black_box(source.locale());
            std::hint::black_box(source.toc_path());
            std::hint::black_box(source.html_path());
            std::hint::black_box(source.page_title());
        }
    }

    #[cfg(feature = "snapshot-experiment-alloc")]
    fn black_box_name(read: X1MappedReadHandle<'_>, name: X1NameView) {
        std::hint::black_box(read.string(name.primary()));
        if let Some(alias) = name.alias() {
            std::hint::black_box(read.string(alias));
        }
    }

    #[cfg(feature = "snapshot-experiment-alloc")]
    fn black_box_signature(read: X1MappedReadHandle<'_>, signature: X1SignatureView<'_>) {
        std::hint::black_box(read.string(signature.text()));
        for parameter in signature.parameters() {
            std::hint::black_box(read.string(parameter.name()));
            std::hint::black_box(parameter.required());
            black_box_type_refs(read, parameter.type_refs());
        }
        black_box_type_refs(read, signature.return_type_refs());
    }

    #[cfg(feature = "snapshot-experiment-alloc")]
    fn black_box_type_refs<'a>(
        read: X1MappedReadHandle<'a>,
        type_refs: impl ExactSizeIterator<Item = X1TypeRefView<'a>>,
    ) {
        for type_ref in type_refs {
            std::hint::black_box(read.string(type_ref.name()));
            std::hint::black_box(type_ref.target_kind());
            std::hint::black_box(type_ref.target_ok());
            for target in type_ref.ambiguous_targets() {
                std::hint::black_box(read.string(target));
            }
            std::hint::black_box(type_ref.type_template_key());
            if let Some(binding) = type_ref.template_binding() {
                std::hint::black_box(binding.template_key());
                for argument in binding.arguments() {
                    std::hint::black_box(argument);
                }
            }
        }
    }

    fn test_identity() -> X1ArtifactIdentity {
        X1ArtifactIdentity {
            source_path: "/tmp/8.3.0.0/fixture.hbk".to_string(),
            source_bytes: 7,
            source_sha256: "0".repeat(64),
            locale: "ru".to_string(),
            source_locale: "ru".to_string(),
            platform_version: "8.3.0.0".to_string(),
            provider_identity: "fixture-provider".to_string(),
            provider_bytes: 11,
            provider_sha256: "1".repeat(64),
            provider_schema: SUPPORTED_PROVIDER_SCHEMA,
            extraction_schema: SUPPORTED_EXTRACTION_SCHEMA,
        }
    }

    fn runtime_expectation(identity: &X1ArtifactIdentity) -> X1RuntimeExpectation {
        X1RuntimeExpectation {
            platform_version: identity.platform_version.clone(),
            locale: identity.locale.clone(),
            source_locale: identity.source_locale.clone(),
            source_sha256: identity.source_sha256.clone(),
        }
    }

    fn open_controlled_generation(
        path: &Path,
        expected: &X1RuntimeExpectation,
    ) -> Result<X1MappedGeneration, SearchError> {
        // SAFETY: Every test owns its unique temporary generation path, makes
        // the file read-only before open, and never mutates it while the
        // returned mapping exists.
        unsafe { X1MappedGeneration::open(path, expected) }
    }

    fn assert_compatibility_mismatch(
        path: &Path,
        expected: &X1RuntimeExpectation,
        expected_field: &'static str,
    ) {
        let Err(error) = open_controlled_generation(path, expected) else {
            panic!("mismatched runtime expectation must be rejected");
        };
        assert!(matches!(
            error,
            SearchError::SnapshotArtifact {
                source: HbkFactSnapshotArtifactError::CompatibilityMismatch { field, .. },
                ..
            } if field == expected_field
        ));
    }

    fn write_readonly_artifact(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(path, permissions).unwrap();
    }

    fn rewrite_payload_checksum(bytes: &mut [u8]) {
        let checksum = fnv1a(&bytes[HEADER_LEN + SECTION_COUNT * DIRECTORY_ENTRY_LEN..]);
        bytes[32..40].copy_from_slice(&checksum.to_le_bytes());
    }

    fn fixture_index_builder(source: &Path) -> SearchIndexBuilder {
        let mut builder = SearchIndexBuilder::new();
        builder.add_language_fact(language::LanguageFact {
            id: "fixture-language-type".to_string(),
            source_family: language::LanguageSourceFamily::Shlang,
            domain: language::LanguageDomain::BslLanguage,
            family: language::LanguageFactFamily::Type,
            name: model::LocalizedName {
                primary: "Строка".to_string(),
                alias: Some("String".to_string()),
            },
            syntax: None,
            signatures: Vec::new(),
            type_refs: Vec::new(),
            return_types: Vec::new(),
            description: None,
            provenance: language::LanguageFactProvenance {
                source_hbk: source.to_string_lossy().into_owned(),
                locale: "ru".to_string(),
                html_path: "fixture.html".to_string(),
                page_title: "Строка".to_string(),
                anchor: None,
            },
        });
        builder
    }

    fn fixture_snapshot() -> HbkFactSnapshot {
        let empty_fact_csr = CsrIndex::<HbkFactRef, StringId>::from_pairs(Vec::new());
        let empty_relation_csr = CsrIndex::<RelationLookupKey, HbkFactRef>::from_pairs(Vec::new());
        HbkFactSnapshot {
            strings: vec![
                "ru".to_string(),
                "Запрос".to_string(),
                "platform_type:Запрос".to_string(),
                "запрос".to_string(),
                "/tmp/8.3.0.0/fixture.hbk".to_string(),
            ],
            source_locale: Some(StringId(0)),
            platform_types: vec![HbkPlatformType {
                id: StringId(2),
                name: HbkName {
                    primary: StringId(1),
                    alias: None,
                },
                metadata_template: None,
                type_template_key: None,
                availability_contexts: Vec::new(),
            }],
            type_members: Vec::new(),
            callables: Vec::new(),
            globals: Vec::new(),
            query_tables: Vec::new(),
            query_fields: Vec::new(),
            query_parameters: Vec::new(),
            language_facts: Vec::new(),
            enums: Vec::new(),
            enum_values: Vec::new(),
            fact_ids: vec![
                IdLookup {
                    key: StringId(2),
                    value: HbkFactRef::PlatformType(HbkPlatformTypeId(0)),
                },
                IdLookup {
                    key: StringId(3),
                    value: HbkFactRef::PlatformType(HbkPlatformTypeId(0)),
                },
            ],
            platform_type_ids: vec![IdLookup {
                key: StringId(2),
                value: HbkPlatformTypeId(0),
            }],
            platform_type_names: vec![NameLookup {
                key: StringId(3),
                value: HbkPlatformTypeId(0),
            }],
            platform_type_templates: Vec::new(),
            member_ids: Vec::new(),
            members_by_owner: CsrIndex::from_pairs(Vec::new()),
            members_by_owner_name: Vec::new(),
            members_by_owner_name_kind: Vec::new(),
            callable_ids: Vec::new(),
            callables_by_owner: CsrIndex::from_pairs(Vec::new()),
            callables_by_owner_name: Vec::new(),
            constructors_by_type: CsrIndex::from_pairs(Vec::new()),
            global_names: Vec::new(),
            globals_by_domain_name_kind: Vec::new(),
            module_event_names: Vec::new(),
            module_contexts_by_domain_language_kind: Vec::new(),
            query_table_ids: Vec::new(),
            query_table_names: Vec::new(),
            query_table_syntax_names: Vec::new(),
            query_table_identifiers: Vec::new(),
            query_fields_by_table: CsrIndex::from_pairs(Vec::new()),
            query_fields_by_table_name: Vec::new(),
            query_parameters_by_table: CsrIndex::from_pairs(Vec::new()),
            query_parameters_by_table_name: Vec::new(),
            language_ids: Vec::new(),
            language_names: Vec::new(),
            enum_ids: Vec::new(),
            enum_names: Vec::new(),
            enum_value_ids: Vec::new(),
            enum_values_by_enum: CsrIndex::from_pairs(Vec::new()),
            enum_values_by_enum_name: Vec::new(),
            availability_by_fact: empty_fact_csr,
            availability_since_by_fact: Vec::new(),
            source_by_fact: vec![FactSourceLookup {
                fact: HbkFactRef::PlatformType(HbkPlatformTypeId(0)),
                source: test_source(),
            }],
            relations_by_source_kind: empty_relation_csr,
        }
    }

    fn test_source() -> HbkFactSource {
        HbkFactSource {
            hbk_path: StringId(4),
            locale: StringId(0),
            toc_path: Some(StringId(3)),
            html_path: StringId(2),
            page_title: StringId(1),
        }
    }

    fn template_binding_fixture_snapshot() -> HbkFactSnapshot {
        let mut snapshot = fixture_snapshot();
        let metadata_kind = StringId(snapshot.strings.len() as u32);
        snapshot.strings.push("metadata-kind".to_string());
        let parameter = StringId(snapshot.strings.len() as u32);
        snapshot.strings.push("owner-parameter".to_string());
        let family = StringId(snapshot.strings.len() as u32);
        snapshot.strings.push("template-family".to_string());
        let variant = StringId(snapshot.strings.len() as u32);
        snapshot.strings.push("template-variant".to_string());
        let member_id = StringId(snapshot.strings.len() as u32);
        snapshot.strings.push("member:Value".to_string());
        let member_name = StringId(snapshot.strings.len() as u32);
        snapshot.strings.push("Value".to_string());
        let target_name = StringId(snapshot.strings.len() as u32);
        snapshot.strings.push("Target".to_string());
        let key = HbkPlatformTypeTemplateKey { family, variant };

        snapshot.platform_types[0].metadata_template = Some(HbkMetadataTemplate {
            metadata_kind,
            template_parameters: vec![parameter],
        });
        snapshot.platform_types[0].type_template_key = Some(key);
        snapshot.platform_type_templates = vec![TypeTemplateLookup {
            family,
            variant,
            value: HbkPlatformTypeId(0),
        }];
        snapshot.type_members = vec![HbkTypeMember {
            id: member_id,
            owner: HbkPlatformTypeId(0),
            kind: HbkTypeMemberKind::Property,
            name: HbkName {
                primary: member_name,
                alias: None,
            },
            type_refs: vec![HbkTypeRef {
                name: target_name,
                target: HbkTypeRefTarget::Unresolved,
                type_template_key: Some(key),
                template_binding: Some(HbkTypeTemplateBinding {
                    template_key: key,
                    arguments: vec![model::TemplateParameterBinding::OwnerParameter {
                        owner_parameter_index: 0,
                        target_parameter_index: 0,
                    }],
                }),
            }],
            availability_contexts: Vec::new(),
        }];
        snapshot.members_by_owner =
            CsrIndex::from_pairs(vec![(HbkPlatformTypeId(0), HbkTypeMemberId(0))]);
        snapshot
    }

    fn forward_payload_fixture_snapshot() -> HbkFactSnapshot {
        let mut snapshot = template_binding_fixture_snapshot();
        let server = push_fixture_string(&mut snapshot, "server");
        let thin = push_fixture_string(&mut snapshot, "thin_client");
        let web = push_fixture_string(&mut snapshot, "web_client");
        let second_type_id = push_fixture_string(&mut snapshot, "platform_type:Second");
        let second_type_name = push_fixture_string(&mut snapshot, "Second");
        let second_type_key = push_fixture_string(&mut snapshot, "second");
        snapshot.platform_types.push(HbkPlatformType {
            id: second_type_id,
            name: HbkName {
                primary: second_type_name,
                alias: None,
            },
            metadata_template: None,
            type_template_key: None,
            availability_contexts: vec![server],
        });
        snapshot.platform_type_ids.push(IdLookup {
            key: second_type_id,
            value: HbkPlatformTypeId(1),
        });
        snapshot
            .platform_type_ids
            .sort_by_key(|lookup| snapshot.strings[lookup.key.0 as usize].clone());
        snapshot.platform_type_names.push(NameLookup {
            key: second_type_key,
            value: HbkPlatformTypeId(1),
        });
        snapshot
            .platform_type_names
            .sort_by_key(|lookup| snapshot.strings[lookup.key.0 as usize].clone());

        let method_id = push_fixture_string(&mut snapshot, "member:ServerMethod");
        let method_name = push_fixture_string(&mut snapshot, "ServerMethod");
        let thin_method_id = push_fixture_string(&mut snapshot, "member:ThinMethod");
        let thin_method_name = push_fixture_string(&mut snapshot, "ThinMethod");
        let second_member_id = push_fixture_string(&mut snapshot, "member:SecondValue");
        let second_member_name = push_fixture_string(&mut snapshot, "SecondValue");
        snapshot.type_members[0].availability_contexts.clear();
        snapshot.type_members.push(HbkTypeMember {
            id: method_id,
            owner: HbkPlatformTypeId(0),
            kind: HbkTypeMemberKind::Method,
            name: HbkName {
                primary: method_name,
                alias: None,
            },
            type_refs: Vec::new(),
            availability_contexts: vec![server],
        });
        snapshot.type_members.push(HbkTypeMember {
            id: thin_method_id,
            owner: HbkPlatformTypeId(0),
            kind: HbkTypeMemberKind::Method,
            name: HbkName {
                primary: thin_method_name,
                alias: None,
            },
            type_refs: Vec::new(),
            availability_contexts: vec![thin, web],
        });
        snapshot.type_members.push(HbkTypeMember {
            id: second_member_id,
            owner: HbkPlatformTypeId(1),
            kind: HbkTypeMemberKind::Property,
            name: HbkName {
                primary: second_member_name,
                alias: None,
            },
            type_refs: Vec::new(),
            availability_contexts: Vec::new(),
        });
        snapshot.members_by_owner = CsrIndex::from_pairs(vec![
            (HbkPlatformTypeId(0), HbkTypeMemberId(0)),
            (HbkPlatformTypeId(0), HbkTypeMemberId(1)),
            (HbkPlatformTypeId(0), HbkTypeMemberId(2)),
            (HbkPlatformTypeId(1), HbkTypeMemberId(3)),
        ]);

        let callable_id = push_fixture_string(&mut snapshot, "callable:ServerMethod");
        let callable_name = push_fixture_string(&mut snapshot, "ServerMethodCallable");
        let global_callable_id = push_fixture_string(&mut snapshot, "callable:GlobalMethod");
        let global_callable_name = push_fixture_string(&mut snapshot, "GlobalMethodCallable");
        let signature_text = push_fixture_string(&mut snapshot, "ServerMethod(Value)");
        let parameter_name = push_fixture_string(&mut snapshot, "Value");
        let return_name = push_fixture_string(&mut snapshot, "Boolean");
        let parameter_ref = HbkTypeRef {
            name: snapshot.type_members[0].type_refs[0].name,
            target: HbkTypeRefTarget::Unresolved,
            type_template_key: None,
            template_binding: None,
        };
        let return_ref = HbkTypeRef {
            name: return_name,
            target: HbkTypeRefTarget::Ok(return_name),
            type_template_key: None,
            template_binding: None,
        };
        snapshot.type_members[2].type_refs = vec![HbkTypeRef {
            name: return_name,
            target: HbkTypeRefTarget::Ambiguous(vec![return_name, parameter_ref.name]),
            type_template_key: None,
            template_binding: None,
        }];
        let signature = HbkSignature {
            text: signature_text,
            parameters: vec![HbkParameter {
                name: parameter_name,
                required: true,
                type_refs: vec![parameter_ref.clone()],
            }],
            return_type_refs: vec![return_ref.clone()],
        };
        snapshot.callables = vec![
            HbkCallable {
                id: callable_id,
                owner: Some(HbkPlatformTypeId(0)),
                kind: HbkCallableKind::Method,
                name: HbkName {
                    primary: callable_name,
                    alias: None,
                },
                signatures: vec![signature.clone()],
                return_type_refs: vec![return_ref.clone()],
                availability_contexts: vec![server],
            },
            HbkCallable {
                id: global_callable_id,
                owner: None,
                kind: HbkCallableKind::GlobalMethod,
                name: HbkName {
                    primary: global_callable_name,
                    alias: None,
                },
                signatures: vec![signature.clone()],
                return_type_refs: vec![return_ref.clone()],
                availability_contexts: Vec::new(),
            },
        ];

        let global_universal_id = push_fixture_string(&mut snapshot, "global:Universal");
        let global_universal_name = push_fixture_string(&mut snapshot, "Universal");
        let global_server_id = push_fixture_string(&mut snapshot, "global:Server");
        let global_server_name = push_fixture_string(&mut snapshot, "ServerGlobal");
        let global_thin_id = push_fixture_string(&mut snapshot, "global:Thin");
        let global_thin_name = push_fixture_string(&mut snapshot, "ThinGlobal");
        snapshot.globals = vec![
            HbkGlobalFact {
                id: global_universal_id,
                kind: HbkGlobalFactKind::Method,
                domain: HbkLanguageDomain::Bsl,
                name: HbkName {
                    primary: global_universal_name,
                    alias: None,
                },
                callable: Some(HbkCallableId(1)),
                type_refs: Vec::new(),
            },
            HbkGlobalFact {
                id: global_server_id,
                kind: HbkGlobalFactKind::Method,
                domain: HbkLanguageDomain::Bsl,
                name: HbkName {
                    primary: global_server_name,
                    alias: None,
                },
                callable: Some(HbkCallableId(1)),
                type_refs: Vec::new(),
            },
            HbkGlobalFact {
                id: global_thin_id,
                kind: HbkGlobalFactKind::Property,
                domain: HbkLanguageDomain::Bsl,
                name: HbkName {
                    primary: global_thin_name,
                    alias: None,
                },
                callable: None,
                type_refs: vec![return_ref.clone()],
            },
        ];
        snapshot.availability_by_fact = CsrIndex::from_pairs(vec![
            (HbkFactRef::Global(HbkGlobalFactId(1)), server),
            (HbkFactRef::Global(HbkGlobalFactId(1)), thin),
            (HbkFactRef::Global(HbkGlobalFactId(2)), thin),
        ]);

        let table_id = push_fixture_string(&mut snapshot, "query_table:Sales");
        let table_name = push_fixture_string(&mut snapshot, "Sales");
        let table_alias = push_fixture_string(&mut snapshot, "Продажи");
        let syntax_name = push_fixture_string(&mut snapshot, "SalesSyntax");
        let identifier = push_fixture_string(&mut snapshot, "SALES");
        let field_id = push_fixture_string(&mut snapshot, "query_field:Amount");
        let field_name = push_fixture_string(&mut snapshot, "Amount");
        let field_note = push_fixture_string(&mut snapshot, "Money value");
        let query_parameter_id = push_fixture_string(&mut snapshot, "query_parameter:Period");
        let query_parameter_name = push_fixture_string(&mut snapshot, "Period");
        let default_value = push_fixture_string(&mut snapshot, "Today");
        snapshot.query_tables = vec![HbkQueryTable {
            id: table_id,
            name: HbkName {
                primary: table_name,
                alias: Some(table_alias),
            },
            syntax: Some(HbkName {
                primary: syntax_name,
                alias: None,
            }),
            identifier: Some(identifier),
            role: Some(model::QueryTableRole::Additional),
            owner_path: vec![HbkName {
                primary: table_name,
                alias: Some(table_alias),
            }],
            template_parameters: vec![parameter_name],
        }];
        snapshot.query_fields = vec![HbkQueryField {
            id: field_id,
            owner: HbkQueryTableId(0),
            name: HbkName {
                primary: field_name,
                alias: None,
            },
            type_refs: vec![return_ref.clone()],
            note: Some(field_note),
        }];
        snapshot.query_parameters = vec![HbkQueryParameter {
            id: query_parameter_id,
            owner: HbkQueryTableId(0),
            name: HbkName {
                primary: query_parameter_name,
                alias: None,
            },
            type_refs: vec![parameter_ref.clone()],
            default_value: Some(default_value),
        }];

        let language_id = push_fixture_string(&mut snapshot, "language:Function");
        let language_name = push_fixture_string(&mut snapshot, "LanguageFunction");
        snapshot.language_facts = vec![HbkLanguageFact {
            id: language_id,
            kind: SearchDocumentKind::LanguageFunction,
            domain: HbkLanguageDomain::Bsl,
            name: HbkName {
                primary: language_name,
                alias: None,
            },
            signatures: vec![signature],
            type_refs: vec![parameter_ref],
            return_type_refs: vec![return_ref],
        }];

        let enum_id = push_fixture_string(&mut snapshot, "enum:Color");
        let enum_name = push_fixture_string(&mut snapshot, "Color");
        let enum_value_id = push_fixture_string(&mut snapshot, "enum_value:Red");
        let enum_value_name = push_fixture_string(&mut snapshot, "Red");
        snapshot.enums = vec![HbkEnum {
            id: enum_id,
            name: HbkName {
                primary: enum_name,
                alias: None,
            },
        }];
        snapshot.enum_values = vec![HbkEnumValue {
            id: enum_value_id,
            owner: HbkEnumId(0),
            name: HbkName {
                primary: enum_value_name,
                alias: None,
            },
        }];

        let source = test_source();
        snapshot.source_by_fact = vec![
            FactSourceLookup {
                fact: HbkFactRef::PlatformType(HbkPlatformTypeId(0)),
                source,
            },
            FactSourceLookup {
                fact: HbkFactRef::TypeMember(HbkTypeMemberId(0)),
                source,
            },
            FactSourceLookup {
                fact: HbkFactRef::Callable(HbkCallableId(0)),
                source,
            },
            FactSourceLookup {
                fact: HbkFactRef::Global(HbkGlobalFactId(0)),
                source,
            },
            FactSourceLookup {
                fact: HbkFactRef::QueryTable(HbkQueryTableId(0)),
                source,
            },
            FactSourceLookup {
                fact: HbkFactRef::QueryField(HbkQueryFieldId(0)),
                source,
            },
            FactSourceLookup {
                fact: HbkFactRef::QueryParameter(HbkQueryParameterId(0)),
                source,
            },
            FactSourceLookup {
                fact: HbkFactRef::LanguageFact(HbkLanguageFactId(0)),
                source,
            },
            FactSourceLookup {
                fact: HbkFactRef::Enum(HbkEnumId(0)),
                source,
            },
            FactSourceLookup {
                fact: HbkFactRef::EnumValue(HbkEnumValueId(0)),
                source,
            },
        ];
        snapshot.source_by_fact.sort_by_key(|lookup| lookup.fact);

        snapshot
    }

    fn lookup_fixture_snapshot() -> HbkFactSnapshot {
        let mut snapshot = forward_payload_fixture_snapshot();
        let shared_type = push_fixture_string(&mut snapshot, "Shared Type");
        let shared_type_key = push_fixture_string(&mut snapshot, "sharedtype");
        snapshot.platform_types[0].name.alias = Some(shared_type);
        snapshot.platform_types[1].name.alias = Some(shared_type);

        let shared_member = push_fixture_string(&mut snapshot, "Shared Member");
        let shared_member_key = push_fixture_string(&mut snapshot, "sharedmember");
        snapshot.type_members[1].name.alias = Some(shared_member);
        snapshot.type_members[2].name.alias = Some(shared_member);

        let shared_global = push_fixture_string(&mut snapshot, "Shared Global");
        let shared_global_key = push_fixture_string(&mut snapshot, "sharedglobal");
        snapshot.globals[0].name.alias = Some(shared_global);
        snapshot.globals[1].name.alias = Some(shared_global);

        let shared_field = push_fixture_string(&mut snapshot, "Shared Field");
        let shared_field_key = push_fixture_string(&mut snapshot, "sharedfield");
        snapshot.query_fields[0].name.alias = Some(shared_field);
        let second_field_id = push_fixture_string(&mut snapshot, "query_field:Tax");
        let second_field_name = push_fixture_string(&mut snapshot, "Tax");
        snapshot.query_fields.push(HbkQueryField {
            id: second_field_id,
            owner: HbkQueryTableId(0),
            name: HbkName {
                primary: second_field_name,
                alias: Some(shared_field),
            },
            type_refs: Vec::new(),
            note: None,
        });

        let shared_parameter = push_fixture_string(&mut snapshot, "Shared Parameter");
        let shared_parameter_key = push_fixture_string(&mut snapshot, "sharedparameter");
        snapshot.query_parameters[0].name.alias = Some(shared_parameter);
        let second_parameter_id = push_fixture_string(&mut snapshot, "query_parameter:Limit");
        let second_parameter_name = push_fixture_string(&mut snapshot, "Limit");
        snapshot.query_parameters.push(HbkQueryParameter {
            id: second_parameter_id,
            owner: HbkQueryTableId(0),
            name: HbkName {
                primary: second_parameter_name,
                alias: Some(shared_parameter),
            },
            type_refs: Vec::new(),
            default_value: None,
        });

        let shared_value = push_fixture_string(&mut snapshot, "Shared Value");
        let shared_value_key = push_fixture_string(&mut snapshot, "sharedvalue");
        snapshot.enum_values[0].name.alias = Some(shared_value);
        let second_value_id = push_fixture_string(&mut snapshot, "enum_value:Blue");
        let second_value_name = push_fixture_string(&mut snapshot, "Blue");
        snapshot.enum_values.push(HbkEnumValue {
            id: second_value_id,
            owner: HbkEnumId(0),
            name: HbkName {
                primary: second_value_name,
                alias: Some(shared_value),
            },
        });

        let duplicate_fact_id = push_fixture_string(&mut snapshot, "duplicate-fact-id");
        let callable_name_key = push_fixture_string(&mut snapshot, "servermethodcallable");
        let module_owner = push_fixture_string(&mut snapshot, "commonmodule");
        let module_event = push_fixture_string(&mut snapshot, "eventname");
        let language_key = push_fixture_string(&mut snapshot, "bsl");
        let table_name_key = push_fixture_string(&mut snapshot, "sales");
        let table_syntax_key = push_fixture_string(&mut snapshot, "salessyntax");
        let table_identifier = push_fixture_string(&mut snapshot, "SALES ID");
        let table_identifier_key = push_fixture_string(&mut snapshot, "salesid");
        snapshot.query_tables[0].identifier = Some(table_identifier);
        let language_name_key = push_fixture_string(&mut snapshot, "languagefunction");
        let enum_name_key = push_fixture_string(&mut snapshot, "color");
        let available_since = push_fixture_string(&mut snapshot, "8.3.0");
        let relation_kind = push_fixture_string(&mut snapshot, "typereference");

        snapshot.fact_ids = vec![
            IdLookup {
                key: duplicate_fact_id,
                value: HbkFactRef::PlatformType(HbkPlatformTypeId(0)),
            },
            IdLookup {
                key: duplicate_fact_id,
                value: HbkFactRef::Global(HbkGlobalFactId(0)),
            },
        ];
        snapshot.platform_type_ids = snapshot
            .platform_types
            .iter()
            .enumerate()
            .map(|(index, value)| IdLookup {
                key: value.id,
                value: HbkPlatformTypeId(index as u32),
            })
            .collect();
        snapshot.platform_type_names = vec![
            NameLookup {
                key: shared_type_key,
                value: HbkPlatformTypeId(0),
            },
            NameLookup {
                key: shared_type_key,
                value: HbkPlatformTypeId(1),
            },
        ];
        snapshot.member_ids = snapshot
            .type_members
            .iter()
            .enumerate()
            .map(|(index, value)| IdLookup {
                key: value.id,
                value: HbkTypeMemberId(index as u32),
            })
            .collect();
        snapshot.members_by_owner_name = vec![
            OwnerNameLookup {
                owner: HbkPlatformTypeId(0),
                key: shared_member_key,
                value: HbkTypeMemberId(1),
            },
            OwnerNameLookup {
                owner: HbkPlatformTypeId(0),
                key: shared_member_key,
                value: HbkTypeMemberId(2),
            },
        ];
        snapshot.members_by_owner_name_kind = vec![
            MemberNameKindLookup {
                owner: HbkPlatformTypeId(0),
                key: shared_member_key,
                kind: Some(HbkTypeMemberKind::Method),
                value: HbkTypeMemberId(1),
            },
            MemberNameKindLookup {
                owner: HbkPlatformTypeId(0),
                key: shared_member_key,
                kind: Some(HbkTypeMemberKind::Method),
                value: HbkTypeMemberId(2),
            },
        ];
        snapshot.callable_ids = snapshot
            .callables
            .iter()
            .enumerate()
            .map(|(index, value)| IdLookup {
                key: value.id,
                value: HbkCallableId(index as u32),
            })
            .collect();
        snapshot.callables_by_owner =
            CsrIndex::from_pairs(vec![(HbkPlatformTypeId(0), HbkCallableId(0))]);
        snapshot.callables_by_owner_name = vec![OwnerNameLookup {
            owner: HbkPlatformTypeId(0),
            key: callable_name_key,
            value: HbkCallableId(0),
        }];
        snapshot.constructors_by_type =
            CsrIndex::from_pairs(vec![(HbkPlatformTypeId(0), HbkCallableId(0))]);
        snapshot.global_names = vec![
            NameLookup {
                key: shared_global_key,
                value: HbkGlobalFactId(0),
            },
            NameLookup {
                key: shared_global_key,
                value: HbkGlobalFactId(1),
            },
        ];
        snapshot.globals_by_domain_name_kind = vec![
            GlobalNameKindLookup {
                domain: HbkLanguageDomain::Bsl,
                key: shared_global_key,
                kind: Some(HbkGlobalFactKind::Method),
                value: HbkGlobalFactId(0),
            },
            GlobalNameKindLookup {
                domain: HbkLanguageDomain::Bsl,
                key: shared_global_key,
                kind: Some(HbkGlobalFactKind::Method),
                value: HbkGlobalFactId(1),
            },
        ];
        snapshot.module_event_names = vec![
            OwnerNameLookup {
                owner: module_owner,
                key: module_event,
                value: HbkCallableId(0),
            },
            OwnerNameLookup {
                owner: module_owner,
                key: module_event,
                value: HbkCallableId(1),
            },
        ];
        snapshot.module_contexts_by_domain_language_kind = vec![
            ModuleContextLookup {
                domain: HbkLanguageDomain::Bsl,
                language_key,
                module_kind: module_owner,
                value: HbkCallableId(0),
            },
            ModuleContextLookup {
                domain: HbkLanguageDomain::Bsl,
                language_key,
                module_kind: module_owner,
                value: HbkCallableId(1),
            },
        ];
        snapshot.query_table_ids = vec![IdLookup {
            key: snapshot.query_tables[0].id,
            value: HbkQueryTableId(0),
        }];
        snapshot.query_table_names = vec![NameLookup {
            key: table_name_key,
            value: HbkQueryTableId(0),
        }];
        snapshot.query_table_syntax_names = vec![NameLookup {
            key: table_syntax_key,
            value: HbkQueryTableId(0),
        }];
        snapshot.query_table_identifiers = vec![NameLookup {
            key: table_identifier_key,
            value: HbkQueryTableId(0),
        }];
        snapshot.query_fields_by_table = CsrIndex::from_pairs(vec![
            (HbkQueryTableId(0), HbkQueryFieldId(0)),
            (HbkQueryTableId(0), HbkQueryFieldId(1)),
        ]);
        snapshot.query_fields_by_table_name = vec![
            OwnerNameLookup {
                owner: HbkQueryTableId(0),
                key: shared_field_key,
                value: HbkQueryFieldId(0),
            },
            OwnerNameLookup {
                owner: HbkQueryTableId(0),
                key: shared_field_key,
                value: HbkQueryFieldId(1),
            },
        ];
        snapshot.query_parameters_by_table = CsrIndex::from_pairs(vec![
            (HbkQueryTableId(0), HbkQueryParameterId(0)),
            (HbkQueryTableId(0), HbkQueryParameterId(1)),
        ]);
        snapshot.query_parameters_by_table_name = vec![
            OwnerNameLookup {
                owner: HbkQueryTableId(0),
                key: shared_parameter_key,
                value: HbkQueryParameterId(0),
            },
            OwnerNameLookup {
                owner: HbkQueryTableId(0),
                key: shared_parameter_key,
                value: HbkQueryParameterId(1),
            },
        ];
        snapshot.language_ids = vec![IdLookup {
            key: snapshot.language_facts[0].id,
            value: HbkLanguageFactId(0),
        }];
        snapshot.language_names = vec![NameLookup {
            key: language_name_key,
            value: HbkLanguageFactId(0),
        }];
        snapshot.enum_ids = vec![IdLookup {
            key: snapshot.enums[0].id,
            value: HbkEnumId(0),
        }];
        snapshot.enum_names = vec![NameLookup {
            key: enum_name_key,
            value: HbkEnumId(0),
        }];
        snapshot.enum_value_ids = snapshot
            .enum_values
            .iter()
            .enumerate()
            .map(|(index, value)| IdLookup {
                key: value.id,
                value: HbkEnumValueId(index as u32),
            })
            .collect();
        snapshot.enum_values_by_enum = CsrIndex::from_pairs(vec![
            (HbkEnumId(0), HbkEnumValueId(0)),
            (HbkEnumId(0), HbkEnumValueId(1)),
        ]);
        snapshot.enum_values_by_enum_name = vec![
            OwnerNameLookup {
                owner: HbkEnumId(0),
                key: shared_value_key,
                value: HbkEnumValueId(0),
            },
            OwnerNameLookup {
                owner: HbkEnumId(0),
                key: shared_value_key,
                value: HbkEnumValueId(1),
            },
        ];
        snapshot.availability_since_by_fact = vec![FactStringLookup {
            fact: HbkFactRef::Global(HbkGlobalFactId(1)),
            value: available_since,
        }];
        snapshot.relations_by_source_kind = CsrIndex::from_pairs(vec![
            (
                RelationLookupKey {
                    source: HbkFactRef::Global(HbkGlobalFactId(1)),
                    kind: relation_kind,
                },
                HbkFactRef::PlatformType(HbkPlatformTypeId(0)),
            ),
            (
                RelationLookupKey {
                    source: HbkFactRef::Global(HbkGlobalFactId(1)),
                    kind: relation_kind,
                },
                HbkFactRef::TypeMember(HbkTypeMemberId(1)),
            ),
        ]);

        let strings = &snapshot.strings;
        let by_id = |left: StringId, right: StringId| {
            strings[left.0 as usize].cmp(&strings[right.0 as usize])
        };
        snapshot.fact_ids.sort_by(|left, right| {
            by_id(left.key, right.key).then_with(|| left.value.cmp(&right.value))
        });
        snapshot.platform_type_ids.sort_by(|left, right| {
            by_id(left.key, right.key).then_with(|| left.value.cmp(&right.value))
        });
        snapshot.platform_type_names.sort_by(|left, right| {
            by_id(left.key, right.key).then_with(|| left.value.cmp(&right.value))
        });
        snapshot.member_ids.sort_by(|left, right| {
            by_id(left.key, right.key).then_with(|| left.value.cmp(&right.value))
        });
        snapshot.callable_ids.sort_by(|left, right| {
            by_id(left.key, right.key).then_with(|| left.value.cmp(&right.value))
        });
        snapshot.enum_value_ids.sort_by(|left, right| {
            by_id(left.key, right.key).then_with(|| left.value.cmp(&right.value))
        });

        snapshot
    }

    fn assert_lookup_eq<T: std::fmt::Debug + PartialEq>(
        actual: impl IntoIterator<Item = T>,
        expected: impl IntoIterator<Item = T>,
    ) {
        assert_eq!(
            actual.into_iter().collect::<Vec<_>>(),
            expected.into_iter().collect::<Vec<_>>()
        );
    }

    fn push_fixture_string(snapshot: &mut HbkFactSnapshot, value: &str) -> StringId {
        if let Some(index) = snapshot
            .strings
            .iter()
            .position(|candidate| candidate == value)
        {
            return StringId(index as u32);
        }
        let id = StringId(snapshot.strings.len() as u32);
        snapshot.strings.push(value.to_string());
        id
    }

    fn assert_name(view: X1NameView, owned: &HbkName) {
        assert_eq!(view.primary(), owned.primary);
        assert_eq!(view.alias(), owned.alias);
    }

    fn assert_type_refs<'a>(
        views: impl ExactSizeIterator<Item = X1TypeRefView<'a>>,
        owned: &[HbkTypeRef],
    ) {
        assert_eq!(views.len(), owned.len());
        for (view, owned) in views.zip(owned) {
            assert_eq!(view.name(), owned.name);
            assert_eq!(view.type_template_key(), owned.type_template_key);
            match &owned.target {
                HbkTypeRefTarget::Ok(id) => {
                    assert_eq!(view.target_kind(), X1TypeRefTargetKind::Ok);
                    assert_eq!(view.target_ok(), Some(*id));
                    assert_eq!(view.ambiguous_targets().len(), 0);
                }
                HbkTypeRefTarget::Unresolved => {
                    assert_eq!(view.target_kind(), X1TypeRefTargetKind::Unresolved);
                    assert_eq!(view.target_ok(), None);
                    assert_eq!(view.ambiguous_targets().len(), 0);
                }
                HbkTypeRefTarget::Ambiguous(ids) => {
                    assert_eq!(view.target_kind(), X1TypeRefTargetKind::Ambiguous);
                    assert_eq!(view.target_ok(), None);
                    assert_eq!(view.ambiguous_targets().collect::<Vec<_>>(), *ids);
                }
            }
            match (&view.template_binding(), &owned.template_binding) {
                (Some(binding), Some(owned_binding)) => {
                    assert_eq!(binding.template_key(), owned_binding.template_key);
                    assert_eq!(
                        binding.arguments().collect::<Vec<_>>(),
                        owned_binding.arguments
                    );
                }
                (None, None) => {}
                _ => panic!("mapped template binding differs from owned payload"),
            }
        }
    }

    fn fixture_build_report(
        root: &Path,
        source_bytes: &[u8],
    ) -> (
        HbkFactSnapshotBuildReport,
        X1ArtifactIdentity,
        PathBuf,
        PathBuf,
    ) {
        let source_dir = root.join("8.3.27.1859");
        fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("shcntx_ru.hbk");
        fs::write(&source, source_bytes).unwrap();
        let index_path = root.join("provider.sqlite");
        let metadata = IndexMetadata {
            locale: "ru".to_string(),
            source_locale: "ru".to_string(),
            source_hbk: source.to_string_lossy().into_owned(),
            source_extraction_schema_version: SUPPORTED_EXTRACTION_SCHEMA,
        };
        build_index_from_builder(&index_path, &metadata, fixture_index_builder(&source)).unwrap();
        let report = HbkFactSnapshot::from_path_with_stage_timings(&index_path).unwrap();
        let identity = artifact_identity(&report).unwrap();
        (report, identity, source, index_path)
    }

    fn make_readonly(path: &Path) {
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(path, permissions).unwrap();
    }

    fn overwrite_readonly(path: &Path, bytes: &[u8]) {
        make_writable(path);
        fs::write(path, bytes).unwrap();
        make_readonly(path);
    }

    fn make_writable(path: &Path) {
        let mut permissions = fs::metadata(path).unwrap().permissions();
        #[cfg(unix)]
        permissions.set_mode(permissions.mode() | 0o200);
        #[cfg(not(unix))]
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions).unwrap();
    }

    fn write_content_addressed_test_slot(slot: &Path, bytes: &[u8]) -> PathBuf {
        let generations = slot.join(X1_SLOT_GENERATIONS_DIR);
        fs::create_dir_all(&generations).unwrap();
        fs::write(slot.join(X1_SLOT_LOCK_FILE), b"").unwrap();
        let generation_name = generation_file_name(&bytes_sha256(bytes)).unwrap();
        let generation = generations.join(&generation_name);
        write_readonly_artifact(&generation, bytes);
        write_readonly_artifact(
            &slot.join(X1_SLOT_CURRENT_FILE),
            format!("{generation_name}\n").as_bytes(),
        );
        generation
    }

    fn slot_generation_names(slot: &Path) -> Vec<String> {
        let generations = slot.join(X1_SLOT_GENERATIONS_DIR);
        if !generations.exists() {
            return Vec::new();
        }
        let mut names = fs::read_dir(generations)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| {
                name.starts_with(X1_GENERATION_PREFIX) && name.ends_with(X1_GENERATION_SUFFIX)
            })
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    fn slot_tree_names(slot: &Path) -> Vec<String> {
        let mut names = fs::read_dir(slot)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        names.extend(
            slot_generation_names(slot)
                .into_iter()
                .map(|name| format!("{X1_SLOT_GENERATIONS_DIR}/{name}")),
        );
        names.sort();
        names
    }

    fn temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{unique}-{name}"))
    }
}
