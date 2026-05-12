fn query_function_fact(
    input: LanguagePageInput<'_>,
    document: &Html,
    title: &str,
    page_key: &str,
) -> LanguageFact {
    let syntax = first_pre_text(document);
    language_function_fact(
        input,
        title,
        page_key,
        None,
        CallableFactParts {
            name: query_function_name(title, page_key, &syntax),
            syntax,
            parameters: first_strong_parameter(document).into_iter().collect(),
            return_types: return_type_after_label(
                document,
                &["Возвращаемое значение", "Return value"],
            )
            .into_iter()
            .collect(),
            description: first_plain_paragraph(document),
        },
    )
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
    vec![language_function_fact(
        input,
        title,
        "SKD_Functions_Strings",
        Some("StringLength"),
        CallableFactParts {
            name,
            syntax,
            parameters,
            return_types: Vec::new(),
            description: Some(title.to_string()),
        },
    )]
}

struct CallableFactParts {
    name: LocalizedName,
    syntax: Option<String>,
    parameters: Vec<LanguageParameter>,
    return_types: Vec<String>,
    description: Option<String>,
}

fn language_function_fact(
    input: LanguagePageInput<'_>,
    page_title: &str,
    page_key: &str,
    anchor: Option<&str>,
    parts: CallableFactParts,
) -> LanguageFact {
    let signatures = parts
        .syntax
        .clone()
        .map(|text| LanguageSignature {
            text,
            parameters: parts.parameters,
        })
        .into_iter()
        .collect();
    let mut fact = language_fact(
        input,
        LanguageDomain::QueryLanguage,
        LanguageFactFamily::Function,
        page_title,
        page_key,
        anchor,
        parts.name,
        parts.syntax,
        Vec::new(),
        parts.return_types,
        parts.description,
    );
    fact.signatures = signatures;
    fact
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
