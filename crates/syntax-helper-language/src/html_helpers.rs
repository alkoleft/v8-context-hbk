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
