fn heading_only_markdown(title: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        String::new()
    } else {
        format!("# {title}\n")
    }
}

fn is_heading_only_toc_path(html_path: &str) -> bool {
    html_path.is_empty() || is_content_node_placeholder_path(html_path)
}

fn is_content_node_placeholder_path(html_path: &str) -> bool {
    html_path.starts_with("_CONTENTS_NODE_")
}

fn documentation_error_is_missing_page(error: &DocumentationError) -> bool {
    match error {
        DocumentationError::PageRead { source, .. } => {
            matches!(source.as_ref(), BookError::MissingZipEntry { .. })
        }
    }
}

fn normalize_markdown(markdown: String) -> String {
    let normalized = markdown.replace('\r', "").replace('\u{a0}', " ");
    let lines = normalized
        .trim_matches(['\u{feff}', '\n'])
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>();
    let mut output = lines.join("\n").trim().to_string();
    if !output.is_empty() {
        output.push('\n');
    }
    output
}

fn ensure_markdown_heading(title: &str, markdown: String) -> String {
    if title.trim().is_empty()
        || markdown
            .lines()
            .find(|line| !line.trim().is_empty())
            .is_some_and(|line| line.starts_with('#'))
    {
        return markdown;
    }

    let mut output = format!("# {}\n", title.trim());
    if !markdown.trim().is_empty() {
        output.push('\n');
        output.push_str(markdown.trim_end());
        output.push('\n');
    }
    output
}
