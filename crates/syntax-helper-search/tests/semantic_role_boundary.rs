use std::fs;
use std::path::{Path, PathBuf};

fn manifests_below(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut manifests = Vec::new();

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "target") {
                    continue;
                }
                pending.push(path);
            } else if path.file_name().is_some_and(|name| name == "Cargo.toml") {
                manifests.push(path);
            }
        }
    }

    manifests
}

#[test]
fn neutral_dependency_and_role_ownership_have_one_hbk_path() {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = package_root.join("../..").canonicalize().unwrap();
    let mut dependency_manifests = manifests_below(&workspace_root)
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .unwrap()
                .contains("v8-context-semantic-entities")
        })
        .collect::<Vec<_>>();
    dependency_manifests.sort();
    let mut expected = vec![
        workspace_root.join("Cargo.toml"),
        package_root.join("Cargo.toml"),
    ];
    expected.sort();

    assert_eq!(dependency_manifests, expected);

    let crate_manifest = fs::read_to_string(package_root.join("Cargo.toml")).unwrap();
    assert!(crate_manifest.contains("v8-context-semantic-entities.workspace = true"));
    assert!(!crate_manifest.contains("v8-context ="));
    assert!(!crate_manifest.contains("analyze-"));

    let crate_facade = fs::read_to_string(package_root.join("src/lib.rs")).unwrap();
    let snapshot_facade = fs::read_to_string(package_root.join("src/snapshot/mod.rs")).unwrap();
    for facade in [crate_facade.as_str(), snapshot_facade.as_str()] {
        assert!(!facade.contains("pub use v8_context_semantic_entities"));
        assert!(!facade.contains("pub use v8_context_semantic_entities::*"));
    }
}

#[test]
fn role_module_has_only_direct_views_and_one_filtered_property_seam() {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(package_root.join("src/snapshot/semantic_roles.rs")).unwrap();

    for direct_impl in [
        "impl CallableView for HbkCallableView<'_>",
        "impl SignatureView for HbkSignatureView<'_>",
        "impl ParameterView for HbkParameterView<'_>",
        "impl PropertyView for HbkPropertyView<'_>",
        "impl TypeDeclarationView for HbkPlatformTypeView<'_>",
    ] {
        assert!(source.contains(direct_impl), "missing {direct_impl}");
    }
    assert_eq!(source.matches("pub struct HbkPropertyView").count(), 1);
    assert_eq!(source.matches("pub fn property_role").count(), 2);
    assert!(source.contains("Member(HbkTypeMemberView<'a>)"));
    assert!(source.contains("Global(HbkGlobalFactView<'a>)"));
    assert!(source.contains("HbkLanguageDomain::Bsl"));

    for prohibited in [
        "pub struct HbkSemantic",
        "struct SemanticSnapshot",
        "struct SemanticCatalog",
        "struct SemanticIndex",
        "struct SemanticRegistry",
        "fn match_argument_count",
        "-> String",
        "-> Vec<",
        "Box<dyn",
        "name: String",
        "signatures: Vec",
        "parameters: Vec",
        "type_refs: Vec",
    ] {
        assert!(
            !source.contains(prohibited),
            "prohibited semantic mirror or allocating path: {prohibited}"
        );
    }
}
