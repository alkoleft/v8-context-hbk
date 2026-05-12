#[cfg(test)]
mod tests {
    use super::*;
    use hbk_book::test_utils::{fixture_container, zip_bytes, zip_entries};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn accepts_supported_export_combinations() {
        let raw = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "target/book-export/raw",
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");
        assert_eq!(raw.source_path(), Path::new("fmtdui_ru.hbk"));
        assert_eq!(raw.output_root(), Path::new("target/book-export/raw"));
        assert_eq!(raw.format(), BookExportFormat::Raw);
        assert_eq!(raw.hierarchy(), BookExportHierarchy::Raw);

        let markdown = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "target/book-export/markdown",
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");
        assert_eq!(markdown.format(), BookExportFormat::Markdown);
        assert_eq!(markdown.hierarchy(), BookExportHierarchy::Toc);
    }

    #[test]
    fn rejects_unsupported_export_combinations() {
        let raw_toc = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "target/book-export/raw-toc",
            BookExportFormat::Raw,
            BookExportHierarchy::Toc,
        )
        .expect_err("raw/toc must stay unsupported until specified");
        assert_eq!(
            raw_toc,
            BookExportError::UnsupportedCombination {
                format: BookExportFormat::Raw,
                hierarchy: BookExportHierarchy::Toc,
            }
        );
        assert_eq!(
            raw_toc.to_string(),
            "unsupported book export combination: format=raw, hierarchy=toc"
        );

