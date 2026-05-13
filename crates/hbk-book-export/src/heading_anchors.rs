#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkdownHeadingAnchorTarget {
    level: usize,
    text: String,
    id: String,
}

fn markdown_heading_anchor_targets(html: &str) -> Vec<MarkdownHeadingAnchorTarget> {
    let fragment = Html::parse_fragment(html);
    let heading_selector =
        Selector::parse("h1, h2, h3, h4, h5, h6").expect("static selector must be valid");
    let anchor_selector = Selector::parse("[name], [id]").expect("static selector must be valid");
    let mut targets = Vec::new();

    for heading in fragment.select(&heading_selector) {
        let Some(level) = markdown_heading_level(heading.value().name()) else {
            continue;
        };
        let Some(id) = element_anchor_id(heading)
            .or_else(|| heading.select(&anchor_selector).find_map(element_anchor_id))
        else {
            continue;
        };
        let text = normalize_markdown_heading_text(&heading.text().collect::<String>());
        if text.is_empty() {
            continue;
        }
        targets.push(MarkdownHeadingAnchorTarget { level, text, id });
    }

    targets
}

fn markdown_heading_level(tag_name: &str) -> Option<usize> {
    let level = tag_name.strip_prefix('h')?.parse::<usize>().ok()?;
    (1..=6).contains(&level).then_some(level)
}

fn element_anchor_id(element: ElementRef<'_>) -> Option<String> {
    element
        .value()
        .attr("name")
        .or_else(|| element.value().attr("id"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn materialize_markdown_heading_anchors(
    markdown: &str,
    targets: &[MarkdownHeadingAnchorTarget],
) -> String {
    if targets.is_empty() {
        return markdown.to_string();
    }

    let mut output = String::with_capacity(markdown.len() + targets.len() * 24);
    let mut next_target = 0;
    for segment in markdown.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        if let Some((level, text)) = parse_markdown_heading_line(line) {
            if !targets
                .get(next_target)
                .is_some_and(|target| target.level == level && target.text == text)
            {
                output.push_str(segment);
                continue;
            }
            output.push_str("<a id=\"");
            output.push_str(&encode_double_quoted_attribute(&targets[next_target].id));
            output.push_str("\"></a>\n");
            next_target += 1;
        }
        output.push_str(segment);
    }
    output
}

fn parse_markdown_heading_line(line: &str) -> Option<(usize, String)> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if !(1..=6).contains(&hashes)
        || !line
            .as_bytes()
            .get(hashes)
            .is_some_and(u8::is_ascii_whitespace)
    {
        return None;
    }
    let text = normalize_markdown_heading_text(&line[hashes..]);
    (!text.is_empty()).then_some((hashes, text))
}

fn normalize_markdown_heading_text(text: &str) -> String {
    text.replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
