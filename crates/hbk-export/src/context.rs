use std::path::PathBuf;

use crate::error::ExportError;
use crate::stream::StreamingSyntaxHelperExport;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonExporter {
    output_dir: PathBuf,
}

impl JsonExporter {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
        }
    }

    pub fn start_platform_context_stream(
        &self,
        locale: &str,
        source_locale: &str,
    ) -> Result<StreamingSyntaxHelperExport, ExportError> {
        StreamingSyntaxHelperExport::start(self.output_dir.clone(), locale, source_locale)
    }
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
