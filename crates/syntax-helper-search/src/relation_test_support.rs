#[cfg(test)]
fn relations_from_documents(documents: &[SearchDocument]) -> Vec<Relation> {
    let mut relations = Vec::new();
    visit_relations_from_documents(documents, |relation| {
        relations.push(relation);
        Ok::<_, std::convert::Infallible>(())
    })
    .expect("infallible relation collection must not fail");
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
