use std::collections::BTreeSet;
use std::convert::Infallible;
use std::path::{Path, PathBuf};

use super::*;
use crate::catalog::collect_catalog_pages;
use crate::reader::{parse_extraction_pages_into, query_table_member_owner};
use hbk_book::HbkBook;
use hbk_book::Toc;
use hbk_docs::{PageContent, parse_page_html};

#[derive(Default)]
struct RecordingSink {
    seen: Vec<String>,
    events: Vec<GlobalContextEvent>,
    platform_types: Vec<PlatformType>,
    query_tables: Vec<QueryTable>,
    type_properties: Vec<PlatformProperty>,
    diagnostics: Vec<SyntaxHelperDiagnostic>,
}

impl RecordingSink {
    fn push_name(&mut self, kind: &str, name: &LocalizedName) {
        self.seen.push(format!("{kind}:{}", name.primary));
    }
}

impl SyntaxHelperSink for RecordingSink {
    type Error = Infallible;

    fn global_context(&mut self, record: GlobalContext) -> Result<(), Self::Error> {
        self.push_name("global_context", &record.name);
        Ok(())
    }

    fn global_method(&mut self, record: GlobalMethod) -> Result<(), Self::Error> {
        self.push_name("global_method", &record.name);
        Ok(())
    }

    fn global_property(&mut self, record: GlobalProperty) -> Result<(), Self::Error> {
        self.push_name("global_property", &record.name);
        Ok(())
    }

    fn global_context_event(&mut self, record: GlobalContextEvent) -> Result<(), Self::Error> {
        self.push_name("global_context_event", &record.name);
        self.events.push(record);
        Ok(())
    }

    fn platform_type(&mut self, record: PlatformType) -> Result<(), Self::Error> {
        self.push_name("platform_type", &record.name);
        self.platform_types.push(record);
        Ok(())
    }

    fn query_table(&mut self, record: QueryTable) -> Result<(), Self::Error> {
        self.seen.push(format!("query_table:{}", record.name));
        self.query_tables.push(record);
        Ok(())
    }

    fn type_method(&mut self, record: PlatformMethod) -> Result<(), Self::Error> {
        self.push_name("type_method", &record.name);
        Ok(())
    }

    fn type_property(&mut self, record: PlatformProperty) -> Result<(), Self::Error> {
        self.push_name("type_property", &record.name);
        self.type_properties.push(record);
        Ok(())
    }

    fn table_field(&mut self, record: QueryTableField) -> Result<(), Self::Error> {
        self.seen.push(format!(
            "table_field:{}:{}",
            record.owner.primary, record.name
        ));
        Ok(())
    }

    fn table_parameter(&mut self, record: QueryTableParameter) -> Result<(), Self::Error> {
        self.seen.push(format!(
            "table_parameter:{}:{}",
            record.owner.primary, record.name
        ));
        Ok(())
    }

    fn constructor(&mut self, record: Constructor) -> Result<(), Self::Error> {
        self.push_name("constructor", &record.name);
        Ok(())
    }

    fn enum_definition(&mut self, record: EnumDefinition) -> Result<(), Self::Error> {
        self.push_name("enum", &record.name);
        Ok(())
    }

    fn enum_value(&mut self, record: EnumValue) -> Result<(), Self::Error> {
        self.push_name("enum_value", &record.name);
        Ok(())
    }

    fn diagnostic(&mut self, record: SyntaxHelperDiagnostic) -> Result<(), Self::Error> {
        self.seen.push(format!("diagnostic:{}", record.code));
        self.diagnostics.push(record);
        Ok(())
    }
}

#[test]
fn discovers_roots_and_traverses_catalogs_from_fixture_toc() {
    let toc = fixture_toc();
    let discovery =
        discover_roots_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
            Ok(fixture_content(&toc, html_path))
        })
        .expect("root discovery must succeed");

    assert!(discovery.has_kind(RootSectionKind::GlobalContext));
    assert!(discovery.has_kind(RootSectionKind::EnumCatalog));
    assert!(discovery.has_kind(RootSectionKind::TypeObjectCatalog));
    assert_eq!(discovery.roots.len(), 3);

    let classes = discovery
        .roots
        .iter()
        .flat_map(|root| root.pages.iter().map(|page| page.class))
        .collect::<BTreeSet<_>>();
    assert!(classes.contains(&PageClass::GlobalMethod));
    assert!(classes.contains(&PageClass::GlobalProperty));
    assert!(classes.contains(&PageClass::Enum));
    assert!(classes.contains(&PageClass::EnumValue));
    assert!(classes.contains(&PageClass::ObjectType));
    assert!(classes.contains(&PageClass::ObjectMethod));
    assert!(classes.contains(&PageClass::ObjectProperty));
    assert!(classes.contains(&PageClass::Constructor));

    assert_eq!(discovery.diagnostics.len(), 1);
    assert_eq!(discovery.diagnostics[0].code, "UNKNOWN_PAGE_CLASS");
    assert_eq!(
        discovery.diagnostics[0].severity,
        DiagnosticSeverity::Warning
    );
    assert_eq!(
        discovery.diagnostics[0].source.hbk_path,
        Path::new("shcntx_ru.hbk")
    );
    assert_eq!(discovery.diagnostics[0].source.locale, "ru");
    assert_eq!(
        discovery.diagnostics[0].source.toc_path.as_deref(),
        Some("3")
    );
    assert_eq!(
        discovery.diagnostics[0].source.html_path,
        "objects/unknown.html"
    );
    assert_eq!(
        discovery.diagnostics[0].source.page_title,
        "Неизвестный раздел"
    );
    assert_eq!(discovery.diagnostics[0].parser_stage, "root_discovery");
}

#[test]
fn supports_event_and_table_audit_families_and_keeps_toc_only_gap_diagnostic() {
    let toc = Toc::parse(
        r#"{
                10
                {1,0,3,2,3,4,{0,0,{0,0,{"ru","Глобальный контекст"}},"/objects/Global context.html"}}
                {2,1,0,{0,0,{0,0,{"ru","ПолучитьОбщуюПалитруЦветовЗначковТаблицы"}},"/objects/Global context/GetCommonTableIconsColorPalette6560.html"}}
                {3,1,1,5,{0,0,{0,0,{"ru","События"}},"/objects/Global context/events/catalog375.html"}}
                {4,0,3,6,7,9,{0,0,{0,0,{"ru","Универсальные коллекции значений"}},"/objects/catalog234.html"}}
                {5,3,0,{0,0,{0,0,{"ru","ПередЗавершениемРаботыСистемы"}},"/objects/Global context/events/catalog375/BeforeExit378.html"}}
                {6,4,0,{0,0,{0,0,{"ru","Массив"}},"/objects/catalog234/Array.html"}}
                {7,4,1,8,{0,0,{0,0,{"ru","Таблица активности"}},"/tables/catalog1/table2.html"}}
                {8,7,0,{0,0,{0,0,{"ru","Активность"}},"/tables/catalog1/table2/fields/Active4.html"}}
                {9,4,1,10,{0,0,{0,0,{"ru","Таблица параметров"}},"/tables/catalog1/table3.html"}}
                {10,9,0,{0,0,{0,0,{"ru","Параметр"}},"/tables/catalog1/table3/params/param1.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");

    let discovery =
        discover_roots_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
            Ok(fixture_content_from_raw(
                &toc,
                "shcntx_ru.hbk",
                "ru",
                html_path,
                r#"<html><body><h1 class="V8SH_pagetitle">Раздел</h1></body></html>"#,
            ))
        })
        .expect("root discovery must succeed");

    let codes = discovery
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    assert_eq!(codes, vec!["UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE"]);
    assert!(discovery.diagnostics.iter().all(|diagnostic| {
        diagnostic.severity == DiagnosticSeverity::Warning
            && diagnostic.parser_stage == "root_discovery"
            && diagnostic.source.toc_path.is_some()
            && !diagnostic.message.is_empty()
    }));

    let classes = discovery
        .roots
        .iter()
        .flat_map(|root| root.pages.iter().map(|page| page.class))
        .collect::<BTreeSet<_>>();
    assert!(classes.contains(&PageClass::ModuleEvent));
    assert!(classes.contains(&PageClass::QueryTableField));
    assert!(classes.contains(&PageClass::QueryTableParameter));

    let mut sink = RecordingSink::default();
    parse_extraction_pages_into(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc,
        discovery,
        |html_path| {
            Ok(fixture_content_from_raw(
                &toc,
                "shcntx_ru.hbk",
                "ru",
                html_path,
                r#"<html><body><h1 class="V8SH_pagetitle">Раздел</h1></body></html>"#,
            ))
        },
        &mut sink,
    )
    .expect("fixture extraction must succeed");
    assert!(
        sink.seen
            .contains(&"table_field:Таблица активности:Раздел".to_string())
    );
    assert!(
        sink.seen
            .contains(&"table_parameter:Таблица параметров:Раздел".to_string())
    );
}

