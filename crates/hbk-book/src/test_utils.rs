use std::fs;
use std::io::{self, Cursor, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub fn write_fixture_hbk(path: &Path, entities: Vec<(&str, Option<Vec<u8>>)>) -> io::Result<()> {
    fs::write(path, fixture_container(entities))
}

pub fn zip_bytes(name: &str, body: &[u8]) -> Vec<u8> {
    zip_entries(vec![(name, body)])
}

pub fn zip_entries(entries: Vec<(&str, &[u8])>) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut output);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, body) in entries {
            zip.start_file(name, options).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }
    output.into_inner()
}

pub fn fixture_container(entities: Vec<(&str, Option<Vec<u8>>)>) -> Vec<u8> {
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

fn push_block(bytes: &mut Vec<u8>, payload_size: usize, chunk: &[u8], next: Option<usize>) {
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
