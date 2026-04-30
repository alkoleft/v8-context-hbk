use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum ContainerError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    InvalidContainer {
        path: PathBuf,
        message: String,
    },
    InvalidBlock {
        path: PathBuf,
        offset: usize,
        message: String,
    },
    InvalidDescriptor {
        path: PathBuf,
        offset: usize,
        message: String,
    },
    InvalidEntityName {
        path: PathBuf,
        offset: usize,
        source: std::string::FromUtf16Error,
    },
    MissingEntity {
        path: PathBuf,
        entity_name: String,
    },
    EntityHasNoBody {
        path: PathBuf,
        entity_name: String,
    },
}

impl fmt::Display for ContainerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "failed to read HBK file '{}': {source}", path.display())
            }
            Self::InvalidContainer { path, message } => {
                write!(f, "invalid HBK container '{}': {message}", path.display())
            }
            Self::InvalidBlock {
                path,
                offset,
                message,
            } => write!(
                f,
                "invalid HBK block in '{}' at offset {offset}: {message}",
                path.display()
            ),
            Self::InvalidDescriptor {
                path,
                offset,
                message,
            } => write!(
                f,
                "invalid HBK entity descriptor in '{}' at offset {offset}: {message}",
                path.display()
            ),
            Self::InvalidEntityName {
                path,
                offset,
                source,
            } => write!(
                f,
                "invalid HBK entity name in '{}' at offset {offset}: {source}",
                path.display()
            ),
            Self::MissingEntity { path, entity_name } => write!(
                f,
                "HBK entity '{entity_name}' is not present in '{}'",
                path.display()
            ),
            Self::EntityHasNoBody { path, entity_name } => write!(
                f,
                "HBK entity '{entity_name}' has no readable body in '{}'",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ContainerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidEntityName { source, .. } => Some(source),
            _ => None,
        }
    }
}
