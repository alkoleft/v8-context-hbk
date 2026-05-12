fn syntax_type_ref_gaps(
    index: Option<PathBuf>,
    limit: usize,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let index = SearchIndex::open_read_only(resolve_index_path(index))?;
    let report = index.type_reference_gap_report(limit)?;
    match format {
        OutputFormat::Text => print_type_reference_gap_report_text(&report),
        OutputFormat::Json => print_type_reference_gap_report_json(&report),
    }
}

#[allow(clippy::enum_variant_names)]
enum RootLookup {
    ById(String),
    ByName(String),
    ByOwnerMember(String, String),
}

struct ClassifiedGetQuery<'a> {
    value: Value,
    lookup: GetLookup<'a>,
}

enum GetLookup<'a> {
    TypeIdentityById(&'a str),
    TypeIdentityByName(&'a str),
    TypeIdentityByAlias(&'a str),
    DocumentById(&'a str),
    ExactName(&'a str),
    MemberList(&'a str),
    OwnerTypeMember {
        owner_type_id: &'a str,
        member: &'a str,
    },
    OwnerMember {
        owner: &'a str,
        member: &'a str,
    },
    CallableById(&'a str),
    CallableByOwnerType {
        owner_type_id: &'a str,
        callable: &'a str,
    },
    Unsupported(&'static str),
}

fn classify_get_query(args: &GetArgs) -> ClassifiedGetQuery<'_> {
    let (value, lookup) = match (
        args.kind.as_deref(),
        args.id.as_deref(),
        args.name.as_deref(),
        args.alias.as_deref(),
        args.owner.as_deref(),
        args.owner_type_id.as_deref(),
        args.member.as_deref(),
        args.members_of.as_deref(),
        args.callable_id.as_deref(),
        args.callable.as_deref(),
    ) {
        (Some("platform_type"), Some(id), None, None, None, None, None, None, None, None) => (
            json!({ "kind": "type_identity", "id": id }),
            GetLookup::TypeIdentityById(id),
        ),
        (Some("platform_type"), None, Some(name), None, None, None, None, None, None, None) => (
            json!({ "kind": "type_identity", "name": name }),
            GetLookup::TypeIdentityByName(name),
        ),
        (Some("platform_type"), None, None, Some(alias), None, None, None, None, None, None) => (
            json!({ "kind": "type_identity", "alias": alias }),
            GetLookup::TypeIdentityByAlias(alias),
        ),
        (Some(_), _, _, _, _, _, _, _, _, _) => (
            json!({ "kind": "invalid" }),
            GetLookup::Unsupported(
                "syntax get --kind currently supports only platform_type with exactly one of --id, --name or --alias",
            ),
        ),
        (None, Some(id), None, None, None, None, None, None, None, None) => (
            json!({ "kind": "document_id", "id": id }),
            GetLookup::DocumentById(id),
        ),
        (None, None, Some(name), None, None, None, None, None, None, None) => (
            json!({ "kind": "exact_name", "name": name }),
            GetLookup::ExactName(name),
        ),
        (None, None, None, None, None, None, None, Some(type_id), None, None) => (
            json!({ "kind": "member_list", "owner_type_id": type_id }),
            GetLookup::MemberList(type_id),
        ),
        (None, None, None, None, None, Some(owner_type_id), Some(member), None, None, None) => (
            json!({ "kind": "owner_type_member", "owner_type_id": owner_type_id, "name": member }),
            GetLookup::OwnerTypeMember {
                owner_type_id,
                member,
            },
        ),
        (None, None, None, None, Some(owner), None, Some(member), None, None, None) => (
            json!({ "kind": "owner_member", "owner": owner, "member": member }),
            GetLookup::OwnerMember { owner, member },
        ),
        (None, None, None, None, None, None, None, None, Some(callable_id), None) => (
            json!({ "kind": "callable_overloads", "id": callable_id }),
            GetLookup::CallableById(callable_id),
        ),
        (None, None, None, None, None, Some(owner_type_id), None, None, None, Some(callable)) => (
            json!({ "kind": "callable_overloads", "owner_type_id": owner_type_id, "name": callable }),
            GetLookup::CallableByOwnerType {
                owner_type_id,
                callable,
            },
        ),
        _ => (
            json!({ "kind": "invalid" }),
            GetLookup::Unsupported(
                "syntax get requires exactly one root: --id, --name, --kind platform_type with --id/--name/--alias, --members-of, --owner-type-id with --member/--callable, --callable-id, or both --owner and --member",
            ),
        ),
    };
    ClassifiedGetQuery { value, lookup }
}

enum GetLookupResult {
    Hits(Vec<SearchHit>),
    NotFound,
    Unsupported(&'static str),
}

fn get_lookup(
    index: &SearchIndex,
    lookup: GetLookup<'_>,
) -> Result<GetLookupResult, Box<dyn std::error::Error>> {
    let result = match lookup {
        GetLookup::TypeIdentityById(id) => index
            .type_identity_by_id(id)?
            .map_or(GetLookupResult::NotFound, |hit| {
                GetLookupResult::Hits(vec![hit])
            }),
        GetLookup::TypeIdentityByName(name) => {
            GetLookupResult::Hits(index.type_identities_by_name(name)?)
        }
        GetLookup::TypeIdentityByAlias(alias) => {
            GetLookupResult::Hits(index.type_identities_by_alias(alias)?)
        }
        GetLookup::DocumentById(id) => index
            .get_by_id(id)?
            .map_or(GetLookupResult::NotFound, |hit| {
                GetLookupResult::Hits(vec![hit])
            }),
        GetLookup::ExactName(name) => GetLookupResult::Hits(index.get_by_name(name)?),
        GetLookup::MemberList(type_id) => {
            if !type_id.starts_with("platform_type:") {
                return Ok(GetLookupResult::Unsupported(
                    "syntax get --members-of requires an exact platform_type provider id",
                ));
            }
            if index.type_identity_by_id(type_id)?.is_none() {
                GetLookupResult::NotFound
            } else {
                GetLookupResult::Hits(index.members_by_type_id(type_id)?)
            }
        }
        GetLookup::OwnerTypeMember {
            owner_type_id,
            member,
        } => {
            if index.type_identity_by_id(owner_type_id)?.is_none() {
                GetLookupResult::NotFound
            } else {
                GetLookupResult::Hits(index.member_by_owner_type_id(owner_type_id, member)?)
            }
        }
        GetLookup::OwnerMember { owner, member } => {
            let roots = owner_member_roots(index, owner, member)?;
            if roots.is_empty() {
                GetLookupResult::NotFound
            } else {
                GetLookupResult::Hits(roots)
            }
        }
        GetLookup::CallableById(callable_id) => index
            .callable_by_id(callable_id)?
            .map_or(GetLookupResult::NotFound, |hit| {
                GetLookupResult::Hits(vec![hit])
            }),
        GetLookup::CallableByOwnerType {
            owner_type_id,
            callable,
        } => {
            if index.type_identity_by_id(owner_type_id)?.is_none() {
                GetLookupResult::NotFound
            } else {
                GetLookupResult::Hits(index.callable_by_owner_type_id(owner_type_id, callable)?)
            }
        }
        GetLookup::Unsupported(message) => GetLookupResult::Unsupported(message),
    };
    Ok(result)
}

fn type_identity_candidates(
    index: &SearchIndex,
    name: &str,
) -> Result<Vec<SearchHit>, Box<dyn std::error::Error>> {
    let mut candidates = index.type_identities_by_name(name)?;
    for alias_hit in index.type_identities_by_alias(name)? {
        if !candidates
            .iter()
            .any(|hit| hit.document.id == alias_hit.document.id)
        {
            candidates.push(alias_hit);
        }
    }
    candidates.sort_by(|left, right| left.document.id.cmp(&right.document.id));
    Ok(candidates)
}

fn owner_member_roots(
    index: &SearchIndex,
    owner: &str,
    member: &str,
) -> Result<Vec<SearchHit>, Box<dyn std::error::Error>> {
    let owner_candidates = type_identity_candidates(index, owner)?;
    if owner_candidates.len() > 1 {
        return Ok(owner_candidates);
    }
    if let Some(owner_type) = owner_candidates.first() {
        return Ok(index.member_by_owner_type_id(&owner_type.document.id, member)?);
    }
    Ok(index.get_by_owner_member(owner, member)?)
}

fn is_supported_type_graph_root_kind(kind: SearchDocumentKind) -> bool {
    matches!(
        kind,
        SearchDocumentKind::PlatformType
            | SearchDocumentKind::TypeProperty
            | SearchDocumentKind::TypeMethod
            | SearchDocumentKind::Constructor
            | SearchDocumentKind::GlobalMethod
            | SearchDocumentKind::ModuleEvent
            | SearchDocumentKind::TypeEvent
    )
}

#[allow(clippy::too_many_arguments)]
fn related_query_value(
    id: Option<&str>,
    name: Option<&str>,
    owner: Option<&str>,
    member: Option<&str>,
    depth: u32,
    edge: Option<&str>,
    limit: Option<usize>,
    compact: bool,
    graph: bool,
) -> Value {
    let root = match (id, name, owner, member) {
        (Some(id), None, None, None) => json!({ "id": id }),
        (None, Some(name), None, None) => json!({ "name": name }),
        (None, None, Some(owner), Some(member)) => json!({ "owner": owner, "member": member }),
        _ => json!({ "invalid": true }),
    };
    let mut query = json!({
        "kind": if graph { "type_graph" } else { related_query_kind(edge) },
        "root": root,
        "depth": depth.min(5)
    });
    if let Some(edge) = edge {
        query["edge"] = json!(edge);
    }
    if let Some(limit) = limit {
        query["limit"] = json!(limit);
    }
    if compact {
        query["output"] = json!("compact");
    }
    if graph {
        query["output"] = json!("graph");
    }
    query
}

fn search_query_value(query: &str, mode: SearchMode, limit: Option<usize>) -> Value {
    let mut value = json!({
        "kind": "search",
        "mode": search_mode_name(mode),
        "text": query,
    });
    if let Some(limit) = limit {
        value["limit"] = json!(limit);
    }
    value
}

fn search_mode_name(mode: SearchMode) -> &'static str {
    match mode {
        SearchMode::Keywords => "keywords",
        SearchMode::Fuzzy => "fuzzy",
    }
}

fn is_supported_edge_filter(edge: &str) -> bool {
    matches!(edge, "has_type" | "returns" | "constructs" | "member_of")
}

fn related_query_kind(edge: Option<&str>) -> &'static str {
    match edge {
        Some("has_type" | "returns" | "constructs") => "type_references",
        _ => "related",
    }
}
