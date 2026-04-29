use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use serde_json::json;
use v8_context_hbk::export::JsonExporter;
use v8_context_hbk::hbk::book::HbkBook;
use v8_context_hbk::hbk::container::HbkContainer;
use v8_context_hbk::hbk::toc::{Toc, TocPage};
use v8_context_hbk::syntax_helper::SyntaxHelperReader;

#[derive(Debug, Parser)]
#[command(version, about = "Read and inspect 1C HBK help book containers")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
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
    SyntaxHelper {
        #[arg(value_name = "HBK_FILE")]
        book: PathBuf,
        #[arg(long, value_name = "DIR")]
        output: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TocFormat {
    Text,
    Json,
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
        Command::SyntaxHelper { book, output } => syntax_helper(book, output)?,
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

fn syntax_helper(book_path: PathBuf, output: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let book = HbkBook::open(book_path)?;
    let context = SyntaxHelperReader::new(&book).extract()?;
    let summary = JsonExporter::new(output).export_syntax_helper(&book, &context)?;

    println!("output: {}", summary.output_dir.display());
    println!(
        "locale: {} (source: {})",
        summary.locale, summary.source_locale
    );
    println!("files: {}", summary.files.len());
    println!("global_contexts: {}", context.global_contexts.len());
    println!("global_methods: {}", context.global_methods.len());
    println!("global_properties: {}", context.global_properties.len());
    println!("platform_types: {}", context.platform_types.len());
    println!("type_methods: {}", context.type_methods.len());
    println!("type_properties: {}", context.type_properties.len());
    println!("constructors: {}", context.constructors.len());
    println!("enums: {}", context.enums.len());
    println!("enum_values: {}", context.enum_values.len());
    println!("diagnostics: {}", context.diagnostics.len());

    Ok(())
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
