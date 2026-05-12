#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookExportFormat {
    Raw,
    Markdown,
}

impl fmt::Display for BookExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raw => f.write_str("raw"),
            Self::Markdown => f.write_str("markdown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookExportHierarchy {
    Raw,
    Toc,
}

impl fmt::Display for BookExportHierarchy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raw => f.write_str("raw"),
            Self::Toc => f.write_str("toc"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookExportRequest {
    source_path: PathBuf,
    output_root: PathBuf,
    format: BookExportFormat,
    hierarchy: BookExportHierarchy,
}

impl BookExportRequest {
    pub fn new(
        source_path: impl Into<PathBuf>,
        output_root: impl Into<PathBuf>,
        format: BookExportFormat,
        hierarchy: BookExportHierarchy,
    ) -> Result<Self, BookExportError> {
        let output_root = output_root.into();
        validate_output_root(&output_root)?;
        validate_combination(format, hierarchy)?;
        Ok(Self {
            source_path: source_path.into(),
            output_root,
            format,
            hierarchy,
        })
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn output_root(&self) -> &Path {
        &self.output_root
    }

    pub fn format(&self) -> BookExportFormat {
        self.format
    }

    pub fn hierarchy(&self) -> BookExportHierarchy {
        self.hierarchy
    }
}
