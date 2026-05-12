fn write_site_data(
    output_root: PathBuf,
    data_root: &Path,
    site: SiteData,
    books: &[SourceBook],
    progress: &mut impl FnMut(SiteGenerationProgress<'_>),
) -> Result<SiteGenerationResult, SiteGenerationError> {
    let mut files = Vec::new();
    let total_files = generated_site_file_count(&site);
    let mut written_files = 0usize;
    create_directory(data_root)?;
    let manifest_path = data_root.join("manifest.json");
    written_files += 1;
    progress(SiteGenerationProgress::ArtifactWriting {
        current: written_files,
        total: total_files,
        kind: GeneratedSiteFileKind::Manifest,
        path: &manifest_path,
    });
    files.push(write_json(manifest_path, &site.manifest)?);
    for locale in site.locales {
        let locale_root = data_root.join("locales").join(&locale.locale);
        let sections_root = locale_root.join("toc-sections");
        let pages_root = locale_root.join("pages");
        create_directory(&sections_root)?;
        create_directory(&pages_root)?;
        let toc_root_path = locale_root.join("toc-root.json");
        written_files += 1;
        progress(SiteGenerationProgress::ArtifactWriting {
            current: written_files,
            total: total_files,
            kind: GeneratedSiteFileKind::TocRoot,
            path: &toc_root_path,
        });
        files.push(write_json(
            toc_root_path,
            &TocRootArtifact {
                schema_version: SCHEMA_VERSION,
                locale: locale.locale.clone(),
                nodes: locale.nodes,
            },
        )?);
        for section in locale.sections {
            let section_path = sections_root.join(format!("{}.json", section.id.as_str()));
            written_files += 1;
            progress(SiteGenerationProgress::ArtifactWriting {
                current: written_files,
                total: total_files,
                kind: GeneratedSiteFileKind::TocSection,
                path: &section_path,
            });
            files.push(write_json(
                section_path,
                &TocSectionArtifact {
                    schema_version: SCHEMA_VERSION,
                    locale: section.locale,
                    parent_id: section.id,
                    nodes: section.nodes,
                },
            )?);
        }
        let link_targets = locale_link_targets(&locale.pages, books);
        let mut page_loaders = BTreeMap::new();
        for page in &locale.pages {
            let page_path = pages_root.join(page_markdown_relative_path(page));
            written_files += 1;
            progress(SiteGenerationProgress::ArtifactWriting {
                current: written_files,
                total: total_files,
                kind: GeneratedSiteFileKind::Page,
                path: &page_path,
            });
            files.push(write_markdown_page(
                &page_path,
                page,
                &link_targets,
                books,
                &mut page_loaders,
            )?);
        }
    }
    Ok(SiteGenerationResult::new(
        output_root,
        files,
        site.locale_count,
        site.book_count,
        site.toc_node_count,
        site.page_count,
    ))
}

fn generated_site_file_count(site: &SiteData) -> usize {
    1 + site
        .locales
        .iter()
        .map(|locale| 1 + locale.sections.len() + locale.pages.len())
        .sum::<usize>()
}

fn write_markdown_page<'a>(
    output_path: &Path,
    page: &SitePageArtifactPlan,
    link_targets: &LocaleLinkTargets,
    books: &'a [SourceBook],
    page_loaders: &mut BTreeMap<SiteBookId, BookMarkdownPageLoader<'a>>,
) -> Result<GeneratedSiteFile, SiteGenerationError> {
    let book = books
        .iter()
        .find(|book| book.id == page.book_id)
        .expect("page plan must refer to a loaded source book");
    let current_output_path = page_markdown_relative_path(page);
    if !page_loaders.contains_key(&book.id) {
        let loader = BookExporter::new(&book.book)
            .markdown_page_loader()
            .map_err(|source| SiteGenerationError::Markdown {
                path: book.book.path().to_path_buf(),
                html_path: page.html_path.clone(),
                source: Box::new(source),
            })?;
        page_loaders.insert(book.id.clone(), loader);
    }
    let page_link_targets = PageLinkTargets {
        locale: link_targets,
        book_id: &page.book_id,
    };
    let source_book_ids = link_targets
        .book_source_ids
        .get(&page.book_id)
        .expect("page plan must refer to source book link ids");
    let markdown = page_loaders
        .get_mut(&book.id)
        .expect("page loader must exist for source book")
        .linked_markdown_toc_page(
            &page.html_path,
            &page.title,
            &current_output_path,
            &page_link_targets,
            source_book_ids,
        )
        .map_err(|source| SiteGenerationError::Markdown {
            path: book.book.path().to_path_buf(),
            html_path: page.html_path.clone(),
            source: Box::new(source),
        })?
        .markdown()
        .to_string();
    let markdown = collapse_current_page_fragment_links(markdown, &page.page_id);
    write_text(output_path.to_path_buf(), &markdown)
}

