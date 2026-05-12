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
