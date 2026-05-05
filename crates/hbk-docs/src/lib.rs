use std::fmt;
use std::path::{Path, PathBuf};

use scraper::node::Node;
use scraper::{ElementRef, Html, Selector};

use hbk_book::{
    BookError, FileStorageReader, HbkBook, Toc, normalize_storage_path_owned,
    normalize_storage_path_segments,
};

#[derive(Debug)]
pub struct DocumentationReader<'a> {
    book: &'a HbkBook,
}

impl<'a> DocumentationReader<'a> {
    pub fn new(book: &'a HbkBook) -> Self {
        Self { book }
    }

    pub fn load_page(&self, html_path: &str) -> Result<PageContent, DocumentationError> {
        self.page_loader()?.load_page(html_path)
    }

    pub fn page_loader(&self) -> Result<DocumentationPageLoader<'a>, DocumentationError> {
        let storage =
            self.book
                .file_storage_reader()
                .map_err(|source| DocumentationError::PageRead {
                    path: self.book.path().to_path_buf(),
                    html_path: String::new(),
                    source,
                })?;
        Ok(DocumentationPageLoader {
            book: self.book,
            storage,
        })
    }
}

#[derive(Debug)]
pub struct DocumentationPageLoader<'a> {
    book: &'a HbkBook,
    storage: FileStorageReader,
}

impl DocumentationPageLoader<'_> {
    pub fn load_page(&mut self, html_path: &str) -> Result<PageContent, DocumentationError> {
        let raw_html =
            self.storage
                .read_page(html_path)
                .map_err(|source| DocumentationError::PageRead {
                    path: self.book.path().to_path_buf(),
                    html_path: normalize_storage_path_owned(html_path),
                    source,
                })?;
        Ok(parse_page_html(
            self.book.path(),
            self.book.locale().source_code(),
            self.book.toc(),
            html_path,
            &raw_html,
            |path| self.storage.read_file(path).is_ok(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageContent {
    pub source: PageSource,
    pub title: String,
    pub raw_html: String,
    pub body_text: String,
    pub text_preview: String,
    pub links: Vec<ResolvedLink>,
    pub diagnostics: Vec<LinkDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSource {
    pub hbk_path: PathBuf,
    pub locale: String,
    pub toc_path: Option<String>,
    pub html_path: String,
    pub toc_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLink {
    pub raw_href: String,
    pub normalized_path: Option<String>,
    pub title: Option<String>,
    pub status: LinkStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStatus {
    Resolved,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub hbk_path: PathBuf,
    pub locale: String,
    pub html_path: String,
    pub page_title: String,
    pub raw_href: String,
    pub normalized_path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Warning,
}

#[derive(Debug)]
pub enum DocumentationError {
    PageRead {
        path: PathBuf,
        html_path: String,
        source: BookError,
    },
}

impl fmt::Display for DocumentationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PageRead {
                path,
                html_path,
                source,
            } => write!(
                f,
                "failed to read documentation page '{html_path}' from '{}': {source}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for DocumentationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::PageRead { source, .. } => Some(source),
        }
    }
}

pub fn parse_page_html(
    hbk_path: &Path,
    locale: &str,
    toc: &Toc,
    html_path: &str,
    raw_html: &str,
    mut storage_contains: impl FnMut(&str) -> bool,
) -> PageContent {
    let normalized_page_path = normalize_storage_path_owned(html_path);
    let toc_page = toc
        .flat_pages()
        .find(|flat_page| flat_page.page.html_path == normalized_page_path);
    let toc_path = toc_page
        .as_ref()
        .map(|flat_page| flat_page.index_path.to_string());
    let toc_title = toc_page
        .as_ref()
        .map(|flat_page| flat_page.page.title.display().to_string());
    let document = Html::parse_document(raw_html);
    let title = select_first_text(&document, "title")
        .or_else(|| select_first_text(&document, "h1"))
        .or_else(|| toc_title.clone())
        .unwrap_or_default();
    let body_text = normalized_body_text(&document);
    let text_preview = text_preview(&body_text);
    let (links, diagnostics) = extract_links(
        &document,
        toc,
        hbk_path,
        locale,
        &normalized_page_path,
        &title,
        &mut storage_contains,
    );

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
        links,
        diagnostics,
    }
}

fn select_first_text(document: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).expect("static selector must be valid");
    document.select(&selector).find_map(|element| {
        let text = normalize_whitespace(element.text());
        (!text.is_empty()).then_some(text)
    })
}

fn normalized_body_text(document: &Html) -> String {
    let selector = Selector::parse("body").expect("static selector must be valid");
    document
        .select(&selector)
        .next()
        .map(normalize_element_text)
        .unwrap_or_else(|| normalize_element_text(document.root_element()))
}

fn text_preview(body_text: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 240;
    body_text.chars().take(MAX_PREVIEW_CHARS).collect()
}

fn normalize_whitespace<'a>(parts: impl Iterator<Item = &'a str>) -> String {
    let mut collector = TextCollector::default();
    for part in parts {
        collector.push_text(part);
    }
    collector.finish()
}

fn normalize_element_text(element: ElementRef<'_>) -> String {
    let mut collector = TextCollector::default();
    collect_element_text(element, &mut collector);
    collector.finish()
}

fn collect_element_text(element: ElementRef<'_>, collector: &mut TextCollector) {
    for child in element.children() {
        match child.value() {
            Node::Text(text) => collector.push_text(text),
            Node::Element(element) => {
                let tag_name = element.name();
                if tag_name == "br" || is_block_text_element(tag_name) {
                    collector.ensure_separator();
                }
                if let Some(child_element) = ElementRef::wrap(child) {
                    collect_element_text(child_element, collector);
                }
                if tag_name == "br" || is_block_text_element(tag_name) {
                    collector.ensure_separator();
                }
            }
            _ => {}
        }
    }
}

fn is_block_text_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "figcaption"
            | "figure"
            | "footer"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "td"
            | "th"
            | "tr"
            | "ul"
    )
}

