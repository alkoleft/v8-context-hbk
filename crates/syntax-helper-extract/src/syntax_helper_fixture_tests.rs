use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const MANIFEST: &str = include_str!("../../../tests/fixtures/syntax-helper/manifest.tsv");

#[derive(Debug)]
struct ManifestEntry<'a> {
    parser_kind: &'a str,
    source_hbk: &'a str,
    page_title: &'a str,
    fixture_path: &'a str,
    reason: &'a str,
}

#[test]
fn syntax_assistant_fixture_manifest_covers_required_parser_kinds() {
    let entries = parse_manifest();
    let actual_kinds = entries
        .iter()
        .map(|entry| entry.parser_kind)
        .collect::<BTreeSet<_>>();
    let required_kinds = BTreeSet::from([
        "global_context",
        "global_method",
        "global_property",
        "object_type",
        "object_method",
        "object_property",
        "constructor",
        "enum",
        "enum_value",
        "root_catalog",
    ]);

    assert_eq!(actual_kinds, required_kinds);
    assert!(
        entries
            .iter()
            .any(|entry| entry.source_hbk.ends_with("shcntx_ru.hbk"))
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.source_hbk.ends_with("shcntx_root.hbk"))
    );
    assert!(
        entries
            .iter()
            .filter(|entry| entry.parser_kind == "root_catalog")
            .count()
            >= 3
    );
    assert!(
        entries
            .iter()
            .filter(|entry| entry.parser_kind == "root_catalog")
            .all(|entry| entry.reason.contains("TOC records")),
        "root/catalog HTML fixtures must document that catalog children are represented by TOC records"
    );
}

#[test]
fn syntax_assistant_fixture_manifest_points_to_real_html_fragments() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for entry in parse_manifest() {
        let path = workspace_root.join(entry.fixture_path);
        let html = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));
        assert!(
            html.contains("<html") || html.contains("<body") || html.contains("V8SH_"),
            "{} must look like a real Syntax Assistant HTML fragment",
            path.display()
        );
        assert!(
            !entry.page_title.trim().is_empty(),
            "{} must record a page title",
            entry.fixture_path
        );
        assert!(
            !entry.reason.trim().is_empty(),
            "{} must record a fixture reason",
            entry.fixture_path
        );
    }
}

fn parse_manifest() -> Vec<ManifestEntry<'static>> {
    MANIFEST
        .lines()
        .filter(|line| {
            !line.trim().is_empty() && !line.starts_with('#') && !line.starts_with("parser_kind")
        })
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            assert_eq!(
                fields.len(),
                6,
                "manifest row must have 6 tab-separated fields"
            );
            ManifestEntry {
                parser_kind: fields[0],
                source_hbk: fields[1],
                page_title: fields[3],
                fixture_path: fields[4],
                reason: fields[5],
            }
        })
        .collect()
}
