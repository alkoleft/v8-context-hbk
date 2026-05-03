pub(crate) fn has_token_prefix(label: &str, prefixes: &[&str]) -> bool {
    label
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| prefixes.iter().any(|prefix| token.starts_with(prefix)))
}
