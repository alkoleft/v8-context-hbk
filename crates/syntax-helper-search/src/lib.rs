use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::convert::Infallible;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, OptionalExtension, Statement, params};
use strsim::levenshtein;
use syntax_helper_language as language;
pub use syntax_helper_model as model;

pub const INDEX_SCHEMA_VERSION: u32 = 12;
const TYPE_REFERENCE_RELATION_WEIGHT: i64 = 12;

type TypeTemplateRow = (
    String,
    Vec<String>,
    Option<model::PlatformTypeTemplateKey>,
    Option<String>,
);

#[derive(Debug, Clone)]
struct TypeTemplateFact {
    key: model::PlatformTypeTemplateKey,
    parameters: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IndexMetadata {
    pub locale: String,
    pub source_locale: String,
    pub source_hbk: String,
    pub source_extraction_schema_version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Keywords,
    Fuzzy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub document: SearchDocument,
    pub score: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedHit {
    pub document: SearchDocument,
    pub depth: u32,
    pub via: Vec<RelationStep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationStep {
    pub from: String,
    pub to: String,
    pub edge_kind: String,
    pub label: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchDocumentKind {
    PlatformType,
    TypeProperty,
    TypeMethod,
    Constructor,
    GlobalMethod,
    GlobalProperty,
    ModuleEvent,
    TypeEvent,
    UnknownEvent,
    QueryTable,
    QueryTableField,
    QueryTableParameter,
    LanguageType,
    LanguageConstruct,
    LanguageFunction,
    LanguageOperator,
    LanguageKeyword,
    LanguageLiteral,
    Enum,
    EnumValue,
}

impl SearchDocumentKind {
    pub const ALL: [Self; 20] = [
        Self::PlatformType,
        Self::TypeProperty,
        Self::TypeMethod,
        Self::Constructor,
        Self::GlobalMethod,
        Self::GlobalProperty,
        Self::ModuleEvent,
        Self::TypeEvent,
        Self::UnknownEvent,
        Self::QueryTable,
        Self::QueryTableField,
        Self::QueryTableParameter,
        Self::LanguageType,
        Self::LanguageConstruct,
        Self::LanguageFunction,
        Self::LanguageOperator,
        Self::LanguageKeyword,
        Self::LanguageLiteral,
        Self::Enum,
        Self::EnumValue,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PlatformType => "platform_type",
            Self::TypeProperty => "type_property",
            Self::TypeMethod => "type_method",
            Self::Constructor => "constructor",
            Self::GlobalMethod => "global_method",
            Self::GlobalProperty => "global_property",
            Self::ModuleEvent => "module_event",
            Self::TypeEvent => "type_event",
            Self::UnknownEvent => "unknown_event",
            Self::QueryTable => "query_table",
            Self::QueryTableField => "query_table_field",
            Self::QueryTableParameter => "query_table_parameter",
            Self::LanguageType => "language_type",
            Self::LanguageConstruct => "language_construct",
            Self::LanguageFunction => "language_function",
            Self::LanguageOperator => "language_operator",
            Self::LanguageKeyword => "language_keyword",
            Self::LanguageLiteral => "language_literal",
            Self::Enum => "enum",
            Self::EnumValue => "enum_value",
        }
    }

    pub fn from_storage(value: &str) -> Option<Self> {
        match value {
            "platform_type" => Some(Self::PlatformType),
            "type_property" => Some(Self::TypeProperty),
            "type_method" => Some(Self::TypeMethod),
            "constructor" => Some(Self::Constructor),
            "global_method" => Some(Self::GlobalMethod),
            "global_property" => Some(Self::GlobalProperty),
            "module_event" => Some(Self::ModuleEvent),
            "type_event" => Some(Self::TypeEvent),
            "unknown_event" => Some(Self::UnknownEvent),
            "query_table" => Some(Self::QueryTable),
            "query_table_field" => Some(Self::QueryTableField),
            "query_table_parameter" => Some(Self::QueryTableParameter),
            "language_type" => Some(Self::LanguageType),
            "language_construct" => Some(Self::LanguageConstruct),
            "language_function" => Some(Self::LanguageFunction),
            "language_operator" => Some(Self::LanguageOperator),
            "language_keyword" => Some(Self::LanguageKeyword),
            "language_literal" => Some(Self::LanguageLiteral),
            "enum" => Some(Self::Enum),
            "enum_value" => Some(Self::EnumValue),
            _ => None,
        }
    }

    pub fn priority(self) -> i64 {
        match self {
            Self::PlatformType => 10,
            Self::TypeProperty => 20,
            Self::TypeMethod => 30,
            Self::Constructor => 40,
            Self::GlobalMethod => 50,
            Self::GlobalProperty => 60,
            Self::ModuleEvent => 70,
            Self::TypeEvent => 80,
            Self::UnknownEvent => 90,
            Self::QueryTable => 100,
            Self::QueryTableField => 110,
            Self::QueryTableParameter => 120,
            Self::LanguageType => 125,
            Self::LanguageConstruct => 126,
            Self::LanguageFunction => 127,
            Self::LanguageOperator => 128,
            Self::LanguageKeyword => 129,
            Self::LanguageLiteral => 130,
            Self::Enum => 140,
            Self::EnumValue => 150,
        }
    }

    pub fn is_callable(self) -> bool {
        matches!(
            self,
            Self::GlobalMethod
                | Self::TypeMethod
                | Self::Constructor
                | Self::ModuleEvent
                | Self::TypeEvent
                | Self::UnknownEvent
                | Self::LanguageFunction
        )
    }

    pub fn is_language(self) -> bool {
        matches!(
            self,
            Self::LanguageType
                | Self::LanguageConstruct
                | Self::LanguageFunction
                | Self::LanguageOperator
                | Self::LanguageKeyword
                | Self::LanguageLiteral
        )
    }

    pub fn type_ref_kind(self) -> &'static str {
        match self {
            Self::PlatformType => "extends",
            Self::QueryTableField => "query_field_type",
            Self::QueryTableParameter => "query_parameter_type",
            Self::TypeProperty
            | Self::TypeMethod
            | Self::Constructor
            | Self::GlobalMethod
            | Self::GlobalProperty
            | Self::ModuleEvent
            | Self::TypeEvent
            | Self::UnknownEvent
            | Self::QueryTable
            | Self::LanguageType
            | Self::LanguageConstruct
            | Self::LanguageFunction
            | Self::LanguageOperator
            | Self::LanguageKeyword
            | Self::LanguageLiteral
            | Self::Enum
            | Self::EnumValue => "property_type",
        }
    }

    pub fn public_type_ref_kinds(self) -> &'static [&'static str] {
        match self {
            Self::PlatformType => &["extends"],
            Self::QueryTableField => &["query_field_type"],
            Self::QueryTableParameter => &["query_parameter_type"],
            Self::TypeProperty
            | Self::TypeMethod
            | Self::Constructor
            | Self::GlobalMethod
            | Self::GlobalProperty
            | Self::ModuleEvent
            | Self::TypeEvent
            | Self::UnknownEvent
            | Self::QueryTable
            | Self::LanguageType
            | Self::LanguageConstruct
            | Self::LanguageFunction
            | Self::LanguageOperator
            | Self::LanguageKeyword
            | Self::LanguageLiteral
            | Self::Enum
            | Self::EnumValue => &["property_type"],
        }
    }
}

impl fmt::Display for SearchDocumentKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug)]
struct UnknownSearchDocumentKind(String);

impl fmt::Display for UnknownSearchDocumentKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown search document kind '{}'", self.0)
    }
}

impl std::error::Error for UnknownSearchDocumentKind {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchDocument {
    pub id: String,
    pub kind: SearchDocumentKind,
    pub name: model::LocalizedName,
    pub owner: Option<model::LocalizedName>,
    pub signatures: Vec<SearchSignature>,
    pub type_refs: Vec<String>,
    pub return_types: Vec<String>,
    pub type_ref_facts: Vec<SearchTypeRef>,
    pub return_type_facts: Vec<SearchTypeRef>,
    pub description: Option<String>,
    pub preview: String,
    pub parameter_terms: Vec<String>,
    pub relation_keys: Vec<String>,
    pub owner_relation_key: Option<String>,
    pub explicit_type_ref_ids: Vec<Option<String>>,
    pub explicit_return_type_ref_ids: Vec<Option<String>>,
    pub availability_contexts: Vec<String>,
    pub available_since: Option<String>,
    pub metadata_kind: Option<String>,
    pub template_parameters: Vec<String>,
    pub type_template_key: Option<model::PlatformTypeTemplateKey>,
    pub type_template_classification_diagnostic: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexBuildReport {
    pub warnings: Vec<IndexBuildWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexBuildWarning {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchSignature {
    pub text: String,
    pub parameters: Vec<SearchParameter>,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchParameter {
    pub name: String,
    pub required: bool,
    pub type_refs: Vec<String>,
    pub type_ref_facts: Vec<SearchTypeRef>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchTypeRef {
    pub name: String,
    pub target_type_id: Option<String>,
    pub type_template_key: Option<model::PlatformTypeTemplateKey>,
    pub template_binding: Option<model::TypeTemplateBinding>,
}

impl SearchDocument {
    fn signature_text_lines(&self) -> Vec<String> {
        self.signatures
            .iter()
            .map(|signature| signature.text.clone())
            .filter(|text| !text.is_empty())
            .collect()
    }

    fn with_section_facts(mut self, facts: &model::SectionFacts) -> Self {
        self.availability_contexts = facts
            .availability
            .contexts
            .iter()
            .map(|context| availability_context_code(*context).to_string())
            .collect();
        self.available_since = facts
            .available_since
            .as_ref()
            .and_then(|fact| fact.version.clone());
        self
    }
}

#[derive(Debug, Clone, Copy)]
enum TypeIdentityLookup {
    Primary,
    Alias,
}

#[derive(Debug, Default)]
pub struct SearchIndexBuilder {
    drafts: Vec<DocumentDraft>,
    platform_types: Vec<PlatformTypeIdentityInput>,
    query_tables: Vec<QueryTableIdentityInput>,
    enums: Vec<EnumIdentityInput>,
}

impl SearchIndexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_language_fact(&mut self, fact: language::LanguageFact) {
        self.drafts.push(DocumentDraft::new(
            language_document(&fact),
            DraftIdentity::Immediate(fact.id),
        ));
    }

    fn into_documents(self, source_locale: &str) -> Result<DocumentsBuild, SearchError> {
        let identities =
            DocumentIdentities::from_inputs(&self.platform_types, &self.query_tables, &self.enums);
        let mut documents = self
            .drafts
            .into_iter()
            .map(|draft| draft.into_document(&identities))
            .collect::<Result<Vec<_>, _>>()?;
        documents.sort_by(|left, right| {
            kind_priority(left.kind)
                .cmp(&kind_priority(right.kind))
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut warnings = deduplicate_documents(&mut documents);
        warnings.extend(classify_platform_type_templates(
            &mut documents,
            source_locale,
        ));
        validate_document_id_collisions(&documents)?;
        Ok(DocumentsBuild {
            documents,
            warnings,
        })
    }
}

impl model::SyntaxHelperSink for SearchIndexBuilder {
    type Error = Infallible;

    fn global_context(&mut self, _record: model::GlobalContext) -> Result<(), Self::Error> {
        Ok(())
    }

    fn global_method(&mut self, record: model::GlobalMethod) -> Result<(), Self::Error> {
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::GlobalMethod,
                None,
                &record.name,
                &record.signatures,
                &record.return_types,
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::Immediate(document_identity(
                SearchDocumentKind::GlobalMethod.as_str(),
                None,
                &record.name,
            )),
        ));
        Ok(())
    }

    fn global_property(&mut self, record: model::GlobalProperty) -> Result<(), Self::Error> {
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::GlobalProperty,
                None,
                &record.name,
                &[],
                &[],
                &record.type_refs,
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::Immediate(document_identity(
                SearchDocumentKind::GlobalProperty.as_str(),
                None,
                &record.name,
            )),
        ));
        Ok(())
    }

    fn global_context_event(
        &mut self,
        record: model::GlobalContextEvent,
    ) -> Result<(), Self::Error> {
        let owner = event_owner(&record);
        let kind = match record.semantic.record_family {
            model::RecordFamily::ModuleEvent => SearchDocumentKind::ModuleEvent,
            model::RecordFamily::TypeEvent => SearchDocumentKind::TypeEvent,
            _ => SearchDocumentKind::UnknownEvent,
        };
        self.drafts.push(DocumentDraft::new(
            document(
                kind,
                owner.as_ref(),
                &record.name,
                &record.signatures,
                &[],
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::Immediate(document_identity(
                kind.as_str(),
                owner.as_ref(),
                &record.name,
            )),
        ));
        Ok(())
    }

    fn platform_type(&mut self, record: model::PlatformType) -> Result<(), Self::Error> {
        self.platform_types.push(PlatformTypeIdentityInput {
            identity: record.identity.clone(),
            name_primary: record.name.primary.clone(),
            semantic: record.semantic.clone(),
        });
        let mut document = document(
            SearchDocumentKind::PlatformType,
            None,
            &record.name,
            &[],
            &[],
            &record
                .extends
                .iter()
                .map(type_ref_from_name)
                .collect::<Vec<_>>(),
            record.description.as_deref(),
            String::new(),
        )
        .with_section_facts(&record.facts);
        if record.type_kind == model::PlatformTypeKind::MetadataTemplate
            && let Some(metadata_kind) = record.metadata_kind
        {
            document.metadata_kind = Some(metadata_kind);
            document.template_parameters = record.template_parameters;
            document.type_template_key = record.type_template_key;
        }
        self.drafts.push(DocumentDraft::new(
            document,
            DraftIdentity::PlatformType {
                name_primary: record.name.primary,
                semantic: record.semantic,
            },
        ));
        Ok(())
    }

    fn query_table(&mut self, record: model::QueryTable) -> Result<(), Self::Error> {
        let name = model::LocalizedName {
            primary: record.name.clone(),
            alias: record
                .syntax
                .as_ref()
                .and_then(|syntax| syntax.alias.clone()),
        };
        self.query_tables.push(QueryTableIdentityInput {
            identity: record.identity.clone(),
            name_primary: record.name.clone(),
            identifier: record.identifier.clone(),
            semantic: record.semantic.clone(),
        });
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::QueryTable,
                None,
                &name,
                &[],
                &[],
                &[],
                record.description.as_deref(),
                String::new(),
            ),
            DraftIdentity::QueryTable {
                name_primary: record.name,
                identifier: record.identifier,
                semantic: record.semantic,
            },
        ));
        Ok(())
    }

