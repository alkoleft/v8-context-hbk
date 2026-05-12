#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Instant;

    use context_resolver_core::{
        CallableLookup, CompositeResolver, ContextResolver, ContextSource, GlobalContextLanguage,
        GlobalContextQuery, MemberQuery, PlatformTypeTemplateKey, RelationKind, ResolveContext,
        ResolveStatus, TemplateParameterBinding, TypeLookup,
    };
    use syntax_helper_language::{LanguagePageInput, LanguageSourceFamily, extract_language_facts};
    use syntax_helper_model as model;
    use syntax_helper_model::SyntaxHelperSink;
    use syntax_helper_search::{IndexMetadata, SearchIndexBuilder, build_index_from_builder};

    use super::*;

    #[test]
    fn platform_adapter_opens_read_only_index_from_path() {
        let source = fixture_source();
        let path = fixture_index_path("platform-adapter-open-read-only.sqlite");
        let adapter = PlatformSearchSource::open_read_only_with_source_id(&path, source.clone())
            .expect("platform adapter must open index path");

        assert_eq!(adapter.descriptor().id, source.clone());

        let response = adapter
            .resolve_type(
                TypeLookup::ExactName {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    name: "ОтборКомпоновкиДанных",
                },
                &ResolveContext::all(),
            )
            .expect("type lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(response.facts.len(), 1);
    }

    #[test]
    fn platform_adapter_resolves_alias_and_metadata_template_info() {
        let source = fixture_source();
        let index = fixture_index("platform-adapter-template-alias.sqlite");
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());

        let response = adapter
            .resolve_type(
                TypeLookup::ExactAlias {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    alias: "CatalogManager.<Catalog name>",
                },
                &ResolveContext::all(),
            )
            .expect("alias type lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        let template = response.facts.first().expect("template must resolve");
        assert_eq!(
            template.id.0.local_id,
            "platform_type:СправочникМенеджер.<Имя справочника>"
        );
        assert_eq!(
            template
                .info
                .metadata_template
                .as_ref()
                .expect("template metadata must be exposed")
                .metadata_kind,
            "СправочникМенеджер"
        );
        assert_eq!(
            template.info.metadata_template.as_ref().unwrap().parameters,
            vec!["Имя справочника".to_string()]
        );
        assert_eq!(
            template.info.type_template_key,
            Some(PlatformTypeTemplateKey::new("Catalog", "Manager"))
        );

        let by_kind = adapter
            .resolve_type(
                TypeLookup::PlatformTypeTemplate {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    key: &PlatformTypeTemplateKey::new("Catalog", "Manager"),
                },
                &ResolveContext::all(),
            )
            .expect("semantic type template lookup must not fail");
        assert_eq!(by_kind.status, ResolveStatus::Ok);
        assert_eq!(by_kind.facts[0].id, template.id);
    }

    #[test]
    fn platform_adapter_exposes_template_owner_parameter_binding() {
        let source = fixture_source();
        let index = fixture_index("platform-adapter-generic-binding.sqlite");
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());
        let owner = adapter
            .resolve_type(
                TypeLookup::PlatformTypeTemplate {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    key: &PlatformTypeTemplateKey::new("Document", "Object"),
                },
                &ResolveContext::all(),
            )
            .expect("document object template lookup must not fail")
            .facts
            .into_iter()
            .next()
            .expect("document object template must resolve");

        let response = adapter
            .members(
                &owner.id,
                MemberQuery {
                    name: Some("Ссылка"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("type-template member lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        let property_type = response.facts[0]
            .info
            .types
            .first()
            .expect("type-template property type must be exposed");
        assert_eq!(
            property_type.resolved_id().map(|id| id.0.local_id.as_str()),
            Some("platform_type:ДокументСсылка.<Имя документа>")
        );
        let binding = property_type
            .template_binding
            .as_ref()
            .expect("template owner-parameter binding must be visible");
        assert_eq!(
            binding.template_key,
            PlatformTypeTemplateKey::new("Document", "Ref")
        );
        assert_eq!(
            binding.arguments,
            vec![TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: 0,
                target_parameter_index: 0,
            }]
        );
    }

    #[test]
    fn platform_adapter_exposes_template_constructor_result_binding() {
        let source = fixture_source();
        let index = fixture_index("platform-adapter-generic-constructor-binding.sqlite");
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());
        let owner = TypeId(FactId::new(
            source,
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ДокументОбъект.<Имя документа>",
        ));

        let constructor = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&owner),
                    name: "Новый ДокументОбъект.<Имя документа>()",
                },
                &ResolveContext::all(),
            )
            .expect("type-template constructor lookup must not fail");

        assert_eq!(constructor.status, ResolveStatus::Ok);
        let result_type = constructor.facts[0]
            .info
            .return_types
            .first()
            .expect("type-template constructor result type must be exposed");
        assert_eq!(
            result_type.resolved_id().map(|id| id.0.local_id.as_str()),
            Some("platform_type:ДокументОбъект.<Имя документа>")
        );
        let binding = result_type
            .template_binding
            .as_ref()
            .expect("type-template constructor result binding must be visible");
        assert_eq!(
            binding.template_key,
            PlatformTypeTemplateKey::new("Document", "Object")
        );
        assert_eq!(
            binding.arguments,
            vec![TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: 0,
                target_parameter_index: 0,
            }]
        );
    }

    #[test]
    fn platform_adapter_resolves_type_member_callable_and_relations() {
        let source = fixture_source();
        let filter = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ОтборКомпоновкиДанных",
        ));
        let settings = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:НастройкиКомпоновкиДанных",
        ));
        let index = fixture_index("platform-adapter.sqlite");
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());

        let started = Instant::now();
        let type_response = adapter
            .resolve_type(
                TypeLookup::Id(&filter),
                &ResolveContext {
                    active_sources: std::slice::from_ref(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    scope: None,
                },
            )
            .expect("type lookup must not fail");
        assert_eq!(type_response.status, ResolveStatus::Ok);
        assert_eq!(
            type_response.facts[0].fact.name.primary,
            "ОтборКомпоновкиДанных"
        );
        assert!(started.elapsed().as_millis() < 100);

        let started = Instant::now();
        let members = adapter
            .members(
                &filter,
                MemberQuery {
                    name: None,
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("member listing must not fail");
        assert!(
            members
                .facts
                .iter()
                .any(|member| member.fact.name.primary == "Элементы")
        );
        assert!(started.elapsed().as_millis() < 100);

        let filter_member = adapter
            .members(
                &settings,
                MemberQuery {
                    name: Some("Отбор"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("owner member lookup must not fail");
        assert_eq!(filter_member.facts.len(), 1);
        assert_eq!(filter_member.facts[0].owner, settings);

        let started = Instant::now();
        let has_type = adapter
            .related(
                &filter_member.facts[0].id.0,
                RelationKind::HasType,
                &ResolveContext::all(),
            )
            .expect("has_type traversal must not fail");
        let relation_elapsed = started.elapsed();
        assert_eq!(
            has_type.facts[0].id.local_id,
            "platform_type:ОтборКомпоновкиДанных"
        );
        assert!(relation_elapsed.as_millis() < 100);

        let started = Instant::now();
        let callable = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Найти",
                },
                &ResolveContext::all(),
            )
            .expect("callable lookup must not fail");
        let callable_elapsed = started.elapsed();
        assert_eq!(callable.status, ResolveStatus::Ok);
        assert_eq!(
            callable.facts[0].info.signatures[0].parameters[0].name,
            "Значение"
        );
        assert!(callable_elapsed.as_millis() < 100);
        assert_eq!(
            callable.facts[0].info.return_types[0].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:ЭлементОтбораКомпоновкиДанных",
            )))
        );

        let returns = adapter
            .related(
                &callable.facts[0].id.0,
                RelationKind::Returns,
                &ResolveContext::all(),
            )
            .expect("returns traversal must not fail");
        assert_eq!(
            returns.facts[0].id.local_id,
            "platform_type:ЭлементОтбораКомпоновкиДанных"
        );

        let constructor = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Новый ОтборКомпоновкиДанных()",
                },
                &ResolveContext::all(),
            )
            .expect("constructor lookup must not fail");
        assert_eq!(constructor.status, ResolveStatus::Ok);
        assert_eq!(constructor.facts.len(), 1);
        assert_eq!(
            constructor.facts[0].info.return_types[0].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:ОтборКомпоновкиДанных",
            )))
        );
        let constructs = adapter
            .related(
                &constructor.facts[0].id.0,
                RelationKind::Constructs,
                &ResolveContext::all(),
            )
            .expect("constructs traversal must not fail");
        assert_eq!(
            constructs.facts[0].id.local_id,
            "platform_type:ОтборКомпоновкиДанных"
        );

        let member_of = adapter
            .related(
                &filter_member.facts[0].id.0,
                RelationKind::MemberOf,
                &ResolveContext::all(),
            )
            .expect("member_of traversal must not fail");
        assert_eq!(
            member_of.facts[0].id.local_id,
            "platform_type:НастройкиКомпоновкиДанных"
        );
    }

    #[test]
    fn platform_adapter_exposes_bsl_global_context_and_ownerless_global_callable() {
        let source = fixture_source();
        let adapter = PlatformSearchSource::with_source_id(
            fixture_index("platform-global-context.sqlite"),
            source.clone(),
        );

        let scope = adapter
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Bsl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("platform global context lookup must not fail");

        assert_eq!(scope.status, ResolveStatus::Ok);
        let scope = scope.facts.first().expect("BSL global scope must resolve");
        assert_eq!(scope.language, GlobalContextLanguage::Bsl);
        assert_eq!(scope.sources, vec![source.clone()]);
        assert!(
            scope
                .methods
                .iter()
                .any(|method| method.fact.name.primary == "Сообщить")
        );
        assert!(scope.properties.iter().any(|property| {
            property.name.primary == "ТекущийОтбор"
                && matches!(property.details, FactDetails::Member(_))
                && property.owner.is_none()
        }));
        let global_property = scope
            .properties
            .iter()
            .find(|property| property.name.primary == "ТекущийОтбор")
            .expect("global property must be present in BSL global context")
            .id
            .clone();
        let resolved_property = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&global_property),
                &ResolveContext::all(),
            )
            .expect("global property id lookup must not fail");
        assert_eq!(resolved_property.status, ResolveStatus::Ok);
        assert_eq!(resolved_property.facts[0].id.kind, FactKind::Global);

        let callable = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: None,
                    name: "Сообщить",
                },
                &ResolveContext::all(),
            )
            .expect("ownerless global method lookup must not fail");

        assert_eq!(callable.status, ResolveStatus::Ok);
        assert_eq!(callable.facts.len(), 1);
        assert_eq!(callable.facts[0].info.kind, CallableKind::GlobalMethod);
        assert_eq!(callable.facts[0].owner, None);
    }

    #[test]
    fn platform_adapter_exposes_provider_backed_module_context_events() {
        let source = fixture_source();
        let adapter = PlatformSearchSource::with_source_id(
            fixture_index("platform-module-context.sqlite"),
            source.clone(),
        );

        let context = adapter
            .module_context(
                ModuleContextQuery {
                    language: GlobalContextLanguage::Bsl,
                    domain: LanguageDomain::PlatformApi,
                    kind: ModuleContextKind::Form,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("module context lookup must not fail");

        assert_eq!(context.status, ResolveStatus::Ok);
        let context = context
            .facts
            .first()
            .expect("form module context must resolve");
        assert_eq!(context.id.kind, FactKind::ModuleContext);
        assert_eq!(context.kind, ModuleContextKind::Form);
        assert_eq!(context.sources, vec![source.clone()]);
        assert_eq!(context.self_member, None);
        let context_fact = context
            .facts
            .first()
            .expect("module context handle fact must be returned");
        assert_eq!(context_fact.id, context.id);
        assert!(matches!(
            context_fact.details,
            FactDetails::ModuleContext(_)
        ));
        let resolved_context_fact = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&context_fact.id),
                &ResolveContext::all(),
            )
            .expect("module context fact lookup must not fail");
        assert_eq!(resolved_context_fact.status, ResolveStatus::Ok);
        assert_eq!(resolved_context_fact.facts[0], *context_fact);
        assert!(
            context
                .methods
                .iter()
                .any(|method| method.fact.name.primary == "Сообщить")
        );
        assert!(
            context
                .properties
                .iter()
                .any(|property| property.name.primary == "ТекущийОтбор")
        );
        let event = context
            .events
            .iter()
            .find(|event| event.fact.name.primary == "ПриОткрытии")
            .expect("form module event must be returned");
        assert_eq!(event.id.0.source, source.clone());
        assert_eq!(event.id.0.domain, LanguageDomain::PlatformApi);
        assert_eq!(event.info.kind, CallableKind::Event);
        assert_eq!(event.fact.owner.as_ref(), Some(&context.id));
        assert_eq!(event.fact.name.alias.as_deref(), Some("OnOpen"));
        assert_eq!(event.info.signatures.len(), 1);
        assert_eq!(event.info.signatures[0].parameters[0].name, "Отказ");

        let availability = adapter
            .availability(&event.id.0, &ResolveContext::all())
            .expect("event availability lookup must not fail");
        assert_eq!(availability.status, ResolveStatus::Ok);
        assert_eq!(
            availability.facts[0].availability.since.as_deref(),
            Some("8.3.1")
        );
    }

    #[test]
    fn platform_adapter_does_not_fabricate_unsupported_module_self_member() {
        let adapter = PlatformSearchSource::with_source_id(
            fixture_index("platform-module-context-unsupported.sqlite"),
            fixture_source(),
        );

        let context = adapter
            .module_context(
                ModuleContextQuery {
                    language: GlobalContextLanguage::Bsl,
                    domain: LanguageDomain::PlatformApi,
                    kind: ModuleContextKind::Command,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("unsupported module context lookup must not fail");

        assert_eq!(context.status, ResolveStatus::Unsupported);
        assert!(context.facts.is_empty());
        assert!(
            context.diagnostics[0]
                .message
                .contains("not provider-backed")
        );
    }

    #[test]
    fn platform_adapter_type_event_member_id_round_trips_and_exact_miss_is_not_found() {
        let source = fixture_source();
        let filter = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ОтборКомпоновкиДанных",
        ));
        let adapter = PlatformSearchSource::with_source_id(
            fixture_index("platform-member-edge.sqlite"),
            source,
        );

        let missing = adapter
            .members(
                &filter,
                MemberQuery {
                    name: Some("НетТакогоЧлена"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("exact member miss must not fail");
        assert_eq!(missing.status, ResolveStatus::NotFound);

        let events = adapter
            .members(
                &filter,
                MemberQuery {
                    name: Some("ПередЗаписью"),
                    kind: Some(MemberQueryKind::Event),
                },
                &ResolveContext::all(),
            )
            .expect("type event member lookup must not fail");
        assert_eq!(events.status, ResolveStatus::Ok);
        assert_eq!(events.facts.len(), 1);
        assert_eq!(events.facts[0].info.kind, MemberKind::Event);

        let resolved = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&events.facts[0].id.0),
                &ResolveContext::all(),
            )
            .expect("type event member id lookup must not fail");
        assert_eq!(resolved.status, ResolveStatus::Ok);
        assert_eq!(resolved.facts[0].id.kind, FactKind::Member);
        assert_eq!(resolved.facts[0].name.primary, "ПередЗаписью");
        assert!(matches!(resolved.facts[0].details, FactDetails::Member(_)));
    }

    #[test]
    fn platform_adapter_binds_type_events_to_semantic_owner_identity() {
        let path = temp_path("platform-type-event-semantic-owner.sqlite");
        let mut builder = SearchIndexBuilder::new();
        builder
            .platform_type(platform_type_with_owner_path("ДубльТип", "Первый"))
            .expect("first duplicate platform type must sink");
        builder
            .platform_type(platform_type_with_owner_path("ДубльТип", "Второй"))
            .expect("second duplicate platform type must sink");
        let mut event =
            type_event_with_owner_path(&["Второй", "ДубльТип", "События"], "ПередЗаписью");
        event.owner_identity = Some("platform_type:ДубльТип:Второй".to_string());
        builder
            .global_context_event(event)
            .expect("type event must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");

        let source = fixture_source();
        let adapter = PlatformSearchSource::with_source_id(open_index(&path), source.clone());
        let duplicate_types = open_index(&path)
            .type_identities_by_name("ДубльТип")
            .expect("duplicate type identities must be readable");
        assert_eq!(duplicate_types.len(), 2);
        let second_owner = duplicate_types
            .iter()
            .find(|hit| hit.document.id.contains("Второй"))
            .expect("second semantic owner identity must be present");
        let first_owner = duplicate_types
            .iter()
            .find(|hit| hit.document.id.contains("Первый"))
            .expect("first semantic owner identity must be present");

        let second_members = adapter
            .members(
                &TypeId(FactId::new(
                    source.clone(),
                    LanguageDomain::PlatformApi,
                    FactKind::Type,
                    second_owner.document.id.clone(),
                )),
                MemberQuery {
                    name: Some("ПередЗаписью"),
                    kind: Some(MemberQueryKind::Event),
                },
                &ResolveContext::all(),
            )
            .expect("second owner event lookup must not fail");
        assert_eq!(second_members.status, ResolveStatus::Ok);
        assert_eq!(second_members.facts.len(), 1);

        let first_members = adapter
            .members(
                &TypeId(FactId::new(
                    source,
                    LanguageDomain::PlatformApi,
                    FactKind::Type,
                    first_owner.document.id.clone(),
                )),
                MemberQuery {
                    name: Some("ПередЗаписью"),
                    kind: Some(MemberQueryKind::Event),
                },
                &ResolveContext::all(),
            )
            .expect("first owner event lookup must not fail");
        assert_eq!(first_members.status, ResolveStatus::NotFound);
    }

    #[test]
    fn platform_adapter_keeps_signature_return_out_of_callable_return() {
        let source = fixture_source();
        let filter = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ОтборКомпоновкиДанных",
        ));
        let index = fixture_index_with_signature_only_return(
            "platform-adapter-signature-only-return.sqlite",
        );
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());

        let callable = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Найти",
                },
                &ResolveContext::all(),
            )
            .expect("callable lookup must not fail");

        assert_eq!(callable.status, ResolveStatus::Ok);
        assert!(
            callable.facts[0].info.return_types.is_empty(),
            "signature return types must not be folded back into callable return types"
        );
        let signature_return = callable.facts[0].info.signatures[0]
            .return_types
            .first()
            .expect("signature-level return type must be exposed");
        assert_eq!(
            signature_return
                .resolved_id()
                .map(|id| id.0.local_id.as_str()),
            Some("platform_type:ЭлементОтбораКомпоновкиДанных")
        );
    }

    #[test]
    fn platform_adapter_preserves_type_ref_resolution_status_and_template_binding() {
        let source = fixture_source();
        let path = temp_path("platform-adapter-type-ref-resolution.sqlite");
        let mut builder = SearchIndexBuilder::new();
        for record in [
            platform_template_type(
                "ДокументМенеджер.<Имя документа>",
                "DocumentManager.<Document name>",
                "ДокументМенеджер",
                "Имя документа",
            ),
            platform_template_type(
                "ДокументОбъект.<Имя документа>",
                "DocumentObject.<Document name>",
                "ДокументОбъект",
                "Имя документа",
            ),
            platform_template_type(
                "ДокументСсылка.<Имя документа>",
                "DocumentRef.<Document name>",
                "ДокументСсылка",
                "Имя документа",
            ),
            platform_type("РазрешенныйТип", None),
            platform_type_with_owner_path("ДубльТип", "Первый"),
            platform_type_with_owner_path("ДубльТип", "Второй"),
        ] {
            builder
                .platform_type(record)
                .expect("platform type must sink");
        }
        builder
            .type_property(model::PlatformProperty {
                owner: name(
                    "ДокументОбъект.<Имя документа>",
                    Some("DocumentObject.<Document name>"),
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name("Поле", None),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![
                    model::TypeRef {
                        name: "ДокументСсылка".to_string(),
                    },
                    model::TypeRef {
                        name: "РазрешенныйТип".to_string(),
                    },
                    model::TypeRef {
                        name: "НесуществующийТип".to_string(),
                    },
                    model::TypeRef {
                        name: "ДубльТип".to_string(),
                    },
                ],
                description: None,
                facts: model::SectionFacts::default(),
                source: source_ref("owner-field"),
            })
            .expect("property must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");
        let adapter = PlatformSearchSource::with_source_id(open_index(&path), source.clone());
        let owner = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ДокументОбъект.<Имя документа>",
        ));

        let members = adapter
            .members(
                &owner,
                MemberQuery {
                    name: Some("Поле"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("member lookup must not fail");

        let types = &members.facts[0].info.types;
        assert_eq!(types.len(), 4);
        assert_eq!(
            types[0].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:ДокументСсылка.<Имя документа>",
            )))
        );
        assert_eq!(
            types[0].resolved_id().map(|id| id.0.local_id.as_str()),
            Some("platform_type:ДокументСсылка.<Имя документа>")
        );
        let binding = types[0]
            .template_binding
            .as_ref()
            .expect("resolved template target must carry owner-parameter binding");
        assert_eq!(
            binding.template_key,
            PlatformTypeTemplateKey::new("Document", "Ref")
        );
        assert_eq!(
            binding.arguments,
            vec![TemplateParameterBinding::OwnerParameter {
                owner_parameter_index: 0,
                target_parameter_index: 0,
            }]
        );
        assert_eq!(
            types[1].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:РазрешенныйТип",
            )))
        );
        assert_eq!(types[1].template_binding, None);
        assert_eq!(types[2].target, TypeRefTarget::Unresolved);
        assert_eq!(types[2].template_binding, None);
        assert_eq!(
            types[3].target,
            TypeRefTarget::Ambiguous(vec![
                TypeId(FactId::new(
                    source.clone(),
                    LanguageDomain::PlatformApi,
                    FactKind::Type,
                    "platform_type:ДубльТип:Второй",
                )),
                TypeId(FactId::new(
                    source,
                    LanguageDomain::PlatformApi,
                    FactKind::Type,
                    "platform_type:ДубльТип:Первый",
                )),
            ])
        );
        assert_eq!(types[3].template_binding, None);
    }

    #[test]
    fn platform_adapter_does_not_expose_query_table_documents() {
        let source = fixture_source();
        let index = fixture_index("platform-adapter-query-table-hidden.sqlite");
        for id in [
            "query_table:ОсновнаяТаблица",
            "query_table_field:query_table:ОсновнаяТаблица:Период",
            "query_table_parameter:query_table:ОсновнаяТаблица:Дата",
        ] {
            assert!(
                index
                    .get_by_id(id)
                    .expect("search provider id lookup must not fail")
                    .is_some(),
                "{id} must stay available as a search/provider document"
            );
        }
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());

        for (kind, local_id) in [
            (FactKind::QueryTable, "query_table:ОсновнаяТаблица"),
            (
                FactKind::QueryField,
                "query_table_field:query_table:ОсновнаяТаблица:Период",
            ),
            (
                FactKind::QueryParameter,
                "query_table_parameter:query_table:ОсновнаяТаблица:Дата",
            ),
        ] {
            let fact = FactId::new(source.clone(), LanguageDomain::PlatformApi, kind, local_id);
            let response = adapter
                .resolve(
                    context_resolver_core::ResolveQuery::Id(&fact),
                    &ResolveContext::all(),
                )
                .expect("query_table* id lookup must not fail");

            assert_eq!(response.status, ResolveStatus::NotFound);
        }
    }

    #[test]
    fn platform_adapter_does_not_synthesize_constructor_return_from_owner() {
        let source = fixture_source();
        let filter = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ОтборКомпоновкиДанных",
        ));
        let index = fixture_index_without_constructor_result(
            "platform-adapter-missing-constructor-result.sqlite",
        );
        let adapter = PlatformSearchSource::with_source_id(index, source);

        let constructor = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Новый ОтборКомпоновкиДанных()",
                },
                &ResolveContext::all(),
            )
            .expect("constructor lookup must not fail");

        assert_eq!(constructor.status, ResolveStatus::Ok);
        assert_eq!(constructor.facts.len(), 1);
        assert!(
            constructor.facts[0].info.return_types.is_empty(),
            "constructor return type must require explicit return/constructs evidence"
        );

        let constructs = adapter
            .related(
                &constructor.facts[0].id.0,
                RelationKind::Constructs,
                &ResolveContext::all(),
            )
            .expect("constructs traversal must not fail");
        assert_eq!(constructs.status, ResolveStatus::Ok);
        assert!(
            constructs.facts.is_empty(),
            "constructs traversal must require edge-specific source evidence"
        );
    }

    #[test]
    fn language_adapter_opens_read_only_index_from_path() {
        let path = language_fixture_index("language-adapter-open-read-only.sqlite");
        let source = SourceId::new("shlang");
        let adapter = LanguageSearchSource::open_shlang_read_only(&path)
            .expect("language adapter must open index path");

        assert_eq!(adapter.descriptor().id, source.clone());

        let response = adapter
            .resolve_type(
                TypeLookup::ExactName {
                    source: Some(&source),
                    domain: Some(LanguageDomain::BslLanguage),
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("type lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(response.facts.len(), 1);
    }

    #[test]
    fn language_adapter_preserves_domain_identity_and_ambiguity() {
        let path = language_fixture_index("language-resolver-ambiguity.sqlite");
        let shlang = SourceId::new("shlang");
        let shquery = SourceId::new("shquery");
        let dcsui = SourceId::new("dcsui");
        let resolver = CompositeResolver::new(vec![
            Box::new(LanguageSearchSource::shlang(open_index(&path))),
            Box::new(LanguageSearchSource::shquery(open_index(&path))),
            Box::new(LanguageSearchSource::dcsui(open_index(&path))),
        ]);

        let ambiguous = resolver
            .resolve(
                context_resolver_core::ResolveQuery::ExactName {
                    source: None,
                    domain: None,
                    kind: None,
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("language fact lookup must not fail");
        assert_eq!(ambiguous.status, ResolveStatus::Ambiguous);
        let ambiguous_ids = ambiguous
            .candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.id.source.as_str().to_string(),
                    candidate.id.local_id.as_str().to_string(),
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(ambiguous_ids.contains(&("shlang".to_string(), "def_String".to_string())));
        assert!(ambiguous_ids.contains(&("shquery".to_string(), "STRING".to_string())));
        assert!(ambiguous_ids.contains(&("shquery".to_string(), "LitString".to_string())));

        let ambiguous_types = resolver
            .resolve_type(
                TypeLookup::ExactName {
                    source: None,
                    domain: None,
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("language type lookup must not fail");
        assert_eq!(ambiguous_types.status, ResolveStatus::Ambiguous);
        let ambiguous_type_ids = ambiguous_types
            .candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.id.source.as_str().to_string(),
                    candidate.id.local_id.as_str().to_string(),
                )
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(ambiguous_type_ids.contains(&("shlang".to_string(), "def_String".to_string())));
        assert!(ambiguous_type_ids.contains(&("shquery".to_string(), "LitString".to_string())));

        let started = Instant::now();
        let bsl_string = resolver
            .resolve_type(
                TypeLookup::ExactName {
                    source: Some(&shlang),
                    domain: Some(LanguageDomain::BslLanguage),
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("constrained BSL lookup must not fail");
        assert_eq!(bsl_string.status, ResolveStatus::Ok);
        assert_eq!(bsl_string.facts[0].id.0.local_id, "def_String");
        assert_eq!(bsl_string.facts[0].id.0.domain, LanguageDomain::BslLanguage);
        assert!(started.elapsed().as_millis() < 100);

        let query_string = CallableId(FactId::new(
            shquery.clone(),
            LanguageDomain::QueryLanguage,
            FactKind::Callable,
            "STRING",
        ));
        let query_function = resolver
            .callable(CallableLookup::Id(&query_string), &ResolveContext::all())
            .expect("query function lookup must not fail");
        assert_eq!(query_function.status, ResolveStatus::Ok);
        assert_eq!(query_function.facts[0].id.0.source, shquery);
        assert_eq!(query_function.facts[0].fact.name.primary, "СТРОКА");

        let query_literal = TypeId(FactId::new(
            SourceId::new("shquery"),
            LanguageDomain::QueryLanguage,
            FactKind::Type,
            "LitString",
        ));
        let literal = resolver
            .resolve_type(TypeLookup::Id(&query_literal), &ResolveContext::all())
            .expect("query literal lookup must not fail");
        assert_eq!(literal.status, ResolveStatus::Ok);
        assert_eq!(literal.facts[0].id.0.local_id, "LitString");

        let skd_string_length = CallableId(FactId::new(
            dcsui,
            LanguageDomain::QueryLanguage,
            FactKind::Callable,
            "SKD_Functions_Strings#StringLength",
        ));
        let skd_function = resolver
            .callable(
                CallableLookup::Id(&skd_string_length),
                &ResolveContext::all(),
            )
            .expect("SKD function lookup must not fail");
        assert_eq!(skd_function.status, ResolveStatus::Ok);
        assert_eq!(skd_function.facts[0].fact.name.primary, "ДлинаСтроки");
        assert_eq!(
            skd_function.facts[0].info.signatures[0].parameters[0].name,
            "Строка"
        );
    }

    #[test]
    fn language_adapter_exposes_bsl_and_sdbl_global_contexts_separately() {
        let path = language_fixture_index("language-global-context.sqlite");
        let shlang = LanguageSearchSource::shlang(open_index(&path));
        let shquery = LanguageSearchSource::shquery(open_index(&path));

        let bsl_scope = shlang
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Bsl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("BSL global context lookup must not fail");
        assert_eq!(bsl_scope.status, ResolveStatus::Ok);
        assert!(bsl_scope.facts[0].facts.iter().any(|fact| {
            fact.id.domain == LanguageDomain::BslLanguage && fact.id.local_id == "def_String"
        }));
        assert!(
            bsl_scope.facts[0]
                .facts
                .iter()
                .all(|fact| fact.id.domain != LanguageDomain::QueryLanguage)
        );

        let sdbl_scope = shquery
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Sdbl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("SDBL global context lookup must not fail");
        assert_eq!(sdbl_scope.status, ResolveStatus::Ok);
        assert!(sdbl_scope.facts[0].methods.iter().any(|method| {
            method.id.0.domain == LanguageDomain::QueryLanguage && method.id.0.local_id == "STRING"
        }));
        assert!(
            sdbl_scope.facts[0]
                .facts
                .iter()
                .all(|fact| fact.id.domain == LanguageDomain::QueryLanguage)
        );
    }

    #[test]
    fn language_adapter_traverses_only_explicit_extracted_type_edges() {
        let path = language_fixture_index("language-resolver-relations.sqlite");
        let shquery = SourceId::new("shquery");
        let dcsui = SourceId::new("dcsui");
        let query_adapter = LanguageSearchSource::shquery(open_index(&path));
        let adapter = LanguageSearchSource::dcsui(open_index(&path));
        let query_string = FactId::new(
            shquery,
            LanguageDomain::QueryLanguage,
            FactKind::Callable,
            "STRING",
        );
        let string_length = FactId::new(
            dcsui,
            LanguageDomain::QueryLanguage,
            FactKind::Callable,
            "SKD_Functions_Strings#StringLength",
        );

        let query_return = query_adapter
            .related(&query_string, RelationKind::Returns, &ResolveContext::all())
            .expect("query return traversal must not fail");
        assert_eq!(query_return.status, ResolveStatus::Ok);
        assert!(
            query_return.facts.iter().any(|fact| {
                fact.id.source.as_str() == "shquery"
                    && fact.id.domain == LanguageDomain::QueryLanguage
                    && fact.id.local_id == "LitString"
            }),
            "query STRING return must use the explicit query-language string literal/type edge"
        );
        assert!(
            query_return
                .facts
                .iter()
                .all(|fact| fact.id.source.as_str() != "shlang"),
            "query STRING return must not choose the BSL string type by same-name lookup"
        );

        let started = Instant::now();
        let related = adapter
            .related(
                &string_length,
                RelationKind::HasType,
                &ResolveContext::all(),
            )
            .expect("language relation traversal must not fail");
        assert!(started.elapsed().as_millis() < 100);
        assert_eq!(related.status, ResolveStatus::Ok);
        assert!(
            related.facts.iter().any(|fact| {
                fact.id.source.as_str() == "shlang"
                    && fact.id.domain == LanguageDomain::BslLanguage
                    && fact.id.local_id == "def_String"
            }),
            "SKD parameter type must traverse to the explicit BSL string type edge"
        );
    }

    fn fixture_source() -> SourceId {
        SourceId::new("test-platform")
    }

    fn fixture_index(file_name: &str) -> SearchIndex {
        let path = fixture_index_path(file_name);
        SearchIndex::open_read_only(path).expect("index must open")
    }

    fn fixture_index_without_constructor_result(file_name: &str) -> SearchIndex {
        let path = fixture_index_path(file_name);
        let connection = rusqlite::Connection::open(&path).expect("index must open for mutation");
        let constructor_id =
            "constructor:platform_type:ОтборКомпоновкиДанных:Новый ОтборКомпоновкиДанных()";
        connection
            .execute(
                "DELETE FROM type_refs WHERE source_document_id = ?1 AND ref_kind = 'constructor_result'",
                [constructor_id],
            )
            .expect("constructor result type ref must be removable");
        connection
            .execute(
                "DELETE FROM relations WHERE source_id = ?1 AND edge_kind = 'constructs'",
                [constructor_id],
            )
            .expect("constructor relation must be removable");
        drop(connection);
        SearchIndex::open_read_only(path).expect("index must open")
    }

    fn fixture_index_with_signature_only_return(file_name: &str) -> SearchIndex {
        let path = fixture_index_path(file_name);
        let connection = rusqlite::Connection::open(&path).expect("index must open for mutation");
        let method_id = "type_method:platform_type:ОтборКомпоновкиДанных:Найти";
        let signature_id: String = connection
            .query_row(
                "SELECT signature_id FROM signatures WHERE callable_id = ?1 ORDER BY ordinal LIMIT 1",
                [method_id],
                |row| row.get(0),
            )
            .expect("fixture method signature must exist");
        connection
            .execute(
                "UPDATE type_refs
                 SET source_signature_id = ?1,
                     source_signature_ordinal = 0
                 WHERE source_document_id = ?2
                   AND ref_kind = 'return_type'",
                rusqlite::params![signature_id, method_id],
            )
            .expect("fixture return type must become signature-scoped");
        drop(connection);
        SearchIndex::open_read_only(path).expect("index must open")
    }

    fn fixture_index_path(file_name: &str) -> PathBuf {
        let path = temp_path(file_name);
        let mut builder = SearchIndexBuilder::new();
        for record in [
            platform_type("НастройкиКомпоновкиДанных", None),
            platform_type("ОтборКомпоновкиДанных", Some("DataCompositionFilter")),
            platform_template_type(
                "СправочникМенеджер.<Имя справочника>",
                "CatalogManager.<Catalog name>",
                "СправочникМенеджер",
                "Имя справочника",
            ),
            platform_template_type(
                "ДокументМенеджер.<Имя документа>",
                "DocumentManager.<Document name>",
                "ДокументМенеджер",
                "Имя документа",
            ),
            platform_template_type(
                "ДокументОбъект.<Имя документа>",
                "DocumentObject.<Document name>",
                "ДокументОбъект",
                "Имя документа",
            ),
            platform_template_type(
                "ДокументСсылка.<Имя документа>",
                "DocumentRef.<Document name>",
                "ДокументСсылка",
                "Имя документа",
            ),
            platform_type("КоллекцияЭлементовОтбораКомпоновкиДанных", None),
            platform_type("ЭлементОтбораКомпоновкиДанных", None),
        ] {
            builder
                .platform_type(record)
                .expect("platform type must sink");
        }
        builder
            .type_property(model::PlatformProperty {
                owner: name("НастройкиКомпоновкиДанных", None),
                owner_identity: Some("platform_type:НастройкиКомпоновкиДанных".to_string()),
                name: name("Отбор", None),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ОтборКомпоновкиДанных".to_string(),
                }],
                description: Some("Фильтр настроек.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("settings-filter"),
            })
            .expect("property must sink");
        builder
            .type_property(model::PlatformProperty {
                owner: name("ОтборКомпоновкиДанных", None),
                owner_identity: Some("platform_type:ОтборКомпоновкиДанных".to_string()),
                name: name("Элементы", None),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "КоллекцияЭлементовОтбораКомпоновкиДанных".to_string(),
                }],
                description: Some("Элементы фильтра.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("filter-items"),
            })
            .expect("property must sink");
        builder
            .type_method(model::PlatformMethod {
                owner: name("ОтборКомпоновкиДанных", None),
                owner_identity: Some("platform_type:ОтборКомпоновкиДанных".to_string()),
                name: name("Найти", None),
                semantic: model::SemanticContext::default(),
                signatures: vec![model::Signature {
                    text: "Найти(<Значение>)".to_string(),
                    parameters: vec![model::Parameter {
                        name: "Значение".to_string(),
                        required: true,
                        type_refs: vec![model::TypeRef {
                            name: "ОтборКомпоновкиДанных".to_string(),
                        }],
                        description: None,
                    }],
                    return_types: Vec::new(),
                    variant: None,
                }],
                return_types: vec![model::TypeRef {
                    name: "ЭлементОтбораКомпоновкиДанных".to_string(),
                }],
                description: Some("Ищет элемент.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("filter-find"),
            })
            .expect("method must sink");
        builder
            .global_method(model::GlobalMethod {
                name: name("Сообщить", Some("Message")),
                signatures: vec![model::Signature {
                    text: "Сообщить(<Сообщение>)".to_string(),
                    parameters: vec![model::Parameter {
                        name: "Сообщение".to_string(),
                        required: true,
                        type_refs: vec![model::TypeRef {
                            name: "Строка".to_string(),
                        }],
                        description: None,
                    }],
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
            .global_property(model::GlobalProperty {
                name: name("ТекущийОтбор", Some("CurrentFilter")),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "ОтборКомпоновкиДанных".to_string(),
                }],
                description: Some("Тестовое глобальное свойство.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("global-current-filter"),
            })
            .expect("global property must sink");
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
                source: source_ref("document-object-ref"),
            })
            .expect("type-template property must sink");
        builder
            .constructor(model::Constructor {
                owner: name("ОтборКомпоновкиДанных", None),
                owner_identity: Some("platform_type:ОтборКомпоновкиДанных".to_string()),
                name: name("Новый ОтборКомпоновкиДанных()", None),
                semantic: model::SemanticContext::default(),
                signatures: vec![model::Signature {
                    text: "Новый ОтборКомпоновкиДанных()".to_string(),
                    parameters: Vec::new(),
                    return_types: Vec::new(),
                    variant: None,
                }],
                description: Some("Создает фильтр.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("filter-constructor"),
            })
            .expect("constructor must sink");
        builder
            .constructor(model::Constructor {
                owner: name(
                    "ДокументОбъект.<Имя документа>",
                    Some("DocumentObject.<Document name>"),
                ),
                owner_identity: Some("platform_type:ДокументОбъект.<Имя документа>".to_string()),
                name: name(
                    "Новый ДокументОбъект.<Имя документа>()",
                    Some("New DocumentObject.<Document name>()"),
                ),
                semantic: model::SemanticContext::default(),
                signatures: vec![model::Signature {
                    text: "Новый ДокументОбъект.<Имя документа>()".to_string(),
                    parameters: Vec::new(),
                    return_types: Vec::new(),
                    variant: None,
                }],
                description: Some("Creates document object.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("document-object-constructor"),
            })
            .expect("type-template constructor must sink");
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:ОсновнаяТаблица".to_string()),
                name: "Основная таблица".to_string(),
                syntax: Some(name("ОсновнаяТаблица", None)),
                identifier: Some("ОсновнаяТаблица".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Primary,
                description: Some("Query provider fact.".to_string()),
                source: source_ref("query-table"),
            })
            .expect("query table must sink");
        builder
            .table_field(model::QueryTableField {
                owner: name("ОсновнаяТаблица", None),
                owner_identity: Some("query_table:ОсновнаяТаблица".to_string()),
                name: "Период".to_string(),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTableField,
                ),
                type_refs: vec![model::TypeRef {
                    name: "Дата".to_string(),
                }],
                description: Some("Query field provider fact.".to_string()),
                note: None,
                source: source_ref("query-table-field"),
            })
            .expect("query table field must sink");
        builder
            .table_parameter(model::QueryTableParameter {
                owner: name("ОсновнаяТаблица", None),
                owner_identity: Some("query_table:ОсновнаяТаблица".to_string()),
                name: "Дата".to_string(),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTableParameter,
                ),
                type_refs: vec![model::TypeRef {
                    name: "Дата".to_string(),
                }],
                description: Some("Query parameter provider fact.".to_string()),
                default_value: None,
                source: source_ref("query-table-parameter"),
            })
            .expect("query table parameter must sink");
        builder
            .global_context_event(type_event("ОтборКомпоновкиДанных", "ПередЗаписью"))
            .expect("type event must sink");
        builder
            .global_context_event(module_event(
                model::ModuleKind::Form,
                &["Форма"],
                "ПриОткрытии",
                "OnOpen",
            ))
            .expect("module event must sink");
        build_index_from_builder(&path, &metadata(), builder).expect("index must build");
        path
    }

    fn platform_type(primary: &str, alias: Option<&str>) -> model::PlatformType {
        model::PlatformType {
            identity: None,
            name: name(primary, alias),
            semantic: model::SemanticContext::default(),
            type_kind: model::PlatformTypeKind::Regular,
            object_kind: None,
            extends: Vec::new(),
            metadata_kind: None,
            template_parameters: Vec::new(),
            type_template_key: None,
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: Some(format!("{primary} description.")),
            facts: model::SectionFacts::default(),
            source: source_ref(primary),
        }
    }

    fn platform_type_with_owner_path(primary: &str, owner_path: &str) -> model::PlatformType {
        let mut record = platform_type(primary, None);
        record.semantic = model::SemanticContext::new(
            model::BranchKind::PlatformObjects,
            model::RecordFamily::PlatformType,
        )
        .with_owner_path(vec![name(owner_path, None)]);
        record
    }

    fn platform_template_type(
        primary: &str,
        alias: &str,
        metadata_kind: &str,
        template_parameter: &str,
    ) -> model::PlatformType {
        let mut record = platform_type(primary, Some(alias));
        record.type_kind = model::PlatformTypeKind::MetadataTemplate;
        record.metadata_kind = Some(metadata_kind.to_string());
        record.template_parameters = vec![template_parameter.to_string()];
        record
    }

    fn type_event(owner: &str, primary: &str) -> model::GlobalContextEvent {
        type_event_with_owner_path(&[owner], primary)
    }

    fn module_event(
        kind: model::ModuleKind,
        owner_path: &[&str],
        primary: &str,
        alias: &str,
    ) -> model::GlobalContextEvent {
        model::GlobalContextEvent {
            name: name(primary, Some(alias)),
            owner_identity: None,
            semantic: model::SemanticContext::new(
                model::BranchKind::ManagedForms,
                model::RecordFamily::ModuleEvent,
            )
            .with_owner_path(owner_path.iter().map(|owner| name(owner, None)).collect()),
            module: model::ModuleEventContext {
                kind,
                owner_path: owner_path.iter().map(|owner| name(owner, None)).collect(),
            },
            signatures: vec![model::Signature {
                text: format!("{primary}(<Отказ>)"),
                parameters: vec![model::Parameter {
                    name: "Отказ".to_string(),
                    required: true,
                    type_refs: Vec::new(),
                    description: None,
                }],
                return_types: Vec::new(),
                variant: None,
            }],
            description: Some("module event description".to_string()),
            facts: model::SectionFacts {
                availability: model::Availability {
                    contexts: Vec::new(),
                },
                examples: Vec::new(),
                see_also: Vec::new(),
                available_since: Some(model::VersionFact {
                    version: Some("8.3.1".to_string()),
                    text: "Available since version 8.3.1.".to_string(),
                }),
            },
            source: source_ref(&format!("module-event-{primary}")),
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
            source: source_ref(&format!("{}.{}", owner_path.join("."), primary)),
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
            source_hbk: "/fixtures/shcntx_ru.hbk".to_string(),
            source_extraction_schema_version: 11,
        }
    }

    fn language_fixture_index(file_name: &str) -> PathBuf {
        let path = temp_path(file_name);
        let mut builder = SearchIndexBuilder::new();
        for fact in language_fixture_facts() {
            builder.add_language_fact(fact);
        }
        build_index_from_builder(&path, &metadata(), builder).expect("language index must build");
        path
    }

    fn language_fixture_facts() -> Vec<syntax_helper_language::LanguageFact> {
        [
            (
                LanguageSourceFamily::Shlang,
                "def_String",
                "shlang_def_string_ru.html",
            ),
            (
                LanguageSourceFamily::Shquery,
                "STRING",
                "shquery_string_ru.html",
            ),
            (
                LanguageSourceFamily::Shquery,
                "LitString",
                "shquery_lit_string_ru.html",
            ),
            (
                LanguageSourceFamily::Dcsui,
                "SKD_Functions_Strings",
                "dcsui_functions_strings_ru.html",
            ),
        ]
        .into_iter()
        .flat_map(|(source_family, html_path, fixture_name)| {
            let html = std::fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../tests/fixtures/syntax-helper-language")
                    .join(fixture_name),
            )
            .expect("language fixture must be readable");
            extract_language_facts(LanguagePageInput {
                source_hbk: "fixture.hbk",
                source_family,
                locale: "ru",
                html_path,
                html: &html,
            })
        })
        .collect()
    }

    fn open_index(path: &std::path::Path) -> SearchIndex {
        SearchIndex::open_read_only(path).expect("index must open")
    }

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "v8-context-hbk-context-resolver-search-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }
}
