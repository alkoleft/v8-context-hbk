use std::fmt;
use std::path::{Component, Path, PathBuf};

use hbk_book::HbkBook;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookExportError {
    InvalidOutputRoot {
        output_root: PathBuf,
        reason: OutputRootError,
    },
    UnsupportedCombination {
        format: BookExportFormat,
        hierarchy: BookExportHierarchy,
    },
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
        }
    }
}

impl std::error::Error for BookExportError {}

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
