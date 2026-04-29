use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

use hbk_book::HbkBook;
use syntax_helper_model::{self as model, PlatformContext, SyntaxHelperSink};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonExporter {
    output_dir: PathBuf,
}

pub type PlatformContextExporter = JsonExporter;

impl JsonExporter {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
        }
    }

    pub fn export_syntax_helper(
        &self,
        book: &HbkBook,
        context: &PlatformContext,
    ) -> Result<JsonExportSummary, ExportError> {
        self.export_platform_context(
            book.locale().export_code(),
            book.locale().source_code(),
            &book.path().display().to_string(),
            context,
        )
    }

    pub fn start_syntax_helper_stream(
        &self,
        book: &HbkBook,
    ) -> Result<StreamingSyntaxHelperExport, ExportError> {
        self.start_platform_context_stream(book.locale().export_code(), book.locale().source_code())
    }

    pub fn start_platform_context_stream(
        &self,
        locale: &str,
        source_locale: &str,
    ) -> Result<StreamingSyntaxHelperExport, ExportError> {
        StreamingSyntaxHelperExport::start(self.output_dir.clone(), locale, source_locale)
    }

    pub fn export_platform_context(
        &self,
        locale: &str,
        source_locale: &str,
        _source_hbk: &str,
        context: &PlatformContext,
    ) -> Result<JsonExportSummary, ExportError> {
        fs::create_dir_all(&self.output_dir).map_err(|source| ExportError::Io {
            path: self.output_dir.clone(),
            source,
        })?;

        let metadata = ExportMetadata {
            schema_version: SCHEMA_VERSION,
            locale,
            source_locale,
            files: EXPORT_FILES.to_vec(),
        };

        let mut files = Vec::new();
        files.push(self.write_file("metadata.json", &metadata)?);
        let global_methods = context
            .global_methods
            .iter()
            .map(ConsumerGlobalMethod::from)
            .collect::<Vec<_>>();
        files.push(self.write_records(
            "global-methods.json",
            locale,
            source_locale,
            "global_method",
            &global_methods,
        )?);
        let global_properties = context
            .global_properties
            .iter()
            .map(ConsumerGlobalProperty::from)
            .collect::<Vec<_>>();
        files.push(self.write_records(
            "global-properties.json",
            locale,
            source_locale,
            "global_property",
            &global_properties,
        )?);
        let platform_types = context
            .platform_types
            .iter()
            .map(ConsumerPlatformType::from)
            .collect::<Vec<_>>();
        files.push(self.write_records(
            "platform-types.json",
            locale,
            source_locale,
            "platform_type",
            &platform_types,
        )?);
        let type_methods = context
            .type_methods
            .iter()
            .map(ConsumerPlatformMethod::from)
            .collect::<Vec<_>>();
        files.push(self.write_records(
            "type-methods.json",
            locale,
            source_locale,
            "type_method",
            &type_methods,
        )?);
        let type_properties = context
            .type_properties
            .iter()
            .map(ConsumerPlatformProperty::from)
            .collect::<Vec<_>>();
        files.push(self.write_records(
            "type-properties.json",
            locale,
            source_locale,
            "type_property",
            &type_properties,
        )?);
        let constructors = context
            .constructors
            .iter()
            .map(ConsumerConstructor::from)
            .collect::<Vec<_>>();
        files.push(self.write_records(
            "constructors.json",
            locale,
            source_locale,
            "constructor",
            &constructors,
        )?);
        let enums = context
            .enums
            .iter()
            .map(ConsumerEnumDefinition::from)
            .collect::<Vec<_>>();
        files.push(self.write_records("enums.json", locale, source_locale, "enum", &enums)?);
        let enum_values = context
            .enum_values
            .iter()
            .map(ConsumerEnumValue::from)
            .collect::<Vec<_>>();
        files.push(self.write_records(
            "enum-values.json",
            locale,
            source_locale,
            "enum_value",
            &enum_values,
        )?);
        files.push(self.write_records(
            "diagnostics.json",
            locale,
            source_locale,
            "diagnostic",
            &context.diagnostics,
        )?);

        Ok(JsonExportSummary {
            output_dir: self.output_dir.clone(),
            locale: locale.to_string(),
            source_locale: source_locale.to_string(),
            files,
            counts: JsonExportCounts::from(context),
        })
    }

    fn write_records<T: Serialize>(
        &self,
        file_name: &'static str,
        locale: &str,
        source_locale: &str,
        record_kind: &'static str,
        records: &[T],
    ) -> Result<PathBuf, ExportError> {
        let envelope = RecordsEnvelope {
            schema_version: SCHEMA_VERSION,
            locale,
            source_locale,
            record_kind,
            records,
        };
        self.write_file(file_name, &envelope)
    }

    fn write_file<T: Serialize>(
        &self,
        file_name: &'static str,
        value: &T,
    ) -> Result<PathBuf, ExportError> {
        write_json_file(&self.output_dir, file_name, value)
    }
}