fn collapse_current_page_fragment_links(markdown: String, page_id: &SitePageId) -> String {
    let same_page_prefix = format!("]({}.md#", page_id.as_str());
    if !markdown.contains(&same_page_prefix) {
        return markdown;
    }
    markdown.replace(&same_page_prefix, "](#")
}

#[derive(Debug)]
struct LocaleLinkTargets {
    prefixed_targets: HashMap<String, PathBuf>,
    book_targets: BTreeMap<SiteBookId, HashMap<String, PathBuf>>,
    book_source_ids: BTreeMap<SiteBookId, HashSet<String>>,
}

fn locale_link_targets(
    locale_pages: &[SitePageArtifactPlan],
    books: &[SourceBook],
) -> LocaleLinkTargets {
    let book_source_ids = books
        .iter()
        .map(|book| (book.id.clone(), source_book_link_ids(book)))
        .collect::<BTreeMap<_, _>>();
    let mut prefixed_targets = HashMap::new();
    let mut book_targets: BTreeMap<SiteBookId, HashMap<String, PathBuf>> = BTreeMap::new();
    for page in locale_pages {
        let relative_path = page_markdown_relative_path(page);
        for (book_id, html_path) in &page.link_aliases {
            let normalized_html_path = normalize_storage_path(html_path).to_string();
            if normalized_html_path.is_empty() {
                continue;
            }
            book_targets
                .entry(book_id.clone())
                .or_default()
                .entry(normalized_html_path.clone())
                .or_insert_with(|| relative_path.clone());
            if let Some(source_ids) = book_source_ids.get(book_id) {
                for source_id in source_ids {
                    prefixed_targets
                        .entry(format!("{source_id}/{normalized_html_path}"))
                        .or_insert_with(|| relative_path.clone());
                }
            }
        }
    }
    LocaleLinkTargets {
        prefixed_targets,
        book_targets,
        book_source_ids,
    }
}

struct PageLinkTargets<'a> {
    locale: &'a LocaleLinkTargets,
    book_id: &'a SiteBookId,
}

impl MarkdownLinkTargets for PageLinkTargets<'_> {
    fn markdown_link_target(
        &self,
        normalized_path: &str,
        source_book_ids: &HashSet<String>,
    ) -> Option<&PathBuf> {
        self.locale
            .book_targets
            .get(self.book_id)
            .and_then(|targets| targets.get(normalized_path))
            .or_else(|| self.locale.prefixed_targets.get(normalized_path))
            .or_else(|| {
                normalized_path
                    .split_once('/')
                    .filter(|(book_segment, _)| source_book_ids.contains(*book_segment))
                    .and_then(|(_, path_without_book_segment)| {
                        self.locale
                            .book_targets
                            .get(self.book_id)
                            .and_then(|targets| targets.get(path_without_book_segment))
                    })
            })
    }
}

fn source_book_link_ids(book: &SourceBook) -> HashSet<String> {
    let mut ids = HashSet::new();
    ids.insert(book.id.as_str().to_string());
    if !book.book.meta().book_name.is_empty() {
        ids.insert(book.book.meta().book_name.clone());
    }
    let stem = path_file_stem(book.book.path());
    if !stem.is_empty() {
        ids.insert(stem.clone());
        if let Some((base, _)) = stem.rsplit_once('_') {
            ids.insert(base.to_string());
        }
    }
    ids
}

fn page_markdown_relative_path(page: &SitePageArtifactPlan) -> PathBuf {
    PathBuf::from(page_markdown_file_name(&page.page_id))
}
