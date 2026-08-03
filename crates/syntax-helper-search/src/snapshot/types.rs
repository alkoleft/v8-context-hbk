use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringId(pub(super) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkPlatformTypeId(pub(super) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkTypeMemberId(pub(super) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkCallableId(pub(super) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkGlobalFactId(pub(super) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkQueryTableId(pub(super) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkQueryFieldId(pub(super) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkQueryParameterId(pub(super) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkLanguageFactId(pub(super) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkEnumId(pub(super) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HbkEnumValueId(pub(super) u32);

/// Compatibility identity supplied by the caller when opening an X1 slot.
///
/// These values are deliberately external to the artifact: a slot is never
/// allowed to declare itself compatible with the running platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkFactSnapshotExpectation {
    pub platform_version: String,
    pub locale: String,
    pub source_locale: String,
    pub source_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HbkAvailabilityFilterMode {
    Any,
    All,
}

/// Validated provider-native availability predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HbkAvailabilityFilter {
    pub(super) requested_mask: u16,
    pub(super) mode: HbkAvailabilityFilterMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkAvailabilityFilterError {
    code: String,
}

impl HbkAvailabilityFilter {
    pub fn any<'a>(
        codes: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, HbkAvailabilityFilterError> {
        Self::from_codes(codes, HbkAvailabilityFilterMode::Any)
    }

    pub fn all<'a>(
        codes: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, HbkAvailabilityFilterError> {
        Self::from_codes(codes, HbkAvailabilityFilterMode::All)
    }

    /// Matches every fact, including facts with explicit availability.
    pub const fn unfiltered() -> Self {
        Self {
            requested_mask: 0,
            mode: HbkAvailabilityFilterMode::All,
        }
    }

    pub const fn mode(self) -> HbkAvailabilityFilterMode {
        self.mode
    }

    pub const fn is_unfiltered(self) -> bool {
        self.requested_mask == 0 && matches!(self.mode, HbkAvailabilityFilterMode::All)
    }

    fn from_codes<'a>(
        codes: impl IntoIterator<Item = &'a str>,
        mode: HbkAvailabilityFilterMode,
    ) -> Result<Self, HbkAvailabilityFilterError> {
        let mut requested_mask = 0_u16;
        for code in codes {
            let bit =
                availability_context_code_bit(code).ok_or_else(|| HbkAvailabilityFilterError {
                    code: code.to_string(),
                })?;
            requested_mask |= bit;
        }
        Ok(Self {
            requested_mask,
            mode,
        })
    }

    pub(super) fn includes_mask(self, available_mask: u16, has_explicit: bool) -> bool {
        if !has_explicit {
            return true;
        }
        match self.mode {
            HbkAvailabilityFilterMode::Any => available_mask & self.requested_mask != 0,
            HbkAvailabilityFilterMode::All => {
                available_mask & self.requested_mask == self.requested_mask
            }
        }
    }
}

impl HbkAvailabilityFilterError {
    pub fn code(&self) -> &str {
        &self.code
    }
}

impl std::fmt::Display for HbkAvailabilityFilterError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "unknown HBK availability context code '{}'",
            self.code
        )
    }
}

impl std::error::Error for HbkAvailabilityFilterError {}

pub(super) fn availability_context_code_bit(code: &str) -> Option<u16> {
    match code {
        "thin_client" => Some(1 << 0),
        "web_client" => Some(1 << 1),
        "mobile_client" => Some(1 << 2),
        "server" => Some(1 << 3),
        "thick_client" => Some(1 << 4),
        "external_connection" => Some(1 << 5),
        "mobile_application_client" => Some(1 << 6),
        "mobile_application_server" => Some(1 << 7),
        "mobile_standalone_server" => Some(1 << 8),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkName {
    pub primary: StringId,
    pub alias: Option<StringId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HbkTypeMemberKind {
    Property,
    Method,
    Event,
    EnumValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HbkCallableKind {
    Method,
    Constructor,
    GlobalMethod,
    Event,
    LanguageFunction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HbkGlobalFactKind {
    Method,
    Property,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HbkLanguageDomain {
    Bsl,
    Query,
    DataComposition,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkPlatformType {
    pub id: StringId,
    pub name: HbkName,
    pub metadata_template: Option<HbkMetadataTemplate>,
    pub type_template_key: Option<HbkPlatformTypeTemplateKey>,
    pub availability_contexts: Vec<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkMetadataTemplate {
    pub metadata_kind: StringId,
    pub template_parameters: Vec<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkTypeMember {
    pub id: StringId,
    pub owner: HbkPlatformTypeId,
    pub kind: HbkTypeMemberKind,
    pub name: HbkName,
    pub type_refs: Vec<HbkTypeRef>,
    pub availability_contexts: Vec<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkCallable {
    pub id: StringId,
    pub owner: Option<HbkPlatformTypeId>,
    pub kind: HbkCallableKind,
    pub name: HbkName,
    pub signatures: Vec<HbkSignature>,
    pub return_type_refs: Vec<HbkTypeRef>,
    pub availability_contexts: Vec<StringId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HbkFactSource {
    pub(super) hbk_path: StringId,
    pub(super) locale: StringId,
    pub(super) toc_path: Option<StringId>,
    pub(super) html_path: StringId,
    pub(super) page_title: StringId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkSignature {
    pub text: StringId,
    pub parameters: Vec<HbkParameter>,
    pub return_type_refs: Vec<HbkTypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkParameter {
    pub name: StringId,
    pub required: bool,
    pub type_refs: Vec<HbkTypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkGlobalFact {
    pub id: StringId,
    pub kind: HbkGlobalFactKind,
    pub domain: HbkLanguageDomain,
    pub name: HbkName,
    pub callable: Option<HbkCallableId>,
    pub type_refs: Vec<HbkTypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkQueryTable {
    pub id: StringId,
    pub name: HbkName,
    pub syntax: Option<HbkName>,
    pub identifier: Option<StringId>,
    pub role: Option<model::QueryTableRole>,
    pub owner_path: Vec<HbkName>,
    pub template_parameters: Vec<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkQueryField {
    pub id: StringId,
    pub owner: HbkQueryTableId,
    pub name: HbkName,
    pub type_refs: Vec<HbkTypeRef>,
    pub note: Option<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkQueryParameter {
    pub id: StringId,
    pub owner: HbkQueryTableId,
    pub name: HbkName,
    pub type_refs: Vec<HbkTypeRef>,
    pub default_value: Option<StringId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkLanguageFact {
    pub id: StringId,
    pub kind: SearchDocumentKind,
    pub domain: HbkLanguageDomain,
    pub name: HbkName,
    pub signatures: Vec<HbkSignature>,
    pub type_refs: Vec<HbkTypeRef>,
    pub return_type_refs: Vec<HbkTypeRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkEnum {
    pub id: StringId,
    pub name: HbkName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkEnumValue {
    pub id: StringId,
    pub owner: HbkEnumId,
    pub name: HbkName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkTypeRef {
    pub name: StringId,
    pub target: HbkTypeRefTarget,
    pub type_template_key: Option<HbkPlatformTypeTemplateKey>,
    pub template_binding: Option<HbkTypeTemplateBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HbkTypeRefTarget {
    Ok(StringId),
    Unresolved,
    Ambiguous(Vec<StringId>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HbkPlatformTypeTemplateKey {
    pub family: StringId,
    pub variant: StringId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbkTypeTemplateBinding {
    pub template_key: HbkPlatformTypeTemplateKey,
    pub arguments: Vec<model::TemplateParameterBinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HbkFactRef {
    PlatformType(HbkPlatformTypeId),
    TypeMember(HbkTypeMemberId),
    Callable(HbkCallableId),
    Global(HbkGlobalFactId),
    QueryTable(HbkQueryTableId),
    QueryField(HbkQueryFieldId),
    QueryParameter(HbkQueryParameterId),
    LanguageFact(HbkLanguageFactId),
    Enum(HbkEnumId),
    EnumValue(HbkEnumValueId),
}
