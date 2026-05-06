use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ExportError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    f,
                    "failed to write JSON export '{}': {source}",
                    path.display()
                )
            }
            Self::Json { path, source } => {
                write!(
                    f,
                    "failed to serialize JSON export '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
        }
    }
}
