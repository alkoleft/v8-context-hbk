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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteGenerationRequest {
    output_root: PathBuf,
    source: SiteSource,
}

impl SiteGenerationRequest {
    pub fn explicit_files(
        output_root: impl Into<PathBuf>,
        files: Vec<PathBuf>,
    ) -> Result<Self, SiteGenerationError> {
        if files.is_empty() {
            return Err(SiteGenerationError::EmptySourceList);
        }
        Ok(Self {
            output_root: output_root.into(),
            source: SiteSource::ExplicitFiles(files),
        })
    }

    pub fn source_directory(
        output_root: impl Into<PathBuf>,
        source_dir: impl Into<PathBuf>,
        include_file_names: Vec<String>,
    ) -> Self {
        Self {
            output_root: output_root.into(),
            source: SiteSource::Directory {
                source_dir: source_dir.into(),
                include_file_names,
            },
        }
    }

    pub fn output_root(&self) -> &Path {
        &self.output_root
    }

    pub fn source(&self) -> &SiteSource {
        &self.source
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SiteSource {
    ExplicitFiles(Vec<PathBuf>),
    Directory {
        source_dir: PathBuf,
        include_file_names: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedSiteFileKind {
    Manifest,
    TocRoot,
    TocSection,
    Page,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteGenerationProgress<'a> {
    SourceBooksDiscovered {
        count: usize,
    },
    SourceBookLoading {
        current: usize,
        total: usize,
        path: &'a Path,
    },
    SourceBooksLoaded {
        count: usize,
    },
    SiteDataBuilt {
        locale_count: usize,
        toc_node_count: usize,
        page_count: usize,
    },
    ArtifactWriting {
        current: usize,
        total: usize,
        kind: GeneratedSiteFileKind,
        path: &'a Path,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct SiteBookId(String);

impl SiteBookId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SiteBookId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct SitePageId(String);

impl SitePageId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SitePageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct SiteTocNodeId(String);

impl SiteTocNodeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SiteTocNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteGenerationResult {
    output_root: PathBuf,
    files: Vec<GeneratedSiteFile>,
    locale_count: usize,
    book_count: usize,
    toc_node_count: usize,
    page_count: usize,
}

impl SiteGenerationResult {
    fn new(
        output_root: PathBuf,
        files: Vec<GeneratedSiteFile>,
        locale_count: usize,
        book_count: usize,
        toc_node_count: usize,
        page_count: usize,
    ) -> Self {
        Self {
            output_root,
            files,
            locale_count,
            book_count,
            toc_node_count,
            page_count,
        }
    }

    pub fn output_root(&self) -> &Path {
        &self.output_root
    }

    pub fn files(&self) -> &[GeneratedSiteFile] {
        &self.files
    }

    pub fn locale_count(&self) -> usize {
        self.locale_count
    }

    pub fn book_count(&self) -> usize {
        self.book_count
    }

    pub fn toc_node_count(&self) -> usize {
        self.toc_node_count
    }

    pub fn page_count(&self) -> usize {
        self.page_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedSiteFile {
    path: PathBuf,
    bytes_written: u64,
}

impl GeneratedSiteFile {
    fn new(path: PathBuf, bytes_written: u64) -> Self {
        Self {
            path,
            bytes_written,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }
}

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

pub struct DocSiteGenerator;

impl DocSiteGenerator {
    pub fn generate(
        request: &SiteGenerationRequest,
    ) -> Result<SiteGenerationResult, SiteGenerationError> {
        Self::generate_with_progress(request, |_| {})
    }

    pub fn generate_with_progress<F>(
        request: &SiteGenerationRequest,
        mut progress: F,
    ) -> Result<SiteGenerationResult, SiteGenerationError>
    where
        F: FnMut(SiteGenerationProgress<'_>),
    {
        let paths = discover_source_books(request.source())?;
        progress(SiteGenerationProgress::SourceBooksDiscovered { count: paths.len() });
        if paths.is_empty() {
            return Err(SiteGenerationError::EmptyCorpus);
        }
        let books = load_source_books(paths, &mut progress)?;
        progress(SiteGenerationProgress::SourceBooksLoaded { count: books.len() });
        let data_root = request.output_root().join("data");
        let site = build_site_data(&books);
        progress(SiteGenerationProgress::SiteDataBuilt {
            locale_count: site.locale_count,
            toc_node_count: site.toc_node_count,
            page_count: site.page_count,
        });
        write_site_data(
            request.output_root().to_path_buf(),
            &data_root,
            site,
            &books,
            &mut progress,
        )
    }
}

pub fn discover_source_books(source: &SiteSource) -> Result<Vec<PathBuf>, SiteGenerationError> {
    match source {
        SiteSource::ExplicitFiles(files) => {
            if files.is_empty() {
                return Err(SiteGenerationError::EmptySourceList);
            }
            let mut files = files.clone();
            files.sort_by_key(|left| path_sort_key(left));
            Ok(files)
        }
        SiteSource::Directory {
            source_dir,
            include_file_names,
        } => {
            if !source_dir.exists() {
                return Err(SiteGenerationError::MissingSourceDirectory {
                    source_dir: source_dir.clone(),
                });
            }
            if !source_dir.is_dir() {
                return Err(SiteGenerationError::SourceDirectoryNotDirectory {
                    source_dir: source_dir.clone(),
                });
            }
            let include_filter: BTreeSet<&str> =
                include_file_names.iter().map(String::as_str).collect();
            let mut paths = Vec::new();
            for entry in fs::read_dir(source_dir).map_err(|source| SiteGenerationError::Io {
                path: source_dir.clone(),
                source,
            })? {
                let entry = entry.map_err(|source| SiteGenerationError::Io {
                    path: source_dir.clone(),
                    source,
                })?;
                let path = entry.path();
                if !path.is_file() || !has_hbk_extension(&path) {
                    continue;
                }
                let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                    continue;
                };
                if include_filter.is_empty() || include_filter.contains(file_name) {
                    paths.push(path);
                }
            }
            paths.sort_by_key(|left| path_sort_key(left));
            Ok(paths)
        }
    }
}

#[derive(Debug)]
struct SourceBook {
    id: SiteBookId,
    file_name: String,
    file_size_bytes: u64,
    locale: String,
    title: String,
    book: HbkBook,
}

fn load_source_books<F>(
    paths: Vec<PathBuf>,
    progress: &mut F,
) -> Result<Vec<SourceBook>, SiteGenerationError>
where
    F: FnMut(SiteGenerationProgress<'_>),
{
    let total = paths.len();
    let mut opened = Vec::with_capacity(paths.len());
    for (index, path) in paths.into_iter().enumerate() {
        progress(SiteGenerationProgress::SourceBookLoading {
            current: index + 1,
            total,
            path: &path,
        });
        let book = HbkBook::open(&path).map_err(|source| SiteGenerationError::Book {
            path: path.clone(),
            source,
        })?;
        opened.push(book);
    }
    opened.sort_by(|left, right| {
        (
            left.locale().export_code(),
            path_file_name(left.path()),
            left.path().display().to_string(),
        )
            .cmp(&(
                right.locale().export_code(),
                path_file_name(right.path()),
                right.path().display().to_string(),
            ))
    });

    let mut used_ids = BTreeSet::new();
    let mut books = Vec::with_capacity(opened.len());
    for book in opened {
        let locale = book.locale().export_code().to_string();
        validate_locale_code(book.path(), &locale)?;
        let file_size_bytes = fs::metadata(book.path())
            .map_err(|source| SiteGenerationError::Io {
                path: book.path().to_path_buf(),
                source,
            })?
            .len();
        let base = slugify(&path_file_stem(book.path()));
        let id = unique_id(base, &mut used_ids);
        let title = if !book.meta().description.is_empty() {
            book.meta().description.clone()
        } else {
            book.meta().book_name.clone()
        };
        books.push(SourceBook {
            id: SiteBookId::new(id),
            file_name: path_file_name(book.path()),
            file_size_bytes,
            locale,
            title,
            book,
        });
    }
    Ok(books)
}

#[derive(Debug)]
struct SiteData {
    manifest: SiteManifest,
    locales: Vec<LocaleSiteData>,
    locale_count: usize,
    book_count: usize,
    toc_node_count: usize,
    page_count: usize,
}

#[derive(Debug)]
struct LocaleSiteData {
    locale: String,
    nodes: Vec<SiteTocNode>,
    sections: Vec<SiteTocSection>,
    pages: Vec<SitePageArtifactPlan>,
}

#[derive(Debug, Serialize)]
struct SiteManifest {
    schema_version: u32,
    generator: &'static str,
    generator_version: &'static str,
    build_id: String,
    locales: Vec<String>,
    books: BTreeMap<String, Vec<ManifestBook>>,
    toc_roots: BTreeMap<String, String>,
    page_roots: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
struct ManifestBook {
    book_id: SiteBookId,
    file_name: String,
    title: String,
    locale: String,
    file_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SiteTocNode {
    id: SiteTocNodeId,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    book_id: Option<SiteBookId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_id: Option<SitePageId>,
    has_children: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    children_path: Option<String>,
}

impl SiteTocNode {
    pub fn id(&self) -> &SiteTocNodeId {
        &self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn book_id(&self) -> Option<&SiteBookId> {
        self.book_id.as_ref()
    }

    pub fn page_id(&self) -> Option<&SitePageId> {
        self.page_id.as_ref()
    }

    pub fn has_children(&self) -> bool {
        self.has_children
    }

    pub fn children_path(&self) -> Option<&str> {
        self.children_path.as_deref()
    }
}

#[derive(Debug, Serialize)]
struct TocRootArtifact {
    schema_version: u32,
    locale: String,
    nodes: Vec<SiteTocNode>,
}

#[derive(Debug, Clone, Serialize)]
struct SiteTocSection {
    id: SiteTocNodeId,
    locale: String,
    nodes: Vec<SiteTocNode>,
}

#[derive(Debug, Serialize)]
struct TocSectionArtifact {
    schema_version: u32,
    locale: String,
    parent_id: SiteTocNodeId,
    nodes: Vec<SiteTocNode>,
}

#[derive(Debug, Clone)]
struct SitePageArtifactPlan {
    book_id: SiteBookId,
    link_aliases: BTreeSet<(SiteBookId, String)>,
    page_id: SitePageId,
    title: String,
    html_path: String,
}

#[derive(Debug, Clone)]
struct ResolvedPageTarget {
    book_id: SiteBookId,
    page_key: String,
    title: String,
    html_path: String,
}

#[derive(Debug, Clone)]
enum PlaceholderTargetCandidate {
    One(ResolvedPageTarget),
    Ambiguous,
}

#[derive(Debug)]
struct TocNodeBuilder {
    title: String,
    id_seed: String,
    merge_key: Option<String>,
    book_id: Option<SiteBookId>,
    page_id: Option<SitePageId>,
    children: Vec<TocNodeBuilder>,
}

fn build_site_data(books: &[SourceBook]) -> SiteData {
    let mut locale_books: BTreeMap<String, Vec<&SourceBook>> = BTreeMap::new();
    for book in books {
        locale_books
            .entry(book.locale.clone())
            .or_default()
            .push(book);
    }

    let mut locales = Vec::new();
    let mut manifest_books = BTreeMap::new();
    let mut toc_roots = BTreeMap::new();
    let mut toc_node_count = 0;
    let mut page_count = 0;

    for (locale, books) in locale_books {
        let mut builders = Vec::new();
        let mut pages = Vec::new();
        let mut page_plan_indexes = HashMap::new();
        let resolved_placeholder_targets = collect_resolved_placeholder_targets(&books);
        let mut books_manifest = Vec::new();
        for book in books {
            books_manifest.push(ManifestBook {
                book_id: book.id.clone(),
                file_name: book.file_name.clone(),
                title: book.title.clone(),
                locale: locale.clone(),
                file_size_bytes: book.file_size_bytes,
            });
            append_toc_pages(
                &mut builders,
                &mut pages,
                &mut page_plan_indexes,
                &resolved_placeholder_targets,
                book,
                book.book.toc().pages(),
                &[],
                &[],
                &[],
            );
        }
        let mut sections = Vec::new();
        let nodes = finalize_nodes(
            &locale,
            builders,
            &mut sections,
            &mut toc_node_count,
            &mut page_count,
        );
        toc_roots.insert(locale.clone(), format!("locales/{locale}/toc-root.json"));
        manifest_books.insert(locale.clone(), books_manifest);
        locales.push(LocaleSiteData {
            locale,
            nodes,
            sections,
            pages,
        });
    }

    let manifest = SiteManifest {
        schema_version: SCHEMA_VERSION,
        generator: GENERATOR_NAME,
        generator_version: env!("CARGO_PKG_VERSION"),
        build_id: build_id(books),
        locales: locales.iter().map(|locale| locale.locale.clone()).collect(),
        books: manifest_books,
        toc_roots,
        page_roots: locales
            .iter()
            .map(|locale| {
                (
                    locale.locale.clone(),
                    format!("locales/{}/pages", locale.locale),
                )
            })
            .collect(),
    };

    SiteData {
        manifest,
        book_count: books.len(),
        locale_count: locales.len(),
        locales,
        toc_node_count,
        page_count,
    }
}

#[allow(clippy::too_many_arguments)]
fn append_toc_pages(
    output: &mut Vec<TocNodeBuilder>,
    page_plans: &mut Vec<SitePageArtifactPlan>,
    page_plan_indexes: &mut HashMap<String, usize>,
    resolved_placeholder_targets: &HashMap<String, ResolvedPageTarget>,
    book: &SourceBook,
    pages: &[TocPage],
    parent_toc_path: &[usize],
    parent_title_path: &[String],
    parent_label_path: &[String],
) {
    for (index, page) in pages.iter().enumerate() {
        let mut toc_path = parent_toc_path.to_vec();
        toc_path.push(index);
        let title = display_title(page);
        let page_bearing = !page.html_path.trim().is_empty();
        if page_bearing {
            let normalized_address = normalized_page_address(&page.html_path);
            let placeholder_target = if is_content_node_placeholder_path(&normalized_address) {
                resolved_placeholder_targets.get(&placeholder_branch_key(parent_label_path, &title))
            } else {
                None
            };
            let (owner_book_id, page_key, plan_title, plan_html_path) =
                if let Some(target) = placeholder_target {
                    (
                        target.book_id.clone(),
                        target.page_key.clone(),
                        target.title.clone(),
                        target.html_path.clone(),
                    )
                } else {
                    (
                        book.id.clone(),
                        normalized_address.clone(),
                        title.clone(),
                        page.html_path.clone(),
                    )
                };
            let merge_key = page_address_merge_key(&page_key);
            let page_plan_index = match page_plan_indexes.get(&merge_key).copied() {
                Some(index) => {
                    page_plans[index]
                        .link_aliases
                        .insert((book.id.clone(), page.html_path.clone()));
                    index
                }
                None => {
                    let page_id = page_id(book, &page_key);
                    let mut link_aliases =
                        BTreeSet::from([(owner_book_id.clone(), plan_html_path.clone())]);
                    link_aliases.insert((book.id.clone(), page.html_path.clone()));
                    let index = page_plans.len();
                    page_plans.push(SitePageArtifactPlan {
                        book_id: owner_book_id,
                        link_aliases,
                        page_id: page_id.clone(),
                        title: plan_title,
                        html_path: plan_html_path,
                    });
                    page_plan_indexes.insert(merge_key.clone(), index);
                    index
                }
            };
            let page_id = page_plans[page_plan_index].page_id.clone();
            let node_book_id = page_plans[page_plan_index].book_id.clone();
            let mut title_path = parent_title_path.to_vec();
            title_path.push(format!("page:{}", page_id.as_str()));
            let mut label_path = parent_label_path.to_vec();
            label_path.push(normalize_title_key(&title));
            let mut node = TocNodeBuilder {
                title,
                id_seed: format!("page|{}", page_id.as_str()),
                merge_key: Some(merge_key),
                book_id: Some(node_book_id),
                page_id: Some(page_id),
                children: Vec::new(),
            };
            append_toc_pages(
                &mut node.children,
                page_plans,
                page_plan_indexes,
                resolved_placeholder_targets,
                book,
                &page.children,
                &toc_path,
                &title_path,
                &label_path,
            );
            append_or_merge_node(output, node);
        } else {
            let merge_key = section_title_merge_key(&title);
            let id_seed = format!(
                "section|{}|{}",
                book.locale,
                section_seed(parent_title_path, &title)
            );
            let mut incoming = TocNodeBuilder {
                title: title.clone(),
                id_seed,
                merge_key: Some(merge_key.clone()),
                book_id: None,
                page_id: None,
                children: Vec::new(),
            };
            let mut title_path = parent_title_path.to_vec();
            title_path.push(normalize_title_key(&title));
            let mut label_path = parent_label_path.to_vec();
            label_path.push(normalize_title_key(&title));
            append_toc_pages(
                &mut incoming.children,
                page_plans,
                page_plan_indexes,
                resolved_placeholder_targets,
                book,
                &page.children,
                &toc_path,
                &title_path,
                &label_path,
            );
            append_or_merge_node(output, incoming);
        }
    }
}

fn collect_resolved_placeholder_targets(
    books: &[&SourceBook],
) -> HashMap<String, ResolvedPageTarget> {
    let mut candidates = HashMap::new();
    for book in books {
        collect_concrete_page_targets(&mut candidates, book, book.book.toc().pages(), &[]);
    }
    candidates
        .into_iter()
        .filter_map(|(key, candidate)| match candidate {
            PlaceholderTargetCandidate::One(target) => Some((key, target)),
            PlaceholderTargetCandidate::Ambiguous => None,
        })
        .collect()
}

fn collect_concrete_page_targets(
    candidates: &mut HashMap<String, PlaceholderTargetCandidate>,
    book: &SourceBook,
    pages: &[TocPage],
    parent_label_path: &[String],
) {
    for page in pages {
        let title = display_title(page);
        let normalized_address = normalized_page_address(&page.html_path);
        if !normalized_address.is_empty() && !is_content_node_placeholder_path(&normalized_address)
        {
            let key = placeholder_branch_key(parent_label_path, &title);
            let target = ResolvedPageTarget {
                book_id: book.id.clone(),
                page_key: normalized_address,
                title: title.clone(),
                html_path: page.html_path.clone(),
            };
            record_placeholder_target_candidate(candidates, key, target);
        }
        let mut label_path = parent_label_path.to_vec();
        label_path.push(normalize_title_key(&title));
        collect_concrete_page_targets(candidates, book, &page.children, &label_path);
    }
}

fn record_placeholder_target_candidate(
    candidates: &mut HashMap<String, PlaceholderTargetCandidate>,
    key: String,
    target: ResolvedPageTarget,
) {
    match candidates.entry(key) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(PlaceholderTargetCandidate::One(target));
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            let candidate = entry.get_mut();
            match candidate {
                PlaceholderTargetCandidate::One(existing) => {
                    if existing.page_key != target.page_key {
                        *candidate = PlaceholderTargetCandidate::Ambiguous;
                    }
                }
                PlaceholderTargetCandidate::Ambiguous => {}
            }
        }
    }
}

fn append_or_merge_node(output: &mut Vec<TocNodeBuilder>, node: TocNodeBuilder) {
    if let Some(merge_key) = node.merge_key.as_deref()
        && let Some(existing) = output
            .iter_mut()
            .find(|candidate| candidate.merge_key.as_deref() == Some(merge_key))
    {
        merge_children(&mut existing.children, node.children);
        return;
    }
    output.push(node);
}

fn merge_children(output: &mut Vec<TocNodeBuilder>, incoming: Vec<TocNodeBuilder>) {
    for node in incoming {
        append_or_merge_node(output, node);
    }
}

fn finalize_nodes(
    locale: &str,
    builders: Vec<TocNodeBuilder>,
    sections: &mut Vec<SiteTocSection>,
    toc_node_count: &mut usize,
    page_count: &mut usize,
) -> Vec<SiteTocNode> {
    let mut nodes = Vec::with_capacity(builders.len());
    for builder in builders {
        *toc_node_count += 1;
        if builder.page_id.is_some() {
            *page_count += 1;
        }
        let id = node_id(locale, &builder);
        let child_nodes = finalize_nodes(
            locale,
            builder.children,
            sections,
            toc_node_count,
            page_count,
        );
        let children_path = if child_nodes.is_empty() {
            None
        } else {
            let path = format!("toc-sections/{}.json", id.as_str());
            sections.push(SiteTocSection {
                id: id.clone(),
                locale: locale.to_string(),
                nodes: child_nodes,
            });
            Some(path)
        };
        nodes.push(SiteTocNode {
            id,
            title: builder.title,
            book_id: builder.book_id,
            page_id: builder.page_id,
            has_children: children_path.is_some(),
            children_path,
        });
    }
    nodes
}

fn write_site_data(
    output_root: PathBuf,
    data_root: &Path,
    site: SiteData,
    books: &[SourceBook],
    progress: &mut impl FnMut(SiteGenerationProgress<'_>),
) -> Result<SiteGenerationResult, SiteGenerationError> {
    let mut files = Vec::new();
    let total_files = generated_site_file_count(&site);
    let mut written_files = 0usize;
    create_directory(data_root)?;
    let manifest_path = data_root.join("manifest.json");
    written_files += 1;
    progress(SiteGenerationProgress::ArtifactWriting {
        current: written_files,
        total: total_files,
        kind: GeneratedSiteFileKind::Manifest,
        path: &manifest_path,
    });
    files.push(write_json(manifest_path, &site.manifest)?);
    for locale in site.locales {
        let locale_root = data_root.join("locales").join(&locale.locale);
        let sections_root = locale_root.join("toc-sections");
        let pages_root = locale_root.join("pages");
        create_directory(&sections_root)?;
        create_directory(&pages_root)?;
        let toc_root_path = locale_root.join("toc-root.json");
        written_files += 1;
        progress(SiteGenerationProgress::ArtifactWriting {
            current: written_files,
            total: total_files,
            kind: GeneratedSiteFileKind::TocRoot,
            path: &toc_root_path,
        });
        files.push(write_json(
            toc_root_path,
            &TocRootArtifact {
                schema_version: SCHEMA_VERSION,
                locale: locale.locale.clone(),
                nodes: locale.nodes,
            },
        )?);
        for section in locale.sections {
            let section_path = sections_root.join(format!("{}.json", section.id.as_str()));
            written_files += 1;
            progress(SiteGenerationProgress::ArtifactWriting {
                current: written_files,
                total: total_files,
                kind: GeneratedSiteFileKind::TocSection,
                path: &section_path,
            });
            files.push(write_json(
                section_path,
                &TocSectionArtifact {
                    schema_version: SCHEMA_VERSION,
                    locale: section.locale,
                    parent_id: section.id,
                    nodes: section.nodes,
                },
            )?);
        }
        let link_targets = locale_link_targets(&locale.pages, books);
        let mut page_loaders = BTreeMap::new();
        for page in &locale.pages {
            let page_path = pages_root.join(page_markdown_relative_path(page));
            written_files += 1;
            progress(SiteGenerationProgress::ArtifactWriting {
                current: written_files,
                total: total_files,
                kind: GeneratedSiteFileKind::Page,
                path: &page_path,
            });
            files.push(write_markdown_page(
                &page_path,
                page,
                &link_targets,
                books,
                &mut page_loaders,
            )?);
        }
    }
    Ok(SiteGenerationResult::new(
        output_root,
        files,
        site.locale_count,
        site.book_count,
        site.toc_node_count,
        site.page_count,
    ))
}

fn generated_site_file_count(site: &SiteData) -> usize {
    1 + site
        .locales
        .iter()
        .map(|locale| 1 + locale.sections.len() + locale.pages.len())
        .sum::<usize>()
}

fn write_markdown_page<'a>(
    output_path: &Path,
    page: &SitePageArtifactPlan,
    link_targets: &LocaleLinkTargets,
    books: &'a [SourceBook],
    page_loaders: &mut BTreeMap<SiteBookId, BookMarkdownPageLoader<'a>>,
) -> Result<GeneratedSiteFile, SiteGenerationError> {
    let book = books
        .iter()
        .find(|book| book.id == page.book_id)
        .expect("page plan must refer to a loaded source book");
    let current_output_path = page_markdown_relative_path(page);
    if !page_loaders.contains_key(&book.id) {
        let loader = BookExporter::new(&book.book)
            .markdown_page_loader()
            .map_err(|source| SiteGenerationError::Markdown {
                path: book.book.path().to_path_buf(),
                html_path: page.html_path.clone(),
                source: Box::new(source),
            })?;
        page_loaders.insert(book.id.clone(), loader);
    }
    let page_link_targets = PageLinkTargets {
        locale: link_targets,
        book_id: &page.book_id,
    };
    let source_book_ids = link_targets
        .book_source_ids
        .get(&page.book_id)
        .expect("page plan must refer to source book link ids");
    let markdown = page_loaders
        .get_mut(&book.id)
        .expect("page loader must exist for source book")
        .linked_markdown_toc_page(
            &page.html_path,
            &page.title,
            &current_output_path,
            &page_link_targets,
            source_book_ids,
        )
        .map_err(|source| SiteGenerationError::Markdown {
            path: book.book.path().to_path_buf(),
            html_path: page.html_path.clone(),
            source: Box::new(source),
        })?
        .markdown()
        .to_string();
    let markdown = collapse_current_page_fragment_links(markdown, &page.page_id);
    write_text(output_path.to_path_buf(), &markdown)
}

fn collapse_current_page_fragment_links(markdown: String, page_id: &SitePageId) -> String {
    let same_page_prefix = format!("]({}.md#", page_id.as_str());
    if !markdown.contains(&same_page_prefix) {
        return markdown;
    }
    markdown.replace(&same_page_prefix, "](#")
}

#[derive(Debug)]
struct LocaleLinkTargets {
    prefixed_targets: HashMap<String, PathBuf>,
    book_targets: BTreeMap<SiteBookId, HashMap<String, PathBuf>>,
    book_source_ids: BTreeMap<SiteBookId, HashSet<String>>,
}

fn locale_link_targets(
    locale_pages: &[SitePageArtifactPlan],
    books: &[SourceBook],
) -> LocaleLinkTargets {
    let book_source_ids = books
        .iter()
        .map(|book| (book.id.clone(), source_book_link_ids(book)))
        .collect::<BTreeMap<_, _>>();
    let mut prefixed_targets = HashMap::new();
    let mut book_targets: BTreeMap<SiteBookId, HashMap<String, PathBuf>> = BTreeMap::new();
    for page in locale_pages {
        let relative_path = page_markdown_relative_path(page);
        for (book_id, html_path) in &page.link_aliases {
            let normalized_html_path = normalize_storage_path(html_path).to_string();
            if normalized_html_path.is_empty() {
                continue;
            }
            book_targets
                .entry(book_id.clone())
                .or_default()
                .entry(normalized_html_path.clone())
                .or_insert_with(|| relative_path.clone());
            if let Some(source_ids) = book_source_ids.get(book_id) {
                for source_id in source_ids {
                    prefixed_targets
                        .entry(format!("{source_id}/{normalized_html_path}"))
                        .or_insert_with(|| relative_path.clone());
                }
            }
        }
    }
    LocaleLinkTargets {
        prefixed_targets,
        book_targets,
        book_source_ids,
    }
}

struct PageLinkTargets<'a> {
    locale: &'a LocaleLinkTargets,
    book_id: &'a SiteBookId,
}

impl MarkdownLinkTargets for PageLinkTargets<'_> {
    fn markdown_link_target(
        &self,
        normalized_path: &str,
        source_book_ids: &HashSet<String>,
    ) -> Option<&PathBuf> {
        self.locale
            .book_targets
            .get(self.book_id)
            .and_then(|targets| targets.get(normalized_path))
            .or_else(|| self.locale.prefixed_targets.get(normalized_path))
            .or_else(|| {
                normalized_path
                    .split_once('/')
                    .filter(|(book_segment, _)| source_book_ids.contains(*book_segment))
                    .and_then(|(_, path_without_book_segment)| {
                        self.locale
                            .book_targets
                            .get(self.book_id)
                            .and_then(|targets| targets.get(path_without_book_segment))
                    })
            })
    }
}

fn source_book_link_ids(book: &SourceBook) -> HashSet<String> {
    let mut ids = HashSet::new();
    ids.insert(book.id.as_str().to_string());
    if !book.book.meta().book_name.is_empty() {
        ids.insert(book.book.meta().book_name.clone());
    }
    let stem = path_file_stem(book.book.path());
    if !stem.is_empty() {
        ids.insert(stem.clone());
        if let Some((base, _)) = stem.rsplit_once('_') {
            ids.insert(base.to_string());
        }
    }
    ids
}

fn page_markdown_relative_path(page: &SitePageArtifactPlan) -> PathBuf {
    PathBuf::from(page_markdown_file_name(&page.page_id))
}

fn page_markdown_file_name(page_id: &SitePageId) -> String {
    format!("{}.md", page_id.as_str())
}

fn write_json(
    path: PathBuf,
    value: &impl Serialize,
) -> Result<GeneratedSiteFile, SiteGenerationError> {
    if let Some(parent) = path.parent() {
        create_directory(parent)?;
    }
    let bytes = serde_json::to_vec(value).map_err(|source| SiteGenerationError::Json {
        path: path.clone(),
        source,
    })?;
    let bytes_written = bytes.len() as u64;
    fs::write(&path, bytes).map_err(|source| SiteGenerationError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(GeneratedSiteFile::new(path, bytes_written))
}

fn write_text(path: PathBuf, text: &str) -> Result<GeneratedSiteFile, SiteGenerationError> {
    if let Some(parent) = path.parent() {
        create_directory(parent)?;
    }
    let bytes_written = text.len() as u64;
    fs::write(&path, text.as_bytes()).map_err(|source| SiteGenerationError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(GeneratedSiteFile::new(path, bytes_written))
}

fn create_directory(path: &Path) -> Result<(), SiteGenerationError> {
    fs::create_dir_all(path).map_err(|source| SiteGenerationError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn page_id(book: &SourceBook, page_key: &str) -> SitePageId {
    let hash = stable_hash_hex(&format!("{}|{}", book.locale, page_key));
    SitePageId::new(format!("page-{}-{hash}", book.locale))
}

fn node_id(locale: &str, builder: &TocNodeBuilder) -> SiteTocNodeId {
    let hash = stable_hash_hex(&builder.id_seed);
    SiteTocNodeId::new(format!("node-{locale}-{}-{hash}", slugify(&builder.title)))
}

fn section_seed(parent_title_path: &[String], title: &str) -> String {
    let mut path = parent_title_path.to_vec();
    path.push(normalize_title_key(title));
    path.join("/")
}

fn section_title_merge_key(title: &str) -> String {
    format!("section-title|{}", normalize_title_key(title))
}

fn page_address_merge_key(page_key: &str) -> String {
    format!("page-address|{page_key}")
}

fn normalized_page_address(html_path: &str) -> String {
    normalize_storage_path(html_path).to_string()
}

fn is_content_node_placeholder_path(html_path: &str) -> bool {
    html_path.starts_with("_CONTENTS_NODE_")
}

fn placeholder_branch_key(parent_label_path: &[String], title: &str) -> String {
    let mut path = parent_label_path.to_vec();
    path.push(normalize_title_key(title));
    format!("placeholder-branch|{}", path.join("/"))
}

fn validate_locale_code(path: &Path, locale: &str) -> Result<(), SiteGenerationError> {
    let valid = !locale.is_empty()
        && locale
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(SiteGenerationError::UnsupportedLocale {
            path: path.to_path_buf(),
            locale: locale.to_string(),
        })
    }
}

fn build_id(books: &[SourceBook]) -> String {
    let mut seed = String::new();
    for book in books {
        seed.push_str(book.id.as_str());
        seed.push('|');
        seed.push_str(&book.file_name);
        seed.push('|');
        seed.push_str(&book.file_size_bytes.to_string());
        seed.push('\n');
    }
    format!("build-{}", stable_hash_hex(&seed))
}

fn display_title(page: &TocPage) -> String {
    let title = page.title.display().trim();
    if title.is_empty() {
        "Untitled".to_string()
    } else {
        title.to_string()
    }
}

fn normalize_title_key(title: &str) -> String {
    title
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn unique_id(base: String, used_ids: &mut BTreeSet<String>) -> String {
    let base = if base.is_empty() {
        "book".to_string()
    } else {
        base
    };
    if used_ids.insert(base.clone()) {
        return base;
    }
    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if used_ids.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded counter must find a unique id")
}

fn slugify(value: &str) -> String {
    let mut output = String::new();
    let mut pending_separator = false;
    for character in value.trim().chars() {
        if character.is_alphanumeric() {
            if pending_separator && !output.is_empty() {
                output.push('-');
            }
            for lowercase in character.to_lowercase() {
                output.push(lowercase);
            }
            pending_separator = false;
        } else {
            pending_separator = true;
        }
    }
    if output.is_empty() {
        "item".to_string()
    } else {
        output
    }
}

fn stable_hash_hex(value: &str) -> String {
    let mut hasher = StableFnv64::default();
    hasher.write(value.as_bytes());
    format!("{:016x}", hasher.finish())
}

#[derive(Default)]
struct StableFnv64(u64);

impl Hasher for StableFnv64 {
    fn finish(&self) -> u64 {
        if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        }
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut hash = if self.0 == 0 {
            0xcbf29ce484222325
        } else {
            self.0
        };
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.0 = hash;
    }
}

fn has_hbk_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("hbk"))
}

fn path_sort_key(path: &Path) -> (String, String) {
    (path_file_name(path), path.display().to_string())
}

fn path_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string()
}

fn path_file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hbk_book::test_utils::{fixture_container, zip_bytes, zip_entries};
    use serde_json::Value;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn discovers_directory_books_with_include_filter_in_deterministic_order() {
        let workspace = TempWorkspace::new("discovery");
        fs::write(workspace.path().join("b_ru.hbk"), b"b").expect("fixture file must be written");
        fs::write(workspace.path().join("a_ru.hbk"), b"a").expect("fixture file must be written");
        fs::write(workspace.path().join("ignored.txt"), b"x")
            .expect("fixture file must be written");
        fs::write(workspace.path().join("c_ru.hbk"), b"c").expect("fixture file must be written");

        let source = SiteSource::Directory {
            source_dir: workspace.path().to_path_buf(),
            include_file_names: vec!["b_ru.hbk".to_string(), "a_ru.hbk".to_string()],
        };

        let discovered = discover_source_books(&source).expect("source discovery must succeed");
        let file_names: Vec<_> = discovered.iter().map(|path| path_file_name(path)).collect();

        assert_eq!(file_names, vec!["a_ru.hbk", "b_ru.hbk"]);
    }

    #[test]
    fn writes_manifest_root_section_and_page_markdown_artifacts() {
        let workspace = TempWorkspace::new("artifacts");
        let first = workspace.path().join("alpha_ru.hbk");
        let second = workspace.path().join("beta_ru.hbk");
        write_book_fixture_with_toc(
            &first,
            "Alpha",
            "alpha",
            r#"{
                4
                {1,0,2,2,3,{0,0,{0,0,{"ru","Общее"}{"en","Common"}},""}}
                {2,1,1,4,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/alpha/page.html"}}
                {3,1,0,{0,0,{0,0,{"ru","Раздел"}{"en","Section"}},"/alpha/section.html"}}
                {4,2,0,{0,0,{0,0,{"ru","Подраздел"}{"en","Subsection"}},""}}
            }"#,
            vec![
                (
                    "alpha/page.html",
                    "<html><body><h1>Страница</h1><p>alpha page body</p><a href=\"#Local\">local</a><a href=\"v8help://Alpha/alpha/section.html#Anchor\">section</a><h2 id=\"Local\">Local</h2></body></html>".as_bytes(),
                ),
                (
                    "alpha/section.html",
                    b"<html><body><h1 id=\"Anchor\">Section</h1><p>section body</p></body></html>",
                ),
            ],
        );
        write_book_fixture_with_toc(
            &second,
            "Beta",
            "beta",
            r#"{
                3
                {1,0,1,2,{0,0,{0,0,{"ru","Общее"}{"en","Common"}},""}}
                {2,1,1,3,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/beta/page.html"}}
                {3,2,0,{0,0,{0,0,{"ru","Подраздел"}{"en","Subsection"}},""}}
            }"#,
            vec![(
                "beta/page.html",
                "<html><body><h1>Страница</h1><p>beta page body</p><a href=\"v8help://alpha/alpha/page.html\">alpha</a></body></html>".as_bytes(),
            )],
        );
        let output = workspace.path().join("out");
        let request =
            SiteGenerationRequest::explicit_files(&output, vec![second.clone(), first.clone()])
                .expect("explicit request must be valid");

        let result = DocSiteGenerator::generate(&request).expect("site data must generate");

        assert_eq!(result.locale_count(), 1);
        assert_eq!(result.book_count(), 2);
        assert_eq!(result.page_count(), 3);
        assert!(output.join("data/locales/ru/pages").exists());
        assert!(output.join("data/manifest.json").exists());
        assert!(output.join("data/locales/ru/toc-root.json").exists());
        assert!(
            result
                .files()
                .iter()
                .any(|file| file.path().ends_with("data/manifest.json") && file.bytes_written() > 0)
        );

        let manifest = read_json(output.join("data/manifest.json"));
        assert_eq!(manifest["schema_version"], 1);
        assert_eq!(manifest["generator"], "hbk-doc-site");
        assert_eq!(manifest["generator_version"], env!("CARGO_PKG_VERSION"));
        assert!(manifest["build_id"].as_str().unwrap().starts_with("build-"));
        assert_eq!(manifest["locales"], serde_json::json!(["ru"]));
        assert_eq!(manifest["books"]["ru"][0]["book_id"], "alpha-ru");
        assert_eq!(manifest["books"]["ru"][1]["book_id"], "beta-ru");
        assert!(
            manifest["books"]["ru"][0]["file_size_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert_eq!(
            manifest["toc_roots"]["ru"],
            serde_json::json!("locales/ru/toc-root.json")
        );
        assert_eq!(
            manifest["page_roots"]["ru"],
            serde_json::json!("locales/ru/pages")
        );

        let root = read_json(output.join("data/locales/ru/toc-root.json"));
        let root_nodes = root["nodes"]
            .as_array()
            .expect("root nodes must be an array");
        assert_eq!(root_nodes.len(), 1, "{root}");
        assert_eq!(root_nodes[0]["title"], "Общее");
        assert_eq!(root_nodes[0]["has_children"], true);
        let children_path = root_nodes[0]["children_path"]
            .as_str()
            .expect("merged section must have children_path");
        let section = read_json(output.join("data/locales/ru").join(children_path));
        let section_nodes = section["nodes"]
            .as_array()
            .expect("section nodes must be an array");
        assert_eq!(section_nodes.len(), 3, "{section}");

        let duplicate_pages: Vec<_> = section_nodes
            .iter()
            .filter(|node| node["title"] == "Страница")
            .collect();
        assert_eq!(duplicate_pages.len(), 2, "{section}");
        assert_eq!(duplicate_pages[0]["book_id"], "alpha-ru");
        assert_eq!(duplicate_pages[1]["book_id"], "beta-ru");
        assert_ne!(duplicate_pages[0]["page_id"], duplicate_pages[1]["page_id"]);
        let alpha_children_path = duplicate_pages[0]["children_path"]
            .as_str()
            .expect("first duplicate page must keep its child section path");
        let beta_children_path = duplicate_pages[1]["children_path"]
            .as_str()
            .expect("second duplicate page must keep its child section path");
        assert_ne!(alpha_children_path, beta_children_path);
        let alpha_children = read_json(output.join("data/locales/ru").join(alpha_children_path));
        let beta_children = read_json(output.join("data/locales/ru").join(beta_children_path));
        assert_eq!(alpha_children["nodes"][0]["title"], "Подраздел");
        assert_eq!(beta_children["nodes"][0]["title"], "Подраздел");

        let alpha_page_id = duplicate_pages[0]["page_id"]
            .as_str()
            .expect("duplicate page must expose page_id");
        let beta_page_id = duplicate_pages[1]["page_id"]
            .as_str()
            .expect("duplicate page must expose page_id");
        let alpha_markdown = fs::read_to_string(
            output
                .join("data/locales/ru/pages")
                .join(format!("{alpha_page_id}.md")),
        )
        .expect("alpha page Markdown must be written");
        let beta_markdown = fs::read_to_string(
            output
                .join("data/locales/ru/pages")
                .join(format!("{beta_page_id}.md")),
        )
        .expect("beta page Markdown must be written");
        assert!(alpha_markdown.contains("# Страница"));
        assert!(alpha_markdown.contains("alpha page body"));
        assert!(alpha_markdown.contains("[local](#Local)"));
        assert!(!alpha_markdown.contains(&format!("[local]({alpha_page_id}.md#Local)")));
        assert!(!alpha_markdown.contains("[local](index.md#Local)"));
        assert!(alpha_markdown.contains("[section]("));
        assert!(alpha_markdown.contains("#Anchor"));
        assert!(!alpha_markdown.contains("v8help://Alpha"));
        assert!(!alpha_markdown.contains("/alpha/section.html"));
        assert!(beta_markdown.contains("# Страница"));
        assert!(beta_markdown.contains("beta page body"));
        assert!(beta_markdown.contains("[alpha]("));
        assert!(!beta_markdown.contains("v8help://alpha"));
        let alpha_page_file_name = format!("{alpha_page_id}.md");
        assert!(result.files().iter().any(|file| {
            file.path().file_name().and_then(|name| name.to_str())
                == Some(alpha_page_file_name.as_str())
        }));
    }

    #[test]
    fn merges_page_bearing_toc_nodes_by_normalized_address() {
        let workspace = TempWorkspace::new("page-address-merge");
        let first = workspace.path().join("alpha_ru.hbk");
        let second = workspace.path().join("beta_ru.hbk");
        write_book_fixture_with_toc(
            &first,
            "Alpha",
            "alpha",
            r#"{
                3
                {1,0,1,2,{0,0,{0,0,{"ru","Навигационный заголовок"}{"en","Navigation title"}},"/shared/page.html"}}
                {2,1,0,{0,0,{0,0,{"ru","Дочерняя страница Alpha"}{"en","Alpha child"}},"/alpha/child.html"}}
                {3,0,0,{0,0,{0,0,{"ru","Отдельная страница"}{"en","Separate page"}},"/alpha/separate.html"}}
            }"#,
            vec![
                (
                    "shared/page.html",
                    r#"<html><head><title>Ненадежный HTML title</title></head><body><p>alpha body</p><a href="v8help://Beta/shared/page.html">duplicate</a></body></html>"#.as_bytes(),
                ),
                (
                    "alpha/child.html",
                    b"<html><body><h1>Alpha child</h1></body></html>",
                ),
                (
                    "alpha/separate.html",
                    b"<html><body><h1>Separate</h1></body></html>",
                ),
            ],
        );
        write_book_fixture_with_toc(
            &second,
            "Beta",
            "beta",
            r#"{
                2
                {1,0,1,2,{0,0,{0,0,{"ru","Другой TOC заголовок"}{"en","Other TOC title"}},"shared/page.html"}}
                {2,1,0,{0,0,{0,0,{"ru","Дочерняя страница Beta"}{"en","Beta child"}},"/beta/child.html"}}
            }"#,
            vec![
                (
                    "shared/page.html",
                    b"<html><body><h1>Beta HTML title</h1><p>beta body</p></body></html>",
                ),
                (
                    "beta/child.html",
                    b"<html><body><h1>Beta child</h1></body></html>",
                ),
            ],
        );
        let output = workspace.path().join("out");
        let request =
            SiteGenerationRequest::explicit_files(&output, vec![second.clone(), first.clone()])
                .expect("explicit request must be valid");

        let result = DocSiteGenerator::generate(&request).expect("site data must generate");

        assert_eq!(result.page_count(), 4);
        let root = read_json(output.join("data/locales/ru/toc-root.json"));
        let root_nodes = root["nodes"]
            .as_array()
            .expect("root nodes must be an array");
        assert_eq!(root_nodes.len(), 2, "{root}");
        let merged = &root_nodes[0];
        assert_eq!(merged["title"], "Навигационный заголовок");
        assert_eq!(merged["book_id"], "alpha-ru");
        let merged_page_id = merged["page_id"]
            .as_str()
            .expect("merged node must expose page_id");
        let children_path = merged["children_path"]
            .as_str()
            .expect("merged node must keep merged child sections");
        let children = read_json(output.join("data/locales/ru").join(children_path));
        let child_titles: Vec<_> = children["nodes"]
            .as_array()
            .expect("child nodes must be an array")
            .iter()
            .map(|node| node["title"].as_str().unwrap())
            .collect();
        assert_eq!(
            child_titles,
            vec!["Дочерняя страница Alpha", "Дочерняя страница Beta"]
        );

        let page_files = fs::read_dir(output.join("data/locales/ru/pages"))
            .expect("pages directory must exist")
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(page_files.len(), 4);
        assert!(
            output
                .join("data/locales/ru/pages")
                .join(format!("{merged_page_id}.md"))
                .exists()
        );
        let merged_markdown = fs::read_to_string(
            output
                .join("data/locales/ru/pages")
                .join(format!("{merged_page_id}.md")),
        )
        .expect("merged page Markdown must be written");
        assert!(merged_markdown.contains("alpha body"));
        assert!(!merged_markdown.contains("beta body"));
        assert!(merged_markdown.contains("[duplicate]("));
        assert!(!merged_markdown.contains("v8help://Beta"));
    }

    #[test]
    fn merges_content_node_placeholder_pages_by_address() {
        let workspace = TempWorkspace::new("content-node-page-identity");
        let first = workspace.path().join("alpha_ru.hbk");
        let second = workspace.path().join("beta_ru.hbk");
        write_book_fixture_with_toc(
            &first,
            "Alpha",
            "alpha",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Раздел Alpha"}{"en","Alpha section"}},"_CONTENTS_NODE_file3"}}
            }"#,
            vec![],
        );
        write_book_fixture_with_toc(
            &second,
            "Beta",
            "beta",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Раздел Beta"}{"en","Beta section"}},"_CONTENTS_NODE_file3"}}
            }"#,
            vec![],
        );
        let output = workspace.path().join("out");
        let request = SiteGenerationRequest::explicit_files(&output, vec![second, first])
            .expect("explicit request must be valid");

        let result = DocSiteGenerator::generate(&request).expect("site data must generate");

        assert_eq!(result.page_count(), 1);
        let root = read_json(output.join("data/locales/ru/toc-root.json"));
        let root_nodes = root["nodes"]
            .as_array()
            .expect("root nodes must be an array");
        assert_eq!(root_nodes.len(), 1, "{root}");
        assert_eq!(root_nodes[0]["title"], "Раздел Alpha");
        assert_eq!(root_nodes[0]["book_id"], "alpha-ru");
        assert!(
            root_nodes[0]["page_id"]
                .as_str()
                .unwrap()
                .starts_with("page-ru-")
        );
    }

    #[test]
    fn resolves_placeholder_page_branch_to_single_concrete_target() {
        let workspace = TempWorkspace::new("placeholder-to-concrete-page");
        let placeholder = workspace.path().join("alpha_ru.hbk");
        let concrete = workspace.path().join("beta_ru.hbk");
        write_book_fixture_with_toc(
            &placeholder,
            "Alpha",
            "alpha",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Общий раздел"}{"en","Shared section"}},"_CONTENTS_NODE_file3"}}
            }"#,
            vec![],
        );
        write_book_fixture_with_toc(
            &concrete,
            "Beta",
            "beta",
            r##"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Общий раздел"}{"en","Shared section"}},"/real/page.html"}}
            }"##,
            vec![(
                "real/page.html",
                r##"<html><body><p>real body</p><a href="v8help://Alpha/_CONTENTS_NODE_file3#Details">placeholder link</a><h2 id="Details">Details</h2></body></html>"##.as_bytes(),
            )],
        );
        let output = workspace.path().join("out");
        let request = SiteGenerationRequest::explicit_files(&output, vec![placeholder, concrete])
            .expect("explicit request must be valid");

        let result = DocSiteGenerator::generate(&request).expect("site data must generate");

        assert_eq!(result.page_count(), 1);
        let root = read_json(output.join("data/locales/ru/toc-root.json"));
        let root_nodes = root["nodes"]
            .as_array()
            .expect("root nodes must be an array");
        assert_eq!(root_nodes.len(), 1, "{root}");
        assert_eq!(root_nodes[0]["title"], "Общий раздел");
        assert_eq!(root_nodes[0]["book_id"], "beta-ru");
        let page_id = root_nodes[0]["page_id"]
            .as_str()
            .expect("resolved page must expose page id");
        let markdown = fs::read_to_string(
            output
                .join("data/locales/ru/pages")
                .join(format!("{page_id}.md")),
        )
        .expect("resolved page Markdown must be written");
        assert!(markdown.contains("real body"));
        assert!(markdown.contains("[placeholder link]("));
        assert!(markdown.contains("#Details"));
        assert!(!markdown.contains("v8help://Alpha"));
        assert!(!markdown.contains("_CONTENTS_NODE_file3"));
    }

    #[test]
    fn keeps_placeholder_page_when_concrete_target_is_ambiguous() {
        let workspace = TempWorkspace::new("placeholder-ambiguous-page");
        let placeholder = workspace.path().join("alpha_ru.hbk");
        let first_concrete = workspace.path().join("beta_ru.hbk");
        let second_concrete = workspace.path().join("gamma_ru.hbk");
        write_book_fixture_with_toc(
            &placeholder,
            "Alpha",
            "alpha",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Общий раздел"}{"en","Shared section"}},"_CONTENTS_NODE_file3"}}
            }"#,
            vec![],
        );
        write_book_fixture_with_toc(
            &first_concrete,
            "Beta",
            "beta",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Общий раздел"}{"en","Shared section"}},"/first/page.html"}}
            }"#,
            vec![("first/page.html", b"<html><body><p>first</p></body></html>")],
        );
        write_book_fixture_with_toc(
            &second_concrete,
            "Gamma",
            "gamma",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Общий раздел"}{"en","Shared section"}},"/second/page.html"}}
            }"#,
            vec![(
                "second/page.html",
                b"<html><body><p>second</p></body></html>",
            )],
        );
        let output = workspace.path().join("out");
        let request = SiteGenerationRequest::explicit_files(
            &output,
            vec![placeholder, first_concrete, second_concrete],
        )
        .expect("explicit request must be valid");

        let result = DocSiteGenerator::generate(&request).expect("site data must generate");

        assert_eq!(result.page_count(), 3);
        let root = read_json(output.join("data/locales/ru/toc-root.json"));
        let root_nodes = root["nodes"]
            .as_array()
            .expect("root nodes must be an array");
        assert_eq!(root_nodes.len(), 3, "{root}");
        assert_eq!(root_nodes[0]["book_id"], "alpha-ru");
        assert_eq!(root_nodes[1]["book_id"], "beta-ru");
        assert_eq!(root_nodes[2]["book_id"], "gamma-ru");
        assert_ne!(root_nodes[0]["page_id"], root_nodes[1]["page_id"]);
        assert_ne!(root_nodes[0]["page_id"], root_nodes[2]["page_id"]);
    }

