use scraper::{ElementRef, Html, Selector};
use serde::Serialize;
use syntax_helper_model::LocalizedName;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageSourceFamily {
    Shlang,
    Shquery,
    Dcsui,
}

impl LanguageSourceFamily {
    pub fn source_id(self) -> &'static str {
        match self {
            Self::Shlang => "shlang",
            Self::Shquery => "shquery",
            Self::Dcsui => "dcsui",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageDomain {
    BslLanguage,
    QueryLanguage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageFactFamily {
    Construct,
    Type,
    Function,
    Operator,
    Keyword,
    Literal,
}

impl LanguageFactFamily {
    pub fn document_kind(self) -> &'static str {
        match self {
            Self::Construct => "language_construct",
            Self::Type => "language_type",
            Self::Function => "language_function",
            Self::Operator => "language_operator",
            Self::Keyword => "language_keyword",
            Self::Literal => "language_literal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageFact {
    pub id: String,
    pub source_family: LanguageSourceFamily,
    pub domain: LanguageDomain,
    pub family: LanguageFactFamily,
    pub name: LocalizedName,
    pub syntax: Option<String>,
    pub signatures: Vec<LanguageSignature>,
    pub type_refs: Vec<String>,
    pub return_types: Vec<String>,
    pub description: Option<String>,
    pub provenance: LanguageFactProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageSignature {
    pub text: String,
    pub parameters: Vec<LanguageParameter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageParameter {
    pub name: String,
    pub required: bool,
    pub type_refs: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageFactProvenance {
    pub source_hbk: String,
    pub locale: String,
    pub html_path: String,
    pub page_title: String,
    pub anchor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguagePageInput<'a> {
    pub source_hbk: &'a str,
    pub source_family: LanguageSourceFamily,
    pub locale: &'a str,
    pub html_path: &'a str,
    pub html: &'a str,
}

pub fn extract_language_facts(input: LanguagePageInput<'_>) -> Vec<LanguageFact> {
    let document = Html::parse_document(input.html);
    let title = first_text(&document, "h1").unwrap_or_else(|| input.html_path.to_string());
    match input.source_family {
        LanguageSourceFamily::Shlang => extract_shlang(input, &document, &title),
        LanguageSourceFamily::Shquery => extract_shquery(input, &document, &title),
        LanguageSourceFamily::Dcsui => extract_dcsui(input, &document, &title),
    }
}

fn extract_shlang(input: LanguagePageInput<'_>, document: &Html, title: &str) -> Vec<LanguageFact> {
    let Some(page_key) = page_key(input.html_path) else {
        return Vec::new();
    };
    let (family, name) = match page_key {
        "def_String" => (LanguageFactFamily::Type, split_name_alias(title)),
        "def_Func" => (LanguageFactFamily::Construct, split_name_alias(title)),
        _ => return Vec::new(),
    };
    let syntax = (page_key == "def_Func")
        .then(|| first_labeled_paragraph(document, &["Синтаксис", "Syntax"]))
        .flatten();
    vec![language_fact(
        input,
        LanguageDomain::BslLanguage,
        family,
        title,
        page_key,
        None,
        name,
        syntax,
        Vec::new(),
        Vec::new(),
        first_description(document),
    )]
}

fn extract_shquery(
    input: LanguagePageInput<'_>,
    document: &Html,
    title: &str,
) -> Vec<LanguageFact> {
    let Some(page_key) = page_key(input.html_path) else {
        return Vec::new();
    };
    let body = normalized_text(document.root_element());
    match page_key {
        "SELECTStatement" => vec![language_fact(
            input,
            LanguageDomain::QueryLanguage,
            LanguageFactFamily::Construct,
            title,
            page_key,
            None,
            LocalizedName {
                primary: title.to_string(),
                alias: Some("SELECT".to_string()),
            },
            first_block_text(document),
            Vec::new(),
            Vec::new(),
            first_plain_paragraph(document),
        )],
        "SUM" => vec![query_function_fact(input, document, title, page_key)],
        "STRING" => vec![query_function_fact(input, document, title, page_key)],
        "LitString" => vec![language_fact(
            input,
            LanguageDomain::QueryLanguage,
            LanguageFactFamily::Literal,
            title,
            page_key,
            None,
            query_literal_name(title),
            first_block_text(document),
            Vec::new(),
            Vec::new(),
            first_plain_paragraph(document),
        )],
        _ if body.contains("ВЫБРАТЬ") || body.contains("SELECT") => Vec::new(),
        _ => Vec::new(),
    }
}

fn extract_dcsui(input: LanguagePageInput<'_>, document: &Html, title: &str) -> Vec<LanguageFact> {
    let Some(page_key) = page_key(input.html_path) else {
        return Vec::new();
    };
    match page_key {
        "SKD_Functions_Strings" => extract_dcsui_string_functions(input, document, title),
        "SKD_ExtQueryLangv" => extract_dcsui_query_extension(input, document, title),
        _ => Vec::new(),
    }
}

fn query_function_fact(
    input: LanguagePageInput<'_>,
    document: &Html,
    title: &str,
    page_key: &str,
) -> LanguageFact {
    let syntax = first_pre_text(document);
    let parameters = first_strong_parameter(document)
        .into_iter()
        .collect::<Vec<_>>();
    let return_types =
        return_type_after_label(document, &["Возвращаемое значение", "Return value"])
            .into_iter()
            .collect::<Vec<_>>();
    let signatures = syntax
        .clone()
        .map(|text| LanguageSignature { text, parameters })
        .into_iter()
        .collect::<Vec<_>>();
    language_fact(
        input,
        LanguageDomain::QueryLanguage,
        LanguageFactFamily::Function,
        title,
        page_key,
        None,
        query_function_name(title, page_key, &syntax),
        syntax,
        Vec::new(),
        return_types,
        first_plain_paragraph(document),
    )
    .with_signatures(signatures)
}

fn extract_dcsui_string_functions(
    input: LanguagePageInput<'_>,
    document: &Html,
    title: &str,
) -> Vec<LanguageFact> {
    let body = normalized_text(document.root_element());
    let Some(start) = body
        .find("ДлинаСтроки")
        .or_else(|| body.find("StringLength"))
    else {
        return Vec::new();
    };
    let section = &body[start..];
    let syntax = section
        .find("Синтаксис:")
        .map(|index| section[index + "Синтаксис:".len()..].trim())
        .and_then(|tail| tail.split("Параметр").next())
        .or_else(|| {
            section
                .find("Syntax:")
                .map(|index| section[index + "Syntax:".len()..].trim())
                .and_then(|tail| tail.split("Parameter").next())
        })
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty());
    let parameters = vec![LanguageParameter {
        name: if input.locale == "ru" {
            "Строка"
        } else {
            "String"
        }
        .to_string(),
        required: true,
        type_refs: vec![
            if input.locale == "ru" {
                "Строка"
            } else {
                "String"
            }
            .to_string(),
        ],
        description: None,
    }];
    let name = if input.locale == "ru" {
        LocalizedName {
            primary: "ДлинаСтроки".to_string(),
            alias: Some("StringLength".to_string()),
        }
    } else {
        LocalizedName {
            primary: "StringLength".to_string(),
            alias: None,
        }
    };
    vec![
        language_fact(
            input,
            LanguageDomain::QueryLanguage,
            LanguageFactFamily::Function,
            title,
            "SKD_Functions_Strings",
            Some("StringLength"),
            name,
            syntax.clone(),
            Vec::new(),
            Vec::new(),
            Some(title.to_string()),
        )
        .with_signatures(
            syntax
                .map(|text| LanguageSignature { text, parameters })
                .into_iter()
                .collect(),
        ),
    ]
}

fn extract_dcsui_query_extension(
    input: LanguagePageInput<'_>,
    document: &Html,
    title: &str,
) -> Vec<LanguageFact> {
    let body = normalized_text(document.root_element());
    let mut facts = vec![language_fact(
        input,
        LanguageDomain::QueryLanguage,
        LanguageFactFamily::Construct,
        title,
        "SKD_ExtQueryLangv",
        None,
        LocalizedName {
            primary: title.to_string(),
            alias: None,
        },
        None,
        Vec::new(),
        Vec::new(),
        first_plain_paragraph(document),
    )];
    for keyword in ["ВЫБРАТЬ", "ГДЕ"] {
        if body.contains(keyword) {
            facts.push(language_fact(
                input,
                LanguageDomain::QueryLanguage,
                LanguageFactFamily::Keyword,
                title,
                "SKD_ExtQueryLangv",
                Some(keyword),
                LocalizedName {
                    primary: format!("{{{keyword}}}"),
                    alias: None,
                },
                None,
                Vec::new(),
                Vec::new(),
                None,
            ));
        }
    }
    for (keyword, anchor) in [("SELECT", "SELECT"), ("WHERE", "WHERE")] {
        if body.contains(keyword) {
            facts.push(language_fact(
                input,
                LanguageDomain::QueryLanguage,
                LanguageFactFamily::Keyword,
                title,
                "SKD_ExtQueryLangv",
                Some(anchor),
                LocalizedName {
                    primary: format!("{{{keyword}}}"),
                    alias: None,
                },
                None,
                Vec::new(),
                Vec::new(),
                None,
            ));
        }
    }
    facts
}

#[allow(clippy::too_many_arguments)]
fn language_fact(
    input: LanguagePageInput<'_>,
    domain: LanguageDomain,
    family: LanguageFactFamily,
    page_title: &str,
    page_key: &str,
    anchor: Option<&str>,
    name: LocalizedName,
    syntax: Option<String>,
    type_refs: Vec<String>,
    return_types: Vec<String>,
    description: Option<String>,
) -> LanguageFact {
    let local_id = anchor
        .map(|anchor| format!("{page_key}#{anchor}"))
        .unwrap_or_else(|| page_key.to_string());
    LanguageFact {
        id: format!("{}:{local_id}", input.source_family.source_id()),
        source_family: input.source_family,
        domain,
        family,
        name,
        syntax,
        signatures: Vec::new(),
        type_refs,
        return_types,
        description,
        provenance: LanguageFactProvenance {
            source_hbk: input.source_hbk.to_string(),
            locale: input.locale.to_string(),
            html_path: input.html_path.to_string(),
            page_title: page_title.to_string(),
            anchor: anchor.map(ToOwned::to_owned),
        },
    }
}

trait WithSignatures {
    fn with_signatures(self, signatures: Vec<LanguageSignature>) -> Self;
}

impl WithSignatures for LanguageFact {
    fn with_signatures(mut self, signatures: Vec<LanguageSignature>) -> Self {
        self.signatures = signatures;
        self
    }
}

fn page_key(path: &str) -> Option<&str> {
    path.rsplit('/').next().map(|value| {
        value
            .strip_suffix(".html")
            .or_else(|| value.strip_suffix(".htm"))
            .unwrap_or(value)
    })
}

fn split_name_alias(title: &str) -> LocalizedName {
    let title = title.trim();
    if let Some(open) = title.rfind('(')
        && title.ends_with(')')
        && open > 0
    {
        return LocalizedName {
            primary: title[..open].trim().to_string(),
            alias: Some(title[open + 1..title.len() - 1].trim().to_string()),
        };
    }
    LocalizedName {
        primary: title.to_string(),
        alias: None,
    }
}

fn query_function_name(title: &str, page_key: &str, syntax: &Option<String>) -> LocalizedName {
    if page_key == "SUM" {
        return if title.contains("СУММА") {
            LocalizedName {
                primary: "СУММА".to_string(),
                alias: Some("SUM".to_string()),
            }
        } else {
            LocalizedName {
                primary: "SUM".to_string(),
                alias: None,
            }
        };
    }
    if let Some(syntax) = syntax
        && let Some(name) = syntax.split(['(', ' ']).next()
        && !name.trim().is_empty()
    {
        return LocalizedName {
            primary: name.trim().to_string(),
            alias: split_name_alias(title).alias,
        };
    }
    split_name_alias(title)
}

fn query_literal_name(title: &str) -> LocalizedName {
    for prefix in ["Литерал типа ", "Literal of type "] {
        if let Some(value) = title.strip_prefix(prefix) {
            let value = value.trim();
            if !value.is_empty() {
                return LocalizedName {
                    primary: if value == "СТРОКА" {
                        "Строка".to_string()
                    } else {
                        value.to_string()
                    },
                    alias: (value == "СТРОКА").then(|| "STRING".to_string()),
                };
            }
        }
    }
    split_name_alias(title)
}

fn first_text(document: &Html, selector: &str) -> Option<String> {
    let selector = Selector::parse(selector).expect("static selector must be valid");
    document
        .select(&selector)
        .next()
        .map(normalized_text)
        .filter(|text| !text.is_empty())
}

fn first_pre_text(document: &Html) -> Option<String> {
    first_text(document, "pre")
}

fn first_block_text(document: &Html) -> Option<String> {
    first_text(document, "blockquote")
}

fn first_plain_paragraph(document: &Html) -> Option<String> {
    let selector = Selector::parse("p").expect("static selector must be valid");
    document
        .select(&selector)
        .map(normalized_text)
        .find(|text| !text.is_empty() && !is_labeled_text(text))
}

fn first_description(document: &Html) -> Option<String> {
    let selector = Selector::parse("p").expect("static selector must be valid");
    document
        .select(&selector)
        .map(normalized_text)
        .find_map(|text| {
            for label in ["Описание:", "Description"] {
                if let Some((_, value)) = text.split_once(label) {
                    let value = value.trim();
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
            None
        })
        .or_else(|| first_plain_paragraph(document))
}

fn first_labeled_paragraph(document: &Html, labels: &[&str]) -> Option<String> {
    let selector = Selector::parse("p").expect("static selector must be valid");
    document
        .select(&selector)
        .map(normalized_text)
        .find_map(|text| {
            labels.iter().find_map(|label| {
                text.split_once(label).and_then(|(_, value)| {
                    let value = value.trim().trim_start_matches(':').trim();
                    (!value.is_empty()).then_some(value.to_string())
                })
            })
        })
}

fn first_strong_parameter(document: &Html) -> Option<LanguageParameter> {
    let selector = Selector::parse("blockquote p").expect("static selector must be valid");
    document.select(&selector).find_map(|element| {
        let text = normalized_text(element);
        let (name, description) = text
            .split_once(" - ")
            .or_else(|| text.split_once(". "))
            .unwrap_or((text.as_str(), ""));
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        Some(LanguageParameter {
            name: name.to_string(),
            required: true,
            type_refs: type_refs_from_text(description),
            description: (!description.trim().is_empty()).then(|| description.trim().to_string()),
        })
    })
}

fn return_type_after_label(document: &Html, labels: &[&str]) -> Option<String> {
    let selector = Selector::parse("p").expect("static selector must be valid");
    document
        .select(&selector)
        .map(normalized_text)
        .find_map(|text| {
            labels.iter().find_map(|label| {
                text.split_once(label)
                    .and_then(|(_, value)| type_refs_from_text(value).into_iter().next())
            })
        })
}

fn type_refs_from_text(text: &str) -> Vec<String> {
    let candidates = [
        "Строка",
        "String",
        "Число",
        "Number",
        "Булево",
        "Boolean",
        "Дата",
        "Date",
        "NULL",
        "Неопределено",
        "Undefined",
    ];
    candidates
        .into_iter()
        .filter(|candidate| text.contains(candidate))
        .map(ToOwned::to_owned)
        .collect()
}

fn is_labeled_text(text: &str) -> bool {
    ["Синтаксис", "Syntax", "Параметр", "Parameters"]
        .iter()
        .any(|label| text.starts_with(label))
}

fn normalized_text(element: ElementRef<'_>) -> String {
    let mut result = String::new();
    for part in element.text() {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(part);
    }
    result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('\u{a0}', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "../../tests/fixtures/syntax-helper-language";

    #[test]
    fn extracts_bsl_string_type_and_function_construct() {
        let string = fixture_fact(
            LanguageSourceFamily::Shlang,
            "ru",
            "def_String",
            "shlang_def_string_ru.html",
        );
        assert_eq!(string.id, "shlang:def_String");
        assert_eq!(string.domain, LanguageDomain::BslLanguage);
        assert_eq!(string.family, LanguageFactFamily::Type);
        assert_eq!(string.name.primary, "Строка");
        assert_eq!(string.name.alias.as_deref(), Some("String"));

        let func = fixture_fact(
            LanguageSourceFamily::Shlang,
            "ru",
            "def_Func",
            "shlang_def_func_ru.html",
        );
        assert_eq!(func.id, "shlang:def_Func");
        assert_eq!(func.family, LanguageFactFamily::Construct);
        assert!(
            func.syntax
                .as_deref()
                .is_some_and(|text| text.contains("Функция"))
        );
    }

    #[test]
    fn extracts_query_construct_function_and_literal() {
        let select = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "SELECTStatement",
            "shquery_select_statement_ru.html",
        );
        assert_eq!(select.domain, LanguageDomain::QueryLanguage);
        assert_eq!(select.family, LanguageFactFamily::Construct);
        assert!(
            select
                .syntax
                .as_deref()
                .is_some_and(|text| text.contains("ВЫБРАТЬ"))
        );

        let string = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "STRING",
            "shquery_string_ru.html",
        );
        assert_eq!(string.id, "shquery:STRING");
        assert_eq!(string.family, LanguageFactFamily::Function);
        assert_eq!(string.name.primary, "СТРОКА");
        assert!(string.return_types.iter().any(|value| value == "Строка"));

        let sum = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "SUM",
            "shquery_sum_ru.html",
        );
        assert_eq!(sum.id, "shquery:SUM");
        assert_eq!(sum.family, LanguageFactFamily::Function);
        assert_eq!(sum.name.primary, "СУММА");
        assert_eq!(sum.name.alias.as_deref(), Some("SUM"));
        assert!(sum.syntax.is_none());

        let literal = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "LitString",
            "shquery_lit_string_ru.html",
        );
        assert_eq!(literal.id, "shquery:LitString");
        assert_eq!(literal.family, LanguageFactFamily::Literal);
    }

    #[test]
    fn extracts_root_source_language_fixtures_without_locale_identity_suffixes() {
        let bsl = fixture_fact(
            LanguageSourceFamily::Shlang,
            "root",
            "def_String",
            "shlang_def_string_root.html",
        );
        assert_eq!(bsl.id, "shlang:def_String");
        assert_eq!(bsl.name.primary, "String");
        assert_eq!(bsl.provenance.page_title, "String");

        let sum = fixture_fact(
            LanguageSourceFamily::Shquery,
            "root",
            "SUM",
            "shquery_sum_root.html",
        );
        assert_eq!(sum.id, "shquery:SUM");
        assert_eq!(sum.name.primary, "SUM");

        let string = fixture_fact(
            LanguageSourceFamily::Shquery,
            "root",
            "STRING",
            "shquery_string_root.html",
        );
        assert_eq!(string.id, "shquery:STRING");
        assert_eq!(string.name.primary, "STRING");
    }

    #[test]
    fn extracts_dcsui_string_function_and_query_extension_keywords() {
        let facts = fixture_facts(
            LanguageSourceFamily::Dcsui,
            "ru",
            "SKD_Functions_Strings",
            "dcsui_functions_strings_ru.html",
        );
        let string_length = facts
            .iter()
            .find(|fact| fact.id == "dcsui:SKD_Functions_Strings#StringLength")
            .expect("StringLength fact must be extracted");
        assert_eq!(string_length.domain, LanguageDomain::QueryLanguage);
        assert_eq!(string_length.family, LanguageFactFamily::Function);
        assert_eq!(string_length.name.primary, "ДлинаСтроки");

        let facts = fixture_facts(
            LanguageSourceFamily::Dcsui,
            "ru",
            "SKD_ExtQueryLangv",
            "dcsui_ext_query_lang_ru.html",
        );
        assert!(facts.iter().any(|fact| fact.name.primary == "{ВЫБРАТЬ}"));
        assert!(facts.iter().any(|fact| fact.name.primary == "{ГДЕ}"));
    }

    #[test]
    fn identity_keeps_same_display_names_separate() {
        let bsl = fixture_fact(
            LanguageSourceFamily::Shlang,
            "ru",
            "def_String",
            "shlang_def_string_ru.html",
        );
        let query = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "STRING",
            "shquery_string_ru.html",
        );
        let literal = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "LitString",
            "shquery_lit_string_ru.html",
        );
        assert_eq!(bsl.name.primary, "Строка");
        assert_ne!(bsl.id, query.id);
        assert_ne!(bsl.id, literal.id);
        assert_ne!(query.id, literal.id);
    }

    fn fixture_fact(
        source_family: LanguageSourceFamily,
        locale: &str,
        html_path: &str,
        fixture_name: &str,
    ) -> LanguageFact {
        let facts = fixture_facts(source_family, locale, html_path, fixture_name);
        facts
            .iter()
            .find(|fact| fact.id.ends_with(html_path))
            .or_else(|| facts.first())
            .cloned()
            .expect("fixture must produce at least one fact")
    }

    fn fixture_facts(
        source_family: LanguageSourceFamily,
        locale: &str,
        html_path: &str,
        fixture_name: &str,
    ) -> Vec<LanguageFact> {
        let html = std::fs::read_to_string(format!("{BASE}/{fixture_name}"))
            .expect("fixture must be readable");
        extract_language_facts(LanguagePageInput {
            source_hbk: "fixture.hbk",
            source_family,
            locale,
            html_path,
            html: &html,
        })
    }
}
