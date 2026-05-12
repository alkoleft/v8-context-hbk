pub struct DocSiteGenerator;

impl DocSiteGenerator {
    pub fn generate(
        request: &SiteGenerationRequest,
    ) -> Result<SiteGenerationResult, SiteGenerationError> {
        Self::generate_with_progress(request, |_| {})
    }

    pub fn generate_with_progress<F>(
        request: &SiteGenerationRequest,
        mut progress: F,
    ) -> Result<SiteGenerationResult, SiteGenerationError>
    where
        F: FnMut(SiteGenerationProgress<'_>),
    {
        let paths = discover_source_books(request.source())?;
        progress(SiteGenerationProgress::SourceBooksDiscovered { count: paths.len() });
        if paths.is_empty() {
            return Err(SiteGenerationError::EmptyCorpus);
        }
        let books = load_source_books(paths, &mut progress)?;
        progress(SiteGenerationProgress::SourceBooksLoaded { count: books.len() });
        let data_root = request.output_root().join("data");
        let site = build_site_data(&books);
        progress(SiteGenerationProgress::SiteDataBuilt {
            locale_count: site.locale_count,
            toc_node_count: site.toc_node_count,
            page_count: site.page_count,
        });
        write_site_data(
            request.output_root().to_path_buf(),
            &data_root,
            site,
            &books,
            &mut progress,
        )
    }
}
