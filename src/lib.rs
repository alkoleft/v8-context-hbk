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
