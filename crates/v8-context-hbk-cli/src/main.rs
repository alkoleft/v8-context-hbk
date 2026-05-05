use std::path::PathBuf;
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use hbk_book::HbkBook;
use hbk_book::{Toc, TocPage};
use hbk_container::HbkContainer;
use hbk_export::JsonExporter;
use serde_json::{Value, json};
use syntax_helper_extract::{SyntaxHelperReader, SyntaxHelperStreamError};
use syntax_helper_model::LocalizedName;
use syntax_helper_search::{
    IndexMetadata, RelatedHit, SearchDocument, SearchHit, SearchIndex, SearchIndexBuilder,
    SearchMode, build_index_from_builder,
};

const DEFAULT_SEARCH_LIMIT: usize = 20;
const DEFAULT_RELATED_LIMIT: usize = 200;

#[derive(Debug, Parser)]
#[command(version, about = "Read and inspect 1C HBK help book containers")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Command {
    Inspect {
        #[arg(value_name = "HBK_FILE")]
        path: PathBuf,
    },
    Toc {
        #[arg(value_name = "HBK_FILE")]
        path: PathBuf,
        #[arg(long, value_enum, default_value_t = TocFormat::Text)]
        format: TocFormat,
    },
    Page {
        #[arg(value_name = "HBK_FILE")]
        book: PathBuf,
        #[arg(long, value_name = "HTML_PATH")]
        path: String,
    },
    Syntax {
        #[command(subcommand)]
        command: SyntaxCommand,
    },
}

#[derive(Debug, Subcommand)]
enum SyntaxCommand {
    Export {
        #[arg(value_name = "HBK_FILE")]
        book: PathBuf,
        #[arg(long, value_name = "DIR")]
        output: PathBuf,
    },
    Index {
        #[arg(value_name = "HBK_FILE")]
        book: PathBuf,
        #[arg(long, value_name = "INDEX_SQLITE")]
        output: Option<PathBuf>,
    },
    Get {
        #[arg(long, value_name = "INDEX_SQLITE")]
        index: Option<PathBuf>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        alias: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long = "owner-type-id")]
        owner_type_id: Option<String>,
        #[arg(long)]
        member: Option<String>,
        #[arg(long = "members-of")]
        members_of: Option<String>,
        #[arg(long = "callable-id")]
        callable_id: Option<String>,
        #[arg(long)]
        callable: Option<String>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    Constructors {
        #[arg(value_name = "TYPE")]
        name: String,
        #[arg(long, value_name = "INDEX_SQLITE")]
        index: Option<PathBuf>,
        #[arg(long)]
        details: bool,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    Search {
        #[arg(long, value_name = "INDEX_SQLITE")]
        index: Option<PathBuf>,
        #[arg(long)]
        query: String,
        #[arg(long, value_enum, default_value_t = SearchCliMode::Keywords)]
        mode: SearchCliMode,
        #[arg(long, value_parser = parse_positive_usize)]
        limit: Option<usize>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    Related {
        #[arg(long, value_name = "INDEX_SQLITE")]
        index: Option<PathBuf>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        member: Option<String>,
        #[arg(
            long,
            value_name = "EDGE",
            help = "Filter related traversal by edge kind: has_type, returns, constructs or member_of"
        )]
        edge: Option<String>,
        #[arg(long, default_value_t = 5)]
        depth: u32,
        #[arg(long, value_parser = parse_positive_usize)]
        limit: Option<usize>,
        #[arg(long)]
        compact: bool,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TocFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SearchCliMode {
    Keywords,
    Fuzzy,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { path } => inspect(path)?,
        Command::Toc { path, format } => toc(path, format)?,
        Command::Page { book, path } => page(book, &path)?,
        Command::Syntax { command } => syntax(command)?,
    }
    Ok(())
}

fn inspect(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let container = HbkContainer::open(path)?;
    println!("file: {}", container.path().display());
    println!("entities: {}", container.descriptors().len());
    for descriptor in container.descriptors() {
        let body_offset = descriptor
            .body_offset
            .map(|offset| offset.to_string())
            .unwrap_or_else(|| "<none>".to_string());
        println!(
            "- {} descriptor_offset={} header_offset={} body_offset={}",
            descriptor.name, descriptor.descriptor_offset, descriptor.header_offset, body_offset
        );
    }
    Ok(())
}

