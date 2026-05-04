use hbk_docs::PageContent;
use scraper::{Html, Selector};
use syntax_helper_model::{LocalizedName, MemberLink};

pub(crate) fn page_title_name(content: &PageContent) -> LocalizedName {
    name_from_text(
        &select_first_html_text(&content.raw_html, ".V8SH_pagetitle")
            .unwrap_or_else(|| content.title.clone()),
    )
}

pub(crate) fn title_name(content: &PageContent) -> LocalizedName {
    name_from_text(
        &select_first_html_text(&content.raw_html, ".V8SH_title")
            .unwrap_or_else(|| content.title.clone()),
    )
}

pub(crate) fn heading_name(content: &PageContent) -> LocalizedName {
    name_from_text(
        &select_first_html_text(&content.raw_html, ".V8SH_heading")
            .unwrap_or_else(|| content.title.clone()),
    )
}

pub(crate) fn name_from_text(value: &str) -> LocalizedName {
    let value = value.trim();
    if let Some((primary, alias)) = split_parenthesized_alias(value) {
        LocalizedName {
            primary,
            alias: Some(alias),
        }
    } else {
        LocalizedName {
            primary: value.to_string(),
            alias: None,
        }
    }
}

fn split_parenthesized_alias(value: &str) -> Option<(String, String)> {
    let value = value.trim();
    let alias_end = value.strip_suffix(')')?;
    let alias_start = alias_end.rfind(" (")?;
    let primary = alias_end[..alias_start].trim();
    let alias = alias_end[alias_start + 2..].trim();
    (!primary.is_empty() && !alias.is_empty()).then(|| (primary.to_string(), alias.to_string()))
}

pub(crate) fn select_first_html_text(raw_html: &str, selector: &str) -> Option<String> {
    if let Some(class_name) = selector.strip_prefix('.') {
        return select_first_class_text(raw_html, class_name);
    }
    if selector == "title" {
        return select_first_tag_text(raw_html, "title");
    }
    let document = Html::parse_document(raw_html);
    let selector = Selector::parse(selector).expect("static selector must be valid");
    document
        .select(&selector)
        .find_map(|element| non_empty_text(element.text()))
}

pub(crate) fn body_text(raw_html: &str) -> String {
    let body = raw_html
        .find("<body")
        .and_then(|start| raw_html[start..].find('>').map(|offset| start + offset + 1))
        .and_then(|start| {
            raw_html[start..]
                .find("</body>")
                .map(|end| &raw_html[start..start + end])
        })
        .unwrap_or(raw_html);
    text_from_html_fragment(body)
}

fn select_first_class_text(raw_html: &str, class_name: &str) -> Option<String> {
    let class_marker = format!("class=\"{class_name}\"");
    let start = raw_html.find(&class_marker)?;
    let tag_start = raw_html[..start].rfind('<')?;
    let content_start = raw_html[start..]
        .find('>')
        .map(|offset| start + offset + 1)?;
    let tag_name = raw_html[tag_start + 1..]
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_start_matches('/');
    let end_tag = format!("</{tag_name}>");
    let content_end = raw_html[content_start..]
        .find(&end_tag)
        .map(|offset| content_start + offset)?;
    let text = text_from_html_fragment(&raw_html[content_start..content_end]);
    (!text.is_empty()).then_some(text)
}

fn select_first_tag_text(raw_html: &str, tag_name: &str) -> Option<String> {
    let start_tag = format!("<{tag_name}");
    let start = raw_html.find(&start_tag)?;
    let content_start = raw_html[start..]
        .find('>')
        .map(|offset| start + offset + 1)?;
    let end_tag = format!("</{tag_name}>");
    let content_end = raw_html[content_start..]
        .find(&end_tag)
        .map(|offset| content_start + offset)?;
    let text = text_from_html_fragment(&raw_html[content_start..content_end]);
    (!text.is_empty()).then_some(text)
}

fn text_from_html_fragment(fragment: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    let mut entity = String::new();
    let mut in_entity = false;
    let mut chars = fragment.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
                output.push(' ');
            }
            continue;
        }
        if in_entity {
            if ch == ';' {
                output.push_str(decode_entity(&entity));
                entity.clear();
                in_entity = false;
            } else {
                entity.push(ch);
            }
            continue;
        }
        match ch {
            '<' if chars
                .peek()
                .is_some_and(|next| next.is_ascii_alphabetic() || *next == '/') =>
            {
                in_tag = true
            }
            '<' => output.push('<'),
            '&' => in_entity = true,
            ch if ch.is_whitespace() => output.push(' '),
            ch => output.push(ch),
        }
    }
    collapse_whitespace(&output)
}

