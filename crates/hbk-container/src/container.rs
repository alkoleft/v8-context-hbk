use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use memmap2::{Mmap, MmapOptions};

use crate::block::{
    CONTAINER_HEADER_SIZE, FILE_DESCRIPTOR_SIZE, SPLITTER, ensure_offset, read_block_content,
    read_block_content_with_offsets, read_container_header, read_u32_le,
};
use crate::error::ContainerError;
use crate::types::{ContainerHeader, EntityDescriptor, EntityName};

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
        .map(EntityName::from)
        .map_err(|source| ContainerError::InvalidEntityName {
            path: path.to_path_buf(),
            offset,
            source,
        })
}
