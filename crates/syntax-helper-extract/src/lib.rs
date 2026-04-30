mod catalog;
mod discovery;
mod error;
mod html;
mod page_parser;
mod reader;

pub use discovery::discover_roots_with_loader;
pub use error::{SyntaxHelperError, SyntaxHelperStreamError};
pub use page_parser::{
    parse_constructor, parse_enum, parse_enum_value, parse_global_context, parse_global_method,
    parse_global_property, parse_platform_method, parse_platform_property, parse_platform_type,
};
pub use reader::{SyntaxHelperReader, extract_with_loader, extract_with_loader_into};
pub use syntax_helper_model::*;

#[cfg(test)]
pub(crate) use page_parser::{parse_signatures, parse_syntax_page_content};

#[cfg(test)]
mod syntax_helper_fixture_tests;
#[cfg(test)]
mod tests;
