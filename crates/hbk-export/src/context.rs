use std::fs;
use std::path::PathBuf;

use serde::Serialize;

use hbk_book::HbkBook;
use syntax_helper_model::PlatformContext;

use crate::consumer::{
    ConsumerConstructor, ConsumerEvent, ConsumerGlobalMethod, ConsumerGlobalProperty,
    ConsumerPlatformMethod, ConsumerPlatformProperty, ConsumerPlatformType, ExportMetadata,
    RecordsEnvelope, consumer_enums, consumer_query_tables,
};
use crate::error::ExportError;
use crate::manifest::{EXPORT_FILES, SCHEMA_VERSION};
use crate::stream::StreamingSyntaxHelperExport;
use crate::writer::write_json_file;

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
        let events = context
            .global_context_events
            .iter()
            .map(ConsumerEvent::from)
            .collect::<Vec<_>>();
        files.push(self.write_records(
            "module-events.json",
            locale,
            source_locale,
            "module_event",
            &event_records(&events, "module_event"),
        )?);
        files.push(self.write_records(
            "type-events.json",
            locale,
            source_locale,
            "type_event",
            &event_records(&events, "type_event"),
        )?);
        files.push(self.write_records(
            "unknown-events.json",
            locale,
            source_locale,
            "unknown_event",
            &event_records(&events, "unknown_event"),
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
        let query_tables = consumer_query_tables(
            &context.query_tables,
            &context.table_fields,
            &context.table_parameters,
        );
        files.push(self.write_records(
            "query-tables.json",
            locale,
            source_locale,
            "query_table",
            &query_tables,
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
        let enums = consumer_enums(&context.enums, &context.enum_values);
        files.push(self.write_records("enums.json", locale, source_locale, "enum", &enums)?);
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

    pub(crate) fn write_records<T: Serialize>(
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

fn event_records<'a>(
    events: &'a [ConsumerEvent<'a>],
    record_kind: &'static str,
) -> Vec<&'a ConsumerEvent<'a>> {
    events
        .iter()
        .filter(|event| event.record_kind() == record_kind)
        .collect()
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
    pub global_context_events: usize,
    pub platform_types: usize,
    pub query_tables: usize,
    pub type_methods: usize,
    pub type_properties: usize,
    pub table_fields: usize,
    pub table_parameters: usize,
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
            global_context_events: context.global_context_events.len(),
            platform_types: context.platform_types.len(),
            query_tables: context.query_tables.len(),
            type_methods: context.type_methods.len(),
            type_properties: context.type_properties.len(),
            table_fields: context.table_fields.len(),
            table_parameters: context.table_parameters.len(),
            constructors: context.constructors.len(),
            enums: context.enums.len(),
            enum_values: context.enum_values.len(),
            diagnostics: context.diagnostics.len(),
        }
    }
}
