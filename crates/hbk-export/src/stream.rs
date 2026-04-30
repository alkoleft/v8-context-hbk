use std::fs;
use std::path::PathBuf;

use syntax_helper_model::{self as model, SyntaxHelperSink};

use crate::consumer::{
    ConsumerConstructor, ConsumerEnumDefinition, ConsumerEnumValue, ConsumerGlobalMethod,
    ConsumerGlobalProperty, ConsumerPlatformMethod, ConsumerPlatformProperty, ConsumerPlatformType,
    ExportMetadata,
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
    platform_types: RecordFileWriter,
    type_methods: RecordFileWriter,
    type_properties: RecordFileWriter,
    constructors: RecordFileWriter,
    enums: RecordFileWriter,
    enum_values: RecordFileWriter,
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