    #[test]
    fn generate_with_progress_reports_books_planning_and_artifacts() {
        let workspace = TempWorkspace::new("progress");
        let source = workspace.path().join("alpha_ru.hbk");
        write_book_fixture_with_toc(
            &source,
            "Alpha",
            "alpha",
            r#"{
                2
                {1,0,1,2,{0,0,{0,0,{"ru","Раздел"}{"en","Section"}},""}}
                {2,1,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/alpha/page.html"}}
            }"#,
            vec![(
                "alpha/page.html",
                "<html><body><h1>Страница</h1><p>page body</p></body></html>".as_bytes(),
            )],
        );
        let output = workspace.path().join("out");
        let request = SiteGenerationRequest::explicit_files(&output, vec![source.clone()])
            .expect("explicit request must be valid");
        let mut events = Vec::new();

        let result = DocSiteGenerator::generate_with_progress(&request, |event| match event {
            SiteGenerationProgress::SourceBooksDiscovered { count } => {
                events.push(format!("discovered:{count}"));
            }
            SiteGenerationProgress::SourceBookLoading {
                current,
                total,
                path,
            } => {
                events.push(format!(
                    "loading:{current}/{total}:{}",
                    path_file_name(path)
                ));
            }
            SiteGenerationProgress::SourceBooksLoaded { count } => {
                events.push(format!("loaded:{count}"));
            }
            SiteGenerationProgress::SiteDataBuilt {
                locale_count,
                toc_node_count,
                page_count,
            } => {
                events.push(format!(
                    "planned:{locale_count}:{toc_node_count}:{page_count}"
                ));
            }
            SiteGenerationProgress::ArtifactWriting {
                current,
                total,
                kind,
                path: _,
            } => {
                events.push(format!("artifact:{current}/{total}:{kind:?}"));
            }
        })
        .expect("site data must generate");

        assert_eq!(result.book_count(), 1);
        assert_eq!(result.page_count(), 1);
        assert_eq!(
            events,
            vec![
                "discovered:1",
                "loading:1/1:alpha_ru.hbk",
                "loaded:1",
                "planned:1:2:1",
                "artifact:1/4:Manifest",
                "artifact:2/4:TocRoot",
                "artifact:3/4:TocSection",
                "artifact:4/4:Page",
            ]
        );
    }

