use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::Serialize;
use strsim::levenshtein;
use syntax_helper_model as model;

pub const INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct IndexMetadata {
    pub locale: String,
    pub source_locale: String,
    pub source_hbk: String,
    pub source_extraction_schema_version: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Keywords,
    Fuzzy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchHit {
    pub document: SearchDocument,
    pub score: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelatedHit {
    pub document: SearchDocument,
    pub depth: u32,
    pub via: Vec<RelationStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RelationStep {
    pub from: String,
    pub to: String,
    pub edge_kind: String,
    pub label: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SearchDocument {
    pub id: String,
    pub kind: String,
    pub name: model::LocalizedName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<model::LocalizedName>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub signatures: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub type_refs: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub return_types: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub preview: String,
    #[serde(skip)]
    pub relation_keys: Vec<String>,
    #[serde(skip)]
    pub owner_relation_key: Option<String>,
}

#[derive(Debug)]
pub enum SearchError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Sqlite {
        path: PathBuf,
        source: rusqlite::Error,
    },
    WriterLockTimeout {
        path: PathBuf,
    },
    MissingIndex {
        path: PathBuf,
    },
    AmbiguousLookup {
        name: String,
        matches: usize,
    },
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    f,
                    "failed to access search index '{}': {source}",
                    path.display()
                )
            }
            Self::Sqlite { path, source } => {
                write!(
                    f,
                    "failed to use search index '{}': {source}",
                    path.display()
                )
            }
            Self::WriterLockTimeout { path } => {
                write!(
                    f,
                    "timed out waiting for search index writer lock '{}'",
                    path.display()
                )
            }
            Self::MissingIndex { path } => {
                write!(f, "search index does not exist: {}", path.display())
            }
            Self::AmbiguousLookup { name, matches } => {
                write!(
                    f,
                    "ambiguous Syntax Assistant lookup for '{name}': {matches} matches"
                )
            }
        }
    }
}

impl std::error::Error for SearchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Sqlite { source, .. } => Some(source),
            Self::WriterLockTimeout { .. }
            | Self::MissingIndex { .. }
            | Self::AmbiguousLookup { .. } => None,
        }
    }
}

