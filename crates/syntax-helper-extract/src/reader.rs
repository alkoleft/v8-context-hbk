use std::collections::{BTreeMap, BTreeSet};
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
    let parent_identities = parent_identities(&roots, &mut load_page)?;

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
            if matches!(catalog_page.class, PageClass::QueryTable) {
                let table = if let Some(table) =
                    parent_identities.query_table_record(&catalog_page.source.html_path)
                {
                    table
                } else {
                    let content = load_page(&catalog_page.source.html_path)?;
                    let source = source_from_content(&catalog_page.source, &content);
                    let mut table = parse_query_table(&content, source);
                    table.semantic = catalog_page.semantic.clone();
                    table.name = name_from_text(&catalog_page.source.page_title).primary;
                    table.identifier = query_table_identifier(table.syntax.as_ref(), &table.name);
                    table.table_role = query_table_role(table.syntax.as_ref());
                    table.identity = parent_identities.query_table(&catalog_page.source.html_path);
                    table
                };
                if table.syntax.is_none() {
                    sink.diagnostic(missing_query_table_syntax_diagnostic(&table))
                        .map_err(SyntaxHelperStreamError::Sink)?;
                }
                sink.query_table(table)
                    .map_err(SyntaxHelperStreamError::Sink)?;
                continue;
            }
            let content = load_page(&catalog_page.source.html_path)?;
            let source = source_from_content(&catalog_page.source, &content);
            match catalog_page.class {
                PageClass::Catalog | PageClass::Unknown | PageClass::QueryTable => unreachable!(),
                PageClass::GlobalMethod => {
                    let method = parse_global_method(&content, source);
                    emit_overload_return_diagnostics(sink, &method.signatures, &method.source)?;
                    sink.global_method(method)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::GlobalProperty => sink
                    .global_property(parse_global_property(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::ModuleEvent | PageClass::TypeEvent | PageClass::UnknownEvent => {
                    let mut event = parse_global_context_event(&content, source);
                    event.semantic = catalog_page.semantic.clone();
                    if event.semantic.record_family == RecordFamily::ModuleEvent {
                        event.module = module_context(&catalog_page.semantic);
                    } else if event.semantic.record_family == RecordFamily::TypeEvent {
                        event.owner_identity =
                            parent_identities.type_event_owner(&catalog_page, &event.semantic);
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
                    platform_type.identity =
                        parent_identities.platform_type(&catalog_page.source.html_path);
                    sink.platform_type(platform_type)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::ObjectMethod => {
                    let mut method = parse_platform_method(&content, source);
                    method.semantic = catalog_page.semantic.clone();
                    method.owner_identity =
                        parent_identities.platform_type_owner(&catalog_page, &method.owner);
                    emit_overload_return_diagnostics(sink, &method.signatures, &method.source)?;
                    sink.type_method(method)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::ObjectProperty => {
                    let mut property = parse_platform_property(&content, source);
                    property.semantic = catalog_page.semantic.clone();
                    if let Some(owner) = form_parameter_owner(&property.semantic) {
                        property.owner = owner;
                    }
                    property.owner_identity =
                        parent_identities.platform_type_owner(&catalog_page, &property.owner);
                    sink.type_property(property)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::QueryTableField => {
                    if let Some(owner) = query_table_member_owner(&catalog_page.semantic) {
                        let mut field = parse_query_table_field(&content, owner, source);
                        field.semantic = catalog_page.semantic.clone();
                        field.owner_identity =
                            parent_identities.query_table_owner(&catalog_page, &field.owner);
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
                        parameter.owner_identity =
                            parent_identities.query_table_owner(&catalog_page, &parameter.owner);
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
                    constructor.owner_identity =
                        parent_identities.platform_type_owner(&catalog_page, &constructor.owner);
                    sink.constructor(constructor)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::Enum => {
                    let mut enum_definition =
                        parse_enum_for_mode(&content, source, record_detail_mode);
                    enum_definition.identity =
                        parent_identities.enum_definition(&catalog_page.source.html_path);
                    sink.enum_definition(enum_definition)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
                PageClass::EnumValue => {
                    let mut enum_value = parse_enum_value(&content, source);
                    enum_value.owner_identity =
                        parent_identities.enum_owner(&catalog_page.source.html_path);
                    sink.enum_value(enum_value)
                        .map_err(SyntaxHelperStreamError::Sink)?
                }
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

#[derive(Default)]
struct ParentIdentities {
    platform_types: BTreeMap<String, String>,
    platform_types_by_semantic: BTreeMap<String, String>,
    query_tables: BTreeMap<String, String>,
    query_tables_by_semantic: BTreeMap<String, String>,
    query_table_records: BTreeMap<String, QueryTable>,
    enums: BTreeMap<String, String>,
}

impl ParentIdentities {
    fn platform_type(&self, html_path: &str) -> Option<String> {
        self.platform_types.get(html_path).cloned()
    }

    fn query_table(&self, html_path: &str) -> Option<String> {
        self.query_tables.get(html_path).cloned()
    }

    fn query_table_record(&self, html_path: &str) -> Option<QueryTable> {
        self.query_table_records.get(html_path).cloned()
    }

    fn enum_definition(&self, html_path: &str) -> Option<String> {
        self.enums.get(html_path).cloned()
    }

    fn platform_type_owner(
        &self,
        catalog_page: &CatalogPage,
        owner: &LocalizedName,
    ) -> Option<String> {
        platform_parent_html_path(&catalog_page.source.html_path)
            .and_then(|html_path| self.platform_types.get(&html_path).cloned())
            .or_else(|| {
                self.platform_types_by_semantic
                    .get(&platform_type_owner_semantic_key(
                        owner,
                        &catalog_page.semantic,
                    ))
                    .cloned()
            })
    }

    fn query_table_owner(
        &self,
        catalog_page: &CatalogPage,
        owner: &LocalizedName,
    ) -> Option<String> {
        query_table_parent_html_path(&catalog_page.source.html_path)
            .and_then(|html_path| self.query_tables.get(&html_path).cloned())
            .or_else(|| {
                self.query_tables_by_semantic
                    .get(&query_table_semantic_key(
                        &catalog_page.semantic,
                        &owner.primary,
                    ))
                    .cloned()
            })
    }

    fn enum_owner(&self, html_path: &str) -> Option<String> {
        enum_parent_html_path(html_path).and_then(|html_path| self.enums.get(&html_path).cloned())
    }

    fn type_event_owner(
        &self,
        catalog_page: &CatalogPage,
        semantic: &SemanticContext,
    ) -> Option<String> {
        platform_parent_html_path(&catalog_page.source.html_path)
            .and_then(|html_path| self.platform_types.get(&html_path).cloned())
            .or_else(|| {
                type_event_owner_semantic_key(semantic)
                    .and_then(|key| self.platform_types_by_semantic.get(&key).cloned())
            })
    }
}

struct PlatformTypeIdentityInput {
    html_path: String,
    name_primary: String,
    semantic: SemanticContext,
}

struct QueryTableIdentityInput {
    html_path: String,
    record: QueryTable,
}

struct EnumIdentityInput {
    html_path: String,
    name: LocalizedName,
}

fn parent_identities(
    roots: &[RootSection],
    load_page: &mut impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
) -> Result<ParentIdentities, SyntaxHelperError> {
    let mut visited = BTreeSet::new();
    let mut platform_types = Vec::new();
    let mut query_tables = Vec::new();
    let mut enums = Vec::new();

    for root in roots {
        for catalog_page in &root.pages {
            if matches!(catalog_page.class, PageClass::Catalog | PageClass::Unknown)
                || !visited.insert(catalog_page.source.html_path.clone())
            {
                continue;
            }
            match catalog_page.class {
                PageClass::ObjectType if !is_skipped_primitive_literal(&catalog_page.semantic) => {
                    platform_types.push(PlatformTypeIdentityInput {
                        html_path: catalog_page.source.html_path.clone(),
                        name_primary: name_from_text(&catalog_page.source.page_title).primary,
                        semantic: catalog_page.semantic.clone(),
                    });
                }
                PageClass::QueryTable => {
                    let content = load_page(&catalog_page.source.html_path)?;
                    let mut table = parse_query_table(
                        &content,
                        source_from_content(&catalog_page.source, &content),
                    );
                    table.semantic = catalog_page.semantic.clone();
                    table.name = name_from_text(&catalog_page.source.page_title).primary;
                    table.identifier = query_table_identifier(table.syntax.as_ref(), &table.name);
                    table.table_role = query_table_role(table.syntax.as_ref());
                    query_tables.push(QueryTableIdentityInput {
                        html_path: catalog_page.source.html_path.clone(),
                        record: table,
                    });
                }
                PageClass::Enum => {
                    enums.push(EnumIdentityInput {
                        html_path: catalog_page.source.html_path.clone(),
                        name: name_from_text(&catalog_page.source.page_title),
                    });
                }
                PageClass::Catalog
                | PageClass::GlobalMethod
                | PageClass::GlobalProperty
                | PageClass::ModuleEvent
                | PageClass::TypeEvent
                | PageClass::UnknownEvent
                | PageClass::ObjectType
                | PageClass::ObjectMethod
                | PageClass::ObjectProperty
                | PageClass::QueryTableField
                | PageClass::QueryTableParameter
                | PageClass::Constructor
                | PageClass::EnumValue
                | PageClass::Unknown => {}
            }
        }
    }

    let platform_counts = count_identity_keys(
        platform_types
            .iter()
            .map(|record| platform_type_identity_key(&record.name_primary)),
    );
    let query_counts = count_identity_keys(query_tables.iter().map(|record| {
        query_table_identity_key(
            &record.record.name,
            record.record.identifier.as_deref(),
            &record.record.semantic,
        )
    }));
    let enum_counts = count_identity_keys(
        enums
            .iter()
            .map(|record| enum_identity_key(&record.name.primary, &record.html_path)),
    );

    let platform_identity_entries = platform_types
        .into_iter()
        .map(|record| {
            let count = platform_counts
                .get(&platform_type_identity_key(&record.name_primary))
                .copied()
                .unwrap_or_default();
            let identity = platform_type_identity(&record.name_primary, &record.semantic, count);
            (
                record.html_path,
                platform_type_semantic_key(&record.name_primary, &record.semantic),
                identity,
            )
        })
        .collect::<Vec<_>>();
    let platform_types = platform_identity_entries
        .iter()
        .map(|(html_path, _, identity)| (html_path.clone(), identity.clone()))
        .collect::<BTreeMap<_, _>>();
    let platform_types_by_semantic = platform_identity_entries
        .iter()
        .map(|(_, semantic_key, identity)| (semantic_key.clone(), identity.clone()))
        .collect();
    let query_tables = query_tables
        .into_iter()
        .map(|record| {
            let count = query_counts
                .get(&query_table_identity_key(
                    &record.record.name,
                    record.record.identifier.as_deref(),
                    &record.record.semantic,
                ))
                .copied()
                .unwrap_or_default();
            let mut table = record.record;
            table.identity = Some(query_table_identity(
                &table.name,
                table.identifier.as_deref(),
                &table.semantic,
                count,
            ));
            (record.html_path, table)
        })
        .collect::<BTreeMap<_, _>>();
    let query_tables_by_semantic = query_tables
        .values()
        .filter_map(|table| {
            Some((
                query_table_semantic_key(&table.semantic, &table.name),
                table.identity.clone()?,
            ))
        })
        .collect();
    let query_table_identities = query_tables
        .iter()
        .filter_map(|(html_path, table)| Some((html_path.clone(), table.identity.clone()?)))
        .collect();
    let enums = enums
        .into_iter()
        .map(|record| {
            let count = enum_counts
                .get(&enum_identity_key(&record.name.primary, &record.html_path))
                .copied()
                .unwrap_or_default();
            (
                record.html_path.clone(),
                enum_identity(
                    &record.name.primary,
                    record.name.alias.as_deref(),
                    &record.html_path,
                    count,
                ),
            )
        })
        .collect();

    Ok(ParentIdentities {
        platform_types,
        platform_types_by_semantic,
        query_tables: query_table_identities,
        query_tables_by_semantic,
        query_table_records: query_tables,
        enums,
    })
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

fn emit_overload_return_diagnostics<S>(
    sink: &mut S,
    signatures: &[Signature],
    source: &SyntaxHelperSource,
) -> Result<(), SyntaxHelperStreamError<S::Error>>
where
    S: SyntaxHelperSink,
{
    for signature in signatures
        .iter()
        .filter(|signature| signature.return_types.len() > 1)
    {
        sink.diagnostic(SyntaxHelperDiagnostic {
            severity: DiagnosticSeverity::Warning,
            code: "MULTIPLE_OVERLOAD_RETURN_TYPES",
            source: source.clone(),
            parser_stage: "syntax_helper_extract",
            message: format!(
                "signature '{}' has {} overload-specific return type references; all values are preserved for parser/data-contract review",
                signature.text,
                signature.return_types.len()
            ),
        })
        .map_err(SyntaxHelperStreamError::Sink)?;
    }
    Ok(())
}

pub(crate) fn query_table_member_owner(semantic: &SemanticContext) -> Option<LocalizedName> {
    if semantic.branch_kind != BranchKind::QueryTables {
        return None;
    }
    semantic.owner_path.last().cloned()
}

fn platform_parent_html_path(html_path: &str) -> Option<String> {
    parent_html_path_before(
        html_path,
        &[
            "/methods/",
            "/properties/",
            "/formparams/",
            "/ctors/",
            "/events/",
        ],
    )
}

fn query_table_parent_html_path(html_path: &str) -> Option<String> {
    parent_html_path_before(html_path, &["/fields/", "/params/"])
}

fn enum_parent_html_path(html_path: &str) -> Option<String> {
    parent_html_path_before(html_path, &["/properties/"])
}

fn parent_html_path_before(html_path: &str, markers: &[&str]) -> Option<String> {
    markers.iter().find_map(|marker| {
        html_path
            .split_once(marker)
            .map(|(prefix, _)| format!("{prefix}.html"))
    })
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
