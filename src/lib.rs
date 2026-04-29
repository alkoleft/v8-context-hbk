pub mod hbk {
    pub mod container {
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
            #[cfg(test)]
            Memory(Vec<u8>),
            Mapped(Mmap),
        }

        impl AsRef<[u8]> for ContainerData {
            fn as_ref(&self) -> &[u8] {
                match self {
                    #[cfg(test)]
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
                let map = unsafe { MmapOptions::new().map(&file) }.map_err(|source| {
                    ContainerError::Io {
                        path: path.clone(),
                        source,
                    }
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

            #[cfg(test)]
            pub(crate) fn from_bytes(
                path: PathBuf,
                bytes: Vec<u8>,
            ) -> Result<Self, ContainerError> {
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
                        .unwrap_or(descriptor_payload_offset);
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

        fn read_container_header(
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

        fn read_block_content(
            path: &Path,
            bytes: &[u8],
            offset: usize,
        ) -> Result<Vec<u8>, ContainerError> {
            Ok(read_block_content_with_offsets(path, bytes, offset, Some(bytes.len()))?.bytes)
        }

        #[derive(Debug)]
        struct BlockContent {
            bytes: Vec<u8>,
            source_offsets: Vec<usize>,
        }

        impl BlockContent {
            fn source_offset(&self, payload_offset: usize) -> Option<usize> {
                self.source_offsets.get(payload_offset).copied()
            }
        }

        fn read_block_content_with_offsets(
            path: &Path,
            bytes: &[u8],
            offset: usize,
            max_payload_size: Option<usize>,
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
            let mut source_offsets = Vec::with_capacity(first_header.payload_size);
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
                        message: "block chain made no progress before payload was complete"
                            .to_string(),
                    });
                }
                output.extend_from_slice(&bytes[body_offset..body_offset + chunk_len]);
                source_offsets.extend(body_offset..body_offset + chunk_len);
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
            let value =
                std::str::from_utf8(bytes).map_err(|source| ContainerError::InvalidBlock {
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
                let mut bytes = fixture_container(vec![("Book", Some(b"metadata".to_vec()))]);
                let descriptor_reserved_offset = CONTAINER_HEADER_SIZE + BLOCK_HEADER_SIZE + 8;
                bytes[descriptor_reserved_offset..descriptor_reserved_offset + 4]
                    .copy_from_slice(&0_u32.to_le_bytes());

                let error =
                    HbkContainer::from_bytes(PathBuf::from("fixture.hbk"), bytes).unwrap_err();
                assert!(matches!(error, ContainerError::InvalidDescriptor { .. }));
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
                bytes[CONTAINER_HEADER_SIZE + 2..CONTAINER_HEADER_SIZE + 10]
                    .copy_from_slice(b"ffffffff");

                let error =
                    HbkContainer::from_bytes(PathBuf::from("fixture.hbk"), bytes).unwrap_err();
                assert!(matches!(error, ContainerError::InvalidBlock { .. }));
            }

            #[test]
            fn cyclic_block_chain_is_typed_error() {
                let mut bytes =
                    fixture_container(vec![("Book", Some(b"0123456789abcdef".to_vec()))]);
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

            fn push_block(
                bytes: &mut Vec<u8>,
                payload_size: usize,
                chunk: &[u8],
                next: Option<usize>,
            ) {
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
    }

    pub(crate) mod tokens {
        use std::fmt;

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub(crate) struct TokenError {
            message: String,
        }

        impl TokenError {
            fn new(message: impl Into<String>) -> Self {
                Self {
                    message: message.into(),
                }
            }
        }

        impl fmt::Display for TokenError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.message)
            }
        }

        impl std::error::Error for TokenError {}

        pub(crate) fn tokenize(content: &str) -> Vec<String> {
            const BOM: char = '\u{feff}';
            let mut tokens = Vec::new();
            let mut current = String::new();
            let mut chars = content.chars().peekable();
            let mut in_string = false;

            while let Some(ch) = chars.next() {
                match ch {
                    BOM => {}
                    '"' if in_string => {
                        if chars.peek() == Some(&'"') {
                            current.push('"');
                            chars.next();
                        } else {
                            current.push(ch);
                            tokens.push(std::mem::take(&mut current));
                            in_string = false;
                        }
                    }
                    '"' => {
                        push_token(&mut tokens, &mut current);
                        current.push(ch);
                        in_string = true;
                    }
                    _ if in_string => current.push(ch),
                    ch if ch.is_whitespace() => push_token(&mut tokens, &mut current),
                    '{' | '}' | ',' => {
                        push_token(&mut tokens, &mut current);
                        if ch != ',' {
                            tokens.push(ch.to_string());
                        }
                    }
                    _ => current.push(ch),
                }
            }
            push_token(&mut tokens, &mut current);
            tokens
        }

        fn push_token(tokens: &mut Vec<String>, current: &mut String) {
            let token = current.trim();
            if !token.is_empty() {
                tokens.push(token.to_string());
            }
            current.clear();
        }

        pub(crate) struct TokenParser {
            tokens: Vec<String>,
            index: usize,
        }

        impl TokenParser {
            pub(crate) fn new(tokens: Vec<String>) -> Self {
                Self { tokens, index: 0 }
            }

            pub(crate) fn peek(&self) -> Option<&str> {
                self.tokens.get(self.index).map(String::as_str)
            }

            pub(crate) fn next(&mut self, context: impl AsRef<str>) -> Result<String, TokenError> {
                let token = self.tokens.get(self.index).cloned().ok_or_else(|| {
                    TokenError::new(format!("{}: unexpected end of input", context.as_ref()))
                })?;
                self.index += 1;
                Ok(token)
            }

            pub(crate) fn expect(
                &mut self,
                expected: &str,
                context: impl AsRef<str>,
            ) -> Result<(), TokenError> {
                let token = self.next(context.as_ref())?;
                if token != expected {
                    return Err(TokenError::new(format!(
                        "{}: expected '{expected}', got '{token}'",
                        context.as_ref()
                    )));
                }
                Ok(())
            }

            pub(crate) fn number(&mut self, context: impl AsRef<str>) -> Result<usize, TokenError> {
                let token = self.next(context.as_ref())?;
                token.parse::<usize>().map_err(|source| {
                    TokenError::new(format!(
                        "{}: expected number, got '{token}': {source}",
                        context.as_ref()
                    ))
                })
            }

            pub(crate) fn string(
                &mut self,
                context: impl AsRef<str>,
            ) -> Result<String, TokenError> {
                let token = self.next(context.as_ref())?;
                if !token.starts_with('"') || !token.ends_with('"') {
                    return Err(TokenError::new(format!(
                        "{}: expected string, got '{token}'",
                        context.as_ref()
                    )));
                }
                Ok(token[1..token.len() - 1].to_string())
            }

            pub(crate) fn expect_end(&self, context: &str) -> Result<(), TokenError> {
                if self.index == self.tokens.len() {
                    Ok(())
                } else {
                    Err(TokenError::new(format!(
                        "{context}: unexpected trailing token '{}'",
                        self.tokens[self.index]
                    )))
                }
            }
        }
    }

    pub mod book {
        use std::collections::{BTreeMap, BTreeSet};
        use std::fmt;
        use std::io::{self, Cursor, Read};
        use std::path::{Path, PathBuf};

        use zip::ZipArchive;

        use super::container::{ContainerError, HbkContainer};
        use super::toc::{Toc, TocError};
        use super::tokens::{TokenError, TokenParser, tokenize};

        const PACK_BLOCK_NAME: &str = "PackBlock";
        const FILE_STORAGE_NAME: &str = "FileStorage";
        const BOOK_NAME: &str = "Book";

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum BookEntityKind {
            Book,
            PackBlock,
            FileStorage,
        }

        impl BookEntityKind {
            pub fn entity_name(self) -> &'static str {
                match self {
                    Self::Book => BOOK_NAME,
                    Self::PackBlock => PACK_BLOCK_NAME,
                    Self::FileStorage => FILE_STORAGE_NAME,
                }
            }
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct BookMeta {
            pub book_name: String,
            pub description: String,
            pub tags: Vec<String>,
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum BookLocale {
            Ru,
            Root,
            Other(String),
        }

        impl BookLocale {
            pub fn infer_from_path(path: &Path) -> Self {
                let stem = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default();
                if stem.ends_with("_ru") {
                    Self::Ru
                } else if stem.ends_with("_root") {
                    Self::Root
                } else if let Some((_, suffix)) = stem.rsplit_once('_') {
                    Self::Other(suffix.to_string())
                } else {
                    Self::Other(String::new())
                }
            }

            pub fn source_code(&self) -> &str {
                match self {
                    Self::Ru => "ru",
                    Self::Root => "root",
                    Self::Other(value) => value,
                }
            }

            pub fn export_code(&self) -> &str {
                match self {
                    Self::Root => "en",
                    _ => self.source_code(),
                }
            }
        }

        #[derive(Debug)]
        pub enum BookError {
            Container(ContainerError),
            InvalidUtf8 {
                path: PathBuf,
                entity_name: &'static str,
                source: std::string::FromUtf8Error,
            },
            InvalidZip {
                path: PathBuf,
                entity_name: &'static str,
                source: zip::result::ZipError,
            },
            Io {
                path: PathBuf,
                entry_name: String,
                source: io::Error,
            },
            MissingZipEntry {
                path: PathBuf,
                entry_name: String,
            },
            Meta(MetaError),
            Toc(TocError),
        }

        impl fmt::Display for BookError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Self::Container(source) => write!(f, "{source}"),
                    Self::InvalidUtf8 {
                        path,
                        entity_name,
                        source,
                    } => write!(
                        f,
                        "HBK entity '{entity_name}' in '{}' is not UTF-8: {source}",
                        path.display()
                    ),
                    Self::InvalidZip {
                        path,
                        entity_name,
                        source,
                    } => write!(
                        f,
                        "HBK entity '{entity_name}' in '{}' is not a readable ZIP stream: {source}",
                        path.display()
                    ),
                    Self::Io {
                        path,
                        entry_name,
                        source,
                    } => write!(
                        f,
                        "failed to read ZIP entry '{entry_name}' from '{}': {source}",
                        path.display()
                    ),
                    Self::MissingZipEntry { path, entry_name } => write!(
                        f,
                        "ZIP entry '{entry_name}' is not present in HBK FileStorage '{}'",
                        path.display()
                    ),
                    Self::Meta(source) => write!(f, "{source}"),
                    Self::Toc(source) => write!(f, "{source}"),
                }
            }
        }

        impl std::error::Error for BookError {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                match self {
                    Self::Container(source) => Some(source),
                    Self::InvalidUtf8 { source, .. } => Some(source),
                    Self::InvalidZip { source, .. } => Some(source),
                    Self::Io { source, .. } => Some(source),
                    Self::Meta(source) => Some(source),
                    Self::Toc(source) => Some(source),
                    Self::MissingZipEntry { .. } => None,
                }
            }
        }

        impl From<ContainerError> for BookError {
            fn from(value: ContainerError) -> Self {
                Self::Container(value)
            }
        }

        impl From<MetaError> for BookError {
            fn from(value: MetaError) -> Self {
                Self::Meta(value)
            }
        }

        impl From<TocError> for BookError {
            fn from(value: TocError) -> Self {
                Self::Toc(value)
            }
        }

        #[derive(Debug)]
        pub struct HbkBook {
            container: HbkContainer,
            meta: BookMeta,
            locale: BookLocale,
            toc: Toc,
            file_storage: Vec<u8>,
        }

        impl HbkBook {
            pub fn open(path: impl AsRef<Path>) -> Result<Self, BookError> {
                Self::from_container(HbkContainer::open(path)?)
            }

            pub fn path(&self) -> &Path {
                self.container.path()
            }

            pub fn meta(&self) -> &BookMeta {
                &self.meta
            }

            pub fn locale(&self) -> &BookLocale {
                &self.locale
            }

            pub fn toc(&self) -> &Toc {
                &self.toc
            }

            pub fn read_file(&self, path: &str) -> Result<Vec<u8>, BookError> {
                let entry_name = normalize_storage_path(path).to_string();
                if entry_name.is_empty() {
                    return Err(BookError::MissingZipEntry {
                        path: self.path().to_path_buf(),
                        entry_name,
                    });
                }
                let mut archive = ZipArchive::new(Cursor::new(self.file_storage.as_slice()))
                    .map_err(|source| BookError::InvalidZip {
                        path: self.path().to_path_buf(),
                        entity_name: FILE_STORAGE_NAME,
                        source,
                    })?;
                let mut entry = archive
                    .by_name(&entry_name)
                    .map_err(|source| match source {
                        zip::result::ZipError::FileNotFound => BookError::MissingZipEntry {
                            path: self.path().to_path_buf(),
                            entry_name: entry_name.clone(),
                        },
                        source => BookError::InvalidZip {
                            path: self.path().to_path_buf(),
                            entity_name: FILE_STORAGE_NAME,
                            source,
                        },
                    })?;
                let mut bytes = Vec::new();
                entry
                    .read_to_end(&mut bytes)
                    .map_err(|source| BookError::Io {
                        path: self.path().to_path_buf(),
                        entry_name,
                        source,
                    })?;
                Ok(bytes)
            }

            pub fn read_page(&self, path: &str) -> Result<String, BookError> {
                let bytes = self.read_file(path)?;
                String::from_utf8(bytes).map_err(|source| BookError::InvalidUtf8 {
                    path: self.path().to_path_buf(),
                    entity_name: FILE_STORAGE_NAME,
                    source,
                })
            }

            pub fn read_pages<'p>(
                &self,
                paths: impl IntoIterator<Item = &'p str>,
            ) -> Result<BTreeMap<String, String>, BookError> {
                let mut requested = BTreeSet::new();
                for path in paths {
                    let entry_name = normalize_storage_path(path).to_string();
                    if entry_name.is_empty() {
                        return Err(BookError::MissingZipEntry {
                            path: self.path().to_path_buf(),
                            entry_name,
                        });
                    }
                    requested.insert(entry_name);
                }
                let mut archive = ZipArchive::new(Cursor::new(self.file_storage.as_slice()))
                    .map_err(|source| BookError::InvalidZip {
                        path: self.path().to_path_buf(),
                        entity_name: FILE_STORAGE_NAME,
                        source,
                    })?;
                let mut pages = BTreeMap::new();
                for index in 0..archive.len() {
                    let mut entry =
                        archive
                            .by_index(index)
                            .map_err(|source| BookError::InvalidZip {
                                path: self.path().to_path_buf(),
                                entity_name: FILE_STORAGE_NAME,
                                source,
                            })?;
                    let entry_name = entry.name().to_string();
                    if !requested.contains(&entry_name) {
                        continue;
                    }
                    let mut bytes = Vec::new();
                    entry
                        .read_to_end(&mut bytes)
                        .map_err(|source| BookError::Io {
                            path: self.path().to_path_buf(),
                            entry_name: entry_name.clone(),
                            source,
                        })?;
                    let page =
                        String::from_utf8(bytes).map_err(|source| BookError::InvalidUtf8 {
                            path: self.path().to_path_buf(),
                            entity_name: FILE_STORAGE_NAME,
                            source,
                        })?;
                    pages.insert(entry_name, page);
                    if pages.len() == requested.len() {
                        break;
                    }
                }
                if let Some(entry_name) = requested
                    .iter()
                    .find(|entry_name| !pages.contains_key(*entry_name))
                {
                    return Err(BookError::MissingZipEntry {
                        path: self.path().to_path_buf(),
                        entry_name: entry_name.clone(),
                    });
                }
                Ok(pages)
            }

            fn from_container(container: HbkContainer) -> Result<Self, BookError> {
                let path = container.path().to_path_buf();
                let meta_text = entity_utf8(&container, BookEntityKind::Book)?;
                let meta = parse_book_meta(&meta_text)?;
                let file_storage =
                    container.read_entity(BookEntityKind::FileStorage.entity_name())?;
                let toc = match container.read_entity(BookEntityKind::PackBlock.entity_name()) {
                    Ok(pack_block) => {
                        let toc_bytes = read_first_zip_entry(&path, PACK_BLOCK_NAME, &pack_block)?;
                        let toc_text = String::from_utf8(toc_bytes).map_err(|source| {
                            BookError::InvalidUtf8 {
                                path: path.clone(),
                                entity_name: PACK_BLOCK_NAME,
                                source,
                            }
                        })?;
                        Toc::parse(&toc_text)?
                    }
                    Err(ContainerError::EntityHasNoBody { entity_name, .. })
                        if entity_name == PACK_BLOCK_NAME =>
                    {
                        Toc::from_storage_paths(list_storage_page_paths(&path, &file_storage)?)
                    }
                    Err(source) => return Err(BookError::Container(source)),
                };
                Ok(Self {
                    locale: BookLocale::infer_from_path(container.path()),
                    container,
                    meta,
                    toc,
                    file_storage,
                })
            }

            #[cfg(test)]
            fn from_bytes(path: PathBuf, bytes: Vec<u8>) -> Result<Self, BookError> {
                Self::from_container(HbkContainer::from_bytes(path, bytes)?)
            }
        }

        fn entity_utf8(
            container: &HbkContainer,
            entity_kind: BookEntityKind,
        ) -> Result<String, BookError> {
            let entity_name = entity_kind.entity_name();
            let bytes = container.read_entity(entity_name)?;
            String::from_utf8(bytes).map_err(|source| BookError::InvalidUtf8 {
                path: container.path().to_path_buf(),
                entity_name,
                source,
            })
        }

        fn read_first_zip_entry(
            path: &Path,
            entity_name: &'static str,
            bytes: &[u8],
        ) -> Result<Vec<u8>, BookError> {
            let mut archive =
                ZipArchive::new(Cursor::new(bytes)).map_err(|source| BookError::InvalidZip {
                    path: path.to_path_buf(),
                    entity_name,
                    source,
                })?;
            let mut entry = archive
                .by_index(0)
                .map_err(|source| BookError::InvalidZip {
                    path: path.to_path_buf(),
                    entity_name,
                    source,
                })?;
            let mut output = Vec::new();
            entry
                .read_to_end(&mut output)
                .map_err(|source| BookError::Io {
                    path: path.to_path_buf(),
                    entry_name: entry.name().to_string(),
                    source,
                })?;
            Ok(output)
        }

        fn list_storage_page_paths(path: &Path, bytes: &[u8]) -> Result<Vec<String>, BookError> {
            let mut archive =
                ZipArchive::new(Cursor::new(bytes)).map_err(|source| BookError::InvalidZip {
                    path: path.to_path_buf(),
                    entity_name: FILE_STORAGE_NAME,
                    source,
                })?;
            let mut paths = Vec::new();
            for index in 0..archive.len() {
                let entry = archive
                    .by_index(index)
                    .map_err(|source| BookError::InvalidZip {
                        path: path.to_path_buf(),
                        entity_name: FILE_STORAGE_NAME,
                        source,
                    })?;
                let name = entry.name();
                if !entry.is_dir() && !name.starts_with("__") {
                    paths.push(name.trim_start_matches('/').to_string());
                }
            }
            Ok(paths)
        }

        pub fn normalize_storage_path(path: &str) -> &str {
            path.trim_start_matches('/')
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct MetaError {
            message: String,
        }

        impl MetaError {
            fn new(message: impl Into<String>) -> Self {
                Self {
                    message: message.into(),
                }
            }
        }

        impl fmt::Display for MetaError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "invalid HBK Book metadata: {}", self.message)
            }
        }

        impl std::error::Error for MetaError {}

        impl From<TokenError> for MetaError {
            fn from(value: TokenError) -> Self {
                Self::new(value.to_string())
            }
        }

        pub fn parse_book_meta(content: &str) -> Result<BookMeta, MetaError> {
            let tokens = tokenize(content);
            let mut parser = TokenParser::new(tokens);
            parser.expect("{", "PageDescription: expected '{'")?;
            parser.number("PageDescription: expected type")?;
            let book_name = parser.string("PageDescription: expected bookName")?;
            let description = parse_file_name(&mut parser)?;
            let tag_count = parser.number("PageDescription: expected tagCount")?;
            let mut tags = Vec::new();
            for index in 0..tag_count {
                tags.push(parser.string(format!("Tags: expected tag #{}", index + 1))?);
                parse_number_pair(&mut parser)?;
            }
            let trailing_zero = parser.number("PageDescription: expected trailing zero")?;
            if trailing_zero != 0 {
                return Err(MetaError::new(format!(
                    "PageDescription: expected trailing zero, got {trailing_zero}"
                )));
            }
            parser.expect("}", "PageDescription: expected closing '}'")?;
            parser.expect_end("PageDescription")?;
            Ok(BookMeta {
                book_name,
                description,
                tags,
            })
        }

        fn parse_file_name(parser: &mut TokenParser) -> Result<String, MetaError> {
            parser.expect("{", "FileName: expected '{'")?;
            parser.number("FileName: expected first number")?;
            parser.number("FileName: expected second number")?;
            parser.expect("{", "FileName: expected name object '{'")?;
            parser.string("FileName: expected language")?;
            let file_name = parser.string("FileName: expected fileName")?;
            parser.expect("}", "FileName: expected name object closing '}'")?;
            parser.expect("}", "FileName: expected closing '}'")?;
            Ok(file_name)
        }

        fn parse_number_pair(parser: &mut TokenParser) -> Result<(), MetaError> {
            parser.expect("{", "NumberPair: expected '{'")?;
            parser.number("NumberPair: expected first number")?;
            parser.number("NumberPair: expected second number")?;
            parser.expect("}", "NumberPair: expected closing '}'")?;
            Ok(())
        }

        #[cfg(test)]
        mod tests {
            use super::*;
            use crate::hbk::docs::DocumentationReader;
            use std::io::Cursor;
            use std::io::Write;
            use zip::write::SimpleFileOptions;
            use zip::{CompressionMethod, ZipWriter};

            #[test]
            fn infers_source_and_export_locale() {
                assert_eq!(
                    BookLocale::infer_from_path(Path::new("fmtdui_ru.hbk")).source_code(),
                    "ru"
                );
                let root = BookLocale::infer_from_path(Path::new("fmtdui_root.hbk"));
                assert_eq!(root.source_code(), "root");
                assert_eq!(root.export_code(), "en");
            }

            #[test]
            fn exposes_book_entity_names() {
                assert_eq!(BookEntityKind::Book.entity_name(), "Book");
                assert_eq!(BookEntityKind::PackBlock.entity_name(), "PackBlock");
                assert_eq!(BookEntityKind::FileStorage.entity_name(), "FileStorage");
            }

            #[test]
            fn parses_book_metadata() {
                let meta = parse_book_meta(
                    r#"{1,"Interface", {1,2,{"en","fmtdui"}}, 2, "tag1", {0,0}, "tag2", {0,0}, 0}"#,
                )
                .expect("metadata must parse");
                assert_eq!(meta.book_name, "Interface");
                assert_eq!(meta.description, "fmtdui");
                assert_eq!(meta.tags, vec!["tag1", "tag2"]);
            }

            #[test]
            fn normalizes_storage_paths() {
                assert_eq!(normalize_storage_path("/docs/page.html"), "docs/page.html");
                assert_eq!(normalize_storage_path("docs/page.html"), "docs/page.html");
            }

            #[test]
            fn opens_book_toc_and_page_from_container_entities() {
                let toc = r#"{
                    1
                    {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/docs/page.html"}}
                }"#;
                let book = HbkBook::from_bytes(
                    PathBuf::from("fmtdui_ru.hbk"),
                    fixture_container(vec![
                        (
                            "Book",
                            Some(
                                r#"{1,"Interface", {1,2,{"ru","fmtdui"}}, 1, "tag", {0,0}, 0}"#
                                    .as_bytes()
                                    .to_vec(),
                            ),
                        ),
                        ("PackBlock", Some(zip_bytes("toc.txt", toc.as_bytes()))),
                        (
                            "FileStorage",
                            Some(zip_bytes("docs/page.html", b"<html>page</html>")),
                        ),
                    ]),
                )
                .expect("book must open");

                assert_eq!(book.locale().source_code(), "ru");
                assert_eq!(book.meta().book_name, "Interface");
                assert_eq!(
                    book.toc()
                        .find_by_html_path("/docs/page.html")
                        .unwrap()
                        .title
                        .display(),
                    "Страница"
                );
                assert_eq!(
                    book.read_page("/docs/page.html").unwrap(),
                    "<html>page</html>"
                );
            }

            #[test]
            fn opens_book_with_storage_toc_when_pack_block_has_no_body() {
                let book = HbkBook::from_bytes(
                    PathBuf::from("fmtdui_ru.hbk"),
                    fixture_container(vec![
                        (
                            "Book",
                            Some(
                                r#"{1,"Interface", {1,2,{"ru","fmtdui"}}, 1, "tag", {0,0}, 0}"#
                                    .as_bytes()
                                    .to_vec(),
                            ),
                        ),
                        ("PackBlock", None),
                        (
                            "FileStorage",
                            Some(zip_bytes("docs/page.html", b"<html>page</html>")),
                        ),
                    ]),
                )
                .expect("book without readable PackBlock must open");

                assert_eq!(book.toc().pages()[0].html_path, "docs/page.html");
                assert_eq!(
                    book.toc()
                        .find_by_html_path("/docs/page.html")
                        .unwrap()
                        .title
                        .display(),
                    "docs/page.html"
                );
                assert_eq!(
                    book.read_page("/docs/page.html").unwrap(),
                    "<html>page</html>"
                );
            }

            #[test]
            fn documentation_reader_loads_page_from_book_storage() {
                let toc = r#"{
                    1
                    {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/docs/page.html"}}
                }"#;
                let book = HbkBook::from_bytes(
                    PathBuf::from("fmtdui_ru.hbk"),
                    fixture_container(vec![
                        (
                            "Book",
                            Some(
                                r#"{1,"Interface", {1,2,{"ru","fmtdui"}}, 1, "tag", {0,0}, 0}"#
                                    .as_bytes()
                                    .to_vec(),
                            ),
                        ),
                        ("PackBlock", Some(zip_bytes("toc.txt", toc.as_bytes()))),
                        (
                            "FileStorage",
                            Some(zip_entries(vec![
                                (
                                    "docs/page.html",
                                    br#"<html><head><title>Loaded</title></head><body>Text <a href="../assets/resource.bin">asset</a></body></html>"#,
                                ),
                                ("assets/resource.bin", b"payload"),
                            ])),
                        ),
                    ]),
                )
                .expect("book must open");

                let page = DocumentationReader::new(&book)
                    .load_page("/docs/page.html")
                    .expect("documentation page must load");

                assert_eq!(page.title, "Loaded");
                assert_eq!(
                    page.raw_html,
                    r#"<html><head><title>Loaded</title></head><body>Text <a href="../assets/resource.bin">asset</a></body></html>"#
                );
                assert_eq!(page.source.html_path, "docs/page.html");
                assert_eq!(page.links.len(), 1);
                assert_eq!(
                    page.links[0].normalized_path.as_deref(),
                    Some("assets/resource.bin")
                );
                assert_eq!(page.links[0].status, crate::hbk::docs::LinkStatus::Resolved);
            }

            fn zip_bytes(name: &str, body: &[u8]) -> Vec<u8> {
                zip_entries(vec![(name, body)])
            }

            fn zip_entries(entries: Vec<(&str, &[u8])>) -> Vec<u8> {
                let mut output = Cursor::new(Vec::new());
                {
                    let mut zip = ZipWriter::new(&mut output);
                    let options = SimpleFileOptions::default()
                        .compression_method(CompressionMethod::Deflated);
                    for (name, body) in entries {
                        zip.start_file(name, options).unwrap();
                        zip.write_all(body).unwrap();
                    }
                    zip.finish().unwrap();
                }
                output.into_inner()
            }

            fn fixture_container(entities: Vec<(&str, Option<Vec<u8>>)>) -> Vec<u8> {
                const BLOCK_HEADER_SIZE: usize = 31;
                const FILE_DESCRIPTOR_SIZE: usize = 12;
                const SPLITTER: u32 = i32::MAX as u32;

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
                        push_block(&mut bytes, body.len(), &body, None);
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

            fn push_block(
                bytes: &mut Vec<u8>,
                payload_size: usize,
                chunk: &[u8],
                next: Option<usize>,
            ) {
                const SPLITTER: u32 = i32::MAX as u32;
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
        }
    }

    pub mod docs {
        use std::fmt;
        use std::path::{Path, PathBuf};

        use scraper::node::Node;
        use scraper::{ElementRef, Html, Selector};

        use super::book::{BookError, HbkBook, normalize_storage_path};
        use super::toc::Toc;

        #[derive(Debug)]
        pub struct DocumentationReader<'a> {
            book: &'a HbkBook,
        }

        impl<'a> DocumentationReader<'a> {
            pub fn new(book: &'a HbkBook) -> Self {
                Self { book }
            }

            pub fn load_page(&self, html_path: &str) -> Result<PageContent, DocumentationError> {
                let raw_html = self.book.read_page(html_path).map_err(|source| {
                    DocumentationError::PageRead {
                        path: self.book.path().to_path_buf(),
                        html_path: normalize_storage_path(html_path).to_string(),
                        source,
                    }
                })?;
                Ok(parse_page_html(
                    self.book.path(),
                    self.book.locale().source_code(),
                    self.book.toc(),
                    html_path,
                    &raw_html,
                    |path| self.book.read_file(path).is_ok(),
                ))
            }
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct PageContent {
            pub source: PageSource,
            pub title: String,
            pub raw_html: String,
            pub body_text: String,
            pub text_preview: String,
            pub links: Vec<ResolvedLink>,
            pub diagnostics: Vec<LinkDiagnostic>,
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct PageSource {
            pub hbk_path: PathBuf,
            pub locale: String,
            pub toc_path: Option<String>,
            pub html_path: String,
            pub toc_title: Option<String>,
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct ResolvedLink {
            pub raw_href: String,
            pub normalized_path: Option<String>,
            pub title: Option<String>,
            pub status: LinkStatus,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum LinkStatus {
            Resolved,
            Unresolved,
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct LinkDiagnostic {
            pub severity: DiagnosticSeverity,
            pub code: &'static str,
            pub hbk_path: PathBuf,
            pub locale: String,
            pub html_path: String,
            pub page_title: String,
            pub raw_href: String,
            pub normalized_path: Option<String>,
            pub message: String,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum DiagnosticSeverity {
            Warning,
        }

        #[derive(Debug)]
        pub enum DocumentationError {
            PageRead {
                path: PathBuf,
                html_path: String,
                source: BookError,
            },
        }

        impl fmt::Display for DocumentationError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    Self::PageRead {
                        path,
                        html_path,
                        source,
                    } => write!(
                        f,
                        "failed to read documentation page '{html_path}' from '{}': {source}",
                        path.display()
                    ),
                }
            }
        }

        impl std::error::Error for DocumentationError {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                match self {
                    Self::PageRead { source, .. } => Some(source),
                }
            }
        }

        pub fn parse_page_html(
            hbk_path: &Path,
            locale: &str,
            toc: &Toc,
            html_path: &str,
            raw_html: &str,
            storage_contains: impl Fn(&str) -> bool,
        ) -> PageContent {
            let normalized_page_path = normalize_storage_path(html_path).to_string();
            let toc_page = toc
                .flat_pages()
                .find(|flat_page| flat_page.page.html_path == normalized_page_path);
            let toc_path = toc_page
                .as_ref()
                .map(|flat_page| flat_page.index_path.to_string());
            let toc_title = toc_page
                .as_ref()
                .map(|flat_page| flat_page.page.title.display().to_string());
            let document = Html::parse_document(raw_html);
            let title = select_first_text(&document, "title")
                .or_else(|| select_first_text(&document, "h1"))
                .or_else(|| toc_title.clone())
                .unwrap_or_default();
            let body_text = normalized_body_text(&document);
            let text_preview = text_preview(&body_text);
            let (links, diagnostics) = extract_links(
                &document,
                toc,
                hbk_path,
                locale,
                &normalized_page_path,
                &title,
                storage_contains,
            );

            PageContent {
                source: PageSource {
                    hbk_path: hbk_path.to_path_buf(),
                    locale: locale.to_string(),
                    toc_path,
                    html_path: normalized_page_path,
                    toc_title,
                },
                title,
                raw_html: raw_html.to_string(),
                body_text,
                text_preview,
                links,
                diagnostics,
            }
        }

        fn select_first_text(document: &Html, selector: &str) -> Option<String> {
            let selector = Selector::parse(selector).expect("static selector must be valid");
            document.select(&selector).find_map(|element| {
                let text = normalize_whitespace(element.text());
                (!text.is_empty()).then_some(text)
            })
        }

        fn normalized_body_text(document: &Html) -> String {
            let selector = Selector::parse("body").expect("static selector must be valid");
            document
                .select(&selector)
                .next()
                .map(normalize_element_text)
                .unwrap_or_else(|| normalize_element_text(document.root_element()))
        }

        fn text_preview(body_text: &str) -> String {
            const MAX_PREVIEW_CHARS: usize = 240;
            body_text.chars().take(MAX_PREVIEW_CHARS).collect()
        }

        fn normalize_whitespace<'a>(parts: impl Iterator<Item = &'a str>) -> String {
            let mut collector = TextCollector::default();
            for part in parts {
                collector.push_text(part);
            }
            collector.finish()
        }

        fn normalize_element_text(element: ElementRef<'_>) -> String {
            let mut collector = TextCollector::default();
            collect_element_text(element, &mut collector);
            collector.finish()
        }

        fn collect_element_text(element: ElementRef<'_>, collector: &mut TextCollector) {
            for child in element.children() {
                match child.value() {
                    Node::Text(text) => collector.push_text(text),
                    Node::Element(element) => {
                        let tag_name = element.name();
                        if tag_name == "br" || is_block_text_element(tag_name) {
                            collector.ensure_separator();
                        }
                        if let Some(child_element) = ElementRef::wrap(child) {
                            collect_element_text(child_element, collector);
                        }
                        if tag_name == "br" || is_block_text_element(tag_name) {
                            collector.ensure_separator();
                        }
                    }
                    _ => {}
                }
            }
        }

        fn is_block_text_element(tag_name: &str) -> bool {
            matches!(
                tag_name,
                "address"
                    | "article"
                    | "aside"
                    | "blockquote"
                    | "dd"
                    | "div"
                    | "dl"
                    | "dt"
                    | "figcaption"
                    | "figure"
                    | "footer"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "header"
                    | "hr"
                    | "li"
                    | "main"
                    | "nav"
                    | "ol"
                    | "p"
                    | "pre"
                    | "section"
                    | "table"
                    | "td"
                    | "th"
                    | "tr"
                    | "ul"
            )
        }

        #[derive(Debug, Default)]
        struct TextCollector {
            output: String,
            pending_separator: bool,
        }

        impl TextCollector {
            fn push_text(&mut self, text: &str) {
                for ch in text.chars() {
                    if ch.is_whitespace() {
                        self.ensure_separator();
                    } else {
                        self.flush_separator();
                        self.output.push(ch);
                    }
                }
            }

            fn ensure_separator(&mut self) {
                if !self.output.is_empty() {
                    self.pending_separator = true;
                }
            }

            fn flush_separator(&mut self) {
                if self.pending_separator && !self.output.ends_with(' ') {
                    self.output.push(' ');
                }
                self.pending_separator = false;
            }

            fn finish(self) -> String {
                self.output
            }
        }

        fn extract_links(
            document: &Html,
            toc: &Toc,
            hbk_path: &Path,
            locale: &str,
            current_html_path: &str,
            page_title: &str,
            storage_contains: impl Fn(&str) -> bool,
        ) -> (Vec<ResolvedLink>, Vec<LinkDiagnostic>) {
            let selector = Selector::parse("a[href]").expect("static selector must be valid");
            let mut links = Vec::new();
            let mut diagnostics = Vec::new();

            for element in document.select(&selector) {
                let raw_href = element.value().attr("href").unwrap_or_default().to_string();
                let normalized_path = normalize_link_target(current_html_path, &raw_href);
                let resolved_page = normalized_path
                    .as_deref()
                    .and_then(|path| toc.find_by_html_path(path));

                if let Some(page) = resolved_page {
                    links.push(ResolvedLink {
                        raw_href,
                        normalized_path,
                        title: Some(page.title.display().to_string()),
                        status: LinkStatus::Resolved,
                    });
                } else if normalized_path.as_deref().is_some_and(&storage_contains) {
                    links.push(ResolvedLink {
                        raw_href,
                        normalized_path,
                        title: None,
                        status: LinkStatus::Resolved,
                    });
                } else {
                    let message = match normalized_path.as_deref() {
                        Some(path) => format!("link target '{path}' is not present in the TOC"),
                        None => "link cannot be normalized to an internal HBK page".to_string(),
                    };
                    diagnostics.push(LinkDiagnostic {
                        severity: DiagnosticSeverity::Warning,
                        code: "UNRESOLVED_LINK",
                        hbk_path: hbk_path.to_path_buf(),
                        locale: locale.to_string(),
                        html_path: current_html_path.to_string(),
                        page_title: page_title.to_string(),
                        raw_href: raw_href.clone(),
                        normalized_path: normalized_path.clone(),
                        message,
                    });
                    links.push(ResolvedLink {
                        raw_href,
                        normalized_path,
                        title: None,
                        status: LinkStatus::Unresolved,
                    });
                }
            }

            (links, diagnostics)
        }

        fn normalize_link_target(current_html_path: &str, href: &str) -> Option<String> {
            let href = href.trim();
            if href.is_empty() {
                return None;
            }
            if href.starts_with('#') {
                return Some(current_html_path.to_string());
            }
            if is_unsupported_scheme(href) {
                return None;
            }

            let v8help_target = href.strip_prefix("v8help://");
            let without_scheme = v8help_target.unwrap_or(href);
            let path_part = without_scheme
                .split(['#', '?'])
                .next()
                .unwrap_or_default()
                .trim();
            if path_part.is_empty() {
                return Some(current_html_path.to_string());
            }

            let candidate = if v8help_target.is_some() || path_part.starts_with('/') {
                path_part.to_string()
            } else {
                let base = current_html_path.rsplit_once('/').map(|(base, _)| base);
                match base {
                    Some(base) if !base.is_empty() => format!("{base}/{path_part}"),
                    _ => path_part.to_string(),
                }
            };
            normalize_path_segments(&candidate)
        }

        fn is_unsupported_scheme(href: &str) -> bool {
            href.contains(':') && !href.starts_with("v8help://")
        }

        fn normalize_path_segments(path: &str) -> Option<String> {
            let mut segments = Vec::new();
            for segment in path.trim_start_matches('/').split('/') {
                match segment {
                    "" | "." => {}
                    ".." => {
                        segments.pop()?;
                    }
                    value => segments.push(value),
                }
            }
            (!segments.is_empty()).then(|| segments.join("/"))
        }

        #[cfg(test)]
        mod tests {
            use super::*;
            use crate::hbk::book::HbkBook;
            use crate::hbk::toc::Toc;

            #[test]
            fn extracts_title_text_preview_and_provenance() {
                let toc = Toc::parse(
                    r#"{
                        1
                        {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/docs/page.html"}}
                    }"#,
                )
                .expect("toc must parse");

                let content = parse_page_html(
                    Path::new("fmtdui_ru.hbk"),
                    "ru",
                    &toc,
                    "/docs/page.html",
                    r#"<html><head><title>HTML title</title></head>
                    <body><h1>Body title</h1><p> First  text
                    with spacing. </p></body></html>"#,
                    |_| false,
                );

                assert_eq!(content.source.hbk_path, PathBuf::from("fmtdui_ru.hbk"));
                assert_eq!(content.source.locale, "ru");
                assert_eq!(content.source.html_path, "docs/page.html");
                assert_eq!(content.source.toc_path.as_deref(), Some("0"));
                assert_eq!(content.source.toc_title.as_deref(), Some("Страница"));
                assert_eq!(content.title, "HTML title");
                assert_eq!(content.text_preview, "Body title First text with spacing.");
            }

            #[test]
            fn normalized_text_separates_block_siblings_without_breaking_inline_quotes() {
                let toc = Toc::parse(
                    r#"{
                        1
                        {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/docs/page.html"}}
                    }"#,
                )
                .expect("toc must parse");

                let content = parse_page_html(
                    Path::new("fmtdui_ru.hbk"),
                    "ru",
                    &toc,
                    "/docs/page.html",
                    r#"<html><body><p>Alpha</p><p>Beta "<strong>Gamma</strong>"</p></body></html>"#,
                    |_| false,
                );

                assert_eq!(content.body_text, "Alpha Beta \"Gamma\"");
            }

            #[test]
            fn normalized_text_keeps_adjacent_inline_nodes_joined() {
                let toc = Toc::parse(
                    r#"{
                        1
                        {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/docs/page.html"}}
                    }"#,
                )
                .expect("toc must parse");

                let content = parse_page_html(
                    Path::new("fmtdui_ru.hbk"),
                    "ru",
                    &toc,
                    "/docs/page.html",
                    r#"<html><body><p>foo<strong>bar</strong></p></body></html>"#,
                    |_| false,
                );

                assert_eq!(content.body_text, "foobar");
            }

            #[test]
            fn normalizes_and_resolves_internal_links() {
                let toc = Toc::parse(
                    r#"{
                        3
                        {1,0,0,{0,0,{0,0,{"ru","Current"}},"/docs/current.html"}}
                        {2,0,0,{0,0,{0,0,{"ru","Relative"}},"/docs/next.html"}}
                        {3,0,0,{0,0,{0,0,{"ru","Parent"}},"/parent.html"}}
                    }"#,
                )
                .expect("toc must parse");

                let content = parse_page_html(
                    Path::new("fmtdui_ru.hbk"),
                    "ru",
                    &toc,
                    "docs/current.html",
                    r##"<html><body>
                        <a href="next.html#section">next</a>
                        <a href="../parent.html">parent</a>
                        <a href="#local">local</a>
                        <a href="v8help://docs/next.html?query">v8</a>
                    </body></html>"##,
                    |_| false,
                );

                let paths = content
                    .links
                    .iter()
                    .map(|link| link.normalized_path.as_deref())
                    .collect::<Vec<_>>();
                assert_eq!(
                    paths,
                    vec![
                        Some("docs/next.html"),
                        Some("parent.html"),
                        Some("docs/current.html"),
                        Some("docs/next.html")
                    ]
                );
                assert!(
                    content
                        .links
                        .iter()
                        .all(|link| link.status == LinkStatus::Resolved)
                );
                assert!(content.diagnostics.is_empty());
            }

            #[test]
            fn reports_unresolved_links_without_dropping_them() {
                let toc = Toc::parse(
                    r#"{
                        1
                        {1,0,0,{0,0,{0,0,{"ru","Current"}},"/docs/current.html"}}
                    }"#,
                )
                .expect("toc must parse");

                let content = parse_page_html(
                    Path::new("fmtdui_ru.hbk"),
                    "ru",
                    &toc,
                    "docs/current.html",
                    r#"<html><body>
                        <a href="missing.html">missing</a>
                        <a href="https://example.invalid/page">external</a>
                    </body></html>"#,
                    |_| false,
                );

                assert_eq!(content.links.len(), 2);
                assert_eq!(
                    content.links[0].normalized_path.as_deref(),
                    Some("docs/missing.html")
                );
                assert_eq!(content.links[0].status, LinkStatus::Unresolved);
                assert_eq!(content.links[1].normalized_path, None);
                assert_eq!(content.links[1].status, LinkStatus::Unresolved);
                assert_eq!(content.diagnostics.len(), 2);
                assert!(content.diagnostics.iter().all(|diagnostic| {
                    diagnostic.severity == DiagnosticSeverity::Warning
                        && diagnostic.code == "UNRESOLVED_LINK"
                        && diagnostic.hbk_path == PathBuf::from("fmtdui_ru.hbk")
                        && diagnostic.locale == "ru"
                        && diagnostic.html_path == "docs/current.html"
                }));
            }

            #[test]
            fn fixture_pages_have_stable_text_and_link_snapshots() {
                let toc = Toc::parse(
                    r#"{
                        1
                        {1,0,0,{0,0,{0,0,{"ru","Форматированная строка"}{"en","Formatted string"}},"/form_formattedstringedit"}}
                    }"#,
                )
                .expect("toc must parse");

                for fixture in [
                    (
                        "fmtdui_ru.hbk",
                        "ru",
                        "Конструктор строк на разных языках",
                        include_str!(
                            "../tests/fixtures/docs/fmtdui_ru_form_formattedstringedit.html"
                        ),
                        include_str!(
                            "../tests/fixtures/docs/fmtdui_ru_form_formattedstringedit.text"
                        ),
                    ),
                    (
                        "fmtdui_root.hbk",
                        "root",
                        "Constructor of strings in different languages",
                        include_str!(
                            "../tests/fixtures/docs/fmtdui_root_form_formattedstringedit.html"
                        ),
                        include_str!(
                            "../tests/fixtures/docs/fmtdui_root_form_formattedstringedit.text"
                        ),
                    ),
                ] {
                    let (hbk_path, locale, title, html, expected_text) = fixture;
                    let content = parse_page_html(
                        Path::new(hbk_path),
                        locale,
                        &toc,
                        "form_formattedstringedit",
                        html,
                        |_| false,
                    );

                    assert_eq!(content.raw_html, html);
                    assert_eq!(content.title, title);
                    assert_eq!(content.body_text, expected_text.trim());
                    assert_eq!(
                        content.text_preview,
                        expected_text.chars().take(240).collect::<String>()
                    );
                    assert!(content.links.is_empty());
                    assert!(content.diagnostics.is_empty());
                }
            }

            #[test]
            fn fixture_links_have_stable_resolution_snapshot() {
                let toc = Toc::parse(
                    r#"{
                        2
                        {1,0,0,{0,0,{0,0,{"ru","Current"}},"/docs/current.html"}}
                        {2,0,0,{0,0,{0,0,{"ru","Next"}},"/docs/next.html"}}
                    }"#,
                )
                .expect("toc must parse");
                let html = include_str!("../tests/fixtures/docs/fmtdui_link_handling.html");
                let expected = include_str!("../tests/fixtures/docs/fmtdui_link_handling.links")
                    .lines()
                    .collect::<Vec<_>>();

                let content = parse_page_html(
                    Path::new("fmtdui_ru.hbk"),
                    "ru",
                    &toc,
                    "docs/current.html",
                    html,
                    |path| path == "shared/topic.html",
                );

                let actual = content
                    .links
                    .iter()
                    .map(|link| {
                        format!(
                            "{} -> {} {}",
                            link.raw_href,
                            link.normalized_path.as_deref().unwrap_or("<none>"),
                            match link.status {
                                LinkStatus::Resolved => "resolved",
                                LinkStatus::Unresolved => "unresolved",
                            }
                        )
                    })
                    .collect::<Vec<_>>();

                assert_eq!(actual, expected);
                assert_eq!(content.diagnostics.len(), 1);
                assert_eq!(
                    content.diagnostics[0].normalized_path.as_deref(),
                    Some("docs/missing.html")
                );
            }

            #[test]
            fn real_fmtdui_page_loads_when_platform_fixture_exists() {
                let cases = [
                    (
                        Path::new("/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk"),
                        include_str!("../tests/fixtures/known-pages/fmtdui_ru.page").trim(),
                    ),
                    (
                        Path::new("/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk"),
                        include_str!("../tests/fixtures/known-pages/fmtdui_root.page").trim(),
                    ),
                ];

                for (path, page_path) in cases {
                    if !path.exists() {
                        continue;
                    }

                    let book = HbkBook::open(path).expect("platform HBK must open");
                    let page = DocumentationReader::new(&book)
                        .load_page(page_path)
                        .expect("known platform page must load");

                    assert_eq!(page.source.html_path, page_path);
                    assert!(!page.raw_html.is_empty());
                    assert!(!page.body_text.is_empty());
                    assert!(!page.title.is_empty());
                }
            }
        }
    }

    pub mod toc {
        use std::fmt;

        use super::tokens::{TokenError, TokenParser, tokenize};

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct LocalizedTitle {
            pub en: String,
            pub ru: String,
        }

        impl LocalizedTitle {
            pub fn display(&self) -> &str {
                if !self.ru.is_empty() {
                    &self.ru
                } else {
                    &self.en
                }
            }
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct TocPath(Vec<usize>);

        impl TocPath {
            pub fn root(index: usize) -> Self {
                Self(vec![index])
            }

            pub fn child(&self, index: usize) -> Self {
                let mut values = self.0.clone();
                values.push(index);
                Self(values)
            }

            pub fn indexes(&self) -> &[usize] {
                &self.0
            }
        }

        impl fmt::Display for TocPath {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                for (index, value) in self.0.iter().enumerate() {
                    if index > 0 {
                        f.write_str(".")?;
                    }
                    write!(f, "{value}")?;
                }
                Ok(())
            }
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct TocPage {
            pub id: usize,
            pub parent_id: usize,
            pub title: LocalizedTitle,
            pub html_path: String,
            pub children: Vec<TocPage>,
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Toc {
            pages: Vec<TocPage>,
        }

        impl Toc {
            pub fn empty() -> Self {
                Self { pages: Vec::new() }
            }

            pub(crate) fn from_storage_paths(paths: Vec<String>) -> Self {
                let pages = paths
                    .into_iter()
                    .enumerate()
                    .map(|(index, path)| TocPage {
                        id: index + 1,
                        parent_id: 0,
                        title: LocalizedTitle {
                            en: path.clone(),
                            ru: String::new(),
                        },
                        html_path: path,
                        children: Vec::new(),
                    })
                    .collect();
                Self { pages }
            }

            pub fn parse(content: &str) -> Result<Self, TocError> {
                let chunks = parse_chunks(content)?;
                let pages = build_tree(0, &chunks);
                Ok(Self { pages })
            }

            pub fn pages(&self) -> &[TocPage] {
                &self.pages
            }

            pub fn find_by_html_path(&self, html_path: &str) -> Option<&TocPage> {
                let normalized = html_path.trim_start_matches('/');
                self.flat_pages()
                    .find(|page| page.page.html_path == normalized)
                    .map(|page| page.page)
            }

            pub fn find_by_index_path(&self, path: &TocPath) -> Option<&TocPage> {
                let mut pages = self.pages.as_slice();
                let mut found = None;
                for index in path.indexes() {
                    let page = pages.get(*index)?;
                    found = Some(page);
                    pages = &page.children;
                }
                found
            }

            pub fn flat_pages(&self) -> impl Iterator<Item = FlatTocPage<'_>> {
                let mut output = Vec::new();
                for (index, page) in self.pages.iter().enumerate() {
                    flatten_page(page, TocPath::root(index), &mut output);
                }
                output.into_iter()
            }
        }

        #[derive(Debug, Clone)]
        pub struct FlatTocPage<'a> {
            pub index_path: TocPath,
            pub page: &'a TocPage,
        }

        fn flatten_page<'a>(
            page: &'a TocPage,
            index_path: TocPath,
            output: &mut Vec<FlatTocPage<'a>>,
        ) {
            output.push(FlatTocPage {
                index_path: index_path.clone(),
                page,
            });
            for (index, child) in page.children.iter().enumerate() {
                flatten_page(child, index_path.child(index), output);
            }
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct TocError {
            message: String,
        }

        impl TocError {
            fn new(message: impl Into<String>) -> Self {
                Self {
                    message: message.into(),
                }
            }
        }

        impl fmt::Display for TocError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "invalid HBK TOC: {}", self.message)
            }
        }

        impl std::error::Error for TocError {}

        impl From<TokenError> for TocError {
            fn from(value: TokenError) -> Self {
                Self::new(value.to_string())
            }
        }

        #[derive(Debug)]
        struct TocChunk {
            id: usize,
            parent_id: usize,
            title: LocalizedTitle,
            html_path: String,
        }

        fn parse_chunks(content: &str) -> Result<Vec<TocChunk>, TocError> {
            let mut parser = TokenParser::new(tokenize(content));
            parser.expect("{", "TableOfContent: expected '{'")?;
            parser.number("TableOfContent: expected chunk count")?;
            let mut chunks = Vec::new();
            while parser.peek().is_some_and(|token| token != "}") {
                chunks.push(parse_chunk(&mut parser)?);
            }
            parser.expect("}", "TableOfContent: expected closing '}'")?;
            parser.expect_end("TableOfContent")?;
            Ok(chunks)
        }

        fn parse_chunk(parser: &mut TokenParser) -> Result<TocChunk, TocError> {
            parser.expect("{", "Chunk: expected '{'")?;
            let id = parser.number("Chunk: expected id")?;
            let parent_id = parser.number("Chunk: expected parent id")?;
            let child_count = parser.number("Chunk: expected child count")?;
            for index in 0..child_count {
                parser.number(format!("Chunk: expected child id #{}", index + 1))?;
            }
            parser.expect("{", "Properties: expected '{'")?;
            parser.number("Properties: expected first number")?;
            parser.number("Properties: expected second number")?;
            let title = parse_title(parser)?;
            let html_path = parser
                .string("Properties: expected HTML path")?
                .trim_start_matches('/')
                .to_string();
            parser.expect("}", "Properties: expected closing '}'")?;
            parser.expect("}", "Chunk: expected closing '}'")?;
            Ok(TocChunk {
                id,
                parent_id,
                title,
                html_path,
            })
        }

        fn parse_title(parser: &mut TokenParser) -> Result<LocalizedTitle, TocError> {
            parser.expect("{", "NameContainer: expected '{'")?;
            parser.number("NameContainer: expected first number")?;
            parser.number("NameContainer: expected second number")?;
            let mut names = Vec::new();
            while parser.peek().is_some_and(|token| token != "}") {
                parser.expect("{", "NameObject: expected '{'")?;
                let language = parser.string("NameObject: expected language")?;
                let title = parser.string("NameObject: expected title")?;
                parser.expect("}", "NameObject: expected closing '}'")?;
                names.push((language, title));
            }
            parser.expect("}", "NameContainer: expected closing '}'")?;
            let mut title = LocalizedTitle {
                en: String::new(),
                ru: String::new(),
            };
            for (language, value) in names {
                match language.as_str() {
                    "en" | "#" => title.en = value,
                    "ru" => title.ru = value,
                    _ => {}
                }
            }
            Ok(title)
        }

        fn build_tree(parent_id: usize, chunks: &[TocChunk]) -> Vec<TocPage> {
            chunks
                .iter()
                .filter(|chunk| chunk.parent_id == parent_id)
                .map(|chunk| TocPage {
                    id: chunk.id,
                    parent_id: chunk.parent_id,
                    title: chunk.title.clone(),
                    html_path: chunk.html_path.clone(),
                    children: build_tree(chunk.id, chunks),
                })
                .collect()
        }

        #[cfg(test)]
        mod tests {
            use super::*;

            #[test]
            fn parses_toc_tree_and_lookups() {
                let toc = Toc::parse(
                    r#"{
                        2
                        {1,0,1,2,{0,0,{0,0,{"ru","Корень"}{"en","Root"}},"/root.html"}}
                        {2,1,0,{0,0,{0,0,{"ru","Дочерняя"}{"en","Child"}},"/child.html"}}
                    }"#,
                )
                .expect("toc must parse");

                assert_eq!(toc.pages().len(), 1);
                assert_eq!(toc.pages()[0].children.len(), 1);
                assert_eq!(
                    toc.find_by_html_path("/child.html")
                        .unwrap()
                        .title
                        .display(),
                    "Дочерняя"
                );
                assert_eq!(
                    toc.find_by_index_path(&TocPath(vec![0, 0]))
                        .unwrap()
                        .html_path,
                    "child.html"
                );
            }

            #[test]
            fn assigns_localized_titles_by_language_code() {
                let toc = Toc::parse(
                    r#"{
                        2
                        {1,0,0,{0,0,{0,0,{"en","Root"}{"ru","Корень"}},"/root.html"}}
                        {2,0,0,{0,0,{0,0,{"ru","Только русский"}},"/ru.html"}}
                    }"#,
                )
                .expect("toc must parse");

                assert_eq!(toc.pages()[0].title.en, "Root");
                assert_eq!(toc.pages()[0].title.ru, "Корень");
                assert_eq!(toc.pages()[1].title.en, "");
                assert_eq!(toc.pages()[1].title.ru, "Только русский");
            }
        }
    }
}

