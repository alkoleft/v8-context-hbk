use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use hbk_book::{
    BookError, HbkBook, Toc, TocPage, normalize_storage_path, normalize_storage_path_segments,
};
use hbk_docs::{DocumentationError, DocumentationPageLoader, DocumentationReader, PageContent};
use quick_html2md::{MarkdownOptions, html_to_markdown_with_options};
use scraper::node::Node;
use scraper::{ElementRef, Html, Selector};

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
                self.export_markdown_toc(request)
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

    pub fn markdown_toc_page(
        &self,
        html_path: &str,
        title: &str,
    ) -> Result<BookMarkdownPage, BookExportError> {
        let normalized_html_path = normalize_storage_path(html_path);
        let markdown = if is_heading_only_toc_path(normalized_html_path) {
            heading_only_markdown(title)
        } else {
            match DocumentationReader::new(self.book).load_page(normalized_html_path) {
                Ok(page) => page_content_to_markdown(&page),
                Err(error) if documentation_error_is_missing_page(&error) => {
                    heading_only_markdown(title)
                }
                Err(error) => return Err(error.into()),
            }
        };
        Ok(BookMarkdownPage {
            html_path: normalized_html_path.to_string(),
            title: title.to_string(),
            markdown,
        })
    }

    pub fn linked_markdown_toc_page(
        &self,
        html_path: &str,
        title: &str,
        current_output_path: &Path,
        link_targets: &HashMap<String, PathBuf>,
        source_book_ids: &HashSet<String>,
    ) -> Result<BookMarkdownPage, BookExportError> {
        let normalized_html_path = normalize_storage_path(html_path);
        let markdown = if is_heading_only_toc_path(normalized_html_path) {
            heading_only_markdown(title)
        } else {
            match DocumentationReader::new(self.book).load_page(normalized_html_path) {
                Ok(page) => page_content_to_linked_markdown(
                    &page,
                    current_output_path,
                    link_targets,
                    source_book_ids,
                ),
                Err(error) if documentation_error_is_missing_page(&error) => {
                    heading_only_markdown(title)
                }
                Err(error) => return Err(error.into()),
            }
        };
        Ok(BookMarkdownPage {
            html_path: normalized_html_path.to_string(),
            title: title.to_string(),
            markdown,
        })
    }

    pub fn markdown_page_loader(&self) -> Result<BookMarkdownPageLoader<'a>, BookExportError> {
        let loader = DocumentationReader::new(self.book).page_loader()?;
        Ok(BookMarkdownPageLoader { loader })
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

    fn export_markdown_toc(
        &self,
        request: &BookExportRequest,
    ) -> Result<BookExportResult, BookExportError> {
        let plans = plan_markdown_toc_exports(request.output_root(), self.book.toc());
        let link_targets = markdown_link_targets(&plans);
        let source_book_ids = source_book_link_ids(self.book);
        create_directory(request.output_root())?;

        let mut loader = DocumentationReader::new(self.book).page_loader()?;
        let mut exported_files = Vec::with_capacity(plans.len());
        for plan in plans {
            let markdown = if is_heading_only_toc_path(&plan.html_path) {
                heading_only_markdown(&plan.title)
            } else {
                match loader.load_page(&plan.html_path) {
                    Ok(page) => page_content_to_linked_markdown(
                        &page,
                        &plan.relative_path,
                        &link_targets,
                        &source_book_ids,
                    ),
                    Err(error) if documentation_error_is_missing_page(&error) => {
                        heading_only_markdown(&plan.title)
                    }
                    Err(error) => return Err(error.into()),
                }
            };
            if let Some(parent) = plan.output_path.parent() {
                create_directory(parent)?;
            }
            fs::write(&plan.output_path, markdown.as_bytes()).map_err(|source| {
                BookExportError::Io {
                    path: plan.output_path.clone(),
                    operation: BookExportIoOperation::WriteFile,
                    source,
                }
            })?;
            exported_files.push(BookExportedFile::new(
                plan.output_path,
                markdown.len() as u64,
            ));
        }

        Ok(BookExportResult::new(
            request.output_root().to_path_buf(),
            exported_files,
        ))
    }
}

pub trait MarkdownLinkTargets {
    fn markdown_link_target(
        &self,
        normalized_path: &str,
        source_book_ids: &HashSet<String>,
    ) -> Option<&PathBuf>;
}

impl MarkdownLinkTargets for HashMap<String, PathBuf> {
    fn markdown_link_target(
        &self,
        normalized_path: &str,
        source_book_ids: &HashSet<String>,
    ) -> Option<&PathBuf> {
        self.get(normalized_path).or_else(|| {
            normalized_path
                .split_once('/')
                .filter(|(book_segment, _)| source_book_ids.contains(*book_segment))
                .and_then(|(_, path_without_book_segment)| self.get(path_without_book_segment))
        })
    }
}

#[derive(Debug)]
pub struct BookMarkdownPageLoader<'a> {
    loader: DocumentationPageLoader<'a>,
}

