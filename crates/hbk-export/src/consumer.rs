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
pub(crate) struct ConsumerGlobalMethod<'a> {
    name: &'a model::LocalizedName,
    signatures: &'a [model::Signature],
    return_types: &'a [model::TypeRef],
    description: &'a Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::GlobalMethod> for ConsumerGlobalMethod<'a> {
    fn from(method: &'a model::GlobalMethod) -> Self {
        Self {
            name: &method.name,
            signatures: &method.signatures,
            return_types: &method.return_types,
            description: &method.description,
            facts: ConsumerSectionFacts::from(&method.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerGlobalProperty<'a> {
    name: &'a model::LocalizedName,
    usage: &'a Option<String>,
    type_refs: &'a [model::TypeRef],
    description: &'a Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::GlobalProperty> for ConsumerGlobalProperty<'a> {
    fn from(property: &'a model::GlobalProperty) -> Self {
        Self {
            name: &property.name,
            usage: &property.usage,
            type_refs: &property.type_refs,
            description: &property.description,
            facts: ConsumerSectionFacts::from(&property.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerGlobalContextEvent<'a> {
    name: &'a model::LocalizedName,
    signatures: &'a [model::Signature],
    description: &'a Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::GlobalContextEvent> for ConsumerGlobalContextEvent<'a> {
    fn from(event: &'a model::GlobalContextEvent) -> Self {
        Self {
            name: &event.name,
            signatures: &event.signatures,
            description: &event.description,
            facts: ConsumerSectionFacts::from(&event.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPlatformType<'a> {
    name: &'a model::LocalizedName,
    description: &'a Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::PlatformType> for ConsumerPlatformType<'a> {
    fn from(platform_type: &'a model::PlatformType) -> Self {
        Self {
            name: &platform_type.name,
            description: &platform_type.description,
            facts: ConsumerSectionFacts::from(&platform_type.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPlatformMethod<'a> {
    owner: &'a model::LocalizedName,
    name: &'a model::LocalizedName,
    signatures: &'a [model::Signature],
    return_types: &'a [model::TypeRef],
    description: &'a Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::PlatformMethod> for ConsumerPlatformMethod<'a> {
    fn from(method: &'a model::PlatformMethod) -> Self {
        Self {
            owner: &method.owner,
            name: &method.name,
            signatures: &method.signatures,
            return_types: &method.return_types,
            description: &method.description,
            facts: ConsumerSectionFacts::from(&method.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerPlatformProperty<'a> {
    owner: &'a model::LocalizedName,
    name: &'a model::LocalizedName,
    usage: &'a Option<String>,
    type_refs: &'a [model::TypeRef],
    description: &'a Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::PlatformProperty> for ConsumerPlatformProperty<'a> {
    fn from(property: &'a model::PlatformProperty) -> Self {
        Self {
            owner: &property.owner,
            name: &property.name,
            usage: &property.usage,
            type_refs: &property.type_refs,
            description: &property.description,
            facts: ConsumerSectionFacts::from(&property.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerQueryTableField<'a> {
    owner: &'a model::LocalizedName,
    name: &'a model::LocalizedName,
    type_refs: &'a [model::TypeRef],
    description: &'a Option<String>,
    note: &'a Option<String>,
}

impl<'a> From<&'a model::QueryTableField> for ConsumerQueryTableField<'a> {
    fn from(field: &'a model::QueryTableField) -> Self {
        Self {
            owner: &field.owner,
            name: &field.name,
            type_refs: &field.type_refs,
            description: &field.description,
            note: &field.note,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerQueryTableParameter<'a> {
    owner: &'a model::LocalizedName,
    name: &'a model::LocalizedName,
    required: bool,
    type_refs: &'a [model::TypeRef],
    description: &'a Option<String>,
    default_value: &'a Option<String>,
}

impl<'a> From<&'a model::QueryTableParameter> for ConsumerQueryTableParameter<'a> {
    fn from(parameter: &'a model::QueryTableParameter) -> Self {
        Self {
            owner: &parameter.owner,
            name: &parameter.name,
            required: parameter.required,
            type_refs: &parameter.type_refs,
            description: &parameter.description,
            default_value: &parameter.default_value,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerConstructor<'a> {
    owner: &'a model::LocalizedName,
    name: &'a model::LocalizedName,
    signatures: &'a [model::Signature],
    description: &'a Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::Constructor> for ConsumerConstructor<'a> {
    fn from(constructor: &'a model::Constructor) -> Self {
        Self {
            owner: &constructor.owner,
            name: &constructor.name,
            signatures: &constructor.signatures,
            description: &constructor.description,
            facts: ConsumerSectionFacts::from(&constructor.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerEnumDefinition<'a> {
    name: &'a model::LocalizedName,
    description: &'a Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::EnumDefinition> for ConsumerEnumDefinition<'a> {
    fn from(enum_definition: &'a model::EnumDefinition) -> Self {
        Self {
            name: &enum_definition.name,
            description: &enum_definition.description,
            facts: ConsumerSectionFacts::from(&enum_definition.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ConsumerEnumValue<'a> {
    owner: &'a model::LocalizedName,
    name: &'a model::LocalizedName,
    description: &'a Option<String>,
    #[serde(flatten)]
    facts: ConsumerSectionFacts<'a>,
}

impl<'a> From<&'a model::EnumValue> for ConsumerEnumValue<'a> {
    fn from(enum_value: &'a model::EnumValue) -> Self {
        Self {
            owner: &enum_value.owner,
            name: &enum_value.name,
            description: &enum_value.description,
            facts: ConsumerSectionFacts::from(&enum_value.facts),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerSectionFacts<'a> {
    availability: &'a model::Availability,
    examples: &'a [model::ExampleBlock],
    see_also: Vec<ConsumerSeeAlsoRef<'a>>,
    available_since: &'a Option<model::VersionFact>,
}

impl<'a> From<&'a model::SectionFacts> for ConsumerSectionFacts<'a> {
    fn from(facts: &'a model::SectionFacts) -> Self {
        Self {
            availability: &facts.availability,
            examples: &facts.examples,
            see_also: facts
                .see_also
                .iter()
                .map(ConsumerSeeAlsoRef::from)
                .collect(),
            available_since: &facts.available_since,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ConsumerSeeAlsoRef<'a> {
    name: &'a model::LocalizedName,
}

impl<'a> From<&'a model::MemberLink> for ConsumerSeeAlsoRef<'a> {
    fn from(link: &'a model::MemberLink) -> Self {
        Self { name: &link.name }
    }
}
