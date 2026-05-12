#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteGenerationRequest {
    output_root: PathBuf,
    source: SiteSource,
}

impl SiteGenerationRequest {
    pub fn explicit_files(
        output_root: impl Into<PathBuf>,
        files: Vec<PathBuf>,
    ) -> Result<Self, SiteGenerationError> {
        if files.is_empty() {
            return Err(SiteGenerationError::EmptySourceList);
        }
        Ok(Self {
            output_root: output_root.into(),
            source: SiteSource::ExplicitFiles(files),
        })
    }

    pub fn source_directory(
        output_root: impl Into<PathBuf>,
        source_dir: impl Into<PathBuf>,
        include_file_names: Vec<String>,
    ) -> Self {
        Self {
            output_root: output_root.into(),
            source: SiteSource::Directory {
                source_dir: source_dir.into(),
                include_file_names,
            },
        }
    }

    pub fn output_root(&self) -> &Path {
        &self.output_root
    }

    pub fn source(&self) -> &SiteSource {
        &self.source
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteSource {
    ExplicitFiles(Vec<PathBuf>),
    Directory {
        source_dir: PathBuf,
        include_file_names: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedSiteFileKind {
    Manifest,
    TocRoot,
    TocSection,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteGenerationProgress<'a> {
    SourceBooksDiscovered {
        count: usize,
    },
    SourceBookLoading {
        current: usize,
        total: usize,
        path: &'a Path,
    },
    SourceBooksLoaded {
        count: usize,
    },
    SiteDataBuilt {
        locale_count: usize,
        toc_node_count: usize,
        page_count: usize,
    },
    ArtifactWriting {
        current: usize,
        total: usize,
        kind: GeneratedSiteFileKind,
        path: &'a Path,
    },
}