impl BookMarkdownPageLoader<'_> {
    pub fn linked_markdown_toc_page(
        &mut self,
        html_path: &str,
        title: &str,
        current_output_path: &Path,
        link_targets: &impl MarkdownLinkTargets,
        source_book_ids: &HashSet<String>,
    ) -> Result<BookMarkdownPage, BookExportError> {
        let normalized_html_path = normalize_storage_path(html_path);
        let markdown = if is_heading_only_toc_path(normalized_html_path) {
            heading_only_markdown(title)
        } else {
            match self.loader.load_raw_page(normalized_html_path) {
                Ok(raw_html) => raw_page_to_linked_markdown(
                    &raw_html,
                    normalized_html_path,
                    title,
                    current_output_path,
                    link_targets,
                    source_book_ids,
                ),
                Err(error) if documentation_error_is_missing_page(&error) => {
                    heading_only_markdown(title)
                }
                Err(error) => return Err(error.into()),
            }
        };
        Ok(BookMarkdownPage {
            html_path: normalized_html_path.to_string(),
            title: title.to_string(),
            markdown,
        })
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

#[derive(Debug, Clone)]
struct MarkdownTocExportPlan {
    html_path: String,
    title: String,
    relative_path: PathBuf,
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

fn plan_markdown_toc_exports(output_root: &Path, toc: &Toc) -> Vec<MarkdownTocExportPlan> {
    let mut plans = Vec::new();
    append_markdown_toc_pages(output_root, toc.pages(), &[], &mut plans);
    plans
}

fn append_markdown_toc_pages(
    output_root: &Path,
    pages: &[TocPage],
    parent_segments: &[String],
    plans: &mut Vec<MarkdownTocExportPlan>,
) {
    let mut used_segments = HashSet::new();
    for page in pages {
        let segment =
            unique_toc_segment(title_path_segment(page.title.display()), &mut used_segments);
        let mut segments = parent_segments.to_vec();
        segments.push(segment);
        let relative_path = markdown_page_relative_path(&segments);
        plans.push(MarkdownTocExportPlan {
            html_path: page.html_path.clone(),
            title: page.title.display().to_string(),
            output_path: output_root.join(&relative_path),
            relative_path,
        });
        append_markdown_toc_pages(output_root, &page.children, &segments, plans);
    }
}

fn markdown_page_relative_path(segments: &[String]) -> PathBuf {
    let mut path = PathBuf::new();
    for segment in segments {
        path.push(segment);
    }
    path.push("index.md");
    path
}

fn unique_toc_segment(base: String, used_segments: &mut HashSet<String>) -> String {
    if used_segments.insert(base.clone()) {
        return base;
    }
    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if used_segments.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded counter must find a unique segment")
}

fn title_path_segment(title: &str) -> String {
    let mut output = String::new();
    let mut pending_separator = false;
    for character in title.trim().chars() {
        if character.is_alphanumeric() {
            if pending_separator && !output.is_empty() {
                output.push('-');
            }
            output.extend(character.to_lowercase());
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }
    if output.is_empty() {
        "page".to_string()
    } else {
        output
    }
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
    let anchor_targets = markdown_heading_anchor_targets(&page.raw_html);
    let html = normalize_code_examples(&page.raw_html);
    let markdown = html_to_markdown_with_options(&html, &options);
    let markdown = ensure_markdown_heading(&page.title, normalize_markdown(markdown));
    materialize_markdown_heading_anchors(&markdown, &anchor_targets)
}

fn page_content_to_linked_markdown(
    page: &PageContent,
    current_output_path: &Path,
    link_targets: &impl MarkdownLinkTargets,
    source_book_ids: &HashSet<String>,
) -> String {
    let html = rewrite_page_link_targets(page, current_output_path, link_targets, source_book_ids);
    let anchor_targets = markdown_heading_anchor_targets(&html);
    let html = normalize_code_examples(&html);
    let options = MarkdownOptions::new()
        .include_links(true)
        .include_images(false)
        .preserve_tables(true)
        .escape_special_chars(false);
    let markdown = html_to_markdown_with_options(&html, &options);
    let markdown = ensure_markdown_heading(&page.title, normalize_markdown(markdown));
    materialize_markdown_heading_anchors(&markdown, &anchor_targets)
}

fn raw_page_to_linked_markdown(
    raw_html: &str,
    html_path: &str,
    title: &str,
    current_output_path: &Path,
    link_targets: &impl MarkdownLinkTargets,
    source_book_ids: &HashSet<String>,
) -> String {
    let empty_replacements = HashMap::new();
    let html = replace_href_attributes(
        raw_html,
        &empty_replacements,
        html_path,
        current_output_path,
        link_targets,
        source_book_ids,
    );
    let anchor_targets = markdown_heading_anchor_targets(&html);
    let html = normalize_code_examples(&html);
    let options = MarkdownOptions::new()
        .include_links(true)
        .include_images(false)
        .preserve_tables(true)
        .escape_special_chars(false);
    let markdown = html_to_markdown_with_options(&html, &options);
    let markdown = normalize_markdown(markdown);
    let title = if markdown_starts_with_heading(&markdown) {
        title.to_string()
    } else {
        raw_html_page_title(raw_html, title)
    };
    let markdown = ensure_markdown_heading(&title, markdown);
    materialize_markdown_heading_anchors(&markdown, &anchor_targets)
}

fn markdown_starts_with_heading(markdown: &str) -> bool {
    markdown
        .lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.starts_with('#'))
}

fn raw_html_page_title(raw_html: &str, fallback: &str) -> String {
    first_html_element_text(raw_html, "title")
        .or_else(|| first_html_element_text(raw_html, "h1"))
        .unwrap_or_else(|| fallback.to_string())
}

fn first_html_element_text(raw_html: &str, tag_name: &str) -> Option<String> {
    let open_pattern = format!("<{tag_name}");
    let close_pattern = format!("</{tag_name}");
    let mut cursor = 0;
    while let Some(open_start) = find_ascii_case_insensitive(raw_html, cursor, &open_pattern) {
        let open_end = raw_html[open_start..]
            .find('>')
            .map(|offset| open_start + offset + 1)?;
        let close_start = find_ascii_case_insensitive(raw_html, open_end, &close_pattern)?;
        let text = normalize_html_text(&raw_html[open_end..close_start]);
        if !text.is_empty() {
            return Some(text);
        }
        cursor = close_start + close_pattern.len();
    }
    None
}

fn normalize_html_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(character),
            _ => {}
        }
    }
    decode_basic_html_entities(&text)
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_basic_html_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn markdown_link_targets(plans: &[MarkdownTocExportPlan]) -> HashMap<String, PathBuf> {
    let mut targets = HashMap::new();
    for plan in plans {
        if !is_heading_only_toc_path(&plan.html_path) {
            targets
                .entry(plan.html_path.clone())
                .or_insert_with(|| plan.relative_path.clone());
        }
    }
    targets
}

fn rewrite_page_link_targets(
    page: &PageContent,
    current_output_path: &Path,
    link_targets: &impl MarkdownLinkTargets,
    source_book_ids: &HashSet<String>,
) -> String {
    let mut replacements: HashMap<String, Option<String>> = HashMap::new();
    for link in &page.links {
        if is_external_href(&link.raw_href) {
            continue;
        }
        let replacement = link
            .normalized_path
            .as_deref()
            .and_then(|target| link_targets.markdown_link_target(target, source_book_ids))
            .map(|target| {
                append_markdown_link_fragment(
                    relative_markdown_link(current_output_path, target),
                    &link.raw_href,
                )
            });
        replacements
            .entry(link.raw_href.clone())
            .and_modify(|current| {
                if current.is_none() {
                    *current = replacement.clone();
                }
            })
            .or_insert(replacement);
    }

    replace_href_attributes(
        &page.raw_html,
        &replacements,
        &page.source.html_path,
        current_output_path,
        link_targets,
        source_book_ids,
    )
}

