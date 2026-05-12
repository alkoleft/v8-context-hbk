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
