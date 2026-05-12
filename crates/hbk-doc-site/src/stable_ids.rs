fn page_markdown_file_name(page_id: &SitePageId) -> String {
    format!("{}.md", page_id.as_str())
}

fn write_json(
    path: PathBuf,
    value: &impl Serialize,
) -> Result<GeneratedSiteFile, SiteGenerationError> {
    if let Some(parent) = path.parent() {
        create_directory(parent)?;
    }
    let bytes = serde_json::to_vec(value).map_err(|source| SiteGenerationError::Json {
        path: path.clone(),
        source,
    })?;
    let bytes_written = bytes.len() as u64;
    fs::write(&path, bytes).map_err(|source| SiteGenerationError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(GeneratedSiteFile::new(path, bytes_written))
}

fn write_text(path: PathBuf, text: &str) -> Result<GeneratedSiteFile, SiteGenerationError> {
    if let Some(parent) = path.parent() {
        create_directory(parent)?;
    }
    let bytes_written = text.len() as u64;
    fs::write(&path, text.as_bytes()).map_err(|source| SiteGenerationError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(GeneratedSiteFile::new(path, bytes_written))
}

fn create_directory(path: &Path) -> Result<(), SiteGenerationError> {
    fs::create_dir_all(path).map_err(|source| SiteGenerationError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn page_id(book: &SourceBook, page_key: &str) -> SitePageId {
    let hash = stable_hash_hex(&format!("{}|{}", book.locale, page_key));
    SitePageId::new(format!("page-{}-{hash}", book.locale))
}

fn node_id(locale: &str, builder: &TocNodeBuilder) -> SiteTocNodeId {
    let hash = stable_hash_hex(&builder.id_seed);
    SiteTocNodeId::new(format!("node-{locale}-{}-{hash}", site_slug(&builder.title)))
}

fn section_seed(parent_title_path: &[String], title: &str) -> String {
    let mut path = parent_title_path.to_vec();
    path.push(normalize_title_key(title));
    path.join("/")
}

fn section_title_merge_key(title: &str) -> String {
    format!("section-title|{}", normalize_title_key(title))
}

fn page_address_merge_key(page_key: &str) -> String {
    format!("page-address|{page_key}")
}

fn normalized_page_address(html_path: &str) -> String {
    normalize_storage_path(html_path).to_string()
}

fn is_content_node_placeholder_path(html_path: &str) -> bool {
    html_path.starts_with("_CONTENTS_NODE_")
}

fn placeholder_branch_key(parent_label_path: &[String], title: &str) -> String {
    let mut path = parent_label_path.to_vec();
    path.push(normalize_title_key(title));
    format!("placeholder-branch|{}", path.join("/"))
}

fn validate_locale_code(path: &Path, locale: &str) -> Result<(), SiteGenerationError> {
    let valid = !locale.is_empty()
        && locale
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(SiteGenerationError::UnsupportedLocale {
            path: path.to_path_buf(),
            locale: locale.to_string(),
        })
    }
}

fn build_id(books: &[SourceBook]) -> String {
    let mut seed = String::new();
    for book in books {
        seed.push_str(book.id.as_str());
        seed.push('|');
        seed.push_str(&book.file_name);
        seed.push('|');
        seed.push_str(&book.file_size_bytes.to_string());
        seed.push('\n');
    }
    format!("build-{}", stable_hash_hex(&seed))
}

fn display_title(page: &TocPage) -> String {
    let title = page.title.display().trim();
    if title.is_empty() {
        "Untitled".to_string()
    } else {
        title.to_string()
    }
}

fn normalize_title_key(title: &str) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn unique_id(base: String, used_ids: &mut BTreeSet<String>) -> String {
    let base = if base.is_empty() {
        "book".to_string()
    } else {
        base
    };
    if used_ids.insert(base.clone()) {
        return base;
    }
    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if used_ids.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded counter must find a unique id")
}

fn site_slug(value: &str) -> String {
    let slug = library_slugify(value);
    if slug.is_empty() {
        "item".to_string()
    } else {
        slug
    }
}

fn stable_hash_hex(value: &str) -> String {
    let mut hasher = FnvHasher::default();
    hasher.write(value.as_bytes());
    format!("{:016x}", hasher.finish())
}

fn has_hbk_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("hbk"))
}

fn path_sort_key(path: &Path) -> (String, String) {
    (path_file_name(path), path.display().to_string())
}

fn path_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string()
}

fn path_file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string()
}
