fn syntax(command: SyntaxCommand) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        SyntaxCommand::Export { book, output } => syntax_export(book, output),
        SyntaxCommand::Index { book, output } => syntax_index(book, output),
        SyntaxCommand::Get {
            index,
            kind,
            id,
            name,
            alias,
            owner,
            owner_type_id,
            member,
            members_of,
            callable_id,
            callable,
            format,
        } => syntax_get(
            index,
            GetArgs {
                kind,
                id,
                name,
                alias,
                owner,
                owner_type_id,
                member,
                members_of,
                callable_id,
                callable,
            },
            format,
        ),
        SyntaxCommand::Constructors {
            index,
            name,
            details,
            format,
        } => syntax_constructors(index, &name, details, format),
        SyntaxCommand::Search {
            index,
            query,
            mode,
            limit,
            format,
        } => syntax_search(index, &query, mode, limit, format),
        SyntaxCommand::Related {
            index,
            id,
            name,
            owner,
            member,
            edge,
            depth,
            limit,
            compact,
            graph,
            format,
        } => syntax_related(
            index,
            RelatedArgs {
                id,
                name,
                owner,
                member,
                edge,
                depth,
                limit,
                compact,
                graph,
            },
            format,
        ),
        SyntaxCommand::TypeRefGaps {
            index,
            limit,
            format,
        } => syntax_type_ref_gaps(index, limit, format),
    }
}

fn syntax_export(book_path: PathBuf, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let book = HbkBook::open(book_path)?;
    let mut export = JsonExporter::new(output)
        .start_platform_context_stream(book.locale().export_code(), book.locale().source_code())?;
    if let Err(error) = SyntaxHelperReader::new(&book).extract_into(&mut export) {
        let _ = export.abort();
        return Err(Box::new(error));
    }
    let summary = export.finish()?;

    println!("output: {}", summary.output_dir.display());
    println!(
        "locale: {} (source: {})",
        summary.locale, summary.source_locale
    );
    println!("files: {}", summary.files.len());
    println!("global_contexts: {}", summary.counts.global_contexts);
    println!("global_methods: {}", summary.counts.global_methods);
    println!("global_properties: {}", summary.counts.global_properties);
    println!(
        "global_context_events: {}",
        summary.counts.global_context_events
    );
    println!("platform_types: {}", summary.counts.platform_types);
    println!("query_tables: {}", summary.counts.query_tables);
    println!("type_methods: {}", summary.counts.type_methods);
    println!("type_properties: {}", summary.counts.type_properties);
    println!("table_fields: {}", summary.counts.table_fields);
    println!("table_parameters: {}", summary.counts.table_parameters);
    println!("constructors: {}", summary.counts.constructors);
    println!("enums: {}", summary.counts.enums);
    println!("enum_values: {}", summary.counts.enum_values);
    println!("parser_warnings: {}", summary.counts.diagnostics);

    Ok(())
}

fn syntax_index(
    book_path: PathBuf,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = resolve_index_path(output);
    let started = Instant::now();
    let book = HbkBook::open(&book_path)?;
    let mut builder = SearchIndexBuilder::new();
    if let Err(error) = SyntaxHelperReader::new(&book).extract_into(&mut builder) {
        return Err(syntax_stream_error(error));
    }
    let metadata = IndexMetadata {
        locale: book.locale().export_code().to_string(),
        source_locale: book.locale().source_code().to_string(),
        source_hbk: book.path().display().to_string(),
        source_extraction_schema_version: 11,
    };
    let report = build_index_from_builder_with_report(&output, &metadata, builder)?;
    for warning in report.warnings {
        eprintln!("warning[{}]: {}", warning.code, warning.message);
    }
    println!("index: {}", output.display());
    println!(
        "locale: {} (source: {})",
        metadata.locale, metadata.source_locale
    );
    println!("documents: {}", syntax_document_count(&output)?);
    println!("elapsed_ms: {}", started.elapsed().as_millis());
    Ok(())
}

fn syntax_stream_error(
    error: SyntaxHelperStreamError<std::convert::Infallible>,
) -> Box<dyn std::error::Error> {
    match error {
        SyntaxHelperStreamError::Source(source) => Box::new(source),
        SyntaxHelperStreamError::Sink(never) => match never {},
    }
}

#[derive(Default)]
struct GetArgs {
    kind: Option<String>,
    id: Option<String>,
    name: Option<String>,
    alias: Option<String>,
    owner: Option<String>,
    owner_type_id: Option<String>,
    member: Option<String>,
    members_of: Option<String>,
    callable_id: Option<String>,
    callable: Option<String>,
}

