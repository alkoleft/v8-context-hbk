fn normalize_code_examples(html: &str) -> String {
    let html = normalize_code_example_tables(html);
    let html = normalize_layout_blockquote_tables(&html);
    normalize_query_code_blockquotes(&html)
}

fn normalize_code_example_tables(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    while let Some(start) = find_ascii_case_insensitive(html, cursor, "<table") {
        let Some(end_tag_start) = find_ascii_case_insensitive(html, start, "</table") else {
            break;
        };
        let Some(end_tag_end) = html[end_tag_start..]
            .find('>')
            .map(|offset| end_tag_start + offset + 1)
        else {
            break;
        };
        let table_html = &html[start..end_tag_end];
        output.push_str(&html[cursor..start]);
        if let Some(code_block) = code_example_table_to_pre(table_html) {
            output.push_str(&code_block);
        } else {
            output.push_str(table_html);
        }
        cursor = end_tag_end;
    }
    output.push_str(&html[cursor..]);
    output
}

fn code_example_table_to_pre(table_html: &str) -> Option<String> {
    if !html_contains_ascii_case_insensitive(table_html, "courier") {
        return None;
    }

    let fragment = Html::parse_fragment(table_html);
    let cell_selector = Selector::parse("td, th").expect("static selector must be valid");
    let mut cells = fragment.select(&cell_selector);
    let cell = cells.next()?;
    if cells.next().is_some() {
        return None;
    }

    let mut code = String::new();
    collect_code_example_text(cell, &mut code);
    let code = normalize_code_example_text(&code);
    (!code.is_empty()).then(|| {
        format!(
            "<pre><code class=\"language-bsl\">{}</code></pre>",
            encode_text(&code)
        )
    })
}

fn normalize_layout_blockquote_tables(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    while let Some(start) = find_ascii_case_insensitive(html, cursor, "<blockquote") {
        let Some(end_tag_start) = find_ascii_case_insensitive(html, start, "</blockquote") else {
            break;
        };
        let Some(end_tag_end) = html[end_tag_start..]
            .find('>')
            .map(|offset| end_tag_start + offset + 1)
        else {
            break;
        };
        let blockquote_html = &html[start..end_tag_end];
        output.push_str(&html[cursor..start]);
        if let Some(blockquote) = layout_blockquote_tables_to_html(blockquote_html) {
            output.push_str(&blockquote);
        } else {
            output.push_str(blockquote_html);
        }
        cursor = end_tag_end;
    }
    output.push_str(&html[cursor..]);
    output
}

fn layout_blockquote_tables_to_html(blockquote_html: &str) -> Option<String> {
    if !html_contains_ascii_case_insensitive(blockquote_html, "<table")
        || html_contains_ascii_case_insensitive(blockquote_html, "courier")
        || html_contains_ascii_case_insensitive(blockquote_html, "href")
    {
        return None;
    }

    let fragment = Html::parse_fragment(blockquote_html);
    let blockquote_selector = Selector::parse("blockquote").expect("static selector must be valid");
    let table_selector = Selector::parse("table").expect("static selector must be valid");
    let row_selector = Selector::parse("tr").expect("static selector must be valid");
    let cell_selector = Selector::parse("td, th").expect("static selector must be valid");

    let blockquote = fragment.select(&blockquote_selector).next()?;
    let mut table_count = 0;
    let mut lines = Vec::new();

    for table in blockquote.select(&table_selector) {
        table_count += 1;
        for row in table.select(&row_selector) {
            let mut row_cells = Vec::new();
            for cell in row.select(&cell_selector) {
                let text = normalize_layout_cell_text(&cell.text().collect::<String>());
                if !text.is_empty() {
                    row_cells.push(text);
                }
            }
            match row_cells.len() {
                0 => {}
                1 => lines.push(row_cells.remove(0)),
                _ => return None,
            }
        }
    }

    if table_count < 2 || lines.len() < 2 {
        return None;
    }

    let mut output = String::from("<blockquote>");
    for line in lines {
        output.push_str("<p>");
        output.push_str(&encode_text(&line));
        output.push_str("</p>");
    }
    output.push_str("</blockquote>");
    Some(output)
}

fn normalize_layout_cell_text(text: &str) -> String {
    decode_basic_html_entities(text)
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_query_code_blockquotes(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;
    while let Some(start) = find_ascii_case_insensitive(html, cursor, "<blockquote") {
        let Some(end_tag_start) = find_ascii_case_insensitive(html, start, "</blockquote") else {
            break;
        };
        let Some(end_tag_end) = html[end_tag_start..]
            .find('>')
            .map(|offset| end_tag_start + offset + 1)
        else {
            break;
        };
        let blockquote_html = &html[start..end_tag_end];
        output.push_str(&html[cursor..start]);
        if let Some(code_block) = query_code_blockquote_to_pre(blockquote_html) {
            output.push_str(&code_block);
        } else {
            output.push_str(blockquote_html);
        }
        cursor = end_tag_end;
    }
    output.push_str(&html[cursor..]);
    output
}

fn query_code_blockquote_to_pre(blockquote_html: &str) -> Option<String> {
    if !html_contains_ascii_case_insensitive(blockquote_html, "courier")
        || html_contains_ascii_case_insensitive(blockquote_html, "href")
    {
        return None;
    }

    let fragment = Html::parse_fragment(blockquote_html);
    let blockquote_selector = Selector::parse("blockquote").expect("static selector must be valid");
    let blockquote = fragment.select(&blockquote_selector).next()?;
    let mut code = String::new();
    collect_code_example_text(blockquote, &mut code);
    let code = normalize_code_example_text(&code);
    (!code.is_empty()).then(|| {
        format!(
            "<pre><code class=\"language-sdbl\">{}</code></pre>",
            encode_text(&code)
        )
    })
}

fn collect_code_example_text(element: ElementRef<'_>, output: &mut String) {
    for child in element.children() {
        match child.value() {
            Node::Text(text) => output.push_str(text),
            Node::Element(element) => {
                let tag_name = element.name();
                if tag_name.eq_ignore_ascii_case("br") {
                    output.push('\n');
                } else if let Some(child_element) = ElementRef::wrap(child) {
                    collect_code_example_text(child_element, output);
                }
            }
            _ => {}
        }
    }
}

fn normalize_code_example_text(code: &str) -> String {
    let code = code.replace('\r', "").replace('\u{a0}', " ");
    let mut lines = code.lines().map(str::trim_end).collect::<Vec<_>>();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let mut output = lines.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    output
}

fn html_contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    find_ascii_case_insensitive(haystack, 0, needle).is_some()
}

fn find_ascii_case_insensitive(haystack: &str, from: usize, needle: &str) -> Option<usize> {
    if needle.is_empty() || from >= haystack.len() {
        return None;
    }
    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    if needle.len() > haystack.len() {
        return None;
    }

    (from..=haystack.len() - needle.len()).find(|start| {
        haystack[*start..*start + needle.len()]
            .iter()
            .zip(needle)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
    })
}
