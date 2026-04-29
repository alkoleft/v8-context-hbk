use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use hbk_book::{BookError, FlatTocPage, HbkBook, Toc, TocPage};
use hbk_docs::{DocumentationError, DocumentationReader, PageContent, PageSource};
use scraper::{Html, Selector};
pub use syntax_helper_model::*;

#[derive(Debug)]
pub struct SyntaxHelperReader<'a> {
    book: &'a HbkBook,
}

impl<'a> SyntaxHelperReader<'a> {
    pub fn new(book: &'a HbkBook) -> Self {
        Self { book }
    }

    pub fn discover_roots(&self) -> Result<RootDiscovery, SyntaxHelperError> {
        discover_roots_with_loader(
            self.book.path(),
            self.book.locale().source_code(),
            self.book.toc(),
            |html_path| {
                DocumentationReader::new(self.book)
                    .load_page(html_path)
                    .map_err(SyntaxHelperError::Documentation)
            },
        )
    }

    pub fn extract(&self) -> Result<PlatformContext, SyntaxHelperError> {
        let root_paths = self
            .book
            .toc()
            .flat_pages()
            .filter(|flat_page| {
                flat_page.index_path.indexes().len() == 1
                    && is_syntax_helper_path(&flat_page.page.html_path)
            })
            .map(|flat_page| flat_page.page.html_path.clone())
            .collect::<Vec<_>>();
        let root_pages = self
            .book
            .read_pages(root_paths.iter().map(String::as_str))?;
        let discovery = discover_roots_with_loader(
            self.book.path(),
            self.book.locale().source_code(),
            self.book.toc(),
            |html_path| {
                let raw_html = root_pages.get(html_path).ok_or_else(|| {
                    SyntaxHelperError::Book(BookError::MissingZipEntry {
                        path: self.book.path().to_path_buf(),
                        entry_name: html_path.to_string(),
                    })
                })?;
                Ok(parse_syntax_page_content(
                    self.book.path(),
                    self.book.locale().source_code(),
                    self.book.toc(),
                    html_path,
                    raw_html,
                ))
            },
        )?;
        let page_paths = extraction_page_paths(&discovery);
        let pages = self
            .book
            .read_pages(page_paths.iter().map(String::as_str))?;
        let discovery = extraction_discovery(discovery);
        parse_extraction_pages(
            self.book.path(),
            self.book.locale().source_code(),
            self.book.toc(),
            discovery,
            |html_path| {
                let raw_html = pages.get(html_path).ok_or_else(|| {
                    SyntaxHelperError::Book(BookError::MissingZipEntry {
                        path: self.book.path().to_path_buf(),
                        entry_name: html_path.to_string(),
                    })
                })?;
                Ok(parse_syntax_page_content(
                    self.book.path(),
                    self.book.locale().source_code(),
                    self.book.toc(),
                    html_path,
                    raw_html,
                ))
            },
        )
    }
}

#[derive(Debug)]
pub enum SyntaxHelperError {
    Book(BookError),
    Documentation(DocumentationError),
}

impl fmt::Display for SyntaxHelperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Book(source) => write!(f, "{source}"),
            Self::Documentation(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for SyntaxHelperError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Book(source) => Some(source),
            Self::Documentation(source) => Some(source),
        }
    }
}

impl From<BookError> for SyntaxHelperError {
    fn from(value: BookError) -> Self {
        Self::Book(value)
    }
}

pub fn discover_roots_with_loader(
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
        let Some(kind) = classify_root(&flat_page.page, &page) else {
            diagnostics.push(unknown_page_diagnostic(source));
            continue;
        };
        let pages = collect_catalog_pages(hbk_path, locale, &flat_page.page, &flat_page);
        diagnostics.extend(
            pages
                .iter()
                .filter(|page| page.class == PageClass::Unknown)
                .cloned()
                .map(|page| unknown_page_diagnostic(page.source)),
        );
        roots.push(RootSection {
            kind,
            source,
            pages,
        });
    }

    Ok(RootDiscovery { roots, diagnostics })
}

pub fn extract_with_loader(
    hbk_path: &Path,
    locale: &str,
    toc: &Toc,
    mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
) -> Result<PlatformContext, SyntaxHelperError> {
    let discovery = discover_roots_with_loader(hbk_path, locale, toc, &mut load_page)?;
    parse_extraction_pages(hbk_path, locale, toc, discovery, load_page)
}

fn extraction_page_paths(discovery: &RootDiscovery) -> Vec<String> {
    let mut paths = BTreeSet::new();
    for root in &discovery.roots {
        if root.kind == RootSectionKind::GlobalContext {
            paths.insert(root.source.html_path.clone());
        }
        for page in &root.pages {
            if !matches!(page.class, PageClass::Catalog | PageClass::Unknown) {
                paths.insert(page.source.html_path.clone());
            }
        }
    }
    paths.into_iter().collect()
}

fn extraction_discovery(mut discovery: RootDiscovery) -> RootDiscovery {
    for root in &mut discovery.roots {
        root.pages
            .retain(|page| !matches!(page.class, PageClass::Catalog | PageClass::Unknown));
    }
    discovery
}

fn parse_extraction_pages(
    _hbk_path: &Path,
    _locale: &str,
    _toc: &Toc,
    discovery: RootDiscovery,
    mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
) -> Result<PlatformContext, SyntaxHelperError> {
    let mut context = PlatformContext {
        diagnostics: discovery.diagnostics,
        ..PlatformContext::default()
    };
    let mut visited = BTreeSet::new();

    for root in &discovery.roots {
        for catalog_page in &root.pages {
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
                PageClass::GlobalMethod => context
                    .global_methods
                    .push(parse_global_method(&content, source)),
                PageClass::GlobalProperty => context
                    .global_properties
                    .push(parse_global_property(&content, source)),
                PageClass::ObjectType => context
                    .platform_types
                    .push(parse_platform_type(&content, source)),
                PageClass::ObjectMethod => context
                    .type_methods
                    .push(parse_platform_method(&content, source)),
                PageClass::ObjectProperty => context
                    .type_properties
                    .push(parse_platform_property(&content, source)),
                PageClass::Constructor => context
                    .constructors
                    .push(parse_constructor(&content, source)),
                PageClass::Enum => context.enums.push(parse_enum(&content, source)),
                PageClass::EnumValue => {
                    context.enum_values.push(parse_enum_value(&content, source))
                }
            }
        }

        if root.kind == RootSectionKind::GlobalContext
            && visited.insert(root.source.html_path.clone())
        {
            let content = load_page(&root.source.html_path)?;
            let source = source_from_content(&root.source, &content);
            context
                .global_contexts
                .push(parse_global_context(&content, source));
        }
    }

    Ok(context)
}

