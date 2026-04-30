use std::path::{Path, PathBuf};

use super::*;
use crate::block::{
    BLOCK_HEADER_SIZE, CONTAINER_HEADER_SIZE, FILE_DESCRIPTOR_SIZE, SPLITTER, read_u32_le,
};

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
