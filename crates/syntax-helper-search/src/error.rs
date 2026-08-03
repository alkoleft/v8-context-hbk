#[derive(Debug, Error)]
pub enum SearchError {
    #[error("failed to access search index '{}': {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to use search index '{}': {source}", path.display())]
    Sqlite {
        path: PathBuf,
        #[source]
        source: rusqlite::Error,
    },
    #[error("timed out waiting for search index writer lock '{}'", path.display())]
    WriterLockTimeout {
        path: PathBuf,
    },
    #[error("HBK fact snapshot slot is in use: {}", path.display())]
    SnapshotInUse {
        path: PathBuf,
    },
    #[error("search index does not exist: {}", path.display())]
    MissingIndex {
        path: PathBuf,
    },
    #[error("unsupported search index schema version in '{}': expected {expected}, got {actual}; rebuild the index", path.display())]
    UnsupportedSchemaVersion {
        path: PathBuf,
        expected: u32,
        actual: String,
    },
    #[error("duplicate Syntax Assistant search document id '{id}': {count} documents")]
    DuplicateDocumentId {
        id: String,
        count: usize,
    },
    #[error("missing Syntax Assistant parent identity for {kind} '{name}' owned by '{owner}'")]
    MissingParentIdentity {
        kind: String,
        name: String,
        owner: String,
    },
    #[error("ambiguous Syntax Assistant lookup for '{name}': {matches} matches")]
    AmbiguousLookup {
        name: String,
        matches: usize,
    },
    #[error("invalid search index metadata in '{}': key '{key}' has value '{value}'", path.display())]
    InvalidMetadata {
        path: PathBuf,
        key: &'static str,
        value: String,
        #[source]
        source: std::num::ParseIntError,
    },
    #[error("missing search index metadata in '{}': key '{key}'", path.display())]
    MissingMetadata {
        path: PathBuf,
        key: &'static str,
    },
    #[error("invalid HBK fact snapshot artifact '{}': {source}", path.display())]
    SnapshotArtifact {
        path: PathBuf,
        #[source]
        source: HbkFactSnapshotArtifactError,
    },
}

#[derive(Debug, Error)]
pub enum HbkFactSnapshotArtifactError {
    #[error("{message}")]
    Invalid { message: String },
    #[error("compatibility mismatch for {field}: expected '{expected}', got '{actual}'")]
    CompatibilityMismatch {
        field: &'static str,
        expected: String,
        actual: String,
    },
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
