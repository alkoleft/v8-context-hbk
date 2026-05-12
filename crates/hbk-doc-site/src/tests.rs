#[cfg(test)]
mod tests {
    use super::*;
    use hbk_book::test_utils::{fixture_container, zip_bytes, zip_entries};
    use serde_json::Value;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn discovers_directory_books_with_include_filter_in_deterministic_order() {
        let workspace = TempWorkspace::new("discovery");
        fs::write(workspace.path().join("b_ru.hbk"), b"b").expect("fixture file must be written");
        fs::write(workspace.path().join("a_ru.hbk"), b"a").expect("fixture file must be written");
        fs::write(workspace.path().join("ignored.txt"), b"x")
            .expect("fixture file must be written");
        fs::write(workspace.path().join("c_ru.hbk"), b"c").expect("fixture file must be written");

        let source = SiteSource::Directory {
            source_dir: workspace.path().to_path_buf(),
            include_file_names: vec!["b_ru.hbk".to_string(), "a_ru.hbk".to_string()],
        };

        let discovered = discover_source_books(&source).expect("source discovery must succeed");
        let file_names: Vec<_> = discovered.iter().map(|path| path_file_name(path)).collect();

        assert_eq!(file_names, vec!["a_ru.hbk", "b_ru.hbk"]);
    }

