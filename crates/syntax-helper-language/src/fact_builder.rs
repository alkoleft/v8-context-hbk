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
