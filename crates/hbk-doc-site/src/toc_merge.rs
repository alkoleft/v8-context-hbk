fn build_site_data(books: &[SourceBook]) -> SiteData {
    let mut locale_books: BTreeMap<String, Vec<&SourceBook>> = BTreeMap::new();
    for book in books {
        locale_books
            .entry(book.locale.clone())
            .or_default()
            .push(book);
    }

    let mut locales = Vec::new();
    let mut manifest_books = BTreeMap::new();
    let mut toc_roots = BTreeMap::new();
    let mut toc_node_count = 0;
    let mut page_count = 0;

    for (locale, books) in locale_books {
        let mut builders = Vec::new();
        let mut pages = Vec::new();
        let mut page_plan_indexes = HashMap::new();
        let resolved_placeholder_targets = collect_resolved_placeholder_targets(&books);
        let mut books_manifest = Vec::new();
        for book in books {
            books_manifest.push(ManifestBook {
                book_id: book.id.clone(),
                file_name: book.file_name.clone(),
                title: book.title.clone(),
                locale: locale.clone(),
                file_size_bytes: book.file_size_bytes,
            });
            append_toc_pages(
                &mut builders,
                &mut pages,
                &mut page_plan_indexes,
                &resolved_placeholder_targets,
                book,
                book.book.toc().pages(),
                &[],
                &[],
                &[],
            );
        }
        let mut sections = Vec::new();
        let nodes = finalize_nodes(
            &locale,
            builders,
            &mut sections,
            &mut toc_node_count,
            &mut page_count,
        );
        toc_roots.insert(locale.clone(), format!("locales/{locale}/toc-root.json"));
        manifest_books.insert(locale.clone(), books_manifest);
        locales.push(LocaleSiteData {
            locale,
            nodes,
            sections,
            pages,
        });
    }

    let manifest = SiteManifest {
        schema_version: SCHEMA_VERSION,
        generator: GENERATOR_NAME,
        generator_version: env!("CARGO_PKG_VERSION"),
        build_id: build_id(books),
        locales: locales.iter().map(|locale| locale.locale.clone()).collect(),
        books: manifest_books,
        toc_roots,
        page_roots: locales
            .iter()
            .map(|locale| {
                (
                    locale.locale.clone(),
                    format!("locales/{}/pages", locale.locale),
                )
            })
            .collect(),
    };

    SiteData {
        manifest,
        book_count: books.len(),
        locale_count: locales.len(),
        locales,
        toc_node_count,
        page_count,
    }
}

#[allow(clippy::too_many_arguments)]
fn append_toc_pages(
    output: &mut Vec<TocNodeBuilder>,
    page_plans: &mut Vec<SitePageArtifactPlan>,
    page_plan_indexes: &mut HashMap<String, usize>,
    resolved_placeholder_targets: &HashMap<String, ResolvedPageTarget>,
    book: &SourceBook,
    pages: &[TocPage],
    parent_toc_path: &[usize],
    parent_title_path: &[String],
    parent_label_path: &[String],
) {
    for (index, page) in pages.iter().enumerate() {
        let mut toc_path = parent_toc_path.to_vec();
        toc_path.push(index);
        let title = display_title(page);
        let page_bearing = !page.html_path.trim().is_empty();
        if page_bearing {
            let normalized_address = normalized_page_address(&page.html_path);
            let placeholder_target = if is_content_node_placeholder_path(&normalized_address) {
                resolved_placeholder_targets.get(&placeholder_branch_key(parent_label_path, &title))
            } else {
                None
            };
            let (owner_book_id, page_key, plan_title, plan_html_path) =
                if let Some(target) = placeholder_target {
                    (
                        target.book_id.clone(),
                        target.page_key.clone(),
                        target.title.clone(),
                        target.html_path.clone(),
                    )
                } else {
                    (
                        book.id.clone(),
                        normalized_address.clone(),
                        title.clone(),
                        page.html_path.clone(),
                    )
                };
            let merge_key = page_address_merge_key(&page_key);
            let page_plan_index = match page_plan_indexes.get(&merge_key).copied() {
                Some(index) => {
                    page_plans[index]
                        .link_aliases
                        .insert((book.id.clone(), page.html_path.clone()));
                    index
                }
                None => {
                    let page_id = page_id(book, &page_key);
                    let mut link_aliases =
                        BTreeSet::from([(owner_book_id.clone(), plan_html_path.clone())]);
                    link_aliases.insert((book.id.clone(), page.html_path.clone()));
                    let index = page_plans.len();
                    page_plans.push(SitePageArtifactPlan {
                        book_id: owner_book_id,
                        link_aliases,
                        page_id: page_id.clone(),
                        title: plan_title,
                        html_path: plan_html_path,
                    });
                    page_plan_indexes.insert(merge_key.clone(), index);
                    index
                }
            };
            let page_id = page_plans[page_plan_index].page_id.clone();
            let node_book_id = page_plans[page_plan_index].book_id.clone();
            let mut title_path = parent_title_path.to_vec();
            title_path.push(format!("page:{}", page_id.as_str()));
            let mut label_path = parent_label_path.to_vec();
            label_path.push(normalize_title_key(&title));
            let mut node = TocNodeBuilder {
                title,
                id_seed: format!("page|{}", page_id.as_str()),
                merge_key: Some(merge_key),
                book_id: Some(node_book_id),
                page_id: Some(page_id),
                children: Vec::new(),
            };
            append_toc_pages(
                &mut node.children,
                page_plans,
                page_plan_indexes,
                resolved_placeholder_targets,
                book,
                &page.children,
                &toc_path,
                &title_path,
                &label_path,
            );
            append_or_merge_node(output, node);
        } else {
            let merge_key = section_title_merge_key(&title);
            let id_seed = format!(
                "section|{}|{}",
                book.locale,
                section_seed(parent_title_path, &title)
            );
            let mut incoming = TocNodeBuilder {
                title: title.clone(),
                id_seed,
                merge_key: Some(merge_key.clone()),
                book_id: None,
                page_id: None,
                children: Vec::new(),
            };
            let mut title_path = parent_title_path.to_vec();
            title_path.push(normalize_title_key(&title));
            let mut label_path = parent_label_path.to_vec();
            label_path.push(normalize_title_key(&title));
            append_toc_pages(
                &mut incoming.children,
                page_plans,
                page_plan_indexes,
                resolved_placeholder_targets,
                book,
                &page.children,
                &toc_path,
                &title_path,
                &label_path,
            );
            append_or_merge_node(output, incoming);
        }
    }
}

