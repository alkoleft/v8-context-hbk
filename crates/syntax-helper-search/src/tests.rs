#[cfg(test)]
mod tests {
    use super::*;
    use model::SyntaxHelperSink;
    use syntax_helper_language::{LanguagePageInput, LanguageSourceFamily, extract_language_facts};

    #[test]
    fn document_kind_roundtrips_storage_strings_and_priorities() {
        let expected = [
            (SearchDocumentKind::PlatformType, "platform_type", 10),
            (SearchDocumentKind::TypeProperty, "type_property", 20),
            (SearchDocumentKind::TypeMethod, "type_method", 30),
            (SearchDocumentKind::Constructor, "constructor", 40),
            (SearchDocumentKind::GlobalMethod, "global_method", 50),
            (SearchDocumentKind::GlobalProperty, "global_property", 60),
            (SearchDocumentKind::ModuleEvent, "module_event", 70),
            (SearchDocumentKind::TypeEvent, "type_event", 80),
            (SearchDocumentKind::UnknownEvent, "unknown_event", 90),
            (SearchDocumentKind::QueryTable, "query_table", 100),
            (
                SearchDocumentKind::QueryTableField,
                "query_table_field",
                110,
            ),
            (
                SearchDocumentKind::QueryTableParameter,
                "query_table_parameter",
                120,
            ),
            (SearchDocumentKind::LanguageType, "language_type", 125),
            (
                SearchDocumentKind::LanguageConstruct,
                "language_construct",
                126,
            ),
            (
                SearchDocumentKind::LanguageFunction,
                "language_function",
                127,
            ),
            (
                SearchDocumentKind::LanguageOperator,
                "language_operator",
                128,
            ),
            (SearchDocumentKind::LanguageKeyword, "language_keyword", 129),
            (SearchDocumentKind::LanguageLiteral, "language_literal", 130),
            (SearchDocumentKind::Enum, "enum", 140),
            (SearchDocumentKind::EnumValue, "enum_value", 150),
        ];

        assert_eq!(expected.len(), SearchDocumentKind::ALL.len());
        for (kind, stored, priority) in expected {
            assert_eq!(kind.as_str(), stored);
            assert_eq!(SearchDocumentKind::from_storage(stored), Some(kind));
            assert_eq!(kind.priority(), priority);
        }
        assert_eq!(SearchDocumentKind::from_storage("unexpected_kind"), None);
    }