fn write_json_file<T: Serialize>(
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonExportSummary {
    pub output_dir: PathBuf,
    pub locale: String,
    pub source_locale: String,
    pub files: Vec<PathBuf>,
    pub counts: JsonExportCounts,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JsonExportCounts {
    pub global_contexts: usize,
    pub global_methods: usize,
    pub global_properties: usize,
    pub platform_types: usize,
    pub type_methods: usize,
    pub type_properties: usize,
    pub constructors: usize,
    pub enums: usize,
    pub enum_values: usize,
    pub diagnostics: usize,
}

impl From<&PlatformContext> for JsonExportCounts {
    fn from(context: &PlatformContext) -> Self {
        Self {
            global_contexts: context.global_contexts.len(),
            global_methods: context.global_methods.len(),
            global_properties: context.global_properties.len(),
            platform_types: context.platform_types.len(),
            type_methods: context.type_methods.len(),
            type_properties: context.type_properties.len(),
            constructors: context.constructors.len(),
            enums: context.enums.len(),
            enum_values: context.enum_values.len(),
            diagnostics: context.diagnostics.len(),
        }
    }
}

pub struct StreamingSyntaxHelperExport {
    output_dir: PathBuf,
    locale: String,
    source_locale: String,
    files: Vec<PathBuf>,
    counts: JsonExportCounts,
    global_methods: RecordFileWriter,
    global_properties: RecordFileWriter,
    platform_types: RecordFileWriter,
    type_methods: RecordFileWriter,
    type_properties: RecordFileWriter,
    constructors: RecordFileWriter,
    enums: RecordFileWriter,
    enum_values: RecordFileWriter,
    diagnostics: RecordFileWriter,
}

impl StreamingSyntaxHelperExport {
    fn start(output_dir: PathBuf, locale: &str, source_locale: &str) -> Result<Self, ExportError> {
        fs::create_dir_all(&output_dir).map_err(|source| ExportError::Io {
            path: output_dir.clone(),
            source,
        })?;

        let metadata = ExportMetadata {
            schema_version: SCHEMA_VERSION,
            locale,
            source_locale,
            files: EXPORT_FILES.to_vec(),
        };
        let metadata_path = write_json_file(&output_dir, "metadata.json", &metadata)?;

        let mut files = vec![metadata_path];
        let global_methods = open_record_file(
            &output_dir,
            &mut files,
            "global-methods.json",
            locale,
            source_locale,
            "global_method",
        )?;
        let global_properties = open_record_file(
            &output_dir,
            &mut files,
            "global-properties.json",
            locale,
            source_locale,
            "global_property",
        )?;
        let platform_types = open_record_file(
            &output_dir,
            &mut files,
            "platform-types.json",
            locale,
            source_locale,
            "platform_type",
        )?;
        let type_methods = open_record_file(
            &output_dir,
            &mut files,
            "type-methods.json",
            locale,
            source_locale,
            "type_method",
        )?;
        let type_properties = open_record_file(
            &output_dir,
            &mut files,
            "type-properties.json",
            locale,
            source_locale,
            "type_property",
        )?;
        let constructors = open_record_file(
            &output_dir,
            &mut files,
            "constructors.json",
            locale,
            source_locale,
            "constructor",
        )?;
        let enums = open_record_file(
            &output_dir,
            &mut files,
            "enums.json",
            locale,
            source_locale,
            "enum",
        )?;
        let enum_values = open_record_file(
            &output_dir,
            &mut files,
            "enum-values.json",
            locale,
            source_locale,
            "enum_value",
        )?;
        let diagnostics = open_record_file(
            &output_dir,
            &mut files,
            "diagnostics.json",
            locale,
            source_locale,
            "diagnostic",
        )?;

        Ok(Self {
            output_dir,
            locale: locale.to_string(),
            source_locale: source_locale.to_string(),
            files,
            counts: JsonExportCounts::default(),
            global_methods,
            global_properties,
            platform_types,
            type_methods,
            type_properties,
            constructors,
            enums,
            enum_values,
            diagnostics,
        })
    }

    pub fn finish(mut self) -> Result<JsonExportSummary, ExportError> {
        self.global_methods.finish()?;
        self.global_properties.finish()?;
        self.platform_types.finish()?;
        self.type_methods.finish()?;
        self.type_properties.finish()?;
        self.constructors.finish()?;
        self.enums.finish()?;
        self.enum_values.finish()?;
        self.diagnostics.finish()?;

        Ok(JsonExportSummary {
            output_dir: self.output_dir,
            locale: self.locale,
            source_locale: self.source_locale,
            files: self.files,
            counts: self.counts,
        })
    }

    pub fn abort(self) -> Result<(), ExportError> {
        let Self {
            files,
            global_methods,
            global_properties,
            platform_types,
            type_methods,
            type_properties,
            constructors,
            enums,
            enum_values,
            diagnostics,
            ..
        } = self;

        global_methods.close_unfinished();
        global_properties.close_unfinished();
        platform_types.close_unfinished();
        type_methods.close_unfinished();
        type_properties.close_unfinished();
        constructors.close_unfinished();
        enums.close_unfinished();
        enum_values.close_unfinished();
        diagnostics.close_unfinished();

        remove_export_files(files)
    }
}

impl SyntaxHelperSink for StreamingSyntaxHelperExport {
    type Error = ExportError;

    fn global_context(&mut self, _record: model::GlobalContext) -> Result<(), Self::Error> {
        self.counts.global_contexts += 1;
        Ok(())
    }

    fn global_method(&mut self, record: model::GlobalMethod) -> Result<(), Self::Error> {
        self.global_methods
            .write_record(&ConsumerGlobalMethod::from(&record))?;
        self.counts.global_methods += 1;
        Ok(())
    }

    fn global_property(&mut self, record: model::GlobalProperty) -> Result<(), Self::Error> {
        self.global_properties
            .write_record(&ConsumerGlobalProperty::from(&record))?;
        self.counts.global_properties += 1;
        Ok(())
    }

    fn platform_type(&mut self, record: model::PlatformType) -> Result<(), Self::Error> {
        self.platform_types
            .write_record(&ConsumerPlatformType::from(&record))?;
        self.counts.platform_types += 1;
        Ok(())
    }

    fn type_method(&mut self, record: model::PlatformMethod) -> Result<(), Self::Error> {
        self.type_methods
            .write_record(&ConsumerPlatformMethod::from(&record))?;
        self.counts.type_methods += 1;
        Ok(())
    }

    fn type_property(&mut self, record: model::PlatformProperty) -> Result<(), Self::Error> {
        self.type_properties
            .write_record(&ConsumerPlatformProperty::from(&record))?;
        self.counts.type_properties += 1;
        Ok(())
    }

    fn constructor(&mut self, record: model::Constructor) -> Result<(), Self::Error> {
        self.constructors
            .write_record(&ConsumerConstructor::from(&record))?;
        self.counts.constructors += 1;
        Ok(())
    }

    fn enum_definition(&mut self, record: model::EnumDefinition) -> Result<(), Self::Error> {
        self.enums
            .write_record(&ConsumerEnumDefinition::from(&record))?;
        self.counts.enums += 1;
        Ok(())
    }

    fn enum_value(&mut self, record: model::EnumValue) -> Result<(), Self::Error> {
        self.enum_values
            .write_record(&ConsumerEnumValue::from(&record))?;
        self.counts.enum_values += 1;
        Ok(())
    }

    fn diagnostic(&mut self, record: model::SyntaxHelperDiagnostic) -> Result<(), Self::Error> {
        self.diagnostics.write_record(&record)?;
        self.counts.diagnostics += 1;
        Ok(())
    }
}

struct RecordFileWriter {
    path: PathBuf,
    writer: BufWriter<File>,
    first_record: bool,
    finished: bool,
}

fn open_record_file(
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

fn remove_export_files(files: Vec<PathBuf>) -> Result<(), ExportError> {
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

    fn write_record<T: Serialize + ?Sized>(&mut self, record: &T) -> Result<(), ExportError> {
        if !self.first_record {
            self.write_raw(b",")?;
        }
        self.write_json(record)?;
        self.first_record = false;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), ExportError> {
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

    fn close_unfinished(self) {
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

#[derive(Debug)]
pub enum ExportError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    f,
                    "failed to write JSON export '{}': {source}",
                    path.display()
                )
            }
            Self::Json { path, source } => {
                write!(
                    f,
                    "failed to serialize JSON export '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ExportMetadata<'a> {
    schema_version: u32,
    locale: &'a str,
    source_locale: &'a str,
    files: Vec<ExportFile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExportFile {
    pub file_name: &'static str,
    pub record_kind: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct RecordsEnvelope<'a, T: Serialize> {
    schema_version: u32,
    locale: &'a str,
    source_locale: &'a str,
    record_kind: &'static str,
    records: &'a [T],
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerGlobalMethod<'a> {
    name: &'a model::LocalizedName,
    signatures: &'a [model::Signature],
    return_types: &'a [model::TypeRef],
    description: &'a Option<String>,
}

impl<'a> From<&'a model::GlobalMethod> for ConsumerGlobalMethod<'a> {
    fn from(method: &'a model::GlobalMethod) -> Self {
        Self {
            name: &method.name,
            signatures: &method.signatures,
            return_types: &method.return_types,
            description: &method.description,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerGlobalProperty<'a> {
    name: &'a model::LocalizedName,
    usage: &'a Option<String>,
    type_refs: &'a [model::TypeRef],
    description: &'a Option<String>,
}

impl<'a> From<&'a model::GlobalProperty> for ConsumerGlobalProperty<'a> {
    fn from(property: &'a model::GlobalProperty) -> Self {
        Self {
            name: &property.name,
            usage: &property.usage,
            type_refs: &property.type_refs,
            description: &property.description,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerPlatformType<'a> {
    name: &'a model::LocalizedName,
    description: &'a Option<String>,
}

impl<'a> From<&'a model::PlatformType> for ConsumerPlatformType<'a> {
    fn from(platform_type: &'a model::PlatformType) -> Self {
        Self {
            name: &platform_type.name,
            description: &platform_type.description,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerPlatformMethod<'a> {
    owner: &'a model::LocalizedName,
    name: &'a model::LocalizedName,
    signatures: &'a [model::Signature],
    return_types: &'a [model::TypeRef],
    description: &'a Option<String>,
}

impl<'a> From<&'a model::PlatformMethod> for ConsumerPlatformMethod<'a> {
    fn from(method: &'a model::PlatformMethod) -> Self {
        Self {
            owner: &method.owner,
            name: &method.name,
            signatures: &method.signatures,
            return_types: &method.return_types,
            description: &method.description,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerPlatformProperty<'a> {
    owner: &'a model::LocalizedName,
    name: &'a model::LocalizedName,
    usage: &'a Option<String>,
    type_refs: &'a [model::TypeRef],
    description: &'a Option<String>,
}

impl<'a> From<&'a model::PlatformProperty> for ConsumerPlatformProperty<'a> {
    fn from(property: &'a model::PlatformProperty) -> Self {
        Self {
            owner: &property.owner,
            name: &property.name,
            usage: &property.usage,
            type_refs: &property.type_refs,
            description: &property.description,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerConstructor<'a> {
    owner: &'a model::LocalizedName,
    name: &'a model::LocalizedName,
    signatures: &'a [model::Signature],
    description: &'a Option<String>,
}

impl<'a> From<&'a model::Constructor> for ConsumerConstructor<'a> {
    fn from(constructor: &'a model::Constructor) -> Self {
        Self {
            owner: &constructor.owner,
            name: &constructor.name,
            signatures: &constructor.signatures,
            description: &constructor.description,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerEnumDefinition<'a> {
    name: &'a model::LocalizedName,
    description: &'a Option<String>,
}

impl<'a> From<&'a model::EnumDefinition> for ConsumerEnumDefinition<'a> {
    fn from(enum_definition: &'a model::EnumDefinition) -> Self {
        Self {
            name: &enum_definition.name,
            description: &enum_definition.description,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerEnumValue<'a> {
    owner: &'a model::LocalizedName,
    name: &'a model::LocalizedName,
    description: &'a Option<String>,
}

impl<'a> From<&'a model::EnumValue> for ConsumerEnumValue<'a> {
    fn from(enum_value: &'a model::EnumValue) -> Self {
        Self {
            owner: &enum_value.owner,
            name: &enum_value.name,
            description: &enum_value.description,
        }
    }
}

pub const EXPORT_FILES: &[ExportFile] = &[
    ExportFile {
        file_name: "global-methods.json",
        record_kind: "global_method",
    },
    ExportFile {
        file_name: "global-properties.json",
        record_kind: "global_property",
    },
    ExportFile {
        file_name: "platform-types.json",
        record_kind: "platform_type",
    },
    ExportFile {
        file_name: "type-methods.json",
        record_kind: "type_method",
    },
    ExportFile {
        file_name: "type-properties.json",
        record_kind: "type_property",
    },
    ExportFile {
        file_name: "constructors.json",
        record_kind: "constructor",
    },
    ExportFile {
        file_name: "enums.json",
        record_kind: "enum",
    },
    ExportFile {
        file_name: "enum-values.json",
        record_kind: "enum_value",
    },
    ExportFile {
        file_name: "diagnostics.json",
        record_kind: "diagnostic",
    },
];

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use hbk_book::BookLocale;
    use serde_json::Value;
    use syntax_helper_model as model;

    use super::*;

    #[test]
    fn root_locale_maps_to_export_locale_en() {
        assert_eq!(
            BookLocale::infer_from_path(Path::new("shcntx_root.hbk")).export_code(),
            "en"
        );
    }

    fn source() -> model::SyntaxHelperSource {
        model::SyntaxHelperSource {
            hbk_path: PathBuf::from("/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk"),
            locale: "ru".to_string(),
            toc_path: Some("0.1".to_string()),
            html_path: "objects/Global context/methods/catalog1566/XMLString1567.html".to_string(),
            page_title: "Глобальный контекст.XMLСтрока".to_string(),
        }
    }

    fn name(primary: &str) -> model::LocalizedName {
        model::LocalizedName {
            primary: primary.to_string(),
            alias: None,
        }
    }

    fn link(primary: &str) -> model::MemberLink {
        model::MemberLink {
            name: name(primary),
            html_path: format!("objects/{primary}.html"),
        }
    }

    fn read_json(path: impl AsRef<Path>) -> Value {
        let path = path.as_ref();
        let json = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));
        serde_json::from_str(&json)
            .unwrap_or_else(|error| panic!("{} must be valid JSON: {error}", path.display()))
    }

    fn assert_no_keys(value: &Value, keys: &[&str]) {
        for key in keys {
            assert!(
                value.get(*key).is_none(),
                "field '{key}' must be absent from {value}"
            );
        }
    }

    #[test]
    fn platform_context_serializes_with_source_provenance() {
        let source = source();
        let context = PlatformContext {
            global_methods: vec![model::GlobalMethod {
                name: model::LocalizedName {
                    primary: "XMLСтрока".to_string(),
                    alias: Some("XMLString".to_string()),
                },
                signatures: Vec::new(),
                return_types: Vec::new(),
                description: None,
                source,
            }],
            ..PlatformContext::default()
        };

        let json = serde_json::to_value(&context).expect("context must serialize");
        assert_eq!(
            json["global_methods"][0]["source"]["html_path"],
            "objects/Global context/methods/catalog1566/XMLString1567.html"
        );
        assert_eq!(json["global_methods"][0]["source"]["locale"], "ru");
    }

    #[test]
    fn export_file_manifest_documents_canonical_json_files() {
        let expected = [
            "global-methods.json",
            "global-properties.json",
            "platform-types.json",
            "type-methods.json",
            "type-properties.json",
            "constructors.json",
            "enums.json",
            "enum-values.json",
            "diagnostics.json",
        ];

        assert_eq!(
            EXPORT_FILES
                .iter()
                .map(|file| file.file_name)
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn records_envelope_json_is_parseable_and_non_empty() {
        let dir =
            std::env::temp_dir().join(format!("v8-context-hbk-export-test-{}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("stale export test dir must be removable");
        }
        fs::create_dir_all(&dir).expect("export test dir must be creatable");
        let exporter = JsonExporter::new(&dir);
        let path = exporter
            .write_records(
                "global-methods.json",
                "en",
                "root",
                "global_method",
                &Vec::<Value>::new(),
            )
            .expect("record envelope must be writable");

        let json = fs::read_to_string(&path).expect("record envelope must be readable");
        assert!(!json.is_empty());
        let parsed: Value =
            serde_json::from_str(&json).expect("record envelope must be valid JSON");
        assert_eq!(parsed["locale"], "en");
        assert_eq!(parsed["source_locale"], "root");
        assert!(parsed.get("source_hbk").is_none());

        fs::remove_dir_all(&dir).expect("export test dir must be removable");
    }

    #[test]
    fn exporter_writes_full_canonical_file_set() {
        let dir = std::env::temp_dir().join(format!(
            "v8-context-hbk-export-full-test-{}",
            std::process::id()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("stale export test dir must be removable");
        }

        let summary = JsonExporter::new(&dir)
            .export_platform_context("en", "root", "shcntx_root.hbk", &PlatformContext::default())
            .expect("full canonical export must be writable");

        assert_eq!(summary.files.len(), EXPORT_FILES.len() + 1);
        assert!(!dir.join("global-contexts.json").exists());
        for file in &summary.files {
            let json = fs::read_to_string(file)
                .unwrap_or_else(|error| panic!("{} must be readable: {error}", file.display()));
            assert!(!json.is_empty(), "{} must be non-empty", file.display());
            serde_json::from_str::<Value>(&json)
                .unwrap_or_else(|error| panic!("{} must be valid JSON: {error}", file.display()));
        }

        let metadata = read_json(dir.join("metadata.json"));
        assert_eq!(metadata["locale"], "en");
        assert_eq!(metadata["source_locale"], "root");
        assert!(metadata.get("source_hbk").is_none());
        assert_eq!(
            metadata["files"]
                .as_array()
                .expect("files must be an array")
                .len(),
            EXPORT_FILES.len()
        );

        fs::remove_dir_all(&dir).expect("export test dir must be removable");
    }

    #[test]
    fn exporter_writes_lean_consumer_records_and_diagnostics_source() {
        let dir = std::env::temp_dir().join(format!(
            "v8-context-hbk-export-lean-test-{}",
            std::process::id()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("stale export test dir must be removable");
        }

        let source = source();
        let context = PlatformContext {
            global_methods: vec![model::GlobalMethod {
                name: name("XMLСтрока"),
                signatures: vec![model::Signature {
                    text: "XMLСтрока(Значение)".to_string(),
                    parameters: Vec::new(),
                }],
                return_types: vec![model::TypeRef {
                    name: "Строка".to_string(),
                }],
                description: Some("Creates an XML string.".to_string()),
                source: source.clone(),
            }],
            platform_types: vec![model::PlatformType {
                name: name("Массив"),
                method_links: vec![link("Добавить")],
                constructor_links: vec![link("Массив")],
                description: Some("Array type.".to_string()),
                source: source.clone(),
            }],
            type_methods: vec![model::PlatformMethod {
                owner: name("Массив"),
                name: name("Добавить"),
                signatures: Vec::new(),
                return_types: Vec::new(),
                description: None,
                source: source.clone(),
            }],
            enums: vec![model::EnumDefinition {
                name: name("ТипЗначенияJSON"),
                value_links: vec![link("КонецМассива")],
                description: None,
                source: source.clone(),
            }],
            diagnostics: vec![model::SyntaxHelperDiagnostic {
                severity: model::DiagnosticSeverity::Warning,
                code: "UNKNOWN_PAGE_CLASS",
                source,
                parser_stage: "root_discovery",
                message: "unknown page class".to_string(),
            }],
            ..PlatformContext::default()
        };

        JsonExporter::new(&dir)
            .export_platform_context("ru", "ru", "shcntx_ru.hbk", &context)
            .expect("lean export must be writable");

        assert!(!dir.join("global-contexts.json").exists());
        assert_no_keys(&read_json(dir.join("metadata.json")), &["source_hbk"]);

        let forbidden = [
            "source",
            "source_hbk",
            "toc_path",
            "html_path",
            "page_title",
            "method_links",
            "constructor_links",
            "value_links",
        ];
        for file_name in [
            "global-methods.json",
            "platform-types.json",
            "type-methods.json",
            "enums.json",
        ] {
            let json = read_json(dir.join(file_name));
            assert_no_keys(&json, &["source_hbk"]);
            for record in json["records"]
                .as_array()
                .expect("records must be an array")
            {
                assert_no_keys(record, &forbidden);
            }
        }

        let diagnostics = read_json(dir.join("diagnostics.json"));
        assert_no_keys(&diagnostics, &["source_hbk"]);
        let diagnostic = &diagnostics["records"][0];
        assert!(diagnostic.get("source").is_some());
        assert_eq!(diagnostic["source"]["locale"], "ru");
        assert_eq!(
            diagnostic["source"]["html_path"],
            "objects/Global context/methods/catalog1566/XMLString1567.html"
        );

        fs::remove_dir_all(&dir).expect("export test dir must be removable");
    }

    #[test]
    fn streaming_export_writes_lean_records_without_full_context() {
        use syntax_helper_model::SyntaxHelperSink as _;

        let dir = std::env::temp_dir().join(format!(
            "v8-context-hbk-stream-export-test-{}",
            std::process::id()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("stale export test dir must be removable");
        }

        let source = source();
        let mut export = JsonExporter::new(&dir)
            .start_platform_context_stream("ru", "ru")
            .expect("streaming export must start");
        export
            .global_context(model::GlobalContext {
                name: name("Глобальный контекст"),
                property_links: vec![link("Справочники")],
                method_links: vec![link("XMLСтрока")],
                description: None,
                source: source.clone(),
            })
            .expect("global context count must be accepted");
        export
            .global_method(model::GlobalMethod {
                name: name("XMLСтрока"),
                signatures: Vec::new(),
                return_types: Vec::new(),
                description: Some("Creates an XML string.".to_string()),
                source: source.clone(),
            })
            .expect("global method must be writable");
        export
            .diagnostic(model::SyntaxHelperDiagnostic {
                severity: model::DiagnosticSeverity::Warning,
                code: "UNKNOWN_PAGE_CLASS",
                source,
                parser_stage: "root_discovery",
                message: "unknown page class".to_string(),
            })
            .expect("diagnostic must be writable");

        let summary = export.finish().expect("streaming export must finish");
        assert_eq!(summary.files.len(), EXPORT_FILES.len() + 1);
        assert_eq!(summary.counts.global_contexts, 1);
        assert_eq!(summary.counts.global_methods, 1);
        assert_eq!(summary.counts.diagnostics, 1);
        assert!(!dir.join("global-contexts.json").exists());

        let global_methods = read_json(dir.join("global-methods.json"));
        assert_eq!(global_methods["records"].as_array().unwrap().len(), 1);
        assert_no_keys(&global_methods["records"][0], &["source", "method_links"]);

        let platform_types = read_json(dir.join("platform-types.json"));
        assert!(platform_types["records"].as_array().unwrap().is_empty());

        let diagnostics = read_json(dir.join("diagnostics.json"));
        assert_eq!(diagnostics["records"].as_array().unwrap().len(), 1);
        assert!(diagnostics["records"][0].get("source").is_some());

        fs::remove_dir_all(&dir).expect("export test dir must be removable");
    }

    #[test]
    fn streaming_export_abort_removes_incomplete_json_files() {
        use syntax_helper_model::SyntaxHelperSink as _;

        let dir = std::env::temp_dir().join(format!(
            "v8-context-hbk-stream-export-abort-test-{}",
            std::process::id()
        ));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("stale export test dir must be removable");
        }

        let source = source();
        let mut export = JsonExporter::new(&dir)
            .start_platform_context_stream("ru", "ru")
            .expect("streaming export must start");
        export
            .global_method(model::GlobalMethod {
                name: name("XMLСтрока"),
                signatures: Vec::new(),
                return_types: Vec::new(),
                description: None,
                source,
            })
            .expect("global method must be writable before abort");

        export.abort().expect("incomplete export must be removable");

        assert!(dir.exists());
        assert!(!dir.join("metadata.json").exists());
        for file in EXPORT_FILES {
            assert!(
                !dir.join(file.file_name).exists(),
                "{} must be removed",
                file.file_name
            );
        }

        fs::remove_dir_all(&dir).expect("export test dir must be removable");
    }
}