pub(crate) fn text_lines_from_html_fragment(fragment: &str) -> String {
    let with_breaks = fragment
        .replace("<BR>", "\n")
        .replace("<BR/>", "\n")
        .replace("<BR />", "\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n")
        .replace("</div>", "\n");
    with_breaks
        .lines()
        .map(text_from_html_fragment)
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn anchor_links(section_html: &str, current_html_path: &str) -> Vec<MemberLink> {
    let mut links = Vec::new();
    let mut rest = section_html;
    while let Some(anchor_start) = rest.find("<a ") {
        rest = &rest[anchor_start..];
        let Some(tag_end) = rest.find('>') else {
            break;
        };
        let tag = &rest[..tag_end + 1];
        let Some(raw_href) = attr_value(tag, "href") else {
            rest = &rest[tag_end + 1..];
            continue;
        };
        let Some(anchor_end) = rest[tag_end + 1..].find("</a>") else {
            break;
        };
        let inner = &rest[tag_end + 1..tag_end + 1 + anchor_end];
        let text = text_from_html_fragment(inner);
        if !text.is_empty() {
            links.push(MemberLink {
                name: name_from_text(&text),
                html_path: normalize_member_href(current_html_path, &raw_href),
            });
        }
        rest = &rest[tag_end + 1 + anchor_end + 4..];
    }
    links
}

fn attr_value(tag: &str, attr_name: &str) -> Option<String> {
    let attr = format!("{attr_name}=\"");
    let start = tag.find(&attr)? + attr.len();
    let end = tag[start..].find('"')?;
    Some(tag[start..start + end].to_string())
}

pub(crate) fn bracketed_name_ranges(section: &str) -> Vec<(usize, usize, String)> {
    let mut ranges = Vec::new();
    let mut offset = 0;
    while let Some(start) = section[offset..].find('<').map(|start| offset + start) {
        let Some(end) = section[start + 1..].find('>').map(|end| start + 1 + end) else {
            break;
        };
        ranges.push((start, end + 1, section[start + 1..end].to_string()));
        offset = end + 1;
    }
    ranges
}

fn decode_entity(entity: &str) -> &str {
    match entity {
        "lt" => "<",
        "gt" => ">",
        "amp" => "&",
        "quot" => "\"",
        "nbsp" => " ",
        _ => "",
    }
}

pub(crate) fn links_in_section(content: &PageContent, labels: &[&str]) -> Vec<MemberLink> {
    let Some(section_html) = section_html(&content.raw_html, labels) else {
        return Vec::new();
    };
    anchor_links(section_html, &content.source.html_path)
}

pub(crate) fn see_also_links_in_section(content: &PageContent, labels: &[&str]) -> Vec<MemberLink> {
    let Some(section_html) = section_html(&content.raw_html, labels) else {
        return Vec::new();
    };
    compose_owner_member_links(anchor_links(section_html, &content.source.html_path))
}

pub(crate) fn section_text(content: &PageContent, labels: &[&str]) -> Option<String> {
    let body = &content.body_text;
    let (label, start) = find_label(body, labels)?;
    let section_start = start + label.len();
    let section_end = text_section_end(body, section_start, label);
    let value = body[section_start..section_end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub(crate) fn section_html<'a>(raw_html: &'a str, labels: &[&str]) -> Option<&'a str> {
    let (label, start) = find_html_section_label(raw_html, labels)?;
    let heading = html_section_heading(raw_html, start);
    let section_start = html_section_body_start(raw_html, start, label, heading);
    let section_end = html_section_end(raw_html, section_start, label, heading);
    Some(&raw_html[section_start..section_end])
}

fn find_html_section_label<'a>(raw_html: &str, labels: &'a [&str]) -> Option<(&'a str, usize)> {
    find_v8_chapter_label(raw_html, labels).or_else(|| find_label(raw_html, labels))
}

fn find_v8_chapter_label<'a>(raw_html: &str, labels: &'a [&str]) -> Option<(&'a str, usize)> {
    let mut offset = 0;
    while let Some(tag_start) = find_next_tag(raw_html, offset, "p") {
        let Some(tag_end) = raw_html[tag_start..]
            .find('>')
            .map(|index| tag_start + index)
        else {
            break;
        };
        if tag_contains_v8_chapter_class(&raw_html[tag_start..=tag_end]) {
            let body_start = tag_end + 1;
            let body_end = find_closing_p(raw_html, body_start).unwrap_or(raw_html.len());
            if let Some((label, index)) = labels.iter().find_map(|label| {
                raw_html[body_start..body_end]
                    .find(label)
                    .map(|index| (*label, body_start + index))
            }) {
                return Some((label, index));
            }
        }
        offset = tag_end + 1;
    }
    None
}

