use std::fmt;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::Serialize;

use hbk_book::HbkBook;
use syntax_helper_model::PlatformContext;

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

    pub fn export_platform_context(
        &self,
        locale: &str,
        source_locale: &str,
        source_hbk: &str,
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
            source_hbk,
            files: EXPORT_FILES.to_vec(),
        };

        let mut files = Vec::new();
        files.push(self.write_file("metadata.json", &metadata)?);
        files.push(self.write_records(
            "global-contexts.json",
            locale,
            source_locale,
            source_hbk,
            "global_context",
            &context.global_contexts,
        )?);
        files.push(self.write_records(
            "global-methods.json",
            locale,
            source_locale,
            source_hbk,
            "global_method",
            &context.global_methods,
        )?);
        files.push(self.write_records(
            "global-properties.json",
            locale,
            source_locale,
            source_hbk,
            "global_property",
            &context.global_properties,
        )?);
        files.push(self.write_records(
            "platform-types.json",
            locale,
            source_locale,
            source_hbk,
            "platform_type",
            &context.platform_types,
        )?);
        files.push(self.write_records(
            "type-methods.json",
            locale,
            source_locale,
            source_hbk,
            "type_method",
            &context.type_methods,
        )?);
        files.push(self.write_records(
            "type-properties.json",
            locale,
            source_locale,
            source_hbk,
            "type_property",
            &context.type_properties,
        )?);
        files.push(self.write_records(
            "constructors.json",
            locale,
            source_locale,
            source_hbk,
            "constructor",
            &context.constructors,
        )?);
        files.push(self.write_records(
            "enums.json",
            locale,
            source_locale,
            source_hbk,
            "enum",
            &context.enums,
        )?);
        files.push(self.write_records(
            "enum-values.json",
            locale,
            source_locale,
            source_hbk,
            "enum_value",
            &context.enum_values,
        )?);
        files.push(self.write_records(
            "diagnostics.json",
            locale,
            source_locale,
            source_hbk,
            "diagnostic",
            &context.diagnostics,
        )?);

        Ok(JsonExportSummary {
            output_dir: self.output_dir.clone(),
            locale: locale.to_string(),
            source_locale: source_locale.to_string(),
            files,
        })
    }

    fn write_records<T: Serialize>(
        &self,
        file_name: &'static str,
        locale: &str,
        source_locale: &str,
        source_hbk: &str,
        record_kind: &'static str,
        records: &[T],
    ) -> Result<PathBuf, ExportError> {
        let envelope = RecordsEnvelope {
            schema_version: SCHEMA_VERSION,
            locale,
            source_locale,
            source_hbk,
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
        let path = self.output_dir.join(file_name);
        let bytes = serde_json::to_vec_pretty(value).map_err(|source| ExportError::Json {
            path: path.clone(),
            source,
        })?;
        fs::write(&path, bytes).map_err(|source| ExportError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonExportSummary {
    pub output_dir: PathBuf,
    pub locale: String,
    pub source_locale: String,
    pub files: Vec<PathBuf>,
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
    source_hbk: &'a str,
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
    source_hbk: &'a str,
    record_kind: &'static str,
    records: &'a [T],
}

pub const EXPORT_FILES: &[ExportFile] = &[
    ExportFile {
        file_name: "global-contexts.json",
        record_kind: "global_context",
    },
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
    use syntax_helper_model::GlobalMethod;

    use super::*;

    #[test]
    fn root_locale_maps_to_export_locale_en() {
        assert_eq!(
            BookLocale::infer_from_path(Path::new("shcntx_root.hbk")).export_code(),
            "en"
        );
    }

    #[test]
    fn platform_context_serializes_with_source_provenance() {
        let source = syntax_helper_model::SyntaxHelperSource {
            hbk_path: PathBuf::from("/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk"),
            locale: "ru".to_string(),
            toc_path: Some("0.1".to_string()),
            html_path: "objects/Global context/methods/catalog1566/XMLString1567.html".to_string(),
            page_title: "Глобальный контекст.XMLСтрока".to_string(),
        };
        let context = PlatformContext {
            global_methods: vec![GlobalMethod {
                name: syntax_helper_model::LocalizedName {
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
            "global-contexts.json",
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
                "shcntx_root.hbk",
                "global_method",
                &Vec::<GlobalMethod>::new(),
            )
            .expect("record envelope must be writable");

        let json = fs::read_to_string(&path).expect("record envelope must be readable");
        assert!(!json.is_empty());
        let parsed: serde_json::Value =
            serde_json::from_str(&json).expect("record envelope must be valid JSON");
        assert_eq!(parsed["locale"], "en");
        assert_eq!(parsed["source_locale"], "root");

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
        for file in &summary.files {
            let json = fs::read_to_string(file)
                .unwrap_or_else(|error| panic!("{} must be readable: {error}", file.display()));
            assert!(!json.is_empty(), "{} must be non-empty", file.display());
            serde_json::from_str::<serde_json::Value>(&json)
                .unwrap_or_else(|error| panic!("{} must be valid JSON: {error}", file.display()));
        }

        let metadata =
            fs::read_to_string(dir.join("metadata.json")).expect("metadata.json must be readable");
        let metadata: serde_json::Value =
            serde_json::from_str(&metadata).expect("metadata.json must be valid JSON");
        assert_eq!(metadata["locale"], "en");
        assert_eq!(metadata["source_locale"], "root");
        assert_eq!(
            metadata["files"]
                .as_array()
                .expect("files must be an array")
                .len(),
            EXPORT_FILES.len()
        );

        fs::remove_dir_all(&dir).expect("export test dir must be removable");
    }
}
