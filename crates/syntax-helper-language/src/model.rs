#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageSourceFamily {
    Shlang,
    Shquery,
    Dcsui,
}

impl LanguageSourceFamily {
    pub fn source_id(self) -> &'static str {
        match self {
            Self::Shlang => "shlang",
            Self::Shquery => "shquery",
            Self::Dcsui => "dcsui",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageDomain {
    BslLanguage,
    QueryLanguage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageFactFamily {
    Construct,
    Type,
    Function,
    Operator,
    Keyword,
    Literal,
}

impl LanguageFactFamily {
    pub fn document_kind(self) -> &'static str {
        match self {
            Self::Construct => "language_construct",
            Self::Type => "language_type",
            Self::Function => "language_function",
            Self::Operator => "language_operator",
            Self::Keyword => "language_keyword",
            Self::Literal => "language_literal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageFact {
    pub id: String,
    pub source_family: LanguageSourceFamily,
    pub domain: LanguageDomain,
    pub family: LanguageFactFamily,
    pub name: LocalizedName,
    pub syntax: Option<String>,
    pub signatures: Vec<LanguageSignature>,
    pub type_refs: Vec<String>,
    pub return_types: Vec<String>,
    pub description: Option<String>,
    pub provenance: LanguageFactProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageSignature {
    pub text: String,
    pub parameters: Vec<LanguageParameter>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageParameter {
    pub name: String,
    pub required: bool,
    pub type_refs: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageFactProvenance {
    pub source_hbk: String,
    pub locale: String,
    pub html_path: String,
    pub page_title: String,
    pub anchor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguagePageInput<'a> {
    pub source_hbk: &'a str,
    pub source_family: LanguageSourceFamily,
    pub locale: &'a str,
    pub html_path: &'a str,
    pub html: &'a str,
}
