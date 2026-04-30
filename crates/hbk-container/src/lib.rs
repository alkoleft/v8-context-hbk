use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use memmap2::{Mmap, MmapOptions};

const CONTAINER_HEADER_SIZE: usize = 16;
const FILE_DESCRIPTOR_SIZE: usize = 12;
const BLOCK_HEADER_SIZE: usize = 31;
const SPLITTER: u32 = i32::MAX as u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerHeader {
    pub free_block_offset: u32,
    pub default_block_size: u32,
    pub entity_count_hint: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockHeader {
    pub offset: usize,
    pub payload_size: usize,
    pub block_size: usize,
    pub next_block_offset: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityName(String);

impl EntityName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for EntityName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDescriptor {
    pub name: EntityName,
    pub descriptor_offset: usize,
    pub header_offset: usize,
    pub body_offset: Option<usize>,
}

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

#[derive(Debug)]
pub struct HbkContainer {
    path: PathBuf,
    data: ContainerData,
    header: ContainerHeader,
    descriptors: Vec<EntityDescriptor>,
    entity_offsets: BTreeMap<String, Option<usize>>,
}

#[derive(Debug)]
enum ContainerData {
    #[cfg(any(test, feature = "test-utils"))]
    Memory(Vec<u8>),
    Mapped(Mmap),
}

impl AsRef<[u8]> for ContainerData {
    fn as_ref(&self) -> &[u8] {
        match self {
            #[cfg(any(test, feature = "test-utils"))]
            Self::Memory(bytes) => bytes,
            Self::Mapped(map) => map,
        }
    }
}

impl HbkContainer {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ContainerError> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path).map_err(|source| ContainerError::Io {
            path: path.clone(),
            source,
        })?;
        // The map is read-only and tied to this container value; the file is not mutated here.
        let map =
            unsafe { MmapOptions::new().map(&file) }.map_err(|source| ContainerError::Io {
                path: path.clone(),
                source,
            })?;
        Self::from_data(path, ContainerData::Mapped(map))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn header(&self) -> ContainerHeader {
        self.header
    }

    pub fn descriptors(&self) -> &[EntityDescriptor] {
        &self.descriptors
    }

    pub fn entity_names(&self) -> impl Iterator<Item = &EntityName> {
        self.descriptors.iter().map(|descriptor| &descriptor.name)
    }

    pub fn read_entity(&self, name: &str) -> Result<Vec<u8>, ContainerError> {
        let offset = self.entity_offsets.get(name).copied().ok_or_else(|| {
            ContainerError::MissingEntity {
                path: self.path.clone(),
                entity_name: name.to_string(),
            }
        })?;
        let offset = offset.ok_or_else(|| ContainerError::EntityHasNoBody {
            path: self.path.clone(),
            entity_name: name.to_string(),
        })?;
        read_block_content(&self.path, self.data.as_ref(), offset)
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn from_bytes(path: PathBuf, bytes: Vec<u8>) -> Result<Self, ContainerError> {
        Self::from_data(path, ContainerData::Memory(bytes))
    }

    fn from_data(path: PathBuf, data: ContainerData) -> Result<Self, ContainerError> {
        let bytes = data.as_ref();
        let header = read_container_header(&path, bytes)?;
        let descriptors_block = read_block_content_with_offsets(
            &path,
            bytes,
            CONTAINER_HEADER_SIZE,
            Some(bytes.len()),
        )?;
        let descriptors_bytes = &descriptors_block.bytes;
        if descriptors_bytes.len() % FILE_DESCRIPTOR_SIZE != 0 {
            return Err(ContainerError::InvalidContainer {
                path,
                message: format!(
                    "descriptor block size {} is not divisible by {FILE_DESCRIPTOR_SIZE}",
                    descriptors_bytes.len()
                ),
            });
        }

        let mut descriptors = Vec::new();
        let mut entity_offsets = BTreeMap::new();
        for (index, chunk) in descriptors_bytes
            .chunks_exact(FILE_DESCRIPTOR_SIZE)
            .enumerate()
        {
            let descriptor_payload_offset = index * FILE_DESCRIPTOR_SIZE;
            let descriptor_offset = descriptors_block
                .source_offset(descriptor_payload_offset)
                .ok_or_else(|| ContainerError::InvalidDescriptor {
                    path: path.clone(),
                    offset: descriptor_payload_offset,
                    message: "descriptor source offset is unavailable".to_string(),
                })?;
            let header_offset = read_u32_le(chunk, 0) as usize;
            let body_offset_raw = read_u32_le(chunk, 4);
            let reserved = read_u32_le(chunk, 8);
            if reserved != SPLITTER {
                return Err(ContainerError::InvalidDescriptor {
                    path,
                    offset: descriptor_offset,
                    message: format!(
                        "reserved splitter is 0x{reserved:08x}, expected 0x{SPLITTER:08x}"
                    ),
                });
            }

            let name = read_entity_name(&path, bytes, header_offset)?;
            let body_offset = if body_offset_raw == SPLITTER {
                None
            } else {
                let offset = body_offset_raw as usize;
                ensure_offset(&path, bytes, offset, "entity body")?;
                Some(offset)
            };
            entity_offsets.insert(name.as_str().to_string(), body_offset);
            descriptors.push(EntityDescriptor {
                name,
                descriptor_offset,
                header_offset,
                body_offset,
            });
        }

        Ok(Self {
            path,
            data,
            header,
            descriptors,
            entity_offsets,
        })
    }
}

fn read_container_header(path: &Path, bytes: &[u8]) -> Result<ContainerHeader, ContainerError> {
    if bytes.len() < CONTAINER_HEADER_SIZE {
        return Err(ContainerError::InvalidContainer {
            path: path.to_path_buf(),
            message: format!(
                "file has {} bytes, expected at least {CONTAINER_HEADER_SIZE}",
                bytes.len()
            ),
        });
    }
    Ok(ContainerHeader {
        free_block_offset: read_u32_le(bytes, 0),
        default_block_size: read_u32_le(bytes, 4),
        entity_count_hint: read_u32_le(bytes, 8),
        reserved: read_u32_le(bytes, 12),
    })
}

fn read_entity_name(
    path: &Path,
    bytes: &[u8],
    offset: usize,
) -> Result<EntityName, ContainerError> {
    let block = read_block_content(path, bytes, offset)?;
    if block.len() < 24 {
        return Err(ContainerError::InvalidBlock {
            path: path.to_path_buf(),
            offset,
            message: format!(
                "entity header payload has {} bytes, expected at least 24",
                block.len()
            ),
        });
    }
    let name_bytes = &block[20..block.len() - 4];
    if name_bytes.len() % 2 != 0 {
        return Err(ContainerError::InvalidBlock {
            path: path.to_path_buf(),
            offset,
            message: "entity name byte length is not valid UTF-16LE".to_string(),
        });
    }

    let code_units = name_bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    String::from_utf16(&code_units)
        .map(EntityName)
        .map_err(|source| ContainerError::InvalidEntityName {
            path: path.to_path_buf(),
            offset,
            source,
        })
}

fn read_block_content(path: &Path, bytes: &[u8], offset: usize) -> Result<Vec<u8>, ContainerError> {
    Ok(read_block_content_impl(path, bytes, offset, Some(bytes.len()), SourceOffsets::Omit)?.bytes)
}

#[derive(Debug)]
struct BlockContent {
    bytes: Vec<u8>,
    source_offsets: Option<Vec<usize>>,
}

impl BlockContent {
    fn source_offset(&self, payload_offset: usize) -> Option<usize> {
        self.source_offsets.as_ref()?.get(payload_offset).copied()
    }
}

#[derive(Debug, Clone, Copy)]
enum SourceOffsets {
    Collect,
    Omit,
}

fn read_block_content_with_offsets(
    path: &Path,
    bytes: &[u8],
    offset: usize,
    max_payload_size: Option<usize>,
) -> Result<BlockContent, ContainerError> {
    read_block_content_impl(
        path,
        bytes,
        offset,
        max_payload_size,
        SourceOffsets::Collect,
    )
}

fn read_block_content_impl(
    path: &Path,
    bytes: &[u8],
    offset: usize,
    max_payload_size: Option<usize>,
    source_offset_mode: SourceOffsets,
) -> Result<BlockContent, ContainerError> {
    let first_header = read_block_header(path, bytes, offset)?;
    if max_payload_size.is_some_and(|max| first_header.payload_size > max) {
        return Err(ContainerError::InvalidBlock {
            path: path.to_path_buf(),
            offset: first_header.offset,
            message: format!(
                "declared payload size {} exceeds file size {}",
                first_header.payload_size,
                bytes.len()
            ),
        });
    }

    let mut output = Vec::with_capacity(first_header.payload_size);
    let mut source_offsets = match source_offset_mode {
        SourceOffsets::Collect => Some(Vec::with_capacity(first_header.payload_size)),
        SourceOffsets::Omit => None,
    };
    let mut written = 0;
    let mut header = first_header;
    let mut visited_offsets = BTreeSet::new();

    loop {
        if !visited_offsets.insert(header.offset) {
            return Err(ContainerError::InvalidBlock {
                path: path.to_path_buf(),
                offset: header.offset,
                message: "block chain contains a cycle".to_string(),
            });
        }
        let body_offset = header.offset + BLOCK_HEADER_SIZE;
        ensure_range(path, bytes, body_offset, header.block_size, "block body")?;
        let remaining = first_header.payload_size.saturating_sub(written);
        let chunk_len = header.block_size.min(remaining);
        if chunk_len == 0 && remaining > 0 {
            return Err(ContainerError::InvalidBlock {
                path: path.to_path_buf(),
                offset: header.offset,
                message: "block chain made no progress before payload was complete".to_string(),
            });
        }
        output.extend_from_slice(&bytes[body_offset..body_offset + chunk_len]);
        if let Some(source_offsets) = &mut source_offsets {
            source_offsets.extend(body_offset..body_offset + chunk_len);
        }
        written += chunk_len;

        match header.next_block_offset {
            Some(next_offset) if written < first_header.payload_size => {
                header = read_block_header(path, bytes, next_offset)?;
            }
            Some(_) => break,
            None => break,
        }
    }

    if written != first_header.payload_size {
        return Err(ContainerError::InvalidBlock {
            path: path.to_path_buf(),
            offset: first_header.offset,
            message: format!(
                "block chain ended after {written} bytes, expected {}",
                first_header.payload_size
            ),
        });
    }

    Ok(BlockContent {
        bytes: output,
        source_offsets,
    })
}

fn read_block_header(
    path: &Path,
    bytes: &[u8],
    offset: usize,
) -> Result<BlockHeader, ContainerError> {
    ensure_range(path, bytes, offset, BLOCK_HEADER_SIZE, "block header")?;
    let header = &bytes[offset..offset + BLOCK_HEADER_SIZE];
    if header[0..2] != [b'\r', b'\n'] || header[29..31] != [b'\r', b'\n'] {
        return Err(ContainerError::InvalidBlock {
            path: path.to_path_buf(),
            offset,
            message: "block header CRLF markers are invalid".to_string(),
        });
    }
    if header[10] != b' ' || header[19] != b' ' || header[28] != b' ' {
        return Err(ContainerError::InvalidBlock {
            path: path.to_path_buf(),
            offset,
            message: "block header separators are invalid".to_string(),
        });
    }
    let payload_size = read_hex_usize(path, offset, &header[2..10], "payload size")?;
    let block_size = read_hex_usize(path, offset, &header[11..19], "block size")?;
    let next_block = read_hex_u32(path, offset, &header[20..28], "next block")?;
    let next_block_offset = if next_block == SPLITTER {
        None
    } else {
        Some(next_block as usize)
    };
    Ok(BlockHeader {
        offset,
        payload_size,
        block_size,
        next_block_offset,
    })
}

fn read_hex_usize(
    path: &Path,
    offset: usize,
    bytes: &[u8],
    field: &str,
) -> Result<usize, ContainerError> {
    Ok(read_hex_u32(path, offset, bytes, field)? as usize)
}

fn read_hex_u32(
    path: &Path,
    offset: usize,
    bytes: &[u8],
    field: &str,
) -> Result<u32, ContainerError> {
    let value = std::str::from_utf8(bytes).map_err(|source| ContainerError::InvalidBlock {
        path: path.to_path_buf(),
        offset,
        message: format!("{field} is not UTF-8 hex text: {source}"),
    })?;
    u32::from_str_radix(value, 16).map_err(|source| ContainerError::InvalidBlock {
        path: path.to_path_buf(),
        offset,
        message: format!("{field} '{value}' is not valid hex: {source}"),
    })
}

fn ensure_offset(
    path: &Path,
    bytes: &[u8],
    offset: usize,
    label: &str,
) -> Result<(), ContainerError> {
    if offset >= bytes.len() {
        return Err(ContainerError::InvalidContainer {
            path: path.to_path_buf(),
            message: format!(
                "{label} offset {offset} is outside file size {}",
                bytes.len()
            ),
        });
    }
    Ok(())
}

fn ensure_range(
    path: &Path,
    bytes: &[u8],
    offset: usize,
    len: usize,
    label: &str,
) -> Result<(), ContainerError> {
    if offset.checked_add(len).is_none_or(|end| end > bytes.len()) {
        return Err(ContainerError::InvalidBlock {
            path: path.to_path_buf(),
            offset,
            message: format!("{label} length {len} is outside file size {}", bytes.len()),
        });
    }
    Ok(())
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_entity_names_and_bytes() {
        let container = HbkContainer::from_bytes(
            PathBuf::from("fixture.hbk"),
            fixture_container(vec![
                ("Book", Some(b"metadata".to_vec())),
                ("PackBlock", Some(b"toc".to_vec())),
            ]),
        )
        .expect("fixture must parse");

        let names = container
            .entity_names()
            .map(EntityName::as_str)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["Book", "PackBlock"]);
        assert_eq!(container.read_entity("Book").unwrap(), b"metadata");
    }

    #[test]
    fn reads_chained_entity_body() {
        let payload = b"0123456789abcdef".to_vec();
        let container = HbkContainer::from_bytes(
            PathBuf::from("fixture.hbk"),
            fixture_container(vec![("FileStorage", Some(payload.clone()))]),
        )
        .expect("fixture must parse");

        assert_eq!(container.read_entity("FileStorage").unwrap(), payload);
    }

    #[test]
    fn missing_entity_reports_path_and_name() {
        let container = HbkContainer::from_bytes(
            PathBuf::from("fixture.hbk"),
            fixture_container(vec![("Book", Some(b"metadata".to_vec()))]),
        )
        .expect("fixture must parse");

        let error = container.read_entity("Missing").unwrap_err();
        assert!(matches!(
            error,
            ContainerError::MissingEntity {
                ref path,
                ref entity_name
            } if path == Path::new("fixture.hbk") && entity_name == "Missing"
        ));
    }

    #[test]
    fn invalid_descriptor_splitter_is_typed_error() {
        let mut bytes = fixture_container(vec![
            ("Book", Some(b"metadata".to_vec())),
            ("FileStorage", Some(b"storage".to_vec())),
        ]);
        let descriptor_index = 1;
        let descriptor_reserved_offset =
            CONTAINER_HEADER_SIZE + BLOCK_HEADER_SIZE + descriptor_index * FILE_DESCRIPTOR_SIZE + 8;
        bytes[descriptor_reserved_offset..descriptor_reserved_offset + 4]
            .copy_from_slice(&0_u32.to_le_bytes());

        let error = HbkContainer::from_bytes(PathBuf::from("fixture.hbk"), bytes).unwrap_err();
        assert!(matches!(
            error,
            ContainerError::InvalidDescriptor { offset, .. }
                if offset == CONTAINER_HEADER_SIZE + BLOCK_HEADER_SIZE + descriptor_index * FILE_DESCRIPTOR_SIZE
        ));
    }

    #[test]
    fn enumerates_entities_without_body() {
        let container = HbkContainer::from_bytes(
            PathBuf::from("fixture.hbk"),
            fixture_container(vec![
                ("PackBlock", None),
                ("Book", Some(b"metadata".to_vec())),
            ]),
        )
        .expect("fixture must parse");

        let names = container
            .entity_names()
            .map(EntityName::as_str)
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["PackBlock", "Book"]);
        assert!(matches!(
            container.read_entity("PackBlock").unwrap_err(),
            ContainerError::EntityHasNoBody { .. }
        ));
        assert_eq!(container.descriptors()[0].body_offset, None);
    }

    #[test]
    fn oversized_declared_payload_is_typed_error() {
        let mut bytes = fixture_container(vec![("Book", Some(b"metadata".to_vec()))]);
        bytes[CONTAINER_HEADER_SIZE + 2..CONTAINER_HEADER_SIZE + 10].copy_from_slice(b"ffffffff");

        let error = HbkContainer::from_bytes(PathBuf::from("fixture.hbk"), bytes).unwrap_err();
        assert!(matches!(error, ContainerError::InvalidBlock { .. }));
    }

    #[test]
    fn cyclic_block_chain_is_typed_error() {
        let mut bytes = fixture_container(vec![("Book", Some(b"0123456789abcdef".to_vec()))]);
        let body_offset = descriptor_body_offset(&bytes, 0);
        patch_next_block(&mut bytes, body_offset, body_offset);

        let container = HbkContainer::from_bytes(PathBuf::from("fixture.hbk"), bytes)
            .expect("container metadata must parse");
        let error = container.read_entity("Book").unwrap_err();
        assert!(matches!(error, ContainerError::InvalidBlock { .. }));
    }

    #[test]
    fn platform_fmtdui_smoke_when_enabled() {
        if std::env::var_os("V8_CONTEXT_HBK_REAL_SMOKE").is_none() {
            return;
        }

        for path in [
            "/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk",
            "/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk",
        ] {
            let container = HbkContainer::open(path).expect("platform HBK must open");
            assert!(
                container
                    .descriptors()
                    .iter()
                    .any(|descriptor| descriptor.name.as_str() == "PackBlock"),
                "{path} must enumerate PackBlock"
            );
            assert!(
                container
                    .descriptors()
                    .iter()
                    .any(|descriptor| descriptor.name.as_str() == "Book"),
                "{path} must contain Book"
            );
            assert!(
                container
                    .descriptors()
                    .iter()
                    .any(|descriptor| descriptor.name.as_str() == "FileStorage"),
                "{path} must contain FileStorage"
            );
            let book = container
                .read_entity("Book")
                .expect("Book must be readable");
            assert!(!book.is_empty());
            std::str::from_utf8(&book).expect("Book must be UTF-8 metadata");
        }
    }

    fn fixture_container(entities: Vec<(&str, Option<Vec<u8>>)>) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&SPLITTER.to_le_bytes());
        bytes.extend_from_slice(&512_u32.to_le_bytes());
        bytes.extend_from_slice(&(entities.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());

        let descriptor_payload_size = entities.len() * FILE_DESCRIPTOR_SIZE;
        let descriptor_block_offset = bytes.len();
        push_block(
            &mut bytes,
            descriptor_payload_size,
            &vec![0; descriptor_payload_size],
            None,
        );

        let mut descriptors = Vec::new();
        for (name, body) in entities {
            let header_offset = bytes.len() as u32;
            push_block(
                &mut bytes,
                entity_header_payload(name).len(),
                &entity_header_payload(name),
                None,
            );

            let body_offset = if let Some(body) = body {
                let body_offset = bytes.len() as u32;
                push_chained_body(&mut bytes, &body);
                body_offset
            } else {
                SPLITTER
            };

            descriptors.extend_from_slice(&header_offset.to_le_bytes());
            descriptors.extend_from_slice(&body_offset.to_le_bytes());
            descriptors.extend_from_slice(&SPLITTER.to_le_bytes());
        }

        let descriptor_body_offset = descriptor_block_offset + BLOCK_HEADER_SIZE;
        bytes[descriptor_body_offset..descriptor_body_offset + descriptors.len()]
            .copy_from_slice(&descriptors);
        bytes
    }

    fn entity_header_payload(name: &str) -> Vec<u8> {
        let mut payload = vec![0; 20];
        for code_unit in name.encode_utf16() {
            payload.extend_from_slice(&code_unit.to_le_bytes());
        }
        payload.extend_from_slice(&[0; 4]);
        payload
    }

    fn push_chained_body(bytes: &mut Vec<u8>, body: &[u8]) {
        if body.len() <= 8 {
            push_block(bytes, body.len(), body, None);
            return;
        }

        let first_chunk = &body[..8];
        let second_chunk = &body[8..];
        let first_offset = bytes.len();
        push_block(bytes, body.len(), first_chunk, Some(0));
        let second_offset = bytes.len();
        patch_next_block(bytes, first_offset, second_offset);
        push_block(bytes, body.len(), second_chunk, None);
    }

    fn push_block(bytes: &mut Vec<u8>, payload_size: usize, chunk: &[u8], next: Option<usize>) {
        bytes.extend_from_slice(
            format!(
                "\r\n{payload_size:08x} {block_size:08x} {next:08x} \r\n",
                block_size = chunk.len(),
                next = next.unwrap_or(SPLITTER as usize)
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(chunk);
    }

    fn patch_next_block(bytes: &mut [u8], block_offset: usize, next_offset: usize) {
        let start = block_offset + 20;
        bytes[start..start + 8].copy_from_slice(format!("{next_offset:08x}").as_bytes());
    }

    fn descriptor_body_offset(bytes: &[u8], index: usize) -> usize {
        let descriptor_offset =
            CONTAINER_HEADER_SIZE + BLOCK_HEADER_SIZE + index * FILE_DESCRIPTOR_SIZE;
        read_u32_le(bytes, descriptor_offset + 4) as usize
    }
}
