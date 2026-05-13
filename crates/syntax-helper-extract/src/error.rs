#[cfg(test)]
use std::convert::Infallible;

use hbk_book::BookError;
use hbk_docs::DocumentationError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyntaxHelperError {
    #[error("{0}")]
    Book(#[from] BookError),
    #[error("{0}")]
    Documentation(#[source] Box<DocumentationError>),
}

impl From<DocumentationError> for SyntaxHelperError {
    fn from(value: DocumentationError) -> Self {
        Self::Documentation(Box::new(value))
    }
}

#[derive(Debug, Error)]
pub enum SyntaxHelperStreamError<SinkError> {
    #[error("{0}")]
    Source(#[from] SyntaxHelperError),
    #[error("{0}")]
    Sink(#[source] SinkError),
}

#[cfg(test)]
pub(crate) fn infallible_stream_error(
    error: SyntaxHelperStreamError<Infallible>,
) -> SyntaxHelperError {
    match error {
        SyntaxHelperStreamError::Source(source) => source,
        SyntaxHelperStreamError::Sink(never) => match never {},
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn syntax_helper_documentation_error_preserves_source_chain() {
        let documentation = DocumentationError::PageRead {
            path: PathBuf::from("book.hbk"),
            html_path: "missing.html".to_string(),
            source: Box::new(BookError::MissingZipEntry {
                path: PathBuf::from("book.hbk"),
                entry_name: "missing.html".to_string(),
            }),
        };

        let error = SyntaxHelperError::from(documentation);

        assert!(error.source().is_some());
    }

    #[test]
    fn stream_sink_error_preserves_source_chain() {
        let error: SyntaxHelperStreamError<io::Error> =
            SyntaxHelperStreamError::Sink(io::Error::new(io::ErrorKind::Other, "sink"));

        assert!(error.source().is_some());
    }
}