    fn type_method(&mut self, record: model::PlatformMethod) -> Result<(), Self::Error> {
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::TypeMethod,
                Some(&record.owner),
                &record.name,
                &record.signatures,
                &record.return_types,
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::TypeOwned {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn type_property(&mut self, record: model::PlatformProperty) -> Result<(), Self::Error> {
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::TypeProperty,
                Some(&record.owner),
                &record.name,
                &[],
                &[],
                &record.type_refs,
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::TypeOwned {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn table_field(&mut self, record: model::QueryTableField) -> Result<(), Self::Error> {
        let name = model::LocalizedName {
            primary: record.name,
            alias: None,
        };
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::QueryTableField,
                Some(&record.owner),
                &name,
                &[],
                &[],
                &record.type_refs,
                record.description.as_deref(),
                String::new(),
            ),
            DraftIdentity::QueryMember {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn table_parameter(&mut self, record: model::QueryTableParameter) -> Result<(), Self::Error> {
        let name = model::LocalizedName {
            primary: record.name,
            alias: None,
        };
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::QueryTableParameter,
                Some(&record.owner),
                &name,
                &[],
                &[],
                &record.type_refs,
                record.description.as_deref(),
                String::new(),
            ),
            DraftIdentity::QueryMember {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn constructor(&mut self, record: model::Constructor) -> Result<(), Self::Error> {
        let name = model::LocalizedName {
            primary: record
                .signatures
                .first()
                .map(|signature| signature.text.clone())
                .unwrap_or_else(|| format!("Новый {}", record.owner.primary)),
            alias: record.name.alias,
        };
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::Constructor,
                Some(&record.owner),
                &name,
                &record.signatures,
                &[],
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::TypeOwned {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn enum_definition(&mut self, record: model::EnumDefinition) -> Result<(), Self::Error> {
        self.enums.push(EnumIdentityInput {
            identity: record.identity.clone(),
            name_primary: record.name.primary.clone(),
            name_alias: record.name.alias.clone(),
            source_html_path: record.source.html_path.clone(),
        });
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::Enum,
                None,
                &record.name,
                &[],
                &[],
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::Enum {
                name_primary: record.name.primary,
                name_alias: record.name.alias,
                source_html_path: record.source.html_path,
            },
        ));
        Ok(())
    }

    fn enum_value(&mut self, record: model::EnumValue) -> Result<(), Self::Error> {
        self.drafts.push(DocumentDraft::new(
            document(
                SearchDocumentKind::EnumValue,
                Some(&record.owner),
                &record.name,
                &[],
                &[],
                &[],
                record.description.as_deref(),
                String::new(),
            )
            .with_section_facts(&record.facts),
            DraftIdentity::EnumValue {
                owner_identity: record.owner_identity,
            },
        ));
        Ok(())
    }

    fn diagnostic(&mut self, _record: model::SyntaxHelperDiagnostic) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum SearchError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Sqlite {
        path: PathBuf,
        source: rusqlite::Error,
    },
    WriterLockTimeout {
        path: PathBuf,
    },
    MissingIndex {
        path: PathBuf,
    },
    UnsupportedSchemaVersion {
        path: PathBuf,
        expected: u32,
        actual: String,
    },
    DuplicateDocumentId {
        id: String,
        count: usize,
    },
    MissingParentIdentity {
        kind: String,
        name: String,
        owner: String,
    },
    AmbiguousLookup {
        name: String,
        matches: usize,
    },
    InvalidMetadata {
        path: PathBuf,
        key: &'static str,
        value: String,
        source: std::num::ParseIntError,
    },
    MissingMetadata {
        path: PathBuf,
        key: &'static str,
    },
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    f,
                    "failed to access search index '{}': {source}",
                    path.display()
                )
            }
            Self::Sqlite { path, source } => {
                write!(
                    f,
                    "failed to use search index '{}': {source}",
                    path.display()
                )
            }
            Self::WriterLockTimeout { path } => {
                write!(
                    f,
                    "timed out waiting for search index writer lock '{}'",
                    path.display()
                )
            }
            Self::MissingIndex { path } => {
                write!(f, "search index does not exist: {}", path.display())
            }
            Self::UnsupportedSchemaVersion {
                path,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "unsupported search index schema version in '{}': expected {expected}, got {actual}; rebuild the index",
                    path.display()
                )
            }
            Self::DuplicateDocumentId { id, count } => {
                write!(
                    f,
                    "duplicate Syntax Assistant search document id '{id}': {count} documents"
                )
            }
            Self::MissingParentIdentity { kind, name, owner } => {
                write!(
                    f,
                    "missing Syntax Assistant parent identity for {kind} '{name}' owned by '{owner}'"
                )
            }
            Self::AmbiguousLookup { name, matches } => {
                write!(
                    f,
                    "ambiguous Syntax Assistant lookup for '{name}': {matches} matches"
                )
            }
            Self::InvalidMetadata {
                path, key, value, ..
            } => {
                write!(
                    f,
                    "invalid search index metadata in '{}': key '{key}' has value '{value}'",
                    path.display()
                )
            }
            Self::MissingMetadata { path, key } => {
                write!(
                    f,
                    "missing search index metadata in '{}': key '{key}'",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for SearchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Sqlite { source, .. } => Some(source),
            Self::InvalidMetadata { source, .. } => Some(source),
            Self::WriterLockTimeout { .. }
            | Self::MissingIndex { .. }
            | Self::UnsupportedSchemaVersion { .. }
            | Self::DuplicateDocumentId { .. }
            | Self::MissingParentIdentity { .. }
            | Self::AmbiguousLookup { .. }
            | Self::MissingMetadata { .. } => None,
        }
    }
}

impl SearchError {
    fn metadata_parse(
        path: PathBuf,
        key: &'static str,
        value: String,
        source: std::num::ParseIntError,
    ) -> Self {
        Self::InvalidMetadata {
            path,
            key,
            value,
            source,
        }
    }
}

pub fn build_index_from_builder(
    path: impl AsRef<Path>,
    metadata: &IndexMetadata,
    builder: SearchIndexBuilder,
) -> Result<(), SearchError> {
    build_index_from_builder_with_report(path, metadata, builder).map(|_| ())
}

pub fn build_index_from_builder_with_report(
    path: impl AsRef<Path>,
    metadata: &IndexMetadata,
    builder: SearchIndexBuilder,
) -> Result<IndexBuildReport, SearchError> {
    let build = builder.into_documents(&metadata.source_locale)?;
    build_index_from_documents(path, metadata, build.documents)?;
    Ok(IndexBuildReport {
        warnings: build.warnings,
    })
}

fn build_index_from_documents(
    path: impl AsRef<Path>,
    metadata: &IndexMetadata,
    documents: Vec<SearchDocument>,
) -> Result<(), SearchError> {
    validate_document_id_collisions(&documents)?;
    let path = path.as_ref();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| SearchError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let _lock = WriterLock::acquire(path)?;
    let temp_path = temp_index_path(path);
    remove_sqlite_artifacts(&temp_path)?;

    let result = build_index_file(&temp_path, metadata, documents).and_then(|()| {
        remove_sqlite_sidecars(path)?;
        fs::rename(&temp_path, path).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })
    });
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn build_index_file(
    path: &Path,
    metadata: &IndexMetadata,
    documents: Vec<SearchDocument>,
) -> Result<(), SearchError> {
    let mut connection = Connection::open(path).map_err(|source| SearchError::Sqlite {
        path: path.to_path_buf(),
        source,
    })?;
    create_schema(&connection, path)?;
    write_metadata(&connection, path, metadata)?;
    let transaction = connection
        .transaction()
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    insert_documents(&transaction, path, &documents)?;
    insert_normalized_facts(&transaction, path, &documents)?;
    rebuild_document_fts(&transaction, path)?;
    insert_relations_from_documents(&transaction, path, &documents)?;
    create_lookup_indexes(&transaction, path)?;
    transaction.commit().map_err(|source| SearchError::Sqlite {
        path: path.to_path_buf(),
        source,
    })?;
    validate_index(&connection, path)
}

fn deduplicate_documents(documents: &mut Vec<SearchDocument>) -> Vec<IndexBuildWarning> {
    let mut counts = BTreeMap::<String, usize>::new();
    for document in documents.iter() {
        *counts.entry(document.id.clone()).or_default() += 1;
    }
    let warnings = counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(id, count)| IndexBuildWarning {
            code: "DUPLICATE_DOCUMENT_ID",
            message: format!(
                "duplicate Syntax Assistant search document id '{id}': {count} documents; kept the last document"
            ),
        })
        .collect::<Vec<_>>();
    if warnings.is_empty() {
        return warnings;
    }

    let mut remaining = counts;
    documents.retain(|document| {
        let Some(count) = remaining.get_mut(&document.id) else {
            return true;
        };
        if *count <= 1 {
            return true;
        }
        *count -= 1;
        false
    });
    warnings
}

fn classify_platform_type_templates(
    documents: &mut [SearchDocument],
    source_locale: &str,
) -> Vec<IndexBuildWarning> {
    let allow_primary_fallback = source_locale == "root";
    let by_name = relation_lookup(documents);
    let type_id_by_normalized_id = documents
        .iter()
        .filter(|document| document.kind == SearchDocumentKind::PlatformType)
        .map(|document| (normalize_lookup_key(&document.id), document.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let template_infos = documents
        .iter()
        .filter(|document| {
            document.kind == SearchDocumentKind::PlatformType && document.metadata_kind.is_some()
        })
        .filter_map(|document| {
            Some(PlatformTypeTemplateInfo {
                id: document.id.clone(),
                base: model::platform_type_template_base_for_source(
                    &document.name,
                    allow_primary_fallback,
                ),
                metadata_kind: document.metadata_kind.clone()?,
            })
        })
        .collect::<Vec<_>>();
    let mut family_roots = template_infos
        .iter()
        .filter_map(|template| {
            template
                .base
                .as_deref()
                .and_then(manager_family_root)
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    family_roots.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    family_roots.dedup();

    let mut classified = BTreeMap::<String, model::PlatformTypeTemplateKey>::new();
    let mut diagnostics = BTreeMap::<String, String>::new();
    let mut unassigned = BTreeSet::<String>::new();
    for template in &template_infos {
        let Some(base) = template.base.as_deref() else {
            unassigned.insert(template.id.clone());
            continue;
        };
        if let Some(root) = family_roots
            .iter()
            .find(|root| base.starts_with(root.as_str()))
        {
            let key = model::PlatformTypeTemplateKey::new(
                root.clone(),
                type_template_variant(root, base),
            );
            diagnostics.insert(
                template.id.clone(),
                format!("manager_root family={} variant={}", key.family, key.variant),
            );
            classified.insert(template.id.clone(), key);
        } else {
            unassigned.insert(template.id.clone());
        }
    }

    let template_id_by_metadata_kind = unique_lookup(template_infos.iter().map(|template| {
        (
            normalize_lookup_key(&template.metadata_kind),
            template.id.clone(),
        )
    }));
    let mut direct_scores = BTreeMap::<String, BTreeMap<String, usize>>::new();
    for document in documents.iter() {
        let owner_type_id = owner_type_id(document, &by_name, &type_id_by_normalized_id);
        let owner_key = owner_type_id
            .as_deref()
            .and_then(|type_id| classified.get(type_id));
        let owner_unassigned_id = owner_type_id
            .as_deref()
            .filter(|type_id| unassigned.contains(*type_id));
        for target_name in document_type_template_ref_names(document) {
            let Some(target_type_id) = template_id_by_metadata_kind
                .get(&normalize_lookup_key(target_name))
                .and_then(|type_id| type_id.as_deref())
            else {
                continue;
            };
            if unassigned.contains(target_type_id) {
                if let Some(owner_key) = owner_key {
                    *direct_scores
                        .entry(target_type_id.to_string())
                        .or_default()
                        .entry(owner_key.family.clone())
                        .or_default() += 1;
                }
            } else if let Some(owner_unassigned_id) = owner_unassigned_id
                && let Some(target_key) = classified.get(target_type_id)
            {
                *direct_scores
                    .entry(owner_unassigned_id.to_string())
                    .or_default()
                    .entry(target_key.family.clone())
                    .or_default() += 1;
            }
        }
    }

    let mut warnings = Vec::new();
    for template in template_infos
        .iter()
        .filter(|template| unassigned.contains(template.id.as_str()))
    {
        let Some(base) = template.base.as_deref() else {
            let message = format!(
                "type template '{}' has no alias base and primary fallback is only allowed for root source locale",
                template.id
            );
            diagnostics.insert(template.id.clone(), message.clone());
            warnings.push(IndexBuildWarning {
                code: "TYPE_TEMPLATE_UNCLASSIFIED",
                message,
            });
            continue;
        };
        match direct_scores.get(&template.id) {
            Some(scores) if scores.len() == 1 => {
                let (family, score) = scores.iter().next().expect("one score must exist");
                let key = model::PlatformTypeTemplateKey::new(
                    family.clone(),
                    type_template_variant(family, base),
                );
                diagnostics.insert(
                    template.id.clone(),
                    format!(
                        "direct_type_ref family={} variant={} score={}",
                        key.family, key.variant, score
                    ),
                );
                classified.insert(template.id.clone(), key);
            }
            Some(scores) if !scores.is_empty() => {
                let message = format!(
                    "type template '{}' has multiple direct type-template family candidates: {}",
                    template.id,
                    scores
                        .keys()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                diagnostics.insert(template.id.clone(), message.clone());
                warnings.push(IndexBuildWarning {
                    code: "TYPE_TEMPLATE_AMBIGUOUS_FAMILY",
                    message,
                });
            }
            _ => {
                let message = format!(
                    "type template '{}' has no manager-root or direct type-template type-reference family evidence",
                    template.id
                );
                diagnostics.insert(template.id.clone(), message.clone());
                warnings.push(IndexBuildWarning {
                    code: "TYPE_TEMPLATE_UNCLASSIFIED",
                    message,
                });
            }
        }
    }

    for document in documents {
        if let Some(key) = classified.get(&document.id) {
            document.type_template_key = Some(key.clone());
        }
        if let Some(diagnostic) = diagnostics.get(&document.id) {
            document.type_template_classification_diagnostic = Some(diagnostic.clone());
        }
    }
    warnings
}

#[derive(Debug)]
struct PlatformTypeTemplateInfo {
    id: String,
    base: Option<String>,
    metadata_kind: String,
}

fn manager_family_root(base: &str) -> Option<&str> {
    base.strip_suffix("Manager")
        .or_else(|| base.strip_suffix("Менеджер"))
        .filter(|root| !root.is_empty())
}

fn type_template_variant(family: &str, base: &str) -> String {
    base.strip_prefix(family)
        .filter(|suffix| !suffix.is_empty())
        .unwrap_or(base)
        .to_string()
}

fn unique_lookup(
    items: impl Iterator<Item = (String, String)>,
) -> BTreeMap<String, Option<String>> {
    let mut lookup = BTreeMap::<String, Option<String>>::new();
    for (key, value) in items {
        match lookup.get_mut(&key) {
            Some(existing) if existing.as_deref() == Some(value.as_str()) => {}
            Some(existing) => *existing = None,
            None => {
                lookup.insert(key, Some(value));
            }
        }
    }
    lookup
}

fn document_type_template_ref_names(document: &SearchDocument) -> impl Iterator<Item = &str> {
    document
        .type_refs
        .iter()
        .chain(document.return_types.iter())
        .map(String::as_str)
        .chain(
            document
                .signatures
                .iter()
                .flat_map(|signature| signature.parameters.iter())
                .flat_map(|parameter| parameter.type_refs.iter().map(String::as_str)),
        )
}

fn validate_document_id_collisions(documents: &[SearchDocument]) -> Result<(), SearchError> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for document in documents {
        *counts.entry(document.id.as_str()).or_default() += 1;
    }
    if let Some((id, count)) = counts.into_iter().find(|(_, count)| *count > 1) {
        return Err(SearchError::DuplicateDocumentId {
            id: id.to_string(),
            count,
        });
    }
    Ok(())
}

pub struct SearchIndex {
    path: PathBuf,
    connection: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredIndexMetadata {
    pub locale: String,
    pub source_locale: String,
    pub source_hbk: String,
    pub source_extraction_schema_version: u32,
}

impl SearchIndex {
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self, SearchError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(SearchError::MissingIndex {
                path: path.to_path_buf(),
            });
        }
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
        validate_schema_version(&connection, path)?;
        Ok(Self {
            path: path.to_path_buf(),
            connection,
        })
    }

    pub fn metadata(&self) -> Result<StoredIndexMetadata, SearchError> {
        let source_extraction_schema_version =
            self.metadata_value("source_extraction_schema_version")?;
        Ok(StoredIndexMetadata {
            locale: self.metadata_value("locale")?,
            source_locale: self.metadata_value("source_locale")?,
            source_hbk: self.metadata_value("source_hbk")?,
            source_extraction_schema_version: source_extraction_schema_version.parse().map_err(
                |source| {
                    SearchError::metadata_parse(
                        self.path.clone(),
                        "source_extraction_schema_version",
                        source_extraction_schema_version.clone(),
                        source,
                    )
                },
            )?,
        })
    }

    pub fn get_by_name(&self, name: &str) -> Result<Vec<SearchHit>, SearchError> {
        let key = normalize_lookup_key(name);
        self.get_by_key(&key)
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<SearchHit>, SearchError> {
        self.document(id)
            .map(|document| document.map(|document| SearchHit { document, score: 0 }))
    }

    pub fn get_by_owner_member(
        &self,
        owner: &str,
        member: &str,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let key = owner_member_key(owner, member);
        self.get_by_key(&key)
    }

    pub fn type_identity_by_id(&self, type_id: &str) -> Result<Option<SearchHit>, SearchError> {
        let Some(document_id) = self.type_identity_document_id(type_id)? else {
            return Ok(None);
        };
        self.get_by_id(&document_id)
    }

    pub fn type_identities_by_name(&self, name: &str) -> Result<Vec<SearchHit>, SearchError> {
        self.type_identities_by_lookup_key(&normalize_lookup_key(name), TypeIdentityLookup::Primary)
    }

    pub fn type_identities_by_alias(&self, alias: &str) -> Result<Vec<SearchHit>, SearchError> {
        self.type_identities_by_lookup_key(&normalize_lookup_key(alias), TypeIdentityLookup::Alias)
    }

    pub fn type_template_by_key(
        &self,
        kind: &model::PlatformTypeTemplateKey,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary,
                        d.owner_alias, d.signature_text, d.description, d.availability_contexts,
                        d.available_since
                 FROM type_templates t
                 JOIN documents d ON d.id = t.document_id
                 WHERE t.template_family = ?1
                   AND t.template_variant = ?2
                 ORDER BY d.name_primary, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![kind.family, kind.variant], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    pub fn members_by_type_id(&self, type_id: &str) -> Result<Vec<SearchHit>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM members m \
                 JOIN documents d ON d.id = m.document_id \
                 WHERE m.owner_type_id = ?1 \
                 ORDER BY d.kind_priority, m.name_primary, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([type_id], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    pub fn member_by_owner_type_id(
        &self,
        owner_type_id: &str,
        member: &str,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let normalized_member = normalize_lookup_key(member);
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM document_names n \
                 JOIN members m INDEXED BY members_document_owner_idx \
                   ON m.document_id = n.document_id \
                  AND m.owner_type_id = ?1 \
                 JOIN documents d ON d.id = m.document_id \
                 WHERE n.key = ?2 \
                   AND n.key_kind IN ('primary', 'alias') \
                 ORDER BY d.kind_priority, m.name_primary, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![owner_type_id, normalized_member], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    pub fn constructors_by_name(&self, name: &str) -> Result<Vec<SearchHit>, SearchError> {
        let Some(root) = self.root_by_name(name)? else {
            return Ok(Vec::new());
        };
        self.constructors_by_type_id(&root.document.id)
    }

    pub fn constructors_by_type_id(&self, type_id: &str) -> Result<Vec<SearchHit>, SearchError> {
        self.owned_documents_by_kind(type_id, "constructor", 100)
    }

    pub fn callable_by_id(&self, callable_id: &str) -> Result<Option<SearchHit>, SearchError> {
        if !self.callable_exists(callable_id)? {
            return Ok(None);
        }
        self.get_by_id(callable_id)
    }

    pub fn callable_by_owner_type_id(
        &self,
        owner_type_id: &str,
        callable: &str,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let normalized_callable = normalize_lookup_key(callable);
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM document_names n \
                 JOIN callables c INDEXED BY callables_document_owner_idx \
                   ON c.document_id = n.document_id \
                  AND c.owner_type_id = ?1 \
                 JOIN documents d ON d.id = c.document_id \
                 WHERE n.key = ?2 \
                   AND n.key_kind IN ('primary', 'alias') \
                 ORDER BY d.kind_priority, d.name_primary, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![owner_type_id, normalized_callable], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    pub fn search(
        &self,
        query: &str,
        mode: SearchMode,
        limit: usize,
    ) -> Result<Vec<SearchHit>, SearchError> {
        match mode {
            SearchMode::Keywords => self.keyword_search(query, limit),
            SearchMode::Fuzzy => self.fuzzy_search(query, limit),
        }
    }

    pub fn related_by_name(
        &self,
        name: &str,
        max_depth: u32,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        let Some(root) = self.root_by_name(name)? else {
            return Ok(Vec::new());
        };
        self.related(&root.document.id, max_depth.min(5), limit)
    }

    pub fn related_by_id(
        &self,
        id: &str,
        max_depth: u32,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        if self.document(id)?.is_none() {
            return Ok(Vec::new());
        }
        self.related(id, max_depth.min(5), limit)
    }

    pub fn related_by_id_and_edge(
        &self,
        id: &str,
        edge_kind: &str,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        let Some(source_document) = self.document(id)? else {
            return Ok(Vec::new());
        };
        if matches!(edge_kind, "has_type" | "returns" | "constructs") {
            let hits = self.related_type_refs_by_id_and_edge(id, edge_kind, limit)?;
            if !hits.is_empty() {
                return Ok(hits);
            }
            if !source_document.kind.is_language() {
                return Ok(Vec::new());
            }
        }
        let mut hits = Vec::new();
        for edge in self.edges_by_kind(id, edge_kind)? {
            let Some(document) = self.document(&edge.to)? else {
                continue;
            };
            hits.push(RelatedHit {
                document,
                depth: 1,
                via: vec![edge],
            });
            if hits.len() >= limit {
                break;
            }
        }
        Ok(hits)
    }

    fn related_type_refs_by_id_and_edge(
        &self,
        id: &str,
        edge_kind: &str,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        let ref_kind = edge_ref_kind(edge_kind);
        let mut statement = self
            .connection
            .prepare(
                "SELECT target_type_id, target_type_name
                 FROM type_refs
                 WHERE source_document_id = ?1
                   AND ref_kind = ?2
                   AND target_type_id IS NOT NULL
                 ORDER BY ordinal, target_type_id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![id, ref_kind], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|source| self.sqlite(source))?;
        let rows = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        let mut hits = Vec::new();
        for (target_id, target_name) in rows {
            let Some(document) = self.document(&target_id)? else {
                continue;
            };
            hits.push(RelatedHit {
                document,
                depth: 1,
                via: vec![RelationStep {
                    from: id.to_string(),
                    to: target_id,
                    edge_kind: edge_kind.to_string(),
                    label: target_name,
                    evidence: "type_ref".to_string(),
                }],
            });
            if hits.len() >= limit {
                break;
            }
        }
        Ok(hits)
    }

    pub fn related_by_owner_member(
        &self,
        owner: &str,
        member: &str,
        max_depth: u32,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        let roots = self.get_by_owner_member(owner, member)?;
        let Some(root) = roots.first() else {
            return Ok(Vec::new());
        };
        if roots.len() > 1 {
            return Err(SearchError::AmbiguousLookup {
                name: format!("{owner}.{member}"),
                matches: roots.len(),
            });
        }
        self.related(&root.document.id, max_depth.min(5), limit)
    }

    pub fn document_count(&self) -> Result<i64, SearchError> {
        self.connection
            .query_row("SELECT count(*) FROM documents", [], |row| row.get(0))
            .map_err(|source| self.sqlite(source))
    }

    pub fn owner_type_id_for_document(
        &self,
        document_id: &str,
    ) -> Result<Option<String>, SearchError> {
        self.connection
            .query_row(
                "SELECT owner_type_id FROM members WHERE document_id = ?1
                 UNION
                 SELECT owner_type_id FROM callables WHERE document_id = ?1 AND owner_type_id IS NOT NULL
                 LIMIT 1",
                [document_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|source| self.sqlite(source))
    }

    pub fn target_type_ids_for_document(
        &self,
        document_id: &str,
    ) -> Result<Vec<String>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT target_type_id
                 FROM type_refs
                 WHERE source_document_id = ?1 AND target_type_id IS NOT NULL
                 ORDER BY target_type_id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([document_id], |row| row.get(0))
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    fn get_by_key(&self, key: &str) -> Result<Vec<SearchHit>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM document_names n \
                 JOIN documents d ON d.id = n.document_id \
                 WHERE n.key = ?1 \
                 ORDER BY d.kind_priority, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([key], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    fn type_identity_document_id(&self, type_id: &str) -> Result<Option<String>, SearchError> {
        self.connection
            .query_row(
                "SELECT document_id FROM type_identities WHERE type_id = ?1",
                [type_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|source| self.sqlite(source))
    }

    fn type_identities_by_lookup_key(
        &self,
        key: &str,
        lookup: TypeIdentityLookup,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let key_kind = match lookup {
            TypeIdentityLookup::Primary => "primary",
            TypeIdentityLookup::Alias => "alias",
        };
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM document_names n \
                 JOIN type_identities t ON t.document_id = n.document_id \
                 JOIN documents d ON d.id = t.document_id \
                 WHERE n.key = ?1 \
                   AND n.key_kind = ?2 \
                 ORDER BY d.kind_priority, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([key, key_kind], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    fn callable_exists(&self, callable_id: &str) -> Result<bool, SearchError> {
        self.connection
            .query_row(
                "SELECT 1 FROM callables WHERE callable_id = ?1",
                [callable_id],
                |_| Ok(()),
            )
            .optional()
            .map(|value| value.is_some())
            .map_err(|source| self.sqlite(source))
    }

    fn root_by_name(&self, name: &str) -> Result<Option<SearchHit>, SearchError> {
        let roots = self.get_by_name(name)?;
        let Some(root) = roots.first() else {
            return Ok(None);
        };
        if roots.len() > 1 {
            return Err(SearchError::AmbiguousLookup {
                name: name.to_string(),
                matches: roots.len(),
            });
        }
        Ok(Some(root.clone()))
    }

    fn owned_documents_by_kind(
        &self,
        owner_id: &str,
        kind: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM relations r \
                 JOIN documents d ON d.id = r.target_id \
                 WHERE r.source_id = ?1 AND r.edge_kind = 'owns' AND d.kind = ?2 \
                 ORDER BY d.kind_priority, d.id \
                 LIMIT ?3",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![owner_id, kind, limit as i64], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    fn keyword_search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, SearchError> {
        let fts_query = fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since, \
                 CAST(bm25(document_fts) * 1000000 AS INTEGER) \
                 FROM document_fts \
                 JOIN documents d ON d.id = document_fts.document_id \
                 WHERE document_fts MATCH ?1 \
                 ORDER BY bm25(document_fts), d.kind_priority, d.id \
                 LIMIT ?2",
            )
            .map_err(|source| self.sqlite(source))?;
        let sql_limit = limit.saturating_mul(10).max(50) as i64;
        let rows = statement
            .query_map(params![fts_query, sql_limit], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: row.get(10)?,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let mut hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        hits = self.hydrate_hits(hits)?;
        hits.sort_by(|left, right| {
            keyword_order(query, &left.document)
                .cmp(&keyword_order(query, &right.document))
                .then_with(|| left.score.cmp(&right.score))
                .then_with(|| left.document.id.cmp(&right.document.id))
        });
        hits.truncate(limit);
        Ok(hits)
    }

    fn fuzzy_search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, SearchError> {
        let normalized_query = normalize_lookup_key(query);
        let mut candidates = self.fuzzy_candidates(&normalized_query, limit.saturating_mul(20))?;
        candidates.sort_by(|left, right| {
            let left_score = fuzzy_score(&normalized_query, &left.document);
            let right_score = fuzzy_score(&normalized_query, &right.document);
            left_score
                .cmp(&right_score)
                .then_with(|| {
                    left.document
                        .kind
                        .as_str()
                        .cmp(right.document.kind.as_str())
                })
                .then_with(|| left.document.id.cmp(&right.document.id))
        });
        Ok(candidates
            .into_iter()
            .filter_map(|mut hit| {
                let distance = fuzzy_score(&normalized_query, &hit.document);
                (distance <= fuzzy_threshold(&normalized_query)).then(|| {
                    hit.score = distance as i64;
                    hit
                })
            })
            .take(limit)
            .collect())
    }

    fn fuzzy_candidates(
        &self,
        normalized_query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let prefix = normalized_query.chars().take(4).collect::<String>();
        if prefix.is_empty() {
            return Ok(Vec::new());
        }
        let pattern = format!("{prefix}%");
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM document_names n \
                 JOIN documents d ON d.id = n.document_id \
                 WHERE n.key LIKE ?1 \
                 ORDER BY d.kind_priority, d.id \
                 LIMIT ?2",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![pattern, limit as i64], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    fn related(
        &self,
        root_id: &str,
        max_depth: u32,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        let mut queue = VecDeque::from([(root_id.to_string(), 0, Vec::<RelationStep>::new())]);
        let mut visited = BTreeSet::from([root_id.to_string()]);
        let mut hits = Vec::new();

        while let Some((document_id, depth, path)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in self.edges(&document_id)? {
                if !visited.insert(edge.to.clone()) {
                    continue;
                }
                let Some(document) = self.document(&edge.to)? else {
                    continue;
                };
                let mut via = path.clone();
                via.push(edge.clone());
                hits.push(RelatedHit {
                    document,
                    depth: depth + 1,
                    via: via.clone(),
                });
                if hits.len() >= limit {
                    return Ok(hits);
                }
                queue.push_back((edge.to, depth + 1, via));
            }
        }
        Ok(hits)
    }

    fn edges(&self, document_id: &str) -> Result<Vec<RelationStep>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT source_id, target_id, edge_kind, label, evidence \
                 FROM relations \
                 WHERE source_id = ?1 \
                 ORDER BY weight, edge_kind, target_id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([document_id], |row| {
                let source: String = row.get(0)?;
                let target: String = row.get(1)?;
                Ok(RelationStep {
                    from: source,
                    to: target,
                    edge_kind: row.get(2)?,
                    label: row.get(3)?,
                    evidence: row.get(4)?,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    fn edges_by_kind(
        &self,
        document_id: &str,
        edge_kind: &str,
    ) -> Result<Vec<RelationStep>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT source_id, target_id, edge_kind, label, evidence \
                 FROM relations \
                 WHERE source_id = ?1 AND edge_kind = ?2 \
                 ORDER BY weight, target_id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![document_id, edge_kind], |row| {
                let source: String = row.get(0)?;
                let target: String = row.get(1)?;
                Ok(RelationStep {
                    from: source,
                    to: target,
                    edge_kind: row.get(2)?,
                    label: row.get(3)?,
                    evidence: row.get(4)?,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    fn document(&self, id: &str) -> Result<Option<SearchDocument>, SearchError> {
        let document = self
            .connection
            .query_row(
                "SELECT id, kind, name_primary, name_alias, owner_primary, owner_alias, \
                 signature_text, description, availability_contexts, available_since \
                 FROM documents WHERE id = ?1",
                [id],
                document_from_row,
            )
            .optional()
            .map_err(|source| self.sqlite(source))?;
        document
            .map(|document| self.hydrate_document(document))
            .transpose()
    }

    fn hydrate_hits(&self, hits: Vec<SearchHit>) -> Result<Vec<SearchHit>, SearchError> {
        hits.into_iter()
            .map(|hit| {
                self.hydrate_document(hit.document)
                    .map(|document| SearchHit { document, ..hit })
            })
            .collect()
    }

    fn hydrate_document(
        &self,
        mut document: SearchDocument,
    ) -> Result<SearchDocument, SearchError> {
        let mut type_refs = Vec::new();
        let mut type_ref_facts = Vec::new();
        for ref_kind in document.kind.public_type_ref_kinds() {
            let facts = self.type_ref_facts(&document.id, ref_kind)?;
            type_refs.extend(facts.iter().map(|type_ref| type_ref.name.clone()));
            type_ref_facts.extend(facts);
        }
        document.type_refs = type_refs;
        document.type_ref_facts = type_ref_facts;
        let return_ref_kind = if document.kind == SearchDocumentKind::Constructor {
            "constructor_result"
        } else {
            "return_type"
        };
        document.return_type_facts = self.type_ref_facts(&document.id, return_ref_kind)?;
        document.return_types = document
            .return_type_facts
            .iter()
            .map(|type_ref| type_ref.name.clone())
            .collect();
        document.signatures = self.signatures_for(&document)?;
        document.parameter_terms = self.parameter_terms_for(&document)?;
        if let Some((
            metadata_kind,
            template_parameters,
            type_template_key,
            type_template_classification_diagnostic,
        )) = self.type_template_for(&document.id)?
        {
            document.metadata_kind = Some(metadata_kind);
            document.template_parameters = template_parameters;
            document.type_template_key = type_template_key;
            document.type_template_classification_diagnostic =
                type_template_classification_diagnostic;
        }
        Ok(document)
    }

    fn type_template_for(&self, document_id: &str) -> Result<Option<TypeTemplateRow>, SearchError> {
        self.connection
            .query_row(
                "SELECT metadata_kind, template_parameters, template_family, template_variant,
                        template_classification_diagnostic
                 FROM type_templates WHERE document_id = ?1",
                [document_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        split_lines(row.get::<_, String>(1)?),
                        type_template_key_from_codes(
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                        ),
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| self.sqlite(source))
    }

    fn type_ref_facts(
        &self,
        document_id: &str,
        ref_kind: &str,
    ) -> Result<Vec<SearchTypeRef>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT target_type_name, target_type_id, type_template_family,
                        type_template_variant, template_binding_kind,
                        template_binding_owner_parameter_index,
                        template_binding_target_parameter_index,
                        template_binding_arguments
                 FROM type_refs
                 WHERE source_document_id = ?1 AND ref_kind = ?2
                 ORDER BY source_signature_ordinal, source_parameter_ordinal, ordinal, target_type_name",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![document_id, ref_kind], search_type_ref_from_row)
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    fn signatures_for(
        &self,
        document: &SearchDocument,
    ) -> Result<Vec<SearchSignature>, SearchError> {
        let signature_texts = document.signature_text_lines();
        let mut statement = self
            .connection
            .prepare(
                "SELECT signature_id, ordinal, title, description
                 FROM signatures
                 WHERE callable_id = ?1
                 ORDER BY ordinal",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([document.id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|source| self.sqlite(source))?;
        let rows = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        let mut signatures = Vec::new();
        for (signature_id, ordinal, title, description) in rows {
            signatures.push(SearchSignature {
                text: signature_texts
                    .get(ordinal as usize)
                    .cloned()
                    .unwrap_or_default(),
                parameters: self.parameters_for(&signature_id)?,
                title,
                description,
            });
        }
        if signatures.is_empty() {
            signatures.extend(signature_texts.into_iter().map(|text| SearchSignature {
                text,
                parameters: Vec::new(),
                title: None,
                description: None,
            }));
        }
        Ok(signatures)
    }

    fn parameters_for(&self, signature_id: &str) -> Result<Vec<SearchParameter>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT ordinal, name, required, description
                 FROM parameters
                 WHERE signature_id = ?1
                 ORDER BY ordinal",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([signature_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|source| self.sqlite(source))?;
        let rows = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        rows.into_iter()
            .map(|(ordinal, name, required, description)| {
                self.parameter_type_facts(signature_id, ordinal)
                    .map(|type_ref_facts| SearchParameter {
                        name,
                        required,
                        type_refs: type_ref_facts
                            .iter()
                            .map(|type_ref| type_ref.name.clone())
                            .collect(),
                        type_ref_facts,
                        description,
                    })
            })
            .collect()
    }

    fn parameter_type_facts(
        &self,
        signature_id: &str,
        parameter_ordinal: i64,
    ) -> Result<Vec<SearchTypeRef>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT target_type_name, target_type_id, type_template_family,
                        type_template_variant, template_binding_kind,
                        template_binding_owner_parameter_index,
                        template_binding_target_parameter_index,
                        template_binding_arguments
                 FROM type_refs
                 WHERE source_signature_id = ?1
                   AND source_parameter_ordinal = ?2
                   AND ref_kind = 'parameter_type'
                 ORDER BY ordinal, target_type_name",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(
                params![signature_id, parameter_ordinal],
                search_type_ref_from_row,
            )
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    fn parameter_terms_for(&self, document: &SearchDocument) -> Result<Vec<String>, SearchError> {
        let mut terms = BTreeSet::new();
        for signature in &document.signatures {
            for parameter in &signature.parameters {
                terms.insert(parameter.name.clone());
                terms.extend(parameter.type_refs.iter().cloned());
            }
        }
        Ok(terms.into_iter().collect())
    }

    fn sqlite(&self, source: rusqlite::Error) -> SearchError {
        SearchError::Sqlite {
            path: self.path.clone(),
            source,
        }
    }

    fn metadata_value(&self, key: &'static str) -> Result<String, SearchError> {
        self.connection
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .map_err(|source| match source {
                rusqlite::Error::QueryReturnedNoRows => SearchError::MissingMetadata {
                    path: self.path.clone(),
                    key,
                },
                source => self.sqlite(source),
            })
    }
}

fn validate_schema_version(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    let actual = connection
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(|source| match source {
            rusqlite::Error::QueryReturnedNoRows => SearchError::MissingMetadata {
                path: path.to_path_buf(),
                key: "schema_version",
            },
            source => SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            },
        })?;
    if actual == INDEX_SCHEMA_VERSION.to_string() {
        Ok(())
    } else {
        Err(SearchError::UnsupportedSchemaVersion {
            path: path.to_path_buf(),
            expected: INDEX_SCHEMA_VERSION,
            actual,
        })
    }
}

#[derive(Debug, Clone)]
struct Relation {
    source_id: String,
    target_id: String,
    edge_kind: &'static str,
    label: String,
    evidence: &'static str,
    weight: i64,
}

#[derive(Debug)]
struct DocumentDraft {
    document: SearchDocument,
    identity: DraftIdentity,
}

struct DocumentsBuild {
    documents: Vec<SearchDocument>,
    warnings: Vec<IndexBuildWarning>,
}

impl DocumentDraft {
    fn new(document: SearchDocument, identity: DraftIdentity) -> Self {
        Self { document, identity }
    }

    fn into_document(
        mut self,
        identities: &DocumentIdentities,
    ) -> Result<SearchDocument, SearchError> {
        match self.identity {
            DraftIdentity::Immediate(id) => {
                self.document.id = id;
            }
            DraftIdentity::PlatformType {
                name_primary,
                semantic,
            } => {
                self.document.id = identities.platform_type_identity_by(&name_primary, &semantic);
                self.document
                    .relation_keys
                    .push(identity_relation_key(&self.document.id));
            }
            DraftIdentity::TypeOwned { owner_identity } => {
                let owner_identity = missing_owner_identity_error(&self.document, owner_identity)?;
                self.document.id = owned_document_identity(
                    self.document.kind.as_str(),
                    &owner_identity,
                    &self.document.name.primary,
                );
                self.document.owner_relation_key = Some(identity_relation_key(&owner_identity));
            }
            DraftIdentity::QueryTable {
                name_primary,
                identifier,
                semantic,
            } => {
                self.document.id = identities.query_table_identity_by(
                    &name_primary,
                    identifier.as_deref(),
                    &semantic,
                );
                self.document
                    .relation_keys
                    .push(semantic_relation_key(&semantic, &name_primary));
                self.document
                    .relation_keys
                    .push(identity_relation_key(&self.document.id));
            }
            DraftIdentity::QueryMember { owner_identity } => {
                let owner_identity = missing_owner_identity_error(&self.document, owner_identity)?;
                self.document.id = owned_document_identity(
                    self.document.kind.as_str(),
                    &owner_identity,
                    &self.document.name.primary,
                );
                self.document.owner_relation_key = Some(identity_relation_key(&owner_identity));
            }
            DraftIdentity::Enum {
                name_primary,
                name_alias,
                source_html_path,
            } => {
                self.document.id = identities.enum_identity_by(
                    &name_primary,
                    name_alias.as_deref(),
                    &source_html_path,
                );
                self.document
                    .relation_keys
                    .push(identity_relation_key(&self.document.id));
            }
            DraftIdentity::EnumValue { owner_identity } => {
                let owner_identity = missing_owner_identity_error(&self.document, owner_identity)?;
                self.document.id = owned_document_identity(
                    SearchDocumentKind::EnumValue.as_str(),
                    &owner_identity,
                    &self.document.name.primary,
                );
                self.document.owner_relation_key = Some(identity_relation_key(&owner_identity));
            }
        }
        Ok(self.document)
    }
}

fn missing_owner_identity_error(
    document: &SearchDocument,
    owner_identity: Option<String>,
) -> Result<String, SearchError> {
    owner_identity.ok_or_else(|| SearchError::MissingParentIdentity {
        kind: document.kind.as_str().to_string(),
        name: document.name.primary.clone(),
        owner: document
            .owner
            .as_ref()
            .map(model::LocalizedName::display_name)
            .unwrap_or_default(),
    })
}

#[derive(Debug)]
enum DraftIdentity {
    Immediate(String),
    PlatformType {
        name_primary: String,
        semantic: model::SemanticContext,
    },
    TypeOwned {
        owner_identity: Option<String>,
    },
    QueryTable {
        name_primary: String,
        identifier: Option<String>,
        semantic: model::SemanticContext,
    },
    QueryMember {
        owner_identity: Option<String>,
    },
    Enum {
        name_primary: String,
        name_alias: Option<String>,
        source_html_path: String,
    },
    EnumValue {
        owner_identity: Option<String>,
    },
}

#[derive(Debug)]
struct PlatformTypeIdentityInput {
    identity: Option<String>,
    name_primary: String,
    semantic: model::SemanticContext,
}

#[derive(Debug)]
struct QueryTableIdentityInput {
    identity: Option<String>,
    name_primary: String,
    identifier: Option<String>,
    semantic: model::SemanticContext,
}

#[derive(Debug)]
struct EnumIdentityInput {
    identity: Option<String>,
    name_primary: String,
    name_alias: Option<String>,
    source_html_path: String,
}

fn create_schema(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = OFF;
             PRAGMA synchronous = OFF;
             PRAGMA locking_mode = EXCLUSIVE;
             PRAGMA temp_store = MEMORY;
             CREATE TABLE meta (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE documents (
                 id TEXT PRIMARY KEY,
                 kind TEXT NOT NULL,
                 kind_priority INTEGER NOT NULL,
                 name_primary TEXT NOT NULL,
                 name_alias TEXT,
                 owner_primary TEXT,
                 owner_alias TEXT,
                 signature_text TEXT NOT NULL,
                 description TEXT,
                 availability_contexts TEXT NOT NULL,
                 available_since TEXT
             );
             CREATE TABLE type_identities (
                 type_id TEXT PRIMARY KEY,
                 document_id TEXT NOT NULL REFERENCES documents(id),
                 name_primary TEXT NOT NULL,
                 name_alias TEXT
             );
             CREATE TABLE type_templates (
                 document_id TEXT PRIMARY KEY REFERENCES documents(id),
                 metadata_kind TEXT NOT NULL,
                 template_parameters TEXT NOT NULL,
                 template_family TEXT,
                 template_variant TEXT,
                 template_classification_diagnostic TEXT
             );
             CREATE TABLE members (
                 member_id TEXT PRIMARY KEY,
                 owner_type_id TEXT NOT NULL REFERENCES type_identities(type_id),
                 member_kind TEXT NOT NULL,
                 name_primary TEXT NOT NULL,
                 name_alias TEXT,
                 document_id TEXT NOT NULL REFERENCES documents(id)
             );
             CREATE TABLE callables (
                 callable_id TEXT PRIMARY KEY,
                 document_id TEXT NOT NULL REFERENCES documents(id),
                 callable_kind TEXT NOT NULL,
                 owner_type_id TEXT REFERENCES type_identities(type_id)
             );
             CREATE TABLE signatures (
                 signature_id TEXT PRIMARY KEY,
                 callable_id TEXT NOT NULL REFERENCES callables(callable_id),
                 ordinal INTEGER NOT NULL,
                 title TEXT,
                 description TEXT
             );
             CREATE TABLE parameters (
                 parameter_id TEXT PRIMARY KEY,
                 signature_id TEXT NOT NULL REFERENCES signatures(signature_id),
                 ordinal INTEGER NOT NULL,
                 name TEXT NOT NULL,
                 required INTEGER NOT NULL,
                 description TEXT
             );
             CREATE TABLE type_refs (
                 source_document_id TEXT NOT NULL REFERENCES documents(id),
                 ref_kind TEXT NOT NULL,
                 ordinal INTEGER NOT NULL,
                 source_signature_id TEXT REFERENCES signatures(signature_id),
                 source_signature_ordinal INTEGER,
                 source_parameter_ordinal INTEGER,
                 target_type_name TEXT NOT NULL,
                 target_type_id TEXT REFERENCES type_identities(type_id),
                 type_template_family TEXT,
                 type_template_variant TEXT,
                 template_binding_kind TEXT,
                 template_binding_owner_parameter_index INTEGER,
                 template_binding_target_parameter_index INTEGER,
                 template_binding_arguments TEXT
             );
             CREATE TABLE document_names (
                 key TEXT NOT NULL,
                 key_kind TEXT NOT NULL,
                 document_id TEXT NOT NULL REFERENCES documents(id)
             );
             CREATE TABLE document_search (
                 rowid INTEGER PRIMARY KEY,
                 document_id TEXT NOT NULL REFERENCES documents(id),
                 name_primary TEXT NOT NULL,
                 name_alias TEXT,
                 owner TEXT,
                 signatures TEXT NOT NULL,
                 parameters TEXT NOT NULL,
                 type_names TEXT NOT NULL,
                 return_names TEXT NOT NULL,
                 description TEXT
             );
             CREATE VIRTUAL TABLE document_fts USING fts5(
                 document_id UNINDEXED,
                 name_primary,
                 name_alias,
                 owner,
                 signatures,
                 parameters,
                 type_names,
                 return_names,
                 description,
                 content = 'document_search',
                 content_rowid = 'rowid',
                 tokenize = 'unicode61 remove_diacritics 0'
             );
             CREATE TABLE relations (
                 source_id TEXT NOT NULL REFERENCES documents(id),
                 target_id TEXT NOT NULL REFERENCES documents(id),
                 edge_kind TEXT NOT NULL,
                 label TEXT NOT NULL,
                 evidence TEXT NOT NULL,
                 weight INTEGER NOT NULL
             );",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })
}

fn create_lookup_indexes(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    connection
        .execute_batch(
            "CREATE INDEX document_names_key_idx ON document_names(key, key_kind, document_id);
             CREATE INDEX documents_owner_member_idx ON documents(owner_primary, name_primary);
             CREATE INDEX type_identities_name_idx ON type_identities(name_primary, name_alias, type_id);
             CREATE INDEX type_identities_document_idx ON type_identities(document_id);
             CREATE INDEX type_templates_template_key_idx ON type_templates(
                template_family, template_variant, document_id
             );
             CREATE INDEX members_owner_idx ON members(owner_type_id, member_kind, name_primary, document_id);
             CREATE INDEX members_document_owner_idx ON members(document_id, owner_type_id);
             CREATE INDEX callables_document_idx ON callables(document_id, callable_kind);
             CREATE INDEX callables_document_owner_idx ON callables(document_id, owner_type_id);
             CREATE INDEX signatures_callable_idx ON signatures(callable_id, ordinal);
             CREATE INDEX parameters_signature_idx ON parameters(signature_id, ordinal);
             CREATE INDEX type_refs_source_idx ON type_refs(source_document_id, ref_kind, ordinal);
             CREATE INDEX type_refs_signature_idx ON type_refs(source_signature_id, source_parameter_ordinal, ordinal);
             CREATE INDEX type_refs_target_idx ON type_refs(target_type_id, ref_kind, source_document_id);
             CREATE INDEX relations_source_idx ON relations(source_id, edge_kind, target_id);
             CREATE INDEX relations_target_idx ON relations(target_id, edge_kind, source_id);",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })
}

fn write_metadata(
    connection: &Connection,
    path: &Path,
    metadata: &IndexMetadata,
) -> Result<(), SearchError> {
    let built_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());
    for (key, value) in [
        ("schema_version", INDEX_SCHEMA_VERSION.to_string()),
        ("locale", metadata.locale.clone()),
        ("source_locale", metadata.source_locale.clone()),
        ("source_hbk", metadata.source_hbk.clone()),
        (
            "source_extraction_schema_version",
            metadata.source_extraction_schema_version.to_string(),
        ),
        ("built_at", built_at),
        ("builder_version", env!("CARGO_PKG_VERSION").to_string()),
    ] {
        connection
            .execute(
                "INSERT INTO meta(key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}

fn insert_documents(
    connection: &Connection,
    path: &Path,
    documents: &[SearchDocument],
) -> Result<(), SearchError> {
    let mut document_statement = connection
        .prepare(
            "INSERT INTO documents(
                id, kind, kind_priority, name_primary, name_alias, owner_primary, owner_alias,
                signature_text, description, availability_contexts, available_since
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut name_statement = connection
        .prepare("INSERT INTO document_names(key, key_kind, document_id) VALUES (?1, ?2, ?3)")
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut fts_statement = connection
        .prepare(
            "INSERT INTO document_search(
                rowid,
                document_id, name_primary, name_alias, owner, signatures, parameters,
                type_names, return_names, description
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;

    for (index, document) in documents.iter().enumerate() {
        let rowid = (index + 1) as i64;
        let signatures = document.signature_text_lines().join("\n");
        let parameters = document.parameter_terms.join("\n");
        let type_names = document.type_refs.join("\n");
        let return_names = document.return_types.join("\n");
        let owner_primary = document.owner.as_ref().map(|owner| owner.primary.as_str());
        let owner_alias = document
            .owner
            .as_ref()
            .and_then(|owner| owner.alias.as_deref());
        document_statement
            .execute(params![
                document.id,
                document.kind.as_str(),
                kind_priority(document.kind),
                document.name.primary,
                document.name.alias,
                owner_primary,
                owner_alias,
                signatures,
                document.description,
                document.availability_contexts.join("\n"),
                document.available_since,
            ])
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
        insert_name_keys(&mut name_statement, path, document)?;
        fts_statement
            .execute(params![
                rowid,
                document.id,
                searchable_name(&document.name.primary),
                document.name.alias.as_deref().map(searchable_name),
                document
                    .owner
                    .as_ref()
                    .map(|owner| searchable_name(&owner.display_name())),
                searchable_text(&signatures),
                searchable_text(&parameters),
                searchable_text(&type_names),
                searchable_text(&return_names),
                document.description.as_deref().map(searchable_text),
            ])
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}

fn insert_normalized_facts(
    connection: &Connection,
    path: &Path,
    documents: &[SearchDocument],
) -> Result<(), SearchError> {
    let by_name = relation_lookup(documents);
    let mut type_id_by_key = BTreeMap::new();
    let mut type_id_by_normalized_id = BTreeMap::new();
    let mut type_id_by_metadata_kind = BTreeMap::new();
    let mut type_template_by_type_id = BTreeMap::new();
    for document in documents
        .iter()
        .filter(|document| document.kind == SearchDocumentKind::PlatformType)
    {
        insert_type_lookup_key(
            &mut type_id_by_key,
            normalize_lookup_key(&document.name.primary),
            &document.id,
        );
        if let Some(alias) = &document.name.alias {
            insert_type_lookup_key(
                &mut type_id_by_key,
                normalize_lookup_key(alias),
                &document.id,
            );
        }
        type_id_by_normalized_id.insert(normalize_lookup_key(&document.id), document.id.clone());
        if let Some(metadata_kind) = &document.metadata_kind {
            insert_type_lookup_key(
                &mut type_id_by_metadata_kind,
                normalize_lookup_key(metadata_kind),
                &document.id,
            );
        }
        if let Some(kind) = &document.type_template_key {
            type_template_by_type_id.insert(
                document.id.clone(),
                TypeTemplateFact {
                    key: kind.clone(),
                    parameters: document.template_parameters.clone(),
                },
            );
        }
    }

    let mut type_statement = connection
        .prepare(
            "INSERT INTO type_identities(type_id, document_id, name_primary, name_alias)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut type_template_statement = connection
        .prepare(
            "INSERT INTO type_templates(
                document_id, metadata_kind, template_parameters, template_family,
                template_variant, template_classification_diagnostic
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut member_statement = connection
        .prepare(
            "INSERT INTO members(member_id, owner_type_id, member_kind, name_primary, name_alias, document_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut callable_statement = connection
        .prepare(
            "INSERT INTO callables(callable_id, document_id, callable_kind, owner_type_id)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut signature_statement = connection
        .prepare(
            "INSERT INTO signatures(signature_id, callable_id, ordinal, title, description)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut parameter_statement = connection
        .prepare(
            "INSERT INTO parameters(parameter_id, signature_id, ordinal, name, required, description)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    let mut type_ref_statement = connection
        .prepare(
            "INSERT INTO type_refs(
                source_document_id, ref_kind, ordinal, source_signature_id,
                source_signature_ordinal, source_parameter_ordinal, target_type_name,
                target_type_id, type_template_family, type_template_variant,
                template_binding_kind, template_binding_owner_parameter_index,
                template_binding_target_parameter_index, template_binding_arguments
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;

    for document in documents
        .iter()
        .filter(|document| document.kind == SearchDocumentKind::PlatformType)
    {
        type_statement
            .execute(params![
                document.id,
                document.id,
                document.name.primary,
                document.name.alias,
            ])
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
        if let Some(metadata_kind) = &document.metadata_kind {
            let type_template_key = document.type_template_key.as_ref();
            type_template_statement
                .execute(params![
                    document.id,
                    metadata_kind,
                    document.template_parameters.join("\n"),
                    type_template_key.map(|kind| kind.family.as_str()),
                    type_template_key.map(|kind| kind.variant.as_str()),
                    document.type_template_classification_diagnostic.as_deref(),
                ])
                .map_err(|source| SearchError::Sqlite {
                    path: path.to_path_buf(),
                    source,
                })?;
        }
    }

    for document in documents {
        let owner_type_id = owner_type_id(document, &by_name, &type_id_by_normalized_id);

        if let Some(owner_type_id) = owner_type_id.as_deref()
            && matches!(
                document.kind,
                SearchDocumentKind::TypeMethod
                    | SearchDocumentKind::TypeProperty
                    | SearchDocumentKind::Constructor
                    | SearchDocumentKind::TypeEvent
            )
        {
            member_statement
                .execute(params![
                    document.id,
                    owner_type_id,
                    document.kind.as_str(),
                    document.name.primary,
                    document.name.alias,
                    document.id,
                ])
                .map_err(|source| SearchError::Sqlite {
                    path: path.to_path_buf(),
                    source,
                })?;
        }

        if document.kind.is_callable() {
            callable_statement
                .execute(params![
                    document.id,
                    document.id,
                    document.kind.as_str(),
                    owner_type_id
                ])
                .map_err(|source| SearchError::Sqlite {
                    path: path.to_path_buf(),
                    source,
                })?;
            for (signature_ordinal, signature) in document.signatures.iter().enumerate() {
                let signature_id = signature_id(&document.id, signature_ordinal);
                signature_statement
                    .execute(params![
                        signature_id,
                        document.id,
                        signature_ordinal as i64,
                        signature.title,
                        signature.description,
                    ])
                    .map_err(|source| SearchError::Sqlite {
                        path: path.to_path_buf(),
                        source,
                    })?;
                for (parameter_ordinal, parameter) in signature.parameters.iter().enumerate() {
                    let parameter_id = parameter_id(&signature_id, parameter_ordinal);
                    parameter_statement
                        .execute(params![
                            parameter_id,
                            signature_id,
                            parameter_ordinal as i64,
                            parameter.name,
                            parameter.required,
                            parameter.description,
                        ])
                        .map_err(|source| SearchError::Sqlite {
                            path: path.to_path_buf(),
                            source,
                        })?;
                    for (type_ordinal, type_name) in parameter.type_refs.iter().enumerate() {
                        insert_type_ref(
                            &mut type_ref_statement,
                            path,
                            &type_id_by_key,
                            &type_id_by_metadata_kind,
                            &type_template_by_type_id,
                            TypeRefRow {
                                source_document_id: &document.id,
                                ref_kind: "parameter_type",
                                ordinal: type_ordinal,
                                owner_type_id: owner_type_id.as_deref(),
                                source_signature_id: Some(&signature_id),
                                source_signature_ordinal: Some(signature_ordinal),
                                source_parameter_ordinal: Some(parameter_ordinal),
                                target_type_name: type_name,
                            },
                        )?;
                    }
                }
            }
        }

        for (ordinal, type_name) in document.type_refs.iter().enumerate() {
            insert_type_ref(
                &mut type_ref_statement,
                path,
                &type_id_by_key,
                &type_id_by_metadata_kind,
                &type_template_by_type_id,
                TypeRefRow {
                    source_document_id: &document.id,
                    ref_kind: document.kind.type_ref_kind(),
                    ordinal,
                    owner_type_id: owner_type_id.as_deref(),
                    source_signature_id: None,
                    source_signature_ordinal: None,
                    source_parameter_ordinal: None,
                    target_type_name: type_name,
                },
            )?;
        }
        for (ordinal, type_name) in document.return_types.iter().enumerate() {
            insert_type_ref(
                &mut type_ref_statement,
                path,
                &type_id_by_key,
                &type_id_by_metadata_kind,
                &type_template_by_type_id,
                TypeRefRow {
                    source_document_id: &document.id,
                    ref_kind: "return_type",
                    ordinal,
                    owner_type_id: owner_type_id.as_deref(),
                    source_signature_id: None,
                    source_signature_ordinal: None,
                    source_parameter_ordinal: None,
                    target_type_name: type_name,
                },
            )?;
        }
        if document.kind == SearchDocumentKind::Constructor
            && let (Some(owner), Some(owner_type_id)) = (&document.owner, owner_type_id.as_deref())
        {
            insert_type_ref(
                &mut type_ref_statement,
                path,
                &type_id_by_key,
                &type_id_by_metadata_kind,
                &type_template_by_type_id,
                TypeRefRow {
                    source_document_id: &document.id,
                    ref_kind: "constructor_result",
                    ordinal: 0,
                    owner_type_id: Some(owner_type_id),
                    source_signature_id: None,
                    source_signature_ordinal: None,
                    source_parameter_ordinal: None,
                    target_type_name: &owner.primary,
                },
            )?;
            connection
                .execute(
                    "UPDATE type_refs
                     SET target_type_id = ?1
                     WHERE source_document_id = ?2 AND ref_kind = 'constructor_result'",
                    params![owner_type_id, document.id],
                )
                .map_err(|source| SearchError::Sqlite {
                    path: path.to_path_buf(),
                    source,
                })?;
        }
    }
    Ok(())
}

struct TypeRefRow<'a> {
    source_document_id: &'a str,
    ref_kind: &'a str,
    ordinal: usize,
    owner_type_id: Option<&'a str>,
    source_signature_id: Option<&'a str>,
    source_signature_ordinal: Option<usize>,
    source_parameter_ordinal: Option<usize>,
    target_type_name: &'a str,
}

fn insert_type_ref(
    statement: &mut Statement<'_>,
    path: &Path,
    type_id_by_key: &BTreeMap<String, Option<String>>,
    type_id_by_metadata_kind: &BTreeMap<String, Option<String>>,
    type_template_by_type_id: &BTreeMap<String, TypeTemplateFact>,
    row: TypeRefRow<'_>,
) -> Result<(), SearchError> {
    let mut target_type_id = type_id_by_key
        .get(&normalize_lookup_key(row.target_type_name))
        .and_then(|type_id| type_id.as_deref());
    if target_type_id.is_none() {
        target_type_id = type_id_by_metadata_kind
            .get(&normalize_lookup_key(row.target_type_name))
            .and_then(|type_id| type_id.as_deref());
    }
    let type_template_key =
        target_type_id.and_then(|type_id| type_template_by_type_id.get(type_id));
    let template_binding =
        type_template_binding(row.owner_type_id, target_type_id, type_template_by_type_id);
    statement
        .execute(params![
            row.source_document_id,
            row.ref_kind,
            row.ordinal as i64,
            row.source_signature_id,
            row.source_signature_ordinal.map(|value| value as i64),
            row.source_parameter_ordinal.map(|value| value as i64),
            row.target_type_name,
            target_type_id,
            type_template_key.map(|fact| fact.key.family.as_str()),
            type_template_key.map(|fact| fact.key.variant.as_str()),
            template_binding.as_ref().map(|_| "owner_parameter"),
            template_binding
                .as_ref()
                .and_then(|binding| binding.arguments.first())
                .map(|argument| argument.owner_parameter_index as i64),
            template_binding
                .as_ref()
                .and_then(|binding| binding.arguments.first())
                .map(|argument| argument.target_parameter_index as i64),
            template_binding.as_ref().map(binding_arguments_to_storage),
        ])
        .map(|_| ())
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })
}

#[derive(Debug, Clone)]
struct OwnerParameterBinding {
    arguments: Vec<OwnerParameterBindingArgument>,
}

#[derive(Debug, Clone, Copy)]
struct OwnerParameterBindingArgument {
    owner_parameter_index: usize,
    target_parameter_index: usize,
}

fn type_template_binding(
    owner_type_id: Option<&str>,
    target_type_id: Option<&str>,
    type_template_by_type_id: &BTreeMap<String, TypeTemplateFact>,
) -> Option<OwnerParameterBinding> {
    let owner = type_template_by_type_id.get(owner_type_id?)?;
    let target = type_template_by_type_id.get(target_type_id?)?;
    if owner.key.family != target.key.family {
        return None;
    }
    let mut owner_parameters = BTreeMap::<&str, usize>::new();
    for (index, parameter) in owner.parameters.iter().enumerate() {
        if owner_parameters.insert(parameter.as_str(), index).is_some() {
            return None;
        }
    }
    let mut arguments = Vec::new();
    for (target_parameter_index, parameter) in target.parameters.iter().enumerate() {
        let owner_parameter_index = *owner_parameters.get(parameter.as_str())?;
        arguments.push(OwnerParameterBindingArgument {
            owner_parameter_index,
            target_parameter_index,
        });
    }
    (!arguments.is_empty()).then_some(OwnerParameterBinding { arguments })
}

fn binding_arguments_to_storage(binding: &OwnerParameterBinding) -> String {
    binding
        .arguments
        .iter()
        .map(|argument| {
            format!(
                "{}:{}",
                argument.owner_parameter_index, argument.target_parameter_index
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn insert_type_lookup_key(
    type_id_by_key: &mut BTreeMap<String, Option<String>>,
    key: String,
    type_id: &str,
) {
    match type_id_by_key.get_mut(&key) {
        Some(existing) if existing.as_deref() == Some(type_id) => {}
        Some(existing) => {
            *existing = None;
        }
        None => {
            type_id_by_key.insert(key, Some(type_id.to_string()));
        }
    }
}

fn owner_type_id(
    document: &SearchDocument,
    by_name: &BTreeMap<String, (&SearchDocument, String)>,
    type_id_by_normalized_id: &BTreeMap<String, String>,
) -> Option<String> {
    document
        .owner_relation_key
        .as_deref()
        .and_then(|key| by_name.get(key))
        .map(|(_, id)| id)
        .and_then(|id| type_id_by_normalized_id.get(&normalize_lookup_key(id)))
        .cloned()
}

fn edge_ref_kind(edge_kind: &str) -> &'static str {
    match edge_kind {
        "returns" => "return_type",
        "constructs" => "constructor_result",
        _ => "property_type",
    }
}

fn signature_id(document_id: &str, ordinal: usize) -> String {
    format!("{document_id}:signature:{ordinal}")
}

fn parameter_id(signature_id: &str, ordinal: usize) -> String {
    format!("{signature_id}:parameter:{ordinal}")
}

fn rebuild_document_fts(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    connection
        .execute(
            "INSERT INTO document_fts(document_fts) VALUES ('rebuild')",
            [],
        )
        .map(|_| ())
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })
}

fn insert_name_keys(
    statement: &mut Statement<'_>,
    path: &Path,
    document: &SearchDocument,
) -> Result<(), SearchError> {
    let mut keys = BTreeSet::new();
    keys.insert((normalize_lookup_key(&document.name.primary), "primary"));
    if let Some(alias) = &document.name.alias {
        keys.insert((normalize_lookup_key(alias), "alias"));
    }
    if let Some(owner) = &document.owner {
        keys.insert((
            owner_member_key(&owner.primary, &document.name.primary),
            "owner_member_primary",
        ));
        if let Some(owner_alias) = &owner.alias {
            keys.insert((
                owner_member_key(owner_alias, &document.name.primary),
                "owner_member_alias",
            ));
        }
        if let Some(name_alias) = &document.name.alias {
            keys.insert((
                owner_member_key(&owner.primary, name_alias),
                "owner_member_alias",
            ));
            if let Some(owner_alias) = &owner.alias {
                keys.insert((
                    owner_member_key(owner_alias, name_alias),
                    "owner_member_alias",
                ));
            }
        }
    }
    for (key, kind) in keys {
        statement
            .execute(params![key, kind, document.id])
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}

fn insert_relations_from_documents(
    connection: &Connection,
    path: &Path,
    documents: &[SearchDocument],
) -> Result<(), SearchError> {
    let mut statement = connection
        .prepare(
            "INSERT INTO relations(source_id, target_id, edge_kind, label, evidence, weight)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    visit_relations_from_documents(documents, |relation| {
        statement
            .execute(params![
                relation.source_id,
                relation.target_id,
                relation.edge_kind,
                relation.label,
                relation.evidence,
                relation.weight,
            ])
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
        Ok(())
    })
}

fn visit_relations_from_documents<E>(
    documents: &[SearchDocument],
    mut visit: impl FnMut(Relation) -> Result<(), E>,
) -> Result<(), E> {
    let by_name = relation_lookup(documents);
    let mut emitted = BTreeSet::new();
    for document in documents {
        visit_document_relations(document, &by_name, |relation| {
            if emitted.insert((
                relation.source_id.clone(),
                relation.target_id.clone(),
                relation.edge_kind,
            )) {
                visit(relation)?;
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn visit_document_relations<E>(
    document: &SearchDocument,
    by_name: &BTreeMap<String, (&SearchDocument, String)>,
    mut visit: impl FnMut(Relation) -> Result<(), E>,
) -> Result<(), E> {
    visit_owner_relations(document, by_name, &mut visit)?;
    visit_constructor_relation(document, by_name, &mut visit)?;
    visit_type_reference_relations(document, by_name, &mut visit)
}

fn visit_owner_relations<E>(
    document: &SearchDocument,
    by_name: &BTreeMap<String, (&SearchDocument, String)>,
    visit: &mut impl FnMut(Relation) -> Result<(), E>,
) -> Result<(), E> {
    let Some(owner) = &document.owner else {
        return Ok(());
    };
    let owner_key = document
        .owner_relation_key
        .as_deref()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| normalize_lookup_key(&owner.primary));
    let Some((_, owner_id)) = by_name.get(&owner_key) else {
        return Ok(());
    };
    visit(Relation {
        source_id: owner_id.clone(),
        target_id: document.id.clone(),
        edge_kind: "owns",
        label: format!("{} owns {}", owner.display_name(), document.name.primary),
        evidence: "owner",
        weight: 10,
    })?;
    visit(Relation {
        source_id: document.id.clone(),
        target_id: owner_id.clone(),
        edge_kind: "member_of",
        label: format!(
            "{} member of {}",
            document.name.primary,
            owner.display_name()
        ),
        evidence: "owner",
        weight: 20,
    })
}

fn visit_constructor_relation<E>(
    document: &SearchDocument,
    by_name: &BTreeMap<String, (&SearchDocument, String)>,
    visit: &mut impl FnMut(Relation) -> Result<(), E>,
) -> Result<(), E> {
    if document.kind != SearchDocumentKind::Constructor {
        return Ok(());
    }
    let (Some(owner), Some(owner_key)) = (&document.owner, document.owner_relation_key.as_deref())
    else {
        return Ok(());
    };
    let Some((_, owner_id)) = by_name.get(owner_key) else {
        return Ok(());
    };
    visit(Relation {
        source_id: document.id.clone(),
        target_id: owner_id.clone(),
        edge_kind: "constructs",
        label: format!("constructs {}", owner.display_name()),
        evidence: "structured",
        weight: 15,
    })
}

fn visit_type_reference_relations<E>(
    document: &SearchDocument,
    by_name: &BTreeMap<String, (&SearchDocument, String)>,
    visit: &mut impl FnMut(Relation) -> Result<(), E>,
) -> Result<(), E> {
    for (ordinal, type_name) in document.type_refs.iter().enumerate() {
        let Some(target_key) = explicit_or_fallback_type_ref_key(
            document,
            &document.explicit_type_ref_ids,
            ordinal,
            type_name,
        ) else {
            continue;
        };
        let Some((_, target_id)) = by_name.get(&target_key) else {
            continue;
        };
        visit(Relation {
            source_id: document.id.clone(),
            target_id: target_id.clone(),
            edge_kind: "has_type",
            label: type_name.clone(),
            evidence: "type_ref",
            weight: TYPE_REFERENCE_RELATION_WEIGHT,
        })?;
    }
    for (ordinal, type_name) in document.return_types.iter().enumerate() {
        let Some(target_key) = explicit_or_fallback_type_ref_key(
            document,
            &document.explicit_return_type_ref_ids,
            ordinal,
            type_name,
        ) else {
            continue;
        };
        let Some((_, target_id)) = by_name.get(&target_key) else {
            continue;
        };
        visit(Relation {
            source_id: document.id.clone(),
            target_id: target_id.clone(),
            edge_kind: "returns",
            label: type_name.clone(),
            evidence: "type_ref",
            weight: TYPE_REFERENCE_RELATION_WEIGHT,
        })?;
    }
    Ok(())
}

fn explicit_or_fallback_type_ref_key(
    document: &SearchDocument,
    explicit_ids: &[Option<String>],
    ordinal: usize,
    type_name: &str,
) -> Option<String> {
    explicit_ids
        .get(ordinal)
        .and_then(|id| id.clone())
        .or_else(|| (!document.kind.is_language()).then(|| normalize_lookup_key(type_name)))
}

fn validate_index(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    let count: i64 = connection
        .query_row("SELECT count(*) FROM documents", [], |row| row.get(0))
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    if count == 0 {
        return Err(SearchError::Sqlite {
            path: path.to_path_buf(),
            source: rusqlite::Error::QueryReturnedNoRows,
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn document(
    kind: SearchDocumentKind,
    owner: Option<&model::LocalizedName>,
    name: &model::LocalizedName,
    signatures: &[model::Signature],
    return_types: &[model::TypeRef],
    type_refs: &[model::TypeRef],
    description: Option<&str>,
    id: String,
) -> SearchDocument {
    let parameter_terms = signatures
        .iter()
        .flat_map(|signature| signature.parameters.iter())
        .flat_map(|parameter| {
            std::iter::once(parameter.name.clone()).chain(
                parameter
                    .type_refs
                    .iter()
                    .map(|type_ref| type_ref.name.clone()),
            )
        })
        .collect::<Vec<_>>();
    let signatures = signatures
        .iter()
        .map(SearchSignature::from)
        .collect::<Vec<_>>();
    let return_types = return_types
        .iter()
        .map(|type_ref| type_ref.name.clone())
        .collect::<Vec<_>>();
    let type_refs = type_refs
        .iter()
        .map(|type_ref| type_ref.name.clone())
        .collect::<Vec<_>>();
    SearchDocument {
        id,
        kind,
        name: name.clone(),
        owner: owner.cloned(),
        signatures,
        type_refs,
        return_types,
        type_ref_facts: Vec::new(),
        return_type_facts: Vec::new(),
        description: description.map(ToOwned::to_owned),
        preview: description
            .map(|value| value.chars().take(180).collect())
            .unwrap_or_default(),
        parameter_terms,
        relation_keys: Vec::new(),
        owner_relation_key: None,
        explicit_type_ref_ids: Vec::new(),
        explicit_return_type_ref_ids: Vec::new(),
        availability_contexts: Vec::new(),
        available_since: None,
        metadata_kind: None,
        template_parameters: Vec::new(),
        type_template_key: None,
        type_template_classification_diagnostic: None,
    }
}

impl From<&model::Signature> for SearchSignature {
    fn from(signature: &model::Signature) -> Self {
        Self {
            text: signature.text.clone(),
            parameters: signature
                .parameters
                .iter()
                .map(SearchParameter::from)
                .collect(),
            title: signature
                .variant
                .as_ref()
                .map(|variant| variant.title.clone())
                .filter(|title| !title.is_empty()),
            description: signature
                .variant
                .as_ref()
                .and_then(|variant| variant.description.clone()),
        }
    }
}

impl From<&model::Parameter> for SearchParameter {
    fn from(parameter: &model::Parameter) -> Self {
        Self {
            name: parameter.name.clone(),
            required: parameter.required,
            type_refs: parameter
                .type_refs
                .iter()
                .map(|type_ref| type_ref.name.clone())
                .collect(),
            type_ref_facts: Vec::new(),
            description: parameter.description.clone(),
        }
    }
}

fn language_document(fact: &language::LanguageFact) -> SearchDocument {
    let mut signatures = fact
        .signatures
        .iter()
        .map(|signature| SearchSignature {
            text: signature.text.clone(),
            parameters: signature
                .parameters
                .iter()
                .map(|parameter| SearchParameter {
                    name: parameter.name.clone(),
                    required: parameter.required,
                    type_refs: parameter.type_refs.clone(),
                    type_ref_facts: Vec::new(),
                    description: parameter.description.clone(),
                })
                .collect(),
            title: None,
            description: None,
        })
        .collect::<Vec<_>>();
    if signatures.is_empty()
        && let Some(syntax) = &fact.syntax
        && !syntax.is_empty()
    {
        signatures.push(SearchSignature {
            text: syntax.clone(),
            parameters: Vec::new(),
            title: None,
            description: None,
        });
    }
    let parameter_terms = fact
        .signatures
        .iter()
        .flat_map(|signature| signature.parameters.iter())
        .flat_map(|parameter| {
            std::iter::once(parameter.name.clone()).chain(parameter.type_refs.iter().cloned())
        })
        .chain(fact.type_refs.iter().cloned())
        .chain(fact.return_types.iter().cloned())
        .collect::<Vec<_>>();
    let type_refs = fact
        .type_refs
        .iter()
        .cloned()
        .chain(
            fact.signatures
                .iter()
                .flat_map(|signature| signature.parameters.iter())
                .flat_map(|parameter| parameter.type_refs.iter().cloned()),
        )
        .collect::<Vec<_>>();
    let explicit_type_ref_ids = explicit_language_type_ref_ids(fact, &type_refs);
    let explicit_return_type_ref_ids = explicit_language_type_ref_ids(fact, &fact.return_types);
    SearchDocument {
        id: fact.id.clone(),
        kind: language_document_kind(fact.family),
        name: fact.name.clone(),
        owner: None,
        signatures,
        type_refs,
        return_types: fact.return_types.clone(),
        type_ref_facts: Vec::new(),
        return_type_facts: Vec::new(),
        description: fact.description.clone(),
        preview: fact
            .description
            .as_deref()
            .map(|value| value.chars().take(180).collect())
            .unwrap_or_default(),
        parameter_terms,
        relation_keys: vec![fact.id.clone()],
        owner_relation_key: None,
        explicit_type_ref_ids,
        explicit_return_type_ref_ids,
        availability_contexts: Vec::new(),
        available_since: None,
        metadata_kind: None,
        template_parameters: Vec::new(),
        type_template_key: None,
        type_template_classification_diagnostic: None,
    }
}

fn language_document_kind(family: language::LanguageFactFamily) -> SearchDocumentKind {
    match family {
        language::LanguageFactFamily::Construct => SearchDocumentKind::LanguageConstruct,
        language::LanguageFactFamily::Type => SearchDocumentKind::LanguageType,
        language::LanguageFactFamily::Function => SearchDocumentKind::LanguageFunction,
        language::LanguageFactFamily::Operator => SearchDocumentKind::LanguageOperator,
        language::LanguageFactFamily::Keyword => SearchDocumentKind::LanguageKeyword,
        language::LanguageFactFamily::Literal => SearchDocumentKind::LanguageLiteral,
    }
}

fn explicit_language_type_ref_ids(
    fact: &language::LanguageFact,
    names: &[String],
) -> Vec<Option<String>> {
    names
        .iter()
        .map(|name| explicit_language_type_ref_id(fact.source_family, name))
        .collect()
}

fn explicit_language_type_ref_id(
    source_family: language::LanguageSourceFamily,
    name: &str,
) -> Option<String> {
    let normalized = normalize_lookup_key(name);
    match source_family {
        language::LanguageSourceFamily::Shlang => None,
        language::LanguageSourceFamily::Shquery
            if matches!(normalized.as_str(), "строка" | "string") =>
        {
            Some("shquery:LitString".to_string())
        }
        language::LanguageSourceFamily::Dcsui
            if matches!(normalized.as_str(), "строка" | "string") =>
        {
            Some("shlang:def_String".to_string())
        }
        _ => None,
    }
}

fn type_ref_from_name(name: &model::LocalizedName) -> model::TypeRef {
    model::TypeRef {
        name: name.display_name(),
    }
}

fn event_owner(event: &model::GlobalContextEvent) -> Option<model::LocalizedName> {
    if let Some(owner) = event.module.owner_path.last().cloned() {
        return Some(owner);
    }
    event.semantic.type_event_owner()
}

fn semantic_relation_key(semantic: &model::SemanticContext, fallback: &str) -> String {
    let mut parts = semantic
        .owner_path
        .iter()
        .map(|name| name.primary.as_str())
        .collect::<Vec<_>>();
    if parts
        .last()
        .is_none_or(|last| normalize_lookup_key(last) != normalize_lookup_key(fallback))
    {
        parts.push(fallback);
    }
    normalize_lookup_key(&parts.join("."))
}

#[cfg(test)]
fn relations_from_documents(documents: &[SearchDocument]) -> Vec<Relation> {
    let mut relations = Vec::new();
    visit_relations_from_documents(documents, |relation| {
        relations.push(relation);
        Ok::<_, std::convert::Infallible>(())
    })
    .expect("infallible relation collection must not fail");
    relations.sort_by(|left, right| {
        left.weight
            .cmp(&right.weight)
            .then_with(|| left.source_id.cmp(&right.source_id))
            .then_with(|| left.edge_kind.cmp(right.edge_kind))
            .then_with(|| left.target_id.cmp(&right.target_id))
    });
    relations.dedup_by(|left, right| {
        left.source_id == right.source_id
            && left.target_id == right.target_id
            && left.edge_kind == right.edge_kind
    });
    relations
}

fn relation_lookup(documents: &[SearchDocument]) -> BTreeMap<String, (&SearchDocument, String)> {
    let mut by_name = BTreeMap::<String, (&SearchDocument, String)>::new();
    for document in documents {
        for key in document_lookup_keys(document) {
            match by_name.get(&key) {
                Some((existing, _))
                    if kind_priority(existing.kind) <= kind_priority(document.kind) => {}
                _ => {
                    by_name.insert(key, (document, document.id.clone()));
                }
            }
        }
    }
    by_name
}

fn document_lookup_keys(document: &SearchDocument) -> Vec<String> {
    let mut keys = vec![normalize_lookup_key(&document.name.primary)];
    if let Some(alias) = &document.name.alias {
        keys.push(normalize_lookup_key(alias));
    }
    keys.extend(document.relation_keys.iter().cloned());
    keys
}

fn document_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchDocument> {
    let description: Option<String> = row.get(7)?;
    let kind_text: String = row.get(1)?;
    let kind = SearchDocumentKind::from_storage(&kind_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            Box::new(UnknownSearchDocumentKind(kind_text)),
        )
    })?;
    Ok(SearchDocument {
        id: row.get(0)?,
        kind,
        name: model::LocalizedName {
            primary: row.get(2)?,
            alias: row.get(3)?,
        },
        owner: optional_localized_name(row.get(4)?, row.get(5)?),
        signatures: split_lines(row.get(6)?)
            .into_iter()
            .map(|text| SearchSignature {
                text,
                parameters: Vec::new(),
                title: None,
                description: None,
            })
            .collect(),
        parameter_terms: Vec::new(),
        type_refs: Vec::new(),
        return_types: Vec::new(),
        type_ref_facts: Vec::new(),
        return_type_facts: Vec::new(),
        preview: description
            .as_deref()
            .map(|value| value.chars().take(180).collect())
            .unwrap_or_default(),
        description,
        relation_keys: Vec::new(),
        owner_relation_key: None,
        explicit_type_ref_ids: Vec::new(),
        explicit_return_type_ref_ids: Vec::new(),
        availability_contexts: split_lines(row.get(8)?),
        available_since: row.get(9)?,
        metadata_kind: None,
        template_parameters: Vec::new(),
        type_template_key: None,
        type_template_classification_diagnostic: None,
    })
}

fn search_type_ref_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchTypeRef> {
    let type_template_key = type_template_key_from_codes(
        row.get::<_, Option<String>>(2)?,
        row.get::<_, Option<String>>(3)?,
    );
    let binding_kind: Option<String> = row.get(4)?;
    let owner_parameter_index: Option<i64> = row.get(5)?;
    let target_parameter_index: Option<i64> = row.get(6)?;
    let binding_arguments: Option<String> = row.get(7)?;
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
    Ok(SearchTypeRef {
        name: row.get(0)?,
        target_type_id: row.get(1)?,
        type_template_key,
        template_binding,
    })
}

fn parse_binding_arguments(value: &str) -> Vec<model::TemplateParameterBinding> {
    value
        .lines()
        .filter_map(|line| {
            let (owner, target) = line.split_once(':')?;
            Some(model::TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: owner.parse().ok()?,
                target_parameter_index: target.parse().ok()?,
            })
        })
        .collect()
}

fn availability_context_code(context: model::AvailabilityContext) -> &'static str {
    match context {
        model::AvailabilityContext::ThinClient => "thin_client",
        model::AvailabilityContext::WebClient => "web_client",
        model::AvailabilityContext::MobileClient => "mobile_client",
        model::AvailabilityContext::Server => "server",
        model::AvailabilityContext::ThickClient => "thick_client",
        model::AvailabilityContext::ExternalConnection => "external_connection",
        model::AvailabilityContext::MobileApplicationClient => "mobile_application_client",
        model::AvailabilityContext::MobileApplicationServer => "mobile_application_server",
        model::AvailabilityContext::MobileStandaloneServer => "mobile_standalone_server",
    }
}

fn type_template_key_from_codes(
    family: Option<String>,
    variant: Option<String>,
) -> Option<model::PlatformTypeTemplateKey> {
    Some(model::PlatformTypeTemplateKey::new(family?, variant?))
}

fn optional_localized_name(
    primary: Option<String>,
    alias: Option<String>,
) -> Option<model::LocalizedName> {
    primary.map(|primary| model::LocalizedName { primary, alias })
}

fn split_lines(value: String) -> Vec<String> {
    value
        .lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

struct DocumentIdentities {
    platform_type_ids: BTreeMap<String, String>,
    query_table_ids: BTreeMap<String, String>,
    enum_ids: BTreeMap<String, String>,
}

impl DocumentIdentities {
    fn from_inputs(
        platform_types: &[PlatformTypeIdentityInput],
        query_tables: &[QueryTableIdentityInput],
        enums: &[EnumIdentityInput],
    ) -> Self {
        let platform_type_counts = model::count_identity_keys(
            platform_types
                .iter()
                .map(|record| model::platform_type_identity_key(&record.name_primary)),
        );
        let query_table_counts = model::count_identity_keys(query_tables.iter().map(|record| {
            model::query_table_identity_key(
                &record.name_primary,
                record.identifier.as_deref(),
                &record.semantic,
            )
        }));
        let enum_counts = model::count_identity_keys(enums.iter().map(|record| {
            model::enum_identity_key(&record.name_primary, &record.source_html_path)
        }));
        let platform_type_ids = platform_types
            .iter()
            .map(|record| {
                (
                    model::platform_type_semantic_key(&record.name_primary, &record.semantic),
                    record.identity.clone().unwrap_or_else(|| {
                        model::platform_type_identity(
                            &record.name_primary,
                            &record.semantic,
                            platform_type_counts
                                .get(&model::platform_type_identity_key(&record.name_primary))
                                .copied()
                                .unwrap_or_default(),
                        )
                    }),
                )
            })
            .collect();
        let query_table_ids = query_tables
            .iter()
            .map(|record| {
                (
                    model::query_table_semantic_key(&record.semantic, &record.name_primary),
                    record.identity.clone().unwrap_or_else(|| {
                        model::query_table_identity(
                            &record.name_primary,
                            record.identifier.as_deref(),
                            &record.semantic,
                            query_table_counts
                                .get(&model::query_table_identity_key(
                                    &record.name_primary,
                                    record.identifier.as_deref(),
                                    &record.semantic,
                                ))
                                .copied()
                                .unwrap_or_default(),
                        )
                    }),
                )
            })
            .collect();
        let enum_ids = enums
            .iter()
            .map(|record| {
                (
                    enum_record_key(&record.name_primary, &record.source_html_path),
                    record.identity.clone().unwrap_or_else(|| {
                        model::enum_identity(
                            &record.name_primary,
                            record.name_alias.as_deref(),
                            &record.source_html_path,
                            enum_counts
                                .get(&model::enum_identity_key(
                                    &record.name_primary,
                                    &record.source_html_path,
                                ))
                                .copied()
                                .unwrap_or_default(),
                        )
                    }),
                )
            })
            .collect();

        Self {
            platform_type_ids,
            query_table_ids,
            enum_ids,
        }
    }

    fn platform_type_identity_by(
        &self,
        name_primary: &str,
        semantic: &model::SemanticContext,
    ) -> String {
        self.platform_type_ids
            .get(&model::platform_type_semantic_key(name_primary, semantic))
            .cloned()
            .unwrap_or_else(|| {
                format!("platform_type:{}", model::clean_identity_part(name_primary))
            })
    }

    fn query_table_identity_by(
        &self,
        name_primary: &str,
        identifier: Option<&str>,
        semantic: &model::SemanticContext,
    ) -> String {
        self.query_table_ids
            .get(&model::query_table_semantic_key(semantic, name_primary))
            .cloned()
            .unwrap_or_else(|| model::query_table_identity(name_primary, identifier, semantic, 1))
    }

    fn enum_identity_by(
        &self,
        name_primary: &str,
        name_alias: Option<&str>,
        source_html_path: &str,
    ) -> String {
        self.enum_ids
            .get(&enum_record_key(name_primary, source_html_path))
            .cloned()
            .unwrap_or_else(|| model::enum_identity(name_primary, name_alias, source_html_path, 1))
    }
}

fn document_identity(
    kind: &str,
    owner: Option<&model::LocalizedName>,
    name: &model::LocalizedName,
) -> String {
    match owner {
        Some(owner) => owned_document_identity(
            kind,
            &format!("owner:{}", model::clean_identity_part(&owner.primary)),
            &name.primary,
        ),
        None => format!("{kind}:{}", model::clean_identity_part(&name.primary)),
    }
}

fn owned_document_identity(kind: &str, owner_identity: &str, name: &str) -> String {
    format!(
        "{kind}:{owner_identity}:{}",
        model::clean_identity_part(name)
    )
}

fn identity_relation_key(identity: &str) -> String {
    format!("id:{}", normalize_lookup_key(identity))
}

fn enum_record_key(name_primary: &str, source_html_path: &str) -> String {
    format!(
        "{}:{}",
        model::enum_identity_key(name_primary, source_html_path),
        source_html_path
    )
}

fn kind_priority(kind: SearchDocumentKind) -> i64 {
    kind.priority()
}

fn owner_member_key(owner: &str, member: &str) -> String {
    normalize_lookup_key(&format!("{owner}.{member}"))
}

fn normalize_lookup_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn searchable_name(value: &str) -> String {
    let mut output = String::new();
    let mut previous_lowercase = false;
    for character in value.chars() {
        if character.is_alphanumeric() {
            if previous_lowercase && character.is_uppercase() {
                output.push(' ');
            }
            output.extend(character.to_lowercase());
            previous_lowercase = character.is_lowercase();
        } else {
            output.push(' ');
            previous_lowercase = false;
        }
    }
    output
}

fn searchable_text(value: &str) -> String {
    value
        .split_whitespace()
        .map(searchable_name)
        .collect::<Vec<_>>()
        .join(" ")
}

fn fts_query(value: &str) -> String {
    let mut tokens = searchable_text(value)
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if tokens.iter().any(|token| token == "скд") {
        tokens.extend(
            [
                "система",
                "компоновки",
                "данных",
                "компоновка",
                "data",
                "composition",
            ]
            .into_iter()
            .map(ToOwned::to_owned),
        );
    }
    tokens.sort();
    tokens.dedup();
    tokens
        .into_iter()
        .filter(|token| !token.is_empty())
        .map(|token| format!("\"{}\"*", token.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn fuzzy_score(query: &str, document: &SearchDocument) -> usize {
    let mut candidates = vec![normalize_lookup_key(&document.name.primary)];
    if let Some(alias) = &document.name.alias {
        candidates.push(normalize_lookup_key(alias));
    }
    if let Some(owner) = &document.owner {
        candidates.push(owner_member_key(&owner.primary, &document.name.primary));
        if let Some(alias) = &document.name.alias {
            candidates.push(owner_member_key(&owner.primary, alias));
        }
    }
    candidates
        .iter()
        .map(|candidate| levenshtein(query, candidate))
        .min()
        .unwrap_or(usize::MAX)
}

fn fuzzy_threshold(query: &str) -> usize {
    match query.chars().count() {
        0..=8 => 2,
        9..=20 => 3,
        length => (length / 5).max(4),
    }
}

fn keyword_order(query: &str, document: &SearchDocument) -> (usize, i64) {
    let tokens = searchable_text(query)
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let query_key = tokens.join(" ");
    let first = tokens.first().map(String::as_str).unwrap_or_default();
    let name = searchable_name(&document.name.primary);
    let alias = document
        .name
        .alias
        .as_deref()
        .map(searchable_name)
        .unwrap_or_default();
    let owner_member = document
        .owner
        .as_ref()
        .map(|owner| {
            searchable_name(&format!(
                "{}.{}",
                owner.display_name(),
                document.name.primary
            ))
        })
        .unwrap_or_default();
    if !query_key.is_empty() && (name == query_key || alias == query_key) {
        (0, kind_priority(document.kind))
    } else if !query_key.is_empty() && owner_member == query_key {
        (1, kind_priority(document.kind))
    } else if !first.is_empty() && name.starts_with(first) {
        (2, 0)
    } else if !first.is_empty() && alias.starts_with(first) {
        (3, 0)
    } else if tokens.iter().all(|token| name.contains(token)) {
        (4, 0)
    } else if tokens.iter().all(|token| alias.contains(token)) {
        (5, 0)
    } else {
        (10, 0)
    }
}

fn temp_index_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("index.sqlite");
    path.with_file_name(format!("{file_name}.tmp.{}", std::process::id()))
}

fn remove_sqlite_sidecars(path: &Path) -> Result<(), SearchError> {
    for sidecar in [
        path.with_extension("sqlite-wal"),
        path.with_extension("sqlite-shm"),
    ] {
        match fs::remove_file(&sidecar) {
            Ok(()) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(SearchError::Io {
                    path: sidecar,
                    source,
                });
            }
        }
    }
    Ok(())
}

fn remove_sqlite_artifacts(path: &Path) -> Result<(), SearchError> {
    remove_optional_file(path)?;
    remove_sqlite_sidecars(path)
}

fn remove_optional_file(path: &Path) -> Result<(), SearchError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(SearchError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

struct WriterLock {
    path: PathBuf,
    _file: File,
}

impl WriterLock {
    fn acquire(index_path: &Path) -> Result<Self, SearchError> {
        let path = index_path.with_extension("lock");
        let started = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok(Self { path, _file: file }),
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
                    if started.elapsed() > Duration::from_secs(30) {
                        return Err(SearchError::WriterLockTimeout { path });
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(source) => {
                    return Err(SearchError::Io {
                        path: path.clone(),
                        source,
                    });
                }
            }
        }
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::SyntaxHelperSink;
    use syntax_helper_language::{LanguagePageInput, LanguageSourceFamily, extract_language_facts};

    #[test]
    fn document_kind_roundtrips_storage_strings_and_priorities() {
        let expected = [
            (SearchDocumentKind::PlatformType, "platform_type", 10),
            (SearchDocumentKind::TypeProperty, "type_property", 20),
            (SearchDocumentKind::TypeMethod, "type_method", 30),
            (SearchDocumentKind::Constructor, "constructor", 40),
            (SearchDocumentKind::GlobalMethod, "global_method", 50),
            (SearchDocumentKind::GlobalProperty, "global_property", 60),
            (SearchDocumentKind::ModuleEvent, "module_event", 70),
            (SearchDocumentKind::TypeEvent, "type_event", 80),
            (SearchDocumentKind::UnknownEvent, "unknown_event", 90),
            (SearchDocumentKind::QueryTable, "query_table", 100),
            (
                SearchDocumentKind::QueryTableField,
                "query_table_field",
                110,
            ),
            (
                SearchDocumentKind::QueryTableParameter,
                "query_table_parameter",
                120,
            ),
            (SearchDocumentKind::LanguageType, "language_type", 125),
            (
                SearchDocumentKind::LanguageConstruct,
                "language_construct",
                126,
            ),
            (
                SearchDocumentKind::LanguageFunction,
                "language_function",
                127,
            ),
            (
                SearchDocumentKind::LanguageOperator,
                "language_operator",
                128,
            ),
            (SearchDocumentKind::LanguageKeyword, "language_keyword", 129),
            (SearchDocumentKind::LanguageLiteral, "language_literal", 130),
            (SearchDocumentKind::Enum, "enum", 140),
            (SearchDocumentKind::EnumValue, "enum_value", 150),
        ];

        assert_eq!(expected.len(), SearchDocumentKind::ALL.len());
        for (kind, stored, priority) in expected {
            assert_eq!(kind.as_str(), stored);
            assert_eq!(SearchDocumentKind::from_storage(stored), Some(kind));
            assert_eq!(kind.priority(), priority);
        }
        assert_eq!(SearchDocumentKind::from_storage("unexpected_kind"), None);
    }

    #[test]
    fn index_accepts_language_facts_with_distinct_source_qualified_ids() {
        let path = temp_path("language.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for fact in language_fixture_facts("ru") {
            builder.add_language_fact(fact);
        }
        build_index_from_builder(&path, &metadata(), builder).expect("language index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let bsl = index
            .get_by_id("shlang:def_String")
            .expect("id lookup must work")
            .expect("BSL string type must be indexed");
        assert_eq!(bsl.document.kind, SearchDocumentKind::LanguageType);
        assert_eq!(bsl.document.name.primary, "Строка");

        let function_construct = index
            .get_by_id("shlang:def_Func")
            .expect("id lookup must work")
            .expect("BSL function construct must be indexed");
        assert_eq!(
            function_construct.document.kind,
            SearchDocumentKind::LanguageConstruct
        );
        assert!(
            function_construct
                .document
                .signatures
                .iter()
                .any(|signature| signature.text.contains("Функция"))
        );

        let select = index
            .get_by_id("shquery:SELECTStatement")
            .expect("id lookup must work")
            .expect("query SELECT construct must be indexed");
        assert_eq!(select.document.kind, SearchDocumentKind::LanguageConstruct);
        assert!(
            select
                .document
                .signatures
                .iter()
                .any(|signature| signature.text.contains("ВЫБРАТЬ"))
        );

        let sum = index
            .get_by_id("shquery:SUM")
            .expect("id lookup must work")
            .expect("query SUM function must be indexed");
        assert_eq!(sum.document.kind, SearchDocumentKind::LanguageFunction);
        assert_eq!(sum.document.name.primary, "СУММА");

        let query = index
            .get_by_id("shquery:STRING")
            .expect("id lookup must work")
            .expect("query STRING function must be indexed");
        assert_eq!(query.document.kind, SearchDocumentKind::LanguageFunction);
        assert_eq!(query.document.name.primary, "СТРОКА");

        let skd = index
            .get_by_id("dcsui:SKD_Functions_Strings#StringLength")
            .expect("id lookup must work")
            .expect("SKD string function must be indexed");
        assert_eq!(skd.document.kind, SearchDocumentKind::LanguageFunction);
        assert_eq!(skd.document.name.primary, "ДлинаСтроки");

        let string_hits = index
            .get_by_name("Строка")
            .expect("same-display lookup must work");
        let ids = string_hits
            .iter()
            .map(|hit| hit.document.id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains("shlang:def_String"));
        assert!(ids.contains("shquery:STRING"));
        assert!(ids.contains("shquery:LitString"));
    }

    #[test]
    fn metadata_template_type_info_survives_index_roundtrip() {
        let path = temp_path("metadata-template.sqlite");
        let mut builder = SearchIndexBuilder::new();
        let mut manager = platform_type(
            "СправочникМенеджер.<Имя справочника>",
            Some("CatalogManager.<Catalog name>"),
            "Catalog manager template.",
        );
        manager.type_kind = model::PlatformTypeKind::MetadataTemplate;
        manager.metadata_kind = Some("СправочникМенеджер".to_string());
        manager.template_parameters = vec!["Имя справочника".to_string()];
        builder
            .platform_type(manager)
            .expect("platform template must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hit = index
            .type_identities_by_alias("CatalogManager.<Catalog name>")
            .expect("alias lookup must not fail")
            .pop()
            .expect("template type must resolve by alias");

        assert_eq!(
            hit.document.id,
            "platform_type:СправочникМенеджер.<Имя справочника>"
        );
        assert_eq!(
            hit.document.metadata_kind.as_deref(),
            Some("СправочникМенеджер")
        );
        assert_eq!(
            hit.document.template_parameters,
            vec!["Имя справочника".to_string()]
        );
        assert_eq!(
            hit.document.type_template_key,
            Some(model::PlatformTypeTemplateKey::new("Catalog", "Manager"))
        );

        let by_kind = index
            .type_template_by_key(&model::PlatformTypeTemplateKey::new("Catalog", "Manager"))
            .expect("semantic template lookup must not fail");
        assert_eq!(by_kind.len(), 1);
        assert_eq!(by_kind[0].document.id, hit.document.id);
    }

    #[test]
    fn type_template_classification_uses_longest_manager_root() {
        let path = temp_path("type-template-longest-root.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for (primary, alias) in [
            (
                "ДокументМенеджер.<Имя документа>",
                "DocumentManager.<Document name>",
            ),
            (
                "ЖурналДокументовМенеджер.<Имя журнала документов>",
                "DocumentJournalManager.<Document journal name>",
            ),
            (
                "ЖурналДокументовСсылка.<Имя журнала документов>",
                "DocumentJournalRef.<Document journal name>",
            ),
        ] {
            let mut record = platform_type(primary, Some(alias), "Template.");
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Имя".to_string()];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }

        build_index_from_builder(&path, &metadata(), builder).expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hit = index
            .type_template_by_key(&model::PlatformTypeTemplateKey::new(
                "DocumentJournal",
                "Ref",
            ))
            .expect("type-template lookup must not fail")
            .pop()
            .expect("document journal ref must resolve");

        assert_eq!(
            hit.document.type_template_key,
            Some(model::PlatformTypeTemplateKey::new(
                "DocumentJournal",
                "Ref",
            ))
        );
    }

    #[test]
    fn type_template_classification_uses_external_data_source_table_before_source() {
        let path = temp_path("type-template-external-data-source-longest-root.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for (primary, alias) in [
            (
                "ВнешнийИсточникДанныхМенеджер.<Имя внешнего источника>",
                "ExternalDataSourceManager.<External data source name>",
            ),
            (
                "ВнешнийИсточникДанныхТаблицаМенеджер.<Имя внешнего источника, Имя таблицы>",
                "ExternalDataSourceTableManager.<External data source name, Table name>",
            ),
            (
                "ВнешнийИсточникДанныхТаблицаСсылка.<Имя внешнего источника, Имя таблицы>",
                "ExternalDataSourceTableRef.<External data source name, Table name>",
            ),
        ] {
            let mut record = platform_type(primary, Some(alias), "Template.");
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec![
                "Имя внешнего источника".to_string(),
                "Имя таблицы".to_string(),
            ];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }

        build_index_from_builder(&path, &metadata(), builder).expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hit = index
            .type_template_by_key(&model::PlatformTypeTemplateKey::new(
                "ExternalDataSourceTable",
                "Ref",
            ))
            .expect("type-template lookup must not fail")
            .pop()
            .expect("external data source table ref must resolve");

        assert_eq!(
            hit.document.type_template_key,
            Some(model::PlatformTypeTemplateKey::new(
                "ExternalDataSourceTable",
                "Ref",
            ))
        );
    }

    #[test]
    fn type_template_classification_allows_primary_fallback_only_for_root_source() {
        let mut root_metadata = metadata();
        root_metadata.locale = "en".to_string();
        root_metadata.source_locale = "root".to_string();
        let root_path = temp_path("type-template-root-primary-fallback.sqlite");
        let mut root_builder = SearchIndexBuilder::new();
        for primary in [
            "DocumentManager.<Document name>",
            "DocumentRef.<Document name>",
        ] {
            let mut record = platform_type(primary, None, "Template.");
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Document name".to_string()];
            root_builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        build_index_from_builder(&root_path, &root_metadata, root_builder)
            .expect("root index must build");
        let root_index = SearchIndex::open_read_only(&root_path).expect("index must open");
        assert_eq!(
            root_index
                .type_template_by_key(&model::PlatformTypeTemplateKey::new("Document", "Ref"))
                .expect("type-template lookup must not fail")
                .len(),
            1
        );

        let ru_path = temp_path("type-template-ru-primary-no-fallback.sqlite");
        let mut ru_builder = SearchIndexBuilder::new();
        let mut record = platform_type("ДокументСсылка.<Имя документа>", None, "Template.");
        record.type_kind = model::PlatformTypeKind::MetadataTemplate;
        record.metadata_kind = Some("ДокументСсылка".to_string());
        record.template_parameters = vec!["Имя документа".to_string()];
        ru_builder
            .platform_type(record)
            .expect("platform template must sink");
        let report = build_index_from_builder_with_report(&ru_path, &metadata(), ru_builder)
            .expect("ru index must build");

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "TYPE_TEMPLATE_UNCLASSIFIED");
        let ru_index = SearchIndex::open_read_only(&ru_path).expect("index must open");
        let hit = ru_index
            .type_identity_by_id("platform_type:ДокументСсылка.<Имя документа>")
            .expect("lookup must not fail")
            .expect("template must exist");
        assert!(hit.document.type_template_key.is_none());
        assert!(
            hit.document
                .type_template_classification_diagnostic
                .as_deref()
                .is_some_and(|value| value.contains("primary fallback"))
        );
    }

    #[test]
    fn type_template_classification_uses_direct_refs_for_unassigned_templates() {
        let path = temp_path("type-template-direct-ref-family.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for (primary, alias) in [
            (
                "ПланВидовРасчетаМенеджер.<Имя плана видов расчета>",
                "ChartOfCalculationTypesManager.<Chart name>",
            ),
            (
                "ПланВидовРасчетаОбъект.<Имя плана видов расчета>",
                "ChartOfCalculationTypesObject.<Chart name>",
            ),
            (
                "ПланВидовРасчетаСсылка.<Имя плана видов расчета>",
                "ChartOfCalculationTypesRef.<Chart name>",
            ),
            (
                "БазовыеВидыРасчета.<Имя плана видов расчета>",
                "BaseCalculationTypes.<Chart name>",
            ),
            (
                "БазовыеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "BaseCalculationTypesRow.<Chart name>",
            ),
            (
                "ВедущиеВидыРасчета.<Имя плана видов расчета>",
                "LeadingCalculationTypes.<Chart name>",
            ),
            (
                "ВедущиеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "LeadingCalculationTypesRow.<Chart name>",
            ),
            (
                "ВытесняющиеВидыРасчета.<Имя плана видов расчета>",
                "DisplacingCalculationTypes.<Chart name>",
            ),
            (
                "ВытесняющиеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "DisplacingCalculationTypesRow.<Chart name>",
            ),
        ] {
            let mut record = platform_type(primary, Some(alias), "Template.");
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Имя плана видов расчета".to_string()];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        for (name_primary, name_alias, target_type) in [
            (
                "БазовыеВидыРасчета",
                "BaseCalculationTypes",
                "БазовыеВидыРасчета",
            ),
            (
                "ВедущиеВидыРасчета",
                "LeadingCalculationTypes",
                "ВедущиеВидыРасчета",
            ),
            (
                "ВытесняющиеВидыРасчета",
                "DisplacingCalculationTypes",
                "ВытесняющиеВидыРасчета",
            ),
        ] {
            builder
                .type_property(model::PlatformProperty {
                    owner: name(
                        "ПланВидовРасчетаОбъект.<Имя плана видов расчета>",
                        Some("ChartOfCalculationTypesObject.<Chart name>"),
                    ),
                    owner_identity: Some(
                        "platform_type:ПланВидовРасчетаОбъект.<Имя плана видов расчета>"
                            .to_string(),
                    ),
                    name: name(name_primary, Some(name_alias)),
                    semantic: model::SemanticContext::default(),
                    usage: None,
                    type_refs: vec![model::TypeRef {
                        name: target_type.to_string(),
                    }],
                    description: None,
                    facts: model::SectionFacts::default(),
                    source: source(name_primary),
                })
                .expect("property must sink");
        }
        for (owner_primary, owner_alias) in [
            (
                "БазовыеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "BaseCalculationTypesRow.<Chart name>",
            ),
            (
                "ВедущиеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "LeadingCalculationTypesRow.<Chart name>",
            ),
            (
                "ВытесняющиеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "DisplacingCalculationTypesRow.<Chart name>",
            ),
        ] {
            builder
                .type_property(model::PlatformProperty {
                    owner: name(owner_primary, Some(owner_alias)),
                    owner_identity: Some(format!("platform_type:{owner_primary}")),
                    name: name("ВидРасчета", Some("CalculationType")),
                    semantic: model::SemanticContext::default(),
                    usage: None,
                    type_refs: vec![model::TypeRef {
                        name: "ПланВидовРасчетаСсылка".to_string(),
                    }],
                    description: None,
                    facts: model::SectionFacts::default(),
                    source: source(owner_primary),
                })
                .expect("property must sink");
        }

        let report = build_index_from_builder_with_report(&path, &metadata(), builder)
            .expect("index must build");
        assert!(
            report
                .warnings
                .iter()
                .all(|warning| !warning.code.starts_with("TYPE_TEMPLATE_"))
        );
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        for (variant, expected_id) in [
            (
                "BaseCalculationTypes",
                "platform_type:БазовыеВидыРасчета.<Имя плана видов расчета>",
            ),
            (
                "BaseCalculationTypesRow",
                "platform_type:БазовыеВидыРасчетаСтрока.<Имя плана видов расчета>",
            ),
            (
                "LeadingCalculationTypes",
                "platform_type:ВедущиеВидыРасчета.<Имя плана видов расчета>",
            ),
            (
                "LeadingCalculationTypesRow",
                "platform_type:ВедущиеВидыРасчетаСтрока.<Имя плана видов расчета>",
            ),
            (
                "DisplacingCalculationTypes",
                "platform_type:ВытесняющиеВидыРасчета.<Имя плана видов расчета>",
            ),
            (
                "DisplacingCalculationTypesRow",
                "platform_type:ВытесняющиеВидыРасчетаСтрока.<Имя плана видов расчета>",
            ),
        ] {
            let hits = index
                .type_template_by_key(&model::PlatformTypeTemplateKey::new(
                    "ChartOfCalculationTypes",
                    variant,
                ))
                .expect("type-template lookup must not fail");
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].document.id, expected_id);
            assert!(
                hits[0]
                    .document
                    .type_template_classification_diagnostic
                    .as_deref()
                    .is_some_and(|value| value.starts_with("direct_type_ref "))
            );
        }
    }

    #[test]
    fn type_template_classification_reports_unclassified_templates() {
        let path = temp_path("type-template-unclassified.sqlite");
        let mut builder = SearchIndexBuilder::new();
        let mut record = platform_type(
            "ОдиночныйШаблон.<Имя>",
            Some("SingleTemplate.<Name>"),
            "Template.",
        );
        record.type_kind = model::PlatformTypeKind::MetadataTemplate;
        record.metadata_kind = Some("ОдиночныйШаблон".to_string());
        record.template_parameters = vec!["Имя".to_string()];
        builder
            .platform_type(record)
            .expect("platform template must sink");

        let report = build_index_from_builder_with_report(&path, &metadata(), builder)
            .expect("index must build");
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "TYPE_TEMPLATE_UNCLASSIFIED");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hit = index
            .type_identity_by_id("platform_type:ОдиночныйШаблон.<Имя>")
            .expect("lookup must not fail")
            .expect("template must exist");
        assert_eq!(
            hit.document
                .type_template_classification_diagnostic
                .as_deref(),
            Some(report.warnings[0].message.as_str())
        );
    }

    #[test]
    fn type_template_classification_persists_ambiguous_diagnostics() {
        let path = temp_path("type-template-ambiguous.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for (primary, alias) in [
            ("ПервыйМенеджер.<Имя>", "FirstManager.<Name>"),
            ("ПервыйОбъект.<Имя>", "FirstObject.<Name>"),
            ("ВторойМенеджер.<Имя>", "SecondManager.<Name>"),
            ("ВторойОбъект.<Имя>", "SecondObject.<Name>"),
            ("ОбщийШаблон.<Имя>", "SharedTemplate.<Name>"),
        ] {
            let mut record = platform_type(primary, Some(alias), "Template.");
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Имя".to_string()];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        for (owner_primary, owner_alias, property_primary) in [
            ("ПервыйОбъект.<Имя>", "FirstObject.<Name>", "ОбщийИзПервого"),
            (
                "ВторойОбъект.<Имя>",
                "SecondObject.<Name>",
                "ОбщийИзВторого",
            ),
        ] {
            builder
                .type_property(model::PlatformProperty {
                    owner: name(owner_primary, Some(owner_alias)),
                    owner_identity: Some(format!("platform_type:{owner_primary}")),
                    name: name(property_primary, None),
                    semantic: model::SemanticContext::default(),
                    usage: None,
                    type_refs: vec![model::TypeRef {
                        name: "ОбщийШаблон".to_string(),
                    }],
                    description: None,
                    facts: model::SectionFacts::default(),
                    source: source(property_primary),
                })
                .expect("property must sink");
        }

        let report = build_index_from_builder_with_report(&path, &metadata(), builder)
            .expect("index must build");
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "TYPE_TEMPLATE_AMBIGUOUS_FAMILY");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hit = index
            .type_identity_by_id("platform_type:ОбщийШаблон.<Имя>")
            .expect("lookup must not fail")
            .expect("template must exist");
        assert_eq!(
            hit.document
                .type_template_classification_diagnostic
                .as_deref(),
            Some(report.warnings[0].message.as_str())
        );
    }

    #[test]
    fn type_template_type_refs_preserve_owner_parameter_binding() {
        let path = temp_path("type-template-binding.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for mut record in [
            platform_type(
                "ДокументМенеджер.<Имя документа>",
                Some("DocumentManager.<Document name>"),
                "Document manager template.",
            ),
            platform_type(
                "ДокументОбъект.<Имя документа>",
                Some("DocumentObject.<Document name>"),
                "Document object template.",
            ),
            platform_type(
                "ДокументСсылка.<Имя документа>",
                Some("DocumentRef.<Document name>"),
                "Document reference template.",
            ),
        ] {
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Имя документа".to_string()];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        builder
            .type_property(model::PlatformProperty {
                owner: name(
                    "ДокументОбъект.<Имя документа>",
                    Some("DocumentObject.<Document name>"),
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name("Ссылка", Some("Ref")),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ДокументСсылка".to_string(),
                }],
                description: Some("Document reference.".to_string()),
                facts: model::SectionFacts::default(),
                source: source("document-object-ref"),
            })
            .expect("type-template property must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let property = index
            .member_by_owner_type_id("platform_type:ДокументОбъект.<Имя документа>", "Ссылка")
            .expect("property lookup must not fail")
            .pop()
            .expect("property must resolve");

        assert_eq!(property.document.type_ref_facts.len(), 1);
        let binding = property.document.type_ref_facts[0]
            .template_binding
            .as_ref()
            .expect("type ref must preserve template binding");
        assert_eq!(
            binding.template_key,
            model::PlatformTypeTemplateKey::new("Document", "Ref")
        );
        assert_eq!(
            binding.arguments,
            vec![model::TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: 0,
                target_parameter_index: 0,
            }]
        );
    }

    #[test]
    fn type_template_type_refs_preserve_multiple_owner_parameter_bindings() {
        let path = temp_path("type-template-multi-binding.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for mut record in [
            platform_type(
                "ВнешнийИсточникДанныхТаблицаМенеджер.<Имя внешнего источника, Имя таблицы>",
                Some("ExternalDataSourceTableManager.<External data source name, Table name>"),
                "External table manager template.",
            ),
            platform_type(
                "ВнешнийИсточникДанныхТаблицаОбъект.<Имя внешнего источника, Имя таблицы>",
                Some("ExternalDataSourceTableObject.<External data source name, Table name>"),
                "External table object template.",
            ),
            platform_type(
                "ВнешнийИсточникДанныхТаблицаСсылка.<Имя внешнего источника, Имя таблицы>",
                Some("ExternalDataSourceTableRef.<External data source name, Table name>"),
                "External table reference template.",
            ),
        ] {
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec![
                "Имя внешнего источника".to_string(),
                "Имя таблицы".to_string(),
            ];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        builder
            .type_property(model::PlatformProperty {
                owner: name(
                    "ВнешнийИсточникДанныхТаблицаОбъект.<Имя внешнего источника, Имя таблицы>",
                    Some("ExternalDataSourceTableObject.<External data source name, Table name>"),
                ),
                owner_identity: Some(
                    "platform_type:ВнешнийИсточникДанныхТаблицаОбъект.<Имя внешнего источника, Имя таблицы>"
                        .to_string(),
                ),
                name: name("Ссылка", Some("Ref")),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ВнешнийИсточникДанныхТаблицаСсылка".to_string(),
                }],
                description: Some("External table reference.".to_string()),
                facts: model::SectionFacts::default(),
                source: source("external-data-source-table-object-ref"),
            })
            .expect("type-template property must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let property = index
            .member_by_owner_type_id(
                "platform_type:ВнешнийИсточникДанныхТаблицаОбъект.<Имя внешнего источника, Имя таблицы>",
                "Ссылка",
            )
            .expect("property lookup must not fail")
            .pop()
            .expect("property must resolve");

        let binding = property.document.type_ref_facts[0]
            .template_binding
            .as_ref()
            .expect("type ref must preserve template binding");
        assert_eq!(
            binding.template_key,
            model::PlatformTypeTemplateKey::new("ExternalDataSourceTable", "Ref")
        );
        assert_eq!(
            binding.arguments,
            vec![
                model::TemplateParameterBinding::OwnerParameter {
                    owner_parameter_index: 0,
                    target_parameter_index: 0,
                },
                model::TemplateParameterBinding::OwnerParameter {
                    owner_parameter_index: 1,
                    target_parameter_index: 1,
                },
            ]
        );
    }

    #[test]
    fn type_template_binding_does_not_choose_ambiguous_type_ref_target() {
        let path = temp_path("type-template-binding-ambiguous.sqlite");
        let mut builder = SearchIndexBuilder::new();
        let mut owner = platform_type(
            "ДокументОбъект.<Имя документа>",
            Some("DocumentObject.<Document name>"),
            "Document object template.",
        );
        owner.type_kind = model::PlatformTypeKind::MetadataTemplate;
        owner.metadata_kind = Some("ДокументОбъект".to_string());
        owner.template_parameters = vec!["Имя документа".to_string()];
        builder
            .platform_type(owner)
            .expect("owner template must sink");
        for owner_path in ["Документы продаж", "Документы склада"] {
            let mut reference = platform_type(
                "ДокументСсылка.<Имя документа>",
                Some("DocumentRef.<Document name>"),
                "Document reference template.",
            );
            reference.semantic = semantic_path(model::RecordFamily::PlatformType, &[owner_path]);
            reference.type_kind = model::PlatformTypeKind::MetadataTemplate;
            reference.metadata_kind = Some("ДокументСсылка".to_string());
            reference.template_parameters = vec!["Имя документа".to_string()];
            builder
                .platform_type(reference)
                .expect("target template must sink");
        }
        builder
            .type_property(model::PlatformProperty {
                owner: name(
                    "ДокументОбъект.<Имя документа>",
                    Some("DocumentObject.<Document name>"),
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name("Ссылка", Some("Ref")),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ДокументСсылка".to_string(),
                }],
                description: Some("Document reference.".to_string()),
                facts: model::SectionFacts::default(),
                source: source("ambiguous-document-object-ref"),
            })
            .expect("type-template property must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let property = index
            .member_by_owner_type_id("platform_type:ДокументОбъект.<Имя документа>", "Ссылка")
            .expect("property lookup must not fail")
            .pop()
            .expect("property must resolve");

        assert_eq!(property.document.type_ref_facts.len(), 1);
        assert_eq!(property.document.type_ref_facts[0].target_type_id, None);
        assert_eq!(property.document.type_ref_facts[0].template_binding, None);
    }

    #[test]
    fn index_accepts_root_language_facts_with_same_logical_ids() {
        let path = temp_path("language-root.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for fact in language_fixture_facts("root") {
            builder.add_language_fact(fact);
        }
        build_index_from_builder(&path, &metadata(), builder)
            .expect("root language index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let bsl = index
            .get_by_id("shlang:def_String")
            .expect("id lookup must work")
            .expect("root BSL string type must be indexed");
        assert_eq!(bsl.document.kind, SearchDocumentKind::LanguageType);
        assert_eq!(bsl.document.name.primary, "String");

        let sum = index
            .get_by_id("shquery:SUM")
            .expect("id lookup must work")
            .expect("root query SUM function must be indexed");
        assert_eq!(sum.document.kind, SearchDocumentKind::LanguageFunction);
        assert_eq!(sum.document.name.primary, "SUM");

        let skd = index
            .get_by_id("dcsui:SKD_Functions_Strings#StringLength")
            .expect("id lookup must work")
            .expect("root SKD string function must be indexed");
        assert_eq!(skd.document.kind, SearchDocumentKind::LanguageFunction);
        assert_eq!(skd.document.name.primary, "StringLength");
    }

    #[test]
    fn language_relations_use_source_qualified_targets_not_same_name_winners() {
        let bsl_string = language_fact(
            "shlang:def_String",
            LanguageSourceFamily::Shlang,
            language::LanguageDomain::BslLanguage,
            language::LanguageFactFamily::Type,
            name("Строка", Some("String")),
        );
        let query_literal = language_fact(
            "shquery:LitString",
            LanguageSourceFamily::Shquery,
            language::LanguageDomain::QueryLanguage,
            language::LanguageFactFamily::Literal,
            name("Строка", Some("STRING")),
        );
        let bsl_boolean = language_fact(
            "shlang:def_Boolean",
            LanguageSourceFamily::Shlang,
            language::LanguageDomain::BslLanguage,
            language::LanguageFactFamily::Type,
            name("Булево", Some("Boolean")),
        );
        let query_boolean = language_fact(
            "shquery:LitBoolean",
            LanguageSourceFamily::Shquery,
            language::LanguageDomain::QueryLanguage,
            language::LanguageFactFamily::Literal,
            name("Булево", Some("BOOLEAN")),
        );
        let mut query_string = language_fact(
            "shquery:STRING",
            LanguageSourceFamily::Shquery,
            language::LanguageDomain::QueryLanguage,
            language::LanguageFactFamily::Function,
            name("СТРОКА", Some("STRING")),
        );
        let mut query_boolean_fn = language_fact(
            "shquery:BOOLEAN",
            LanguageSourceFamily::Shquery,
            language::LanguageDomain::QueryLanguage,
            language::LanguageFactFamily::Function,
            name("БУЛЕВО", Some("BOOLEAN")),
        );
        query_string.signatures = vec![language::LanguageSignature {
            text: "СТРОКА(<Значение>)".to_string(),
            parameters: vec![language::LanguageParameter {
                name: "Значение".to_string(),
                required: true,
                type_refs: vec!["Строка".to_string()],
                description: None,
            }],
        }];
        query_string.return_types = vec!["Строка".to_string()];
        query_boolean_fn.signatures = vec![language::LanguageSignature {
            text: "БУЛЕВО(<Значение>)".to_string(),
            parameters: vec![language::LanguageParameter {
                name: "Значение".to_string(),
                required: true,
                type_refs: vec!["Булево".to_string()],
                description: None,
            }],
        }];
        query_boolean_fn.return_types = vec!["Булево".to_string()];

        let documents = vec![
            language_document(&bsl_string),
            language_document(&bsl_boolean),
            language_document(&query_literal),
            language_document(&query_boolean),
            language_document(&query_string),
            language_document(&query_boolean_fn),
        ];
        let relations = relations_from_documents(&documents);

        assert!(relations.iter().any(|relation| {
            relation.source_id == "shquery:STRING"
                && relation.target_id == "shquery:LitString"
                && relation.edge_kind == "has_type"
        }));
        assert!(relations.iter().any(|relation| {
            relation.source_id == "shquery:STRING"
                && relation.target_id == "shquery:LitString"
                && relation.edge_kind == "returns"
        }));
        assert!(!relations.iter().any(|relation| {
            relation.source_id == "shquery:STRING" && relation.target_id == "shlang:def_String"
        }));
        assert!(!relations.iter().any(|relation| {
            relation.source_id == "shquery:BOOLEAN"
                && matches!(
                    relation.target_id.as_str(),
                    "shlang:def_Boolean" | "shquery:LitBoolean"
                )
        }));
    }

    #[test]
    fn index_supports_exact_keyword_fuzzy_and_related_queries() {
        let path = temp_path("query.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");

        let exact = index
            .get_by_name("DataCompositionFilter")
            .expect("exact lookup must work");
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].document.name.primary, "ОтборКомпоновкиДанных");
        assert_eq!(exact[0].document.kind, SearchDocumentKind::PlatformType);

        let event = index
            .get_by_name("ПередЗаписью")
            .expect("event lookup must work");
        assert_eq!(event[0].document.kind, SearchDocumentKind::TypeEvent);

        let member = index
            .get_by_owner_member("НастройкиКомпоновкиДанных", "Отбор")
            .expect("owner/member lookup must work");
        assert_eq!(member[0].document.type_refs, ["ОтборКомпоновкиДанных"]);

        let by_id = index
            .get_by_id("type_property:platform_type:НастройкиКомпоновкиДанных:Отбор")
            .expect("id lookup must work")
            .expect("member document id must exist");
        assert_eq!(by_id.document.name.primary, "Отбор");

        let type_identity = index
            .type_identities_by_alias("DataCompositionFilter")
            .expect("type alias lookup must work");
        assert_eq!(type_identity.len(), 1);
        assert_eq!(
            type_identity[0].document.id,
            "platform_type:ОтборКомпоновкиДанных"
        );

        let members = index
            .members_by_type_id("platform_type:ОтборКомпоновкиДанных")
            .expect("member listing must work");
        assert!(members.iter().any(|hit| {
            hit.document.kind == SearchDocumentKind::TypeProperty
                && hit.document.name.primary == "Элементы"
        }));
        assert!(members.iter().any(|hit| {
            hit.document.kind == SearchDocumentKind::TypeMethod
                && hit.document.name.primary == "ПолучитьОбъектПоИдентификатору"
        }));

        let owner_type_member = index
            .member_by_owner_type_id("platform_type:НастройкиКомпоновкиДанных", "Отбор")
            .expect("owner type member lookup must work");
        assert_eq!(owner_type_member.len(), 1);
        assert_eq!(
            owner_type_member[0].document.id,
            "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"
        );

        let callable = index
            .callable_by_owner_type_id(
                "platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных",
                "Добавить",
            )
            .expect("owner type callable lookup must work");
        assert_eq!(callable.len(), 1);
        assert_eq!(
            callable[0].document.return_types,
            ["ЭлементОтбораКомпоновкиДанных"]
        );

        let keyword = index
            .search("отбор скд", SearchMode::Keywords, 10)
            .expect("keyword search must work");
        assert_eq!(keyword[0].document.name.primary, "ОтборКомпоновкиДанных");

        let fuzzy = index
            .search("ОтборКомпоновкиДаных", SearchMode::Fuzzy, 10)
            .expect("fuzzy search must work");
        assert_eq!(fuzzy[0].document.name.primary, "ОтборКомпоновкиДанных");

        let ambiguous_related = index
            .related_by_name("ОтборКомпоновкиДанных", 5, 20)
            .expect_err("plain-name related root must report ambiguity");
        assert!(matches!(
            ambiguous_related,
            SearchError::AmbiguousLookup { matches: 2, .. }
        ));
        let related = index
            .related_by_id("platform_type:ОтборКомпоновкиДанных", 5, 20)
            .expect("id-root related search must work");
        let names = related
            .iter()
            .map(|hit| hit.document.name.primary.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"Новый ОтборКомпоновкиДанных()"));
        assert!(names.contains(&"Элементы"));
        assert!(names.contains(&"Добавить"));
        assert!(names.contains(&"ЛевоеЗначение"));

        let related_by_owner_member = index
            .related_by_owner_member("НастройкиКомпоновкиДанных", "Отбор", 5, 20)
            .expect("owner/member related search must work");
        assert!(
            related_by_owner_member
                .iter()
                .any(|hit| hit.document.name.primary == "ОтборКомпоновкиДанных")
        );
        assert!(
            related_by_owner_member
                .iter()
                .any(|hit| hit.document.name.primary == "Добавить")
        );
        assert!(related_by_owner_member.iter().any(|hit| {
            hit.document.owner.as_ref().is_some_and(|owner| {
                owner.primary == "ЭлементОтбораКомпоновкиДанных"
                    && hit.document.name.primary == "ЛевоеЗначение"
            })
        }));

        let related_by_id = index
            .related_by_id(
                "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор",
                5,
                20,
            )
            .expect("id-root related search must work");
        assert_eq!(related_by_id, related_by_owner_member);

        let type_refs = index
            .related_by_id_and_edge(
                "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор",
                "has_type",
                20,
            )
            .expect("edge-filtered related search must work");
        assert_eq!(type_refs.len(), 1);
        assert_eq!(
            type_refs[0].document.id,
            "platform_type:ОтборКомпоновкиДанных"
        );

        let owner_refs = index
            .related_by_id_and_edge(
                "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор",
                "member_of",
                20,
            )
            .expect("member_of edge-filtered related search must work");
        assert_eq!(owner_refs.len(), 1);
        assert_eq!(
            owner_refs[0].document.id,
            "platform_type:НастройкиКомпоновкиДанных"
        );
        assert_eq!(owner_refs[0].via[0].edge_kind, "member_of");

        let ambiguous_constructors = index
            .constructors_by_name("ОтборКомпоновкиДанных")
            .expect_err("plain constructor type root must report ambiguity");
        assert!(matches!(
            ambiguous_constructors,
            SearchError::AmbiguousLookup { matches: 2, .. }
        ));
        let constructors = index
            .constructors_by_type_id("platform_type:ОтборКомпоновкиДанных")
            .expect("type-id constructor lookup must work");
        assert_eq!(constructors.len(), 1);
        assert_eq!(
            constructors[0].document.signature_text_lines(),
            ["Новый ОтборКомпоновкиДанных()"]
        );
    }

    #[test]
    fn owner_type_member_and_callable_lookup_match_primary_and_alias_names() {
        let path = temp_path("owner-type-primary-alias-lookup.sqlite");
        let mut context = fixture_context();
        context.type_properties.push(model::PlatformProperty {
            owner: name("НастройкиКомпоновкиДанных", None),
            owner_identity: None,
            name: name("ПользовательскийОтбор", Some("CustomFilter")),
            semantic: model::SemanticContext::default(),
            usage: None,
            type_refs: vec![model::TypeRef {
                name: "ОтборКомпоновкиДанных".to_string(),
            }],
            description: Some("ПользовательскийОтбор description".to_string()),
            facts: model::SectionFacts::default(),
            source: source("НастройкиКомпоновкиДанных.ПользовательскийОтбор"),
        });
        context.type_methods.push(model::PlatformMethod {
            owner: name("ОтборКомпоновкиДанных", None),
            owner_identity: None,
            name: name("Найти", Some("Find")),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: "Найти()".to_string(),
                parameters: Vec::new(),
                variant: None,
            }],
            return_types: vec![model::TypeRef {
                name: "ЭлементОтбораКомпоновкиДанных".to_string(),
            }],
            description: Some("Найти description".to_string()),
            facts: model::SectionFacts::default(),
            source: source("ОтборКомпоновкиДанных.Найти"),
        });
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let member_by_primary = index
            .member_by_owner_type_id(
                "platform_type:НастройкиКомпоновкиДанных",
                "ПользовательскийОтбор",
            )
            .expect("member primary lookup must work");
        let member_by_alias = index
            .member_by_owner_type_id("platform_type:НастройкиКомпоновкиДанных", "CustomFilter")
            .expect("member alias lookup must work");
        assert_eq!(member_by_primary, member_by_alias);
        assert_eq!(member_by_alias.len(), 1);
        assert_eq!(
            member_by_alias[0].document.id,
            "type_property:platform_type:НастройкиКомпоновкиДанных:ПользовательскийОтбор"
        );

        let callable_by_primary = index
            .callable_by_owner_type_id("platform_type:ОтборКомпоновкиДанных", "Найти")
            .expect("callable primary lookup must work");
        let callable_by_alias = index
            .callable_by_owner_type_id("platform_type:ОтборКомпоновкиДанных", "Find")
            .expect("callable alias lookup must work");
        assert_eq!(callable_by_primary, callable_by_alias);
        assert_eq!(callable_by_alias.len(), 1);
        assert_eq!(
            callable_by_alias[0].document.id,
            "type_method:platform_type:ОтборКомпоновкиДанных:Найти"
        );
    }

    #[test]
    fn keyword_search_prefers_exact_identity_for_simple_symbol() {
        let path = temp_path("simple-symbol-ranking.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type("Структура", Some("Structure"), "Коллекция значений."),
                platform_type(
                    "СтруктураНастроекКомпоновкиДанных",
                    None,
                    "Структура настроек компоновки данных.",
                ),
                platform_type(
                    "НастройкиКомпоновкиДанных",
                    None,
                    "Настройки системы компоновки данных.",
                ),
            ],
            type_properties: vec![
                type_property("НастройкиКомпоновкиДанных", "Структура", "Структура"),
                type_property(
                    "СтруктураНастроекКомпоновкиДанных",
                    "Структура",
                    "Структура",
                ),
            ],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");

        let hits = index
            .search("Структура", SearchMode::Keywords, 10)
            .expect("keyword search must work");
        assert_eq!(hits[0].document.id, "platform_type:Структура");
        assert!(hits.iter().skip(1).any(|hit| {
            hit.document.kind == SearchDocumentKind::TypeProperty
                && hit.document.name.primary == "Структура"
        }));
    }

    #[test]
    fn keyword_search_keeps_task_oriented_query_table_ranking() {
        let path = temp_path("task-query-ranking.sqlite");
        let context = model::PlatformContext {
            query_tables: vec![
                query_table(
                    "РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии",
                    "Работа с запросами.Таблицы запросов.РегистрБухгалтерии.Таблица изменений",
                    "РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии",
                ),
                query_table(
                    "РегистрБухгалтерииОсновнаяТаблица",
                    "Работа с запросами.Таблицы запросов.РегистрБухгалтерии.Основная таблица",
                    "РегистрБухгалтерииОсновнаяТаблица",
                ),
            ],
            table_fields: vec![query_table_field(
                "РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии",
                "Работа с запросами.Таблицы запросов.РегистрБухгалтерии.Таблица изменений",
                "Регистратор",
            )],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");

        let hits = index
            .search("таблица регистра бухгалтерии", SearchMode::Keywords, 10)
            .expect("keyword search must work");
        assert_eq!(
            hits[0].document.id,
            "query_table:РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии"
        );
    }

    #[test]
    fn constructor_json_preserves_structured_parameters_after_sqlite_roundtrip() {
        let path = temp_path("http-constructor-json.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![platform_type(
                "HTTPСоединение",
                Some("HTTPConnection"),
                "HTTP connection.",
            )],
            constructors: vec![http_connection_constructor()],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let document_columns = table_columns(&index.connection, "documents");
        assert!(!document_columns.contains(&"signature_json".to_string()));
        assert!(!document_columns.contains(&"preview".to_string()));
        assert_eq!(
            index
                .connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM parameters p
                     JOIN type_refs r ON r.source_signature_id = p.signature_id
                      AND r.source_parameter_ordinal = p.ordinal
                     WHERE p.name = 'ИспользоватьАутентификациюОС'
                       AND r.ref_kind = 'parameter_type'
                       AND r.target_type_name = 'Булево'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("normalized parameter type ref query must work"),
            1
        );
        assert_eq!(
            index
                .connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM type_refs
                     WHERE source_document_id LIKE 'constructor:%HTTPСоединение%'
                       AND ref_kind = 'constructor_result'
                       AND target_type_id = 'platform_type:HTTPСоединение'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("constructor result type ref query must work"),
            1
        );
        let constructors = index
            .constructors_by_name("HTTPСоединение")
            .expect("constructor lookup must work");
        assert_eq!(constructors.len(), 1);
        assert_eq!(
            constructors[0].document.signatures[0].text,
            "Новый HTTPСоединение(<Сервер>, <Порт>, <ИспользоватьАутентификациюОС>)"
        );
        assert!(
            constructors[0]
                .document
                .parameter_terms
                .contains(&"ИспользоватьАутентификациюОС".to_string())
        );
        assert!(
            constructors[0]
                .document
                .parameter_terms
                .contains(&"Булево".to_string())
        );

        let signatures = &constructors[0].document.signatures;
        assert_eq!(signatures.len(), 1);
        assert_eq!(
            signatures[0].text,
            "Новый HTTPСоединение(<Сервер>, <Порт>, <ИспользоватьАутентификациюОС>)"
        );
        let os_auth = signatures[0]
            .parameters
            .iter()
            .find(|parameter| parameter.name == "ИспользоватьАутентификациюОС")
            .expect("OS authentication parameter must be present");
        assert!(!os_auth.required);
        assert!(os_auth.type_refs.iter().any(|value| value == "Булево"));
    }

    #[test]
    fn constructor_duplicate_ids_keep_last_document_with_warning() {
        let context = model::PlatformContext {
            platform_types: vec![platform_type(
                "МенеджерКриптографии",
                None,
                "Crypto manager.",
            )],
            constructors: vec![
                constructor_with_name(
                    "МенеджерКриптографии",
                    "Без инициализации модуля криптографии",
                    "Новый МенеджерКриптографии(<ИспользованиеИнтерактивногоРежима>)",
                ),
                constructor_with_name(
                    "МенеджерКриптографии",
                    "Для инициализации",
                    "Новый МенеджерКриптографии(<ИспользованиеИнтерактивногоРежима>)",
                ),
            ],
            ..model::PlatformContext::default()
        };

        let build = builder_from_context(&context)
            .into_documents("ru")
            .expect("same-signature constructor duplicates must not collide");
        assert_eq!(build.warnings.len(), 1);
        assert_eq!(build.warnings[0].code, "DUPLICATE_DOCUMENT_ID");
        assert!(build.warnings[0].message.contains("kept the last document"));
        let constructors = build
            .documents
            .iter()
            .filter(|document| document.kind == SearchDocumentKind::Constructor)
            .collect::<Vec<_>>();
        assert_eq!(constructors.len(), 1);
        assert_eq!(
            constructors[0].id,
            "constructor:platform_type:МенеджерКриптографии:Новый МенеджерКриптографии(<ИспользованиеИнтерактивногоРежима>)"
        );
        assert_eq!(
            constructors[0].description.as_deref(),
            Some("Для инициализации description")
        );
    }

    #[test]
    fn streaming_builder_preserves_expected_document_and_relation_shape() {
        let context = fixture_context();
        let builder_documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("fixture documents must not collide")
            .documents;
        let ids = builder_documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"platform_type:ОтборКомпоновкиДанных"));
        assert!(ids.contains(&"type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"));
        assert!(ids.contains(
            &"constructor:platform_type:ОтборКомпоновкиДанных:Новый ОтборКомпоновкиДанных()"
        ));
        let builder_relations = relations_from_documents(&builder_documents)
            .into_iter()
            .map(|relation| (relation.source_id, relation.target_id, relation.edge_kind))
            .collect::<Vec<_>>();
        assert!(builder_relations.iter().any(|(source, target, edge)| {
            source == "platform_type:ОтборКомпоновкиДанных"
                && target == "type_property:platform_type:ОтборКомпоновкиДанных:Элементы"
                && *edge == "owns"
        }));
        assert!(builder_relations.iter().any(|(source, target, edge)| {
            source == "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"
                && target == "platform_type:ОтборКомпоновкиДанных"
                && *edge == "has_type"
        }));
    }

    #[test]
    fn streaming_builder_builds_sqlite_index_with_expected_queries_and_relations() {
        let path = temp_path("streaming-builder.sqlite");
        build_index_from_builder(&path, &metadata(), builder_from_context(&fixture_context()))
            .expect("streaming builder must build SQLite index");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        assert_eq!(
            index
                .metadata()
                .expect("index metadata must be readable")
                .source_extraction_schema_version,
            metadata().source_extraction_schema_version
        );
        let search_rows: usize = index
            .connection
            .query_row("SELECT COUNT(*) FROM document_search", [], |row| row.get(0))
            .expect("content rows must be stored");
        let fts_rows: usize = index
            .connection
            .query_row("SELECT COUNT(*) FROM document_fts", [], |row| row.get(0))
            .expect("fts rows must be rebuilt from content rows");
        assert_eq!(search_rows, fts_rows);

        let exact = index
            .get_by_name("DataCompositionFilter")
            .expect("exact lookup must work");
        assert_eq!(exact[0].document.name.primary, "ОтборКомпоновкиДанных");

        let related = index
            .related_by_id("platform_type:ОтборКомпоновкиДанных", 5, 20)
            .expect("id-root related search must work");
        assert!(related.iter().any(|hit| {
            hit.document.kind == SearchDocumentKind::Constructor
                && hit.document.name.primary == "Новый ОтборКомпоновкиДанных()"
        }));
        assert!(related.iter().any(|hit| {
            hit.document.kind == SearchDocumentKind::TypeProperty
                && hit.document.name.primary == "Элементы"
        }));
        assert_eq!(
            index
                .connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM members
                     WHERE owner_type_id = 'platform_type:ОтборКомпоновкиДанных'
                       AND member_kind = 'type_property'
                       AND name_primary = 'Элементы'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("normalized member query must work"),
            1
        );
        assert_eq!(
            index
                .connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM type_refs
                     WHERE source_document_id = 'type_property:platform_type:НастройкиКомпоновкиДанных:Отбор'
                       AND ref_kind = 'property_type'
                       AND target_type_id = 'platform_type:ОтборКомпоновкиДанных'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("normalized property type ref query must work"),
            1
        );
    }

    #[test]
    fn sqlite_relation_rows_match_shared_relation_builder() {
        let path = temp_path("relation-builder-parity.sqlite");
        let documents = builder_from_context(&fixture_context())
            .into_documents("ru")
            .expect("fixture documents must not collide")
            .documents;
        let expected = relations_from_documents(&documents)
            .into_iter()
            .map(|relation| {
                (
                    relation.source_id,
                    relation.target_id,
                    relation.edge_kind.to_string(),
                    relation.label,
                    relation.evidence.to_string(),
                    relation.weight,
                )
            })
            .collect::<Vec<_>>();

        build_index_from_documents(&path, &metadata(), documents).expect("index must build");
        let connection = Connection::open(&path).expect("index must open for relation inspection");
        let mut statement = connection
            .prepare(
                "SELECT source_id, target_id, edge_kind, label, evidence, weight
                 FROM relations
                 ORDER BY weight, source_id, edge_kind, target_id",
            )
            .expect("relation query must prepare");
        let actual = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .expect("relation query must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("relation rows must deserialize");

        assert_eq!(actual, expected);
    }

    #[test]
    fn index_build_rejects_duplicate_document_ids_before_sqlite_write() {
        let path = temp_path("duplicate-document-id.sqlite");
        let documents = vec![
            document(
                SearchDocumentKind::GlobalMethod,
                None,
                &name("Сообщить", None),
                &[],
                &[],
                &[],
                Some("first source page"),
                "global_method:Сообщить".to_string(),
            ),
            document(
                SearchDocumentKind::GlobalMethod,
                None,
                &name("Сообщить", None),
                &[],
                &[],
                &[],
                Some("second source page"),
                "global_method:Сообщить".to_string(),
            ),
        ];

        let error = build_index_from_documents(&path, &metadata(), documents)
            .expect_err("duplicate document ids must reject index build");

        assert!(matches!(
            error,
            SearchError::DuplicateDocumentId {
                ref id,
                count: 2,
            } if id == "global_method:Сообщить"
        ));
        assert!(
            !path.exists(),
            "duplicate detection must run before SQLite index creation"
        );
    }

    #[test]
    fn streaming_builder_keeps_last_toc_marker_duplicate_with_warning() {
        let path = temp_path("builder-duplicate-document-id.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![platform_type_with_owner_path("ГруппаФормы", "Форма")],
            type_properties: vec![
                type_property_with_owner_path("ГруппаФормы", "Форма", "Видимость", "Булево"),
                type_property_with_owner_path(
                    "ГруппаФормы",
                    "Форма",
                    "Видимость#&^@^%&*^#1",
                    "Булево",
                ),
            ],
            ..model::PlatformContext::default()
        };

        let report = build_index_from_builder_with_report(
            &path,
            &metadata(),
            builder_from_context(&context),
        )
        .expect("TOC-marker duplicates must not reject index build");

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "DUPLICATE_DOCUMENT_ID");
        assert!(
            report.warnings[0]
                .message
                .contains("type_property:platform_type:ГруппаФормы:Видимость")
        );
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        assert_eq!(
            index
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM documents WHERE id = 'type_property:platform_type:ГруппаФормы:Видимость'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("document count must be readable"),
            1
        );
    }

    #[test]
    fn read_only_open_rejects_stale_schema_version() {
        let path = temp_path("stale-schema.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");
        {
            let connection = Connection::open(&path).expect("index must open for fixture mutation");
            connection
                .execute(
                    "UPDATE meta SET value = '2' WHERE key = 'schema_version'",
                    [],
                )
                .expect("schema version fixture mutation must work");
        }

        let error = match SearchIndex::open_read_only(&path) {
            Ok(_) => panic!("stale index must be rejected"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            SearchError::UnsupportedSchemaVersion {
                expected: INDEX_SCHEMA_VERSION,
                ..
            }
        ));
        assert!(
            error.to_string().contains("rebuild the index"),
            "stale schema error should tell the user how to recover"
        );
    }

    #[test]
    fn read_only_open_reports_missing_schema_metadata() {
        let path = temp_path("missing-schema-metadata.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");
        {
            let connection = Connection::open(&path).expect("index must open for fixture mutation");
            connection
                .execute("DELETE FROM meta WHERE key = 'schema_version'", [])
                .expect("schema metadata deletion must work");
        }

        match SearchIndex::open_read_only(&path) {
            Err(SearchError::MissingMetadata { key, .. }) => assert_eq!(key, "schema_version"),
            Ok(_) => panic!("expected missing schema metadata error, got open index"),
            Err(error) => panic!("expected missing schema metadata error, got {error}"),
        }
    }

    #[test]
    fn metadata_reader_reports_missing_metadata_key() {
        let path = temp_path("missing-metadata.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");
        {
            let connection = Connection::open(&path).expect("index must open for fixture mutation");
            connection
                .execute("DELETE FROM meta WHERE key = 'source_hbk'", [])
                .expect("metadata fixture mutation must work");
        }
        let index = SearchIndex::open_read_only(&path).expect("schema-compatible index must open");

        match index.metadata() {
            Err(SearchError::MissingMetadata { key, .. }) => assert_eq!(key, "source_hbk"),
            other => panic!("expected missing metadata error, got {other:?}"),
        }
    }

    #[test]
    fn query_connections_are_read_only_and_repeatable() {
        let path = temp_path("readonly.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");
        let left = SearchIndex::open_read_only(&path).expect("first reader must open");
        let right = SearchIndex::open_read_only(&path).expect("second reader must open");
        assert_eq!(
            left.get_by_name("ОтборКомпоновкиДанных").unwrap(),
            right.get_by_name("ОтборКомпоновкиДанных").unwrap()
        );
        let write_result = left.connection.execute("DELETE FROM documents", []);
        assert!(
            write_result.is_err(),
            "read-only connection must reject writes"
        );
    }

    #[test]
    fn rebuild_replaces_previous_complete_index() {
        let path = temp_path("replace.sqlite");
        let mut context = fixture_context();
        build_test_index_from_context(&path, &metadata(), &context)
            .expect("first index must build");
        fs::write(path.with_extension("sqlite-wal"), b"stale wal")
            .expect("stale wal sidecar must be writable");
        fs::write(path.with_extension("sqlite-shm"), b"stale shm")
            .expect("stale shm sidecar must be writable");
        context.platform_types[0].description = Some("updated description".to_string());
        build_test_index_from_context(&path, &metadata(), &context)
            .expect("replacement index must build");
        assert!(!path.with_extension("sqlite-wal").exists());
        assert!(!path.with_extension("sqlite-shm").exists());
        let index = SearchIndex::open_read_only(&path).expect("index must open");
        let exact = index.get_by_name("ОтборКомпоновкиДанных").unwrap();
        assert_eq!(
            exact[0].document.description.as_deref(),
            Some("updated description")
        );
    }

    #[test]
    fn rebuild_cleans_stale_temporary_index_artifacts_before_creation() {
        let path = temp_path("stale-temp.sqlite");
        let temp_path = temp_index_path(&path);
        fs::write(&temp_path, b"not a sqlite database").expect("stale temp file must be writable");
        fs::write(temp_path.with_extension("sqlite-wal"), b"stale temp wal")
            .expect("stale temp wal must be writable");
        fs::write(temp_path.with_extension("sqlite-shm"), b"stale temp shm")
            .expect("stale temp shm must be writable");

        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index build must clean stale temp artifacts first");

        assert!(!temp_path.exists());
        assert!(!temp_path.with_extension("sqlite-wal").exists());
        assert!(!temp_path.with_extension("sqlite-shm").exists());
        assert!(SearchIndex::open_read_only(&path).is_ok());
    }

    #[test]
    fn concurrent_writers_are_serialized_by_lock() {
        let path = temp_path("writers.sqlite");
        let left_path = path.clone();
        let right_path = path.clone();
        let left = std::thread::spawn(move || {
            build_test_index_from_context(left_path, &metadata(), &fixture_context())
        });
        let right = std::thread::spawn(move || {
            build_test_index_from_context(right_path, &metadata(), &fixture_context())
        });
        left.join()
            .expect("left writer must not panic")
            .expect("left writer must build");
        right
            .join()
            .expect("right writer must not panic")
            .expect("right writer must build");
        let index = SearchIndex::open_read_only(&path).expect("final index must open");
        assert!(index.document_count().expect("document count must work") > 0);
    }

    #[test]
    fn query_table_identity_uses_identifier_and_semantic_variant_for_members() {
        let context = model::PlatformContext {
            query_tables: vec![
                query_table(
                    "ОстаткиИОбороты",
                    "Таблицы регистра накопления",
                    "Основная таблица",
                ),
                query_table(
                    "ОстаткиИОбороты",
                    "Таблицы регистра бухгалтерии (без поддержки корреспонденции)",
                    "Основная таблица",
                ),
            ],
            table_fields: vec![query_table_field(
                "Основная таблица",
                "Таблицы регистра бухгалтерии (без поддержки корреспонденции)",
                "Сумма",
            )],
            table_parameters: vec![query_table_parameter(
                "Основная таблица",
                "Таблицы регистра накопления",
                "Период",
            )],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("query table identities must not collide")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"query_table:ОстаткиИОбороты:Таблицы регистра накопления"));
        assert!(ids.contains(
            &"query_table:ОстаткиИОбороты:Таблицы регистра бухгалтерии (без поддержки корреспонденции)"
        ));
        assert!(ids.contains(
            &"query_table_field:query_table:ОстаткиИОбороты:Таблицы регистра бухгалтерии (без поддержки корреспонденции):Сумма"
        ));
        assert!(ids.contains(
            &"query_table_parameter:query_table:ОстаткиИОбороты:Таблицы регистра накопления:Период"
        ));

        let relations = relations_from_documents(&documents);
        assert!(relations.iter().any(|relation| {
            relation.source_id == "query_table:ОстаткиИОбороты:Таблицы регистра накопления"
                && relation.target_id
                    == "query_table_parameter:query_table:ОстаткиИОбороты:Таблицы регистра накопления:Период"
        }));
        assert!(relations.iter().any(|relation| {
            relation.source_id
                == "query_table:ОстаткиИОбороты:Таблицы регистра бухгалтерии (без поддержки корреспонденции)"
                && relation.target_id
                    == "query_table_field:query_table:ОстаткиИОбороты:Таблицы регистра бухгалтерии (без поддержки корреспонденции):Сумма"
        }));
    }

    #[test]
    fn query_table_member_identity_uses_toc_shaped_parent_table_identity() {
        let mut table = query_table("Задача", "", "Основная таблица");
        table.semantic = semantic_path(
            model::RecordFamily::QueryTable,
            &["Работа с запросами", "Таблицы запросов", "Таблицы задач"],
        );
        let mut field = query_table_field("Основная таблица", "", "<Имя общего реквизита>");
        field.semantic = semantic_path(
            model::RecordFamily::QueryTableField,
            &[
                "Работа с запросами",
                "Таблицы запросов",
                "Таблицы задач",
                "Основная таблица",
            ],
        );
        let context = model::PlatformContext {
            query_tables: vec![table],
            table_fields: vec![field],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("query table member identity must use parent table")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"query_table:Задача"));
        assert!(ids.contains(&"query_table_field:query_table:Задача:<Имя общего реквизита>"));
        assert!(
            !ids.contains(&"query_table_field:query_table:Основная таблица:<Имя общего реквизита>")
        );

        let relations = relations_from_documents(&documents);
        assert!(relations.iter().any(|relation| {
            relation.source_id == "query_table:Задача"
                && relation.target_id
                    == "query_table_field:query_table:Задача:<Имя общего реквизита>"
        }));
    }

    #[test]
    fn missing_query_table_parent_identity_rejects_member_indexing() {
        let mut builder = SearchIndexBuilder::new();
        let mut table = query_table("Задача", "Таблицы задач", "Основная таблица");
        table.identity = Some("query_table:Задача".to_string());
        builder.query_table(table).unwrap();

        let mut field = query_table_field("Основная таблица", "", "<Имя общего реквизита>");
        field.semantic = semantic_path(
            model::RecordFamily::QueryTableField,
            &[
                "Работа с запросами",
                "Таблицы запросов",
                "Таблицы задач",
                "Основная таблица",
            ],
        );
        builder.table_field(field).unwrap();

        let error = match builder.into_documents("ru") {
            Ok(_) => panic!("missing parent identity must reject query table members"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            SearchError::MissingParentIdentity {
                ref kind,
                ref name,
                ..
            } if kind == "query_table_field" && name == "<Имя общего реквизита>"
        ));
    }

    #[test]
    fn member_identity_prefers_precomputed_parent_identity() {
        let mut task_table = query_table("Задача", "Таблицы задач", "Основная таблица");
        task_table.identity = Some("query_table:precomputed-task-table".to_string());
        let mut task_field = query_table_field("Основная таблица", "", "Номер");
        task_field.semantic = semantic_path(
            model::RecordFamily::QueryTableField,
            &["Таблицы задач", "Основная таблица"],
        );
        task_field.owner_identity = Some("query_table:precomputed-task-table".to_string());

        let mut form_items = platform_type_with_owner_path("ЭлементыФормы", "Формы");
        form_items.identity = Some("platform_type:ЭлементыФормы:precomputed".to_string());
        let mut form_property = type_property_with_owner_path(
            "ЭлементыФормы",
            "Формы:ЭлементыФормы",
            "ТекущийЭлемент",
            "ПолеФормы",
        );
        form_property.owner_identity = Some("platform_type:ЭлементыФормы:precomputed".to_string());

        let context = model::PlatformContext {
            query_tables: vec![task_table],
            table_fields: vec![task_field],
            platform_types: vec![form_items],
            type_properties: vec![form_property],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("precomputed parent identities must index")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"query_table_field:query_table:precomputed-task-table:Номер"));
        assert!(
            ids.contains(&"type_property:platform_type:ЭлементыФормы:precomputed:ТекущийЭлемент")
        );
        assert!(!ids.contains(&"query_table_field:query_table:Задача:Номер"));
        assert!(!ids.contains(&"type_property:platform_type:ЭлементыФормы:ТекущийЭлемент"));
    }

    #[test]
    fn type_event_identity_uses_semantic_owner_path_not_event_group() {
        let context = model::PlatformContext {
            global_context_events: vec![
                type_event_with_owner_path(&["Форма", "Поле формы", "События"], "ОбработкаВыбора"),
                type_event_with_owner_path(
                    &["Форма", "Табличное поле формы", "События"],
                    "ОбработкаВыбора",
                ),
            ],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("type event identities must use semantic owner")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"type_event:owner:Форма.Поле формы:ОбработкаВыбора"));
        assert!(ids.contains(&"type_event:owner:Форма.Табличное поле формы:ОбработкаВыбора"));
        assert!(!ids.contains(&"type_event:owner:События:ОбработкаВыбора"));
    }

    #[test]
    fn missing_syntax_query_table_identity_uses_semantic_owner_path() {
        let mut task_table = query_table("", "Таблицы задач", "Основная таблица");
        task_table.syntax = None;
        task_table.identifier = None;
        task_table.table_role = model::QueryTableRole::Unknown;
        let context = model::PlatformContext {
            query_tables: vec![task_table],
            table_fields: vec![query_table_field(
                "Основная таблица",
                "Таблицы задач",
                "Наименование",
            )],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("missing-syntax query table identities must not collide")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(!ids.contains(&"query_table:"));
        assert!(ids.contains(&"query_table:Таблицы задач:Основная таблица"));
        assert!(ids.contains(
            &"query_table_field:query_table:Таблицы задач:Основная таблица:Наименование"
        ));
    }

    #[test]
    fn type_identity_keeps_semantic_variants_and_strips_toc_markers() {
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type_with_owner_path("ЭлементыФормы", "Форма"),
                platform_type_with_owner_path("ЭлементыФормы", "ФормаКлиентскогоПриложения"),
                platform_type_with_owner_path("ГруппаФормы", "Форма"),
            ],
            type_properties: vec![
                type_property_with_owner_path("ЭлементыФормы", "Форма", "ТекущийЭлемент", "Строка"),
                type_property_with_owner_path(
                    "ЭлементыФормы",
                    "ФормаКлиентскогоПриложения",
                    "ТекущийЭлемент",
                    "Строка",
                ),
                type_property_with_owner_path("ГруппаФормы", "Форма", "Видимость", "Булево"),
            ],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("semantic type variants must not collide")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"platform_type:ЭлементыФормы:Форма"));
        assert!(ids.contains(&"platform_type:ЭлементыФормы:ФормаКлиентскогоПриложения"));
        assert!(ids.contains(&"type_property:platform_type:ЭлементыФормы:Форма:ТекущийЭлемент"));
        assert!(ids.contains(
            &"type_property:platform_type:ЭлементыФормы:ФормаКлиентскогоПриложения:ТекущийЭлемент"
        ));
        assert!(!ids.iter().any(|id| id.contains("#&^@^%&*^#")));
    }

    #[test]
    fn normalized_type_refs_do_not_choose_hidden_winner_for_duplicate_type_names() {
        let path = temp_path("duplicate-type-ref.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type_with_owner_path("ЭлементыФормы", "Форма"),
                platform_type_with_owner_path("ЭлементыФормы", "ФормаКлиентскогоПриложения"),
                platform_type_with_owner_path("ГруппаФормы", "Форма"),
            ],
            type_properties: vec![type_property_with_owner_path(
                "ГруппаФормы",
                "Форма",
                "Элементы",
                "ЭлементыФормы",
            )],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let duplicate_count: i64 = index
            .connection
            .query_row(
                "SELECT COUNT(*) FROM type_identities WHERE name_primary = 'ЭлементыФормы'",
                [],
                |row| row.get(0),
            )
            .expect("duplicate type identity count must be readable");
        assert_eq!(duplicate_count, 2);
        let target_type_id: Option<String> = index
            .connection
            .query_row(
                "SELECT target_type_id
                 FROM type_refs
                 WHERE source_document_id = 'type_property:platform_type:ГруппаФормы:Элементы'
                   AND ref_kind = 'property_type'
                   AND target_type_name = 'ЭлементыФормы'",
                [],
                |row| row.get(0),
            )
            .expect("ambiguous type ref row must exist");
        assert_eq!(target_type_id, None);

        let related_type_refs = index
            .related_by_id_and_edge(
                "type_property:platform_type:ГруппаФормы:Элементы",
                "has_type",
                20,
            )
            .expect("edge-filtered type refs must query normalized rows");
        assert!(
            related_type_refs.is_empty(),
            "edge-filtered traversal must not choose a hidden duplicate type identity"
        );
    }

    #[test]
    fn type_identity_lookup_returns_all_same_name_variants_deterministically() {
        let path = temp_path("duplicate-type-lookup.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type_with_owner_path("ЭлементыФормы", "Форма"),
                platform_type_with_owner_path("ЭлементыФормы", "ФормаКлиентскогоПриложения"),
            ],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hits = index
            .type_identities_by_name("ЭлементыФормы")
            .expect("type identity lookup must work");
        let ids = hits
            .iter()
            .map(|hit| hit.document.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            [
                "platform_type:ЭлементыФормы:Форма",
                "platform_type:ЭлементыФормы:ФормаКлиентскогоПриложения",
            ]
        );
    }

    #[test]
    fn type_identity_lookup_uses_indexed_sql_plan() {
        let path = temp_path("type-lookup-query-plan.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type_with_owner_path("ЭлементыФормы", "Форма"),
                platform_type_with_owner_path("ЭлементыФормы", "ФормаКлиентскогоПриложения"),
            ],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let mut statement = index
            .connection
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary,
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since
                 FROM document_names n
                 JOIN type_identities t ON t.document_id = n.document_id
                 JOIN documents d ON d.id = t.document_id
                 WHERE n.key = ?1
                   AND n.key_kind = ?2
                 ORDER BY d.kind_priority, d.id",
            )
            .expect("query plan must prepare");
        let plan = statement
            .query_map(
                [normalize_lookup_key("ЭлементыФормы"), "primary".to_string()],
                |row| row.get::<_, String>(3),
            )
            .expect("query plan must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("query plan rows must be readable");

        assert!(
            plan.iter()
                .any(|detail| detail.contains("document_names_key_idx"))
        );
        assert!(
            plan.iter()
                .any(|detail| detail.contains("type_identities_document_idx"))
        );
        assert!(
            !plan.iter().any(|detail| detail == "SCAN t"),
            "type identity lookup must not scan all type identities: {plan:?}"
        );
    }

    #[test]
    fn owner_type_exact_lookups_use_indexed_sql_plan() {
        let path = temp_path("owner-type-lookup-query-plan.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let mut member_statement = index
            .connection
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary,
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since
                 FROM document_names n
                 JOIN members m INDEXED BY members_document_owner_idx
                   ON m.document_id = n.document_id
                  AND m.owner_type_id = ?1
                 JOIN documents d ON d.id = m.document_id
                 WHERE n.key = ?2
                   AND n.key_kind IN ('primary', 'alias')
                 ORDER BY d.kind_priority, m.name_primary, d.id",
            )
            .expect("member query plan must prepare");
        let member_plan = member_statement
            .query_map(
                params![
                    "platform_type:НастройкиКомпоновкиДанных",
                    normalize_lookup_key("Отбор")
                ],
                |row| row.get::<_, String>(3),
            )
            .expect("member query plan must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("member query plan rows must be readable");
        assert!(
            member_plan
                .iter()
                .any(|detail| detail.contains("document_names_key_idx"))
        );
        assert!(
            member_plan
                .iter()
                .any(|detail| detail.contains("members_document_owner_idx")),
            "member lookup must use document/owner index: {member_plan:?}"
        );
        assert!(
            !member_plan.iter().any(|detail| detail == "SCAN m"),
            "member lookup must not scan all member rows: {member_plan:?}"
        );

        let mut callable_statement = index
            .connection
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary,
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since
                 FROM document_names n
                 JOIN callables c INDEXED BY callables_document_owner_idx
                   ON c.document_id = n.document_id
                  AND c.owner_type_id = ?1
                 JOIN documents d ON d.id = c.document_id
                 WHERE n.key = ?2
                   AND n.key_kind IN ('primary', 'alias')
                 ORDER BY d.kind_priority, d.name_primary, d.id",
            )
            .expect("callable query plan must prepare");
        let callable_plan = callable_statement
            .query_map(
                params![
                    "platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных",
                    normalize_lookup_key("Добавить")
                ],
                |row| row.get::<_, String>(3),
            )
            .expect("callable query plan must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("callable query plan rows must be readable");
        assert!(
            callable_plan
                .iter()
                .any(|detail| detail.contains("document_names_key_idx"))
        );
        assert!(
            callable_plan
                .iter()
                .any(|detail| detail.contains("callables_document_owner_idx")),
            "callable lookup must use document/owner index: {callable_plan:?}"
        );
        assert!(
            !callable_plan.iter().any(|detail| detail == "SCAN c"),
            "callable lookup must not scan all callable rows: {callable_plan:?}"
        );
    }

    #[test]
    fn enum_identity_distinguishes_metadata_property_enums() {
        let mut enum_value = enum_value("Видимость", "Использовать");
        enum_value.owner_identity = Some("enum:system:Видимость".to_string());
        let context = model::PlatformContext {
            enums: vec![
                enum_definition("Видимость", "objects/catalog2/catalog999/Visible.html"),
                enum_definition(
                    "Видимость",
                    "objects/catalog1649/Form/properties/Visible.html",
                ),
            ],
            enum_values: vec![enum_value],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("enum semantic variants must not collide")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"enum:system:Видимость"));
        assert!(ids.contains(&"enum:metadata_property:Видимость"));
        assert_eq!(
            ids.iter()
                .filter(|id| **id == "enum_value:enum:system:Видимость:Использовать")
                .count(),
            1
        );
    }

    #[test]
    fn enum_identity_uses_alias_variant_for_duplicate_system_enum_names() {
        let context = model::PlatformContext {
            enums: vec![
                enum_definition_with_alias(
                    "ИспользованиеТекущейСтроки",
                    "SelectedRowsUse",
                    "objects/catalog2/catalog111/SelectedRowsUse.html",
                ),
                enum_definition_with_alias(
                    "ИспользованиеТекущейСтроки",
                    "CurrentRowUse",
                    "objects/catalog2/catalog222/CurrentRowUse.html",
                ),
            ],
            enum_values: vec![
                enum_value_with_owner_alias(
                    "ИспользованиеТекущейСтроки",
                    "SelectedRowsUse",
                    "Авто",
                ),
                enum_value_with_owner_alias("ИспользованиеТекущейСтроки", "CurrentRowUse", "Авто"),
            ],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("alias-backed enum variants must not collide")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"enum:system:ИспользованиеТекущейСтроки:SelectedRowsUse"));
        assert!(ids.contains(&"enum:system:ИспользованиеТекущейСтроки:CurrentRowUse"));
        assert!(
            ids.contains(&"enum_value:enum:system:ИспользованиеТекущейСтроки:SelectedRowsUse:Авто")
        );
        assert!(
            ids.contains(&"enum_value:enum:system:ИспользованиеТекущейСтроки:CurrentRowUse:Авто")
        );
    }

    fn metadata() -> IndexMetadata {
        IndexMetadata {
            locale: "ru".to_string(),
            source_locale: "ru".to_string(),
            source_hbk: "fixture.hbk".to_string(),
            source_extraction_schema_version: 11,
        }
    }

    fn language_fixture_facts(locale: &str) -> Vec<language::LanguageFact> {
        let suffix = match locale {
            "root" => "root",
            _ => "ru",
        };
        let mut fixtures = vec![
            (
                LanguageSourceFamily::Shlang,
                "def_String",
                format!("shlang_def_string_{suffix}.html"),
            ),
            (
                LanguageSourceFamily::Shlang,
                "def_Func",
                format!("shlang_def_func_{suffix}.html"),
            ),
            (
                LanguageSourceFamily::Shquery,
                "SELECTStatement",
                format!("shquery_select_statement_{suffix}.html"),
            ),
            (
                LanguageSourceFamily::Shquery,
                "SUM",
                format!("shquery_sum_{suffix}.html"),
            ),
            (
                LanguageSourceFamily::Shquery,
                "STRING",
                format!("shquery_string_{suffix}.html"),
            ),
            (
                LanguageSourceFamily::Dcsui,
                "SKD_Functions_Strings",
                format!("dcsui_functions_strings_{suffix}.html"),
            ),
        ];
        if locale == "ru" {
            fixtures.push((
                LanguageSourceFamily::Shquery,
                "LitString",
                "shquery_lit_string_ru.html".to_string(),
            ));
        }
        fixtures
            .into_iter()
            .flat_map(|(source_family, html_path, fixture_name)| {
                let html = std::fs::read_to_string(
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../tests/fixtures/syntax-helper-language")
                        .join(&fixture_name),
                )
                .expect("language fixture must be readable");
                extract_language_facts(LanguagePageInput {
                    source_hbk: "fixture.hbk",
                    source_family,
                    locale,
                    html_path,
                    html: &html,
                })
            })
            .collect()
    }

    fn language_fact(
        id: &str,
        source_family: LanguageSourceFamily,
        domain: language::LanguageDomain,
        family: language::LanguageFactFamily,
        name: model::LocalizedName,
    ) -> language::LanguageFact {
        language::LanguageFact {
            id: id.to_string(),
            source_family,
            domain,
            family,
            name,
            syntax: None,
            signatures: Vec::new(),
            type_refs: Vec::new(),
            return_types: Vec::new(),
            description: None,
            provenance: language::LanguageFactProvenance {
                source_hbk: "fixture.hbk".to_string(),
                locale: "ru".to_string(),
                html_path: id.to_string(),
                page_title: id.to_string(),
                anchor: None,
            },
        }
    }

    fn fixture_context() -> model::PlatformContext {
        model::PlatformContext {
            platform_types: vec![
                platform_type(
                    "ОтборКомпоновкиДанных",
                    Some("DataCompositionFilter"),
                    "Объект системы компоновки данных для настройки отбора.",
                ),
                platform_type(
                    "НастройкиКомпоновкиДанных",
                    Some("DataCompositionSettings"),
                    "Настройки системы компоновки данных.",
                ),
                platform_type(
                    "КоллекцияЭлементовОтбораКомпоновкиДанных",
                    Some("DataCompositionFilterItems"),
                    "Коллекция элементов отбора.",
                ),
                platform_type(
                    "ЭлементОтбораКомпоновкиДанных",
                    Some("DataCompositionFilterItem"),
                    "Элемент отбора.",
                ),
                platform_type(
                    "БиблиотекаКартинок",
                    Some("PictureLib"),
                    "Библиотека картинок.",
                ),
            ],
            type_properties: vec![
                type_property("БиблиотекаКартинок", "ОтборКомпоновкиДанных", "Картинка"),
                type_property(
                    "НастройкиКомпоновкиДанных",
                    "Отбор",
                    "ОтборКомпоновкиДанных",
                ),
                type_property(
                    "ОтборКомпоновкиДанных",
                    "Элементы",
                    "КоллекцияЭлементовОтбораКомпоновкиДанных",
                ),
                type_property(
                    "ЭлементОтбораКомпоновкиДанных",
                    "ЛевоеЗначение",
                    "Произвольный",
                ),
            ],
            type_methods: vec![
                type_method(
                    "ОтборКомпоновкиДанных",
                    "ПолучитьОбъектПоИдентификатору",
                    "ЭлементОтбораКомпоновкиДанных",
                ),
                type_method(
                    "КоллекцияЭлементовОтбораКомпоновкиДанных",
                    "Добавить",
                    "ЭлементОтбораКомпоновкиДанных",
                ),
            ],
            constructors: vec![constructor(
                "ОтборКомпоновкиДанных",
                "Новый ОтборКомпоновкиДанных()",
            )],
            global_context_events: vec![type_event("ОтборКомпоновкиДанных", "ПередЗаписью")],
            ..model::PlatformContext::default()
        }
    }

    fn builder_from_context(context: &model::PlatformContext) -> SearchIndexBuilder {
        let context = context_with_test_owner_identities(context);
        let mut builder = SearchIndexBuilder::new();
        for record in context.global_contexts.iter().cloned() {
            builder.global_context(record).unwrap();
        }
        for record in context.global_methods.iter().cloned() {
            builder.global_method(record).unwrap();
        }
        for record in context.global_properties.iter().cloned() {
            builder.global_property(record).unwrap();
        }
        for record in context.global_context_events.iter().cloned() {
            builder.global_context_event(record).unwrap();
        }
        for record in context.platform_types.iter().cloned() {
            builder.platform_type(record).unwrap();
        }
        for record in context.query_tables.iter().cloned() {
            builder.query_table(record).unwrap();
        }
        for record in context.type_methods.iter().cloned() {
            builder.type_method(record).unwrap();
        }
        for record in context.type_properties.iter().cloned() {
            builder.type_property(record).unwrap();
        }
        for record in context.table_fields.iter().cloned() {
            builder.table_field(record).unwrap();
        }
        for record in context.table_parameters.iter().cloned() {
            builder.table_parameter(record).unwrap();
        }
        for record in context.constructors.iter().cloned() {
            builder.constructor(record).unwrap();
        }
        for record in context.enums.iter().cloned() {
            builder.enum_definition(record).unwrap();
        }
        for record in context.enum_values.iter().cloned() {
            builder.enum_value(record).unwrap();
        }
        for record in context.diagnostics.iter().cloned() {
            builder.diagnostic(record).unwrap();
        }
        builder
    }

    fn context_with_test_owner_identities(
        context: &model::PlatformContext,
    ) -> model::PlatformContext {
        let mut context = context.clone();
        let identities = DocumentIdentities::from_inputs(
            &context
                .platform_types
                .iter()
                .map(|record| PlatformTypeIdentityInput {
                    identity: record.identity.clone(),
                    name_primary: record.name.primary.clone(),
                    semantic: record.semantic.clone(),
                })
                .collect::<Vec<_>>(),
            &context
                .query_tables
                .iter()
                .map(|record| QueryTableIdentityInput {
                    identity: record.identity.clone(),
                    name_primary: record.name.clone(),
                    identifier: record.identifier.clone(),
                    semantic: record.semantic.clone(),
                })
                .collect::<Vec<_>>(),
            &context
                .enums
                .iter()
                .map(|record| EnumIdentityInput {
                    identity: record.identity.clone(),
                    name_primary: record.name.primary.clone(),
                    name_alias: record.name.alias.clone(),
                    source_html_path: record.source.html_path.clone(),
                })
                .collect::<Vec<_>>(),
        );
        for record in &mut context.type_methods {
            if record.owner_identity.is_none() {
                record.owner_identity = identities
                    .platform_type_ids
                    .get(&model::platform_type_semantic_key(
                        &record.owner.primary,
                        &record.semantic,
                    ))
                    .cloned();
            }
        }
        for record in &mut context.type_properties {
            if record.owner_identity.is_none() {
                record.owner_identity = identities
                    .platform_type_ids
                    .get(&model::platform_type_semantic_key(
                        &record.owner.primary,
                        &record.semantic,
                    ))
                    .cloned();
            }
        }
        for record in &mut context.constructors {
            if record.owner_identity.is_none() {
                record.owner_identity = identities
                    .platform_type_ids
                    .get(&model::platform_type_semantic_key(
                        &record.owner.primary,
                        &record.semantic,
                    ))
                    .cloned();
            }
        }
        for record in &mut context.table_fields {
            if record.owner_identity.is_none() {
                record.owner_identity = identities
                    .query_table_ids
                    .get(&model::query_table_semantic_key(
                        &record.semantic,
                        &record.owner.primary,
                    ))
                    .cloned();
            }
        }
        for record in &mut context.table_parameters {
            if record.owner_identity.is_none() {
                record.owner_identity = identities
                    .query_table_ids
                    .get(&model::query_table_semantic_key(
                        &record.semantic,
                        &record.owner.primary,
                    ))
                    .cloned();
            }
        }
        for record in &mut context.enum_values {
            if record.owner_identity.is_none() {
                record.owner_identity =
                    unique_enum_owner_identity(&identities, &context.enums, record);
            }
        }
        context
    }

    fn unique_enum_owner_identity(
        identities: &DocumentIdentities,
        enums: &[model::EnumDefinition],
        enum_value: &model::EnumValue,
    ) -> Option<String> {
        let mut matches = enums.iter().filter_map(|enum_definition| {
            enum_definition
                .name
                .matches(
                    enum_value
                        .owner
                        .alias
                        .as_deref()
                        .unwrap_or(&enum_value.owner.primary),
                )
                .then(|| {
                    identities
                        .enum_ids
                        .get(&enum_record_key(
                            &enum_definition.name.primary,
                            &enum_definition.source.html_path,
                        ))
                        .cloned()
                })
                .flatten()
        });
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
    }

    fn build_test_index_from_context(
        path: impl AsRef<Path>,
        metadata: &IndexMetadata,
        context: &model::PlatformContext,
    ) -> Result<(), SearchError> {
        build_index_from_builder(path, metadata, builder_from_context(context))
    }

    fn platform_type(primary: &str, alias: Option<&str>, description: &str) -> model::PlatformType {
        model::PlatformType {
            identity: None,
            name: name(primary, alias),
            semantic: model::SemanticContext::default(),
            type_kind: model::PlatformTypeKind::Regular,
            object_kind: Some(model::PlatformObjectKind::RegularPlatformType),
            extends: Vec::new(),
            metadata_kind: None,
            template_parameters: Vec::new(),
            type_template_key: None,
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: Some(description.to_string()),
            facts: model::SectionFacts::default(),
            source: source(primary),
        }
    }

    fn platform_type_with_owner_path(primary: &str, owner: &str) -> model::PlatformType {
        let mut record = platform_type(primary, None, "type description");
        record.semantic = semantic(model::RecordFamily::PlatformType, owner);
        record
    }

    fn type_property(owner: &str, primary: &str, type_ref: &str) -> model::PlatformProperty {
        model::PlatformProperty {
            owner: name(owner, None),
            owner_identity: None,
            name: name(primary, None),
            semantic: model::SemanticContext::default(),
            usage: None,
            type_refs: vec![model::TypeRef {
                name: type_ref.to_string(),
            }],
            description: Some(format!("{primary} description")),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn type_property_with_owner_path(
        owner: &str,
        owner_path: &str,
        primary: &str,
        type_ref: &str,
    ) -> model::PlatformProperty {
        let mut record = type_property(owner, primary, type_ref);
        record.semantic = semantic(model::RecordFamily::TypeProperty, owner_path);
        record
    }

    fn query_table(identifier: &str, owner_path: &str, table_name: &str) -> model::QueryTable {
        model::QueryTable {
            identity: None,
            name: table_name.to_string(),
            syntax: None,
            identifier: (!identifier.is_empty()).then(|| identifier.to_string()),
            semantic: semantic(model::RecordFamily::QueryTable, owner_path),
            table_role: model::QueryTableRole::Primary,
            description: Some("table description".to_string()),
            source: source(table_name),
        }
    }

    fn query_table_field(owner: &str, owner_path: &str, primary: &str) -> model::QueryTableField {
        model::QueryTableField {
            owner: name(owner, None),
            owner_identity: None,
            name: primary.to_string(),
            semantic: semantic(model::RecordFamily::QueryTableField, owner_path),
            type_refs: Vec::new(),
            description: Some("field description".to_string()),
            note: None,
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn query_table_parameter(
        owner: &str,
        owner_path: &str,
        primary: &str,
    ) -> model::QueryTableParameter {
        model::QueryTableParameter {
            owner: name(owner, None),
            owner_identity: None,
            name: primary.to_string(),
            semantic: semantic(model::RecordFamily::QueryTableParameter, owner_path),
            type_refs: Vec::new(),
            description: Some("parameter description".to_string()),
            default_value: None,
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn enum_definition(primary: &str, html_path: &str) -> model::EnumDefinition {
        enum_definition_with_alias(primary, "", html_path)
    }

    fn enum_definition_with_alias(
        primary: &str,
        alias: &str,
        html_path: &str,
    ) -> model::EnumDefinition {
        model::EnumDefinition {
            identity: None,
            name: name(primary, (!alias.is_empty()).then_some(alias)),
            value_links: Vec::new(),
            description: Some("enum description".to_string()),
            facts: model::SectionFacts::default(),
            source: source_with_html_path(primary, html_path),
        }
    }

    fn enum_value(owner: &str, primary: &str) -> model::EnumValue {
        enum_value_with_owner_alias(owner, "", primary)
    }

    fn enum_value_with_owner_alias(
        owner: &str,
        owner_alias: &str,
        primary: &str,
    ) -> model::EnumValue {
        model::EnumValue {
            owner: name(owner, (!owner_alias.is_empty()).then_some(owner_alias)),
            owner_identity: None,
            name: name(primary, None),
            description: Some("value description".to_string()),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn semantic(record_family: model::RecordFamily, owner_path: &str) -> model::SemanticContext {
        semantic_path(record_family, &[owner_path])
    }

    fn semantic_path(
        record_family: model::RecordFamily,
        owner_path: &[&str],
    ) -> model::SemanticContext {
        model::SemanticContext::new(model::BranchKind::PlatformObjects, record_family)
            .with_owner_path(owner_path.iter().map(|value| name(value, None)).collect())
    }

    fn type_method(owner: &str, primary: &str, return_type: &str) -> model::PlatformMethod {
        model::PlatformMethod {
            owner: name(owner, None),
            owner_identity: None,
            name: name(primary, None),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: format!("{primary}()"),
                parameters: Vec::new(),
                variant: None,
            }],
            return_types: vec![model::TypeRef {
                name: return_type.to_string(),
            }],
            description: Some(format!("{primary} description")),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn constructor(owner: &str, signature: &str) -> model::Constructor {
        constructor_with_name(owner, signature, signature)
    }

    fn constructor_with_name(owner: &str, primary: &str, signature: &str) -> model::Constructor {
        model::Constructor {
            owner: name(owner, None),
            owner_identity: None,
            name: name(primary, None),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: signature.to_string(),
                parameters: Vec::new(),
                variant: None,
            }],
            description: Some(format!("{primary} description")),
            facts: model::SectionFacts::default(),
            source: source(signature),
        }
    }

    fn http_connection_constructor() -> model::Constructor {
        model::Constructor {
            owner: name("HTTPСоединение", Some("HTTPConnection")),
            owner_identity: None,
            name: name("По параметрам соединения", None),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: "Новый HTTPСоединение(<Сервер>, <Порт>, <ИспользоватьАутентификациюОС>)"
                    .to_string(),
                parameters: vec![
                    model::Parameter {
                        name: "Сервер".to_string(),
                        required: true,
                        type_refs: vec![model::TypeRef {
                            name: "Строка".to_string(),
                        }],
                        description: Some("Имя сервера.".to_string()),
                    },
                    model::Parameter {
                        name: "Порт".to_string(),
                        required: false,
                        type_refs: vec![model::TypeRef {
                            name: "Число".to_string(),
                        }],
                        description: Some("Порт соединения.".to_string()),
                    },
                    model::Parameter {
                        name: "ИспользоватьАутентификациюОС".to_string(),
                        required: false,
                        type_refs: vec![model::TypeRef {
                            name: "Булево".to_string(),
                        }],
                        description: Some(
                            "Использовать аутентификацию операционной системы.".to_string(),
                        ),
                    },
                ],
                variant: None,
            }],
            description: None,
            facts: model::SectionFacts::default(),
            source: source("HTTPСоединение"),
        }
    }

    fn type_event(owner: &str, primary: &str) -> model::GlobalContextEvent {
        type_event_with_owner_path(&[owner], primary)
    }

    fn type_event_with_owner_path(owner_path: &[&str], primary: &str) -> model::GlobalContextEvent {
        model::GlobalContextEvent {
            name: name(primary, Some("BeforeWrite")),
            semantic: model::SemanticContext::new(
                model::BranchKind::PlatformObjects,
                model::RecordFamily::TypeEvent,
            )
            .with_owner_path(owner_path.iter().map(|owner| name(owner, None)).collect()),
            module: model::ModuleEventContext::default(),
            signatures: vec![model::Signature {
                text: format!("{primary}()"),
                parameters: Vec::new(),
                variant: None,
            }],
            description: Some("event description".to_string()),
            facts: model::SectionFacts::default(),
            source: source(&format!("{}.{}", owner_path.join("."), primary)),
        }
    }

    fn name(primary: &str, alias: Option<&str>) -> model::LocalizedName {
        model::LocalizedName {
            primary: primary.to_string(),
            alias: alias.map(ToOwned::to_owned),
        }
    }

    fn source(name: &str) -> model::SyntaxHelperSource {
        source_with_html_path(name, &format!("{name}.html"))
    }

    fn source_with_html_path(name: &str, html_path: &str) -> model::SyntaxHelperSource {
        model::SyntaxHelperSource {
            hbk_path: PathBuf::from("fixture.hbk"),
            locale: "ru".to_string(),
            toc_path: Some(name.to_string()),
            html_path: html_path.to_string(),
            page_title: name.to_string(),
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "v8-context-hbk-search-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        path
    }

    fn table_columns(connection: &Connection, table: &str) -> Vec<String> {
        let mut statement = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("table info statement must prepare");
        statement
            .query_map([], |row| row.get(1))
            .expect("table info query must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("table info rows must parse")
    }
}
