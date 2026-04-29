use std::collections::{BTreeMap, BTreeSet};
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
    container: HbkContainer,
    meta: BookMeta,
    locale: BookLocale,
    toc: Toc,
    file_storage: Vec<u8>,
}

impl HbkBook {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, BookError> {
        Self::from_container(HbkContainer::open(path)?)
    }

    pub fn path(&self) -> &Path {
        self.container.path()
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

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, BookError> {
        let entry_name = normalize_storage_path(path).to_string();
        if entry_name.is_empty() {
            return Err(BookError::MissingZipEntry {
                path: self.path().to_path_buf(),
                entry_name,
            });
        }
        let mut archive =
            ZipArchive::new(Cursor::new(self.file_storage.as_slice())).map_err(|source| {
                BookError::InvalidZip {
                    path: self.path().to_path_buf(),
                    entity_name: FILE_STORAGE_NAME,
                    source,
                }
            })?;
        let mut entry = archive
            .by_name(&entry_name)
            .map_err(|source| match source {
                zip::result::ZipError::FileNotFound => BookError::MissingZipEntry {
                    path: self.path().to_path_buf(),
                    entry_name: entry_name.clone(),
                },
                source => BookError::InvalidZip {
                    path: self.path().to_path_buf(),
                    entity_name: FILE_STORAGE_NAME,
                    source,
                },
            })?;
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|source| BookError::Io {
                path: self.path().to_path_buf(),
                entry_name,
                source,
            })?;
        Ok(bytes)
    }

    pub fn read_page(&self, path: &str) -> Result<String, BookError> {
        let bytes = self.read_file(path)?;
        String::from_utf8(bytes).map_err(|source| BookError::InvalidUtf8 {
            path: self.path().to_path_buf(),
            entity_name: FILE_STORAGE_NAME,
            source,
        })
    }

    pub fn read_pages<'p>(
        &self,
        paths: impl IntoIterator<Item = &'p str>,
    ) -> Result<BTreeMap<String, String>, BookError> {
        let mut requested = BTreeSet::new();
        for path in paths {
            let entry_name = normalize_storage_path(path).to_string();
            if entry_name.is_empty() {
                return Err(BookError::MissingZipEntry {
                    path: self.path().to_path_buf(),
                    entry_name,
                });
            }
            requested.insert(entry_name);
        }
        let mut archive =
            ZipArchive::new(Cursor::new(self.file_storage.as_slice())).map_err(|source| {
                BookError::InvalidZip {
                    path: self.path().to_path_buf(),
                    entity_name: FILE_STORAGE_NAME,
                    source,
                }
            })?;
        let mut pages = BTreeMap::new();
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|source| BookError::InvalidZip {
                    path: self.path().to_path_buf(),
                    entity_name: FILE_STORAGE_NAME,
                    source,
                })?;
            let entry_name = entry.name().to_string();
            if !requested.contains(&entry_name) {
                continue;
            }
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|source| BookError::Io {
                    path: self.path().to_path_buf(),
                    entry_name: entry_name.clone(),
                    source,
                })?;
            let page = String::from_utf8(bytes).map_err(|source| BookError::InvalidUtf8 {
                path: self.path().to_path_buf(),
                entity_name: FILE_STORAGE_NAME,
                source,
            })?;
            pages.insert(entry_name, page);
            if pages.len() == requested.len() {
                break;
            }
        }
        if let Some(entry_name) = requested
            .iter()
            .find(|entry_name| !pages.contains_key(*entry_name))
        {
            return Err(BookError::MissingZipEntry {
                path: self.path().to_path_buf(),
                entry_name: entry_name.clone(),
            });
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
            locale: BookLocale::infer_from_path(container.path()),
            container,
            meta,
            toc,
            file_storage,
        })
    }

    #[cfg(test)]
    fn from_bytes(path: PathBuf, bytes: Vec<u8>) -> Result<Self, BookError> {
        Self::from_container(HbkContainer::from_bytes(path, bytes)?)
    }
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
    use std::io::Cursor;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

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
        let book = HbkBook::from_bytes(
            PathBuf::from("fmtdui_ru.hbk"),
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
        .expect("book must open");

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
        let book = HbkBook::from_bytes(
            PathBuf::from("fmtdui_ru.hbk"),
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
        .expect("book without readable PackBlock must open");

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

    fn zip_bytes(name: &str, body: &[u8]) -> Vec<u8> {
        zip_entries(vec![(name, body)])
    }

    fn zip_entries(entries: Vec<(&str, &[u8])>) -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut output);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            for (name, body) in entries {
                zip.start_file(name, options).unwrap();
                zip.write_all(body).unwrap();
            }
            zip.finish().unwrap();
        }
        output.into_inner()
    }

    fn fixture_container(entities: Vec<(&str, Option<Vec<u8>>)>) -> Vec<u8> {
        const BLOCK_HEADER_SIZE: usize = 31;
        const FILE_DESCRIPTOR_SIZE: usize = 12;
        const SPLITTER: u32 = i32::MAX as u32;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&SPLITTER.to_le_bytes());
        bytes.extend_from_slice(&512_u32.to_le_bytes());
        bytes.extend_from_slice(&(entities.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());

        let descriptor_payload_size = entities.len() * FILE_DESCRIPTOR_SIZE;
        let descriptor_block_offset = bytes.len();
        push_block(
            &mut bytes,
            descriptor_payload_size,
            &vec![0; descriptor_payload_size],
            None,
        );

        let mut descriptors = Vec::new();
        for (name, body) in entities {
            let header_offset = bytes.len() as u32;
            push_block(
                &mut bytes,
                entity_header_payload(name).len(),
                &entity_header_payload(name),
                None,
            );

            let body_offset = if let Some(body) = body {
                let body_offset = bytes.len() as u32;
                push_block(&mut bytes, body.len(), &body, None);
                body_offset
            } else {
                SPLITTER
            };

            descriptors.extend_from_slice(&header_offset.to_le_bytes());
            descriptors.extend_from_slice(&body_offset.to_le_bytes());
            descriptors.extend_from_slice(&SPLITTER.to_le_bytes());
        }

        let descriptor_body_offset = descriptor_block_offset + BLOCK_HEADER_SIZE;
        bytes[descriptor_body_offset..descriptor_body_offset + descriptors.len()]
            .copy_from_slice(&descriptors);
        bytes
    }

    fn entity_header_payload(name: &str) -> Vec<u8> {
        let mut payload = vec![0; 20];
        for code_unit in name.encode_utf16() {
            payload.extend_from_slice(&code_unit.to_le_bytes());
        }
        payload.extend_from_slice(&[0; 4]);
        payload
    }

    fn push_block(bytes: &mut Vec<u8>, payload_size: usize, chunk: &[u8], next: Option<usize>) {
        const SPLITTER: u32 = i32::MAX as u32;
        bytes.extend_from_slice(
            format!(
                "\r\n{payload_size:08x} {block_size:08x} {next:08x} \r\n",
                block_size = chunk.len(),
                next = next.unwrap_or(SPLITTER as usize)
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(chunk);
    }
}
