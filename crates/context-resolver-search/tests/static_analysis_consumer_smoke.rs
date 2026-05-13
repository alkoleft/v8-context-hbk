#[test]
fn static_analysis_consumer_uses_prebuilt_index_through_resolver_adapters() {
    let index_path = provider_setup::build_provider_index();

    analyzer_lookup::assert_static_analysis_lookup_surface(&index_path);

    let _ = std::fs::remove_file(index_path);
}

mod provider_setup {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use syntax_helper_language::{LanguagePageInput, LanguageSourceFamily, extract_language_facts};
    use syntax_helper_model as model;
    use syntax_helper_model::SyntaxHelperSink;
    use syntax_helper_search::{IndexMetadata, SearchIndexBuilder, build_index_from_builder};

    pub fn build_provider_index() -> PathBuf {
        let path = temp_path("static-analysis-consumer-smoke.sqlite");
        let mut builder = SearchIndexBuilder::new();

        for record in [
            platform_type("НастройкиКомпоновкиДанных"),
            platform_type("ОтборКомпоновкиДанных"),
            platform_type("ЭлементОтбораКомпоновкиДанных"),
            platform_type_template(
                "ДокументМенеджер.<Имя документа>",
                "DocumentManager.<Document name>",
            ),
            platform_type_template(
                "ДокументОбъект.<Имя документа>",
                "DocumentObject.<Document name>",
            ),
            platform_type_template(
                "ДокументСсылка.<Имя документа>",
                "DocumentRef.<Document name>",
            ),
        ] {
            builder
                .platform_type(record)
                .expect("platform type must sink");
        }
        builder
            .enum_definition(enum_definition(
                "ОбновлениеПредопределенныхДанных",
                "PredefinedDataUpdate",
            ))
            .expect("enum definition must sink");
        builder
            .type_property(model::PlatformProperty {
                owner: name("НастройкиКомпоновкиДанных"),
                owner_identity: Some("platform_type:НастройкиКомпоновкиДанных".to_string()),
                name: name("Отбор"),
                semantic: model::SemanticContext::new(
                    model::BranchKind::PlatformObjects,
                    model::RecordFamily::TypeProperty,
                ),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ОтборКомпоновкиДанных".to_string(),
                }],
                description: Some("Фильтр настроек.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("settings-filter"),
            })
            .expect("platform property must sink");
        builder
            .type_property(model::PlatformProperty {
                owner: name("НастройкиКомпоновкиДанных"),
                owner_identity: Some("platform_type:НастройкиКомпоновкиДанных".to_string()),
                name: name("Обновление"),
                semantic: model::SemanticContext::new(
                    model::BranchKind::PlatformObjects,
                    model::RecordFamily::TypeProperty,
                ),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ОбновлениеПредопределенныхДанных".to_string(),
                }],
                description: Some("Режим обновления предопределенных данных.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("settings-predefined-data-update"),
            })
            .expect("enum-backed property must sink");
        builder
            .type_method(model::PlatformMethod {
                owner: name("ОтборКомпоновкиДанных"),
                owner_identity: Some("platform_type:ОтборКомпоновкиДанных".to_string()),
                name: name("Найти"),
                semantic: model::SemanticContext::new(
                    model::BranchKind::PlatformObjects,
                    model::RecordFamily::TypeMethod,
                ),
                signatures: vec![model::Signature {
                    text: "Найти(<Значение>)".to_string(),
                    parameters: vec![model::Parameter {
                        name: "Значение".to_string(),
                        required: true,
                        type_refs: vec![model::TypeRef {
                            name: "ОтборКомпоновкиДанных".to_string(),
                        }],
                        description: Some("Искомое значение.".to_string()),
                    }],
                    return_types: Vec::new(),
                    variant: None,
                }],
                return_types: vec![model::TypeRef {
                    name: "ЭлементОтбораКомпоновкиДанных".to_string(),
                }],
                description: Some("Ищет элемент фильтра.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("filter-find"),
            })
            .expect("platform method must sink");
        builder
            .type_method(model::PlatformMethod {
                owner: name_with_alias(
                    "ДокументОбъект.<Имя документа>",
                    "DocumentObject.<Document name>",
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name("Связать"),
                semantic: model::SemanticContext::new(
                    model::BranchKind::PlatformObjects,
                    model::RecordFamily::TypeMethod,
                ),
                signatures: vec![model::Signature {
                    text: "Связать(<Ссылка>)".to_string(),
                    parameters: vec![model::Parameter {
                        name: "Ссылка".to_string(),
                        required: true,
                        type_refs: vec![model::TypeRef {
                            name: "ДокументСсылка".to_string(),
                        }],
                        description: Some("Ссылка на документ.".to_string()),
                    }],
                    return_types: vec![model::TypeRef {
                        name: "ДокументСсылка".to_string(),
                    }],
                    variant: None,
                }],
                return_types: Vec::new(),
                description: Some("Связывает объект с ссылкой.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("document-object-link"),
            })
            .expect("type-template method must sink");
        builder
            .global_method(model::GlobalMethod {
                name: name("Сообщить"),
                signatures: vec![model::Signature {
                    text: "Сообщить(<Сообщение>)".to_string(),
                    parameters: Vec::new(),
                    return_types: Vec::new(),
                    variant: None,
                }],
                return_types: Vec::new(),
                description: Some("Выводит сообщение.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("global-message"),
            })
            .expect("global method must sink");
        builder
            .global_method(model::GlobalMethod {
                name: name("ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы"),
                signatures: vec![model::Signature {
                    text: "ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы(<Режим>)"
                        .to_string(),
                    parameters: vec![model::Parameter {
                        name: "Режим".to_string(),
                        required: false,
                        type_refs: vec![model::TypeRef {
                            name: "ОбновлениеПредопределенныхДанных".to_string(),
                        }],
                        description: Some("Режим обновления.".to_string()),
                    }],
                    return_types: Vec::new(),
                    variant: None,
                }],
                return_types: vec![model::TypeRef {
                    name: "ОбновлениеПредопределенныхДанных".to_string(),
                }],
                description: Some("Возвращает режим обновления.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("global-predefined-data-update"),
            })
            .expect("enum-backed global method must sink");

        for fact in shlang_string_facts() {
            builder.add_language_fact(fact);
        }

        build_index_from_builder(&path, &metadata(), builder).expect("provider index must build");
        path
    }

    fn shlang_string_facts() -> Vec<syntax_helper_language::LanguageFact> {
        let html = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/syntax-helper-language/shlang_def_string_ru.html"),
        )
        .expect("language fixture must be readable");

        extract_language_facts(LanguagePageInput {
            source_hbk: "fixture-shlang.hbk",
            source_family: LanguageSourceFamily::Shlang,
            locale: "ru",
            html_path: "def_String",
            html: &html,
        })
    }

    fn platform_type(primary: &str) -> model::PlatformType {
        let facts = if primary == "ОтборКомпоновкиДанных" {
            model::SectionFacts {
                availability: model::Availability {
                    contexts: vec![
                        model::AvailabilityContext::ThinClient,
                        model::AvailabilityContext::Server,
                    ],
                },
                available_since: Some(model::VersionFact {
                    version: Some("8.3.6".to_string()),
                    text: "Available since 8.3.6".to_string(),
                }),
                ..model::SectionFacts::default()
            }
        } else {
            model::SectionFacts::default()
        };
        model::PlatformType {
            identity: None,
            name: name(primary),
            semantic: model::SemanticContext::new(
                model::BranchKind::PlatformObjects,
                model::RecordFamily::PlatformType,
            ),
            type_kind: model::PlatformTypeKind::Regular,
            object_kind: None,
            extends: Vec::new(),
            metadata_kind: None,
            template_parameters: Vec::new(),
            type_template_key: None,
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: Some(format!("{primary} description.")),
            facts,
            source: source_ref(primary),
        }
    }

    fn platform_type_template(primary: &str, alias: &str) -> model::PlatformType {
        let mut record = platform_type(primary);
        record.name = name_with_alias(primary, alias);
        record.type_kind = model::PlatformTypeKind::MetadataTemplate;
        record.metadata_kind = record.name.primary.split('.').next().map(str::to_string);
        record.template_parameters = vec!["Имя документа".to_string()];
        record
    }

    fn enum_definition(primary: &str, alias: &str) -> model::EnumDefinition {
        model::EnumDefinition {
            identity: None,
            name: name_with_alias(primary, alias),
            value_links: Vec::new(),
            description: Some(format!("{primary} enum description.")),
            facts: model::SectionFacts::default(),
            source: model::SyntaxHelperSource {
                hbk_path: PathBuf::from("/fixtures/shcntx_ru.hbk"),
                locale: "ru".to_string(),
                toc_path: Some(primary.to_string()),
                html_path: format!("objects/catalog2/{primary}.html"),
                page_title: primary.to_string(),
            },
        }
    }

    fn name(primary: &str) -> model::LocalizedName {
        model::LocalizedName {
            primary: primary.to_string(),
            alias: None,
        }
    }

    fn name_with_alias(primary: &str, alias: &str) -> model::LocalizedName {
        model::LocalizedName {
            primary: primary.to_string(),
            alias: Some(alias.to_string()),
        }
    }

    fn source_ref(title: &str) -> model::SyntaxHelperSource {
        model::SyntaxHelperSource {
            hbk_path: PathBuf::from("/fixtures/shcntx_ru.hbk"),
            locale: "ru".to_string(),
            toc_path: Some(title.to_string()),
            html_path: format!("{title}.html"),
            page_title: title.to_string(),
        }
    }

    fn metadata() -> IndexMetadata {
        IndexMetadata {
            locale: "ru".to_string(),
            source_locale: "ru".to_string(),
            source_hbk: "/fixtures/provider-smoke.hbk".to_string(),
            source_extraction_schema_version: 11,
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be after Unix epoch")
            .as_nanos();
        path.push(format!(
            "v8-context-hbk-{}-{unique}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }
}

mod analyzer_lookup {
    use std::path::Path;

    use context_resolver_core::{
        AvailabilityContext, CallableLookup, CompositeResolver, ContextResolver, FactDetails,
        FactKind, GlobalContextLanguage, GlobalContextQuery, LanguageDomain, MemberQuery,
        PlatformTypeTemplateKey, RelationKind, ResolveContext, ResolveQuery, ResolveStatus,
        SourceId, TemplateParameterBinding, TypeLookup,
    };
    use context_resolver_search::{LanguageSearchSource, PlatformSearchSource};

    pub fn assert_static_analysis_lookup_surface(index_path: &Path) {
        let platform_source = SourceId::new("consumer-platform");
        let shlang_source = SourceId::new("shlang");
        let resolver = CompositeResolver::new(vec![
            Box::new(
                PlatformSearchSource::open_read_only_with_source_id(
                    index_path,
                    platform_source.clone(),
                )
                .expect("platform source must open existing provider index"),
            ),
            Box::new(
                LanguageSearchSource::open_shlang_read_only(index_path)
                    .expect("language source must open existing provider index"),
            ),
        ]);

        let global_context = resolver
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Bsl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("BSL global context lookup must not fail");
        assert_eq!(global_context.status, ResolveStatus::Ok);
        let global_context = global_context
            .facts
            .first()
            .expect("BSL global context must resolve");
        assert!(
            global_context
                .facts
                .iter()
                .any(|fact| fact.id.domain == LanguageDomain::BslLanguage)
        );
        assert!(
            global_context
                .methods
                .iter()
                .any(|method| method.id.0.domain == LanguageDomain::PlatformApi)
        );

        let settings = resolver
            .resolve_type(
                TypeLookup::ExactName {
                    source: Some(&platform_source),
                    domain: Some(LanguageDomain::PlatformApi),
                    name: "НастройкиКомпоновкиДанных",
                },
                &ResolveContext::all(),
            )
            .expect("platform type lookup must not fail");
        assert_eq!(settings.status, ResolveStatus::Ok);
        let settings = settings
            .facts
            .first()
            .expect("platform type must resolve")
            .id
            .clone();

        let filter_member = resolver
            .members(
                &settings,
                MemberQuery {
                    name: Some("Отбор"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("member lookup must not fail");
        assert_eq!(filter_member.status, ResolveStatus::Ok);
        let filter_member = filter_member
            .facts
            .first()
            .expect("platform member must resolve");
        assert_eq!(
            filter_member
                .info
                .types
                .first()
                .and_then(|type_ref| type_ref.resolved_id())
                .expect("member type id must be resolved")
                .0
                .local_id,
            "platform_type:ОтборКомпоновкиДанных"
        );
        let predefined_update = resolver
            .members(
                &settings,
                MemberQuery {
                    name: Some("Обновление"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("enum-backed member lookup must not fail");
        assert_eq!(predefined_update.status, ResolveStatus::Ok);
        assert_eq!(
            predefined_update.facts[0].info.types[0]
                .resolved_id()
                .expect("property enum type id must be resolved")
                .0
                .local_id,
            "enum:system:ОбновлениеПредопределенныхДанных"
        );
        let enum_type_id = predefined_update.facts[0].info.types[0]
            .resolved_id()
            .expect("property enum type id must be resolved")
            .clone();
        let enum_fact = resolver
            .resolve(ResolveQuery::Id(&enum_type_id.0), &ResolveContext::all())
            .expect("enum-backed type id lookup must not fail");
        assert_eq!(enum_fact.status, ResolveStatus::Ok);
        assert_eq!(enum_fact.facts[0].id.kind, FactKind::Type);
        assert!(matches!(enum_fact.facts[0].details, FactDetails::Type(_)));
        let related_enum = resolver
            .related(
                &predefined_update.facts[0].id.0,
                RelationKind::HasType,
                &ResolveContext::all(),
            )
            .expect("enum-backed type relation traversal must not fail");
        assert_eq!(related_enum.status, ResolveStatus::Ok);
        assert_eq!(related_enum.facts[0].id.kind, FactKind::Type);
        assert_eq!(
            related_enum.facts[0].id.local_id,
            "enum:system:ОбновлениеПредопределенныхДанных"
        );
        assert!(matches!(
            related_enum.facts[0].details,
            FactDetails::Type(_)
        ));
        let filter_type = filter_member
            .info
            .types
            .first()
            .and_then(|type_ref| type_ref.resolved_id().cloned())
            .expect("member type id must be available for callable owner lookup");

        let availability = resolver
            .availability(&filter_type.0, &ResolveContext::all())
            .expect("availability lookup must not fail");
        assert_eq!(availability.status, ResolveStatus::Ok);
        assert_eq!(
            availability.facts[0].availability.contexts,
            vec![AvailabilityContext::ThinClient, AvailabilityContext::Server]
        );
        assert_eq!(
            availability.facts[0].availability.since.as_deref(),
            Some("8.3.6")
        );

        let callable = resolver
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter_type),
                    name: "Найти",
                },
                &ResolveContext::all(),
            )
            .expect("callable lookup must not fail");
        assert_eq!(callable.status, ResolveStatus::Ok);
        let callable = callable.facts.first().expect("callable must resolve");
        assert_eq!(callable.fact.name.primary, "Найти");
        assert_eq!(callable.info.signatures[0].parameters[0].name, "Значение");
        assert_eq!(
            callable.info.return_types[0]
                .resolved_id()
                .expect("return type id must be resolved")
                .0
                .local_id,
            "platform_type:ЭлементОтбораКомпоновкиДанных"
        );

        let update_callable = resolver
            .callable(
                CallableLookup::OwnerName {
                    owner: None,
                    name: "ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
                },
                &ResolveContext::all(),
            )
            .expect("enum-backed global callable lookup must not fail");
        assert_eq!(update_callable.status, ResolveStatus::Ok);
        let update_callable = update_callable
            .facts
            .first()
            .expect("enum-backed global callable must resolve");
        assert_eq!(
            update_callable.info.return_types[0]
                .resolved_id()
                .expect("callable enum return type id must be resolved")
                .0
                .local_id,
            "enum:system:ОбновлениеПредопределенныхДанных"
        );
        assert_eq!(
            update_callable.info.signatures[0].parameters[0].types[0]
                .resolved_id()
                .expect("callable enum parameter type id must be resolved")
                .0
                .local_id,
            "enum:system:ОбновлениеПредопределенныхДанных"
        );

        let document_object = resolver
            .resolve_type(
                TypeLookup::PlatformTypeTemplate {
                    source: Some(&platform_source),
                    domain: Some(LanguageDomain::PlatformApi),
                    key: &PlatformTypeTemplateKey::new("Document", "Object"),
                },
                &ResolveContext::all(),
            )
            .expect("template type lookup must not fail");
        assert_eq!(document_object.status, ResolveStatus::Ok);
        let document_object = document_object
            .facts
            .first()
            .expect("document object template must resolve")
            .id
            .clone();

        let template_callable = resolver
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&document_object),
                    name: "Связать",
                },
                &ResolveContext::all(),
            )
            .expect("template callable lookup must not fail");
        assert_eq!(template_callable.status, ResolveStatus::Ok);
        let template_callable = template_callable
            .facts
            .first()
            .expect("template callable must resolve");
        let parameter_binding = template_callable.info.signatures[0].parameters[0].types[0]
            .template_binding
            .as_ref()
            .expect("parameter type binding must survive adapter mapping");
        assert_eq!(
            parameter_binding.template_key,
            PlatformTypeTemplateKey::new("Document", "Ref")
        );
        assert_eq!(
            parameter_binding.arguments,
            vec![TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: 0,
                target_parameter_index: 0,
            }]
        );
        assert_eq!(
            template_callable.info.signatures[0].return_types[0]
                .template_binding
                .as_ref(),
            Some(parameter_binding)
        );

        let bsl_string = resolver
            .resolve_type(
                TypeLookup::ExactName {
                    source: Some(&shlang_source),
                    domain: Some(LanguageDomain::BslLanguage),
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("BSL language fact lookup must not fail");
        assert_eq!(bsl_string.status, ResolveStatus::Ok);
        assert_eq!(bsl_string.facts[0].id.0.source, shlang_source);
        assert_eq!(bsl_string.facts[0].id.0.domain, LanguageDomain::BslLanguage);
        assert_eq!(bsl_string.facts[0].id.0.local_id, "def_String");
    }
}
