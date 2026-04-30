use std::collections::BTreeSet;
use std::path::Path;

use hbk_book::{HbkBook, Toc};
use hbk_docs::{DocumentationReader, PageContent};
use syntax_helper_model::*;

use crate::discovery::discover_roots_with_loader;
use crate::error::{SyntaxHelperError, SyntaxHelperStreamError, infallible_stream_error};
use crate::page_parser::{
    parse_constructor, parse_enum, parse_enum_value, parse_global_context, parse_global_method,
    parse_global_property, parse_platform_method, parse_platform_property, parse_platform_type,
    parse_syntax_page_content_with_index_owned, source_from_content, syntax_toc_index,
};

#[derive(Debug)]
pub struct SyntaxHelperReader<'a> {
    book: &'a HbkBook,
}

impl<'a> SyntaxHelperReader<'a> {
    pub fn new(book: &'a HbkBook) -> Self {
        Self { book }
    }

    pub fn discover_roots(&self) -> Result<RootDiscovery, SyntaxHelperError> {
        let mut documentation = DocumentationReader::new(self.book)
            .page_loader()
            .map_err(SyntaxHelperError::from)?;
        discover_roots_with_loader(
            self.book.path(),
            self.book.locale().source_code(),
            self.book.toc(),
            |html_path| {
                documentation
                    .load_page(html_path)
                    .map_err(SyntaxHelperError::from)
            },
        )
    }

    pub fn extract(&self) -> Result<PlatformContext, SyntaxHelperError> {
        let mut context = PlatformContext::default();
        self.extract_into(&mut context)
            .map_err(infallible_stream_error)?;
        Ok(context)
    }

    pub fn extract_into<S>(&self, sink: &mut S) -> Result<(), SyntaxHelperStreamError<S::Error>>
    where
        S: SyntaxHelperSink,
    {
        let toc_index = syntax_toc_index(self.book.toc());
        let mut file_storage = self
            .book
            .file_storage_reader()
            .map_err(SyntaxHelperError::from)?;
        let mut load_page = |html_path: &str| {
            let raw_html = file_storage.read_page(html_path)?;
            Ok(parse_syntax_page_content_with_index_owned(
                self.book.path(),
                self.book.locale().source_code(),
                &toc_index,
                html_path,
                raw_html,
            ))
        };
        let discovery = discover_roots_with_loader(
            self.book.path(),
            self.book.locale().source_code(),
            self.book.toc(),
            &mut load_page,
        )?;
        let discovery = extraction_discovery(discovery);
        parse_extraction_pages_into(
            self.book.path(),
            self.book.locale().source_code(),
            self.book.toc(),
            discovery,
            load_page,
            sink,
        )
    }
}

pub fn extract_with_loader(
    hbk_path: &Path,
    locale: &str,
    toc: &Toc,
    mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
) -> Result<PlatformContext, SyntaxHelperError> {
    let mut context = PlatformContext::default();
    extract_with_loader_into(hbk_path, locale, toc, &mut load_page, &mut context)
        .map_err(infallible_stream_error)?;
    Ok(context)
}

pub fn extract_with_loader_into<S>(
    hbk_path: &Path,
    locale: &str,
    toc: &Toc,
    mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
    sink: &mut S,
) -> Result<(), SyntaxHelperStreamError<S::Error>>
where
    S: SyntaxHelperSink,
{
    let discovery = discover_roots_with_loader(hbk_path, locale, toc, &mut load_page)?;
    parse_extraction_pages_into(hbk_path, locale, toc, discovery, load_page, sink)
}

fn extraction_discovery(mut discovery: RootDiscovery) -> RootDiscovery {
    for root in &mut discovery.roots {
        root.pages
            .retain(|page| !matches!(page.class, PageClass::Catalog | PageClass::Unknown));
    }
    discovery
}

fn parse_extraction_pages_into<S>(
    _hbk_path: &Path,
    _locale: &str,
    _toc: &Toc,
    discovery: RootDiscovery,
    mut load_page: impl FnMut(&str) -> Result<PageContent, SyntaxHelperError>,
    sink: &mut S,
) -> Result<(), SyntaxHelperStreamError<S::Error>>
where
    S: SyntaxHelperSink,
{
    let mut visited = BTreeSet::new();
    let RootDiscovery { roots, diagnostics } = discovery;

    for diagnostic in diagnostics {
        sink.diagnostic(diagnostic)
            .map_err(SyntaxHelperStreamError::Sink)?;
    }

    for root in roots {
        let RootSection {
            kind,
            source: root_source,
            pages,
        } = root;

        for catalog_page in pages {
            if matches!(catalog_page.class, PageClass::Catalog | PageClass::Unknown) {
                continue;
            }
            if !visited.insert(catalog_page.source.html_path.clone()) {
                continue;
            }
            let content = load_page(&catalog_page.source.html_path)?;
            let source = source_from_content(&catalog_page.source, &content);
            match catalog_page.class {
                PageClass::Catalog | PageClass::Unknown => unreachable!(),
                PageClass::GlobalMethod => sink
                    .global_method(parse_global_method(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::GlobalProperty => sink
                    .global_property(parse_global_property(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::ObjectType => sink
                    .platform_type(parse_platform_type(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::ObjectMethod => sink
                    .type_method(parse_platform_method(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::ObjectProperty => sink
                    .type_property(parse_platform_property(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::Constructor => sink
                    .constructor(parse_constructor(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::Enum => sink
                    .enum_definition(parse_enum(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
                PageClass::EnumValue => sink
                    .enum_value(parse_enum_value(&content, source))
                    .map_err(SyntaxHelperStreamError::Sink)?,
            }
        }

        if kind == RootSectionKind::GlobalContext && visited.insert(root_source.html_path.clone()) {
            let content = load_page(&root_source.html_path)?;
            let source = source_from_content(&root_source, &content);
            sink.global_context(parse_global_context(&content, source))
                .map_err(SyntaxHelperStreamError::Sink)?;
        }
    }

    Ok(())
}
