use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use super::toc::{Toc, TocError};
use super::tokens::{TokenError, TokenParser, tokenize};
use hbk_container::{ContainerError, HbkContainer};

const PACK_BLOCK_NAME: &str = "PackBlock";
const FILE_STORAGE_NAME: &str = "FileStorage";
const BOOK_NAME: &str = "Book";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookEntityKind {
    Book,
    PackBlock,
    FileStorage,
}

impl BookEntityKind {
    pub fn entity_name(self) -> &'static str {
        match self {
            Self::Book => BOOK_NAME,
            Self::PackBlock => PACK_BLOCK_NAME,
            Self::FileStorage => FILE_STORAGE_NAME,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookMeta {
    pub book_name: String,
    pub description: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookLocale {
    Ru,
    Root,
    Other(String),
}

impl BookLocale {
    pub fn infer_from_path(path: &Path) -> Self {
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if stem.ends_with("_ru") {
            Self::Ru
        } else if stem.ends_with("_root") {
            Self::Root
        } else if let Some((_, suffix)) = stem.rsplit_once('_') {
            Self::Other(suffix.to_string())
        } else {
            Self::Other(String::new())
        }
    }

    pub fn source_code(&self) -> &str {
        match self {
            Self::Ru => "ru",
            Self::Root => "root",
            Self::Other(value) => value,
        }
    }

    pub fn export_code(&self) -> &str {
        match self {
            Self::Root => "en",
            _ => self.source_code(),
        }
    }
}

#[derive(Debug)]
pub enum BookError {
    Container(ContainerError),
    InvalidUtf8 {
        path: PathBuf,
        entity_name: &'static str,
        source: std::string::FromUtf8Error,
    },
    InvalidZip {
        path: PathBuf,
        entity_name: &'static str,
        source: zip::result::ZipError,
    },
    Io {
        path: PathBuf,
        entry_name: String,
        source: io::Error,
    },
    MissingZipEntry {
        path: PathBuf,
        entry_name: String,
    },
    Meta(MetaError),
    Toc(TocError),
}

impl fmt::Display for BookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Container(source) => write!(f, "{source}"),
            Self::InvalidUtf8 {
                path,
                entity_name,
                source,
            } => write!(
                f,
                "HBK entity '{entity_name}' in '{}' is not UTF-8: {source}",
                path.display()
            ),
            Self::InvalidZip {
                path,
                entity_name,
                source,
            } => write!(
                f,
                "HBK entity '{entity_name}' in '{}' is not a readable ZIP stream: {source}",
                path.display()
            ),
            Self::Io {
                path,
                entry_name,
                source,
            } => write!(
                f,
                "failed to read ZIP entry '{entry_name}' from '{}': {source}",
                path.display()
            ),
            Self::MissingZipEntry { path, entry_name } => write!(
                f,
                "ZIP entry '{entry_name}' is not present in HBK FileStorage '{}'",
                path.display()
            ),
            Self::Meta(source) => write!(f, "{source}"),
            Self::Toc(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for BookError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Container(source) => Some(source),
            Self::InvalidUtf8 { source, .. } => Some(source),
            Self::InvalidZip { source, .. } => Some(source),
            Self::Io { source, .. } => Some(source),
            Self::Meta(source) => Some(source),
            Self::Toc(source) => Some(source),
            Self::MissingZipEntry { .. } => None,
        }
    }
}

impl From<ContainerError> for BookError {
    fn from(value: ContainerError) -> Self {
        Self::Container(value)
    }
}

impl From<MetaError> for BookError {
    fn from(value: MetaError) -> Self {
        Self::Meta(value)
    }
}

impl From<TocError> for BookError {
    fn from(value: TocError) -> Self {
        Self::Toc(value)
    }
}

#[derive(Debug)]
pub struct HbkBook {
    path: PathBuf,
    meta: BookMeta,
    locale: BookLocale,
    toc: Toc,
}

impl HbkBook {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, BookError> {
        Self::from_container(HbkContainer::open(path)?)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn meta(&self) -> &BookMeta {
        &self.meta
    }

