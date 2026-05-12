#[cfg(test)]
mod tests {
    use super::*;
    use hbk_book::test_utils::{fixture_container, zip_bytes, zip_entries};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use syntax_helper_model as model;
    use syntax_helper_model::SyntaxHelperSink;
    use syntax_helper_search::SearchDocumentKind;

    #[test]
    fn provider_response_uses_versioned_envelope() {
        let response = provider_response(
            "get",
            "unsupported",
            json!({ "kind": "invalid" }),
            Vec::new(),
            unsupported_query_diagnostic("invalid root"),
        );

        assert_eq!(response["schema_version"], 1);
        assert_eq!(response["command"], "get");
        assert_eq!(response["status"], "unsupported");
        assert_eq!(response["results"].as_array().unwrap().len(), 0);
        assert_eq!(response["diagnostics"][0]["code"], "UNSUPPORTED_QUERY");
    }

    #[test]
    fn search_query_records_explicit_limit() {
        let query = search_query_value("Структура", SearchMode::Keywords, Some(3));

        assert_eq!(query["kind"], "search");
        assert_eq!(query["mode"], "keywords");
        assert_eq!(query["text"], "Структура");
        assert_eq!(query["limit"], 3);
    }

    #[test]
    fn related_query_records_limit_and_compact_output() {
        let query = related_query_value(
            Some("platform_type:Структура"),
            None,
            None,
            None,
            7,
            None,
            Some(2),
            true,
            false,
        );

        assert_eq!(query["kind"], "related");
        assert_eq!(query["root"]["id"], "platform_type:Структура");
        assert_eq!(query["depth"], 5);
        assert_eq!(query["limit"], 2);
        assert_eq!(query["output"], "compact");
    }

    #[test]
    fn related_member_of_edge_stays_related_query_kind() {
        let query = related_query_value(
            Some("type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"),
            None,
            None,
            None,
            5,
            Some("member_of"),
            Some(1),
            false,
            false,
        );

        assert!(is_supported_edge_filter("member_of"));
        assert_eq!(query["kind"], "related");
        assert_eq!(query["edge"], "member_of");
    }

    #[test]
    fn related_graph_query_records_graph_output_and_limit() {
        let query = related_query_value(
            Some("type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"),
            None,
            None,
            None,
            9,
            None,
            Some(80),
            false,
            true,
        );

        assert_eq!(query["kind"], "type_graph");
        assert_eq!(
            query["root"]["id"],
            "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"
        );
        assert_eq!(query["depth"], 5);
        assert_eq!(query["limit"], 80);
        assert_eq!(query["output"], "graph");
    }

    #[test]
    fn related_graph_command_parses_existing_command_family() {
        let cli = Cli::try_parse_from([
            "v8-context-hbk",
            "syntax",
            "related",
            "--id",
            "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор",
            "--graph",
            "--limit",
            "1",
            "--format",
            "json",
        ])
        .expect("syntax related --graph command must parse");

        match cli.command {
            Command::Syntax {
                command:
                    SyntaxCommand::Related {
                        id,
                        graph,
                        limit,
                        compact,
                        edge,
                        format,
                        ..
                    },
            } => {
                assert_eq!(
                    id.as_deref(),
                    Some("type_property:platform_type:НастройкиКомпоновкиДанных:Отбор")
                );
                assert!(graph);
                assert_eq!(limit, Some(1));
                assert!(!compact);
                assert!(edge.is_none());
                assert!(matches!(format, OutputFormat::Json));
            }
            other => panic!("expected syntax related --graph command, got {other:?}"),
        }
    }

    #[test]
    fn related_graph_root_kind_guard_keeps_non_platform_domains_out() {
        for accepted in [
            SearchDocumentKind::PlatformType,
            SearchDocumentKind::TypeProperty,
            SearchDocumentKind::TypeMethod,
            SearchDocumentKind::Constructor,
            SearchDocumentKind::GlobalMethod,
            SearchDocumentKind::ModuleEvent,
            SearchDocumentKind::TypeEvent,
        ] {
            assert!(
                is_supported_type_graph_root_kind(accepted),
                "{accepted:?} must remain an accepted graph root"
            );
        }

        for rejected in [
            SearchDocumentKind::GlobalProperty,
            SearchDocumentKind::QueryTable,
            SearchDocumentKind::QueryTableField,
            SearchDocumentKind::LanguageType,
            SearchDocumentKind::EnumValue,
        ] {
            assert!(
                !is_supported_type_graph_root_kind(rejected),
                "{rejected:?} must not be accepted as a type graph root"
            );
        }
    }

