#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteGenerationResult {
    output_root: PathBuf,
    files: Vec<GeneratedSiteFile>,
    locale_count: usize,
    book_count: usize,
    toc_node_count: usize,
    page_count: usize,
}

impl SiteGenerationResult {
    fn new(
        output_root: PathBuf,
        files: Vec<GeneratedSiteFile>,
        locale_count: usize,
        book_count: usize,
        toc_node_count: usize,
        page_count: usize,
    ) -> Self {
        Self {
            output_root,
            files,
            locale_count,
            book_count,
            toc_node_count,
            page_count,
        }
    }

    pub fn output_root(&self) -> &Path {
        &self.output_root
    }

    pub fn files(&self) -> &[GeneratedSiteFile] {
        &self.files
    }

    pub fn locale_count(&self) -> usize {
        self.locale_count
    }

    pub fn book_count(&self) -> usize {
        self.book_count
    }

    pub fn toc_node_count(&self) -> usize {
        self.toc_node_count
    }

    pub fn page_count(&self) -> usize {
        self.page_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedSiteFile {
    path: PathBuf,
    bytes_written: u64,
}

impl GeneratedSiteFile {
    fn new(path: PathBuf, bytes_written: u64) -> Self {
        Self {
            path,
            bytes_written,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}