pub fn build_index(
    path: impl AsRef<Path>,
    metadata: &IndexMetadata,
    context: &model::PlatformContext,
) -> Result<(), SearchError> {
    let path = path.as_ref();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| SearchError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let _lock = WriterLock::acquire(path)?;
    let temp_path = temp_index_path(path);
    if temp_path.exists() {
        fs::remove_file(&temp_path).map_err(|source| SearchError::Io {
            path: temp_path.clone(),
            source,
        })?;
    }

    let result = build_index_file(&temp_path, metadata, context).and_then(|()| {
        remove_sqlite_sidecars(path)?;
        fs::rename(&temp_path, path).map_err(|source| SearchError::Io {
            path: path.to_path_buf(),
            source,
        })
    });
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn build_index_file(
    path: &Path,
    metadata: &IndexMetadata,
    context: &model::PlatformContext,
) -> Result<(), SearchError> {
    let mut connection = Connection::open(path).map_err(|source| SearchError::Sqlite {
        path: path.to_path_buf(),
        source,
    })?;
    create_schema(&connection, path)?;
    write_metadata(&connection, path, metadata)?;
    let documents = documents_from_context(context);
    let relations = relations_from_documents(&documents);
    let transaction = connection
        .transaction()
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    insert_documents(&transaction, path, &documents)?;
    insert_relations(&transaction, path, &relations)?;
    transaction.commit().map_err(|source| SearchError::Sqlite {
        path: path.to_path_buf(),
        source,
    })?;
    validate_index(&connection, path)
}

pub struct SearchIndex {
    path: PathBuf,
    connection: Connection,
}

impl SearchIndex {
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self, SearchError> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(SearchError::MissingIndex {
                path: path.to_path_buf(),
            });
        }
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            connection,
        })
    }

    pub fn get_by_name(&self, name: &str) -> Result<Vec<SearchHit>, SearchError> {
        let key = normalize_lookup_key(name);
        self.get_by_key(&key)
    }

    pub fn get_by_owner_member(
        &self,
        owner: &str,
        member: &str,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let key = owner_member_key(owner, member);
        self.get_by_key(&key)
    }

    pub fn search(
        &self,
        query: &str,
        mode: SearchMode,
        limit: usize,
    ) -> Result<Vec<SearchHit>, SearchError> {
        match mode {
            SearchMode::Keywords => self.keyword_search(query, limit),
            SearchMode::Fuzzy => self.fuzzy_search(query, limit),
        }
    }

    pub fn related_by_name(
        &self,
        name: &str,
        max_depth: u32,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        let roots = self.get_by_name(name)?;
        let Some(root) = roots.first() else {
            return Ok(Vec::new());
        };
        if roots.len() > 1 {
            return Err(SearchError::AmbiguousLookup {
                name: name.to_string(),
                matches: roots.len(),
            });
        }
        self.related(&root.document.id, max_depth.min(5), limit)
    }

    pub fn document_count(&self) -> Result<i64, SearchError> {
        self.connection
            .query_row("SELECT count(*) FROM documents", [], |row| row.get(0))
            .map_err(|source| self.sqlite(source))
    }

    fn get_by_key(&self, key: &str) -> Result<Vec<SearchHit>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.parameter_text, d.type_names, \
                 d.return_names, d.description, d.preview \
                 FROM document_names n \
                 JOIN documents d ON d.id = n.document_id \
                 WHERE n.key = ?1 \
                 ORDER BY d.kind_priority, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([key], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        let ownerless = hits
            .iter()
            .filter(|hit| hit.document.owner.is_none())
            .cloned()
            .collect::<Vec<_>>();
        if ownerless.is_empty() {
            Ok(hits)
        } else {
            Ok(ownerless)
        }
    }

    fn keyword_search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, SearchError> {
        let fts_query = fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.parameter_text, d.type_names, \
                 d.return_names, d.description, d.preview, CAST(bm25(document_fts) * 1000000 AS INTEGER) \
                 FROM document_fts \
                 JOIN documents d ON d.id = document_fts.document_id \
                 WHERE document_fts MATCH ?1 \
                 ORDER BY bm25(document_fts), d.kind_priority, d.id \
                 LIMIT ?2",
            )
            .map_err(|source| self.sqlite(source))?;
        let sql_limit = limit.saturating_mul(10).max(50) as i64;
        let rows = statement
            .query_map(params![fts_query, sql_limit], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: row.get(12)?,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let mut hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        hits.sort_by(|left, right| {
            keyword_order(query, &left.document)
                .cmp(&keyword_order(query, &right.document))
                .then_with(|| left.score.cmp(&right.score))
                .then_with(|| left.document.id.cmp(&right.document.id))
        });
        hits.truncate(limit);
        Ok(hits)
    }

    fn fuzzy_search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, SearchError> {
        let normalized_query = normalize_lookup_key(query);
        let mut candidates = self.fuzzy_candidates(&normalized_query, limit.saturating_mul(20))?;
        candidates.sort_by(|left, right| {
            let left_score = fuzzy_score(&normalized_query, &left.document);
            let right_score = fuzzy_score(&normalized_query, &right.document);
            left_score
                .cmp(&right_score)
                .then_with(|| left.document.kind.cmp(&right.document.kind))
                .then_with(|| left.document.id.cmp(&right.document.id))
        });
        Ok(candidates
            .into_iter()
            .filter_map(|mut hit| {
                let distance = fuzzy_score(&normalized_query, &hit.document);
                (distance <= fuzzy_threshold(&normalized_query)).then(|| {
                    hit.score = distance as i64;
                    hit
                })
            })
            .take(limit)
            .collect())
    }

    fn fuzzy_candidates(
        &self,
        normalized_query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let prefix = normalized_query.chars().take(4).collect::<String>();
        if prefix.is_empty() {
            return Ok(Vec::new());
        }
        let pattern = format!("{prefix}%");
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.parameter_text, d.type_names, \
                 d.return_names, d.description, d.preview \
                 FROM document_names n \
                 JOIN documents d ON d.id = n.document_id \
                 WHERE n.key LIKE ?1 \
                 ORDER BY d.kind_priority, d.id \
                 LIMIT ?2",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![pattern, limit as i64], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    fn related(
        &self,
        root_id: &str,
        max_depth: u32,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        let mut queue = VecDeque::from([(root_id.to_string(), 0, Vec::<RelationStep>::new())]);
        let mut visited = BTreeSet::from([root_id.to_string()]);
        let mut hits = Vec::new();

        while let Some((document_id, depth, path)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            for edge in self.edges(&document_id)? {
                if !visited.insert(edge.to.clone()) {
                    continue;
                }
                let Some(document) = self.document(&edge.to)? else {
                    continue;
                };
                let mut via = path.clone();
                via.push(edge.clone());
                hits.push(RelatedHit {
                    document,
                    depth: depth + 1,
                    via: via.clone(),
                });
                if hits.len() >= limit {
                    return Ok(hits);
                }
                queue.push_back((edge.to, depth + 1, via));
            }
        }
        Ok(hits)
    }

    fn edges(&self, document_id: &str) -> Result<Vec<RelationStep>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT source_id, target_id, edge_kind, label, evidence \
                 FROM relations \
                 WHERE source_id = ?1 \
                 ORDER BY weight, edge_kind, target_id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([document_id], |row| {
                let source: String = row.get(0)?;
                let target: String = row.get(1)?;
                Ok(RelationStep {
                    from: source,
                    to: target,
                    edge_kind: row.get(2)?,
                    label: row.get(3)?,
                    evidence: row.get(4)?,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    fn document(&self, id: &str) -> Result<Option<SearchDocument>, SearchError> {
        self.connection
            .query_row(
                "SELECT id, kind, name_primary, name_alias, owner_primary, owner_alias, \
                 signature_text, parameter_text, type_names, return_names, description, preview \
                 FROM documents WHERE id = ?1",
                [id],
                document_from_row,
            )
            .optional()
            .map_err(|source| self.sqlite(source))
    }

    fn sqlite(&self, source: rusqlite::Error) -> SearchError {
        SearchError::Sqlite {
            path: self.path.clone(),
            source,
        }
    }
}

#[derive(Debug, Clone)]
struct Relation {
    source_id: String,
    target_id: String,
    edge_kind: &'static str,
    label: String,
    evidence: &'static str,
    weight: i64,
}

fn create_schema(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    connection
        .execute_batch(
            "PRAGMA journal_mode = DELETE;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE meta (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE documents (
                 id TEXT PRIMARY KEY,
                 kind TEXT NOT NULL,
                 kind_priority INTEGER NOT NULL,
                 name_primary TEXT NOT NULL,
                 name_alias TEXT,
                 owner_primary TEXT,
                 owner_alias TEXT,
                 signature_text TEXT NOT NULL,
                 parameter_text TEXT NOT NULL,
                 type_names TEXT NOT NULL,
                 return_names TEXT NOT NULL,
                 description TEXT,
                 preview TEXT NOT NULL
             );
             CREATE TABLE document_names (
                 key TEXT NOT NULL,
                 key_kind TEXT NOT NULL,
                 document_id TEXT NOT NULL REFERENCES documents(id)
             );
             CREATE INDEX document_names_key_idx ON document_names(key, key_kind, document_id);
             CREATE INDEX documents_owner_member_idx ON documents(owner_primary, name_primary);
             CREATE VIRTUAL TABLE document_fts USING fts5(
                 document_id UNINDEXED,
                 name_primary,
                 name_alias,
                 owner,
                 signatures,
                 parameters,
                 type_names,
                 return_names,
                 description,
                 tokenize = 'unicode61 remove_diacritics 0'
             );
             CREATE TABLE relations (
                 source_id TEXT NOT NULL REFERENCES documents(id),
                 target_id TEXT NOT NULL REFERENCES documents(id),
                 edge_kind TEXT NOT NULL,
                 label TEXT NOT NULL,
                 evidence TEXT NOT NULL,
                 weight INTEGER NOT NULL
             );
             CREATE INDEX relations_source_idx ON relations(source_id, edge_kind, target_id);
             CREATE INDEX relations_target_idx ON relations(target_id, edge_kind, source_id);",
        )
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })
}