    #[test]
    fn type_ref_gaps_command_parses_existing_index_path() {
        let cli = Cli::try_parse_from([
            "v8-context-hbk",
            "syntax",
            "type-ref-gaps",
            "--index",
            "target/type-ref-gaps.sqlite",
            "--limit",
            "5",
            "--format",
            "json",
        ])
        .expect("type-ref-gaps command must parse");

        match cli.command {
            Command::Syntax {
                command:
                    SyntaxCommand::TypeRefGaps {
                        index,
                        limit,
                        format,
                    },
            } => {
                assert_eq!(index, Some(PathBuf::from("target/type-ref-gaps.sqlite")));
                assert_eq!(limit, 5);
                assert!(matches!(format, OutputFormat::Json));
            }
            other => panic!("expected syntax type-ref-gaps command, got {other:?}"),
        }
    }

    #[test]
    fn get_query_classifier_records_type_identity_root_once() {
        let args = GetArgs {
            kind: Some("platform_type".to_string()),
            name: Some("HTTPСоединение".to_string()),
            ..GetArgs::default()
        };

        let query = classify_get_query(&args);

        assert_eq!(query.value["kind"], "type_identity");
        assert_eq!(query.value["name"], "HTTPСоединение");
        assert!(matches!(
            query.lookup,
            GetLookup::TypeIdentityByName("HTTPСоединение")
        ));
    }

    #[test]
    fn get_query_classifier_records_owner_type_callable_root_once() {
        let args = GetArgs {
            owner_type_id: Some("platform_type:HTTPСоединение".to_string()),
            callable: Some("УстановитьТелоИзСтроки".to_string()),
            ..GetArgs::default()
        };

        let query = classify_get_query(&args);

        assert_eq!(query.value["kind"], "callable_overloads");
        assert_eq!(query.value["owner_type_id"], "platform_type:HTTPСоединение");
        assert_eq!(query.value["name"], "УстановитьТелоИзСтроки");
        assert!(matches!(
            query.lookup,
            GetLookup::CallableByOwnerType {
                owner_type_id: "platform_type:HTTPСоединение",
                callable: "УстановитьТелоИзСтроки"
            }
        ));
    }

    #[test]
    fn get_query_classifier_preserves_unsupported_kind_message() {
        let args = GetArgs {
            kind: Some("query_type".to_string()),
            name: Some("Строка".to_string()),
            ..GetArgs::default()
        };

        let query = classify_get_query(&args);

        assert_eq!(query.value["kind"], "invalid");
        assert!(matches!(
            query.lookup,
            GetLookup::Unsupported(
                "syntax get --kind currently supports only platform_type with exactly one of --id, --name or --alias"
            )
        ));
    }

    #[test]
    fn get_query_classifier_preserves_invalid_root_message() {
        let args = GetArgs {
            id: Some("platform_type:Строка".to_string()),
            name: Some("Строка".to_string()),
            ..GetArgs::default()
        };

        let query = classify_get_query(&args);

        assert_eq!(query.value["kind"], "invalid");
        assert!(matches!(
            query.lookup,
            GetLookup::Unsupported(
                "syntax get requires exactly one root: --id, --name, --kind platform_type with --id/--name/--alias, --members-of, --owner-type-id with --member/--callable, --callable-id, or both --owner and --member"
            )
        ));
    }

