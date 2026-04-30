use std::collections::BTreeSet;
use std::convert::Infallible;
use std::path::{Path, PathBuf};

use super::*;
use hbk_book::HbkBook;
use hbk_book::Toc;
use hbk_docs::{PageContent, parse_page_html};

#[derive(Default)]
struct RecordingSink {
    seen: Vec<String>,
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

    fn platform_type(&mut self, record: PlatformType) -> Result<(), Self::Error> {
        self.push_name("platform_type", &record.name);
        Ok(())
    }

    fn type_method(&mut self, record: PlatformMethod) -> Result<(), Self::Error> {
        self.push_name("type_method", &record.name);
        Ok(())
    }

    fn type_property(&mut self, record: PlatformProperty) -> Result<(), Self::Error> {
        self.push_name("type_property", &record.name);
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
    assert!(
        global_property
            .type_refs
            .iter()
            .any(|type_ref| type_ref.name == "СправочникиМенеджер")
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

    let method = parse_platform_method(
        &fixture_content(&toc, "objects/catalog234/Array/methods/Add772.html"),
        source("objects/catalog234/Array/methods/Add772.html"),
    );
    assert_eq!(method.owner.primary, "Массив");
    assert_eq!(method.name.primary, "Добавить");
    assert!(!method.signatures[0].parameters[0].required);

    let property = parse_platform_property(
        &fixture_content(
            &toc,
            "objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html",
        ),
        source("objects/catalog1649/catalog1677/FormGroup/properties/Visible7192.html"),
    );
    assert_eq!(property.owner.primary, "ГруппаФормы");
    assert_eq!(property.name.alias.as_deref(), Some("Visible"));
    assert!(
        property
            .type_refs
            .iter()
            .any(|type_ref| type_ref.name == "Булево")
    );

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
fn lookup_helpers_find_exact_names_and_aliases() {
    let toc = fixture_toc();
    let context = extract_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
        Ok(fixture_content(&toc, html_path))
    })
    .expect("fixture extraction must succeed");

    let global_member = context
        .find_global_member("XMLString")
        .expect("global member lookup must not be ambiguous")
        .expect("global member must be found by alias");
    assert!(
        matches!(global_member, GlobalMemberRef::Method(method) if method.name.primary == "XMLСтрока")
    );

    let platform_type = context
        .find_type("Array")
        .expect("type lookup must not be ambiguous")
        .expect("type must be found by alias");
    assert_eq!(platform_type.name.primary, "Массив");

    let type_member = context
        .find_type_member("Array", "Add")
        .expect("type member lookup must not be ambiguous")
        .expect("type member must be found by aliases");
    assert!(
        matches!(type_member, TypeMemberRef::Method(method) if method.name.primary == "Добавить")
    );

    let constructors = context
        .constructors_for_type("Array")
        .expect("constructor lookup must not be ambiguous")
        .expect("type must be found by alias");
    assert_eq!(constructors.len(), 1);
    assert_eq!(constructors[0].name.primary, "По количеству элементов");
}

#[test]
fn lookup_helpers_return_missing_without_guessing() {
    let context = PlatformContext::default();

    assert_eq!(context.find_global_member("Missing").unwrap(), None);
    assert_eq!(context.find_type("Missing").unwrap(), None);
    assert_eq!(context.find_type_member("Array", "Missing").unwrap(), None);
    assert_eq!(context.constructors_for_type("Array").unwrap(), None);
}

#[test]
fn constructor_lookup_distinguishes_type_without_constructors() {
    let context = PlatformContext {
        platform_types: vec![PlatformType {
            name: LocalizedName {
                primary: "Тест".to_string(),
                alias: Some("Test".to_string()),
            },
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: None,
            source: source("objects/Test.html"),
        }],
        ..PlatformContext::default()
    };

    let constructors = context
        .constructors_for_type("Test")
        .expect("constructor lookup must not be ambiguous")
        .expect("existing type must be distinguished from a missing type");
    assert!(constructors.is_empty());
}

#[test]
fn type_bound_lookup_does_not_cross_match_alias_to_other_primary_name() {
    let aliased_type = LocalizedName {
        primary: "Тест".to_string(),
        alias: Some("Test".to_string()),
    };
    let other_type = LocalizedName {
        primary: "Test".to_string(),
        alias: None,
    };
    let context = PlatformContext {
        platform_types: vec![
            PlatformType {
                name: aliased_type,
                method_links: Vec::new(),
                constructor_links: Vec::new(),
                description: None,
                source: source("objects/AliasedTest.html"),
            },
            PlatformType {
                name: other_type.clone(),
                method_links: Vec::new(),
                constructor_links: Vec::new(),
                description: None,
                source: source("objects/OtherTest.html"),
            },
        ],
        type_methods: vec![PlatformMethod {
            owner: other_type.clone(),
            name: LocalizedName {
                primary: "Ping".to_string(),
                alias: None,
            },
            signatures: Vec::new(),
            return_types: Vec::new(),
            description: None,
            source: source("objects/OtherTest/methods/Ping.html"),
        }],
        constructors: vec![Constructor {
            owner: other_type,
            name: LocalizedName {
                primary: "New".to_string(),
                alias: None,
            },
            signatures: Vec::new(),
            description: None,
            source: source("objects/OtherTest/ctors/New.html"),
        }],
        ..PlatformContext::default()
    };

    assert_eq!(context.find_type_member("Тест", "Ping").unwrap(), None);
    assert!(
        context
            .constructors_for_type("Тест")
            .expect("type lookup must not be ambiguous")
            .expect("aliased type must exist")
            .is_empty()
    );
}

#[test]
fn lookup_helpers_report_ambiguous_exact_matches() {
    let toc = fixture_toc();
    let mut context = extract_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
        Ok(fixture_content(&toc, html_path))
    })
    .expect("fixture extraction must succeed");

    let duplicate_global = GlobalMethod {
        name: LocalizedName {
            primary: "XMLString".to_string(),
            alias: None,
        },
        signatures: Vec::new(),
        return_types: Vec::new(),
        description: None,
        source: source("objects/duplicate-global.html"),
    };
    context.global_methods.push(duplicate_global);

    assert!(matches!(
        context.find_global_member("XMLString"),
        Err(LookupError::Ambiguous {
            kind: LookupKind::GlobalMember,
            ..
        })
    ));

    let duplicate_member = PlatformProperty {
        owner: context
            .find_type("Array")
            .expect("type lookup must not be ambiguous before duplicate type")
            .expect("fixture type must exist")
            .name
            .clone(),
        name: LocalizedName {
            primary: "Add".to_string(),
            alias: None,
        },
        usage: None,
        type_refs: Vec::new(),
        description: None,
        source: source("objects/duplicate-member.html"),
    };
    context.type_properties.push(duplicate_member);

    assert!(matches!(
        context.find_type_member("Array", "Add"),
        Err(LookupError::Ambiguous {
            kind: LookupKind::TypeMember,
            ..
        })
    ));

    let duplicate_type = PlatformType {
        name: LocalizedName {
            primary: "Array".to_string(),
            alias: None,
        },
        method_links: Vec::new(),
        constructor_links: Vec::new(),
        description: None,
        source: source("objects/duplicate-type.html"),
    };
    context.platform_types.push(duplicate_type);

    assert!(matches!(
        context.find_type("Array"),
        Err(LookupError::Ambiguous {
            kind: LookupKind::Type,
            ..
        })
    ));

    let mut context_with_ambiguous_type =
        extract_with_loader(Path::new("shcntx_ru.hbk"), "ru", &toc, |html_path| {
            Ok(fixture_content(&toc, html_path))
        })
        .expect("fixture extraction must succeed");
    context_with_ambiguous_type
        .platform_types
        .push(PlatformType {
            name: LocalizedName {
                primary: "Array".to_string(),
                alias: None,
            },
            method_links: Vec::new(),
            constructor_links: Vec::new(),
            description: None,
            source: source("objects/duplicate-type.html"),
        });

    assert!(matches!(
        context_with_ambiguous_type.find_type_member("Array", "Add"),
        Err(LookupError::Ambiguous {
            kind: LookupKind::Type,
            ..
        })
    ));

    assert!(matches!(
        context.constructors_for_type("Array"),
        Err(LookupError::Ambiguous {
            kind: LookupKind::Type,
            ..
        })
    ));
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
    assert!(!context.type_methods.is_empty());
    assert!(!context.type_properties.is_empty());
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

fn source(html_path: &str) -> SyntaxHelperSource {
    SyntaxHelperSource {
        hbk_path: PathBuf::from("shcntx_ru.hbk"),
        locale: "ru".to_string(),
        toc_path: None,
        html_path: html_path.to_string(),
        page_title: String::new(),
    }
}
