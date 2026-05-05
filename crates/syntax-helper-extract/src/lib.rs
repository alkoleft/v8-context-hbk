mod catalog;
mod discovery;
mod error;
mod html;
mod label_match;
mod page_parser;
mod reader;

pub use error::{SyntaxHelperError, SyntaxHelperStreamError};
pub use reader::SyntaxHelperReader;

#[cfg(test)]
pub(crate) use page_parser::{parse_signatures, parse_syntax_page_content};

#[cfg(test)]
mod syntax_helper_fixture_tests;
#[cfg(test)]
mod tests;