fn write_metadata(
    connection: &Connection,
    path: &Path,
    metadata: &IndexMetadata,
) -> Result<(), SearchError> {
    let built_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string());
    for (key, value) in [
        ("schema_version", INDEX_SCHEMA_VERSION.to_string()),
        ("locale", metadata.locale.clone()),
        ("source_locale", metadata.source_locale.clone()),
        ("source_hbk", metadata.source_hbk.clone()),
        (
            "source_extraction_schema_version",
            metadata.source_extraction_schema_version.to_string(),
        ),
        ("built_at", built_at),
        ("builder_version", env!("CARGO_PKG_VERSION").to_string()),
    ] {
        connection
            .execute(
                "INSERT INTO meta(key, value) VALUES (?1, ?2)",
                params![key, value],
            )
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}

fn insert_documents(
    connection: &Connection,
    path: &Path,
    documents: &[SearchDocument],
) -> Result<(), SearchError> {
    for document in documents {
        let signatures = document.signatures.join("\n");
        let parameters = document.parameters.join("\n");
        let type_names = document.type_refs.join("\n");
        let return_names = document.return_types.join("\n");
        let owner_primary = document.owner.as_ref().map(|owner| owner.primary.as_str());
        let owner_alias = document
            .owner
            .as_ref()
            .and_then(|owner| owner.alias.as_deref());
        connection
            .execute(
                "INSERT INTO documents(
                    id, kind, kind_priority, name_primary, name_alias, owner_primary, owner_alias,
                    signature_text, parameter_text, type_names, return_names, description, preview
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    document.id,
                    document.kind,
                    kind_priority(&document.kind),
                    document.name.primary,
                    document.name.alias,
                    owner_primary,
                    owner_alias,
                    signatures,
                    parameters,
                    type_names,
                    return_names,
                    document.description,
                    document.preview,
                ],
            )
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
        insert_name_keys(connection, path, document)?;
        connection
            .execute(
                "INSERT INTO document_fts(
                    document_id, name_primary, name_alias, owner, signatures, parameters,
                    type_names, return_names, description
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    document.id,
                    searchable_name(&document.name.primary),
                    document.name.alias.as_deref().map(searchable_name),
                    document
                        .owner
                        .as_ref()
                        .map(|owner| searchable_name(&display_name(owner))),
                    searchable_text(&signatures),
                    searchable_text(&parameters),
                    searchable_text(&type_names),
                    searchable_text(&return_names),
                    document.description.as_deref().map(searchable_text),
                ],
            )
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}