fn html_section_body_start(
    raw_html: &str,
    label_start: usize,
    label: &str,
    heading: HtmlSectionHeading,
) -> usize {
    if heading == HtmlSectionHeading::V8Chapter {
        return find_closing_p(raw_html, label_start)
            .map(|index| index + 4)
            .unwrap_or(label_start + label.len());
    }
    label_start + label.len()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HtmlSectionHeading {
    V8Chapter,
    Other,
}

fn html_section_heading(raw_html: &str, label_start: usize) -> HtmlSectionHeading {
    let tag_start = raw_html[..label_start].rfind('<');
    let tag_end = raw_html[label_start..]
        .find('>')
        .map(|index| label_start + index);
    if tag_start.zip(tag_end).is_some_and(|(tag_start, tag_end)| {
        tag_contains_v8_chapter_class(&raw_html[tag_start..=tag_end])
    }) {
        HtmlSectionHeading::V8Chapter
    } else {
        HtmlSectionHeading::Other
    }
}

fn text_section_end(value: &str, section_start: usize, current_label: &str) -> usize {
    text_section_boundaries()
        .filter(|candidate| *candidate != current_label)
        .filter_map(|candidate| {
            value[section_start..]
                .find(candidate)
                .map(|index| section_start + index)
        })
        .min()
        .unwrap_or(value.len())
}

fn html_section_end(
    value: &str,
    section_start: usize,
    current_label: &str,
    heading: HtmlSectionHeading,
) -> usize {
    if heading == HtmlSectionHeading::V8Chapter {
        return [
            next_v8_chapter_start(value, section_start),
            next_html_footer_start(value, section_start),
        ]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(value.len());
    }

    text_section_boundaries()
        .chain(HTML_SECTION_BOUNDARIES.iter().copied())
        .filter(|candidate| *candidate != current_label)
        .filter_map(|candidate| {
            value[section_start..]
                .find(candidate)
                .map(|index| section_start + index)
        })
        .min()
        .unwrap_or(value.len())
}

fn text_section_boundaries() -> impl Iterator<Item = &'static str> {
    ALL_SECTION_LABELS
        .iter()
        .chain(SERVICE_FOOTER_LABELS.iter())
        .copied()
}

fn find_label<'a>(value: &str, labels: &'a [&str]) -> Option<(&'a str, usize)> {
    labels
        .iter()
        .filter_map(|label| value.find(label).map(|index| (*label, index)))
        .min_by_key(|(_, index)| *index)
}

fn next_v8_chapter_start(value: &str, start: usize) -> Option<usize> {
    let mut offset = start;
    while let Some(tag_start) = find_next_tag(value, offset, "p") {
        let Some(tag_end) = value[tag_start..].find('>').map(|index| tag_start + index) else {
            break;
        };
        if tag_contains_v8_chapter_class(&value[tag_start..=tag_end]) {
            return Some(tag_start);
        }
        offset = tag_end + 1;
    }
    None
}

fn find_next_tag(value: &str, start: usize, tag_name: &str) -> Option<usize> {
    let lower_tag_name = tag_name.to_ascii_lowercase();
    let mut offset = start;
    while let Some(index) = value[offset..].find('<').map(|index| offset + index) {
        let after_start = &value[index + 1..];
        if after_start.starts_with('/') {
            offset = index + 1;
            continue;
        }
        let tag = after_start
            .split([' ', '>', '\t', '\n', '\r'])
            .next()
            .unwrap_or_default();
        if tag.to_ascii_lowercase() == lower_tag_name {
            return Some(index);
        }
        offset = index + 1;
    }
    None
}

fn tag_contains_v8_chapter_class(tag: &str) -> bool {
    let lower = tag.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut search_from = 0;

    while let Some(relative_index) = lower[search_from..].find("class") {
        let name_start = search_from + relative_index;
        let name_end = name_start + "class".len();
        let before_ok = name_start == 0 || !is_attr_name_byte(bytes[name_start - 1]);
        let after_ok = name_end >= bytes.len() || !is_attr_name_byte(bytes[name_end]);
        if !before_ok || !after_ok {
            search_from = name_end;
            continue;
        }

        let mut index = name_end;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            search_from = name_end;
            continue;
        }
        index += 1;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            index += 1;
        }
        let Some(quote @ (b'"' | b'\'')) = bytes.get(index).copied() else {
            search_from = name_end;
            continue;
        };
        index += 1;
        let value_start = index;
        while bytes.get(index).is_some_and(|byte| *byte != quote) {
            index += 1;
        }

        let class_value = &lower[value_start..index];
        if class_value
            .split_ascii_whitespace()
            .any(|class| class == "v8sh_chapter")
        {
            return true;
        }
        search_from = index;
    }

    false
}