pub fn parse_global_context(content: &PageContent, source: SyntaxHelperSource) -> GlobalContext {
    GlobalContext {
        name: page_title_name(content),
        property_links: links_in_section(content, &["Свойства:", "Properties:"]),
        method_links: links_in_section(content, &["Методы:", "Methods:"]),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_global_method(content: &PageContent, source: SyntaxHelperSource) -> GlobalMethod {
    GlobalMethod {
        name: heading_name(content),
        signatures: parse_signatures(content),
        return_types: type_refs_from_section(content, &["Возвращаемое значение:", "Return value:"]),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_global_property(content: &PageContent, source: SyntaxHelperSource) -> GlobalProperty {
    GlobalProperty {
        name: heading_name(content),
        usage: section_text(content, &["Использование:", "Use:"]),
        type_refs: type_refs_from_section(content, &["Описание:", "Description:"]),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_platform_type(content: &PageContent, source: SyntaxHelperSource) -> PlatformType {
    PlatformType {
        name: page_title_name(content),
        method_links: links_in_section(content, &["Методы:", "Methods:"]),
        constructor_links: links_in_section(content, &["Конструкторы:", "Constructors:"]),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_platform_method(content: &PageContent, source: SyntaxHelperSource) -> PlatformMethod {
    PlatformMethod {
        owner: title_name(content),
        name: heading_name(content),
        signatures: parse_signatures(content),
        return_types: type_refs_from_section(content, &["Возвращаемое значение:", "Return value:"]),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_platform_property(
    content: &PageContent,
    source: SyntaxHelperSource,
) -> PlatformProperty {
    PlatformProperty {
        owner: title_name(content),
        name: heading_name(content),
        usage: section_text(content, &["Использование:", "Use:"]),
        type_refs: type_refs_from_section(content, &["Описание:", "Description:"]),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_constructor(content: &PageContent, source: SyntaxHelperSource) -> Constructor {
    Constructor {
        owner: title_name(content),
        name: heading_name(content),
        signatures: parse_signatures(content),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_enum(content: &PageContent, source: SyntaxHelperSource) -> EnumDefinition {
    EnumDefinition {
        name: page_title_name(content),
        value_links: links_in_section(content, &["Значения", "Values"]),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_enum_value(content: &PageContent, source: SyntaxHelperSource) -> EnumValue {
    EnumValue {
        owner: title_name(content),
        name: heading_name(content),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
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

fn collect_catalog_pages(
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

fn unknown_page_diagnostic(source: SyntaxHelperSource) -> SyntaxHelperDiagnostic {
    SyntaxHelperDiagnostic {
        severity: DiagnosticSeverity::Warning,
        code: "UNKNOWN_PAGE_CLASS",
        source,
        parser_stage: "root_discovery",
        message: "Syntax Assistant page could not be classified for traversal".to_string(),
    }
}

fn normalized_title(page: &TocPage) -> String {
    normalized_text(page.title.display())
}

fn normalized_text(value: &str) -> String {
    value.trim().to_lowercase()
}

fn source_from_content(fallback: &SyntaxHelperSource, content: &PageContent) -> SyntaxHelperSource {
    SyntaxHelperSource {
        hbk_path: content.source.hbk_path.clone(),
        locale: content.source.locale.clone(),
        toc_path: content
            .source
            .toc_path
            .clone()
            .or_else(|| fallback.toc_path.clone()),
        html_path: content.source.html_path.clone(),
        page_title: if content.title.is_empty() {
            fallback.page_title.clone()
        } else {
            content.title.clone()
        },
    }
}

fn parse_syntax_page_content(
    hbk_path: &Path,
    locale: &str,
    toc: &Toc,
    html_path: &str,
    raw_html: &str,
) -> PageContent {
    let normalized_page_path = html_path.trim_start_matches('/').to_string();
    let toc_page = toc
        .flat_pages()
        .find(|flat_page| flat_page.page.html_path == normalized_page_path);
    let toc_path = toc_page
        .as_ref()
        .map(|flat_page| flat_page.index_path.to_string());
    let toc_title = toc_page
        .as_ref()
        .map(|flat_page| flat_page.page.title.display().to_string());
    let title = select_first_html_text(raw_html, ".V8SH_pagetitle")
        .or_else(|| select_first_html_text(raw_html, "title"))
        .or_else(|| toc_title.clone())
        .unwrap_or_default();
    let body_text = body_text(raw_html);
    let text_preview = body_text.chars().take(240).collect();

    PageContent {
        source: PageSource {
            hbk_path: hbk_path.to_path_buf(),
            locale: locale.to_string(),
            toc_path,
            html_path: normalized_page_path,
            toc_title,
        },
        title,
        raw_html: raw_html.to_string(),
        body_text,
        text_preview,
        links: Vec::new(),
        diagnostics: Vec::new(),
    }
}

fn page_title_name(content: &PageContent) -> LocalizedName {
    name_from_text(
        &select_first_html_text(&content.raw_html, ".V8SH_pagetitle")
            .unwrap_or_else(|| content.title.clone()),
    )
}

fn title_name(content: &PageContent) -> LocalizedName {
    name_from_text(
        &select_first_html_text(&content.raw_html, ".V8SH_title")
            .unwrap_or_else(|| content.title.clone()),
    )
}

fn heading_name(content: &PageContent) -> LocalizedName {
    name_from_text(
        &select_first_html_text(&content.raw_html, ".V8SH_heading")
            .unwrap_or_else(|| content.title.clone()),
    )
}

fn name_from_text(value: &str) -> LocalizedName {
    let value = value.trim();
    if let Some((primary, alias)) = split_parenthesized_alias(value) {
        LocalizedName {
            primary,
            alias: Some(alias),
        }
    } else {
        LocalizedName {
            primary: value.to_string(),
            alias: None,
        }
    }
}

fn split_parenthesized_alias(value: &str) -> Option<(String, String)> {
    let value = value.trim();
    let alias_end = value.strip_suffix(')')?;
    let alias_start = alias_end.rfind(" (")?;
    let primary = alias_end[..alias_start].trim();
    let alias = alias_end[alias_start + 2..].trim();
    (!primary.is_empty() && !alias.is_empty()).then(|| (primary.to_string(), alias.to_string()))
}

fn select_first_html_text(raw_html: &str, selector: &str) -> Option<String> {
    if let Some(class_name) = selector.strip_prefix('.') {
        return select_first_class_text(raw_html, class_name);
    }
    if selector == "title" {
        return select_first_tag_text(raw_html, "title");
    }
    let document = Html::parse_document(raw_html);
    let selector = Selector::parse(selector).expect("static selector must be valid");
    document
        .select(&selector)
        .find_map(|element| non_empty_text(element.text()))
}

fn body_text(raw_html: &str) -> String {
    let body = raw_html
        .find("<body")
        .and_then(|start| raw_html[start..].find('>').map(|offset| start + offset + 1))
        .and_then(|start| {
            raw_html[start..]
                .find("</body>")
                .map(|end| &raw_html[start..start + end])
        })
        .unwrap_or(raw_html);
    text_from_html_fragment(body)
}

fn select_first_class_text(raw_html: &str, class_name: &str) -> Option<String> {
    let class_marker = format!("class=\"{class_name}\"");
    let start = raw_html.find(&class_marker)?;
    let tag_start = raw_html[..start].rfind('<')?;
    let content_start = raw_html[start..]
        .find('>')
        .map(|offset| start + offset + 1)?;
    let tag_name = raw_html[tag_start + 1..]
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_start_matches('/');
    let end_tag = format!("</{tag_name}>");
    let content_end = raw_html[content_start..]
        .find(&end_tag)
        .map(|offset| content_start + offset)?;
    let text = text_from_html_fragment(&raw_html[content_start..content_end]);
    (!text.is_empty()).then_some(text)
}

fn select_first_tag_text(raw_html: &str, tag_name: &str) -> Option<String> {
    let start_tag = format!("<{tag_name}");
    let start = raw_html.find(&start_tag)?;
    let content_start = raw_html[start..]
        .find('>')
        .map(|offset| start + offset + 1)?;
    let end_tag = format!("</{tag_name}>");
    let content_end = raw_html[content_start..]
        .find(&end_tag)
        .map(|offset| content_start + offset)?;
    let text = text_from_html_fragment(&raw_html[content_start..content_end]);
    (!text.is_empty()).then_some(text)
}

fn text_from_html_fragment(fragment: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    let mut entity = String::new();
    let mut in_entity = false;
    let mut chars = fragment.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
                output.push(' ');
            }
            continue;
        }
        if in_entity {
            if ch == ';' {
                output.push_str(decode_entity(&entity));
                entity.clear();
                in_entity = false;
            } else {
                entity.push(ch);
            }
            continue;
        }
        match ch {
            '<' if chars
                .peek()
                .is_some_and(|next| next.is_ascii_alphabetic() || *next == '/') =>
            {
                in_tag = true
            }
            '<' => output.push('<'),
            '&' => in_entity = true,
            ch if ch.is_whitespace() => output.push(' '),
            ch => output.push(ch),
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_lines_from_html_fragment(fragment: &str) -> String {
    let with_breaks = fragment
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n");
    with_breaks
        .lines()
        .map(text_from_html_fragment)
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn anchor_links(section_html: &str, current_html_path: &str) -> Vec<MemberLink> {
    let mut links = Vec::new();
    let mut rest = section_html;
    while let Some(anchor_start) = rest.find("<a ") {
        rest = &rest[anchor_start..];
        let Some(tag_end) = rest.find('>') else {
            break;
        };
        let tag = &rest[..tag_end + 1];
        let Some(raw_href) = attr_value(tag, "href") else {
            rest = &rest[tag_end + 1..];
            continue;
        };
        let Some(anchor_end) = rest[tag_end + 1..].find("</a>") else {
            break;
        };
        let inner = &rest[tag_end + 1..tag_end + 1 + anchor_end];
        let text = text_from_html_fragment(inner);
        if !text.is_empty() {
            links.push(MemberLink {
                name: name_from_text(&text),
                html_path: normalize_member_href(current_html_path, &raw_href),
            });
        }
        rest = &rest[tag_end + 1 + anchor_end + 4..];
    }
    links
}

fn attr_value(tag: &str, attr_name: &str) -> Option<String> {
    let attr = format!("{attr_name}=\"");
    let start = tag.find(&attr)? + attr.len();
    let end = tag[start..].find('"')?;
    Some(tag[start..start + end].to_string())
}

fn bracketed_name_ranges(section: &str) -> Vec<(usize, usize, String)> {
    let mut ranges = Vec::new();
    let mut offset = 0;
    while let Some(start) = section[offset..].find('<').map(|start| offset + start) {
        let Some(end) = section[start + 1..].find('>').map(|end| start + 1 + end) else {
            break;
        };
        ranges.push((start, end + 1, section[start + 1..end].to_string()));
        offset = end + 1;
    }
    ranges
}

fn decode_entity(entity: &str) -> &str {
    match entity {
        "lt" => "<",
        "gt" => ">",
        "amp" => "&",
        "quot" => "\"",
        "nbsp" => " ",
        _ => "",
    }
}

fn links_in_section(content: &PageContent, labels: &[&str]) -> Vec<MemberLink> {
    let Some(section_html) = section_html(&content.raw_html, labels) else {
        return Vec::new();
    };
    anchor_links(&section_html, &content.source.html_path)
}

fn parse_signatures(content: &PageContent) -> Vec<Signature> {
    let Some(section_html) = section_html(&content.raw_html, &["Синтаксис:", "Syntax:"])
    else {
        return Vec::new();
    };
    let parameters = parse_parameters(content);
    text_lines_from_html_fragment(&section_html)
        .split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| Signature {
            text: line.to_string(),
            parameters: parameters_for_signature(line, &parameters),
        })
        .collect()
}

fn parse_parameters(content: &PageContent) -> Vec<Parameter> {
    let Some(section) = section_text(content, &["Параметры:", "Parameters:"]) else {
        return Vec::new();
    };
    let ranges = bracketed_name_ranges(&section);
    ranges
        .iter()
        .enumerate()
        .filter_map(|(index, (_start, end, name))| {
            if name.trim().is_empty() {
                return None;
            }
            let next_start = ranges
                .get(index + 1)
                .map(|(next_start, _, _)| *next_start)
                .unwrap_or(section.len());
            let parameter_text = &section[*end..next_start];
            let lower = parameter_text.to_lowercase();
            let required = !(lower.contains("необязательный") || lower.contains("optional"));
            let type_refs = parse_type_refs(parameter_text);
            let description = parameter_text
                .split_once('.')
                .map(|(_, tail)| tail.trim())
                .filter(|tail| !tail.is_empty())
                .map(ToOwned::to_owned);
            Some(Parameter {
                name: name.trim().to_string(),
                required,
                type_refs: type_refs.clone(),
                description,
            })
        })
        .collect()
}

fn parameters_for_signature(signature: &str, parameters: &[Parameter]) -> Vec<Parameter> {
    parameters
        .iter()
        .filter(|parameter| signature.contains(&format!("<{}>", parameter.name)))
        .cloned()
        .collect()
}

fn type_refs_from_section(content: &PageContent, labels: &[&str]) -> Vec<TypeRef> {
    section_text(content, labels)
        .map(|section| parse_type_refs(&section))
        .unwrap_or_default()
}

fn parse_type_refs(section: &str) -> Vec<TypeRef> {
    let Some((_, after_type)) = section.split_once("Тип:") else {
        return Vec::new();
    };
    let type_part = after_type
        .split_once('.')
        .map(|(head, _)| head)
        .unwrap_or(after_type);
    type_part
        .split([',', ';'])
        .map(|value| value.trim().trim_matches('.'))
        .filter(|value| !value.is_empty())
        .map(|value| TypeRef {
            name: value.to_string(),
        })
        .collect()
}

fn section_text(content: &PageContent, labels: &[&str]) -> Option<String> {
    let body = &content.body_text;
    let (label, start) = find_label(body, labels)?;
    let section_start = start + label.len();
    let section_end = ALL_SECTION_LABELS
        .iter()
        .filter(|candidate| **candidate != label)
        .filter_map(|candidate| {
            body[section_start..]
                .find(candidate)
                .map(|index| section_start + index)
        })
        .min()
        .unwrap_or(body.len());
    let value = body[section_start..section_end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn section_html(raw_html: &str, labels: &[&str]) -> Option<String> {
    let (label, start) = find_label(raw_html, labels)?;
    let chapter_end = raw_html[start..]
        .find("</p>")
        .map(|index| start + index + 4)?;
    let section_end = ALL_SECTION_LABELS
        .iter()
        .filter(|candidate| **candidate != label)
        .filter_map(|candidate| {
            raw_html[chapter_end..]
                .find(candidate)
                .map(|index| chapter_end + index)
        })
        .min()
        .unwrap_or(raw_html.len());
    Some(raw_html[chapter_end..section_end].to_string())
}

fn find_label<'a>(value: &str, labels: &'a [&str]) -> Option<(&'a str, usize)> {
    labels
        .iter()
        .filter_map(|label| value.find(label).map(|index| (*label, index)))
        .min_by_key(|(_, index)| *index)
}

fn normalize_member_href(current_html_path: &str, href: &str) -> String {
    let without_scheme = href
        .strip_prefix("v8help://SyntaxHelperContext/")
        .or_else(|| href.strip_prefix("v8help://"))
        .unwrap_or(href);
    let path = without_scheme.split(['#', '?']).next().unwrap_or_default();
    if path.starts_with('/') || path.starts_with("objects/") {
        return path.trim_start_matches('/').to_string();
    }
    let base = current_html_path
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or("");
    if base.is_empty() {
        path.to_string()
    } else {
        format!("{base}/{path}")
    }
}

fn non_empty_text<'a>(parts: impl Iterator<Item = &'a str>) -> Option<String> {
    let text = parts.collect::<Vec<_>>().join(" ");
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

const ALL_SECTION_LABELS: &[&str] = &[
    "Свойства:",
    "Properties:",
    "Методы:",
    "Methods:",
    "События:",
    "Events:",
    "Синтаксис:",
    "Syntax:",
    "Параметры:",
    "Parameters:",
    "Возвращаемое значение:",
    "Return value:",
    "Использование:",
    "Use:",
    "Значения",
    "Values",
    "Элементы коллекции:",
    "Collection items:",
    "Конструкторы:",
    "Constructors:",
    "Описание:",
    "Description:",
    "Примечание:",
    "Note:",
    "Использование в версии:",
    "Available since:",
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use super::*;
    use hbk_book::HbkBook;
    use hbk_book::Toc;
    use hbk_docs::parse_page_html;

    #[test]
    fn discovers_roots_and_traverses_catalogs_from_fixture_toc() {
        let toc = fixture_toc();
        let discovery =
            discover_roots_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
                Ok(fixture_content(&toc, html_path))
            })
            .expect("root discovery must succeed");

        assert!(discovery.has_kind(RootSectionKind::GlobalContext));
        assert!(discovery.has_kind(RootSectionKind::EnumCatalog));
        assert!(discovery.has_kind(RootSectionKind::TypeObjectCatalog));
        assert_eq!(discovery.roots.len(), 3);

        let classes = discovery
            .roots
            .iter()
            .flat_map(|root| root.pages.iter().map(|page| page.class))
            .collect::<BTreeSet<_>>();
        assert!(classes.contains(&PageClass::GlobalMethod));
        assert!(classes.contains(&PageClass::GlobalProperty));
        assert!(classes.contains(&PageClass::Enum));
        assert!(classes.contains(&PageClass::EnumValue));
        assert!(classes.contains(&PageClass::ObjectType));
        assert!(classes.contains(&PageClass::ObjectMethod));
        assert!(classes.contains(&PageClass::ObjectProperty));
        assert!(classes.contains(&PageClass::Constructor));

        assert_eq!(discovery.diagnostics.len(), 1);
        assert_eq!(discovery.diagnostics[0].code, "UNKNOWN_PAGE_CLASS");
        assert_eq!(
            discovery.diagnostics[0].severity,
            DiagnosticSeverity::Warning
        );
        assert_eq!(
            discovery.diagnostics[0].source.hbk_path,
            Path::new("shcntx_ru.hbk")
        );
        assert_eq!(discovery.diagnostics[0].source.locale, "ru");
        assert_eq!(
            discovery.diagnostics[0].source.toc_path.as_deref(),
            Some("3")
        );
        assert_eq!(
            discovery.diagnostics[0].source.html_path,
            "objects/unknown.html"
        );
        assert_eq!(
            discovery.diagnostics[0].source.page_title,
            "Неизвестный раздел"
        );
        assert_eq!(discovery.diagnostics[0].parser_stage, "root_discovery");
    }

    #[test]
    fn parses_representative_specialized_fixture_pages() {
        let toc = fixture_toc();

        let global_context = parse_global_context(
            &fixture_content(&toc, "objects/Global context.html"),
            source("objects/Global context.html"),
        );
        assert_eq!(global_context.name.primary, "Глобальный контекст");
        assert!(
            global_context
                .method_links
                .iter()
                .any(|link| link.name.primary == "XMLСтрока"
                    && link.name.alias.as_deref() == Some("XMLString"))
        );
        assert!(
            global_context
                .property_links
                .iter()
                .any(|link| link.name.primary == "WebSocketКлиентСоединения")
        );

        let global_method = parse_global_method(
            &fixture_content(
                &toc,
                "objects/Global context/methods/catalog1566/XMLString1567.html",
            ),
            source("objects/Global context/methods/catalog1566/XMLString1567.html"),
        );
        assert_eq!(global_method.name.primary, "XMLСтрока");
        assert_eq!(global_method.name.alias.as_deref(), Some("XMLString"));
        assert_eq!(global_method.signatures[0].text, "XMLСтрока(<Значение>)");
        assert!(global_method.signatures[0].parameters[0].required);
        assert!(
            global_method
                .return_types
                .iter()
                .any(|type_ref| type_ref.name == "Строка")
        );

        let global_property = parse_global_property(
            &fixture_content(&toc, "objects/Global context/properties/Catalogs336.html"),
            source("objects/Global context/properties/Catalogs336.html"),
        );
        assert_eq!(global_property.name.primary, "Справочники");
        assert_eq!(global_property.name.alias.as_deref(), Some("Catalogs"));
        assert_eq!(global_property.usage.as_deref(), Some("Только чтение."));
        assert!(
            global_property
                .type_refs
                .iter()
                .any(|type_ref| type_ref.name == "СправочникиМенеджер")
        );

        let platform_type = parse_platform_type(
            &fixture_content(&toc, "objects/catalog234/Array.html"),
            source("objects/catalog234/Array.html"),
        );
        assert_eq!(platform_type.name.primary, "Массив");
        assert!(
            platform_type
                .method_links
                .iter()
                .any(|link| link.name.alias.as_deref() == Some("Add"))
        );
        assert!(
            platform_type
                .constructor_links
                .iter()
                .any(|link| link.name.primary == "По количеству элементов")
        );

        let method = parse_platform_method(
            &fixture_content(&toc, "objects/catalog234/Array/methods/Add772.html"),
            source("objects/catalog234/Array/methods/Add772.html"),
        );
        assert_eq!(method.owner.primary, "Массив");
        assert_eq!(method.name.primary, "Добавить");
        assert!(!method.signatures[0].parameters[0].required);

        let property = parse_platform_property(
            &fixture_content(
                &toc,
                "objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html",
            ),
            source("objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html"),
        );
        assert_eq!(property.owner.primary, "ГруппаФормы");
        assert_eq!(property.name.alias.as_deref(), Some("Visible"));
        assert!(
            property
                .type_refs
                .iter()
                .any(|type_ref| type_ref.name == "Булево")
        );

        let constructor = parse_constructor(
            &fixture_content(&toc, "objects/catalog234/Array/ctors/ctor13.html"),
            source("objects/catalog234/Array/ctors/ctor13.html"),
        );
        assert_eq!(constructor.owner.primary, "Массив");
        assert_eq!(constructor.name.primary, "По количеству элементов");
        assert_eq!(
            constructor.signatures[0].text,
            "Новый Массив(<КоличествоЭлементов1>,...,<КоличествоЭлементовN>)"
        );

        let enum_definition = parse_enum(
            &fixture_content(&toc, "objects/catalog2/catalog2300/JSONValueType.html"),
            source("objects/catalog2/catalog2300/JSONValueType.html"),
        );
        assert_eq!(enum_definition.name.primary, "ТипЗначенияJSON");
        assert!(
            enum_definition
                .value_links
                .iter()
                .any(|link| link.name.alias.as_deref() == Some("ArrayEnd"))
        );

        let enum_value = parse_enum_value(
            &fixture_content(
                &toc,
                "objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html",
            ),
            source("objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html"),
        );
        assert_eq!(enum_value.owner.primary, "ТипЗначенияJSON");
        assert_eq!(enum_value.name.primary, "КонецМассива");
        assert!(
            enum_value
                .description
                .as_deref()
                .is_some_and(|text| text.contains("JSON"))
        );
    }

    #[test]
    fn extracts_platform_context_from_fixture_toc() {
        let toc = fixture_toc();
        let context = extract_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
            Ok(fixture_content(&toc, html_path))
        })
        .expect("fixture extraction must succeed");

        assert_eq!(context.global_contexts.len(), 1);
        assert!(
            context
                .global_methods
                .iter()
                .any(|method| method.name.alias.as_deref() == Some("XMLString"))
        );
        assert!(
            context
                .global_properties
                .iter()
                .any(|property| property.name.alias.as_deref() == Some("Catalogs"))
        );
        assert!(
            context
                .platform_types
                .iter()
                .any(|platform_type| platform_type.name.alias.as_deref() == Some("Array"))
        );
        assert!(
            context
                .type_methods
                .iter()
                .any(|method| method.name.alias.as_deref() == Some("Add"))
        );
        assert!(
            context
                .type_properties
                .iter()
                .any(|property| property.name.alias.as_deref() == Some("Visible"))
        );
        assert!(
            context
                .constructors
                .iter()
                .any(|constructor| constructor.name.primary == "По количеству элементов")
        );
        assert!(
            context
                .enums
                .iter()
                .any(|enum_definition| enum_definition.name.alias.as_deref()
                    == Some("JSONValueType"))
        );
        assert!(
            context
                .enum_values
                .iter()
                .any(|enum_value| enum_value.name.alias.as_deref() == Some("ArrayEnd"))
        );
        assert_eq!(context.diagnostics.len(), 1);
    }

    #[test]
    fn lookup_helpers_find_exact_names_and_aliases() {
        let toc = fixture_toc();
        let context = extract_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
            Ok(fixture_content(&toc, html_path))
        })
        .expect("fixture extraction must succeed");

        let global_member = context
            .find_global_member("XMLString")
            .expect("global member lookup must not be ambiguous")
            .expect("global member must be found by alias");
        assert!(
            matches!(global_member, GlobalMemberRef::Method(method) if method.name.primary == "XMLСтрока")
        );

        let platform_type = context
            .find_type("Array")
            .expect("type lookup must not be ambiguous")
            .expect("type must be found by alias");
        assert_eq!(platform_type.name.primary, "Массив");

        let type_member = context
            .find_type_member("Array", "Add")
            .expect("type member lookup must not be ambiguous")
            .expect("type member must be found by aliases");
        assert!(
            matches!(type_member, TypeMemberRef::Method(method) if method.name.primary == "Добавить")
        );

        let constructors = context
            .constructors_for_type("Array")
            .expect("constructor lookup must not be ambiguous")
            .expect("type must be found by alias");
        assert_eq!(constructors.len(), 1);
        assert_eq!(constructors[0].name.primary, "По количеству элементов");
    }

    #[test]
    fn lookup_helpers_return_missing_without_guessing() {
        let context = PlatformContext::default();

        assert_eq!(context.find_global_member("Missing").unwrap(), None);
        assert_eq!(context.find_type("Missing").unwrap(), None);
        assert_eq!(context.find_type_member("Array", "Missing").unwrap(), None);
        assert_eq!(context.constructors_for_type("Array").unwrap(), None);
    }

    #[test]
    fn constructor_lookup_distinguishes_type_without_constructors() {
        let context = PlatformContext {
            platform_types: vec![PlatformType {
                name: LocalizedName {
                    primary: "Тест".to_string(),
                    alias: Some("Test".to_string()),
                },
                method_links: Vec::new(),
                constructor_links: Vec::new(),
                description: None,
                source: source("objects/Test.html"),
            }],
            ..PlatformContext::default()
        };

        let constructors = context
            .constructors_for_type("Test")
            .expect("constructor lookup must not be ambiguous")
            .expect("existing type must be distinguished from a missing type");
        assert!(constructors.is_empty());
    }

    #[test]
    fn type_bound_lookup_does_not_cross_match_alias_to_other_primary_name() {
        let aliased_type = LocalizedName {
            primary: "Тест".to_string(),
            alias: Some("Test".to_string()),
        };
        let other_type = LocalizedName {
            primary: "Test".to_string(),
            alias: None,
        };
        let context = PlatformContext {
            platform_types: vec![
                PlatformType {
                    name: aliased_type,
                    method_links: Vec::new(),
                    constructor_links: Vec::new(),
                    description: None,
                    source: source("objects/AliasedTest.html"),
                },
                PlatformType {
                    name: other_type.clone(),
                    method_links: Vec::new(),
                    constructor_links: Vec::new(),
                    description: None,
                    source: source("objects/OtherTest.html"),
                },
            ],
            type_methods: vec![PlatformMethod {
                owner: other_type.clone(),
                name: LocalizedName {
                    primary: "Ping".to_string(),
                    alias: None,
                },
                signatures: Vec::new(),
                return_types: Vec::new(),
                description: None,
                source: source("objects/OtherTest/methods/Ping.html"),
            }],
            constructors: vec![Constructor {
                owner: other_type,
                name: LocalizedName {
                    primary: "New".to_string(),
                    alias: None,
                },
                signatures: Vec::new(),
                description: None,
                source: source("objects/OtherTest/ctors/New.html"),
            }],
            ..PlatformContext::default()
        };

        assert_eq!(context.find_type_member("Тест", "Ping").unwrap(), None);
        assert!(
            context
                .constructors_for_type("Тест")
                .expect("type lookup must not be ambiguous")
                .expect("aliased type must exist")
                .is_empty()
        );
    }

    #[test]
    fn lookup_helpers_report_ambiguous_exact_matches() {
        let toc = fixture_toc();
        let mut context =
            extract_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
                Ok(fixture_content(&toc, html_path))
            })
            .expect("fixture extraction must succeed");

        let duplicate_global = GlobalMethod {
            name: LocalizedName {
                primary: "XMLString".to_string(),
                alias: None,
            },
            signatures: Vec::new(),
            return_types: Vec::new(),
            description: None,
            source: source("objects/duplicate-global.html"),
        };
        context.global_methods.push(duplicate_global);

        assert!(matches!(
            context.find_global_member("XMLString"),
            Err(LookupError::Ambiguous {
                kind: LookupKind::GlobalMember,
                ..
            })
        ));

        let duplicate_member = PlatformProperty {
            owner: context
                .find_type("Array")
                .expect("type lookup must not be ambiguous before duplicate type")
                .expect("fixture type must exist")
                .name
                .clone(),
            name: LocalizedName {
                primary: "Add".to_string(),
                alias: None,
            },
            usage: None,
            type_refs: Vec::new(),
            description: None,
            source: source("objects/duplicate-member.html"),
        };
        context.type_properties.push(duplicate_member);

        assert!(matches!(
            context.find_type_member("Array", "Add"),
            Err(LookupError::Ambiguous {
                kind: LookupKind::TypeMember,
                ..
            })
        ));

        let duplicate_type = PlatformType {
            name: LocalizedName {
                primary: "Array".to_string(),
                alias: None,
            },
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: None,
            source: source("objects/duplicate-type.html"),
        };
        context.platform_types.push(duplicate_type);

        assert!(matches!(
            context.find_type("Array"),
            Err(LookupError::Ambiguous {
                kind: LookupKind::Type,
                ..
            })
        ));

        let mut context_with_ambiguous_type =
            extract_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
                Ok(fixture_content(&toc, html_path))
            })
            .expect("fixture extraction must succeed");
        context_with_ambiguous_type
            .platform_types
            .push(PlatformType {
                name: LocalizedName {
                    primary: "Array".to_string(),
                    alias: None,
                },
                method_links: Vec::new(),
                constructor_links: Vec::new(),
                description: None,
                source: source("objects/duplicate-type.html"),
            });

        assert!(matches!(
            context_with_ambiguous_type.find_type_member("Array", "Add"),
            Err(LookupError::Ambiguous {
                kind: LookupKind::Type,
                ..
            })
        ));

        assert!(matches!(
            context.constructors_for_type("Array"),
            Err(LookupError::Ambiguous {
                kind: LookupKind::Type,
                ..
            })
        ));
    }

    #[test]
    fn binds_parameters_to_the_signature_that_mentions_them() {
        let toc = fixture_toc();
        let html = r#"
            <html><body>
            <h1 class="V8SH_pagetitle">Тест.Метод</h1>
            <p class="V8SH_title">Тест</p>
            <p class="V8SH_heading">Метод</p>
            <p class="V8SH_chapter">Синтаксис:</p>
            Метод()<br>
            Метод(&lt;СтрокаЗначение&gt;, &lt;ЧислоЗначение&gt;)
            <p class="V8SH_chapter">Параметры:</p>
            <div class="V8SH_rubric"><p>&lt;СтрокаЗначение&gt; (обязательный)</p></div>
            Тип: Строка. Первый параметр.
            <div class="V8SH_rubric"><p>&lt;ЧислоЗначение&gt; (необязательный)</p></div>
            Тип: Число. Второй параметр.
            </body></html>
        "#;
        let content = parse_syntax_page_content(
            Path::new("shcntx_ru.hbk"),
            "ru",
            &toc,
            "objects/catalog234/Test/methods/Method.html",
            html,
        );
        let signatures = parse_signatures(&content);

        assert_eq!(signatures.len(), 2);
        assert!(signatures[0].parameters.is_empty());
        assert_eq!(signatures[1].parameters.len(), 2);
        assert_eq!(signatures[1].parameters[0].name, "СтрокаЗначение");
        assert_eq!(signatures[1].parameters[1].name, "ЧислоЗначение");
        assert!(signatures[1].parameters[0].required);
        assert!(!signatures[1].parameters[1].required);
        assert_eq!(signatures[1].parameters[0].type_refs[0].name, "Строка");
        assert_eq!(signatures[1].parameters[1].type_refs[0].name, "Число");
    }

    #[test]
    fn real_shcntx_ru_root_discovery_includes_required_root_candidates_when_fixture_exists() {
        let path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk");
        if !path.exists() {
            eprintln!(
                "real-platform root discovery smoke skipped because {} is unavailable",
                path.display()
            );
            return;
        }

        let book = HbkBook::open(path).expect("real Syntax Assistant book must open");
        let discovery = SyntaxHelperReader::new(&book)
            .discover_roots()
            .expect("real Syntax Assistant roots must be discoverable");

        assert!(discovery.has_kind(RootSectionKind::GlobalContext));
        assert!(discovery.has_kind(RootSectionKind::EnumCatalog));
        assert!(discovery.has_kind(RootSectionKind::TypeObjectCatalog));

        let global_context = discovery
            .roots
            .iter()
            .find(|root| root.kind == RootSectionKind::GlobalContext)
            .expect("global context root must be present");
        assert_eq!(
            global_context.source.html_path,
            "objects/Global context.html"
        );
        assert!(
            global_context
                .pages
                .iter()
                .any(|page| page.class == PageClass::GlobalMethod)
        );
        assert!(
            global_context
                .pages
                .iter()
                .any(|page| page.class == PageClass::GlobalProperty)
        );

        let enum_catalog = discovery
            .roots
            .iter()
            .find(|root| root.kind == RootSectionKind::EnumCatalog)
            .expect("enum catalog root must be present");
        assert!(
            enum_catalog
                .pages
                .iter()
                .any(|page| page.class == PageClass::Enum)
        );
        assert!(
            enum_catalog
                .pages
                .iter()
                .any(|page| page.class == PageClass::EnumValue)
        );

        let type_catalog = discovery
            .roots
            .iter()
            .find(|root| {
                root.kind == RootSectionKind::TypeObjectCatalog
                    && root.source.html_path == "objects/catalog234.html"
            })
            .expect("known type/object catalog root must be present");
        assert!(
            type_catalog
                .pages
                .iter()
                .any(|page| page.class == PageClass::ObjectType)
        );
    }

    #[test]
    fn real_shcntx_ru_extraction_returns_required_families_when_fixture_exists() {
        let path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk");
        if !path.exists() {
            eprintln!(
                "real-platform Syntax Assistant extraction smoke skipped because {} is unavailable",
                path.display()
            );
            return;
        }

        let book = HbkBook::open(path).expect("real Syntax Assistant book must open");
        let context = SyntaxHelperReader::new(&book)
            .extract()
            .expect("real Syntax Assistant extraction must succeed");

        assert!(!context.global_methods.is_empty());
        assert!(!context.global_properties.is_empty());
        assert!(!context.platform_types.is_empty());
        assert!(!context.type_methods.is_empty());
        assert!(!context.type_properties.is_empty());
        assert!(!context.constructors.is_empty());
        assert!(!context.enums.is_empty());
        assert!(!context.enum_values.is_empty());
    }

    #[test]
    fn real_shcntx_root_root_discovery_includes_required_root_candidates_when_fixture_exists() {
        let path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk");
        if !path.exists() {
            eprintln!(
                "real-platform root discovery smoke skipped because {} is unavailable",
                path.display()
            );
            return;
        }

        let book = HbkBook::open(path).expect("real root Syntax Assistant book must open");
        let discovery = SyntaxHelperReader::new(&book)
            .discover_roots()
            .expect("real root Syntax Assistant roots must be discoverable");

        assert!(discovery.has_kind(RootSectionKind::GlobalContext));
        assert!(discovery.has_kind(RootSectionKind::EnumCatalog));
        assert!(discovery.has_kind(RootSectionKind::TypeObjectCatalog));

        let enum_catalog = discovery
            .roots
            .iter()
            .find(|root| root.kind == RootSectionKind::EnumCatalog)
            .expect("root-source enum catalog root must be present");
        assert_eq!(enum_catalog.source.html_path, "objects/catalog2.html");
        assert!(
            enum_catalog
                .pages
                .iter()
                .any(|page| page.class == PageClass::Enum)
        );
    }

    fn fixture_toc() -> Toc {
        Toc::parse(
            r#"{
                14
                {1,0,2,2,3,{0,0,{0,0,{"ru","Глобальный контекст"}},"/objects/Global context.html"}}
                {2,1,1,4,{0,0,{0,0,{"ru","Свойства"}},"/objects/Global context/properties/catalog.html"}}
                {3,1,1,5,{0,0,{0,0,{"ru","Методы"}},"/objects/Global context/methods/catalog.html"}}
                {4,2,0,{0,0,{0,0,{"ru","Глобальный контекст.Справочники"}},"/objects/Global context/properties/Catalogs336.html"}}
                {5,3,0,{0,0,{0,0,{"ru","Глобальный контекст.XMLСтрока"}},"/objects/Global context/methods/catalog1566/XMLString1567.html"}}
                {6,0,1,7,{0,0,{0,0,{"ru","Системные перечисления"}},"/objects/catalog2.html"}}
                {7,6,1,8,{0,0,{0,0,{"ru","ТипЗначенияJSON"}},"/objects/catalog2/catalog2300/JSONValueType.html"}}
                {8,7,0,{0,0,{0,0,{"ru","ТипЗначенияJSON.КонецМассива"}},"/objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html"}}
                {9,0,1,10,{0,0,{0,0,{"ru","Универсальные коллекции значений"}},"/objects/catalog234.html"}}
                {10,9,3,11,12,13,{0,0,{0,0,{"ru","Массив"}},"/objects/catalog234/Array.html"}}
                {11,10,0,{0,0,{0,0,{"ru","Массив.Добавить"}},"/objects/catalog234/Array/methods/Add772.html"}}
                {12,10,0,{0,0,{0,0,{"ru","Массив.По количеству элементов"}},"/objects/catalog234/Array/ctors/ctor13.html"}}
                {13,10,0,{0,0,{0,0,{"ru","ГруппаФормы.Видимость"}},"/objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html"}}
                {14,0,0,{0,0,{0,0,{"ru","Неизвестный раздел"}},"/objects/unknown.html"}}
            }"#,
        )
        .expect("fixture TOC must parse")
    }

    fn fixture_content(toc: &Toc, html_path: &str) -> PageContent {
        let html = match html_path {
            "objects/Global context.html" => {
                include_str!("../../../tests/fixtures/syntax-helper/global_context_ru.html")
            }
            "objects/Global context/properties/Catalogs336.html" => {
                include_str!(
                    "../../../tests/fixtures/syntax-helper/global_property_catalogs_ru.html"
                )
            }
            "objects/Global context/methods/catalog1566/XMLString1567.html" => {
                include_str!(
                    "../../../tests/fixtures/syntax-helper/global_method_xmlstring_ru.html"
                )
            }
            "objects/catalog2.html" => {
                include_str!("../../../tests/fixtures/syntax-helper/root_catalog_enums_ru.html")
            }
            "objects/catalog2/catalog2300/JSONValueType.html" => {
                include_str!("../../../tests/fixtures/syntax-helper/enum_json_value_type_ru.html")
            }
            "objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html" => {
                include_str!(
                    "../../../tests/fixtures/syntax-helper/enum_value_json_array_end_ru.html"
                )
            }
            "objects/catalog234.html" => {
                include_str!("../../../tests/fixtures/syntax-helper/root_catalog_types_ru.html")
            }
            "objects/catalog234/Array.html" => {
                include_str!("../../../tests/fixtures/syntax-helper/object_array_ru.html")
            }
            "objects/catalog234/Array/methods/Add772.html" => {
                include_str!(
                    "../../../tests/fixtures/syntax-helper/object_method_array_add_ru.html"
                )
            }
            "objects/catalog234/Array/ctors/ctor13.html" => {
                include_str!(
                    "../../../tests/fixtures/syntax-helper/constructor_array_by_count_ru.html"
                )
            }
            "objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html" => {
                include_str!(
                    "../../../tests/fixtures/syntax-helper/object_property_formgroup_visible_ru.html"
                )
            }
            "objects/unknown.html" => {
                r#"<html><body><h1 class="V8SH_pagetitle">Неизвестный раздел</h1></body></html>"#
            }
            other => panic!("unexpected fixture page load: {other}"),
        };
        parse_page_html(
            Path::new("shcntx_ru.hbk"),
            "ru",
            toc,
            html_path,
            html,
            |_| false,
        )
    }

    fn source(html_path: &str) -> SyntaxHelperSource {
        SyntaxHelperSource {
            hbk_path: PathBuf::from("shcntx_ru.hbk"),
            locale: "ru".to_string(),
            toc_path: None,
            html_path: html_path.to_string(),
            page_title: String::new(),
        }
    }
}

#[cfg(test)]
mod syntax_helper_fixture_tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    const MANIFEST: &str = include_str!("../../../tests/fixtures/syntax-helper/manifest.tsv");

    #[derive(Debug)]
    struct ManifestEntry<'a> {
        parser_kind: &'a str,
        source_hbk: &'a str,
        page_title: &'a str,
        fixture_path: &'a str,
        reason: &'a str,
    }

    #[test]
    fn syntax_assistant_fixture_manifest_covers_required_parser_kinds() {
        let entries = parse_manifest();
        let actual_kinds = entries
            .iter()
            .map(|entry| entry.parser_kind)
            .collect::<BTreeSet<_>>();
        let required_kinds = BTreeSet::from([
            "global_context",
            "global_method",
            "global_property",
            "object_type",
            "object_method",
            "object_property",
            "constructor",
            "enum",
            "enum_value",
            "root_catalog",
        ]);

        assert_eq!(actual_kinds, required_kinds);
        assert!(
            entries
                .iter()
                .any(|entry| entry.source_hbk.ends_with("shcntx_ru.hbk"))
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.source_hbk.ends_with("shcntx_root.hbk"))
        );
        assert!(
            entries
                .iter()
                .filter(|entry| entry.parser_kind == "root_catalog")
                .count()
                >= 3
        );
        assert!(
            entries
                .iter()
                .filter(|entry| entry.parser_kind == "root_catalog")
                .all(|entry| entry.reason.contains("TOC records")),
            "root/catalog HTML fixtures must document that catalog children are represented by TOC records"
        );
    }

    #[test]
    fn syntax_assistant_fixture_manifest_points_to_real_html_fragments() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for entry in parse_manifest() {
            let path = workspace_root.join(entry.fixture_path);
            let html = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));
            assert!(
                html.contains("<html") || html.contains("<body") || html.contains("V8SH_"),
                "{} must look like a real Syntax Assistant HTML fragment",
                path.display()
            );
            assert!(
                !entry.page_title.trim().is_empty(),
                "{} must record a page title",
                entry.fixture_path
            );
            assert!(
                !entry.reason.trim().is_empty(),
                "{} must record a fixture reason",
                entry.fixture_path
            );
        }
    }

    fn parse_manifest() -> Vec<ManifestEntry<'static>> {
        MANIFEST
            .lines()
            .filter(|line| {
                !line.trim().is_empty()
                    && !line.starts_with('#')
                    && !line.starts_with("parser_kind")
            })
            .map(|line| {
                let fields = line.split('\t').collect::<Vec<_>>();
                assert_eq!(
                    fields.len(),
                    6,
                    "manifest row must have 6 tab-separated fields"
                );
                ManifestEntry {
                    parser_kind: fields[0],
                    source_hbk: fields[1],
                    page_title: fields[3],
                    fixture_path: fields[4],
                    reason: fields[5],
                }
            })
            .collect()
    }
}
