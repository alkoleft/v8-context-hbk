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