fn is_attr_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':')
}

fn find_closing_p(value: &str, start: usize) -> Option<usize> {
    value[start..]
        .to_ascii_lowercase()
        .find("</p>")
        .map(|index| start + index)
}

fn next_html_footer_start(value: &str, start: usize) -> Option<usize> {
    HTML_SECTION_BOUNDARIES
        .iter()
        .filter_map(|candidate| value[start..].find(candidate).map(|index| start + index))
        .min()
}

fn normalize_member_href(current_html_path: &str, href: &str) -> String {
    let without_scheme = href
        .strip_prefix("v8help://SyntaxHelperContext/")
        .or_else(|| href.strip_prefix("v8help://"))
        .unwrap_or(href);
    let path = without_scheme.split(['#', '?']).next().unwrap_or_default();
    if path.starts_with('/') || path.starts_with("objects/") {
        return path.trim_start_matches('/').to_string();
    }
    let base = current_html_path
        .rsplit_once('/')
        .map(|(base, _)| base)
        .unwrap_or("");
    if base.is_empty() {
        path.to_string()
    } else {
        format!("{base}/{path}")
    }
}

fn compose_owner_member_links(links: Vec<MemberLink>) -> Vec<MemberLink> {
    let mut output = Vec::with_capacity(links.len());
    let mut index = 0;
    while let Some(link) = links.get(index) {
        if let Some(next) = links.get(index + 1)
            && is_member_path_of(&link.html_path, &next.html_path)
        {
            output.push(MemberLink {
                name: compose_names(&link.name, &next.name),
                html_path: next.html_path.clone(),
            });
            index += 2;
        } else {
            output.push(link.clone());
            index += 1;
        }
    }
    output
}

fn is_member_path_of(owner_path: &str, member_path: &str) -> bool {
    let owner_base = owner_path
        .strip_suffix(".html")
        .unwrap_or(owner_path)
        .trim_end_matches('/');
    member_path
        .strip_prefix(owner_base)
        .is_some_and(|tail| tail.starts_with('/'))
}

fn compose_names(owner: &LocalizedName, member: &LocalizedName) -> LocalizedName {
    LocalizedName {
        primary: format!("{}.{}", owner.primary, member.primary),
        alias: owner
            .alias
            .as_ref()
            .zip(member.alias.as_ref())
            .map(|(owner, member)| format!("{owner}.{member}")),
    }
}

fn non_empty_text<'a>(parts: impl Iterator<Item = &'a str>) -> Option<String> {
    let text = parts.collect::<Vec<_>>().join(" ");
    let text = collapse_whitespace(&text);
    (!text.is_empty()).then_some(text)
}

fn collapse_whitespace(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut pending_space = false;
    for ch in value.chars() {
        if ch.is_whitespace() {
            pending_space = !output.is_empty();
            continue;
        }
        if pending_space {
            output.push(' ');
            pending_space = false;
        }
        output.push(ch);
    }
    output
}

const ALL_SECTION_LABELS: &[&str] = &[
    "Свойства:",
    "Properties:",
    "Методы:",
    "Methods:",
    "События:",
    "Events:",
    "Синтаксис:",
    "Syntax:",
    "Параметры:",
    "Parameters:",
    "Возвращаемое значение:",
    "Return value:",
    "Returned value:",
    "Использование:",
    "Use:",
    "Значения",
    "Values",
    "Элементы коллекции:",
    "Collection items:",
    "Конструкторы:",
    "Constructors:",
    "Описание:",
    "Description:",
    "Доступность:",
    "Availability:",
    "Примечание:",
    "Note:",
    "Пример:",
    "Example:",
    "См. также:",
    "See also:",
    "Использование в версии:",
    "Available since:",
    "Вариант синтаксиса:",
    "Syntax variant:",
    "Описание варианта метода:",
    "Description of method variant:",
];

const SERVICE_FOOTER_LABELS: &[&str] = &["Методическая информация", "Methodical information"];

const HTML_SECTION_BOUNDARIES: &[&str] = &["<HR", "<hr"];
