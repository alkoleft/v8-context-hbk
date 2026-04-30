use std::convert::Infallible;
use std::fmt;

use hbk_book::BookError;
use hbk_docs::DocumentationError;

#[derive(Debug)]
pub enum SyntaxHelperError {
    Book(BookError),
    Documentation(Box<DocumentationError>),
}

impl fmt::Display for SyntaxHelperError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Book(source) => write!(f, "{source}"),
            Self::Documentation(source) => write!(f, "{source}"),
        }
    }
}

impl std::error::Error for SyntaxHelperError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Book(source) => Some(source),
            Self::Documentation(source) => Some(source),
        }
    }
}

impl From<BookError> for SyntaxHelperError {
    fn from(value: BookError) -> Self {
        Self::Book(value)
    }
}

impl From<DocumentationError> for SyntaxHelperError {
    fn from(value: DocumentationError) -> Self {
        Self::Documentation(Box::new(value))
    }
}

#[derive(Debug)]
pub enum SyntaxHelperStreamError<SinkError> {
    Source(SyntaxHelperError),
    Sink(SinkError),
}

impl<SinkError> fmt::Display for SyntaxHelperStreamError<SinkError>
where
    SinkError: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(source) => write!(f, "{source}"),
            Self::Sink(source) => write!(f, "{source}"),
        }
    }
}

impl<SinkError> std::error::Error for SyntaxHelperStreamError<SinkError>
where
    SinkError: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(source) => Some(source),
            Self::Sink(source) => Some(source),
        }
    }
}

impl<SinkError> From<SyntaxHelperError> for SyntaxHelperStreamError<SinkError> {
    fn from(value: SyntaxHelperError) -> Self {
        Self::Source(value)
    }
}

pub(crate) fn infallible_stream_error(
    error: SyntaxHelperStreamError<Infallible>,
) -> SyntaxHelperError {
    match error {
        SyntaxHelperStreamError::Source(source) => source,
        SyntaxHelperStreamError::Sink(never) => match never {},
    }
}
