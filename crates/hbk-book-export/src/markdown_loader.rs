#[derive(Debug)]
pub struct BookMarkdownPageLoader<'a> {
    loader: DocumentationPageLoader<'a>,
}

impl BookMarkdownPageLoader<'_> {
    pub fn linked_markdown_toc_page(
        &mut self,
        html_path: &str,
        title: &str,
        current_output_path: &Path,
        link_targets: &impl MarkdownLinkTargets,
        source_book_ids: &HashSet<String>,
    ) -> Result<BookMarkdownPage, BookExportError> {
        let normalized_html_path = normalize_storage_path(html_path);
        let markdown = if is_heading_only_toc_path(normalized_html_path) {
            heading_only_markdown(title)
        } else {
            match self.loader.load_raw_page(normalized_html_path) {
                Ok(raw_html) => raw_page_to_linked_markdown(
                    &raw_html,
                    normalized_html_path,
                    title,
                    current_output_path,
                    link_targets,
                    source_book_ids,
                ),
                Err(error) if documentation_error_is_missing_page(&error) => {
                    heading_only_markdown(title)
                }
                Err(error) => return Err(error.into()),
            }
        };
        Ok(BookMarkdownPage {
            html_path: normalized_html_path.to_string(),
            title: title.to_string(),
            markdown,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookMarkdownPage {
    html_path: String,
    title: String,
    markdown: String,
}

impl BookMarkdownPage {
    pub fn html_path(&self) -> &str {
        &self.html_path
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    fn from_page_content(page: PageContent) -> Self {
        let markdown = page_content_to_markdown(&page);
        Self {
            html_path: page.source.html_path,
            title: page.title,
            markdown,
        }
    }
}