fn syntax_get(
    index: Option<PathBuf>,
    args: GetArgs,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let index = SearchIndex::open_read_only(resolve_index_path(index))?;
    let query = classify_get_query(&args);
    let hits = match get_lookup(&index, query.lookup)? {
        GetLookupResult::Hits(hits) => hits,
        GetLookupResult::Unsupported(message) => {
            if matches!(format, OutputFormat::Json) {
                return print_provider_response(
                    "get",
                    "unsupported",
                    query.value,
                    Vec::new(),
                    unsupported_query_diagnostic(message),
                );
            }
            return Err(message.into());
        }
        GetLookupResult::NotFound => {
            return match format {
                OutputFormat::Text => {
                    println!("get: no matches");
                    Ok(())
                }
                OutputFormat::Json => {
                    let diagnostics = provider_diagnostics("not_found", &query.value, &[]);
                    print_provider_response(
                        "get",
                        "not_found",
                        query.value,
                        Vec::new(),
                        diagnostics,
                    )
                }
            };
        }
    };
    match format {
        OutputFormat::Text => print_hits_text("get", &hits),
        OutputFormat::Json => {
            print_provider_hits_with_index("get", query.value, &hits, Some(&index))
        }
    }
}

fn syntax_constructors(
    index: Option<PathBuf>,
    name: &str,
    details: bool,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let index = SearchIndex::open_read_only(resolve_index_path(index))?;
    let query = json!({ "kind": "constructor", "name": name });
    let type_candidates = type_identity_candidates(&index, name)?;
    if type_candidates.len() != 1 {
        return match format {
            OutputFormat::Text => {
                if type_candidates.is_empty() {
                    println!("constructors: no matches");
                } else {
                    println!(
                        "constructors: ambiguous type ({} matches)",
                        type_candidates.len()
                    );
                }
                Ok(())
            }
            OutputFormat::Json => print_related_root_diagnostic(
                "constructors",
                query,
                type_candidates,
                OutputFormat::Json,
            ),
        };
    }
    let hits = index.constructors_by_type_id(&type_candidates[0].document.id)?;
    match format {
        OutputFormat::Text => print_constructor_hits_text(&hits, details),
        OutputFormat::Json => print_provider_hits("constructors", query, &hits),
    }
}

fn syntax_search(
    index: Option<PathBuf>,
    query: &str,
    mode: SearchCliMode,
    limit: Option<usize>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let index = SearchIndex::open_read_only(resolve_index_path(index))?;
    let mode = match mode {
        SearchCliMode::Keywords => SearchMode::Keywords,
        SearchCliMode::Fuzzy => SearchMode::Fuzzy,
    };
    let effective_limit = limit.unwrap_or(DEFAULT_SEARCH_LIMIT);
    let hits = index.search(query, mode, effective_limit)?;
    let query_value = search_query_value(query, mode, limit);
    match format {
        OutputFormat::Text => print_hits_text("search", &hits),
        OutputFormat::Json => print_provider_hits("search", query_value, &hits),
    }
}

struct RelatedArgs {
    id: Option<String>,
    name: Option<String>,
    owner: Option<String>,
    member: Option<String>,
    edge: Option<String>,
    depth: u32,
    limit: Option<usize>,
    compact: bool,
    graph: bool,
}

