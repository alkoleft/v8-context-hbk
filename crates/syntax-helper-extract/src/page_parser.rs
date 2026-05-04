use std::collections::HashMap;
use std::path::Path;

use hbk_book::Toc;
use hbk_docs::{PageContent, PageSource};
use syntax_helper_model::*;

use crate::html::{
    body_text, bracketed_name_ranges, heading_name, links_in_section, page_title_name,
    section_html, section_text, see_also_links_in_section, select_first_html_text,
    text_lines_from_html_fragment, title_name,
};

pub fn parse_global_context(content: &PageContent, source: SyntaxHelperSource) -> GlobalContext {
    parse_global_context_for_mode(content, source, SyntaxHelperRecordDetailMode::Full)
}

pub(crate) fn parse_global_context_for_mode(
    content: &PageContent,
    source: SyntaxHelperSource,
    mode: SyntaxHelperRecordDetailMode,
) -> GlobalContext {
    if mode == SyntaxHelperRecordDetailMode::LeanConsumerExport {
        return GlobalContext {
            name: LocalizedName {
                primary: String::new(),
                alias: None,
            },
            property_links: Vec::new(),
            method_links: Vec::new(),
            description: None,
            source,
        };
    }

    GlobalContext {
        name: page_title_name(content),
        property_links: links_in_section(content, &["Свойства:", "Properties:"]),
        method_links: links_in_section(content, &["Методы:", "Methods:"]),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_global_method(content: &PageContent, source: SyntaxHelperSource) -> GlobalMethod {
    GlobalMethod {
        name: heading_name(content),
        signatures: parse_signatures(content),
        return_types: type_refs_from_section(
            content,
            &["Возвращаемое значение:", "Return value:", "Returned value:"],
        ),
        description: section_text(content, &["Описание:", "Description:"]),
        facts: section_facts(content),
        source,
    }
}

pub fn parse_global_property(content: &PageContent, source: SyntaxHelperSource) -> GlobalProperty {
    GlobalProperty {
        name: heading_name(content),
        usage: section_text(content, &["Использование:", "Use:"]),
        type_refs: type_refs_from_section(content, &["Описание:", "Description:"]),
        description: section_text(content, &["Описание:", "Description:"]),
        facts: section_facts(content),
        source,
    }
}

pub fn parse_global_context_event(
    content: &PageContent,
    source: SyntaxHelperSource,
) -> GlobalContextEvent {
    GlobalContextEvent {
        name: heading_name(content),
        semantic: SemanticContext::new(BranchKind::GlobalContext, RecordFamily::ModuleEvent),
        module: ModuleEventContext::default(),
        signatures: parse_signatures(content),
        description: section_text(content, &["Описание:", "Description:"]),
        facts: section_facts(content),
        source,
    }
}

pub fn parse_platform_type(content: &PageContent, source: SyntaxHelperSource) -> PlatformType {
    parse_platform_type_for_mode(content, source, SyntaxHelperRecordDetailMode::Full)
}

pub(crate) fn parse_platform_type_for_mode(
    content: &PageContent,
    source: SyntaxHelperSource,
    mode: SyntaxHelperRecordDetailMode,
) -> PlatformType {
    let parse_links = mode == SyntaxHelperRecordDetailMode::Full;
    PlatformType {
        name: page_title_name(content),
        semantic: SemanticContext::new(BranchKind::PlatformObjects, RecordFamily::PlatformType),
        type_kind: PlatformTypeKind::Regular,
        extends: Vec::new(),
        metadata_kind: None,
        template_parameters: Vec::new(),
        method_links: if parse_links {
            links_in_section(content, &["Методы:", "Methods:"])
        } else {
            Vec::new()
        },
        constructor_links: if parse_links {
            links_in_section(content, &["Конструкторы:", "Constructors:"])
        } else {
            Vec::new()
        },
        description: section_text(content, &["Описание:", "Description:"]),
        facts: section_facts(content),
        source,
    }
}

pub fn parse_query_table(content: &PageContent, source: SyntaxHelperSource) -> QueryTable {
    QueryTable {
        name: page_title_name(content).primary,
        semantic: SemanticContext::new(BranchKind::QueryTables, RecordFamily::QueryTable),
        table_role: QueryTableRole::Unknown,
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_platform_method(content: &PageContent, source: SyntaxHelperSource) -> PlatformMethod {
    PlatformMethod {
        owner: title_name(content),
        name: heading_name(content),
        semantic: SemanticContext::new(BranchKind::PlatformObjects, RecordFamily::TypeMethod),
        signatures: parse_signatures(content),
        return_types: type_refs_from_section(
            content,
            &["Возвращаемое значение:", "Return value:", "Returned value:"],
        ),
        description: section_text(content, &["Описание:", "Description:"]),
        facts: section_facts(content),
        source,
    }
}

pub fn parse_platform_property(
    content: &PageContent,
    source: SyntaxHelperSource,
) -> PlatformProperty {
    PlatformProperty {
        owner: title_name(content),
        name: heading_name(content),
        semantic: SemanticContext::new(BranchKind::PlatformObjects, RecordFamily::TypeProperty),
        usage: section_text(content, &["Использование:", "Use:"]),
        type_refs: type_refs_from_section(content, &["Описание:", "Description:"]),
        description: section_text(content, &["Описание:", "Description:"]),
        facts: section_facts(content),
        source,
    }
}

pub fn parse_query_table_field(
    content: &PageContent,
    owner: LocalizedName,
    source: SyntaxHelperSource,
) -> QueryTableField {
    let body = detail_body_after_heading(content);
    QueryTableField {
        owner,
        name: page_title_name(content).primary,
        semantic: SemanticContext::new(BranchKind::QueryTables, RecordFamily::QueryTableField),
        type_refs: parse_type_refs(&body),
        description: description_after_type(&body),
        note: section_text(content, &["Примечание:", "Note:"]),
        source,
    }
}

pub fn parse_query_table_parameter(
    content: &PageContent,
    owner: LocalizedName,
    source: SyntaxHelperSource,
) -> QueryTableParameter {
    let body = first_chapter_body(content).unwrap_or_else(|| detail_body_after_heading(content));
    QueryTableParameter {
        owner,
        name: page_title_name(content).primary,
        semantic: SemanticContext::new(BranchKind::QueryTables, RecordFamily::QueryTableParameter),
        type_refs: parse_type_refs(&body),
        description: table_parameter_description(&body),
        default_value: default_value(&body),
        source,
    }
}

pub fn parse_constructor(content: &PageContent, source: SyntaxHelperSource) -> Constructor {
    Constructor {
        owner: title_name(content),
        name: heading_name(content),
        semantic: SemanticContext::new(BranchKind::PlatformObjects, RecordFamily::TypeConstructor),
        signatures: parse_signatures(content),
        description: section_text(content, &["Описание:", "Description:"]),
        facts: section_facts(content),
        source,
    }
}

pub fn parse_enum(content: &PageContent, source: SyntaxHelperSource) -> EnumDefinition {
    parse_enum_for_mode(content, source, SyntaxHelperRecordDetailMode::Full)
}

pub(crate) fn parse_enum_for_mode(
    content: &PageContent,
    source: SyntaxHelperSource,
    mode: SyntaxHelperRecordDetailMode,
) -> EnumDefinition {
    EnumDefinition {
        name: page_title_name(content),
        value_links: if mode == SyntaxHelperRecordDetailMode::Full {
            links_in_section(content, &["Значения", "Values"])
        } else {
            Vec::new()
        },
        description: section_text(content, &["Описание:", "Description:"]),
        facts: section_facts(content),
        source,
    }
}

pub fn parse_enum_value(content: &PageContent, source: SyntaxHelperSource) -> EnumValue {
    EnumValue {
        owner: title_name(content),
        name: heading_name(content),
        description: section_text(content, &["Описание:", "Description:"]),
        facts: section_facts(content),
        source,
    }
}

fn section_facts(content: &PageContent) -> SectionFacts {
    SectionFacts {
        availability: availability(content),
        examples: examples(content),
        see_also: see_also_links_in_section(content, &["См. также:", "See also:"]),
        available_since: available_since(content),
    }
}

fn availability(content: &PageContent) -> Availability {
    let contexts = section_text(content, &["Доступность:", "Availability:"])
        .map(|text| availability_contexts(&text))
        .unwrap_or_default();
    Availability { contexts }
}

fn availability_contexts(text: &str) -> Vec<AvailabilityContext> {
    let tokens = text
        .split([',', ';', '.'])
        .map(normalize_availability_token)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    AVAILABILITY_CONTEXT_LABELS
        .iter()
        .filter_map(|(context, labels)| {
            tokens
                .iter()
                .any(|token| labels.iter().any(|label| token == label))
                .then_some(*context)
        })
        .collect()
}

fn normalize_availability_token(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace('\u{2011}', "-")
        .replace('\u{2013}', "-")
}

const AVAILABILITY_CONTEXT_LABELS: &[(AvailabilityContext, &[&str])] = &[
    (
        AvailabilityContext::ThinClient,
        &["thin client", "тонкий клиент"],
    ),
    (
        AvailabilityContext::WebClient,
        &["web-client", "web client", "веб-клиент", "веб клиент"],
    ),
    (
        AvailabilityContext::MobileClient,
        &["mobile client", "мобильный клиент"],
    ),
    (AvailabilityContext::Server, &["server", "сервер"]),
    (
        AvailabilityContext::ThickClient,
        &["thick client", "толстый клиент"],
    ),
    (
        AvailabilityContext::ExternalConnection,
        &["external connection", "внешнее соединение"],
    ),
    (
        AvailabilityContext::MobileApplicationClient,
        &[
            "mobile application (client)",
            "мобильное приложение (клиент)",
        ],
    ),
    (
        AvailabilityContext::MobileApplicationServer,
        &[
            "mobile application (server)",
            "мобильное приложение (сервер)",
        ],
    ),
    (
        AvailabilityContext::MobileStandaloneServer,
        &["mobile standalone server", "мобильный автономный сервер"],
    ),
];

fn examples(content: &PageContent) -> Vec<ExampleBlock> {
    let Some(section_html) = section_html(&content.raw_html, &["Пример:", "Example:"]) else {
        return Vec::new();
    };
    let text = text_lines_from_html_fragment(section_html)
        .trim()
        .to_string();
    let text = normalize_example_text(&text);
    if text.is_empty() {
        Vec::new()
    } else {
        vec![ExampleBlock { text }]
    }
}

fn normalize_example_text(text: &str) -> String {
    let mut in_string = false;
    text.lines()
        .map(|line| normalize_example_line(line, &mut in_string))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn normalize_example_line(line: &str, in_string: &mut bool) -> String {
    let mut output = String::with_capacity(line.len());
    let mut pending_space = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            if pending_space && should_keep_space_before(ch, output.chars().last(), *in_string) {
                output.push(' ');
            }
            pending_space = false;
            output.push(ch);
            if *in_string && chars.peek().is_some_and(|next| *next == '"') {
                if let Some(escaped) = chars.next() {
                    output.push(escaped);
                }
            } else {
                *in_string = !*in_string;
            }
            continue;
        }
        if ch.is_whitespace() {
            pending_space = !output.is_empty();
            continue;
        }
        if pending_space && should_keep_space_before(ch, output.chars().last(), *in_string) {
            output.push(' ');
        }
        pending_space = false;
        output.push(ch);
    }
    output.trim().to_string()
}

fn should_keep_space_before(ch: char, previous: Option<char>, in_string: bool) -> bool {
    if in_string {
        return true;
    }
    if matches!(ch, '.' | ',' | ';' | ')' | ']' | ':') {
        return false;
    }
    if matches!(ch, '(' | '[') {
        return false;
    }
    !matches!(previous, Some('.' | '(' | '['))
}

fn available_since(content: &PageContent) -> Option<VersionFact> {
    select_first_html_text(&content.raw_html, ".V8SH_versionInfo")
        .or_else(|| section_text(content, &["Использование в версии:", "Available since:"]))
        .or_else(|| select_first_html_text(&content.raw_html, ".__SINCE_SHOW_STYLE__"))
        .and_then(|text| version_fact(text.trim()))
}

fn version_fact(text: &str) -> Option<VersionFact> {
    (!text.is_empty()).then(|| VersionFact {
        version: extract_version(text),
        text: text.to_string(),
    })
}

fn extract_version(text: &str) -> Option<String> {
    let start = text
        .char_indices()
        .find_map(|(index, ch)| ch.is_ascii_digit().then_some(index))?;
    let end = text[start..]
        .char_indices()
        .find_map(|(offset, ch)| (!(ch.is_ascii_digit() || ch == '.')).then_some(start + offset))
        .unwrap_or(text.len());
    let version = text[start..end].trim_end_matches('.');
    (!version.is_empty()).then(|| version.to_string())
}

pub(crate) fn source_from_content(
    fallback: &SyntaxHelperSource,
    content: &PageContent,
) -> SyntaxHelperSource {
    SyntaxHelperSource {
        hbk_path: content.source.hbk_path.clone(),
        locale: content.source.locale.clone(),
        toc_path: content
            .source
            .toc_path
            .clone()
            .or_else(|| fallback.toc_path.clone()),
        html_path: content.source.html_path.clone(),
        page_title: if content.title.is_empty() {
            fallback.page_title.clone()
        } else {
            content.title.clone()
        },
    }
}

#[cfg(test)]
pub(crate) fn parse_syntax_page_content(
    hbk_path: &Path,
    locale: &str,
    toc: &Toc,
    html_path: &str,
    raw_html: &str,
) -> PageContent {
    let toc_index = syntax_toc_index(toc);
    parse_syntax_page_content_with_index(hbk_path, locale, &toc_index, html_path, raw_html)
}

#[cfg(test)]
pub(crate) fn parse_syntax_page_content_with_index(
    hbk_path: &Path,
    locale: &str,
    toc_index: &SyntaxTocIndex,
    html_path: &str,
    raw_html: &str,
) -> PageContent {
    parse_syntax_page_content_with_index_owned(
        hbk_path,
        locale,
        toc_index,
        html_path,
        raw_html.to_string(),
    )
}

pub(crate) fn parse_syntax_page_content_with_index_owned(
    hbk_path: &Path,
    locale: &str,
    toc_index: &SyntaxTocIndex,
    html_path: &str,
    raw_html: String,
) -> PageContent {
    let normalized_page_path = html_path.trim_start_matches('/').to_string();
    let toc_page = toc_index.get(&normalized_page_path);
    let toc_path = toc_page.and_then(|page| page.toc_path.clone());
    let toc_title = toc_page.and_then(|page| page.toc_title.clone());
    let title = select_first_html_text(&raw_html, ".V8SH_pagetitle")
        .or_else(|| select_first_html_text(&raw_html, "title"))
        .or_else(|| toc_title.clone())
        .unwrap_or_default();
    let body_text = body_text(&raw_html);
    let text_preview = body_text.chars().take(240).collect();

    PageContent {
        source: PageSource {
            hbk_path: hbk_path.to_path_buf(),
            locale: locale.to_string(),
            toc_path,
            html_path: normalized_page_path,
            toc_title,
        },
        title,
        raw_html,
        body_text,
        text_preview,
        links: Vec::new(),
        diagnostics: Vec::new(),
    }
}

pub(crate) type SyntaxTocIndex = HashMap<String, SyntaxTocPage>;

#[derive(Debug, Clone)]
pub(crate) struct SyntaxTocPage {
    toc_path: Option<String>,
    toc_title: Option<String>,
}

pub(crate) fn syntax_toc_index(toc: &Toc) -> SyntaxTocIndex {
    toc.flat_pages()
        .map(|flat_page| {
            (
                flat_page.page.html_path.clone(),
                SyntaxTocPage {
                    toc_path: Some(flat_page.index_path.to_string()),
                    toc_title: Some(flat_page.page.title.display().to_string()),
                },
            )
        })
        .collect()
}

const SYNTAX_LABELS: &[&str] = &["Синтаксис:", "Syntax:"];
const PARAMETER_LABELS: &[&str] = &["Параметры:", "Parameters:"];
const VARIANT_LABELS: &[&str] = &["Вариант синтаксиса:", "Syntax variant:"];
const VARIANT_DESCRIPTION_LABELS: &[&str] = &[
    "Описание варианта метода:",
    "Description of method variant:",
];

pub(crate) fn parse_signatures(content: &PageContent) -> Vec<Signature> {
    let variant_signatures = parse_variant_signatures(&content.raw_html);
    if !variant_signatures.is_empty() {
        return variant_signatures;
    }

    let Some(section_html) = section_html(&content.raw_html, SYNTAX_LABELS) else {
        return Vec::new();
    };
    let parameters = parse_parameters(content);
    signatures_from_section(section_html, &parameters, None)
}

fn parse_variant_signatures(raw_html: &str) -> Vec<Signature> {
    variant_blocks(raw_html)
        .into_iter()
        .flat_map(|block| {
            let block_html = &raw_html[block.body_start..block.body_end];
            let Some(syntax_html) = section_html(block_html, SYNTAX_LABELS) else {
                return Vec::new();
            };
            let variant = SyntaxVariant {
                title: block.title,
                description: variant_description(block_html),
            };
            let parameters = parse_parameters_from_html(block_html);
            signatures_from_section(syntax_html, &parameters, Some(&variant))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VariantBlock {
    title: String,
    body_start: usize,
    body_end: usize,
}

fn variant_blocks(raw_html: &str) -> Vec<VariantBlock> {
    let labels = label_positions(raw_html, VARIANT_LABELS);
    labels
        .iter()
        .enumerate()
        .filter_map(|(index, (start, label))| {
            let (title, body_start) = variant_title_and_body_start(raw_html, *start, label)?;
            let body_end = labels
                .get(index + 1)
                .map(|(next_start, _)| *next_start)
                .unwrap_or(raw_html.len());
            Some(VariantBlock {
                title,
                body_start,
                body_end,
            })
        })
        .collect()
}

fn label_positions<'a>(raw_html: &str, labels: &'a [&'a str]) -> Vec<(usize, &'a str)> {
    let mut positions = labels
        .iter()
        .flat_map(|label| {
            raw_html
                .match_indices(label)
                .map(move |(index, _)| (index, *label))
        })
        .collect::<Vec<_>>();
    positions.sort_by_key(|(index, _)| *index);
    positions
}

fn variant_title_and_body_start(
    raw_html: &str,
    label_start: usize,
    label: &str,
) -> Option<(String, usize)> {
    let title_start = label_start + label.len();
    let chapter_end = raw_html[title_start..]
        .find("</p>")
        .map(|offset| title_start + offset)?;
    let title = text_lines_from_html_fragment(&raw_html[title_start..chapter_end])
        .trim()
        .to_string();
    Some((title, chapter_end + "</p>".len()))
}

fn variant_description(raw_html: &str) -> Option<String> {
    section_html(raw_html, VARIANT_DESCRIPTION_LABELS)
        .map(text_lines_from_html_fragment)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn signatures_from_section(
    section_html: &str,
    parameters: &[Parameter],
    variant: Option<&SyntaxVariant>,
) -> Vec<Signature> {
    text_lines_from_html_fragment(section_html)
        .split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| Signature {
            text: line.to_string(),
            parameters: parameters_for_signature(line, parameters),
            variant: variant.cloned(),
        })
        .collect()
}

fn parse_parameters(content: &PageContent) -> Vec<Parameter> {
    parse_parameters_from_html(&content.raw_html)
}

fn parse_parameters_from_html(raw_html: &str) -> Vec<Parameter> {
    let Some(section) = section_html(raw_html, PARAMETER_LABELS) else {
        return Vec::new();
    };
    let rubric_parameters = parse_rubric_parameters(section);
    if rubric_parameters.is_empty() {
        parse_parameters_from_text(&text_lines_from_html_fragment(section))
    } else {
        rubric_parameters
    }
}

fn parse_rubric_parameters(section_html: &str) -> Vec<Parameter> {
    let rubrics = rubric_ranges(section_html);
    rubrics
        .iter()
        .enumerate()
        .filter_map(|(index, rubric)| {
            let name = parameter_name_from_rubric(&rubric.title)?;
            let next_start = rubrics
                .get(index + 1)
                .map(|next| next.tag_start)
                .unwrap_or(section_html.len());
            let body = text_lines_from_html_fragment(&section_html[rubric.body_start..next_start]);
            let lower = rubric.title.to_lowercase();
            Some(Parameter {
                name,
                required: !(lower.contains("необязательный") || lower.contains("optional")),
                type_refs: parse_type_refs(&body),
                description: description_after_type(&body),
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RubricRange {
    tag_start: usize,
    body_start: usize,
    title: String,
}

fn rubric_ranges(section_html: &str) -> Vec<RubricRange> {
    let mut output = Vec::new();
    let mut offset = 0;
    while let Some(class_start) = section_html[offset..]
        .find("class=\"V8SH_rubric\"")
        .map(|index| offset + index)
    {
        let Some(tag_start) = section_html[..class_start].rfind('<') else {
            break;
        };
        let Some(content_start) = section_html[class_start..]
            .find('>')
            .map(|index| class_start + index + 1)
        else {
            break;
        };
        let Some(content_end) = section_html[content_start..]
            .find("</div>")
            .map(|index| content_start + index)
        else {
            break;
        };
        let title = text_lines_from_html_fragment(&section_html[content_start..content_end])
            .trim()
            .to_string();
        output.push(RubricRange {
            tag_start,
            body_start: content_end + "</div>".len(),
            title,
        });
        offset = content_end + "</div>".len();
    }
    output
}

fn parameter_name_from_rubric(value: &str) -> Option<String> {
    bracketed_name_ranges(value)
        .into_iter()
        .next()
        .map(|(_, _, name)| name.trim().to_string())
        .or_else(|| {
            let name = value
                .split_once('(')
                .map(|(name, _)| name)
                .unwrap_or(value)
                .trim();
            (!name.is_empty()).then(|| name.to_string())
        })
}

fn parse_parameters_from_text(section: &str) -> Vec<Parameter> {
    let ranges = bracketed_name_ranges(&section);
    ranges
        .iter()
        .enumerate()
        .filter_map(|(index, (_start, end, name))| {
            if name.trim().is_empty() {
                return None;
            }
            let next_start = ranges
                .get(index + 1)
                .map(|(next_start, _, _)| *next_start)
                .unwrap_or(section.len());
            let parameter_text = &section[*end..next_start];
            let lower = parameter_text.to_lowercase();
            let required = !(lower.contains("необязательный") || lower.contains("optional"));
            let type_refs = parse_type_refs(parameter_text);
            let description = parameter_text
                .split_once('.')
                .map(|(_, tail)| tail.trim())
                .filter(|tail| !tail.is_empty())
                .map(ToOwned::to_owned);
            Some(Parameter {
                name: name.trim().to_string(),
                required,
                type_refs,
                description,
            })
        })
        .collect()
}

fn parameters_for_signature(signature: &str, parameters: &[Parameter]) -> Vec<Parameter> {
    parameters
        .iter()
        .filter(|parameter| signature_contains_parameter(signature, parameter.name.as_str()))
        .cloned()
        .collect()
}

fn signature_contains_parameter(signature: &str, parameter_name: &str) -> bool {
    let mut search_start = 0;
    while let Some(offset) = signature[search_start..].find('<') {
        let parameter_start = search_start + offset + 1;
        let after_start = &signature[parameter_start..];
        if after_start
            .strip_prefix(parameter_name)
            .is_some_and(|after_name| after_name.starts_with('>'))
        {
            return true;
        }
        search_start = parameter_start;
    }
    false
}

fn type_refs_from_section(content: &PageContent, labels: &[&str]) -> Vec<TypeRef> {
    section_text(content, labels)
        .map(|section| parse_type_refs(&section))
        .unwrap_or_default()
}

fn parse_type_refs(section: &str) -> Vec<TypeRef> {
    let Some(after_type) = [
        "Тип параметра:",
        "Parameter type:",
        "Type of parameter:",
        "Тип:",
        "Type:",
    ]
    .iter()
    .find_map(|label| section.split_once(label).map(|(_, after_type)| after_type)) else {
        return Vec::new();
    };
    let type_part = after_type
        .split_once('.')
        .map(|(head, _)| head)
        .unwrap_or(after_type);
    type_part
        .split([',', ';'])
        .map(|value| value.trim().trim_matches('.'))
        .filter(|value| !value.is_empty())
        .map(|value| TypeRef {
            name: value.to_string(),
        })
        .collect()
}

fn detail_body_after_heading(content: &PageContent) -> String {
    html_after_first_class(&content.raw_html, "V8SH_heading")
        .or_else(|| html_after_first_class(&content.raw_html, "V8SH_pagetitle"))
        .map(text_lines_from_html_fragment)
        .unwrap_or_else(|| content.body_text.clone())
}

fn first_chapter_body(content: &PageContent) -> Option<String> {
    html_after_first_class(&content.raw_html, "V8SH_chapter").map(text_lines_from_html_fragment)
}

fn html_after_first_class<'a>(raw_html: &'a str, class_name: &str) -> Option<&'a str> {
    let class_marker = format!("class=\"{class_name}\"");
    let start = raw_html.find(&class_marker)?;
    let content_start = raw_html[start..]
        .find('>')
        .map(|offset| start + offset + 1)?;
    let tag_name = raw_html[..start]
        .rfind('<')
        .map(|tag_start| {
            raw_html[tag_start + 1..start]
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .trim_start_matches('/')
        })
        .unwrap_or_default();
    let end_tag = format!("</{tag_name}>");
    let body_start = raw_html[content_start..]
        .find(&end_tag)
        .map(|offset| content_start + offset + end_tag.len())?;
    let body_end = raw_html[body_start..]
        .find("<HR")
        .map(|offset| body_start + offset)
        .unwrap_or(raw_html.len());
    Some(&raw_html[body_start..body_end])
}

fn description_after_type(text: &str) -> Option<String> {
    let description = [
        "Тип параметра:",
        "Parameter type:",
        "Type of parameter:",
        "Тип:",
        "Type:",
    ]
    .iter()
    .find_map(|label| text.split_once(label).map(|(_, after_type)| after_type))
    .and_then(|after_type| after_type.split_once('.').map(|(_, tail)| tail))
    .unwrap_or(text);
    clean_free_text(before_any_label(
        description,
        &[
            "Примечание:",
            "Note:",
            "Значение по умолчанию:",
            "Default value:",
        ],
    ))
}

fn table_parameter_description(text: &str) -> Option<String> {
    clean_free_text(before_any_label(
        description_after_type(text).as_deref().unwrap_or(text),
        &["Значение по умолчанию:", "Default value:"],
    ))
}

fn default_value(text: &str) -> Option<String> {
    let value = ["Значение по умолчанию:", "Default value:"]
        .iter()
        .find_map(|label| text.split_once(label).map(|(_, value)| value))?;
    clean_free_text(value).map(|value| value.trim_end_matches('.').to_string())
}

fn before_any_label<'a>(text: &'a str, labels: &[&str]) -> &'a str {
    labels
        .iter()
        .filter_map(|label| text.find(label))
        .min()
        .map(|index| &text[..index])
        .unwrap_or(text)
}

fn clean_free_text(text: &str) -> Option<String> {
    let text = before_any_label(text, &["Методическая информация", "Methodical information"]);
    let text = text.trim().trim_matches('.').trim();
    (!text.is_empty()).then(|| text.to_string())
}
