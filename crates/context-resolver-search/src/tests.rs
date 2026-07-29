#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Instant;

    use context_resolver_core::{
        CallableLookup, CompositeResolver, ContextResolver, ContextSource, GlobalContextLanguage,
        GlobalContextQuery, MemberQuery, MemberQueryKind, ModuleContextMemberLookup,
        ModuleContextMembersLookup,
        ModuleContextKind, PlatformTypeTemplateKey, RelationKind, ResolveContext,
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

    fn collect_catalog_availability<I>(
        (contexts, since): (I, Option<&str>),
    ) -> (Vec<AvailabilityContext>, Option<String>)
    where
        I: Iterator<Item = AvailabilityContext>,
    {
        (
            contexts.collect(),
            since.map(str::to_string),
        )
    }

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
        assert_send_sync::<HbkBslContextCatalog>();
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
        let enum_availability = adapter
            .availability(&enum_returns.facts[0].id, &ResolveContext::all())
            .expect("snapshot enum-as-type availability lookup must not fail");
        assert_eq!(enum_availability.status, ResolveStatus::Ok);

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
    fn bsl_catalog_exposes_borrowed_platform_context_records_without_sql_fallback() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<HbkBslContextCatalog>();

        let source = fixture_source();
        let index_path = fixture_index_path("bsl-catalog-direct.sqlite");
        let snapshot =
            Arc::new(HbkFactSnapshot::from_path(&index_path).expect("snapshot must build"));
        std::fs::remove_file(&index_path).expect("catalog must not need SQLite file");
        let catalog = HbkBslContextCatalog::with_source_id(Arc::clone(&snapshot), source.clone());

        assert_eq!(catalog.source_id(), &source);
        assert_eq!(catalog.source_locale(), Some("ru"));

        let (settings_id, settings) = catalog
            .platform_type_by_id("platform_type:НастройкиКомпоновкиДанных")
            .expect("settings type must resolve by id");
        assert_eq!(catalog.string(settings.name.primary), "НастройкиКомпоновкиДанных");
        assert_eq!(
            catalog
                .platform_types_by_name("НастройкиКомпоновкиДанных")
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![settings_id]
        );

        let generated = catalog
            .generated_self_types("metadata.generated-self.catalog-manager")
            .collect::<Vec<_>>();
        assert_eq!(generated.len(), 1);
        assert_eq!(
            catalog.string(generated[0].1.name.primary),
            "СправочникМенеджер.<Имя справочника>"
        );
        assert_eq!(
            catalog
                .platform_types_by_template_key("Catalog", "Manager")
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![generated[0].0]
        );

        let (member_id, member) = catalog
            .member_by_name(settings_id, "Отбор")
            .next()
            .expect("settings filter member must resolve by name");
        assert_eq!(
            catalog.member_by_id(catalog.string(member.id)).map(|(id, _)| id),
            Some(member_id)
        );
        assert_eq!(
            catalog
                .members(settings_id)
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![member_id]
        );
        assert_eq!(
            catalog
                .member_by_name_kind(settings_id, "Отбор", Some(member.kind))
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![member_id]
        );

        let (filter_id, _) = catalog
            .platform_type_by_id("platform_type:ОтборКомпоновкиДанных")
            .expect("filter type must resolve by id");
        let (callable_id, callable) = catalog
            .callable_by_name(filter_id, "Найти")
            .next()
            .expect("filter callable must resolve by name");
        assert_eq!(
            catalog
                .callable_by_id(catalog.string(callable.id))
                .map(|(id, _)| id),
            Some(callable_id)
        );
        assert!(
            catalog
                .callables(filter_id)
                .any(|(id, _)| id == callable_id)
        );
        assert!(
            catalog
                .constructors(filter_id)
                .any(|(_, constructor)| catalog.string(constructor.name.primary)
                    == "Новый ОтборКомпоновкиДанных()")
        );

        let (global_property_id, global_property) = catalog
            .global_property_by_name("ТекущийОтбор")
            .next()
            .expect("global property must resolve by name");
        assert_eq!(
            catalog
                .global_by_id(catalog.string(global_property.id))
                .map(|(id, _)| id),
            Some(global_property_id)
        );
        assert!(catalog
            .global_by_id(catalog.string(global_property.id))
            .is_some_and(|(_, global)| global.domain == syntax_helper_search::HbkLanguageDomain::Bsl));
        assert!(
            catalog
                .global_properties()
                .any(|(id, _)| id == global_property_id)
        );
        let (_, _, global_callable_id, global_callable) = catalog
            .global_method_by_name("Сообщить")
            .next()
            .expect("global method must resolve by name");
        assert_eq!(catalog.string(global_callable.name.primary), "Сообщить");
        assert!(
            catalog
                .global_methods()
                .any(|(_, _, id, _)| id == global_callable_id)
        );

        let (event_id, event) = catalog
            .module_context_event_by_name(ModuleContextKind::Form, "ПриОткрытии")
            .next()
            .expect("form module event must resolve by name");
        let scoped_name_events = {
            let event_iter = {
                let event_name = String::from("ПриОткрытии");
                catalog.module_context_event_by_name(ModuleContextKind::Form, event_name.as_str())
            };
            event_iter.collect::<Vec<_>>()
        };
        assert_eq!(scoped_name_events.len(), 1);
        assert_eq!(scoped_name_events[0].0, event_id);
        assert_eq!(catalog.string(event.name.alias.unwrap()), "OnOpen");
        assert!(
            catalog
                .module_context_events(ModuleContextKind::Form)
                .any(|(id, _)| id == event_id)
        );
        assert_eq!(
            catalog
                .module_context_events(ModuleContextKind::Command)
                .count(),
            0
        );

        assert_eq!(
            collect_catalog_availability(catalog.platform_type_availability(settings_id)),
            (Vec::new(), None)
        );
        assert_eq!(
            collect_catalog_availability(catalog.member_availability(member_id)),
            (Vec::new(), None)
        );
        assert_eq!(
            collect_catalog_availability(catalog.callable_availability(callable_id)),
            (Vec::new(), None)
        );
        assert_eq!(
            collect_catalog_availability(catalog.global_availability(global_property_id)),
            (Vec::new(), None)
        );
        assert_eq!(
            collect_catalog_availability(catalog.callable_availability(event_id)),
            (
                vec![
                    AvailabilityContext::ThinClient,
                    AvailabilityContext::Server,
                ],
                Some("8.3.1".to_string()),
            )
        );
    }

    #[test]
    fn bsl_catalog_matches_snapshot_adapter_projection_boundary() {
        let source = fixture_source();
        let index_path = fixture_index_path("bsl-catalog-projection-boundary.sqlite");
        let snapshot =
            Arc::new(HbkFactSnapshot::from_path(&index_path).expect("snapshot must build"));
        std::fs::remove_file(&index_path).expect("catalog parity must not need SQLite file");
        let adapter = PlatformSnapshotSource::with_source_id(Arc::clone(&snapshot), source.clone());
        let catalog = HbkBslContextCatalog::with_source_id(snapshot, source.clone());

        let catalog_type_projection = |(_, ty): (
            syntax_helper_search::HbkPlatformTypeId,
            &syntax_helper_search::HbkPlatformType,
        )| {
            (
                project_hbk_fact_id(&catalog, FactKind::Type, catalog.string(ty.id)),
                catalog.string(ty.name.primary).to_string(),
            )
        };
        let adapter_type_projection = |ty: &ResolvedType| {
            (ty.id.0.clone(), ty.fact.name.primary.clone())
        };
        let expected_generated = catalog
            .generated_self_types("metadata.generated-self.catalog-manager")
            .map(catalog_type_projection)
            .collect::<Vec<_>>();
        let actual_generated = adapter
            .resolve_type(
                TypeLookup::GeneratedSelfTemplate {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    generated_self_role: "metadata.generated-self.catalog-manager",
                },
                &ResolveContext::all(),
            )
            .expect("snapshot generated-self lookup must not fail")
            .facts
            .iter()
            .map(adapter_type_projection)
            .collect::<Vec<_>>();
        assert_eq!(actual_generated, expected_generated);

        let (settings_id, _) = catalog
            .platform_type_by_id("platform_type:НастройкиКомпоновкиДанных")
            .expect("settings type must resolve by id");
        let settings = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:НастройкиКомпоновкиДанных",
        ));
        let catalog_member_projection = |(id, member): (
            syntax_helper_search::HbkTypeMemberId,
            &syntax_helper_search::HbkTypeMember,
        )| {
            (
                project_hbk_fact_id(
                    &catalog,
                    FactKind::Member,
                    catalog.string(catalog.snapshot().type_member(id).id),
                ),
                catalog.string(member.name.primary).to_string(),
                project_hbk_fact_id(
                    &catalog,
                    FactKind::Type,
                    catalog.string(catalog.snapshot().platform_type(member.owner).id),
                ),
                member
                    .type_refs
                    .iter()
                    .map(|type_ref| project_hbk_type_ref(&catalog, type_ref))
                    .collect::<Vec<_>>(),
            )
        };
        let adapter_member_projection = |member: &ResolvedMember| {
            (
                member.id.0.clone(),
                member.fact.name.primary.clone(),
                member.owner.0.clone(),
                member.info.types.clone(),
            )
        };
        let expected_members = catalog
            .members(settings_id)
            .map(catalog_member_projection)
            .collect::<Vec<_>>();
        let actual_members = adapter
            .members(
                &settings,
                MemberQuery {
                    name: None,
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("snapshot member enumeration must not fail")
            .facts
            .iter()
            .map(adapter_member_projection)
            .collect::<Vec<_>>();
        assert_eq!(actual_members, expected_members);

        let (member_id, _) = catalog
            .member_by_name(settings_id, "Отбор")
            .next()
            .expect("settings filter member must resolve by name");
        let expected_exact_members = catalog
            .member_by_name(settings_id, "Отбор")
            .map(catalog_member_projection)
            .collect::<Vec<_>>();
        let actual_exact_members = adapter
            .members(
                &settings,
                MemberQuery {
                    name: Some("Отбор"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("snapshot exact member lookup must not fail")
            .facts
            .iter()
            .map(adapter_member_projection)
            .collect::<Vec<_>>();
        assert_eq!(actual_exact_members, expected_exact_members);

        let (filter_id, _) = catalog
            .platform_type_by_id("platform_type:ОтборКомпоновкиДанных")
            .expect("filter type must resolve by id");
        let filter = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ОтборКомпоновкиДанных",
        ));
        let catalog_callable_projection = |(_id, callable): (
            syntax_helper_search::HbkCallableId,
            &syntax_helper_search::HbkCallable,
        )| {
            (
                project_hbk_callable_fact_id(&catalog, callable),
                catalog.string(callable.name.primary).to_string(),
                callable.owner.map(|owner| {
                    project_hbk_fact_id(
                        &catalog,
                        FactKind::Type,
                        catalog.string(catalog.snapshot().platform_type(owner).id),
                    )
                }),
                callable
                    .signatures
                    .iter()
                    .map(|signature| project_hbk_signature(&catalog, signature))
                    .collect::<Vec<_>>(),
                callable
                    .return_type_refs
                    .iter()
                    .map(|type_ref| project_hbk_type_ref(&catalog, type_ref))
                    .collect::<Vec<_>>(),
            )
        };
        let adapter_callable_projection = |callable: &ResolvedCallable| {
            (
                callable.id.0.clone(),
                callable.fact.name.primary.clone(),
                callable.owner.as_ref().map(|owner| owner.0.clone()),
                callable.info.signatures.clone(),
                callable.info.return_types.clone(),
            )
        };
        let expected_callables = catalog
            .callable_by_name(filter_id, "Найти")
            .map(catalog_callable_projection)
            .collect::<Vec<_>>();
        let actual_callables = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Найти",
                },
                &ResolveContext::all(),
            )
            .expect("snapshot exact callable lookup must not fail")
            .facts
            .iter()
            .map(adapter_callable_projection)
            .collect::<Vec<_>>();
        assert_eq!(actual_callables, expected_callables);
        let (callable_id, _) = catalog
            .callable_by_name(filter_id, "Найти")
            .next()
            .expect("filter callable must resolve by name");

        let (generated_id, generated_record) = catalog
            .generated_self_types("metadata.generated-self.catalog-manager")
            .next()
            .expect("generated-self catalog manager must resolve");
        let generated_type = TypeId(project_hbk_fact_id(
            &catalog,
            FactKind::Type,
            catalog.string(generated_record.id),
        ));
        let expected_generated_members = catalog
            .members(generated_id)
            .map(catalog_member_projection)
            .collect::<Vec<_>>();
        let actual_generated_members = adapter
            .members(
                &generated_type,
                MemberQuery {
                    name: None,
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("generated-self member enumeration must not fail")
            .facts
            .iter()
            .map(adapter_member_projection)
            .collect::<Vec<_>>();
        assert_eq!(actual_generated_members, expected_generated_members);

        let expected_generated_callables = catalog
            .callables(generated_id)
            .map(catalog_callable_projection)
            .collect::<Vec<_>>();
        let actual_generated_callables = catalog
            .callables(generated_id)
            .map(|(id, callable)| {
                let name = catalog.string(callable.name.primary);
                assert_eq!(
                    catalog
                        .callable_by_name(generated_id, name)
                        .map(|(candidate, _)| candidate)
                        .collect::<Vec<_>>(),
                    vec![id],
                    "generated-self callable point lookup must match enumeration"
                );
                adapter
                    .callable(
                        CallableLookup::OwnerName {
                            owner: Some(&generated_type),
                            name,
                        },
                        &ResolveContext::all(),
                    )
                    .expect("generated-self callable projection must not fail")
                    .facts
                    .first()
                    .map(adapter_callable_projection)
                    .expect("generated-self callable must resolve")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual_generated_callables,
            expected_generated_callables
        );

        let global_context = adapter
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Bsl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("snapshot global context lookup must not fail");
        assert_eq!(global_context.status, ResolveStatus::Ok);
        let global_context = global_context
            .facts
            .first()
            .expect("global context must resolve");

        let expected_global_methods = catalog
            .global_methods()
            .map(|(_, _, id, callable)| catalog_callable_projection((id, callable)))
            .collect::<Vec<_>>();
        let actual_global_methods = global_context
            .methods
            .iter()
            .map(adapter_callable_projection)
            .collect::<Vec<_>>();
        assert_eq!(actual_global_methods, expected_global_methods);

        let catalog_global_projection = |(id, global): (
            syntax_helper_search::HbkGlobalFactId,
            &syntax_helper_search::HbkGlobalFact,
        )| {
            (
                project_hbk_fact_id(
                    &catalog,
                    FactKind::Global,
                    catalog.string(catalog.snapshot().global_fact(id).id),
                ),
                catalog.string(global.name.primary).to_string(),
                global
                    .type_refs
                    .iter()
                    .map(|type_ref| project_hbk_type_ref(&catalog, type_ref))
                    .collect::<Vec<_>>(),
            )
        };
        let adapter_global_projection = |global: &ContextFact| {
            let FactDetails::Member(info) = &global.details else {
                panic!("global property must expose member details");
            };
            (
                global.id.clone(),
                global.name.primary.clone(),
                info.types.clone(),
            )
        };
        let expected_global_properties = catalog
            .global_properties()
            .map(catalog_global_projection)
            .collect::<Vec<_>>();
        let actual_global_properties = global_context
            .properties
            .iter()
            .map(adapter_global_projection)
            .collect::<Vec<_>>();
        assert_eq!(actual_global_properties, expected_global_properties);
        let (global_property_id, _) = catalog
            .global_property_by_name("ТекущийОтбор")
            .next()
            .expect("global property must resolve by name");

        let module_context = adapter
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
        assert_eq!(module_context.status, ResolveStatus::Ok);
        let module_context = module_context
            .facts
            .first()
            .expect("form module context must resolve");

        let module_methods = module_context
            .methods
            .iter()
            .map(adapter_callable_projection)
            .collect::<Vec<_>>();
        let module_properties = module_context
            .properties
            .iter()
            .map(adapter_global_projection)
            .collect::<Vec<_>>();
        assert_eq!(module_methods, expected_global_methods);
        assert_eq!(module_properties, expected_global_properties);

        let expected_form_events = catalog
            .module_context_events(ModuleContextKind::Form)
            .map(catalog_callable_projection)
            .collect::<Vec<_>>();
        let actual_form_events = module_context
            .events
            .iter()
            .map(adapter_callable_projection)
            .collect::<Vec<_>>();
        assert_eq!(actual_form_events, expected_form_events);

        let expected_exact_event = catalog
            .module_context_event_by_name(ModuleContextKind::Form, "ПриОткрытии")
            .map(catalog_callable_projection)
            .collect::<Vec<_>>();
        let exact_event = adapter
            .module_context_member(
                ModuleContextMemberLookup {
                    language: GlobalContextLanguage::Bsl,
                    domain: LanguageDomain::PlatformApi,
                    module_kind: ModuleContextKind::Form,
                    name: "ПриОткрытии",
                    kind: MemberQueryKind::Event,
                },
                &ResolveContext::all(),
            )
            .expect("snapshot exact module event lookup must not fail");
        let actual_exact_event = exact_event
            .facts
            .iter()
            .map(|member| match member {
                context_resolver_core::ResolvedBslContextMember::Callable(callable) => {
                    adapter_callable_projection(callable)
                }
                context_resolver_core::ResolvedBslContextMember::Property(_) => {
                    panic!("exact event lookup must return callable events")
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(actual_exact_event, expected_exact_event);
        let (event_id, _) = catalog
            .module_context_event_by_name(ModuleContextKind::Form, "ПриОткрытии")
            .next()
            .expect("form module event must resolve by name");

        let actual_availability = |id: &FactId| {
            let response = adapter
                .availability(id, &ResolveContext::all())
                .expect("snapshot availability lookup must not fail");
            assert_eq!(response.status, ResolveStatus::Ok);
            let fact = response
                .facts
                .first()
                .expect("snapshot availability must return one fact");
            (
                fact.availability.contexts.clone(),
                fact.availability.since.clone(),
            )
        };
        for (id, expected) in [
            (
                settings.0.clone(),
                collect_catalog_availability(catalog.platform_type_availability(settings_id)),
            ),
            (
                FactId::new(
                    source.clone(),
                    LanguageDomain::PlatformApi,
                    FactKind::Member,
                    catalog.string(catalog.snapshot().type_member(member_id).id),
                ),
                collect_catalog_availability(catalog.member_availability(member_id)),
            ),
            (
                FactId::new(
                    source.clone(),
                    LanguageDomain::PlatformApi,
                    FactKind::Callable,
                    catalog.string(catalog.snapshot().callable(callable_id).id),
                ),
                collect_catalog_availability(catalog.callable_availability(callable_id)),
            ),
            (
                FactId::new(
                    source.clone(),
                    LanguageDomain::PlatformApi,
                    FactKind::Global,
                    catalog.string(catalog.snapshot().global_fact(global_property_id).id),
                ),
                collect_catalog_availability(catalog.global_availability(global_property_id)),
            ),
            (
                FactId::new(
                    source.clone(),
                    LanguageDomain::PlatformApi,
                    FactKind::Callable,
                    catalog.string(catalog.snapshot().callable(event_id).id),
                ),
                collect_catalog_availability(catalog.callable_availability(event_id)),
            ),
        ] {
            assert_eq!(actual_availability(&id), expected, "{}", id.local_id);
        }
    }

    #[test]
    fn project_hbk_member_kind_projects_all_snapshot_variants() {
        for (snapshot, expected) in [
            (HbkTypeMemberKind::Property, MemberKind::Property),
            (HbkTypeMemberKind::Method, MemberKind::Method),
            (HbkTypeMemberKind::Event, MemberKind::Event),
            (HbkTypeMemberKind::EnumValue, MemberKind::EnumValue),
        ] {
            assert_eq!(project_hbk_member_kind(snapshot), expected);
        }
    }

    #[test]
    fn project_hbk_callable_kind_projects_all_snapshot_variants() {
        for (snapshot, expected) in [
            (HbkCallableKind::Method, CallableKind::Method),
            (HbkCallableKind::Constructor, CallableKind::Constructor),
            (HbkCallableKind::GlobalMethod, CallableKind::GlobalMethod),
            (HbkCallableKind::Event, CallableKind::Event),
            (HbkCallableKind::LanguageFunction, CallableKind::GlobalMethod),
        ] {
            assert_eq!(project_hbk_callable_kind(snapshot), expected);
        }
    }

    #[test]
    fn project_hbk_member_query_kind_projects_all_core_query_variants() {
        for (query, expected) in [
            (MemberQueryKind::Property, HbkTypeMemberKind::Property),
            (MemberQueryKind::Method, HbkTypeMemberKind::Method),
            (MemberQueryKind::Event, HbkTypeMemberKind::Event),
            (MemberQueryKind::EnumValue, HbkTypeMemberKind::EnumValue),
        ] {
            assert_eq!(project_hbk_member_query_kind(query), expected);
        }
    }

    #[test]
    fn project_hbk_callable_fact_id_classifies_all_snapshot_callable_variants() {
        let source = fixture_source();
        let index = fixture_index("hbk-callable-fact-id-projection.sqlite");
        let snapshot =
            Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must materialize"));
        let catalog = HbkBslContextCatalog::with_source_id(snapshot, source.clone());
        let (_, _, _, seed) = catalog
            .global_methods()
            .next()
            .expect("fixture must expose a callable seed");
        let local_id = catalog.string(seed.id).to_string();

        for (kind, expected_fact_kind) in [
            (HbkCallableKind::Method, FactKind::Callable),
            (HbkCallableKind::Constructor, FactKind::Constructor),
            (HbkCallableKind::GlobalMethod, FactKind::Callable),
            (HbkCallableKind::Event, FactKind::Callable),
            (HbkCallableKind::LanguageFunction, FactKind::Callable),
        ] {
            let callable = syntax_helper_search::HbkCallable {
                kind,
                ..(*seed).clone()
            };
            let id = project_hbk_callable_fact_id(&catalog, &callable);

            assert_eq!(id.source, source, "{kind:?}");
            assert_eq!(id.domain, LanguageDomain::PlatformApi, "{kind:?}");
            assert_eq!(id.kind, expected_fact_kind, "{kind:?}");
            assert_eq!(id.local_id, local_id, "{kind:?}");
        }
    }

    #[test]
    fn metadata_module_context_kind_is_public_and_preserves_exact_selectors() {
        for (selector, expected) in [
            ("metadata.module-role.common", ModuleContextKind::Common),
            ("metadata.module-role.command", ModuleContextKind::Command),
            ("metadata.module-role.object", ModuleContextKind::Object),
            ("metadata.module-role.manager", ModuleContextKind::Manager),
            ("metadata.module-role.form", ModuleContextKind::Form),
            ("metadata.module-role.record-set", ModuleContextKind::RecordSet),
        ] {
            assert_eq!(
                context_resolver_core::metadata_module_context_kind(selector),
                Some(expected),
                "{selector}"
            );
        }
        assert_eq!(
            context_resolver_core::metadata_module_context_kind("metadata.module-role.session"),
            None
        );
        assert_eq!(
            context_resolver_core::metadata_module_context_kind("unknown"),
            None
        );
    }

    #[test]
    #[ignore = "manual measurement probe for borrowed BSL catalog OpenSpec batch"]
    fn bsl_catalog_measurement_probe() {
        let source = fixture_source();
        let index_path = fixture_index_path("bsl-catalog-measurement-probe.sqlite");
        let snapshot =
            Arc::new(HbkFactSnapshot::from_path(&index_path).expect("snapshot must build"));
        std::fs::remove_file(&index_path).expect("measurement probe must not need SQLite file");
        println!("compat_deleted_sqlite_success=1");
        compat_adapter_sequence(Arc::clone(&snapshot), source.clone());
        println!("direct_deleted_sqlite_success=1");
        direct_bsl_catalog_sequence(snapshot, source);
    }

    fn compat_adapter_sequence(snapshot: Arc<HbkFactSnapshot>, source: SourceId) {
        let adapter = PlatformSnapshotSource::with_source_id(snapshot, source.clone());
        let settings = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:НастройкиКомпоновкиДанных",
        ));
        let filter = TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            "platform_type:ОтборКомпоновкиДанных",
        ));

        let global_context = adapter
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Bsl,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("compat global context lookup must not fail");
        assert_eq!(global_context.status, ResolveStatus::Ok);
        let global_scope = global_context
            .facts
            .first()
            .expect("compat global context must return one scope");

        let generated_self = adapter
            .resolve_type(
                TypeLookup::GeneratedSelfTemplate {
                    source: Some(&source),
                    domain: Some(LanguageDomain::PlatformApi),
                    generated_self_role: "metadata.generated-self.catalog-manager",
                },
                &ResolveContext::all(),
            )
            .expect("compat generated-self lookup must not fail");
        assert_eq!(generated_self.status, ResolveStatus::Ok);

        let exact_members = adapter
            .members(
                &settings,
                MemberQuery {
                    name: Some("Отбор"),
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("compat exact member lookup must not fail");
        assert_eq!(exact_members.status, ResolveStatus::Ok);
        let exact_member = exact_members
            .facts
            .first()
            .expect("compat exact member must resolve");

        let all_members = adapter
            .members(
                &settings,
                MemberQuery {
                    name: None,
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("compat member enumeration must not fail");
        assert_eq!(all_members.status, ResolveStatus::Ok);

        let callable = adapter
            .callable(
                CallableLookup::OwnerName {
                    owner: Some(&filter),
                    name: "Найти",
                },
                &ResolveContext::all(),
            )
            .expect("compat callable lookup must not fail");
        assert_eq!(callable.status, ResolveStatus::Ok);

        let module_context = adapter
            .module_context(
                ModuleContextQuery {
                    language: GlobalContextLanguage::Bsl,
                    domain: LanguageDomain::PlatformApi,
                    kind: ModuleContextKind::Form,
                    sources: &[],
                },
                &ResolveContext::all(),
            )
            .expect("compat module context lookup must not fail");
        assert_eq!(module_context.status, ResolveStatus::Ok);
        let module_scope = module_context
            .facts
            .first()
            .expect("compat module context must return one scope");

        let module_event = adapter
            .module_context_member(
                ModuleContextMemberLookup {
                    language: GlobalContextLanguage::Bsl,
                    domain: LanguageDomain::PlatformApi,
                    module_kind: ModuleContextKind::Form,
                    name: "ПриОткрытии",
                    kind: MemberQueryKind::Event,
                },
                &ResolveContext::all(),
            )
            .expect("compat exact module event lookup must not fail");
        assert_eq!(module_event.status, ResolveStatus::Ok);

        let availability = adapter
            .availability(&exact_member.id.0, &ResolveContext::all())
            .expect("compat member availability lookup must not fail");
        assert_eq!(availability.status, ResolveStatus::Ok);
        let availability_fact = availability
            .facts
            .first()
            .expect("compat availability must return one fact");

        println!("compat_global_context_invocations=1");
        println!("compat_global_context_responses={}", global_context.facts.len());
        println!("compat_global_methods={}", global_scope.methods.len());
        println!("compat_global_properties={}", global_scope.properties.len());
        println!("compat_generated_self_responses={}", generated_self.facts.len());
        println!("compat_exact_member_responses={}", exact_members.facts.len());
        println!("compat_member_enum_responses={}", all_members.facts.len());
        println!("compat_callable_responses={}", callable.facts.len());
        println!("compat_module_context_responses={}", module_context.facts.len());
        println!("compat_module_context_methods={}", module_scope.methods.len());
        println!("compat_module_context_properties={}", module_scope.properties.len());
        println!("compat_module_context_events={}", module_scope.events.len());
        println!("compat_module_event_responses={}", module_event.facts.len());
        println!("compat_availability_responses={}", availability.facts.len());
        println!(
            "compat_availability_contexts={}",
            availability_fact.availability.contexts.len()
        );
        println!(
            "compat_availability_since_present={}",
            usize::from(availability_fact.availability.since.is_some())
        );
    }

    fn direct_bsl_catalog_sequence(snapshot: Arc<HbkFactSnapshot>, source: SourceId) {
        let catalog = HbkBslContextCatalog::with_source_id(snapshot, source);
        let (settings_id, _) = catalog
            .platform_type_by_id("platform_type:НастройкиКомпоновкиДанных")
            .expect("direct settings type must resolve");
        let (filter_id, _) = catalog
            .platform_type_by_id("platform_type:ОтборКомпоновкиДанных")
            .expect("direct filter type must resolve");

        let generated_self = catalog
            .generated_self_types("metadata.generated-self.catalog-manager")
            .count();
        let (exact_member_id, _) = catalog
            .member_by_name(settings_id, "Отбор")
            .next()
            .expect("direct exact member must resolve");
        let exact_members = 1;
        let all_members = catalog.members(settings_id).count();
        let callables = catalog.callable_by_name(filter_id, "Найти").count();
        let global_methods = catalog.global_methods().count();
        let global_properties = catalog.global_properties().count();
        let module_events = catalog
            .module_context_events(ModuleContextKind::Form)
            .count();
        let exact_module_events = catalog
            .module_context_event_by_name(ModuleContextKind::Form, "ПриОткрытии")
            .count();
        let (availability_contexts, available_since) =
            catalog.member_availability(exact_member_id);

        println!("direct_source_locale_present={}", usize::from(catalog.source_locale().is_some()));
        println!("direct_generated_self_records={generated_self}");
        println!("direct_exact_member_records={exact_members}");
        println!("direct_member_enum_records={all_members}");
        println!("direct_callable_records={callables}");
        println!("direct_global_method_records={global_methods}");
        println!("direct_global_property_records={global_properties}");
        println!("direct_module_event_records={module_events}");
        println!("direct_exact_module_event_records={exact_module_events}");
        println!("direct_availability_contexts={}", availability_contexts.count());
        println!(
            "direct_availability_since_present={}",
            usize::from(available_since.is_some())
        );
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
    fn platform_adapter_resolves_exact_module_members_without_context_enumeration() {
        let adapter = PlatformSearchSource::with_source_id(
            fixture_index("platform-module-member-exact.sqlite"),
            fixture_source(),
        );
        let request = |name, kind| ModuleContextMemberLookup {
            language: GlobalContextLanguage::Bsl,
            domain: LanguageDomain::PlatformApi,
            module_kind: ModuleContextKind::Form,
            name,
            kind,
        };

        let property = adapter
            .module_context_member(request("ТекущийОтбор", MemberQueryKind::Property), &ResolveContext::all())
            .expect("exact property lookup must not fail");
        assert_eq!(property.status, ResolveStatus::Ok);
        let method = adapter
            .module_context_member(request("Сообщить", MemberQueryKind::Method), &ResolveContext::all())
            .expect("exact method lookup must not fail");
        assert_eq!(method.status, ResolveStatus::Ok);
        let event = adapter
            .module_context_member(request("ПриОткрытии", MemberQueryKind::Event), &ResolveContext::all())
            .expect("exact event lookup must not fail");
        assert_eq!(event.status, ResolveStatus::Ok);
        assert_eq!(
            adapter
                .module_context_member(request("Неизвестно", MemberQueryKind::Event), &ResolveContext::all())
                .expect("missing exact event lookup must not fail")
                .status,
            ResolveStatus::NotFound
        );
        assert_eq!(
            adapter
                .module_context_member(
                    ModuleContextMemberLookup {
                        module_kind: ModuleContextKind::Command,
                        ..request("ПриОткрытии", MemberQueryKind::Event)
                    },
                    &ResolveContext::all(),
                )
                .expect("unsupported exact event context lookup must not fail")
                .status,
            ResolveStatus::Unsupported
        );
    }

    #[test]
    fn platform_adapters_enumerate_module_members_without_context_snapshot_filtering() {
        let source = fixture_source();
        let index = fixture_index("platform-module-member-enumeration.sqlite");
        let snapshot =
            Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must materialize"));
        let query = ModuleContextMembersLookup {
            language: GlobalContextLanguage::Bsl,
            domain: LanguageDomain::PlatformApi,
            module_kind: ModuleContextKind::Form,
        };

        let search = PlatformSearchSource::with_source_id(index, source.clone());
        let search_response = search
            .module_context_members(query, &ResolveContext::all())
            .expect("SQL module member enumeration must not fail");
        assert_eq!(search_response.status, ResolveStatus::Ok);
        assert!(
            search_response
                .facts
                .iter()
                .any(|member| matches!(member, context_resolver_core::ResolvedBslContextMember::Property(fact) if fact.name.primary == "ТекущийОтбор"))
        );
        assert!(
            search_response
                .facts
                .iter()
                .any(|member| matches!(member, context_resolver_core::ResolvedBslContextMember::Callable(callable) if callable.fact.name.primary == "Сообщить"))
        );
        assert!(
            search_response
                .facts
                .iter()
                .any(|member| matches!(member, context_resolver_core::ResolvedBslContextMember::Callable(callable) if callable.fact.name.primary == "ПриОткрытии"))
        );

        let snapshot = PlatformSnapshotSource::with_source_id(snapshot, source);
        let snapshot_response = snapshot
            .module_context_members(query, &ResolveContext::all())
            .expect("snapshot module member enumeration must not fail");
        assert_eq!(snapshot_response.status, ResolveStatus::Ok);
        assert_eq!(snapshot_response.facts.len(), search_response.facts.len());
    }

    #[test]
    fn platform_snapshot_resolves_exact_module_events_without_context_enumeration() {
        let source = fixture_source();
        let index = fixture_index("platform-snapshot-module-member-exact.sqlite");
        let snapshot =
            Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must materialize"));
        let adapter = PlatformSnapshotSource::with_source_id(snapshot, source);

        let response = adapter
            .module_context_member(
                ModuleContextMemberLookup {
                    language: GlobalContextLanguage::Bsl,
                    domain: LanguageDomain::PlatformApi,
                    module_kind: ModuleContextKind::Form,
                    name: "ПриОткрытии",
                    kind: MemberQueryKind::Event,
                },
                &ResolveContext::all(),
            )
            .expect("exact snapshot event lookup must not fail");

        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(response.facts.len(), 1);

        let unsupported = adapter
            .module_context_member(
                ModuleContextMemberLookup {
                    module_kind: ModuleContextKind::Command,
                    ..ModuleContextMemberLookup {
                        language: GlobalContextLanguage::Bsl,
                        domain: LanguageDomain::PlatformApi,
                        module_kind: ModuleContextKind::Form,
                        name: "ПриОткрытии",
                        kind: MemberQueryKind::Event,
                    }
                },
                &ResolveContext::all(),
            )
            .expect("unsupported snapshot exact event context lookup must not fail");
        assert_eq!(unsupported.status, ResolveStatus::Unsupported);
    }

    #[test]
    fn exact_module_event_ambiguity_is_preserved_by_sql_and_snapshot_adapters() {
        let source = fixture_source();
        let index = ambiguous_module_member_index("platform-module-member-ambiguous.sqlite");
        let snapshot =
            Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must materialize"));
        let query = ModuleContextMemberLookup {
            language: GlobalContextLanguage::Bsl,
            domain: LanguageDomain::PlatformApi,
            module_kind: ModuleContextKind::Form,
            name: "ПриОткрытии",
            kind: MemberQueryKind::Event,
        };

        let search = PlatformSearchSource::with_source_id(index, source.clone());
        let search_response = search
            .module_context_member(query, &ResolveContext::all())
            .expect("ambiguous SQL event lookup must not fail");
        assert_eq!(search_response.status, ResolveStatus::Ambiguous);
        assert_eq!(search_response.candidates.len(), 2);

        let snapshot = PlatformSnapshotSource::with_source_id(snapshot, source);
        let snapshot_response = snapshot
            .module_context_member(query, &ResolveContext::all())
            .expect("ambiguous snapshot event lookup must not fail");
        assert_eq!(snapshot_response.status, ResolveStatus::Ambiguous);
        assert_eq!(snapshot_response.candidates.len(), 2);
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
    fn platform_search_source_enumerates_complete_raw_members_by_owner_and_kind() {
        let source = fixture_source();
        let adapter = PlatformSearchSource::with_source_id(
            fixture_index("platform-search-member-enumeration.sqlite"),
            source.clone(),
        );

        assert_platform_member_enumeration_contract(&adapter, &source);
    }

    #[test]
    fn platform_snapshot_source_enumerates_complete_raw_members_by_owner_and_kind() {
        let source = fixture_source();
        let index = fixture_index("platform-snapshot-member-enumeration.sqlite");
        let snapshot =
            Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must materialize"));
        let adapter = PlatformSnapshotSource::with_source_id(snapshot, source.clone());

        assert_platform_member_enumeration_contract(&adapter, &source);
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
    fn sdbl_catalog_exposes_borrowed_tables_fields_parameters_and_selectors() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<crate::HbkSdblQueryCatalog>();

        let query_source = SourceId::new("custom-query");
        let platform_source = SourceId::new("custom-platform");
        let index_path = fixture_index_path("sdbl-catalog-direct.sqlite");
        let index = open_index(&index_path);
        let snapshot = Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must build"));
        drop(index);
        std::fs::remove_file(&index_path).expect("catalog must not need SQLite file");

        let catalog = crate::HbkSdblQueryCatalog::with_source_ids(
            snapshot,
            query_source.clone(),
            platform_source.clone(),
        );
        assert_eq!(catalog.source_id(), &query_source);
        assert_eq!(catalog.platform_source_id(), &platform_source);
        assert_eq!(catalog.source_locale(), Some("ru"));

        let (table_id, table) = catalog
            .query_table_by_id("query_table:ОсновнаяТаблица")
            .expect("query table must resolve by id");
        assert_eq!(catalog.string(table.id), "query_table:ОсновнаяТаблица");
        assert!(catalog.query_tables().any(|(id, _)| id == table_id));
        assert_eq!(
            catalog
                .query_tables_by_name("Основная таблица")
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![table_id]
        );
        let by_temporary_table_name = {
            let name = String::from("Основная таблица");
            catalog.query_tables_by_name(&name)
        };
        assert_eq!(
            by_temporary_table_name
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![table_id]
        );
        assert_eq!(
            catalog
                .query_tables_by_identifier("ОсновнаяТаблица")
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![table_id]
        );
        assert_eq!(
            catalog
                .query_tables_by_syntax("ОсновнаяТаблица.<Имя таблицы>")
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![table_id]
        );

        let fields = catalog.query_fields(table_id).collect::<Vec<_>>();
        assert_eq!(fields.len(), 2);
        let (period_field_id, period_field) = catalog
            .query_field_by_id("query_table_field:query_table:ОсновнаяТаблица:Период")
            .expect("query field must resolve by id");
        assert_eq!(catalog.string(period_field.id), "query_table_field:query_table:ОсновнаяТаблица:Период");
        assert_eq!(period_field.owner, table_id);
        assert!(fields.iter().any(|(id, _)| *id == period_field_id));
        assert_eq!(
            catalog
                .query_field_by_name(table_id, "Период")
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![period_field_id]
        );
        let by_temporary_field_name = {
            let name = String::from("Период");
            catalog.query_field_by_name(table_id, &name)
        };
        assert_eq!(
            by_temporary_field_name
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![period_field_id]
        );

        let parameters = catalog.query_parameters(table_id).collect::<Vec<_>>();
        assert_eq!(parameters.len(), 1);
        let (date_parameter_id, date_parameter) = catalog
            .query_parameter_by_id("query_table_parameter:query_table:ОсновнаяТаблица:Дата")
            .expect("query parameter must resolve by id");
        assert_eq!(
            catalog.string(date_parameter.id),
            "query_table_parameter:query_table:ОсновнаяТаблица:Дата"
        );
        assert_eq!(date_parameter.owner, table_id);
        assert_eq!(parameters[0].0, date_parameter_id);
        assert_eq!(
            catalog
                .query_parameter_by_name(table_id, "Дата")
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![date_parameter_id]
        );
        let by_temporary_parameter_name = {
            let name = String::from("Дата");
            catalog.query_parameter_by_name(table_id, &name)
        };
        assert_eq!(
            by_temporary_parameter_name
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![date_parameter_id]
        );

        for (local_id, expected) in [
            (
                "query_table:Справочник",
                Some("metadata.sdbl.query-source.catalog"),
            ),
            (
                "query_table:Документ",
                Some("metadata.sdbl.query-source.document"),
            ),
            (
                "query_table:РегистрСведений",
                Some("metadata.sdbl.query-source.information-register"),
            ),
            (
                "query_table:РегистрНакопления",
                Some("metadata.sdbl.query-source.accumulation-register"),
            ),
            (
                "query_table:РегистрБухгалтерии",
                Some("metadata.sdbl.query-source.accounting-register"),
            ),
            (
                "query_table:РегистрРасчета",
                Some("metadata.sdbl.query-source.calculation-register"),
            ),
            ("query_table:БизнесПроцесс", None),
        ] {
            let (id, _) = catalog
                .query_table_by_id(local_id)
                .expect("selector table must resolve");
            assert_eq!(catalog.metadata_source_selector(id), expected);
        }
        assert_eq!(
            catalog.metadata_source_selector_for_identifier(Some("Unknown")),
            None
        );
    }

    #[test]
    fn sdbl_catalog_matches_snapshot_adapter_projection_boundary() {
        let query_source = SourceId::new("custom-query");
        let platform_source = SourceId::new("custom-platform");
        let index_path = fixture_index_path("sdbl-catalog-snapshot-parity.sqlite");
        let index = open_index(&index_path);
        let snapshot = Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must build"));
        drop(index);
        std::fs::remove_file(&index_path).expect("projection paths must not need SQLite file");

        let adapter = QueryTableSnapshotSource::with_source_ids(
            snapshot.clone(),
            query_source.clone(),
            platform_source.clone(),
        );
        let catalog = crate::HbkSdblQueryCatalog::with_source_ids(
            snapshot,
            query_source.clone(),
            platform_source.clone(),
        );
        let context = ResolveContext::all();
        let fact_id = |kind, local_id: &str| {
            FactId::new(
                query_source.clone(),
                LanguageDomain::QueryLanguage,
                kind,
                local_id,
            )
        };
        let assert_type_ref_projection = |raw: &syntax_helper_search::HbkTypeRef,
                                          projected: &TypeRef| {
            assert_eq!(projected.name, catalog.string(raw.name));
            match (&raw.target, &projected.target) {
                (syntax_helper_search::HbkTypeRefTarget::Ok(raw_id), TypeRefTarget::Ok(id)) => {
                    assert_eq!(id.0.source, platform_source);
                    assert_eq!(id.0.domain, LanguageDomain::PlatformApi);
                    assert_eq!(id.0.kind, FactKind::Type);
                    assert_eq!(id.0.local_id, catalog.string(*raw_id));
                }
                (
                    syntax_helper_search::HbkTypeRefTarget::Unresolved,
                    TypeRefTarget::Unresolved,
                ) => {}
                (
                    syntax_helper_search::HbkTypeRefTarget::Ambiguous(raw_candidates),
                    TypeRefTarget::Ambiguous(candidates),
                ) => {
                    assert_eq!(candidates.len(), raw_candidates.len());
                    for (raw_id, id) in raw_candidates.iter().zip(candidates) {
                        assert_eq!(id.0.source, platform_source);
                        assert_eq!(id.0.domain, LanguageDomain::PlatformApi);
                        assert_eq!(id.0.kind, FactKind::Type);
                        assert_eq!(id.0.local_id, catalog.string(*raw_id));
                    }
                }
                (raw, projected) => panic!("type-ref target mismatch: {raw:?} != {projected:?}"),
            }
            match (&raw.template_binding, &projected.template_binding) {
                (None, None) => {}
                (Some(raw_binding), Some(binding)) => {
                    assert_eq!(
                        binding.template_key.family,
                        catalog.string(raw_binding.template_key.family)
                    );
                    assert_eq!(
                        binding.template_key.variant,
                        catalog.string(raw_binding.template_key.variant)
                    );
                    assert_eq!(binding.arguments.len(), raw_binding.arguments.len());
                    for (raw_argument, argument) in
                        raw_binding.arguments.iter().zip(&binding.arguments)
                    {
                        match (raw_argument, argument) {
                            (
                                syntax_helper_search::model::TemplateParameterBinding::OwnerParameter {
                                    owner_parameter_index: raw_owner,
                                    target_parameter_index: raw_target,
                                },
                                TemplateParameterBinding::OwnerParameter {
                                    owner_parameter_index,
                                    target_parameter_index,
                                },
                            ) => {
                                assert_eq!(owner_parameter_index, raw_owner);
                                assert_eq!(target_parameter_index, raw_target);
                            }
                        }
                    }
                }
                (raw, projected) => {
                    panic!("type-ref template binding mismatch: {raw:?} != {projected:?}")
                }
            }
        };

        let global_context = adapter
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Sdbl,
                    sources: &[],
                },
                &context,
            )
            .expect("SDBL global context projection must not fail");
        assert_eq!(global_context.status, ResolveStatus::Ok);
        let global_facts = global_context
            .facts
            .iter()
            .flat_map(|context| context.facts.iter())
            .collect::<Vec<_>>();

        let global_table_ids = global_facts
            .iter()
            .filter(|fact| fact.id.kind == FactKind::QueryTable)
            .map(|fact| fact.id.local_id.as_str())
            .collect::<Vec<_>>();
        let catalog_table_ids = catalog
            .query_tables()
            .map(|(_, table)| catalog.string(table.id))
            .collect::<Vec<_>>();
        assert_eq!(global_table_ids, catalog_table_ids);

        let (table_id, table) = catalog
            .query_table_by_id("query_table:ОсновнаяТаблица")
            .expect("catalog table id lookup must resolve");
        let table_fact_id = fact_id(FactKind::QueryTable, catalog.string(table.id));
        let table_response = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&table_fact_id),
                &context,
            )
            .expect("snapshot table id lookup must not fail");
        assert_eq!(table_response.status, ResolveStatus::Ok);
        assert_eq!(table_response.facts.len(), 1);
        let table_fact = &table_response.facts[0];
        assert_eq!(table_fact.id, table_fact_id);
        let FactDetails::QueryTable(table_info) = &table_fact.details else {
            panic!("snapshot table must expose query table details");
        };
        assert_eq!(table_fact.name.primary, catalog.string(table.name.primary));
        assert_eq!(
            table_fact.name.alias.as_deref(),
            table.name.alias.map(|id| catalog.string(id))
        );
        assert_eq!(
            table_info.syntax.as_ref().map(|name| name.primary.as_str()),
            table.syntax.as_ref().map(|name| catalog.string(name.primary))
        );
        assert_eq!(
            table_info.syntax.as_ref().and_then(|name| name.alias.as_deref()),
            table
                .syntax
                .as_ref()
                .and_then(|name| name.alias.map(|id| catalog.string(id)))
        );
        assert_eq!(
            table_info.identifier.as_deref(),
            table.identifier.map(|id| catalog.string(id))
        );
        assert_eq!(table_info.table_role, QueryTableRole::Primary);
        assert_eq!(
            table_info.template_parameters,
            table
                .template_parameters
                .iter()
                .map(|id| catalog.string(*id).to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            table_info
                .owner_path
                .iter()
                .map(|name| (name.primary.as_str(), name.alias.as_deref()))
                .collect::<Vec<_>>(),
            table
                .owner_path
                .iter()
                .map(|name| {
                    (
                        catalog.string(name.primary),
                        name.alias.map(|id| catalog.string(id)),
                    )
                })
                .collect::<Vec<_>>()
        );

        for (name, expected) in [
            ("Основная таблица", &table_fact_id),
            ("ОсновнаяТаблица.<Имя таблицы>", &table_fact_id),
            ("ОсновнаяТаблица", &table_fact_id),
        ] {
            let facts = adapter
                .resolve(
                    context_resolver_core::ResolveQuery::ExactName {
                        source: Some(&query_source),
                        domain: Some(LanguageDomain::QueryLanguage),
                        kind: Some(FactKind::QueryTable),
                        name,
                    },
                    &context,
                )
                .expect("snapshot table point lookup must not fail")
                .facts;
            assert_eq!(facts.iter().map(|fact| &fact.id).collect::<Vec<_>>(), vec![expected]);
        }
        assert_eq!(
            catalog
                .query_tables_by_name("Основная таблица")
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![table_id]
        );
        assert_eq!(
            catalog
                .query_tables_by_syntax("ОсновнаяТаблица.<Имя таблицы>")
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![table_id]
        );
        assert_eq!(
            catalog
                .query_tables_by_identifier("ОсновнаяТаблица")
                .map(|(id, _)| id)
                .collect::<Vec<_>>(),
            vec![table_id]
        );

        let fields = adapter
            .query_fields(&table_fact_id, &context)
            .expect("snapshot field enumeration must not fail");
        assert_eq!(fields.status, ResolveStatus::Ok);
        let catalog_fields = catalog.query_fields(table_id).collect::<Vec<_>>();
        assert_eq!(fields.facts.len(), catalog_fields.len());
        assert_eq!(
            fields
                .facts
                .iter()
                .map(|fact| fact.id.local_id.as_str())
                .collect::<Vec<_>>(),
            catalog_fields
                .iter()
                .map(|(_, field)| catalog.string(field.id))
                .collect::<Vec<_>>()
        );
        for (field_id, field) in catalog_fields {
            let local_id = catalog.string(field.id);
            let expected_id = fact_id(FactKind::QueryField, local_id);
            let field_fact = fields
                .facts
                .iter()
                .find(|fact| fact.id == expected_id)
                .expect("snapshot field enumeration must include catalog field");
            assert_eq!(field_fact.owner.as_ref(), Some(&table_fact_id));
            assert_eq!(field_fact.name.primary, catalog.string(field.name.primary));
            assert_eq!(
                field_fact.name.alias.as_deref(),
                field.name.alias.map(|id| catalog.string(id))
            );
            let FactDetails::QueryField(field_info) = &field_fact.details else {
                panic!("snapshot field must expose query field details");
            };
            assert_eq!(field_info.owner, table_fact_id);
            assert_eq!(field_info.types.len(), field.type_refs.len());
            for (raw, projected) in field.type_refs.iter().zip(&field_info.types) {
                assert_type_ref_projection(raw, projected);
            }
            assert_eq!(
                field_info.note.as_deref(),
                field.note.map(|id| catalog.string(id))
            );
            assert_eq!(
                catalog
                    .query_field_by_name(table_id, catalog.string(field.name.primary))
                    .map(|(id, _)| id)
                    .collect::<Vec<_>>(),
                vec![field_id]
            );
            assert_eq!(
                adapter
                    .query_fields_by_name(&table_fact_id, catalog.string(field.name.primary), &context)
                    .expect("snapshot field point lookup must not fail")
                    .facts,
                vec![field_fact.clone()]
            );
        }

        let parameters = adapter
            .query_parameters(&table_fact_id, &context)
            .expect("snapshot parameter enumeration must not fail");
        assert_eq!(parameters.status, ResolveStatus::Ok);
        let catalog_parameters = catalog.query_parameters(table_id).collect::<Vec<_>>();
        assert_eq!(parameters.facts.len(), catalog_parameters.len());
        assert_eq!(
            parameters
                .facts
                .iter()
                .map(|fact| fact.id.local_id.as_str())
                .collect::<Vec<_>>(),
            catalog_parameters
                .iter()
                .map(|(_, parameter)| catalog.string(parameter.id))
                .collect::<Vec<_>>()
        );
        for (parameter_id, parameter) in catalog_parameters {
            let local_id = catalog.string(parameter.id);
            let expected_id = fact_id(FactKind::QueryParameter, local_id);
            let parameter_fact = parameters
                .facts
                .iter()
                .find(|fact| fact.id == expected_id)
                .expect("snapshot parameter enumeration must include catalog parameter");
            assert_eq!(parameter_fact.owner.as_ref(), Some(&table_fact_id));
            assert_eq!(parameter_fact.name.primary, catalog.string(parameter.name.primary));
            assert_eq!(
                parameter_fact.name.alias.as_deref(),
                parameter.name.alias.map(|id| catalog.string(id))
            );
            let FactDetails::QueryParameter(parameter_info) = &parameter_fact.details else {
                panic!("snapshot parameter must expose query parameter details");
            };
            assert_eq!(parameter_info.owner, table_fact_id);
            assert_eq!(parameter_info.types.len(), parameter.type_refs.len());
            for (raw, projected) in parameter.type_refs.iter().zip(&parameter_info.types) {
                assert_type_ref_projection(raw, projected);
            }
            assert_eq!(
                parameter_info.default_value.as_deref(),
                parameter.default_value.map(|id| catalog.string(id))
            );
            assert_eq!(
                catalog
                    .query_parameter_by_name(table_id, catalog.string(parameter.name.primary))
                    .map(|(id, _)| id)
                    .collect::<Vec<_>>(),
                vec![parameter_id]
            );
            assert_eq!(
                adapter
                    .query_parameters_by_name(
                        &table_fact_id,
                        catalog.string(parameter.name.primary),
                        &context,
                    )
                    .expect("snapshot parameter point lookup must not fail")
                    .facts,
                vec![parameter_fact.clone()]
            );
        }

        for local_id in [
            "query_table:Справочник",
            "query_table:Документ",
            "query_table:РегистрСведений",
            "query_table:РегистрНакопления",
            "query_table:РегистрБухгалтерии",
            "query_table:РегистрРасчета",
        ] {
            let (selector_table_id, _) = catalog
                .query_table_by_id(local_id)
                .expect("catalog selector table must resolve");
            let selector_response = adapter
                .resolve(
                    context_resolver_core::ResolveQuery::Id(&fact_id(FactKind::QueryTable, local_id)),
                    &context,
                )
                .expect("snapshot selector table lookup must not fail");
            assert_eq!(selector_response.status, ResolveStatus::Ok);
            let FactDetails::QueryTable(selector_info) = &selector_response.facts[0].details else {
                panic!("snapshot selector table must expose query table details");
            };
            assert_eq!(
                selector_info.sdbl_metadata_source_selector.as_deref(),
                catalog.metadata_source_selector(selector_table_id)
            );
        }
    }

    #[test]
    fn sdbl_catalog_and_sql_projection_gate_selectors_by_locale() {
        let path = temp_path("sdbl-catalog-non-ru-selector.sqlite");
        let mut builder = SearchIndexBuilder::new();
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:Справочник".to_string()),
                name: "Catalog table".to_string(),
                syntax: Some(name("Справочник.<Имя справочника>", Some("Catalog.<Catalog name>"))),
                identifier: Some("Справочник".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Primary,
                description: Some("Non-ru selector fixture.".to_string()),
                source: source_ref_with_locale("query-table-catalog-en", "en"),
            })
            .expect("non-ru query table must sink");
        build_index_from_builder(&path, &metadata_with_locale("en"), builder)
            .expect("non-ru query table index must build");

        let index = SearchIndex::open_read_only(&path).expect("non-ru index must open");
        let sql_adapter = LanguageSearchSource::new_query_tables(
            "shcntx-query",
            fixture_source(),
            index,
        );
        let table_id = FactId::new(
            SourceId::new("shcntx-query"),
            LanguageDomain::QueryLanguage,
            FactKind::QueryTable,
            "query_table:Справочник",
        );
        let response = sql_adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&table_id),
                &ResolveContext::all(),
            )
            .expect("non-ru SQL query table lookup must not fail");
        let FactDetails::QueryTable(info) = &response.facts[0].details else {
            panic!("non-ru SQL query table must expose table details");
        };
        assert_eq!(info.sdbl_metadata_source_selector, None);

        let snapshot = Arc::new(
            HbkFactSnapshot::from_index(&sql_adapter.index).expect("non-ru snapshot must build"),
        );
        let catalog = crate::HbkSdblQueryCatalog::new(snapshot);
        let (catalog_table_id, _) = catalog
            .query_table_by_id("query_table:Справочник")
            .expect("non-ru catalog table must resolve");
        assert_eq!(catalog.source_locale(), Some("en"));
        assert_eq!(catalog.metadata_source_selector(catalog_table_id), None);
        assert_eq!(
            crate::hbk_catalogs::sdbl::sdbl_metadata_source_selector(
                Some("ru"),
                Some("Справочник")
            ),
            Some("metadata.sdbl.query-source.catalog")
        );
        assert_eq!(
            crate::hbk_catalogs::sdbl::sdbl_metadata_source_selector(
                Some("en"),
                Some("Справочник")
            ),
            None
        );
        assert_eq!(
            crate::hbk_catalogs::sdbl::sdbl_metadata_source_selector(Some("ru"), Some("Unknown")),
            None
        );
    }

    #[test]
    fn borrowed_catalog_source_guard_keeps_typed_owners_and_projection_boundary() {
        let bsl_catalog = include_str!("hbk_catalogs/bsl.rs");
        let sdbl_catalog = include_str!("hbk_catalogs/sdbl.rs");
        let catalog_mod = include_str!("hbk_catalogs/mod.rs");
        let imports = include_str!("imports.rs");
        let core = include_str!("../../context-resolver-core/src/lib.rs");
        let language_adapter = include_str!("language_adapter.rs");
        let mapping = include_str!("mapping.rs");
        let snapshot_adapter = include_str!("snapshot_adapter.rs");
        let platform_snapshot_adapter = snapshot_adapter
            .split("impl QueryTableSnapshotSource")
            .next()
            .expect("platform snapshot implementation must precede query snapshot implementation");
        let lib = include_str!("lib.rs");

        for (path, source) in [
            ("hbk_catalogs/bsl.rs", bsl_catalog),
            ("hbk_catalogs/sdbl.rs", sdbl_catalog),
            ("hbk_catalogs/mod.rs", catalog_mod),
        ] {
            for forbidden in [
                "ContextFact",
                "Resolved",
                "SearchIndex",
                "rusqlite",
                "from_index",
                "pub trait",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "{path} must keep borrowed catalog APIs typed and must not own {forbidden}"
                );
            }
        }

        assert!(
            !bsl_catalog.contains("(&[StringId], Option<StringId>)")
                && !bsl_catalog.contains("Option<StringId>)"),
            "public BSL catalog availability must not expose raw context/version StringId values"
        );
        assert_eq!(
            count_occurrences(mapping, "pub fn project_hbk_fact_id("),
            1,
            "stable HBK FactId projection must have one public owner"
        );
        assert_eq!(
            count_occurrences(mapping, "pub fn project_hbk_member_kind("),
            1,
            "HBK member-kind projection must have one public owner"
        );
        assert_eq!(
            count_occurrences(mapping, "pub fn project_hbk_callable_kind("),
            1,
            "HBK callable-kind projection must have one public owner"
        );
        assert_eq!(
            count_occurrences(mapping, "pub fn project_hbk_member_query_kind("),
            1,
            "core member-query to HBK member-kind projection must have one public owner"
        );
        assert_eq!(
            count_occurrences(mapping, "pub fn project_hbk_callable_fact_id("),
            1,
            "HBK callable FactId projection must have one public owner"
        );
        assert_eq!(
            count_occurrences(mapping, "pub fn project_hbk_type_ref("),
            1,
            "HBK type-reference projection must have one public owner"
        );
        assert_eq!(
            count_occurrences(mapping, "pub fn project_hbk_signature("),
            1,
            "HBK signature projection must have one public owner"
        );
        for projection in [
            "project_hbk_fact_id",
            "project_hbk_member_kind",
            "project_hbk_callable_kind",
            "project_hbk_member_query_kind",
            "project_hbk_callable_fact_id",
            "project_hbk_type_ref",
            "project_hbk_signature",
        ] {
            assert!(
                platform_snapshot_adapter.contains(projection),
                "snapshot compatibility adapter must reuse {projection}"
            );
            assert!(
                !bsl_catalog.contains(projection),
                "borrowed catalog must not emit generic projected payloads through {projection}"
            );
        }
        assert!(
            !platform_snapshot_adapter.contains("fn map_type_ref(")
                && !platform_snapshot_adapter.contains("fn map_type_ref_target("),
            "platform snapshot adapter must not retain a second HBK type-reference projection"
        );
        assert!(
            !contains_hbk_kind_projection_duplicate(platform_snapshot_adapter),
            "platform snapshot adapter must not retain a second HBK kind projection"
        );
        assert!(
            !contains_hbk_callable_fact_kind_classifier(platform_snapshot_adapter),
            "platform snapshot adapter must not retain a second callable FactKind classifier"
        );
        assert!(
            contains_hbk_kind_projection_duplicate(
                r#"
                fn member_query_kind_to_snapshot(kind: MemberQueryKind) -> HbkTypeMemberKind {
                    match kind {
                        MemberQueryKind::Property => HbkTypeMemberKind::Property,
                        MemberQueryKind::Method => HbkTypeMemberKind::Method,
                        MemberQueryKind::Event => HbkTypeMemberKind::Event,
                        MemberQueryKind::EnumValue => HbkTypeMemberKind::EnumValue,
                    }
                }
                "#,
            ),
            "projection guard must reject the former member-query helper"
        );
        assert!(
            contains_hbk_callable_fact_kind_classifier(
                r#"
                let fact_kind = if callable.kind == HbkCallableKind::Constructor {
                    FactKind::Constructor
                } else {
                    FactKind::Callable
                };
                "#,
            ),
            "projection guard must reject a repeated constructor classifier"
        );
        assert_eq!(
            count_occurrences(mapping, "fn availability_context_from_code("),
            1,
            "availability code decoding must keep one production owner"
        );
        assert_eq!(
            count_occurrences(core, "pub fn metadata_module_context_kind("),
            1,
            "metadata module-role translation must have one public core owner"
        );
        assert!(
            !contains_metadata_module_role_mapping(mapping)
                && !contains_metadata_module_role_mapping(bsl_catalog)
                && !contains_metadata_module_role_mapping(snapshot_adapter)
                && !contains_metadata_module_role_mapping(language_adapter),
            "search/catalog code must not duplicate core metadata module-role selector literals"
        );
        assert!(
            !contains_section8_projection_holder_or_selector_wrapper(mapping)
                && !contains_section8_projection_holder_or_selector_wrapper(bsl_catalog)
                && !contains_section8_projection_holder_or_selector_wrapper(snapshot_adapter)
                && !contains_section8_projection_holder_or_selector_wrapper(language_adapter)
                && !contains_section8_projection_holder_or_selector_wrapper(lib),
            "Section 8 must not introduce projection holders or selector wrappers"
        );
        assert!(
            contains_metadata_module_role_mapping(
                r#"
                match selector {
                    "metadata.module-role.object" => Some(ModuleContextKind::Object),
                    _ => None,
                }
                "#,
            ) && contains_metadata_module_role_mapping(
                r#"
                match module_kind {
                    ModuleKind::Object => ModuleContextKind::Object,
                    ModuleKind::Manager => ModuleContextKind::Manager,
                    ModuleKind::Form => ModuleContextKind::Form,
                }
                "#,
            ),
            "metadata module-role guard must reject literal and ModuleKind tables"
        );
        assert!(
            contains_section8_projection_holder_or_selector_wrapper(
                "struct HbkProjectionOwner { source: SourceId }"
            ) && contains_section8_projection_holder_or_selector_wrapper(
                "struct MetadataModuleRoleSelector(String);"
            ),
            "Section 8 guard must reject projection holders and selector wrappers"
        );

        for (path, source) in [
            ("hbk_catalogs/bsl.rs", bsl_catalog),
            ("hbk_catalogs/sdbl.rs", sdbl_catalog),
            ("hbk_catalogs/mod.rs", catalog_mod),
            ("lib.rs", lib),
        ] {
            for forbidden in ["-> HbkFactRef", "Item = HbkFactRef", "pub use HbkFactRef"] {
                assert!(
                    !source.contains(forbidden),
                    "{path} must not export HbkFactRef through the catalog API"
                );
            }
        }

        assert_snapshot_source_stores_only_catalog(
            imports,
            "PlatformSnapshotSource",
            "catalog: HbkBslContextCatalog,",
        );
        assert_snapshot_source_stores_only_catalog(
            imports,
            "QueryTableSnapshotSource",
            "catalog: HbkSdblQueryCatalog,",
        );

        for selector in [
            "metadata.sdbl.query-source.catalog",
            "metadata.sdbl.query-source.document",
            "metadata.sdbl.query-source.information-register",
            "metadata.sdbl.query-source.accumulation-register",
            "metadata.sdbl.query-source.accounting-register",
            "metadata.sdbl.query-source.calculation-register",
        ] {
            assert_eq!(
                count_occurrences(sdbl_catalog, selector),
                1,
                "{selector} must have exactly one owner in hbk_catalogs/sdbl.rs"
            );
            for (path, source) in [
                ("hbk_catalogs/bsl.rs", bsl_catalog),
                ("mapping.rs", mapping),
                ("language_adapter.rs", language_adapter),
                ("snapshot_adapter.rs", snapshot_adapter),
                ("imports.rs", imports),
                ("lib.rs", lib),
            ] {
                assert!(
                    !source.contains(selector),
                    "{selector} must not be duplicated in {path}"
                );
            }
        }

        assert!(
            snapshot_adapter.contains("ContextFact")
                && snapshot_adapter.contains("ResolvedType")
                && snapshot_adapter.contains("ResolvedGlobalContext"),
            "generic DTO projection must remain in the snapshot adapter compatibility boundary"
        );
    }

    #[test]
    #[ignore = "measurement probe; run explicitly for SDBL catalog compatibility measurements"]
    fn sdbl_catalog_measurement_probe() {
        let query_source = SourceId::new("shcntx-query");
        let platform_source = fixture_source();
        let index_path = fixture_index_path("sdbl-compat-measurement-probe.sqlite");
        let index = open_index(&index_path);
        let snapshot = Arc::new(HbkFactSnapshot::from_index(&index).expect("snapshot must build"));
        drop(index);
        std::fs::remove_file(&index_path).expect("measurement probe must not need SQLite file");
        println!("compat_deleted_sqlite_success=1");

        let adapter = QueryTableSnapshotSource::with_source_ids(
            snapshot.clone(),
            query_source.clone(),
            platform_source.clone(),
        );
        compat_sdbl_adapter_sequence(&adapter, &query_source);

        let catalog =
            crate::HbkSdblQueryCatalog::with_source_ids(snapshot, query_source, platform_source);
        println!("direct_deleted_sqlite_success=1");
        direct_sdbl_catalog_sequence(&catalog);
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

        for (local_id, expected) in [
            (
                "query_table:Справочник",
                Some("metadata.sdbl.query-source.catalog"),
            ),
            (
                "query_table:Документ",
                Some("metadata.sdbl.query-source.document"),
            ),
            (
                "query_table:РегистрСведений",
                Some("metadata.sdbl.query-source.information-register"),
            ),
            (
                "query_table:РегистрНакопления",
                Some("metadata.sdbl.query-source.accumulation-register"),
            ),
            (
                "query_table:РегистрБухгалтерии",
                Some("metadata.sdbl.query-source.accounting-register"),
            ),
            (
                "query_table:РегистрРасчета",
                Some("metadata.sdbl.query-source.calculation-register"),
            ),
            ("query_table:БизнесПроцесс", None),
            (
                "query_table:РегистрСведенийТаблицаСрезаПоследних",
                None,
            ),
        ] {
            let response = adapter
                .resolve(
                    context_resolver_core::ResolveQuery::Id(&FactId::new(
                        SourceId::new("shcntx-query"),
                        LanguageDomain::QueryLanguage,
                        FactKind::QueryTable,
                        local_id,
                    )),
                    &ResolveContext::all(),
                )
                .expect("query table selector lookup must not fail");
            assert_eq!(response.status, ResolveStatus::Ok);
            let FactDetails::QueryTable(info) = &response.facts[0].details else {
                panic!("query table fact must expose query table details");
            };
            assert_eq!(info.sdbl_metadata_source_selector.as_deref(), expected);
        }

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

    fn compat_sdbl_adapter_sequence(adapter: &QueryTableSnapshotSource, query_source: &SourceId) {
        let context = ResolveContext::all();
        let global = adapter
            .global_context(
                GlobalContextQuery::Language {
                    language: GlobalContextLanguage::Sdbl,
                    sources: &[],
                },
                &context,
            )
            .expect("compat SDBL global context must not fail");
        assert_eq!(global.status, ResolveStatus::Ok);
        println!("compat_global_context_invocations=1");
        println!("compat_global_response_count={}", global.facts.len());
        let global_facts = global
            .facts
            .iter()
            .flat_map(|context| context.facts.iter())
            .collect::<Vec<_>>();
        println!("compat_global_fact_total={}", global_facts.len());
        println!(
            "compat_global_query_table_count={}",
            global_facts
                .iter()
                .filter(|fact| fact.id.kind == FactKind::QueryTable)
                .count()
        );
        println!(
            "compat_global_query_field_count={}",
            global_facts
                .iter()
                .filter(|fact| fact.id.kind == FactKind::QueryField)
                .count()
        );
        println!(
            "compat_global_query_parameter_count={}",
            global_facts
                .iter()
                .filter(|fact| fact.id.kind == FactKind::QueryParameter)
                .count()
        );

        let table_id = FactId::new(
            query_source.clone(),
            LanguageDomain::QueryLanguage,
            FactKind::QueryTable,
            "query_table:ОсновнаяТаблица",
        );
        let table = adapter
            .resolve(
                context_resolver_core::ResolveQuery::Id(&table_id),
                &context,
            )
            .expect("compat SDBL table id lookup must not fail");
        assert_eq!(table.status, ResolveStatus::Ok);
        println!("compat_table_id_response_count={}", table.facts.len());
        let exact = adapter
            .resolve(
                context_resolver_core::ResolveQuery::ExactName {
                    source: Some(query_source),
                    domain: Some(LanguageDomain::QueryLanguage),
                    kind: Some(FactKind::QueryTable),
                    name: "Основная таблица",
                },
                &context,
            )
            .expect("compat SDBL exact name lookup must not fail");
        assert_eq!(exact.status, ResolveStatus::Ok);
        println!("compat_table_exact_response_count={}", exact.facts.len());

        let fields = adapter
            .query_fields(&table_id, &context)
            .expect("compat SDBL field enumeration must not fail");
        assert_eq!(fields.status, ResolveStatus::Ok);
        println!("compat_field_enum_count={}", fields.facts.len());
        let field_exact = adapter
            .query_fields_by_name(&table_id, "Период", &context)
            .expect("compat SDBL field exact lookup must not fail");
        assert_eq!(field_exact.status, ResolveStatus::Ok);
        println!("compat_field_exact_count={}", field_exact.facts.len());

        let parameters = adapter
            .query_parameters(&table_id, &context)
            .expect("compat SDBL parameter enumeration must not fail");
        assert_eq!(parameters.status, ResolveStatus::Ok);
        println!("compat_parameter_enum_count={}", parameters.facts.len());
        let parameter_exact = adapter
            .query_parameters_by_name(&table_id, "Дата", &context)
            .expect("compat SDBL parameter exact lookup must not fail");
        assert_eq!(parameter_exact.status, ResolveStatus::Ok);
        println!(
            "compat_parameter_exact_count={}",
            parameter_exact.facts.len()
        );

        let selector_projection_count = [
            "query_table:Справочник",
            "query_table:Документ",
            "query_table:РегистрСведений",
            "query_table:РегистрНакопления",
            "query_table:РегистрБухгалтерии",
            "query_table:РегистрРасчета",
        ]
        .into_iter()
        .filter(|local_id| {
            let response = adapter
                .resolve(
                    context_resolver_core::ResolveQuery::Id(&FactId::new(
                        query_source.clone(),
                        LanguageDomain::QueryLanguage,
                        FactKind::QueryTable,
                        *local_id,
                    )),
                    &context,
                )
                .expect("compat SDBL selector lookup must not fail");
            assert_eq!(response.status, ResolveStatus::Ok);
            response.facts.iter().any(|fact| {
                matches!(
                    &fact.details,
                    FactDetails::QueryTable(info)
                        if info.sdbl_metadata_source_selector.is_some()
                )
            })
        })
        .count();
        println!(
            "compat_selector_projection_count={}",
            selector_projection_count
        );
    }

    fn direct_sdbl_catalog_sequence(catalog: &crate::HbkSdblQueryCatalog) {
        println!(
            "direct_source_locale_present={}",
            usize::from(catalog.source_locale().is_some())
        );
        println!("direct_all_table_count={}", catalog.query_tables().count());
        let (table_id, _) = catalog
            .query_table_by_id("query_table:ОсновнаяТаблица")
            .expect("direct SDBL primary table must resolve");
        println!("direct_primary_table_point_count=1");
        println!(
            "direct_table_name_count={}",
            catalog.query_tables_by_name("Основная таблица").count()
        );
        println!(
            "direct_table_syntax_count={}",
            catalog
                .query_tables_by_syntax("ОсновнаяТаблица.<Имя таблицы>")
                .count()
        );
        println!(
            "direct_table_identifier_count={}",
            catalog
                .query_tables_by_identifier("ОсновнаяТаблица")
                .count()
        );
        println!(
            "direct_field_enum_count={}",
            catalog.query_fields(table_id).count()
        );
        println!(
            "direct_field_id_count={}",
            usize::from(
                catalog
                    .query_field_by_id("query_table_field:query_table:ОсновнаяТаблица:Период")
                    .is_some()
            )
        );
        println!(
            "direct_field_name_count={}",
            catalog.query_field_by_name(table_id, "Период").count()
        );
        println!(
            "direct_parameter_enum_count={}",
            catalog.query_parameters(table_id).count()
        );
        println!(
            "direct_parameter_id_count={}",
            usize::from(
                catalog
                    .query_parameter_by_id("query_table_parameter:query_table:ОсновнаяТаблица:Дата")
                    .is_some()
            )
        );
        println!(
            "direct_parameter_name_count={}",
            catalog.query_parameter_by_name(table_id, "Дата").count()
        );
        let selector_count = [
            "query_table:Справочник",
            "query_table:Документ",
            "query_table:РегистрСведений",
            "query_table:РегистрНакопления",
            "query_table:РегистрБухгалтерии",
            "query_table:РегистрРасчета",
        ]
        .into_iter()
        .filter(|local_id| {
            catalog
                .query_table_by_id(local_id)
                .and_then(|(id, _)| catalog.metadata_source_selector(id))
                .is_some()
        })
        .count();
        assert_eq!(selector_count, 6);
        println!("direct_selector_present_count={selector_count}");
        let unknown_selector_absent = catalog
            .metadata_source_selector_for_identifier(Some("Unknown"))
            .is_none();
        assert!(unknown_selector_absent);
        println!(
            "direct_unknown_selector_absent={}",
            usize::from(unknown_selector_absent)
        );
    }

    fn fixture_source() -> SourceId {
        SourceId::new("test-platform")
    }

    fn assert_platform_member_enumeration_contract(
        adapter: &impl ContextSource,
        source: &SourceId,
    ) {
        let filter = type_owner_id(source, "platform_type:ОтборКомпоновкиДанных");
        let all_filter_members = adapter
            .members(
                &filter,
                MemberQuery {
                    name: None,
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("unfiltered platform member enumeration must not fail");
        assert_eq!(all_filter_members.status, ResolveStatus::Ok);
        assert_member_present(
            &all_filter_members.facts,
            "Элементы",
            MemberKind::Property,
        );
        assert_member_present(&all_filter_members.facts, "Найти", MemberKind::Method);
        assert_member_present(&all_filter_members.facts, "ПередЗаписью", MemberKind::Event);

        assert_kind_filter(adapter, &filter, MemberQueryKind::Property, "Элементы");
        assert_kind_filter(adapter, &filter, MemberQueryKind::Method, "Найти");
        assert_kind_filter(
            adapter,
            &filter,
            MemberQueryKind::Event,
            "ПередЗаписью",
        );

        let enum_owner = type_owner_id(
            source,
            "enum:system:ОбновлениеПредопределенныхДанных",
        );
        let enum_members = adapter
            .members(
                &enum_owner,
                MemberQuery {
                    name: None,
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("unfiltered enum-value member enumeration must not fail");
        assert_eq!(enum_members.status, ResolveStatus::Ok);
        assert_member_present(&enum_members.facts, "Обновлять", MemberKind::EnumValue);
        assert_kind_filter(
            adapter,
            &enum_owner,
            MemberQueryKind::EnumValue,
            "Обновлять",
        );
        assert_named_enum_member_queries_are_not_found(adapter, &enum_owner);

        let empty_owner = type_owner_id(source, "platform_type:Дата");
        let empty = adapter
            .members(
                &empty_owner,
                MemberQuery {
                    name: None,
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("empty existing owner enumeration must not fail");
        assert_eq!(empty.status, ResolveStatus::Ok);
        assert!(empty.facts.is_empty());

        let absent_owner = type_owner_id(source, "platform_type:НетТакогоТипа");
        let absent = adapter
            .members(
                &absent_owner,
                MemberQuery {
                    name: None,
                    kind: None,
                },
                &ResolveContext::all(),
            )
            .expect("absent owner enumeration must not fail");
        assert_eq!(absent.status, ResolveStatus::NotFound);

        let inactive_source = [SourceId::new("other-platform")];
        let inactive = adapter
            .members(
                &filter,
                MemberQuery {
                    name: None,
                    kind: None,
                },
                &ResolveContext {
                    active_sources: &inactive_source,
                    domain: None,
                    scope: None,
                },
            )
            .expect("inactive source enumeration must not fail");
        assert_eq!(inactive.status, ResolveStatus::NotFound);
    }

    fn type_owner_id(source: &SourceId, local_id: &str) -> TypeId {
        TypeId(FactId::new(
            source.clone(),
            LanguageDomain::PlatformApi,
            FactKind::Type,
            local_id,
        ))
    }

    fn assert_kind_filter(
        adapter: &impl ContextSource,
        owner: &TypeId,
        kind: MemberQueryKind,
        expected_name: &str,
    ) {
        let response = adapter
            .members(
                owner,
                MemberQuery {
                    name: None,
                    kind: Some(kind),
                },
                &ResolveContext::all(),
            )
            .expect("kind-filtered member enumeration must not fail");
        assert_eq!(response.status, ResolveStatus::Ok);
        assert_eq!(response.facts.len(), 1);
        assert_eq!(response.facts[0].fact.name.primary, expected_name);
        assert_eq!(response.facts[0].info.kind.query_kind(), kind);
    }

    fn assert_named_enum_member_queries_are_not_found(
        adapter: &impl ContextSource,
        owner: &TypeId,
    ) {
        for kind in [
            None,
            Some(MemberQueryKind::EnumValue),
            Some(MemberQueryKind::Property),
        ] {
            let response = adapter
                .members(
                    owner,
                    MemberQuery {
                        name: Some("Обновлять"),
                        kind,
                    },
                    &ResolveContext::all(),
                )
                .expect("named enum-value member lookup must not fail");
            assert_eq!(response.status, ResolveStatus::NotFound);
            assert!(response.facts.is_empty());
        }
    }

    fn assert_member_present(facts: &[ResolvedMember], name: &str, kind: MemberKind) {
        assert!(
            facts
                .iter()
                .any(|fact| fact.fact.name.primary == name && fact.info.kind == kind),
            "{name} {kind:?} must be enumerated"
        );
    }

    fn fixture_index(file_name: &str) -> SearchIndex {
        let path = fixture_index_path(file_name);
        SearchIndex::open_read_only(path).expect("index must open")
    }

    fn ambiguous_module_member_index(file_name: &str) -> SearchIndex {
        let path = temp_path(file_name);
        let mut builder = SearchIndexBuilder::new();
        for (owner, alias) in [("ПерваяФорма", "FirstOnOpen"), ("ВтораяФорма", "SecondOnOpen")] {
            builder
                .global_context_event(module_event(
                    model::ModuleKind::Form,
                    &[owner],
                    "ПриОткрытии",
                    alias,
                ))
                .expect("ambiguous module event must sink");
        }
        build_index_from_builder(&path, &metadata(), builder)
            .expect("ambiguous module-event index must build");
        SearchIndex::open_read_only(path).expect("ambiguous module-event index must open")
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
                owner: name("СправочникМенеджер.<Имя справочника>", None),
                owner_identity: Some(
                    "platform_type:СправочникМенеджер.<Имя справочника>".to_string(),
                ),
                name: name("Метаданные", Some("Metadata")),
                semantic: model::SemanticContext::default(),
                usage: None,
                type_refs: vec![model::TypeRef {
                    name: "Структура".to_string(),
                }],
                description: Some("Метаданные справочника.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("catalog-manager-metadata"),
            })
            .expect("generated-self property must sink");
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
            .type_method(model::PlatformMethod {
                owner: name("СправочникМенеджер.<Имя справочника>", None),
                owner_identity: Some(
                    "platform_type:СправочникМенеджер.<Имя справочника>".to_string(),
                ),
                name: name("НайтиПоКоду", Some("FindByCode")),
                semantic: model::SemanticContext::default(),
                signatures: vec![model::Signature {
                    text: "НайтиПоКоду(<Код>)".to_string(),
                    parameters: vec![model::Parameter {
                        name: "Код".to_string(),
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
                description: Some("Ищет справочник по коду.".to_string()),
                facts: model::SectionFacts::default(),
                source: source_ref("catalog-manager-find-by-code"),
            })
            .expect("generated-self method must sink");
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
                identity: Some("query_table:Справочник".to_string()),
                name: "Таблица справочника".to_string(),
                syntax: Some(name(
                    "Справочник.<Имя справочника>",
                    Some("Catalog.<Catalog name>"),
                )),
                identifier: Some("Справочник".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Primary,
                description: Some("Catalog query source fact.".to_string()),
                source: source_ref("query-table-catalog"),
            })
            .expect("catalog query table must sink");
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:Документ".to_string()),
                name: "Таблица документа".to_string(),
                syntax: Some(name(
                    "Документ.<Имя документа>",
                    Some("Document.<Document name>"),
                )),
                identifier: Some("Документ".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Primary,
                description: Some("Document query source fact.".to_string()),
                source: source_ref("query-table-document"),
            })
            .expect("document query table must sink");
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:РегистрСведений".to_string()),
                name: "Таблица регистра сведений".to_string(),
                syntax: Some(name(
                    "РегистрСведений.<Имя регистра сведений>",
                    Some("InformationRegister.<Information register name>"),
                )),
                identifier: Some("РегистрСведений".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Primary,
                description: Some("Information register query source fact.".to_string()),
                source: source_ref("query-table-information-register"),
            })
            .expect("information register query table must sink");
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:РегистрНакопления".to_string()),
                name: "Таблица регистра накопления".to_string(),
                syntax: Some(name(
                    "РегистрНакопления.<Имя регистра накопления>",
                    Some("AccumulationRegister.<Accumulation register name>"),
                )),
                identifier: Some("РегистрНакопления".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Primary,
                description: Some("Accumulation register query source fact.".to_string()),
                source: source_ref("query-table-accumulation-register"),
            })
            .expect("accumulation register query table must sink");
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:РегистрБухгалтерии".to_string()),
                name: "Таблица регистра бухгалтерии".to_string(),
                syntax: Some(name(
                    "РегистрБухгалтерии.<Имя регистра бухгалтерии>",
                    Some("AccountingRegister.<Accounting register name>"),
                )),
                identifier: Some("РегистрБухгалтерии".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Primary,
                description: Some("Accounting register query source fact.".to_string()),
                source: source_ref("query-table-accounting-register"),
            })
            .expect("accounting register query table must sink");
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:РегистрРасчета".to_string()),
                name: "Таблица регистра расчета".to_string(),
                syntax: Some(name(
                    "РегистрРасчета.<Имя регистра расчета>",
                    Some("CalculationRegister.<Calculation register name>"),
                )),
                identifier: Some("РегистрРасчета".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Primary,
                description: Some("Calculation register query source fact.".to_string()),
                source: source_ref("query-table-calculation-register"),
            })
            .expect("calculation register query table must sink");
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:БизнесПроцесс".to_string()),
                name: "Таблица бизнес-процессов".to_string(),
                syntax: Some(name(
                    "БизнесПроцесс.<Имя бизнес-процесса>",
                    Some("BusinessProcess.<Business process name>"),
                )),
                identifier: Some("БизнесПроцесс".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Primary,
                description: Some("Deferred business process query source fact.".to_string()),
                source: source_ref("query-table-business-process"),
            })
            .expect("business process query table must sink");
        builder
            .query_table(model::QueryTable {
                identity: Some("query_table:РегистрСведенийТаблицаСрезаПоследних".to_string()),
                name: "Таблица среза последних".to_string(),
                syntax: Some(name(
                    "РегистрСведений.<Имя регистра сведений>.СрезПоследних",
                    Some("InformationRegister.<Information register name>.SliceLast"),
                )),
                identifier: Some("РегистрСведенийТаблицаСрезаПоследних".to_string()),
                semantic: model::SemanticContext::new(
                    model::BranchKind::QueryTables,
                    model::RecordFamily::QueryTable,
                ),
                table_role: model::QueryTableRole::Additional,
                description: Some("Deferred additional query source fact.".to_string()),
                source: source_ref("query-table-slice-last"),
            })
            .expect("slice-last query table must sink");
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
                    contexts: vec![
                        model::AvailabilityContext::ThinClient,
                        model::AvailabilityContext::Server,
                    ],
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
        source_ref_with_locale(title, "ru")
    }

    fn source_ref_with_locale(title: &str, locale: &str) -> model::SyntaxHelperSource {
        model::SyntaxHelperSource {
            hbk_path: PathBuf::from(format!("/fixtures/shcntx_{locale}.hbk")),
            locale: locale.to_string(),
            toc_path: Some(title.to_string()),
            html_path: format!("{title}.html"),
            page_title: title.to_string(),
        }
    }

    fn metadata() -> IndexMetadata {
        metadata_with_locale("ru")
    }

    fn metadata_with_locale(locale: &str) -> IndexMetadata {
        IndexMetadata {
            locale: locale.to_string(),
            source_locale: locale.to_string(),
            source_hbk: format!("/fixtures/shcntx_{locale}.hbk"),
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

    fn assert_snapshot_source_stores_only_catalog(
        source: &str,
        struct_name: &str,
        expected_body: &str,
    ) {
        let body = struct_body(source, struct_name);
        assert_eq!(
            collapse_whitespace(body),
            collapse_whitespace(expected_body),
            "{struct_name} must store only the borrowed catalog handle"
        );
        for forbidden in ["HbkFactSnapshot", "worker_handle", "SearchIndex"] {
            assert!(
                !body.contains(forbidden),
                "{struct_name} must not keep catalog-covered storage field {forbidden}"
            );
        }
    }

    fn struct_body<'a>(source: &'a str, struct_name: &str) -> &'a str {
        let marker = format!("pub struct {struct_name} {{");
        source
            .split_once(&marker)
            .unwrap_or_else(|| panic!("{struct_name} definition must exist"))
            .1
            .split_once("\n}")
            .unwrap_or_else(|| panic!("{struct_name} body must close"))
            .0
    }

    fn collapse_whitespace(source: &str) -> String {
        source.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn contains_hbk_kind_projection_duplicate(source: &str) -> bool {
        source.contains("fn member_kind_from_snapshot(")
            || source.contains("fn callable_kind_from_snapshot(")
            || source.contains("fn member_query_kind_to_snapshot(")
            || source.contains("HbkCallableKind::LanguageFunction => CallableKind::GlobalMethod")
            || (source.contains("MemberQueryKind::Property => HbkTypeMemberKind::Property")
                && source.contains("MemberQueryKind::Method => HbkTypeMemberKind::Method")
                && source.contains("MemberQueryKind::Event => HbkTypeMemberKind::Event")
                && source.contains("MemberQueryKind::EnumValue => HbkTypeMemberKind::EnumValue"))
    }

    fn contains_hbk_callable_fact_kind_classifier(source: &str) -> bool {
        source.contains("let fact_kind =")
            || (source.contains("HbkCallableKind::Constructor => FactKind::Constructor")
                && source.contains("FactKind::Callable"))
    }

    fn contains_metadata_module_role_mapping(source: &str) -> bool {
        source.contains("metadata.module-role.")
            || (source.contains("ModuleKind::Object => ModuleContextKind::Object")
                && source.contains("ModuleKind::Manager => ModuleContextKind::Manager"))
    }

    fn contains_section8_projection_holder_or_selector_wrapper(source: &str) -> bool {
        let compact = collapse_whitespace(source);
        compact.contains("struct HbkProjection")
            || compact.contains("struct HbkBslProjection")
            || compact.contains("struct ProjectionOwner")
            || compact.contains("struct MetadataModuleRoleSelector")
            || compact.contains("struct MetadataModuleContextSelector")
            || compact.contains("enum MetadataModuleRoleSelector")
            || compact.contains("enum MetadataModuleContextSelector")
    }

    fn count_occurrences(source: &str, needle: &str) -> usize {
        source.match_indices(needle).count()
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
