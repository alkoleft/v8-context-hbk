use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand, ValueEnum};
use hbk_book::HbkBook;
use hbk_book::{Toc, TocPage};
use hbk_book_export::{
    BookExportFormat, BookExportHierarchy, BookExportRequest, BookExportResult, BookExporter,
};
use hbk_container::HbkContainer;
use hbk_doc_site::{
    DocSiteGenerator, SiteGenerationProgress, SiteGenerationRequest, SiteGenerationResult,
};
use hbk_syntax_export::JsonExporter;
use serde_json::{Value, json};
use syntax_helper_extract::{SyntaxHelperReader, SyntaxHelperStreamError};
#[cfg(test)]
use syntax_helper_search::build_index_from_builder;
use syntax_helper_search::{
    IndexMetadata, RelatedHit, SearchDocument, SearchDocumentKind, SearchHit, SearchIndex,
    SearchIndexBuilder, SearchMode, SearchTypeRef, SearchTypeRefTarget, TypeReferenceGap,
    TypeReferenceGapExample, TypeReferenceGapReport, TypeReferenceRoleReport,
    build_index_from_builder_with_report,
};

const DEFAULT_SEARCH_LIMIT: usize = 20;
const DEFAULT_RELATED_LIMIT: usize = 200;
const INTERACTIVE_PROGRESS_UPDATE_INTERVAL: Duration = Duration::from_millis(200);