    #[test]
    fn compact_related_fact_keeps_identity_and_omits_bulky_fields() {
        let document = SearchDocument {
            id: "type_method:platform_type:Тест:Выполнить".to_string(),
            kind: SearchDocumentKind::TypeMethod,
            name: name("Выполнить"),
            owner: Some(name("Тест")),
            signatures: vec![syntax_helper_search::SearchSignature {
                text: "Выполнить(Параметр)".to_string(),
                parameters: Vec::new(),
                return_types: Vec::new(),
                return_type_facts: Vec::new(),
                title: None,
                description: None,
            }],
            type_refs: Vec::new(),
            return_types: vec!["Булево".to_string()],
            type_ref_facts: Vec::new(),
            return_type_facts: Vec::new(),
            description: Some("Detailed description".to_string()),
            preview: "Detailed description".to_string(),
            parameter_terms: Vec::new(),
            relation_keys: Vec::new(),
            owner_relation_key: None,
            explicit_type_ref_ids: Vec::new(),
            explicit_return_type_ref_ids: Vec::new(),
            availability_contexts: Vec::new(),
            available_since: None,
            metadata_kind: None,
            template_parameters: Vec::new(),
            type_template_key: None,
            type_template_classification_diagnostic: None,
        };

        let fact = document_fact(&document, ProviderFactDetail::Compact);

        assert_eq!(fact["id"], document.id);
        assert_eq!(fact["kind"], document.kind.as_str());
        assert_eq!(fact["name"]["primary"], "Выполнить");
        assert_eq!(fact["owner"], "Тест");
        assert!(fact.get("signatures").is_none());
        assert!(fact.get("return").is_none());
        assert!(fact.get("description").is_none());
    }

    #[test]
    fn full_provider_fact_keeps_export_compatible_fields() {
        let document = SearchDocument {
            id: "type_method:platform_type:Тест:Выполнить".to_string(),
            kind: SearchDocumentKind::TypeMethod,
            name: name("Выполнить"),
            owner: Some(name("Тест")),
            signatures: vec![syntax_helper_search::SearchSignature {
                text: "Выполнить(Параметр)".to_string(),
                parameters: vec![syntax_helper_search::SearchParameter {
                    name: "Параметр".to_string(),
                    required: true,
                    type_refs: vec!["Строка".to_string()],
                    type_ref_facts: Vec::new(),
                    description: Some("Input value".to_string()),
                }],
                return_types: vec!["Дата".to_string()],
                return_type_facts: Vec::new(),
                title: Some("Основной вариант".to_string()),
                description: Some("Variant description".to_string()),
            }],
            type_refs: vec!["Строка".to_string()],
            return_types: vec!["Булево".to_string()],
            type_ref_facts: Vec::new(),
            return_type_facts: Vec::new(),
            description: Some("Detailed description".to_string()),
            preview: "Detailed description".to_string(),
            parameter_terms: Vec::new(),
            relation_keys: Vec::new(),
            owner_relation_key: None,
            explicit_type_ref_ids: Vec::new(),
            explicit_return_type_ref_ids: Vec::new(),
            availability_contexts: Vec::new(),
            available_since: None,
            metadata_kind: None,
            template_parameters: Vec::new(),
            type_template_key: None,
            type_template_classification_diagnostic: None,
        };

        let fact = document_fact(&document, ProviderFactDetail::Full);

        assert_eq!(fact["id"], document.id);
        assert_eq!(fact["kind"], document.kind.as_str());
        assert_eq!(fact["name"]["primary"], "Выполнить");
        assert_eq!(fact["owner"], "Тест");
        assert_eq!(fact["types"], json!(["Строка"]));
        assert_eq!(fact["return"], json!(["Булево"]));
        assert_eq!(fact["description"], "Detailed description");
        assert!(fact.get("type_template_key").is_none());
        assert!(fact.get("generic_template_key").is_none());
        assert!(fact.get("template_binding").is_none());
        assert!(fact.get("generic_binding").is_none());
        let signature = &fact["signatures"][0];
        assert!(signature.get("text").is_none());
        assert_eq!(signature["title"], "Основной вариант");
        assert_eq!(signature["description"], "Variant description");
        assert_eq!(signature["parameters"][0]["name"], "Параметр");
        assert_eq!(signature["parameters"][0]["required"], true);
        assert_eq!(signature["parameters"][0]["types"], json!(["Строка"]));
        assert_eq!(signature["parameters"][0]["description"], "Input value");
        assert_eq!(signature["return"], json!(["Дата"]));
    }

