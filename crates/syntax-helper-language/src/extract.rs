pub fn extract_language_facts(input: LanguagePageInput<'_>) -> Vec<LanguageFact> {
    let document = Html::parse_document(input.html);
    let title = first_text(&document, "h1").unwrap_or_else(|| input.html_path.to_string());
    match input.source_family {
        LanguageSourceFamily::Shlang => extract_shlang(input, &document, &title),
        LanguageSourceFamily::Shquery => extract_shquery(input, &document, &title),
        LanguageSourceFamily::Dcsui => extract_dcsui(input, &document, &title),
    }
}
