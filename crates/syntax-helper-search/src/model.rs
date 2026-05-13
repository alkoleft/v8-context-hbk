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
pub struct TypeReferenceGapReport {
    pub total: usize,
    pub resolved: usize,
    pub unresolved: usize,
    pub ambiguous: usize,
    pub template_bindings: usize,
    pub roles: Vec<TypeReferenceRoleReport>,
    pub top_unresolved: Vec<TypeReferenceGap>,
    pub top_ambiguous: Vec<TypeReferenceGap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeReferenceRoleReport {
    pub role: String,
    pub total: usize,
    pub resolved: usize,
    pub unresolved: usize,
    pub ambiguous: usize,
    pub template_bindings: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeReferenceGap {
    pub role: String,
    pub target_type_name: String,
    pub count: usize,
    pub examples: Vec<TypeReferenceGapExample>,
    pub candidate_type_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeReferenceGapExample {
    pub source_document_id: String,
    pub source_kind: SearchDocumentKind,
    pub source_name: model::LocalizedName,
    pub source_owner: Option<model::LocalizedName>,
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

#[derive(Debug, Error)]
#[error("unknown search document kind '{0}'")]
struct UnknownSearchDocumentKind(String);

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
    pub return_types: Vec<String>,
    pub return_type_facts: Vec<SearchTypeRef>,
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
    pub target: SearchTypeRefTarget,
    pub type_template_key: Option<model::PlatformTypeTemplateKey>,
    pub template_binding: Option<model::TypeTemplateBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchTypeRefTarget {
    Ok(String),
    Unresolved,
    Ambiguous(Vec<String>),
}

impl SearchTypeRefTarget {
    pub fn target_type_id(&self) -> Option<&str> {
        match self {
            Self::Ok(type_id) => Some(type_id.as_str()),
            Self::Unresolved | Self::Ambiguous(_) => None,
        }
    }

    pub fn candidate_type_ids(&self) -> &[String] {
        match self {
            Self::Ambiguous(candidates) => candidates,
            Self::Ok(_) | Self::Unresolved => &[],
        }
    }
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