    #[test]
    fn module_event_index_preserves_provider_neutral_module_context_kind() {
        let path = temp_path("module-context-kind.sqlite");
        let mut builder = SearchIndexBuilder::new();
        builder
            .global_context_event(module_event(
                model::ModuleKind::Form,
                &["Форма", "Form"],
                "ПриОткрытии",
            ))
            .expect("module event must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open");

        let events = index
            .get_by_name("module_context:form")
            .expect("module context relation lookup must not fail");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].document.name.primary, "ПриОткрытии");
        assert_eq!(events[0].document.kind, SearchDocumentKind::ModuleEvent);
    }

    #[test]
    fn index_accepts_language_facts_with_distinct_source_qualified_ids() {
        let path = temp_path("language.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for fact in language_fixture_facts("ru") {
            builder.add_language_fact(fact);
        }
        build_index_from_builder(&path, &metadata(), builder).expect("language index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let bsl = index
            .get_by_id("shlang:def_String")
            .expect("id lookup must work")
            .expect("BSL string type must be indexed");
        assert_eq!(bsl.document.kind, SearchDocumentKind::LanguageType);
        assert_eq!(bsl.document.name.primary, "Строка");

        let function_construct = index
            .get_by_id("shlang:def_Func")
            .expect("id lookup must work")
            .expect("BSL function construct must be indexed");
        assert_eq!(
            function_construct.document.kind,
            SearchDocumentKind::LanguageConstruct
        );
        assert!(
            function_construct
                .document
                .signatures
                .iter()
                .any(|signature| signature.text.contains("Функция"))
        );

        let select = index
            .get_by_id("shquery:SELECTStatement")
            .expect("id lookup must work")
            .expect("query SELECT construct must be indexed");
        assert_eq!(select.document.kind, SearchDocumentKind::LanguageConstruct);
        assert!(
            select
                .document
                .signatures
                .iter()
                .any(|signature| signature.text.contains("ВЫБРАТЬ"))
        );

        let sum = index
            .get_by_id("shquery:SUM")
            .expect("id lookup must work")
            .expect("query SUM function must be indexed");
        assert_eq!(sum.document.kind, SearchDocumentKind::LanguageFunction);
        assert_eq!(sum.document.name.primary, "СУММА");

        let query = index
            .get_by_id("shquery:STRING")
            .expect("id lookup must work")
            .expect("query STRING function must be indexed");
        assert_eq!(query.document.kind, SearchDocumentKind::LanguageFunction);
        assert_eq!(query.document.name.primary, "СТРОКА");

        let skd = index
            .get_by_id("dcsui:SKD_Functions_Strings#StringLength")
            .expect("id lookup must work")
            .expect("SKD string function must be indexed");
        assert_eq!(skd.document.kind, SearchDocumentKind::LanguageFunction);
        assert_eq!(skd.document.name.primary, "ДлинаСтроки");

        let string_hits = index
            .get_by_name("Строка")
            .expect("same-display lookup must work");
        let ids = string_hits
            .iter()
            .map(|hit| hit.document.id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(ids.contains("shlang:def_String"));
        assert!(ids.contains("shquery:STRING"));
        assert!(ids.contains("shquery:LitString"));
    }

    #[test]
    fn metadata_template_type_info_survives_index_roundtrip() {
        let path = temp_path("metadata-template.sqlite");
        let mut builder = SearchIndexBuilder::new();
        let mut manager = platform_type(
            "СправочникМенеджер.<Имя справочника>",
            Some("CatalogManager.<Catalog name>"),
            "Catalog manager template.",
        );
        manager.type_kind = model::PlatformTypeKind::MetadataTemplate;
        manager.metadata_kind = Some("СправочникМенеджер".to_string());
        manager.template_parameters = vec!["Имя справочника".to_string()];
        builder
            .platform_type(manager)
            .expect("platform template must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hit = index
            .type_identities_by_alias("CatalogManager.<Catalog name>")
            .expect("alias lookup must not fail")
            .pop()
            .expect("template type must resolve by alias");

        assert_eq!(
            hit.document.id,
            "platform_type:СправочникМенеджер.<Имя справочника>"
        );
        assert_eq!(
            hit.document.metadata_kind.as_deref(),
            Some("СправочникМенеджер")
        );
        assert_eq!(
            hit.document.template_parameters,
            vec!["Имя справочника".to_string()]
        );
        assert_eq!(
            hit.document.type_template_key,
            Some(model::PlatformTypeTemplateKey::new("Catalog", "Manager"))
        );

        let by_kind = index
            .type_template_by_key(&model::PlatformTypeTemplateKey::new("Catalog", "Manager"))
            .expect("semantic template lookup must not fail");
        assert_eq!(by_kind.len(), 1);
        assert_eq!(by_kind[0].document.id, hit.document.id);
    }

    #[test]
    fn type_template_classification_uses_longest_manager_root() {
        let path = temp_path("type-template-longest-root.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for (primary, alias) in [
            (
                "ДокументМенеджер.<Имя документа>",
                "DocumentManager.<Document name>",
            ),
            (
                "ЖурналДокументовМенеджер.<Имя журнала документов>",
                "DocumentJournalManager.<Document journal name>",
            ),
            (
                "ЖурналДокументовСсылка.<Имя журнала документов>",
                "DocumentJournalRef.<Document journal name>",
            ),
        ] {
            let mut record = platform_type(primary, Some(alias), "Template.");
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Имя".to_string()];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }

        build_index_from_builder(&path, &metadata(), builder).expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hit = index
            .type_template_by_key(&model::PlatformTypeTemplateKey::new(
                "DocumentJournal",
                "Ref",
            ))
            .expect("type-template lookup must not fail")
            .pop()
            .expect("document journal ref must resolve");

        assert_eq!(
            hit.document.type_template_key,
            Some(model::PlatformTypeTemplateKey::new(
                "DocumentJournal",
                "Ref",
            ))
        );
    }

    #[test]
    fn type_template_classification_uses_external_data_source_table_before_source() {
        let path = temp_path("type-template-external-data-source-longest-root.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for (primary, alias) in [
            (
                "ВнешнийИсточникДанныхМенеджер.<Имя внешнего источника>",
                "ExternalDataSourceManager.<External data source name>",
            ),
            (
                "ВнешнийИсточникДанныхТаблицаМенеджер.<Имя внешнего источника, Имя таблицы>",
                "ExternalDataSourceTableManager.<External data source name, Table name>",
            ),
            (
                "ВнешнийИсточникДанныхТаблицаСсылка.<Имя внешнего источника, Имя таблицы>",
                "ExternalDataSourceTableRef.<External data source name, Table name>",
            ),
        ] {
            let mut record = platform_type(primary, Some(alias), "Template.");
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec![
                "Имя внешнего источника".to_string(),
                "Имя таблицы".to_string(),
            ];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }

        build_index_from_builder(&path, &metadata(), builder).expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hit = index
            .type_template_by_key(&model::PlatformTypeTemplateKey::new(
                "ExternalDataSourceTable",
                "Ref",
            ))
            .expect("type-template lookup must not fail")
            .pop()
            .expect("external data source table ref must resolve");

        assert_eq!(
            hit.document.type_template_key,
            Some(model::PlatformTypeTemplateKey::new(
                "ExternalDataSourceTable",
                "Ref",
            ))
        );
    }

    #[test]
    fn type_template_classification_allows_primary_fallback_only_for_root_source() {
        let mut root_metadata = metadata();
        root_metadata.locale = "en".to_string();
        root_metadata.source_locale = "root".to_string();
        let root_path = temp_path("type-template-root-primary-fallback.sqlite");
        let mut root_builder = SearchIndexBuilder::new();
        for primary in [
            "DocumentManager.<Document name>",
            "DocumentRef.<Document name>",
        ] {
            let mut record = platform_type(primary, None, "Template.");
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Document name".to_string()];
            root_builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        build_index_from_builder(&root_path, &root_metadata, root_builder)
            .expect("root index must build");
        let root_index = SearchIndex::open_read_only(&root_path).expect("index must open");
        assert_eq!(
            root_index
                .type_template_by_key(&model::PlatformTypeTemplateKey::new("Document", "Ref"))
                .expect("type-template lookup must not fail")
                .len(),
            1
        );

        let ru_path = temp_path("type-template-ru-primary-no-fallback.sqlite");
        let mut ru_builder = SearchIndexBuilder::new();
        let mut record = platform_type("ДокументСсылка.<Имя документа>", None, "Template.");
        record.type_kind = model::PlatformTypeKind::MetadataTemplate;
        record.metadata_kind = Some("ДокументСсылка".to_string());
        record.template_parameters = vec!["Имя документа".to_string()];
        ru_builder
            .platform_type(record)
            .expect("platform template must sink");
        let report = build_index_from_builder_with_report(&ru_path, &metadata(), ru_builder)
            .expect("ru index must build");

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "TYPE_TEMPLATE_UNCLASSIFIED");
        let ru_index = SearchIndex::open_read_only(&ru_path).expect("index must open");
        let hit = ru_index
            .type_identity_by_id("platform_type:ДокументСсылка.<Имя документа>")
            .expect("lookup must not fail")
            .expect("template must exist");
        assert!(hit.document.type_template_key.is_none());
        assert!(
            hit.document
                .type_template_classification_diagnostic
                .as_deref()
                .is_some_and(|value| value.contains("primary fallback"))
        );
    }

    #[test]
    fn type_template_classification_uses_direct_refs_for_unassigned_templates() {
        let path = temp_path("type-template-direct-ref-family.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for (primary, alias) in [
            (
                "ПланВидовРасчетаМенеджер.<Имя плана видов расчета>",
                "ChartOfCalculationTypesManager.<Chart name>",
            ),
            (
                "ПланВидовРасчетаОбъект.<Имя плана видов расчета>",
                "ChartOfCalculationTypesObject.<Chart name>",
            ),
            (
                "ПланВидовРасчетаСсылка.<Имя плана видов расчета>",
                "ChartOfCalculationTypesRef.<Chart name>",
            ),
            (
                "БазовыеВидыРасчета.<Имя плана видов расчета>",
                "BaseCalculationTypes.<Chart name>",
            ),
            (
                "БазовыеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "BaseCalculationTypesRow.<Chart name>",
            ),
            (
                "ВедущиеВидыРасчета.<Имя плана видов расчета>",
                "LeadingCalculationTypes.<Chart name>",
            ),
            (
                "ВедущиеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "LeadingCalculationTypesRow.<Chart name>",
            ),
            (
                "ВытесняющиеВидыРасчета.<Имя плана видов расчета>",
                "DisplacingCalculationTypes.<Chart name>",
            ),
            (
                "ВытесняющиеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "DisplacingCalculationTypesRow.<Chart name>",
            ),
        ] {
            let mut record = platform_type(primary, Some(alias), "Template.");
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Имя плана видов расчета".to_string()];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        for (name_primary, name_alias, target_type) in [
            (
                "БазовыеВидыРасчета",
                "BaseCalculationTypes",
                "БазовыеВидыРасчета",
            ),
            (
                "ВедущиеВидыРасчета",
                "LeadingCalculationTypes",
                "ВедущиеВидыРасчета",
            ),
            (
                "ВытесняющиеВидыРасчета",
                "DisplacingCalculationTypes",
                "ВытесняющиеВидыРасчета",
            ),
        ] {
            builder
                .type_property(model::PlatformProperty {
                    owner: name(
                        "ПланВидовРасчетаОбъект.<Имя плана видов расчета>",
                        Some("ChartOfCalculationTypesObject.<Chart name>"),
                    ),
                    owner_identity: Some(
                        "platform_type:ПланВидовРасчетаОбъект.<Имя плана видов расчета>"
                            .to_string(),
                    ),
                    name: name(name_primary, Some(name_alias)),
                    semantic: model::SemanticContext::default(),
                    usage: None,
                    type_refs: vec![model::TypeRef {
                        name: target_type.to_string(),
                    }],
                    description: None,
                    facts: model::SectionFacts::default(),
                    source: source(name_primary),
                })
                .expect("property must sink");
        }
        for (owner_primary, owner_alias) in [
            (
                "БазовыеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "BaseCalculationTypesRow.<Chart name>",
            ),
            (
                "ВедущиеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "LeadingCalculationTypesRow.<Chart name>",
            ),
            (
                "ВытесняющиеВидыРасчетаСтрока.<Имя плана видов расчета>",
                "DisplacingCalculationTypesRow.<Chart name>",
            ),
        ] {
            builder
                .type_property(model::PlatformProperty {
                    owner: name(owner_primary, Some(owner_alias)),
                    owner_identity: Some(format!("platform_type:{owner_primary}")),
                    name: name("ВидРасчета", Some("CalculationType")),
                    semantic: model::SemanticContext::default(),
                    usage: None,
                    type_refs: vec![model::TypeRef {
                        name: "ПланВидовРасчетаСсылка".to_string(),
                    }],
                    description: None,
                    facts: model::SectionFacts::default(),
                    source: source(owner_primary),
                })
                .expect("property must sink");
        }

        let report = build_index_from_builder_with_report(&path, &metadata(), builder)
            .expect("index must build");
        assert!(
            report
                .warnings
                .iter()
                .all(|warning| !warning.code.starts_with("TYPE_TEMPLATE_"))
        );
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        for (variant, expected_id) in [
            (
                "BaseCalculationTypes",
                "platform_type:БазовыеВидыРасчета.<Имя плана видов расчета>",
            ),
            (
                "BaseCalculationTypesRow",
                "platform_type:БазовыеВидыРасчетаСтрока.<Имя плана видов расчета>",
            ),
            (
                "LeadingCalculationTypes",
                "platform_type:ВедущиеВидыРасчета.<Имя плана видов расчета>",
            ),
            (
                "LeadingCalculationTypesRow",
                "platform_type:ВедущиеВидыРасчетаСтрока.<Имя плана видов расчета>",
            ),
            (
                "DisplacingCalculationTypes",
                "platform_type:ВытесняющиеВидыРасчета.<Имя плана видов расчета>",
            ),
            (
                "DisplacingCalculationTypesRow",
                "platform_type:ВытесняющиеВидыРасчетаСтрока.<Имя плана видов расчета>",
            ),
        ] {
            let hits = index
                .type_template_by_key(&model::PlatformTypeTemplateKey::new(
                    "ChartOfCalculationTypes",
                    variant,
                ))
                .expect("type-template lookup must not fail");
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].document.id, expected_id);
            assert!(
                hits[0]
                    .document
                    .type_template_classification_diagnostic
                    .as_deref()
                    .is_some_and(|value| value.starts_with("direct_type_ref "))
            );
        }
    }

    #[test]
    fn type_template_classification_reports_unclassified_templates() {
        let path = temp_path("type-template-unclassified.sqlite");
        let mut builder = SearchIndexBuilder::new();
        let mut record = platform_type(
            "ОдиночныйШаблон.<Имя>",
            Some("SingleTemplate.<Name>"),
            "Template.",
        );
        record.type_kind = model::PlatformTypeKind::MetadataTemplate;
        record.metadata_kind = Some("ОдиночныйШаблон".to_string());
        record.template_parameters = vec!["Имя".to_string()];
        builder
            .platform_type(record)
            .expect("platform template must sink");

        let report = build_index_from_builder_with_report(&path, &metadata(), builder)
            .expect("index must build");
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "TYPE_TEMPLATE_UNCLASSIFIED");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hit = index
            .type_identity_by_id("platform_type:ОдиночныйШаблон.<Имя>")
            .expect("lookup must not fail")
            .expect("template must exist");
        assert_eq!(
            hit.document
                .type_template_classification_diagnostic
                .as_deref(),
            Some(report.warnings[0].message.as_str())
        );
    }

    #[test]
    fn type_template_classification_persists_ambiguous_diagnostics() {
        let path = temp_path("type-template-ambiguous.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for (primary, alias) in [
            ("ПервыйМенеджер.<Имя>", "FirstManager.<Name>"),
            ("ПервыйОбъект.<Имя>", "FirstObject.<Name>"),
            ("ВторойМенеджер.<Имя>", "SecondManager.<Name>"),
            ("ВторойОбъект.<Имя>", "SecondObject.<Name>"),
            ("ОбщийШаблон.<Имя>", "SharedTemplate.<Name>"),
        ] {
            let mut record = platform_type(primary, Some(alias), "Template.");
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Имя".to_string()];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        for (owner_primary, owner_alias, property_primary) in [
            ("ПервыйОбъект.<Имя>", "FirstObject.<Name>", "ОбщийИзПервого"),
            (
                "ВторойОбъект.<Имя>",
                "SecondObject.<Name>",
                "ОбщийИзВторого",
            ),
        ] {
            builder
                .type_property(model::PlatformProperty {
                    owner: name(owner_primary, Some(owner_alias)),
                    owner_identity: Some(format!("platform_type:{owner_primary}")),
                    name: name(property_primary, None),
                    semantic: model::SemanticContext::default(),
                    usage: None,
                    type_refs: vec![model::TypeRef {
                        name: "ОбщийШаблон".to_string(),
                    }],
                    description: None,
                    facts: model::SectionFacts::default(),
                    source: source(property_primary),
                })
                .expect("property must sink");
        }

        let report = build_index_from_builder_with_report(&path, &metadata(), builder)
            .expect("index must build");
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "TYPE_TEMPLATE_AMBIGUOUS_FAMILY");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hit = index
            .type_identity_by_id("platform_type:ОбщийШаблон.<Имя>")
            .expect("lookup must not fail")
            .expect("template must exist");
        assert_eq!(
            hit.document
                .type_template_classification_diagnostic
                .as_deref(),
            Some(report.warnings[0].message.as_str())
        );
    }

    #[test]
    fn type_template_type_refs_preserve_owner_parameter_binding() {
        let path = temp_path("type-template-binding.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for mut record in [
            platform_type(
                "ДокументМенеджер.<Имя документа>",
                Some("DocumentManager.<Document name>"),
                "Document manager template.",
            ),
            platform_type(
                "ДокументОбъект.<Имя документа>",
                Some("DocumentObject.<Document name>"),
                "Document object template.",
            ),
            platform_type(
                "ДокументСсылка.<Имя документа>",
                Some("DocumentRef.<Document name>"),
                "Document reference template.",
            ),
        ] {
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Имя документа".to_string()];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        builder
            .type_property(model::PlatformProperty {
                owner: name(
                    "ДокументОбъект.<Имя документа>",
                    Some("DocumentObject.<Document name>"),
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name("Ссылка", Some("Ref")),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ДокументСсылка".to_string(),
                }],
                description: Some("Document reference.".to_string()),
                facts: model::SectionFacts::default(),
                source: source("document-object-ref"),
            })
            .expect("type-template property must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let property = index
            .member_by_owner_type_id("platform_type:ДокументОбъект.<Имя документа>", "Ссылка")
            .expect("property lookup must not fail")
            .pop()
            .expect("property must resolve");

        assert_eq!(property.document.type_ref_facts.len(), 1);
        let binding = property.document.type_ref_facts[0]
            .template_binding
            .as_ref()
            .expect("type ref must preserve template binding");
        assert_eq!(
            binding.template_key,
            model::PlatformTypeTemplateKey::new("Document", "Ref")
        );
        assert_eq!(
            binding.arguments,
            vec![model::TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: 0,
                target_parameter_index: 0,
            }]
        );
    }

    #[test]
    fn type_template_type_refs_preserve_multiple_owner_parameter_bindings() {
        let path = temp_path("type-template-multi-binding.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for mut record in [
            platform_type(
                "ВнешнийИсточникДанныхТаблицаМенеджер.<Имя внешнего источника, Имя таблицы>",
                Some("ExternalDataSourceTableManager.<External data source name, Table name>"),
                "External table manager template.",
            ),
            platform_type(
                "ВнешнийИсточникДанныхТаблицаОбъект.<Имя внешнего источника, Имя таблицы>",
                Some("ExternalDataSourceTableObject.<External data source name, Table name>"),
                "External table object template.",
            ),
            platform_type(
                "ВнешнийИсточникДанныхТаблицаСсылка.<Имя внешнего источника, Имя таблицы>",
                Some("ExternalDataSourceTableRef.<External data source name, Table name>"),
                "External table reference template.",
            ),
        ] {
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec![
                "Имя внешнего источника".to_string(),
                "Имя таблицы".to_string(),
            ];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        builder
            .type_property(model::PlatformProperty {
                owner: name(
                    "ВнешнийИсточникДанныхТаблицаОбъект.<Имя внешнего источника, Имя таблицы>",
                    Some("ExternalDataSourceTableObject.<External data source name, Table name>"),
                ),
                owner_identity: Some(
                    "platform_type:ВнешнийИсточникДанныхТаблицаОбъект.<Имя внешнего источника, Имя таблицы>"
                        .to_string(),
                ),
                name: name("Ссылка", Some("Ref")),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ВнешнийИсточникДанныхТаблицаСсылка".to_string(),
                }],
                description: Some("External table reference.".to_string()),
                facts: model::SectionFacts::default(),
                source: source("external-data-source-table-object-ref"),
            })
            .expect("type-template property must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let property = index
            .member_by_owner_type_id(
                "platform_type:ВнешнийИсточникДанныхТаблицаОбъект.<Имя внешнего источника, Имя таблицы>",
                "Ссылка",
            )
            .expect("property lookup must not fail")
            .pop()
            .expect("property must resolve");

        let binding = property.document.type_ref_facts[0]
            .template_binding
            .as_ref()
            .expect("type ref must preserve template binding");
        assert_eq!(
            binding.template_key,
            model::PlatformTypeTemplateKey::new("ExternalDataSourceTable", "Ref")
        );
        assert_eq!(
            binding.arguments,
            vec![
                model::TemplateParameterBinding::OwnerParameter {
                    owner_parameter_index: 0,
                    target_parameter_index: 0,
                },
                model::TemplateParameterBinding::OwnerParameter {
                    owner_parameter_index: 1,
                    target_parameter_index: 1,
                },
            ]
        );
    }

    #[test]
    fn type_template_callable_parameter_and_return_refs_preserve_owner_parameter_binding() {
        let path = temp_path("type-template-callable-binding.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for mut record in [
            platform_type(
                "ДокументМенеджер.<Имя документа>",
                Some("DocumentManager.<Document name>"),
                "Document manager template.",
            ),
            platform_type(
                "ДокументОбъект.<Имя документа>",
                Some("DocumentObject.<Document name>"),
                "Document object template.",
            ),
            platform_type(
                "ДокументСсылка.<Имя документа>",
                Some("DocumentRef.<Document name>"),
                "Document reference template.",
            ),
        ] {
            record.type_kind = model::PlatformTypeKind::MetadataTemplate;
            record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
            record.template_parameters = vec!["Имя документа".to_string()];
            builder
                .platform_type(record)
                .expect("platform template must sink");
        }
        builder
            .type_method(model::PlatformMethod {
                owner: name(
                    "ДокументОбъект.<Имя документа>",
                    Some("DocumentObject.<Document name>"),
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name("Связать", Some("Link")),
                semantic: model::SemanticContext::default(),
                signatures: vec![model::Signature {
                    text: "Связать(<Ссылка>)".to_string(),
                    parameters: vec![model::Parameter {
                        name: "Ссылка".to_string(),
                        required: true,
                        type_refs: vec![model::TypeRef {
                            name: "ДокументСсылка".to_string(),
                        }],
                        description: Some("Document reference.".to_string()),
                    }],
                    return_types: vec![model::TypeRef {
                        name: "ДокументСсылка".to_string(),
                    }],
                    variant: None,
                }],
                return_types: Vec::new(),
                description: Some("Returns a linked document reference.".to_string()),
                facts: model::SectionFacts::default(),
                source: source("document-object-link"),
            })
            .expect("type-template method must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let method = index
            .callable_by_owner_type_id("platform_type:ДокументОбъект.<Имя документа>", "Связать")
            .expect("method lookup must not fail")
            .pop()
            .expect("method must resolve");

        let parameter_binding = method.document.signatures[0].parameters[0].type_ref_facts[0]
            .template_binding
            .as_ref()
            .expect("parameter type ref must preserve template binding");
        assert_eq!(
            parameter_binding.template_key,
            model::PlatformTypeTemplateKey::new("Document", "Ref")
        );
        assert_eq!(
            parameter_binding.arguments,
            vec![model::TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: 0,
                target_parameter_index: 0,
            }]
        );

        let return_binding = method.document.signatures[0].return_type_facts[0]
            .template_binding
            .as_ref()
            .expect("signature return type ref must preserve template binding");
        assert_eq!(return_binding, parameter_binding);
    }

    #[test]
    fn type_template_binding_does_not_choose_ambiguous_type_ref_target() {
        let path = temp_path("type-template-binding-ambiguous.sqlite");
        let mut builder = SearchIndexBuilder::new();
        let mut owner = platform_type(
            "ДокументОбъект.<Имя документа>",
            Some("DocumentObject.<Document name>"),
            "Document object template.",
        );
        owner.type_kind = model::PlatformTypeKind::MetadataTemplate;
        owner.metadata_kind = Some("ДокументОбъект".to_string());
        owner.template_parameters = vec!["Имя документа".to_string()];
        builder
            .platform_type(owner)
            .expect("owner template must sink");
        for owner_path in ["Документы продаж", "Документы склада"] {
            let mut reference = platform_type(
                "ДокументСсылка.<Имя документа>",
                Some("DocumentRef.<Document name>"),
                "Document reference template.",
            );
            reference.semantic = semantic_path(model::RecordFamily::PlatformType, &[owner_path]);
            reference.type_kind = model::PlatformTypeKind::MetadataTemplate;
            reference.metadata_kind = Some("ДокументСсылка".to_string());
            reference.template_parameters = vec!["Имя документа".to_string()];
            builder
                .platform_type(reference)
                .expect("target template must sink");
        }
        builder
            .type_property(model::PlatformProperty {
                owner: name(
                    "ДокументОбъект.<Имя документа>",
                    Some("DocumentObject.<Document name>"),
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name("Ссылка", Some("Ref")),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ДокументСсылка".to_string(),
                }],
                description: Some("Document reference.".to_string()),
                facts: model::SectionFacts::default(),
                source: source("ambiguous-document-object-ref"),
            })
            .expect("type-template property must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let property = index
            .member_by_owner_type_id("platform_type:ДокументОбъект.<Имя документа>", "Ссылка")
            .expect("property lookup must not fail")
            .pop()
            .expect("property must resolve");

        assert_eq!(property.document.type_ref_facts.len(), 1);
        assert!(matches!(
            property.document.type_ref_facts[0].target,
            SearchTypeRefTarget::Ambiguous(_)
        ));
        assert_eq!(property.document.type_ref_facts[0].template_binding, None);
    }

    #[test]
    fn index_accepts_root_language_facts_with_same_logical_ids() {
        let path = temp_path("language-root.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for fact in language_fixture_facts("root") {
            builder.add_language_fact(fact);
        }
        build_index_from_builder(&path, &metadata(), builder)
            .expect("root language index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let bsl = index
            .get_by_id("shlang:def_String")
            .expect("id lookup must work")
            .expect("root BSL string type must be indexed");
        assert_eq!(bsl.document.kind, SearchDocumentKind::LanguageType);
        assert_eq!(bsl.document.name.primary, "String");

        let sum = index
            .get_by_id("shquery:SUM")
            .expect("id lookup must work")
            .expect("root query SUM function must be indexed");
        assert_eq!(sum.document.kind, SearchDocumentKind::LanguageFunction);
        assert_eq!(sum.document.name.primary, "SUM");

        let skd = index
            .get_by_id("dcsui:SKD_Functions_Strings#StringLength")
            .expect("id lookup must work")
            .expect("root SKD string function must be indexed");
        assert_eq!(skd.document.kind, SearchDocumentKind::LanguageFunction);
        assert_eq!(skd.document.name.primary, "StringLength");
    }

    #[test]
    fn language_relations_use_source_qualified_targets_not_same_name_winners() {
        let bsl_string = language_fact(
            "shlang:def_String",
            LanguageSourceFamily::Shlang,
            language::LanguageDomain::BslLanguage,
            language::LanguageFactFamily::Type,
            name("Строка", Some("String")),
        );
        let query_literal = language_fact(
            "shquery:LitString",
            LanguageSourceFamily::Shquery,
            language::LanguageDomain::QueryLanguage,
            language::LanguageFactFamily::Literal,
            name("Строка", Some("STRING")),
        );
        let bsl_boolean = language_fact(
            "shlang:def_Boolean",
            LanguageSourceFamily::Shlang,
            language::LanguageDomain::BslLanguage,
            language::LanguageFactFamily::Type,
            name("Булево", Some("Boolean")),
        );
        let query_boolean = language_fact(
            "shquery:LitBoolean",
            LanguageSourceFamily::Shquery,
            language::LanguageDomain::QueryLanguage,
            language::LanguageFactFamily::Literal,
            name("Булево", Some("BOOLEAN")),
        );
        let mut query_string = language_fact(
            "shquery:STRING",
            LanguageSourceFamily::Shquery,
            language::LanguageDomain::QueryLanguage,
            language::LanguageFactFamily::Function,
            name("СТРОКА", Some("STRING")),
        );
        let mut query_boolean_fn = language_fact(
            "shquery:BOOLEAN",
            LanguageSourceFamily::Shquery,
            language::LanguageDomain::QueryLanguage,
            language::LanguageFactFamily::Function,
            name("БУЛЕВО", Some("BOOLEAN")),
        );
        query_string.signatures = vec![language::LanguageSignature {
            text: "СТРОКА(<Значение>)".to_string(),
            parameters: vec![language::LanguageParameter {
                name: "Значение".to_string(),
                required: true,
                type_refs: vec!["Строка".to_string()],
                description: None,
            }],
        }];
        query_string.return_types = vec!["Строка".to_string()];
        query_boolean_fn.signatures = vec![language::LanguageSignature {
            text: "БУЛЕВО(<Значение>)".to_string(),
            parameters: vec![language::LanguageParameter {
                name: "Значение".to_string(),
                required: true,
                type_refs: vec!["Булево".to_string()],
                description: None,
            }],
        }];
        query_boolean_fn.return_types = vec!["Булево".to_string()];

        let documents = vec![
            language_document(&bsl_string),
            language_document(&bsl_boolean),
            language_document(&query_literal),
            language_document(&query_boolean),
            language_document(&query_string),
            language_document(&query_boolean_fn),
        ];
        let relations = relations_from_documents(&documents);

        assert!(relations.iter().any(|relation| {
            relation.source_id == "shquery:STRING"
                && relation.target_id == "shquery:LitString"
                && relation.edge_kind == "has_type"
        }));
        assert!(relations.iter().any(|relation| {
            relation.source_id == "shquery:STRING"
                && relation.target_id == "shquery:LitString"
                && relation.edge_kind == "returns"
        }));
        assert!(!relations.iter().any(|relation| {
            relation.source_id == "shquery:STRING" && relation.target_id == "shlang:def_String"
        }));
        assert!(!relations.iter().any(|relation| {
            relation.source_id == "shquery:BOOLEAN"
                && matches!(
                    relation.target_id.as_str(),
                    "shlang:def_Boolean" | "shquery:LitBoolean"
                )
        }));
    }

    #[test]
    fn index_supports_exact_keyword_fuzzy_and_related_queries() {
        let path = temp_path("query.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");

        let exact = index
            .get_by_name("DataCompositionFilter")
            .expect("exact lookup must work");
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].document.name.primary, "ОтборКомпоновкиДанных");
        assert_eq!(exact[0].document.kind, SearchDocumentKind::PlatformType);

        let event = index
            .get_by_name("ПередЗаписью")
            .expect("event lookup must work");
        assert_eq!(event[0].document.kind, SearchDocumentKind::TypeEvent);

        let member = index
            .get_by_owner_member("НастройкиКомпоновкиДанных", "Отбор")
            .expect("owner/member lookup must work");
        assert_eq!(member[0].document.type_refs, ["ОтборКомпоновкиДанных"]);

        let by_id = index
            .get_by_id("type_property:platform_type:НастройкиКомпоновкиДанных:Отбор")
            .expect("id lookup must work")
            .expect("member document id must exist");
        assert_eq!(by_id.document.name.primary, "Отбор");

        let type_identity = index
            .type_identities_by_alias("DataCompositionFilter")
            .expect("type alias lookup must work");
        assert_eq!(type_identity.len(), 1);
        assert_eq!(
            type_identity[0].document.id,
            "platform_type:ОтборКомпоновкиДанных"
        );

        let members = index
            .members_by_type_id("platform_type:ОтборКомпоновкиДанных")
            .expect("member listing must work");
        assert!(members.iter().any(|hit| {
            hit.document.kind == SearchDocumentKind::TypeProperty
                && hit.document.name.primary == "Элементы"
        }));
        assert!(members.iter().any(|hit| {
            hit.document.kind == SearchDocumentKind::TypeMethod
                && hit.document.name.primary == "ПолучитьОбъектПоИдентификатору"
        }));

        let owner_type_member = index
            .member_by_owner_type_id("platform_type:НастройкиКомпоновкиДанных", "Отбор")
            .expect("owner type member lookup must work");
        assert_eq!(owner_type_member.len(), 1);
        assert_eq!(
            owner_type_member[0].document.id,
            "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"
        );

        let callable = index
            .callable_by_owner_type_id(
                "platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных",
                "Добавить",
            )
            .expect("owner type callable lookup must work");
        assert_eq!(callable.len(), 1);
        assert_eq!(
            callable[0].document.return_types,
            ["ЭлементОтбораКомпоновкиДанных"]
        );

        let keyword = index
            .search("отбор скд", SearchMode::Keywords, 10)
            .expect("keyword search must work");
        assert_eq!(keyword[0].document.name.primary, "ОтборКомпоновкиДанных");

        let fuzzy = index
            .search("ОтборКомпоновкиДаных", SearchMode::Fuzzy, 10)
            .expect("fuzzy search must work");
        assert_eq!(fuzzy[0].document.name.primary, "ОтборКомпоновкиДанных");

        let ambiguous_related = index
            .related_by_name("ОтборКомпоновкиДанных", 5, 20)
            .expect_err("plain-name related root must report ambiguity");
        assert!(matches!(
            ambiguous_related,
            SearchError::AmbiguousLookup { matches: 2, .. }
        ));
        let related = index
            .related_by_id("platform_type:ОтборКомпоновкиДанных", 5, 20)
            .expect("id-root related search must work");
        let names = related
            .iter()
            .map(|hit| hit.document.name.primary.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"Новый ОтборКомпоновкиДанных()"));
        assert!(names.contains(&"Элементы"));
        assert!(names.contains(&"Добавить"));
        assert!(names.contains(&"ЛевоеЗначение"));

        let related_by_owner_member = index
            .related_by_owner_member("НастройкиКомпоновкиДанных", "Отбор", 5, 20)
            .expect("owner/member related search must work");
        assert!(
            related_by_owner_member
                .iter()
                .any(|hit| hit.document.name.primary == "ОтборКомпоновкиДанных")
        );
        assert!(
            related_by_owner_member
                .iter()
                .any(|hit| hit.document.name.primary == "Добавить")
        );
        assert!(related_by_owner_member.iter().any(|hit| {
            hit.document.owner.as_ref().is_some_and(|owner| {
                owner.primary == "ЭлементОтбораКомпоновкиДанных"
                    && hit.document.name.primary == "ЛевоеЗначение"
            })
        }));

        let related_by_id = index
            .related_by_id(
                "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор",
                5,
                20,
            )
            .expect("id-root related search must work");
        assert_eq!(related_by_id, related_by_owner_member);

        let type_refs = index
            .related_by_id_and_edge(
                "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор",
                "has_type",
                20,
            )
            .expect("edge-filtered related search must work");
        assert_eq!(type_refs.len(), 1);
        assert_eq!(
            type_refs[0].document.id,
            "platform_type:ОтборКомпоновкиДанных"
        );

        let owner_refs = index
            .related_by_id_and_edge(
                "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор",
                "member_of",
                20,
            )
            .expect("member_of edge-filtered related search must work");
        assert_eq!(owner_refs.len(), 1);
        assert_eq!(
            owner_refs[0].document.id,
            "platform_type:НастройкиКомпоновкиДанных"
        );
        assert_eq!(owner_refs[0].via[0].edge_kind, "member_of");

        let ambiguous_constructors = index
            .constructors_by_name("ОтборКомпоновкиДанных")
            .expect_err("plain constructor type root must report ambiguity");
        assert!(matches!(
            ambiguous_constructors,
            SearchError::AmbiguousLookup { matches: 2, .. }
        ));
        let constructors = index
            .constructors_by_type_id("platform_type:ОтборКомпоновкиДанных")
            .expect("type-id constructor lookup must work");
        assert_eq!(constructors.len(), 1);
        assert_eq!(
            constructors[0].document.signature_text_lines(),
            ["Новый ОтборКомпоновкиДанных()"]
        );
    }

    #[test]
    fn index_preserves_overload_specific_return_types_on_signature_rows() {
        let path = temp_path("signature-return.sqlite");
        let mut context = fixture_context();
        context.type_methods.push(model::PlatformMethod {
            owner: name("ОтборКомпоновкиДанных", None),
            owner_identity: Some("platform_type:ОтборКомпоновкиДанных".to_string()),
            name: name("ПолучитьЗначение", None),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: "ПолучитьЗначение()".to_string(),
                parameters: Vec::new(),
                return_types: vec![model::TypeRef {
                    name: "Строка".to_string(),
                }],
                variant: None,
            }],
            return_types: Vec::new(),
            description: Some("Получает значение.".to_string()),
            facts: model::SectionFacts::default(),
            source: source("ОтборКомпоновкиДанных.ПолучитьЗначение"),
        });
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open");
        let callable = index
            .callable_by_owner_type_id("platform_type:ОтборКомпоновкиДанных", "ПолучитьЗначение")
            .expect("callable lookup must work");

        assert_eq!(callable.len(), 1);
        assert!(callable[0].document.return_types.is_empty());
        assert_eq!(callable[0].document.signatures[0].return_types, ["Строка"]);

        let connection = Connection::open(&path).expect("index sqlite must open");
        assert!(connection
            .query_row(
                "SELECT 1
                 FROM type_refs
                 WHERE source_document_id = 'type_method:platform_type:ОтборКомпоновкиДанных:ПолучитьЗначение'
                   AND ref_kind = 'return_type'
                   AND source_signature_id IS NOT NULL
                   AND source_signature_ordinal = 0
                   AND target_type_name = 'Строка'
                 LIMIT 1",
                [],
                |_| Ok(()),
            )
            .is_ok());
    }

    #[test]
    fn owner_type_member_and_callable_lookup_match_primary_and_alias_names() {
        let path = temp_path("owner-type-primary-alias-lookup.sqlite");
        let mut context = fixture_context();
        context.type_properties.push(model::PlatformProperty {
            owner: name("НастройкиКомпоновкиДанных", None),
            owner_identity: None,
            name: name("ПользовательскийОтбор", Some("CustomFilter")),
            semantic: model::SemanticContext::default(),
            usage: None,
            type_refs: vec![model::TypeRef {
                name: "ОтборКомпоновкиДанных".to_string(),
            }],
            description: Some("ПользовательскийОтбор description".to_string()),
            facts: model::SectionFacts::default(),
            source: source("НастройкиКомпоновкиДанных.ПользовательскийОтбор"),
        });
        context.type_methods.push(model::PlatformMethod {
            owner: name("ОтборКомпоновкиДанных", None),
            owner_identity: None,
            name: name("Найти", Some("Find")),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: "Найти()".to_string(),
                parameters: Vec::new(),
                return_types: Vec::new(),
                variant: None,
            }],
            return_types: vec![model::TypeRef {
                name: "ЭлементОтбораКомпоновкиДанных".to_string(),
            }],
            description: Some("Найти description".to_string()),
            facts: model::SectionFacts::default(),
            source: source("ОтборКомпоновкиДанных.Найти"),
        });
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let member_by_primary = index
            .member_by_owner_type_id(
                "platform_type:НастройкиКомпоновкиДанных",
                "ПользовательскийОтбор",
            )
            .expect("member primary lookup must work");
        let member_by_alias = index
            .member_by_owner_type_id("platform_type:НастройкиКомпоновкиДанных", "CustomFilter")
            .expect("member alias lookup must work");
        assert_eq!(member_by_primary, member_by_alias);
        assert_eq!(member_by_alias.len(), 1);
        assert_eq!(
            member_by_alias[0].document.id,
            "type_property:platform_type:НастройкиКомпоновкиДанных:ПользовательскийОтбор"
        );

        let callable_by_primary = index
            .callable_by_owner_type_id("platform_type:ОтборКомпоновкиДанных", "Найти")
            .expect("callable primary lookup must work");
        let callable_by_alias = index
            .callable_by_owner_type_id("platform_type:ОтборКомпоновкиДанных", "Find")
            .expect("callable alias lookup must work");
        assert_eq!(callable_by_primary, callable_by_alias);
        assert_eq!(callable_by_alias.len(), 1);
        assert_eq!(
            callable_by_alias[0].document.id,
            "type_method:platform_type:ОтборКомпоновкиДанных:Найти"
        );
    }

    #[test]
    fn keyword_search_prefers_exact_identity_for_simple_symbol() {
        let path = temp_path("simple-symbol-ranking.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type("Структура", Some("Structure"), "Коллекция значений."),
                platform_type(
                    "СтруктураНастроекКомпоновкиДанных",
                    None,
                    "Структура настроек компоновки данных.",
                ),
                platform_type(
                    "НастройкиКомпоновкиДанных",
                    None,
                    "Настройки системы компоновки данных.",
                ),
            ],
            type_properties: vec![
                type_property("НастройкиКомпоновкиДанных", "Структура", "Структура"),
                type_property(
                    "СтруктураНастроекКомпоновкиДанных",
                    "Структура",
                    "Структура",
                ),
            ],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");

        let hits = index
            .search("Структура", SearchMode::Keywords, 10)
            .expect("keyword search must work");
        assert_eq!(hits[0].document.id, "platform_type:Структура");
        assert!(hits.iter().skip(1).any(|hit| {
            hit.document.kind == SearchDocumentKind::TypeProperty
                && hit.document.name.primary == "Структура"
        }));
    }

    #[test]
    fn keyword_search_keeps_task_oriented_query_table_ranking() {
        let path = temp_path("task-query-ranking.sqlite");
        let context = model::PlatformContext {
            query_tables: vec![
                query_table(
                    "РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии",
                    "Работа с запросами.Таблицы запросов.РегистрБухгалтерии.Таблица изменений",
                    "РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии",
                ),
                query_table(
                    "РегистрБухгалтерииОсновнаяТаблица",
                    "Работа с запросами.Таблицы запросов.РегистрБухгалтерии.Основная таблица",
                    "РегистрБухгалтерииОсновнаяТаблица",
                ),
            ],
            table_fields: vec![query_table_field(
                "РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии",
                "Работа с запросами.Таблицы запросов.РегистрБухгалтерии.Таблица изменений",
                "Регистратор",
            )],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");

        let hits = index
            .search("таблица регистра бухгалтерии", SearchMode::Keywords, 10)
            .expect("keyword search must work");
        assert_eq!(
            hits[0].document.id,
            "query_table:РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии"
        );
    }

    #[test]
    fn constructor_json_preserves_structured_parameters_after_sqlite_roundtrip() {
        let path = temp_path("http-constructor-json.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![platform_type(
                "HTTPСоединение",
                Some("HTTPConnection"),
                "HTTP connection.",
            )],
            constructors: vec![http_connection_constructor()],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let document_columns = table_columns(&index.connection, "documents");
        assert!(!document_columns.contains(&"signature_json".to_string()));
        assert!(!document_columns.contains(&"preview".to_string()));
        assert_eq!(
            index
                .connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM parameters p
                     JOIN type_refs r ON r.source_signature_id = p.signature_id
                      AND r.source_parameter_ordinal = p.ordinal
                     WHERE p.name = 'ИспользоватьАутентификациюОС'
                       AND r.ref_kind = 'parameter_type'
                       AND r.target_type_name = 'Булево'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("normalized parameter type ref query must work"),
            1
        );
        assert_eq!(
            index
                .connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM type_refs
                     WHERE source_document_id LIKE 'constructor:%HTTPСоединение%'
                       AND ref_kind = 'constructor_result'
                       AND target_type_id = 'platform_type:HTTPСоединение'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("constructor result type ref query must work"),
            1
        );
        let constructors = index
            .constructors_by_name("HTTPСоединение")
            .expect("constructor lookup must work");
        assert_eq!(constructors.len(), 1);
        assert_eq!(
            constructors[0].document.signatures[0].text,
            "Новый HTTPСоединение(<Сервер>, <Порт>, <ИспользоватьАутентификациюОС>)"
        );
        assert!(
            constructors[0]
                .document
                .parameter_terms
                .contains(&"ИспользоватьАутентификациюОС".to_string())
        );
        assert!(
            constructors[0]
                .document
                .parameter_terms
                .contains(&"Булево".to_string())
        );

        let signatures = &constructors[0].document.signatures;
        assert_eq!(signatures.len(), 1);
        assert_eq!(
            signatures[0].text,
            "Новый HTTPСоединение(<Сервер>, <Порт>, <ИспользоватьАутентификациюОС>)"
        );
        let os_auth = signatures[0]
            .parameters
            .iter()
            .find(|parameter| parameter.name == "ИспользоватьАутентификациюОС")
            .expect("OS authentication parameter must be present");
        assert!(!os_auth.required);
        assert!(os_auth.type_refs.iter().any(|value| value == "Булево"));
    }

    #[test]
    fn constructor_duplicate_ids_keep_last_document_with_warning() {
        let context = model::PlatformContext {
            platform_types: vec![platform_type(
                "МенеджерКриптографии",
                None,
                "Crypto manager.",
            )],
            constructors: vec![
                constructor_with_name(
                    "МенеджерКриптографии",
                    "Без инициализации модуля криптографии",
                    "Новый МенеджерКриптографии(<ИспользованиеИнтерактивногоРежима>)",
                ),
                constructor_with_name(
                    "МенеджерКриптографии",
                    "Для инициализации",
                    "Новый МенеджерКриптографии(<ИспользованиеИнтерактивногоРежима>)",
                ),
            ],
            ..model::PlatformContext::default()
        };

        let build = builder_from_context(&context)
            .into_documents("ru")
            .expect("same-signature constructor duplicates must not collide");
        assert_eq!(build.warnings.len(), 1);
        assert_eq!(build.warnings[0].code, "DUPLICATE_DOCUMENT_ID");
        assert!(build.warnings[0].message.contains("kept the last document"));
        let constructors = build
            .documents
            .iter()
            .filter(|document| document.kind == SearchDocumentKind::Constructor)
            .collect::<Vec<_>>();
        assert_eq!(constructors.len(), 1);
        assert_eq!(
            constructors[0].id,
            "constructor:platform_type:МенеджерКриптографии:Новый МенеджерКриптографии(<ИспользованиеИнтерактивногоРежима>)"
        );
        assert_eq!(
            constructors[0].description.as_deref(),
            Some("Для инициализации description")
        );
    }

    #[test]
    fn streaming_builder_preserves_expected_document_and_relation_shape() {
        let context = fixture_context();
        let builder_documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("fixture documents must not collide")
            .documents;
        let ids = builder_documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"platform_type:ОтборКомпоновкиДанных"));
        assert!(ids.contains(&"type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"));
        assert!(ids.contains(
            &"constructor:platform_type:ОтборКомпоновкиДанных:Новый ОтборКомпоновкиДанных()"
        ));
        let builder_relations = relations_from_documents(&builder_documents)
            .into_iter()
            .map(|relation| (relation.source_id, relation.target_id, relation.edge_kind))
            .collect::<Vec<_>>();
        assert!(builder_relations.iter().any(|(source, target, edge)| {
            source == "platform_type:ОтборКомпоновкиДанных"
                && target == "type_property:platform_type:ОтборКомпоновкиДанных:Элементы"
                && *edge == "owns"
        }));
        assert!(builder_relations.iter().any(|(source, target, edge)| {
            source == "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор"
                && target == "platform_type:ОтборКомпоновкиДанных"
                && *edge == "has_type"
        }));
    }

    #[test]
    fn streaming_builder_builds_sqlite_index_with_expected_queries_and_relations() {
        let path = temp_path("streaming-builder.sqlite");
        build_index_from_builder(&path, &metadata(), builder_from_context(&fixture_context()))
            .expect("streaming builder must build SQLite index");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        assert_eq!(
            index
                .metadata()
                .expect("index metadata must be readable")
                .source_extraction_schema_version,
            metadata().source_extraction_schema_version
        );
        let search_rows: usize = index
            .connection
            .query_row("SELECT COUNT(*) FROM document_search", [], |row| row.get(0))
            .expect("content rows must be stored");
        let fts_rows: usize = index
            .connection
            .query_row("SELECT COUNT(*) FROM document_fts", [], |row| row.get(0))
            .expect("fts rows must be rebuilt from content rows");
        assert_eq!(search_rows, fts_rows);

        let exact = index
            .get_by_name("DataCompositionFilter")
            .expect("exact lookup must work");
        assert_eq!(exact[0].document.name.primary, "ОтборКомпоновкиДанных");

        let related = index
            .related_by_id("platform_type:ОтборКомпоновкиДанных", 5, 20)
            .expect("id-root related search must work");
        assert!(related.iter().any(|hit| {
            hit.document.kind == SearchDocumentKind::Constructor
                && hit.document.name.primary == "Новый ОтборКомпоновкиДанных()"
        }));
        assert!(related.iter().any(|hit| {
            hit.document.kind == SearchDocumentKind::TypeProperty
                && hit.document.name.primary == "Элементы"
        }));
        assert_eq!(
            index
                .connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM members
                     WHERE owner_type_id = 'platform_type:ОтборКомпоновкиДанных'
                       AND member_kind = 'type_property'
                       AND name_primary = 'Элементы'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("normalized member query must work"),
            1
        );
        assert_eq!(
            index
                .connection
                .query_row(
                    "SELECT COUNT(*)
                     FROM type_refs
                     WHERE source_document_id = 'type_property:platform_type:НастройкиКомпоновкиДанных:Отбор'
                       AND ref_kind = 'property_type'
                       AND target_type_id = 'platform_type:ОтборКомпоновкиДанных'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("normalized property type ref query must work"),
            1
        );
    }

    #[test]
    fn sqlite_relation_rows_match_shared_relation_builder() {
        let path = temp_path("relation-builder-parity.sqlite");
        let documents = builder_from_context(&fixture_context())
            .into_documents("ru")
            .expect("fixture documents must not collide")
            .documents;
        let expected = relations_from_documents(&documents)
            .into_iter()
            .map(|relation| {
                (
                    relation.source_id,
                    relation.target_id,
                    relation.edge_kind.to_string(),
                    relation.label,
                    relation.evidence.to_string(),
                    relation.weight,
                )
            })
            .collect::<Vec<_>>();

        build_index_from_documents(&path, &metadata(), documents).expect("index must build");
        let connection = Connection::open(&path).expect("index must open for relation inspection");
        let mut statement = connection
            .prepare(
                "SELECT source_id, target_id, edge_kind, label, evidence, weight
                 FROM relations
                 ORDER BY weight, source_id, edge_kind, target_id",
            )
            .expect("relation query must prepare");
        let actual = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .expect("relation query must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("relation rows must deserialize");

        assert_eq!(actual, expected);
    }

    #[test]
    fn index_build_rejects_duplicate_document_ids_before_sqlite_write() {
        let path = temp_path("duplicate-document-id.sqlite");
        let documents = vec![
            document(
                SearchDocumentKind::GlobalMethod,
                None,
                &name("Сообщить", None),
                &[],
                &[],
                &[],
                Some("first source page"),
                "global_method:Сообщить".to_string(),
            ),
            document(
                SearchDocumentKind::GlobalMethod,
                None,
                &name("Сообщить", None),
                &[],
                &[],
                &[],
                Some("second source page"),
                "global_method:Сообщить".to_string(),
            ),
        ];

        let error = build_index_from_documents(&path, &metadata(), documents)
            .expect_err("duplicate document ids must reject index build");

        assert!(matches!(
            error,
            SearchError::DuplicateDocumentId {
                ref id,
                count: 2,
            } if id == "global_method:Сообщить"
        ));
        assert!(
            !path.exists(),
            "duplicate detection must run before SQLite index creation"
        );
    }

    #[test]
    fn streaming_builder_keeps_last_toc_marker_duplicate_with_warning() {
        let path = temp_path("builder-duplicate-document-id.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![platform_type_with_owner_path("ГруппаФормы", "Форма")],
            type_properties: vec![
                type_property_with_owner_path("ГруппаФормы", "Форма", "Видимость", "Булево"),
                type_property_with_owner_path(
                    "ГруппаФормы",
                    "Форма",
                    "Видимость#&^@^%&*^#1",
                    "Булево",
                ),
            ],
            ..model::PlatformContext::default()
        };

        let report = build_index_from_builder_with_report(
            &path,
            &metadata(),
            builder_from_context(&context),
        )
        .expect("TOC-marker duplicates must not reject index build");

        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].code, "DUPLICATE_DOCUMENT_ID");
        assert!(
            report.warnings[0]
                .message
                .contains("type_property:platform_type:ГруппаФормы:Видимость")
        );
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        assert_eq!(
            index
                .connection
                .query_row(
                    "SELECT COUNT(*) FROM documents WHERE id = 'type_property:platform_type:ГруппаФормы:Видимость'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("document count must be readable"),
            1
        );
    }

    #[test]
    fn read_only_open_rejects_stale_schema_version() {
        let path = temp_path("stale-schema.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");
        {
            let connection = Connection::open(&path).expect("index must open for fixture mutation");
            connection
                .execute(
                    "UPDATE meta SET value = '2' WHERE key = 'schema_version'",
                    [],
                )
                .expect("schema version fixture mutation must work");
        }

        let error = match SearchIndex::open_read_only(&path) {
            Ok(_) => panic!("stale index must be rejected"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            SearchError::UnsupportedSchemaVersion {
                expected: INDEX_SCHEMA_VERSION,
                ..
            }
        ));
        assert!(
            error.to_string().contains("rebuild the index"),
            "stale schema error should tell the user how to recover"
        );
    }

    #[test]
    fn read_only_open_reports_missing_schema_metadata() {
        let path = temp_path("missing-schema-metadata.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");
        {
            let connection = Connection::open(&path).expect("index must open for fixture mutation");
            connection
                .execute("DELETE FROM meta WHERE key = 'schema_version'", [])
                .expect("schema metadata deletion must work");
        }

        match SearchIndex::open_read_only(&path) {
            Err(SearchError::MissingMetadata { key, .. }) => assert_eq!(key, "schema_version"),
            Ok(_) => panic!("expected missing schema metadata error, got open index"),
            Err(error) => panic!("expected missing schema metadata error, got {error}"),
        }
    }

    #[test]
    fn metadata_reader_reports_missing_metadata_key() {
        let path = temp_path("missing-metadata.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");
        {
            let connection = Connection::open(&path).expect("index must open for fixture mutation");
            connection
                .execute("DELETE FROM meta WHERE key = 'source_hbk'", [])
                .expect("metadata fixture mutation must work");
        }
        let index = SearchIndex::open_read_only(&path).expect("schema-compatible index must open");

        match index.metadata() {
            Err(SearchError::MissingMetadata { key, .. }) => assert_eq!(key, "source_hbk"),
            other => panic!("expected missing metadata error, got {other:?}"),
        }
    }

    #[test]
    fn query_connections_are_read_only_and_repeatable() {
        let path = temp_path("readonly.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");
        let left = SearchIndex::open_read_only(&path).expect("first reader must open");
        let right = SearchIndex::open_read_only(&path).expect("second reader must open");
        assert_eq!(
            left.get_by_name("ОтборКомпоновкиДанных").unwrap(),
            right.get_by_name("ОтборКомпоновкиДанных").unwrap()
        );
        let write_result = left.connection.execute("DELETE FROM documents", []);
        assert!(
            write_result.is_err(),
            "read-only connection must reject writes"
        );
    }

    #[test]
    fn rebuild_replaces_previous_complete_index() {
        let path = temp_path("replace.sqlite");
        let mut context = fixture_context();
        build_test_index_from_context(&path, &metadata(), &context)
            .expect("first index must build");
        fs::write(path.with_extension("sqlite-wal"), b"stale wal")
            .expect("stale wal sidecar must be writable");
        fs::write(path.with_extension("sqlite-shm"), b"stale shm")
            .expect("stale shm sidecar must be writable");
        context.platform_types[0].description = Some("updated description".to_string());
        build_test_index_from_context(&path, &metadata(), &context)
            .expect("replacement index must build");
        assert!(!path.with_extension("sqlite-wal").exists());
        assert!(!path.with_extension("sqlite-shm").exists());
        let index = SearchIndex::open_read_only(&path).expect("index must open");
        let exact = index.get_by_name("ОтборКомпоновкиДанных").unwrap();
        assert_eq!(
            exact[0].document.description.as_deref(),
            Some("updated description")
        );
    }

    #[test]
    fn rebuild_cleans_stale_temporary_index_artifacts_before_creation() {
        let path = temp_path("stale-temp.sqlite");
        let temp_path = temp_index_path(&path);
        fs::write(&temp_path, b"not a sqlite database").expect("stale temp file must be writable");
        fs::write(temp_path.with_extension("sqlite-wal"), b"stale temp wal")
            .expect("stale temp wal must be writable");
        fs::write(temp_path.with_extension("sqlite-shm"), b"stale temp shm")
            .expect("stale temp shm must be writable");

        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index build must clean stale temp artifacts first");

        assert!(!temp_path.exists());
        assert!(!temp_path.with_extension("sqlite-wal").exists());
        assert!(!temp_path.with_extension("sqlite-shm").exists());
        assert!(SearchIndex::open_read_only(&path).is_ok());
    }

    #[test]
    fn concurrent_writers_are_serialized_by_lock() {
        let path = temp_path("writers.sqlite");
        let left_path = path.clone();
        let right_path = path.clone();
        let left = std::thread::spawn(move || {
            build_test_index_from_context(left_path, &metadata(), &fixture_context())
        });
        let right = std::thread::spawn(move || {
            build_test_index_from_context(right_path, &metadata(), &fixture_context())
        });
        left.join()
            .expect("left writer must not panic")
            .expect("left writer must build");
        right
            .join()
            .expect("right writer must not panic")
            .expect("right writer must build");
        let index = SearchIndex::open_read_only(&path).expect("final index must open");
        assert!(index.document_count().expect("document count must work") > 0);
    }

    #[test]
    fn query_table_identity_uses_identifier_and_semantic_variant_for_members() {
        let context = model::PlatformContext {
            query_tables: vec![
                query_table(
                    "ОстаткиИОбороты",
                    "Таблицы регистра накопления",
                    "Основная таблица",
                ),
                query_table(
                    "ОстаткиИОбороты",
                    "Таблицы регистра бухгалтерии (без поддержки корреспонденции)",
                    "Основная таблица",
                ),
            ],
            table_fields: vec![query_table_field(
                "Основная таблица",
                "Таблицы регистра бухгалтерии (без поддержки корреспонденции)",
                "Сумма",
            )],
            table_parameters: vec![query_table_parameter(
                "Основная таблица",
                "Таблицы регистра накопления",
                "Период",
            )],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("query table identities must not collide")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"query_table:ОстаткиИОбороты:Таблицы регистра накопления"));
        assert!(ids.contains(
            &"query_table:ОстаткиИОбороты:Таблицы регистра бухгалтерии (без поддержки корреспонденции)"
        ));
        assert!(ids.contains(
            &"query_table_field:query_table:ОстаткиИОбороты:Таблицы регистра бухгалтерии (без поддержки корреспонденции):Сумма"
        ));
        assert!(ids.contains(
            &"query_table_parameter:query_table:ОстаткиИОбороты:Таблицы регистра накопления:Период"
        ));

        let relations = relations_from_documents(&documents);
        assert!(relations.iter().any(|relation| {
            relation.source_id == "query_table:ОстаткиИОбороты:Таблицы регистра накопления"
                && relation.target_id
                    == "query_table_parameter:query_table:ОстаткиИОбороты:Таблицы регистра накопления:Период"
        }));
        assert!(relations.iter().any(|relation| {
            relation.source_id
                == "query_table:ОстаткиИОбороты:Таблицы регистра бухгалтерии (без поддержки корреспонденции)"
                && relation.target_id
                    == "query_table_field:query_table:ОстаткиИОбороты:Таблицы регистра бухгалтерии (без поддержки корреспонденции):Сумма"
        }));
    }

    #[test]
    fn query_table_member_identity_uses_toc_shaped_parent_table_identity() {
        let mut table = query_table("Задача", "", "Основная таблица");
        table.semantic = semantic_path(
            model::RecordFamily::QueryTable,
            &["Работа с запросами", "Таблицы запросов", "Таблицы задач"],
        );
        let mut field = query_table_field("Основная таблица", "", "<Имя общего реквизита>");
        field.semantic = semantic_path(
            model::RecordFamily::QueryTableField,
            &[
                "Работа с запросами",
                "Таблицы запросов",
                "Таблицы задач",
                "Основная таблица",
            ],
        );
        let context = model::PlatformContext {
            query_tables: vec![table],
            table_fields: vec![field],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("query table member identity must use parent table")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"query_table:Задача"));
        assert!(ids.contains(&"query_table_field:query_table:Задача:<Имя общего реквизита>"));
        assert!(
            !ids.contains(&"query_table_field:query_table:Основная таблица:<Имя общего реквизита>")
        );

        let relations = relations_from_documents(&documents);
        assert!(relations.iter().any(|relation| {
            relation.source_id == "query_table:Задача"
                && relation.target_id
                    == "query_table_field:query_table:Задача:<Имя общего реквизита>"
        }));
    }

    #[test]
    fn missing_query_table_parent_identity_rejects_member_indexing() {
        let mut builder = SearchIndexBuilder::new();
        let mut table = query_table("Задача", "Таблицы задач", "Основная таблица");
        table.identity = Some("query_table:Задача".to_string());
        builder.query_table(table).unwrap();

        let mut field = query_table_field("Основная таблица", "", "<Имя общего реквизита>");
        field.semantic = semantic_path(
            model::RecordFamily::QueryTableField,
            &[
                "Работа с запросами",
                "Таблицы запросов",
                "Таблицы задач",
                "Основная таблица",
            ],
        );
        builder.table_field(field).unwrap();

        let error = match builder.into_documents("ru") {
            Ok(_) => panic!("missing parent identity must reject query table members"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            SearchError::MissingParentIdentity {
                ref kind,
                ref name,
                ..
            } if kind == "query_table_field" && name == "<Имя общего реквизита>"
        ));
    }

    #[test]
    fn member_identity_prefers_precomputed_parent_identity() {
        let mut task_table = query_table("Задача", "Таблицы задач", "Основная таблица");
        task_table.identity = Some("query_table:precomputed-task-table".to_string());
        let mut task_field = query_table_field("Основная таблица", "", "Номер");
        task_field.semantic = semantic_path(
            model::RecordFamily::QueryTableField,
            &["Таблицы задач", "Основная таблица"],
        );
        task_field.owner_identity = Some("query_table:precomputed-task-table".to_string());

        let mut form_items = platform_type_with_owner_path("ЭлементыФормы", "Формы");
        form_items.identity = Some("platform_type:ЭлементыФормы:precomputed".to_string());
        let mut form_property = type_property_with_owner_path(
            "ЭлементыФормы",
            "Формы:ЭлементыФормы",
            "ТекущийЭлемент",
            "ПолеФормы",
        );
        form_property.owner_identity = Some("platform_type:ЭлементыФормы:precomputed".to_string());

        let context = model::PlatformContext {
            query_tables: vec![task_table],
            table_fields: vec![task_field],
            platform_types: vec![form_items],
            type_properties: vec![form_property],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("precomputed parent identities must index")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"query_table_field:query_table:precomputed-task-table:Номер"));
        assert!(
            ids.contains(&"type_property:platform_type:ЭлементыФормы:precomputed:ТекущийЭлемент")
        );
        assert!(!ids.contains(&"query_table_field:query_table:Задача:Номер"));
        assert!(!ids.contains(&"type_property:platform_type:ЭлементыФормы:ТекущийЭлемент"));
    }

    #[test]
    fn type_event_identity_uses_semantic_owner_path_not_event_group() {
        let context = model::PlatformContext {
            global_context_events: vec![
                type_event_with_owner_path(&["Форма", "Поле формы", "События"], "ОбработкаВыбора"),
                type_event_with_owner_path(
                    &["Форма", "Табличное поле формы", "События"],
                    "ОбработкаВыбора",
                ),
            ],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("type event identities must use semantic owner")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"type_event:platform_type:Форма.Поле формы:ОбработкаВыбора"));
        assert!(
            ids.contains(&"type_event:platform_type:Форма.Табличное поле формы:ОбработкаВыбора")
        );
        assert!(!ids.contains(&"type_event:platform_type:События:ОбработкаВыбора"));
    }

    #[test]
    fn missing_syntax_query_table_identity_uses_semantic_owner_path() {
        let mut task_table = query_table("", "Таблицы задач", "Основная таблица");
        task_table.syntax = None;
        task_table.identifier = None;
        task_table.table_role = model::QueryTableRole::Unknown;
        let context = model::PlatformContext {
            query_tables: vec![task_table],
            table_fields: vec![query_table_field(
                "Основная таблица",
                "Таблицы задач",
                "Наименование",
            )],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("missing-syntax query table identities must not collide")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(!ids.contains(&"query_table:"));
        assert!(ids.contains(&"query_table:Таблицы задач:Основная таблица"));
        assert!(ids.contains(
            &"query_table_field:query_table:Таблицы задач:Основная таблица:Наименование"
        ));
    }

    #[test]
    fn type_identity_keeps_semantic_variants_and_strips_toc_markers() {
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type_with_owner_path("ЭлементыФормы", "Форма"),
                platform_type_with_owner_path("ЭлементыФормы", "ФормаКлиентскогоПриложения"),
                platform_type_with_owner_path("ГруппаФормы", "Форма"),
            ],
            type_properties: vec![
                type_property_with_owner_path("ЭлементыФормы", "Форма", "ТекущийЭлемент", "Строка"),
                type_property_with_owner_path(
                    "ЭлементыФормы",
                    "ФормаКлиентскогоПриложения",
                    "ТекущийЭлемент",
                    "Строка",
                ),
                type_property_with_owner_path("ГруппаФормы", "Форма", "Видимость", "Булево"),
            ],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("semantic type variants must not collide")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"platform_type:ЭлементыФормы:Форма"));
        assert!(ids.contains(&"platform_type:ЭлементыФормы:ФормаКлиентскогоПриложения"));
        assert!(ids.contains(&"type_property:platform_type:ЭлементыФормы:Форма:ТекущийЭлемент"));
        assert!(ids.contains(
            &"type_property:platform_type:ЭлементыФормы:ФормаКлиентскогоПриложения:ТекущийЭлемент"
        ));
        assert!(!ids.iter().any(|id| id.contains("#&^@^%&*^#")));
    }

    #[test]
    fn normalized_type_refs_do_not_choose_hidden_winner_for_duplicate_type_names() {
        let path = temp_path("duplicate-type-ref.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type_with_owner_path("ЭлементыФормы", "Форма"),
                platform_type_with_owner_path("ЭлементыФормы", "ФормаКлиентскогоПриложения"),
                platform_type_with_owner_path("ГруппаФормы", "Форма"),
            ],
            type_properties: vec![type_property_with_owner_path(
                "ГруппаФормы",
                "Форма",
                "Элементы",
                "ЭлементыФормы",
            )],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let duplicate_count: i64 = index
            .connection
            .query_row(
                "SELECT COUNT(*) FROM type_identities WHERE name_primary = 'ЭлементыФормы'",
                [],
                |row| row.get(0),
            )
            .expect("duplicate type identity count must be readable");
        assert_eq!(duplicate_count, 2);
        let (target_type_id, status, candidates): (Option<String>, String, Option<String>) = index
            .connection
            .query_row(
                "SELECT target_type_id, target_resolution_status, target_candidate_type_ids
                 FROM type_refs
                 WHERE source_document_id = 'type_property:platform_type:ГруппаФормы:Элементы'
                   AND ref_kind = 'property_type'
                   AND target_type_name = 'ЭлементыФормы'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("ambiguous type ref row must exist");
        assert_eq!(target_type_id, None);
        assert_eq!(status, "ambiguous");
        assert_eq!(
            candidates
                .expect("ambiguous candidates must be stored")
                .lines()
                .collect::<Vec<_>>(),
            vec![
                "platform_type:ЭлементыФормы:Форма",
                "platform_type:ЭлементыФормы:ФормаКлиентскогоПриложения"
            ]
        );

        let related_type_refs = index
            .related_by_id_and_edge(
                "type_property:platform_type:ГруппаФормы:Элементы",
                "has_type",
                20,
            )
            .expect("edge-filtered type refs must query normalized rows");
        assert!(
            related_type_refs.is_empty(),
            "edge-filtered traversal must not choose a hidden duplicate type identity"
        );
        let related = index
            .related_by_id("type_property:platform_type:ГруппаФормы:Элементы", 1, 20)
            .expect("generic related traversal must query without hidden type winners");
        assert!(
            related
                .iter()
                .flat_map(|hit| hit.via.iter())
                .all(|edge| edge.edge_kind != "has_type"),
            "generic related traversal must not expose legacy has_type winners for ambiguous rows"
        );
    }

    #[test]
    fn exact_type_ref_spelling_disambiguates_whitespace_collisions() {
        let path = temp_path("exact-type-ref-spelling.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type("Владелец", None, "owner"),
                platform_type(
                    "Настройка сервиса",
                    Some("IServiceSetting"),
                    "COM service setting",
                ),
                platform_type(
                    "НастройкаСервиса",
                    Some("ServiceSetting"),
                    "server service setting",
                ),
            ],
            type_properties: vec![
                type_property("Владелец", "ComSetting", "Настройка сервиса"),
                type_property("Владелец", "ServerSetting", "НастройкаСервиса"),
            ],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let rows = index
            .connection
            .prepare(
                "SELECT source_document_id, target_type_name, target_type_id, target_resolution_status,
                        target_candidate_type_ids
                 FROM type_refs
                 WHERE source_document_id IN (
                    'type_property:platform_type:Владелец:ComSetting',
                    'type_property:platform_type:Владелец:ServerSetting'
                 )
                 ORDER BY source_document_id",
            )
            .expect("query must prepare")
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .expect("query must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("rows must decode");

        assert_eq!(
            rows,
            vec![
                (
                    "type_property:platform_type:Владелец:ComSetting".to_string(),
                    "Настройка сервиса".to_string(),
                    Some("platform_type:Настройка сервиса".to_string()),
                    "ok".to_string(),
                    None,
                ),
                (
                    "type_property:platform_type:Владелец:ServerSetting".to_string(),
                    "НастройкаСервиса".to_string(),
                    Some("platform_type:НастройкаСервиса".to_string()),
                    "ok".to_string(),
                    None,
                ),
            ]
        );
    }

    #[test]
    fn type_ref_resolution_combines_name_and_metadata_kind_candidates() {
        let path = temp_path("mixed-name-metadata-type-ref.sqlite");
        let mut template = platform_type(
            "ДубльТип.<Имя объекта>",
            Some("DuplicateType.<Object name>"),
            "metadata template",
        );
        template.type_kind = model::PlatformTypeKind::MetadataTemplate;
        template.metadata_kind = Some("ДубльТип".to_string());
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type("Владелец", None, "owner"),
                platform_type("ДубльТип", None, "regular type"),
                template,
            ],
            type_properties: vec![type_property("Владелец", "Поле", "ДубльТип")],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let (target_type_id, status, candidates): (Option<String>, String, Option<String>) = index
            .connection
            .query_row(
                "SELECT target_type_id, target_resolution_status, target_candidate_type_ids
                 FROM type_refs
                 WHERE source_document_id = 'type_property:platform_type:Владелец:Поле'
                   AND ref_kind = 'property_type'
                   AND target_type_name = 'ДубльТип'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("mixed type ref row must exist");

        assert_eq!(target_type_id, None);
        assert_eq!(status, "ambiguous");
        assert_eq!(
            candidates
                .expect("mixed candidates must be stored")
                .lines()
                .collect::<Vec<_>>(),
            vec![
                "platform_type:ДубльТип",
                "platform_type:ДубльТип.<Имя объекта>"
            ]
        );
    }

    #[test]
    fn enum_document_becomes_type_ref_target_without_platform_type_identity() {
        let path = temp_path("enum-type-ref-target.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![platform_type("Владелец", None, "owner")],
            enums: vec![enum_definition_with_alias(
                "ОбновлениеПредопределенныхДанных",
                "PredefinedDataUpdate",
                "objects/catalog2/predefined-data-update.html",
            )],
            type_properties: vec![type_property(
                "Владелец",
                "Обновление",
                "ОбновлениеПредопределенныхДанных",
            )],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let (target_type_id, status, candidates): (Option<String>, String, Option<String>) = index
            .connection
            .query_row(
                "SELECT target_type_id, target_resolution_status, target_candidate_type_ids
                 FROM type_refs
                 WHERE source_document_id = 'type_property:platform_type:Владелец:Обновление'
                   AND ref_kind = 'property_type'
                   AND target_type_name = 'ОбновлениеПредопределенныхДанных'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("enum type ref row must exist");

        assert_eq!(
            target_type_id,
            Some("enum:system:ОбновлениеПредопределенныхДанных".to_string())
        );
        assert_eq!(status, "ok");
        assert_eq!(candidates, None);

        let identities = index
            .type_identities_by_name("ОбновлениеПредопределенныхДанных")
            .expect("enum type identity lookup must work");
        assert_eq!(identities.len(), 1);
        assert_eq!(
            identities[0].document.id,
            "enum:system:ОбновлениеПредопределенныхДанных"
        );
        assert_eq!(identities[0].document.kind, SearchDocumentKind::Enum);

        let platform_like_count: i64 = index
            .connection
            .query_row(
                "SELECT COUNT(*) FROM type_identities WHERE type_id = 'platform_type:ОбновлениеПредопределенныхДанных'",
                [],
                |row| row.get(0),
            )
            .expect("type identity count must be readable");
        assert_eq!(platform_like_count, 0);

        let related_type_refs = index
            .related_by_id_and_edge(
                "type_property:platform_type:Владелец:Обновление",
                "has_type",
                20,
            )
            .expect("edge-filtered enum type ref must query normalized rows");
        assert_eq!(related_type_refs.len(), 1);
        assert_eq!(
            related_type_refs[0].document.id,
            "enum:system:ОбновлениеПредопределенныхДанных"
        );

        let unresolved_unique_enum_matches: i64 = index
            .connection
            .query_row(
                "SELECT COUNT(*)
                 FROM type_refs r
                 WHERE r.target_resolution_status = 'unresolved'
                   AND (
                     SELECT COUNT(*)
                     FROM documents d
                     WHERE d.kind = 'enum'
                       AND (d.name_primary = r.target_type_name OR d.name_alias = r.target_type_name)
                   ) = 1",
                [],
                |row| row.get(0),
            )
            .expect("unique enum unresolved inventory must be readable");
        assert_eq!(unresolved_unique_enum_matches, 0);
    }

    #[test]
    fn duplicate_enum_type_ref_targets_remain_ambiguous() {
        let path = temp_path("duplicate-enum-type-ref-target.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![platform_type("Владелец", None, "owner")],
            enums: vec![
                enum_definition_with_alias(
                    "ИспользованиеТекущейСтроки",
                    "SelectedRowsUse",
                    "objects/catalog2/selected-rows-use.html",
                ),
                enum_definition_with_alias(
                    "ИспользованиеТекущейСтроки",
                    "CurrentRowUse",
                    "objects/catalog2/current-row-use.html",
                ),
            ],
            type_properties: vec![type_property(
                "Владелец",
                "Использование",
                "ИспользованиеТекущейСтроки",
            )],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let (target_type_id, status, candidates): (Option<String>, String, Option<String>) = index
            .connection
            .query_row(
                "SELECT target_type_id, target_resolution_status, target_candidate_type_ids
                 FROM type_refs
                 WHERE source_document_id = 'type_property:platform_type:Владелец:Использование'
                   AND ref_kind = 'property_type'
                   AND target_type_name = 'ИспользованиеТекущейСтроки'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("duplicate enum type ref row must exist");

        assert_eq!(target_type_id, None);
        assert_eq!(status, "ambiguous");
        assert_eq!(
            candidates
                .expect("ambiguous enum candidates must be stored")
                .lines()
                .collect::<Vec<_>>(),
            vec![
                "enum:system:ИспользованиеТекущейСтроки:CurrentRowUse",
                "enum:system:ИспользованиеТекущейСтроки:SelectedRowsUse",
            ]
        );
    }

    #[test]
    fn type_reference_gap_report_classifies_rows_without_hidden_winners() {
        let path = temp_path("type-reference-gap-report.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type("Владелец", None, "owner"),
                platform_type("РазрешенныйТип", None, "resolved"),
                platform_type_with_owner_path("ДубльТип", "Первый владелец"),
                platform_type_with_owner_path("ДубльТип", "Второй владелец"),
            ],
            type_properties: vec![
                type_property("Владелец", "Разрешенное", "РазрешенныйТип"),
                type_property("Владелец", "Неизвестное", "НесуществующийТип"),
                type_property("Владелец", "Дублирующее", "ДубльТип"),
            ],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let writable = Connection::open(&path).expect("test index must open writable");
        writable
            .execute(
                "UPDATE type_refs
                 SET template_binding_kind = 'owner_parameter'
                 WHERE source_document_id = 'type_property:platform_type:Владелец:Разрешенное'",
                [],
            )
            .expect("test fixture binding flag must update");
        drop(writable);

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let report = index
            .type_reference_gap_report(10)
            .expect("gap report must be readable");

        assert_eq!(report.total, 3);
        assert_eq!(report.resolved, 1);
        assert_eq!(report.unresolved, 1);
        assert_eq!(report.ambiguous, 1);
        assert_eq!(report.template_bindings, 1);
        assert_eq!(report.roles.len(), 1);
        assert_eq!(report.roles[0].role, "property_type");
        assert_eq!(report.roles[0].template_bindings, 1);
        assert_eq!(
            report.top_unresolved[0].target_type_name,
            "НесуществующийТип"
        );
        assert_eq!(
            report.top_unresolved[0].examples[0].source_document_id,
            "type_property:platform_type:Владелец:Неизвестное"
        );
        assert_eq!(report.top_ambiguous[0].target_type_name, "ДубльТип");
        assert_eq!(
            report.top_ambiguous[0].candidate_type_ids,
            vec![
                "platform_type:ДубльТип:Второй владелец".to_string(),
                "platform_type:ДубльТип:Первый владелец".to_string(),
            ]
        );
    }

    #[test]
    fn type_identity_lookup_returns_all_same_name_variants_deterministically() {
        let path = temp_path("duplicate-type-lookup.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type_with_owner_path("ЭлементыФормы", "Форма"),
                platform_type_with_owner_path("ЭлементыФормы", "ФормаКлиентскогоПриложения"),
            ],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let hits = index
            .type_identities_by_name("ЭлементыФормы")
            .expect("type identity lookup must work");
        let ids = hits
            .iter()
            .map(|hit| hit.document.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            [
                "platform_type:ЭлементыФормы:Форма",
                "platform_type:ЭлементыФормы:ФормаКлиентскогоПриложения",
            ]
        );
    }

    #[test]
    fn type_identity_lookup_uses_indexed_sql_plan() {
        let path = temp_path("type-lookup-query-plan.sqlite");
        let context = model::PlatformContext {
            platform_types: vec![
                platform_type_with_owner_path("ЭлементыФормы", "Форма"),
                platform_type_with_owner_path("ЭлементыФормы", "ФормаКлиентскогоПриложения"),
            ],
            ..model::PlatformContext::default()
        };
        build_test_index_from_context(&path, &metadata(), &context).expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let mut statement = index
            .connection
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary,
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since
                 FROM document_names n
                 JOIN type_identities t ON t.document_id = n.document_id
                 JOIN documents d ON d.id = t.document_id
                 WHERE n.key = ?1
                   AND n.key_kind = ?2
                 ORDER BY d.kind_priority, d.id",
            )
            .expect("query plan must prepare");
        let plan = statement
            .query_map(
                [normalize_lookup_key("ЭлементыФормы"), "primary".to_string()],
                |row| row.get::<_, String>(3),
            )
            .expect("query plan must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("query plan rows must be readable");

        assert!(
            plan.iter()
                .any(|detail| detail.contains("document_names_key_idx"))
        );
        assert!(
            plan.iter()
                .any(|detail| detail.contains("type_identities_document_idx"))
        );
        assert!(
            !plan.iter().any(|detail| detail == "SCAN t"),
            "type identity lookup must not scan all type identities: {plan:?}"
        );
    }

    #[test]
    fn owner_type_exact_lookups_use_indexed_sql_plan() {
        let path = temp_path("owner-type-lookup-query-plan.sqlite");
        build_test_index_from_context(&path, &metadata(), &fixture_context())
            .expect("index must build");

        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");
        let mut member_statement = index
            .connection
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary,
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since
                 FROM document_names n
                 JOIN members m INDEXED BY members_document_owner_idx
                   ON m.document_id = n.document_id
                  AND m.owner_type_id = ?1
                 JOIN documents d ON d.id = m.document_id
                 WHERE n.key = ?2
                   AND n.key_kind IN ('primary', 'alias')
                 ORDER BY d.kind_priority, m.name_primary, d.id",
            )
            .expect("member query plan must prepare");
        let member_plan = member_statement
            .query_map(
                params![
                    "platform_type:НастройкиКомпоновкиДанных",
                    normalize_lookup_key("Отбор")
                ],
                |row| row.get::<_, String>(3),
            )
            .expect("member query plan must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("member query plan rows must be readable");
        assert!(
            member_plan
                .iter()
                .any(|detail| detail.contains("document_names_key_idx"))
        );
        assert!(
            member_plan
                .iter()
                .any(|detail| detail.contains("members_document_owner_idx")),
            "member lookup must use document/owner index: {member_plan:?}"
        );
        assert!(
            !member_plan.iter().any(|detail| detail == "SCAN m"),
            "member lookup must not scan all member rows: {member_plan:?}"
        );

        let mut callable_statement = index
            .connection
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary,
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since
                 FROM document_names n
                 JOIN callables c INDEXED BY callables_document_owner_idx
                   ON c.document_id = n.document_id
                  AND c.owner_type_id = ?1
                 JOIN documents d ON d.id = c.document_id
                 WHERE n.key = ?2
                   AND n.key_kind IN ('primary', 'alias')
                 ORDER BY d.kind_priority, d.name_primary, d.id",
            )
            .expect("callable query plan must prepare");
        let callable_plan = callable_statement
            .query_map(
                params![
                    "platform_type:КоллекцияЭлементовОтбораКомпоновкиДанных",
                    normalize_lookup_key("Добавить")
                ],
                |row| row.get::<_, String>(3),
            )
            .expect("callable query plan must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("callable query plan rows must be readable");
        assert!(
            callable_plan
                .iter()
                .any(|detail| detail.contains("document_names_key_idx"))
        );
        assert!(
            callable_plan
                .iter()
                .any(|detail| detail.contains("callables_document_owner_idx")),
            "callable lookup must use document/owner index: {callable_plan:?}"
        );
        assert!(
            !callable_plan.iter().any(|detail| detail == "SCAN c"),
            "callable lookup must not scan all callable rows: {callable_plan:?}"
        );
    }

    #[test]
    fn enum_identity_distinguishes_metadata_property_enums() {
        let mut enum_value = enum_value("Видимость", "Использовать");
        enum_value.owner_identity = Some("enum:system:Видимость".to_string());
        let context = model::PlatformContext {
            enums: vec![
                enum_definition("Видимость", "objects/catalog2/catalog999/Visible.html"),
                enum_definition(
                    "Видимость",
                    "objects/catalog1649/Form/properties/Visible.html",
                ),
            ],
            enum_values: vec![enum_value],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("enum semantic variants must not collide")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"enum:system:Видимость"));
        assert!(ids.contains(&"enum:metadata_property:Видимость"));
        assert_eq!(
            ids.iter()
                .filter(|id| **id == "enum_value:enum:system:Видимость:Использовать")
                .count(),
            1
        );
    }

    #[test]
    fn enum_identity_uses_alias_variant_for_duplicate_system_enum_names() {
        let context = model::PlatformContext {
            enums: vec![
                enum_definition_with_alias(
                    "ИспользованиеТекущейСтроки",
                    "SelectedRowsUse",
                    "objects/catalog2/catalog111/SelectedRowsUse.html",
                ),
                enum_definition_with_alias(
                    "ИспользованиеТекущейСтроки",
                    "CurrentRowUse",
                    "objects/catalog2/catalog222/CurrentRowUse.html",
                ),
            ],
            enum_values: vec![
                enum_value_with_owner_alias(
                    "ИспользованиеТекущейСтроки",
                    "SelectedRowsUse",
                    "Авто",
                ),
                enum_value_with_owner_alias("ИспользованиеТекущейСтроки", "CurrentRowUse", "Авто"),
            ],
            ..model::PlatformContext::default()
        };

        let documents = builder_from_context(&context)
            .into_documents("ru")
            .expect("alias-backed enum variants must not collide")
            .documents;
        let ids = documents
            .iter()
            .map(|document| document.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"enum:system:ИспользованиеТекущейСтроки:SelectedRowsUse"));
        assert!(ids.contains(&"enum:system:ИспользованиеТекущейСтроки:CurrentRowUse"));
        assert!(
            ids.contains(&"enum_value:enum:system:ИспользованиеТекущейСтроки:SelectedRowsUse:Авто")
        );
        assert!(
            ids.contains(&"enum_value:enum:system:ИспользованиеТекущейСтроки:CurrentRowUse:Авто")
        );
    }

    fn metadata() -> IndexMetadata {
        IndexMetadata {
            locale: "ru".to_string(),
            source_locale: "ru".to_string(),
            source_hbk: "fixture.hbk".to_string(),
            source_extraction_schema_version: 11,
        }
    }

    fn language_fixture_facts(locale: &str) -> Vec<language::LanguageFact> {
        let suffix = match locale {
            "root" => "root",
            _ => "ru",
        };
        let mut fixtures = vec![
            (
                LanguageSourceFamily::Shlang,
                "def_String",
                format!("shlang_def_string_{suffix}.html"),
            ),
            (
                LanguageSourceFamily::Shlang,
                "def_Func",
                format!("shlang_def_func_{suffix}.html"),
            ),
            (
                LanguageSourceFamily::Shquery,
                "SELECTStatement",
                format!("shquery_select_statement_{suffix}.html"),
            ),
            (
                LanguageSourceFamily::Shquery,
                "SUM",
                format!("shquery_sum_{suffix}.html"),
            ),
            (
                LanguageSourceFamily::Shquery,
                "STRING",
                format!("shquery_string_{suffix}.html"),
            ),
            (
                LanguageSourceFamily::Dcsui,
                "SKD_Functions_Strings",
                format!("dcsui_functions_strings_{suffix}.html"),
            ),
        ];
        if locale == "ru" {
            fixtures.push((
                LanguageSourceFamily::Shquery,
                "LitString",
                "shquery_lit_string_ru.html".to_string(),
            ));
        }
        fixtures
            .into_iter()
            .flat_map(|(source_family, html_path, fixture_name)| {
                let html = std::fs::read_to_string(
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../../tests/fixtures/syntax-helper-language")
                        .join(&fixture_name),
                )
                .expect("language fixture must be readable");
                extract_language_facts(LanguagePageInput {
                    source_hbk: "fixture.hbk",
                    source_family,
                    locale,
                    html_path,
                    html: &html,
                })
            })
            .collect()
    }

    fn language_fact(
        id: &str,
        source_family: LanguageSourceFamily,
        domain: language::LanguageDomain,
        family: language::LanguageFactFamily,
        name: model::LocalizedName,
    ) -> language::LanguageFact {
        language::LanguageFact {
            id: id.to_string(),
            source_family,
            domain,
            family,
            name,
            syntax: None,
            signatures: Vec::new(),
            type_refs: Vec::new(),
            return_types: Vec::new(),
            description: None,
            provenance: language::LanguageFactProvenance {
                source_hbk: "fixture.hbk".to_string(),
                locale: "ru".to_string(),
                html_path: id.to_string(),
                page_title: id.to_string(),
                anchor: None,
            },
        }
    }

    fn fixture_context() -> model::PlatformContext {
        model::PlatformContext {
            platform_types: vec![
                platform_type(
                    "ОтборКомпоновкиДанных",
                    Some("DataCompositionFilter"),
                    "Объект системы компоновки данных для настройки отбора.",
                ),
                platform_type(
                    "НастройкиКомпоновкиДанных",
                    Some("DataCompositionSettings"),
                    "Настройки системы компоновки данных.",
                ),
                platform_type(
                    "КоллекцияЭлементовОтбораКомпоновкиДанных",
                    Some("DataCompositionFilterItems"),
                    "Коллекция элементов отбора.",
                ),
                platform_type(
                    "ЭлементОтбораКомпоновкиДанных",
                    Some("DataCompositionFilterItem"),
                    "Элемент отбора.",
                ),
                platform_type(
                    "БиблиотекаКартинок",
                    Some("PictureLib"),
                    "Библиотека картинок.",
                ),
            ],
            type_properties: vec![
                type_property("БиблиотекаКартинок", "ОтборКомпоновкиДанных", "Картинка"),
                type_property(
                    "НастройкиКомпоновкиДанных",
                    "Отбор",
                    "ОтборКомпоновкиДанных",
                ),
                type_property(
                    "ОтборКомпоновкиДанных",
                    "Элементы",
                    "КоллекцияЭлементовОтбораКомпоновкиДанных",
                ),
                type_property(
                    "ЭлементОтбораКомпоновкиДанных",
                    "ЛевоеЗначение",
                    "Произвольный",
                ),
            ],
            type_methods: vec![
                type_method(
                    "ОтборКомпоновкиДанных",
                    "ПолучитьОбъектПоИдентификатору",
                    "ЭлементОтбораКомпоновкиДанных",
                ),
                type_method(
                    "КоллекцияЭлементовОтбораКомпоновкиДанных",
                    "Добавить",
                    "ЭлементОтбораКомпоновкиДанных",
                ),
            ],
            constructors: vec![constructor(
                "ОтборКомпоновкиДанных",
                "Новый ОтборКомпоновкиДанных()",
            )],
            global_context_events: vec![type_event("ОтборКомпоновкиДанных", "ПередЗаписью")],
            ..model::PlatformContext::default()
        }
    }

    fn builder_from_context(context: &model::PlatformContext) -> SearchIndexBuilder {
        let context = context_with_test_owner_identities(context);
        let mut builder = SearchIndexBuilder::new();
        for record in context.global_contexts.iter().cloned() {
            builder.global_context(record).unwrap();
        }
        for record in context.global_methods.iter().cloned() {
            builder.global_method(record).unwrap();
        }
        for record in context.global_properties.iter().cloned() {
            builder.global_property(record).unwrap();
        }
        for record in context.global_context_events.iter().cloned() {
            builder.global_context_event(record).unwrap();
        }
        for record in context.platform_types.iter().cloned() {
            builder.platform_type(record).unwrap();
        }
        for record in context.query_tables.iter().cloned() {
            builder.query_table(record).unwrap();
        }
        for record in context.type_methods.iter().cloned() {
            builder.type_method(record).unwrap();
        }
        for record in context.type_properties.iter().cloned() {
            builder.type_property(record).unwrap();
        }
        for record in context.table_fields.iter().cloned() {
            builder.table_field(record).unwrap();
        }
        for record in context.table_parameters.iter().cloned() {
            builder.table_parameter(record).unwrap();
        }
        for record in context.constructors.iter().cloned() {
            builder.constructor(record).unwrap();
        }
        for record in context.enums.iter().cloned() {
            builder.enum_definition(record).unwrap();
        }
        for record in context.enum_values.iter().cloned() {
            builder.enum_value(record).unwrap();
        }
        for record in context.diagnostics.iter().cloned() {
            builder.diagnostic(record).unwrap();
        }
        builder
    }

    fn context_with_test_owner_identities(
        context: &model::PlatformContext,
    ) -> model::PlatformContext {
        let mut context = context.clone();
        let identities = DocumentIdentities::from_inputs(
            &context
                .platform_types
                .iter()
                .map(|record| PlatformTypeIdentityInput {
                    identity: record.identity.clone(),
                    name_primary: record.name.primary.clone(),
                    semantic: record.semantic.clone(),
                })
                .collect::<Vec<_>>(),
            &context
                .query_tables
                .iter()
                .map(|record| QueryTableIdentityInput {
                    identity: record.identity.clone(),
                    name_primary: record.name.clone(),
                    identifier: record.identifier.clone(),
                    semantic: record.semantic.clone(),
                })
                .collect::<Vec<_>>(),
            &context
                .enums
                .iter()
                .map(|record| EnumIdentityInput {
                    identity: record.identity.clone(),
                    name_primary: record.name.primary.clone(),
                    name_alias: record.name.alias.clone(),
                    source_html_path: record.source.html_path.clone(),
                })
                .collect::<Vec<_>>(),
        );
        for record in &mut context.type_methods {
            if record.owner_identity.is_none() {
                record.owner_identity = identities
                    .platform_type_ids
                    .get(&model::platform_type_semantic_key(
                        &record.owner.primary,
                        &record.semantic,
                    ))
                    .cloned();
            }
        }
        for record in &mut context.type_properties {
            if record.owner_identity.is_none() {
                record.owner_identity = identities
                    .platform_type_ids
                    .get(&model::platform_type_semantic_key(
                        &record.owner.primary,
                        &record.semantic,
                    ))
                    .cloned();
            }
        }
        for record in &mut context.constructors {
            if record.owner_identity.is_none() {
                record.owner_identity = identities
                    .platform_type_ids
                    .get(&model::platform_type_semantic_key(
                        &record.owner.primary,
                        &record.semantic,
                    ))
                    .cloned();
            }
        }
        for record in &mut context.table_fields {
            if record.owner_identity.is_none() {
                record.owner_identity = identities
                    .query_table_ids
                    .get(&model::query_table_semantic_key(
                        &record.semantic,
                        &record.owner.primary,
                    ))
                    .cloned();
            }
        }
        for record in &mut context.table_parameters {
            if record.owner_identity.is_none() {
                record.owner_identity = identities
                    .query_table_ids
                    .get(&model::query_table_semantic_key(
                        &record.semantic,
                        &record.owner.primary,
                    ))
                    .cloned();
            }
        }
        for record in &mut context.enum_values {
            if record.owner_identity.is_none() {
                record.owner_identity =
                    unique_enum_owner_identity(&identities, &context.enums, record);
            }
        }
        for record in &mut context.global_context_events {
            if record.owner_identity.is_none() {
                record.owner_identity = model::type_event_owner_semantic_key(&record.semantic)
                    .and_then(|key| identities.platform_type_ids.get(&key).cloned());
            }
        }
        context
    }

    fn unique_enum_owner_identity(
        identities: &DocumentIdentities,
        enums: &[model::EnumDefinition],
        enum_value: &model::EnumValue,
    ) -> Option<String> {
        let mut matches = enums.iter().filter_map(|enum_definition| {
            enum_definition
                .name
                .matches(
                    enum_value
                        .owner
                        .alias
                        .as_deref()
                        .unwrap_or(&enum_value.owner.primary),
                )
                .then(|| {
                    identities
                        .enum_ids
                        .get(&enum_record_key(
                            &enum_definition.name.primary,
                            &enum_definition.source.html_path,
                        ))
                        .cloned()
                })
                .flatten()
        });
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
    }

    fn build_test_index_from_context(
        path: impl AsRef<Path>,
        metadata: &IndexMetadata,
        context: &model::PlatformContext,
    ) -> Result<(), SearchError> {
        build_index_from_builder(path, metadata, builder_from_context(context))
    }

    fn platform_type(primary: &str, alias: Option<&str>, description: &str) -> model::PlatformType {
        model::PlatformType {
            identity: None,
            name: name(primary, alias),
            semantic: model::SemanticContext::default(),
            type_kind: model::PlatformTypeKind::Regular,
            object_kind: Some(model::PlatformObjectKind::RegularPlatformType),
            extends: Vec::new(),
            metadata_kind: None,
            template_parameters: Vec::new(),
            type_template_key: None,
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: Some(description.to_string()),
            facts: model::SectionFacts::default(),
            source: source(primary),
        }
    }

    fn platform_type_with_owner_path(primary: &str, owner: &str) -> model::PlatformType {
        let mut record = platform_type(primary, None, "type description");
        record.semantic = semantic(model::RecordFamily::PlatformType, owner);
        record
    }

    fn type_property(owner: &str, primary: &str, type_ref: &str) -> model::PlatformProperty {
        model::PlatformProperty {
            owner: name(owner, None),
            owner_identity: None,
            name: name(primary, None),
            semantic: model::SemanticContext::default(),
            usage: None,
            type_refs: vec![model::TypeRef {
                name: type_ref.to_string(),
            }],
            description: Some(format!("{primary} description")),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn type_property_with_owner_path(
        owner: &str,
        owner_path: &str,
        primary: &str,
        type_ref: &str,
    ) -> model::PlatformProperty {
        let mut record = type_property(owner, primary, type_ref);
        record.semantic = semantic(model::RecordFamily::TypeProperty, owner_path);
        record
    }

    fn query_table(identifier: &str, owner_path: &str, table_name: &str) -> model::QueryTable {
        model::QueryTable {
            identity: None,
            name: table_name.to_string(),
            syntax: None,
            identifier: (!identifier.is_empty()).then(|| identifier.to_string()),
            semantic: semantic(model::RecordFamily::QueryTable, owner_path),
            table_role: model::QueryTableRole::Primary,
            description: Some("table description".to_string()),
            source: source(table_name),
        }
    }

    fn query_table_field(owner: &str, owner_path: &str, primary: &str) -> model::QueryTableField {
        model::QueryTableField {
            owner: name(owner, None),
            owner_identity: None,
            name: primary.to_string(),
            semantic: semantic(model::RecordFamily::QueryTableField, owner_path),
            type_refs: Vec::new(),
            description: Some("field description".to_string()),
            note: None,
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn query_table_parameter(
        owner: &str,
        owner_path: &str,
        primary: &str,
    ) -> model::QueryTableParameter {
        model::QueryTableParameter {
            owner: name(owner, None),
            owner_identity: None,
            name: primary.to_string(),
            semantic: semantic(model::RecordFamily::QueryTableParameter, owner_path),
            type_refs: Vec::new(),
            description: Some("parameter description".to_string()),
            default_value: None,
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn enum_definition(primary: &str, html_path: &str) -> model::EnumDefinition {
        enum_definition_with_alias(primary, "", html_path)
    }

    fn enum_definition_with_alias(
        primary: &str,
        alias: &str,
        html_path: &str,
    ) -> model::EnumDefinition {
        model::EnumDefinition {
            identity: None,
            name: name(primary, (!alias.is_empty()).then_some(alias)),
            value_links: Vec::new(),
            description: Some("enum description".to_string()),
            facts: model::SectionFacts::default(),
            source: source_with_html_path(primary, html_path),
        }
    }

    fn enum_value(owner: &str, primary: &str) -> model::EnumValue {
        enum_value_with_owner_alias(owner, "", primary)
    }

    fn enum_value_with_owner_alias(
        owner: &str,
        owner_alias: &str,
        primary: &str,
    ) -> model::EnumValue {
        model::EnumValue {
            owner: name(owner, (!owner_alias.is_empty()).then_some(owner_alias)),
            owner_identity: None,
            name: name(primary, None),
            description: Some("value description".to_string()),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn semantic(record_family: model::RecordFamily, owner_path: &str) -> model::SemanticContext {
        semantic_path(record_family, &[owner_path])
    }

    fn semantic_path(
        record_family: model::RecordFamily,
        owner_path: &[&str],
    ) -> model::SemanticContext {
        model::SemanticContext::new(model::BranchKind::PlatformObjects, record_family)
            .with_owner_path(owner_path.iter().map(|value| name(value, None)).collect())
    }

    fn type_method(owner: &str, primary: &str, return_type: &str) -> model::PlatformMethod {
        model::PlatformMethod {
            owner: name(owner, None),
            owner_identity: None,
            name: name(primary, None),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: format!("{primary}()"),
                parameters: Vec::new(),
                return_types: Vec::new(),
                variant: None,
            }],
            return_types: vec![model::TypeRef {
                name: return_type.to_string(),
            }],
            description: Some(format!("{primary} description")),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn constructor(owner: &str, signature: &str) -> model::Constructor {
        constructor_with_name(owner, signature, signature)
    }

    fn constructor_with_name(owner: &str, primary: &str, signature: &str) -> model::Constructor {
        model::Constructor {
            owner: name(owner, None),
            owner_identity: None,
            name: name(primary, None),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: signature.to_string(),
                parameters: Vec::new(),
                return_types: Vec::new(),
                variant: None,
            }],
            description: Some(format!("{primary} description")),
            facts: model::SectionFacts::default(),
            source: source(signature),
        }
    }

    fn http_connection_constructor() -> model::Constructor {
        model::Constructor {
            owner: name("HTTPСоединение", Some("HTTPConnection")),
            owner_identity: None,
            name: name("По параметрам соединения", None),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: "Новый HTTPСоединение(<Сервер>, <Порт>, <ИспользоватьАутентификациюОС>)"
                    .to_string(),
                parameters: vec![
                    model::Parameter {
                        name: "Сервер".to_string(),
                        required: true,
                        type_refs: vec![model::TypeRef {
                            name: "Строка".to_string(),
                        }],
                        description: Some("Имя сервера.".to_string()),
                    },
                    model::Parameter {
                        name: "Порт".to_string(),
                        required: false,
                        type_refs: vec![model::TypeRef {
                            name: "Число".to_string(),
                        }],
                        description: Some("Порт соединения.".to_string()),
                    },
                    model::Parameter {
                        name: "ИспользоватьАутентификациюОС".to_string(),
                        required: false,
                        type_refs: vec![model::TypeRef {
                            name: "Булево".to_string(),
                        }],
                        description: Some(
                            "Использовать аутентификацию операционной системы.".to_string(),
                        ),
                    },
                ],
                return_types: Vec::new(),
                variant: None,
            }],
            description: None,
            facts: model::SectionFacts::default(),
            source: source("HTTPСоединение"),
        }
    }

    fn type_event(owner: &str, primary: &str) -> model::GlobalContextEvent {
        type_event_with_owner_path(&[owner], primary)
    }

    fn module_event(
        kind: model::ModuleKind,
        owner_path: &[&str],
        primary: &str,
    ) -> model::GlobalContextEvent {
        model::GlobalContextEvent {
            name: name(primary, Some("OnOpen")),
            owner_identity: None,
            semantic: model::SemanticContext::new(
                model::BranchKind::ManagedForms,
                model::RecordFamily::ModuleEvent,
            )
            .with_owner_path(
                owner_path
                    .iter()
                    .map(|owner| name(owner, Some("Form")).clone())
                    .collect(),
            ),
            module: model::ModuleEventContext {
                kind,
                owner_path: owner_path
                    .iter()
                    .map(|owner| {
                        if *owner == "Form" {
                            name("Форма", Some("Form"))
                        } else {
                            name(owner, None)
                        }
                    })
                    .collect(),
            },
            signatures: vec![model::Signature {
                text: format!("{primary}()"),
                parameters: Vec::new(),
                return_types: Vec::new(),
                variant: None,
            }],
            description: Some("module event description".to_string()),
            facts: model::SectionFacts::default(),
            source: source(&format!("module-{primary}")),
        }
    }

    fn type_event_with_owner_path(owner_path: &[&str], primary: &str) -> model::GlobalContextEvent {
        model::GlobalContextEvent {
            name: name(primary, Some("BeforeWrite")),
            owner_identity: type_event_test_owner_identity(owner_path),
            semantic: model::SemanticContext::new(
                model::BranchKind::PlatformObjects,
                model::RecordFamily::TypeEvent,
            )
            .with_owner_path(owner_path.iter().map(|owner| name(owner, None)).collect()),
            module: model::ModuleEventContext::default(),
            signatures: vec![model::Signature {
                text: format!("{primary}()"),
                parameters: Vec::new(),
                return_types: Vec::new(),
                variant: None,
            }],
            description: Some("event description".to_string()),
            facts: model::SectionFacts::default(),
            source: source(&format!("{}.{}", owner_path.join("."), primary)),
        }
    }

    fn type_event_test_owner_identity(owner_path: &[&str]) -> Option<String> {
        let mut owner_path = owner_path;
        if owner_path
            .last()
            .is_some_and(|name| matches!(name.trim().to_lowercase().as_str(), "события" | "events"))
        {
            owner_path = &owner_path[..owner_path.len() - 1];
        }
        (!owner_path.is_empty()).then(|| format!("platform_type:{}", owner_path.join(".")))
    }

    fn name(primary: &str, alias: Option<&str>) -> model::LocalizedName {
        model::LocalizedName {
            primary: primary.to_string(),
            alias: alias.map(ToOwned::to_owned),
        }
    }

    fn source(name: &str) -> model::SyntaxHelperSource {
        source_with_html_path(name, &format!("{name}.html"))
    }

    fn source_with_html_path(name: &str, html_path: &str) -> model::SyntaxHelperSource {
        model::SyntaxHelperSource {
            hbk_path: PathBuf::from("fixture.hbk"),
            locale: "ru".to_string(),
            toc_path: Some(name.to_string()),
            html_path: html_path.to_string(),
            page_title: name.to_string(),
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "v8-context-hbk-search-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        path
    }

    fn table_columns(connection: &Connection, table: &str) -> Vec<String> {
        let mut statement = connection
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("table info statement must prepare");
        statement
            .query_map([], |row| row.get(1))
            .expect("table info query must run")
            .collect::<Result<Vec<_>, _>>()
            .expect("table info rows must parse")
    }
}
