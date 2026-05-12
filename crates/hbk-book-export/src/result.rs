#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookExportResult {
    output_root: PathBuf,
    files: Vec<BookExportedFile>,
}

impl BookExportResult {
    pub fn new(output_root: impl Into<PathBuf>, files: Vec<BookExportedFile>) -> Self {
        Self {
            output_root: output_root.into(),
            files,
        }
    }

    pub fn output_root(&self) -> &Path {
        &self.output_root
    }

    pub fn files(&self) -> &[BookExportedFile] {
        &self.files
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookExportedFile {
    path: PathBuf,
    bytes_written: u64,
}

impl BookExportedFile {
    pub fn new(path: impl Into<PathBuf>, bytes_written: u64) -> Self {
        Self {
            path: path.into(),
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