    #[test]
    fn graph_meta_reports_type_reference_resolution_without_fact_leakage() {
        let document = SearchDocument {
            id: "type_method:platform_type:Тест:Выполнить".to_string(),
            kind: SearchDocumentKind::TypeMethod,
            name: name("Выполнить"),
            owner: Some(name("Тест")),
            signatures: vec![syntax_helper_search::SearchSignature {
                text: "Выполнить(Параметр)".to_string(),
                parameters: vec![syntax_helper_search::SearchParameter {
                    name: "Параметр".to_string(),
                    required: true,
                    type_refs: vec!["НеизвестныйТип".to_string()],
                    type_ref_facts: vec![SearchTypeRef {
                        name: "НеизвестныйТип".to_string(),
                        target: SearchTypeRefTarget::Unresolved,
                        type_template_key: None,
                        template_binding: None,
                    }],
                    description: None,
                }],
                return_types: vec!["ДубльТип".to_string()],
                return_type_facts: vec![SearchTypeRef {
                    name: "ДубльТип".to_string(),
                    target: SearchTypeRefTarget::Ambiguous(vec![
                        "platform_type:ДубльТип:Первый".to_string(),
                        "platform_type:ДубльТип:Второй".to_string(),
                    ]),
                    type_template_key: None,
                    template_binding: None,
                }],
                title: None,
                description: None,
            }],
            type_refs: Vec::new(),
            return_types: Vec::new(),
            type_ref_facts: Vec::new(),
            return_type_facts: vec![SearchTypeRef {
                name: "Строка".to_string(),
                target: SearchTypeRefTarget::Ok("platform_type:Строка".to_string()),
                type_template_key: Some(model::PlatformTypeTemplateKey::new("String", "Value")),
                template_binding: Some(model::TypeTemplateBinding {
                    template_key: model::PlatformTypeTemplateKey::new("String", "Value"),
                    arguments: vec![model::TemplateParameterBinding::OwnerParameter {
                        owner_parameter_index: 0,
                        target_parameter_index: 0,
                    }],
                }),
            }],
            description: Some("Detailed description".to_string()),
            preview: "Detailed description".to_string(),
            parameter_terms: Vec::new(),
            relation_keys: Vec::new(),
            owner_relation_key: None,
            explicit_type_ref_ids: Vec::new(),
            explicit_return_type_ref_ids: Vec::new(),
            availability_contexts: Vec::new(),
            available_since: None,
            metadata_kind: None,
            template_parameters: Vec::new(),
            type_template_key: None,
            type_template_classification_diagnostic: None,
        };

        let fact = document_fact(&document, ProviderFactDetail::Full);
        let type_references = graph_type_references(&document);
        let results = vec![json!({
            "fact": fact,
            "meta": {
                "root": true,
                "depth": 0,
                "path": [],
                "type_references": type_references,
            }
        })];
        let diagnostics = graph_type_reference_diagnostics(&results);

        assert!(results[0]["fact"].get("type_references").is_none());
        assert!(results[0]["fact"].get("type_refs").is_none());
        assert_eq!(results[0]["meta"]["type_references"][0]["role"], "return");
        assert_eq!(
            results[0]["meta"]["type_references"][0]["target_type_id"],
            "platform_type:Строка"
        );
        assert_eq!(
            results[0]["meta"]["type_references"][0]["template_binding"],
            json!({
                "template_key": {
                    "family": "String",
                    "variant": "Value",
                },
                "arguments": [{
                    "owner_parameter": {
                        "owner_parameter_index": 0,
                        "target_parameter_index": 0,
                    }
                }],
            })
        );
        assert!(
            results[0]["meta"]["type_references"][0]["template_binding"]
                .as_object()
                .unwrap()
                .keys()
                .all(|key| key == "template_key" || key == "arguments")
        );
        assert_eq!(results[0]["meta"]["type_references"][0]["status"], "ok");
        assert_eq!(results[0]["meta"]["type_references"][0]["name"], "Строка");
        assert!(
            results[0]["meta"]["type_references"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["role"] == "signature_return"
                    && value["status"] == "ambiguous"
                    && value["candidate_type_ids"].as_array().unwrap().len() == 2)
        );
        assert!(
            results[0]["meta"]["type_references"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["role"] == "parameter_type"
                    && value["status"] == "unresolved"
                    && value["parameter_name"] == "Параметр"
                    && value["signature_ordinal"] == 0
                    && value["parameter_ordinal"] == 0)
        );
        assert!(
            results[0]["meta"]["type_references"]
                .as_array()
                .unwrap()
                .iter()
                .all(|value| value.get("type_template_key").is_none()
                    && value.get("generic_template_key").is_none()
                    && value.get("generic_binding").is_none())
        );
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic["code"] == "AMBIGUOUS_TYPE_REFERENCE"
                && diagnostic["source_id"] == document.id
                && diagnostic["role"] == "signature_return"
        }));
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic["code"] == "UNRESOLVED_TYPE_REFERENCE"
                && diagnostic["source_id"] == document.id
                && diagnostic["role"] == "parameter_type"
        }));
    }

    #[test]
    fn owner_member_lookup_reports_ambiguous_owner_before_member_filtering() {
        let path = temp_path("ambiguous-owner-member.sqlite");
        let mut builder = SearchIndexBuilder::new();
        builder
            .platform_type(platform_type_with_owner_path("ЭлементыФормы", "Форма"))
            .unwrap();
        builder
            .platform_type(platform_type_with_owner_path(
                "ЭлементыФормы",
                "Форма клиентского приложения",
            ))
            .unwrap();
        builder
            .type_method(type_method_with_owner_path(
                "ЭлементыФормы",
                "Форма",
                "Добавить",
            ))
            .unwrap();
        build_index_from_builder(&path, &metadata(), builder).unwrap();

        let index = SearchIndex::open_read_only(&path).unwrap();
        let roots = owner_member_roots(&index, "ЭлементыФормы", "Добавить").unwrap();

        assert_eq!(roots.len(), 2);
        assert!(
            roots
                .iter()
                .all(|hit| hit.document.kind == SearchDocumentKind::PlatformType)
        );
        assert_eq!(roots[0].document.id, "platform_type:ЭлементыФормы:Форма");
        assert_eq!(
            roots[1].document.id,
            "platform_type:ЭлементыФормы:Форма клиентского приложения"
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn constructor_lookup_reports_ambiguous_type_name_before_owner_selection() {
        let path = temp_path("ambiguous-constructor-type.sqlite");
        let mut builder = SearchIndexBuilder::new();
        builder
            .platform_type(platform_type_with_owner_path("ЭлементыФормы", "Форма"))
            .unwrap();
        builder
            .platform_type(platform_type_with_owner_path(
                "ЭлементыФормы",
                "Форма клиентского приложения",
            ))
            .unwrap();
        builder
            .constructor(constructor(
                "ЭлементыФормы",
                "platform_type:ЭлементыФормы:Форма",
            ))
            .unwrap();
        build_index_from_builder(&path, &metadata(), builder).unwrap();

        let index = SearchIndex::open_read_only(&path).unwrap();
        let candidates = type_identity_candidates(&index, "ЭлементыФормы").unwrap();

        assert_eq!(candidates.len(), 2);
        assert!(
            candidates
                .iter()
                .all(|hit| hit.document.kind == SearchDocumentKind::PlatformType)
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn top_level_export_command_parses_raw_raw_request() {
        let cli = Cli::try_parse_from([
            "v8-context-hbk",
            "export",
            "fmtdui_ru.hbk",
            "--output",
            "target/book-export/raw",
            "--format",
            "raw",
            "--hierarchy",
            "raw",
        ])
        .expect("top-level export command must parse");

        match cli.command {
            Command::Export {
                book,
                output,
                format,
                hierarchy,
            } => {
                assert_eq!(book, PathBuf::from("fmtdui_ru.hbk"));
                assert_eq!(output, PathBuf::from("target/book-export/raw"));
                assert!(matches!(format, BookExportCliFormat::Raw));
                assert!(matches!(hierarchy, BookExportCliHierarchy::Raw));
            }
            other => panic!("expected top-level export command, got {other:?}"),
        }
    }

    #[test]
    fn site_generate_command_parses_include_filters() {
        let cli = Cli::try_parse_from([
            "v8-context-hbk",
            "site",
            "generate",
            "/opt/1cv8/x86_64/8.5.1.1150",
            "--output",
            "target/doc-site",
            "--include",
            "fmtdui_ru.hbk",
            "--include",
            "shlang_ru.hbk",
        ])
        .expect("site generate command must parse");

        match cli.command {
            Command::Site {
                command:
                    SiteCommand::Generate {
                        source_dir,
                        output,
                        include_file_names,
                    },
            } => {
                assert_eq!(source_dir, PathBuf::from("/opt/1cv8/x86_64/8.5.1.1150"));
                assert_eq!(output, PathBuf::from("target/doc-site"));
                assert_eq!(include_file_names, vec!["fmtdui_ru.hbk", "shlang_ru.hbk"]);
            }
            other => panic!("expected site generate command, got {other:?}"),
        }
    }

    #[test]
    fn top_level_export_writes_raw_storage_files() {
        let workspace = temp_workspace("cli-raw-success");
        let source_path = workspace.join("fmtdui_ru.hbk");
        write_book_fixture(
            &source_path,
            vec![
                ("docs/page.html", b"<html>page</html>".as_ref()),
                ("assets/./style.css", b"body {}".as_ref()),
            ],
        );
        let output_root = workspace.join("out");

        let result = export_book_content(
            source_path,
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw top-level export path must succeed");

        assert_eq!(
            fs::read(output_root.join("docs/page.html")).expect("page must be exported"),
            b"<html>page</html>"
        );
        assert_eq!(
            fs::read(output_root.join("assets/style.css")).expect("asset must be exported"),
            b"body {}"
        );
        assert_eq!(result.files().len(), 2);
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn top_level_export_writes_markdown_toc_files() {
        let workspace = temp_workspace("cli-markdown-success");
        let source_path = workspace.join("fmtdui_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Справка"}{"en","Help"}},"/docs/page.html"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![(
                "docs/page.html",
                "<html><body><h1>Справка</h1><p>Markdown page</p></body></html>".as_bytes(),
            )],
        );
        let output_root = workspace.join("out");

        let result = export_book_content(
            source_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc top-level export path must succeed");

        let markdown_path = output_root.join("справка/index.md");
        let markdown = fs::read_to_string(markdown_path).expect("Markdown page must be exported");
        assert!(markdown.contains("# Справка"));
        assert!(markdown.contains("Markdown page"));
        assert_eq!(result.files().len(), 1);
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn site_generate_writes_page_markdown_data_files() {
        let workspace = temp_workspace("cli-site-success");
        let source_path = workspace.join("fmtdui_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Справка"}{"en","Help"}},"/docs/page.html"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![(
                "docs/page.html",
                "<html><body><h1>Справка</h1><p>Site page</p></body></html>".as_bytes(),
            )],
        );
        let output_root = workspace.join("out");

        let run = generate_site_data(
            workspace.clone(),
            output_root.clone(),
            vec!["fmtdui_ru.hbk".to_string()],
        )
        .expect("site generation must succeed");

        assert_eq!(run.result.book_count(), 1);
        assert_eq!(run.result.page_count(), 1);
        assert!(output_root.join("data/manifest.json").exists());
        let pages_root = output_root.join("data/locales/ru/pages");
        let page = fs::read_dir(pages_root)
            .expect("pages directory must exist")
            .next()
            .expect("one page file must exist")
            .unwrap()
            .path();
        let markdown = fs::read_to_string(page).expect("page Markdown must be readable");
        assert!(markdown.contains("# Справка"));
        assert!(markdown.contains("Site page"));
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn site_generate_artifact_progress_uses_sparse_milestones() {
        assert!(should_print_artifact_progress(1, 250));
        assert!(!should_print_artifact_progress(62, 250));
        assert!(should_print_artifact_progress(100, 250));
        assert!(should_print_artifact_progress(200, 250));
        assert!(!should_print_artifact_progress(201, 250));
        assert!(should_print_artifact_progress(250, 250));
        assert!(!should_print_artifact_progress(2_499, 66_730));
        assert!(should_print_artifact_progress(2_500, 66_730));
        assert!(should_print_artifact_progress(66_730, 66_730));
        assert!(!should_print_artifact_progress(1, 0));
    }

    #[test]
    fn site_generate_source_book_progress_uses_sparse_milestones() {
        assert!(should_print_source_book_progress(1, 116));
        assert!(!should_print_source_book_progress(11, 116));
        assert!(should_print_source_book_progress(12, 116));
        assert!(should_print_source_book_progress(24, 116));
        assert!(should_print_source_book_progress(116, 116));
        assert!(!should_print_source_book_progress(1, 0));
    }

    #[test]
    fn site_generate_progress_messages_include_last_file_name() {
        let book_path = PathBuf::from("/tmp/platform/shcntx_ru.hbk");
        let page_path = PathBuf::from("/tmp/site/data/locales/ru/pages/page-1.md");

        assert_eq!(
            progress_message(
                SiteGenerationProgress::SourceBookLoading {
                    current: 1,
                    total: 116,
                    path: &book_path,
                },
                true,
            )
            .as_deref(),
            Some("progress: loading source books: 1/116 (shcntx_ru.hbk)")
        );
        assert_eq!(
            progress_message(
                SiteGenerationProgress::ArtifactWriting {
                    current: 2_500,
                    total: 66_730,
                    kind: hbk_doc_site::GeneratedSiteFileKind::Page,
                    path: &page_path,
                },
                false,
            )
            .as_deref(),
            Some("progress: writing artifacts: 2500/66730 (page-1.md)")
        );
    }

    #[test]
    fn site_generate_interactive_progress_is_time_throttled() {
        let page_path = PathBuf::from("/tmp/site/data/locales/ru/pages/page-1.md");
        let recent_update = Some(INTERACTIVE_PROGRESS_UPDATE_INTERVAL / 2);
        let delayed_update = Some(INTERACTIVE_PROGRESS_UPDATE_INTERVAL);

        assert!(should_render_interactive_progress(
            SiteGenerationProgress::SiteDataBuilt {
                locale_count: 1,
                toc_node_count: 267,
                page_count: 254,
            },
            recent_update,
        ));
        assert!(should_render_interactive_progress(
            SiteGenerationProgress::ArtifactWriting {
                current: 1,
                total: 66_730,
                kind: hbk_doc_site::GeneratedSiteFileKind::Page,
                path: &page_path,
            },
            recent_update,
        ));
        assert!(!should_render_interactive_progress(
            SiteGenerationProgress::ArtifactWriting {
                current: 2,
                total: 66_730,
                kind: hbk_doc_site::GeneratedSiteFileKind::Page,
                path: &page_path,
            },
            recent_update,
        ));
        assert!(should_render_interactive_progress(
            SiteGenerationProgress::ArtifactWriting {
                current: 2,
                total: 66_730,
                kind: hbk_doc_site::GeneratedSiteFileKind::Page,
                path: &page_path,
            },
            delayed_update,
        ));
        assert!(should_render_interactive_progress(
            SiteGenerationProgress::ArtifactWriting {
                current: 66_730,
                total: 66_730,
                kind: hbk_doc_site::GeneratedSiteFileKind::Page,
                path: &page_path,
            },
            recent_update,
        ));
    }

    #[test]
    fn site_generate_reports_missing_source_directory_before_writing() {
        let workspace = temp_workspace("cli-site-missing-source");
        let source_dir = workspace.join("missing");
        let output_root = workspace.join("out");

        let error = generate_site_data(source_dir.clone(), output_root.clone(), Vec::new())
            .expect_err("missing source directory must be rejected");

        assert_eq!(
            error.to_string(),
            format!(
                "documentation site source directory '{}' does not exist",
                source_dir.display()
            )
        );
        assert!(!output_root.exists());
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn site_generate_reports_empty_corpus_before_writing() {
        let workspace = temp_workspace("cli-site-empty-corpus");
        let output_root = workspace.join("out");

        let error = generate_site_data(
            workspace.clone(),
            output_root.clone(),
            vec!["missing_ru.hbk".to_string()],
        )
        .expect_err("empty included corpus must be rejected");

        assert_eq!(
            error.to_string(),
            "documentation site source corpus is empty"
        );
        assert!(!output_root.exists());
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn site_generate_reports_unsupported_input_without_panic() {
        let workspace = temp_workspace("cli-site-unsupported-input");
        let source_path = workspace.join("bad_ru.hbk");
        fs::write(&source_path, b"not an hbk container").expect("bad fixture must be written");
        let output_root = workspace.join("out");

        let error = generate_site_data(
            workspace.clone(),
            output_root.clone(),
            vec!["bad_ru.hbk".to_string()],
        )
        .expect_err("unsupported HBK input must be rejected");

        assert!(
            error
                .to_string()
                .starts_with("failed to read documentation site book")
        );
        assert!(!output_root.exists());
        let _ = fs::remove_dir_all(workspace);
    }

    #[test]
    fn top_level_export_reports_unsupported_matrix_before_opening_book() {
        for (format, hierarchy, expected) in [
            (
                BookExportFormat::Raw,
                BookExportHierarchy::Toc,
                "unsupported book export combination: format=raw, hierarchy=toc",
            ),
            (
                BookExportFormat::Markdown,
                BookExportHierarchy::Raw,
                "unsupported book export combination: format=markdown, hierarchy=raw",
            ),
        ] {
            let error = export_book_content(
                PathBuf::from("missing.hbk"),
                PathBuf::from("target/book-export/unsupported-cli"),
                format,
                hierarchy,
            )
            .expect_err("raw/toc and markdown/raw must stay unsupported");

            assert_eq!(error.to_string(), expected);
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("v8-context-hbk-cli-{unique}-{name}"))
    }

    fn temp_workspace(name: &str) -> PathBuf {
        let path = temp_path(name);
        fs::create_dir_all(&path).expect("temp workspace must be created");
        path
    }

    fn write_book_fixture(path: &std::path::Path, storage_entries: Vec<(&str, &[u8])>) {
        fs::write(
            path,
            fixture_container(vec![
                (
                    "Book",
                    Some(
                        r#"{1,"Interface", {1,2,{"ru","fmtdui"}}, 1, "tag", {0,0}, 0}"#
                            .as_bytes()
                            .to_vec(),
                    ),
                ),
                ("PackBlock", None),
                ("FileStorage", Some(zip_entries(storage_entries))),
            ]),
        )
        .expect("fixture HBK must be written");
    }

    fn write_book_fixture_with_toc(
        path: &std::path::Path,
        toc: &str,
        storage_entries: Vec<(&str, &[u8])>,
    ) {
        fs::write(
            path,
            fixture_container(vec![
                (
                    "Book",
                    Some(
                        r#"{1,"Interface", {1,2,{"ru","fmtdui"}}, 1, "tag", {0,0}, 0}"#
                            .as_bytes()
                            .to_vec(),
                    ),
                ),
                ("PackBlock", Some(zip_bytes("toc.txt", toc.as_bytes()))),
                ("FileStorage", Some(zip_entries(storage_entries))),
            ]),
        )
        .expect("fixture HBK must be written");
    }

    fn metadata() -> IndexMetadata {
        IndexMetadata {
            locale: "ru".to_string(),
            source_locale: "ru".to_string(),
            source_hbk: "fixture.hbk".to_string(),
            source_extraction_schema_version: 11,
        }
    }

    fn platform_type_with_owner_path(primary: &str, owner: &str) -> model::PlatformType {
        model::PlatformType {
            identity: None,
            name: name(primary),
            semantic: semantic(model::RecordFamily::PlatformType, owner),
            type_kind: model::PlatformTypeKind::Regular,
            object_kind: Some(model::PlatformObjectKind::RegularPlatformType),
            extends: Vec::new(),
            metadata_kind: None,
            template_parameters: Vec::new(),
            type_template_key: None,
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: Some("type description".to_string()),
            facts: model::SectionFacts::default(),
            source: source(primary),
        }
    }

    fn type_method_with_owner_path(
        owner: &str,
        owner_path: &str,
        primary: &str,
    ) -> model::PlatformMethod {
        model::PlatformMethod {
            owner: name(owner),
            owner_identity: Some(format!("platform_type:{owner}:{owner_path}")),
            name: name(primary),
            semantic: semantic(model::RecordFamily::TypeMethod, owner_path),
            signatures: vec![model::Signature {
                text: format!("{primary}()"),
                parameters: Vec::new(),
                return_types: Vec::new(),
                variant: None,
            }],
            return_types: Vec::new(),
            description: Some("method description".to_string()),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn constructor(owner: &str, owner_identity: &str) -> model::Constructor {
        model::Constructor {
            owner: name(owner),
            owner_identity: Some(owner_identity.to_string()),
            name: name("По умолчанию"),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: format!("Новый {owner}()"),
                parameters: Vec::new(),
                return_types: Vec::new(),
                variant: None,
            }],
            description: None,
            facts: model::SectionFacts::default(),
            source: source(owner),
        }
    }

    fn semantic(record_family: model::RecordFamily, owner_path: &str) -> model::SemanticContext {
        model::SemanticContext::new(model::BranchKind::PlatformObjects, record_family)
            .with_owner_path(vec![name(owner_path)])
    }

    fn name(primary: &str) -> model::LocalizedName {
        model::LocalizedName {
            primary: primary.to_string(),
            alias: None,
        }
    }

    fn source(title: &str) -> model::SyntaxHelperSource {
        model::SyntaxHelperSource {
            hbk_path: "fixture.hbk".into(),
            locale: "ru".to_string(),
            toc_path: None,
            html_path: format!("{title}.html"),
            page_title: title.to_string(),
        }
    }
}