    #[test]
    fn writes_manifest_root_section_and_page_markdown_artifacts() {
        let workspace = TempWorkspace::new("artifacts");
        let first = workspace.path().join("alpha_ru.hbk");
        let second = workspace.path().join("beta_ru.hbk");
        write_book_fixture_with_toc(
            &first,
            "Alpha",
            "alpha",
            r#"{
                4
                {1,0,2,2,3,{0,0,{0,0,{"ru","Общее"}{"en","Common"}},""}}
                {2,1,1,4,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/alpha/page.html"}}
                {3,1,0,{0,0,{0,0,{"ru","Раздел"}{"en","Section"}},"/alpha/section.html"}}
                {4,2,0,{0,0,{0,0,{"ru","Подраздел"}{"en","Subsection"}},""}}
            }"#,
            vec![
                (
                    "alpha/page.html",
                    "<html><body><h1>Страница</h1><p>alpha page body</p><a href=\"#Local\">local</a><a href=\"v8help://Alpha/alpha/section.html#Anchor\">section</a><h2 id=\"Local\">Local</h2></body></html>".as_bytes(),
                ),
                (
                    "alpha/section.html",
                    b"<html><body><h1 id=\"Anchor\">Section</h1><p>section body</p></body></html>",
                ),
            ],
        );
        write_book_fixture_with_toc(
            &second,
            "Beta",
            "beta",
            r#"{
                3
                {1,0,1,2,{0,0,{0,0,{"ru","Общее"}{"en","Common"}},""}}
                {2,1,1,3,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/beta/page.html"}}
                {3,2,0,{0,0,{0,0,{"ru","Подраздел"}{"en","Subsection"}},""}}
            }"#,
            vec![(
                "beta/page.html",
                "<html><body><h1>Страница</h1><p>beta page body</p><a href=\"v8help://alpha/alpha/page.html\">alpha</a></body></html>".as_bytes(),
            )],
        );
        let output = workspace.path().join("out");
        let request =
            SiteGenerationRequest::explicit_files(&output, vec![second.clone(), first.clone()])
                .expect("explicit request must be valid");

        let result = DocSiteGenerator::generate(&request).expect("site data must generate");

        assert_eq!(result.locale_count(), 1);
        assert_eq!(result.book_count(), 2);
        assert_eq!(result.page_count(), 3);
        assert!(output.join("data/locales/ru/pages").exists());
        assert!(output.join("data/manifest.json").exists());
        assert!(output.join("data/locales/ru/toc-root.json").exists());
        assert!(
            result
                .files()
                .iter()
                .any(|file| file.path().ends_with("data/manifest.json") && file.bytes_written() > 0)
        );

        let manifest = read_json(output.join("data/manifest.json"));
        assert_eq!(manifest["schema_version"], 1);
        assert_eq!(manifest["generator"], "hbk-doc-site");
        assert_eq!(manifest["generator_version"], env!("CARGO_PKG_VERSION"));
        assert!(manifest["build_id"].as_str().unwrap().starts_with("build-"));
        assert_eq!(manifest["locales"], serde_json::json!(["ru"]));
        assert_eq!(manifest["books"]["ru"][0]["book_id"], "alpha-ru");
        assert_eq!(manifest["books"]["ru"][1]["book_id"], "beta-ru");
        assert!(
            manifest["books"]["ru"][0]["file_size_bytes"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert_eq!(
            manifest["toc_roots"]["ru"],
            serde_json::json!("locales/ru/toc-root.json")
        );
        assert_eq!(
            manifest["page_roots"]["ru"],
            serde_json::json!("locales/ru/pages")
        );

        let root = read_json(output.join("data/locales/ru/toc-root.json"));
        let root_nodes = root["nodes"]
            .as_array()
            .expect("root nodes must be an array");
        assert_eq!(root_nodes.len(), 1, "{root}");
        assert_eq!(root_nodes[0]["title"], "Общее");
        assert!(
            root_nodes[0]["id"].as_str().unwrap().is_ascii(),
            "site node ids should use URL-safe ASCII slugs: {root}"
        );
        assert_eq!(root_nodes[0]["has_children"], true);
        let children_path = root_nodes[0]["children_path"]
            .as_str()
            .expect("merged section must have children_path");
        assert!(
            children_path.is_ascii(),
            "lazy TOC section paths should use URL-safe ASCII ids: {children_path}"
        );
        let section = read_json(output.join("data/locales/ru").join(children_path));
        let section_nodes = section["nodes"]
            .as_array()
            .expect("section nodes must be an array");
        assert_eq!(section_nodes.len(), 3, "{section}");

        let duplicate_pages: Vec<_> = section_nodes
            .iter()
            .filter(|node| node["title"] == "Страница")
            .collect();
        assert_eq!(duplicate_pages.len(), 2, "{section}");
        assert_eq!(duplicate_pages[0]["book_id"], "alpha-ru");
        assert_eq!(duplicate_pages[1]["book_id"], "beta-ru");
        assert_ne!(duplicate_pages[0]["page_id"], duplicate_pages[1]["page_id"]);
        let alpha_children_path = duplicate_pages[0]["children_path"]
            .as_str()
            .expect("first duplicate page must keep its child section path");
        let beta_children_path = duplicate_pages[1]["children_path"]
            .as_str()
            .expect("second duplicate page must keep its child section path");
        assert_ne!(alpha_children_path, beta_children_path);
        let alpha_children = read_json(output.join("data/locales/ru").join(alpha_children_path));
        let beta_children = read_json(output.join("data/locales/ru").join(beta_children_path));
        assert_eq!(alpha_children["nodes"][0]["title"], "Подраздел");
        assert_eq!(beta_children["nodes"][0]["title"], "Подраздел");

        let alpha_page_id = duplicate_pages[0]["page_id"]
            .as_str()
            .expect("duplicate page must expose page_id");
        let beta_page_id = duplicate_pages[1]["page_id"]
            .as_str()
            .expect("duplicate page must expose page_id");
        let alpha_markdown = fs::read_to_string(
            output
                .join("data/locales/ru/pages")
                .join(format!("{alpha_page_id}.md")),
        )
        .expect("alpha page Markdown must be written");
        let beta_markdown = fs::read_to_string(
            output
                .join("data/locales/ru/pages")
                .join(format!("{beta_page_id}.md")),
        )
        .expect("beta page Markdown must be written");
        assert!(alpha_markdown.contains("# Страница"));
        assert!(alpha_markdown.contains("alpha page body"));
        assert!(alpha_markdown.contains("[local](#Local)"));
        assert!(!alpha_markdown.contains(&format!("[local]({alpha_page_id}.md#Local)")));
        assert!(!alpha_markdown.contains("[local](index.md#Local)"));
        assert!(alpha_markdown.contains("[section]("));
        assert!(alpha_markdown.contains("#Anchor"));
        assert!(!alpha_markdown.contains("v8help://Alpha"));
        assert!(!alpha_markdown.contains("/alpha/section.html"));
        assert!(beta_markdown.contains("# Страница"));
        assert!(beta_markdown.contains("beta page body"));
        assert!(beta_markdown.contains("[alpha]("));
        assert!(!beta_markdown.contains("v8help://alpha"));
        let alpha_page_file_name = format!("{alpha_page_id}.md");
        assert!(result.files().iter().any(|file| {
            file.path().file_name().and_then(|name| name.to_str())
                == Some(alpha_page_file_name.as_str())
        }));
    }

    #[test]
    fn merges_page_bearing_toc_nodes_by_normalized_address() {
        let workspace = TempWorkspace::new("page-address-merge");
        let first = workspace.path().join("alpha_ru.hbk");
        let second = workspace.path().join("beta_ru.hbk");
        write_book_fixture_with_toc(
            &first,
            "Alpha",
            "alpha",
            r#"{
                3
                {1,0,1,2,{0,0,{0,0,{"ru","Навигационный заголовок"}{"en","Navigation title"}},"/shared/page.html"}}
                {2,1,0,{0,0,{0,0,{"ru","Дочерняя страница Alpha"}{"en","Alpha child"}},"/alpha/child.html"}}
                {3,0,0,{0,0,{0,0,{"ru","Отдельная страница"}{"en","Separate page"}},"/alpha/separate.html"}}
            }"#,
            vec![
                (
                    "shared/page.html",
                    r#"<html><head><title>Ненадежный HTML title</title></head><body><p>alpha body</p><a href="v8help://Beta/shared/page.html">duplicate</a></body></html>"#.as_bytes(),
                ),
                (
                    "alpha/child.html",
                    b"<html><body><h1>Alpha child</h1></body></html>",
                ),
                (
                    "alpha/separate.html",
                    b"<html><body><h1>Separate</h1></body></html>",
                ),
            ],
        );
        write_book_fixture_with_toc(
            &second,
            "Beta",
            "beta",
            r#"{
                2
                {1,0,1,2,{0,0,{0,0,{"ru","Другой TOC заголовок"}{"en","Other TOC title"}},"shared/page.html"}}
                {2,1,0,{0,0,{0,0,{"ru","Дочерняя страница Beta"}{"en","Beta child"}},"/beta/child.html"}}
            }"#,
            vec![
                (
                    "shared/page.html",
                    b"<html><body><h1>Beta HTML title</h1><p>beta body</p></body></html>",
                ),
                (
                    "beta/child.html",
                    b"<html><body><h1>Beta child</h1></body></html>",
                ),
            ],
        );
        let output = workspace.path().join("out");
        let request =
            SiteGenerationRequest::explicit_files(&output, vec![second.clone(), first.clone()])
                .expect("explicit request must be valid");

        let result = DocSiteGenerator::generate(&request).expect("site data must generate");

        assert_eq!(result.page_count(), 4);
        let root = read_json(output.join("data/locales/ru/toc-root.json"));
        let root_nodes = root["nodes"]
            .as_array()
            .expect("root nodes must be an array");
        assert_eq!(root_nodes.len(), 2, "{root}");
        let merged = &root_nodes[0];
        assert_eq!(merged["title"], "Навигационный заголовок");
        assert_eq!(merged["book_id"], "alpha-ru");
        let merged_page_id = merged["page_id"]
            .as_str()
            .expect("merged node must expose page_id");
        let children_path = merged["children_path"]
            .as_str()
            .expect("merged node must keep merged child sections");
        let children = read_json(output.join("data/locales/ru").join(children_path));
        let child_titles: Vec<_> = children["nodes"]
            .as_array()
            .expect("child nodes must be an array")
            .iter()
            .map(|node| node["title"].as_str().unwrap())
            .collect();
        assert_eq!(
            child_titles,
            vec!["Дочерняя страница Alpha", "Дочерняя страница Beta"]
        );

        let page_files = fs::read_dir(output.join("data/locales/ru/pages"))
            .expect("pages directory must exist")
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(page_files.len(), 4);
        assert!(
            output
                .join("data/locales/ru/pages")
                .join(format!("{merged_page_id}.md"))
                .exists()
        );
        let merged_markdown = fs::read_to_string(
            output
                .join("data/locales/ru/pages")
                .join(format!("{merged_page_id}.md")),
        )
        .expect("merged page Markdown must be written");
        assert!(merged_markdown.contains("alpha body"));
        assert!(!merged_markdown.contains("beta body"));
        assert!(merged_markdown.contains("[duplicate]("));
        assert!(!merged_markdown.contains("v8help://Beta"));
    }

