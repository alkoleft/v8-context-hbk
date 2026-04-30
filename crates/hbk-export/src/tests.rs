use std::fs;
use std::path::{Path, PathBuf};

use hbk_book::BookLocale;
use serde_json::Value;
use syntax_helper_model::{self as model, PlatformContext};

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

fn facts() -> model::SectionFacts {
    model::SectionFacts {
        availability: model::Availability {
            contexts: vec![
                model::AvailabilityContext::ThinClient,
                model::AvailabilityContext::Server,
            ],
        },
        examples: vec![model::ExampleBlock {
            text: "XMLWriter.WriteText(XMLString(MaturityDate));".to_string(),
        }],
        see_also: vec![model::MemberLink {
            name: name("XMLЗначение"),
            html_path: "objects/Global context/methods/catalog1566/XMLValue1568.html".to_string(),
        }],
        available_since: Some(model::VersionFact {
            version: Some("8.0".to_string()),
            text: "Доступен, начиная с версии 8.0.".to_string(),
        }),
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
            facts: model::SectionFacts::default(),
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
    let parsed: Value = serde_json::from_str(&json).expect("record envelope must be valid JSON");
    assert_eq!(parsed["schema_version"], 3);
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
    assert_eq!(metadata["schema_version"], 3);
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
                variant: Some(model::SyntaxVariant {
                    title: "По значению".to_string(),
                    description: Some("Creates an XML string from a value.".to_string()),
                }),
            }],
            return_types: vec![model::TypeRef {
                name: "Строка".to_string(),
            }],
            description: Some("Creates an XML string.".to_string()),
            facts: facts(),
            source: source.clone(),
        }],
        platform_types: vec![model::PlatformType {
            name: name("Массив"),
            method_links: vec![link("Добавить")],
            constructor_links: vec![link("Массив")],
            description: Some("Array type.".to_string()),
            facts: model::SectionFacts::default(),
            source: source.clone(),
        }],
        type_methods: vec![model::PlatformMethod {
            owner: name("Массив"),
            name: name("Добавить"),
            signatures: Vec::new(),
            return_types: Vec::new(),
            description: None,
            facts: model::SectionFacts::default(),
            source: source.clone(),
        }],
        enums: vec![model::EnumDefinition {
            name: name("ТипЗначенияJSON"),
            value_links: vec![link("КонецМассива")],
            description: None,
            facts: model::SectionFacts::default(),
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
    let metadata = read_json(dir.join("metadata.json"));
    assert_eq!(metadata["schema_version"], 3);
    assert_no_keys(&metadata, &["source_hbk"]);

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
    let global_methods = read_json(dir.join("global-methods.json"));
    let method = &global_methods["records"][0];
    assert_eq!(
        method["availability"]["contexts"],
        serde_json::json!(["thin_client", "server"])
    );
    assert_eq!(
        method["examples"][0]["text"],
        "XMLWriter.WriteText(XMLString(MaturityDate));"
    );
    assert_eq!(method["see_also"][0]["name"]["primary"], "XMLЗначение");
    assert!(method["see_also"][0].get("html_path").is_none());
    assert_eq!(method["available_since"]["version"], "8.0");
    assert_eq!(method["signatures"][0]["variant"]["title"], "По значению");
    assert_eq!(
        method["signatures"][0]["variant"]["description"],
        "Creates an XML string from a value."
    );
    assert!(
        method["signatures"][0]["variant"]
            .get("html_path")
            .is_none()
    );
    assert!(method.get("source").is_none());

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
            facts: facts(),
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
    assert_eq!(
        global_methods["records"][0]["availability"]["contexts"],
        serde_json::json!(["thin_client", "server"])
    );
    assert_eq!(
        global_methods["records"][0]["see_also"][0]["name"]["primary"],
        "XMLЗначение"
    );
    assert!(
        global_methods["records"][0]["see_also"][0]
            .get("html_path")
            .is_none()
    );

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
            facts: model::SectionFacts::default(),
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
