#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "../../tests/fixtures/syntax-helper-language";

    #[test]
    fn extracts_bsl_string_type_and_function_construct() {
        let string = fixture_fact(
            LanguageSourceFamily::Shlang,
            "ru",
            "def_String",
            "shlang_def_string_ru.html",
        );
        assert_eq!(string.id, "shlang:def_String");
        assert_eq!(string.domain, LanguageDomain::BslLanguage);
        assert_eq!(string.family, LanguageFactFamily::Type);
        assert_eq!(string.name.primary, "Строка");
        assert_eq!(string.name.alias.as_deref(), Some("String"));

        let func = fixture_fact(
            LanguageSourceFamily::Shlang,
            "ru",
            "def_Func",
            "shlang_def_func_ru.html",
        );
        assert_eq!(func.id, "shlang:def_Func");
        assert_eq!(func.family, LanguageFactFamily::Construct);
        assert!(
            func.syntax
                .as_deref()
                .is_some_and(|text| text.contains("Функция"))
        );
    }

    #[test]
    fn extracts_direct_bsl_primitive_type_pages() {
        let expected = [
            ("def_Null", "shlang_def_null_ru.html", "Null", None),
            (
                "def_Undefined",
                "shlang_def_undefined_ru.html",
                "Неопределено",
                Some("Undefined"),
            ),
            (
                "def_Number",
                "shlang_def_number_ru.html",
                "Число",
                Some("Number"),
            ),
            (
                "def_String",
                "shlang_def_string_ru.html",
                "Строка",
                Some("String"),
            ),
            (
                "def_Date",
                "shlang_def_date_ru.html",
                "Дата",
                Some("Date"),
            ),
            (
                "def_Boolean",
                "shlang_def_boolean_ru.html",
                "Булево",
                Some("Boolean"),
            ),
            ("def_Type", "shlang_def_type_ru.html", "Тип", Some("Type")),
        ];

        for (html_path, fixture_name, primary, alias) in expected {
            let fact = fixture_fact(LanguageSourceFamily::Shlang, "ru", html_path, fixture_name);
            assert_eq!(fact.id, format!("shlang:{html_path}"));
            assert_eq!(fact.domain, LanguageDomain::BslLanguage);
            assert_eq!(fact.family, LanguageFactFamily::Type);
            assert_eq!(fact.name.primary, primary);
            assert_eq!(fact.name.alias.as_deref(), alias);
            assert!(
                fact.description.is_some(),
                "{html_path} must preserve source-backed description"
            );
        }
    }

    #[test]
    fn ignores_nested_bsl_primitive_literal_pages() {
        let facts = fixture_facts(
            LanguageSourceFamily::Shlang,
            "ru",
            "def_BooleanTrue",
            "shlang_def_boolean_true_ru.html",
        );

        assert!(facts.is_empty());
    }

    #[test]
    fn extracts_query_construct_function_and_literal() {
        let select = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "SELECTStatement",
            "shquery_select_statement_ru.html",
        );
        assert_eq!(select.domain, LanguageDomain::QueryLanguage);
        assert_eq!(select.family, LanguageFactFamily::Construct);
        assert!(
            select
                .syntax
                .as_deref()
                .is_some_and(|text| text.contains("ВЫБРАТЬ"))
        );

        let string = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "STRING",
            "shquery_string_ru.html",
        );
        assert_eq!(string.id, "shquery:STRING");
        assert_eq!(string.family, LanguageFactFamily::Function);
        assert_eq!(string.name.primary, "СТРОКА");
        assert!(string.return_types.iter().any(|value| value == "Строка"));
        let signature = string
            .signatures
            .first()
            .expect("query string function must keep a structured signature");
        assert_eq!(signature.text, "СТРОКА (<Значение>)");
        let parameter = signature
            .parameters
            .first()
            .expect("query string function must keep a structured parameter");
        assert_eq!(parameter.name, "Значение");
        assert!(parameter.type_refs.iter().any(|value| value == "Строка"));

        let sum = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "SUM",
            "shquery_sum_ru.html",
        );
        assert_eq!(sum.id, "shquery:SUM");
        assert_eq!(sum.family, LanguageFactFamily::Function);
        assert_eq!(sum.name.primary, "СУММА");
        assert_eq!(sum.name.alias.as_deref(), Some("SUM"));
        assert!(sum.syntax.is_none());

        let literal = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "LitString",
            "shquery_lit_string_ru.html",
        );
        assert_eq!(literal.id, "shquery:LitString");
        assert_eq!(literal.family, LanguageFactFamily::Literal);
    }

    #[test]
    fn extracts_root_source_language_fixtures_without_locale_identity_suffixes() {
        let bsl = fixture_fact(
            LanguageSourceFamily::Shlang,
            "root",
            "def_String",
            "shlang_def_string_root.html",
        );
        assert_eq!(bsl.id, "shlang:def_String");
        assert_eq!(bsl.name.primary, "String");
        assert_eq!(bsl.provenance.page_title, "String");

        let sum = fixture_fact(
            LanguageSourceFamily::Shquery,
            "root",
            "SUM",
            "shquery_sum_root.html",
        );
        assert_eq!(sum.id, "shquery:SUM");
        assert_eq!(sum.name.primary, "SUM");

        let string = fixture_fact(
            LanguageSourceFamily::Shquery,
            "root",
            "STRING",
            "shquery_string_root.html",
        );
        assert_eq!(string.id, "shquery:STRING");
        assert_eq!(string.name.primary, "STRING");
    }

    #[test]
    fn extracts_dcsui_string_function_and_query_extension_keywords() {
        let facts = fixture_facts(
            LanguageSourceFamily::Dcsui,
            "ru",
            "SKD_Functions_Strings",
            "dcsui_functions_strings_ru.html",
        );
        let string_length = facts
            .iter()
            .find(|fact| fact.id == "dcsui:SKD_Functions_Strings#StringLength")
            .expect("StringLength fact must be extracted");
        assert_eq!(string_length.domain, LanguageDomain::QueryLanguage);
        assert_eq!(string_length.family, LanguageFactFamily::Function);
        assert_eq!(string_length.name.primary, "ДлинаСтроки");
        assert_eq!(
            string_length.provenance.anchor.as_deref(),
            Some("StringLength")
        );
        let signature = string_length
            .signatures
            .first()
            .expect("SKD string function must keep a structured signature");
        assert_eq!(signature.text, "ДлинаСтроки(<Строка>)");
        let parameter = signature
            .parameters
            .first()
            .expect("SKD string function must keep a structured parameter");
        assert_eq!(parameter.name, "Строка");
        assert_eq!(parameter.type_refs, vec!["Строка".to_string()]);

        let facts = fixture_facts(
            LanguageSourceFamily::Dcsui,
            "ru",
            "SKD_ExtQueryLangv",
            "dcsui_ext_query_lang_ru.html",
        );
        assert!(facts.iter().any(|fact| fact.name.primary == "{ВЫБРАТЬ}"));
        assert!(facts.iter().any(|fact| fact.name.primary == "{ГДЕ}"));
    }

    #[test]
    fn identity_keeps_same_display_names_separate() {
        let bsl = fixture_fact(
            LanguageSourceFamily::Shlang,
            "ru",
            "def_String",
            "shlang_def_string_ru.html",
        );
        let query = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "STRING",
            "shquery_string_ru.html",
        );
        let literal = fixture_fact(
            LanguageSourceFamily::Shquery,
            "ru",
            "LitString",
            "shquery_lit_string_ru.html",
        );
        assert_eq!(bsl.name.primary, "Строка");
        assert_ne!(bsl.id, query.id);
        assert_ne!(bsl.id, literal.id);
        assert_ne!(query.id, literal.id);
    }

    fn fixture_fact(
        source_family: LanguageSourceFamily,
        locale: &str,
        html_path: &str,
        fixture_name: &str,
    ) -> LanguageFact {
        let facts = fixture_facts(source_family, locale, html_path, fixture_name);
        facts
            .iter()
            .find(|fact| fact.id.ends_with(html_path))
            .or_else(|| facts.first())
            .cloned()
            .expect("fixture must produce at least one fact")
    }

    fn fixture_facts(
        source_family: LanguageSourceFamily,
        locale: &str,
        html_path: &str,
        fixture_name: &str,
    ) -> Vec<LanguageFact> {
        let html = std::fs::read_to_string(format!("{BASE}/{fixture_name}"))
            .expect("fixture must be readable");
        extract_language_facts(LanguagePageInput {
            source_hbk: "fixture.hbk",
            source_family,
            locale,
            html_path,
            html: &html,
        })
    }
}
