fn print_provider_hits(
    command: &str,
    query: Value,
    hits: &[SearchHit],
) -> Result<(), Box<dyn std::error::Error>> {
    print_provider_hits_with_index(command, query, hits, None)
}

fn print_provider_hits_with_index(
    command: &str,
    query: Value,
    hits: &[SearchHit],
    index: Option<&SearchIndex>,
) -> Result<(), Box<dyn std::error::Error>> {
    let status = provider_status(command, &query, hits.len());
    let diagnostics = provider_diagnostics(status, &query, hits);
    let results = if status == "ambiguous" {
        Vec::new()
    } else {
        hits.iter()
            .enumerate()
            .map(|(rank_index, hit)| {
                let mut meta = json!({ "rank": rank_index + 1 });
                if command == "search" {
                    meta["score"] = json!(hit.score);
                }
                if let Some(search_index) = index {
                    add_provider_resolution_meta(search_index, &hit.document, &mut meta)?;
                }
                Ok(json!({
                    "fact": document_fact(&hit.document, ProviderFactDetail::Full),
                    "meta": meta,
                }))
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?
    };
    print_provider_response(command, status, query, results, diagnostics)
}

fn add_provider_resolution_meta(
    index: &SearchIndex,
    document: &SearchDocument,
    meta: &mut Value,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(owner_type_id) = index.owner_type_id_for_document(&document.id)? {
        meta["owner_type_id"] = json!(owner_type_id);
    }
    let target_type_ids = index.target_type_ids_for_document(&document.id)?;
    if !target_type_ids.is_empty() {
        meta["target_type_ids"] = json!(target_type_ids);
    }
    Ok(())
}

fn print_provider_related_hits(
    query: Value,
    hits: &[RelatedHit],
    compact: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let results = hits
        .iter()
        .map(|hit| related_result_value(hit, compact))
        .collect::<Vec<_>>();
    print_provider_response("related", "ok", query, results, Vec::new())
}

fn print_provider_type_graph(
    index: &SearchIndex,
    query: Value,
    root: SearchHit,
    hits: &[RelatedHit],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut seen = std::collections::BTreeSet::from([root.document.id.clone()]);
    let mut graph_results = Vec::with_capacity(hits.len() + 1);
    graph_results.push(type_graph_root_result(index, &root.document)?);
    for hit in hits {
        if !seen.insert(hit.document.id.clone()) {
            continue;
        }
        graph_results.push(type_graph_related_result(index, hit)?);
    }
    let diagnostics = graph_type_reference_diagnostics(&graph_results);
    print_provider_response("related", "ok", query, graph_results, diagnostics)
}

fn type_graph_root_result(
    index: &SearchIndex,
    document: &SearchDocument,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut meta = json!({
        "root": true,
        "depth": 0,
        "path": [],
    });
    add_graph_document_meta(index, document, &mut meta)?;
    Ok(json!({
        "fact": document_fact(document, ProviderFactDetail::Full),
        "meta": meta,
    }))
}

fn type_graph_related_result(
    index: &SearchIndex,
    hit: &RelatedHit,
) -> Result<Value, Box<dyn std::error::Error>> {
    let mut meta = json!({
        "depth": hit.depth,
        "path": hit.via.iter().map(relation_step_value).collect::<Vec<_>>(),
    });
    add_graph_document_meta(index, &hit.document, &mut meta)?;
    Ok(json!({
        "fact": document_fact(&hit.document, ProviderFactDetail::Full),
        "meta": meta,
    }))
}

fn add_graph_document_meta(
    index: &SearchIndex,
    document: &SearchDocument,
    meta: &mut Value,
) -> Result<(), Box<dyn std::error::Error>> {
    add_provider_resolution_meta(index, document, meta)?;
    let type_references = graph_type_references(document);
    if !type_references.is_empty() {
        meta["type_references"] = json!(type_references);
    }
    Ok(())
}

fn graph_type_references(document: &SearchDocument) -> Vec<Value> {
    let mut references = Vec::new();
    references.extend(
        document
            .type_ref_facts
            .iter()
            .map(|type_ref| graph_type_ref_value("type", type_ref, None, None, None)),
    );
    references.extend(
        document
            .return_type_facts
            .iter()
            .map(|type_ref| graph_type_ref_value("return", type_ref, None, None, None)),
    );
    for (signature_ordinal, signature) in document.signatures.iter().enumerate() {
        references.extend(signature.return_type_facts.iter().map(|type_ref| {
            graph_type_ref_value(
                "signature_return",
                type_ref,
                Some(signature_ordinal),
                None,
                None,
            )
        }));
        for (parameter_ordinal, parameter) in signature.parameters.iter().enumerate() {
            references.extend(parameter.type_ref_facts.iter().map(|type_ref| {
                graph_type_ref_value(
                    "parameter_type",
                    type_ref,
                    Some(signature_ordinal),
                    Some(parameter_ordinal),
                    Some(parameter.name.as_str()),
                )
            }));
        }
    }
    references
}

fn graph_type_ref_value(
    role: &str,
    type_ref: &SearchTypeRef,
    signature_ordinal: Option<usize>,
    parameter_ordinal: Option<usize>,
    parameter_name: Option<&str>,
) -> Value {
    let mut value = json!({
        "role": role,
        "name": type_ref.name,
        "status": graph_type_ref_status(&type_ref.target),
    });
    if let Some(target_type_id) = type_ref.target.target_type_id() {
        value["target_type_id"] = json!(target_type_id);
    }
    let candidate_type_ids = type_ref.target.candidate_type_ids();
    if !candidate_type_ids.is_empty() {
        value["candidate_type_ids"] = json!(candidate_type_ids);
    }
    if let Some(signature_ordinal) = signature_ordinal {
        value["signature_ordinal"] = json!(signature_ordinal);
    }
    if let Some(parameter_ordinal) = parameter_ordinal {
        value["parameter_ordinal"] = json!(parameter_ordinal);
    }
    if let Some(parameter_name) = parameter_name {
        value["parameter_name"] = json!(parameter_name);
    }
    if let Some(binding) = &type_ref.template_binding {
        value["template_binding"] = template_binding_value(binding);
    }
    value
}

fn template_binding_value(binding: &syntax_helper_search::model::TypeTemplateBinding) -> Value {
    json!({
        "template_key": {
            "family": binding.template_key.family.as_str(),
            "variant": binding.template_key.variant.as_str(),
        },
        "arguments": binding.arguments.iter().map(template_binding_argument_value).collect::<Vec<_>>(),
    })
}

fn template_binding_argument_value(
    argument: &syntax_helper_search::model::TemplateParameterBinding,
) -> Value {
    match argument {
        syntax_helper_search::model::TemplateParameterBinding::OwnerParameter {
            owner_parameter_index,
            target_parameter_index,
        } => json!({
            "owner_parameter": {
                "owner_parameter_index": owner_parameter_index,
                "target_parameter_index": target_parameter_index,
            }
        }),
    }
}

fn graph_type_ref_status(target: &SearchTypeRefTarget) -> &'static str {
    match target {
        SearchTypeRefTarget::Ok(_) => "ok",
        SearchTypeRefTarget::Unresolved => "unresolved",
        SearchTypeRefTarget::Ambiguous(_) => "ambiguous",
    }
}

fn graph_type_reference_diagnostics(results: &[Value]) -> Vec<Value> {
    let mut diagnostics = Vec::new();
    for result in results {
        let Some(source_id) = result["fact"]["id"].as_str() else {
            continue;
        };
        for type_ref in result["meta"]["type_references"]
            .as_array()
            .into_iter()
            .flatten()
        {
            match type_ref["status"].as_str() {
                Some("unresolved") => diagnostics.push(json!({
                    "code": "UNRESOLVED_TYPE_REFERENCE",
                    "message": "The graph contains an unresolved type reference.",
                    "source_id": source_id,
                    "role": type_ref["role"],
                    "name": type_ref["name"],
                })),
                Some("ambiguous") => diagnostics.push(json!({
                    "code": "AMBIGUOUS_TYPE_REFERENCE",
                    "message": "The graph contains an ambiguous type reference.",
                    "source_id": source_id,
                    "role": type_ref["role"],
                    "name": type_ref["name"],
                    "candidate_type_ids": type_ref["candidate_type_ids"].clone(),
                })),
                _ => {}
            }
        }
    }
    diagnostics
}

fn related_result_value(hit: &RelatedHit, compact: bool) -> Value {
    json!({
        "fact": document_fact(
            &hit.document,
            if compact {
                ProviderFactDetail::Compact
            } else {
                ProviderFactDetail::Full
            },
        ),
        "meta": {
            "depth": hit.depth,
            "path": hit.via.iter().map(relation_step_value).collect::<Vec<_>>(),
        },
    })
}

fn relation_step_value(step: &syntax_helper_search::RelationStep) -> Value {
    json!({
        "from": step.from,
        "to": step.to,
        "edge_kind": step.edge_kind,
        "label": step.label,
        "evidence": step.evidence,
    })
}

fn print_related_root_diagnostic(
    command: &str,
    query: Value,
    roots: Vec<SearchHit>,
    format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match format {
        OutputFormat::Text => {
            if roots.is_empty() {
                println!("{command}: no matches");
            } else {
                println!("{command}: ambiguous root ({} matches)", roots.len());
            }
            Ok(())
        }
        OutputFormat::Json => {
            let status = if roots.is_empty() {
                "not_found"
            } else {
                "ambiguous"
            };
            let diagnostics = provider_diagnostics(status, &query, &roots);
            print_provider_response(command, status, query, Vec::new(), diagnostics)
        }
    }
}

fn provider_status(command: &str, query: &Value, hit_count: usize) -> &'static str {
    if query["kind"] == "member_list" {
        return "ok";
    }
    match (command, hit_count) {
        ("search", _) => "ok",
        (_, 0) => "not_found",
        ("get", count) if count > 1 => "ambiguous",
        (_, _) => "ok",
    }
}

