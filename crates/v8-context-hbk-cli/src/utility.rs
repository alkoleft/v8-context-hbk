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