#[derive(Debug, Default)]
struct TextCollector {
    output: String,
    pending_separator: bool,
}

impl TextCollector {
    fn push_text(&mut self, text: &str) {
        for ch in text.chars() {
            if ch.is_whitespace() {
                self.ensure_separator();
            } else {
                self.flush_separator();
                self.output.push(ch);
            }
        }
    }

    fn ensure_separator(&mut self) {
        if !self.output.is_empty() {
            self.pending_separator = true;
        }
    }

    fn flush_separator(&mut self) {
        if self.pending_separator && !self.output.ends_with(' ') {
            self.output.push(' ');
        }
        self.pending_separator = false;
    }

    fn finish(self) -> String {
        self.output
    }
}

fn extract_links(
    document: &Html,
    toc: &Toc,
    hbk_path: &Path,
    locale: &str,
    current_html_path: &str,
    page_title: &str,
    mut storage_contains: impl FnMut(&str) -> bool,
) -> (Vec<ResolvedLink>, Vec<LinkDiagnostic>) {
    let selector = Selector::parse("a[href]").expect("static selector must be valid");
    let mut links = Vec::new();
    let mut diagnostics = Vec::new();

    for element in document.select(&selector) {
        let raw_href = element.value().attr("href").unwrap_or_default().to_string();
        let normalized_path = normalize_link_target(current_html_path, &raw_href);
        let resolved_page = normalized_path
            .as_deref()
            .and_then(|path| toc.find_by_html_path(path));

        if let Some(page) = resolved_page {
            links.push(ResolvedLink {
                raw_href,
                normalized_path,
                title: Some(page.title.display().to_string()),
                status: LinkStatus::Resolved,
            });
        } else if normalized_path
            .as_deref()
            .is_some_and(&mut storage_contains)
        {
            links.push(ResolvedLink {
                raw_href,
                normalized_path,
                title: None,
                status: LinkStatus::Resolved,
            });
        } else {
            let message = match normalized_path.as_deref() {
                Some(path) => format!("link target '{path}' is not present in the TOC"),
                None => "link cannot be normalized to an internal HBK page".to_string(),
            };
            diagnostics.push(LinkDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "UNRESOLVED_LINK",
                hbk_path: hbk_path.to_path_buf(),
                locale: locale.to_string(),
                html_path: current_html_path.to_string(),
                page_title: page_title.to_string(),
                raw_href: raw_href.clone(),
                normalized_path: normalized_path.clone(),
                message,
            });
            links.push(ResolvedLink {
                raw_href,
                normalized_path,
                title: None,
                status: LinkStatus::Unresolved,
            });
        }
    }

    (links, diagnostics)
}

