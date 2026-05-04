use std::path::Path;

use hbk_book::{FlatTocPage, TocPage};
use syntax_helper_model::*;

use crate::label_match::has_token_prefix;

pub(crate) fn collect_catalog_pages(
    hbk_path: &Path,
    locale: &str,
    root_page: &TocPage,
    root_flat_page: &FlatTocPage<'_>,
) -> Vec<CatalogPage> {
    let mut pages = Vec::new();
    pages.push(CatalogPage {
        class: PageClass::Catalog,
        semantic: SemanticContext::new(branch_kind(root_page, &[]), RecordFamily::Catalog),
        source: source_from_toc(hbk_path, locale, root_flat_page),
    });
    let ancestors = vec![root_page];
    for (index, child) in root_page.children.iter().enumerate() {
        collect_child_catalog_pages(
            hbk_path,
            locale,
            child,
            &ancestors,
            root_flat_page.index_path.child(index),
            &mut pages,
        );
    }
    pages
}

fn collect_child_catalog_pages(
    hbk_path: &Path,
    locale: &str,
    page: &TocPage,
    ancestors: &[&TocPage],
    toc_path: hbk_book::TocPath,
    pages: &mut Vec<CatalogPage>,
) {
    let branch = branch_kind(page, ancestors);
    let class = classify_catalog_page(page, branch);
    pages.push(CatalogPage {
        class,
        semantic: semantic_context(locale, branch, class, page, ancestors),
        source: SyntaxHelperSource {
            hbk_path: hbk_path.to_path_buf(),
            locale: locale.to_string(),
            toc_path: Some(toc_path.to_string()),
            html_path: page.html_path.clone(),
            page_title: page.title.display().to_string(),
        },
    });
    let mut child_ancestors = ancestors.to_vec();
    child_ancestors.push(page);
    for (index, child) in page.children.iter().enumerate() {
        collect_child_catalog_pages(
            hbk_path,
            locale,
            child,
            &child_ancestors,
            toc_path.child(index),
            pages,
        );
    }
}

fn classify_catalog_page(page: &TocPage, branch: BranchKind) -> PageClass {
    let path = page.html_path.as_str();
    if is_catalog_path(path) {
        PageClass::Catalog
    } else if path.starts_with("objects/Global context/events/") {
        PageClass::ModuleEvent
    } else if path.starts_with("objects/Global context/methods/") {
        PageClass::GlobalMethod
    } else if path.starts_with("objects/Global context/properties/") {
        PageClass::GlobalProperty
    } else if branch == BranchKind::QueryTables && path.contains("/fields/") {
        PageClass::QueryTableField
    } else if branch == BranchKind::QueryTables && path.contains("/params/") {
        PageClass::QueryTableParameter
    } else if branch == BranchKind::QueryTables && path.starts_with("tables/") {
        PageClass::QueryTable
    } else if path.contains("/events/") {
        PageClass::ModuleEvent
    } else if path.contains("/methods/") {
        PageClass::ObjectMethod
    } else if path.contains("/properties/") && path.contains("/catalog2/") {
        PageClass::EnumValue
    } else if path.contains("/properties/") {
        PageClass::ObjectProperty
    } else if path.contains("/ctors/") {
        PageClass::Constructor
    } else if path.starts_with("objects/catalog2/") {
        PageClass::Enum
    } else if path.starts_with("objects/catalog") {
        PageClass::ObjectType
    } else if !page.children.is_empty() {
        PageClass::Catalog
    } else {
        PageClass::Unknown
    }
}

fn is_catalog_path(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with("catalog") && name.ends_with(".html"))
}

fn semantic_context(
    locale: &str,
    branch: BranchKind,
    class: PageClass,
    page: &TocPage,
    ancestors: &[&TocPage],
) -> SemanticContext {
    let family = record_family(class);
    let owner_path = owner_path(locale, class, page, ancestors);
    SemanticContext::new(branch, family).with_owner_path(owner_path)
}

fn record_family(class: PageClass) -> RecordFamily {
    match class {
        PageClass::Catalog => RecordFamily::Catalog,
        PageClass::GlobalMethod => RecordFamily::GlobalMethod,
        PageClass::GlobalProperty => RecordFamily::GlobalProperty,
        PageClass::ModuleEvent => RecordFamily::ModuleEvent,
        PageClass::ObjectType => RecordFamily::PlatformType,
        PageClass::QueryTable => RecordFamily::QueryTable,
        PageClass::ObjectMethod => RecordFamily::TypeMethod,
        PageClass::ObjectProperty => RecordFamily::TypeProperty,
        PageClass::QueryTableField => RecordFamily::QueryTableField,
        PageClass::QueryTableParameter => RecordFamily::QueryTableParameter,
        PageClass::Constructor => RecordFamily::TypeConstructor,
        PageClass::Enum => RecordFamily::SystemEnum,
        PageClass::EnumValue => RecordFamily::SystemEnumValue,
        PageClass::Unknown => RecordFamily::Unknown,
    }
}

