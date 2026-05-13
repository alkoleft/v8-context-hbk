use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("failed to read HBK file '{}': {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid HBK container '{}': {message}", path.display())]
    InvalidContainer { path: PathBuf, message: String },
    #[error("invalid HBK block in '{}' at offset {offset}: {message}", path.display())]
    InvalidBlock {
        path: PathBuf,
        offset: usize,
        message: String,
    },
    #[error("invalid HBK entity descriptor in '{}' at offset {offset}: {message}", path.display())]
    InvalidDescriptor {
        path: PathBuf,
        offset: usize,
        message: String,
    },
    #[error("invalid HBK entity name in '{}' at offset {offset}: {source}", path.display())]
    InvalidEntityName {
        path: PathBuf,
        offset: usize,
        #[source]
        source: std::string::FromUtf16Error,
    },
    #[error("HBK entity '{entity_name}' is not present in '{}'", path.display())]
    MissingEntity { path: PathBuf, entity_name: String },
    #[error("HBK entity '{entity_name}' has no readable body in '{}'", path.display())]
    EntityHasNoBody { path: PathBuf, entity_name: String },
}
