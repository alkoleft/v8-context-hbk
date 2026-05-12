fn markdown_link_targets(plans: &[MarkdownTocExportPlan]) -> HashMap<String, PathBuf> {
    let mut targets = HashMap::new();
    for plan in plans {
        if !is_heading_only_toc_path(&plan.html_path) {
            targets
                .entry(plan.html_path.clone())
                .or_insert_with(|| plan.relative_path.clone());
        }
    }
    targets
}

fn rewrite_page_link_targets(
    page: &PageContent,
    current_output_path: &Path,
    link_targets: &impl MarkdownLinkTargets,
    source_book_ids: &HashSet<String>,
) -> String {
    let mut replacements: HashMap<String, Option<String>> = HashMap::new();
    for link in &page.links {
        if is_external_href(&link.raw_href) {
            continue;
        }
        let replacement = link
            .normalized_path
            .as_deref()
            .and_then(|target| link_targets.markdown_link_target(target, source_book_ids))
            .map(|target| {
                append_markdown_link_fragment(
                    relative_markdown_link(current_output_path, target),
                    &link.raw_href,
                )
            });
        replacements
            .entry(link.raw_href.clone())
            .and_modify(|current| {
                if current.is_none() {
                    *current = replacement.clone();
                }
            })
            .or_insert(replacement);
    }

    replace_href_attributes(
        &page.raw_html,
        &replacements,
        &page.source.html_path,
        current_output_path,
        link_targets,
        source_book_ids,
    )
}

fn is_external_href(href: &str) -> bool {
    href.contains(':') && !href.trim_start().starts_with("v8help://")
}

fn replace_href_attributes(
    html: &str,
    replacements: &HashMap<String, Option<String>>,
    current_html_path: &str,
    current_output_path: &Path,
    link_targets: &impl MarkdownLinkTargets,
    source_book_ids: &HashSet<String>,
) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    while let Some(position) = find_href_attribute(html, cursor) {
        let Some(attribute) = parse_href_attribute(html, position) else {
            let next = advance_one_char(html, position);
            output.push_str(&html[cursor..next]);
            cursor = next;
            continue;
        };
        let replacement = replacements.get(attribute.value).cloned().or_else(|| {
            href_replacement_for_raw_value(
                current_html_path,
                current_output_path,
                link_targets,
                source_book_ids,
                attribute.value,
            )
        });
        let Some(replacement) = replacement else {
            output.push_str(&html[cursor..attribute.end]);
            cursor = attribute.end;
            continue;
        };

        output.push_str(&html[cursor..attribute.start]);
        if let Some(target) = replacement {
            output.push_str(&html[attribute.start..attribute.value_start]);
            output.push_str(&target);
            output.push_str(&html[attribute.value_end..attribute.end]);
        }
        cursor = attribute.end;
    }
    output.push_str(&html[cursor..]);
    output
}

fn href_replacement_for_raw_value(
    current_html_path: &str,
    current_output_path: &Path,
    link_targets: &impl MarkdownLinkTargets,
    source_book_ids: &HashSet<String>,
    raw_href: &str,
) -> Option<Option<String>> {
    if is_external_href(raw_href) {
        return None;
    }
    let target = normalize_markdown_link_target(current_html_path, raw_href)
        .as_deref()
        .and_then(|target| link_targets.markdown_link_target(target, source_book_ids))
        .map(|target| {
            append_markdown_link_fragment(
                relative_markdown_link(current_output_path, target),
                raw_href,
            )
        });
    Some(target)
}

fn append_markdown_link_fragment(mut target: String, raw_href: &str) -> String {
    if let Some(fragment) = markdown_link_fragment(raw_href) {
        target.push('#');
        target.push_str(fragment);
    }
    target
}

fn markdown_link_fragment(raw_href: &str) -> Option<&str> {
    raw_href
        .split_once('#')
        .map(|(_, fragment)| fragment.split('?').next().unwrap_or_default().trim())
        .filter(|fragment| !fragment.is_empty())
}

