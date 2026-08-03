use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Cursor, Read, Write};
use std::ops::Range;
use std::path::Path;

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

#[allow(dead_code)]
struct X1MappedGeneration {
    _file: File,
    mmap: Mmap,
    _sections: Vec<Section>,
    counts: HbkFactSnapshotCounts,
    source_locale: StringId,
    identity: X1ArtifactIdentity,
}

#[allow(dead_code)]
impl X1MappedGeneration {
    /// # Safety
    ///
    /// The caller must guarantee that the explicit generation file cannot be
    /// modified or truncated for the returned owner's lifetime. Task 3.5 will
    /// uphold this precondition with the stable-slot shared reader lock before
    /// this becomes a public runtime open operation.
    unsafe fn open(path: &Path, expected: &X1RuntimeExpectation) -> Result<Self, SearchError> {
        let before = fs::metadata(path).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        validate_generation_metadata(path, &before)?;

        let file = File::open(path).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let file_metadata = file.metadata().map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
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
        if after.len() != file_metadata.len() {
            return Err(snapshot_artifact_invalid(
                path,
                "X1 artifact size changed while validating mapping",
            ));
        }

        Ok(Self {
            _file: file,
            mmap,
            _sections: sections,
            counts,
            source_locale,
            identity,
        })
    }

    fn artifact_len(&self) -> usize {
        self.mmap.len()
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
    compare: impl Fn(T) -> Ordering,
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
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

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
            toc_path: None,
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

    fn temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{unique}-{name}"))
    }
}