pub mod syntax_helper {
    use std::collections::BTreeSet;
    use std::fmt;
    use std::path::{Path, PathBuf};

    use scraper::{Html, Selector};

    use crate::hbk::book::{BookError, HbkBook};
    use crate::hbk::docs::{DocumentationError, DocumentationReader, PageContent, PageSource};
    use crate::hbk::toc::{FlatTocPage, Toc, TocPage};

    #[derive(Debug)]
    pub struct SyntaxHelperReader<'a> {
        book: &'a HbkBook,
    }

    impl<'a> SyntaxHelperReader<'a> {
        pub fn new(book: &'a HbkBook) -> Self {
            Self { book }
        }

        pub fn discover_roots(&self) -> Result<RootDiscovery, SyntaxHelperError> {
            discover_roots_with_loader(
                self.book.path(),
                self.book.locale().source_code(),
                self.book.toc(),
                |html_path| {
                    DocumentationReader::new(self.book)
                        .load_page(html_path)
                        .map_err(SyntaxHelperError::Documentation)
                },
            )
        }

        pub fn extract(&self) -> Result<PlatformContext, SyntaxHelperError> {
            let root_paths = self
                .book
                .toc()
                .flat_pages()
                .filter(|flat_page| {
                    flat_page.index_path.indexes().len() == 1
                        && is_syntax_helper_path(&flat_page.page.html_path)
                })
                .map(|flat_page| flat_page.page.html_path.clone())
                .collect::<Vec<_>>();
            let root_pages = self
                .book
                .read_pages(root_paths.iter().map(String::as_str))?;
            let discovery = discover_roots_with_loader(
                self.book.path(),
                self.book.locale().source_code(),
                self.book.toc(),
                |html_path| {
                    let raw_html = root_pages.get(html_path).ok_or_else(|| {
                        SyntaxHelperError::Book(BookError::MissingZipEntry {
                            path: self.book.path().to_path_buf(),
                            entry_name: html_path.to_string(),
                        })
                    })?;
                    Ok(parse_syntax_page_content(
                        self.book.path(),
                        self.book.locale().source_code(),
                        self.book.toc(),
                        html_path,
                        raw_html,
                    ))
                },
            )?;
            let page_paths = primary_extraction_page_paths(&discovery);
            let pages = self
                .book
                .read_pages(page_paths.iter().map(String::as_str))?;
            let discovery = primary_extraction_discovery(discovery);
            parse_extraction_pages(
                self.book.path(),
                self.book.locale().source_code(),
                self.book.toc(),
                discovery,
                |html_path| {
                    let raw_html = pages.get(html_path).ok_or_else(|| {
                        SyntaxHelperError::Book(BookError::MissingZipEntry {
                            path: self.book.path().to_path_buf(),
                            entry_name: html_path.to_string(),
                        })
                    })?;
                    Ok(parse_syntax_page_content(
                        self.book.path(),
                        self.book.locale().source_code(),
                        self.book.toc(),
                        html_path,
                        raw_html,
                    ))
                },
            )
        }
    }

    #[derive(Debug)]
    pub enum SyntaxHelperError {
        Book(BookError),
        Documentation(DocumentationError),
    }

    impl fmt::Display for SyntaxHelperError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Book(source) => write!(f, "{source}"),
                Self::Documentation(source) => write!(f, "{source}"),
            }
        }
    }

    impl std::error::Error for SyntaxHelperError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Self::Book(source) => Some(source),
                Self::Documentation(source) => Some(source),
            }
        }
    }

    impl From<BookError> for SyntaxHelperError {
        fn from(value: BookError) -> Self {
            Self::Book(value)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RootDiscovery {
        pub roots: Vec<RootSection>,
        pub diagnostics: Vec<SyntaxHelperDiagnostic>,
    }

    impl RootDiscovery {
        pub fn has_kind(&self, kind: RootSectionKind) -> bool {
            self.roots.iter().any(|root| root.kind == kind)
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RootSection {
        pub kind: RootSectionKind,
        pub source: SyntaxHelperSource,
        pub pages: Vec<CatalogPage>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum RootSectionKind {
        GlobalContext,
        EnumCatalog,
        TypeObjectCatalog,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CatalogPage {
        pub class: PageClass,
        pub source: SyntaxHelperSource,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum PageClass {
        Catalog,
        GlobalMethod,
        GlobalProperty,
        ObjectType,
        ObjectMethod,
        ObjectProperty,
        Constructor,
        Enum,
        EnumValue,
        Unknown,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SyntaxHelperSource {
        pub hbk_path: PathBuf,
        pub locale: String,
        pub toc_path: Option<String>,
        pub html_path: String,
        pub page_title: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct SyntaxHelperDiagnostic {
        pub severity: DiagnosticSeverity,
        pub code: &'static str,
        pub source: SyntaxHelperSource,
        pub parser_stage: &'static str,
        pub message: String,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DiagnosticSeverity {
        Warning,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub struct PlatformContext {
        pub global_contexts: Vec<GlobalContext>,
        pub global_methods: Vec<GlobalMethod>,
        pub global_properties: Vec<GlobalProperty>,
        pub platform_types: Vec<PlatformType>,
        pub type_methods: Vec<PlatformMethod>,
        pub type_properties: Vec<PlatformProperty>,
        pub constructors: Vec<Constructor>,
        pub enums: Vec<EnumDefinition>,
        pub enum_values: Vec<EnumValue>,
        pub diagnostics: Vec<SyntaxHelperDiagnostic>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GlobalContext {
        pub name: LocalizedName,
        pub property_links: Vec<MemberLink>,
        pub method_links: Vec<MemberLink>,
        pub description: Option<String>,
        pub source: SyntaxHelperSource,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GlobalMethod {
        pub name: LocalizedName,
        pub signatures: Vec<Signature>,
        pub return_types: Vec<TypeRef>,
        pub description: Option<String>,
        pub source: SyntaxHelperSource,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct GlobalProperty {
        pub name: LocalizedName,
        pub usage: Option<String>,
        pub type_refs: Vec<TypeRef>,
        pub description: Option<String>,
        pub source: SyntaxHelperSource,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PlatformType {
        pub name: LocalizedName,
        pub method_links: Vec<MemberLink>,
        pub constructor_links: Vec<MemberLink>,
        pub description: Option<String>,
        pub source: SyntaxHelperSource,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PlatformMethod {
        pub owner: LocalizedName,
        pub name: LocalizedName,
        pub signatures: Vec<Signature>,
        pub return_types: Vec<TypeRef>,
        pub description: Option<String>,
        pub source: SyntaxHelperSource,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct PlatformProperty {
        pub owner: LocalizedName,
        pub name: LocalizedName,
        pub usage: Option<String>,
        pub type_refs: Vec<TypeRef>,
        pub description: Option<String>,
        pub source: SyntaxHelperSource,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Constructor {
        pub owner: LocalizedName,
        pub name: LocalizedName,
        pub signatures: Vec<Signature>,
        pub description: Option<String>,
        pub source: SyntaxHelperSource,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct EnumDefinition {
        pub name: LocalizedName,
        pub value_links: Vec<MemberLink>,
        pub description: Option<String>,
        pub source: SyntaxHelperSource,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct EnumValue {
        pub owner: LocalizedName,
        pub name: LocalizedName,
        pub description: Option<String>,
        pub source: SyntaxHelperSource,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Signature {
        pub text: String,
        pub parameters: Vec<Parameter>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Parameter {
        pub name: String,
        pub required: bool,
        pub type_refs: Vec<TypeRef>,
        pub description: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TypeRef {
        pub name: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct LocalizedName {
        pub primary: String,
        pub alias: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct MemberLink {
        pub name: LocalizedName,
        pub html_path: String,
    }

    pub fn discover_roots_with_loader(
        hbk_path: &Path,
        locale: &str,
        toc: &Toc,
        mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
    ) -> Result<RootDiscovery, SyntaxHelperError> {
        let mut roots = Vec::new();
        let mut diagnostics = Vec::new();

        for flat_page in toc.flat_pages().filter(|flat_page| {
            flat_page.index_path.indexes().len() == 1
                && is_syntax_helper_path(&flat_page.page.html_path)
        }) {
            let page = load_page(&flat_page.page.html_path)?;
            let source = source_from_page(hbk_path, locale, &flat_page, &page);
            let Some(kind) = classify_root(&flat_page.page, &page) else {
                diagnostics.push(unknown_page_diagnostic(source));
                continue;
            };
            let pages = collect_catalog_pages(hbk_path, locale, &flat_page.page, &flat_page);
            diagnostics.extend(
                pages
                    .iter()
                    .filter(|page| page.class == PageClass::Unknown)
                    .cloned()
                    .map(|page| unknown_page_diagnostic(page.source)),
            );
            roots.push(RootSection {
                kind,
                source,
                pages,
            });
        }

        Ok(RootDiscovery { roots, diagnostics })
    }

    pub fn extract_with_loader(
        hbk_path: &Path,
        locale: &str,
        toc: &Toc,
        mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
    ) -> Result<PlatformContext, SyntaxHelperError> {
        let discovery = discover_roots_with_loader(hbk_path, locale, toc, &mut load_page)?;
        parse_extraction_pages(hbk_path, locale, toc, discovery, load_page)
    }

    fn primary_extraction_page_paths(discovery: &RootDiscovery) -> Vec<String> {
        let mut paths = BTreeSet::new();
        for root in &discovery.roots {
            if root.kind == RootSectionKind::GlobalContext {
                paths.insert(root.source.html_path.clone());
            }
            for page in &root.pages {
                if matches!(
                    page.class,
                    PageClass::GlobalMethod
                        | PageClass::GlobalProperty
                        | PageClass::ObjectType
                        | PageClass::Enum
                ) {
                    paths.insert(page.source.html_path.clone());
                }
            }
        }
        paths.into_iter().collect()
    }

    fn primary_extraction_discovery(mut discovery: RootDiscovery) -> RootDiscovery {
        for root in &mut discovery.roots {
            root.pages.retain(|page| {
                matches!(
                    page.class,
                    PageClass::Catalog
                        | PageClass::Unknown
                        | PageClass::GlobalMethod
                        | PageClass::GlobalProperty
                        | PageClass::ObjectType
                        | PageClass::Enum
                )
            });
        }
        discovery
    }

    fn parse_extraction_pages(
        _hbk_path: &Path,
        _locale: &str,
        _toc: &Toc,
        discovery: RootDiscovery,
        mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
    ) -> Result<PlatformContext, SyntaxHelperError> {
        let mut context = PlatformContext {
            diagnostics: discovery.diagnostics,
            ..PlatformContext::default()
        };
        let mut visited = BTreeSet::new();

        for root in &discovery.roots {
            for catalog_page in &root.pages {
                if matches!(catalog_page.class, PageClass::Catalog | PageClass::Unknown) {
                    continue;
                }
                if !visited.insert(catalog_page.source.html_path.clone()) {
                    continue;
                }
                let content = load_page(&catalog_page.source.html_path)?;
                let source = source_from_content(&catalog_page.source, &content);
                match catalog_page.class {
                    PageClass::Catalog | PageClass::Unknown => unreachable!(),
                    PageClass::GlobalMethod => context
                        .global_methods
                        .push(parse_global_method(&content, source)),
                    PageClass::GlobalProperty => context
                        .global_properties
                        .push(parse_global_property(&content, source)),
                    PageClass::ObjectType => context
                        .platform_types
                        .push(parse_platform_type(&content, source)),
                    PageClass::ObjectMethod => context
                        .type_methods
                        .push(parse_platform_method(&content, source)),
                    PageClass::ObjectProperty => context
                        .type_properties
                        .push(parse_platform_property(&content, source)),
                    PageClass::Constructor => context
                        .constructors
                        .push(parse_constructor(&content, source)),
                    PageClass::Enum => context.enums.push(parse_enum(&content, source)),
                    PageClass::EnumValue => {
                        context.enum_values.push(parse_enum_value(&content, source))
                    }
                }
            }

            if root.kind == RootSectionKind::GlobalContext
                && visited.insert(root.source.html_path.clone())
            {
                let content = load_page(&root.source.html_path)?;
                let source = source_from_content(&root.source, &content);
                context
                    .global_contexts
                    .push(parse_global_context(&content, source));
            }
        }

        Ok(context)
    }

    pub fn parse_global_context(
        content: &PageContent,
        source: SyntaxHelperSource,
    ) -> GlobalContext {
        GlobalContext {
            name: page_title_name(content),
            property_links: links_in_section(content, &["Свойства:", "Properties:"]),
            method_links: links_in_section(content, &["Методы:", "Methods:"]),
            description: section_text(content, &["Описание:", "Description:"]),
            source,
        }
    }

    pub fn parse_global_method(content: &PageContent, source: SyntaxHelperSource) -> GlobalMethod {
        GlobalMethod {
            name: heading_name(content),
            signatures: parse_signatures(content),
            return_types: type_refs_from_section(
                content,
                &["Возвращаемое значение:", "Return value:"],
            ),
            description: section_text(content, &["Описание:", "Description:"]),
            source,
        }
    }

    pub fn parse_global_property(
        content: &PageContent,
        source: SyntaxHelperSource,
    ) -> GlobalProperty {
        GlobalProperty {
            name: heading_name(content),
            usage: section_text(content, &["Использование:", "Use:"]),
            type_refs: type_refs_from_section(content, &["Описание:", "Description:"]),
            description: section_text(content, &["Описание:", "Description:"]),
            source,
        }
    }

    pub fn parse_platform_type(content: &PageContent, source: SyntaxHelperSource) -> PlatformType {
        PlatformType {
            name: page_title_name(content),
            method_links: links_in_section(content, &["Методы:", "Methods:"]),
            constructor_links: links_in_section(content, &["Конструкторы:", "Constructors:"]),
            description: section_text(content, &["Описание:", "Description:"]),
            source,
        }
    }

    pub fn parse_platform_method(
        content: &PageContent,
        source: SyntaxHelperSource,
    ) -> PlatformMethod {
        PlatformMethod {
            owner: title_name(content),
            name: heading_name(content),
            signatures: parse_signatures(content),
            return_types: type_refs_from_section(
                content,
                &["Возвращаемое значение:", "Return value:"],
            ),
            description: section_text(content, &["Описание:", "Description:"]),
            source,
        }
    }

    pub fn parse_platform_property(
        content: &PageContent,
        source: SyntaxHelperSource,
    ) -> PlatformProperty {
        PlatformProperty {
            owner: title_name(content),
            name: heading_name(content),
            usage: section_text(content, &["Использование:", "Use:"]),
            type_refs: type_refs_from_section(content, &["Описание:", "Description:"]),
            description: section_text(content, &["Описание:", "Description:"]),
            source,
        }
    }

    pub fn parse_constructor(content: &PageContent, source: SyntaxHelperSource) -> Constructor {
        Constructor {
            owner: title_name(content),
            name: heading_name(content),
            signatures: parse_signatures(content),
            description: section_text(content, &["Описание:", "Description:"]),
            source,
        }
    }

    pub fn parse_enum(content: &PageContent, source: SyntaxHelperSource) -> EnumDefinition {
        EnumDefinition {
            name: page_title_name(content),
            value_links: links_in_section(content, &["Значения", "Values"]),
            description: section_text(content, &["Описание:", "Description:"]),
            source,
        }
    }

    pub fn parse_enum_value(content: &PageContent, source: SyntaxHelperSource) -> EnumValue {
        EnumValue {
            owner: title_name(content),
            name: heading_name(content),
            description: section_text(content, &["Описание:", "Description:"]),
            source,
        }
    }

    fn is_syntax_helper_path(html_path: &str) -> bool {
        html_path.starts_with("objects/")
    }

    fn classify_root(page: &TocPage, content: &PageContent) -> Option<RootSectionKind> {
        if is_global_context_page(page) {
            return Some(RootSectionKind::GlobalContext);
        }
        if is_enum_catalog_page(content) {
            return Some(RootSectionKind::EnumCatalog);
        }
        if is_type_object_catalog_page(page) {
            return Some(RootSectionKind::TypeObjectCatalog);
        }
        None
    }

    fn is_global_context_page(page: &TocPage) -> bool {
        let title = normalized_title(page);
        page.children.iter().any(|child| {
            child
                .html_path
                .starts_with("objects/Global context/methods/")
                || child
                    .html_path
                    .starts_with("objects/Global context/properties/")
        }) || title == "глобальный контекст"
            || title == "global context"
    }

    fn is_enum_catalog_page(content: &PageContent) -> bool {
        let title = normalized_text(&content.title);
        let body = normalized_text(&content.body_text);
        title == "системные перечисления"
            || title == "system enums"
            || title == "system enumerations"
            || body.contains("системные перечисления")
            || body.contains("system enums")
            || body.contains("system enumerations")
    }

    fn is_type_object_catalog_page(page: &TocPage) -> bool {
        page.html_path.starts_with("objects/catalog")
            && !page.children.is_empty()
            && page.children.iter().any(|child| {
                child
                    .html_path
                    .starts_with(page.html_path.trim_end_matches(".html"))
            })
    }

    fn collect_catalog_pages(
        hbk_path: &Path,
        locale: &str,
        root_page: &TocPage,
        root_flat_page: &FlatTocPage<'_>,
    ) -> Vec<CatalogPage> {
        let mut pages = Vec::new();
        pages.push(CatalogPage {
            class: PageClass::Catalog,
            source: source_from_toc(hbk_path, locale, root_flat_page),
        });
        for (index, child) in root_page.children.iter().enumerate() {
            collect_child_catalog_pages(
                hbk_path,
                locale,
                child,
                root_flat_page.index_path.child(index),
                &mut pages,
            );
        }
        pages
    }

    fn collect_child_catalog_pages(
        hbk_path: &Path,
        locale: &str,
        page: &TocPage,
        toc_path: crate::hbk::toc::TocPath,
        pages: &mut Vec<CatalogPage>,
    ) {
        pages.push(CatalogPage {
            class: classify_catalog_page(page),
            source: SyntaxHelperSource {
                hbk_path: hbk_path.to_path_buf(),
                locale: locale.to_string(),
                toc_path: Some(toc_path.to_string()),
                html_path: page.html_path.clone(),
                page_title: page.title.display().to_string(),
            },
        });
        for (index, child) in page.children.iter().enumerate() {
            collect_child_catalog_pages(hbk_path, locale, child, toc_path.child(index), pages);
        }
    }

    fn classify_catalog_page(page: &TocPage) -> PageClass {
        let path = page.html_path.as_str();
        if is_catalog_path(path) {
            PageClass::Catalog
        } else if path.starts_with("objects/Global context/methods/") {
            PageClass::GlobalMethod
        } else if path.starts_with("objects/Global context/properties/") {
            PageClass::GlobalProperty
        } else if path.contains("/methods/") {
            PageClass::ObjectMethod
        } else if path.contains("/properties/") && path.contains("/catalog2/") {
            PageClass::EnumValue
        } else if path.contains("/properties/") {
            PageClass::ObjectProperty
        } else if path.contains("/ctors/") {
            PageClass::Constructor
        } else if path.starts_with("objects/catalog2/") {
            PageClass::Enum
        } else if path.starts_with("objects/catalog") {
            PageClass::ObjectType
        } else if !page.children.is_empty() {
            PageClass::Catalog
        } else {
            PageClass::Unknown
        }
    }

    fn is_catalog_path(path: &str) -> bool {
        path.rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("catalog") && name.ends_with(".html"))
    }

    fn source_from_page(
        hbk_path: &Path,
        locale: &str,
        flat_page: &FlatTocPage<'_>,
        content: &PageContent,
    ) -> SyntaxHelperSource {
        SyntaxHelperSource {
            hbk_path: hbk_path.to_path_buf(),
            locale: locale.to_string(),
            toc_path: Some(flat_page.index_path.to_string()),
            html_path: flat_page.page.html_path.clone(),
            page_title: if content.title.is_empty() {
                flat_page.page.title.display().to_string()
            } else {
                content.title.clone()
            },
        }
    }

    fn source_from_toc(
        hbk_path: &Path,
        locale: &str,
        flat_page: &FlatTocPage<'_>,
    ) -> SyntaxHelperSource {
        SyntaxHelperSource {
            hbk_path: hbk_path.to_path_buf(),
            locale: locale.to_string(),
            toc_path: Some(flat_page.index_path.to_string()),
            html_path: flat_page.page.html_path.clone(),
            page_title: flat_page.page.title.display().to_string(),
        }
    }

    fn unknown_page_diagnostic(source: SyntaxHelperSource) -> SyntaxHelperDiagnostic {
        SyntaxHelperDiagnostic {
            severity: DiagnosticSeverity::Warning,
            code: "UNKNOWN_PAGE_CLASS",
            source,
            parser_stage: "root_discovery",
            message: "Syntax Assistant page could not be classified for traversal".to_string(),
        }
    }

    fn normalized_title(page: &TocPage) -> String {
        normalized_text(page.title.display())
    }

    fn normalized_text(value: &str) -> String {
        value.trim().to_lowercase()
    }

    fn source_from_content(
        fallback: &SyntaxHelperSource,
        content: &PageContent,
    ) -> SyntaxHelperSource {
        SyntaxHelperSource {
            hbk_path: content.source.hbk_path.clone(),
            locale: content.source.locale.clone(),
            toc_path: content
                .source
                .toc_path
                .clone()
                .or_else(|| fallback.toc_path.clone()),
            html_path: content.source.html_path.clone(),
            page_title: if content.title.is_empty() {
                fallback.page_title.clone()
            } else {
                content.title.clone()
            },
        }
    }

    fn parse_syntax_page_content(
        hbk_path: &Path,
        locale: &str,
        toc: &Toc,
        html_path: &str,
        raw_html: &str,
    ) -> PageContent {
        let normalized_page_path = html_path.trim_start_matches('/').to_string();
        let toc_page = toc
            .flat_pages()
            .find(|flat_page| flat_page.page.html_path == normalized_page_path);
        let toc_path = toc_page
            .as_ref()
            .map(|flat_page| flat_page.index_path.to_string());
        let toc_title = toc_page
            .as_ref()
            .map(|flat_page| flat_page.page.title.display().to_string());
        let title = select_first_html_text(raw_html, ".V8SH_pagetitle")
            .or_else(|| select_first_html_text(raw_html, "title"))
            .or_else(|| toc_title.clone())
            .unwrap_or_default();
        let body_text = body_text(raw_html);
        let text_preview = body_text.chars().take(240).collect();

        PageContent {
            source: PageSource {
                hbk_path: hbk_path.to_path_buf(),
                locale: locale.to_string(),
                toc_path,
                html_path: normalized_page_path,
                toc_title,
            },
            title,
            raw_html: raw_html.to_string(),
            body_text,
            text_preview,
            links: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn page_title_name(content: &PageContent) -> LocalizedName {
        name_from_text(
            &select_first_html_text(&content.raw_html, ".V8SH_pagetitle")
                .unwrap_or_else(|| content.title.clone()),
        )
    }

    fn title_name(content: &PageContent) -> LocalizedName {
        name_from_text(
            &select_first_html_text(&content.raw_html, ".V8SH_title")
                .unwrap_or_else(|| content.title.clone()),
        )
    }

    fn heading_name(content: &PageContent) -> LocalizedName {
        name_from_text(
            &select_first_html_text(&content.raw_html, ".V8SH_heading")
                .unwrap_or_else(|| content.title.clone()),
        )
    }

    fn name_from_text(value: &str) -> LocalizedName {
        let value = value.trim();
        if let Some((primary, alias)) = split_parenthesized_alias(value) {
            LocalizedName {
                primary,
                alias: Some(alias),
            }
        } else {
            LocalizedName {
                primary: value.to_string(),
                alias: None,
            }
        }
    }

    fn split_parenthesized_alias(value: &str) -> Option<(String, String)> {
        let value = value.trim();
        let alias_end = value.strip_suffix(')')?;
        let alias_start = alias_end.rfind(" (")?;
        let primary = alias_end[..alias_start].trim();
        let alias = alias_end[alias_start + 2..].trim();
        (!primary.is_empty() && !alias.is_empty()).then(|| (primary.to_string(), alias.to_string()))
    }

    fn select_first_html_text(raw_html: &str, selector: &str) -> Option<String> {
        if let Some(class_name) = selector.strip_prefix('.') {
            return select_first_class_text(raw_html, class_name);
        }
        if selector == "title" {
            return select_first_tag_text(raw_html, "title");
        }
        let document = Html::parse_document(raw_html);
        let selector = Selector::parse(selector).expect("static selector must be valid");
        document
            .select(&selector)
            .find_map(|element| non_empty_text(element.text()))
    }

    fn body_text(raw_html: &str) -> String {
        let body = raw_html
            .find("<body")
            .and_then(|start| raw_html[start..].find('>').map(|offset| start + offset + 1))
            .and_then(|start| {
                raw_html[start..]
                    .find("</body>")
                    .map(|end| &raw_html[start..start + end])
            })
            .unwrap_or(raw_html);
        text_from_html_fragment(body)
    }

    fn select_first_class_text(raw_html: &str, class_name: &str) -> Option<String> {
        let class_marker = format!("class=\"{class_name}\"");
        let start = raw_html.find(&class_marker)?;
        let tag_start = raw_html[..start].rfind('<')?;
        let content_start = raw_html[start..]
            .find('>')
            .map(|offset| start + offset + 1)?;
        let tag_name = raw_html[tag_start + 1..]
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_start_matches('/');
        let end_tag = format!("</{tag_name}>");
        let content_end = raw_html[content_start..]
            .find(&end_tag)
            .map(|offset| content_start + offset)?;
        let text = text_from_html_fragment(&raw_html[content_start..content_end]);
        (!text.is_empty()).then_some(text)
    }

    fn select_first_tag_text(raw_html: &str, tag_name: &str) -> Option<String> {
        let start_tag = format!("<{tag_name}");
        let start = raw_html.find(&start_tag)?;
        let content_start = raw_html[start..]
            .find('>')
            .map(|offset| start + offset + 1)?;
        let end_tag = format!("</{tag_name}>");
        let content_end = raw_html[content_start..]
            .find(&end_tag)
            .map(|offset| content_start + offset)?;
        let text = text_from_html_fragment(&raw_html[content_start..content_end]);
        (!text.is_empty()).then_some(text)
    }

    fn text_from_html_fragment(fragment: &str) -> String {
        let mut output = String::new();
        let mut in_tag = false;
        let mut entity = String::new();
        let mut in_entity = false;
        let mut chars = fragment.chars().peekable();
        while let Some(ch) = chars.next() {
            if in_tag {
                if ch == '>' {
                    in_tag = false;
                    output.push(' ');
                }
                continue;
            }
            if in_entity {
                if ch == ';' {
                    output.push_str(decode_entity(&entity));
                    entity.clear();
                    in_entity = false;
                } else {
                    entity.push(ch);
                }
                continue;
            }
            match ch {
                '<' if chars
                    .peek()
                    .is_some_and(|next| next.is_ascii_alphabetic() || *next == '/') =>
                {
                    in_tag = true
                }
                '<' => output.push('<'),
                '&' => in_entity = true,
                ch if ch.is_whitespace() => output.push(' '),
                ch => output.push(ch),
            }
        }
        output.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn text_lines_from_html_fragment(fragment: &str) -> String {
        let with_breaks = fragment
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("</p>", "\n")
            .replace("</div>", "\n");
        with_breaks
            .lines()
            .map(text_from_html_fragment)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn anchor_links(section_html: &str, current_html_path: &str) -> Vec<MemberLink> {
        let mut links = Vec::new();
        let mut rest = section_html;
        while let Some(anchor_start) = rest.find("<a ") {
            rest = &rest[anchor_start..];
            let Some(tag_end) = rest.find('>') else {
                break;
            };
            let tag = &rest[..tag_end + 1];
            let Some(raw_href) = attr_value(tag, "href") else {
                rest = &rest[tag_end + 1..];
                continue;
            };
            let Some(anchor_end) = rest[tag_end + 1..].find("</a>") else {
                break;
            };
            let inner = &rest[tag_end + 1..tag_end + 1 + anchor_end];
            let text = text_from_html_fragment(inner);
            if !text.is_empty() {
                links.push(MemberLink {
                    name: name_from_text(&text),
                    html_path: normalize_member_href(current_html_path, &raw_href),
                });
            }
            rest = &rest[tag_end + 1 + anchor_end + 4..];
        }
        links
    }

    fn attr_value(tag: &str, attr_name: &str) -> Option<String> {
        let attr = format!("{attr_name}=\"");
        let start = tag.find(&attr)? + attr.len();
        let end = tag[start..].find('"')?;
        Some(tag[start..start + end].to_string())
    }

    fn bracketed_name_ranges(section: &str) -> Vec<(usize, usize, String)> {
        let mut ranges = Vec::new();
        let mut offset = 0;
        while let Some(start) = section[offset..].find('<').map(|start| offset + start) {
            let Some(end) = section[start + 1..].find('>').map(|end| start + 1 + end) else {
                break;
            };
            ranges.push((start, end + 1, section[start + 1..end].to_string()));
            offset = end + 1;
        }
        ranges
    }

    fn decode_entity(entity: &str) -> &str {
        match entity {
            "lt" => "<",
            "gt" => ">",
            "amp" => "&",
            "quot" => "\"",
            "nbsp" => " ",
            _ => "",
        }
    }

    fn links_in_section(content: &PageContent, labels: &[&str]) -> Vec<MemberLink> {
        let Some(section_html) = section_html(&content.raw_html, labels) else {
            return Vec::new();
        };
        anchor_links(&section_html, &content.source.html_path)
    }

    fn parse_signatures(content: &PageContent) -> Vec<Signature> {
        let Some(section_html) = section_html(&content.raw_html, &["Синтаксис:", "Syntax:"])
        else {
            return Vec::new();
        };
        let parameters = parse_parameters(content);
        text_lines_from_html_fragment(&section_html)
            .split('\n')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| Signature {
                text: line.to_string(),
                parameters: parameters_for_signature(line, &parameters),
            })
            .collect()
    }

    fn parse_parameters(content: &PageContent) -> Vec<Parameter> {
        let Some(section) = section_text(content, &["Параметры:", "Parameters:"]) else {
            return Vec::new();
        };
        let ranges = bracketed_name_ranges(&section);
        ranges
            .iter()
            .enumerate()
            .filter_map(|(index, (_start, end, name))| {
                if name.trim().is_empty() {
                    return None;
                }
                let next_start = ranges
                    .get(index + 1)
                    .map(|(next_start, _, _)| *next_start)
                    .unwrap_or(section.len());
                let parameter_text = &section[*end..next_start];
                let lower = parameter_text.to_lowercase();
                let required = !(lower.contains("необязательный") || lower.contains("optional"));
                let type_refs = parse_type_refs(parameter_text);
                let description = parameter_text
                    .split_once('.')
                    .map(|(_, tail)| tail.trim())
                    .filter(|tail| !tail.is_empty())
                    .map(ToOwned::to_owned);
                Some(Parameter {
                    name: name.trim().to_string(),
                    required,
                    type_refs: type_refs.clone(),
                    description,
                })
            })
            .collect()
    }

    fn parameters_for_signature(signature: &str, parameters: &[Parameter]) -> Vec<Parameter> {
        parameters
            .iter()
            .filter(|parameter| signature.contains(&format!("<{}>", parameter.name)))
            .cloned()
            .collect()
    }

    fn type_refs_from_section(content: &PageContent, labels: &[&str]) -> Vec<TypeRef> {
        section_text(content, labels)
            .map(|section| parse_type_refs(&section))
            .unwrap_or_default()
    }

    fn parse_type_refs(section: &str) -> Vec<TypeRef> {
        let Some((_, after_type)) = section.split_once("Тип:") else {
            return Vec::new();
        };
        let type_part = after_type
            .split_once('.')
            .map(|(head, _)| head)
            .unwrap_or(after_type);
        type_part
            .split([',', ';'])
            .map(|value| value.trim().trim_matches('.'))
            .filter(|value| !value.is_empty())
            .map(|value| TypeRef {
                name: value.to_string(),
            })
            .collect()
    }

    fn section_text(content: &PageContent, labels: &[&str]) -> Option<String> {
        let body = &content.body_text;
        let (label, start) = find_label(body, labels)?;
        let section_start = start + label.len();
        let section_end = ALL_SECTION_LABELS
            .iter()
            .filter(|candidate| **candidate != label)
            .filter_map(|candidate| {
                body[section_start..]
                    .find(candidate)
                    .map(|index| section_start + index)
            })
            .min()
            .unwrap_or(body.len());
        let value = body[section_start..section_end].trim();
        (!value.is_empty()).then(|| value.to_string())
    }

    fn section_html(raw_html: &str, labels: &[&str]) -> Option<String> {
        let (label, start) = find_label(raw_html, labels)?;
        let chapter_end = raw_html[start..]
            .find("</p>")
            .map(|index| start + index + 4)?;
        let section_end = ALL_SECTION_LABELS
            .iter()
            .filter(|candidate| **candidate != label)
            .filter_map(|candidate| {
                raw_html[chapter_end..]
                    .find(candidate)
                    .map(|index| chapter_end + index)
            })
            .min()
            .unwrap_or(raw_html.len());
        Some(raw_html[chapter_end..section_end].to_string())
    }

    fn find_label<'a>(value: &str, labels: &'a [&str]) -> Option<(&'a str, usize)> {
        labels
            .iter()
            .filter_map(|label| value.find(label).map(|index| (*label, index)))
            .min_by_key(|(_, index)| *index)
    }

    fn normalize_member_href(current_html_path: &str, href: &str) -> String {
        let without_scheme = href
            .strip_prefix("v8help://SyntaxHelperContext/")
            .or_else(|| href.strip_prefix("v8help://"))
            .unwrap_or(href);
        let path = without_scheme.split(['#', '?']).next().unwrap_or_default();
        if path.starts_with('/') || path.starts_with("objects/") {
            return path.trim_start_matches('/').to_string();
        }
        let base = current_html_path
            .rsplit_once('/')
            .map(|(base, _)| base)
            .unwrap_or("");
        if base.is_empty() {
            path.to_string()
        } else {
            format!("{base}/{path}")
        }
    }

    fn non_empty_text<'a>(parts: impl Iterator<Item = &'a str>) -> Option<String> {
        let text = parts.collect::<Vec<_>>().join(" ");
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        (!text.is_empty()).then_some(text)
    }

    const ALL_SECTION_LABELS: &[&str] = &[
        "Свойства:",
        "Properties:",
        "Методы:",
        "Methods:",
        "События:",
        "Events:",
        "Синтаксис:",
        "Syntax:",
        "Параметры:",
        "Parameters:",
        "Возвращаемое значение:",
        "Return value:",
        "Использование:",
        "Use:",
        "Значения",
        "Values",
        "Элементы коллекции:",
        "Collection items:",
        "Конструкторы:",
        "Constructors:",
        "Описание:",
        "Description:",
        "Примечание:",
        "Note:",
        "Использование в версии:",
        "Available since:",
    ];

    #[cfg(test)]
    mod tests {
        use std::collections::BTreeSet;
        use std::path::{Path, PathBuf};

        use super::*;
        use crate::hbk::book::HbkBook;
        use crate::hbk::docs::parse_page_html;
        use crate::hbk::toc::Toc;

        #[test]
        fn discovers_roots_and_traverses_catalogs_from_fixture_toc() {
            let toc = fixture_toc();
            let discovery =
                discover_roots_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
                    Ok(fixture_content(&toc, html_path))
                })
                .expect("root discovery must succeed");

            assert!(discovery.has_kind(RootSectionKind::GlobalContext));
            assert!(discovery.has_kind(RootSectionKind::EnumCatalog));
            assert!(discovery.has_kind(RootSectionKind::TypeObjectCatalog));
            assert_eq!(discovery.roots.len(), 3);

            let classes = discovery
                .roots
                .iter()
                .flat_map(|root| root.pages.iter().map(|page| page.class))
                .collect::<BTreeSet<_>>();
            assert!(classes.contains(&PageClass::GlobalMethod));
            assert!(classes.contains(&PageClass::GlobalProperty));
            assert!(classes.contains(&PageClass::Enum));
            assert!(classes.contains(&PageClass::EnumValue));
            assert!(classes.contains(&PageClass::ObjectType));
            assert!(classes.contains(&PageClass::ObjectMethod));
            assert!(classes.contains(&PageClass::ObjectProperty));
            assert!(classes.contains(&PageClass::Constructor));

            assert_eq!(discovery.diagnostics.len(), 1);
            assert_eq!(discovery.diagnostics[0].code, "UNKNOWN_PAGE_CLASS");
            assert_eq!(
                discovery.diagnostics[0].severity,
                DiagnosticSeverity::Warning
            );
            assert_eq!(
                discovery.diagnostics[0].source.hbk_path,
                Path::new("shcntx_ru.hbk")
            );
            assert_eq!(discovery.diagnostics[0].source.locale, "ru");
            assert_eq!(
                discovery.diagnostics[0].source.toc_path.as_deref(),
                Some("3")
            );
            assert_eq!(
                discovery.diagnostics[0].source.html_path,
                "objects/unknown.html"
            );
            assert_eq!(
                discovery.diagnostics[0].source.page_title,
                "Неизвестный раздел"
            );
            assert_eq!(discovery.diagnostics[0].parser_stage, "root_discovery");
        }

        #[test]
        fn parses_representative_specialized_fixture_pages() {
            let toc = fixture_toc();

            let global_context = parse_global_context(
                &fixture_content(&toc, "objects/Global context.html"),
                source("objects/Global context.html"),
            );
            assert_eq!(global_context.name.primary, "Глобальный контекст");
            assert!(
                global_context
                    .method_links
                    .iter()
                    .any(|link| link.name.primary == "XMLСтрока"
                        && link.name.alias.as_deref() == Some("XMLString"))
            );
            assert!(
                global_context
                    .property_links
                    .iter()
                    .any(|link| link.name.primary == "WebSocketКлиентСоединения")
            );

            let global_method = parse_global_method(
                &fixture_content(
                    &toc,
                    "objects/Global context/methods/catalog1566/XMLString1567.html",
                ),
                source("objects/Global context/methods/catalog1566/XMLString1567.html"),
            );
            assert_eq!(global_method.name.primary, "XMLСтрока");
            assert_eq!(global_method.name.alias.as_deref(), Some("XMLString"));
            assert_eq!(global_method.signatures[0].text, "XMLСтрока(<Значение>)");
            assert!(global_method.signatures[0].parameters[0].required);
            assert!(
                global_method
                    .return_types
                    .iter()
                    .any(|type_ref| type_ref.name == "Строка")
            );

            let global_property = parse_global_property(
                &fixture_content(&toc, "objects/Global context/properties/Catalogs336.html"),
                source("objects/Global context/properties/Catalogs336.html"),
            );
            assert_eq!(global_property.name.primary, "Справочники");
            assert_eq!(global_property.name.alias.as_deref(), Some("Catalogs"));
            assert_eq!(global_property.usage.as_deref(), Some("Только чтение."));
            assert!(
                global_property
                    .type_refs
                    .iter()
                    .any(|type_ref| type_ref.name == "СправочникиМенеджер")
            );

            let platform_type = parse_platform_type(
                &fixture_content(&toc, "objects/catalog234/Array.html"),
                source("objects/catalog234/Array.html"),
            );
            assert_eq!(platform_type.name.primary, "Массив");
            assert!(
                platform_type
                    .method_links
                    .iter()
                    .any(|link| link.name.alias.as_deref() == Some("Add"))
            );
            assert!(
                platform_type
                    .constructor_links
                    .iter()
                    .any(|link| link.name.primary == "По количеству элементов")
            );

            let method = parse_platform_method(
                &fixture_content(&toc, "objects/catalog234/Array/methods/Add772.html"),
                source("objects/catalog234/Array/methods/Add772.html"),
            );
            assert_eq!(method.owner.primary, "Массив");
            assert_eq!(method.name.primary, "Добавить");
            assert!(!method.signatures[0].parameters[0].required);

            let property = parse_platform_property(
                &fixture_content(
                    &toc,
                    "objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html",
                ),
                source("objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html"),
            );
            assert_eq!(property.owner.primary, "ГруппаФормы");
            assert_eq!(property.name.alias.as_deref(), Some("Visible"));
            assert!(
                property
                    .type_refs
                    .iter()
                    .any(|type_ref| type_ref.name == "Булево")
            );

            let constructor = parse_constructor(
                &fixture_content(&toc, "objects/catalog234/Array/ctors/ctor13.html"),
                source("objects/catalog234/Array/ctors/ctor13.html"),
            );
            assert_eq!(constructor.owner.primary, "Массив");
            assert_eq!(constructor.name.primary, "По количеству элементов");
            assert_eq!(
                constructor.signatures[0].text,
                "Новый Массив(<КоличествоЭлементов1>,...,<КоличествоЭлементовN>)"
            );

            let enum_definition = parse_enum(
                &fixture_content(&toc, "objects/catalog2/catalog2300/JSONValueType.html"),
                source("objects/catalog2/catalog2300/JSONValueType.html"),
            );
            assert_eq!(enum_definition.name.primary, "ТипЗначенияJSON");
            assert!(
                enum_definition
                    .value_links
                    .iter()
                    .any(|link| link.name.alias.as_deref() == Some("ArrayEnd"))
            );

            let enum_value = parse_enum_value(
                &fixture_content(
                    &toc,
                    "objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html",
                ),
                source("objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html"),
            );
            assert_eq!(enum_value.owner.primary, "ТипЗначенияJSON");
            assert_eq!(enum_value.name.primary, "КонецМассива");
            assert!(
                enum_value
                    .description
                    .as_deref()
                    .is_some_and(|text| text.contains("JSON"))
            );
        }

        #[test]
        fn extracts_platform_context_from_fixture_toc() {
            let toc = fixture_toc();
            let context =
                extract_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
                    Ok(fixture_content(&toc, html_path))
                })
                .expect("fixture extraction must succeed");

            assert_eq!(context.global_contexts.len(), 1);
            assert!(
                context
                    .global_methods
                    .iter()
                    .any(|method| method.name.alias.as_deref() == Some("XMLString"))
            );
            assert!(
                context
                    .global_properties
                    .iter()
                    .any(|property| property.name.alias.as_deref() == Some("Catalogs"))
            );
            assert!(
                context
                    .platform_types
                    .iter()
                    .any(|platform_type| platform_type.name.alias.as_deref() == Some("Array"))
            );
            assert!(
                context
                    .type_methods
                    .iter()
                    .any(|method| method.name.alias.as_deref() == Some("Add"))
            );
            assert!(
                context
                    .type_properties
                    .iter()
                    .any(|property| property.name.alias.as_deref() == Some("Visible"))
            );
            assert!(
                context
                    .constructors
                    .iter()
                    .any(|constructor| constructor.name.primary == "По количеству элементов")
            );
            assert!(
                context
                    .enums
                    .iter()
                    .any(|enum_definition| enum_definition.name.alias.as_deref()
                        == Some("JSONValueType"))
            );
            assert!(
                context
                    .enum_values
                    .iter()
                    .any(|enum_value| enum_value.name.alias.as_deref() == Some("ArrayEnd"))
            );
            assert_eq!(context.diagnostics.len(), 1);
        }

        #[test]
        fn binds_parameters_to_the_signature_that_mentions_them() {
            let toc = fixture_toc();
            let html = r#"
                <html><body>
                <h1 class="V8SH_pagetitle">Тест.Метод</h1>
                <p class="V8SH_title">Тест</p>
                <p class="V8SH_heading">Метод</p>
                <p class="V8SH_chapter">Синтаксис:</p>
                Метод()<br>
                Метод(&lt;СтрокаЗначение&gt;, &lt;ЧислоЗначение&gt;)
                <p class="V8SH_chapter">Параметры:</p>
                <div class="V8SH_rubric"><p>&lt;СтрокаЗначение&gt; (обязательный)</p></div>
                Тип: Строка. Первый параметр.
                <div class="V8SH_rubric"><p>&lt;ЧислоЗначение&gt; (необязательный)</p></div>
                Тип: Число. Второй параметр.
                </body></html>
            "#;
            let content = parse_syntax_page_content(
                Path::new("shcntx_ru.hbk"),
                "ru",
                &toc,
                "objects/catalog234/Test/methods/Method.html",
                html,
            );
            let signatures = parse_signatures(&content);

            assert_eq!(signatures.len(), 2);
            assert!(signatures[0].parameters.is_empty());
            assert_eq!(signatures[1].parameters.len(), 2);
            assert_eq!(signatures[1].parameters[0].name, "СтрокаЗначение");
            assert_eq!(signatures[1].parameters[1].name, "ЧислоЗначение");
            assert!(signatures[1].parameters[0].required);
            assert!(!signatures[1].parameters[1].required);
            assert_eq!(signatures[1].parameters[0].type_refs[0].name, "Строка");
            assert_eq!(signatures[1].parameters[1].type_refs[0].name, "Число");
        }

        #[test]
        fn real_shcntx_ru_root_discovery_includes_required_root_candidates_when_fixture_exists() {
            let path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk");
            if !path.exists() {
                eprintln!(
                    "real-platform root discovery smoke skipped because {} is unavailable",
                    path.display()
                );
                return;
            }

            let book = HbkBook::open(path).expect("real Syntax Assistant book must open");
            let discovery = SyntaxHelperReader::new(&book)
                .discover_roots()
                .expect("real Syntax Assistant roots must be discoverable");

            assert!(discovery.has_kind(RootSectionKind::GlobalContext));
            assert!(discovery.has_kind(RootSectionKind::EnumCatalog));
            assert!(discovery.has_kind(RootSectionKind::TypeObjectCatalog));

            let global_context = discovery
                .roots
                .iter()
                .find(|root| root.kind == RootSectionKind::GlobalContext)
                .expect("global context root must be present");
            assert_eq!(
                global_context.source.html_path,
                "objects/Global context.html"
            );
            assert!(
                global_context
                    .pages
                    .iter()
                    .any(|page| page.class == PageClass::GlobalMethod)
            );
            assert!(
                global_context
                    .pages
                    .iter()
                    .any(|page| page.class == PageClass::GlobalProperty)
            );

            let enum_catalog = discovery
                .roots
                .iter()
                .find(|root| root.kind == RootSectionKind::EnumCatalog)
                .expect("enum catalog root must be present");
            assert!(
                enum_catalog
                    .pages
                    .iter()
                    .any(|page| page.class == PageClass::Enum)
            );
            assert!(
                enum_catalog
                    .pages
                    .iter()
                    .any(|page| page.class == PageClass::EnumValue)
            );

            let type_catalog = discovery
                .roots
                .iter()
                .find(|root| {
                    root.kind == RootSectionKind::TypeObjectCatalog
                        && root.source.html_path == "objects/catalog234.html"
                })
                .expect("known type/object catalog root must be present");
            assert!(
                type_catalog
                    .pages
                    .iter()
                    .any(|page| page.class == PageClass::ObjectType)
            );
        }

        #[test]
        fn real_shcntx_ru_extraction_returns_required_families_when_fixture_exists() {
            let path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk");
            if !path.exists() {
                eprintln!(
                    "real-platform Syntax Assistant extraction smoke skipped because {} is unavailable",
                    path.display()
                );
                return;
            }

            let book = HbkBook::open(path).expect("real Syntax Assistant book must open");
            let context = SyntaxHelperReader::new(&book)
                .extract()
                .expect("real Syntax Assistant extraction must succeed");

            assert!(!context.global_methods.is_empty());
            assert!(!context.global_properties.is_empty());
            assert!(!context.platform_types.is_empty());
            assert!(!context.enums.is_empty());
        }

        #[test]
        fn real_shcntx_root_root_discovery_includes_required_root_candidates_when_fixture_exists() {
            let path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk");
            if !path.exists() {
                eprintln!(
                    "real-platform root discovery smoke skipped because {} is unavailable",
                    path.display()
                );
                return;
            }

            let book = HbkBook::open(path).expect("real root Syntax Assistant book must open");
            let discovery = SyntaxHelperReader::new(&book)
                .discover_roots()
                .expect("real root Syntax Assistant roots must be discoverable");

            assert!(discovery.has_kind(RootSectionKind::GlobalContext));
            assert!(discovery.has_kind(RootSectionKind::EnumCatalog));
            assert!(discovery.has_kind(RootSectionKind::TypeObjectCatalog));

            let enum_catalog = discovery
                .roots
                .iter()
                .find(|root| root.kind == RootSectionKind::EnumCatalog)
                .expect("root-source enum catalog root must be present");
            assert_eq!(enum_catalog.source.html_path, "objects/catalog2.html");
            assert!(
                enum_catalog
                    .pages
                    .iter()
                    .any(|page| page.class == PageClass::Enum)
            );
        }

        fn fixture_toc() -> Toc {
            Toc::parse(
                r#"{
                    14
                    {1,0,2,2,3,{0,0,{0,0,{"ru","Глобальный контекст"}},"/objects/Global context.html"}}
                    {2,1,1,4,{0,0,{0,0,{"ru","Свойства"}},"/objects/Global context/properties/catalog.html"}}
                    {3,1,1,5,{0,0,{0,0,{"ru","Методы"}},"/objects/Global context/methods/catalog.html"}}
                    {4,2,0,{0,0,{0,0,{"ru","Глобальный контекст.Справочники"}},"/objects/Global context/properties/Catalogs336.html"}}
                    {5,3,0,{0,0,{0,0,{"ru","Глобальный контекст.XMLСтрока"}},"/objects/Global context/methods/catalog1566/XMLString1567.html"}}
                    {6,0,1,7,{0,0,{0,0,{"ru","Системные перечисления"}},"/objects/catalog2.html"}}
                    {7,6,1,8,{0,0,{0,0,{"ru","ТипЗначенияJSON"}},"/objects/catalog2/catalog2300/JSONValueType.html"}}
                    {8,7,0,{0,0,{0,0,{"ru","ТипЗначенияJSON.КонецМассива"}},"/objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html"}}
                    {9,0,1,10,{0,0,{0,0,{"ru","Универсальные коллекции значений"}},"/objects/catalog234.html"}}
                    {10,9,3,11,12,13,{0,0,{0,0,{"ru","Массив"}},"/objects/catalog234/Array.html"}}
                    {11,10,0,{0,0,{0,0,{"ru","Массив.Добавить"}},"/objects/catalog234/Array/methods/Add772.html"}}
                    {12,10,0,{0,0,{0,0,{"ru","Массив.По количеству элементов"}},"/objects/catalog234/Array/ctors/ctor13.html"}}
                    {13,10,0,{0,0,{0,0,{"ru","ГруппаФормы.Видимость"}},"/objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html"}}
                    {14,0,0,{0,0,{0,0,{"ru","Неизвестный раздел"}},"/objects/unknown.html"}}
                }"#,
            )
            .expect("fixture TOC must parse")
        }

        fn fixture_content(toc: &Toc, html_path: &str) -> PageContent {
            let html = match html_path {
                "objects/Global context.html" => {
                    include_str!("../tests/fixtures/syntax-helper/global_context_ru.html")
                }
                "objects/Global context/properties/Catalogs336.html" => {
                    include_str!("../tests/fixtures/syntax-helper/global_property_catalogs_ru.html")
                }
                "objects/Global context/methods/catalog1566/XMLString1567.html" => {
                    include_str!("../tests/fixtures/syntax-helper/global_method_xmlstring_ru.html")
                }
                "objects/catalog2.html" => {
                    include_str!("../tests/fixtures/syntax-helper/root_catalog_enums_ru.html")
                }
                "objects/catalog2/catalog2300/JSONValueType.html" => {
                    include_str!("../tests/fixtures/syntax-helper/enum_json_value_type_ru.html")
                }
                "objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html" => {
                    include_str!(
                        "../tests/fixtures/syntax-helper/enum_value_json_array_end_ru.html"
                    )
                }
                "objects/catalog234.html" => {
                    include_str!("../tests/fixtures/syntax-helper/root_catalog_types_ru.html")
                }
                "objects/catalog234/Array.html" => {
                    include_str!("../tests/fixtures/syntax-helper/object_array_ru.html")
                }
                "objects/catalog234/Array/methods/Add772.html" => {
                    include_str!("../tests/fixtures/syntax-helper/object_method_array_add_ru.html")
                }
                "objects/catalog234/Array/ctors/ctor13.html" => {
                    include_str!(
                        "../tests/fixtures/syntax-helper/constructor_array_by_count_ru.html"
                    )
                }
                "objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html" => {
                    include_str!(
                        "../tests/fixtures/syntax-helper/object_property_formgroup_visible_ru.html"
                    )
                }
                "objects/unknown.html" => {
                    r#"<html><body><h1 class="V8SH_pagetitle">Неизвестный раздел</h1></body></html>"#
                }
                other => panic!("unexpected fixture page load: {other}"),
            };
            parse_page_html(
                Path::new("shcntx_ru.hbk"),
                "ru",
                toc,
                html_path,
                html,
                |_| false,
            )
        }

        fn source(html_path: &str) -> SyntaxHelperSource {
            SyntaxHelperSource {
                hbk_path: PathBuf::from("shcntx_ru.hbk"),
                locale: "ru".to_string(),
                toc_path: None,
                html_path: html_path.to_string(),
                page_title: String::new(),
            }
        }
    }
}

#[cfg(test)]
mod syntax_helper_fixture_tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;

    const MANIFEST: &str = include_str!("../tests/fixtures/syntax-helper/manifest.tsv");

    #[derive(Debug)]
    struct ManifestEntry<'a> {
        parser_kind: &'a str,
        source_hbk: &'a str,
        page_title: &'a str,
        fixture_path: &'a str,
        reason: &'a str,
    }

    #[test]
    fn syntax_assistant_fixture_manifest_covers_required_parser_kinds() {
        let entries = parse_manifest();
        let actual_kinds = entries
            .iter()
            .map(|entry| entry.parser_kind)
            .collect::<BTreeSet<_>>();
        let required_kinds = BTreeSet::from([
            "global_context",
            "global_method",
            "global_property",
            "object_type",
            "object_method",
            "object_property",
            "constructor",
            "enum",
            "enum_value",
            "root_catalog",
        ]);

        assert_eq!(actual_kinds, required_kinds);
        assert!(
            entries
                .iter()
                .any(|entry| entry.source_hbk.ends_with("shcntx_ru.hbk"))
        );
        assert!(
            entries
                .iter()
                .any(|entry| entry.source_hbk.ends_with("shcntx_root.hbk"))
        );
        assert!(
            entries
                .iter()
                .filter(|entry| entry.parser_kind == "root_catalog")
                .count()
                >= 3
        );
        assert!(
            entries
                .iter()
                .filter(|entry| entry.parser_kind == "root_catalog")
                .all(|entry| entry.reason.contains("TOC records")),
            "root/catalog HTML fixtures must document that catalog children are represented by TOC records"
        );
    }

    #[test]
    fn syntax_assistant_fixture_manifest_points_to_real_html_fragments() {
        for entry in parse_manifest() {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(entry.fixture_path);
            let html = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));

            assert!(
                html.contains("<h1 class=\"V8SH_pagetitle\">"),
                "{} must keep the real Syntax Assistant page-title marker",
                entry.fixture_path
            );
            assert!(
                html.contains(entry.page_title),
                "{} must contain manifest page title '{}'",
                entry.fixture_path,
                entry.page_title
            );
            assert!(
                html.contains("V8SH_") || entry.parser_kind == "root_catalog",
                "{} must preserve Syntax Assistant class markers",
                entry.fixture_path
            );
        }
    }

    fn parse_manifest() -> Vec<ManifestEntry<'static>> {
        let mut lines = MANIFEST.lines();
        assert_eq!(
            lines.next(),
            Some("parser_kind\tsource_hbk\thtml_path\tpage_title\tfixture_path\treason")
        );

        lines
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
            .map(|(index, line)| {
                let fields = line.split('\t').collect::<Vec<_>>();
                assert_eq!(
                    fields.len(),
                    6,
                    "manifest line {} must have 6 tab-separated fields",
                    index + 2
                );
                assert!(
                    fields[1].ends_with(".hbk"),
                    "manifest line {} must record source HBK file",
                    index + 2
                );
                assert!(
                    fields[2].ends_with(".html"),
                    "manifest line {} must record HTML path",
                    index + 2
                );
                ManifestEntry {
                    parser_kind: fields[0],
                    source_hbk: fields[1],
                    page_title: fields[3],
                    fixture_path: fields[4],
                    reason: fields[5],
                }
            })
            .collect()
    }
}
