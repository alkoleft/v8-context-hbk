use serde::Serialize;

use syntax_helper_model as model;

use crate::manifest::ExportFile;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExportMetadata<'a> {
    pub(crate) schema_version: u32,
    pub(crate) locale: &'a str,
    pub(crate) source_locale: &'a str,
    pub(crate) files: Vec<ExportFile>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RecordsEnvelope<'a, T: Serialize> {
    pub(crate) schema_version: u32,
    pub(crate) locale: &'a str,
    pub(crate) source_locale: &'a str,
    pub(crate) record_kind: &'static str,
    pub(crate) records: &'a [T],
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerLocalizedName<'a> {
    primary: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    alias: Option<&'a str>,
}

impl<'a> From<&'a model::LocalizedName> for ConsumerLocalizedName<'a> {
    fn from(name: &'a model::LocalizedName) -> Self {
        Self {
            primary: &name.primary,
            alias: name.alias.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerGlobalMethod<'a> {
    name: ConsumerLocalizedName<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    signatures: Vec<ConsumerSignature<'a>>,
    #[serde(rename = "return")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    return_types: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::GlobalMethod> for ConsumerGlobalMethod<'a> {
    fn from(method: &'a model::GlobalMethod) -> Self {
        Self {
            name: ConsumerLocalizedName::from(&method.name),
            signatures: consumer_signatures(&method.signatures),
            return_types: type_ref_names(&method.return_types),
            description: method.description.as_deref(),
            facts: ConsumerSectionFacts::from(&method.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerGlobalProperty<'a> {
    name: ConsumerLocalizedName<'a>,
    usage: &'static str,
    #[serde(rename = "types")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    type_refs: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::GlobalProperty> for ConsumerGlobalProperty<'a> {
    fn from(property: &'a model::GlobalProperty) -> Self {
        Self {
            name: ConsumerLocalizedName::from(&property.name),
            usage: property_usage(&property.usage),
            type_refs: type_ref_names(&property.type_refs),
            description: property_description(&property.description, &property.type_refs),
            facts: ConsumerSectionFacts::from(&property.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerGlobalContextEvent<'a> {
    record_family: model::RecordFamily,
    branch_kind: model::BranchKind,
    module: ConsumerModuleContext<'a>,
    name: ConsumerLocalizedName<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    signatures: Vec<ConsumerSignature<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::GlobalContextEvent> for ConsumerGlobalContextEvent<'a> {
    fn from(event: &'a model::GlobalContextEvent) -> Self {
        Self {
            record_family: event.semantic.record_family,
            branch_kind: event.semantic.branch_kind,
            module: ConsumerModuleContext::from(&event.module),
            name: ConsumerLocalizedName::from(&event.name),
            signatures: consumer_signatures(&event.signatures),
            description: event.description.as_deref(),
            facts: ConsumerSectionFacts::from(&event.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPlatformType<'a> {
    branch_kind: model::BranchKind,
    type_kind: model::PlatformTypeKind,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owner_path: Vec<&'a str>,
    name: ConsumerLocalizedName<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extends: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata_kind: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    template_parameters: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::PlatformType> for ConsumerPlatformType<'a> {
    fn from(platform_type: &'a model::PlatformType) -> Self {
        Self {
            branch_kind: platform_type.semantic.branch_kind,
            type_kind: platform_type.type_kind,
            owner_path: semantic_owner_path(&platform_type.semantic),
            name: ConsumerLocalizedName::from(&platform_type.name),
            extends: localized_name_primaries(&platform_type.extends),
            metadata_kind: platform_type.metadata_kind.as_deref(),
            template_parameters: platform_type
                .template_parameters
                .iter()
                .map(String::as_str)
                .collect(),
            description: platform_type.description.as_deref(),
            facts: ConsumerSectionFacts::from(&platform_type.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPlatformMethod<'a> {
    owner: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owner_path: Vec<&'a str>,
    name: ConsumerLocalizedName<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    signatures: Vec<ConsumerSignature<'a>>,
    #[serde(rename = "return")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    return_types: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::PlatformMethod> for ConsumerPlatformMethod<'a> {
    fn from(method: &'a model::PlatformMethod) -> Self {
        Self {
            owner: &method.owner.primary,
            owner_path: semantic_owner_path(&method.semantic),
            name: ConsumerLocalizedName::from(&method.name),
            signatures: consumer_signatures(&method.signatures),
            return_types: type_ref_names(&method.return_types),
            description: method.description.as_deref(),
            facts: ConsumerSectionFacts::from(&method.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPlatformProperty<'a> {
    owner: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owner_path: Vec<&'a str>,
    name: ConsumerLocalizedName<'a>,
    usage: &'static str,
    #[serde(rename = "types")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    type_refs: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::PlatformProperty> for ConsumerPlatformProperty<'a> {
    fn from(property: &'a model::PlatformProperty) -> Self {
        Self {
            owner: &property.owner.primary,
            owner_path: semantic_owner_path(&property.semantic),
            name: ConsumerLocalizedName::from(&property.name),
            usage: property_usage(&property.usage),
            type_refs: type_ref_names(&property.type_refs),
            description: property_description(&property.description, &property.type_refs),
            facts: ConsumerSectionFacts::from(&property.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerQueryTableField<'a> {
    owner: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owner_path: Vec<&'a str>,
    name: ConsumerLocalizedName<'a>,
    #[serde(rename = "types")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    type_refs: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<&'a str>,
}

impl<'a> From<&'a model::QueryTableField> for ConsumerQueryTableField<'a> {
    fn from(field: &'a model::QueryTableField) -> Self {
        Self {
            owner: &field.owner.primary,
            owner_path: semantic_owner_path(&field.semantic),
            name: ConsumerLocalizedName::from(&field.name),
            type_refs: type_ref_names(&field.type_refs),
            description: field.description.as_deref(),
            note: field.note.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerQueryTableParameter<'a> {
    owner: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owner_path: Vec<&'a str>,
    name: ConsumerLocalizedName<'a>,
    required: bool,
    #[serde(rename = "types")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    type_refs: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_value: Option<&'a str>,
}

impl<'a> From<&'a model::QueryTableParameter> for ConsumerQueryTableParameter<'a> {
    fn from(parameter: &'a model::QueryTableParameter) -> Self {
        Self {
            owner: &parameter.owner.primary,
            owner_path: semantic_owner_path(&parameter.semantic),
            name: ConsumerLocalizedName::from(&parameter.name),
            required: parameter.required,
            type_refs: type_ref_names(&parameter.type_refs),
            description: parameter.description.as_deref(),
            default_value: parameter.default_value.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerConstructor<'a> {
    owner: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owner_path: Vec<&'a str>,
    name: ConsumerLocalizedName<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    signatures: Vec<ConsumerSignature<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::Constructor> for ConsumerConstructor<'a> {
    fn from(constructor: &'a model::Constructor) -> Self {
        Self {
            owner: &constructor.owner.primary,
            owner_path: semantic_owner_path(&constructor.semantic),
            name: ConsumerLocalizedName::from(&constructor.name),
            signatures: consumer_signatures(&constructor.signatures),
            description: constructor.description.as_deref(),
            facts: ConsumerSectionFacts::from(&constructor.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerModuleContext<'a> {
    kind: model::ModuleKind,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    owner_path: Vec<&'a str>,
}

impl<'a> From<&'a model::ModuleEventContext> for ConsumerModuleContext<'a> {
    fn from(module: &'a model::ModuleEventContext) -> Self {
        Self {
            kind: module.kind,
            owner_path: localized_name_primaries(&module.owner_path),
        }
    }
}

fn semantic_owner_path(semantic: &model::SemanticContext) -> Vec<&str> {
    localized_name_primaries(&semantic.owner_path)
}

fn localized_name_primaries(names: &[model::LocalizedName]) -> Vec<&str> {
    names.iter().map(|name| name.primary.as_str()).collect()
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerEnumDefinition<'a> {
    name: ConsumerLocalizedName<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    values: Vec<ConsumerEnumValue<'a>>,
}

impl<'a> ConsumerEnumDefinition<'a> {
    fn new(
        enum_definition: &'a model::EnumDefinition,
        enum_values: Vec<&'a model::EnumValue>,
    ) -> Self {
        let enum_since = version_since(&enum_definition.facts.available_since);
        Self {
            name: ConsumerLocalizedName::from(&enum_definition.name),
            description: enum_definition.description.as_deref(),
            facts: ConsumerSectionFacts::from(&enum_definition.facts),
            values: enum_values
                .into_iter()
                .map(|value| ConsumerEnumValue::new(value, enum_since))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerEnumValue<'a> {
    name: ConsumerLocalizedName<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    availability: Option<ConsumerAvailability<'a>>,
}

impl<'a> ConsumerEnumValue<'a> {
    fn new(enum_value: &'a model::EnumValue, enum_since: Option<&'a str>) -> Self {
        let value_since = version_since(&enum_value.facts.available_since);
        Self {
            name: ConsumerLocalizedName::from(&enum_value.name),
            description: enum_value.description.as_deref(),
            availability: (value_since.is_some() && value_since != enum_since).then(|| {
                ConsumerAvailability {
                    contexts: Vec::new(),
                    since: value_since,
                }
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerSectionFacts<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    availability: Option<ConsumerAvailability<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    examples: Vec<&'a model::ExampleBlock>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    see_also: Vec<&'a str>,
}

impl<'a> From<&'a model::SectionFacts> for ConsumerSectionFacts<'a> {
    fn from(facts: &'a model::SectionFacts) -> Self {
        Self {
            availability: ConsumerAvailability::from_facts(facts),
            examples: facts.examples.iter().collect(),
            see_also: facts
                .see_also
                .iter()
                .map(|link| link.name.primary.as_str())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerAvailability<'a> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    contexts: Vec<model::AvailabilityContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<&'a str>,
}

impl<'a> ConsumerAvailability<'a> {
    fn from_facts(facts: &'a model::SectionFacts) -> Option<Self> {
        let contexts = facts.availability.contexts.clone();
        let since = version_since(&facts.available_since);
        (!contexts.is_empty() || since.is_some()).then_some(Self { contexts, since })
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerSignature<'a> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parameters: Vec<ConsumerParameter<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
}

impl<'a> From<&'a model::Signature> for ConsumerSignature<'a> {
    fn from(signature: &'a model::Signature) -> Self {
        Self {
            parameters: signature
                .parameters
                .iter()
                .map(ConsumerParameter::from)
                .collect(),
            title: signature
                .variant
                .as_ref()
                .map(|variant| variant.title.as_str())
                .filter(|title| !title.is_empty()),
            description: signature
                .variant
                .as_ref()
                .and_then(|variant| variant.description.as_deref()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerParameter<'a> {
    name: &'a str,
    required: bool,
    #[serde(rename = "types")]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    type_refs: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
}

impl<'a> From<&'a model::Parameter> for ConsumerParameter<'a> {
    fn from(parameter: &'a model::Parameter) -> Self {
        Self {
            name: &parameter.name,
            required: parameter.required,
            type_refs: type_ref_names(&parameter.type_refs),
            description: parameter.description.as_deref(),
        }
    }
}

pub(crate) fn consumer_enums<'a>(
    enums: &'a [model::EnumDefinition],
    enum_values: &'a [model::EnumValue],
) -> Vec<ConsumerEnumDefinition<'a>> {
    let grouped_values = group_enum_values(enums, enum_values);
    enums
        .iter()
        .zip(grouped_values)
        .map(|(enum_definition, enum_values)| {
            ConsumerEnumDefinition::new(enum_definition, enum_values)
        })
        .collect()
}

fn group_enum_values<'a>(
    enums: &'a [model::EnumDefinition],
    enum_values: &'a [model::EnumValue],
) -> Vec<Vec<&'a model::EnumValue>> {
    let mut grouped_values = vec![Vec::new(); enums.len()];
    for enum_value in enum_values {
        if let Some(owner_index) = enum_owner_index(enums, enum_value) {
            grouped_values[owner_index].push(enum_value);
        }
    }
    grouped_values
}

fn enum_owner_index(
    enums: &[model::EnumDefinition],
    enum_value: &model::EnumValue,
) -> Option<usize> {
    enums
        .iter()
        .position(|enum_definition| enum_definition.name == enum_value.owner)
        .or_else(|| {
            unique_enum_position(enums, |enum_definition| {
                enum_definition.name.primary == enum_value.owner.primary
            })
        })
        .or_else(|| {
            unique_enum_position(enums, |enum_definition| {
                enum_definition.name.matches(&enum_value.owner.primary)
                    || enum_value.owner.matches(&enum_definition.name.primary)
                    || enum_value
                        .owner
                        .alias
                        .as_deref()
                        .is_some_and(|alias| enum_definition.name.matches(alias))
            })
        })
}

fn unique_enum_position(
    enums: &[model::EnumDefinition],
    mut matches: impl FnMut(&model::EnumDefinition) -> bool,
) -> Option<usize> {
    let mut positions = enums
        .iter()
        .enumerate()
        .filter_map(|(index, enum_definition)| matches(enum_definition).then_some(index));
    let first = positions.next()?;
    positions.next().is_none().then_some(first)
}

fn consumer_signatures(signatures: &[model::Signature]) -> Vec<ConsumerSignature<'_>> {
    signatures.iter().map(ConsumerSignature::from).collect()
}

fn type_ref_names(type_refs: &[model::TypeRef]) -> Vec<&str> {
    type_refs
        .iter()
        .map(|type_ref| type_ref.name.as_str())
        .collect()
}

fn version_since(version_fact: &Option<model::VersionFact>) -> Option<&str> {
    version_fact
        .as_ref()
        .and_then(|fact| fact.version.as_deref())
}

fn property_usage(usage: &Option<String>) -> &'static str {
    let Some(usage) = usage.as_deref() else {
        return "Unknown";
    };
    let usage = usage
        .trim()
        .trim_matches('.')
        .trim()
        .to_lowercase()
        .replace('\u{2011}', "-")
        .replace('\u{2013}', "-");
    match usage.as_str() {
        "только чтение" | "read only" | "read" | "чтение" => "Read",
        "только запись" | "write only" | "write" | "запись" => "Write",
        "чтение и запись" | "read and write" | "read/write" | "read, write" => {
            "ReadWrite"
        }
        _ => "Unknown",
    }
}

fn property_description(
    description: &Option<String>,
    type_refs: &[model::TypeRef],
) -> Option<String> {
    let description = description.as_deref()?;
    let description = if type_refs.is_empty() {
        description
    } else {
        strip_leading_property_type_prose(description)
    };
    let description = description.trim();
    (!description.is_empty()).then(|| description.to_string())
}

fn strip_leading_property_type_prose(description: &str) -> &str {
    let trimmed = description.trim_start();
    let Some(type_prose) = trimmed
        .strip_prefix("Тип:")
        .or_else(|| trimmed.strip_prefix("Type:"))
    else {
        return description;
    };
    type_prose
        .split_once('.')
        .map(|(_, remainder)| remainder.trim_start())
        .unwrap_or("")
}
