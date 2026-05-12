fn print_hits_text(command: &str, hits: &[SearchHit]) -> Result<(), Box<dyn std::error::Error>> {
    if hits.is_empty() {
        println!("{command}: no matches");
    }
    for hit in hits {
        let owner = hit
            .document
            .owner
            .as_ref()
            .map(|owner| owner.display_name())
            .unwrap_or_default();
        let prefix = if owner.is_empty() {
            String::new()
        } else {
            format!("{owner}.")
        };
        println!(
            "{}{} [{}]",
            prefix, hit.document.name.primary, hit.document.kind
        );
        if let Some(alias) = &hit.document.name.alias {
            println!("  alias: {alias}");
        }
        if !hit.document.type_refs.is_empty() {
            println!("  types: {}", hit.document.type_refs.join(", "));
        }
        if !hit.document.return_types.is_empty() {
            println!("  return: {}", hit.document.return_types.join(", "));
        }
        if !hit.document.preview.is_empty() {
            println!("  {}", hit.document.preview);
        }
    }
    Ok(())
}

fn print_related_hits_text(hits: &[RelatedHit]) -> Result<(), Box<dyn std::error::Error>> {
    for hit in hits {
        let owner = hit
            .document
            .owner
            .as_ref()
            .map(|owner| owner.display_name())
            .unwrap_or_default();
        let prefix = if owner.is_empty() {
            String::new()
        } else {
            format!("{owner}.")
        };
        println!(
            "{}{} [{}] depth={}",
            prefix, hit.document.name.primary, hit.document.kind, hit.depth
        );
        for step in &hit.via {
            println!("  - {} -> {} ({})", step.from, step.to, step.edge_kind);
        }
    }
    Ok(())
}

fn print_constructor_hits_text(
    hits: &[SearchHit],
    details: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if hits.is_empty() {
        println!("constructors: no matches");
    }
    for hit in hits {
        print_constructor_text_hit(hit, details);
    }
    Ok(())
}