    #[test]
    fn merges_content_node_placeholder_pages_by_address() {
        let workspace = TempWorkspace::new("content-node-page-identity");
        let first = workspace.path().join("alpha_ru.hbk");
        let second = workspace.path().join("beta_ru.hbk");
        write_book_fixture_with_toc(
            &first,
            "Alpha",
            "alpha",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Раздел Alpha"}{"en","Alpha section"}},"_CONTENTS_NODE_file3"}}
            }"#,
            vec![],
        );
        write_book_fixture_with_toc(
            &second,
            "Beta",
            "beta",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Раздел Beta"}{"en","Beta section"}},"_CONTENTS_NODE_file3"}}
            }"#,
            vec![],
        );
        let output = workspace.path().join("out");
        let request = SiteGenerationRequest::explicit_files(&output, vec![second, first])
            .expect("explicit request must be valid");

        let result = DocSiteGenerator::generate(&request).expect("site data must generate");

        assert_eq!(result.page_count(), 1);
        let root = read_json(output.join("data/locales/ru/toc-root.json"));
        let root_nodes = root["nodes"]
            .as_array()
            .expect("root nodes must be an array");
        assert_eq!(root_nodes.len(), 1, "{root}");
        assert_eq!(root_nodes[0]["title"], "Раздел Alpha");
        assert_eq!(root_nodes[0]["book_id"], "alpha-ru");
        assert!(
            root_nodes[0]["page_id"]
                .as_str()
                .unwrap()
                .starts_with("page-ru-")
        );
    }

    #[test]
    fn resolves_placeholder_page_branch_to_single_concrete_target() {
        let workspace = TempWorkspace::new("placeholder-to-concrete-page");
        let placeholder = workspace.path().join("alpha_ru.hbk");
        let concrete = workspace.path().join("beta_ru.hbk");
        write_book_fixture_with_toc(
            &placeholder,
            "Alpha",
            "alpha",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Общий раздел"}{"en","Shared section"}},"_CONTENTS_NODE_file3"}}
            }"#,
            vec![],
        );
        write_book_fixture_with_toc(
            &concrete,
            "Beta",
            "beta",
            r##"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Общий раздел"}{"en","Shared section"}},"/real/page.html"}}
            }"##,
            vec![(
                "real/page.html",
                r##"<html><body><p>real body</p><a href="v8help://Alpha/_CONTENTS_NODE_file3#Details">placeholder link</a><h2 id="Details">Details</h2></body></html>"##.as_bytes(),
            )],
        );
        let output = workspace.path().join("out");
        let request = SiteGenerationRequest::explicit_files(&output, vec![placeholder, concrete])
            .expect("explicit request must be valid");

        let result = DocSiteGenerator::generate(&request).expect("site data must generate");

        assert_eq!(result.page_count(), 1);
        let root = read_json(output.join("data/locales/ru/toc-root.json"));
        let root_nodes = root["nodes"]
            .as_array()
            .expect("root nodes must be an array");
        assert_eq!(root_nodes.len(), 1, "{root}");
        assert_eq!(root_nodes[0]["title"], "Общий раздел");
        assert_eq!(root_nodes[0]["book_id"], "beta-ru");
        let page_id = root_nodes[0]["page_id"]
            .as_str()
            .expect("resolved page must expose page id");
        let markdown = fs::read_to_string(
            output
                .join("data/locales/ru/pages")
                .join(format!("{page_id}.md")),
        )
        .expect("resolved page Markdown must be written");
        assert!(markdown.contains("real body"));
        assert!(markdown.contains("[placeholder link]("));
        assert!(markdown.contains("#Details"));
        assert!(!markdown.contains("v8help://Alpha"));
        assert!(!markdown.contains("_CONTENTS_NODE_file3"));
    }

    #[test]
    fn keeps_placeholder_page_when_concrete_target_is_ambiguous() {
        let workspace = TempWorkspace::new("placeholder-ambiguous-page");
        let placeholder = workspace.path().join("alpha_ru.hbk");
        let first_concrete = workspace.path().join("beta_ru.hbk");
        let second_concrete = workspace.path().join("gamma_ru.hbk");
        write_book_fixture_with_toc(
            &placeholder,
            "Alpha",
            "alpha",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Общий раздел"}{"en","Shared section"}},"_CONTENTS_NODE_file3"}}
            }"#,
            vec![],
        );
        write_book_fixture_with_toc(
            &first_concrete,
            "Beta",
            "beta",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Общий раздел"}{"en","Shared section"}},"/first/page.html"}}
            }"#,
            vec![("first/page.html", b"<html><body><p>first</p></body></html>")],
        );
        write_book_fixture_with_toc(
            &second_concrete,
            "Gamma",
            "gamma",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Общий раздел"}{"en","Shared section"}},"/second/page.html"}}
            }"#,
            vec![(
                "second/page.html",
                b"<html><body><p>second</p></body></html>",
            )],
        );
        let output = workspace.path().join("out");
        let request = SiteGenerationRequest::explicit_files(
            &output,
            vec![placeholder, first_concrete, second_concrete],
        )
        .expect("explicit request must be valid");

        let result = DocSiteGenerator::generate(&request).expect("site data must generate");

        assert_eq!(result.page_count(), 3);
        let root = read_json(output.join("data/locales/ru/toc-root.json"));
        let root_nodes = root["nodes"]
            .as_array()
            .expect("root nodes must be an array");
        assert_eq!(root_nodes.len(), 3, "{root}");
        assert_eq!(root_nodes[0]["book_id"], "alpha-ru");
        assert_eq!(root_nodes[1]["book_id"], "beta-ru");
        assert_eq!(root_nodes[2]["book_id"], "gamma-ru");
        assert_ne!(root_nodes[0]["page_id"], root_nodes[1]["page_id"]);
        assert_ne!(root_nodes[0]["page_id"], root_nodes[2]["page_id"]);
    }

    #[test]
    fn site_identity_helpers_use_library_backed_slug_and_fnv() {
        assert_eq!(stable_hash_hex("foobar"), "85944171f73967e8");
        let slug = site_slug("Общий раздел");
        assert!(!slug.is_empty());
        assert!(
            slug.bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
            "slug should be URL-safe ASCII: {slug}"
        );
        assert_eq!(site_slug("!!!"), "item");
    }

    #[test]
    fn generate_with_progress_reports_books_planning_and_artifacts() {
        let workspace = TempWorkspace::new("progress");
        let source = workspace.path().join("alpha_ru.hbk");
        write_book_fixture_with_toc(
            &source,
            "Alpha",
            "alpha",
            r#"{
                2
                {1,0,1,2,{0,0,{0,0,{"ru","Раздел"}{"en","Section"}},""}}
                {2,1,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/alpha/page.html"}}
            }"#,
            vec![(
                "alpha/page.html",
                "<html><body><h1>Страница</h1><p>page body</p></body></html>".as_bytes(),
            )],
        );
        let output = workspace.path().join("out");
        let request = SiteGenerationRequest::explicit_files(&output, vec![source.clone()])
            .expect("explicit request must be valid");
        let mut events = Vec::new();

        let result = DocSiteGenerator::generate_with_progress(&request, |event| match event {
            SiteGenerationProgress::SourceBooksDiscovered { count } => {
                events.push(format!("discovered:{count}"));
            }
            SiteGenerationProgress::SourceBookLoading {
                current,
                total,
                path,
            } => {
                events.push(format!(
                    "loading:{current}/{total}:{}",
                    path_file_name(path)
                ));
            }
            SiteGenerationProgress::SourceBooksLoaded { count } => {
                events.push(format!("loaded:{count}"));
            }
            SiteGenerationProgress::SiteDataBuilt {
                locale_count,
                toc_node_count,
                page_count,
            } => {
                events.push(format!(
                    "planned:{locale_count}:{toc_node_count}:{page_count}"
                ));
            }
            SiteGenerationProgress::ArtifactWriting {
                current,
                total,
                kind,
                path: _,
            } => {
                events.push(format!("artifact:{current}/{total}:{kind:?}"));
            }
        })
        .expect("site data must generate");

        assert_eq!(result.book_count(), 1);
        assert_eq!(result.page_count(), 1);
        assert_eq!(
            events,
            vec![
                "discovered:1",
                "loading:1/1:alpha_ru.hbk",
                "loaded:1",
                "planned:1:2:1",
                "artifact:1/4:Manifest",
                "artifact:2/4:TocRoot",
                "artifact:3/4:TocSection",
                "artifact:4/4:Page",
            ]
        );
    }

    #[test]
    fn rejects_unsafe_locale_code_before_writing_locale_artifacts() {
        let workspace = TempWorkspace::new("bad-locale");
        let source = workspace.path().join("alpha_...hbk");
        write_book_fixture_with_toc(
            &source,
            "Alpha",
            "alpha",
            r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/alpha/page.html"}}
            }"#,
            vec![("alpha/page.html", b"<html><body>alpha</body></html>")],
        );
        let output = workspace.path().join("out");
        let request = SiteGenerationRequest::explicit_files(&output, vec![source.clone()])
            .expect("explicit request must be valid");

        let error = DocSiteGenerator::generate(&request)
            .expect_err("unsafe locale path segment must be rejected");

        match error {
            SiteGenerationError::UnsupportedLocale { path, locale } => {
                assert_eq!(path, source);
                assert_eq!(locale, "..");
            }
            other => panic!("unexpected error: {other}"),
        }
        assert!(!output.exists());
    }

    #[test]
    fn generated_toc_artifacts_are_deterministic_across_runs() {
        let workspace = TempWorkspace::new("deterministic");
        let source = workspace.path().join("alpha_ru.hbk");
        write_book_fixture_with_toc(
            &source,
            "Alpha",
            "alpha",
            r#"{
                2
                {1,0,1,2,{0,0,{0,0,{"ru","Корень"}{"en","Root"}},""}}
                {2,1,0,{0,0,{0,0,{"ru","Страница"}{"en","Page"}},"/alpha/page.html"}}
            }"#,
            vec![("alpha/page.html", b"<html><body>alpha</body></html>")],
        );

        let output_one = workspace.path().join("out-one");
        let output_two = workspace.path().join("out-two");
        let request_one = SiteGenerationRequest::explicit_files(&output_one, vec![source.clone()])
            .expect("first request must be valid");
        let request_two = SiteGenerationRequest::explicit_files(&output_two, vec![source])
            .expect("second request must be valid");

        DocSiteGenerator::generate(&request_one).expect("first generation must succeed");
        DocSiteGenerator::generate(&request_two).expect("second generation must succeed");

        assert_eq!(
            fs::read_to_string(output_one.join("data/manifest.json")).unwrap(),
            fs::read_to_string(output_two.join("data/manifest.json")).unwrap()
        );
        assert_eq!(
            fs::read_to_string(output_one.join("data/locales/ru/toc-root.json")).unwrap(),
            fs::read_to_string(output_two.join("data/locales/ru/toc-root.json")).unwrap()
        );
        assert_eq!(
            only_page_file_name(&output_one),
            only_page_file_name(&output_two)
        );
        assert_eq!(
            fs::read_to_string(only_page_file(&output_one)).unwrap(),
            fs::read_to_string(only_page_file(&output_two)).unwrap()
        );
        let section_one = only_section_file(&output_one);
        let section_two = only_section_file(&output_two);
        assert_eq!(
            fs::read_to_string(section_one).unwrap(),
            fs::read_to_string(section_two).unwrap()
        );
    }

    fn write_book_fixture_with_toc(
        path: &Path,
        book_name: &str,
        description: &str,
        toc: &str,
        storage_entries: Vec<(&str, &[u8])>,
    ) {
        fs::write(
            path,
            fixture_container(vec![
                (
                    "Book",
                    Some(
                        format!(
                            r#"{{1,"{book_name}", {{1,2,{{"ru","{description}"}}}}, 1, "tag", {{0,0}}, 0}}"#
                        )
                        .into_bytes(),
                    ),
                ),
                ("PackBlock", Some(zip_bytes("toc.txt", toc.as_bytes()))),
                ("FileStorage", Some(zip_entries(storage_entries))),
            ]),
        )
        .expect("fixture HBK must be written");
    }

    fn read_json(path: impl AsRef<Path>) -> Value {
        let text = fs::read_to_string(path).expect("JSON artifact must be readable");
        serde_json::from_str(&text).expect("JSON artifact must parse")
    }

    fn only_section_file(output: &Path) -> PathBuf {
        let sections_dir = output.join("data/locales/ru/toc-sections");
        let mut files = fs::read_dir(sections_dir)
            .expect("sections directory must exist")
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(files.len(), 1);
        files.remove(0)
    }

    fn only_page_file(output: &Path) -> PathBuf {
        let pages_dir = output.join("data/locales/ru/pages");
        let mut files = fs::read_dir(pages_dir)
            .expect("pages directory must exist")
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        files.sort();
        assert_eq!(files.len(), 1);
        files.remove(0)
    }

    fn only_page_file_name(output: &Path) -> String {
        only_page_file(output)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    struct TempWorkspace {
        path: PathBuf,
    }

    impl TempWorkspace {
        fn new(name: &str) -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "v8-context-hbk-doc-site-test-{name}-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("temp workspace must be created");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
