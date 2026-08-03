use std::io::Read;

use sha2::{Digest, Sha256};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HbkFactSnapshotBuildInputIdentity {
    pub(super) provider_schema_version: u32,
    source_index_bytes: u64,
    pub(super) provider_sha256: String,
    locale: String,
    source_locale: String,
    source_hbk: String,
    source_hbk_sha256: Option<String>,
    pub(super) source_extraction_schema_version: u32,
}

impl HbkFactSnapshotBuildInputIdentity {
    pub(super) fn from_index(path: &Path, index: &SearchIndex) -> Result<Self, SearchError> {
        let metadata = index.metadata()?;
        let file_metadata = fs::metadata(path).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let source_index_bytes = file_metadata.len();
        let provider_sha256 = file_sha256(path).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let source_hbk_path = Path::new(&metadata.source_hbk);
        let source_hbk_sha256 = match file_sha256(source_hbk_path) {
            Ok(digest) => Some(digest),
            Err(source) if source.kind() == io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(SearchError::Io {
                    path: source_hbk_path.to_path_buf(),
                    source,
                });
            }
        };
        Ok(Self {
            provider_schema_version: INDEX_SCHEMA_VERSION,
            source_index_bytes,
            provider_sha256,
            locale: metadata.locale,
            source_locale: metadata.source_locale,
            source_hbk: metadata.source_hbk,
            source_hbk_sha256,
            source_extraction_schema_version: metadata.source_extraction_schema_version,
        })
    }
}

pub(super) fn file_sha256(path: impl AsRef<Path>) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