fn branch_kind(page: &TocPage, ancestors: &[&TocPage]) -> BranchKind {
    let path = page.html_path.as_str();
    let labels = ancestors
        .iter()
        .chain(std::iter::once(&page))
        .map(|page| normalized_title(page))
        .collect::<Vec<_>>();

    if path.starts_with("tables/")
        || labels
            .iter()
            .any(|label| label.contains("таблицы запросов") || label.contains("query tables"))
    {
        return BranchKind::QueryTables;
    }
    if labels
        .iter()
        .any(|label| label.contains("примитивные типы") || label.contains("primitive types"))
    {
        return BranchKind::PrimitiveTypes;
    }
    if path.starts_with("objects/Global context/") || path == "objects/Global context.html" {
        return BranchKind::GlobalContext;
    }
    if path.starts_with("objects/catalog2") {
        return BranchKind::SystemEnums;
    }
    if labels.iter().any(|label| {
        label.contains("прикладные объекты")
            || label.contains("application objects")
            || label.contains("metadata")
    }) {
        return BranchKind::MetadataObjects;
    }
    if labels
        .iter()
        .any(|label| has_token_prefix(label, &["форм", "form"]))
    {
        return BranchKind::ManagedForms;
    }
    if labels
        .iter()
        .any(|label| label.contains("automation") || label.contains("external api"))
    {
        return BranchKind::AutomationExternalApi;
    }
    if path.starts_with("objects/catalog") {
        return BranchKind::PlatformObjects;
    }
    BranchKind::Unknown
}

fn owner_path(
    locale: &str,
    class: PageClass,
    page: &TocPage,
    ancestors: &[&TocPage],
) -> Vec<LocalizedName> {
    match class {
        PageClass::QueryTable => ancestors
            .iter()
            .filter(|ancestor| include_query_owner_path_node(ancestor))
            .map(|ancestor| semantic_page_name(locale, ancestor))
            .collect(),
        PageClass::QueryTableField | PageClass::QueryTableParameter => ancestors
            .iter()
            .filter(|ancestor| include_query_owner_path_node(ancestor))
            .map(|ancestor| semantic_page_name(locale, ancestor))
            .collect(),
        PageClass::ModuleEvent
        | PageClass::ObjectProperty
        | PageClass::Constructor
        | PageClass::ObjectMethod => ancestors
            .iter()
            .skip(1)
            .filter(|ancestor| !ancestor.title.display().trim().is_empty())
            .map(|ancestor| semantic_page_name(locale, ancestor))
            .collect(),
        PageClass::ObjectType => ancestors
            .iter()
            .filter(|ancestor| {
                branch_kind(page, ancestors) != BranchKind::PlatformObjects || {
                    !ancestor.html_path.is_empty()
                }
            })
            .map(|ancestor| semantic_page_name(locale, ancestor))
            .collect(),
        _ => Vec::new(),
    }
}

fn include_query_owner_path_node(page: &TocPage) -> bool {
    let label = normalized_title(page);
    page.html_path.starts_with("objects/catalog")
        || page.html_path.starts_with("tables/")
        || label.contains("таблицы")
        || label.contains("table")
        || label.contains("регистр")
        || label.contains("register")
}

fn semantic_page_name(locale: &str, page: &TocPage) -> LocalizedName {
    let title = if matches!(locale, "root" | "en") && !page.title.en.is_empty() {
        &page.title.en
    } else if locale == "ru" && !page.title.ru.is_empty() {
        &page.title.ru
    } else {
        page.title.display()
    };
    LocalizedName {
        primary: title.trim().to_string(),
        alias: None,
    }
}

fn normalized_title(page: &TocPage) -> String {
    page.title.display().trim().to_lowercase()
}

fn source_from_toc(
    hbk_path: &Path,
    locale: &str,
    flat_page: &FlatTocPage<'_>,
) -> SyntaxHelperSource {
    SyntaxHelperSource {
        hbk_path: hbk_path.to_path_buf(),
        locale: locale.to_string(),
        toc_path: Some(flat_page.index_path.to_string()),
        html_path: flat_page.page.html_path.clone(),
        page_title: flat_page.page.title.display().to_string(),
    }
}