fn insert_name_keys(
    connection: &Connection,
    path: &Path,
    document: &SearchDocument,
) -> Result<(), SearchError> {
    let mut keys = BTreeSet::new();
    keys.insert((normalize_lookup_key(&document.name.primary), "primary"));
    if let Some(alias) = &document.name.alias {
        keys.insert((normalize_lookup_key(alias), "alias"));
    }
    if let Some(owner) = &document.owner {
        keys.insert((
            owner_member_key(&owner.primary, &document.name.primary),
            "owner_member_primary",
        ));
        if let Some(owner_alias) = &owner.alias {
            keys.insert((
                owner_member_key(owner_alias, &document.name.primary),
                "owner_member_alias",
            ));
        }
        if let Some(name_alias) = &document.name.alias {
            keys.insert((
                owner_member_key(&owner.primary, name_alias),
                "owner_member_alias",
            ));
            if let Some(owner_alias) = &owner.alias {
                keys.insert((
                    owner_member_key(owner_alias, name_alias),
                    "owner_member_alias",
                ));
            }
        }
    }
    for (key, kind) in keys {
        connection
            .execute(
                "INSERT INTO document_names(key, key_kind, document_id) VALUES (?1, ?2, ?3)",
                params![key, kind, document.id],
            )
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}

fn insert_relations(
    connection: &Connection,
    path: &Path,
    relations: &[Relation],
) -> Result<(), SearchError> {
    for relation in relations {
        connection
            .execute(
                "INSERT INTO relations(source_id, target_id, edge_kind, label, evidence, weight)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    relation.source_id,
                    relation.target_id,
                    relation.edge_kind,
                    relation.label,
                    relation.evidence,
                    relation.weight,
                ],
            )
            .map_err(|source| SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            })?;
    }
    Ok(())
}

fn validate_index(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    let count: i64 = connection
        .query_row("SELECT count(*) FROM documents", [], |row| row.get(0))
        .map_err(|source| SearchError::Sqlite {
            path: path.to_path_buf(),
            source,
        })?;
    if count == 0 {
        return Err(SearchError::Sqlite {
            path: path.to_path_buf(),
            source: rusqlite::Error::QueryReturnedNoRows,
        });
    }
    Ok(())
}

fn documents_from_context(context: &model::PlatformContext) -> Vec<SearchDocument> {
    let mut documents = Vec::new();
    for record in &context.global_methods {
        documents.push(document(
            "global_method",
            None,
            &record.name,
            &record.signatures,
            &record.return_types,
            &[],
            record.description.as_deref(),
            &record.source,
        ));
    }
    for record in &context.global_properties {
        documents.push(document(
            "global_property",
            None,
            &record.name,
            &[],
            &[],
            &record.type_refs,
            record.description.as_deref(),
            &record.source,
        ));
    }
    for record in &context.global_context_events {
        let owner = event_owner(record);
        let kind = match record.semantic.record_family {
            model::RecordFamily::ModuleEvent => "module_event",
            model::RecordFamily::TypeEvent => "type_event",
            _ => "unknown_event",
        };
        documents.push(document(
            kind,
            owner.as_ref(),
            &record.name,
            &record.signatures,
            &[],
            &[],
            record.description.as_deref(),
            &record.source,
        ));
    }
    for record in &context.platform_types {
        documents.push(document(
            "platform_type",
            None,
            &record.name,
            &[],
            &[],
            &record
                .extends
                .iter()
                .map(type_ref_from_name)
                .collect::<Vec<_>>(),
            record.description.as_deref(),
            &record.source,
        ));
    }
    for record in &context.type_methods {
        documents.push(document(
            "type_method",
            Some(&record.owner),
            &record.name,
            &record.signatures,
            &record.return_types,
            &[],
            record.description.as_deref(),
            &record.source,
        ));
    }
    for record in &context.type_properties {
        documents.push(document(
            "type_property",
            Some(&record.owner),
            &record.name,
            &[],
            &[],
            &record.type_refs,
            record.description.as_deref(),
            &record.source,
        ));
    }
    for record in &context.constructors {
        let name = model::LocalizedName {
            primary: record
                .signatures
                .first()
                .map(|signature| signature.text.clone())
                .unwrap_or_else(|| format!("Новый {}", record.owner.primary)),
            alias: record.name.alias.clone(),
        };
        documents.push(document(
            "constructor",
            Some(&record.owner),
            &name,
            &record.signatures,
            &[],
            &[],
            record.description.as_deref(),
            &record.source,
        ));
    }
    for record in &context.query_tables {
        let name = model::LocalizedName {
            primary: record.name.clone(),
            alias: record
                .syntax
                .as_ref()
                .and_then(|syntax| syntax.alias.clone()),
        };
        let mut table = document(
            "query_table",
            None,
            &name,
            &[],
            &[],
            &[],
            record.description.as_deref(),
            &record.source,
        );
        table
            .relation_keys
            .push(semantic_relation_key(&record.semantic, &name.primary));
        documents.push(table);
    }
    for record in &context.table_fields {
        let name = model::LocalizedName {
            primary: record.name.clone(),
            alias: None,
        };
        let mut field = document(
            "query_table_field",
            Some(&record.owner),
            &name,
            &[],
            &[],
            &record.type_refs,
            record.description.as_deref(),
            &record.source,
        );
        field.owner_relation_key = Some(semantic_relation_key(
            &record.semantic,
            &record.owner.primary,
        ));
        documents.push(field);
    }
    for record in &context.table_parameters {
        let name = model::LocalizedName {
            primary: record.name.clone(),
            alias: None,
        };
        let mut parameter = document(
            "query_table_parameter",
            Some(&record.owner),
            &name,
            &[],
            &[],
            &record.type_refs,
            record.description.as_deref(),
            &record.source,
        );
        parameter.owner_relation_key = Some(semantic_relation_key(
            &record.semantic,
            &record.owner.primary,
        ));
        documents.push(parameter);
    }
    for record in &context.enums {
        documents.push(document(
            "enum",
            None,
            &record.name,
            &[],
            &[],
            &[],
            record.description.as_deref(),
            &record.source,
        ));
    }
    for record in &context.enum_values {
        documents.push(document(
            "enum_value",
            Some(&record.owner),
            &record.name,
            &[],
            &[],
            &[],
            record.description.as_deref(),
            &record.source,
        ));
    }
    documents.sort_by(|left, right| {
        kind_priority(&left.kind)
            .cmp(&kind_priority(&right.kind))
            .then_with(|| left.id.cmp(&right.id))
    });
    documents
}

