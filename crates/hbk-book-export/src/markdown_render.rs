fn page_content_to_markdown(page: &PageContent) -> String {
    let options = MarkdownOptions::new()
        .include_links(false)
        .include_images(false)
        .preserve_tables(true)
        .escape_special_chars(false);
    let anchor_targets = markdown_heading_anchor_targets(&page.raw_html);
    let html = normalize_code_examples(&page.raw_html);
    let markdown = html_to_markdown_with_options(&html, &options);
    let markdown = ensure_markdown_heading(&page.title, normalize_markdown(markdown));
    materialize_markdown_heading_anchors(&markdown, &anchor_targets)
}

fn page_content_to_linked_markdown(
    page: &PageContent,
    current_output_path: &Path,
    link_targets: &impl MarkdownLinkTargets,
    source_book_ids: &HashSet<String>,
) -> String {
    let html = rewrite_page_link_targets(page, current_output_path, link_targets, source_book_ids);
    let anchor_targets = markdown_heading_anchor_targets(&html);
    let html = normalize_code_examples(&html);
    let options = MarkdownOptions::new()
        .include_links(true)
        .include_images(false)
        .preserve_tables(true)
        .escape_special_chars(false);
    let markdown = html_to_markdown_with_options(&html, &options);
    let markdown = ensure_markdown_heading(&page.title, normalize_markdown(markdown));
    materialize_markdown_heading_anchors(&markdown, &anchor_targets)
}

fn raw_page_to_linked_markdown(
    raw_html: &str,
    html_path: &str,
    title: &str,
    current_output_path: &Path,
    link_targets: &impl MarkdownLinkTargets,
    source_book_ids: &HashSet<String>,
) -> String {
    let empty_replacements = HashMap::new();
    let html = replace_href_attributes(
        raw_html,
        &empty_replacements,
        html_path,
        current_output_path,
        link_targets,
        source_book_ids,
    );
    let anchor_targets = markdown_heading_anchor_targets(&html);
    let html = normalize_code_examples(&html);
    let options = MarkdownOptions::new()
        .include_links(true)
        .include_images(false)
        .preserve_tables(true)
        .escape_special_chars(false);
    let markdown = html_to_markdown_with_options(&html, &options);
    let markdown = normalize_markdown(markdown);
    let title = if markdown_starts_with_heading(&markdown) {
        title.to_string()
    } else {
        raw_html_page_title(raw_html, title)
    };
    let markdown = ensure_markdown_heading(&title, markdown);
    materialize_markdown_heading_anchors(&markdown, &anchor_targets)
}

fn markdown_starts_with_heading(markdown: &str) -> bool {
    markdown
        .lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.starts_with('#'))
}

fn raw_html_page_title(raw_html: &str, fallback: &str) -> String {
    first_html_element_text(raw_html, "title")
        .or_else(|| first_html_element_text(raw_html, "h1"))
        .unwrap_or_else(|| fallback.to_string())
}

fn first_html_element_text(raw_html: &str, tag_name: &str) -> Option<String> {
    let fragment = Html::parse_fragment(raw_html);
    let selector = Selector::parse(tag_name).expect("static selector must be valid");
    fragment
        .select(&selector)
        .find_map(|element| {
            let text = normalize_html_text(&element.text().collect::<String>());
            (!text.is_empty()).then_some(text)
        })
}

fn normalize_html_text(html: &str) -> String {
    decode_basic_html_entities(html)
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_basic_html_entities(text: &str) -> String {
    decode_html_entities(text).into_owned()
}
