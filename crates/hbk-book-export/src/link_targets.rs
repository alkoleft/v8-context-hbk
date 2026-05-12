pub trait MarkdownLinkTargets {
    fn markdown_link_target(
        &self,
        normalized_path: &str,
        source_book_ids: &HashSet<String>,
    ) -> Option<&PathBuf>;
}

impl MarkdownLinkTargets for HashMap<String, PathBuf> {
    fn markdown_link_target(
        &self,
        normalized_path: &str,
        source_book_ids: &HashSet<String>,
    ) -> Option<&PathBuf> {
        self.get(normalized_path).or_else(|| {
            normalized_path
                .split_once('/')
                .filter(|(book_segment, _)| source_book_ids.contains(*book_segment))
                .and_then(|(_, path_without_book_segment)| self.get(path_without_book_segment))
        })
    }
}
