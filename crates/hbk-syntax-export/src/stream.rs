use std::fs;
use std::path::PathBuf;

use syntax_helper_model::{self as model, SyntaxHelperSink};

use crate::consumer::{
    ConsumerConstructor, ConsumerEvent, ConsumerGlobalMethod, ConsumerGlobalProperty,
    ConsumerPlatformMethod, ConsumerPlatformProperty, ConsumerPlatformType, ExportMetadata,
    consumer_enums, consumer_query_tables,
};
use crate::context::{JsonExportCounts, JsonExportSummary};
use crate::error::ExportError;
use crate::manifest::{EXPORT_FILES, SCHEMA_VERSION};
use crate::writer::{RecordFileWriter, open_record_file, remove_export_files, write_json_file};

pub struct StreamingSyntaxHelperExport {
    output_dir: PathBuf,
    locale: String,
    source_locale: String,
    files: Vec<PathBuf>,
    counts: JsonExportCounts,
    global_methods: RecordFileWriter,
    global_properties: RecordFileWriter,
    module_events: RecordFileWriter,
    type_events: RecordFileWriter,
    unknown_events: RecordFileWriter,
    platform_types: RecordFileWriter,
    type_methods: RecordFileWriter,
    type_properties: RecordFileWriter,
    query_tables: RecordFileWriter,
    query_table_records: Vec<model::QueryTable>,
    query_table_fields: Vec<model::QueryTableField>,
    query_table_parameters: Vec<model::QueryTableParameter>,
    constructors: RecordFileWriter,
    enums: RecordFileWriter,
    enum_definitions: Vec<model::EnumDefinition>,
    enum_values: Vec<model::EnumValue>,
    diagnostics: RecordFileWriter,
}

impl StreamingSyntaxHelperExport {
    pub(crate) fn start(
        output_dir: PathBuf,
        locale: &str,
        source_locale: &str,
    ) -> Result<Self, ExportError> {
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
        let module_events = open_record_file(
            &output_dir,
            &mut files,
            "module-events.json",
            locale,
            source_locale,
            "module_event",
        )?;
        let type_events = open_record_file(
            &output_dir,
            &mut files,
            "type-events.json",
            locale,
            source_locale,
            "type_event",
        )?;
        let unknown_events = open_record_file(
            &output_dir,
            &mut files,
            "unknown-events.json",
            locale,
            source_locale,
            "unknown_event",
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
        let query_tables = open_record_file(
            &output_dir,
            &mut files,
            "query-tables.json",
            locale,
            source_locale,
            "query_table",
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
            module_events,
            type_events,
            unknown_events,
            platform_types,
            type_methods,
            type_properties,
            query_tables,
            query_table_records: Vec::new(),
            query_table_fields: Vec::new(),
            query_table_parameters: Vec::new(),
            constructors,
            enums,
            enum_definitions: Vec::new(),
            enum_values: Vec::new(),
            diagnostics,
        })
    }

    pub fn finish(mut self) -> Result<JsonExportSummary, ExportError> {
        let enums = consumer_enums(&self.enum_definitions, &self.enum_values);
        for enum_record in &enums {
            self.enums.write_record(enum_record)?;
        }
        let query_tables = consumer_query_tables(
            &self.query_table_records,
            &self.query_table_fields,
            &self.query_table_parameters,
        );
        for query_table in &query_tables {
            self.query_tables.write_record(query_table)?;
        }

        self.global_methods.finish()?;
        self.global_properties.finish()?;
        self.module_events.finish()?;
        self.type_events.finish()?;
        self.unknown_events.finish()?;
        self.platform_types.finish()?;
        self.type_methods.finish()?;
        self.type_properties.finish()?;
        self.query_tables.finish()?;
        self.constructors.finish()?;
        self.enums.finish()?;
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
            module_events,
            type_events,
            unknown_events,
            platform_types,
            type_methods,
            type_properties,
            query_tables,
            constructors,
            enums,
            diagnostics,
            ..
        } = self;

        global_methods.close_unfinished();
        global_properties.close_unfinished();
        module_events.close_unfinished();
        type_events.close_unfinished();
        unknown_events.close_unfinished();
        platform_types.close_unfinished();
        type_methods.close_unfinished();
        type_properties.close_unfinished();
        query_tables.close_unfinished();
        constructors.close_unfinished();
        enums.close_unfinished();
        diagnostics.close_unfinished();

        remove_export_files(files)
    }
}

impl SyntaxHelperSink for StreamingSyntaxHelperExport {
    type Error = ExportError;

    fn record_detail_mode(&self) -> model::SyntaxHelperRecordDetailMode {
        model::SyntaxHelperRecordDetailMode::LeanConsumerExport
    }

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

    fn global_context_event(
        &mut self,
        record: model::GlobalContextEvent,
    ) -> Result<(), Self::Error> {
        let event = ConsumerEvent::from(&record);
        match event.record_kind() {
            "module_event" => self.module_events.write_record(&event)?,
            "type_event" => self.type_events.write_record(&event)?,
            "unknown_event" => self.unknown_events.write_record(&event)?,
            _ => self.unknown_events.write_record(&event)?,
        }
        self.counts.global_context_events += 1;
        Ok(())
    }

    fn platform_type(&mut self, record: model::PlatformType) -> Result<(), Self::Error> {
        self.platform_types
            .write_record(&ConsumerPlatformType::from(&record))?;
        self.counts.platform_types += 1;
        Ok(())
    }

    fn query_table(&mut self, record: model::QueryTable) -> Result<(), Self::Error> {
        self.query_table_records.push(record);
        self.counts.query_tables += 1;
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

    fn table_field(&mut self, record: model::QueryTableField) -> Result<(), Self::Error> {
        self.query_table_fields.push(record);
        self.counts.table_fields += 1;
        Ok(())
    }

    fn table_parameter(&mut self, record: model::QueryTableParameter) -> Result<(), Self::Error> {
        self.query_table_parameters.push(record);
        self.counts.table_parameters += 1;
        Ok(())
    }

    fn constructor(&mut self, record: model::Constructor) -> Result<(), Self::Error> {
        self.constructors
            .write_record(&ConsumerConstructor::from(&record))?;
        self.counts.constructors += 1;
        Ok(())
    }

    fn enum_definition(&mut self, record: model::EnumDefinition) -> Result<(), Self::Error> {
        self.enum_definitions.push(record);
        self.counts.enums += 1;
        Ok(())
    }

    fn enum_value(&mut self, record: model::EnumValue) -> Result<(), Self::Error> {
        self.enum_values.push(record);
        self.counts.enum_values += 1;
        Ok(())
    }

    fn diagnostic(&mut self, record: model::SyntaxHelperDiagnostic) -> Result<(), Self::Error> {
        self.diagnostics.write_record(&record)?;
        self.counts.diagnostics += 1;
        Ok(())
    }
}