    #[test]
    fn rejects_unsafe_locale_code_before_writing_locale_artifacts() {
        let workspace = TempWorkspace::new("bad-locale");
        let source = workspace.path().join("alpha_...hbk");
        write_book_fixture_with_toc(
            &source,
            "Alpha",
            "alpha",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/alpha/page.html"}}
            }"#,
            vec![("alpha/page.html", b"<html><body>alpha</body></html>")],
        );
        let output = workspace.path().join("out");
        let request = SiteGenerationRequest::explicit_files(&output, vec![source.clone()])
            .expect("explicit request must be valid");

        let error = DocSiteGenerator::generate(&request)
            .expect_err("unsafe locale path segment must be rejected");

        match error {
            SiteGenerationError::UnsupportedLocale { path, locale } => {
                assert_eq!(path, source);
                assert_eq!(locale, "..");
            }
            other => panic!("unexpected error: {other}"),
        }
        assert!(!output.exists());
    }

    #[test]
    fn generated_toc_artifacts_are_deterministic_across_runs() {
        let workspace = TempWorkspace::new("deterministic");
        let source = workspace.path().join("alpha_ru.hbk");
        write_book_fixture_with_toc(
            &source,
            "Alpha",
            "alpha",
            r#"{
                2
                {1,0,1,2,{0,0,{0,0,{"ru","Корень"}{"en","Root"}},""}}
                {2,1,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/alpha/page.html"}}
            }"#,
            vec![("alpha/page.html", b"<html><body>alpha</body></html>")],
        );

        let output_one = workspace.path().join("out-one");
        let output_two = workspace.path().join("out-two");
        let request_one = SiteGenerationRequest::explicit_files(&output_one, vec![source.clone()])
            .expect("first request must be valid");
        let request_two = SiteGenerationRequest::explicit_files(&output_two, vec![source])
            .expect("second request must be valid");

        DocSiteGenerator::generate(&request_one).expect("first generation must succeed");
        DocSiteGenerator::generate(&request_two).expect("second generation must succeed");

        assert_eq!(
            fs::read_to_string(output_one.join("data/manifest.json")).unwrap(),
            fs::read_to_string(output_two.join("data/manifest.json")).unwrap()
        );
        assert_eq!(
            fs::read_to_string(output_one.join("data/locales/ru/toc-root.json")).unwrap(),
            fs::read_to_string(output_two.join("data/locales/ru/toc-root.json")).unwrap()
        );
        assert_eq!(
            only_page_file_name(&output_one),
            only_page_file_name(&output_two)
        );
        assert_eq!(
            fs::read_to_string(only_page_file(&output_one)).unwrap(),
            fs::read_to_string(only_page_file(&output_two)).unwrap()
        );
        let section_one = only_section_file(&output_one);
        let section_two = only_section_file(&output_two);
        assert_eq!(
            fs::read_to_string(section_one).unwrap(),
            fs::read_to_string(section_two).unwrap()
        );
    }

    fn write_book_fixture_with_toc(
        path: &Path,
        book_name: &str,
        description: &str,
        toc: &str,
        storage_entries: Vec<(&str, &[u8])>,
    ) {
        fs::write(
            path,
            fixture_container(vec![
                (
                    "Book",
                    Some(
                        format!(
                            r#"{{1,"{book_name}", {{1,2,{{"ru","{description}"}}}}, 1, "tag", {{0,0}}, 0}}"#
                        )
                        .into_bytes(),
                    ),
                ),
                ("PackBlock", Some(zip_bytes("toc.txt", toc.as_bytes()))),
                ("FileStorage", Some(zip_entries(storage_entries))),
            ]),
        )
        .expect("fixture HBK must be written");
    }

    fn read_json(path: impl AsRef<Path>) -> Value {
        let text = fs::read_to_string(path).expect("JSON artifact must be readable");
        serde_json::from_str(&text).expect("JSON artifact must parse")
    }

    fn only_section_file(output: &Path) -> PathBuf {
        let sections_dir = output.join("data/locales/ru/toc-sections");
        let mut files = fs::read_dir(sections_dir)
            .expect("sections directory must exist")
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(files.len(), 1);
        files.remove(0)
    }

    fn only_page_file(output: &Path) -> PathBuf {
        let pages_dir = output.join("data/locales/ru/pages");
        let mut files = fs::read_dir(pages_dir)
            .expect("pages directory must exist")
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(files.len(), 1);
        files.remove(0)
    }

    fn only_page_file_name(output: &Path) -> String {
        only_page_file(output)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    struct TempWorkspace {
        path: PathBuf,
    }

    impl TempWorkspace {
        fn new(name: &str) -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "v8-context-hbk-doc-site-test-{name}-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("temp workspace must be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
