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
        ] {
            builder
                .platform_type(record)
                .expect("platform type must sink");
        }
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
            generic_template_kind: None,
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: Some(format!("{primary} description.")),
            facts,
            source: source_ref(primary),
        }
    }

    fn name(primary: &str) -> model::LocalizedName {
        model::LocalizedName {
            primary: primary.to_string(),
            alias: None,
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
        AvailabilityContext, CallableLookup, CompositeResolver, ContextResolver, LanguageDomain,
        MemberQuery, ResolveContext, ResolveStatus, SourceId, TypeLookup,
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
                .and_then(|type_ref| type_ref.id.as_ref())
                .expect("member type id must be resolved")
                .0
                .local_id,
            "platform_type:ОтборКомпоновкиДанных"
        );
        let filter_type = filter_member
            .info
            .types
            .first()
            .and_then(|type_ref| type_ref.id.clone())
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
                .id
                .as_ref()
                .expect("return type id must be resolved")
                .0
                .local_id,
            "platform_type:ЭлементОтбораКомпоновкиДанных"
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
