use std::collections::BTreeMap;
use std::convert::Infallible;
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

    pub fn type_event_owner(&self) -> Option<LocalizedName> {
        if self.record_family != RecordFamily::TypeEvent {
            return None;
        }
        let mut owner_path = self.owner_path.as_slice();
        if owner_path
            .last()
            .is_some_and(|name| event_group_label(&name.primary))
        {
            owner_path = &owner_path[..owner_path.len() - 1];
        }
        match owner_path {
            [] => None,
            [owner] => Some(owner.clone()),
            owners => Some(LocalizedName {
                primary: owners
                    .iter()
                    .map(|name| name.primary.as_str())
                    .collect::<Vec<_>>()
                    .join("."),
                alias: None,
            }),
        }
    }
}

impl Default for SemanticContext {
    fn default() -> Self {
        Self::new(BranchKind::Unknown, RecordFamily::Unknown)
    }
}

fn event_group_label(label: &str) -> bool {
    let label = label.trim().to_lowercase();
    matches!(label.as_str(), "события" | "events")
}

pub fn platform_type_identity(
    name_primary: &str,
    semantic: &SemanticContext,
    same_primary_count: usize,
) -> String {
    let base = clean_identity_part(name_primary);
    if same_primary_count <= 1 {
        format!("platform_type:{base}")
    } else {
        format!(
            "platform_type:{base}:{}",
            semantic_variant(&semantic.owner_path)
        )
    }
}

pub fn platform_type_identity_key(name_primary: &str) -> String {
    base_name_key(name_primary)
}

pub fn platform_type_semantic_key(name_primary: &str, semantic: &SemanticContext) -> String {
    semantic_record_key(name_primary, semantic)
}

pub fn platform_type_owner_semantic_key(
    owner: &LocalizedName,
    semantic: &SemanticContext,
) -> String {
    semantic_relation_key(semantic, &owner.primary)
}

pub fn generic_platform_template_base(name: &LocalizedName) -> Option<String> {
    generic_platform_template_base_for_source(name, false)
}

pub fn generic_platform_template_base_for_source(
    name: &LocalizedName,
    allow_primary_fallback: bool,
) -> Option<String> {
    name.alias
        .as_deref()
        .and_then(generic_platform_template_base_from_name)
        .or_else(|| {
            allow_primary_fallback
                .then(|| generic_platform_template_base_from_name(&name.primary))
                .flatten()
        })
        .map(str::to_string)
}

pub fn query_table_identity(
    name_primary: &str,
    identifier: Option<&str>,
    semantic: &SemanticContext,
    same_base_count: usize,
) -> String {
    let base = query_table_identity_base(name_primary, identifier, semantic);
    if same_base_count <= 1 {
        format!("query_table:{base}")
    } else {
        format!(
            "query_table:{base}:{}",
            semantic_variant(&semantic.owner_path)
        )
    }
}

pub fn query_table_identity_key(
    name_primary: &str,
    identifier: Option<&str>,
    semantic: &SemanticContext,
) -> String {
    normalize_identity_lookup_key(&query_table_identity_base(
        name_primary,
        identifier,
        semantic,
    ))
}

pub fn query_table_semantic_key(semantic: &SemanticContext, fallback: &str) -> String {
    semantic_relation_key(semantic, fallback)
}

pub fn enum_identity(
    name_primary: &str,
    name_alias: Option<&str>,
    source_html_path: &str,
    same_key_count: usize,
) -> String {
    let base = clean_identity_part(name_primary);
    let kind = enum_kind(source_html_path);
    let identity = format!("enum:{kind}:{base}");
    if same_key_count <= 1 {
        return identity;
    }
    name_alias
        .map(clean_identity_part)
        .filter(|alias| !alias.is_empty())
        .map(|alias| format!("{identity}:{alias}"))
        .unwrap_or(identity)
}

pub fn enum_identity_key(name_primary: &str, source_html_path: &str) -> String {
    format!(
        "{}:{}",
        enum_kind(source_html_path),
        base_name_key(name_primary)
    )
}

