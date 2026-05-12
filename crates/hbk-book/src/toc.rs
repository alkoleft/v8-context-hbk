use std::fmt;

use crate::normalize_storage_path_owned;

use super::tokens::{TokenError, TokenParser};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizedTitle {
    pub en: String,
    pub ru: String,
}

impl LocalizedTitle {
    pub fn display(&self) -> &str {
        if !self.ru.is_empty() {
            &self.ru
        } else {
            &self.en
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocPath(Vec<usize>);

impl TocPath {
    pub fn root(index: usize) -> Self {
        Self(vec![index])
    }

    pub fn child(&self, index: usize) -> Self {
        let mut values = self.0.clone();
        values.push(index);
        Self(values)
    }

    pub fn indexes(&self) -> &[usize] {
        &self.0
    }
}

impl fmt::Display for TocPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, value) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(".")?;
            }
            write!(f, "{value}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocPage {
    pub id: usize,
    pub parent_id: usize,
    pub title: LocalizedTitle,
    pub html_path: String,
    pub children: Vec<TocPage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toc {
    pages: Vec<TocPage>,
}

impl Toc {
    pub fn empty() -> Self {
        Self { pages: Vec::new() }
    }

    pub(crate) fn from_storage_paths(paths: Vec<String>) -> Self {
        let pages = paths
            .into_iter()
            .enumerate()
            .map(|(index, path)| TocPage {
                id: index + 1,
                parent_id: 0,
                title: LocalizedTitle {
                    en: path.clone(),
                    ru: String::new(),
                },
                html_path: path,
                children: Vec::new(),
            })
            .collect();
        Self { pages }
    }

    pub fn parse(content: &str) -> Result<Self, TocError> {
        let chunks = parse_chunks(content)?;
        let pages = build_tree(0, &chunks);
        Ok(Self { pages })
    }

    pub fn pages(&self) -> &[TocPage] {
        &self.pages
    }

    pub fn find_by_html_path(&self, html_path: &str) -> Option<&TocPage> {
        let normalized = normalize_storage_path_owned(html_path);
        self.flat_pages()
            .find(|page| page.page.html_path == normalized)
            .map(|page| page.page)
    }

    pub fn find_by_index_path(&self, path: &TocPath) -> Option<&TocPage> {
        let mut pages = self.pages.as_slice();
        let mut found = None;
        for index in path.indexes() {
            let page = pages.get(*index)?;
            found = Some(page);
            pages = &page.children;
        }
        found
    }

    pub fn flat_pages(&self) -> impl Iterator<Item = FlatTocPage<'_>> {
        let mut output = Vec::new();
        for (index, page) in self.pages.iter().enumerate() {
            flatten_page(page, TocPath::root(index), &mut output);
        }
        output.into_iter()
    }
}

#[derive(Debug, Clone)]
pub struct FlatTocPage<'a> {
    pub index_path: TocPath,
    pub page: &'a TocPage,
}

fn flatten_page<'a>(page: &'a TocPage, index_path: TocPath, output: &mut Vec<FlatTocPage<'a>>) {
    output.push(FlatTocPage {
        index_path: index_path.clone(),
        page,
    });
    for (index, child) in page.children.iter().enumerate() {
        flatten_page(child, index_path.child(index), output);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocError {
    message: String,
}

impl TocError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid HBK TOC: {}", self.message)
    }
}

impl std::error::Error for TocError {}

impl From<TokenError> for TocError {
    fn from(value: TokenError) -> Self {
        Self::new(value.to_string())
    }
}

#[derive(Debug)]
struct TocChunk {
    id: usize,
    parent_id: usize,
    title: LocalizedTitle,
    html_path: String,
}

fn parse_chunks(content: &str) -> Result<Vec<TocChunk>, TocError> {
    let mut parser = TokenParser::new(content);
    parser.expect("{", "TableOfContent: expected '{'")?;
    parser.number("TableOfContent: expected chunk count")?;
    let mut chunks = Vec::new();
    while parser.has_more() && !parser.next_is('}') {
        chunks.push(parse_chunk(&mut parser)?);
    }
    parser.expect("}", "TableOfContent: expected closing '}'")?;
    parser.expect_end("TableOfContent")?;
    Ok(chunks)
}