    pub fn locale(&self) -> &BookLocale {
        &self.locale
    }

    pub fn toc(&self) -> &Toc {
        &self.toc
    }

    pub fn file_storage_reader(&self) -> Result<FileStorageReader, BookError> {
        FileStorageReader::new(self.path(), read_file_storage(self.path())?)
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, BookError> {
        let mut reader = self.file_storage_reader()?;
        reader.read_file(path)
    }

    pub fn read_page(&self, path: &str) -> Result<String, BookError> {
        let mut reader = self.file_storage_reader()?;
        reader.read_page(path)
    }

    pub fn read_pages<'p>(
        &self,
        paths: impl IntoIterator<Item = &'p str>,
    ) -> Result<BTreeMap<String, String>, BookError> {
        let mut pages = BTreeMap::new();
        let mut reader = self.file_storage_reader()?;
        for path in paths {
            let entry_name = normalize_storage_path(path).to_string();
            if pages.contains_key(&entry_name) {
                continue;
            }
            let page = reader.read_page(&entry_name)?;
            pages.insert(entry_name, page);
        }
        Ok(pages)
    }

    fn from_container(container: HbkContainer) -> Result<Self, BookError> {
        let path = container.path().to_path_buf();
        let meta_text = entity_utf8(&container, BookEntityKind::Book)?;
        let meta = parse_book_meta(&meta_text)?;
        let file_storage = container.read_entity(BookEntityKind::FileStorage.entity_name())?;
        let toc = match container.read_entity(BookEntityKind::PackBlock.entity_name()) {
            Ok(pack_block) => {
                let toc_bytes = read_first_zip_entry(&path, PACK_BLOCK_NAME, &pack_block)?;
                let toc_text =
                    String::from_utf8(toc_bytes).map_err(|source| BookError::InvalidUtf8 {
                        path: path.clone(),
                        entity_name: PACK_BLOCK_NAME,
                        source,
                    })?;
                Toc::parse(&toc_text)?
            }
            Err(ContainerError::EntityHasNoBody { entity_name, .. })
                if entity_name == PACK_BLOCK_NAME =>
            {
                Toc::from_storage_paths(list_storage_page_paths(&path, &file_storage)?)
            }
            Err(source) => return Err(BookError::Container(source)),
        };
        Ok(Self {
            locale: BookLocale::infer_from_path(&path),
            path,
            meta,
            toc,
        })
    }
}

#[derive(Debug)]
pub struct FileStorageReader {
    path: PathBuf,
    archive: ZipArchive<Cursor<Vec<u8>>>,
}

impl FileStorageReader {
    fn new(path: impl AsRef<Path>, bytes: Vec<u8>) -> Result<Self, BookError> {
        let path = path.as_ref().to_path_buf();
        let archive =
            ZipArchive::new(Cursor::new(bytes)).map_err(|source| BookError::InvalidZip {
                path: path.clone(),
                entity_name: FILE_STORAGE_NAME,
                source,
            })?;
        Ok(Self { path, archive })
    }

    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>, BookError> {
        let entry_name = normalize_storage_path(path).to_string();
        if entry_name.is_empty() {
            return Err(BookError::MissingZipEntry {
                path: self.path.clone(),
                entry_name,
            });
        }
        let mut entry = self
            .archive
            .by_name(&entry_name)
            .map_err(|source| match source {
                zip::result::ZipError::FileNotFound => BookError::MissingZipEntry {
                    path: self.path.clone(),
                    entry_name: entry_name.clone(),
                },
                source => BookError::InvalidZip {
                    path: self.path.clone(),
                    entity_name: FILE_STORAGE_NAME,
                    source,
                },
            })?;
        let mut bytes = Vec::with_capacity(zip_entry_capacity(entry.size()));
        entry
            .read_to_end(&mut bytes)
            .map_err(|source| BookError::Io {
                path: self.path.clone(),
                entry_name,
                source,
            })?;
        Ok(bytes)
    }