pub fn enum_kind(source_html_path: &str) -> &'static str {
    if source_html_path.starts_with("objects/catalog2/")
        || source_html_path == "objects/catalog2.html"
    {
        "system"
    } else {
        "metadata_property"
    }
}

pub fn count_identity_keys(keys: impl Iterator<Item = String>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for key in keys {
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

pub fn clean_identity_part(value: &str) -> String {
    strip_toc_duplicate_marker(value).trim().to_string()
}

pub fn normalize_identity_lookup_key(value: &str) -> String {
    strip_toc_duplicate_marker(value).trim().to_lowercase()
}

fn query_table_identity_base(
    name_primary: &str,
    identifier: Option<&str>,
    semantic: &SemanticContext,
) -> String {
    if let Some(identifier) = identifier {
        let identifier = clean_identity_part(identifier);
        if !identifier.is_empty() {
            return identifier;
        }
    }
    semantic_record_key(name_primary, semantic)
}

fn semantic_relation_key(semantic: &SemanticContext, fallback: &str) -> String {
    let mut parts = semantic
        .owner_path
        .iter()
        .map(|name| name.primary.as_str())
        .collect::<Vec<_>>();
    if parts.last().is_none_or(|last| {
        normalize_identity_lookup_key(last) != normalize_identity_lookup_key(fallback)
    }) {
        parts.push(fallback);
    }
    parts
        .into_iter()
        .map(normalize_identity_lookup_key)
        .collect::<Vec<_>>()
        .join(":")
}

fn semantic_record_key(name: &str, semantic: &SemanticContext) -> String {
    let mut parts = semantic
        .owner_path
        .iter()
        .map(|name| clean_identity_part(&name.primary))
        .collect::<Vec<_>>();
    parts.push(clean_identity_part(name));
    parts.join(":")
}

fn semantic_variant(owner_path: &[LocalizedName]) -> String {
    owner_path
        .iter()
        .rev()
        .find(|name| !name.primary.trim().is_empty())
        .map(|name| clean_identity_part(&name.primary))
        .unwrap_or_else(|| "semantic_variant".to_string())
}

fn base_name_key(value: &str) -> String {
    normalize_identity_lookup_key(value)
}

fn strip_toc_duplicate_marker(value: &str) -> &str {
    value.split("#&^@^%&*^#").next().unwrap_or(value)
}

pub fn generic_platform_template_base_from_name(name: &str) -> Option<&str> {
    if !name.contains('<') || !name.contains('>') {
        return None;
    }
    name.split(['.', '<'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_event_owner_uses_json_compatible_semantic_projection() {
        let semantic = SemanticContext::new(BranchKind::ManagedForms, RecordFamily::TypeEvent)
            .with_owner_path(vec![name("Форма"), name("Поле формы"), name("События")]);

        let owner = semantic
            .type_event_owner()
            .expect("type event owner must be projected");

        assert_eq!(owner.primary, "Форма.Поле формы");
        assert_eq!(owner.alias, None);
    }

    #[test]
    fn type_event_owner_ignores_non_type_event_records() {
        let semantic = SemanticContext::new(BranchKind::ManagedForms, RecordFamily::ModuleEvent)
            .with_owner_path(vec![name("События")]);

        assert_eq!(semantic.type_event_owner(), None);
    }

    #[test]
    fn parent_identity_helpers_build_domain_owned_parent_ids() {
        let type_semantic =
            SemanticContext::new(BranchKind::ManagedForms, RecordFamily::PlatformType)
                .with_owner_path(vec![name("Обычная форма")]);
        assert_eq!(
            platform_type_identity("ЭлементыФормы#&^@^%&*^#1", &type_semantic, 2),
            "platform_type:ЭлементыФормы:Обычная форма"
        );
        assert_eq!(
            platform_type_semantic_key("ЭлементыФормы#&^@^%&*^#1", &type_semantic),
            "Обычная форма:ЭлементыФормы"
        );

        let table_semantic =
            SemanticContext::new(BranchKind::QueryTables, RecordFamily::QueryTable)
                .with_owner_path(vec![name("Таблицы задач")]);
        assert_eq!(
            query_table_identity("Основная таблица", Some("Задача"), &table_semantic, 1),
            "query_table:Задача"
        );
        assert_eq!(
            query_table_identity("Основная таблица", None, &table_semantic, 2),
            "query_table:Таблицы задач:Основная таблица:Таблицы задач"
        );

        assert_eq!(
            enum_identity(
                "ИспользованиеТекущейСтроки",
                Some("UseCurrentRow"),
                "objects/catalog2/catalog1/UseCurrentRow.html",
                2,
            ),
            "enum:system:ИспользованиеТекущейСтроки:UseCurrentRow"
        );
        assert_eq!(
            enum_identity(
                "Вид",
                None,
                "objects/catalog1649/catalog1677/FormGroup/properties/View.html",
                1,
            ),
            "enum:metadata_property:Вид"
        );
    }

    #[test]
    fn generic_platform_template_base_prefers_alias_and_requires_explicit_primary_fallback() {
        assert_eq!(
            generic_platform_template_base(&LocalizedName {
                primary: "СправочникСсылка.<Имя справочника>".to_string(),
                alias: Some("CatalogRef.<Catalog name>".to_string()),
            }),
            Some("CatalogRef".to_string())
        );
        assert_eq!(
            generic_platform_template_base(&LocalizedName {
                primary: "DocumentObject.<Document name>".to_string(),
                alias: None,
            }),
            None
        );
        assert_eq!(
            generic_platform_template_base_for_source(
                &LocalizedName {
                    primary: "DocumentObject.<Document name>".to_string(),
                    alias: None,
                },
                true,
            ),
            Some("DocumentObject".to_string())
        );
        assert_eq!(
            generic_platform_template_base(&LocalizedName {
                primary: "HTTPСоединение".to_string(),
                alias: Some("HTTPConnection".to_string()),
            }),
            None
        );
    }

    #[test]
    fn identity_lookup_keys_strip_toc_duplicate_markers() {
        assert_eq!(
            clean_identity_part(" Основная таблица#&^@^%&*^#1 "),
            "Основная таблица"
        );
        assert_eq!(
            normalize_identity_lookup_key(" Основная Таблица#&^@^%&*^#1 "),
            "основная таблица"
        );
    }

    fn name(primary: &str) -> LocalizedName {
        LocalizedName {
            primary: primary.to_string(),
            alias: None,
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    pub name: LocalizedName,
    pub semantic: SemanticContext,
    pub type_kind: PlatformTypeKind,
    pub object_kind: Option<PlatformObjectKind>,
    pub extends: Vec<LocalizedName>,
    pub metadata_kind: Option<String>,
    pub template_parameters: Vec<String>,
    pub generic_template_key: Option<GenericPlatformTemplateKey>,
    pub method_links: Vec<MemberLink>,
    pub constructor_links: Vec<MemberLink>,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueryTable {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    pub name: String,
    pub syntax: Option<LocalizedName>,
    pub identifier: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_identity: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_identity: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_identity: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_identity: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_identity: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct GenericPlatformTemplateKey {
    pub family: String,
    pub variant: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GenericTypeBinding {
    pub template_key: GenericPlatformTemplateKey,
    pub arguments: Vec<GenericArgumentBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenericArgumentBinding {
    OwnerParameter {
        owner_parameter_index: usize,
        target_parameter_index: usize,
    },
}

impl GenericPlatformTemplateKey {
    pub fn new(family: impl Into<String>, variant: impl Into<String>) -> Self {
        Self {
            family: family.into(),
            variant: variant.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnumDefinition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    pub name: LocalizedName,
    pub value_links: Vec<MemberLink>,
    pub description: Option<String>,
    pub facts: SectionFacts,
    pub source: SyntaxHelperSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnumValue {
    pub owner: LocalizedName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_identity: Option<String>,
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

    pub fn display_name(&self) -> String {
        match &self.alias {
            Some(alias) => format!("{} ({alias})", self.primary),
            None => self.primary.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MemberLink {
    pub name: LocalizedName,
    pub html_path: String,
}