fn is_external_href(href: &str) -> bool {
    href.contains(':') && !href.trim_start().starts_with("v8help://")
}

fn replace_href_attributes(
    html: &str,
    replacements: &HashMap<String, Option<String>>,
    current_html_path: &str,
    current_output_path: &Path,
    link_targets: &impl MarkdownLinkTargets,
    source_book_ids: &HashSet<String>,
) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    while let Some(position) = find_href_attribute(html, cursor) {
        let Some(attribute) = parse_href_attribute(html, position) else {
            let next = advance_one_char(html, position);
            output.push_str(&html[cursor..next]);
            cursor = next;
            continue;
        };
        let replacement = replacements.get(attribute.value).cloned().or_else(|| {
            href_replacement_for_raw_value(
                current_html_path,
                current_output_path,
                link_targets,
                source_book_ids,
                attribute.value,
            )
        });
        let Some(replacement) = replacement else {
            output.push_str(&html[cursor..attribute.end]);
            cursor = attribute.end;
            continue;
        };

        output.push_str(&html[cursor..attribute.start]);
        if let Some(target) = replacement {
            output.push_str(&html[attribute.start..attribute.value_start]);
            output.push_str(&target);
            output.push_str(&html[attribute.value_end..attribute.end]);
        }
        cursor = attribute.end;
    }
    output.push_str(&html[cursor..]);
    output
}

fn href_replacement_for_raw_value(
    current_html_path: &str,
    current_output_path: &Path,
    link_targets: &impl MarkdownLinkTargets,
    source_book_ids: &HashSet<String>,
    raw_href: &str,
) -> Option<Option<String>> {
    if is_external_href(raw_href) {
        return None;
    }
    let target = normalize_markdown_link_target(current_html_path, raw_href)
        .as_deref()
        .and_then(|target| link_targets.markdown_link_target(target, source_book_ids))
        .map(|target| {
            append_markdown_link_fragment(
                relative_markdown_link(current_output_path, target),
                raw_href,
            )
        });
    Some(target)
}

fn append_markdown_link_fragment(mut target: String, raw_href: &str) -> String {
    if let Some(fragment) = markdown_link_fragment(raw_href) {
        target.push('#');
        target.push_str(fragment);
    }
    target
}

fn markdown_link_fragment(raw_href: &str) -> Option<&str> {
    raw_href
        .split_once('#')
        .map(|(_, fragment)| fragment.split('?').next().unwrap_or_default().trim())
        .filter(|fragment| !fragment.is_empty())
}

fn source_book_link_ids(book: &HbkBook) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Some(stem) = book.path().file_stem().and_then(|value| value.to_str()) {
        ids.insert(stem.to_string());
        if let Some((base, _)) = stem.rsplit_once('_') {
            ids.insert(base.to_string());
        }
    }
    if !book.meta().book_name.is_empty() {
        ids.insert(book.meta().book_name.clone());
    }
    ids
}

fn normalize_markdown_link_target(current_html_path: &str, href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }
    if href.starts_with('#') {
        return Some(current_html_path.to_string());
    }
    if is_external_href(href) {
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
        match current_html_path.rsplit_once('/') {
            Some((base, _)) if !base.is_empty() => format!("{base}/{path_part}"),
            _ => path_part.to_string(),
        }
    };
    normalize_storage_path_segments(&candidate)
}

#[derive(Debug, Clone, Copy)]
struct HrefAttribute<'a> {
    start: usize,
    end: usize,
    value_start: usize,
    value_end: usize,
    value: &'a str,
}

fn find_href_attribute(html: &str, from: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut index = from;
    while index + 4 <= bytes.len() {
        if bytes[index..index + 4].eq_ignore_ascii_case(b"href")
            && html_attribute_name_start_boundary(bytes, index)
            && html_attribute_name_boundary(bytes, index + 4)
        {
            return Some(index);
        }
        index = advance_one_char(html, index);
    }
    None
}

fn html_attribute_name_start_boundary(bytes: &[u8], index: usize) -> bool {
    index == 0 || html_attribute_name_boundary(bytes, index - 1)
}

fn html_attribute_name_boundary(bytes: &[u8], index: usize) -> bool {
    index == 0
        || index >= bytes.len()
        || !matches!(
            bytes[index],
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b':'
        )
}

fn parse_href_attribute(html: &str, start: usize) -> Option<HrefAttribute<'_>> {
    let bytes = html.as_bytes();
    let mut index = start + 4;
    skip_ascii_whitespace(bytes, &mut index);
    if bytes.get(index) != Some(&b'=') {
        return None;
    }
    index += 1;
    skip_ascii_whitespace(bytes, &mut index);
    let quote = *bytes.get(index)?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let value_start = index + 1;
    let value_end = bytes[value_start..]
        .iter()
        .position(|byte| *byte == quote)
        .map(|offset| value_start + offset)?;
    Some(HrefAttribute {
        start,
        end: value_end + 1,
        value_start,
        value_end,
        value: &html[value_start..value_end],
    })
}

fn skip_ascii_whitespace(bytes: &[u8], index: &mut usize) {
    while bytes.get(*index).is_some_and(u8::is_ascii_whitespace) {
        *index += 1;
    }
}

fn advance_one_char(value: &str, index: usize) -> usize {
    value[index..]
        .chars()
        .next()
        .map(|character| index + character.len_utf8())
        .unwrap_or(value.len())
}

fn relative_markdown_link(current_output_path: &Path, target_output_path: &Path) -> String {
    let current_dir = current_output_path
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let current_components = path_components(current_dir);
    let target_components = path_components(target_output_path);
    let common = current_components
        .iter()
        .zip(&target_components)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = Vec::new();
    for _ in common..current_components.len() {
        parts.push("..".to_string());
    }
    parts.extend(target_components.into_iter().skip(common));
    if parts.is_empty() {
        "index.md".to_string()
    } else {
        parts.join("/")
    }
}

fn path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkdownHeadingAnchorTarget {
    level: usize,
    text: String,
    id: String,
}

