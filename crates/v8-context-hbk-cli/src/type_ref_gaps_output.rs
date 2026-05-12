fn print_constructor_text_hit(hit: &SearchHit, details: bool) {
    if hit.document.signatures.is_empty() {
        println!("{}", hit.document.name.primary);
    } else {
        for signature in &hit.document.signatures {
            println!("{}", signature.text);
        }
    }

    if !details {
        return;
    }

    if let Some(owner) = &hit.document.owner {
        println!("  owner: {}", owner.display_name());
    }
    if let Some(alias) = &hit.document.name.alias {
        println!("  alias: {alias}");
    }
    if let Some(description) = &hit.document.description {
        println!("  {description}");
    } else if !hit.document.preview.is_empty() {
        println!("  {}", hit.document.preview);
    }
}

fn print_type_reference_gap_report_text(
    report: &TypeReferenceGapReport,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "type_references: total={} resolved={} unresolved={} ambiguous={} template_bindings={}",
        report.total,
        report.resolved,
        report.unresolved,
        report.ambiguous,
        report.template_bindings
    );
    println!("roles:");
    for role in &report.roles {
        println!(
            "- {}: total={} resolved={} unresolved={} ambiguous={} template_bindings={}",
            role.role,
            role.total,
            role.resolved,
            role.unresolved,
            role.ambiguous,
            role.template_bindings
        );
    }
    print_gap_section_text("top_unresolved", &report.top_unresolved);
    print_gap_section_text("top_ambiguous", &report.top_ambiguous);
    Ok(())
}

fn print_gap_section_text(title: &str, gaps: &[TypeReferenceGap]) {
    println!("{title}:");
    for gap in gaps {
        println!(
            "- {} [{}] count={}",
            gap.target_type_name, gap.role, gap.count
        );
        if !gap.candidate_type_ids.is_empty() {
            println!("  candidates: {}", gap.candidate_type_ids.join(", "));
        }
        for example in &gap.examples {
            let owner = example
                .source_owner
                .as_ref()
                .map(|owner| format!(" owner={}", owner.display_name()))
                .unwrap_or_default();
            println!(
                "  example: {} [{}] name={}{}",
                example.source_document_id,
                example.source_kind.as_str(),
                example.source_name.display_name(),
                owner
            );
        }
    }
}

fn print_type_reference_gap_report_json(
    report: &TypeReferenceGapReport,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = json!({
        "schema_version": 1,
        "command": "type-ref-gaps",
        "total": report.total,
        "resolved": report.resolved,
        "unresolved": report.unresolved,
        "ambiguous": report.ambiguous,
        "template_bindings": report.template_bindings,
        "roles": report.roles.iter().map(type_reference_role_value).collect::<Vec<_>>(),
        "top_unresolved": report.top_unresolved.iter().map(type_reference_gap_value).collect::<Vec<_>>(),
        "top_ambiguous": report.top_ambiguous.iter().map(type_reference_gap_value).collect::<Vec<_>>(),
    });
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}

fn type_reference_role_value(role: &TypeReferenceRoleReport) -> Value {
    json!({
        "role": role.role,
        "total": role.total,
        "resolved": role.resolved,
        "unresolved": role.unresolved,
        "ambiguous": role.ambiguous,
        "template_bindings": role.template_bindings,
    })
}

fn type_reference_gap_value(gap: &TypeReferenceGap) -> Value {
    json!({
        "role": gap.role,
        "target_type_name": gap.target_type_name,
        "count": gap.count,
        "candidate_type_ids": gap.candidate_type_ids,
        "examples": gap.examples.iter().map(type_reference_gap_example_value).collect::<Vec<_>>(),
    })
}

fn type_reference_gap_example_value(example: &TypeReferenceGapExample) -> Value {
    json!({
        "source_document_id": example.source_document_id,
        "source_kind": example.source_kind.as_str(),
        "source_name": example.source_name,
        "source_owner": example.source_owner,
    })
}
