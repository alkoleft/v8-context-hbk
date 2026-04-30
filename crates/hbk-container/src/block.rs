use std::collections::BTreeSet;
use std::path::Path;

use crate::error::ContainerError;
use crate::types::{BlockHeader, ContainerHeader};

pub(crate) const CONTAINER_HEADER_SIZE: usize = 16;
pub(crate) const FILE_DESCRIPTOR_SIZE: usize = 12;
pub(crate) const BLOCK_HEADER_SIZE: usize = 31;
pub(crate) const SPLITTER: u32 = i32::MAX as u32;

pub(crate) fn read_container_header(
    path: &Path,
    bytes: &[u8],
) -> Result<ContainerHeader, ContainerError> {
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

pub(crate) fn read_block_content(
    path: &Path,
    bytes: &[u8],
    offset: usize,
) -> Result<Vec<u8>, ContainerError> {
    Ok(read_block_content_impl(path, bytes, offset, Some(bytes.len()), SourceOffsets::Omit)?.bytes)
}

#[derive(Debug)]
pub(crate) struct BlockContent {
    pub(crate) bytes: Vec<u8>,
    source_offsets: Option<Vec<usize>>,
}

impl BlockContent {
    pub(crate) fn source_offset(&self, payload_offset: usize) -> Option<usize> {
        self.source_offsets.as_ref()?.get(payload_offset).copied()
    }
}

#[derive(Debug, Clone, Copy)]
enum SourceOffsets {
    Collect,
    Omit,
}

pub(crate) fn read_block_content_with_offsets(
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

pub(crate) fn ensure_offset(
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

pub(crate) fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
