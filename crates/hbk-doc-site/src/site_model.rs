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
