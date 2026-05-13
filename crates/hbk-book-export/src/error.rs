#[derive(Debug, Error)]
pub enum BookExportError {
    #[error("invalid book export output root '{}': {reason}", output_root.display())]
    InvalidOutputRoot {
        output_root: PathBuf,
        reason: OutputRootError,
    },
    #[error("unsupported book export combination: format={format}, hierarchy={hierarchy}")]
    UnsupportedCombination {
        format: BookExportFormat,
        hierarchy: BookExportHierarchy,
    },
    #[error("unsafe FileStorage path '{entry_name}' cannot be exported: {reason}")]
    UnsafeStoragePath {
        entry_name: String,
        reason: StoragePathError,
    },
    #[error("FileStorage path '{entry_name}' maps to duplicate export path '{}'", normalized_path.display())]
    DuplicateStoragePath {
        entry_name: String,
        normalized_path: PathBuf,
    },
    #[error("FileStorage path '{entry_name}' maps to '{}' which collides with '{}'", normalized_path.display(), existing_path.display())]
    StoragePathCollision {
        entry_name: String,
        normalized_path: PathBuf,
        existing_path: PathBuf,
    },
    #[error("TOC page '{html_path}' is not present in the opened HBK book")]
    TocPageNotFound {
        html_path: String,
    },
    #[error("book export request source '{}' does not match opened book '{}'", request_source_path.display(), book_path.display())]
    SourcePathMismatch {
        request_source_path: PathBuf,
        book_path: PathBuf,
    },
    #[error("failed to read HBK book for export: {0}")]
    Book(#[from] BookError),
    #[error("failed to read documentation page for export: {0}")]
    Documentation(#[from] DocumentationError),
    #[error("failed to {operation} '{}': {source}", path.display())]
    Io {
        path: PathBuf,
        operation: BookExportIoOperation,
        #[source]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum OutputRootError {
    #[error("path must contain at least one directory name")]
    MissingDirectoryName,
    #[error("path must not contain '..' segments")]
    ParentSegment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StoragePathError {
    #[error("path must contain at least one file name")]
    Empty,
    #[error("path must not contain '..' segments")]
    ParentSegment,
    #[error("path must be relative")]
    Absolute,
    #[error("path must not contain a Windows drive prefix")]
    WindowsPrefix,
    #[error("path must use '/' separators")]
    BackslashSeparator,
}
