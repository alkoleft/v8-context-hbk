pub fn normalize_storage_path(path: &str) -> &str {
    path.trim_start_matches('/')
}

pub fn normalize_storage_path_owned(path: &str) -> String {
    normalize_storage_path(path).to_string()
}

pub fn normalize_storage_path_segments(path: &str) -> Option<String> {
    let mut segments = Vec::new();
    for segment in normalize_storage_path(path).split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            value => segments.push(value),
        }
    }
    (!segments.is_empty()).then(|| segments.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_storage_paths() {
        assert_eq!(normalize_storage_path("/docs/page.html"), "docs/page.html");
        assert_eq!(normalize_storage_path("docs/page.html"), "docs/page.html");
        assert_eq!(
            normalize_storage_path_owned("/docs/page.html"),
            "docs/page.html"
        );
    }

    #[test]
    fn normalizes_storage_path_segments() {
        assert_eq!(
            normalize_storage_path_segments("/docs/./chapter/../page.html").as_deref(),
            Some("docs/page.html")
        );
        assert_eq!(normalize_storage_path_segments("../page.html"), None);
        assert_eq!(
            normalize_storage_path_segments("#local"),
            Some("#local".to_string())
        );
        assert_eq!(normalize_storage_path_segments("/"), None);
    }
}
