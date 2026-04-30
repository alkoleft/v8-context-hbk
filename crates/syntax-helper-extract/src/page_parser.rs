use std::collections::HashMap;
use std::path::Path;

use hbk_book::Toc;
use hbk_docs::{PageContent, PageSource};
use syntax_helper_model::*;

use crate::html::{
    body_text, bracketed_name_ranges, heading_name, links_in_section, page_title_name,
    section_html, section_text, select_first_html_text, text_lines_from_html_fragment, title_name,
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
        source,
    }
}

pub fn parse_global_property(content: &PageContent, source: SyntaxHelperSource) -> GlobalProperty {
    GlobalProperty {
        name: heading_name(content),
        usage: section_text(content, &["Использование:", "Use:"]),
        type_refs: type_refs_from_section(content, &["Описание:", "Description:"]),
        description: section_text(content, &["Описание:", "Description:"]),
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
        source,
    }
}

pub fn parse_platform_method(content: &PageContent, source: SyntaxHelperSource) -> PlatformMethod {
    PlatformMethod {
        owner: title_name(content),
        name: heading_name(content),
        signatures: parse_signatures(content),
        return_types: type_refs_from_section(
            content,
            &["Возвращаемое значение:", "Return value:", "Returned value:"],
        ),
        description: section_text(content, &["Описание:", "Description:"]),
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
        usage: section_text(content, &["Использование:", "Use:"]),
        type_refs: type_refs_from_section(content, &["Описание:", "Description:"]),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
}

pub fn parse_constructor(content: &PageContent, source: SyntaxHelperSource) -> Constructor {
    Constructor {
        owner: title_name(content),
        name: heading_name(content),
        signatures: parse_signatures(content),
        description: section_text(content, &["Описание:", "Description:"]),
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
        source,
    }
}

pub fn parse_enum_value(content: &PageContent, source: SyntaxHelperSource) -> EnumValue {
    EnumValue {
        owner: title_name(content),
        name: heading_name(content),
        description: section_text(content, &["Описание:", "Description:"]),
        source,
    }
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

pub(crate) fn parse_signatures(content: &PageContent) -> Vec<Signature> {
    let Some(section_html) = section_html(&content.raw_html, &["Синтаксис:", "Syntax:"])
    else {
        return Vec::new();
    };
    let parameters = parse_parameters(content);
    text_lines_from_html_fragment(section_html)
        .split('\n')
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| Signature {
            text: line.to_string(),
            parameters: parameters_for_signature(line, &parameters),
        })
        .collect()
}

fn parse_parameters(content: &PageContent) -> Vec<Parameter> {
    let Some(section) = section_text(content, &["Параметры:", "Parameters:"]) else {
        return Vec::new();
    };
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
    let Some(after_type) = ["Тип:", "Type:"]
        .iter()
        .find_map(|label| section.split_once(label).map(|(_, after_type)| after_type))
    else {
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
