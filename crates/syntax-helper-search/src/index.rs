pub struct SearchIndex {
    path: PathBuf,
    connection: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredIndexMetadata {
    pub locale: String,
    pub source_locale: String,
    pub source_hbk: String,
    pub source_extraction_schema_version: u32,
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
        validate_schema_version(&connection, path)?;
        Ok(Self {
            path: path.to_path_buf(),
            connection,
        })
    }

    pub fn metadata(&self) -> Result<StoredIndexMetadata, SearchError> {
        let source_extraction_schema_version =
            self.metadata_value("source_extraction_schema_version")?;
        Ok(StoredIndexMetadata {
            locale: self.metadata_value("locale")?,
            source_locale: self.metadata_value("source_locale")?,
            source_hbk: self.metadata_value("source_hbk")?,
            source_extraction_schema_version: source_extraction_schema_version.parse().map_err(
                |source| {
                    SearchError::metadata_parse(
                        self.path.clone(),
                        "source_extraction_schema_version",
                        source_extraction_schema_version.clone(),
                        source,
                    )
                },
            )?,
        })
    }

    pub fn get_by_name(&self, name: &str) -> Result<Vec<SearchHit>, SearchError> {
        let key = normalize_lookup_key(name);
        self.get_by_key(&key)
    }

    pub fn get_by_id(&self, id: &str) -> Result<Option<SearchHit>, SearchError> {
        self.document(id)
            .map(|document| document.map(|document| SearchHit { document, score: 0 }))
    }

    pub fn get_by_owner_member(
        &self,
        owner: &str,
        member: &str,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let key = owner_member_key(owner, member);
        self.get_by_key(&key)
    }

    pub fn documents_by_kind(
        &self,
        kind: SearchDocumentKind,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT id, kind, name_primary, name_alias, owner_primary,
                        owner_alias, signature_text, description, availability_contexts,
                        available_since
                 FROM documents
                 WHERE kind = ?1
                 ORDER BY kind_priority, name_primary, id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([kind.as_str()], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    pub fn type_identity_by_id(&self, type_id: &str) -> Result<Option<SearchHit>, SearchError> {
        let Some(document_id) = self.type_identity_document_id(type_id)? else {
            return Ok(None);
        };
        self.get_by_id(&document_id)
    }

    pub fn type_identities_by_name(&self, name: &str) -> Result<Vec<SearchHit>, SearchError> {
        self.type_identities_by_lookup_key(&normalize_lookup_key(name), TypeIdentityLookup::Primary)
    }

    pub fn type_identities_by_alias(&self, alias: &str) -> Result<Vec<SearchHit>, SearchError> {
        self.type_identities_by_lookup_key(&normalize_lookup_key(alias), TypeIdentityLookup::Alias)
    }

    pub fn type_template_by_key(
        &self,
        kind: &model::PlatformTypeTemplateKey,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary,
                        d.owner_alias, d.signature_text, d.description, d.availability_contexts,
                        d.available_since
                 FROM type_templates t
                 JOIN documents d ON d.id = t.document_id
                 WHERE t.template_family = ?1
                   AND t.template_variant = ?2
                 ORDER BY d.name_primary, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![kind.family, kind.variant], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    pub fn members_by_type_id(&self, type_id: &str) -> Result<Vec<SearchHit>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM members m \
                 JOIN documents d ON d.id = m.document_id \
                 WHERE m.owner_type_id = ?1 \
                 ORDER BY d.kind_priority, m.name_primary, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([type_id], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    pub fn member_by_owner_type_id(
        &self,
        owner_type_id: &str,
        member: &str,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let normalized_member = normalize_lookup_key(member);
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM document_names n \
                 JOIN members m INDEXED BY members_document_owner_idx \
                   ON m.document_id = n.document_id \
                  AND m.owner_type_id = ?1 \
                 JOIN documents d ON d.id = m.document_id \
                 WHERE n.key = ?2 \
                   AND n.key_kind IN ('primary', 'alias') \
                 ORDER BY d.kind_priority, m.name_primary, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![owner_type_id, normalized_member], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    pub fn constructors_by_name(&self, name: &str) -> Result<Vec<SearchHit>, SearchError> {
        let Some(root) = self.root_by_name(name)? else {
            return Ok(Vec::new());
        };
        self.constructors_by_type_id(&root.document.id)
    }

    pub fn constructors_by_type_id(&self, type_id: &str) -> Result<Vec<SearchHit>, SearchError> {
        self.owned_documents_by_kind(type_id, "constructor", 100)
    }

    pub fn callable_by_id(&self, callable_id: &str) -> Result<Option<SearchHit>, SearchError> {
        if !self.callable_exists(callable_id)? {
            return Ok(None);
        }
        self.get_by_id(callable_id)
    }

    pub fn callable_by_owner_type_id(
        &self,
        owner_type_id: &str,
        callable: &str,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let normalized_callable = normalize_lookup_key(callable);
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM document_names n \
                 JOIN callables c INDEXED BY callables_document_owner_idx \
                   ON c.document_id = n.document_id \
                  AND c.owner_type_id = ?1 \
                 JOIN documents d ON d.id = c.document_id \
                 WHERE n.key = ?2 \
                   AND n.key_kind IN ('primary', 'alias') \
                 ORDER BY d.kind_priority, d.name_primary, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![owner_type_id, normalized_callable], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
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
        let Some(root) = self.root_by_name(name)? else {
            return Ok(Vec::new());
        };
        self.related(&root.document.id, max_depth.min(5), limit)
    }

    pub fn related_by_id(
        &self,
        id: &str,
        max_depth: u32,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        if self.document(id)?.is_none() {
            return Ok(Vec::new());
        }
        self.related(id, max_depth.min(5), limit)
    }

    pub fn related_by_id_and_edge(
        &self,
        id: &str,
        edge_kind: &str,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        let Some(source_document) = self.document(id)? else {
            return Ok(Vec::new());
        };
        if matches!(edge_kind, "has_type" | "returns" | "constructs") {
            let hits = self.related_type_refs_by_id_and_edge(id, edge_kind, limit)?;
            if !hits.is_empty() {
                return Ok(hits);
            }
            if !source_document.kind.is_language()
                && !matches!(
                    source_document.kind,
                    SearchDocumentKind::QueryTableField | SearchDocumentKind::QueryTableParameter
                )
            {
                return Ok(Vec::new());
            }
        }
        let mut hits = Vec::new();
        for edge in self.edges_by_kind(id, edge_kind)? {
            let Some(document) = self.document(&edge.to)? else {
                continue;
            };
            hits.push(RelatedHit {
                document,
                depth: 1,
                via: vec![edge],
            });
            if hits.len() >= limit {
                break;
            }
        }
        Ok(hits)
    }

    fn related_type_refs_by_id_and_edge(
        &self,
        id: &str,
        edge_kind: &str,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        let ref_kind = edge_ref_kind(edge_kind);
        let mut statement = self
            .connection
            .prepare(
                "SELECT target_type_id, target_type_name
                 FROM type_refs
                 WHERE source_document_id = ?1
                   AND ref_kind = ?2
                   AND target_type_id IS NOT NULL
                 ORDER BY ordinal, target_type_id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![id, ref_kind], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|source| self.sqlite(source))?;
        let rows = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        let mut hits = Vec::new();
        for (target_id, target_name) in rows {
            let Some(document) = self.document(&target_id)? else {
                continue;
            };
            hits.push(RelatedHit {
                document,
                depth: 1,
                via: vec![RelationStep {
                    from: id.to_string(),
                    to: target_id,
                    edge_kind: edge_kind.to_string(),
                    label: target_name,
                    evidence: "type_ref".to_string(),
                }],
            });
            if hits.len() >= limit {
                break;
            }
        }
        Ok(hits)
    }

    pub fn related_by_owner_member(
        &self,
        owner: &str,
        member: &str,
        max_depth: u32,
        limit: usize,
    ) -> Result<Vec<RelatedHit>, SearchError> {
        let roots = self.get_by_owner_member(owner, member)?;
        let Some(root) = roots.first() else {
            return Ok(Vec::new());
        };
        if roots.len() > 1 {
            return Err(SearchError::AmbiguousLookup {
                name: format!("{owner}.{member}"),
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

    pub fn owner_type_id_for_document(
        &self,
        document_id: &str,
    ) -> Result<Option<String>, SearchError> {
        self.connection
            .query_row(
                "SELECT owner_type_id FROM members WHERE document_id = ?1
                 UNION
                 SELECT owner_type_id FROM callables WHERE document_id = ?1 AND owner_type_id IS NOT NULL
                 LIMIT 1",
                [document_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|source| self.sqlite(source))
    }

    pub fn target_type_ids_for_document(
        &self,
        document_id: &str,
    ) -> Result<Vec<String>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT target_type_id
                 FROM type_refs
                 WHERE source_document_id = ?1 AND target_type_id IS NOT NULL
                 ORDER BY target_type_id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([document_id], |row| row.get(0))
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    pub fn type_reference_gap_report(
        &self,
        top_limit: usize,
    ) -> Result<TypeReferenceGapReport, SearchError> {
        let mut role_counts = BTreeMap::<String, TypeReferenceCounts>::new();
        let mut total_counts = TypeReferenceCounts::default();
        let mut unresolved = BTreeMap::<(String, String), GapAccumulator>::new();
        let mut ambiguous = BTreeMap::<(String, String), GapAccumulator>::new();

        let mut statement = self
            .connection
            .prepare(
                "SELECT r.source_document_id, r.ref_kind, r.target_type_name,
                        r.target_resolution_status, r.target_candidate_type_ids,
                        r.template_binding_kind,
                        d.kind, d.name_primary, d.name_alias, d.owner_primary, d.owner_alias
                 FROM type_refs r
                 JOIN documents d ON d.id = r.source_document_id
                 ORDER BY r.ref_kind, r.target_type_name, r.source_document_id, r.ordinal",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([], |row| {
                let kind_value: String = row.get(6)?;
                let kind = SearchDocumentKind::from_storage(&kind_value)
                    .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
                let source_owner_primary: Option<String> = row.get(9)?;
                let source_owner_alias: Option<String> = row.get(10)?;
                Ok(TypeReferenceMeasurementRow {
                    source_document_id: row.get(0)?,
                    role: row.get(1)?,
                    target_type_name: row.get(2)?,
                    status: type_reference_status_from_storage(row.get::<_, String>(3)?)?,
                    candidate_type_ids: row
                        .get::<_, Option<String>>(4)?
                        .map(|value| value.lines().map(str::to_string).collect())
                        .unwrap_or_default(),
                    has_template_binding: row.get::<_, Option<String>>(5)?.is_some(),
                    source_kind: kind,
                    source_name: model::LocalizedName {
                        primary: row.get(7)?,
                        alias: row.get(8)?,
                    },
                    source_owner: source_owner_primary.map(|primary| model::LocalizedName {
                        primary,
                        alias: source_owner_alias,
                    }),
                })
            })
            .map_err(|source| self.sqlite(source))?;

        for row in rows {
            let row = row.map_err(|source| self.sqlite(source))?;
            let status = row.status;
            total_counts.add(status, row.has_template_binding);
            role_counts
                .entry(row.role.clone())
                .or_default()
                .add(status, row.has_template_binding);

            match status {
                TypeReferenceStatus::Unresolved => accumulate_gap(&mut unresolved, row),
                TypeReferenceStatus::Ambiguous => accumulate_gap(&mut ambiguous, row),
                TypeReferenceStatus::Resolved => {}
            }
        }

        Ok(TypeReferenceGapReport {
            total: total_counts.total,
            resolved: total_counts.resolved,
            unresolved: total_counts.unresolved,
            ambiguous: total_counts.ambiguous,
            template_bindings: total_counts.template_bindings,
            roles: role_counts
                .into_iter()
                .map(|(role, counts)| TypeReferenceRoleReport {
                    role,
                    total: counts.total,
                    resolved: counts.resolved,
                    unresolved: counts.unresolved,
                    ambiguous: counts.ambiguous,
                    template_bindings: counts.template_bindings,
                })
                .collect(),
            top_unresolved: top_gaps(unresolved, top_limit),
            top_ambiguous: top_gaps(ambiguous, top_limit),
        })
    }

    fn get_by_key(&self, key: &str) -> Result<Vec<SearchHit>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
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
        self.hydrate_hits(hits)
    }

    fn type_identity_document_id(&self, type_id: &str) -> Result<Option<String>, SearchError> {
        self.connection
            .query_row(
                "SELECT document_id FROM type_identities WHERE type_id = ?1",
                [type_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|source| self.sqlite(source))
    }

    fn type_identities_by_lookup_key(
        &self,
        key: &str,
        lookup: TypeIdentityLookup,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let key_kind = match lookup {
            TypeIdentityLookup::Primary => "primary",
            TypeIdentityLookup::Alias => "alias",
        };
        let mut statement = self
            .connection
            .prepare(
                "SELECT DISTINCT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM document_names n \
                 JOIN type_identities t ON t.document_id = n.document_id \
                 JOIN documents d ON d.id = t.document_id \
                 WHERE n.key = ?1 \
                   AND n.key_kind = ?2 \
                 ORDER BY d.kind_priority, d.id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([key, key_kind], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
    }

    fn callable_exists(&self, callable_id: &str) -> Result<bool, SearchError> {
        self.connection
            .query_row(
                "SELECT 1 FROM callables WHERE callable_id = ?1",
                [callable_id],
                |_| Ok(()),
            )
            .optional()
            .map(|value| value.is_some())
            .map_err(|source| self.sqlite(source))
    }

    fn root_by_name(&self, name: &str) -> Result<Option<SearchHit>, SearchError> {
        let roots = self.get_by_name(name)?;
        let Some(root) = roots.first() else {
            return Ok(None);
        };
        if roots.len() > 1 {
            return Err(SearchError::AmbiguousLookup {
                name: name.to_string(),
                matches: roots.len(),
            });
        }
        Ok(Some(root.clone()))
    }

    fn owned_documents_by_kind(
        &self,
        owner_id: &str,
        kind: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT d.id, d.kind, d.name_primary, d.name_alias, d.owner_primary, \
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
                 FROM relations r \
                 JOIN documents d ON d.id = r.target_id \
                 WHERE r.source_id = ?1 AND r.edge_kind = 'owns' AND d.kind = ?2 \
                 ORDER BY d.kind_priority, d.id \
                 LIMIT ?3",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![owner_id, kind, limit as i64], |row| {
                Ok(SearchHit {
                    document: document_from_row(row)?,
                    score: 0,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
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
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since, \
                 CAST(bm25(document_fts) * 1000000 AS INTEGER) \
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
                    score: row.get(10)?,
                })
            })
            .map_err(|source| self.sqlite(source))?;
        let mut hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        hits = self.hydrate_hits(hits)?;
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
                .then_with(|| {
                    left.document
                        .kind
                        .as_str()
                        .cmp(right.document.kind.as_str())
                })
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
                 d.owner_alias, d.signature_text, d.description, d.availability_contexts, d.available_since \
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
        let hits = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        self.hydrate_hits(hits)
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

    fn edges_by_kind(
        &self,
        document_id: &str,
        edge_kind: &str,
    ) -> Result<Vec<RelationStep>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT source_id, target_id, edge_kind, label, evidence \
                 FROM relations \
                 WHERE source_id = ?1 AND edge_kind = ?2 \
                 ORDER BY weight, target_id",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![document_id, edge_kind], |row| {
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
        let document = self
            .connection
            .query_row(
                "SELECT id, kind, name_primary, name_alias, owner_primary, owner_alias, \
                 signature_text, description, availability_contexts, available_since \
                 FROM documents WHERE id = ?1",
                [id],
                document_from_row,
            )
            .optional()
            .map_err(|source| self.sqlite(source))?;
        document
            .map(|document| self.hydrate_document(document))
            .transpose()
    }

    fn hydrate_hits(&self, hits: Vec<SearchHit>) -> Result<Vec<SearchHit>, SearchError> {
        hits.into_iter()
            .map(|hit| {
                self.hydrate_document(hit.document)
                    .map(|document| SearchHit { document, ..hit })
            })
            .collect()
    }

    fn hydrate_document(
        &self,
        mut document: SearchDocument,
    ) -> Result<SearchDocument, SearchError> {
        let mut type_refs = Vec::new();
        let mut type_ref_facts = Vec::new();
        for ref_kind in document.kind.public_type_ref_kinds() {
            let facts = self.type_ref_facts(&document.id, ref_kind)?;
            type_refs.extend(facts.iter().map(|type_ref| type_ref.name.clone()));
            type_ref_facts.extend(facts);
        }
        document.type_refs = type_refs;
        document.type_ref_facts = type_ref_facts;
        let return_ref_kind = if document.kind == SearchDocumentKind::Constructor {
            "constructor_result"
        } else {
            "return_type"
        };
        document.return_type_facts = self.type_ref_facts(&document.id, return_ref_kind)?;
        document.return_types = document
            .return_type_facts
            .iter()
            .map(|type_ref| type_ref.name.clone())
            .collect();
        document.signatures = self.signatures_for(&document)?;
        document.parameter_terms = self.parameter_terms_for(&document)?;
        if let Some((
            metadata_kind,
            template_parameters,
            type_template_key,
            type_template_classification_diagnostic,
        )) = self.type_template_for(&document.id)?
        {
            document.metadata_kind = Some(metadata_kind);
            document.template_parameters = template_parameters;
            document.type_template_key = type_template_key;
            document.type_template_classification_diagnostic =
                type_template_classification_diagnostic;
        }
        if let Some(metadata) = self.document_metadata_for(&document.id)? {
            document.owner_path = metadata.owner_path;
            document.note = metadata.note;
            document.default_value = metadata.default_value;
            document.query_syntax = metadata.query_syntax;
            document.query_identifier = metadata.query_identifier;
            document.query_table_role = metadata.query_table_role;
            if !metadata.template_parameters.is_empty() {
                document.template_parameters = metadata.template_parameters;
            }
            document.source = metadata.source;
        }
        Ok(document)
    }

    fn document_metadata_for(
        &self,
        document_id: &str,
    ) -> Result<Option<DocumentMetadataRow>, SearchError> {
        self.connection
            .query_row(
                "SELECT owner_path, note, default_value, query_syntax_primary, query_syntax_alias,
                        query_identifier, query_table_role, template_parameters, source_hbk_path,
                        source_locale, source_toc_path, source_html_path, source_page_title
                 FROM document_metadata WHERE document_id = ?1",
                [document_id],
                |row| {
                    let source_hbk_path: Option<String> = row.get(8)?;
                    let source_locale: Option<String> = row.get(9)?;
                    let source_html_path: Option<String> = row.get(11)?;
                    let source_page_title: Option<String> = row.get(12)?;
                    Ok(DocumentMetadataRow {
                        owner_path: split_localized_names(row.get::<_, String>(0)?),
                        note: row.get(1)?,
                        default_value: row.get(2)?,
                        query_syntax: optional_localized_name(row.get(3)?, row.get(4)?),
                        query_identifier: row.get(5)?,
                        query_table_role: row
                            .get::<_, Option<String>>(6)?
                            .as_deref()
                            .and_then(query_table_role_from_code),
                        template_parameters: split_lines(row.get::<_, String>(7)?),
                        source: match (source_hbk_path, source_locale, source_html_path, source_page_title) {
                            (Some(hbk_path), Some(locale), Some(html_path), Some(page_title)) => {
                                Some(model::SyntaxHelperSource {
                                    hbk_path: PathBuf::from(hbk_path),
                                    locale,
                                    toc_path: row.get(10)?,
                                    html_path,
                                    page_title,
                                })
                            }
                            _ => None,
                        },
                    })
                },
            )
            .optional()
            .map_err(|source| self.sqlite(source))
    }

    fn type_template_for(&self, document_id: &str) -> Result<Option<TypeTemplateRow>, SearchError> {
        self.connection
            .query_row(
                "SELECT metadata_kind, template_parameters, template_family, template_variant,
                        template_classification_diagnostic
                 FROM type_templates WHERE document_id = ?1",
                [document_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        split_lines(row.get::<_, String>(1)?),
                        type_template_key_from_codes(
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                        ),
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|source| self.sqlite(source))
    }

    fn type_ref_facts(
        &self,
        document_id: &str,
        ref_kind: &str,
    ) -> Result<Vec<SearchTypeRef>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT target_type_name, target_type_id, target_resolution_status,
                        target_candidate_type_ids, type_template_family, type_template_variant,
                        template_binding_kind,
                        template_binding_owner_parameter_index,
                        template_binding_target_parameter_index,
                        template_binding_arguments
                 FROM type_refs
                 WHERE source_document_id = ?1 AND ref_kind = ?2
                   AND source_signature_id IS NULL
                 ORDER BY source_signature_ordinal, source_parameter_ordinal, ordinal, target_type_name",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(params![document_id, ref_kind], search_type_ref_from_row)
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    fn signatures_for(
        &self,
        document: &SearchDocument,
    ) -> Result<Vec<SearchSignature>, SearchError> {
        let signature_texts = document.signature_text_lines();
        let mut statement = self
            .connection
            .prepare(
                "SELECT signature_id, ordinal, title, description
                 FROM signatures
                 WHERE callable_id = ?1
                 ORDER BY ordinal",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([document.id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|source| self.sqlite(source))?;
        let rows = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        let mut signatures = Vec::new();
        for (signature_id, ordinal, title, description) in rows {
            let return_type_facts = self.signature_return_type_facts(&signature_id)?;
            let return_types = return_type_facts
                .iter()
                .map(|type_ref| type_ref.name.clone())
                .collect();
            signatures.push(SearchSignature {
                text: signature_texts
                    .get(ordinal as usize)
                    .cloned()
                    .unwrap_or_default(),
                parameters: self.parameters_for(&signature_id)?,
                return_types,
                return_type_facts,
                title,
                description,
            });
        }
        if signatures.is_empty() {
            signatures.extend(signature_texts.into_iter().map(|text| SearchSignature {
                text,
                parameters: Vec::new(),
                return_types: Vec::new(),
                return_type_facts: Vec::new(),
                title: None,
                description: None,
            }));
        }
        Ok(signatures)
    }

    fn parameters_for(&self, signature_id: &str) -> Result<Vec<SearchParameter>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT ordinal, name, required, description
                 FROM parameters
                 WHERE signature_id = ?1
                 ORDER BY ordinal",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([signature_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|source| self.sqlite(source))?;
        let rows = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))?;
        rows.into_iter()
            .map(|(ordinal, name, required, description)| {
                self.parameter_type_facts(signature_id, ordinal)
                    .map(|type_ref_facts| SearchParameter {
                        name,
                        required,
                        type_refs: type_ref_facts
                            .iter()
                            .map(|type_ref| type_ref.name.clone())
                            .collect(),
                        type_ref_facts,
                        description,
                    })
            })
            .collect()
    }

    fn parameter_type_facts(
        &self,
        signature_id: &str,
        parameter_ordinal: i64,
    ) -> Result<Vec<SearchTypeRef>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT target_type_name, target_type_id, target_resolution_status,
                        target_candidate_type_ids, type_template_family, type_template_variant,
                        template_binding_kind,
                        template_binding_owner_parameter_index,
                        template_binding_target_parameter_index,
                        template_binding_arguments
                 FROM type_refs
                 WHERE source_signature_id = ?1
                   AND source_parameter_ordinal = ?2
                   AND ref_kind = 'parameter_type'
                 ORDER BY ordinal, target_type_name",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map(
                params![signature_id, parameter_ordinal],
                search_type_ref_from_row,
            )
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    fn signature_return_type_facts(
        &self,
        signature_id: &str,
    ) -> Result<Vec<SearchTypeRef>, SearchError> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT target_type_name, target_type_id, target_resolution_status,
                        target_candidate_type_ids, type_template_family, type_template_variant,
                        template_binding_kind,
                        template_binding_owner_parameter_index,
                        template_binding_target_parameter_index,
                        template_binding_arguments
                 FROM type_refs
                 WHERE source_signature_id = ?1
                   AND source_parameter_ordinal IS NULL
                   AND ref_kind = 'return_type'
                 ORDER BY ordinal, target_type_name",
            )
            .map_err(|source| self.sqlite(source))?;
        let rows = statement
            .query_map([signature_id], search_type_ref_from_row)
            .map_err(|source| self.sqlite(source))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|source| self.sqlite(source))
    }

    fn parameter_terms_for(&self, document: &SearchDocument) -> Result<Vec<String>, SearchError> {
        let mut terms = BTreeSet::new();
        for signature in &document.signatures {
            for parameter in &signature.parameters {
                terms.insert(parameter.name.clone());
                terms.extend(parameter.type_refs.iter().cloned());
            }
        }
        Ok(terms.into_iter().collect())
    }

    fn sqlite(&self, source: rusqlite::Error) -> SearchError {
        SearchError::Sqlite {
            path: self.path.clone(),
            source,
        }
    }

    fn metadata_value(&self, key: &'static str) -> Result<String, SearchError> {
        self.connection
            .query_row(
                "SELECT value FROM meta WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .map_err(|source| match source {
                rusqlite::Error::QueryReturnedNoRows => SearchError::MissingMetadata {
                    path: self.path.clone(),
                    key,
                },
                source => self.sqlite(source),
            })
    }
}

fn validate_schema_version(connection: &Connection, path: &Path) -> Result<(), SearchError> {
    let actual = connection
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(|source| match source {
            rusqlite::Error::QueryReturnedNoRows => SearchError::MissingMetadata {
                path: path.to_path_buf(),
                key: "schema_version",
            },
            source => SearchError::Sqlite {
                path: path.to_path_buf(),
                source,
            },
        })?;
    if actual == INDEX_SCHEMA_VERSION.to_string() {
        Ok(())
    } else {
        Err(SearchError::UnsupportedSchemaVersion {
            path: path.to_path_buf(),
            expected: INDEX_SCHEMA_VERSION,
            actual,
        })
    }
}
