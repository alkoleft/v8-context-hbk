fn kind_priority(kind: SearchDocumentKind) -> i64 {
    kind.priority()
}

fn owner_member_key(owner: &str, member: &str) -> String {
    normalize_lookup_key(&format!("{owner}.{member}"))
}

fn normalize_lookup_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn exact_type_ref_lookup_key(value: &str) -> String {
    value.trim().chars().flat_map(char::to_lowercase).collect()
}

fn searchable_name(value: &str) -> String {
    let mut output = String::new();
    let mut previous_lowercase = false;
    for character in value.chars() {
        if character.is_alphanumeric() {
            if previous_lowercase && character.is_uppercase() {
                output.push(' ');
            }
            output.extend(character.to_lowercase());
            previous_lowercase = character.is_lowercase();
        } else {
            output.push(' ');
            previous_lowercase = false;
        }
    }
    output
}

fn searchable_text(value: &str) -> String {
    value
        .split_whitespace()
        .map(searchable_name)
        .collect::<Vec<_>>()
        .join(" ")
}

fn fts_query(value: &str) -> String {
    let mut tokens = searchable_text(value)
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if tokens.iter().any(|token| token == "скд") {
        tokens.extend(
            [
                "система",
                "компоновки",
                "данных",
                "компоновка",
                "data",
                "composition",
            ]
            .into_iter()
            .map(ToOwned::to_owned),
        );
    }
    tokens.sort();
    tokens.dedup();
    tokens
        .into_iter()
        .filter(|token| !token.is_empty())
        .map(|token| format!("\"{}\"*", token.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn fuzzy_score(query: &str, document: &SearchDocument) -> usize {
    let mut candidates = vec![normalize_lookup_key(&document.name.primary)];
    if let Some(alias) = &document.name.alias {
        candidates.push(normalize_lookup_key(alias));
    }
    if let Some(owner) = &document.owner {
        candidates.push(owner_member_key(&owner.primary, &document.name.primary));
        if let Some(alias) = &document.name.alias {
            candidates.push(owner_member_key(&owner.primary, alias));
        }
    }
    candidates
        .iter()
        .map(|candidate| levenshtein(query, candidate))
        .min()
        .unwrap_or(usize::MAX)
}

fn fuzzy_threshold(query: &str) -> usize {
    match query.chars().count() {
        0..=8 => 2,
        9..=20 => 3,
        length => (length / 5).max(4),
    }
}

fn keyword_order(query: &str, document: &SearchDocument) -> (usize, i64) {
    let tokens = searchable_text(query)
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let query_key = tokens.join(" ");
    let first = tokens.first().map(String::as_str).unwrap_or_default();
    let name = searchable_name(&document.name.primary);
    let alias = document
        .name
        .alias
        .as_deref()
        .map(searchable_name)
        .unwrap_or_default();
    let owner_member = document
        .owner
        .as_ref()
        .map(|owner| {
            searchable_name(&format!(
                "{}.{}",
                owner.display_name(),
                document.name.primary
            ))
        })
        .unwrap_or_default();
    if !query_key.is_empty() && (name == query_key || alias == query_key) {
        (0, kind_priority(document.kind))
    } else if !query_key.is_empty() && owner_member == query_key {
        (1, kind_priority(document.kind))
    } else if !first.is_empty() && name.starts_with(first) {
        (2, 0)
    } else if !first.is_empty() && alias.starts_with(first) {
        (3, 0)
    } else if tokens.iter().all(|token| name.contains(token)) {
        (4, 0)
    } else if tokens.iter().all(|token| alias.contains(token)) {
        (5, 0)
    } else {
        (10, 0)
    }
}
