use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

use thiserror::Error;
use zip::ZipArchive;

use super::path::normalize_storage_path_owned;
use super::toc::{Toc, TocError};
use super::tokens::{TokenError, TokenParser};
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

#[derive(Debug, Error)]
pub enum BookError {
    #[error("{0}")]
    Container(#[from] ContainerError),
    #[error("HBK entity '{entity_name}' in '{}' is not UTF-8: {source}", path.display())]
    InvalidUtf8 {
        path: PathBuf,
        entity_name: &'static str,
        #[source]
        source: std::string::FromUtf8Error,
    },
    #[error("HBK entity '{entity_name}' in '{}' is not a readable ZIP stream: {source}", path.display())]
    InvalidZip {
        path: PathBuf,
        entity_name: &'static str,
        #[source]
        source: zip::result::ZipError,
    },
    #[error("failed to read ZIP entry '{entry_name}' from '{}': {source}", path.display())]
    Io {
        path: PathBuf,
        entry_name: String,
        #[source]
        source: io::Error,
    },
    #[error("ZIP entry '{entry_name}' is not present in HBK FileStorage '{}'", path.display())]
    MissingZipEntry { path: PathBuf, entry_name: String },
    #[error("{0}")]
    Meta(#[from] MetaError),
    #[error("{0}")]
    Toc(#[from] TocError),
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

    fn from_container(container: HbkContainer) -> Result<Self, BookError> {
        let path = container.path().to_path_buf();
        let meta_text = entity_utf8(&container, BookEntityKind::Book)?;
        let meta = parse_book_meta(&meta_text)?;
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
                let file_storage =
                    container.read_entity(BookEntityKind::FileStorage.entity_name())?;
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
        let entry_name = normalize_storage_path_owned(path);
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
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|source| BookError::Io {
                path: self.path.clone(),
                entry_name,
                source,
            })?;
        Ok(bytes)
    }

    pub fn file_paths(&mut self) -> Result<Vec<String>, BookError> {
        let mut paths = Vec::new();
        for index in 0..self.archive.len() {
            let entry = self
                .archive
                .by_index(index)
                .map_err(|source| BookError::InvalidZip {
                    path: self.path.clone(),
                    entity_name: FILE_STORAGE_NAME,
                    source,
                })?;
            if !entry.is_dir() {
                paths.push(entry.name().to_string());
            }
        }
        Ok(paths)
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
    let mut output = Vec::new();
    entry
        .read_to_end(&mut output)
        .map_err(|source| BookError::Io {
            path: path.to_path_buf(),
            entry_name: entry.name().to_string(),
            source,
        })?;
    Ok(output)
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
            paths.push(normalize_storage_path_owned(name));
        }
    }
    Ok(paths)
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid HBK Book metadata: {message}")]
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

impl From<TokenError> for MetaError {
    fn from(value: TokenError) -> Self {
        Self::new(value.to_string())
    }
}

pub fn parse_book_meta(content: &str) -> Result<BookMeta, MetaError> {
    let mut parser = TokenParser::new(content);
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
    fn parses_book_metadata_with_legacy_tokenizer_edges() {
        let meta = parse_book_meta(
            "\u{feff}{+1,\"Interface \u{feff}\"\"Core\"\"\", {+1,+2,{\u{feff}\"en\",\"fmtdui\"}}, +1, \"tag, one\", {+0,+0}, +0,}",
        )
        .expect("metadata must parse");

        assert_eq!(meta.book_name, "Interface \"Core\"");
        assert_eq!(meta.description, "fmtdui");
        assert_eq!(meta.tags, vec!["tag, one"]);
    }

    #[test]
    fn rejects_book_metadata_with_non_zero_trailing_marker() {
        let error =
            parse_book_meta(r#"{1,"Interface", {1,2,{"en","fmtdui"}}, 1, "tag", {0,0}, 1}"#)
                .expect_err("non-zero trailing marker must be rejected");

        assert_eq!(
            error.to_string(),
            "invalid HBK Book metadata: PageDescription: expected trailing zero, got 1"
        );
    }

    #[test]
    fn rejects_book_metadata_with_misordered_fields() {
        let error = parse_book_meta(r#"{1,{1,2,{"en","fmtdui"}},"Interface",0,0}"#)
            .expect_err("misordered metadata fields must be rejected");

        assert_eq!(
            error.to_string(),
            "invalid HBK Book metadata: PageDescription: expected bookName: expected string, got '{'"
        );
    }

    #[test]
    fn rejects_book_metadata_with_missing_required_field() {
        let error = parse_book_meta(r#"{1,"Interface", {1,2,{"en","fmtdui"}}, 0}"#)
            .expect_err("missing trailing marker must be rejected");

        assert_eq!(
            error.to_string(),
            "invalid HBK Book metadata: PageDescription: expected trailing zero: expected number, got '}': invalid digit found in string"
        );
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
    fn file_storage_reader_lists_stored_file_paths() {
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
                    Some(zip_entries(vec![
                        ("docs/first.html", b"<html>first</html>"),
                        ("assets/style.css", b"body {}"),
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
            reader.file_paths().expect("paths must be listed"),
            vec!["docs/first.html", "assets/style.css"]
        );
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

    #[test]
    fn zip_entry_metadata_size_does_not_drive_page_read() {
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Страница"}},"/docs/page.html"}}
        }"#;
        let file_storage = zip_with_reported_uncompressed_size(
            zip_bytes("docs/page.html", b"<html>page</html>"),
            u32::MAX,
        );
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
                ("FileStorage", Some(file_storage)),
            ]),
        )
        .expect("fixture must be written");
        let book = HbkBook::open(fixture.path()).expect("book must open");

        let page = book
            .read_page("/docs/page.html")
            .expect("page read must use actual ZIP data, not reported entry size");

        assert_eq!(page, "<html>page</html>");
    }

    #[test]
    fn zip_entry_metadata_size_does_not_drive_pack_block_read() {
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Страница"}},"/docs/page.html"}}
        }"#;
        let pack_block =
            zip_with_reported_uncompressed_size(zip_bytes("toc.txt", toc.as_bytes()), u32::MAX);
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
                ("PackBlock", Some(pack_block)),
                (
                    "FileStorage",
                    Some(zip_bytes("docs/page.html", b"<html>page</html>")),
                ),
            ]),
        )
        .expect("fixture must be written");

        let book = HbkBook::open(fixture.path())
            .expect("TOC read must use actual ZIP data, not reported entry size");

        assert_eq!(
            book.toc()
                .find_by_html_path("/docs/page.html")
                .unwrap()
                .title
                .display(),
            "Страница"
        );
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

    fn zip_with_reported_uncompressed_size(mut bytes: Vec<u8>, reported_size: u32) -> Vec<u8> {
        const LOCAL_FILE_HEADER_SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];
        const CENTRAL_DIRECTORY_SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];

        patch_zip_u32_after_signature(&mut bytes, LOCAL_FILE_HEADER_SIGNATURE, 22, reported_size);
        patch_zip_u32_after_signature(&mut bytes, CENTRAL_DIRECTORY_SIGNATURE, 24, reported_size);
        bytes
    }

    fn patch_zip_u32_after_signature(
        bytes: &mut [u8],
        signature: [u8; 4],
        offset: usize,
        value: u32,
    ) {
        let position = bytes
            .windows(signature.len())
            .position(|window| window == signature)
            .expect("ZIP signature must be present");
        bytes[position + offset..position + offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}
