#[derive(Debug)]
pub enum SearchError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Sqlite {
        path: PathBuf,
        source: rusqlite::Error,
    },
    WriterLockTimeout {
        path: PathBuf,
    },
    MissingIndex {
        path: PathBuf,
    },
    UnsupportedSchemaVersion {
        path: PathBuf,
        expected: u32,
        actual: String,
    },
    DuplicateDocumentId {
        id: String,
        count: usize,
    },
    MissingParentIdentity {
        kind: String,
        name: String,
        owner: String,
    },
    AmbiguousLookup {
        name: String,
        matches: usize,
    },
    InvalidMetadata {
        path: PathBuf,
        key: &'static str,
        value: String,
        source: std::num::ParseIntError,
    },
    MissingMetadata {
        path: PathBuf,
        key: &'static str,
    },
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    f,
                    "failed to access search index '{}': {source}",
                    path.display()
                )
            }
            Self::Sqlite { path, source } => {
                write!(
                    f,
                    "failed to use search index '{}': {source}",
                    path.display()
                )
            }
            Self::WriterLockTimeout { path } => {
                write!(
                    f,
                    "timed out waiting for search index writer lock '{}'",
                    path.display()
                )
            }
            Self::MissingIndex { path } => {
                write!(f, "search index does not exist: {}", path.display())
            }
            Self::UnsupportedSchemaVersion {
                path,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "unsupported search index schema version in '{}': expected {expected}, got {actual}; rebuild the index",
                    path.display()
                )
            }
            Self::DuplicateDocumentId { id, count } => {
                write!(
                    f,
                    "duplicate Syntax Assistant search document id '{id}': {count} documents"
                )
            }
            Self::MissingParentIdentity { kind, name, owner } => {
                write!(
                    f,
                    "missing Syntax Assistant parent identity for {kind} '{name}' owned by '{owner}'"
                )
            }
            Self::AmbiguousLookup { name, matches } => {
                write!(
                    f,
                    "ambiguous Syntax Assistant lookup for '{name}': {matches} matches"
                )
            }
            Self::InvalidMetadata {
                path, key, value, ..
            } => {
                write!(
                    f,
                    "invalid search index metadata in '{}': key '{key}' has value '{value}'",
                    path.display()
                )
            }
            Self::MissingMetadata { path, key } => {
                write!(
                    f,
                    "missing search index metadata in '{}': key '{key}'",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for SearchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Sqlite { source, .. } => Some(source),
            Self::InvalidMetadata { source, .. } => Some(source),
            Self::WriterLockTimeout { .. }
            | Self::MissingIndex { .. }
            | Self::UnsupportedSchemaVersion { .. }
            | Self::DuplicateDocumentId { .. }
            | Self::MissingParentIdentity { .. }
            | Self::AmbiguousLookup { .. }
            | Self::MissingMetadata { .. } => None,
        }
    }
}

impl SearchError {
    fn metadata_parse(
        path: PathBuf,
        key: &'static str,
        value: String,
        source: std::num::ParseIntError,
    ) -> Self {
        Self::InvalidMetadata {
            path,
            key,
            value,
            source,
        }
    }
}
