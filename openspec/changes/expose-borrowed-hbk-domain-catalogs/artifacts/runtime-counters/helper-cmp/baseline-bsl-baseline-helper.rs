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
