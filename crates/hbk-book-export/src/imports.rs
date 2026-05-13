use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use hbk_book::{
    BookError, HbkBook, Toc, TocPage, normalize_storage_path, normalize_storage_path_segments,
};
use hbk_docs::{DocumentationError, DocumentationPageLoader, DocumentationReader, PageContent};
use quick_html2md::{MarkdownOptions, html_to_markdown_with_options};
use scraper::node::Node;
use scraper::{ElementRef, Html, Selector};
use thiserror::Error;
