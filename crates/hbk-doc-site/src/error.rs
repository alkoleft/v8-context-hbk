#[derive(Debug, Error)]
pub enum SiteGenerationError {
    #[error("documentation site source list is empty")]
    EmptySourceList,
    #[error("documentation site source directory '{}' does not exist", source_dir.display())]
    MissingSourceDirectory {
        source_dir: PathBuf,
    },
    #[error("documentation site source path '{}' is not a directory", source_dir.display())]
    SourceDirectoryNotDirectory {
        source_dir: PathBuf,
    },
    #[error("documentation site source corpus is empty")]
    EmptyCorpus,
    #[error("documentation site book '{}' uses unsupported locale code '{locale}'", path.display())]
    UnsupportedLocale {
        path: PathBuf,
        locale: String,
    },
    #[error("failed to read documentation site book '{}': {source}", path.display())]
    Book {
        path: PathBuf,
        #[source]
        source: BookError,
    },
    #[error("failed to generate documentation site Markdown page '{html_path}' from '{}': {source}", path.display())]
    Markdown {
        path: PathBuf,
        html_path: String,
        #[source]
        source: Box<BookExportError>,
    },
    #[error("failed to write '{}': {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to serialize '{}': {source}", path.display())]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}