fn parse_chunk(parser: &mut TokenParser) -> Result<TocChunk, TocError> {
    parser.expect("{", "Chunk: expected '{'")?;
    let id = parser.number("Chunk: expected id")?;
    let parent_id = parser.number("Chunk: expected parent id")?;
    let child_count = parser.number("Chunk: expected child count")?;
    for index in 0..child_count {
        parser.number(format!("Chunk: expected child id #{}", index + 1))?;
    }
    parser.expect("{", "Properties: expected '{'")?;
    parser.number("Properties: expected first number")?;
    parser.number("Properties: expected second number")?;
    let title = parse_title(parser)?;
    let raw_html_path = parser.string("Properties: expected HTML path")?;
    let html_path = normalize_storage_path_owned(&raw_html_path);
    parser.expect("}", "Properties: expected closing '}'")?;
    parser.expect("}", "Chunk: expected closing '}'")?;
    Ok(TocChunk {
        id,
        parent_id,
        title,
        html_path,
    })
}

fn parse_title(parser: &mut TokenParser) -> Result<LocalizedTitle, TocError> {
    parser.expect("{", "NameContainer: expected '{'")?;
    parser.number("NameContainer: expected first number")?;
    parser.number("NameContainer: expected second number")?;
    let mut names = Vec::new();
    while parser.has_more() && !parser.next_is('}') {
        parser.expect("{", "NameObject: expected '{'")?;
        let language = parser.string("NameObject: expected language")?;
        let title = parser.string("NameObject: expected title")?;
        parser.expect("}", "NameObject: expected closing '}'")?;
        names.push((language, title));
    }
    parser.expect("}", "NameContainer: expected closing '}'")?;
    let mut title = LocalizedTitle {
        en: String::new(),
        ru: String::new(),
    };
    for (language, value) in names {
        match language.as_str() {
            "en" | "#" => title.en = value,
            "ru" => title.ru = value,
            _ => {}
        }
    }
    Ok(title)
}

fn build_tree(parent_id: usize, chunks: &[TocChunk]) -> Vec<TocPage> {
    chunks
        .iter()
        .filter(|chunk| chunk.parent_id == parent_id)
        .map(|chunk| TocPage {
            id: chunk.id,
            parent_id: chunk.parent_id,
            title: chunk.title.clone(),
            html_path: chunk.html_path.clone(),
            children: build_tree(chunk.id, chunks),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_toc_tree_and_lookups() {
        let toc = Toc::parse(
            r#"{
                2
                {1,0,1,2,{0,0,{0,0,{"ru","Корень"}{"en","Root"}},"/root.html"}}
                {2,1,0,{0,0,{0,0,{"ru","Дочерняя"}{"en","Child"}},"/child.html"}}
            }"#,
        )
        .expect("toc must parse");

        assert_eq!(toc.pages().len(), 1);
        assert_eq!(toc.pages()[0].children.len(), 1);
        assert_eq!(
            toc.find_by_html_path("/child.html")
                .unwrap()
                .title
                .display(),
            "Дочерняя"
        );
        assert_eq!(
            toc.find_by_index_path(&TocPath(vec![0, 0]))
                .unwrap()
                .html_path,
            "child.html"
        );
    }

    #[test]
    fn assigns_localized_titles_by_language_code() {
        let toc = Toc::parse(
            r#"{
                2
                {1,0,0,{0,0,{0,0,{"en","Root"}{"ru","Корень"}},"/root.html"}}
                {2,0,0,{0,0,{0,0,{"ru","Только русский"}},"/ru.html"}}
            }"#,
        )
        .expect("toc must parse");

        assert_eq!(toc.pages()[0].title.en, "Root");
        assert_eq!(toc.pages()[0].title.ru, "Корень");
        assert_eq!(toc.pages()[1].title.en, "");
        assert_eq!(toc.pages()[1].title.ru, "Только русский");
    }

    #[test]
    fn parses_toc_with_legacy_tokenizer_edges() {
        let toc = Toc::parse(
            "\u{feff}{1,{1,0,0,{0,0,{0,0,{\"ru\",\"Корень \"\"А\"\"\"}},\"/root.html\"}},}",
        )
        .expect("toc must parse");

        assert_eq!(toc.pages()[0].title.ru, "Корень \"А\"");
        assert_eq!(toc.pages()[0].html_path, "root.html");
    }
}