fn syntax_related(
    index: Option<PathBuf>,
    args: RelatedArgs,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let index = SearchIndex::open_read_only(resolve_index_path(index))?;
    let query = related_query_value(
        args.id.as_deref(),
        args.name.as_deref(),
        args.owner.as_deref(),
        args.member.as_deref(),
        args.depth,
        args.edge.as_deref(),
        args.limit,
        args.compact,
        args.graph,
    );
    let effective_limit = args.limit.unwrap_or(DEFAULT_RELATED_LIMIT);
    if args.graph {
        if args.name.is_some()
            || args.owner.is_some()
            || args.member.is_some()
            || args.edge.is_some()
            || args.compact
        {
            if matches!(format, OutputFormat::Json) {
                return print_provider_response(
                    "related",
                    "unsupported",
                    query,
                    Vec::new(),
                    unsupported_query_diagnostic(
                        "syntax related --graph requires exactly one --id root and does not support --edge or --compact",
                    ),
                );
            }
            return Err(
                "syntax related --graph requires exactly one --id root and does not support --edge or --compact"
                    .into(),
            );
        }
        let Some(id) = args.id.as_deref() else {
            if matches!(format, OutputFormat::Json) {
                return print_provider_response(
                    "related",
                    "unsupported",
                    query,
                    Vec::new(),
                    unsupported_query_diagnostic(
                        "syntax related --graph requires exactly one --id root",
                    ),
                );
            }
            return Err("syntax related --graph requires exactly one --id root".into());
        };
        let Some(root) = index.get_by_id(id)? else {
            return print_related_root_diagnostic("related", query, Vec::new(), format);
        };
        if !is_supported_type_graph_root_kind(root.document.kind) {
            let message = "syntax related --graph supports only platform type, owned member and callable roots";
            if matches!(format, OutputFormat::Json) {
                return print_provider_response(
                    "related",
                    "unsupported",
                    query,
                    Vec::new(),
                    unsupported_query_diagnostic(message),
                );
            }
            return Err(message.into());
        }
        if effective_limit == 0 {
            return match format {
                OutputFormat::Text => Ok(()),
                OutputFormat::Json => {
                    print_provider_response("related", "ok", query, Vec::new(), Vec::new())
                }
            };
        }
        let related_limit = effective_limit - 1;
        let hits = if related_limit == 0 {
            Vec::new()
        } else {
            index.related_by_id(id, args.depth, related_limit)?
        };
        return match format {
            OutputFormat::Text => {
                println!(
                    "{} [{}] depth=0",
                    root.document.name.primary, root.document.kind
                );
                print_related_hits_text(&hits)
            }
            OutputFormat::Json => print_provider_type_graph(&index, query, root, &hits),
        };
    }
    if let (Some(id), Some(edge)) = (args.id.as_deref(), args.edge.as_deref()) {
        if args.name.is_some() || args.owner.is_some() || args.member.is_some() {
            if matches!(format, OutputFormat::Json) {
                return print_provider_response(
                    "related",
                    "unsupported",
                    query,
                    Vec::new(),
                    unsupported_query_diagnostic(
                        "syntax related --edge requires exactly one root: --id",
                    ),
                );
            }
            return Err("syntax related --edge requires exactly one root: --id".into());
        }
        if !is_supported_edge_filter(edge) {
            if matches!(format, OutputFormat::Json) {
                return print_provider_response(
                    "related",
                    "unsupported",
                    query,
                    Vec::new(),
                    unsupported_query_diagnostic(
                        "syntax related --edge supports has_type, returns, constructs or member_of",
                    ),
                );
            }
            return Err(
                "syntax related --edge supports has_type, returns, constructs or member_of".into(),
            );
        }
        if index.get_by_id(id)?.is_none() {
            return print_related_root_diagnostic("related", query, Vec::new(), format);
        }
        let hits = index.related_by_id_and_edge(id, edge, effective_limit)?;
        return match format {
            OutputFormat::Text => print_related_hits_text(&hits),
            OutputFormat::Json => print_provider_related_hits(query, &hits, args.compact),
        };
    }
    if args.edge.is_some() {
        if matches!(format, OutputFormat::Json) {
            return print_provider_response(
                "related",
                "unsupported",
                query,
                Vec::new(),
                unsupported_query_diagnostic(
                    "syntax related --edge requires an exact --id root in the first implementation",
                ),
            );
        }
        return Err(
            "syntax related --edge requires an exact --id root in the first implementation".into(),
        );
    }
    let root = match (args.id, args.name, args.owner, args.member) {
        (Some(id), None, None, None) => RootLookup::ById(id),
        (None, Some(name), None, None) => RootLookup::ByName(name),
        (None, None, Some(owner), Some(member)) => RootLookup::ByOwnerMember(owner, member),
        _ => {
            if matches!(format, OutputFormat::Json) {
                return print_provider_response(
                    "related",
                    "unsupported",
                    query,
                    Vec::new(),
                    unsupported_query_diagnostic(
                        "syntax related requires exactly one root: --id, --name, or both --owner and --member",
                    ),
                );
            }
            return Err("syntax related requires exactly one root: --id, --name, or both --owner and --member".into());
        }
    };
    let hits = match &root {
        RootLookup::ById(id) => {
            if index.get_by_id(id)?.is_none() {
                return print_related_root_diagnostic("related", query, Vec::new(), format);
            }
            index.related_by_id(id, args.depth, effective_limit)?
        }
        RootLookup::ByName(name) => {
            let roots = index.get_by_name(name)?;
            if roots.len() != 1 {
                return print_related_root_diagnostic("related", query, roots, format);
            }
            index.related_by_id(&roots[0].document.id, args.depth, effective_limit)?
        }
        RootLookup::ByOwnerMember(owner, member) => {
            let roots = owner_member_roots(&index, owner, member)?;
            if roots.len() != 1 {
                return print_related_root_diagnostic("related", query, roots, format);
            }
            index.related_by_id(&roots[0].document.id, args.depth, effective_limit)?
        }
    };
    match format {
        OutputFormat::Text => print_related_hits_text(&hits),
        OutputFormat::Json => print_provider_related_hits(query, &hits, args.compact),
    }
}
