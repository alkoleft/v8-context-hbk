use std::path::Path;

use hbk_book::{FlatTocPage, TocPage};
use syntax_helper_model::*;

use crate::html::name_from_text;
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
    let mut current_query_table_owner = is_query_table_page_path(&root_page.html_path)
        .then(|| semantic_page_name(locale, root_page));
    for (index, child) in root_page.children.iter().enumerate() {
        current_query_table_owner = collect_child_catalog_pages(
            hbk_path,
            locale,
            child,
            &ancestors,
            root_flat_page.index_path.child(index),
            current_query_table_owner,
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
    current_query_table_owner: Option<LocalizedName>,
    pages: &mut Vec<CatalogPage>,
) -> Option<LocalizedName> {
    let branch = branch_kind(page, ancestors);
    let class = classify_catalog_page(page, ancestors, branch);
    let page_query_table_owner = (class == PageClass::QueryTable)
        .then(|| semantic_page_name(locale, page))
        .or_else(|| current_query_table_owner.clone());
    pages.push(CatalogPage {
        class,
        semantic: semantic_context(
            locale,
            branch,
            class,
            page,
            ancestors,
            page_query_table_owner.as_ref(),
        ),
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
    let mut child_query_table_owner = page_query_table_owner.clone();
    for (index, child) in page.children.iter().enumerate() {
        child_query_table_owner = collect_child_catalog_pages(
            hbk_path,
            locale,
            child,
            &child_ancestors,
            toc_path.child(index),
            child_query_table_owner,
            pages,
        );
    }
    if class == PageClass::QueryTable {
        page_query_table_owner
    } else {
        current_query_table_owner
    }
}

fn classify_catalog_page(page: &TocPage, ancestors: &[&TocPage], branch: BranchKind) -> PageClass {
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
    } else if branch == BranchKind::ManagedForms
        && (path.contains("/params/") || has_form_parameters_context(ancestors))
    {
        PageClass::ObjectProperty
    } else if path.contains("/events/") {
        event_page_class(ancestors, branch)
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

fn event_page_class(ancestors: &[&TocPage], branch: BranchKind) -> PageClass {
    if branch == BranchKind::GlobalContext || has_explicit_module_context(ancestors) {
        PageClass::ModuleEvent
    } else if matches!(
        branch,
        BranchKind::SystemEnums
            | BranchKind::MetadataObjects
            | BranchKind::ManagedForms
            | BranchKind::PlatformObjects
            | BranchKind::AutomationExternalApi
    ) {
        PageClass::TypeEvent
    } else {
        PageClass::UnknownEvent
    }
}

fn has_explicit_module_context(ancestors: &[&TocPage]) -> bool {
    ancestors.iter().any(|ancestor| {
        let label = normalized_title(ancestor);
        label.contains("модул") || label.contains("module")
    })
}

fn has_form_parameters_context(ancestors: &[&TocPage]) -> bool {
    ancestors.iter().any(|ancestor| {
        let label = normalized_title(ancestor);
        label.contains("параметры формы") || label.contains("form parameters")
    })
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
    query_table_owner: Option<&LocalizedName>,
) -> SemanticContext {
    let family = record_family(class);
    let owner_path = owner_path(locale, class, page, ancestors, query_table_owner);
    SemanticContext::new(branch, family).with_owner_path(owner_path)
}

fn record_family(class: PageClass) -> RecordFamily {
    match class {
        PageClass::Catalog => RecordFamily::Catalog,
        PageClass::GlobalMethod => RecordFamily::GlobalMethod,
        PageClass::GlobalProperty => RecordFamily::GlobalProperty,
        PageClass::ModuleEvent => RecordFamily::ModuleEvent,
        PageClass::TypeEvent => RecordFamily::TypeEvent,
        PageClass::UnknownEvent => RecordFamily::UnknownEvent,
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
    if path == "objects/catalog2.html" || path.starts_with("objects/catalog2/") {
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
    query_table_owner: Option<&LocalizedName>,
) -> Vec<LocalizedName> {
    match class {
        PageClass::QueryTable => ancestors
            .iter()
            .filter(|ancestor| include_query_owner_path_node(ancestor))
            .map(|ancestor| semantic_page_name(locale, ancestor))
            .collect(),
        PageClass::QueryTableField | PageClass::QueryTableParameter => {
            let Some(owner) = query_table_owner else {
                return Vec::new();
            };
            let mut owner_path = ancestors
                .iter()
                .filter(|ancestor| include_query_owner_path_node(ancestor))
                .map(|ancestor| semantic_page_name(locale, ancestor))
                .collect::<Vec<_>>();
            owner_path.push(owner.clone());
            owner_path
        }
        PageClass::ModuleEvent
        | PageClass::TypeEvent
        | PageClass::UnknownEvent
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

fn is_query_table_page_path(path: &str) -> bool {
    path.starts_with("tables/")
        && path.ends_with(".html")
        && !path.contains("/fields/")
        && !path.contains("/params/")
}

fn semantic_page_name(locale: &str, page: &TocPage) -> LocalizedName {
    let title = if matches!(locale, "root" | "en") && !page.title.en.is_empty() {
        &page.title.en
    } else if locale == "ru" && !page.title.ru.is_empty() {
        &page.title.ru
    } else {
        page.title.display()
    };
    name_from_text(title)
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