fn provider_diagnostics(status: &str, query: &Value, hits: &[SearchHit]) -> Vec<Value> {
    match status {
        "not_found" => vec![json!({
            "code": "NOT_FOUND",
            "message": "No Syntax Assistant fact matched the query.",
            "query": query,
        })],
        "ambiguous" => vec![json!({
            "code": "AMBIGUOUS",
            "message": "The query matched more than one Syntax Assistant fact.",
            "query": query,
            "candidates": hits.iter().map(|hit| candidate_summary(&hit.document)).collect::<Vec<_>>(),
        })],
        _ => Vec::new(),
    }
}

fn unsupported_query_diagnostic(message: &str) -> Vec<Value> {
    vec![json!({
        "code": "UNSUPPORTED_QUERY",
        "message": message,
    })]
}

fn print_provider_response(
    command: &str,
    status: &str,
    query: Value,
    results: Vec<Value>,
    diagnostics: Vec<Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = provider_response(command, status, query, results, diagnostics);
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

fn provider_response(
    command: &str,
    status: &str,
    query: Value,
    results: Vec<Value>,
    diagnostics: Vec<Value>,
) -> Value {
    json!({
        "schema_version": 1,
        "command": command,
        "status": status,
        "query": query,
        "results": results,
        "diagnostics": diagnostics,
    })
}

fn candidate_summary(document: &SearchDocument) -> Value {
    json!({
        "id": document.id,
        "kind": document.kind.as_str(),
        "name": document.name,
        "owner": document.owner.as_ref().map(|owner| owner.primary.clone()),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProviderFactDetail {
    Full,
    Compact,
}

fn document_fact(document: &SearchDocument, detail: ProviderFactDetail) -> Value {
    let mut fact = json!({
        "id": document.id,
        "kind": document.kind.as_str(),
        "name": document.name,
    });
    if let Some(owner) = &document.owner {
        fact["owner"] = json!(owner.primary);
    }
    if detail == ProviderFactDetail::Compact {
        return fact;
    }
    if !document.signatures.is_empty() {
        fact["signatures"] = json!(
            document
                .signatures
                .iter()
                .map(signature_fact)
                .collect::<Vec<_>>()
        );
    }
    if !document.type_refs.is_empty() {
        fact["types"] = json!(document.type_refs);
    }
    if !document.return_types.is_empty() {
        fact["return"] = json!(document.return_types);
    }
    if let Some(description) = &document.description {
        fact["description"] = json!(description);
    }
    fact
}

fn signature_fact(signature: &syntax_helper_search::SearchSignature) -> Value {
    let mut value = json!({});
    if !signature.parameters.is_empty() {
        value["parameters"] = json!(
            signature
                .parameters
                .iter()
                .map(parameter_fact)
                .collect::<Vec<_>>()
        );
    }
    if let Some(title) = &signature.title {
        value["title"] = json!(title);
    }
    if let Some(description) = &signature.description {
        value["description"] = json!(description);
    }
    if !signature.return_types.is_empty() {
        value["return"] = json!(signature.return_types);
    }
    value
}

fn parameter_fact(parameter: &syntax_helper_search::SearchParameter) -> Value {
    let mut value = json!({
        "name": parameter.name,
        "required": parameter.required,
    });
    if !parameter.type_refs.is_empty() {
        value["types"] = json!(parameter.type_refs);
    }
    if let Some(description) = &parameter.description {
        value["description"] = json!(description);
    }
    value
}
