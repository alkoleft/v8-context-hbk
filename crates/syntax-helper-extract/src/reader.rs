use std::collections::BTreeSet;
use std::path::Path;

use hbk_book::{HbkBook, Toc};
#[cfg(test)]
use hbk_docs::DocumentationReader;
use hbk_docs::PageContent;
use syntax_helper_model::*;

use crate::discovery::discover_roots_with_loader;
#[cfg(test)]
use crate::error::infallible_stream_error;
use crate::error::{SyntaxHelperError, SyntaxHelperStreamError};
use crate::html::name_from_text;
use crate::label_match::has_token_prefix;
use crate::page_parser::{
    parse_constructor, parse_enum_for_mode, parse_enum_value, parse_global_context_event,
    parse_global_context_for_mode, parse_global_method, parse_global_property,
    parse_platform_method, parse_platform_property, parse_platform_type_for_mode,
    parse_query_table, parse_query_table_field, parse_query_table_parameter,
    parse_syntax_page_content_with_index_owned, source_from_content, syntax_toc_index,
};

#[derive(Debug)]
pub struct SyntaxHelperReader<'a> {
    book: &'a HbkBook,
}

impl<'a> SyntaxHelperReader<'a> {
    pub fn new(book: &'a HbkBook) -> Self {
        Self { book }
    }

    #[cfg(test)]
    pub(crate) fn discover_roots(&self) -> Result<RootDiscovery, SyntaxHelperError> {
        let mut documentation = DocumentationReader::new(self.book)
            .page_loader()
            .map_err(SyntaxHelperError::from)?;
        discover_roots_with_loader(
            self.book.path(),
            self.book.locale().source_code(),
            self.book.toc(),
            |html_path| {
                documentation
                    .load_page(html_path)
                    .map_err(SyntaxHelperError::from)
            },
        )
    }

    #[cfg(test)]
    pub(crate) fn extract(&self) -> Result<PlatformContext, SyntaxHelperError> {
        let mut context = PlatformContext::default();
        self.extract_into(&mut context)
            .map_err(infallible_stream_error)?;
        Ok(context)
    }

    pub fn extract_into<S>(&self, sink: &mut S) -> Result<(), SyntaxHelperStreamError<S::Error>>
    where
        S: SyntaxHelperSink,
    {
        let toc_index = syntax_toc_index(self.book.toc());
        let mut file_storage = self
            .book
            .file_storage_reader()
            .map_err(SyntaxHelperError::from)?;
        let mut load_page = |html_path: &str| {
            let raw_html = file_storage.read_page(html_path)?;
            Ok(parse_syntax_page_content_with_index_owned(
                self.book.path(),
                self.book.locale().source_code(),
                &toc_index,
                html_path,
                raw_html,
            ))
        };
        let discovery = discover_roots_with_loader(
            self.book.path(),
            self.book.locale().source_code(),
            self.book.toc(),
            &mut load_page,
        )?;
        let discovery = extraction_discovery(discovery);
        parse_extraction_pages_into(
            self.book.path(),
            self.book.locale().source_code(),
            self.book.toc(),
            discovery,
            load_page,
            sink,
        )
    }
}

#[cfg(test)]
pub(crate) fn extract_with_loader(
    hbk_path: &Path,
    locale: &str,
    toc: &Toc,
    mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
) -> Result<PlatformContext, SyntaxHelperError> {
    let mut context = PlatformContext::default();
    extract_with_loader_into(hbk_path, locale, toc, &mut load_page, &mut context)
        .map_err(infallible_stream_error)?;
    Ok(context)
}

#[cfg(test)]
pub(crate) fn extract_with_loader_into<S>(
    hbk_path: &Path,
    locale: &str,
    toc: &Toc,
    mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
    sink: &mut S,
) -> Result<(), SyntaxHelperStreamError<S::Error>>
where
    S: SyntaxHelperSink,
{
    let discovery = discover_roots_with_loader(hbk_path, locale, toc, &mut load_page)?;
    parse_extraction_pages_into(hbk_path, locale, toc, discovery, load_page, sink)
}

fn extraction_discovery(mut discovery: RootDiscovery) -> RootDiscovery {
    for root in &mut discovery.roots {
        root.pages
            .retain(|page| !matches!(page.class, PageClass::Catalog | PageClass::Unknown));
    }
    discovery
}

