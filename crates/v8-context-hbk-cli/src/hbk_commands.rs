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

fn export_book(
    book_path: PathBuf,
    output: PathBuf,
    format: BookExportFormat,
    hierarchy: BookExportHierarchy,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = export_book_content(book_path, output, format, hierarchy)?;
    println!("output: {}", result.output_root().display());
    println!("format: {format}");
    println!("hierarchy: {hierarchy}");
    println!("files: {}", result.files().len());
    println!(
        "bytes: {}",
        result
            .files()
            .iter()
            .map(|file| file.bytes_written())
            .sum::<u64>()
    );
    Ok(())
}

fn export_book_content(
    book_path: PathBuf,
    output: PathBuf,
    format: BookExportFormat,
    hierarchy: BookExportHierarchy,
) -> Result<BookExportResult, hbk_book_export::BookExportError> {
    validate_cli_book_export_combination(format, hierarchy)?;
    let request = BookExportRequest::new(book_path.clone(), output, format, hierarchy)?;
    let book = HbkBook::open(&book_path)?;
    BookExporter::new(&book).export(&request)
}

fn validate_cli_book_export_combination(
    format: BookExportFormat,
    hierarchy: BookExportHierarchy,
) -> Result<(), hbk_book_export::BookExportError> {
    match (format, hierarchy) {
        (BookExportFormat::Raw, BookExportHierarchy::Raw)
        | (BookExportFormat::Markdown, BookExportHierarchy::Toc) => Ok(()),
        (format, hierarchy) => {
            Err(hbk_book_export::BookExportError::UnsupportedCombination { format, hierarchy })
        }
    }
}