fn normalize_link_target(current_html_path: &str, href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }
    if href.starts_with('#') {
        return Some(current_html_path.to_string());
    }
    if is_unsupported_scheme(href) {
        return None;
    }

    let v8help_target = href.strip_prefix("v8help://");
    let without_scheme = v8help_target.unwrap_or(href);
    let path_part = without_scheme
        .split(['#', '?'])
        .next()
        .unwrap_or_default()
        .trim();
    if path_part.is_empty() {
        return Some(current_html_path.to_string());
    }

    let candidate = if v8help_target.is_some() || path_part.starts_with('/') {
        path_part.to_string()
    } else {
        let base = current_html_path.rsplit_once('/').map(|(base, _)| base);
        match base {
            Some(base) if !base.is_empty() => format!("{base}/{path_part}"),
            _ => path_part.to_string(),
        }
    };
    normalize_storage_path_segments(&candidate)
}

fn is_unsupported_scheme(href: &str) -> bool {
    href.contains(':') && !href.starts_with("v8help://")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbk_book::HbkBook;
    use hbk_book::Toc;
    use hbk_book::test_utils::{write_fixture_hbk, zip_bytes, zip_entries};
    use std::fs;
    use std::io;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn extracts_title_text_preview_and_provenance() {
        let toc = Toc::parse(
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/docs/page.html"}}
            }"#,
        )
        .expect("toc must parse");

        let content = parse_page_html(
            Path::new("fmtdui_ru.hbk"),
            "ru",
            &toc,
            "/docs/page.html",
            r#"<html><head><title>HTML title</title></head>
            <body><h1>Body title</h1><p> First  text
            with spacing. </p></body></html>"#,
            |_| false,
        );

        assert_eq!(content.source.hbk_path, PathBuf::from("fmtdui_ru.hbk"));
        assert_eq!(content.source.locale, "ru");
        assert_eq!(content.source.html_path, "docs/page.html");
        assert_eq!(content.source.toc_path.as_deref(), Some("0"));
        assert_eq!(content.source.toc_title.as_deref(), Some("Страница"));
        assert_eq!(content.title, "HTML title");
        assert_eq!(content.text_preview, "Body title First text with spacing.");
    }

    #[test]
    fn normalized_text_separates_block_siblings_without_breaking_inline_quotes() {
        let toc = Toc::parse(
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/docs/page.html"}}
            }"#,
        )
        .expect("toc must parse");

        let content = parse_page_html(
            Path::new("fmtdui_ru.hbk"),
            "ru",
            &toc,
            "/docs/page.html",
            r#"<html><body><p>Alpha</p><p>Beta "<strong>Gamma</strong>"</p></body></html>"#,
            |_| false,
        );

        assert_eq!(content.body_text, "Alpha Beta \"Gamma\"");
    }

    #[test]
    fn normalized_text_keeps_adjacent_inline_nodes_joined() {
        let toc = Toc::parse(
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/docs/page.html"}}
            }"#,
        )
        .expect("toc must parse");

        let content = parse_page_html(
            Path::new("fmtdui_ru.hbk"),
            "ru",
            &toc,
            "/docs/page.html",
            r#"<html><body><p>foo<strong>bar</strong></p></body></html>"#,
            |_| false,
        );

        assert_eq!(content.body_text, "foobar");
    }

    #[test]
    fn normalizes_and_resolves_internal_links() {
        let toc = Toc::parse(
            r#"{
                3
                {1,0,0,{0,0,{0,0,{"ru","Current"}},"/docs/current.html"}}
                {2,0,0,{0,0,{0,0,{"ru","Relative"}},"/docs/next.html"}}
                {3,0,0,{0,0,{0,0,{"ru","Parent"}},"/parent.html"}}
            }"#,
        )
        .expect("toc must parse");

        let content = parse_page_html(
            Path::new("fmtdui_ru.hbk"),
            "ru",
            &toc,
            "docs/current.html",
            r##"<html><body>
                <a href="next.html#section">next</a>
                <a href="../parent.html">parent</a>
                <a href="#local">local</a>
                <a href="v8help://docs/next.html?query">v8</a>
            </body></html>"##,
            |_| false,
        );

        let paths = content
            .links
            .iter()
            .map(|link| link.normalized_path.as_deref())
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![
                Some("docs/next.html"),
                Some("parent.html"),
                Some("docs/current.html"),
                Some("docs/next.html")
            ]
        );
        assert!(
            content
                .links
                .iter()
                .all(|link| link.status == LinkStatus::Resolved)
        );
        assert!(content.diagnostics.is_empty());
    }

    #[test]
    fn reports_unresolved_links_without_dropping_them() {
        let toc = Toc::parse(
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Current"}},"/docs/current.html"}}
            }"#,
        )
        .expect("toc must parse");

        let content = parse_page_html(
            Path::new("fmtdui_ru.hbk"),
            "ru",
            &toc,
            "docs/current.html",
            r#"<html><body>
                <a href="missing.html">missing</a>
                <a href="https://example.invalid/page">external</a>
            </body></html>"#,
            |_| false,
        );

        assert_eq!(content.links.len(), 2);
        assert_eq!(
            content.links[0].normalized_path.as_deref(),
            Some("docs/missing.html")
        );
        assert_eq!(content.links[0].status, LinkStatus::Unresolved);
        assert_eq!(content.links[1].normalized_path, None);
        assert_eq!(content.links[1].status, LinkStatus::Unresolved);
        assert_eq!(content.diagnostics.len(), 2);
        assert!(content.diagnostics.iter().all(|diagnostic| {
            diagnostic.severity == DiagnosticSeverity::Warning
                && diagnostic.code == "UNRESOLVED_LINK"
                && diagnostic.hbk_path == Path::new("fmtdui_ru.hbk")
                && diagnostic.locale == "ru"
                && diagnostic.html_path == "docs/current.html"
        }));
    }

    #[test]
    fn fixture_pages_have_stable_text_and_link_snapshots() {
        let toc = Toc::parse(
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Форматированная строка"}{"en","Formatted string"}},"/form_formattedstringedit"}}
            }"#,
        )
        .expect("toc must parse");

        for fixture in [
            (
                "fmtdui_ru.hbk",
                "ru",
                "Конструктор строк на разных языках",
                include_str!(
                    "../../../tests/fixtures/docs/fmtdui_ru_form_formattedstringedit.html"
                ),
                include_str!(
                    "../../../tests/fixtures/docs/fmtdui_ru_form_formattedstringedit.text"
                ),
            ),
            (
                "fmtdui_root.hbk",
                "root",
                "Constructor of strings in different languages",
                include_str!(
                    "../../../tests/fixtures/docs/fmtdui_root_form_formattedstringedit.html"
                ),
                include_str!(
                    "../../../tests/fixtures/docs/fmtdui_root_form_formattedstringedit.text"
                ),
            ),
        ] {
            let (hbk_path, locale, title, html, expected_text) = fixture;
            let content = parse_page_html(
                Path::new(hbk_path),
                locale,
                &toc,
                "form_formattedstringedit",
                html,
                |_| false,
            );

            assert_eq!(content.raw_html, html);
            assert_eq!(content.title, title);
            assert_eq!(content.body_text, expected_text.trim());
            assert_eq!(
                content.text_preview,
                expected_text.chars().take(240).collect::<String>()
            );
            assert!(content.links.is_empty());
            assert!(content.diagnostics.is_empty());
        }
    }

    #[test]
    fn fixture_links_have_stable_resolution_snapshot() {
        let toc = Toc::parse(
            r#"{
                2
                {1,0,0,{0,0,{0,0,{"ru","Current"}},"/docs/current.html"}}
                {2,0,0,{0,0,{0,0,{"ru","Next"}},"/docs/next.html"}}
            }"#,
        )
        .expect("toc must parse");
        let html = include_str!("../../../tests/fixtures/docs/fmtdui_link_handling.html");
        let expected = include_str!("../../../tests/fixtures/docs/fmtdui_link_handling.links")
            .lines()
            .collect::<Vec<_>>();

        let content = parse_page_html(
            Path::new("fmtdui_ru.hbk"),
            "ru",
            &toc,
            "docs/current.html",
            html,
            |path| path == "shared/topic.html",
        );

        let actual = content
            .links
            .iter()
            .map(|link| {
                format!(
                    "{} -> {} {}",
                    link.raw_href,
                    link.normalized_path.as_deref().unwrap_or("<none>"),
                    match link.status {
                        LinkStatus::Resolved => "resolved",
                        LinkStatus::Unresolved => "unresolved",
                    }
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(actual, expected);
        assert_eq!(content.diagnostics.len(), 1);
        assert_eq!(
            content.diagnostics[0].normalized_path.as_deref(),
            Some("docs/missing.html")
        );
    }

    #[test]
    fn load_page_resolves_file_storage_link_outside_toc() {
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Current"}},"/docs/current.html"}}
        }"#;
        let fixture = TempHbk::new(
            "fmtdui_ru.hbk",
            vec![
                (
                    "Book",
                    Some(
                        r#"{1,"Interface", {1,2,{"ru","fmtdui"}}, 1, "tag", {0,0}, 0}"#
                            .as_bytes()
                            .to_vec(),
                    ),
                ),
                ("PackBlock", Some(zip_bytes("toc.txt", toc.as_bytes()))),
                (
                    "FileStorage",
                    Some(zip_entries(vec![
                        (
                            "docs/current.html",
                            br#"<html><head><title>Current</title></head>
                            <body><a href="../shared/topic.html">shared</a></body></html>"#,
                        ),
                        ("shared/topic.html", b"<html><body>shared</body></html>"),
                    ])),
                ),
            ],
        )
        .expect("fixture must be written");
        let book = HbkBook::open(fixture.path()).expect("book must open");

        let page = DocumentationReader::new(&book)
            .load_page("/docs/current.html")
            .expect("page must load");

        assert_eq!(page.links.len(), 1);
        assert_eq!(
            page.links[0].normalized_path.as_deref(),
            Some("shared/topic.html")
        );
        assert_eq!(page.links[0].status, LinkStatus::Resolved);
        assert_eq!(page.links[0].title, None);
        assert!(page.diagnostics.is_empty());
    }

    #[test]
    fn real_fmtdui_page_loads_when_platform_fixture_exists() {
        let cases = [
            (
                Path::new("/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk"),
                include_str!("../../../tests/fixtures/known-pages/fmtdui_ru.page").trim(),
            ),
            (
                Path::new("/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk"),
                include_str!("../../../tests/fixtures/known-pages/fmtdui_root.page").trim(),
            ),
        ];

        for (path, page_path) in cases {
            if !path.exists() {
                continue;
            }

            let book = HbkBook::open(path).expect("platform HBK must open");
            let page = DocumentationReader::new(&book)
                .load_page(page_path)
                .expect("known platform page must load");

            assert_eq!(page.source.html_path, page_path);
            assert!(!page.raw_html.is_empty());
            assert!(!page.body_text.is_empty());
            assert!(!page.title.is_empty());
        }
    }

    struct TempHbk {
        path: PathBuf,
    }

    impl TempHbk {
        fn new(file_name: &str, entities: Vec<(&str, Option<Vec<u8>>)>) -> io::Result<Self> {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!(
                "v8-context-hbk-docs-test-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir)?;
            let path = dir.join(file_name);
            write_fixture_hbk(&path, entities)?;
            Ok(Self { path })
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempHbk {
        fn drop(&mut self) {
            if let Some(dir) = self.path.parent() {
                let _ = fs::remove_dir_all(dir);
            }
        }
    }
}