fn toc(path: PathBuf, format: TocFormat) -> Result<(), Box<dyn std::error::Error>> {
    let book = HbkBook::open(path)?;
    match format {
        TocFormat::Text => print_toc_text(book.toc()),
        TocFormat::Json => print_toc_json(book.toc())?,
    }
    Ok(())
}

fn page(book_path: PathBuf, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let book = HbkBook::open(book_path)?;
    let page = book.read_page(path)?;
    print!("{page}");
    Ok(())
}

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
            },
            format,
        ),
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
    build_index_from_builder(&output, &metadata, builder)?;
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
    );
    let effective_limit = args.limit.unwrap_or(DEFAULT_RELATED_LIMIT);
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

#[allow(clippy::enum_variant_names)]
enum RootLookup {
    ById(String),
    ByName(String),
    ByOwnerMember(String, String),
}

struct ClassifiedGetQuery<'a> {
    value: Value,
    lookup: GetLookup<'a>,
}

enum GetLookup<'a> {
    TypeIdentityById(&'a str),
    TypeIdentityByName(&'a str),
    TypeIdentityByAlias(&'a str),
    DocumentById(&'a str),
    ExactName(&'a str),
    MemberList(&'a str),
    OwnerTypeMember {
        owner_type_id: &'a str,
        member: &'a str,
    },
    OwnerMember {
        owner: &'a str,
        member: &'a str,
    },
    CallableById(&'a str),
    CallableByOwnerType {
        owner_type_id: &'a str,
        callable: &'a str,
    },
    Unsupported(&'static str),
}

fn classify_get_query(args: &GetArgs) -> ClassifiedGetQuery<'_> {
    let (value, lookup) = match (
        args.kind.as_deref(),
        args.id.as_deref(),
        args.name.as_deref(),
        args.alias.as_deref(),
        args.owner.as_deref(),
        args.owner_type_id.as_deref(),
        args.member.as_deref(),
        args.members_of.as_deref(),
        args.callable_id.as_deref(),
        args.callable.as_deref(),
    ) {
        (Some("platform_type"), Some(id), None, None, None, None, None, None, None, None) => (
            json!({ "kind": "type_identity", "id": id }),
            GetLookup::TypeIdentityById(id),
        ),
        (Some("platform_type"), None, Some(name), None, None, None, None, None, None, None) => (
            json!({ "kind": "type_identity", "name": name }),
            GetLookup::TypeIdentityByName(name),
        ),
        (Some("platform_type"), None, None, Some(alias), None, None, None, None, None, None) => (
            json!({ "kind": "type_identity", "alias": alias }),
            GetLookup::TypeIdentityByAlias(alias),
        ),
        (Some(_), _, _, _, _, _, _, _, _, _) => (
            json!({ "kind": "invalid" }),
            GetLookup::Unsupported(
                "syntax get --kind currently supports only platform_type with exactly one of --id, --name or --alias",
            ),
        ),
        (None, Some(id), None, None, None, None, None, None, None, None) => (
            json!({ "kind": "document_id", "id": id }),
            GetLookup::DocumentById(id),
        ),
        (None, None, Some(name), None, None, None, None, None, None, None) => (
            json!({ "kind": "exact_name", "name": name }),
            GetLookup::ExactName(name),
        ),
        (None, None, None, None, None, None, None, Some(type_id), None, None) => (
            json!({ "kind": "member_list", "owner_type_id": type_id }),
            GetLookup::MemberList(type_id),
        ),
        (None, None, None, None, None, Some(owner_type_id), Some(member), None, None, None) => (
            json!({ "kind": "owner_type_member", "owner_type_id": owner_type_id, "name": member }),
            GetLookup::OwnerTypeMember {
                owner_type_id,
                member,
            },
        ),
        (None, None, None, None, Some(owner), None, Some(member), None, None, None) => (
            json!({ "kind": "owner_member", "owner": owner, "member": member }),
            GetLookup::OwnerMember { owner, member },
        ),
        (None, None, None, None, None, None, None, None, Some(callable_id), None) => (
            json!({ "kind": "callable_overloads", "id": callable_id }),
            GetLookup::CallableById(callable_id),
        ),
        (None, None, None, None, None, Some(owner_type_id), None, None, None, Some(callable)) => (
            json!({ "kind": "callable_overloads", "owner_type_id": owner_type_id, "name": callable }),
            GetLookup::CallableByOwnerType {
                owner_type_id,
                callable,
            },
        ),
        _ => (
            json!({ "kind": "invalid" }),
            GetLookup::Unsupported(
                "syntax get requires exactly one root: --id, --name, --kind platform_type with --id/--name/--alias, --members-of, --owner-type-id with --member/--callable, --callable-id, or both --owner and --member",
            ),
        ),
    };
    ClassifiedGetQuery { value, lookup }
}

enum GetLookupResult {
    Hits(Vec<SearchHit>),
    NotFound,
    Unsupported(&'static str),
}

fn get_lookup(
    index: &SearchIndex,
    lookup: GetLookup<'_>,
) -> Result<GetLookupResult, Box<dyn std::error::Error>> {
    let result = match lookup {
        GetLookup::TypeIdentityById(id) => index
            .type_identity_by_id(id)?
            .map_or(GetLookupResult::NotFound, |hit| {
                GetLookupResult::Hits(vec![hit])
            }),
        GetLookup::TypeIdentityByName(name) => {
            GetLookupResult::Hits(index.type_identities_by_name(name)?)
        }
        GetLookup::TypeIdentityByAlias(alias) => {
            GetLookupResult::Hits(index.type_identities_by_alias(alias)?)
        }
        GetLookup::DocumentById(id) => index
            .get_by_id(id)?
            .map_or(GetLookupResult::NotFound, |hit| {
                GetLookupResult::Hits(vec![hit])
            }),
        GetLookup::ExactName(name) => GetLookupResult::Hits(index.get_by_name(name)?),
        GetLookup::MemberList(type_id) => {
            if !type_id.starts_with("platform_type:") {
                return Ok(GetLookupResult::Unsupported(
                    "syntax get --members-of requires an exact platform_type provider id",
                ));
            }
            if index.type_identity_by_id(type_id)?.is_none() {
                GetLookupResult::NotFound
            } else {
                GetLookupResult::Hits(index.members_by_type_id(type_id)?)
            }
        }
        GetLookup::OwnerTypeMember {
            owner_type_id,
            member,
        } => {
            if index.type_identity_by_id(owner_type_id)?.is_none() {
                GetLookupResult::NotFound
            } else {
                GetLookupResult::Hits(index.member_by_owner_type_id(owner_type_id, member)?)
            }
        }
        GetLookup::OwnerMember { owner, member } => {
            let roots = owner_member_roots(index, owner, member)?;
            if roots.is_empty() {
                GetLookupResult::NotFound
            } else {
                GetLookupResult::Hits(roots)
            }
        }
        GetLookup::CallableById(callable_id) => index
            .callable_by_id(callable_id)?
            .map_or(GetLookupResult::NotFound, |hit| {
                GetLookupResult::Hits(vec![hit])
            }),
        GetLookup::CallableByOwnerType {
            owner_type_id,
            callable,
        } => {
            if index.type_identity_by_id(owner_type_id)?.is_none() {
                GetLookupResult::NotFound
            } else {
                GetLookupResult::Hits(index.callable_by_owner_type_id(owner_type_id, callable)?)
            }
        }
        GetLookup::Unsupported(message) => GetLookupResult::Unsupported(message),
    };
    Ok(result)
}

fn type_identity_candidates(
    index: &SearchIndex,
    name: &str,
) -> Result<Vec<SearchHit>, Box<dyn std::error::Error>> {
    let mut candidates = index.type_identities_by_name(name)?;
    for alias_hit in index.type_identities_by_alias(name)? {
        if !candidates
            .iter()
            .any(|hit| hit.document.id == alias_hit.document.id)
        {
            candidates.push(alias_hit);
        }
    }
    candidates.sort_by(|left, right| left.document.id.cmp(&right.document.id));
    Ok(candidates)
}

fn owner_member_roots(
    index: &SearchIndex,
    owner: &str,
    member: &str,
) -> Result<Vec<SearchHit>, Box<dyn std::error::Error>> {
    let owner_candidates = type_identity_candidates(index, owner)?;
    if owner_candidates.len() > 1 {
        return Ok(owner_candidates);
    }
    if let Some(owner_type) = owner_candidates.first() {
        return Ok(index.member_by_owner_type_id(&owner_type.document.id, member)?);
    }
    Ok(index.get_by_owner_member(owner, member)?)
}

#[allow(clippy::too_many_arguments)]
fn related_query_value(
    id: Option<&str>,
    name: Option<&str>,
    owner: Option<&str>,
    member: Option<&str>,
    depth: u32,
    edge: Option<&str>,
    limit: Option<usize>,
    compact: bool,
) -> Value {
    let root = match (id, name, owner, member) {
        (Some(id), None, None, None) => json!({ "id": id }),
        (None, Some(name), None, None) => json!({ "name": name }),
        (None, None, Some(owner), Some(member)) => json!({ "owner": owner, "member": member }),
        _ => json!({ "invalid": true }),
    };
    let mut query =
        json!({ "kind": related_query_kind(edge), "root": root, "depth": depth.min(5) });
    if let Some(edge) = edge {
        query["edge"] = json!(edge);
    }
    if let Some(limit) = limit {
        query["limit"] = json!(limit);
    }
    if compact {
        query["output"] = json!("compact");
    }
    query
}

fn search_query_value(query: &str, mode: SearchMode, limit: Option<usize>) -> Value {
    let mut value = json!({
        "kind": "search",
        "mode": search_mode_name(mode),
        "text": query,
    });
    if let Some(limit) = limit {
        value["limit"] = json!(limit);
    }
    value
}

fn search_mode_name(mode: SearchMode) -> &'static str {
    match mode {
        SearchMode::Keywords => "keywords",
        SearchMode::Fuzzy => "fuzzy",
    }
}

fn is_supported_edge_filter(edge: &str) -> bool {
    matches!(edge, "has_type" | "returns" | "constructs" | "member_of")
}

fn related_query_kind(edge: Option<&str>) -> &'static str {
    match edge {
        Some("has_type" | "returns" | "constructs") => "type_references",
        _ => "related",
    }
}

fn print_hits_text(command: &str, hits: &[SearchHit]) -> Result<(), Box<dyn std::error::Error>> {
    if hits.is_empty() {
        println!("{command}: no matches");
    }
    for hit in hits {
        let owner = hit
            .document
            .owner
            .as_ref()
            .map(display_name)
            .unwrap_or_default();
        let prefix = if owner.is_empty() {
            String::new()
        } else {
            format!("{owner}.")
        };
        println!(
            "{}{} [{}]",
            prefix, hit.document.name.primary, hit.document.kind
        );
        if let Some(alias) = &hit.document.name.alias {
            println!("  alias: {alias}");
        }
        if !hit.document.type_refs.is_empty() {
            println!("  types: {}", hit.document.type_refs.join(", "));
        }
        if !hit.document.return_types.is_empty() {
            println!("  return: {}", hit.document.return_types.join(", "));
        }
        if !hit.document.preview.is_empty() {
            println!("  {}", hit.document.preview);
        }
    }
    Ok(())
}

fn print_related_hits_text(hits: &[RelatedHit]) -> Result<(), Box<dyn std::error::Error>> {
    for hit in hits {
        let owner = hit
            .document
            .owner
            .as_ref()
            .map(display_name)
            .unwrap_or_default();
        let prefix = if owner.is_empty() {
            String::new()
        } else {
            format!("{owner}.")
        };
        println!(
            "{}{} [{}] depth={}",
            prefix, hit.document.name.primary, hit.document.kind, hit.depth
        );
        for step in &hit.via {
            println!("  - {} -> {} ({})", step.from, step.to, step.edge_kind);
        }
    }
    Ok(())
}

fn print_constructor_hits_text(
    hits: &[SearchHit],
    details: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if hits.is_empty() {
        println!("constructors: no matches");
    }
    for hit in hits {
        print_constructor_text_hit(hit, details);
    }
    Ok(())
}

fn print_provider_hits(
    command: &str,
    query: Value,
    hits: &[SearchHit],
) -> Result<(), Box<dyn std::error::Error>> {
    print_provider_hits_with_index(command, query, hits, None)
}

fn print_provider_hits_with_index(
    command: &str,
    query: Value,
    hits: &[SearchHit],
    index: Option<&SearchIndex>,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = provider_status(command, &query, hits.len());
    let diagnostics = provider_diagnostics(status, &query, hits);
    let results = if status == "ambiguous" {
        Vec::new()
    } else {
        hits.iter()
            .enumerate()
            .map(|(rank_index, hit)| {
                let mut meta = json!({ "rank": rank_index + 1 });
                if command == "search" {
                    meta["score"] = json!(hit.score);
                }
                if let Some(search_index) = index {
                    add_provider_resolution_meta(search_index, &hit.document, &mut meta)?;
                }
                Ok(json!({
                    "fact": document_fact(&hit.document, ProviderFactDetail::Full),
                    "meta": meta,
                }))
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?
    };
    print_provider_response(command, status, query, results, diagnostics)
}

fn add_provider_resolution_meta(
    index: &SearchIndex,
    document: &SearchDocument,
    meta: &mut Value,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(owner_type_id) = index.owner_type_id_for_document(&document.id)? {
        meta["owner_type_id"] = json!(owner_type_id);
    }
    let target_type_ids = index.target_type_ids_for_document(&document.id)?;
    if !target_type_ids.is_empty() {
        meta["target_type_ids"] = json!(target_type_ids);
    }
    Ok(())
}

fn print_provider_related_hits(
    query: Value,
    hits: &[RelatedHit],
    compact: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let results = hits
        .iter()
        .map(|hit| related_result_value(hit, compact))
        .collect::<Vec<_>>();
    print_provider_response("related", "ok", query, results, Vec::new())
}

fn related_result_value(hit: &RelatedHit, compact: bool) -> Value {
    json!({
        "fact": document_fact(
            &hit.document,
            if compact {
                ProviderFactDetail::Compact
            } else {
                ProviderFactDetail::Full
            },
        ),
        "meta": {
            "depth": hit.depth,
            "path": hit.via,
        },
    })
}

fn print_related_root_diagnostic(
    command: &str,
    query: Value,
    roots: Vec<SearchHit>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Text => {
            if roots.is_empty() {
                println!("{command}: no matches");
            } else {
                println!("{command}: ambiguous root ({} matches)", roots.len());
            }
            Ok(())
        }
        OutputFormat::Json => {
            let status = if roots.is_empty() {
                "not_found"
            } else {
                "ambiguous"
            };
            let diagnostics = provider_diagnostics(status, &query, &roots);
            print_provider_response(command, status, query, Vec::new(), diagnostics)
        }
    }
}

fn provider_status(command: &str, query: &Value, hit_count: usize) -> &'static str {
    if query["kind"] == "member_list" {
        return "ok";
    }
    match (command, hit_count) {
        ("search", _) => "ok",
        (_, 0) => "not_found",
        ("get", count) if count > 1 => "ambiguous",
        (_, _) => "ok",
    }
}

fn provider_diagnostics(status: &str, query: &Value, hits: &[SearchHit]) -> Vec<Value> {
    match status {
        "not_found" => vec![json!({
            "code": "NOT_FOUND",
            "message": "No Syntax Assistant fact matched the query.",
            "query": query,
        })],
        "ambiguous" => vec![json!({
            "code": "AMBIGUOUS",
            "message": "The query matched more than one Syntax Assistant fact.",
            "query": query,
            "candidates": hits.iter().map(|hit| candidate_summary(&hit.document)).collect::<Vec<_>>(),
        })],
        _ => Vec::new(),
    }
}

fn unsupported_query_diagnostic(message: &str) -> Vec<Value> {
    vec![json!({
        "code": "UNSUPPORTED_QUERY",
        "message": message,
    })]
}

fn print_provider_response(
    command: &str,
    status: &str,
    query: Value,
    results: Vec<Value>,
    diagnostics: Vec<Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = provider_response(command, status, query, results, diagnostics);
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

fn provider_response(
    command: &str,
    status: &str,
    query: Value,
    results: Vec<Value>,
    diagnostics: Vec<Value>,
) -> Value {
    json!({
        "schema_version": 1,
        "command": command,
        "status": status,
        "query": query,
        "results": results,
        "diagnostics": diagnostics,
    })
}

fn candidate_summary(document: &SearchDocument) -> Value {
    json!({
        "id": document.id,
        "kind": document.kind,
        "name": document.name,
        "owner": document.owner.as_ref().map(|owner| owner.primary.clone()),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProviderFactDetail {
    Full,
    Compact,
}

fn document_fact(document: &SearchDocument, detail: ProviderFactDetail) -> Value {
    let mut fact = json!({
        "id": document.id,
        "kind": document.kind,
        "name": document.name,
    });
    if let Some(owner) = &document.owner {
        fact["owner"] = json!(owner.primary);
    }
    if detail == ProviderFactDetail::Compact {
        return fact;
    }
    if !document.signatures.is_empty() {
        fact["signatures"] = json!(document.signatures);
    }
    if !document.type_refs.is_empty() {
        fact["types"] = json!(document.type_refs);
    }
    if !document.return_types.is_empty() {
        fact["return"] = json!(document.return_types);
    }
    if let Some(description) = &document.description {
        fact["description"] = json!(description);
    }
    fact
}

fn print_constructor_text_hit(hit: &SearchHit, details: bool) {
    if hit.document.signatures.is_empty() {
        println!("{}", hit.document.name.primary);
    } else {
        for signature in &hit.document.signatures {
            println!("{}", signature.text);
        }
    }

    if !details {
        return;
    }

    if let Some(owner) = &hit.document.owner {
        println!("  owner: {}", display_name(owner));
    }
    if let Some(alias) = &hit.document.name.alias {
        println!("  alias: {alias}");
    }
    if let Some(description) = &hit.document.description {
        println!("  {description}");
    } else if !hit.document.preview.is_empty() {
        println!("  {}", hit.document.preview);
    }
}

fn resolve_index_path(path: Option<PathBuf>) -> PathBuf {
    path.or_else(|| std::env::var_os("V8_CONTEXT_HBK_SYNTAX_INDEX").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(".v8-context-hbk/syntax/index.sqlite"))
}

fn syntax_document_count(path: &std::path::Path) -> Result<i64, Box<dyn std::error::Error>> {
    let index = SearchIndex::open_read_only(path)?;
    Ok(index.document_count()?)
}

fn parse_positive_usize(value: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|error| format!("invalid positive integer: {error}"))?;
    if parsed == 0 {
        return Err("value must be greater than 0".to_string());
    }
    Ok(parsed)
}

fn display_name(name: &LocalizedName) -> String {
    match &name.alias {
        Some(alias) => format!("{} ({alias})", name.primary),
        None => name.primary.clone(),
    }
}

fn print_toc_text(toc: &Toc) {
    for flat_page in toc.flat_pages() {
        let depth = flat_page.index_path.indexes().len().saturating_sub(1);
        println!(
            "{}- {} ({})",
            "  ".repeat(depth),
            flat_page.page.title.display(),
            flat_page.page.html_path
        );
    }
}

fn print_toc_json(toc: &Toc) -> Result<(), Box<dyn std::error::Error>> {
    let pages = toc.pages().iter().map(page_to_json).collect::<Vec<_>>();
    println!("{}", serde_json::to_string_pretty(&pages)?);
    Ok(())
}

fn page_to_json(page: &TocPage) -> serde_json::Value {
    json!({
        "id": page.id,
        "parent_id": page.parent_id,
        "title": {
            "en": page.title.en,
            "ru": page.title.ru,
        },
        "html_path": page.html_path,
        "children": page.children.iter().map(page_to_json).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use syntax_helper_model as model;
    use syntax_helper_model::SyntaxHelperSink;

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
        );

        assert!(is_supported_edge_filter("member_of"));
        assert_eq!(query["kind"], "related");
        assert_eq!(query["edge"], "member_of");
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
            kind: "type_method".to_string(),
            name: name("Выполнить"),
            owner: Some(name("Тест")),
            signatures: vec![syntax_helper_search::SearchSignature {
                text: "Выполнить(Параметр)".to_string(),
                parameters: Vec::new(),
                title: None,
                description: None,
            }],
            type_refs: Vec::new(),
            return_types: vec!["Булево".to_string()],
            description: Some("Detailed description".to_string()),
            preview: "Detailed description".to_string(),
            parameter_terms: Vec::new(),
            relation_keys: Vec::new(),
            owner_relation_key: None,
        };

        let fact = document_fact(&document, ProviderFactDetail::Compact);

        assert_eq!(fact["id"], document.id);
        assert_eq!(fact["kind"], document.kind);
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
            kind: "type_method".to_string(),
            name: name("Выполнить"),
            owner: Some(name("Тест")),
            signatures: vec![syntax_helper_search::SearchSignature {
                text: "Выполнить(Параметр)".to_string(),
                parameters: Vec::new(),
                title: None,
                description: None,
            }],
            type_refs: vec!["Строка".to_string()],
            return_types: vec!["Булево".to_string()],
            description: Some("Detailed description".to_string()),
            preview: "Detailed description".to_string(),
            parameter_terms: Vec::new(),
            relation_keys: Vec::new(),
            owner_relation_key: None,
        };

        let fact = document_fact(&document, ProviderFactDetail::Full);

        assert_eq!(fact["id"], document.id);
        assert_eq!(fact["kind"], document.kind);
        assert_eq!(fact["name"]["primary"], "Выполнить");
        assert_eq!(fact["owner"], "Тест");
        assert_eq!(fact["types"], json!(["Строка"]));
        assert_eq!(fact["return"], json!(["Булево"]));
        assert_eq!(fact["description"], "Detailed description");
        assert_eq!(fact["signatures"].as_array().unwrap().len(), 1);
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
        assert!(roots.iter().all(|hit| hit.document.kind == "platform_type"));
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
        builder.constructor(constructor("ЭлементыФормы")).unwrap();
        build_index_from_builder(&path, &metadata(), builder).unwrap();

        let index = SearchIndex::open_read_only(&path).unwrap();
        let candidates = type_identity_candidates(&index, "ЭлементыФормы").unwrap();

        assert_eq!(candidates.len(), 2);
        assert!(
            candidates
                .iter()
                .all(|hit| hit.document.kind == "platform_type")
        );
        let _ = fs::remove_file(path);
    }

    fn temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("v8-context-hbk-cli-{unique}-{name}"))
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
            name: name(primary),
            semantic: semantic(model::RecordFamily::PlatformType, owner),
            type_kind: model::PlatformTypeKind::Regular,
            object_kind: Some(model::PlatformObjectKind::RegularPlatformType),
            extends: Vec::new(),
            metadata_kind: None,
            template_parameters: Vec::new(),
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
            name: name(primary),
            semantic: semantic(model::RecordFamily::TypeMethod, owner_path),
            signatures: vec![model::Signature {
                text: format!("{primary}()"),
                parameters: Vec::new(),
                variant: None,
            }],
            return_types: Vec::new(),
            description: Some("method description".to_string()),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn constructor(owner: &str) -> model::Constructor {
        model::Constructor {
            owner: name(owner),
            name: name("По умолчанию"),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: format!("Новый {owner}()"),
                parameters: Vec::new(),
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

    fn name(primary: &str) -> LocalizedName {
        LocalizedName {
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
