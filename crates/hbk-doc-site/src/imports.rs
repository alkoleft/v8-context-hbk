use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::hash::Hasher;
use std::io;
use std::path::{Path, PathBuf};

use hbk_book::{BookError, HbkBook, TocPage, normalize_storage_path};
use hbk_book_export::{BookExportError, BookExporter, BookMarkdownPageLoader, MarkdownLinkTargets};
use serde::Serialize;

const SCHEMA_VERSION: u32 = 1;
const GENERATOR_NAME: &str = "hbk-doc-site";
