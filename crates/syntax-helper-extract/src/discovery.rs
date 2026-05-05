use std::path::Path;

use hbk_book::{FlatTocPage, Toc, TocPage};
use hbk_docs::PageContent;
use syntax_helper_model::*;

use crate::catalog::collect_catalog_pages;
use crate::error::SyntaxHelperError;

pub(crate) fn discover_roots_with_loader(
    hbk_path: &Path,
    locale: &str,
    toc: &Toc,
    mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
) -> Result<RootDiscovery, SyntaxHelperError> {
    let mut roots = Vec::new();
    let mut diagnostics = Vec::new();

    for flat_page in toc.flat_pages().filter(|flat_page| {
        flat_page.index_path.indexes().len() == 1
            && is_syntax_helper_path(&flat_page.page.html_path)
    }) {
        let page = load_page(&flat_page.page.html_path)?;
        let source = source_from_page(hbk_path, locale, &flat_page, &page);
        let Some(kind) = classify_root(flat_page.page, &page) else {
            diagnostics.push(unknown_page_diagnostic(source));
            continue;
        };
        let pages = collect_catalog_pages(hbk_path, locale, flat_page.page, &flat_page);
        diagnostics.extend(
            pages
                .iter()
                .filter(|page| page.class == PageClass::Unknown)
                .cloned()
                .map(|page| unhandled_page_diagnostic(page.source)),
        );
        roots.push(RootSection {
            kind,
            source,
            pages,
        });
    }

    Ok(RootDiscovery { roots, diagnostics })
}

fn is_syntax_helper_path(html_path: &str) -> bool {
    html_path.starts_with("objects/")
}

fn classify_root(page: &TocPage, content: &PageContent) -> Option<RootSectionKind> {
    if is_global_context_page(page) {
        return Some(RootSectionKind::GlobalContext);
    }
    if is_enum_catalog_page(content) {
        return Some(RootSectionKind::EnumCatalog);
    }
    if is_type_object_catalog_page(page) {
        return Some(RootSectionKind::TypeObjectCatalog);
    }
    None
}

fn is_global_context_page(page: &TocPage) -> bool {
    let title = normalized_title(page);
    page.children.iter().any(|child| {
        child
            .html_path
            .starts_with("objects/Global context/methods/")
            || child
                .html_path
                .starts_with("objects/Global context/properties/")
    }) || title == "глобальный контекст"
        || title == "global context"
}

fn is_enum_catalog_page(content: &PageContent) -> bool {
    let title = normalized_text(&content.title);
    let body = normalized_text(&content.body_text);
    title == "системные перечисления"
        || title == "system enums"
        || title == "system enumerations"
        || body.contains("системные перечисления")
        || body.contains("system enums")
        || body.contains("system enumerations")
}

fn is_type_object_catalog_page(page: &TocPage) -> bool {
    page.html_path.starts_with("objects/catalog")
        && !page.children.is_empty()
        && page.children.iter().any(|child| {
            child
                .html_path
                .starts_with(page.html_path.trim_end_matches(".html"))
        })
}

fn source_from_page(
    hbk_path: &Path,
    locale: &str,
    flat_page: &FlatTocPage<'_>,
    content: &PageContent,
) -> SyntaxHelperSource {
    SyntaxHelperSource {
        hbk_path: hbk_path.to_path_buf(),
        locale: locale.to_string(),
        toc_path: Some(flat_page.index_path.to_string()),
        html_path: flat_page.page.html_path.clone(),
        page_title: if content.title.is_empty() {
            flat_page.page.title.display().to_string()
        } else {
            content.title.clone()
        },
    }
}

fn unknown_page_diagnostic(source: SyntaxHelperSource) -> SyntaxHelperDiagnostic {
    SyntaxHelperDiagnostic {
        severity: DiagnosticSeverity::Warning,
        code: "UNKNOWN_PAGE_CLASS",
        source,
        parser_stage: "root_discovery",
        message: "Syntax Assistant page could not be classified for traversal".to_string(),
    }
}

fn unhandled_page_diagnostic(source: SyntaxHelperSource) -> SyntaxHelperDiagnostic {
    let (code, message) = classified_page_gap(&source.html_path);
    SyntaxHelperDiagnostic {
        severity: DiagnosticSeverity::Warning,
        code,
        source,
        parser_stage: "root_discovery",
        message: message.to_string(),
    }
}

fn classified_page_gap(html_path: &str) -> (&'static str, &'static str) {
    if is_direct_global_context_page(html_path) {
        return (
            "UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE",
            "Direct global context method-like TOC page is in FR-SH-002 scope but is outside the supported Syntax Assistant method catalog layout",
        );
    }
    (
        "UNKNOWN_PAGE_CLASS",
        "Syntax Assistant page could not be classified for traversal",
    )
}

fn is_direct_global_context_page(html_path: &str) -> bool {
    html_path
        .strip_prefix("objects/Global context/")
        .is_some_and(|tail| !tail.contains('/') && tail.ends_with(".html"))
}

fn normalized_title(page: &TocPage) -> String {
    normalized_text(page.title.display())
}

fn normalized_text(value: &str) -> String {
    value.trim().to_lowercase()
}