fn document(
    kind: &str,
    owner: Option<&model::LocalizedName>,
    name: &model::LocalizedName,
    signatures: &[model::Signature],
    return_types: &[model::TypeRef],
    type_refs: &[model::TypeRef],
    description: Option<&str>,
    source: &model::SyntaxHelperSource,
) -> SearchDocument {
    let parameters = signatures
        .iter()
        .flat_map(|signature| signature.parameters.iter())
        .flat_map(|parameter| {
            std::iter::once(parameter.name.clone()).chain(
                parameter
                    .type_refs
                    .iter()
                    .map(|type_ref| type_ref.name.clone()),
            )
        })
        .collect::<Vec<_>>();
    let signatures = signatures
        .iter()
        .map(|signature| signature.text.clone())
        .collect::<Vec<_>>();
    let return_types = return_types
        .iter()
        .map(|type_ref| type_ref.name.clone())
        .collect::<Vec<_>>();
    let type_refs = type_refs
        .iter()
        .map(|type_ref| type_ref.name.clone())
        .collect::<Vec<_>>();
    let id = document_id(kind, owner, name, source);
    SearchDocument {
        id,
        kind: kind.to_string(),
        name: name.clone(),
        owner: owner.cloned(),
        signatures,
        parameters,
        type_refs,
        return_types,
        description: description.map(ToOwned::to_owned),
        preview: description
            .map(|value| value.chars().take(180).collect())
            .unwrap_or_default(),
        relation_keys: Vec::new(),
        owner_relation_key: None,
    }
}

fn type_ref_from_name(name: &model::LocalizedName) -> model::TypeRef {
    model::TypeRef {
        name: display_name(name),
    }
}

fn event_owner(event: &model::GlobalContextEvent) -> Option<model::LocalizedName> {
    event.module.owner_path.last().cloned().or_else(|| {
        event
            .semantic
            .owner_path
            .last()
            .filter(|_| event.semantic.record_family == model::RecordFamily::TypeEvent)
            .cloned()
    })
}

fn semantic_relation_key(semantic: &model::SemanticContext, fallback: &str) -> String {
    let mut parts = semantic
        .owner_path
        .iter()
        .map(|name| name.primary.as_str())
        .collect::<Vec<_>>();
    if parts
        .last()
        .is_none_or(|last| normalize_lookup_key(last) != normalize_lookup_key(fallback))
    {
        parts.push(fallback);
    }
    normalize_lookup_key(&parts.join("."))
}

fn relations_from_documents(documents: &[SearchDocument]) -> Vec<Relation> {
    let mut by_name = BTreeMap::<String, (&SearchDocument, String)>::new();
    for document in documents {
        for key in document_lookup_keys(document) {
            match by_name.get(&key) {
                Some((existing, _))
                    if kind_priority(&existing.kind) <= kind_priority(&document.kind) => {}
                _ => {
                    by_name.insert(key, (document, document.id.clone()));
                }
            }
        }
    }
    let mut relations = Vec::new();
    for document in documents {
        if let Some(owner) = &document.owner {
            let owner_key = document
                .owner_relation_key
                .as_deref()
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| normalize_lookup_key(&owner.primary));
            if let Some((_, owner_id)) = by_name.get(&owner_key) {
                relations.push(Relation {
                    source_id: owner_id.clone(),
                    target_id: document.id.clone(),
                    edge_kind: "owns",
                    label: format!("{} owns {}", display_name(owner), document.name.primary),
                    evidence: "owner",
                    weight: 10,
                });
                relations.push(Relation {
                    source_id: document.id.clone(),
                    target_id: owner_id.clone(),
                    edge_kind: "member_of",
                    label: format!(
                        "{} member of {}",
                        document.name.primary,
                        display_name(owner)
                    ),
                    evidence: "owner",
                    weight: 20,
                });
            }
        }
        if document.kind == "constructor" {
            if let Some(owner) = &document.owner {
                if let Some((_, owner_id)) = by_name.get(&normalize_lookup_key(&owner.primary)) {
                    relations.push(Relation {
                        source_id: document.id.clone(),
                        target_id: owner_id.clone(),
                        edge_kind: "constructs",
                        label: format!("constructs {}", display_name(owner)),
                        evidence: "structured",
                        weight: 15,
                    });
                }
            }
        }
        for type_name in document
            .type_refs
            .iter()
            .chain(document.return_types.iter())
        {
            if let Some((_, target_id)) = by_name.get(&normalize_lookup_key(type_name)) {
                relations.push(Relation {
                    source_id: document.id.clone(),
                    target_id: target_id.clone(),
                    edge_kind: if document.return_types.contains(type_name) {
                        "returns"
                    } else {
                        "has_type"
                    },
                    label: type_name.clone(),
                    evidence: "type_ref",
                    weight: 30,
                });
            }
        }
    }
    relations.sort_by(|left, right| {
        left.weight
            .cmp(&right.weight)
            .then_with(|| left.source_id.cmp(&right.source_id))
            .then_with(|| left.edge_kind.cmp(right.edge_kind))
            .then_with(|| left.target_id.cmp(&right.target_id))
    });
    relations.dedup_by(|left, right| {
        left.source_id == right.source_id
            && left.target_id == right.target_id
            && left.edge_kind == right.edge_kind
    });
    relations
}