    pub fn read_page(&mut self, path: &str) -> Result<String, BookError> {
        let bytes = self.read_file(path)?;
        String::from_utf8(bytes).map_err(|source| BookError::InvalidUtf8 {
            path: self.path.clone(),
            entity_name: FILE_STORAGE_NAME,
            source,
        })
    }
}

fn read_file_storage(path: &Path) -> Result<Vec<u8>, BookError> {
    Ok(HbkContainer::open(path)?.read_entity(BookEntityKind::FileStorage.entity_name())?)
}

fn entity_utf8(container: &HbkContainer, entity_kind: BookEntityKind) -> Result<String, BookError> {
    let entity_name = entity_kind.entity_name();
    let bytes = container.read_entity(entity_name)?;
    String::from_utf8(bytes).map_err(|source| BookError::InvalidUtf8 {
        path: container.path().to_path_buf(),
        entity_name,
        source,
    })
}

fn read_first_zip_entry(
    path: &Path,
    entity_name: &'static str,
    bytes: &[u8],
) -> Result<Vec<u8>, BookError> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|source| BookError::InvalidZip {
            path: path.to_path_buf(),
            entity_name,
            source,
        })?;
    let mut entry = archive
        .by_index(0)
        .map_err(|source| BookError::InvalidZip {
            path: path.to_path_buf(),
            entity_name,
            source,
        })?;
    let mut output = Vec::with_capacity(zip_entry_capacity(entry.size()));
    entry
        .read_to_end(&mut output)
        .map_err(|source| BookError::Io {
            path: path.to_path_buf(),
            entry_name: entry.name().to_string(),
            source,
        })?;
    Ok(output)
}

fn zip_entry_capacity(size: u64) -> usize {
    usize::try_from(size).unwrap_or(0)
}

fn list_storage_page_paths(path: &Path, bytes: &[u8]) -> Result<Vec<String>, BookError> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|source| BookError::InvalidZip {
            path: path.to_path_buf(),
            entity_name: FILE_STORAGE_NAME,
            source,
        })?;
    let mut paths = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|source| BookError::InvalidZip {
                path: path.to_path_buf(),
                entity_name: FILE_STORAGE_NAME,
                source,
            })?;
        let name = entry.name();
        if !entry.is_dir() && !name.starts_with("__") {
            paths.push(name.trim_start_matches('/').to_string());
        }
    }
    Ok(paths)
}

pub fn normalize_storage_path(path: &str) -> &str {
    path.trim_start_matches('/')
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetaError {
    message: String,
}

impl MetaError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for MetaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid HBK Book metadata: {}", self.message)
    }
}

impl std::error::Error for MetaError {}

impl From<TokenError> for MetaError {
    fn from(value: TokenError) -> Self {
        Self::new(value.to_string())
    }
}

pub fn parse_book_meta(content: &str) -> Result<BookMeta, MetaError> {
    let tokens = tokenize(content);
    let mut parser = TokenParser::new(tokens);
    parser.expect("{", "PageDescription: expected '{'")?;
    parser.number("PageDescription: expected type")?;
    let book_name = parser.string("PageDescription: expected bookName")?;
    let description = parse_file_name(&mut parser)?;
    let tag_count = parser.number("PageDescription: expected tagCount")?;
    let mut tags = Vec::new();
    for index in 0..tag_count {
        tags.push(parser.string(format!("Tags: expected tag #{}", index + 1))?);
        parse_number_pair(&mut parser)?;
    }
    let trailing_zero = parser.number("PageDescription: expected trailing zero")?;
    if trailing_zero != 0 {
        return Err(MetaError::new(format!(
            "PageDescription: expected trailing zero, got {trailing_zero}"
        )));
    }
    parser.expect("}", "PageDescription: expected closing '}'")?;
    parser.expect_end("PageDescription")?;
    Ok(BookMeta {
        book_name,
        description,
        tags,
    })
}

