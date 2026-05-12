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
    Export {
        #[arg(value_name = "HBK_FILE")]
        book: PathBuf,
        #[arg(long, value_name = "DIR")]
        output: PathBuf,
        #[arg(long, value_enum)]
        format: BookExportCliFormat,
        #[arg(long, value_enum)]
        hierarchy: BookExportCliHierarchy,
    },
    Site {
        #[command(subcommand)]
        command: SiteCommand,
    },
    Syntax {
        #[command(subcommand)]
        command: SyntaxCommand,
    },
}

#[derive(Debug, Subcommand)]
enum SiteCommand {
    Generate {
        #[arg(value_name = "SOURCE_DIR")]
        source_dir: PathBuf,
        #[arg(long, value_name = "DIR")]
        output: PathBuf,
        #[arg(long = "include", value_name = "FILE_NAME")]
        include_file_names: Vec<String>,
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
        #[arg(long)]
        graph: bool,
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    TypeRefGaps {
        #[arg(long, value_name = "INDEX_SQLITE")]
        index: Option<PathBuf>,
        #[arg(long, value_parser = parse_positive_usize, default_value_t = 10)]
        limit: usize,
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

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BookExportCliFormat {
    Raw,
    Markdown,
}

impl From<BookExportCliFormat> for BookExportFormat {
    fn from(value: BookExportCliFormat) -> Self {
        match value {
            BookExportCliFormat::Raw => Self::Raw,
            BookExportCliFormat::Markdown => Self::Markdown,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BookExportCliHierarchy {
    Raw,
    Toc,
}

impl From<BookExportCliHierarchy> for BookExportHierarchy {
    fn from(value: BookExportCliHierarchy) -> Self {
        match value {
            BookExportCliHierarchy::Raw => Self::Raw,
            BookExportCliHierarchy::Toc => Self::Toc,
        }
    }
}
