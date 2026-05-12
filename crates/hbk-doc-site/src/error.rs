#[derive(Debug)]
pub enum SiteGenerationError {
    EmptySourceList,
    MissingSourceDirectory {
        source_dir: PathBuf,
    },
    SourceDirectoryNotDirectory {
        source_dir: PathBuf,
    },
    EmptyCorpus,
    UnsupportedLocale {
        path: PathBuf,
        locale: String,
    },
    Book {
        path: PathBuf,
        source: BookError,
    },
    Markdown {
        path: PathBuf,
        html_path: String,
        source: Box<BookExportError>,
    },
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
}

impl fmt::Display for SiteGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySourceList => f.write_str("documentation site source list is empty"),
            Self::MissingSourceDirectory { source_dir } => write!(
                f,
                "documentation site source directory '{}' does not exist",
                source_dir.display()
            ),
            Self::SourceDirectoryNotDirectory { source_dir } => write!(
                f,
                "documentation site source path '{}' is not a directory",
                source_dir.display()
            ),
            Self::EmptyCorpus => f.write_str("documentation site source corpus is empty"),
            Self::UnsupportedLocale { path, locale } => write!(
                f,
                "documentation site book '{}' uses unsupported locale code '{locale}'",
                path.display()
            ),
            Self::Book { path, source } => {
                write!(
                    f,
                    "failed to read documentation site book '{}': {source}",
                    path.display()
                )
            }
            Self::Markdown {
                path,
                html_path,
                source,
            } => write!(
                f,
                "failed to generate documentation site Markdown page '{}' from '{}': {source}",
                html_path,
                path.display()
            ),
            Self::Io { path, source } => {
                write!(f, "failed to write '{}': {source}", path.display())
            }
            Self::Json { path, source } => {
                write!(f, "failed to serialize '{}': {source}", path.display())
            }
        }
    }
}

impl std::error::Error for SiteGenerationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Book { source, .. } => Some(source),
            Self::Markdown { source, .. } => Some(source.as_ref()),
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::EmptySourceList
            | Self::MissingSourceDirectory { .. }
            | Self::SourceDirectoryNotDirectory { .. }
            | Self::EmptyCorpus
            | Self::UnsupportedLocale { .. } => None,
        }
    }
}