fn source_book_link_ids(book: &HbkBook) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Some(stem) = book.path().file_stem().and_then(|value| value.to_str()) {
        ids.insert(stem.to_string());
        if let Some((base, _)) = stem.rsplit_once('_') {
            ids.insert(base.to_string());
        }
    }
    if !book.meta().book_name.is_empty() {
        ids.insert(book.meta().book_name.clone());
    }
    ids
}

fn normalize_markdown_link_target(current_html_path: &str, href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }
    if href.starts_with('#') {
        return Some(current_html_path.to_string());
    }
    if is_external_href(href) {
        return None;
    }

    let v8help_target = href.strip_prefix("v8help://");
    let without_scheme = v8help_target.unwrap_or(href);
    let path_part = without_scheme
        .split(['#', '?'])
        .next()
        .unwrap_or_default()
        .trim();
    if path_part.is_empty() {
        return Some(current_html_path.to_string());
    }

    let candidate = if v8help_target.is_some() || path_part.starts_with('/') {
        path_part.to_string()
    } else {
        match current_html_path.rsplit_once('/') {
            Some((base, _)) if !base.is_empty() => format!("{base}/{path_part}"),
            _ => path_part.to_string(),
        }
    };
    normalize_storage_path_segments(&candidate)
}

#[derive(Debug, Clone, Copy)]
struct HrefAttribute<'a> {
    start: usize,
    end: usize,
    value_start: usize,
    value_end: usize,
    value: &'a str,
}

fn find_href_attribute(html: &str, from: usize) -> Option<usize> {
    let bytes = html.as_bytes();
    let mut index = from;
    while index + 4 <= bytes.len() {
        if bytes[index..index + 4].eq_ignore_ascii_case(b"href")
            && html_attribute_name_start_boundary(bytes, index)
            && html_attribute_name_boundary(bytes, index + 4)
        {
            return Some(index);
        }
        index = advance_one_char(html, index);
    }
    None
}

fn html_attribute_name_start_boundary(bytes: &[u8], index: usize) -> bool {
    index == 0 || html_attribute_name_boundary(bytes, index - 1)
}

fn html_attribute_name_boundary(bytes: &[u8], index: usize) -> bool {
    index == 0
        || index >= bytes.len()
        || !matches!(
            bytes[index],
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b':'
        )
}

fn parse_href_attribute(html: &str, start: usize) -> Option<HrefAttribute<'_>> {
    let bytes = html.as_bytes();
    let mut index = start + 4;
    skip_ascii_whitespace(bytes, &mut index);
    if bytes.get(index) != Some(&b'=') {
        return None;
    }
    index += 1;
    skip_ascii_whitespace(bytes, &mut index);
    let quote = *bytes.get(index)?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let value_start = index + 1;
    let value_end = bytes[value_start..]
        .iter()
        .position(|byte| *byte == quote)
        .map(|offset| value_start + offset)?;
    Some(HrefAttribute {
        start,
        end: value_end + 1,
        value_start,
        value_end,
        value: &html[value_start..value_end],
    })
}

fn skip_ascii_whitespace(bytes: &[u8], index: &mut usize) {
    while bytes.get(*index).is_some_and(u8::is_ascii_whitespace) {
        *index += 1;
    }
}

fn advance_one_char(value: &str, index: usize) -> usize {
    value[index..]
        .chars()
        .next()
        .map(|character| index + character.len_utf8())
        .unwrap_or(value.len())
}

fn relative_markdown_link(current_output_path: &Path, target_output_path: &Path) -> String {
    let current_dir = current_output_path
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let current_components = path_components(current_dir);
    let target_components = path_components(target_output_path);
    let common = current_components
        .iter()
        .zip(&target_components)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = Vec::new();
    for _ in common..current_components.len() {
        parts.push("..".to_string());
    }
    parts.extend(target_components.into_iter().skip(common));
    if parts.is_empty() {
        "index.md".to_string()
    } else {
        parts.join("/")
    }
}

fn path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}