fn markdown_heading_anchor_targets(html: &str) -> Vec<MarkdownHeadingAnchorTarget> {
    let fragment = Html::parse_fragment(html);
    let heading_selector =
        Selector::parse("h1, h2, h3, h4, h5, h6").expect("static selector must be valid");
    let anchor_selector = Selector::parse("[name], [id]").expect("static selector must be valid");
    let mut targets = Vec::new();

    for heading in fragment.select(&heading_selector) {
        let Some(level) = markdown_heading_level(heading.value().name()) else {
            continue;
        };
        let Some(id) = element_anchor_id(heading)
            .or_else(|| heading.select(&anchor_selector).find_map(element_anchor_id))
        else {
            continue;
        };
        let text = normalize_markdown_heading_text(&heading.text().collect::<String>());
        if text.is_empty() {
            continue;
        }
        targets.push(MarkdownHeadingAnchorTarget { level, text, id });
    }

    targets
}

fn markdown_heading_level(tag_name: &str) -> Option<usize> {
    let level = tag_name.strip_prefix('h')?.parse::<usize>().ok()?;
    (1..=6).contains(&level).then_some(level)
}

fn element_anchor_id(element: ElementRef<'_>) -> Option<String> {
    element
        .value()
        .attr("name")
        .or_else(|| element.value().attr("id"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn materialize_markdown_heading_anchors(
    markdown: &str,
    targets: &[MarkdownHeadingAnchorTarget],
) -> String {
    if targets.is_empty() {
        return markdown.to_string();
    }

    let mut output = String::with_capacity(markdown.len() + targets.len() * 24);
    let mut next_target = 0;
    for segment in markdown.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        if let Some((level, text)) = parse_markdown_heading_line(line) {
            if !targets
                .get(next_target)
                .is_some_and(|target| target.level == level && target.text == text)
            {
                output.push_str(segment);
                continue;
            }
            output.push_str("<a id=\"");
            output.push_str(&escape_html_attribute(&targets[next_target].id));
            output.push_str("\"></a>\n");
            next_target += 1;
        }
        output.push_str(segment);
    }
    output
}

fn parse_markdown_heading_line(line: &str) -> Option<(usize, String)> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if !(1..=6).contains(&hashes)
        || !line
            .as_bytes()
            .get(hashes)
            .is_some_and(u8::is_ascii_whitespace)
    {
        return None;
    }
    let text = normalize_markdown_heading_text(&line[hashes..]);
    (!text.is_empty()).then_some((hashes, text))
}

fn normalize_markdown_heading_text(text: &str) -> String {
    text.replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_code_examples(html: &str) -> String {
    let html = normalize_code_example_tables(html);
    let html = normalize_layout_blockquote_tables(&html);
    normalize_query_code_blockquotes(&html)
}

fn normalize_code_example_tables(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    while let Some(start) = find_ascii_case_insensitive(html, cursor, "<table") {
        let Some(end_tag_start) = find_ascii_case_insensitive(html, start, "</table") else {
            break;
        };
        let Some(end_tag_end) = html[end_tag_start..]
            .find('>')
            .map(|offset| end_tag_start + offset + 1)
        else {
            break;
        };
        let table_html = &html[start..end_tag_end];
        output.push_str(&html[cursor..start]);
        if let Some(code_block) = code_example_table_to_pre(table_html) {
            output.push_str(&code_block);
        } else {
            output.push_str(table_html);
        }
        cursor = end_tag_end;
    }
    output.push_str(&html[cursor..]);
    output
}

fn code_example_table_to_pre(table_html: &str) -> Option<String> {
    if !html_contains_ascii_case_insensitive(table_html, "courier") {
        return None;
    }

    let fragment = Html::parse_fragment(table_html);
    let cell_selector = Selector::parse("td, th").expect("static selector must be valid");
    let mut cells = fragment.select(&cell_selector);
    let cell = cells.next()?;
    if cells.next().is_some() {
        return None;
    }

    let mut code = String::new();
    collect_code_example_text(cell, &mut code);
    let code = normalize_code_example_text(&code);
    (!code.is_empty()).then(|| {
        format!(
            "<pre><code class=\"language-bsl\">{}</code></pre>",
            escape_html_text(&code)
        )
    })
}

fn normalize_layout_blockquote_tables(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    while let Some(start) = find_ascii_case_insensitive(html, cursor, "<blockquote") {
        let Some(end_tag_start) = find_ascii_case_insensitive(html, start, "</blockquote") else {
            break;
        };
        let Some(end_tag_end) = html[end_tag_start..]
            .find('>')
            .map(|offset| end_tag_start + offset + 1)
        else {
            break;
        };
        let blockquote_html = &html[start..end_tag_end];
        output.push_str(&html[cursor..start]);
        if let Some(blockquote) = layout_blockquote_tables_to_html(blockquote_html) {
            output.push_str(&blockquote);
        } else {
            output.push_str(blockquote_html);
        }
        cursor = end_tag_end;
    }
    output.push_str(&html[cursor..]);
    output
}

fn layout_blockquote_tables_to_html(blockquote_html: &str) -> Option<String> {
    if !html_contains_ascii_case_insensitive(blockquote_html, "<table")
        || html_contains_ascii_case_insensitive(blockquote_html, "courier")
        || html_contains_ascii_case_insensitive(blockquote_html, "href")
    {
        return None;
    }

    let fragment = Html::parse_fragment(blockquote_html);
    let blockquote_selector = Selector::parse("blockquote").expect("static selector must be valid");
    let table_selector = Selector::parse("table").expect("static selector must be valid");
    let row_selector = Selector::parse("tr").expect("static selector must be valid");
    let cell_selector = Selector::parse("td, th").expect("static selector must be valid");

    let blockquote = fragment.select(&blockquote_selector).next()?;
    let mut table_count = 0;
    let mut lines = Vec::new();

    for table in blockquote.select(&table_selector) {
        table_count += 1;
        for row in table.select(&row_selector) {
            let mut row_cells = Vec::new();
            for cell in row.select(&cell_selector) {
                let text = normalize_layout_cell_text(&cell.text().collect::<String>());
                if !text.is_empty() {
                    row_cells.push(text);
                }
            }
            match row_cells.len() {
                0 => {}
                1 => lines.push(row_cells.remove(0)),
                _ => return None,
            }
        }
    }

    if table_count < 2 || lines.len() < 2 {
        return None;
    }

    let mut output = String::from("<blockquote>");
    for line in lines {
        output.push_str("<p>");
        output.push_str(&escape_html_text(&line));
        output.push_str("</p>");
    }
    output.push_str("</blockquote>");
    Some(output)
}

fn normalize_layout_cell_text(text: &str) -> String {
    decode_basic_html_entities(text)
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_query_code_blockquotes(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    while let Some(start) = find_ascii_case_insensitive(html, cursor, "<blockquote") {
        let Some(end_tag_start) = find_ascii_case_insensitive(html, start, "</blockquote") else {
            break;
        };
        let Some(end_tag_end) = html[end_tag_start..]
            .find('>')
            .map(|offset| end_tag_start + offset + 1)
        else {
            break;
        };
        let blockquote_html = &html[start..end_tag_end];
        output.push_str(&html[cursor..start]);
        if let Some(code_block) = query_code_blockquote_to_pre(blockquote_html) {
            output.push_str(&code_block);
        } else {
            output.push_str(blockquote_html);
        }
        cursor = end_tag_end;
    }
    output.push_str(&html[cursor..]);
    output
}

fn query_code_blockquote_to_pre(blockquote_html: &str) -> Option<String> {
    if !html_contains_ascii_case_insensitive(blockquote_html, "courier")
        || html_contains_ascii_case_insensitive(blockquote_html, "href")
    {
        return None;
    }

    let fragment = Html::parse_fragment(blockquote_html);
    let blockquote_selector = Selector::parse("blockquote").expect("static selector must be valid");
    let blockquote = fragment.select(&blockquote_selector).next()?;
    let mut code = String::new();
    collect_code_example_text(blockquote, &mut code);
    let code = normalize_code_example_text(&code);
    (!code.is_empty()).then(|| {
        format!(
            "<pre><code class=\"language-sdbl\">{}</code></pre>",
            escape_html_text(&code)
        )
    })
}

fn collect_code_example_text(element: ElementRef<'_>, output: &mut String) {
    for child in element.children() {
        match child.value() {
            Node::Text(text) => output.push_str(text),
            Node::Element(element) => {
                let tag_name = element.name();
                if tag_name.eq_ignore_ascii_case("br") {
                    output.push('\n');
                } else if let Some(child_element) = ElementRef::wrap(child) {
                    collect_code_example_text(child_element, output);
                }
            }
            _ => {}
        }
    }
}

fn normalize_code_example_text(code: &str) -> String {
    let code = code.replace('\r', "").replace('\u{a0}', " ");
    let mut lines = code.lines().map(str::trim_end).collect::<Vec<_>>();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let mut output = lines.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    output
}

fn escape_html_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(character),
        }
    }
    output
}

