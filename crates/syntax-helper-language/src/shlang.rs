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