#[test]
fn parses_global_context_event_page() {
    let toc = Toc::parse(
        r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","ПередЗавершениемРаботыСистемы"}},"/objects/Global context/events/catalog375/BeforeExit378.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let content = fixture_content_from_raw(
        &toc,
        "shcntx_ru.hbk",
        "ru",
        "objects/Global context/events/catalog375/BeforeExit378.html",
        r#"<html><body><h1 class="V8SH_pagetitle">Глобальный контекст.ПередЗавершениемРаботыСистемы (Global context.BeforeExit)</h1><p class="V8SH_title">Глобальный контекст (Global context)</p><p class="V8SH_heading">ПередЗавершениемРаботыСистемы (BeforeExit)</p><p class="V8SH_chapter">Синтаксис:</p>ПередЗавершениемРаботыСистемы(&lt;Отказ&gt;)<p class="V8SH_chapter">Параметры:</p><div class="V8SH_rubric">&lt;Отказ&gt;</div>Тип: <a href="v8help://SyntaxHelperLanguage/def_Boolean">Булево</a>. Признак отказа.<p class="V8SH_chapter">Описание:</p><p>Возникает перед завершением работы.</p><p class="V8SH_chapter">Доступность:</p><p>Тонкий клиент, сервер.</p></body></html>"#,
    );

    let event = parse_global_context_event(
        &content,
        source("objects/Global context/events/catalog375/BeforeExit378.html"),
    );

    assert_eq!(event.name.primary, "ПередЗавершениемРаботыСистемы");
    assert_eq!(event.name.alias.as_deref(), Some("BeforeExit"));
    assert_eq!(
        event.signatures[0].text,
        "ПередЗавершениемРаботыСистемы(<Отказ>)"
    );
    assert_eq!(event.signatures[0].parameters.len(), 1);
    assert_eq!(event.signatures[0].parameters[0].name, "Отказ");
    assert!(event.signatures[0].parameters[0].required);
    assert_eq!(
        event.signatures[0].parameters[0].type_refs[0].name,
        "Булево"
    );
    assert_eq!(
        event.description.as_deref(),
        Some("Возникает перед завершением работы.")
    );
    assert_eq!(
        event.facts.availability.contexts,
        vec![AvailabilityContext::ThinClient, AvailabilityContext::Server]
    );
}

#[test]
fn parses_query_table_field_and_parameter_pages() {
    let toc = Toc::parse(
        r#"{
                2
                {1,0,0,{0,0,{0,0,{"ru","Представление"}},"/tables/table58/fields/Presentation464.html"}}
                {2,0,0,{0,0,{0,0,{"ru","Первые"}},"/tables/catalog36/table42/params/param82.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let field_content = fixture_content_from_raw(
        &toc,
        "shcntx_ru.hbk",
        "ru",
        "tables/table58/fields/Presentation464.html",
        r#"<html><body><h1 class="V8SH_pagetitle">Представление (Presentation)</h1><p class="V8SH_heading">Представление (Presentation)</p>Тип: <a href="v8help://SyntaxHelperLanguage/def_String">Строка</a>. <br>Содержит строку-представление бизнес-процесса.<br></body></html>"#,
    );
    let parameter_content = fixture_content_from_raw(
        &toc,
        "shcntx_ru.hbk",
        "ru",
        "tables/catalog36/table42/params/param82.html",
        r#"<html><body><h1 class="V8SH_pagetitle">Первые</h1><p class="V8SH_chapter">Первые (необязательный)</p>Тип параметра: <a href="v8help://SyntaxHelperLanguage/def_Number">Число</a>. <br>Ограничение максимального количества записей.<br>Значение по умолчанию: 0.<br></body></html>"#,
    );

    let owner = LocalizedName {
        primary: "Таблица движений с субконто".to_string(),
        alias: None,
    };
    let field = parse_query_table_field(
        &field_content,
        owner.clone(),
        source("tables/table58/fields/Presentation464.html"),
    );
    let parameter = parse_query_table_parameter(
        &parameter_content,
        owner,
        source("tables/catalog36/table42/params/param82.html"),
    );

    assert_eq!(field.name, "Представление");
    assert_eq!(field.type_refs[0].name, "Строка");
    assert_eq!(
        field.description.as_deref(),
        Some("Содержит строку-представление бизнес-процесса")
    );
    assert_eq!(parameter.name, "Первые");
    assert_eq!(parameter.type_refs[0].name, "Число");
    assert_eq!(
        parameter.description.as_deref(),
        Some("Ограничение максимального количества записей")
    );
    assert_eq!(parameter.default_value.as_deref(), Some("0"));
}

#[test]
fn parses_query_table_syntax_without_claiming_toc_identity() {
    let toc = Toc::parse(
        r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Таблица бизнес-процессов"}},"/tables/table58.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let content = fixture_content_from_raw(
        &toc,
        "shcntx_ru.hbk",
        "ru",
        "tables/table58.html",
        r#"<html><body><h1 class="V8SH_pagetitle">БизнесПроцесс.&lt;Имя бизнес-процесса&gt; (BusinessProcess.&lt;Имя бизнес-процесса&gt;)</h1><p class="V8SH_chapter">Синтаксис</p>БизнесПроцесс.&lt;Имя бизнес-процесса&gt; (BusinessProcess.&lt;Имя бизнес-процесса&gt;)<p class="V8SH_chapter">Поля</p><p class="V8SH_chapter">Описание:</p><p>Предназначена для получения записей бизнес-процессов.</p></body></html>"#,
    );

    let table = parse_query_table(&content, source("tables/table58.html"));

    let syntax = table.syntax.as_ref().expect("syntax must be parsed");
    assert_eq!(syntax.primary, "БизнесПроцесс.<Имя бизнес-процесса>");
    assert_eq!(
        syntax.alias.as_deref(),
        Some("BusinessProcess.<Имя бизнес-процесса>")
    );
    assert!(table.identifier.is_none());
    assert_eq!(table.table_role, QueryTableRole::Unknown);
}

#[test]
fn parses_additional_query_table_syntax_without_claiming_toc_identity() {
    let toc = Toc::parse(
        r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Таблица точек бизнес-процессов"}},"/tables/table57.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let content = fixture_content_from_raw(
        &toc,
        "shcntx_ru.hbk",
        "ru",
        "tables/table57.html",
        r#"<html><body><h1 class="V8SH_pagetitle">БизнесПроцесс.&lt;Имя бизнес-процесса&gt;.Точки (BusinessProcess.&lt;Имя бизнес-процесса&gt;.Points)</h1><p class="V8SH_chapter">Синтаксис</p>БизнесПроцесс.&lt;Имя бизнес-процесса&gt;.Точки (BusinessProcess.&lt;Имя бизнес-процесса&gt;.Points)<p class="V8SH_chapter">Поля</p><p class="V8SH_chapter">Описание:</p><p>Предназначена для получения точек бизнес-процессов.</p></body></html>"#,
    );

    let table = parse_query_table(&content, source("tables/table57.html"));

    let syntax = table.syntax.as_ref().expect("syntax must be parsed");
    assert_eq!(syntax.primary, "БизнесПроцесс.<Имя бизнес-процесса>.Точки");
    assert_eq!(
        syntax.alias.as_deref(),
        Some("BusinessProcess.<Имя бизнес-процесса>.Points")
    );
    assert!(table.identifier.is_none());
    assert_eq!(table.table_role, QueryTableRole::Unknown);
}

#[test]
fn query_table_without_syntax_does_not_fallback_to_display_name() {
    let toc = Toc::parse(
        r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","Основная таблица"}},"/tables/catalog1/table2.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let content = fixture_content_from_raw(
        &toc,
        "shcntx_ru.hbk",
        "ru",
        "tables/catalog1/table2.html",
        r#"<html><body><h1 class="V8SH_pagetitle">Основная таблица</h1><p class="V8SH_chapter">Поля</p><p class="V8SH_chapter">Описание:</p><p>Основная таблица задач.</p></body></html>"#,
    );

    let table = parse_query_table(&content, source("tables/catalog1/table2.html"));

    assert!(table.syntax.is_none());
    assert!(table.identifier.is_none());
    assert_eq!(table.table_role, QueryTableRole::Unknown);

    let empty_syntax_content = fixture_content_from_raw(
        &toc,
        "shcntx_ru.hbk",
        "ru",
        "tables/catalog1/table2.html",
        r#"<html><body><h1 class="V8SH_pagetitle">Основная таблица</h1><p class="V8SH_chapter">Синтаксис</p><p class="V8SH_chapter">Поля</p><p>Наименование</p></body></html>"#,
    );
    let empty_syntax_table =
        parse_query_table(&empty_syntax_content, source("tables/catalog1/table2.html"));
    assert!(empty_syntax_table.syntax.is_none());
    assert!(empty_syntax_table.identifier.is_none());
    assert_eq!(empty_syntax_table.table_role, QueryTableRole::Unknown);
}

#[test]
fn extraction_reports_missing_query_table_syntax_without_dropping_record() {
    let toc = Toc::parse(
        r#"{
                4
                {1,0,3,2,3,4,{0,0,{0,0,{"ru","Универсальные коллекции значений"}},"/objects/catalog234.html"}}
                {2,1,0,{0,0,{0,0,{"ru","Массив"}},"/objects/catalog234/Array.html"}}
                {3,1,0,{0,0,{0,0,{"ru","Основная таблица"}},"/tables/catalog1/table2.html"}}
                {4,1,0,{0,0,{0,0,{"ru","Наименование"}},"/tables/catalog1/table2/fields/Description.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let mut sink = RecordingSink::default();

    let discovery = RootDiscovery {
        roots: vec![RootSection {
            kind: RootSectionKind::TypeObjectCatalog,
            source: SyntaxHelperSource {
                hbk_path: PathBuf::from("shcntx_ru.hbk"),
                locale: "ru".to_string(),
                toc_path: Some("0".to_string()),
                html_path: "objects/catalog234.html".to_string(),
                page_title: "Универсальные коллекции значений".to_string(),
            },
            pages: collect_catalog_pages(
                Path::new("shcntx_ru.hbk"),
                "ru",
                &toc.pages()[0],
                &toc.flat_pages().next().expect("root flat page must exist"),
            ),
        }],
        diagnostics: Vec::new(),
    };

    parse_extraction_pages_into(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc,
        discovery,
        |html_path| {
            let html = match html_path {
                "objects/catalog234.html" => {
                    include_str!("../../../tests/fixtures/syntax-helper/root_catalog_types_ru.html")
                }
                "objects/catalog234/Array.html" => {
                    include_str!("../../../tests/fixtures/syntax-helper/object_array_ru.html")
                }
                "tables/catalog1/table2.html" => {
                    r#"<html><body><h1 class="V8SH_pagetitle">Основная таблица</h1><p class="V8SH_chapter">Поля</p><p class="V8SH_chapter">Описание:</p><p>Основная таблица задач.</p></body></html>"#
                }
                "tables/catalog1/table2/fields/Description.html" => {
                    r#"<html><body><h1 class="V8SH_pagetitle">Наименование</h1><p class="V8SH_heading">Наименование</p>Тип: <a href="v8help://SyntaxHelperLanguage/def_String">Строка</a>. <br>Описание задачи.<br></body></html>"#
                }
                other => panic!("unexpected fixture page load: {other}"),
            };
            Ok(fixture_content_from_raw(
                &toc,
                "shcntx_ru.hbk",
                "ru",
                html_path,
                html,
            ))
        },
        &mut sink,
    )
    .expect("fixture extraction must stream");

    assert!(
        sink.seen
            .contains(&"query_table:Основная таблица".to_string())
    );
    assert!(
        sink.seen
            .contains(&"table_field:Основная таблица:Наименование".to_string())
    );
    let table = sink
        .query_tables
        .iter()
        .find(|table| table.name == "Основная таблица")
        .expect("query table record must be kept");
    assert!(table.syntax.is_none());
    assert!(table.identifier.is_none());
    assert_eq!(table.table_role, QueryTableRole::Unknown);

    let diagnostic = sink
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "MISSING_QUERY_TABLE_SYNTAX")
        .expect("missing syntax diagnostic must be emitted");
    assert_eq!(diagnostic.parser_stage, "query_table");
    assert_eq!(diagnostic.source.page_title, "Основная таблица");
}

#[test]
fn extraction_assigns_query_table_identity_from_toc_context() {
    let toc = Toc::parse(
        r#"{
                4
                {1,0,3,2,3,4,{0,0,{0,0,{"ru","Универсальные коллекции значений"}},"/objects/catalog234.html"}}
                {2,1,0,{0,0,{0,0,{"ru","Массив"}},"/objects/catalog234/Array.html"}}
                {3,1,0,{0,0,{0,0,{"ru","Таблица точек бизнес-процессов"}},"/tables/catalog1/table2.html"}}
                {4,1,0,{0,0,{0,0,{"ru","Наименование"}},"/tables/catalog1/table2/fields/Description.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let mut sink = RecordingSink::default();

    let discovery = RootDiscovery {
        roots: vec![RootSection {
            kind: RootSectionKind::TypeObjectCatalog,
            source: SyntaxHelperSource {
                hbk_path: PathBuf::from("shcntx_ru.hbk"),
                locale: "ru".to_string(),
                toc_path: Some("0".to_string()),
                html_path: "objects/catalog234.html".to_string(),
                page_title: "Универсальные коллекции значений".to_string(),
            },
            pages: collect_catalog_pages(
                Path::new("shcntx_ru.hbk"),
                "ru",
                &toc.pages()[0],
                &toc.flat_pages().next().expect("root flat page must exist"),
            ),
        }],
        diagnostics: Vec::new(),
    };

    parse_extraction_pages_into(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc,
        discovery,
        |html_path| {
            let html = match html_path {
                "objects/catalog234.html" => {
                    include_str!("../../../tests/fixtures/syntax-helper/root_catalog_types_ru.html")
                }
                "objects/catalog234/Array.html" => {
                    include_str!("../../../tests/fixtures/syntax-helper/object_array_ru.html")
                }
                "tables/catalog1/table2.html" => {
                    r#"<html><body><h1 class="V8SH_pagetitle">БизнесПроцесс.&lt;Имя бизнес-процесса&gt;.Точки (BusinessProcess.&lt;Имя бизнес-процесса&gt;.Points)</h1><p class="V8SH_chapter">Синтаксис</p>БизнесПроцесс.&lt;Имя бизнес-процесса&gt;.Точки (BusinessProcess.&lt;Имя бизнес-процесса&gt;.Points)<p class="V8SH_chapter">Поля</p><p class="V8SH_chapter">Описание:</p><p>Предназначена для получения точек бизнес-процессов.</p></body></html>"#
                }
                "tables/catalog1/table2/fields/Description.html" => {
                    r#"<html><body><h1 class="V8SH_pagetitle">Наименование</h1><p class="V8SH_heading">Наименование</p>Тип: <a href="v8help://SyntaxHelperLanguage/def_String">Строка</a>. <br>Описание задачи.<br></body></html>"#
                }
                other => panic!("unexpected fixture page load: {other}"),
            };
            Ok(fixture_content_from_raw(
                &toc,
                "shcntx_ru.hbk",
                "ru",
                html_path,
                html,
            ))
        },
        &mut sink,
    )
    .expect("fixture extraction must stream");

    let table = sink
        .query_tables
        .iter()
        .find(|table| table.name == "Таблица точек бизнес-процессов")
        .expect("query table record must be kept");
    assert_eq!(
        table.identifier.as_deref(),
        Some("БизнесПроцессТаблицаТочекБизнесПроцессов")
    );
    assert_eq!(table.table_role, QueryTableRole::Additional);
    assert_eq!(table.semantic.record_family, RecordFamily::QueryTable);
    assert_eq!(table.semantic.branch_kind, BranchKind::QueryTables);
    assert!(sink.diagnostics.is_empty());
}

#[test]
fn resolves_query_table_member_owner_from_toc_semantic_context() {
    let toc = Toc::parse(
        r#"{
                5
                {1,0,2,2,3,{0,0,{0,0,{"ru","Таблица бизнес-процессов (Business Process Table)"}{"en","Business Process Table"}},"/tables/table58.html"}}
                {2,1,0,{0,0,{0,0,{"ru","Представление"}{"en","Presentation"}},"/tables/table58/fields/Presentation464.html"}}
                {3,1,0,{0,0,{0,0,{"ru","Номер"}{"en","Number"}},"/tables/table58/params/Number.html"}}
                {4,0,1,5,{0,0,{0,0,{"ru","Таблица критерия отбора (Filter Criterion Table)"}{"en","Filter Criterion Table"}},"/tables/catalog36/table42.html"}}
                {5,4,0,{0,0,{0,0,{"ru","Значение"}{"en","Value"}},"/tables/catalog36/table42/params/param82.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");

    let ru_pages = collect_catalog_pages(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc.pages()[0],
        &toc.flat_pages().next().expect("root flat page must exist"),
    );
    let ru_field_owner = ru_pages
        .iter()
        .find(|page| page.source.html_path.ends_with("Presentation464.html"))
        .and_then(|page| query_table_member_owner(&page.semantic))
        .expect("field owner must resolve");
    assert_eq!(ru_field_owner.primary, "Таблица бизнес-процессов");
    assert_eq!(
        ru_field_owner.alias.as_deref(),
        Some("Business Process Table")
    );

    let root_pages = collect_catalog_pages(
        Path::new("shcntx_root.hbk"),
        "root",
        &toc.pages()[0],
        &toc.flat_pages().next().expect("root flat page must exist"),
    );
    let root_field_owner = root_pages
        .iter()
        .find(|page| page.source.html_path.ends_with("Presentation464.html"))
        .and_then(|page| query_table_member_owner(&page.semantic))
        .expect("field owner must resolve");
    assert_eq!(root_field_owner.primary, "Business Process Table");
    assert_eq!(root_field_owner.alias, None);

    let nested_flat = toc
        .flat_pages()
        .find(|page| page.page.html_path.ends_with("table42.html"))
        .expect("nested table flat page must exist");
    let ru_nested_pages = collect_catalog_pages(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc.pages()[1],
        &nested_flat,
    );
    let ru_parameter_owner = ru_nested_pages
        .iter()
        .find(|page| page.source.html_path.ends_with("param82.html"))
        .and_then(|page| query_table_member_owner(&page.semantic))
        .expect("parameter owner must resolve");
    assert_eq!(ru_parameter_owner.primary, "Таблица критерия отбора");
    assert_eq!(
        ru_parameter_owner.alias.as_deref(),
        Some("Filter Criterion Table")
    );

    let root_nested_pages = collect_catalog_pages(
        Path::new("shcntx_root.hbk"),
        "root",
        &toc.pages()[1],
        &nested_flat,
    );
    let root_parameter_owner = root_nested_pages
        .iter()
        .find(|page| page.source.html_path.ends_with("param82.html"))
        .and_then(|page| query_table_member_owner(&page.semantic))
        .expect("parameter owner must resolve");
    assert_eq!(root_parameter_owner.primary, "Filter Criterion Table");
    assert_eq!(root_parameter_owner.alias, None);
}

#[test]
fn query_table_member_owner_does_not_fallback_to_rewritten_html_path() {
    let toc = Toc::parse(
        r#"{
                2
                {1,0,1,2,{0,0,{0,0,{"ru","Универсальные коллекции значений"}},"/objects/catalog234.html"}}
                {2,1,0,{0,0,{0,0,{"ru","Поле без таблицы"}},"/tables/missing/fields/Field1.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");

    let pages = collect_catalog_pages(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc.pages()[0],
        &toc.flat_pages().next().expect("root flat page must exist"),
    );
    let field = pages
        .iter()
        .find(|page| page.source.html_path.ends_with("Field1.html"))
        .expect("field page must be collected");

    assert!(query_table_member_owner(&field.semantic).is_none());
}

#[test]
fn extraction_reports_missing_query_table_member_owner_without_synthesizing_path_owner() {
    let toc = Toc::parse(
        r#"{
                2
                {1,0,1,2,{0,0,{0,0,{"ru","Универсальные коллекции значений"}},"/objects/catalog234.html"}}
                {2,1,0,{0,0,{0,0,{"ru","Поле без таблицы"}},"/tables/missing/fields/Field1.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let mut sink = RecordingSink::default();

    let discovery = RootDiscovery {
        roots: vec![RootSection {
            kind: RootSectionKind::TypeObjectCatalog,
            source: SyntaxHelperSource {
                hbk_path: PathBuf::from("shcntx_ru.hbk"),
                locale: "ru".to_string(),
                toc_path: Some("0".to_string()),
                html_path: "objects/catalog234.html".to_string(),
                page_title: "Универсальные коллекции значений".to_string(),
            },
            pages: collect_catalog_pages(
                Path::new("shcntx_ru.hbk"),
                "ru",
                &toc.pages()[0],
                &toc.flat_pages().next().expect("root flat page must exist"),
            ),
        }],
        diagnostics: Vec::new(),
    };

    parse_extraction_pages_into(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc,
        discovery,
        |html_path| {
            let html = match html_path {
                "objects/catalog234.html" => {
                    include_str!("../../../tests/fixtures/syntax-helper/root_catalog_types_ru.html")
                }
                "tables/missing/fields/Field1.html" => {
                    r#"<html><body><h1 class="V8SH_pagetitle">Поле без таблицы</h1><p class="V8SH_heading">Поле без таблицы</p>Тип: Строка. Описание.</body></html>"#
                }
                other => panic!("unexpected fixture page load: {other}"),
            };
            Ok(fixture_content_from_raw(
                &toc,
                "shcntx_ru.hbk",
                "ru",
                html_path,
                html,
            ))
        },
        &mut sink,
    )
    .expect("fixture extraction must stream");

    assert!(
        !sink
            .seen
            .iter()
            .any(|record| record.starts_with("table_field:"))
    );
    let diagnostic = sink
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "MISSING_QUERY_TABLE_OWNER_CONTEXT")
        .unwrap_or_else(|| {
            panic!(
                "missing owner diagnostic must be emitted; seen={:?}",
                sink.seen
            )
        });
    assert_eq!(diagnostic.parser_stage, "query_table_field");
    assert_eq!(diagnostic.source.hbk_path, PathBuf::from("shcntx_ru.hbk"));
    assert_eq!(diagnostic.source.toc_path.as_deref(), Some("0.0"));
    assert_eq!(
        diagnostic.source.html_path,
        "tables/missing/fields/Field1.html"
    );
    assert_eq!(diagnostic.source.page_title, "Поле без таблицы");
}

#[test]
fn derives_toc_semantic_context_for_ambiguous_query_and_event_pages() {
    let toc = Toc::parse(
        r#"{
                11
                {1,0,2,2,8,{0,0,{0,0,{"ru","Работа с запросами"}},"/objects/catalog213.html"}}
                {2,1,2,3,6,{0,0,{0,0,{"ru","Таблицы запросов"}},""}}
                {3,2,1,4,{0,0,{0,0,{"ru","Таблицы регистра бухгалтерии (без поддержки корреспонденции)"}},""}}
                {4,3,1,5,{0,0,{0,0,{"ru","Таблица остатков и оборотов"}},"/tables/catalog43/table49.html"}}
                {5,4,0,{0,0,{0,0,{"ru","Метод дополнения периодов"}},"/tables/catalog43/table49/params/param70.html"}}
                {6,2,1,7,{0,0,{0,0,{"ru","Таблицы регистра накопления"}},""}}
                {7,6,0,{0,0,{0,0,{"ru","Таблица остатков и оборотов"}},"/tables/catalog8/table12/params/param14.html"}}
                {8,1,1,9,{0,0,{0,0,{"ru","Формы"}},"/objects/catalog1649.html"}}
                {9,8,1,10,{0,0,{0,0,{"ru","Расширение формы клиентского приложения для документов"}},"/objects/catalog1649/catalog1890/Client application form extension for documents.html"}}
                {10,9,0,{0,0,{0,0,{"ru","ПередЗаписью"}{"en","BeforeWrite"}},"/objects/catalog1649/catalog1890/Client application form extension for documents/events/BeforeWrite335.html"}}
                {11,0,0,{0,0,{0,0,{"ru","Глобальный контекст"}},"/objects/Global context.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let root = &toc.pages()[0];
    let root_flat = toc
        .flat_pages()
        .next()
        .expect("fixture root page must exist");

    let pages = collect_catalog_pages(Path::new("shcntx_ru.hbk"), "ru", root, &root_flat);

    let query_parameter_paths = pages
        .iter()
        .filter(|page| page.class == PageClass::QueryTableParameter)
        .map(|page| {
            assert_eq!(page.semantic.branch_kind, BranchKind::QueryTables);
            assert_eq!(
                page.semantic.record_family,
                RecordFamily::QueryTableParameter
            );
            page.semantic
                .owner_path
                .iter()
                .map(|name| name.primary.as_str())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(query_parameter_paths.len(), 2);
    assert_ne!(query_parameter_paths[0], query_parameter_paths[1]);

    let event = pages
        .iter()
        .find(|page| page.source.html_path.ends_with("BeforeWrite335.html"))
        .expect("event page must be collected");
    assert_eq!(event.class, PageClass::TypeEvent);
    assert_eq!(event.semantic.branch_kind, BranchKind::ManagedForms);
    assert_eq!(event.semantic.record_family, RecordFamily::TypeEvent);
    assert!(
        event
            .semantic
            .owner_path
            .iter()
            .any(|name| name.primary.contains("Расширение формы"))
    );
}

#[test]
fn root_form_labels_do_not_match_information_and_form_events_stay_form_modules() {
    let toc = Toc::parse(
        r#"{
                5
                {1,0,2,2,5,{0,0,{0,0,{"en","Platform objects"}},"/objects/catalog.html"}}
                {2,1,1,3,{0,0,{0,0,{"en","Client application form"}},"/objects/catalog1649.html"}}
                {3,2,1,4,{0,0,{0,0,{"en","Client application form extension for documents"}},"/objects/catalog1649/catalog1890/Client application form extension for documents.html"}}
                {4,3,0,{0,0,{0,0,{"en","BeforeWrite"}},"/objects/catalog1649/catalog1890/Client application form extension for documents/events/BeforeWrite335.html"}}
                {5,1,0,{0,0,{0,0,{"en","BinaryDataStorageInformation"}},"/objects/catalog999/BinaryDataStorageInformation.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let root = &toc.pages()[0];
    let root_flat = toc
        .flat_pages()
        .next()
        .expect("fixture root page must exist");

    let pages = collect_catalog_pages(Path::new("shcntx_root.hbk"), "root", root, &root_flat);

    let information = pages
        .iter()
        .find(|page| {
            page.source
                .html_path
                .ends_with("BinaryDataStorageInformation.html")
        })
        .expect("information page must be collected");
    assert_eq!(information.class, PageClass::ObjectType);
    assert_eq!(
        information.semantic.branch_kind,
        BranchKind::PlatformObjects
    );

    let mut sink = RecordingSink::default();
    extract_with_loader_into(
        Path::new("shcntx_root.hbk"),
        "root",
        &toc,
        |html_path| {
            let title = if html_path.ends_with("BeforeWrite335.html") {
                "BeforeWrite"
            } else {
                "Platform objects"
            };
            Ok(fixture_content_from_raw(
                &toc,
                "shcntx_root.hbk",
                "root",
                html_path,
                &format!(
                    r#"<html><body><h1 class="V8SH_pagetitle">{title}</h1><h2>{title}</h2></body></html>"#
                ),
            ))
        },
        &mut sink,
    )
    .expect("fixture extraction must stream");

    let event = sink.events.first().expect("event must be extracted");
    assert_eq!(event.name.primary, "BeforeWrite");
    assert_eq!(event.semantic.record_family, RecordFamily::TypeEvent);
    assert_eq!(event.module.kind, ModuleKind::Unknown);
}

#[test]
fn classifies_form_parameters_as_type_properties() {
    let toc = Toc::parse(
        r#"{
                4
                {1,0,1,2,{0,0,{0,0,{"ru","Формы"}},"/objects/catalog1649.html"}}
                {2,1,1,3,{0,0,{0,0,{"ru","Расширение формы клиентского приложения для документов"}},"/objects/catalog1649/catalog1890/Client application form extension for documents.html"}}
                {3,2,1,4,{0,0,{0,0,{"ru","Параметры формы"}},"/objects/catalog1649/catalog1890/params.html"}}
                {4,3,0,{0,0,{0,0,{"ru","Ключ"}{"en","Key"}},"/objects/catalog1649/catalog1890/Key.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let root = &toc.pages()[0];
    let root_flat = toc
        .flat_pages()
        .next()
        .expect("fixture root page must exist");

    let pages = collect_catalog_pages(Path::new("shcntx_ru.hbk"), "ru", root, &root_flat);
    let parameter = pages
        .iter()
        .find(|page| page.source.html_path.ends_with("/Key.html"))
        .expect("form parameter page must be collected");

    assert_eq!(parameter.class, PageClass::ObjectProperty);
    assert_eq!(parameter.semantic.branch_kind, BranchKind::ManagedForms);
    assert_eq!(parameter.semantic.record_family, RecordFamily::TypeProperty);
    assert!(
        parameter
            .semantic
            .owner_path
            .iter()
            .any(|name| name.primary.contains("Расширение формы"))
    );

    let mut sink = RecordingSink::default();
    extract_with_loader_into(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc,
        |html_path| {
            let title = toc
                .flat_pages()
                .find(|page| page.page.html_path == html_path)
                .map(|page| page.page.title.display().to_string())
                .expect("fixture page must exist in TOC");
            Ok(fixture_content_from_raw(
                &toc,
                "shcntx_ru.hbk",
                "ru",
                html_path,
                &format!(
                    r#"<html><body><h1 class="V8SH_pagetitle">{title}</h1><p class="V8SH_title">{title}</p><p class="V8SH_heading">{title}</p><p class="V8SH_chapter">Описание:</p>Тип: Строка. Описание.</body></html>"#
                ),
            ))
        },
        &mut sink,
    )
    .expect("fixture extraction must stream");

    assert!(
        sink.platform_types
            .iter()
            .all(|record| record.name.primary != "Ключ")
    );
    let property = sink
        .type_properties
        .iter()
        .find(|record| record.name.primary == "Ключ")
        .expect("form parameter must be extracted as a type property");
    assert_eq!(
        property.owner.primary,
        "Расширение формы клиентского приложения для документов"
    );
}

#[test]
fn classifies_platform_owner_object_kind_from_toc_context() {
    let toc = Toc::parse(
        r#"{
                9
                {1,0,3,2,4,7,{0,0,{0,0,{"ru","Платформенные объекты"}},"objects/catalog.html"}}
                {2,1,1,3,{0,0,{0,0,{"ru","Универсальные коллекции"}},"objects/catalog234.html"}}
                {3,2,0,{0,0,{0,0,{"ru","Массив"}{"en","Array"}},"objects/catalog234/Array.html"}}
                {4,1,2,5,6,{0,0,{0,0,{"ru","Формы"}},"objects/catalog1649.html"}}
                {5,4,0,{0,0,{0,0,{"ru","Форма"}},"objects/catalog1649/Form.html"}}
                {6,4,0,{0,0,{0,0,{"ru","Расширение формы клиентского приложения для документов"}},"objects/catalog1649/catalog1890/Client application form extension for documents.html"}}
                {7,1,1,8,{0,0,{0,0,{"ru","Прикладные объекты"}},"objects/catalog125.html"}}
                {8,7,1,9,{0,0,{0,0,{"ru","Документы"}},"objects/catalog125/catalog132.html"}}
                {9,8,0,{0,0,{0,0,{"ru","ДокументОбъект.<Имя документа>"}},"objects/catalog125/catalog132/DocumentObject.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");

    let mut sink = RecordingSink::default();
    extract_with_loader_into(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc,
        |html_path| {
            let title = toc
                .flat_pages()
                .find(|page| page.page.html_path == html_path)
                .map(|page| page.page.title.display().to_string())
                .expect("fixture page must exist in TOC");
            Ok(fixture_content_from_raw(
                &toc,
                "shcntx_ru.hbk",
                "ru",
                html_path,
                &format!(
                    r#"<html><body><h1 class="V8SH_pagetitle">{title}</h1><h2>{title}</h2></body></html>"#
                ),
            ))
        },
        &mut sink,
    )
    .expect("fixture extraction must stream");

    let object_kind = |name: &str| {
        sink.platform_types
            .iter()
            .find(|platform_type| platform_type.name.primary == name)
            .and_then(|platform_type| platform_type.object_kind)
    };

    assert_eq!(
        object_kind("Массив"),
        Some(PlatformObjectKind::RegularPlatformType)
    );
    assert_eq!(object_kind("Форма"), Some(PlatformObjectKind::ManagedForm));
    assert_eq!(
        object_kind("Расширение формы клиентского приложения для документов"),
        Some(PlatformObjectKind::FormExtension)
    );
    assert_eq!(
        object_kind("ДокументОбъект.<Имя документа>"),
        Some(PlatformObjectKind::MetadataObject)
    );
}

#[test]
fn classifies_explicit_module_events_and_unknown_event_fallbacks() {
    let toc = Toc::parse(
        r#"{
                9
                {1,0,3,2,6,8,{0,0,{0,0,{"ru","Платформенные объекты"}},"/objects/catalog.html"}}
                {2,1,1,3,{0,0,{0,0,{"ru","Документы"}},"/objects/catalog1.html"}}
                {3,2,1,4,{0,0,{0,0,{"ru","Модуль менеджера документа"}},"/objects/catalog1/Document manager module.html"}}
                {4,3,0,{0,0,{0,0,{"ru","ОбработкаПолученияФормы"}},"/objects/catalog1/Document manager module/events/GetFormProcessing.html"}}
                {5,1,0,{0,0,{0,0,{"ru","Выбор"}},"/objects/catalog1/Document/events/Choice.html"}}
                {6,0,1,7,{0,0,{0,0,{"ru","Неподдержанный раздел"}},"/misc.html"}}
                {7,6,0,{0,0,{0,0,{"ru","Событие"}},"/misc/events/Event.html"}}
                {8,1,1,9,{0,0,{0,0,{"ru","Модуль объекта документа"}},"/objects/catalog1/Document object module.html"}}
                {9,8,0,{0,0,{0,0,{"ru","ПередЗаписью"}},"/objects/catalog1/Document object module/events/BeforeWrite.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let root_flats = toc.flat_pages().collect::<Vec<_>>();
    let mut pages = collect_catalog_pages(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc.pages()[0],
        &root_flats[0],
    );
    pages.extend(collect_catalog_pages(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc.pages()[1],
        root_flats
            .iter()
            .find(|page| page.page.html_path.ends_with("misc.html"))
            .expect("misc root flat page must exist"),
    ));

    let module_event = pages
        .iter()
        .find(|page| page.source.html_path.ends_with("GetFormProcessing.html"))
        .expect("explicit module event page must be collected");
    assert_eq!(module_event.class, PageClass::ModuleEvent);
    assert_eq!(
        module_event.semantic.record_family,
        RecordFamily::ModuleEvent
    );

    let object_module_event = pages
        .iter()
        .find(|page| page.source.html_path.ends_with("BeforeWrite.html"))
        .expect("explicit object module event page must be collected");
    assert_eq!(object_module_event.class, PageClass::ModuleEvent);
    assert_eq!(
        object_module_event.semantic.record_family,
        RecordFamily::ModuleEvent
    );

    let type_event = pages
        .iter()
        .find(|page| page.source.html_path.ends_with("Choice.html"))
        .expect("type event page must be collected");
    assert_eq!(type_event.class, PageClass::TypeEvent);
    assert_eq!(type_event.semantic.record_family, RecordFamily::TypeEvent);

    let unknown_event = pages
        .iter()
        .find(|page| page.source.html_path.ends_with("Event.html"))
        .expect("unknown event page must be collected");
    assert_eq!(unknown_event.class, PageClass::UnknownEvent);
    assert_eq!(
        unknown_event.semantic.record_family,
        RecordFamily::UnknownEvent
    );
}

#[test]
fn strips_methodical_footer_from_section_text_and_html() {
    let toc = Toc::parse(
        r#"{
                2
                {1,0,0,{0,0,{0,0,{"ru","Поле"}},"/tables/table58/fields/Field1.html"}}
                {2,0,0,{0,0,{0,0,{"ru","Конструктор"}},"/objects/catalog234/Array/ctors/Constructor1.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let owner = LocalizedName {
        primary: "Таблица".to_string(),
        alias: None,
    };
    let field_content = fixture_content_from_raw(
        &toc,
        "shcntx_ru.hbk",
        "ru",
        "tables/table58/fields/Field1.html",
        r#"<html><body><h1 class="V8SH_pagetitle">Поле</h1><p class="V8SH_heading">Поле</p>Тип: <a href="v8help://SyntaxHelperLanguage/def_String">Строка</a>. Описание поля.<p class="V8SH_chapter">Примечание:</p><p>Только для отдельных таблиц.</p><HR><a href="methodical.html">Методическая информация</a></body></html>"#,
    );
    let field = parse_query_table_field(
        &field_content,
        owner,
        source("tables/table58/fields/Field1.html"),
    );
    assert_eq!(field.note.as_deref(), Some("Только для отдельных таблиц."));

    let constructor_content = fixture_content_from_raw(
        &toc,
        "shcntx_ru.hbk",
        "ru",
        "objects/catalog234/Array/ctors/Constructor1.html",
        r#"<html><body><h1 class="V8SH_pagetitle">Массив.Конструктор</h1><p class="V8SH_title">Массив (Array)</p><p class="V8SH_heading">По умолчанию</p><p class="V8SH_chapter">Синтаксис:</p>Новый Массив<HR><a href="methodical.html">Методическая информация</a></body></html>"#,
    );
    let constructor = parse_constructor(
        &constructor_content,
        source("objects/catalog234/Array/ctors/Constructor1.html"),
    );
    assert_eq!(constructor.signatures[0].text, "Новый Массив");
}

#[test]
fn constructor_parameters_keep_inline_notes_inside_parameter_section() {
    let toc = Toc::parse(
        r#"{
                1
                {1,0,0,{0,0,{0,0,{"ru","По умолчанию"}},"/objects/catalog63/catalog578/catalog2125/HTTPConnection/ctors/ctor182.html"}}
            }"#,
    )
    .expect("fixture TOC must parse");
    let content = fixture_content_from_raw(
        &toc,
        "shcntx_ru.hbk",
        "ru",
        "objects/catalog63/catalog578/catalog2125/HTTPConnection/ctors/ctor182.html",
        r#"<html><body><h1 class="V8SH_pagetitle">HTTPСоединение.По умолчанию</h1><p class="V8SH_title">HTTPСоединение (HTTPConnection)</p><p class="V8SH_heading">По умолчанию</p><p>Слово Параметры: здесь не является заголовком секции.</p><p class="V8SH_chapter">Синтаксис:</p>Новый HTTPСоединение(&lt;Сервер&gt;, &lt;Порт&gt;, &lt;Пользователь&gt;, &lt;Пароль&gt;, &lt;Прокси&gt;, &lt;Таймаут&gt;, &lt;ЗащищенноеСоединение&gt;, &lt;ИспользоватьАутентификациюОС&gt;)<p class="V8SH_chapter">Параметры:</p><div class="V8SH_rubric"> <p style="margin-top: 2px; margin-bottom: 1px">&lt;Сервер&gt; (обязательный)</div>Тип: <a href="v8help://SyntaxHelperLanguage/def_String">Строка</a>. <br>Хост сервера, с которым осуществляется соединение.<br>Примечание: Имя хоста не должно содержать указание протокола. Например, example.com.<div class="V8SH_rubric"> <p style="margin-top: 2px; margin-bottom: 1px">&lt;Порт&gt; (необязательный)</div>Тип: <a href="v8help://SyntaxHelperLanguage/def_Number">Число</a>. <br>Порт сервера, с которым осуществляется соединение.<div class="V8SH_rubric"> <p style="margin-top: 2px; margin-bottom: 1px">&lt;Пользователь&gt; (необязательный)</div>Тип: <a href="v8help://SyntaxHelperLanguage/def_String">Строка</a>. <br>Имя пользователя на указанном сервере.<div class="V8SH_rubric"> <p style="margin-top: 2px; margin-bottom: 1px">&lt;Пароль&gt; (необязательный)</div>Тип: <a href="v8help://SyntaxHelperLanguage/def_String">Строка</a>. <br>Пароль пользователя на указанном сервере.<div class="V8SH_rubric"> <p style="margin-top: 2px; margin-bottom: 1px">&lt;Прокси&gt; (необязательный)</div>Тип: <a href="v8help://SyntaxHelperContext/objects/catalog63/catalog578/InternetProxy.html">ИнтернетПрокси</a>. <br>Прокси, используемый для соединения с сервером.<div class="V8SH_rubric"> <p style="margin-top: 2px; margin-bottom: 1px">&lt;Таймаут&gt; (необязательный)</div>Тип: <a href="v8help://SyntaxHelperLanguage/def_Number">Число</a>. <br>Таймаут осуществляемого соединения и операций, в секундах. 0 - не устанавливать таймаут.<div class="V8SH_rubric"> <p style="margin-top: 2px; margin-bottom: 1px">&lt;ЗащищенноеСоединение&gt; (необязательный)</div>Тип: <a href="v8help://SyntaxHelperContext/objects/catalog63/catalog578/catalog2014/OpenSSLSecureConnection.html">ЗащищенноеСоединениеOpenSSL</a>, <a href="v8help://SyntaxHelperLanguage/def_Undefined">Неопределено</a>. <br>Объект защищенного соединения.<div class="V8SH_rubric"> <p style="margin-top: 2px; margin-bottom: 1px">&lt;ИспользоватьАутентификациюОС&gt; (необязательный)</div>Тип: <a href="v8help://SyntaxHelperLanguage/def_Boolean">Булево</a>. <br>Включает использование аутентификации NTLM или Negotiate на сервере.<p class="V8SH_chapter">Описание:</p><p>Создает объект <a href="v8help://SyntaxHelperContext/objects/catalog63/catalog578/catalog2125/HTTPConnection.html">HTTPСоединение</a>.</p></body></html>"#,
    );
    let constructor = parse_constructor(
        &content,
        source("objects/catalog63/catalog578/catalog2125/HTTPConnection/ctors/ctor182.html"),
    );

    let parameters = &constructor.signatures[0].parameters;
    assert_eq!(
        parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>(),
        [
            "Сервер",
            "Порт",
            "Пользователь",
            "Пароль",
            "Прокси",
            "Таймаут",
            "ЗащищенноеСоединение",
            "ИспользоватьАутентификациюОС"
        ]
    );
    assert_parameter_type(parameters, "Сервер", "Строка");
    assert_parameter_type(parameters, "Порт", "Число");
    assert_parameter_type(parameters, "Прокси", "ИнтернетПрокси");
    assert_parameter_type(parameters, "ИспользоватьАутентификациюОС", "Булево");
}

#[test]
fn parses_representative_specialized_fixture_pages() {
    let toc = fixture_toc();

    let global_context = parse_global_context(
        &fixture_content(&toc, "objects/Global context.html"),
        source("objects/Global context.html"),
    );
    assert_eq!(global_context.name.primary, "Глобальный контекст");
    assert!(
        global_context
            .method_links
            .iter()
            .any(|link| link.name.primary == "XMLСтрока"
                && link.name.alias.as_deref() == Some("XMLString"))
    );
    assert!(
        global_context
            .property_links
            .iter()
            .any(|link| link.name.primary == "WebSocketКлиентСоединения")
    );

    let global_method = parse_global_method(
        &fixture_content(
            &toc,
            "objects/Global context/methods/catalog1566/XMLString1567.html",
        ),
        source("objects/Global context/methods/catalog1566/XMLString1567.html"),
    );
    assert_eq!(global_method.name.primary, "XMLСтрока");
    assert_eq!(global_method.name.alias.as_deref(), Some("XMLString"));
    assert_eq!(global_method.signatures[0].text, "XMLСтрока(<Значение>)");
    assert!(global_method.signatures[0].parameters[0].required);
    assert!(
        global_method
            .return_types
            .iter()
            .any(|type_ref| type_ref.name == "Строка")
    );

    let global_property = parse_global_property(
        &fixture_content(&toc, "objects/Global context/properties/Catalogs336.html"),
        source("objects/Global context/properties/Catalogs336.html"),
    );
    assert_eq!(global_property.name.primary, "Справочники");
    assert_eq!(global_property.name.alias.as_deref(), Some("Catalogs"));
    assert_eq!(global_property.usage.as_deref(), Some("Только чтение."));
    assert_eq!(
        global_property.description.as_deref(),
        Some(
            "Тип: СправочникиМенеджер. Используется для доступа к определенным в конфигурации справочникам."
        )
    );
    assert!(
        global_property
            .type_refs
            .iter()
            .any(|type_ref| type_ref.name == "СправочникиМенеджер")
    );
    assert!(
        global_property
            .facts
            .see_also
            .iter()
            .any(|link| link.name.primary == "СправочникиМенеджер")
    );

    let platform_type = parse_platform_type(
        &fixture_content(&toc, "objects/catalog234/Array.html"),
        source("objects/catalog234/Array.html"),
    );
    assert_eq!(platform_type.name.primary, "Массив");
    assert!(
        platform_type
            .method_links
            .iter()
            .any(|link| link.name.alias.as_deref() == Some("Add"))
    );
    assert!(
        platform_type
            .constructor_links
            .iter()
            .any(|link| link.name.primary == "По количеству элементов")
    );
    assert_available_since(&platform_type.facts, "8.0");

    let method = parse_platform_method(
        &fixture_content(&toc, "objects/catalog234/Array/methods/Add772.html"),
        source("objects/catalog234/Array/methods/Add772.html"),
    );
    assert_eq!(method.owner.primary, "Массив");
    assert_eq!(method.name.primary, "Добавить");
    assert!(!method.signatures[0].parameters[0].required);
    assert_available_since(&method.facts, "8.0");

    let property = parse_platform_property(
        &fixture_content(
            &toc,
            "objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html",
        ),
        source("objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html"),
    );
    assert_eq!(property.owner.primary, "ГруппаФормы");
    assert_eq!(property.name.alias.as_deref(), Some("Visible"));
    assert_eq!(
        property.description.as_deref(),
        Some("Тип: Булево. Определяет видимость группы.")
    );
    assert!(
        property
            .type_refs
            .iter()
            .any(|type_ref| type_ref.name == "Булево")
    );
    assert_available_since(&property.facts, "8.2");

    let constructor = parse_constructor(
        &fixture_content(&toc, "objects/catalog234/Array/ctors/ctor13.html"),
        source("objects/catalog234/Array/ctors/ctor13.html"),
    );
    assert_eq!(constructor.owner.primary, "Массив");
    assert_eq!(constructor.name.primary, "По количеству элементов");
    assert_eq!(
        constructor.signatures[0].text,
        "Новый Массив(<КоличествоЭлементов1>,...,<КоличествоЭлементовN>)"
    );
    assert_available_since(&constructor.facts, "8.0");

    let enum_definition = parse_enum(
        &fixture_content(&toc, "objects/catalog2/catalog2300/JSONValueType.html"),
        source("objects/catalog2/catalog2300/JSONValueType.html"),
    );
    assert_eq!(enum_definition.name.primary, "ТипЗначенияJSON");
    assert!(
        enum_definition
            .value_links
            .iter()
            .any(|link| link.name.alias.as_deref() == Some("ArrayEnd"))
    );
    assert_available_since(&enum_definition.facts, "8.3.6");

    let enum_value = parse_enum_value(
        &fixture_content(
            &toc,
            "objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html",
        ),
        source("objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html"),
    );
    assert_eq!(enum_value.owner.primary, "ТипЗначенияJSON");
    assert_eq!(enum_value.name.primary, "КонецМассива");
    assert!(
        enum_value
            .description
            .as_deref()
            .is_some_and(|text| text.contains("JSON"))
    );
    assert_available_since(&enum_value.facts, "8.3.6");
}

#[test]
fn parses_locale_complete_type_references_and_clean_section_boundaries() {
    let toc = fixture_toc();

    let xml_string = parse_global_method(
        &fixture_content_from_raw(
            &toc,
            "shcntx_root.hbk",
            "en",
            "objects/Global context/methods/catalog1566/XMLString1567.html",
            include_str!("../../../tests/fixtures/syntax-helper/global_method_xmlstring_root.html"),
        ),
        source_for(
            "shcntx_root.hbk",
            "en",
            "objects/Global context/methods/catalog1566/XMLString1567.html",
        ),
    );
    assert!(
        xml_string
            .return_types
            .iter()
            .any(|type_ref| type_ref.name == "String")
    );
    let value_parameter = &xml_string.signatures[0].parameters[0];
    assert!(value_parameter.required);
    assert!(
        value_parameter
            .type_refs
            .iter()
            .any(|type_ref| type_ref.name == "Undefined")
    );
    assert!(
        value_parameter
            .type_refs
            .iter()
            .any(|type_ref| type_ref.name == "ValueStorage")
    );
    assert_clean_text(xml_string.description.as_deref().unwrap_or_default());
    assert_clean_text(value_parameter.description.as_deref().unwrap_or_default());
    assert_eq!(
        xml_string.facts.availability.contexts,
        vec![
            AvailabilityContext::ThinClient,
            AvailabilityContext::MobileClient,
            AvailabilityContext::Server,
            AvailabilityContext::ThickClient,
            AvailabilityContext::ExternalConnection,
            AvailabilityContext::MobileApplicationClient,
            AvailabilityContext::MobileApplicationServer,
            AvailabilityContext::MobileStandaloneServer,
        ]
    );
    assert!(
        xml_string
            .facts
            .examples
            .iter()
            .any(|example| example.text.contains("XMLWriter.WriteText"))
    );
    assert!(
        xml_string
            .facts
            .see_also
            .iter()
            .any(|link| link.name.primary == "Global context.XMLValue")
    );
    assert_available_since(&xml_string.facts, "8.0");

    let open_form_root = parse_global_method(
        &fixture_content_from_raw(
            &toc,
            "shcntx_root.hbk",
            "en",
            "objects/Global context/methods/catalog27/OpenForm3765.html",
            include_str!("../../../tests/fixtures/syntax-helper/global_method_openform_root.html"),
        ),
        source_for(
            "shcntx_root.hbk",
            "en",
            "objects/Global context/methods/catalog27/OpenForm3765.html",
        ),
    );
    assert!(
        open_form_root
            .return_types
            .iter()
            .any(|type_ref| type_ref.name == "Form")
    );
    assert!(
        open_form_root
            .return_types
            .iter()
            .any(|type_ref| type_ref.name == "ClientApplicationForm")
    );
    let root_parameters = &open_form_root.signatures[0].parameters;
    assert_parameter_type(root_parameters, "FormName", "String");
    assert_parameter_type(root_parameters, "Parameters", "Structure");
    assert_parameter_type(
        root_parameters,
        "WindowRepresentationMode",
        "FormWindowViewMode",
    );
    assert_clean_text(open_form_root.description.as_deref().unwrap_or_default());
    for parameter in root_parameters {
        assert_clean_text(parameter.description.as_deref().unwrap_or_default());
    }
    assert_eq!(
        open_form_root.facts.availability.contexts,
        vec![
            AvailabilityContext::ThinClient,
            AvailabilityContext::WebClient,
            AvailabilityContext::MobileClient,
            AvailabilityContext::ThickClient,
            AvailabilityContext::MobileApplicationClient,
        ]
    );
    assert!(
        open_form_root
            .facts
            .see_also
            .iter()
            .any(|link| link.name.primary == "GetForm")
    );
    assert_available_since(&open_form_root.facts, "8.2");

    let open_form_ru = parse_global_method(
        &fixture_content_from_raw(
            &toc,
            "shcntx_ru.hbk",
            "ru",
            "objects/Global context/methods/catalog27/OpenForm3765.html",
            include_str!("../../../tests/fixtures/syntax-helper/global_method_openform_ru.html"),
        ),
        source_for(
            "shcntx_ru.hbk",
            "ru",
            "objects/Global context/methods/catalog27/OpenForm3765.html",
        ),
    );
    assert!(
        open_form_ru
            .return_types
            .iter()
            .any(|type_ref| type_ref.name == "Форма")
    );
    let ru_parameters = &open_form_ru.signatures[0].parameters;
    assert_parameter_type(ru_parameters, "ИмяФормы", "Строка");
    assert_parameter_type(ru_parameters, "Параметры", "Структура");
    assert_parameter_type(
        ru_parameters,
        "РежимОтображенияОкна",
        "РежимОтображенияОкнаФормы",
    );
    assert_eq!(
        open_form_ru.facts.availability.contexts,
        vec![
            AvailabilityContext::ThinClient,
            AvailabilityContext::WebClient,
            AvailabilityContext::MobileClient,
            AvailabilityContext::ThickClient,
            AvailabilityContext::MobileApplicationClient,
        ]
    );
    assert!(
        open_form_ru
            .facts
            .see_also
            .iter()
            .any(|link| link.name.primary == "ПолучитьФорму")
    );
    assert_available_since(&open_form_ru.facts, "8.2");

    let array_add = parse_platform_method(
        &fixture_content_from_raw(
            &toc,
            "shcntx_root.hbk",
            "en",
            "objects/catalog234/Array/methods/Add772.html",
            include_str!("../../../tests/fixtures/syntax-helper/object_method_array_add_root.html"),
        ),
        source_for(
            "shcntx_root.hbk",
            "en",
            "objects/catalog234/Array/methods/Add772.html",
        ),
    );
    let value_parameter = &array_add.signatures[0].parameters[0];
    assert!(!value_parameter.required);
    assert!(
        value_parameter
            .type_refs
            .iter()
            .any(|type_ref| type_ref.name == "Arbitrary")
    );
    assert_clean_text(array_add.description.as_deref().unwrap_or_default());
    assert_clean_text(value_parameter.description.as_deref().unwrap_or_default());
    assert_eq!(
        array_add.facts.availability.contexts,
        vec![
            AvailabilityContext::ThinClient,
            AvailabilityContext::WebClient,
            AvailabilityContext::MobileClient,
            AvailabilityContext::Server,
            AvailabilityContext::ThickClient,
            AvailabilityContext::ExternalConnection,
            AvailabilityContext::MobileApplicationClient,
            AvailabilityContext::MobileApplicationServer,
            AvailabilityContext::MobileStandaloneServer,
        ]
    );
    assert!(
        array_add
            .facts
            .examples
            .iter()
            .any(|example| example.text.contains("Array.Add(\"First\")"))
    );
    assert_available_since(&array_add.facts, "8.0");

    let array_type = parse_platform_type(
        &fixture_content_from_raw(
            &toc,
            "shcntx_root.hbk",
            "en",
            "objects/catalog234/Array.html",
            include_str!("../../../tests/fixtures/syntax-helper/object_array_root.html"),
        ),
        source_for("shcntx_root.hbk", "en", "objects/catalog234/Array.html"),
    );
    assert_clean_text(array_type.description.as_deref().unwrap_or_default());
    assert_eq!(
        array_type.facts.availability.contexts,
        vec![
            AvailabilityContext::ThinClient,
            AvailabilityContext::WebClient,
            AvailabilityContext::MobileClient,
            AvailabilityContext::Server,
            AvailabilityContext::ThickClient,
            AvailabilityContext::ExternalConnection,
            AvailabilityContext::MobileApplicationClient,
            AvailabilityContext::MobileApplicationServer,
            AvailabilityContext::MobileStandaloneServer,
        ]
    );
    assert!(
        array_type
            .facts
            .examples
            .iter()
            .any(|example| example.text.contains("Array.Add(\"String added\")"))
    );
    assert_available_since(&array_type.facts, "8.0");
}

#[test]
fn parses_syntax_variants_as_structured_overloads() {
    let toc = fixture_toc();

    let method_ru = parse_platform_method(
        &fixture_content_from_raw(
            &toc,
            "shcntx_ru.hbk",
            "ru",
            "objects/catalog63/catalog1055/DOMDocument/methods/CreateNSResolver2613.html",
            include_str!(
                "../../../tests/fixtures/syntax-helper/object_method_domdocument_create_ns_resolver_ru.html"
            ),
        ),
        source_for(
            "shcntx_ru.hbk",
            "ru",
            "objects/catalog63/catalog1055/DOMDocument/methods/CreateNSResolver2613.html",
        ),
    );
    assert_eq!(method_ru.signatures.len(), 4);
    assert!(
        method_ru
            .return_types
            .iter()
            .any(|type_ref| type_ref.name == "РазыменовательПространствИменDOM")
    );
    let document_ru = assert_signature_variant(
        &method_ru.signatures,
        "На основе документа DOM",
        "СоздатьРазыменовательПИ()",
    );
    assert_variant_description_contains(document_ru, "пространств имен");
    let node_ru = assert_signature_variant(
        &method_ru.signatures,
        "На основании узла DOM",
        "СоздатьРазыменовательПИ(<УзелКонтекста>)",
    );
    assert_parameter_type(&node_ru.parameters, "УзелКонтекста", "АтрибутDOM");
    assert_parameter_type(&node_ru.parameters, "УзелКонтекста", "ДокументDOM");
    assert_parameter_type(&node_ru.parameters, "УзелКонтекста", "ЭлементDOM");
    let map_ru = assert_signature_variant(
        &method_ru.signatures,
        "На основании Соответствия",
        "СоздатьРазыменовательПИ(<Соответствие>)",
    );
    assert_parameter_type(&map_ru.parameters, "Соответствие", "Соответствие");
    let prefix_ru = assert_signature_variant(
        &method_ru.signatures,
        "На основании конкретного префикса и URI пространства имен",
        "СоздатьРазыменовательПИ(<Префикс>, <URIПространстваИмен>)",
    );
    assert_parameter_type(&prefix_ru.parameters, "Префикс", "Строка");
    assert_parameter_type(&prefix_ru.parameters, "URIПространстваИмен", "Строка");
    assert_variant_signatures_are_clean(&method_ru.signatures);

    let method_root = parse_platform_method(
        &fixture_content_from_raw(
            &toc,
            "shcntx_root.hbk",
            "en",
            "objects/catalog63/catalog1055/DOMDocument/methods/CreateNSResolver2613.html",
            include_str!(
                "../../../tests/fixtures/syntax-helper/object_method_domdocument_create_ns_resolver_root.html"
            ),
        ),
        source_for(
            "shcntx_root.hbk",
            "en",
            "objects/catalog63/catalog1055/DOMDocument/methods/CreateNSResolver2613.html",
        ),
    );
    assert_eq!(method_root.signatures.len(), 4);
    assert!(
        method_root
            .return_types
            .iter()
            .any(|type_ref| type_ref.name == "DOMNamespaceResolver")
    );
    let document_root = assert_signature_variant(
        &method_root.signatures,
        "On the basis of DOM document",
        "CreateNSResolver()",
    );
    assert_variant_description_contains(document_root, "namespaces defined in the document");
    let node_root = assert_signature_variant(
        &method_root.signatures,
        "On the basis of DOM node",
        "CreateNSResolver(<ContextNode>)",
    );
    assert_parameter_type(&node_root.parameters, "ContextNode", "DOMAttribute");
    assert_parameter_type(&node_root.parameters, "ContextNode", "DOMDocument");
    assert_parameter_type(&node_root.parameters, "ContextNode", "DOMElement");
    let map_root = assert_signature_variant(
        &method_root.signatures,
        "On the basis of a Map",
        "CreateNSResolver(<Map>)",
    );
    assert_parameter_type(&map_root.parameters, "Map", "Map");
    let prefix_root = assert_signature_variant(
        &method_root.signatures,
        "On the basis of specific prefix and namespace URI",
        "CreateNSResolver(<Prefix>, <NamespaceURI>)",
    );
    assert_parameter_type(&prefix_root.parameters, "Prefix", "String");
    assert_parameter_type(&prefix_root.parameters, "NamespaceURI", "String");
    assert_variant_signatures_are_clean(&method_root.signatures);

    let open_form_root = parse_global_method(
        &fixture_content_from_raw(
            &toc,
            "shcntx_root.hbk",
            "en",
            "objects/Global context/methods/catalog27/OpenForm3765.html",
            include_str!("../../../tests/fixtures/syntax-helper/global_method_openform_root.html"),
        ),
        source_for(
            "shcntx_root.hbk",
            "en",
            "objects/Global context/methods/catalog27/OpenForm3765.html",
        ),
    );
    assert_eq!(open_form_root.signatures.len(), 2);
    assert_signature_variant(
        &open_form_root.signatures,
        "By name",
        "OpenForm(<FormName>, <Parameters>, <Owner>, <Unique>, <Window>, <NavigationLink>, <NotifyOnCloseDescription>, <WindowOpeningMode>, <WindowRepresentationMode>)",
    );
    let by_form = assert_signature_variant(
        &open_form_root.signatures,
        "By form",
        "OpenForm(<Form>, <Window>)",
    );
    assert_parameter_type(&by_form.parameters, "Form", "Form");
    assert!(
        open_form_root
            .return_types
            .iter()
            .any(|type_ref| type_ref.name == "ClientApplicationForm")
    );
    assert_variant_signatures_are_clean(&open_form_root.signatures);
}

#[test]
fn extracts_platform_context_from_fixture_toc() {
    let toc = fixture_toc();
    let context = extract_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
        Ok(fixture_content(&toc, html_path))
    })
    .expect("fixture extraction must succeed");

    assert_eq!(context.global_contexts.len(), 1);
    assert!(
        context
            .global_methods
            .iter()
            .any(|method| method.name.alias.as_deref() == Some("XMLString"))
    );
    assert!(
        context
            .global_properties
            .iter()
            .any(|property| property.name.alias.as_deref() == Some("Catalogs"))
    );
    assert!(
        context
            .platform_types
            .iter()
            .any(|platform_type| platform_type.name.alias.as_deref() == Some("Array"))
    );
    assert!(
        context
            .type_methods
            .iter()
            .any(|method| method.name.alias.as_deref() == Some("Add"))
    );
    assert!(
        context
            .type_properties
            .iter()
            .any(|property| property.name.alias.as_deref() == Some("Visible"))
    );
    assert!(
        context
            .constructors
            .iter()
            .any(|constructor| constructor.name.primary == "По количеству элементов")
    );
    assert!(
        context
            .enums
            .iter()
            .any(|enum_definition| enum_definition.name.alias.as_deref() == Some("JSONValueType"))
    );
    assert!(
        context
            .enum_values
            .iter()
            .any(|enum_value| enum_value.name.alias.as_deref() == Some("ArrayEnd"))
    );
    assert_eq!(context.diagnostics.len(), 1);
}

#[test]
fn extraction_can_stream_fixture_records_in_deterministic_order() {
    let toc = fixture_toc();
    let mut sink = RecordingSink::default();

    extract_with_loader_into(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc,
        |html_path| Ok(fixture_content(&toc, html_path)),
        &mut sink,
    )
    .expect("fixture extraction must stream");

    let expected = [
        "diagnostic:UNKNOWN_PAGE_CLASS",
        "global_property:Справочники",
        "global_method:XMLСтрока",
        "global_context:Глобальный контекст",
        "enum:ТипЗначенияJSON",
        "enum_value:КонецМассива",
        "platform_type:Массив",
        "type_method:Добавить",
        "constructor:По количеству элементов",
        "type_property:Видимость",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(sink.seen, expected);
}

#[test]
fn binds_parameters_to_the_signature_that_mentions_them() {
    let toc = fixture_toc();
    let html = r#"
            <html><body>
            <h1 class="V8SH_pagetitle">Тест.Метод</h1>
            <p class="V8SH_title">Тест</p>
            <p class="V8SH_heading">Метод</p>
            <p class="V8SH_chapter">Синтаксис:</p>
            Метод()<br>
            Метод(&lt;СтрокаЗначение&gt;, &lt;ЧислоЗначение&gt;)
            <p class="V8SH_chapter">Параметры:</p>
            <div class="V8SH_rubric"><p>&lt;СтрокаЗначение&gt; (обязательный)</p></div>
            Тип: Строка. Первый параметр.
            <div class="V8SH_rubric"><p>&lt;ЧислоЗначение&gt; (необязательный)</p></div>
            Тип: Число. Второй параметр.
            </body></html>
        "#;
    let content = parse_syntax_page_content(
        Path::new("shcntx_ru.hbk"),
        "ru",
        &toc,
        "objects/catalog234/Test/methods/Method.html",
        html,
    );
    let signatures = parse_signatures(&content);

    assert_eq!(signatures.len(), 2);
    assert!(signatures[0].parameters.is_empty());
    assert_eq!(signatures[1].parameters.len(), 2);
    assert_eq!(signatures[1].parameters[0].name, "СтрокаЗначение");
    assert_eq!(signatures[1].parameters[1].name, "ЧислоЗначение");
    assert!(signatures[1].parameters[0].required);
    assert!(!signatures[1].parameters[1].required);
    assert_eq!(signatures[1].parameters[0].type_refs[0].name, "Строка");
    assert_eq!(signatures[1].parameters[1].type_refs[0].name, "Число");
}

#[test]
fn parses_inline_example_section_before_availability() {
    let toc = fixture_toc();
    let html = r##"
            <html><body>
            <h1 class="V8SH_pagetitle">Расширение поля формы для поля ввода.ПараметрыВыбора</h1>
            <p class="V8SH_title">Расширение поля формы для поля ввода</p>
            <p class="V8SH_heading">ПараметрыВыбора</p>
            <p class="V8SH_chapter">Описание:</p>
            Тип: <a href="objects/catalog234/FixedArray.html">ФиксированныйМассив</a>. <br>
            Определяет параметры выбора.<br><br>
            Пример:<br>
            <TABLE><TBODY><TR><TD><font face="Courier New">
            <font color="#0000ff">Элементы<font color="#ff0000">.</font>Реквизит1<font color="#ff0000">.</font>ПараметрыВыбора&nbsp;<font color="#ff0000">=</font>&nbsp;НовыеПараметры<font color="#ff0000">;</font></font>
            </font></TD></TR></TBODY></TABLE>
            <p class="V8SH_chapter">Доступность: </p>
            <p>Тонкий клиент, веб-клиент, сервер.</p>
            </body></html>
        "##;
    let property = parse_platform_property(
        &fixture_content_from_raw(
            &toc,
            "shcntx_ru.hbk",
            "ru",
            "objects/catalog1649/catalog1676/Form field extension for a text box/properties/ChoiceParameters8537.html",
            html,
        ),
        source(
            "objects/catalog1649/catalog1676/Form field extension for a text box/properties/ChoiceParameters8537.html",
        ),
    );

    assert_eq!(property.facts.examples.len(), 1);
    assert_eq!(
        property.facts.examples[0].text,
        "Элементы.Реквизит1.ПараметрыВыбора = НовыеПараметры;"
    );
    assert_ne!(
        property.facts.examples[0].text,
        "Тонкий клиент, веб-клиент, сервер."
    );
}

#[test]
fn parses_root_inline_example_section_and_normalizes_code_punctuation() {
    let toc = fixture_toc();
    let html = r##"
            <html><body>
            <h1 class="V8SH_pagetitle">TestType.TestMethod</h1>
            <p class="V8SH_title">TestType</p>
            <p class="V8SH_heading">TestMethod</p>
            <p class="V8SH_chapter">Syntax:</p>
            TestMethod()
            <p class="V8SH_chapter">Description:</p>
            Performs a test.<br><br>
            Example:<br>
            <TABLE><TBODY><TR><TD><font face="Courier New">
            <font color="#0000ff">Items<font color="#ff0000">.</font>Item1<font color="#ff0000">.</font>Value&nbsp;<font color="#ff0000">=</font>&nbsp;Data<font color="#ff0000">;</font><BR>Items<font color="#ff0000">.</font>Item1<font color="#ff0000">.</font>CreateColumns<font color="#ff0000">(</font><font color="#ff0000">)</font><font color="#ff0000">;</font></font>
            </font></TD></TR></TBODY></TABLE>
            <p class="V8SH_chapter">Availability: </p>
            <p>Thin client, server.</p>
            </body></html>
        "##;
    let method = parse_platform_method(
        &fixture_content_from_raw(
            &toc,
            "shcntx_root.hbk",
            "en",
            "objects/catalog234/TestType/methods/TestMethod.html",
            html,
        ),
        source_for(
            "shcntx_root.hbk",
            "en",
            "objects/catalog234/TestType/methods/TestMethod.html",
        ),
    );

    assert_eq!(method.facts.examples.len(), 1);
    assert_eq!(
        method.facts.examples[0].text,
        "Items.Item1.Value = Data;\nItems.Item1.CreateColumns();"
    );
    assert!(!method.facts.examples[0].text.contains("Thin client"));
}

#[test]
fn normalizes_multiline_example_after_string_continuation() {
    let toc = fixture_toc();
    let html = r##"
            <html><body>
            <h1 class="V8SH_pagetitle">ЗадачаОбъект.&lt;Имя задачи&gt;.Записать</h1>
            <p class="V8SH_title">ЗадачаОбъект.&lt;Имя задачи&gt;</p>
            <p class="V8SH_heading">Записать</p>
            <p class="V8SH_chapter">Описание:</p><p>Записывает задачу в базу данных.</p>
            <p class="V8SH_chapter">Пример:</p>
            <TABLE><TBODY><TR><TD><font face="Courier New">
            <font color="#0000ff"><font color="#ff0000">Попытка<BR></font>&nbsp;&nbsp;&nbsp;&nbsp;Объект<font color="#ff0000">.</font>Записать<font color="#ff0000">(</font><font color="#ff0000">)</font><font color="#ff0000">;</font><font color="#ff0000"><BR>Исключение<BR></font>&nbsp;&nbsp;&nbsp;&nbsp;Предупреждение<font color="#ff0000">(</font>НСтр<font color="#ff0000">(</font><font color="#000000">"ru&nbsp;=&nbsp;'Не&nbsp;удалось&nbsp;записать&nbsp;объект';"</font><BR>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<font color="#ff0000">+</font>&nbsp;<font color="#000000">"&nbsp;en&nbsp;=&nbsp;'Can't&nbsp;write&nbsp;object&nbsp;-'"</font><font color="#ff0000">)</font>&nbsp;<font color="#ff0000">+</font>&nbsp;<font color="#000000">"&nbsp;'"</font>&nbsp;<BR>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<font color="#ff0000">+</font>&nbsp;Объект&nbsp;<font color="#ff0000">+</font>&nbsp;'<font color="#000000">"!<BR>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;|"</font>&nbsp;<font color="#ff0000">+</font>&nbsp;ОписаниеОшибки<font color="#ff0000">(</font><font color="#ff0000">)</font><font color="#ff0000">,</font><font color="#000000">&nbsp;60</font><font color="#ff0000">)</font><font color="#ff0000">;</font><font color="#ff0000"><BR>КонецПопытки</font><font color="#ff0000">;</font></font>
            </font></TD></TR></TBODY></TABLE>
            </body></html>
        "##;
    let method = parse_platform_method(
        &fixture_content_from_raw(
            &toc,
            "shcntx_ru.hbk",
            "ru",
            "objects/catalog125/catalog719/object724/methods/Write1937.html",
            html,
        ),
        source("objects/catalog125/catalog719/object724/methods/Write1937.html"),
    );

    assert_eq!(method.facts.examples.len(), 1);
    assert!(
        !method.facts.examples[0]
            .text
            .contains("ОписаниеОшибки ( ) , 60 ) ;")
    );
    assert!(
        method.facts.examples[0]
            .text
            .contains("|\" + ОписаниеОшибки(), 60);"),
        "normalized example must remove spaces around call punctuation: {}",
        method.facts.examples[0].text
    );
}

#[test]
fn see_also_composes_owner_member_links() {
    let toc = fixture_toc();
    let html = r#"
            <html><body>
            <h1 class="V8SH_pagetitle">ЭлементИзбранногоРаботыПользователя</h1>
            <p class="V8SH_title">ЭлементИзбранногоРаботыПользователя</p>
            <p class="V8SH_chapter">Описание:</p><p>Элемент избранного.</p>
            <p class="V8SH_chapter">См. также:</p>
            <a href="v8help://SyntaxHelperContext/objects/catalog1649/catalog1620/UserWorkFavorites.html">ИзбранноеРаботыПользователя</a>, метод
            <a href="v8help://SyntaxHelperContext/objects/catalog1649/catalog1620/UserWorkFavorites/methods/Insert3711.html">Вставить</a><br>
            <a href="v8help://SyntaxHelperContext/objects/Global context.html">Глобальный контекст</a>, свойство
            <a href="v8help://SyntaxHelperContext/objects/Global context/properties/UserWorkHistory7232.html">ИсторияРаботыПользователя</a><br>
            </body></html>
        "#;
    let platform_type = parse_platform_type(
        &fixture_content_from_raw(
            &toc,
            "shcntx_ru.hbk",
            "ru",
            "objects/catalog1649/catalog1620/UserWorkFavoritesItem.html",
            html,
        ),
        source("objects/catalog1649/catalog1620/UserWorkFavoritesItem.html"),
    );
    let see_also = platform_type
        .facts
        .see_also
        .iter()
        .map(|link| link.name.primary.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        see_also,
        vec![
            "ИзбранноеРаботыПользователя.Вставить",
            "Глобальный контекст.ИсторияРаботыПользователя",
        ]
    );
}

#[test]
fn real_shcntx_ru_root_discovery_includes_required_root_candidates_when_fixture_exists() {
    let path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk");
    if !path.exists() {
        eprintln!(
            "real-platform root discovery smoke skipped because {} is unavailable",
            path.display()
        );
        return;
    }

    let book = HbkBook::open(path).expect("real Syntax Assistant book must open");
    let discovery = SyntaxHelperReader::new(&book)
        .discover_roots()
        .expect("real Syntax Assistant roots must be discoverable");

    assert!(discovery.has_kind(RootSectionKind::GlobalContext));
    assert!(discovery.has_kind(RootSectionKind::EnumCatalog));
    assert!(discovery.has_kind(RootSectionKind::TypeObjectCatalog));

    let global_context = discovery
        .roots
        .iter()
        .find(|root| root.kind == RootSectionKind::GlobalContext)
        .expect("global context root must be present");
    assert_eq!(
        global_context.source.html_path,
        "objects/Global context.html"
    );
    assert!(
        global_context
            .pages
            .iter()
            .any(|page| page.class == PageClass::GlobalMethod)
    );
    assert!(
        global_context
            .pages
            .iter()
            .any(|page| page.class == PageClass::GlobalProperty)
    );

    let enum_catalog = discovery
        .roots
        .iter()
        .find(|root| root.kind == RootSectionKind::EnumCatalog)
        .expect("enum catalog root must be present");
    assert!(
        enum_catalog
            .pages
            .iter()
            .any(|page| page.class == PageClass::Enum)
    );
    assert!(
        enum_catalog
            .pages
            .iter()
            .any(|page| page.class == PageClass::EnumValue)
    );

    let type_catalog = discovery
        .roots
        .iter()
        .find(|root| {
            root.kind == RootSectionKind::TypeObjectCatalog
                && root.source.html_path == "objects/catalog234.html"
        })
        .expect("known type/object catalog root must be present");
    assert!(
        type_catalog
            .pages
            .iter()
            .any(|page| page.class == PageClass::ObjectType)
    );
}

#[test]
fn real_shcntx_ru_extraction_returns_required_families_when_fixture_exists() {
    let path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk");
    if !path.exists() {
        eprintln!(
            "real-platform Syntax Assistant extraction smoke skipped because {} is unavailable",
            path.display()
        );
        return;
    }

    let book = HbkBook::open(path).expect("real Syntax Assistant book must open");
    let context = SyntaxHelperReader::new(&book)
        .extract()
        .expect("real Syntax Assistant extraction must succeed");

    assert!(!context.global_methods.is_empty());
    assert!(!context.global_properties.is_empty());
    assert!(!context.platform_types.is_empty());
    assert!(!context.query_tables.is_empty());
    assert!(!context.type_methods.is_empty());
    assert!(!context.type_properties.is_empty());
    assert!(!context.table_fields.is_empty());
    assert!(!context.table_parameters.is_empty());
    assert!(!context.constructors.is_empty());
    assert!(!context.enums.is_empty());
    assert!(!context.enum_values.is_empty());
}

#[test]
fn real_shcntx_root_root_discovery_includes_required_root_candidates_when_fixture_exists() {
    let path = Path::new("/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk");
    if !path.exists() {
        eprintln!(
            "real-platform root discovery smoke skipped because {} is unavailable",
            path.display()
        );
        return;
    }

    let book = HbkBook::open(path).expect("real root Syntax Assistant book must open");
    let discovery = SyntaxHelperReader::new(&book)
        .discover_roots()
        .expect("real root Syntax Assistant roots must be discoverable");

    assert!(discovery.has_kind(RootSectionKind::GlobalContext));
    assert!(discovery.has_kind(RootSectionKind::EnumCatalog));
    assert!(discovery.has_kind(RootSectionKind::TypeObjectCatalog));

    let enum_catalog = discovery
        .roots
        .iter()
        .find(|root| root.kind == RootSectionKind::EnumCatalog)
        .expect("root-source enum catalog root must be present");
    assert_eq!(enum_catalog.source.html_path, "objects/catalog2.html");
    assert!(
        enum_catalog
            .pages
            .iter()
            .any(|page| page.class == PageClass::Enum)
    );
}

fn fixture_toc() -> Toc {
    Toc::parse(
            r#"{
                14
                {1,0,2,2,3,{0,0,{0,0,{"ru","Глобальный контекст"}},"/objects/Global context.html"}}
                {2,1,1,4,{0,0,{0,0,{"ru","Свойства"}},"/objects/Global context/properties/catalog.html"}}
                {3,1,1,5,{0,0,{0,0,{"ru","Методы"}},"/objects/Global context/methods/catalog.html"}}
                {4,2,0,{0,0,{0,0,{"ru","Глобальный контекст.Справочники"}},"/objects/Global context/properties/Catalogs336.html"}}
                {5,3,0,{0,0,{0,0,{"ru","Глобальный контекст.XMLСтрока"}},"/objects/Global context/methods/catalog1566/XMLString1567.html"}}
                {6,0,1,7,{0,0,{0,0,{"ru","Системные перечисления"}},"/objects/catalog2.html"}}
                {7,6,1,8,{0,0,{0,0,{"ru","ТипЗначенияJSON"}},"/objects/catalog2/catalog2300/JSONValueType.html"}}
                {8,7,0,{0,0,{0,0,{"ru","ТипЗначенияJSON.КонецМассива"}},"/objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html"}}
                {9,0,1,10,{0,0,{0,0,{"ru","Универсальные коллекции значений"}},"/objects/catalog234.html"}}
                {10,9,3,11,12,13,{0,0,{0,0,{"ru","Массив"}},"/objects/catalog234/Array.html"}}
                {11,10,0,{0,0,{0,0,{"ru","Массив.Добавить"}},"/objects/catalog234/Array/methods/Add772.html"}}
                {12,10,0,{0,0,{0,0,{"ru","Массив.По количеству элементов"}},"/objects/catalog234/Array/ctors/ctor13.html"}}
                {13,10,0,{0,0,{0,0,{"ru","ГруппаФормы.Видимость"}},"/objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html"}}
                {14,0,0,{0,0,{0,0,{"ru","Неизвестный раздел"}},"/objects/unknown.html"}}
            }"#,
        )
        .expect("fixture TOC must parse")
}

fn fixture_content(toc: &Toc, html_path: &str) -> PageContent {
    let html = match html_path {
        "objects/Global context.html" => {
            include_str!("../../../tests/fixtures/syntax-helper/global_context_ru.html")
        }
        "objects/Global context/properties/Catalogs336.html" => {
            include_str!("../../../tests/fixtures/syntax-helper/global_property_catalogs_ru.html")
        }
        "objects/Global context/methods/catalog1566/XMLString1567.html" => {
            include_str!("../../../tests/fixtures/syntax-helper/global_method_xmlstring_ru.html")
        }
        "objects/catalog2.html" => {
            include_str!("../../../tests/fixtures/syntax-helper/root_catalog_enums_ru.html")
        }
        "objects/catalog2/catalog2300/JSONValueType.html" => {
            include_str!("../../../tests/fixtures/syntax-helper/enum_json_value_type_ru.html")
        }
        "objects/catalog2/catalog2300/JSONValueType/properties/ArrayEnd10574.html" => {
            include_str!("../../../tests/fixtures/syntax-helper/enum_value_json_array_end_ru.html")
        }
        "objects/catalog234.html" => {
            include_str!("../../../tests/fixtures/syntax-helper/root_catalog_types_ru.html")
        }
        "objects/catalog234/Array.html" => {
            include_str!("../../../tests/fixtures/syntax-helper/object_array_ru.html")
        }
        "objects/catalog234/Array/methods/Add772.html" => {
            include_str!("../../../tests/fixtures/syntax-helper/object_method_array_add_ru.html")
        }
        "objects/catalog234/Array/ctors/ctor13.html" => {
            include_str!("../../../tests/fixtures/syntax-helper/constructor_array_by_count_ru.html")
        }
        "objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html" => {
            include_str!(
                "../../../tests/fixtures/syntax-helper/object_property_formgroup_visible_ru.html"
            )
        }
        "objects/unknown.html" => {
            r#"<html><body><h1 class="V8SH_pagetitle">Неизвестный раздел</h1></body></html>"#
        }
        other => panic!("unexpected fixture page load: {other}"),
    };
    parse_page_html(
        Path::new("shcntx_ru.hbk"),
        "ru",
        toc,
        html_path,
        html,
        |_| false,
    )
}

fn fixture_content_from_raw(
    toc: &Toc,
    hbk_path: &str,
    locale: &str,
    html_path: &str,
    html: &str,
) -> PageContent {
    parse_syntax_page_content(Path::new(hbk_path), locale, toc, html_path, html)
}

fn source(html_path: &str) -> SyntaxHelperSource {
    source_for("shcntx_ru.hbk", "ru", html_path)
}

fn source_for(hbk_path: &str, locale: &str, html_path: &str) -> SyntaxHelperSource {
    SyntaxHelperSource {
        hbk_path: PathBuf::from(hbk_path),
        locale: locale.to_string(),
        toc_path: None,
        html_path: html_path.to_string(),
        page_title: String::new(),
    }
}

fn assert_parameter_type(parameters: &[Parameter], parameter_name: &str, type_name: &str) {
    let parameter = parameters
        .iter()
        .find(|parameter| parameter.name == parameter_name)
        .unwrap_or_else(|| panic!("parameter {parameter_name} must be parsed"));
    assert!(
        parameter
            .type_refs
            .iter()
            .any(|type_ref| type_ref.name == type_name),
        "parameter {parameter_name} must include type reference {type_name}"
    );
}

fn assert_signature_variant<'a>(
    signatures: &'a [Signature],
    title: &str,
    text: &str,
) -> &'a Signature {
    let signature = signatures
        .iter()
        .find(|signature| signature.text == text)
        .unwrap_or_else(|| panic!("signature {text} must be parsed"));
    let variant = signature
        .variant
        .as_ref()
        .unwrap_or_else(|| panic!("signature {text} must expose variant metadata"));
    assert_eq!(variant.title, title);
    signature
}

fn assert_variant_description_contains(signature: &Signature, expected: &str) {
    let variant = signature
        .variant
        .as_ref()
        .expect("signature must expose variant metadata");
    let description = variant
        .description
        .as_deref()
        .expect("variant description must be parsed");
    assert!(
        description.contains(expected),
        "variant description must contain {expected:?}: {description}"
    );
}

fn assert_variant_signatures_are_clean(signatures: &[Signature]) {
    for signature in signatures {
        assert_clean_text(&signature.text);
        let variant = signature
            .variant
            .as_ref()
            .expect("variant signature must expose variant metadata");
        assert_clean_text(&variant.title);
        if let Some(description) = variant.description.as_deref() {
            assert_clean_text(description);
        }
        for parameter in &signature.parameters {
            if let Some(description) = parameter.description.as_deref() {
                assert_clean_text(description);
            }
        }
    }
}

fn assert_available_since(facts: &SectionFacts, version: &str) {
    let available_since = facts
        .available_since
        .as_ref()
        .expect("available-since fact must be parsed");
    assert_eq!(available_since.version.as_deref(), Some(version));
    assert!(available_since.text.contains(version));
}

fn assert_clean_text(text: &str) {
    for forbidden in [
        "Доступность:",
        "Availability:",
        "Пример:",
        "Example:",
        "См. также:",
        "See also:",
        "Использование в версии:",
        "Available since:",
        "Возвращаемое значение:",
        "Return value:",
        "Returned value:",
        "Параметры:",
        "Parameters:",
        "Вариант синтаксиса:",
        "Syntax variant:",
        "Описание варианта метода:",
        "Description of method variant:",
        "Методическая информация",
        "Methodical information",
    ] {
        assert!(
            !text.contains(forbidden),
            "text must not contain raw section label {forbidden:?}: {text}"
        );
    }
}
