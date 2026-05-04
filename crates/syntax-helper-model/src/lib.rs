use std::convert::Infallible;
use std::fmt;
use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RootDiscovery {
    pub roots: Vec<RootSection>,
    pub diagnostics: Vec<SyntaxHelperDiagnostic>,
}

impl RootDiscovery {
    pub fn has_kind(&self, kind: RootSectionKind) -> bool {
        self.roots.iter().any(|root| root.kind == kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RootSection {
    pub kind: RootSectionKind,
    pub source: SyntaxHelperSource,
    pub pages: Vec<CatalogPage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootSectionKind {
    GlobalContext,
    EnumCatalog,
    TypeObjectCatalog,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CatalogPage {
    pub class: PageClass,
    pub semantic: SemanticContext,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PageClass {
    Catalog,
    GlobalMethod,
    GlobalProperty,
    ModuleEvent,
    TypeEvent,
    UnknownEvent,
    ObjectType,
    QueryTable,
    ObjectMethod,
    ObjectProperty,
    QueryTableField,
    QueryTableParameter,
    Constructor,
    Enum,
    EnumValue,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchKind {
    GlobalContext,
    SystemEnums,
    PrimitiveTypes,
    MetadataObjects,
    ManagedForms,
    QueryTables,
    PlatformObjects,
    AutomationExternalApi,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordFamily {
    Catalog,
    GlobalMethod,
    GlobalProperty,
    ModuleEvent,
    TypeEvent,
    UnknownEvent,
    PlatformType,
    QueryTable,
    TypeMethod,
    TypeProperty,
    TypeConstructor,
    SystemEnum,
    SystemEnumValue,
    QueryTableField,
    QueryTableParameter,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SemanticContext {
    pub branch_kind: BranchKind,
    pub record_family: RecordFamily,
    pub owner_path: Vec<LocalizedName>,
}

impl SemanticContext {
    pub fn new(branch_kind: BranchKind, record_family: RecordFamily) -> Self {
        Self {
            branch_kind,
            record_family,
            owner_path: Vec::new(),
        }
    }

    pub fn with_owner_path(mut self, owner_path: Vec<LocalizedName>) -> Self {
        self.owner_path = owner_path;
        self
    }
}

impl Default for SemanticContext {
    fn default() -> Self {
        Self::new(BranchKind::Unknown, RecordFamily::Unknown)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyntaxHelperSource {
    pub hbk_path: PathBuf,
    pub locale: String,
    pub toc_path: Option<String>,
    pub html_path: String,
    pub page_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyntaxHelperDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub source: SyntaxHelperSource,
    pub parser_stage: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct PlatformContext {
    pub global_contexts: Vec<GlobalContext>,
    pub global_methods: Vec<GlobalMethod>,
    pub global_properties: Vec<GlobalProperty>,
    pub global_context_events: Vec<GlobalContextEvent>,
    pub platform_types: Vec<PlatformType>,
    pub query_tables: Vec<QueryTable>,
    pub type_methods: Vec<PlatformMethod>,
    pub type_properties: Vec<PlatformProperty>,
    pub table_fields: Vec<QueryTableField>,
    pub table_parameters: Vec<QueryTableParameter>,
    pub constructors: Vec<Constructor>,
    pub enums: Vec<EnumDefinition>,
    pub enum_values: Vec<EnumValue>,
    pub diagnostics: Vec<SyntaxHelperDiagnostic>,
}

impl PlatformContext {
    pub fn find_global_member(
        &self,
        name: &str,
    ) -> Result<Option<GlobalMemberRef<'_>>, LookupError> {
        let mut matches = self
            .global_methods
            .iter()
            .filter(|method| method.name.matches(name))
            .map(GlobalMemberRef::Method)
            .chain(
                self.global_properties
                    .iter()
                    .filter(|property| property.name.matches(name))
                    .map(GlobalMemberRef::Property),
            );
        one_or_ambiguous(&mut matches, LookupKind::GlobalMember, name)
    }

    pub fn find_type(&self, name: &str) -> Result<Option<&PlatformType>, LookupError> {
        let mut matches = self
            .platform_types
            .iter()
            .filter(|platform_type| platform_type.name.matches(name));
        one_or_ambiguous(&mut matches, LookupKind::Type, name)
    }

    pub fn find_type_member(
        &self,
        type_name: &str,
        member_name: &str,
    ) -> Result<Option<TypeMemberRef<'_>>, LookupError> {
        let Some(platform_type) = self.find_type(type_name)? else {
            return Ok(None);
        };
        let mut matches = self
            .type_methods
            .iter()
            .filter(|method| method.owner == platform_type.name && method.name.matches(member_name))
            .map(TypeMemberRef::Method)
            .chain(
                self.type_properties
                    .iter()
                    .filter(|property| {
                        property.owner == platform_type.name && property.name.matches(member_name)
                    })
                    .map(TypeMemberRef::Property),
            );
        one_or_ambiguous(&mut matches, LookupKind::TypeMember, member_name)
    }

    pub fn constructors_for_type(
        &self,
        type_name: &str,
    ) -> Result<Option<Vec<&Constructor>>, LookupError> {
        let Some(platform_type) = self.find_type(type_name)? else {
            return Ok(None);
        };
        Ok(Some(
            self.constructors
                .iter()
                .filter(|constructor| constructor.owner == platform_type.name)
                .collect(),
        ))
    }
}

pub trait SyntaxHelperSink {
    type Error;

    fn record_detail_mode(&self) -> SyntaxHelperRecordDetailMode {
        SyntaxHelperRecordDetailMode::Full
    }

    fn global_context(&mut self, record: GlobalContext) -> Result<(), Self::Error>;
    fn global_method(&mut self, record: GlobalMethod) -> Result<(), Self::Error>;
    fn global_property(&mut self, record: GlobalProperty) -> Result<(), Self::Error>;
    fn global_context_event(&mut self, record: GlobalContextEvent) -> Result<(), Self::Error>;
    fn platform_type(&mut self, record: PlatformType) -> Result<(), Self::Error>;
    fn query_table(&mut self, record: QueryTable) -> Result<(), Self::Error>;
    fn type_method(&mut self, record: PlatformMethod) -> Result<(), Self::Error>;
    fn type_property(&mut self, record: PlatformProperty) -> Result<(), Self::Error>;
    fn table_field(&mut self, record: QueryTableField) -> Result<(), Self::Error>;
    fn table_parameter(&mut self, record: QueryTableParameter) -> Result<(), Self::Error>;
    fn constructor(&mut self, record: Constructor) -> Result<(), Self::Error>;
    fn enum_definition(&mut self, record: EnumDefinition) -> Result<(), Self::Error>;
    fn enum_value(&mut self, record: EnumValue) -> Result<(), Self::Error>;
    fn diagnostic(&mut self, record: SyntaxHelperDiagnostic) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxHelperRecordDetailMode {
    Full,
    LeanConsumerExport,
}

impl SyntaxHelperSink for PlatformContext {
    type Error = Infallible;

    fn global_context(&mut self, record: GlobalContext) -> Result<(), Self::Error> {
        self.global_contexts.push(record);
        Ok(())
    }

    fn global_method(&mut self, record: GlobalMethod) -> Result<(), Self::Error> {
        self.global_methods.push(record);
        Ok(())
    }

    fn global_property(&mut self, record: GlobalProperty) -> Result<(), Self::Error> {
        self.global_properties.push(record);
        Ok(())
    }

    fn global_context_event(&mut self, record: GlobalContextEvent) -> Result<(), Self::Error> {
        self.global_context_events.push(record);
        Ok(())
    }

    fn platform_type(&mut self, record: PlatformType) -> Result<(), Self::Error> {
        self.platform_types.push(record);
        Ok(())
    }

    fn query_table(&mut self, record: QueryTable) -> Result<(), Self::Error> {
        self.query_tables.push(record);
        Ok(())
    }

    fn type_method(&mut self, record: PlatformMethod) -> Result<(), Self::Error> {
        self.type_methods.push(record);
        Ok(())
    }

    fn type_property(&mut self, record: PlatformProperty) -> Result<(), Self::Error> {
        self.type_properties.push(record);
        Ok(())
    }

    fn table_field(&mut self, record: QueryTableField) -> Result<(), Self::Error> {
        self.table_fields.push(record);
        Ok(())
    }

    fn table_parameter(&mut self, record: QueryTableParameter) -> Result<(), Self::Error> {
        self.table_parameters.push(record);
        Ok(())
    }

    fn constructor(&mut self, record: Constructor) -> Result<(), Self::Error> {
        self.constructors.push(record);
        Ok(())
    }

    fn enum_definition(&mut self, record: EnumDefinition) -> Result<(), Self::Error> {
        self.enums.push(record);
        Ok(())
    }

    fn enum_value(&mut self, record: EnumValue) -> Result<(), Self::Error> {
        self.enum_values.push(record);
        Ok(())
    }

    fn diagnostic(&mut self, record: SyntaxHelperDiagnostic) -> Result<(), Self::Error> {
        self.diagnostics.push(record);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalMemberRef<'a> {
    Method(&'a GlobalMethod),
    Property(&'a GlobalProperty),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeMemberRef<'a> {
    Method(&'a PlatformMethod),
    Property(&'a PlatformProperty),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupError {
    Ambiguous { kind: LookupKind, name: String },
}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ambiguous { kind, name } => {
                write!(
                    f,
                    "ambiguous Syntax Assistant {} lookup for '{name}'",
                    kind.label()
                )
            }
        }
    }
}

impl std::error::Error for LookupError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupKind {
    GlobalMember,
    Type,
    TypeMember,
}

impl LookupKind {
    fn label(self) -> &'static str {
        match self {
            Self::GlobalMember => "global member",
            Self::Type => "type",
            Self::TypeMember => "type member",
        }
    }
}

fn one_or_ambiguous<T>(
    matches: &mut impl Iterator<Item = T>,
    kind: LookupKind,
    name: &str,
) -> Result<Option<T>, LookupError> {
    let Some(first) = matches.next() else {
        return Ok(None);
    };
    if matches.next().is_some() {
        return Err(LookupError::Ambiguous {
            kind,
            name: name.to_string(),
        });
    }
    Ok(Some(first))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlobalContext {
    pub name: LocalizedName,
    pub property_links: Vec<MemberLink>,
    pub method_links: Vec<MemberLink>,
    pub description: Option<String>,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlobalMethod {
    pub name: LocalizedName,
    pub signatures: Vec<Signature>,
    pub return_types: Vec<TypeRef>,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlobalProperty {
    pub name: LocalizedName,
    pub usage: Option<String>,
    pub type_refs: Vec<TypeRef>,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlobalContextEvent {
    pub name: LocalizedName,
    pub semantic: SemanticContext,
    pub module: ModuleEventContext,
    pub signatures: Vec<Signature>,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformType {
    pub name: LocalizedName,
    pub semantic: SemanticContext,
    pub type_kind: PlatformTypeKind,
    pub object_kind: Option<PlatformObjectKind>,
    pub extends: Vec<LocalizedName>,
    pub metadata_kind: Option<String>,
    pub template_parameters: Vec<String>,
    pub method_links: Vec<MemberLink>,
    pub constructor_links: Vec<MemberLink>,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueryTable {
    pub name: String,
    pub syntax: Option<LocalizedName>,
    pub identifier: String,
    pub semantic: SemanticContext,
    pub table_role: QueryTableRole,
    pub description: Option<String>,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryTableRole {
    Primary,
    Additional,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformMethod {
    pub owner: LocalizedName,
    pub name: LocalizedName,
    pub semantic: SemanticContext,
    pub signatures: Vec<Signature>,
    pub return_types: Vec<TypeRef>,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlatformProperty {
    pub owner: LocalizedName,
    pub name: LocalizedName,
    pub semantic: SemanticContext,
    pub usage: Option<String>,
    pub type_refs: Vec<TypeRef>,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueryTableField {
    pub owner: LocalizedName,
    pub name: String,
    pub semantic: SemanticContext,
    pub type_refs: Vec<TypeRef>,
    pub description: Option<String>,
    pub note: Option<String>,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueryTableParameter {
    pub owner: LocalizedName,
    pub name: String,
    pub semantic: SemanticContext,
    pub type_refs: Vec<TypeRef>,
    pub description: Option<String>,
    pub default_value: Option<String>,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Constructor {
    pub owner: LocalizedName,
    pub name: LocalizedName,
    pub semantic: SemanticContext,
    pub signatures: Vec<Signature>,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleEventContext {
    pub kind: ModuleKind,
    pub owner_path: Vec<LocalizedName>,
}

impl Default for ModuleEventContext {
    fn default() -> Self {
        Self {
            kind: ModuleKind::Unknown,
            owner_path: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    Session,
    OrdinaryApplication,
    ManagedApplication,
    ExternalConnection,
    Object,
    Manager,
    Form,
    WebService,
    HttpService,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformTypeKind {
    Regular,
    Extension,
    Primitive,
    MetadataTemplate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformObjectKind {
    RegularPlatformType,
    ManagedForm,
    FormExtension,
    MetadataObject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnumDefinition {
    pub name: LocalizedName,
    pub value_links: Vec<MemberLink>,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnumValue {
    pub owner: LocalizedName,
    pub name: LocalizedName,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct SectionFacts {
    pub availability: Availability,
    pub examples: Vec<ExampleBlock>,
    pub see_also: Vec<MemberLink>,
    pub available_since: Option<VersionFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct Availability {
    pub contexts: Vec<AvailabilityContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityContext {
    ThinClient,
    WebClient,
    MobileClient,
    Server,
    ThickClient,
    ExternalConnection,
    MobileApplicationClient,
    MobileApplicationServer,
    MobileStandaloneServer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExampleBlock {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VersionFact {
    pub version: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Signature {
    pub text: String,
    pub parameters: Vec<Parameter>,
    pub variant: Option<SyntaxVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyntaxVariant {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Parameter {
    pub name: String,
    pub required: bool,
    pub type_refs: Vec<TypeRef>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TypeRef {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalizedName {
    pub primary: String,
    pub alias: Option<String>,
}

impl LocalizedName {
    pub fn matches(&self, value: &str) -> bool {
        self.primary == value || self.alias.as_deref() == Some(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemberLink {
    pub name: LocalizedName,
    pub html_path: String,
}