fn parse_file_name(parser: &mut TokenParser) -> Result<String, MetaError> {
    parser.expect("{", "FileName: expected '{'")?;
    parser.number("FileName: expected first number")?;
    parser.number("FileName: expected second number")?;
    parser.expect("{", "FileName: expected name object '{'")?;
    parser.string("FileName: expected language")?;
    let file_name = parser.string("FileName: expected fileName")?;
    parser.expect("}", "FileName: expected name object closing '}'")?;
    parser.expect("}", "FileName: expected closing '}'")?;
    Ok(file_name)
}

fn parse_number_pair(parser: &mut TokenParser) -> Result<(), MetaError> {
    parser.expect("{", "NumberPair: expected '{'")?;
    parser.number("NumberPair: expected first number")?;
    parser.number("NumberPair: expected second number")?;
    parser.expect("}", "NumberPair: expected closing '}'")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{fixture_container, zip_bytes, zip_entries};
    use hbk_container::ContainerError;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn infers_source_and_export_locale() {
        assert_eq!(
            BookLocale::infer_from_path(Path::new("fmtdui_ru.hbk")).source_code(),
            "ru"
        );
        let root = BookLocale::infer_from_path(Path::new("fmtdui_root.hbk"));
        assert_eq!(root.source_code(), "root");
        assert_eq!(root.export_code(), "en");
    }

    #[test]
    fn exposes_book_entity_names() {
        assert_eq!(BookEntityKind::Book.entity_name(), "Book");
        assert_eq!(BookEntityKind::PackBlock.entity_name(), "PackBlock");
        assert_eq!(BookEntityKind::FileStorage.entity_name(), "FileStorage");
    }

    #[test]
    fn parses_book_metadata() {
        let meta = parse_book_meta(
            r#"{1,"Interface", {1,2,{"en","fmtdui"}}, 2, "tag1", {0,0}, "tag2", {0,0}, 0}"#,
        )
        .expect("metadata must parse");
        assert_eq!(meta.book_name, "Interface");
        assert_eq!(meta.description, "fmtdui");
        assert_eq!(meta.tags, vec!["tag1", "tag2"]);
    }

    #[test]
    fn normalizes_storage_paths() {
        assert_eq!(normalize_storage_path("/docs/page.html"), "docs/page.html");
        assert_eq!(normalize_storage_path("docs/page.html"), "docs/page.html");
    }

    #[test]
    fn opens_book_toc_and_page_from_container_entities() {
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/docs/page.html"}}
        }"#;
        let fixture = TempHbk::new(
            "fmtdui_ru.hbk",
            fixture_container(vec![
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
                    Some(zip_bytes("docs/page.html", b"<html>page</html>")),
                ),
            ]),
        )
        .expect("fixture must be written");
        let book = HbkBook::open(fixture.path()).expect("book must open");

        assert_eq!(book.locale().source_code(), "ru");
        assert_eq!(book.meta().book_name, "Interface");
        assert_eq!(
            book.toc()
                .find_by_html_path("/docs/page.html")
                .unwrap()
                .title
                .display(),
            "Страница"
        );
        assert_eq!(
            book.read_page("/docs/page.html").unwrap(),
            "<html>page</html>"
        );
    }

    #[test]
    fn opens_book_with_storage_toc_when_pack_block_has_no_body() {
        let fixture = TempHbk::new(
            "fmtdui_ru.hbk",
            fixture_container(vec![
                (
                    "Book",
                    Some(
                        r#"{1,"Interface", {1,2,{"ru","fmtdui"}}, 1, "tag", {0,0}, 0}"#
                            .as_bytes()
                            .to_vec(),
                    ),
                ),
                ("PackBlock", None),
                (
                    "FileStorage",
                    Some(zip_bytes("docs/page.html", b"<html>page</html>")),
                ),
            ]),
        )
        .expect("fixture must be written");
        let book =
            HbkBook::open(fixture.path()).expect("book without readable PackBlock must open");

        assert_eq!(book.toc().pages()[0].html_path, "docs/page.html");
        assert_eq!(
            book.toc()
                .find_by_html_path("/docs/page.html")
                .unwrap()
                .title
                .display(),
            "docs/page.html"
        );
        assert_eq!(
            book.read_page("/docs/page.html").unwrap(),
            "<html>page</html>"
        );
    }

    #[test]
    fn file_storage_reader_reads_multiple_pages_from_one_book() {
        let toc = r#"{
            2
            {1,0,0,{0,0,{0,0,{"ru","Первая"}},"/docs/first.html"}}
            {2,0,0,{0,0,{0,0,{"ru","Вторая"}},"/docs/second.html"}}
        }"#;
        let fixture = TempHbk::new(
            "fmtdui_ru.hbk",
            fixture_container(vec![
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
                        ("docs/first.html", b"<html>first</html>"),
                        ("docs/second.html", b"<html>second</html>"),
                    ])),
                ),
            ]),
        )
        .expect("fixture must be written");
        let book = HbkBook::open(fixture.path()).expect("book must open");

        let mut reader = book
            .file_storage_reader()
            .expect("FileStorage reader must open");

        assert_eq!(
            reader.read_page("/docs/first.html").unwrap(),
            "<html>first</html>"
        );
        assert_eq!(
            reader.read_page("docs/second.html").unwrap(),
            "<html>second</html>"
        );
    }

    #[test]
    fn read_pages_deduplicates_storage_paths() {
        let toc = r#"{
            2
            {1,0,0,{0,0,{0,0,{"ru","Первая"}},"/docs/first.html"}}
            {2,0,0,{0,0,{0,0,{"ru","Вторая"}},"/docs/second.html"}}
        }"#;
        let fixture = TempHbk::new(
            "fmtdui_ru.hbk",
            fixture_container(vec![
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
                        ("docs/first.html", b"<html>first</html>"),
                        ("docs/second.html", b"<html>second</html>"),
                    ])),
                ),
            ]),
        )
        .expect("fixture must be written");
        let book = HbkBook::open(fixture.path()).expect("book must open");

        let pages = book
            .read_pages(["/docs/first.html", "docs/second.html", "/docs/first.html"])
            .expect("pages must read");

        assert_eq!(pages.len(), 2);
        assert_eq!(pages["docs/first.html"], "<html>first</html>");
        assert_eq!(pages["docs/second.html"], "<html>second</html>");
    }

    #[test]
    fn page_access_requires_source_file_after_open() {
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Страница"}},"/docs/page.html"}}
        }"#;
        let fixture = TempHbk::new(
            "fmtdui_ru.hbk",
            fixture_container(vec![
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
                    Some(zip_bytes("docs/page.html", b"<html>page</html>")),
                ),
            ]),
        )
        .expect("fixture must be written");
        let path = fixture.path().to_path_buf();
        let book = HbkBook::open(&path).expect("book must open");
        fixture.remove_file().expect("fixture file must be removed");

        let error = book.read_page("/docs/page.html").unwrap_err();

        match error {
            BookError::Container(ContainerError::Io {
                path: error_path, ..
            }) => {
                assert_eq!(error_path, path);
            }
            other => panic!("expected source file IO error, got {other}"),
        }
    }

    struct TempHbk {
        path: PathBuf,
    }

    impl TempHbk {
        fn new(file_name: &str, bytes: Vec<u8>) -> io::Result<Self> {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!(
                "v8-context-hbk-book-test-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir)?;
            let path = dir.join(file_name);
            fs::write(&path, bytes)?;
            Ok(Self { path })
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn remove_file(&self) -> io::Result<()> {
            fs::remove_file(&self.path)
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
