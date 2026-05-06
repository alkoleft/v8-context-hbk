use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use hbk_book::{BookError, HbkBook, normalize_storage_path};
use hbk_docs::{DocumentationError, DocumentationReader, PageContent};
use quick_html2md::{MarkdownOptions, html_to_markdown_with_options};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookExportFormat {
    Raw,
    Markdown,
}

impl fmt::Display for BookExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raw => f.write_str("raw"),
            Self::Markdown => f.write_str("markdown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookExportHierarchy {
    Raw,
    Toc,
}

impl fmt::Display for BookExportHierarchy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raw => f.write_str("raw"),
            Self::Toc => f.write_str("toc"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookExportRequest {
    source_path: PathBuf,
    output_root: PathBuf,
    format: BookExportFormat,
    hierarchy: BookExportHierarchy,
}

impl BookExportRequest {
    pub fn new(
        source_path: impl Into<PathBuf>,
        output_root: impl Into<PathBuf>,
        format: BookExportFormat,
        hierarchy: BookExportHierarchy,
    ) -> Result<Self, BookExportError> {
        let output_root = output_root.into();
        validate_output_root(&output_root)?;
        validate_combination(format, hierarchy)?;
        Ok(Self {
            source_path: source_path.into(),
            output_root,
            format,
            hierarchy,
        })
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn output_root(&self) -> &Path {
        &self.output_root
    }

    pub fn format(&self) -> BookExportFormat {
        self.format
    }

    pub fn hierarchy(&self) -> BookExportHierarchy {
        self.hierarchy
    }
}

#[derive(Debug)]
pub struct BookExporter<'a> {
    book: &'a HbkBook,
}

impl<'a> BookExporter<'a> {
    pub fn new(book: &'a HbkBook) -> Self {
        Self { book }
    }

    pub fn book(&self) -> &'a HbkBook {
        self.book
    }

    pub fn validate_request(request: &BookExportRequest) -> Result<(), BookExportError> {
        validate_output_root(request.output_root())?;
        validate_combination(request.format(), request.hierarchy())
    }

    pub fn export(&self, request: &BookExportRequest) -> Result<BookExportResult, BookExportError> {
        Self::validate_request(request)?;
        validate_source_path(request.source_path(), self.book.path())?;
        match (request.format(), request.hierarchy()) {
            (BookExportFormat::Raw, BookExportHierarchy::Raw) => self.export_raw_raw(request),
            (BookExportFormat::Markdown, BookExportHierarchy::Toc) => {
                Err(BookExportError::ExportNotImplemented {
                    format: request.format(),
                    hierarchy: request.hierarchy(),
                })
            }
            (format, hierarchy) => {
                Err(BookExportError::UnsupportedCombination { format, hierarchy })
            }
        }
    }

    pub fn markdown_page(&self, html_path: &str) -> Result<BookMarkdownPage, BookExportError> {
        let normalized_html_path = normalize_storage_path(html_path);
        let toc_page = self
            .book
            .toc()
            .find_by_html_path(normalized_html_path)
            .ok_or_else(|| BookExportError::TocPageNotFound {
                html_path: normalized_html_path.to_string(),
            })?;
        let page = DocumentationReader::new(self.book).load_page(&toc_page.html_path)?;
        Ok(BookMarkdownPage::from_page_content(page))
    }

    fn export_raw_raw(
        &self,
        request: &BookExportRequest,
    ) -> Result<BookExportResult, BookExportError> {
        let mut reader = self.book.file_storage_reader()?;
        let plans = plan_raw_exports(request.output_root(), reader.file_paths()?)?;
        create_directory(request.output_root())?;

        let mut exported_files = Vec::with_capacity(plans.len());
        for plan in plans {
            let bytes = reader.read_file(&plan.entry_name)?;
            if let Some(parent) = plan.output_path.parent() {
                create_directory(parent)?;
            }
            fs::write(&plan.output_path, &bytes).map_err(|source| BookExportError::Io {
                path: plan.output_path.clone(),
                operation: BookExportIoOperation::WriteFile,
                source,
            })?;
            exported_files.push(BookExportedFile::new(plan.output_path, bytes.len() as u64));
        }

        Ok(BookExportResult::new(
            request.output_root().to_path_buf(),
            exported_files,
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookMarkdownPage {
    html_path: String,
    title: String,
    markdown: String,
}

impl BookMarkdownPage {
    pub fn html_path(&self) -> &str {
        &self.html_path
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    fn from_page_content(page: PageContent) -> Self {
        let markdown = page_content_to_markdown(&page);
        Self {
            html_path: page.source.html_path,
            title: page.title,
            markdown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookExportResult {
    output_root: PathBuf,
    files: Vec<BookExportedFile>,
}

impl BookExportResult {
    pub fn new(output_root: impl Into<PathBuf>, files: Vec<BookExportedFile>) -> Self {
        Self {
            output_root: output_root.into(),
            files,
        }
    }

    pub fn output_root(&self) -> &Path {
        &self.output_root
    }

    pub fn files(&self) -> &[BookExportedFile] {
        &self.files
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookExportedFile {
    path: PathBuf,
    bytes_written: u64,
}

impl BookExportedFile {
    pub fn new(path: impl Into<PathBuf>, bytes_written: u64) -> Self {
        Self {
            path: path.into(),
            bytes_written,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}

#[derive(Debug)]
pub enum BookExportError {
    InvalidOutputRoot {
        output_root: PathBuf,
        reason: OutputRootError,
    },
    UnsupportedCombination {
        format: BookExportFormat,
        hierarchy: BookExportHierarchy,
    },
    ExportNotImplemented {
        format: BookExportFormat,
        hierarchy: BookExportHierarchy,
    },
    UnsafeStoragePath {
        entry_name: String,
        reason: StoragePathError,
    },
    DuplicateStoragePath {
        entry_name: String,
        normalized_path: PathBuf,
    },
    StoragePathCollision {
        entry_name: String,
        normalized_path: PathBuf,
        existing_path: PathBuf,
    },
    TocPageNotFound {
        html_path: String,
    },
    SourcePathMismatch {
        request_source_path: PathBuf,
        book_path: PathBuf,
    },
    Book(BookError),
    Documentation(DocumentationError),
    Io {
        path: PathBuf,
        operation: BookExportIoOperation,
        source: io::Error,
    },
}

impl PartialEq for BookExportError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::InvalidOutputRoot {
                    output_root,
                    reason,
                },
                Self::InvalidOutputRoot {
                    output_root: other_output_root,
                    reason: other_reason,
                },
            ) => output_root == other_output_root && reason == other_reason,
            (
                Self::UnsupportedCombination { format, hierarchy },
                Self::UnsupportedCombination {
                    format: other_format,
                    hierarchy: other_hierarchy,
                },
            )
            | (
                Self::ExportNotImplemented { format, hierarchy },
                Self::ExportNotImplemented {
                    format: other_format,
                    hierarchy: other_hierarchy,
                },
            ) => format == other_format && hierarchy == other_hierarchy,
            (
                Self::UnsafeStoragePath { entry_name, reason },
                Self::UnsafeStoragePath {
                    entry_name: other_entry_name,
                    reason: other_reason,
                },
            ) => entry_name == other_entry_name && reason == other_reason,
            (
                Self::DuplicateStoragePath {
                    entry_name,
                    normalized_path,
                },
                Self::DuplicateStoragePath {
                    entry_name: other_entry_name,
                    normalized_path: other_normalized_path,
                },
            ) => entry_name == other_entry_name && normalized_path == other_normalized_path,
            (
                Self::StoragePathCollision {
                    entry_name,
                    normalized_path,
                    existing_path,
                },
                Self::StoragePathCollision {
                    entry_name: other_entry_name,
                    normalized_path: other_normalized_path,
                    existing_path: other_existing_path,
                },
            ) => {
                entry_name == other_entry_name
                    && normalized_path == other_normalized_path
                    && existing_path == other_existing_path
            }
            (
                Self::TocPageNotFound { html_path },
                Self::TocPageNotFound {
                    html_path: other_html_path,
                },
            ) => html_path == other_html_path,
            (
                Self::SourcePathMismatch {
                    request_source_path,
                    book_path,
                },
                Self::SourcePathMismatch {
                    request_source_path: other_request_source_path,
                    book_path: other_book_path,
                },
            ) => request_source_path == other_request_source_path && book_path == other_book_path,
            (
                Self::Io {
                    path,
                    operation,
                    source,
                },
                Self::Io {
                    path: other_path,
                    operation: other_operation,
                    source: other_source,
                },
            ) => {
                path == other_path
                    && operation == other_operation
                    && source.kind() == other_source.kind()
            }
            (Self::Book(source), Self::Book(other_source)) => {
                source.to_string() == other_source.to_string()
            }
            (Self::Documentation(source), Self::Documentation(other_source)) => {
                source.to_string() == other_source.to_string()
            }
            _ => false,
        }
    }
}

impl Eq for BookExportError {}

impl From<BookError> for BookExportError {
    fn from(value: BookError) -> Self {
        Self::Book(value)
    }
}

impl From<DocumentationError> for BookExportError {
    fn from(value: DocumentationError) -> Self {
        Self::Documentation(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookExportIoOperation {
    CreateDirectory,
    WriteFile,
}

impl fmt::Display for BookExportIoOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateDirectory => f.write_str("create directory"),
            Self::WriteFile => f.write_str("write file"),
        }
    }
}

impl fmt::Display for BookExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOutputRoot {
                output_root,
                reason,
            } => write!(
                f,
                "invalid book export output root '{}': {reason}",
                output_root.display()
            ),
            Self::UnsupportedCombination { format, hierarchy } => write!(
                f,
                "unsupported book export combination: format={format}, hierarchy={hierarchy}"
            ),
            Self::ExportNotImplemented { format, hierarchy } => write!(
                f,
                "book export combination is not implemented yet: format={format}, hierarchy={hierarchy}"
            ),
            Self::UnsafeStoragePath { entry_name, reason } => write!(
                f,
                "unsafe FileStorage path '{entry_name}' cannot be exported: {reason}"
            ),
            Self::DuplicateStoragePath {
                entry_name,
                normalized_path,
            } => write!(
                f,
                "FileStorage path '{entry_name}' maps to duplicate export path '{}'",
                normalized_path.display()
            ),
            Self::StoragePathCollision {
                entry_name,
                normalized_path,
                existing_path,
            } => write!(
                f,
                "FileStorage path '{entry_name}' maps to '{}' which collides with '{}'",
                normalized_path.display(),
                existing_path.display()
            ),
            Self::TocPageNotFound { html_path } => {
                write!(
                    f,
                    "TOC page '{html_path}' is not present in the opened HBK book"
                )
            }
            Self::SourcePathMismatch {
                request_source_path,
                book_path,
            } => write!(
                f,
                "book export request source '{}' does not match opened book '{}'",
                request_source_path.display(),
                book_path.display()
            ),
            Self::Book(source) => write!(f, "failed to read HBK book for export: {source}"),
            Self::Documentation(source) => {
                write!(f, "failed to read documentation page for export: {source}")
            }
            Self::Io {
                path,
                operation,
                source,
            } => write!(f, "failed to {operation} '{}': {source}", path.display()),
        }
    }
}

impl std::error::Error for BookExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Book(source) => Some(source),
            Self::Documentation(source) => Some(source),
            Self::Io { source, .. } => Some(source),
            Self::InvalidOutputRoot { .. }
            | Self::UnsupportedCombination { .. }
            | Self::ExportNotImplemented { .. }
            | Self::UnsafeStoragePath { .. }
            | Self::DuplicateStoragePath { .. }
            | Self::StoragePathCollision { .. }
            | Self::TocPageNotFound { .. }
            | Self::SourcePathMismatch { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputRootError {
    MissingDirectoryName,
    ParentSegment,
}

impl fmt::Display for OutputRootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDirectoryName => {
                f.write_str("path must contain at least one directory name")
            }
            Self::ParentSegment => f.write_str("path must not contain '..' segments"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoragePathError {
    Empty,
    ParentSegment,
    Absolute,
    WindowsPrefix,
    BackslashSeparator,
}

impl fmt::Display for StoragePathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("path must contain at least one file name"),
            Self::ParentSegment => f.write_str("path must not contain '..' segments"),
            Self::Absolute => f.write_str("path must be relative"),
            Self::WindowsPrefix => f.write_str("path must not contain a Windows drive prefix"),
            Self::BackslashSeparator => f.write_str("path must use '/' separators"),
        }
    }
}

#[derive(Debug)]
struct RawExportPlan {
    entry_name: String,
    output_path: PathBuf,
}

fn plan_raw_exports(
    output_root: &Path,
    entry_names: Vec<String>,
) -> Result<Vec<RawExportPlan>, BookExportError> {
    let mut seen_paths = HashSet::new();
    let mut plans = Vec::with_capacity(entry_names.len());
    for entry_name in entry_names {
        let relative_path = storage_entry_relative_path(&entry_name)?;
        if !seen_paths.insert(relative_path.clone()) {
            return Err(BookExportError::DuplicateStoragePath {
                entry_name,
                normalized_path: relative_path,
            });
        }
        if let Some(existing_path) = seen_paths
            .iter()
            .find(|existing_path| paths_have_prefix_collision(&relative_path, existing_path))
        {
            return Err(BookExportError::StoragePathCollision {
                entry_name,
                normalized_path: relative_path,
                existing_path: existing_path.clone(),
            });
        }
        plans.push(RawExportPlan {
            output_path: output_root.join(relative_path),
            entry_name,
        });
    }
    Ok(plans)
}

fn storage_entry_relative_path(entry_name: &str) -> Result<PathBuf, BookExportError> {
    let reason = if Path::new(entry_name).is_absolute() {
        Some(StoragePathError::Absolute)
    } else if has_windows_drive_prefix(entry_name) {
        Some(StoragePathError::WindowsPrefix)
    } else if entry_name.contains('\\') {
        Some(StoragePathError::BackslashSeparator)
    } else {
        None
    };
    if let Some(reason) = reason {
        return Err(BookExportError::UnsafeStoragePath {
            entry_name: entry_name.to_string(),
            reason,
        });
    }

    let mut relative_path = PathBuf::new();
    for segment in normalize_storage_path(entry_name).split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                return Err(BookExportError::UnsafeStoragePath {
                    entry_name: entry_name.to_string(),
                    reason: StoragePathError::ParentSegment,
                });
            }
            value => relative_path.push(value),
        }
    }
    if relative_path.as_os_str().is_empty() {
        return Err(BookExportError::UnsafeStoragePath {
            entry_name: entry_name.to_string(),
            reason: StoragePathError::Empty,
        });
    }
    Ok(relative_path)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn paths_have_prefix_collision(left: &Path, right: &Path) -> bool {
    left != right && (left.starts_with(right) || right.starts_with(left))
}

fn create_directory(path: &Path) -> Result<(), BookExportError> {
    fs::create_dir_all(path).map_err(|source| BookExportError::Io {
        path: path.to_path_buf(),
        operation: BookExportIoOperation::CreateDirectory,
        source,
    })
}

fn validate_source_path(
    request_source_path: &Path,
    book_path: &Path,
) -> Result<(), BookExportError> {
    if request_source_path == book_path || canonical_paths_match(request_source_path, book_path) {
        Ok(())
    } else {
        Err(BookExportError::SourcePathMismatch {
            request_source_path: request_source_path.to_path_buf(),
            book_path: book_path.to_path_buf(),
        })
    }
}

fn canonical_paths_match(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn validate_output_root(output_root: &Path) -> Result<(), BookExportError> {
    let mut has_directory_name = false;
    for component in output_root.components() {
        match component {
            Component::Normal(_) => has_directory_name = true,
            Component::ParentDir => {
                return Err(BookExportError::InvalidOutputRoot {
                    output_root: output_root.to_path_buf(),
                    reason: OutputRootError::ParentSegment,
                });
            }
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    if has_directory_name {
        Ok(())
    } else {
        Err(BookExportError::InvalidOutputRoot {
            output_root: output_root.to_path_buf(),
            reason: OutputRootError::MissingDirectoryName,
        })
    }
}

fn validate_combination(
    format: BookExportFormat,
    hierarchy: BookExportHierarchy,
) -> Result<(), BookExportError> {
    match (format, hierarchy) {
        (BookExportFormat::Raw, BookExportHierarchy::Raw)
        | (BookExportFormat::Markdown, BookExportHierarchy::Toc) => Ok(()),
        (format, hierarchy) => Err(BookExportError::UnsupportedCombination { format, hierarchy }),
    }
}

fn page_content_to_markdown(page: &PageContent) -> String {
    let options = MarkdownOptions::new()
        .include_links(false)
        .include_images(false)
        .preserve_tables(true)
        .escape_special_chars(false);
    let markdown = html_to_markdown_with_options(&page.raw_html, &options);
    ensure_markdown_heading(&page.title, normalize_markdown(markdown))
}

fn normalize_markdown(markdown: String) -> String {
    let normalized = markdown.replace('\r', "").replace('\u{a0}', " ");
    let lines = normalized
        .trim_matches(['\u{feff}', '\n'])
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>();
    let mut output = lines.join("\n").trim().to_string();
    if !output.is_empty() {
        output.push('\n');
    }
    output
}

fn ensure_markdown_heading(title: &str, markdown: String) -> String {
    if title.trim().is_empty()
        || markdown
            .lines()
            .find(|line| !line.trim().is_empty())
            .is_some_and(|line| line.starts_with('#'))
    {
        return markdown;
    }

    let mut output = format!("# {}\n", title.trim());
    if !markdown.trim().is_empty() {
        output.push('\n');
        output.push_str(markdown.trim_end());
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbk_book::test_utils::{fixture_container, zip_bytes, zip_entries};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn accepts_supported_export_combinations() {
        let raw = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "target/book-export/raw",
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");
        assert_eq!(raw.source_path(), Path::new("fmtdui_ru.hbk"));
        assert_eq!(raw.output_root(), Path::new("target/book-export/raw"));
        assert_eq!(raw.format(), BookExportFormat::Raw);
        assert_eq!(raw.hierarchy(), BookExportHierarchy::Raw);

        let markdown = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "target/book-export/markdown",
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");
        assert_eq!(markdown.format(), BookExportFormat::Markdown);
        assert_eq!(markdown.hierarchy(), BookExportHierarchy::Toc);
    }

    #[test]
    fn rejects_unsupported_export_combinations() {
        let raw_toc = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "target/book-export/raw-toc",
            BookExportFormat::Raw,
            BookExportHierarchy::Toc,
        )
        .expect_err("raw/toc must stay unsupported until specified");
        assert_eq!(
            raw_toc,
            BookExportError::UnsupportedCombination {
                format: BookExportFormat::Raw,
                hierarchy: BookExportHierarchy::Toc,
            }
        );
        assert_eq!(
            raw_toc.to_string(),
            "unsupported book export combination: format=raw, hierarchy=toc"
        );

        let markdown_raw = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "target/book-export/markdown-raw",
            BookExportFormat::Markdown,
            BookExportHierarchy::Raw,
        )
        .expect_err("markdown/raw must stay unsupported until specified");
        assert_eq!(
            markdown_raw,
            BookExportError::UnsupportedCombination {
                format: BookExportFormat::Markdown,
                hierarchy: BookExportHierarchy::Raw,
            }
        );
    }

    #[test]
    fn rejects_unsafe_output_roots() {
        let empty = BookExportRequest::new(
            "fmtdui_ru.hbk",
            PathBuf::new(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect_err("empty output root must be rejected");
        assert_eq!(
            empty,
            BookExportError::InvalidOutputRoot {
                output_root: PathBuf::new(),
                reason: OutputRootError::MissingDirectoryName,
            }
        );

        let root_only = BookExportRequest::new(
            "fmtdui_ru.hbk",
            Path::new("/"),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect_err("root-only output path must be rejected");
        assert_eq!(
            root_only,
            BookExportError::InvalidOutputRoot {
                output_root: PathBuf::from("/"),
                reason: OutputRootError::MissingDirectoryName,
            }
        );

        let parent = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "target/../book-export",
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect_err("parent-directory output root must be rejected");
        assert_eq!(
            parent,
            BookExportError::InvalidOutputRoot {
                output_root: PathBuf::from("target/../book-export"),
                reason: OutputRootError::ParentSegment,
            }
        );
    }

    #[test]
    fn accepts_absolute_output_root_with_directory_name() {
        let request = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "/tmp/v8-context-hbk-book-export",
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("absolute output roots with a directory name are valid");

        assert_eq!(
            request.output_root(),
            Path::new("/tmp/v8-context-hbk-book-export")
        );
    }

    #[test]
    fn exposes_export_result_file_summary() {
        let result = BookExportResult::new(
            "target/book-export/raw",
            vec![BookExportedFile::new(
                "target/book-export/raw/docs/page.html",
                42,
            )],
        );

        assert_eq!(result.output_root(), Path::new("target/book-export/raw"));
        assert_eq!(result.files()[0].bytes_written(), 42);
        assert_eq!(
            result.files()[0].path(),
            Path::new("target/book-export/raw/docs/page.html")
        );
    }

    #[test]
    fn converts_toc_page_html_to_markdown_without_raw_scaffolding() {
        let workspace = TempWorkspace::new("markdown-page");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r#"{
            2
            {1,0,0,{0,0,{0,0,{"ru","Справка"}{"en","Help"}},"/docs/page.html"}}
            {2,0,0,{0,0,{0,0,{"ru","Связанная"}},"/docs/other.html"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                (
                    "docs/page.html",
                    r#"<html><head>
                        <link rel="stylesheet" href="v8help://service_book/service_style">
                    </head><body>
                        <h1>Справка&nbsp;по синтаксису</h1>
                        <p>Синтаксис: Функция &lt;Имя_функции&gt;</p>
                        <p><a href="other.html">Связанная страница</a></p>
                        <table><tr><th>Имя</th><th>Значение</th></tr><tr><td>ВЫБОР</td><td>CASE</td></tr></table>
                        <img src="assets/pic.png" alt="service image">
                    </body></html>"#
                        .as_bytes(),
                ),
                ("docs/other.html", b"<html><body>other</body></html>"),
            ],
        );
        let book = HbkBook::open(&source_path).expect("book must open");

        let page = BookExporter::new(&book)
            .markdown_page("/docs/page.html")
            .expect("TOC page must convert to Markdown");

        assert_eq!(page.html_path(), "docs/page.html");
        assert_eq!(page.title(), "Справка по синтаксису");
        let markdown = page.markdown();
        assert!(markdown.starts_with("# Справка по синтаксису\n"));
        assert!(markdown.contains("Функция <Имя_функции>"));
        assert!(markdown.contains("Связанная страница"));
        assert!(markdown.contains("ВЫБОР"));
        assert!(markdown.contains("CASE"));
        assert!(markdown.contains('|'));
        assert!(!markdown.contains("other.html"));
        assert!(!markdown.contains("assets/pic.png"));
        assert_no_raw_markdown_scaffolding(markdown);
    }

    #[test]
    fn rejects_markdown_conversion_for_non_toc_storage_page() {
        let workspace = TempWorkspace::new("markdown-non-toc");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Страница"}},"/docs/page.html"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                ("docs/page.html", b"<html><body>page</body></html>"),
                ("docs/unlisted.html", b"<html><body>unlisted</body></html>"),
            ],
        );
        let book = HbkBook::open(&source_path).expect("book must open");

        let error = BookExporter::new(&book)
            .markdown_page("docs/unlisted.html")
            .expect_err("non-TOC storage pages must not be converted as TOC pages");

        assert_eq!(
            error,
            BookExportError::TocPageNotFound {
                html_path: "docs/unlisted.html".to_string(),
            }
        );
    }

    #[test]
    fn real_representative_pages_convert_to_readable_markdown_when_platform_books_exist() {
        struct Case<'a> {
            book_path: &'a str,
            html_path: &'a str,
            expected: &'a [&'a str],
        }

        let cases = [
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/dcsui_ru.hbk",
                html_path: "PresentSKD",
                expected: &[
                    "# Двуязычное представление ключевых слов системы компоновки данных",
                    "ВЫБОР",
                    "CASE",
                    "|",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/dcsui_ru.hbk",
                html_path: "SKD_Functions_Strings",
                expected: &[
                    "# Работа со строками",
                    "ДлинаСтроки",
                    "StringLength",
                    "ДлинаСтроки(<Строка>)",
                    "Подстрока",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shlang_ru.hbk",
                html_path: "def_Func",
                expected: &[
                    "# Функция",
                    "Синтаксис",
                    "Функция <Имя_функции>",
                    "Возврат <Возвращаемое значение>",
                    "КонецФункции",
                    "Ждать",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shlang_ru.hbk",
                html_path: "struct_IfThenElif",
                expected: &[
                    "# Если",
                    "Если <Логическое выражение> Тогда",
                    "ИначеЕсли <Логическое выражение> Тогда",
                    "КонецЕсли",
                    "логического выражения",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shquery_ru.hbk",
                html_path: "syntax_diagram.html",
                expected: &[
                    "# Синтаксическая диаграмма конструкций языка запросов",
                    "<Конструкция языка>",
                    "ЭТО_КЛЮЧЕВОЕ_СЛОВО",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shquery_ru.hbk",
                html_path: "SUM",
                expected: &["# Агрегатная функция СУММА", "Агрегатные функции", "NULL"],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk",
                html_path: "form_formattedstringedit",
                expected: &[
                    "# Конструктор строк на разных языках",
                    "интерфейсных языков",
                    "Обычная строка",
                    "Форматированная строка",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/htmlui_ru.hbk",
                html_path: "form_addtable",
                expected: &[
                    "# Вставка таблицы",
                    "HTML-документы можно вставлять таблицы",
                    "Таблица - Вставить таблицу",
                    "Ячейки можно объединять и делить",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/moxelui_ru.hbk",
                html_path: "form_moxelpagesetupdialog",
                expected: &[
                    "# Параметры страницы табличного документа",
                    "Файл - Параметры страницы",
                    "Колонтитулы",
                    "Авто",
                ],
            },
        ];

        for case in cases {
            let book_path = Path::new(case.book_path);
            if !book_path.exists() {
                continue;
            }

            let book = HbkBook::open(book_path).expect("platform HBK must open");
            let page = BookExporter::new(&book)
                .markdown_page(case.html_path)
                .expect("real TOC page must convert to Markdown");
            let markdown = page.markdown();

            for expected in case.expected {
                assert!(
                    markdown.contains(expected),
                    "expected Markdown for {} {} to contain {expected:?}; got:\n{markdown}",
                    case.book_path,
                    case.html_path
                );
            }
            assert_no_raw_markdown_scaffolding(markdown);
        }
    }

    #[test]
    fn exports_raw_storage_files_under_normalized_paths() {
        let workspace = TempWorkspace::new("raw-success");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        write_book_fixture(
            &source_path,
            vec![
                ("docs/page.html", b"<html>page</html>".as_ref()),
                ("assets/./style.css", b"body {}".as_ref()),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");

        let result = BookExporter::new(&book)
            .export(&request)
            .expect("raw/raw export must succeed");

        assert_eq!(
            fs::read(output_root.join("docs/page.html")).expect("page must be exported"),
            b"<html>page</html>"
        );
        assert_eq!(
            fs::read(output_root.join("assets/style.css")).expect("asset must be exported"),
            b"body {}"
        );
        assert_eq!(result.output_root(), output_root.as_path());
        let exported: Vec<_> = result
            .files()
            .iter()
            .map(|file| {
                (
                    file.path()
                        .strip_prefix(&output_root)
                        .expect("exported file must be under output root")
                        .to_path_buf(),
                    file.bytes_written(),
                )
            })
            .collect();
        assert_eq!(
            exported,
            vec![
                (
                    PathBuf::from("docs/page.html"),
                    b"<html>page</html>".len() as u64,
                ),
                (PathBuf::from("assets/style.css"), b"body {}".len() as u64),
            ]
        );
    }

    #[test]
    fn rejects_unsafe_storage_paths_before_writing() {
        assert_rejects_unsafe_storage_path("../escape.txt", StoragePathError::ParentSegment);
        assert_rejects_unsafe_storage_path("/escape.txt", StoragePathError::Absolute);
        assert_rejects_unsafe_storage_path("C:/escape.txt", StoragePathError::WindowsPrefix);
        assert_rejects_unsafe_storage_path("dir\\escape.txt", StoragePathError::BackslashSeparator);
    }

    #[test]
    fn rejects_duplicate_normalized_storage_paths_before_writing() {
        let workspace = TempWorkspace::new("raw-duplicate");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        write_book_fixture(
            &source_path,
            vec![
                ("docs/./page.html", b"first".as_ref()),
                ("docs/page.html", b"second".as_ref()),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");

        let error = BookExporter::new(&book)
            .export(&request)
            .expect_err("duplicate normalized paths must be rejected");

        assert_eq!(
            error,
            BookExportError::DuplicateStoragePath {
                entry_name: "docs/page.html".to_string(),
                normalized_path: PathBuf::from("docs/page.html"),
            }
        );
        assert!(
            !output_root.exists(),
            "unsafe plan validation must finish before filesystem writes"
        );
    }

    #[test]
    fn rejects_file_directory_storage_path_collisions_before_writing() {
        let workspace = TempWorkspace::new("raw-collision");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        write_book_fixture(
            &source_path,
            vec![
                ("docs", b"file".as_ref()),
                ("docs/page.html", b"page".as_ref()),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");

        let error = BookExporter::new(&book)
            .export(&request)
            .expect_err("file/directory path collision must be rejected");

        assert_eq!(
            error,
            BookExportError::StoragePathCollision {
                entry_name: "docs/page.html".to_string(),
                normalized_path: PathBuf::from("docs/page.html"),
                existing_path: PathBuf::from("docs"),
            }
        );
        assert!(
            !output_root.exists(),
            "path collision validation must finish before filesystem writes"
        );
    }

    #[test]
    fn rejects_request_source_path_mismatch_before_writing() {
        let workspace = TempWorkspace::new("source-mismatch");
        let request_source_path = workspace.path().join("fmtdui_ru.hbk");
        let opened_book_path = workspace.path().join("htmlui_ru.hbk");
        write_book_fixture(
            &request_source_path,
            vec![("docs/request.html", b"request".as_ref())],
        );
        write_book_fixture(
            &opened_book_path,
            vec![("docs/opened.html", b"opened".as_ref())],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&opened_book_path).expect("book must open");
        let request = BookExportRequest::new(
            request_source_path.clone(),
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");

        let error = BookExporter::new(&book)
            .export(&request)
            .expect_err("source path mismatch must be rejected");

        assert_eq!(
            error,
            BookExportError::SourcePathMismatch {
                request_source_path,
                book_path: opened_book_path,
            }
        );
        assert!(
            !output_root.exists(),
            "source mismatch validation must finish before filesystem writes"
        );
    }

    fn assert_rejects_unsafe_storage_path(entry_name: &str, reason: StoragePathError) {
        let workspace = TempWorkspace::new("raw-unsafe");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        write_book_fixture(
            &source_path,
            vec![
                ("docs/page.html", b"ok".as_ref()),
                (entry_name, b"escape".as_ref()),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");

        let error = BookExporter::new(&book)
            .export(&request)
            .expect_err("unsafe storage path must be rejected");

        assert_eq!(
            error,
            BookExportError::UnsafeStoragePath {
                entry_name: entry_name.to_string(),
                reason,
            }
        );
        assert!(
            !output_root.exists(),
            "unsafe path validation must finish before filesystem writes"
        );
    }

    fn write_book_fixture(path: &Path, storage_entries: Vec<(&str, &[u8])>) {
        fs::write(
            path,
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
                ("FileStorage", Some(zip_entries(storage_entries))),
            ]),
        )
        .expect("fixture HBK must be written");
    }

    fn write_book_fixture_with_toc(path: &Path, toc: &str, storage_entries: Vec<(&str, &[u8])>) {
        fs::write(
            path,
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
                ("FileStorage", Some(zip_entries(storage_entries))),
            ]),
        )
        .expect("fixture HBK must be written");
    }

    fn assert_no_raw_markdown_scaffolding(markdown: &str) {
        for forbidden in [
            "<html",
            "<body",
            "<p",
            "<a ",
            "<h1",
            "<h2",
            "<table",
            "<tr",
            "<td",
            "<ul",
            "<li",
            "<div",
            "<span",
            "</",
            "&nbsp;",
            "v8help://service_book/service_style",
            "/opt/1cv8",
            ".hbk",
            ".html",
            "toc_index",
            "toc-index",
        ] {
            assert!(
                !markdown.contains(forbidden),
                "Markdown must not contain raw service/provenance fragment {forbidden:?}:\n{markdown}"
            );
        }
    }

    struct TempWorkspace {
        path: PathBuf,
    }

    impl TempWorkspace {
        fn new(name: &str) -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "v8-context-hbk-book-export-test-{name}-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("temp workspace must be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
