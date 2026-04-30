use std::path::Path;

use hbk_book::{FlatTocPage, TocPage};
use syntax_helper_model::*;

pub(crate) fn collect_catalog_pages(
    hbk_path: &Path,
    locale: &str,
    root_page: &TocPage,
    root_flat_page: &FlatTocPage<'_>,
) -> Vec<CatalogPage> {
    let mut pages = Vec::new();
    pages.push(CatalogPage {
        class: PageClass::Catalog,
        source: source_from_toc(hbk_path, locale, root_flat_page),
    });
    for (index, child) in root_page.children.iter().enumerate() {
        collect_child_catalog_pages(
            hbk_path,
            locale,
            child,
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
    toc_path: hbk_book::TocPath,
    pages: &mut Vec<CatalogPage>,
) {
    pages.push(CatalogPage {
        class: classify_catalog_page(page),
        source: SyntaxHelperSource {
            hbk_path: hbk_path.to_path_buf(),
            locale: locale.to_string(),
            toc_path: Some(toc_path.to_string()),
            html_path: page.html_path.clone(),
            page_title: page.title.display().to_string(),
        },
    });
    for (index, child) in page.children.iter().enumerate() {
        collect_child_catalog_pages(hbk_path, locale, child, toc_path.child(index), pages);
    }
}

fn classify_catalog_page(page: &TocPage) -> PageClass {
    let path = page.html_path.as_str();
    if is_catalog_path(path) {
        PageClass::Catalog
    } else if path.starts_with("objects/Global context/methods/") {
        PageClass::GlobalMethod
    } else if path.starts_with("objects/Global context/properties/") {
        PageClass::GlobalProperty
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