fn collect_resolved_placeholder_targets(
    books: &[&SourceBook],
) -> HashMap<String, ResolvedPageTarget> {
    let mut candidates = HashMap::new();
    for book in books {
        collect_concrete_page_targets(&mut candidates, book, book.book.toc().pages(), &[]);
    }
    candidates
        .into_iter()
        .filter_map(|(key, candidate)| match candidate {
            PlaceholderTargetCandidate::One(target) => Some((key, target)),
            PlaceholderTargetCandidate::Ambiguous => None,
        })
        .collect()
}

fn collect_concrete_page_targets(
    candidates: &mut HashMap<String, PlaceholderTargetCandidate>,
    book: &SourceBook,
    pages: &[TocPage],
    parent_label_path: &[String],
) {
    for page in pages {
        let title = display_title(page);
        let normalized_address = normalized_page_address(&page.html_path);
        if !normalized_address.is_empty() && !is_content_node_placeholder_path(&normalized_address)
        {
            let key = placeholder_branch_key(parent_label_path, &title);
            let target = ResolvedPageTarget {
                book_id: book.id.clone(),
                page_key: normalized_address,
                title: title.clone(),
                html_path: page.html_path.clone(),
            };
            record_placeholder_target_candidate(candidates, key, target);
        }
        let mut label_path = parent_label_path.to_vec();
        label_path.push(normalize_title_key(&title));
        collect_concrete_page_targets(candidates, book, &page.children, &label_path);
    }
}

fn record_placeholder_target_candidate(
    candidates: &mut HashMap<String, PlaceholderTargetCandidate>,
    key: String,
    target: ResolvedPageTarget,
) {
    match candidates.entry(key) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(PlaceholderTargetCandidate::One(target));
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let candidate = entry.get_mut();
            match candidate {
                PlaceholderTargetCandidate::One(existing) => {
                    if existing.page_key != target.page_key {
                        *candidate = PlaceholderTargetCandidate::Ambiguous;
                    }
                }
                PlaceholderTargetCandidate::Ambiguous => {}
            }
        }
    }
}

fn append_or_merge_node(output: &mut Vec<TocNodeBuilder>, node: TocNodeBuilder) {
    if let Some(merge_key) = node.merge_key.as_deref()
        && let Some(existing) = output
            .iter_mut()
            .find(|candidate| candidate.merge_key.as_deref() == Some(merge_key))
    {
        merge_children(&mut existing.children, node.children);
        return;
    }
    output.push(node);
}

fn merge_children(output: &mut Vec<TocNodeBuilder>, incoming: Vec<TocNodeBuilder>) {
    for node in incoming {
        append_or_merge_node(output, node);
    }
}

fn finalize_nodes(
    locale: &str,
    builders: Vec<TocNodeBuilder>,
    sections: &mut Vec<SiteTocSection>,
    toc_node_count: &mut usize,
    page_count: &mut usize,
) -> Vec<SiteTocNode> {
    let mut nodes = Vec::with_capacity(builders.len());
    for builder in builders {
        *toc_node_count += 1;
        if builder.page_id.is_some() {
            *page_count += 1;
        }
        let id = node_id(locale, &builder);
        let child_nodes = finalize_nodes(
            locale,
            builder.children,
            sections,
            toc_node_count,
            page_count,
        );
        let children_path = if child_nodes.is_empty() {
            None
        } else {
            let path = format!("toc-sections/{}.json", id.as_str());
            sections.push(SiteTocSection {
                id: id.clone(),
                locale: locale.to_string(),
                nodes: child_nodes,
            });
            Some(path)
        };
        nodes.push(SiteTocNode {
            id,
            title: builder.title,
            book_id: builder.book_id,
            page_id: builder.page_id,
            has_children: children_path.is_some(),
            children_path,
        });
    }
    nodes
}