fn document_lookup_keys(document: &SearchDocument) -> Vec<String> {
    let mut keys = vec![normalize_lookup_key(&document.name.primary)];
    if let Some(alias) = &document.name.alias {
        keys.push(normalize_lookup_key(alias));
    }
    keys.extend(document.relation_keys.iter().cloned());
    keys
}

fn document_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchDocument> {
    Ok(SearchDocument {
        id: row.get(0)?,
        kind: row.get(1)?,
        name: model::LocalizedName {
            primary: row.get(2)?,
            alias: row.get(3)?,
        },
        owner: optional_localized_name(row.get(4)?, row.get(5)?),
        signatures: split_lines(row.get(6)?),
        parameters: split_lines(row.get(7)?),
        type_refs: split_lines(row.get(8)?),
        return_types: split_lines(row.get(9)?),
        description: row.get(10)?,
        preview: row.get(11)?,
        relation_keys: Vec::new(),
        owner_relation_key: None,
    })
}

fn optional_localized_name(
    primary: Option<String>,
    alias: Option<String>,
) -> Option<model::LocalizedName> {
    primary.map(|primary| model::LocalizedName { primary, alias })
}

fn split_lines(value: String) -> Vec<String> {
    value
        .lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn document_id(
    kind: &str,
    owner: Option<&model::LocalizedName>,
    name: &model::LocalizedName,
    source: &model::SyntaxHelperSource,
) -> String {
    let owner = owner.map(display_name).unwrap_or_default();
    format!("{kind}:{owner}:{}:{}", display_name(name), source.html_path)
}

fn display_name(name: &model::LocalizedName) -> String {
    match &name.alias {
        Some(alias) => format!("{} ({alias})", name.primary),
        None => name.primary.clone(),
    }
}

fn kind_priority(kind: &str) -> i64 {
    match kind {
        "platform_type" => 10,
        "type_property" => 20,
        "type_method" => 30,
        "constructor" => 40,
        "global_method" => 50,
        "global_property" => 60,
        "module_event" => 70,
        "type_event" => 80,
        "unknown_event" => 90,
        "query_table" => 100,
        "query_table_field" => 110,
        "query_table_parameter" => 120,
        "enum" => 130,
        "enum_value" => 140,
        _ => 999,
    }
}

fn owner_member_key(owner: &str, member: &str) -> String {
    normalize_lookup_key(&format!("{owner}.{member}"))
}

fn normalize_lookup_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn searchable_name(value: &str) -> String {
    let mut output = String::new();
    let mut previous_lowercase = false;
    for character in value.chars() {
        if character.is_alphanumeric() {
            if previous_lowercase && character.is_uppercase() {
                output.push(' ');
            }
            output.extend(character.to_lowercase());
            previous_lowercase = character.is_lowercase();
        } else {
            output.push(' ');
            previous_lowercase = false;
        }
    }
    output
}

fn searchable_text(value: &str) -> String {
    value
        .split_whitespace()
        .map(searchable_name)
        .collect::<Vec<_>>()
        .join(" ")
}

fn fts_query(value: &str) -> String {
    let mut tokens = searchable_text(value)
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if tokens.iter().any(|token| token == "скд") {
        tokens.extend(
            [
                "система",
                "компоновки",
                "данных",
                "компоновка",
                "data",
                "composition",
            ]
            .into_iter()
            .map(ToOwned::to_owned),
        );
    }
    tokens.sort();
    tokens.dedup();
    tokens
        .into_iter()
        .filter(|token| !token.is_empty())
        .map(|token| format!("\"{}\"*", token.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn fuzzy_score(query: &str, document: &SearchDocument) -> usize {
    let mut candidates = vec![normalize_lookup_key(&document.name.primary)];
    if let Some(alias) = &document.name.alias {
        candidates.push(normalize_lookup_key(alias));
    }
    if let Some(owner) = &document.owner {
        candidates.push(owner_member_key(&owner.primary, &document.name.primary));
        if let Some(alias) = &document.name.alias {
            candidates.push(owner_member_key(&owner.primary, alias));
        }
    }
    candidates
        .iter()
        .map(|candidate| levenshtein(query, candidate))
        .min()
        .unwrap_or(usize::MAX)
}

fn fuzzy_threshold(query: &str) -> usize {
    match query.chars().count() {
        0..=8 => 2,
        9..=20 => 3,
        length => (length / 5).max(4),
    }
}

fn keyword_order(query: &str, document: &SearchDocument) -> usize {
    let tokens = searchable_text(query)
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let first = tokens.first().map(String::as_str).unwrap_or_default();
    let name = searchable_name(&document.name.primary);
    let alias = document
        .name
        .alias
        .as_deref()
        .map(searchable_name)
        .unwrap_or_default();
    if !first.is_empty() && name.starts_with(first) {
        0
    } else if !first.is_empty() && alias.starts_with(first) {
        1
    } else if tokens.iter().all(|token| name.contains(token)) {
        2
    } else if tokens.iter().all(|token| alias.contains(token)) {
        3
    } else {
        10
    }
}

fn temp_index_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("index.sqlite");
    path.with_file_name(format!("{file_name}.tmp.{}", std::process::id()))
}

fn remove_sqlite_sidecars(path: &Path) -> Result<(), SearchError> {
    for sidecar in [
        path.with_extension("sqlite-wal"),
        path.with_extension("sqlite-shm"),
    ] {
        match fs::remove_file(&sidecar) {
            Ok(()) => {}
            Err(source) if source.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(SearchError::Io {
                    path: sidecar,
                    source,
                });
            }
        }
    }
    Ok(())
}

struct WriterLock {
    path: PathBuf,
    _file: File,
}

impl WriterLock {
    fn acquire(index_path: &Path) -> Result<Self, SearchError> {
        let path = index_path.with_extension("lock");
        let started = Instant::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => return Ok(Self { path, _file: file }),
                Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
                    if started.elapsed() > Duration::from_secs(30) {
                        return Err(SearchError::WriterLockTimeout { path });
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(source) => {
                    return Err(SearchError::Io {
                        path: path.clone(),
                        source,
                    });
                }
            }
        }
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_supports_exact_keyword_fuzzy_and_related_queries() {
        let path = temp_path("query.sqlite");
        build_index(&path, &metadata(), &fixture_context()).expect("index must build");
        let index = SearchIndex::open_read_only(&path).expect("index must open read-only");

        let exact = index
            .get_by_name("DataCompositionFilter")
            .expect("exact lookup must work");
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].document.name.primary, "ОтборКомпоновкиДанных");

        let event = index
            .get_by_name("ПередЗаписью")
            .expect("event lookup must work");
        assert_eq!(event[0].document.kind, "type_event");

        let member = index
            .get_by_owner_member("НастройкиКомпоновкиДанных", "Отбор")
            .expect("owner/member lookup must work");
        assert_eq!(member[0].document.type_refs, ["ОтборКомпоновкиДанных"]);

        let keyword = index
            .search("отбор скд", SearchMode::Keywords, 10)
            .expect("keyword search must work");
        assert_eq!(keyword[0].document.name.primary, "ОтборКомпоновкиДанных");

        let fuzzy = index
            .search("ОтборКомпоновкиДаных", SearchMode::Fuzzy, 10)
            .expect("fuzzy search must work");
        assert_eq!(fuzzy[0].document.name.primary, "ОтборКомпоновкиДанных");

        let related = index
            .related_by_name("ОтборКомпоновкиДанных", 5, 20)
            .expect("related search must work");
        let names = related
            .iter()
            .map(|hit| hit.document.name.primary.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"Новый ОтборКомпоновкиДанных()"));
        assert!(names.contains(&"Элементы"));
        assert!(names.contains(&"Добавить"));
        assert!(names.contains(&"ЛевоеЗначение"));
    }

    #[test]
    fn query_connections_are_read_only_and_repeatable() {
        let path = temp_path("readonly.sqlite");
        build_index(&path, &metadata(), &fixture_context()).expect("index must build");
        let left = SearchIndex::open_read_only(&path).expect("first reader must open");
        let right = SearchIndex::open_read_only(&path).expect("second reader must open");
        assert_eq!(
            left.get_by_name("ОтборКомпоновкиДанных").unwrap(),
            right.get_by_name("ОтборКомпоновкиДанных").unwrap()
        );
        let write_result = left.connection.execute("DELETE FROM documents", []);
        assert!(
            write_result.is_err(),
            "read-only connection must reject writes"
        );
    }

    #[test]
    fn rebuild_replaces_previous_complete_index() {
        let path = temp_path("replace.sqlite");
        let mut context = fixture_context();
        build_index(&path, &metadata(), &context).expect("first index must build");
        fs::write(path.with_extension("sqlite-wal"), b"stale wal")
            .expect("stale wal sidecar must be writable");
        fs::write(path.with_extension("sqlite-shm"), b"stale shm")
            .expect("stale shm sidecar must be writable");
        context.platform_types[0].description = Some("updated description".to_string());
        build_index(&path, &metadata(), &context).expect("replacement index must build");
        assert!(!path.with_extension("sqlite-wal").exists());
        assert!(!path.with_extension("sqlite-shm").exists());
        let index = SearchIndex::open_read_only(&path).expect("index must open");
        let exact = index.get_by_name("ОтборКомпоновкиДанных").unwrap();
        assert_eq!(
            exact[0].document.description.as_deref(),
            Some("updated description")
        );
    }

    #[test]
    fn concurrent_writers_are_serialized_by_lock() {
        let path = temp_path("writers.sqlite");
        let left_path = path.clone();
        let right_path = path.clone();
        let left =
            std::thread::spawn(move || build_index(left_path, &metadata(), &fixture_context()));
        let right =
            std::thread::spawn(move || build_index(right_path, &metadata(), &fixture_context()));
        left.join()
            .expect("left writer must not panic")
            .expect("left writer must build");
        right
            .join()
            .expect("right writer must not panic")
            .expect("right writer must build");
        let index = SearchIndex::open_read_only(&path).expect("final index must open");
        assert!(index.document_count().expect("document count must work") > 0);
    }

    fn metadata() -> IndexMetadata {
        IndexMetadata {
            locale: "ru".to_string(),
            source_locale: "ru".to_string(),
            source_hbk: "fixture.hbk".to_string(),
            source_extraction_schema_version: 11,
        }
    }

    fn fixture_context() -> model::PlatformContext {
        model::PlatformContext {
            platform_types: vec![
                platform_type(
                    "ОтборКомпоновкиДанных",
                    Some("DataCompositionFilter"),
                    "Объект системы компоновки данных для настройки отбора.",
                ),
                platform_type(
                    "НастройкиКомпоновкиДанных",
                    Some("DataCompositionSettings"),
                    "Настройки системы компоновки данных.",
                ),
                platform_type(
                    "КоллекцияЭлементовОтбораКомпоновкиДанных",
                    Some("DataCompositionFilterItems"),
                    "Коллекция элементов отбора.",
                ),
                platform_type(
                    "ЭлементОтбораКомпоновкиДанных",
                    Some("DataCompositionFilterItem"),
                    "Элемент отбора.",
                ),
            ],
            type_properties: vec![
                type_property("БиблиотекаКартинок", "ОтборКомпоновкиДанных", "Картинка"),
                type_property(
                    "НастройкиКомпоновкиДанных",
                    "Отбор",
                    "ОтборКомпоновкиДанных",
                ),
                type_property(
                    "ОтборКомпоновкиДанных",
                    "Элементы",
                    "КоллекцияЭлементовОтбораКомпоновкиДанных",
                ),
                type_property(
                    "ЭлементОтбораКомпоновкиДанных",
                    "ЛевоеЗначение",
                    "Произвольный",
                ),
            ],
            type_methods: vec![type_method(
                "КоллекцияЭлементовОтбораКомпоновкиДанных",
                "Добавить",
                "ЭлементОтбораКомпоновкиДанных",
            )],
            constructors: vec![constructor(
                "ОтборКомпоновкиДанных",
                "Новый ОтборКомпоновкиДанных()",
            )],
            global_context_events: vec![type_event("ОтборКомпоновкиДанных", "ПередЗаписью")],
            ..model::PlatformContext::default()
        }
    }

    fn platform_type(primary: &str, alias: Option<&str>, description: &str) -> model::PlatformType {
        model::PlatformType {
            name: name(primary, alias),
            semantic: model::SemanticContext::default(),
            type_kind: model::PlatformTypeKind::Regular,
            object_kind: Some(model::PlatformObjectKind::RegularPlatformType),
            extends: Vec::new(),
            metadata_kind: None,
            template_parameters: Vec::new(),
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: Some(description.to_string()),
            facts: model::SectionFacts::default(),
            source: source(primary),
        }
    }

    fn type_property(owner: &str, primary: &str, type_ref: &str) -> model::PlatformProperty {
        model::PlatformProperty {
            owner: name(owner, None),
            name: name(primary, None),
            semantic: model::SemanticContext::default(),
            usage: None,
            type_refs: vec![model::TypeRef {
                name: type_ref.to_string(),
            }],
            description: Some(format!("{primary} description")),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn type_method(owner: &str, primary: &str, return_type: &str) -> model::PlatformMethod {
        model::PlatformMethod {
            owner: name(owner, None),
            name: name(primary, None),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: format!("{primary}()"),
                parameters: Vec::new(),
                variant: None,
            }],
            return_types: vec![model::TypeRef {
                name: return_type.to_string(),
            }],
            description: Some(format!("{primary} description")),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn constructor(owner: &str, signature: &str) -> model::Constructor {
        model::Constructor {
            owner: name(owner, None),
            name: name("По умолчанию", None),
            semantic: model::SemanticContext::default(),
            signatures: vec![model::Signature {
                text: signature.to_string(),
                parameters: Vec::new(),
                variant: None,
            }],
            description: None,
            facts: model::SectionFacts::default(),
            source: source(signature),
        }
    }

    fn type_event(owner: &str, primary: &str) -> model::GlobalContextEvent {
        model::GlobalContextEvent {
            name: name(primary, Some("BeforeWrite")),
            semantic: model::SemanticContext::new(
                model::BranchKind::PlatformObjects,
                model::RecordFamily::TypeEvent,
            )
            .with_owner_path(vec![name(owner, None)]),
            module: model::ModuleEventContext::default(),
            signatures: vec![model::Signature {
                text: format!("{primary}()"),
                parameters: Vec::new(),
                variant: None,
            }],
            description: Some("event description".to_string()),
            facts: model::SectionFacts::default(),
            source: source(&format!("{owner}.{primary}")),
        }
    }

    fn name(primary: &str, alias: Option<&str>) -> model::LocalizedName {
        model::LocalizedName {
            primary: primary.to_string(),
            alias: alias.map(ToOwned::to_owned),
        }
    }

    fn source(name: &str) -> model::SyntaxHelperSource {
        model::SyntaxHelperSource {
            hbk_path: PathBuf::from("fixture.hbk"),
            locale: "ru".to_string(),
            toc_path: Some(name.to_string()),
            html_path: format!("{name}.html"),
            page_title: name.to_string(),
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "v8-context-hbk-search-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        path
    }
}
