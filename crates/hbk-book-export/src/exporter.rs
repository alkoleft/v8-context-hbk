#[derive(Debug)]
pub struct BookExporter<'a> {
    book: &'a HbkBook,
}

impl<'a> BookExporter<'a> {
    pub fn new(book: &'a HbkBook) -> Self {
        Self { book }
    }

    pub fn book(&self) -> &'a HbkBook {
        self.book
    }

    pub fn validate_request(request: &BookExportRequest) -> Result<(), BookExportError> {
        validate_output_root(request.output_root())?;
        validate_combination(request.format(), request.hierarchy())
    }

    pub fn export(&self, request: &BookExportRequest) -> Result<BookExportResult, BookExportError> {
        Self::validate_request(request)?;
        validate_source_path(request.source_path(), self.book.path())?;
        match (request.format(), request.hierarchy()) {
            (BookExportFormat::Raw, BookExportHierarchy::Raw) => self.export_raw_raw(request),
            (BookExportFormat::Markdown, BookExportHierarchy::Toc) => {
                self.export_markdown_toc(request)
            }
            (format, hierarchy) => {
                Err(BookExportError::UnsupportedCombination { format, hierarchy })
            }
        }
    }

    pub fn markdown_page(&self, html_path: &str) -> Result<BookMarkdownPage, BookExportError> {
        let normalized_html_path = normalize_storage_path(html_path);
        let toc_page = self
            .book
            .toc()
            .find_by_html_path(normalized_html_path)
            .ok_or_else(|| BookExportError::TocPageNotFound {
                html_path: normalized_html_path.to_string(),
            })?;
        let page = DocumentationReader::new(self.book).load_page(&toc_page.html_path)?;
        Ok(BookMarkdownPage::from_page_content(page))
    }

    pub fn markdown_toc_page(
        &self,
        html_path: &str,
        title: &str,
    ) -> Result<BookMarkdownPage, BookExportError> {
        let normalized_html_path = normalize_storage_path(html_path);
        let markdown = if is_heading_only_toc_path(normalized_html_path) {
            heading_only_markdown(title)
        } else {
            match DocumentationReader::new(self.book).load_page(normalized_html_path) {
                Ok(page) => page_content_to_markdown(&page),
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

    pub fn linked_markdown_toc_page(
        &self,
        html_path: &str,
        title: &str,
        current_output_path: &Path,
        link_targets: &HashMap<String, PathBuf>,
        source_book_ids: &HashSet<String>,
    ) -> Result<BookMarkdownPage, BookExportError> {
        let normalized_html_path = normalize_storage_path(html_path);
        let markdown = if is_heading_only_toc_path(normalized_html_path) {
            heading_only_markdown(title)
        } else {
            match DocumentationReader::new(self.book).load_page(normalized_html_path) {
                Ok(page) => page_content_to_linked_markdown(
                    &page,
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

    pub fn markdown_page_loader(&self) -> Result<BookMarkdownPageLoader<'a>, BookExportError> {
        let loader = DocumentationReader::new(self.book).page_loader()?;
        Ok(BookMarkdownPageLoader { loader })
    }

    fn export_raw_raw(
        &self,
        request: &BookExportRequest,
    ) -> Result<BookExportResult, BookExportError> {
        let mut reader = self.book.file_storage_reader()?;
        let plans = plan_raw_exports(request.output_root(), reader.file_paths()?)?;
        create_directory(request.output_root())?;

        let mut exported_files = Vec::with_capacity(plans.len());
        for plan in plans {
            let bytes = reader.read_file(&plan.entry_name)?;
            if let Some(parent) = plan.output_path.parent() {
                create_directory(parent)?;
            }
            fs::write(&plan.output_path, &bytes).map_err(|source| BookExportError::Io {
                path: plan.output_path.clone(),
                operation: BookExportIoOperation::WriteFile,
                source,
            })?;
            exported_files.push(BookExportedFile::new(plan.output_path, bytes.len() as u64));
        }

        Ok(BookExportResult::new(
            request.output_root().to_path_buf(),
            exported_files,
        ))
    }

    fn export_markdown_toc(
        &self,
        request: &BookExportRequest,
    ) -> Result<BookExportResult, BookExportError> {
        let plans = plan_markdown_toc_exports(request.output_root(), self.book.toc());
        let link_targets = markdown_link_targets(&plans);
        let source_book_ids = source_book_link_ids(self.book);
        create_directory(request.output_root())?;

        let mut loader = DocumentationReader::new(self.book).page_loader()?;
        let mut exported_files = Vec::with_capacity(plans.len());
        for plan in plans {
            let markdown = if is_heading_only_toc_path(&plan.html_path) {
                heading_only_markdown(&plan.title)
            } else {
                match loader.load_page(&plan.html_path) {
                    Ok(page) => page_content_to_linked_markdown(
                        &page,
                        &plan.relative_path,
                        &link_targets,
                        &source_book_ids,
                    ),
                    Err(error) if documentation_error_is_missing_page(&error) => {
                        heading_only_markdown(&plan.title)
                    }
                    Err(error) => return Err(error.into()),
                }
            };
            if let Some(parent) = plan.output_path.parent() {
                create_directory(parent)?;
            }
            fs::write(&plan.output_path, markdown.as_bytes()).map_err(|source| {
                BookExportError::Io {
                    path: plan.output_path.clone(),
                    operation: BookExportIoOperation::WriteFile,
                    source,
                }
            })?;
            exported_files.push(BookExportedFile::new(
                plan.output_path,
                markdown.len() as u64,
            ));
        }

        Ok(BookExportResult::new(
            request.output_root().to_path_buf(),
            exported_files,
        ))
    }
}