pub(crate) fn parse_extraction_pages_into<S>(
    _hbk_path: &Path,
    _locale: &str,
    _toc: &Toc,
    discovery: RootDiscovery,
    mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
    sink: &mut S,
) -> Result<(), SyntaxHelperStreamError<S::Error>>
where
    S: SyntaxHelperSink,
{
    let mut visited = BTreeSet::new();
    let record_detail_mode = sink.record_detail_mode();
    let RootDiscovery { roots, diagnostics } = discovery;

    for diagnostic in diagnostics {
        sink.diagnostic(diagnostic)
            .map_err(SyntaxHelperStreamError::Sink)?;
    }

    for root in roots {
        let RootSection {
            kind,
            source: root_source,
            pages,
        } = root;

        for catalog_page in pages {
            if matches!(catalog_page.class, PageClass::Catalog | PageClass::Unknown) {
                continue;
            }
            if !visited.insert(catalog_page.source.html_path.clone()) {
                continue;
            }
            let content = load_page(&catalog_page.source.html_path)?;
            let source = source_from_content(&catalog_page.source, &content);
            match catalog_page.class {
                PageClass::Catalog | PageClass::Unknown => unreachable!(),
                PageClass::GlobalMethod => sink
                    .global_method(parse_global_method(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::GlobalProperty => sink
                    .global_property(parse_global_property(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::ModuleEvent | PageClass::TypeEvent | PageClass::UnknownEvent => {
                    let mut event = parse_global_context_event(&content, source);
                    event.semantic = catalog_page.semantic.clone();
                    if event.semantic.record_family == RecordFamily::ModuleEvent {
                        event.module = module_context(&catalog_page.semantic);
                    }
                    sink.global_context_event(event)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::ObjectType => {
                    if is_skipped_primitive_literal(&catalog_page.semantic) {
                        continue;
                    }
                    let mut platform_type =
                        parse_platform_type_for_mode(&content, source, record_detail_mode);
                    platform_type.semantic = catalog_page.semantic.clone();
                    apply_platform_type_semantics(&mut platform_type);
                    sink.platform_type(platform_type)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::QueryTable => {
                    let mut table = parse_query_table(&content, source);
                    table.semantic = catalog_page.semantic.clone();
                    table.name = name_from_text(&catalog_page.source.page_title).primary;
                    table.identifier = query_table_identifier(table.syntax.as_ref(), &table.name);
                    table.table_role = query_table_role(table.syntax.as_ref());
                    if table.syntax.is_none() {
                        sink.diagnostic(missing_query_table_syntax_diagnostic(&table))
                            .map_err(SyntaxHelperStreamError::Sink)?;
                    }
                    sink.query_table(table)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::ObjectMethod => {
                    let mut method = parse_platform_method(&content, source);
                    method.semantic = catalog_page.semantic.clone();
                    sink.type_method(method)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::ObjectProperty => {
                    let mut property = parse_platform_property(&content, source);
                    property.semantic = catalog_page.semantic.clone();
                    if let Some(owner) = form_parameter_owner(&property.semantic) {
                        property.owner = owner;
                    }
                    sink.type_property(property)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::QueryTableField => {
                    if let Some(owner) = query_table_member_owner(&catalog_page.semantic) {
                        let mut field = parse_query_table_field(&content, owner, source);
                        field.semantic = catalog_page.semantic.clone();
                        sink.table_field(field)
                            .map_err(SyntaxHelperStreamError::Sink)?;
                    } else {
                        sink.diagnostic(missing_query_table_owner_diagnostic(
                            source,
                            "query_table_field",
                        ))
                        .map_err(SyntaxHelperStreamError::Sink)?;
                    }
                }
                PageClass::QueryTableParameter => {
                    if let Some(owner) = query_table_member_owner(&catalog_page.semantic) {
                        let mut parameter = parse_query_table_parameter(&content, owner, source);
                        parameter.semantic = catalog_page.semantic.clone();
                        sink.table_parameter(parameter)
                            .map_err(SyntaxHelperStreamError::Sink)?;
                    } else {
                        sink.diagnostic(missing_query_table_owner_diagnostic(
                            source,
                            "query_table_parameter",
                        ))
                        .map_err(SyntaxHelperStreamError::Sink)?;
                    }
                }
                PageClass::Constructor => {
                    let mut constructor = parse_constructor(&content, source);
                    constructor.semantic = catalog_page.semantic.clone();
                    sink.constructor(constructor)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::Enum => sink
                    .enum_definition(parse_enum_for_mode(&content, source, record_detail_mode))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::EnumValue => sink
                    .enum_value(parse_enum_value(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
            }
        }

        if kind == RootSectionKind::GlobalContext && visited.insert(root_source.html_path.clone()) {
            let content = load_page(&root_source.html_path)?;
            let source = source_from_content(&root_source, &content);
            sink.global_context(parse_global_context_for_mode(
                &content,
                source,
                record_detail_mode,
            ))
            .map_err(SyntaxHelperStreamError::Sink)?;
        }
    }

    Ok(())
}

fn missing_query_table_syntax_diagnostic(table: &QueryTable) -> SyntaxHelperDiagnostic {
    SyntaxHelperDiagnostic {
        severity: DiagnosticSeverity::Warning,
        code: "MISSING_QUERY_TABLE_SYNTAX",
        source: table.source.clone(),
        parser_stage: "query_table",
        message: "Query table page has no source Syntax section or has an empty Syntax section; syntax and identifier are not synthesized from the table display name".to_string(),
    }
}

fn missing_query_table_owner_diagnostic(
    source: SyntaxHelperSource,
    parser_stage: &'static str,
) -> SyntaxHelperDiagnostic {
    SyntaxHelperDiagnostic {
        severity: DiagnosticSeverity::Warning,
        code: "MISSING_QUERY_TABLE_OWNER_CONTEXT",
        source,
        parser_stage,
        message: "Query table member page has no TOC-derived query table owner context; owner is not synthesized from the member HTML path".to_string(),
    }
}

pub(crate) fn query_table_member_owner(semantic: &SemanticContext) -> Option<LocalizedName> {
    if semantic.branch_kind != BranchKind::QueryTables {
        return None;
    }
    semantic.owner_path.last().cloned()
}

fn query_table_identifier(syntax: Option<&LocalizedName>, name: &str) -> Option<String> {
    let syntax = syntax?;
    let primary = primary_syntax_segment(syntax)?;
    let identifier = if query_table_syntax_segment_count(syntax) > 2 {
        format!("{}{}", primary, camel_case_identifier_part(name))
    } else {
        compact_identifier_part(primary)
    };
    (!identifier.is_empty()).then_some(identifier)
}

fn query_table_role(syntax: Option<&LocalizedName>) -> QueryTableRole {
    if let Some(syntax) = syntax.filter(|syntax| !syntax.primary.trim().is_empty()) {
        return if query_table_syntax_segment_count(syntax) <= 2 {
            QueryTableRole::Primary
        } else {
            QueryTableRole::Additional
        };
    }
    QueryTableRole::Unknown
}

fn query_table_syntax_segment_count(syntax: &LocalizedName) -> usize {
    syntax
        .primary
        .trim()
        .split('.')
        .filter(|segment| !segment.trim().is_empty())
        .count()
}

fn primary_syntax_segment(syntax: &LocalizedName) -> Option<&str> {
    syntax
        .primary
        .trim()
        .split('.')
        .next()
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
}

fn compact_identifier_part(value: &str) -> String {
    value.split_whitespace().collect()
}

fn camel_case_identifier_part(value: &str) -> String {
    let mut output = String::new();
    let mut capitalize_next = true;
    for ch in value.chars() {
        if ch.is_alphanumeric() {
            if capitalize_next {
                output.extend(ch.to_uppercase());
                capitalize_next = false;
            } else {
                output.push(ch);
            }
        } else {
            capitalize_next = true;
        }
    }
    output
}

fn module_context(semantic: &SemanticContext) -> ModuleEventContext {
    ModuleEventContext {
        kind: module_kind(&semantic.owner_path),
        owner_path: semantic.owner_path.clone(),
    }
}

fn module_kind(owner_path: &[LocalizedName]) -> ModuleKind {
    let labels = owner_path
        .iter()
        .map(|name| {
            format!(
                "{} {}",
                name.primary.to_lowercase(),
                name.alias.as_deref().unwrap_or_default().to_lowercase()
            )
        })
        .collect::<Vec<_>>();
    if labels
        .iter()
        .any(|label| label.contains("внешнего соединения") || label.contains("external connection"))
    {
        ModuleKind::ExternalConnection
    } else if labels.iter().any(|label| {
        label.contains("обычного приложения") || label.contains("ordinary application")
    }) {
        ModuleKind::OrdinaryApplication
    } else if labels
        .iter()
        .any(|label| has_token_prefix(label, &["форм", "form"]))
    {
        ModuleKind::Form
    } else if labels.iter().any(|label| {
        label.contains("события приложения")
            || label.contains("managed application")
            || label.contains("client application")
            || label.contains("application events")
    }) {
        ModuleKind::ManagedApplication
    } else if labels
        .iter()
        .any(|label| label.contains("сеанса") || label.contains("session"))
    {
        ModuleKind::Session
    } else if labels
        .iter()
        .any(|label| label.contains("http") || label.contains("http-сервис"))
    {
        ModuleKind::HttpService
    } else if labels.iter().any(|label| {
        label.contains("web service")
            || label.contains("web-сервис")
            || label.contains("веб-сервис")
    }) {
        ModuleKind::WebService
    } else if labels
        .iter()
        .any(|label| label.contains("менеджер") || label.contains("manager"))
    {
        ModuleKind::Manager
    } else if labels
        .iter()
        .any(|label| label.contains("объект") || label.contains("object"))
    {
        ModuleKind::Object
    } else {
        ModuleKind::Unknown
    }
}

fn form_parameter_owner(semantic: &SemanticContext) -> Option<LocalizedName> {
    if semantic.branch_kind != BranchKind::ManagedForms
        || semantic.record_family != RecordFamily::TypeProperty
    {
        return None;
    }
    semantic.owner_path.windows(2).find_map(|window| {
        let current = window[1].primary.to_lowercase();
        (current.contains("параметры формы") || current.contains("form parameters"))
            .then(|| window[0].clone())
    })
}

fn is_skipped_primitive_literal(semantic: &SemanticContext) -> bool {
    semantic.branch_kind == BranchKind::PrimitiveTypes && semantic.owner_path.len() > 2
}

fn apply_platform_type_semantics(platform_type: &mut PlatformType) {
    platform_type.type_kind = platform_type_kind(platform_type);
    platform_type.object_kind = platform_object_kind(platform_type);
    if platform_type.type_kind == PlatformTypeKind::MetadataTemplate {
        platform_type.metadata_kind = metadata_kind(&platform_type.name.primary);
        platform_type.template_parameters = template_parameters(&platform_type.name.primary);
    }
}

fn platform_type_kind(platform_type: &PlatformType) -> PlatformTypeKind {
    if platform_type.semantic.branch_kind == BranchKind::PrimitiveTypes {
        return PlatformTypeKind::Primitive;
    }
    if is_metadata_template_name(&platform_type.name.primary) {
        return PlatformTypeKind::MetadataTemplate;
    }
    if is_extension_name(&platform_type.name.primary) {
        return PlatformTypeKind::Extension;
    }
    PlatformTypeKind::Regular
}

fn platform_object_kind(platform_type: &PlatformType) -> Option<PlatformObjectKind> {
    match platform_type.semantic.branch_kind {
        BranchKind::PlatformObjects if platform_type.type_kind == PlatformTypeKind::Regular => {
            Some(PlatformObjectKind::RegularPlatformType)
        }
        BranchKind::ManagedForms if platform_type.type_kind == PlatformTypeKind::Extension => {
            Some(PlatformObjectKind::FormExtension)
        }
        BranchKind::ManagedForms => Some(PlatformObjectKind::ManagedForm),
        BranchKind::MetadataObjects => Some(PlatformObjectKind::MetadataObject),
        _ => None,
    }
}

fn is_extension_name(name: &str) -> bool {
    let value = name.to_lowercase();
    value.starts_with("расширение") || value.starts_with("extension")
}

fn is_metadata_template_name(name: &str) -> bool {
    name.contains("<") && name.contains(">")
}

fn metadata_kind(name: &str) -> Option<String> {
    name.split(['.', '<'])
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn template_parameters(name: &str) -> Vec<String> {
    let mut parameters = Vec::new();
    let mut rest = name;
    while let Some(start) = rest.find('<') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('>') else {
            break;
        };
        let parameter = after_start[..end].trim();
        if !parameter.is_empty() {
            parameters.push(parameter.to_string());
        }
        rest = &after_start[end + 1..];
    }
    parameters
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(primary: &str) -> LocalizedName {
        LocalizedName {
            primary: primary.to_string(),
            alias: None,
        }
    }

    #[test]
    fn reader_derives_query_table_identity_from_toc_name_and_syntax() {
        let primary = name("БизнесПроцесс.<Имя бизнес-процесса>");
        assert_eq!(
            query_table_identifier(Some(&primary), "Таблица бизнес-процессов").as_deref(),
            Some("БизнесПроцесс")
        );
        assert_eq!(query_table_role(Some(&primary)), QueryTableRole::Primary);

        let additional = name("БизнесПроцесс.<Имя бизнес-процесса>.Точки");
        assert_eq!(
            query_table_identifier(Some(&additional), "Таблица точек бизнес-процессов").as_deref(),
            Some("БизнесПроцессТаблицаТочекБизнесПроцессов")
        );
        assert_eq!(
            query_table_role(Some(&additional)),
            QueryTableRole::Additional
        );
    }

    #[test]
    fn reader_does_not_synthesize_query_table_identity_without_syntax() {
        assert_eq!(query_table_identifier(None, "Основная таблица"), None);
        assert_eq!(query_table_role(None), QueryTableRole::Unknown);

        let blank = name("  ");
        assert_eq!(
            query_table_identifier(Some(&blank), "Основная таблица"),
            None
        );
        assert_eq!(query_table_role(Some(&blank)), QueryTableRole::Unknown);
    }
}
