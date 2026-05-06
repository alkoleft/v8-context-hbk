use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::ExportError;
use crate::manifest::SCHEMA_VERSION;

pub(crate) fn write_json_file<T: Serialize>(
    output_dir: &Path,
    file_name: &'static str,
    value: &T,
) -> Result<PathBuf, ExportError> {
    let path = output_dir.join(file_name);
    let file = File::create(&path).map_err(|source| ExportError::Io {
        path: path.clone(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, value).map_err(|source| ExportError::Json {
        path: path.clone(),
        source,
    })?;
    writer.flush().map_err(|source| ExportError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

pub(crate) struct RecordFileWriter {
    path: PathBuf,
    writer: BufWriter<File>,
    first_record: bool,
    finished: bool,
}

pub(crate) fn open_record_file(
    output_dir: &Path,
    files: &mut Vec<PathBuf>,
    file_name: &'static str,
    locale: &str,
    source_locale: &str,
    record_kind: &'static str,
) -> Result<RecordFileWriter, ExportError> {
    let writer =
        RecordFileWriter::create(output_dir, file_name, locale, source_locale, record_kind)?;
    files.push(writer.path().to_path_buf());
    Ok(writer)
}

pub(crate) fn remove_export_files(
    files: impl IntoIterator<Item = PathBuf>,
) -> Result<(), ExportError> {
    for path in files {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(ExportError::Io { path, source });
            }
        }
    }
    Ok(())
}

impl RecordFileWriter {
    fn create(
        output_dir: &Path,
        file_name: &'static str,
        locale: &str,
        source_locale: &str,
        record_kind: &'static str,
    ) -> Result<Self, ExportError> {
        let path = output_dir.join(file_name);
        let file = File::create(&path).map_err(|source| ExportError::Io {
            path: path.clone(),
            source,
        })?;
        let mut writer = Self {
            path,
            writer: BufWriter::new(file),
            first_record: true,
            finished: false,
        };
        writer
            .write_raw(format!("{{\"schema_version\":{SCHEMA_VERSION},\"locale\":").as_bytes())?;
        writer.write_json(locale)?;
        writer.write_raw(b",\"source_locale\":")?;
        writer.write_json(source_locale)?;
        writer.write_raw(b",\"record_kind\":")?;
        writer.write_json(record_kind)?;
        writer.write_raw(b",\"records\":[")?;
        Ok(writer)
    }

    fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn write_record<T: Serialize + ?Sized>(
        &mut self,
        record: &T,
    ) -> Result<(), ExportError> {
        if !self.first_record {
            self.write_raw(b",")?;
        }
        self.write_json(record)?;
        self.first_record = false;
        Ok(())
    }

    pub(crate) fn finish(&mut self) -> Result<(), ExportError> {
        if self.finished {
            return Ok(());
        }
        self.write_raw(b"]}")?;
        self.writer.flush().map_err(|source| ExportError::Io {
            path: self.path.clone(),
            source,
        })?;
        self.finished = true;
        Ok(())
    }

    pub(crate) fn close_unfinished(self) {
        let Self { writer, .. } = self;
        drop(writer);
    }

    fn write_raw(&mut self, bytes: &[u8]) -> Result<(), ExportError> {
        self.writer
            .write_all(bytes)
            .map_err(|source| ExportError::Io {
                path: self.path.clone(),
                source,
            })
    }

    fn write_json<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), ExportError> {
        serde_json::to_writer(&mut self.writer, value).map_err(|source| ExportError::Json {
            path: self.path.clone(),
            source,
        })
    }
}