fn escape_html_attribute(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '"' => output.push_str("&quot;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(character),
        }
    }
    output
}

fn html_contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    find_ascii_case_insensitive(haystack, 0, needle).is_some()
}

fn find_ascii_case_insensitive(haystack: &str, from: usize, needle: &str) -> Option<usize> {
    if needle.is_empty() || from >= haystack.len() {
        return None;
    }
    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    if needle.len() > haystack.len() {
        return None;
    }

    (from..=haystack.len() - needle.len()).find(|start| {
        haystack[*start..*start + needle.len()]
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}

fn heading_only_markdown(title: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        String::new()
    } else {
        format!("# {title}\n")
    }
}

fn is_heading_only_toc_path(html_path: &str) -> bool {
    html_path.is_empty() || is_content_node_placeholder_path(html_path)
}

fn is_content_node_placeholder_path(html_path: &str) -> bool {
    html_path.starts_with("_CONTENTS_NODE_")
}

fn documentation_error_is_missing_page(error: &DocumentationError) -> bool {
    match error {
        DocumentationError::PageRead { source, .. } => {
            matches!(source.as_ref(), BookError::MissingZipEntry { .. })
        }
    }
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
    fn markdown_page_loader_rewrites_links_from_raw_html() {
        let workspace = TempWorkspace::new("markdown-loader-links");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r##"{
            2
            {1,0,0,{0,0,{0,0,{"ru","Корень"}},"/docs/root.html"}}
            {2,0,0,{0,0,{0,0,{"ru","Цель"}},"/docs/target.html"}}
        }"##;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                (
                    "docs/root.html",
                    r##"<html><body>
                        <h1>Корень</h1>
                        <p><a href="target.html#Details">Цель</a></p>
                    </body></html>"##
                        .as_bytes(),
                ),
                (
                    "docs/target.html",
                    r##"<html><body><h1 id="Details">Цель</h1></body></html>"##.as_bytes(),
                ),
            ],
        );
        let book = HbkBook::open(&source_path).expect("book must open");
        let mut link_targets = HashMap::new();
        link_targets.insert(
            "docs/target.html".to_string(),
            PathBuf::from("target-page.md"),
        );
        let mut loader = BookExporter::new(&book)
            .markdown_page_loader()
            .expect("markdown page loader must open");

        let page = loader
            .linked_markdown_toc_page(
                "docs/root.html",
                "Корень",
                Path::new("root-page.md"),
                &link_targets,
                &source_book_link_ids(&book),
            )
            .expect("loader must convert raw HTML to linked Markdown");

        assert!(page.markdown().contains("[Цель](target-page.md#Details)"));
        assert!(!page.markdown().contains("target.html"));
    }

    #[test]
    fn markdown_page_loader_prefers_html_title_over_toc_title() {
        let workspace = TempWorkspace::new("markdown-loader-title");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","TOC title"}},"/docs/page.html"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![(
                "docs/page.html",
                r#"<html><head><title>HTML&nbsp;title</title></head><body><p>body</p></body></html>"#
                    .as_bytes(),
            )],
        );
        let book = HbkBook::open(&source_path).expect("book must open");
        let mut loader = BookExporter::new(&book)
            .markdown_page_loader()
            .expect("markdown page loader must open");

        let page = loader
            .linked_markdown_toc_page(
                "docs/page.html",
                "TOC title",
                Path::new("page.md"),
                &HashMap::new(),
                &source_book_link_ids(&book),
            )
            .expect("loader must convert page Markdown");

        assert!(
            page.markdown().starts_with("# HTML title\n"),
            "{}",
            page.markdown()
        );
        assert!(!page.markdown().starts_with("# TOC title\n"));
    }

    #[test]
    fn markdown_page_loader_keeps_missing_toc_page_as_heading_only() {
        let workspace = TempWorkspace::new("markdown-loader-missing");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Отсутствует"}},"/docs/missing.html"}}
        }"#;
        write_book_fixture_with_toc(&source_path, toc, Vec::new());
        let book = HbkBook::open(&source_path).expect("book must open");
        let mut loader = BookExporter::new(&book)
            .markdown_page_loader()
            .expect("markdown page loader must open");

        let page = loader
            .linked_markdown_toc_page(
                "docs/missing.html",
                "Отсутствует",
                Path::new("missing.md"),
                &HashMap::new(),
                &source_book_link_ids(&book),
            )
            .expect("missing TOC storage page must become heading-only Markdown");

        assert_eq!(page.markdown(), "# Отсутствует\n");
    }

    #[test]
    fn exports_markdown_toc_pages_under_deterministic_title_paths() {
        let workspace = TempWorkspace::new("markdown-toc-layout");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r#"{
            5
            {1,0,4,2,3,4,5,{0,0,{0,0,{"ru","Справка"}{"en","Help"}},"/docs/root.html"}}
            {2,1,0,{0,0,{0,0,{"ru","Раздел"}{"en","Section"}},"/docs/child.html"}}
            {3,1,0,{0,0,{0,0,{"ru","Раздел"}{"en","Section"}},"/docs/child-two.html"}}
            {4,1,0,{0,0,{0,0,{"ru","Группа"}{"en","Group"}},""}}
            {5,1,0,{0,0,{0,0,{"ru","Ссылка HTML"}{"en","HTML link"}},"/objects/raw.html"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                (
                    "docs/root.html",
                    r#"<html><body>
                        <h1>Справка</h1>
                        <p><a HREF = "child.html">Раздел</a></p>
                        <p><a href="v8help://fmtdui/docs/child-two.html">Вторая</a></p>
                        <p><a href="v8help://otherbook/docs/child.html">Другая книга</a></p>
                        <p><a href="missing.html">Несуществующая</a></p>
                        <p><a href="https://example.com/help">Внешняя</a></p>
                        <img src="assets/pic.png" alt="Картинка">
                    </body></html>"#
                        .as_bytes(),
                ),
                (
                    "docs/child.html",
                    "<html><body><h1>Раздел</h1><p>Первый</p></body></html>".as_bytes(),
                ),
                (
                    "docs/child-two.html",
                    "<html><body><h1>Раздел</h1><p>Второй</p></body></html>".as_bytes(),
                ),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");

        let result = BookExporter::new(&book)
            .export(&request)
            .expect("markdown/toc export must succeed");

        let exported: Vec<_> = result
            .files()
            .iter()
            .map(|file| {
                file.path()
                    .strip_prefix(&output_root)
                    .expect("exported file must be under output root")
                    .to_path_buf()
            })
            .collect();
        assert_eq!(
            exported,
            vec![
                PathBuf::from("справка/index.md"),
                PathBuf::from("справка/раздел/index.md"),
                PathBuf::from("справка/раздел-2/index.md"),
                PathBuf::from("справка/группа/index.md"),
                PathBuf::from("справка/ссылка-html/index.md"),
            ]
        );

        let root_markdown = fs::read_to_string(output_root.join("справка/index.md"))
            .expect("root page must be exported");
        assert!(
            root_markdown.contains("[Раздел](раздел/index.md)"),
            "{root_markdown}"
        );
        assert!(
            root_markdown.contains("[Вторая](раздел-2/index.md)"),
            "{root_markdown}"
        );
        assert!(root_markdown.contains("Другая книга"));
        assert!(!root_markdown.contains("[Другая книга]"));
        assert!(!root_markdown.contains("otherbook"));
        assert!(root_markdown.contains("Несуществующая"));
        assert!(!root_markdown.contains("child.html"));
        assert!(!root_markdown.contains("missing.html"));
        assert!(root_markdown.contains("[Внешняя](https://example.com/help)"));
        assert!(!root_markdown.contains("assets/pic.png"));
        assert_no_raw_markdown_scaffolding(&root_markdown);

        let heading_only = fs::read_to_string(output_root.join("справка/группа/index.md"))
            .expect("empty TOC path page must be exported");
        assert_eq!(heading_only, "# Группа\n");
        let missing_storage_page =
            fs::read_to_string(output_root.join("справка/ссылка-html/index.md"))
                .expect("missing storage TOC page must be exported as heading-only Markdown");
        assert_eq!(missing_storage_page, "# Ссылка HTML\n");
    }

    #[test]
    fn exports_shared_content_node_placeholders_with_each_toc_title() {
        let workspace = TempWorkspace::new("markdown-content-node-placeholder");
        let source_path = workspace.path().join("shclang_ru.hbk");
        let toc = r#"{
            4
            {1,0,2,2,3,{0,0,{0,0,{"ru","Встроенный язык"}{"en","Language"}},""}}
            {2,1,0,{0,0,{0,0,{"ru","Общее описание встроенного языка"}{"en","General"}},"_CONTENTS_NODE_fileConf"}}
            {3,1,1,4,{0,0,{0,0,{"ru","Общие объекты"}{"en","Common objects"}},"_CONTENTS_NODE_fileConf"}}
            {4,3,0,{0,0,{0,0,{"ru","Основные понятия XBASE"}{"en","XBASE"}},"MainXBase"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                (
                    "_CONTENTS_NODE_fileConf",
                    b"\xef\xbb\xbf<html><body></body></html>",
                ),
                (
                    "MainXBase",
                    "<html><body><h1>Основные понятия XBASE</h1><p>Содержательная страница</p></body></html>"
                        .as_bytes(),
                ),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");

        BookExporter::new(&book)
            .export(&request)
            .expect("markdown/toc export must succeed");

        let general = fs::read_to_string(
            output_root.join("встроенный-язык/общее-описание-встроенного-языка/index.md"),
        )
        .expect("first placeholder page must be exported");
        let common = fs::read_to_string(output_root.join("встроенный-язык/общие-объекты/index.md"))
            .expect("second placeholder page must be exported");
        let real = fs::read_to_string(
            output_root.join("встроенный-язык/общие-объекты/основные-понятия-xbase/index.md"),
        )
        .expect("real child page must be exported");

        assert_eq!(general, "# Общее описание встроенного языка\n");
        assert_eq!(common, "# Общие объекты\n");
        assert!(real.contains("# Основные понятия XBASE"));
        assert!(real.contains("Содержательная страница"));
    }

    #[test]
    fn converts_single_cell_courier_tables_to_markdown_code_blocks() {
        let workspace = TempWorkspace::new("markdown-code-table");
        let source_path = workspace.path().join("shclang_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Работа с пакетными запросами"}{"en","Batch"}},"WorkinWithBath"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![(
                "WorkinWithBath",
                r##"<html><body>
                    <h1>Работа с пакетными запросами</h1>
                    <p>Например:</p>
                    <table width="100%" bgcolor="#f7f7f7"><tbody><tr><td>
                        <font face="Courier New">Запрос&nbsp;=&nbsp;Новый&nbsp;Запрос;<br>
                        Запрос.Текст = "ВЫБРАТЬ<br>
                        &nbsp;&nbsp;&nbsp;&nbsp;|&nbsp;УчетНоменклатуры.Номенклатура<br>
                        &nbsp;&nbsp;&nbsp;&nbsp;|";<br><br>
                        Результат=Запрос.Выполнить();</font>
                    </td></tr></tbody></table>
                </body></html>"##
                    .as_bytes(),
            )],
        );
        let book = HbkBook::open(&source_path).expect("book must open");

        let page = BookExporter::new(&book)
            .markdown_page("WorkinWithBath")
            .expect("TOC page must convert to Markdown");
        let markdown = page.markdown();

        assert!(markdown.contains("```bsl"), "{markdown}");
        assert!(markdown.contains("Запрос = Новый Запрос;"), "{markdown}");
        assert!(markdown.contains("Запрос.Текст = \"ВЫБРАТЬ"), "{markdown}");
        assert!(
            markdown.contains("    | УчетНоменклатуры.Номенклатура"),
            "{markdown}"
        );
        assert!(
            markdown.contains("Результат=Запрос.Выполнить();"),
            "{markdown}"
        );
        assert!(!markdown.contains("| Запрос = Новый Запрос"), "{markdown}");
        assert_no_raw_markdown_scaffolding(markdown);
    }

    #[test]
    fn converts_courier_query_blockquotes_to_sdbl_code_blocks() {
        let workspace = TempWorkspace::new("markdown-sdbl-blockquote");
        let source_path = workspace.path().join("shclang_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Работа с временными таблицами"}{"en","Temp tables"}},"Work with temp table"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![(
                "Work with temp table",
                r##"<html><body>
                    <h1>Работа с временными таблицами</h1>
                    <blockquote style="MARGIN-RIGHT: 0px" dir="ltr"><p><font face="Courier New">ВЫБРАТЬ<br>&nbsp;&nbsp; Код,<br>&nbsp;&nbsp; Наименование<br>ПОМЕСТИТЬ ВременнаяТаблица<br>ИЗ Справочник.Номенклатура</font></p></blockquote>
                </body></html>"##
                    .as_bytes(),
            )],
        );
        let book = HbkBook::open(&source_path).expect("book must open");

        let page = BookExporter::new(&book)
            .markdown_page("Work with temp table")
            .expect("TOC page must convert to Markdown");
        let markdown = page.markdown();

        assert!(markdown.contains("```sdbl"), "{markdown}");
        assert!(markdown.contains("ВЫБРАТЬ\n   Код,"), "{markdown}");
        assert!(
            markdown.contains("ПОМЕСТИТЬ ВременнаяТаблица"),
            "{markdown}"
        );
        assert!(
            markdown.contains("ИЗ Справочник.Номенклатура"),
            "{markdown}"
        );
        assert!(!markdown.contains("> ВЫБРАТЬ"), "{markdown}");
        assert_no_raw_markdown_scaffolding(markdown);
    }

    #[test]
    fn converts_layout_blockquote_tables_to_readable_quote_lines() {
        let workspace = TempWorkspace::new("markdown-layout-blockquote-table");
        let source_path = workspace.path().join("1cv8_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Запуск 1С:Предприятие 8 и параметры запуска"}{"en","Startup"}},"ZIF"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![(
                "ZIF",
                r##"<html><body>
                    <h1>Запуск 1С:Предприятие 8 и параметры запуска</h1>
                    <p>Интерактивная программа запуска откроет список информационных баз.</p>
                    <blockquote style="MARGIN-RIGHT: 0px" dir="ltr">
                    <table id="table5" border="1"><tbody>
                        <tr><td bgcolor="#fffef0" colspan="2">&nbsp;Программа запуска - <strong>1CEStart</strong></td></tr>
                        <tr><td></td><td>&nbsp;&nbsp;</td></tr>
                    </tbody></table>
                    <table id="table6" border="1"><tbody>
                        <tr><td></td><td bgcolor="#fffef0">&nbsp;Интерактивная программа запуска - <strong>1Cv8s</strong></td></tr>
                    </tbody></table>
                    <table id="table7" border="1"><tbody>
                        <tr><td></td><td>&nbsp;</td></tr>
                    </tbody></table>
                    <table id="table8" border="1"><tbody>
                        <tr><td></td><td bgcolor="#fffef0">&nbsp;Клиентское приложение</td></tr>
                    </tbody></table>
                    </blockquote>
                    <p>Программе запуска можно указывать различные параметры командной строки.</p>
                </body></html>"##
                    .as_bytes(),
            )],
        );
        let book = HbkBook::open(&source_path).expect("book must open");

        let page = BookExporter::new(&book)
            .markdown_page("ZIF")
            .expect("TOC page must convert to Markdown");
        let markdown = page.markdown();

        assert!(
            markdown.contains("> Программа запуска - 1CEStart"),
            "{markdown}"
        );
        assert!(
            markdown.contains("> Интерактивная программа запуска - 1Cv8s"),
            "{markdown}"
        );
        assert!(markdown.contains("> Клиентское приложение"), "{markdown}");
        assert!(!markdown.contains("> |"), "{markdown}");
        assert!(!markdown.contains("> | ---"), "{markdown}");
        assert_no_raw_markdown_scaffolding(markdown);
    }

    #[test]
    fn preserves_internal_link_fragments_in_markdown_targets() {
        let workspace = TempWorkspace::new("markdown-link-fragments");
        let source_path = workspace.path().join("shclang_ru.hbk");
        let toc = r#"{
            2
            {1,0,0,{0,0,{0,0,{"ru","Основные понятия XBASE"}{"en","XBASE"}},"MainXBase"}}
            {2,0,0,{0,0,{0,0,{"ru","Другая страница"}{"en","Other"}},"OtherPage"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                (
                    "MainXBase",
                    r##"<html><body>
                        <h1>Основные понятия XBASE</h1>
                        <p><a href="#FieldsRecords">Поля и записи</a></p>
                        <p><a href="OtherPage#Details">Другая страница</a></p>
                        <p><a href="#DirectId">Заголовок с id</a></p>
                        <p><a href="#SecondParams">Вторые параметры</a></p>
                        <h2><a name="FieldsRecords">Поля и записи</a></h2>
                        <h2 id="DirectId">Заголовок с id</h2>
                        <h2><a name="FirstParams"></a>Параметры</h2>
                        <h2><a name="SecondParams"></a>Параметры</h2>
                    </body></html>"##
                        .as_bytes(),
                ),
                (
                    "OtherPage",
                    r##"<html><body><h1>Другая страница</h1><h2><a name="Details">Детали</a></h2></body></html>"##
                        .as_bytes(),
                ),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");

        BookExporter::new(&book)
            .export(&request)
            .expect("markdown/toc export must succeed");

        let markdown = fs::read_to_string(output_root.join("основные-понятия-xbase/index.md"))
            .expect("Markdown page must be exported");
        let other_markdown = fs::read_to_string(output_root.join("другая-страница/index.md"))
            .expect("linked Markdown page must be exported");

        assert!(markdown.contains("[Поля и записи](index.md#FieldsRecords)"));
        assert!(markdown.contains("[Другая страница](../другая-страница/index.md#Details)"));
        assert!(markdown.contains("<a id=\"FieldsRecords\"></a>\n## Поля и записи"));
        assert!(markdown.contains("<a id=\"DirectId\"></a>\n## Заголовок с id"));
        assert!(other_markdown.contains("<a id=\"Details\"></a>\n## Детали"));
        assert!(!markdown.contains("[Поля и записи](index.md)"));
        assert_duplicate_heading_anchors_stay_with_their_source_heading(&markdown);
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
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/1cv8_ru.hbk",
                html_path: "ZIF",
                expected: &[
                    "# Запуск 1С:Предприятие 8 и параметры запуска",
                    "> Программа запуска - 1CEStart",
                    "> Интерактивная программа запуска - 1Cv8s",
                    "> Клиентское приложение",
                ],
            },
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
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk",
                html_path: "WorkinWithBath",
                expected: &[
                    "# Работа с пакетными запросами",
                    "```bsl",
                    "Запрос = Новый Запрос;",
                    "    | УчетНоменклатурыОстаткиИОбороты.Номенклатура,",
                    "Результат=Запрос.Выполнить();",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk",
                html_path: "Work with temp table",
                expected: &[
                    "# Работа с временными таблицами",
                    "<a id=\"Manager\"></a>\n## Менеджер временных таблиц",
                    "<a id=\"Create\"></a>\n## Создание временных таблиц",
                    "<a id=\"Used\"></a>\n## Использование временных таблиц",
                    "<a id=\"Delete\"></a>\n## Удаление временных таблиц",
                    "```sdbl",
                    "ВЫБРАТЬ\n   Код,",
                    "ПОМЕСТИТЬ ВременнаяТаблица",
                    "ИЗ Справочник.Номенклатура",
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
            if case.html_path == "ZIF" {
                assert!(!markdown.contains("> |"), "{markdown}");
                assert!(!markdown.contains("> | ---"), "{markdown}");
            }
            assert_no_raw_markdown_scaffolding(markdown);
        }
    }

    #[test]
    fn real_shclang_content_node_pages_keep_toc_headings_when_platform_book_exists() {
        let book_path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk");
        if !book_path.exists() {
            return;
        }

        let workspace = TempWorkspace::new("real-shclang-content-nodes");
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(book_path).expect("platform HBK must open");
        let request = BookExportRequest::new(
            book_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");

        BookExporter::new(&book)
            .export(&request)
            .expect("markdown/toc export must succeed");

        let common = fs::read_to_string(output_root.join("встроенный-язык/общие-объекты/index.md"))
            .expect("common objects placeholder page must be exported");
        let query =
            fs::read_to_string(output_root.join("встроенный-язык/работа-с-запросами/index.md"))
                .expect("query placeholder page must be exported");

        assert_eq!(common, "# Общие объекты\n");
        assert_eq!(query, "# Работа с запросами\n");
    }

    #[test]
    fn real_shclang_xbase_page_preserves_internal_link_fragments_when_platform_book_exists() {
        let book_path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk");
        if !book_path.exists() {
            return;
        }

        let workspace = TempWorkspace::new("real-shclang-link-fragments");
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(book_path).expect("platform HBK must open");
        let request = BookExportRequest::new(
            book_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");

        BookExporter::new(&book)
            .export(&request)
            .expect("markdown/toc export must succeed");

        let markdown = fs::read_to_string(
            output_root.join("встроенный-язык/общие-объекты/xbase/основные-понятия-xbase/index.md"),
        )
        .expect("XBase Markdown page must be exported");

        assert!(markdown.contains("[Поля и записи](index.md#FieldsRecords)"));
        assert!(markdown.contains("[Работа с индексными файлами](index.md#WorkWithIndexFile)"));
        assert!(markdown.contains("[Ограничения](index.md#constraint)"));
        assert!(markdown.contains("<a id=\"FieldsRecords\"></a>"));
        assert!(markdown.contains("<a id=\"WorkWithIndexFile\"></a>"));
        assert!(markdown.contains("<a id=\"constraint\"></a>"));
        assert!(!markdown.contains("[Поля и записи](index.md)"));
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
            "<a href",
            "<a name",
            "<h1",
            "<h2",
            "<table",
            "<tr",
            "<td",
            "<ul",
            "<li",
            "<div",
            "<span",
            "</html",
            "</body",
            "</p",
            "</h",
            "</table",
            "</tr",
            "</td",
            "</ul",
            "</li",
            "</div",
            "</span",
            "&nbsp;",
            "v8help://service_book/service_style",
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

    fn assert_duplicate_heading_anchors_stay_with_their_source_heading(markdown: &str) {
        let first_heading = markdown
            .find("## Параметры")
            .expect("first duplicate heading must exist");
        let second_heading = markdown[first_heading + "## Параметры".len()..]
            .find("## Параметры")
            .map(|offset| first_heading + "## Параметры".len() + offset)
            .expect("second duplicate heading must exist");
        let first_anchor = markdown
            .find("<a id=\"FirstParams\"></a>")
            .expect("first duplicate heading anchor must exist");
        let second_anchor = markdown
            .find("<a id=\"SecondParams\"></a>")
            .expect("second duplicate heading anchor must exist");

        assert!(first_anchor < first_heading, "{markdown}");
        assert!(
            first_heading < second_anchor && second_anchor < second_heading,
            "{markdown}"
        );
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