        let markdown_raw = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "target/book-export/markdown-raw",
            BookExportFormat::Markdown,
            BookExportHierarchy::Raw,
        )
        .expect_err("markdown/raw must stay unsupported until specified");
        assert_eq!(
            markdown_raw,
            BookExportError::UnsupportedCombination {
                format: BookExportFormat::Markdown,
                hierarchy: BookExportHierarchy::Raw,
            }
        );
    }

    #[test]
    fn rejects_unsafe_output_roots() {
        let empty = BookExportRequest::new(
            "fmtdui_ru.hbk",
            PathBuf::new(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect_err("empty output root must be rejected");
        assert_eq!(
            empty,
            BookExportError::InvalidOutputRoot {
                output_root: PathBuf::new(),
                reason: OutputRootError::MissingDirectoryName,
            }
        );

        let root_only = BookExportRequest::new(
            "fmtdui_ru.hbk",
            Path::new("/"),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect_err("root-only output path must be rejected");
        assert_eq!(
            root_only,
            BookExportError::InvalidOutputRoot {
                output_root: PathBuf::from("/"),
                reason: OutputRootError::MissingDirectoryName,
            }
        );

        let parent = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "target/../book-export",
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect_err("parent-directory output root must be rejected");
        assert_eq!(
            parent,
            BookExportError::InvalidOutputRoot {
                output_root: PathBuf::from("target/../book-export"),
                reason: OutputRootError::ParentSegment,
            }
        );
    }

    #[test]
    fn accepts_absolute_output_root_with_directory_name() {
        let request = BookExportRequest::new(
            "fmtdui_ru.hbk",
            "/tmp/v8-context-hbk-book-export",
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("absolute output roots with a directory name are valid");

        assert_eq!(
            request.output_root(),
            Path::new("/tmp/v8-context-hbk-book-export")
        );
    }

    #[test]
    fn exposes_export_result_file_summary() {
        let result = BookExportResult::new(
            "target/book-export/raw",
            vec![BookExportedFile::new(
                "target/book-export/raw/docs/page.html",
                42,
            )],
        );

        assert_eq!(result.output_root(), Path::new("target/book-export/raw"));
        assert_eq!(result.files()[0].bytes_written(), 42);
        assert_eq!(
            result.files()[0].path(),
            Path::new("target/book-export/raw/docs/page.html")
        );
    }

    #[test]
    fn converts_toc_page_html_to_markdown_without_raw_scaffolding() {
        let workspace = TempWorkspace::new("markdown-page");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r#"{
            2
            {1,0,0,{0,0,{0,0,{"ru","Справка"}{"en","Help"}},"/docs/page.html"}}
            {2,0,0,{0,0,{0,0,{"ru","Связанная"}},"/docs/other.html"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                (
                    "docs/page.html",
                    r#"<html><head>
                        <link rel="stylesheet" href="v8help://service_book/service_style">
                    </head><body>
                        <h1>Справка&nbsp;по синтаксису</h1>
                        <p>Синтаксис: Функция &lt;Имя_функции&gt;</p>
                        <p><a href="other.html">Связанная страница</a></p>
                        <table><tr><th>Имя</th><th>Значение</th></tr><tr><td>ВЫБОР</td><td>CASE</td></tr></table>
                        <img src="assets/pic.png" alt="service image">
                    </body></html>"#
                        .as_bytes(),
                ),
                ("docs/other.html", b"<html><body>other</body></html>"),
            ],
        );
        let book = HbkBook::open(&source_path).expect("book must open");

        let page = BookExporter::new(&book)
            .markdown_page("/docs/page.html")
            .expect("TOC page must convert to Markdown");

        assert_eq!(page.html_path(), "docs/page.html");
        assert_eq!(page.title(), "Справка по синтаксису");
        let markdown = page.markdown();
        assert!(markdown.starts_with("# Справка по синтаксису\n"));
        assert!(markdown.contains("Функция <Имя_функции>"));
        assert!(markdown.contains("Связанная страница"));
        assert!(markdown.contains("ВЫБОР"));
        assert!(markdown.contains("CASE"));
        assert!(markdown.contains('|'));
        assert!(!markdown.contains("other.html"));
        assert!(!markdown.contains("assets/pic.png"));
        assert_no_raw_markdown_scaffolding(markdown);
    }

    #[test]
    fn rejects_markdown_conversion_for_non_toc_storage_page() {
        let workspace = TempWorkspace::new("markdown-non-toc");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Страница"}},"/docs/page.html"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                ("docs/page.html", b"<html><body>page</body></html>"),
                ("docs/unlisted.html", b"<html><body>unlisted</body></html>"),
            ],
        );
        let book = HbkBook::open(&source_path).expect("book must open");

        let error = BookExporter::new(&book)
            .markdown_page("docs/unlisted.html")
            .expect_err("non-TOC storage pages must not be converted as TOC pages");

        assert_eq!(
            error,
            BookExportError::TocPageNotFound {
                html_path: "docs/unlisted.html".to_string(),
            }
        );
    }

    #[test]
    fn markdown_page_loader_rewrites_links_from_raw_html() {
        let workspace = TempWorkspace::new("markdown-loader-links");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r##"{
            2
            {1,0,0,{0,0,{0,0,{"ru","Корень"}},"/docs/root.html"}}
            {2,0,0,{0,0,{0,0,{"ru","Цель"}},"/docs/target.html"}}
        }"##;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                (
                    "docs/root.html",
                    r##"<html><body>
                        <h1>Корень</h1>
                        <p><a href="target.html#Details">Цель</a></p>
                    </body></html>"##
                        .as_bytes(),
                ),
                (
                    "docs/target.html",
                    r##"<html><body><h1 id="Details">Цель</h1></body></html>"##.as_bytes(),
                ),
            ],
        );
        let book = HbkBook::open(&source_path).expect("book must open");
        let mut link_targets = HashMap::new();
        link_targets.insert(
            "docs/target.html".to_string(),
            PathBuf::from("target-page.md"),
        );
        let mut loader = BookExporter::new(&book)
            .markdown_page_loader()
            .expect("markdown page loader must open");

        let page = loader
            .linked_markdown_toc_page(
                "docs/root.html",
                "Корень",
                Path::new("root-page.md"),
                &link_targets,
                &source_book_link_ids(&book),
            )
            .expect("loader must convert raw HTML to linked Markdown");

        assert!(page.markdown().contains("[Цель](target-page.md#Details)"));
        assert!(!page.markdown().contains("target.html"));
    }

    #[test]
    fn markdown_page_loader_prefers_html_title_over_toc_title() {
        let workspace = TempWorkspace::new("markdown-loader-title");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","TOC title"}},"/docs/page.html"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![(
                "docs/page.html",
                r#"<html><head><title>HTML&nbsp;title</title></head><body><p>body</p></body></html>"#
                    .as_bytes(),
            )],
        );
        let book = HbkBook::open(&source_path).expect("book must open");
        let mut loader = BookExporter::new(&book)
            .markdown_page_loader()
            .expect("markdown page loader must open");

        let page = loader
            .linked_markdown_toc_page(
                "docs/page.html",
                "TOC title",
                Path::new("page.md"),
                &HashMap::new(),
                &source_book_link_ids(&book),
            )
            .expect("loader must convert page Markdown");

        assert!(
            page.markdown().starts_with("# HTML title\n"),
            "{}",
            page.markdown()
        );
        assert!(!page.markdown().starts_with("# TOC title\n"));
    }

    #[test]
    fn markdown_page_loader_keeps_missing_toc_page_as_heading_only() {
        let workspace = TempWorkspace::new("markdown-loader-missing");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Отсутствует"}},"/docs/missing.html"}}
        }"#;
        write_book_fixture_with_toc(&source_path, toc, Vec::new());
        let book = HbkBook::open(&source_path).expect("book must open");
        let mut loader = BookExporter::new(&book)
            .markdown_page_loader()
            .expect("markdown page loader must open");

        let page = loader
            .linked_markdown_toc_page(
                "docs/missing.html",
                "Отсутствует",
                Path::new("missing.md"),
                &HashMap::new(),
                &source_book_link_ids(&book),
            )
            .expect("missing TOC storage page must become heading-only Markdown");

        assert_eq!(page.markdown(), "# Отсутствует\n");
    }

    #[test]
    fn exports_markdown_toc_pages_under_deterministic_title_paths() {
        let workspace = TempWorkspace::new("markdown-toc-layout");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        let toc = r#"{
            5
            {1,0,4,2,3,4,5,{0,0,{0,0,{"ru","Справка"}{"en","Help"}},"/docs/root.html"}}
            {2,1,0,{0,0,{0,0,{"ru","Раздел"}{"en","Section"}},"/docs/child.html"}}
            {3,1,0,{0,0,{0,0,{"ru","Раздел"}{"en","Section"}},"/docs/child-two.html"}}
            {4,1,0,{0,0,{0,0,{"ru","Группа"}{"en","Group"}},""}}
            {5,1,0,{0,0,{0,0,{"ru","Ссылка HTML"}{"en","HTML link"}},"/objects/raw.html"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                (
                    "docs/root.html",
                    r#"<html><body>
                        <h1>Справка</h1>
                        <p><a HREF = "child.html">Раздел</a></p>
                        <p><a href="v8help://fmtdui/docs/child-two.html">Вторая</a></p>
                        <p><a href="v8help://otherbook/docs/child.html">Другая книга</a></p>
                        <p><a href="missing.html">Несуществующая</a></p>
                        <p><a href="https://example.com/help">Внешняя</a></p>
                        <img src="assets/pic.png" alt="Картинка">
                    </body></html>"#
                        .as_bytes(),
                ),
                (
                    "docs/child.html",
                    "<html><body><h1>Раздел</h1><p>Первый</p></body></html>".as_bytes(),
                ),
                (
                    "docs/child-two.html",
                    "<html><body><h1>Раздел</h1><p>Второй</p></body></html>".as_bytes(),
                ),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");

        let result = BookExporter::new(&book)
            .export(&request)
            .expect("markdown/toc export must succeed");

        let exported: Vec<_> = result
            .files()
            .iter()
            .map(|file| {
                file.path()
                    .strip_prefix(&output_root)
                    .expect("exported file must be under output root")
                    .to_path_buf()
            })
            .collect();
        assert_eq!(
            exported,
            vec![
                PathBuf::from("справка/index.md"),
                PathBuf::from("справка/раздел/index.md"),
                PathBuf::from("справка/раздел-2/index.md"),
                PathBuf::from("справка/группа/index.md"),
                PathBuf::from("справка/ссылка-html/index.md"),
            ]
        );

        let root_markdown = fs::read_to_string(output_root.join("справка/index.md"))
            .expect("root page must be exported");
        assert!(
            root_markdown.contains("[Раздел](раздел/index.md)"),
            "{root_markdown}"
        );
        assert!(
            root_markdown.contains("[Вторая](раздел-2/index.md)"),
            "{root_markdown}"
        );
        assert!(root_markdown.contains("Другая книга"));
        assert!(!root_markdown.contains("[Другая книга]"));
        assert!(!root_markdown.contains("otherbook"));
        assert!(root_markdown.contains("Несуществующая"));
        assert!(!root_markdown.contains("child.html"));
        assert!(!root_markdown.contains("missing.html"));
        assert!(root_markdown.contains("[Внешняя](https://example.com/help)"));
        assert!(!root_markdown.contains("assets/pic.png"));
        assert_no_raw_markdown_scaffolding(&root_markdown);

        let heading_only = fs::read_to_string(output_root.join("справка/группа/index.md"))
            .expect("empty TOC path page must be exported");
        assert_eq!(heading_only, "# Группа\n");
        let missing_storage_page =
            fs::read_to_string(output_root.join("справка/ссылка-html/index.md"))
                .expect("missing storage TOC page must be exported as heading-only Markdown");
        assert_eq!(missing_storage_page, "# Ссылка HTML\n");
    }

    #[test]
    fn exports_shared_content_node_placeholders_with_each_toc_title() {
        let workspace = TempWorkspace::new("markdown-content-node-placeholder");
        let source_path = workspace.path().join("shclang_ru.hbk");
        let toc = r#"{
            4
            {1,0,2,2,3,{0,0,{0,0,{"ru","Встроенный язык"}{"en","Language"}},""}}
            {2,1,0,{0,0,{0,0,{"ru","Общее описание встроенного языка"}{"en","General"}},"_CONTENTS_NODE_fileConf"}}
            {3,1,1,4,{0,0,{0,0,{"ru","Общие объекты"}{"en","Common objects"}},"_CONTENTS_NODE_fileConf"}}
            {4,3,0,{0,0,{0,0,{"ru","Основные понятия XBASE"}{"en","XBASE"}},"MainXBase"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                (
                    "_CONTENTS_NODE_fileConf",
                    b"\xef\xbb\xbf<html><body></body></html>",
                ),
                (
                    "MainXBase",
                    "<html><body><h1>Основные понятия XBASE</h1><p>Содержательная страница</p></body></html>"
                        .as_bytes(),
                ),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");

        BookExporter::new(&book)
            .export(&request)
            .expect("markdown/toc export must succeed");

        let general = fs::read_to_string(
            output_root.join("встроенный-язык/общее-описание-встроенного-языка/index.md"),
        )
        .expect("first placeholder page must be exported");
        let common = fs::read_to_string(output_root.join("встроенный-язык/общие-объекты/index.md"))
            .expect("second placeholder page must be exported");
        let real = fs::read_to_string(
            output_root.join("встроенный-язык/общие-объекты/основные-понятия-xbase/index.md"),
        )
        .expect("real child page must be exported");

        assert_eq!(general, "# Общее описание встроенного языка\n");
        assert_eq!(common, "# Общие объекты\n");
        assert!(real.contains("# Основные понятия XBASE"));
        assert!(real.contains("Содержательная страница"));
    }

    #[test]
    fn converts_single_cell_courier_tables_to_markdown_code_blocks() {
        let workspace = TempWorkspace::new("markdown-code-table");
        let source_path = workspace.path().join("shclang_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Работа с пакетными запросами"}{"en","Batch"}},"WorkinWithBath"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![(
                "WorkinWithBath",
                r##"<html><body>
                    <h1>Работа с пакетными запросами</h1>
                    <p>Например:</p>
                    <table width="100%" bgcolor="#f7f7f7"><tbody><tr><td>
                        <font face="Courier New">Запрос&nbsp;=&nbsp;Новый&nbsp;Запрос;<br>
                        Запрос.Текст = "ВЫБРАТЬ<br>
                        &nbsp;&nbsp;&nbsp;&nbsp;|&nbsp;УчетНоменклатуры.Номенклатура<br>
                        &nbsp;&nbsp;&nbsp;&nbsp;|";<br><br>
                        Результат=Запрос.Выполнить();</font>
                    </td></tr></tbody></table>
                </body></html>"##
                    .as_bytes(),
            )],
        );
        let book = HbkBook::open(&source_path).expect("book must open");

        let page = BookExporter::new(&book)
            .markdown_page("WorkinWithBath")
            .expect("TOC page must convert to Markdown");
        let markdown = page.markdown();

        assert!(markdown.contains("```bsl"), "{markdown}");
        assert!(markdown.contains("Запрос = Новый Запрос;"), "{markdown}");
        assert!(markdown.contains("Запрос.Текст = \"ВЫБРАТЬ"), "{markdown}");
        assert!(
            markdown.contains("    | УчетНоменклатуры.Номенклатура"),
            "{markdown}"
        );
        assert!(
            markdown.contains("Результат=Запрос.Выполнить();"),
            "{markdown}"
        );
        assert!(!markdown.contains("| Запрос = Новый Запрос"), "{markdown}");
        assert_no_raw_markdown_scaffolding(markdown);
    }

    #[test]
    fn converts_courier_query_blockquotes_to_sdbl_code_blocks() {
        let workspace = TempWorkspace::new("markdown-sdbl-blockquote");
        let source_path = workspace.path().join("shclang_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Работа с временными таблицами"}{"en","Temp tables"}},"Work with temp table"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![(
                "Work with temp table",
                r##"<html><body>
                    <h1>Работа с временными таблицами</h1>
                    <blockquote style="MARGIN-RIGHT: 0px" dir="ltr"><p><font face="Courier New">ВЫБРАТЬ<br>&nbsp;&nbsp; Код,<br>&nbsp;&nbsp; Наименование<br>ПОМЕСТИТЬ ВременнаяТаблица<br>ИЗ Справочник.Номенклатура</font></p></blockquote>
                </body></html>"##
                    .as_bytes(),
            )],
        );
        let book = HbkBook::open(&source_path).expect("book must open");

        let page = BookExporter::new(&book)
            .markdown_page("Work with temp table")
            .expect("TOC page must convert to Markdown");
        let markdown = page.markdown();

        assert!(markdown.contains("```sdbl"), "{markdown}");
        assert!(markdown.contains("ВЫБРАТЬ\n   Код,"), "{markdown}");
        assert!(
            markdown.contains("ПОМЕСТИТЬ ВременнаяТаблица"),
            "{markdown}"
        );
        assert!(
            markdown.contains("ИЗ Справочник.Номенклатура"),
            "{markdown}"
        );
        assert!(!markdown.contains("> ВЫБРАТЬ"), "{markdown}");
        assert_no_raw_markdown_scaffolding(markdown);
    }

    #[test]
    fn converts_layout_blockquote_tables_to_readable_quote_lines() {
        let workspace = TempWorkspace::new("markdown-layout-blockquote-table");
        let source_path = workspace.path().join("1cv8_ru.hbk");
        let toc = r#"{
            1
            {1,0,0,{0,0,{0,0,{"ru","Запуск 1С:Предприятие 8 и параметры запуска"}{"en","Startup"}},"ZIF"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![(
                "ZIF",
                r##"<html><body>
                    <h1>Запуск 1С:Предприятие 8 и параметры запуска</h1>
                    <p>Интерактивная программа запуска откроет список информационных баз.</p>
                    <blockquote style="MARGIN-RIGHT: 0px" dir="ltr">
                    <table id="table5" border="1"><tbody>
                        <tr><td bgcolor="#fffef0" colspan="2">&nbsp;Программа запуска - <strong>1CEStart</strong></td></tr>
                        <tr><td></td><td>&nbsp;&nbsp;</td></tr>
                    </tbody></table>
                    <table id="table6" border="1"><tbody>
                        <tr><td></td><td bgcolor="#fffef0">&nbsp;Интерактивная программа запуска - <strong>1Cv8s</strong></td></tr>
                    </tbody></table>
                    <table id="table7" border="1"><tbody>
                        <tr><td></td><td>&nbsp;</td></tr>
                    </tbody></table>
                    <table id="table8" border="1"><tbody>
                        <tr><td></td><td bgcolor="#fffef0">&nbsp;Клиентское приложение</td></tr>
                    </tbody></table>
                    </blockquote>
                    <p>Программе запуска можно указывать различные параметры командной строки.</p>
                </body></html>"##
                    .as_bytes(),
            )],
        );
        let book = HbkBook::open(&source_path).expect("book must open");

        let page = BookExporter::new(&book)
            .markdown_page("ZIF")
            .expect("TOC page must convert to Markdown");
        let markdown = page.markdown();

        assert!(
            markdown.contains("> Программа запуска - 1CEStart"),
            "{markdown}"
        );
        assert!(
            markdown.contains("> Интерактивная программа запуска - 1Cv8s"),
            "{markdown}"
        );
        assert!(markdown.contains("> Клиентское приложение"), "{markdown}");
        assert!(!markdown.contains("> |"), "{markdown}");
        assert!(!markdown.contains("> | ---"), "{markdown}");
        assert_no_raw_markdown_scaffolding(markdown);
    }

    #[test]
    fn preserves_internal_link_fragments_in_markdown_targets() {
        let workspace = TempWorkspace::new("markdown-link-fragments");
        let source_path = workspace.path().join("shclang_ru.hbk");
        let toc = r#"{
            2
            {1,0,0,{0,0,{0,0,{"ru","Основные понятия XBASE"}{"en","XBASE"}},"MainXBase"}}
            {2,0,0,{0,0,{0,0,{"ru","Другая страница"}{"en","Other"}},"OtherPage"}}
        }"#;
        write_book_fixture_with_toc(
            &source_path,
            toc,
            vec![
                (
                    "MainXBase",
                    r##"<html><body>
                        <h1>Основные понятия XBASE</h1>
                        <p><a href="#FieldsRecords">Поля и записи</a></p>
                        <p><a href="OtherPage#Details">Другая страница</a></p>
                        <p><a href="#DirectId">Заголовок с id</a></p>
                        <p><a href="#SecondParams">Вторые параметры</a></p>
                        <h2><a name="FieldsRecords">Поля и записи</a></h2>
                        <h2 id="DirectId">Заголовок с id</h2>
                        <h2><a name="FirstParams"></a>Параметры</h2>
                        <h2><a name="SecondParams"></a>Параметры</h2>
                    </body></html>"##
                        .as_bytes(),
                ),
                (
                    "OtherPage",
                    r##"<html><body><h1>Другая страница</h1><h2><a name="Details">Детали</a></h2></body></html>"##
                        .as_bytes(),
                ),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");

        BookExporter::new(&book)
            .export(&request)
            .expect("markdown/toc export must succeed");

        let markdown = fs::read_to_string(output_root.join("основные-понятия-xbase/index.md"))
            .expect("Markdown page must be exported");
        let other_markdown = fs::read_to_string(output_root.join("другая-страница/index.md"))
            .expect("linked Markdown page must be exported");

        assert!(markdown.contains("[Поля и записи](index.md#FieldsRecords)"));
        assert!(markdown.contains("[Другая страница](../другая-страница/index.md#Details)"));
        assert!(markdown.contains("<a id=\"FieldsRecords\"></a>\n## Поля и записи"));
        assert!(markdown.contains("<a id=\"DirectId\"></a>\n## Заголовок с id"));
        assert!(other_markdown.contains("<a id=\"Details\"></a>\n## Детали"));
        assert!(!markdown.contains("[Поля и записи](index.md)"));
        assert_duplicate_heading_anchors_stay_with_their_source_heading(&markdown);
    }

    #[test]
    fn real_representative_pages_convert_to_readable_markdown_when_platform_books_exist() {
        struct Case<'a> {
            book_path: &'a str,
            html_path: &'a str,
            expected: &'a [&'a str],
        }

        let cases = [
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/1cv8_ru.hbk",
                html_path: "ZIF",
                expected: &[
                    "# Запуск 1С:Предприятие 8 и параметры запуска",
                    "> Программа запуска - 1CEStart",
                    "> Интерактивная программа запуска - 1Cv8s",
                    "> Клиентское приложение",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/dcsui_ru.hbk",
                html_path: "PresentSKD",
                expected: &[
                    "# Двуязычное представление ключевых слов системы компоновки данных",
                    "ВЫБОР",
                    "CASE",
                    "|",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/dcsui_ru.hbk",
                html_path: "SKD_Functions_Strings",
                expected: &[
                    "# Работа со строками",
                    "ДлинаСтроки",
                    "StringLength",
                    "ДлинаСтроки(<Строка>)",
                    "Подстрока",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shlang_ru.hbk",
                html_path: "def_Func",
                expected: &[
                    "# Функция",
                    "Синтаксис",
                    "Функция <Имя_функции>",
                    "Возврат <Возвращаемое значение>",
                    "КонецФункции",
                    "Ждать",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shlang_ru.hbk",
                html_path: "struct_IfThenElif",
                expected: &[
                    "# Если",
                    "Если <Логическое выражение> Тогда",
                    "ИначеЕсли <Логическое выражение> Тогда",
                    "КонецЕсли",
                    "логического выражения",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk",
                html_path: "WorkinWithBath",
                expected: &[
                    "# Работа с пакетными запросами",
                    "```bsl",
                    "Запрос = Новый Запрос;",
                    "    | УчетНоменклатурыОстаткиИОбороты.Номенклатура,",
                    "Результат=Запрос.Выполнить();",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk",
                html_path: "Work with temp table",
                expected: &[
                    "# Работа с временными таблицами",
                    "<a id=\"Manager\"></a>\n## Менеджер временных таблиц",
                    "<a id=\"Create\"></a>\n## Создание временных таблиц",
                    "<a id=\"Used\"></a>\n## Использование временных таблиц",
                    "<a id=\"Delete\"></a>\n## Удаление временных таблиц",
                    "```sdbl",
                    "ВЫБРАТЬ\n   Код,",
                    "ПОМЕСТИТЬ ВременнаяТаблица",
                    "ИЗ Справочник.Номенклатура",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shquery_ru.hbk",
                html_path: "syntax_diagram.html",
                expected: &[
                    "# Синтаксическая диаграмма конструкций языка запросов",
                    "<Конструкция языка>",
                    "ЭТО_КЛЮЧЕВОЕ_СЛОВО",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/shquery_ru.hbk",
                html_path: "SUM",
                expected: &["# Агрегатная функция СУММА", "Агрегатные функции", "NULL"],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk",
                html_path: "form_formattedstringedit",
                expected: &[
                    "# Конструктор строк на разных языках",
                    "интерфейсных языков",
                    "Обычная строка",
                    "Форматированная строка",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/htmlui_ru.hbk",
                html_path: "form_addtable",
                expected: &[
                    "# Вставка таблицы",
                    "HTML-документы можно вставлять таблицы",
                    "Таблица - Вставить таблицу",
                    "Ячейки можно объединять и делить",
                ],
            },
            Case {
                book_path: "/opt/1cv8/x86_64/8.5.1.1150/moxelui_ru.hbk",
                html_path: "form_moxelpagesetupdialog",
                expected: &[
                    "# Параметры страницы табличного документа",
                    "Файл - Параметры страницы",
                    "Колонтитулы",
                    "Авто",
                ],
            },
        ];

        for case in cases {
            let book_path = Path::new(case.book_path);
            if !book_path.exists() {
                continue;
            }

            let book = HbkBook::open(book_path).expect("platform HBK must open");
            let page = BookExporter::new(&book)
                .markdown_page(case.html_path)
                .expect("real TOC page must convert to Markdown");
            let markdown = page.markdown();

            for expected in case.expected {
                assert!(
                    markdown.contains(expected),
                    "expected Markdown for {} {} to contain {expected:?}; got:\n{markdown}",
                    case.book_path,
                    case.html_path
                );
            }
            if case.html_path == "ZIF" {
                assert!(!markdown.contains("> |"), "{markdown}");
                assert!(!markdown.contains("> | ---"), "{markdown}");
            }
            assert_no_raw_markdown_scaffolding(markdown);
        }
    }

    #[test]
    fn real_shclang_content_node_pages_keep_toc_headings_when_platform_book_exists() {
        let book_path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk");
        if !book_path.exists() {
            return;
        }

        let workspace = TempWorkspace::new("real-shclang-content-nodes");
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(book_path).expect("platform HBK must open");
        let request = BookExportRequest::new(
            book_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");

        BookExporter::new(&book)
            .export(&request)
            .expect("markdown/toc export must succeed");

        let common = fs::read_to_string(output_root.join("встроенный-язык/общие-объекты/index.md"))
            .expect("common objects placeholder page must be exported");
        let query =
            fs::read_to_string(output_root.join("встроенный-язык/работа-с-запросами/index.md"))
                .expect("query placeholder page must be exported");

        assert_eq!(common, "# Общие объекты\n");
        assert_eq!(query, "# Работа с запросами\n");
    }

    #[test]
    fn real_shclang_xbase_page_preserves_internal_link_fragments_when_platform_book_exists() {
        let book_path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk");
        if !book_path.exists() {
            return;
        }

        let workspace = TempWorkspace::new("real-shclang-link-fragments");
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(book_path).expect("platform HBK must open");
        let request = BookExportRequest::new(
            book_path,
            output_root.clone(),
            BookExportFormat::Markdown,
            BookExportHierarchy::Toc,
        )
        .expect("markdown/toc request must be valid");

        BookExporter::new(&book)
            .export(&request)
            .expect("markdown/toc export must succeed");

        let markdown = fs::read_to_string(
            output_root.join("встроенный-язык/общие-объекты/xbase/основные-понятия-xbase/index.md"),
        )
        .expect("XBase Markdown page must be exported");

        assert!(markdown.contains("[Поля и записи](index.md#FieldsRecords)"));
        assert!(markdown.contains("[Работа с индексными файлами](index.md#WorkWithIndexFile)"));
        assert!(markdown.contains("[Ограничения](index.md#constraint)"));
        assert!(markdown.contains("<a id=\"FieldsRecords\"></a>"));
        assert!(markdown.contains("<a id=\"WorkWithIndexFile\"></a>"));
        assert!(markdown.contains("<a id=\"constraint\"></a>"));
        assert!(!markdown.contains("[Поля и записи](index.md)"));
    }

    #[test]
    fn exports_raw_storage_files_under_normalized_paths() {
        let workspace = TempWorkspace::new("raw-success");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        write_book_fixture(
            &source_path,
            vec![
                ("docs/page.html", b"<html>page</html>".as_ref()),
                ("assets/./style.css", b"body {}".as_ref()),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");

        let result = BookExporter::new(&book)
            .export(&request)
            .expect("raw/raw export must succeed");

        assert_eq!(
            fs::read(output_root.join("docs/page.html")).expect("page must be exported"),
            b"<html>page</html>"
        );
        assert_eq!(
            fs::read(output_root.join("assets/style.css")).expect("asset must be exported"),
            b"body {}"
        );
        assert_eq!(result.output_root(), output_root.as_path());
        let exported: Vec<_> = result
            .files()
            .iter()
            .map(|file| {
                (
                    file.path()
                        .strip_prefix(&output_root)
                        .expect("exported file must be under output root")
                        .to_path_buf(),
                    file.bytes_written(),
                )
            })
            .collect();
        assert_eq!(
            exported,
            vec![
                (
                    PathBuf::from("docs/page.html"),
                    b"<html>page</html>".len() as u64,
                ),
                (PathBuf::from("assets/style.css"), b"body {}".len() as u64),
            ]
        );
    }

    #[test]
    fn rejects_unsafe_storage_paths_before_writing() {
        assert_rejects_unsafe_storage_path("../escape.txt", StoragePathError::ParentSegment);
        assert_rejects_unsafe_storage_path("/escape.txt", StoragePathError::Absolute);
        assert_rejects_unsafe_storage_path("C:/escape.txt", StoragePathError::WindowsPrefix);
        assert_rejects_unsafe_storage_path("dir\\escape.txt", StoragePathError::BackslashSeparator);
    }

    #[test]
    fn rejects_duplicate_normalized_storage_paths_before_writing() {
        let workspace = TempWorkspace::new("raw-duplicate");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        write_book_fixture(
            &source_path,
            vec![
                ("docs/./page.html", b"first".as_ref()),
                ("docs/page.html", b"second".as_ref()),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");

        let error = BookExporter::new(&book)
            .export(&request)
            .expect_err("duplicate normalized paths must be rejected");

        assert_eq!(
            error,
            BookExportError::DuplicateStoragePath {
                entry_name: "docs/page.html".to_string(),
                normalized_path: PathBuf::from("docs/page.html"),
            }
        );
        assert!(
            !output_root.exists(),
            "unsafe plan validation must finish before filesystem writes"
        );
    }

    #[test]
    fn rejects_file_directory_storage_path_collisions_before_writing() {
        let workspace = TempWorkspace::new("raw-collision");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        write_book_fixture(
            &source_path,
            vec![
                ("docs", b"file".as_ref()),
                ("docs/page.html", b"page".as_ref()),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");

        let error = BookExporter::new(&book)
            .export(&request)
            .expect_err("file/directory path collision must be rejected");

        assert_eq!(
            error,
            BookExportError::StoragePathCollision {
                entry_name: "docs/page.html".to_string(),
                normalized_path: PathBuf::from("docs/page.html"),
                existing_path: PathBuf::from("docs"),
            }
        );
        assert!(
            !output_root.exists(),
            "path collision validation must finish before filesystem writes"
        );
    }

    #[test]
    fn rejects_request_source_path_mismatch_before_writing() {
        let workspace = TempWorkspace::new("source-mismatch");
        let request_source_path = workspace.path().join("fmtdui_ru.hbk");
        let opened_book_path = workspace.path().join("htmlui_ru.hbk");
        write_book_fixture(
            &request_source_path,
            vec![("docs/request.html", b"request".as_ref())],
        );
        write_book_fixture(
            &opened_book_path,
            vec![("docs/opened.html", b"opened".as_ref())],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&opened_book_path).expect("book must open");
        let request = BookExportRequest::new(
            request_source_path.clone(),
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");

        let error = BookExporter::new(&book)
            .export(&request)
            .expect_err("source path mismatch must be rejected");

        assert_eq!(
            error,
            BookExportError::SourcePathMismatch {
                request_source_path,
                book_path: opened_book_path,
            }
        );
        assert!(
            !output_root.exists(),
            "source mismatch validation must finish before filesystem writes"
        );
    }

    fn assert_rejects_unsafe_storage_path(entry_name: &str, reason: StoragePathError) {
        let workspace = TempWorkspace::new("raw-unsafe");
        let source_path = workspace.path().join("fmtdui_ru.hbk");
        write_book_fixture(
            &source_path,
            vec![
                ("docs/page.html", b"ok".as_ref()),
                (entry_name, b"escape".as_ref()),
            ],
        );
        let output_root = workspace.path().join("out");
        let book = HbkBook::open(&source_path).expect("book must open");
        let request = BookExportRequest::new(
            source_path,
            output_root.clone(),
            BookExportFormat::Raw,
            BookExportHierarchy::Raw,
        )
        .expect("raw/raw request must be valid");

        let error = BookExporter::new(&book)
            .export(&request)
            .expect_err("unsafe storage path must be rejected");

        assert_eq!(
            error,
            BookExportError::UnsafeStoragePath {
                entry_name: entry_name.to_string(),
                reason,
            }
        );
        assert!(
            !output_root.exists(),
            "unsafe path validation must finish before filesystem writes"
        );
    }

    fn write_book_fixture(path: &Path, storage_entries: Vec<(&str, &[u8])>) {
        fs::write(
            path,
            fixture_container(vec![
                (
                    "Book",
                    Some(
                        r#"{1,"Interface", {1,2,{"ru","fmtdui"}}, 1, "tag", {0,0}, 0}"#
                            .as_bytes()
                            .to_vec(),
                    ),
                ),
                ("PackBlock", None),
                ("FileStorage", Some(zip_entries(storage_entries))),
            ]),
        )
        .expect("fixture HBK must be written");
    }

    fn write_book_fixture_with_toc(path: &Path, toc: &str, storage_entries: Vec<(&str, &[u8])>) {
        fs::write(
            path,
            fixture_container(vec![
                (
                    "Book",
                    Some(
                        r#"{1,"Interface", {1,2,{"ru","fmtdui"}}, 1, "tag", {0,0}, 0}"#
                            .as_bytes()
                            .to_vec(),
                    ),
                ),
                ("PackBlock", Some(zip_bytes("toc.txt", toc.as_bytes()))),
                ("FileStorage", Some(zip_entries(storage_entries))),
            ]),
        )
        .expect("fixture HBK must be written");
    }

    fn assert_no_raw_markdown_scaffolding(markdown: &str) {
        for forbidden in [
            "<html",
            "<body",
            "<p",
            "<a href",
            "<a name",
            "<h1",
            "<h2",
            "<table",
            "<tr",
            "<td",
            "<ul",
            "<li",
            "<div",
            "<span",
            "</html",
            "</body",
            "</p",
            "</h",
            "</table",
            "</tr",
            "</td",
            "</ul",
            "</li",
            "</div",
            "</span",
            "&nbsp;",
            "v8help://service_book/service_style",
            ".hbk",
            ".html",
            "toc_index",
            "toc-index",
        ] {
            assert!(
                !markdown.contains(forbidden),
                "Markdown must not contain raw service/provenance fragment {forbidden:?}:\n{markdown}"
            );
        }
    }

    fn assert_duplicate_heading_anchors_stay_with_their_source_heading(markdown: &str) {
        let first_heading = markdown
            .find("## Параметры")
            .expect("first duplicate heading must exist");
        let second_heading = markdown[first_heading + "## Параметры".len()..]
            .find("## Параметры")
            .map(|offset| first_heading + "## Параметры".len() + offset)
            .expect("second duplicate heading must exist");
        let first_anchor = markdown
            .find("<a id=\"FirstParams\"></a>")
            .expect("first duplicate heading anchor must exist");
        let second_anchor = markdown
            .find("<a id=\"SecondParams\"></a>")
            .expect("second duplicate heading anchor must exist");

        assert!(first_anchor < first_heading, "{markdown}");
        assert!(
            first_heading < second_anchor && second_anchor < second_heading,
            "{markdown}"
        );
    }

    struct TempWorkspace {
        path: PathBuf,
    }

    impl TempWorkspace {
        fn new(name: &str) -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "v8-context-hbk-book-export-test-{name}-{}-{}",
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
