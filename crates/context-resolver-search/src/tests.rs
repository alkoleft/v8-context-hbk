#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Instant;

    use context_resolver_core::{
        CallableLookup, CompositeResolver, ContextResolver, ContextSource, GlobalContextLanguage,
        GlobalContextQuery, MemberQuery, PlatformTypeTemplateKey, RelationKind, ResolveContext,
        ResolveError, ResolveStatus, TemplateParameterBinding, TypeLookup,
        WorkerSafeCompositeResolver,
    };
    use syntax_helper_language::{LanguagePageInput, LanguageSourceFamily, extract_language_facts};
    use syntax_helper_model as model;
    use syntax_helper_model::SyntaxHelperSink;
    use syntax_helper_search::{
        HbkFactSnapshot, HbkFactSnapshotCacheStatus, IndexMetadata, SearchIndexBuilder,
        build_index_from_builder,
    };

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
    fn platform_adapter_resolves_generated_self_role_selector() {
        let source = fixture_source();
        let index = fixture_index("platform-adapter-generated-self-selector.sqlite");
        let adapter = PlatformSearchSource::with_source_id(index, source.clone());

        let response = adapter
            .resolve_type(
                TypeLookup::GeneratedSelfTemplate {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    generated_self_role: "metadata.generated-self.catalog-manager",
                },
                &ResolveContext::all(),
            )
            .expect("generated-self template lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(
            response.facts[0].info.type_template_key,
            Some(PlatformTypeTemplateKey::new("Catalog", "Manager"))
        );
    }

    #[test]
    fn platform_adapter_resolves_every_certified_generated_self_selector() {
        let source = fixture_source();
        let adapter = PlatformSearchSource::with_source_id(
            generated_self_selector_index(
                "platform-adapter-generated-self-selector-corpus.sqlite",
                false,
            ),
            source.clone(),
        );

        for (selector, family, variant) in generated_self_selector_records() {
            let response = adapter
                .resolve_type(
                    TypeLookup::GeneratedSelfTemplate {
                        source: Some(&source),
                        domain: Some(LanguageDomain::PlatformApi),
                        generated_self_role: selector,
                    },
                    &ResolveContext::all(),
                )
                .expect("generated-self template lookup must not fail");

            assert_eq!(response.status, ResolveStatus::Ok, "{selector}");
            assert_eq!(response.facts.len(), 1, "{selector}");
            assert_eq!(
                response.facts[0].info.type_template_key,
                Some(PlatformTypeTemplateKey::new(family, variant)),
                "{selector}"
            );
            assert_eq!(response.facts[0].id.0.source, source, "{selector}");
            assert_eq!(
                response.facts[0].id.0.domain,
                LanguageDomain::PlatformApi,
                "{selector}"
            );
        }
    }

    #[test]
    fn platform_snapshot_resolves_every_certified_generated_self_selector() {
        let source = fixture_source();
        let index = generated_self_selector_index(
            "platform-snapshot-generated-self-selector-corpus.sqlite",
            false,
        );
        let snapshot = Arc::new(
            HbkFactSnapshot::from_index(&index)
                .expect("generated-self selector snapshot must materialize"),
        );
        let adapter = PlatformSnapshotSource::with_source_id(snapshot, source.clone());

        for (selector, family, variant) in generated_self_selector_records() {
            let response = adapter
                .resolve_type(
                    TypeLookup::GeneratedSelfTemplate {
                        source: Some(&source),
                        domain: Some(LanguageDomain::PlatformApi),
                        generated_self_role: selector,
                    },
                    &ResolveContext::all(),
                )
                .expect("snapshot generated-self template lookup must not fail");

            assert_eq!(response.status, ResolveStatus::Ok, "{selector}");
            assert_eq!(response.facts.len(), 1, "{selector}");
            assert_eq!(
                response.facts[0].info.type_template_key,
                Some(PlatformTypeTemplateKey::new(family, variant)),
                "{selector}"
            );
            assert_eq!(response.facts[0].id.0.source, source, "{selector}");
            assert_eq!(
                response.facts[0].id.0.domain,
                LanguageDomain::PlatformApi,
                "{selector}"
            );
        }
    }

    #[test]
    fn generated_self_selector_respects_routing_and_failure_contracts() {
        let source = fixture_source();
        let adapter = PlatformSearchSource::with_source_id(
            generated_self_selector_index(
                "platform-adapter-generated-self-selector-statuses.sqlite",
                false,
            ),
            source.clone(),
        );
        let query = |source, domain, generated_self_role| TypeLookup::GeneratedSelfTemplate {
            source,
            domain,
            generated_self_role,
        };

        let wrong_source = SourceId::new("another-platform");
        assert_eq!(
            adapter
                .resolve_type(
                    query(
                        Some(&wrong_source),
                        Some(LanguageDomain::PlatformApi),
                        "metadata.generated-self.catalog-manager",
                    ),
                    &ResolveContext::all(),
                )
                .expect("wrong-source lookup must not fail")
                .status,
            ResolveStatus::NotFound
        );
        assert_eq!(
            adapter
                .resolve_type(
                    query(
                        Some(&source),
                        Some(LanguageDomain::BslLanguage),
                        "metadata.generated-self.catalog-manager",
                    ),
                    &ResolveContext::all(),
                )
                .expect("wrong-domain lookup must not fail")
                .status,
            ResolveStatus::NotFound
        );
        assert_eq!(
            adapter
                .resolve_type(
                    query(
                        Some(&source),
                        Some(LanguageDomain::PlatformApi),
                        "metadata.generated-self.unknown",
                    ),
                    &ResolveContext::all(),
                )
                .expect("unknown-selector lookup must not fail")
                .status,
            ResolveStatus::NotFound
        );

        let language = LanguageSearchSource::shlang(open_index(&language_fixture_index(
            "language-generated-self-selector-unsupported.sqlite",
        )));
        assert_eq!(
            language
                .resolve_type(
                    query(
                        None,
                        None,
                        "metadata.generated-self.catalog-manager",
                    ),
                    &ResolveContext::all(),
                )
                .expect("unsupported-source lookup must not fail")
                .status,
            ResolveStatus::Unsupported
        );

        let ambiguous = PlatformSearchSource::with_source_id(
            generated_self_selector_index(
                "platform-adapter-generated-self-selector-ambiguous.sqlite",
                true,
            ),
            source.clone(),
        )
        .resolve_type(
            query(
                Some(&source),
                Some(LanguageDomain::PlatformApi),
                "metadata.generated-self.catalog-manager",
            ),
            &ResolveContext::all(),
        )
        .expect("ambiguous selector lookup must not fail");
        assert_eq!(ambiguous.status, ResolveStatus::Ambiguous);

        let path = generated_self_selector_index_path(
            "platform-adapter-generated-self-selector-error.sqlite",
            false,
        );
        let failing = PlatformSearchSource::with_source_id(open_index(&path), source.clone());
        rusqlite::Connection::open(&path)
            .expect("fixture database must open for mutation")
            .execute_batch("DROP TABLE type_templates")
            .expect("fixture template table must be removable");
        assert!(matches!(
            failing.resolve_type(
                query(
                    Some(&source),
                    Some(LanguageDomain::PlatformApi),
                    "metadata.generated-self.catalog-manager",
                ),
                &ResolveContext::all(),
            ),
            Err(ResolveError::SourceFailure { .. })
        ));
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
        let enum_return_callable = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: None,
                    name: "ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
                },
                &ResolveContext::all(),
            )
            .expect("enum-return callable lookup must not fail");
        let enum_returns = adapter
            .related(
                &enum_return_callable.facts[0].id.0,
                RelationKind::Returns,
                &ResolveContext::all(),
            )
            .expect("enum returns traversal must not fail");
        assert_eq!(enum_returns.status, ResolveStatus::Ok);
        assert_eq!(enum_returns.facts.len(), 1);
        assert_eq!(enum_returns.facts[0].id.kind, FactKind::Type);
        assert_eq!(
            enum_returns.facts[0].id.local_id,
            "enum:system:ОбновлениеПредопределенныхДанных"
        );
        assert!(matches!(enum_returns.facts[0].details, FactDetails::Type(_)));

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
        let constructors = adapter
            .constructors(&filter, &ResolveContext::all())
            .expect("constructor list lookup must not fail");
        assert_eq!(constructors.status, ResolveStatus::Ok);
        assert_eq!(constructors.facts.len(), 1);
        assert_eq!(
            constructors.facts[0].fact.name.primary,
            "Новый ОтборКомпоновкиДанных()"
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

        let enum_value_id = FactId::new(
            source,
            LanguageDomain::PlatformApi,
            FactKind::EnumValue,
            "enum_value:enum:system:ОбновлениеПредопределенныхДанных:Обновлять",
        );
        let enum_member_of = adapter
            .related(&enum_value_id, RelationKind::MemberOf, &ResolveContext::all())
            .expect("enum value member_of traversal must not fail");
        assert_eq!(enum_member_of.status, ResolveStatus::Ok);
        assert_eq!(enum_member_of.facts.len(), 1);
        assert_eq!(enum_member_of.facts[0].id.kind, FactKind::Enum);
        assert_eq!(
            enum_member_of.facts[0].id.local_id,
            "enum:system:ОбновлениеПредопределенныхДанных"
        );
        assert!(matches!(
            enum_member_of.facts[0].details,
            FactDetails::Enum
        ));
        assert_eq!(
            enum_member_of.facts[0].relations[0].target.kind,
            FactKind::Enum
        );
    }

    #[test]
    fn platform_snapshot_source_resolves_hot_paths_without_search_index_backend() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PlatformSnapshotSource>();
        assert_send_sync::<WorkerSafeCompositeResolver>();

        let source = fixture_source();
        let index_path = fixture_index_path("platform-snapshot-source.sqlite");
        let cache_path = temp_path("platform-snapshot-source.bin");
        let source_report = HbkFactSnapshot::from_path_with_stage_timings(&index_path)
            .expect("snapshot must build");
        source_report
            .write_binary_cache(&cache_path)
            .expect("snapshot cache must write");
        let cached_report = HbkFactSnapshot::from_path_with_binary_cache(&index_path, &cache_path)
            .expect("snapshot cache must load");
        assert_eq!(cached_report.status, HbkFactSnapshotCacheStatus::Loaded);
        let snapshot = Arc::new(cached_report.snapshot);
        std::fs::remove_file(&index_path).expect("snapshot adapter must not need SQLite file");
        let adapter = PlatformSnapshotSource::with_source_id(snapshot.clone(), source.clone());
        let query_source = SourceId::new("shcntx-query");
        let resolver = WorkerSafeCompositeResolver::new(vec![
            Box::new(PlatformSnapshotSource::with_source_id(
                snapshot.clone(),
                source.clone(),
            )),
            Box::new(QueryTableSnapshotSource::with_source_ids(
                snapshot.clone(),
                query_source,
                source.clone(),
            )),
        ]);
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

        let type_response = adapter
            .resolve_type(TypeLookup::Id(&filter), &ResolveContext::all())
            .expect("snapshot type lookup must not fail");
        assert_eq!(type_response.status, ResolveStatus::Ok);
        let resolver_type_response = resolver
            .resolve_type(TypeLookup::Id(&filter), &ResolveContext::all())
            .expect("snapshot resolver composition must not fail");
        assert_eq!(resolver_type_response.status, ResolveStatus::Ok);
        assert_eq!(
            type_response.facts[0].fact.name.primary,
            "ОтборКомпоновкиДанных"
        );
        let template_response = adapter
            .resolve_type(
                TypeLookup::PlatformTypeTemplate {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    key: &PlatformTypeTemplateKey::new("Catalog", "Manager"),
                },
                &ResolveContext::all(),
            )
            .expect("snapshot type-template lookup must not fail");
        assert_eq!(template_response.status, ResolveStatus::Ok);
        let template = template_response
            .facts
            .first()
            .expect("template type must resolve");
        assert_eq!(
            template
                .info
                .metadata_template
                .as_ref()
                .expect("snapshot template metadata must be exposed")
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
        let generated_self_response = adapter
            .resolve_type(
                TypeLookup::GeneratedSelfTemplate {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    generated_self_role: "metadata.generated-self.catalog-manager",
                },
                &ResolveContext::all(),
            )
            .expect("snapshot generated-self template lookup must not fail");
        assert_eq!(generated_self_response.status, ResolveStatus::Ok);
        assert_eq!(generated_self_response.facts[0].id, template.id);

        let filter_member = adapter
            .members(
                &settings,
                MemberQuery {
                    name: Some("Отбор"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("snapshot member lookup must not fail");
        assert_eq!(filter_member.status, ResolveStatus::Ok);
        assert_eq!(filter_member.facts[0].owner, settings);

        let has_type = adapter
            .related(
                &filter_member.facts[0].id.0,
                RelationKind::HasType,
                &ResolveContext::all(),
            )
            .expect("snapshot has_type traversal must not fail");
        assert_eq!(
            has_type.facts[0].id.local_id,
            "platform_type:ОтборКомпоновкиДанных"
        );

        let callable = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Найти",
                },
                &ResolveContext::all(),
            )
            .expect("snapshot callable lookup must not fail");
        assert_eq!(callable.status, ResolveStatus::Ok);
        assert_eq!(
            callable.facts[0].info.signatures[0].parameters[0].name,
            "Значение"
        );
        assert_eq!(
            callable.facts[0].info.return_types[0].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:ЭлементОтбораКомпоновкиДанных",
            )))
        );

        let constructor = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Новый ОтборКомпоновкиДанных()",
                },
                &ResolveContext::all(),
            )
            .expect("snapshot constructor lookup must not fail");
        assert_eq!(constructor.status, ResolveStatus::Ok);

        let enum_return_callable = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: None,
                    name: "ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
                },
                &ResolveContext::all(),
            )
            .expect("snapshot enum-return callable lookup must not fail");
        let enum_returns = adapter
            .related(
                &enum_return_callable.facts[0].id.0,
                RelationKind::Returns,
                &ResolveContext::all(),
            )
            .expect("snapshot enum returns traversal must not fail");
        assert_eq!(enum_returns.status, ResolveStatus::Ok);
        assert_eq!(enum_returns.facts[0].id.kind, FactKind::Type);
        assert_eq!(
            enum_returns.facts[0].id.local_id,
            "enum:system:ОбновлениеПредопределенныхДанных"
        );

        let enum_value_id = FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::EnumValue,
            "enum_value:enum:system:ОбновлениеПредопределенныхДанных:Обновлять",
        );
        let enum_member_of = adapter
            .related(&enum_value_id, RelationKind::MemberOf, &ResolveContext::all())
            .expect("snapshot enum value member_of traversal must not fail");
        assert_eq!(enum_member_of.status, ResolveStatus::Ok);
        assert_eq!(enum_member_of.facts[0].id.kind, FactKind::Enum);

        let scope = adapter
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Bsl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("snapshot global context lookup must not fail");
        assert!(scope.facts[0]
            .methods
            .iter()
            .any(|method| method.fact.name.primary == "Сообщить"));
        assert!(scope.facts[0]
            .properties
            .iter()
            .any(|property| property.name.primary == "ТекущийОтбор"));

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
            .expect("snapshot module context lookup must not fail");
        assert_eq!(context.status, ResolveStatus::Ok);
        assert!(context.facts[0]
            .events
            .iter()
            .any(|event| event.fact.name.primary == "ПриОткрытии"));

        let availability = adapter
            .availability(&filter_member.facts[0].id.0, &ResolveContext::all())
            .expect("snapshot availability lookup must not fail");
        assert_eq!(availability.status, ResolveStatus::Ok);
    }

    #[test]
    fn snapshot_resolver_returns_not_found_for_non_migrated_bsl_language_without_sql_fallback() {
        let platform_source = fixture_source();
        let query_source = SourceId::new("shcntx-query");
        let language_source = SourceId::new("shlang");
        let index_path = fixture_index_path("snapshot-no-bsl-language-fallback.sqlite");
        let index = open_index(&index_path);
        let snapshot = Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must build"));
        drop(index);
        std::fs::remove_file(&index_path).expect("snapshot lookup must not require SQLite file");

        let resolver = WorkerSafeCompositeResolver::new(vec![
            Box::new(PlatformSnapshotSource::with_source_id(
                Arc::clone(&snapshot),
                platform_source.clone(),
            )),
            Box::new(QueryTableSnapshotSource::with_source_ids(
                Arc::clone(&snapshot),
                query_source,
                platform_source,
            )),
        ]);

        let bsl_string = resolver
            .resolve_type(
                TypeLookup::ExactName {
                    source: Some(&language_source),
                    domain: Some(LanguageDomain::BslLanguage),
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("snapshot-only resolver must not fail BSL lookup");
        assert_eq!(bsl_string.status, ResolveStatus::NotFound);
        assert!(
            bsl_string.facts.is_empty(),
            "non-migrated BSL-language facts must not be served by SQL/SearchIndex fallback"
        );

        let bsl_fact = resolver
            .resolve(
                context_resolver_core::ResolveQuery::ExactName {
                    source: Some(&language_source),
                    domain: Some(LanguageDomain::BslLanguage),
                    kind: None,
                    name: "Строка",
                },
                &ResolveContext::all(),
            )
            .expect("snapshot-only resolver must not fail BSL fact lookup");
        assert_eq!(bsl_fact.status, ResolveStatus::NotFound);
        assert!(
            bsl_fact.facts.is_empty(),
            "snapshot composition without LanguageSnapshotSource must not consult LanguageSearchSource"
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
    fn platform_adapter_marks_variadic_signatures() {
        let source = fixture_source();
        let adapter = PlatformSearchSource::with_source_id(
            fixture_index("platform-variadic-signatures.sqlite"),
            source.clone(),
        );

        let min = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: None,
                    name: "Мин",
                },
                &ResolveContext::all(),
            )
            .expect("variadic global method lookup must not fail");

        assert_eq!(min.status, ResolveStatus::Ok);
        assert!(
            min.facts[0].info.signatures[0].variadic,
            "ellipsis signature must be marked variadic"
        );

        let structure = TypeId(FactId::new(
            source,
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:Структура",
        ));
        let constructors = adapter
            .constructors(&structure, &ResolveContext::all())
            .expect("structure constructor lookup must not fail");

        assert_eq!(constructors.status, ResolveStatus::Ok);
        assert!(
            constructors.facts[0].info.signatures[0].variadic,
            "structure keys/values constructor must be marked variadic"
        );
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
    fn query_table_source_exposes_templates_fields_parameters_and_type_refs() {
        let query_source = SourceId::new("shcntx-query");
        let platform_source = fixture_source();
        let index = fixture_index("query-table-source.sqlite");
        let adapter = LanguageSearchSource::new_query_tables(
            query_source.as_str(),
            platform_source.clone(),
            index,
        );
        let capabilities = adapter.capabilities();
        assert!(capabilities.exact_lookup);
        assert!(capabilities.relations);
        assert!(!capabilities.type_lookup);
        assert!(!capabilities.callables);
        assert!(capabilities.global_context);
        let type_lookup = adapter
            .resolve_type(
                TypeLookup::ExactName {
                    source: Some(&query_source),
                    domain: Some(LanguageDomain::QueryLanguage),
                    name: "Дата",
                },
                &ResolveContext::all(),
            )
            .expect("query table type lookup refusal must not fail");
        assert_eq!(type_lookup.status, ResolveStatus::Unsupported);
        let callable_lookup = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: None,
                    name: "Есть",
                },
                &ResolveContext::all(),
            )
            .expect("query table callable lookup refusal must not fail");
        assert_eq!(callable_lookup.status, ResolveStatus::Unsupported);
        let member_lookup = adapter
            .members(
                &TypeId(FactId::new(
                    query_source.clone(),
                    LanguageDomain::QueryLanguage,
                    FactKind::Type,
                    "unused",
                )),
                MemberQuery {
                    name: Some("Период"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("query table member lookup refusal must not fail");
        assert_eq!(member_lookup.status, ResolveStatus::Unsupported);
        let global_context = adapter
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Sdbl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("query table global context lookup must not fail");
        assert_eq!(global_context.status, ResolveStatus::Ok);
        let global_facts = &global_context.facts[0].facts;
        assert!(global_facts
            .iter()
            .any(|fact| fact.id.kind == FactKind::QueryTable));
        assert!(global_facts
            .iter()
            .any(|fact| fact.id.kind == FactKind::QueryField));
        assert!(global_facts
            .iter()
            .any(|fact| fact.id.kind == FactKind::QueryParameter));

        let table_id = FactId::new(
            query_source.clone(),
            LanguageDomain::QueryLanguage,
            FactKind::QueryTable,
            "query_table:ОсновнаяТаблица",
        );
        let table = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&table_id),
                &ResolveContext::all(),
            )
            .expect("query table id lookup must not fail");
        assert_eq!(table.status, ResolveStatus::Ok);
        assert_eq!(table.facts[0].id, table_id);
        let FactDetails::QueryTable(info) = &table.facts[0].details else {
            panic!("query table must expose query table details");
        };
        assert_eq!(info.identifier.as_deref(), Some("ОсновнаяТаблица"));
        assert_eq!(info.table_role, QueryTableRole::Primary);
        assert_eq!(
            info.syntax.as_ref().map(|syntax| syntax.primary.as_str()),
            Some("ОсновнаяТаблица.<Имя таблицы>")
        );
        assert_eq!(
            info.template_parameters,
            vec!["Имя таблицы".to_string(), "Table name".to_string()]
        );
        assert_eq!(info.owner_path[0].primary, "Таблицы запросов");
        let source = info.source.as_ref().expect("query table source evidence");
        assert_eq!(source.source, query_source);
        assert_eq!(source.evidence_id, "query_table:ОсновнаяТаблица");
        assert_eq!(source.locale.as_deref(), Some("ru"));

        let by_name = adapter
            .resolve(
                context_resolver_core::ResolveQuery::ExactName {
                    source: Some(&query_source),
                    domain: Some(LanguageDomain::QueryLanguage),
                    kind: Some(FactKind::QueryTable),
                    name: "Основная таблица",
                },
                &ResolveContext::all(),
            )
            .expect("query table exact-name lookup must not fail");
        assert_eq!(by_name.status, ResolveStatus::Ok);
        assert_eq!(by_name.facts[0].id, table_id);

        let by_identifier = adapter
            .resolve(
                context_resolver_core::ResolveQuery::ExactName {
                    source: Some(&query_source),
                    domain: Some(LanguageDomain::QueryLanguage),
                    kind: Some(FactKind::QueryTable),
                    name: "ОсновнаяТаблица",
                },
                &ResolveContext::all(),
            )
            .expect("query table identifier lookup must not fail");
        assert_eq!(by_identifier.status, ResolveStatus::Ok);
        assert_eq!(by_identifier.facts[0].id, table_id);

        let by_syntax = adapter
            .resolve(
                context_resolver_core::ResolveQuery::ExactName {
                    source: Some(&query_source),
                    domain: Some(LanguageDomain::QueryLanguage),
                    kind: Some(FactKind::QueryTable),
                    name: "ОсновнаяТаблица.<Имя таблицы>",
                },
                &ResolveContext::all(),
            )
            .expect("query table syntax lookup must not fail");
        assert_eq!(by_syntax.status, ResolveStatus::Ok);
        assert_eq!(by_syntax.facts[0].id, table_id);

        let field_id = FactId::new(
            query_source.clone(),
            LanguageDomain::QueryLanguage,
            FactKind::QueryField,
            "query_table_field:query_table:ОсновнаяТаблица:Период",
        );
        let field = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&field_id),
                &ResolveContext::all(),
            )
            .expect("query field id lookup must not fail");
        assert_eq!(field.status, ResolveStatus::Ok);
        assert_eq!(field.facts[0].owner.as_ref(), Some(&table_id));
        let FactDetails::QueryField(field_info) = &field.facts[0].details else {
            panic!("query field must expose query field details");
        };
        assert_eq!(field_info.owner, table_id);
        assert_eq!(field_info.note.as_deref(), Some("Field note."));
        assert_eq!(
            field_info.types[0].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                platform_source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:Дата",
            )))
        );

        let member_of = adapter
            .related(&field_id, RelationKind::MemberOf, &ResolveContext::all())
            .expect("query field member_of traversal must not fail");
        assert_eq!(member_of.status, ResolveStatus::Ok);
        assert_eq!(member_of.facts[0].id, table_id);

        let has_type = adapter
            .related(&field_id, RelationKind::HasType, &ResolveContext::all())
            .expect("query field has_type traversal must not fail");
        assert_eq!(has_type.status, ResolveStatus::Ok);
        assert_eq!(
            has_type.facts[0].id,
            FactId::new(
                platform_source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:Дата",
            )
        );

        let forged_platform_id = FactId::new(
            query_source.clone(),
            LanguageDomain::QueryLanguage,
            FactKind::Type,
            "platform_type:Дата",
        );
        let forged_related = adapter
            .related(
                &forged_platform_id,
                RelationKind::HasType,
                &ResolveContext::all(),
            )
            .expect("forged query-table relation request must not fail");
        assert_eq!(forged_related.status, ResolveStatus::NotFound);

        let parameter_id = FactId::new(
            query_source,
            LanguageDomain::QueryLanguage,
            FactKind::QueryParameter,
            "query_table_parameter:query_table:ОсновнаяТаблица:Дата",
        );
        let parameter = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&parameter_id),
                &ResolveContext::all(),
            )
            .expect("query parameter id lookup must not fail");
        assert_eq!(parameter.status, ResolveStatus::Ok);
        let FactDetails::QueryParameter(parameter_info) = &parameter.facts[0].details else {
            panic!("query parameter must expose query parameter details");
        };
        assert_eq!(
            parameter_info.default_value.as_deref(),
            Some("НачалоПериода")
        );
        assert_eq!(
            parameter_info.types[0].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                platform_source,
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:Дата",
            )))
        );
    }

    #[test]
    fn query_table_snapshot_source_exposes_templates_fields_parameters_and_type_refs() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<QueryTableSnapshotSource>();
        assert_send_sync::<WorkerSafeCompositeResolver>();

        let query_source = SourceId::new("shcntx-query");
        let platform_source = fixture_source();
        let index_path = fixture_index_path("query-table-snapshot-source.sqlite");
        let index = open_index(&index_path);
        let snapshot = Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must build"));
        drop(index);
        std::fs::remove_file(&index_path).expect("snapshot adapter must not need SQLite file");
        let adapter = QueryTableSnapshotSource::with_source_ids(
            snapshot.clone(),
            query_source.clone(),
            platform_source.clone(),
        );

        let capabilities = adapter.capabilities();
        assert!(capabilities.exact_lookup);
        assert!(capabilities.relations);
        assert!(!capabilities.type_lookup);
        assert!(!capabilities.callables);
        assert!(capabilities.global_context);

        let global_context = adapter
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Sdbl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("query table snapshot global context lookup must not fail");
        assert_eq!(global_context.status, ResolveStatus::Ok);
        let global_facts = &global_context.facts[0].facts;
        assert!(global_facts
            .iter()
            .any(|fact| fact.id.kind == FactKind::QueryTable));
        assert!(global_facts
            .iter()
            .any(|fact| fact.id.kind == FactKind::QueryField));
        assert!(global_facts
            .iter()
            .any(|fact| fact.id.kind == FactKind::QueryParameter));

        let table_id = FactId::new(
            query_source.clone(),
            LanguageDomain::QueryLanguage,
            FactKind::QueryTable,
            "query_table:ОсновнаяТаблица",
        );
        let table = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&table_id),
                &ResolveContext::all(),
            )
            .expect("query table snapshot id lookup must not fail");
        assert_eq!(table.status, ResolveStatus::Ok);
        assert_eq!(table.facts[0].id, table_id);
        let FactDetails::QueryTable(info) = &table.facts[0].details else {
            panic!("query table snapshot must expose query table details");
        };
        assert_eq!(info.identifier.as_deref(), Some("ОсновнаяТаблица"));
        assert_eq!(info.table_role, QueryTableRole::Primary);
        assert_eq!(
            info.syntax.as_ref().map(|syntax| syntax.primary.as_str()),
            Some("ОсновнаяТаблица.<Имя таблицы>")
        );
        assert_eq!(
            info.template_parameters,
            vec!["Имя таблицы".to_string(), "Table name".to_string()]
        );
        assert_eq!(info.owner_path[0].primary, "Таблицы запросов");
        let source = info.source.as_ref().expect("query table source evidence");
        assert_eq!(source.source, query_source);
        assert_eq!(source.evidence_id, "query_table:ОсновнаяТаблица");
        assert_eq!(source.locale.as_deref(), Some("ru"));

        for name in [
            "Основная таблица",
            "ОсновнаяТаблица",
            "ОсновнаяТаблица.<Имя таблицы>",
        ] {
            let found = adapter
                .resolve(
                    context_resolver_core::ResolveQuery::ExactName {
                        source: Some(&SourceId::new("shcntx-query")),
                        domain: Some(LanguageDomain::QueryLanguage),
                        kind: Some(FactKind::QueryTable),
                        name,
                    },
                    &ResolveContext::all(),
                )
                .expect("query table snapshot exact-name lookup must not fail");
            assert_eq!(found.status, ResolveStatus::Ok);
            assert_eq!(found.facts[0].id, table_id);
        }

        let field_id = FactId::new(
            SourceId::new("shcntx-query"),
            LanguageDomain::QueryLanguage,
            FactKind::QueryField,
            "query_table_field:query_table:ОсновнаяТаблица:Период",
        );
        let field = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&field_id),
                &ResolveContext::all(),
            )
            .expect("query field snapshot id lookup must not fail");
        assert_eq!(field.status, ResolveStatus::Ok);
        assert_eq!(field.facts[0].owner.as_ref(), Some(&table_id));
        let FactDetails::QueryField(field_info) = &field.facts[0].details else {
            panic!("query field snapshot must expose query field details");
        };
        assert_eq!(field_info.owner, table_id);
        assert_eq!(field_info.note.as_deref(), Some("Field note."));
        assert_eq!(
            field_info.types[0].target,
            TypeRefTarget::Ok(TypeId(FactId::new(
                platform_source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:Дата",
            )))
        );
        let field_by_name = adapter
            .query_fields_by_name(&table_id, "Период", &ResolveContext::all())
            .expect("query field table/name snapshot lookup must not fail");
        assert_eq!(field_by_name.status, ResolveStatus::Ok);
        assert_eq!(field_by_name.facts[0].id, field_id);

        let member_of = adapter
            .related(&field_id, RelationKind::MemberOf, &ResolveContext::all())
            .expect("query field snapshot member_of traversal must not fail");
        assert_eq!(member_of.status, ResolveStatus::Ok);
        assert_eq!(member_of.facts[0].id, table_id);

        let has_type = adapter
            .related(&field_id, RelationKind::HasType, &ResolveContext::all())
            .expect("query field snapshot has_type traversal must not fail");
        assert_eq!(has_type.status, ResolveStatus::Ok);
        assert_eq!(
            has_type.facts[0].id,
            FactId::new(
                platform_source,
                LanguageDomain::PlatformApi,
                FactKind::Type,
                "platform_type:Дата",
            )
        );

        let document_field_id = FactId::new(
            SourceId::new("shcntx-query"),
            LanguageDomain::QueryLanguage,
            FactKind::QueryField,
            "query_table_field:query_table:ОсновнаяТаблица:Документ",
        );
        let document_has_type = adapter
            .related(
                &document_field_id,
                RelationKind::HasType,
                &ResolveContext::all(),
            )
            .expect("query field template has_type traversal must not fail");
        assert_eq!(document_has_type.status, ResolveStatus::Ok);
        let document_type = document_has_type
            .facts
            .first()
            .expect("template query field type must be returned");
        assert_eq!(
            document_type.id.local_id,
            "platform_type:ДокументСсылка.<Имя документа>"
        );
        let FactDetails::Type(type_info) = &document_type.details else {
            panic!("query field has_type target must expose type details");
        };
        assert_eq!(
            type_info
                .metadata_template
                .as_ref()
                .expect("template relation target must preserve metadata template")
                .metadata_kind,
            "ДокументСсылка"
        );
        assert_eq!(
            type_info.metadata_template.as_ref().unwrap().parameters,
            vec!["Имя документа".to_string()]
        );
        assert_eq!(
            type_info.type_template_key,
            Some(PlatformTypeTemplateKey::new("Document", "Ref"))
        );

        let parameter_id = FactId::new(
            SourceId::new("shcntx-query"),
            LanguageDomain::QueryLanguage,
            FactKind::QueryParameter,
            "query_table_parameter:query_table:ОсновнаяТаблица:Дата",
        );
        let parameter = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&parameter_id),
                &ResolveContext::all(),
            )
            .expect("query parameter snapshot id lookup must not fail");
        assert_eq!(parameter.status, ResolveStatus::Ok);
        let FactDetails::QueryParameter(parameter_info) = &parameter.facts[0].details else {
            panic!("query parameter snapshot must expose query parameter details");
        };
        assert_eq!(
            parameter_info.default_value.as_deref(),
            Some("НачалоПериода")
        );
        let parameter_by_name = adapter
            .query_parameters_by_name(&table_id, "Дата", &ResolveContext::all())
            .expect("query parameter table/name snapshot lookup must not fail");
        assert_eq!(parameter_by_name.status, ResolveStatus::Ok);
        assert_eq!(parameter_by_name.facts[0].id, parameter_id);
    }

    #[test]
    fn query_table_snapshot_source_enumerates_only_resolved_table_members() {
        let query_source = SourceId::new("shcntx-query");
        let platform_source = fixture_source();
        let index_path = fixture_index_path("query-table-member-enumeration.sqlite");
        let index = open_index(&index_path);
        let snapshot = Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must build"));
        drop(index);
        std::fs::remove_file(&index_path).expect("snapshot adapter must not need SQLite file");
        let adapter = QueryTableSnapshotSource::with_source_ids(
            snapshot,
            query_source.clone(),
            platform_source,
        );
        let table_id = FactId::new(
            query_source.clone(),
            LanguageDomain::QueryLanguage,
            FactKind::QueryTable,
            "query_table:ОсновнаяТаблица",
        );

        let fields = adapter
            .query_fields(&table_id, &ResolveContext::all())
            .expect("query field enumeration must not fail");
        assert_eq!(fields.status, ResolveStatus::Ok);
        assert_eq!(
            fields
                .facts
                .iter()
                .map(|fact| fact.id.local_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "query_table_field:query_table:ОсновнаяТаблица:Документ",
                "query_table_field:query_table:ОсновнаяТаблица:Период",
            ]
        );
        assert!(fields.facts.iter().all(|fact| fact.owner.as_ref() == Some(&table_id)));
        let FactDetails::QueryField(period) = &fields.facts[1].details else {
            panic!("enumerated field must preserve field evidence");
        };
        assert_eq!(period.note.as_deref(), Some("Field note."));
        assert_eq!(
            period
                .source
                .as_ref()
                .expect("enumerated field must preserve provenance")
                .evidence_id,
            "query_table_field:query_table:ОсновнаяТаблица:Период"
        );
        assert_eq!(
            adapter
                .query_fields_by_name(&table_id, "Период", &ResolveContext::all())
                .expect("query field point lookup must not fail")
                .facts,
            vec![fields.facts[1].clone()]
        );

        let parameters = adapter
            .query_parameters(&table_id, &ResolveContext::all())
            .expect("query parameter enumeration must not fail");
        assert_eq!(parameters.status, ResolveStatus::Ok);
        assert_eq!(
            parameters
                .facts
                .iter()
                .map(|fact| fact.id.local_id.as_str())
                .collect::<Vec<_>>(),
            vec!["query_table_parameter:query_table:ОсновнаяТаблица:Дата"]
        );
        assert!(parameters
            .facts
            .iter()
            .all(|fact| fact.owner.as_ref() == Some(&table_id)));
        let FactDetails::QueryParameter(date) = &parameters.facts[0].details else {
            panic!("enumerated parameter must preserve parameter evidence");
        };
        assert_eq!(date.default_value.as_deref(), Some("НачалоПериода"));
        assert_eq!(
            adapter
                .query_parameters_by_name(&table_id, "Дата", &ResolveContext::all())
                .expect("query parameter point lookup must not fail")
                .facts,
            vec![parameters.facts[0].clone()]
        );

        let empty_table = FactId::new(
            query_source.clone(),
            LanguageDomain::QueryLanguage,
            FactKind::QueryTable,
            "query_table:ПустаяТаблица",
        );
        assert_eq!(
            adapter
                .query_fields(&empty_table, &ResolveContext::all())
                .expect("empty table field enumeration must not fail"),
            ResolveResponse::ok(Vec::new())
        );
        assert_eq!(
            adapter
                .query_parameters(&empty_table, &ResolveContext::all())
                .expect("empty table parameter enumeration must not fail"),
            ResolveResponse::ok(Vec::new())
        );

        for invalid_table in [
            FactId::new(
                SourceId::new("other-query"),
                LanguageDomain::QueryLanguage,
                FactKind::QueryTable,
                "query_table:ОсновнаяТаблица",
            ),
            FactId::new(
                query_source.clone(),
                LanguageDomain::PlatformApi,
                FactKind::QueryTable,
                "query_table:ОсновнаяТаблица",
            ),
            FactId::new(
                query_source.clone(),
                LanguageDomain::QueryLanguage,
                FactKind::QueryField,
                "query_table_field:query_table:ОсновнаяТаблица:Период",
            ),
            FactId::new(
                query_source.clone(),
                LanguageDomain::QueryLanguage,
                FactKind::QueryTable,
                "query_table:НеизвестнаяТаблица",
            ),
        ] {
            assert_eq!(
                adapter
                    .query_fields(&invalid_table, &ResolveContext::all())
                    .expect("invalid table field enumeration must not fail")
                    .status,
                ResolveStatus::NotFound
            );
            assert_eq!(
                adapter
                    .query_parameters(&invalid_table, &ResolveContext::all())
                    .expect("invalid table parameter enumeration must not fail")
                    .status,
                ResolveStatus::NotFound
            );
        }

        let inactive_source = SourceId::new("inactive-query");
        let inactive_context = ResolveContext {
            active_sources: std::slice::from_ref(&inactive_source),
            domain: Some(LanguageDomain::QueryLanguage),
            scope: None,
        };
        assert_eq!(
            adapter
                .query_fields(&table_id, &inactive_context)
                .expect("inactive source field enumeration must not fail")
                .status,
            ResolveStatus::NotFound
        );
        assert_eq!(
            adapter
                .query_parameters(&table_id, &inactive_context)
                .expect("inactive source parameter enumeration must not fail")
                .status,
            ResolveStatus::NotFound
        );
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

        for (name, local_id) in [
            ("Null", "def_Null"),
            ("Неопределено", "def_Undefined"),
            ("Число", "def_Number"),
            ("Дата", "def_Date"),
            ("Булево", "def_Boolean"),
            ("Тип", "def_Type"),
        ] {
            let primitive = resolver
                .resolve_type(
                    TypeLookup::ExactName {
                        source: Some(&shlang),
                        domain: Some(LanguageDomain::BslLanguage),
                        name,
                    },
                    &ResolveContext::all(),
                )
                .expect("constrained BSL primitive lookup must not fail");
            assert_eq!(primitive.status, ResolveStatus::Ok);
            assert_eq!(primitive.facts[0].id.0.local_id, local_id);
            assert_eq!(primitive.facts[0].id.0.domain, LanguageDomain::BslLanguage);
        }

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
        assert!(bsl_scope.facts[0].facts.iter().any(|fact| {
            fact.id.domain == LanguageDomain::BslLanguage && fact.id.local_id == "def_Number"
        }));
        assert!(bsl_scope.facts[0].facts.iter().any(|fact| {
            fact.id.domain == LanguageDomain::BslLanguage && fact.id.local_id == "def_Boolean"
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

    fn generated_self_selector_index(file_name: &str, duplicate_catalog_manager: bool) -> SearchIndex {
        let path = generated_self_selector_index_path(file_name, duplicate_catalog_manager);
        SearchIndex::open_read_only(path).expect("generated-self selector index must open")
    }

    fn generated_self_selector_index_path(
        file_name: &str,
        duplicate_catalog_manager: bool,
    ) -> PathBuf {
        let path = temp_path(file_name);
        let mut builder = SearchIndexBuilder::new();
        for (_, family, variant) in generated_self_selector_records() {
            let base = format!("{family}{variant}");
            let primary = format!("{base}.<Generated>");
            builder
                .platform_type(platform_template_type(&primary, &primary, &base, "Generated"))
                .expect("generated-self template must sink");
        }
        for family in [
            "InformationRegister",
            "AccumulationRegister",
            "AccountingRegister",
            "CalculationRegister",
        ] {
            let base = format!("{family}Manager");
            let primary = format!("{base}.<Generated>");
            builder
                .platform_type(platform_template_type(&primary, &primary, &base, "Generated"))
                .expect("generated-self family root must sink");
        }
        if duplicate_catalog_manager {
            builder
                .platform_type(platform_template_type(
                    "CatalogManagerDuplicate.<Generated>",
                    "CatalogManager.<Generated>",
                    "CatalogManagerDuplicate",
                    "Generated",
                ))
                .expect("duplicate catalog-manager template must sink");
        }
        build_index_from_builder(&path, &metadata(), builder)
            .expect("generated-self selector index must build");
        path
    }

    // Independent companion-contract fixture: this is the documented metadata selector corpus,
    // not a production lookup. It deliberately constructs provider source facts and expected
    // public resolver evidence without importing or reusing the HBK-owned runtime mapping.
    fn generated_self_selector_records() -> [(&'static str, &'static str, &'static str); 20] {
        [
            ("metadata.generated-self.catalog-object", "Catalog", "Object"),
            ("metadata.generated-self.catalog-manager", "Catalog", "Manager"),
            ("metadata.generated-self.document-object", "Document", "Object"),
            ("metadata.generated-self.document-manager", "Document", "Manager"),
            (
                "metadata.generated-self.information-register-record-set",
                "InformationRegister",
                "RecordSet",
            ),
            (
                "metadata.generated-self.accumulation-register-record-set",
                "AccumulationRegister",
                "RecordSet",
            ),
            (
                "metadata.generated-self.accounting-register-record-set",
                "AccountingRegister",
                "RecordSet",
            ),
            (
                "metadata.generated-self.calculation-register-record-set",
                "CalculationRegister",
                "RecordSet",
            ),
            (
                "metadata.generated-self.chart-of-characteristic-types-object",
                "ChartOfCharacteristicTypes",
                "Object",
            ),
            (
                "metadata.generated-self.chart-of-characteristic-types-manager",
                "ChartOfCharacteristicTypes",
                "Manager",
            ),
            (
                "metadata.generated-self.exchange-plan-object",
                "ExchangePlan",
                "Object",
            ),
            (
                "metadata.generated-self.exchange-plan-manager",
                "ExchangePlan",
                "Manager",
            ),
            (
                "metadata.generated-self.business-process-object",
                "BusinessProcess",
                "Object",
            ),
            (
                "metadata.generated-self.business-process-manager",
                "BusinessProcess",
                "Manager",
            ),
            ("metadata.generated-self.task-object", "Task", "Object"),
            ("metadata.generated-self.task-manager", "Task", "Manager"),
            (
                "metadata.generated-self.chart-of-accounts-object",
                "ChartOfAccounts",
                "Object",
            ),
            (
                "metadata.generated-self.chart-of-accounts-manager",
                "ChartOfAccounts",
                "Manager",
            ),
            (
                "metadata.generated-self.chart-of-calculation-types-object",
                "ChartOfCalculationTypes",
                "Object",
            ),
            (
                "metadata.generated-self.chart-of-calculation-types-manager",
                "ChartOfCalculationTypes",
                "Manager",
            ),
        ]
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
            platform_type("Структура", Some("Structure")),
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
            platform_type("Дата", Some("Date")),
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
            .global_method(model::GlobalMethod {
                name: name("Мин", Some("Min")),
                signatures: vec![model::Signature {
                    text: "Мин(<Значение1>,...,<ЗначениеN>)".to_string(),
                    parameters: vec![model::Parameter {
                        name: "Значение1".to_string(),
                        required: true,
                        type_refs: Vec::new(),
                        description: None,
                    }],
                    return_types: Vec::new(),
                    variant: None,
                }],
                return_types: Vec::new(),
                description: Some("Returns minimal value.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("global-min"),
            })
            .expect("variadic global method must sink");
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
            .enum_definition(enum_definition(
                "ОбновлениеПредопределенныхДанных",
                "PredefinedDataUpdate",
            ))
            .expect("enum definition must sink");
        builder
            .enum_value(model::EnumValue {
                owner: name(
                    "ОбновлениеПредопределенныхДанных",
                    Some("PredefinedDataUpdate"),
                ),
                owner_identity: Some("enum:system:ОбновлениеПредопределенныхДанных".to_string()),
                name: name("Обновлять", Some("Update")),
                description: Some("Enum value description.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("predefined-data-update-value"),
            })
            .expect("enum value must sink");
        builder
            .global_method(model::GlobalMethod {
                name: name(
                    "ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы",
                    Some("GetPredefinedDataUpdate"),
                ),
                signatures: vec![model::Signature {
                    text: "ПолучитьОбновлениеПредопределенныхДанныхИнформационнойБазы()"
                        .to_string(),
                    parameters: Vec::new(),
                    return_types: Vec::new(),
                    variant: None,
                }],
                return_types: vec![model::TypeRef {
                    name: "ОбновлениеПредопределенныхДанных".to_string(),
                }],
                description: Some("Returns predefined data update mode.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("global-predefined-data-update"),
            })
            .expect("enum-backed global method must sink");
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
                owner: name("Структура", Some("Structure")),
                owner_identity: Some("platform_type:Структура".to_string()),
                name: name("Новый Структура(<Ключи>, <Значения>)", None),
                semantic: model::SemanticContext::default(),
                signatures: vec![model::Signature {
                    text: "Новый Структура(<Ключи>, <Значения>)".to_string(),
                    parameters: vec![
                        model::Parameter {
                            name: "Ключи".to_string(),
                            required: false,
                            type_refs: Vec::new(),
                            description: None,
                        },
                        model::Parameter {
                            name: "Значения".to_string(),
                            required: false,
                            type_refs: Vec::new(),
                            description: None,
                        },
                    ],
                    return_types: Vec::new(),
                    variant: None,
                }],
                description: Some("Creates structure.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("structure-constructor"),
            })
            .expect("structure constructor must sink");
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
                syntax: Some(name(
                    "ОсновнаяТаблица.<Имя таблицы>",
                    Some("MainTable.<Table name>"),
                )),
                identifier: Some("ОсновнаяТаблица".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                )
                .with_owner_path(vec![name("Таблицы запросов", Some("Query tables"))]),
                table_role: model::QueryTableRole::Primary,
                description: Some("Query provider fact.".to_string()),
                source: source_ref("query-table"),
            })
            .expect("query table must sink");
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:ПустаяТаблица".to_string()),
                name: "Пустая таблица".to_string(),
                syntax: None,
                identifier: Some("ПустаяТаблица".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Additional,
                description: Some("Empty query table provider fact.".to_string()),
                source: source_ref("empty-query-table"),
            })
            .expect("empty query table must sink");
        builder
            .table_field(model::QueryTableField {
                owner: name("ОсновнаяТаблица", None),
                owner_identity: Some("query_table:ОсновнаяТаблица".to_string()),
                name: "Период".to_string(),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTableField,
                )
                .with_owner_path(vec![
                    name("Таблицы запросов", Some("Query tables")),
                    name("Основная таблица", None),
                ]),
                type_refs: vec![model::TypeRef {
                    name: "Дата".to_string(),
                }],
                description: Some("Query field provider fact.".to_string()),
                note: Some("Field note.".to_string()),
                source: source_ref("query-table-field"),
            })
            .expect("query table field must sink");
        builder
            .table_field(model::QueryTableField {
                owner: name("ОсновнаяТаблица", None),
                owner_identity: Some("query_table:ОсновнаяТаблица".to_string()),
                name: "Документ".to_string(),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTableField,
                )
                .with_owner_path(vec![
                    name("Таблицы запросов", Some("Query tables")),
                    name("Основная таблица", None),
                ]),
                type_refs: vec![model::TypeRef {
                    name: "ДокументСсылка.<Имя документа>".to_string(),
                }],
                description: Some("Document query field provider fact.".to_string()),
                note: None,
                source: source_ref("query-table-field-document"),
            })
            .expect("template query table field must sink");
        builder
            .table_parameter(model::QueryTableParameter {
                owner: name("ОсновнаяТаблица", None),
                owner_identity: Some("query_table:ОсновнаяТаблица".to_string()),
                name: "Дата".to_string(),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTableParameter,
                )
                .with_owner_path(vec![
                    name("Таблицы запросов", Some("Query tables")),
                    name("Основная таблица", None),
                ]),
                type_refs: vec![model::TypeRef {
                    name: "Дата".to_string(),
                }],
                description: Some("Query parameter provider fact.".to_string()),
                default_value: Some("НачалоПериода".to_string()),
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

    fn enum_definition(primary: &str, alias: &str) -> model::EnumDefinition {
        let mut source = source_ref(primary);
        source.html_path = format!("objects/catalog2/{alias}.html");
        model::EnumDefinition {
            identity: None,
            name: name(primary, Some(alias)),
            value_links: Vec::new(),
            description: Some(format!("{primary} enum description.")),
            facts: model::SectionFacts::default(),
            source,
        }
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
                "def_Null",
                "shlang_def_null_ru.html",
            ),
            (
                LanguageSourceFamily::Shlang,
                "def_Undefined",
                "shlang_def_undefined_ru.html",
            ),
            (
                LanguageSourceFamily::Shlang,
                "def_Number",
                "shlang_def_number_ru.html",
            ),
            (
                LanguageSourceFamily::Shlang,
                "def_String",
                "shlang_def_string_ru.html",
            ),
            (
                LanguageSourceFamily::Shlang,
                "def_Date",
                "shlang_def_date_ru.html",
            ),
            (
                LanguageSourceFamily::Shlang,
                "def_Boolean",
                "shlang_def_boolean_ru.html",
            ),
            (
                LanguageSourceFamily::Shlang,
                "def_Type",
                "shlang_def_type_ru.html",
            ),
            (
                LanguageSourceFamily::Shlang,
                "def_BooleanTrue",
                "shlang_def_boolean_true_ru.html",
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
