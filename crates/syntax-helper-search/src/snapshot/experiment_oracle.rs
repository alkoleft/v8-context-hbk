//! Canonical, logical snapshot oracle used only by zero-copy experiments.
//!
//! The oracle deliberately serializes public meaning rather than the owned
//! snapshot's numeric IDs or physical layout. It is JSONL so comparison can be
//! streamed and a mismatch can be reported at the first logical record.

use std::borrow::Borrow;
use std::io::{self, Write};

use super::*;

const ORACLE_SCHEMA_VERSION: u32 = 1;
const LOOKUP_TRANSCRIPT_SCHEMA_VERSION: u32 = 1;
const MISSING_LOOKUP_KEY: &str = "__hbk_snapshot_oracle_missing__";

/// Writes the complete logical contents of an owned snapshot as canonical JSONL.
///
/// This is intentionally hidden from normal API documentation. Candidate
/// readers must implement the same schema without first materializing an owned
/// `HbkFactSnapshot`.
#[doc(hidden)]
pub fn write_owned_snapshot_oracle_jsonl(
    snapshot: &HbkFactSnapshot,
    mut writer: impl Write,
) -> io::Result<()> {
    write_oracle_header(snapshot, &mut writer)?;
    write_string_dictionary(snapshot, &mut writer)?;
    write_platform_types(snapshot, &mut writer)?;
    write_type_members(snapshot, &mut writer)?;
    write_callables(snapshot, &mut writer)?;
    write_globals(snapshot, &mut writer)?;
    write_query_tables(snapshot, &mut writer)?;
    write_query_fields(snapshot, &mut writer)?;
    write_query_parameters(snapshot, &mut writer)?;
    write_language_facts(snapshot, &mut writer)?;
    write_enums(snapshot, &mut writer)?;
    write_enum_values(snapshot, &mut writer)?;
    write_fact_state(snapshot, &mut writer)?;
    write_relations(snapshot, &mut writer)
}

/// Writes deterministic public lookup calls and their ordered logical results.
///
/// Hit cases are derived exhaustively from the indexes. Fixed absent keys cover
/// miss behavior. Result ordering is preserved because ordering is observable
/// through the read-handle iterators.
#[doc(hidden)]
pub fn write_owned_snapshot_lookup_transcript_jsonl(
    snapshot: &HbkFactSnapshot,
    mut writer: impl Write,
) -> io::Result<()> {
    writeln!(
        writer,
        "{{\"record\":\"lookup_header\",\"schema\":{LOOKUP_TRANSCRIPT_SCHEMA_VERSION}}}"
    )?;
    write_id_and_name_lookups(snapshot, &mut writer)?;
    write_platform_lookups(snapshot, &mut writer)?;
    write_member_and_callable_lookups(snapshot, &mut writer)?;
    write_global_and_module_lookups(snapshot, &mut writer)?;
    write_query_lookups(snapshot, &mut writer)?;
    write_language_and_enum_lookups(snapshot, &mut writer)?;
    write_state_lookups(snapshot, &mut writer)
}

fn write_string_dictionary(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    let mut strings: Vec<_> = snapshot.strings.iter().map(String::as_str).collect();
    strings.sort_unstable();
    for pair in strings.windows(2) {
        if pair[0] == pair[1] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "snapshot string dictionary contains duplicate text",
            ));
        }
    }
    for text in strings {
        write!(writer, "{{\"record\":\"string\",\"text\":")?;
        write_json_string(writer, text)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_oracle_header(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    let counts = snapshot.counts();
    write!(
        writer,
        "{{\"record\":\"snapshot\",\"schema\":{ORACLE_SCHEMA_VERSION},\"source_locale\":"
    )?;
    write_option_string(writer, snapshot.source_locale())?;
    writeln!(
        writer,
        ",\"counts\":{{\"strings\":{},\"platform_types\":{},\"type_members\":{},\"callables\":{},\"globals\":{},\"query_tables\":{},\"query_fields\":{},\"query_parameters\":{},\"language_facts\":{},\"enums\":{},\"enum_values\":{}}}}}",
        counts.strings,
        counts.platform_types,
        counts.type_members,
        counts.callables,
        counts.globals,
        counts.query_tables,
        counts.query_fields,
        counts.query_parameters,
        counts.language_facts,
        counts.enums,
        counts.enum_values,
    )
}

fn write_platform_types(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    for (index, fact) in snapshot.platform_types.iter().enumerate() {
        let fact_ref = HbkFactRef::PlatformType(HbkPlatformTypeId(index as u32));
        write!(writer, "{{\"record\":\"platform_type\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"name\":")?;
        write_name(writer, snapshot, &fact.name)?;
        write!(writer, ",\"metadata_template\":")?;
        if let Some(template) = &fact.metadata_template {
            write!(writer, "{{\"metadata_kind\":")?;
            write_string_id(writer, snapshot, template.metadata_kind)?;
            write!(writer, ",\"template_parameters\":")?;
            write_string_ids(writer, snapshot, &template.template_parameters)?;
            write!(writer, "}}")?;
        } else {
            write!(writer, "null")?;
        }
        write!(writer, ",\"type_template_key\":")?;
        write_optional_template_key(writer, snapshot, fact.type_template_key)?;
        write!(writer, ",\"availability_contexts\":")?;
        write_string_ids(writer, snapshot, &fact.availability_contexts)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_type_members(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    for (index, fact) in snapshot.type_members.iter().enumerate() {
        let fact_ref = HbkFactRef::TypeMember(HbkTypeMemberId(index as u32));
        write!(writer, "{{\"record\":\"type_member\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"owner\":")?;
        write_fact_key(writer, snapshot, HbkFactRef::PlatformType(fact.owner))?;
        write!(writer, ",\"kind\":")?;
        write_json_string(writer, member_kind(fact.kind))?;
        write!(writer, ",\"name\":")?;
        write_name(writer, snapshot, &fact.name)?;
        write!(writer, ",\"type_refs\":")?;
        write_type_refs(writer, snapshot, &fact.type_refs)?;
        write!(writer, ",\"availability_contexts\":")?;
        write_string_ids(writer, snapshot, &fact.availability_contexts)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_callables(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    for (index, fact) in snapshot.callables.iter().enumerate() {
        let fact_ref = HbkFactRef::Callable(HbkCallableId(index as u32));
        write!(writer, "{{\"record\":\"callable\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"owner\":")?;
        if let Some(owner) = fact.owner {
            write_fact_key(writer, snapshot, HbkFactRef::PlatformType(owner))?;
        } else {
            write!(writer, "null")?;
        }
        write!(writer, ",\"kind\":")?;
        write_json_string(writer, callable_kind(fact.kind))?;
        write!(writer, ",\"name\":")?;
        write_name(writer, snapshot, &fact.name)?;
        write!(writer, ",\"signatures\":")?;
        write_signatures(writer, snapshot, &fact.signatures)?;
        write!(writer, ",\"return_type_refs\":")?;
        write_type_refs(writer, snapshot, &fact.return_type_refs)?;
        write!(writer, ",\"availability_contexts\":")?;
        write_string_ids(writer, snapshot, &fact.availability_contexts)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_globals(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    for (index, fact) in snapshot.globals.iter().enumerate() {
        let fact_ref = HbkFactRef::Global(HbkGlobalFactId(index as u32));
        write!(writer, "{{\"record\":\"global\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"kind\":")?;
        write_json_string(writer, global_kind(fact.kind))?;
        write!(writer, ",\"domain\":")?;
        write_json_string(writer, language_domain(fact.domain))?;
        write!(writer, ",\"name\":")?;
        write_name(writer, snapshot, &fact.name)?;
        write!(writer, ",\"callable\":")?;
        if let Some(callable) = fact.callable {
            write_fact_key(writer, snapshot, HbkFactRef::Callable(callable))?;
        } else {
            write!(writer, "null")?;
        }
        write!(writer, ",\"type_refs\":")?;
        write_type_refs(writer, snapshot, &fact.type_refs)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_query_tables(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    for (index, fact) in snapshot.query_tables.iter().enumerate() {
        let fact_ref = HbkFactRef::QueryTable(HbkQueryTableId(index as u32));
        write!(writer, "{{\"record\":\"query_table\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"name\":")?;
        write_name(writer, snapshot, &fact.name)?;
        write!(writer, ",\"syntax\":")?;
        if let Some(syntax) = &fact.syntax {
            write_name(writer, snapshot, syntax)?;
        } else {
            write!(writer, "null")?;
        }
        write!(writer, ",\"identifier\":")?;
        write_optional_string_id(writer, snapshot, fact.identifier)?;
        write!(writer, ",\"role\":")?;
        if let Some(role) = fact.role {
            write_json_string(writer, query_table_role(role))?;
        } else {
            write!(writer, "null")?;
        }
        write!(writer, ",\"owner_path\":[")?;
        for (path_index, name) in fact.owner_path.iter().enumerate() {
            write_separator(writer, path_index)?;
            write_name(writer, snapshot, name)?;
        }
        write!(writer, "],\"template_parameters\":")?;
        write_string_ids(writer, snapshot, &fact.template_parameters)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_query_fields(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    for (index, fact) in snapshot.query_fields.iter().enumerate() {
        let fact_ref = HbkFactRef::QueryField(HbkQueryFieldId(index as u32));
        write!(writer, "{{\"record\":\"query_field\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"owner\":")?;
        write_fact_key(writer, snapshot, HbkFactRef::QueryTable(fact.owner))?;
        write!(writer, ",\"name\":")?;
        write_name(writer, snapshot, &fact.name)?;
        write!(writer, ",\"type_refs\":")?;
        write_type_refs(writer, snapshot, &fact.type_refs)?;
        write!(writer, ",\"note\":")?;
        write_optional_string_id(writer, snapshot, fact.note)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_query_parameters(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    for (index, fact) in snapshot.query_parameters.iter().enumerate() {
        let fact_ref = HbkFactRef::QueryParameter(HbkQueryParameterId(index as u32));
        write!(writer, "{{\"record\":\"query_parameter\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"owner\":")?;
        write_fact_key(writer, snapshot, HbkFactRef::QueryTable(fact.owner))?;
        write!(writer, ",\"name\":")?;
        write_name(writer, snapshot, &fact.name)?;
        write!(writer, ",\"type_refs\":")?;
        write_type_refs(writer, snapshot, &fact.type_refs)?;
        write!(writer, ",\"default_value\":")?;
        write_optional_string_id(writer, snapshot, fact.default_value)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_language_facts(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    for (index, fact) in snapshot.language_facts.iter().enumerate() {
        let fact_ref = HbkFactRef::LanguageFact(HbkLanguageFactId(index as u32));
        write!(writer, "{{\"record\":\"language_fact\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"kind\":")?;
        write_json_string(writer, fact.kind.as_str())?;
        write!(writer, ",\"domain\":")?;
        write_json_string(writer, language_domain(fact.domain))?;
        write!(writer, ",\"name\":")?;
        write_name(writer, snapshot, &fact.name)?;
        write!(writer, ",\"signatures\":")?;
        write_signatures(writer, snapshot, &fact.signatures)?;
        write!(writer, ",\"type_refs\":")?;
        write_type_refs(writer, snapshot, &fact.type_refs)?;
        write!(writer, ",\"return_type_refs\":")?;
        write_type_refs(writer, snapshot, &fact.return_type_refs)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_enums(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    for (index, fact) in snapshot.enums.iter().enumerate() {
        let fact_ref = HbkFactRef::Enum(HbkEnumId(index as u32));
        write!(writer, "{{\"record\":\"enum\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"name\":")?;
        write_name(writer, snapshot, &fact.name)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_enum_values(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    for (index, fact) in snapshot.enum_values.iter().enumerate() {
        let fact_ref = HbkFactRef::EnumValue(HbkEnumValueId(index as u32));
        write!(writer, "{{\"record\":\"enum_value\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"owner\":")?;
        write_fact_key(writer, snapshot, HbkFactRef::Enum(fact.owner))?;
        write!(writer, ",\"name\":")?;
        write_name(writer, snapshot, &fact.name)?;
        writeln!(writer, "}}")?;
    }
    Ok(())
}

fn write_fact_state(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    try_for_each_fact(snapshot, |fact_ref| {
        write!(writer, "{{\"record\":\"fact_state\",\"key\":")?;
        write_fact_key(writer, snapshot, fact_ref)?;
        write!(writer, ",\"availability_contexts\":")?;
        write_string_ids(
            writer,
            snapshot,
            snapshot.availability_by_fact.values(fact_ref),
        )?;
        write!(writer, ",\"available_since\":")?;
        let available_since = snapshot
            .availability_since_by_fact
            .binary_search_by(|candidate| candidate.fact.cmp(&fact_ref))
            .ok()
            .map(|position| snapshot.availability_since_by_fact[position].value);
        write_optional_string_id(writer, snapshot, available_since)?;
        writeln!(writer, "}}")
    })
}

fn write_relations(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    try_for_each_fact(snapshot, |source| {
        let mut kinds: Vec<_> = snapshot
            .relations_by_source_kind
            .keys
            .iter()
            .filter(|key| key.source == source)
            .map(|key| (snapshot.string(key.kind), key.kind))
            .collect();
        kinds.sort_unstable_by(|left, right| left.0.cmp(right.0));
        for (kind, kind_id) in kinds {
            write!(writer, "{{\"record\":\"relation\",\"source\":")?;
            write_fact_key(writer, snapshot, source)?;
            write!(writer, ",\"kind\":")?;
            write_json_string(writer, kind)?;
            write!(writer, ",\"targets\":")?;
            write_fact_keys(
                writer,
                snapshot,
                snapshot
                    .relations_by_source_kind
                    .values(RelationLookupKey {
                        source,
                        kind: kind_id,
                    })
                    .iter()
                    .copied(),
            )?;
            writeln!(writer, "}}")?;
        }
        Ok(())
    })
}

fn write_name(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    name: &HbkName,
) -> io::Result<()> {
    write!(writer, "{{\"primary\":")?;
    write_string_id(writer, snapshot, name.primary)?;
    write!(writer, ",\"alias\":")?;
    write_optional_string_id(writer, snapshot, name.alias)?;
    write!(writer, "}}")
}

fn write_signatures(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    signatures: &[HbkSignature],
) -> io::Result<()> {
    write!(writer, "[")?;
    for (signature_index, signature) in signatures.iter().enumerate() {
        write_separator(writer, signature_index)?;
        write!(writer, "{{\"text\":")?;
        write_string_id(writer, snapshot, signature.text)?;
        write!(writer, ",\"parameters\":[")?;
        for (parameter_index, parameter) in signature.parameters.iter().enumerate() {
            write_separator(writer, parameter_index)?;
            write!(writer, "{{\"name\":")?;
            write_string_id(writer, snapshot, parameter.name)?;
            write!(writer, ",\"required\":{}", parameter.required)?;
            write!(writer, ",\"type_refs\":")?;
            write_type_refs(writer, snapshot, &parameter.type_refs)?;
            write!(writer, "}}")?;
        }
        write!(writer, "],\"return_type_refs\":")?;
        write_type_refs(writer, snapshot, &signature.return_type_refs)?;
        write!(writer, "}}")?;
    }
    write!(writer, "]")
}

fn write_type_refs(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    type_refs: &[HbkTypeRef],
) -> io::Result<()> {
    write!(writer, "[")?;
    for (index, type_ref) in type_refs.iter().enumerate() {
        write_separator(writer, index)?;
        write!(writer, "{{\"name\":")?;
        write_string_id(writer, snapshot, type_ref.name)?;
        write!(writer, ",\"target\":")?;
        write_type_ref_target(writer, snapshot, &type_ref.target)?;
        write!(writer, ",\"type_template_key\":")?;
        write_optional_template_key(writer, snapshot, type_ref.type_template_key)?;
        write!(writer, ",\"template_binding\":")?;
        write_template_binding(writer, snapshot, type_ref.template_binding.as_ref())?;
        write!(writer, "}}")?;
    }
    write!(writer, "]")
}

fn write_type_ref_target(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    target: &HbkTypeRefTarget,
) -> io::Result<()> {
    match target {
        HbkTypeRefTarget::Ok(id) => {
            write!(writer, "{{\"status\":\"ok\",\"id\":")?;
            write_string_id(writer, snapshot, *id)?;
            write!(writer, ",\"facts\":")?;
            write_fact_refs_for_string_id(writer, snapshot, *id)?;
            write!(writer, "}}")
        }
        HbkTypeRefTarget::Unresolved => write!(writer, "{{\"status\":\"unresolved\"}}"),
        HbkTypeRefTarget::Ambiguous(candidates) => {
            write!(writer, "{{\"status\":\"ambiguous\",\"candidates\":[")?;
            for (index, id) in candidates.iter().copied().enumerate() {
                write_separator(writer, index)?;
                write!(writer, "{{\"id\":")?;
                write_string_id(writer, snapshot, id)?;
                write!(writer, ",\"facts\":")?;
                write_fact_refs_for_string_id(writer, snapshot, id)?;
                write!(writer, "}}")?;
            }
            write!(writer, "]}}")
        }
    }
}

fn write_fact_refs_for_string_id(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    id: StringId,
) -> io::Result<()> {
    let mut refs: Vec<_> = snapshot
        .fact_ids
        .iter()
        .filter(|candidate| candidate.key == id)
        .map(|candidate| candidate.value)
        .collect();
    refs.sort_unstable_by(|left, right| {
        fact_family(*left)
            .cmp(fact_family(*right))
            .then_with(|| fact_id(snapshot, *left).cmp(fact_id(snapshot, *right)))
    });
    write_fact_keys(writer, snapshot, refs)
}

fn write_optional_template_key(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    key: Option<HbkPlatformTypeTemplateKey>,
) -> io::Result<()> {
    if let Some(key) = key {
        write_template_key(writer, snapshot, key)
    } else {
        write!(writer, "null")
    }
}

fn write_template_key(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    key: HbkPlatformTypeTemplateKey,
) -> io::Result<()> {
    write!(writer, "{{\"family\":")?;
    write_string_id(writer, snapshot, key.family)?;
    write!(writer, ",\"variant\":")?;
    write_string_id(writer, snapshot, key.variant)?;
    write!(writer, "}}")
}

fn write_template_binding(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    binding: Option<&HbkTypeTemplateBinding>,
) -> io::Result<()> {
    let Some(binding) = binding else {
        return write!(writer, "null");
    };
    write!(writer, "{{\"template_key\":")?;
    write_template_key(writer, snapshot, binding.template_key)?;
    write!(writer, ",\"arguments\":[")?;
    for (index, argument) in binding.arguments.iter().enumerate() {
        write_separator(writer, index)?;
        match argument {
            model::TemplateParameterBinding::OwnerParameter {
                owner_parameter_index,
                target_parameter_index,
            } => write!(
                writer,
                "{{\"kind\":\"owner_parameter\",\"owner_parameter_index\":{owner_parameter_index},\"target_parameter_index\":{target_parameter_index}}}"
            )?,
        }
    }
    write!(writer, "]}}")
}

fn write_string_ids<I>(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    ids: I,
) -> io::Result<()>
where
    I: IntoIterator,
    I::Item: std::borrow::Borrow<StringId>,
{
    write!(writer, "[")?;
    for (index, id) in ids.into_iter().enumerate() {
        write_separator(writer, index)?;
        write_string_id(writer, snapshot, *id.borrow())?;
    }
    write!(writer, "]")
}

fn write_optional_string_id(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    id: Option<StringId>,
) -> io::Result<()> {
    if let Some(id) = id {
        write_string_id(writer, snapshot, id)
    } else {
        write!(writer, "null")
    }
}

fn write_string_id(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    id: StringId,
) -> io::Result<()> {
    write_json_string(writer, snapshot.string(id))
}

fn write_option_string(writer: &mut impl Write, value: Option<&str>) -> io::Result<()> {
    if let Some(value) = value {
        write_json_string(writer, value)
    } else {
        write!(writer, "null")
    }
}

fn write_fact_keys(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    refs: impl IntoIterator<Item = HbkFactRef>,
) -> io::Result<()> {
    write!(writer, "[")?;
    for (index, fact_ref) in refs.into_iter().enumerate() {
        write_separator(writer, index)?;
        write_fact_key(writer, snapshot, fact_ref)?;
    }
    write!(writer, "]")
}

fn write_fact_key<W: Write + ?Sized>(
    writer: &mut W,
    snapshot: &HbkFactSnapshot,
    fact_ref: HbkFactRef,
) -> io::Result<()> {
    write!(writer, "[")?;
    write_json_string(writer, fact_family(fact_ref))?;
    write!(writer, ",")?;
    write_json_string(writer, fact_id(snapshot, fact_ref))?;
    write!(writer, "]")
}

fn fact_family(fact_ref: HbkFactRef) -> &'static str {
    match fact_ref {
        HbkFactRef::PlatformType(_) => "platform_type",
        HbkFactRef::TypeMember(_) => "type_member",
        HbkFactRef::Callable(_) => "callable",
        HbkFactRef::Global(_) => "global",
        HbkFactRef::QueryTable(_) => "query_table",
        HbkFactRef::QueryField(_) => "query_field",
        HbkFactRef::QueryParameter(_) => "query_parameter",
        HbkFactRef::LanguageFact(_) => "language_fact",
        HbkFactRef::Enum(_) => "enum",
        HbkFactRef::EnumValue(_) => "enum_value",
    }
}

fn fact_id(snapshot: &HbkFactSnapshot, fact_ref: HbkFactRef) -> &str {
    let id = match fact_ref {
        HbkFactRef::PlatformType(id) => snapshot.platform_type(id).id,
        HbkFactRef::TypeMember(id) => snapshot.type_member(id).id,
        HbkFactRef::Callable(id) => snapshot.callable(id).id,
        HbkFactRef::Global(id) => snapshot.global_fact(id).id,
        HbkFactRef::QueryTable(id) => snapshot.query_table(id).id,
        HbkFactRef::QueryField(id) => snapshot.query_field(id).id,
        HbkFactRef::QueryParameter(id) => snapshot.query_parameter(id).id,
        HbkFactRef::LanguageFact(id) => snapshot.language_fact(id).id,
        HbkFactRef::Enum(id) => snapshot.enum_fact(id).id,
        HbkFactRef::EnumValue(id) => snapshot.enum_value(id).id,
    };
    snapshot.string(id)
}

fn try_for_each_fact(
    snapshot: &HbkFactSnapshot,
    mut visit: impl FnMut(HbkFactRef) -> io::Result<()>,
) -> io::Result<()> {
    for index in 0..snapshot.platform_types.len() {
        visit(HbkFactRef::PlatformType(HbkPlatformTypeId(index as u32)))?;
    }
    for index in 0..snapshot.type_members.len() {
        visit(HbkFactRef::TypeMember(HbkTypeMemberId(index as u32)))?;
    }
    for index in 0..snapshot.callables.len() {
        visit(HbkFactRef::Callable(HbkCallableId(index as u32)))?;
    }
    for index in 0..snapshot.globals.len() {
        visit(HbkFactRef::Global(HbkGlobalFactId(index as u32)))?;
    }
    for index in 0..snapshot.query_tables.len() {
        visit(HbkFactRef::QueryTable(HbkQueryTableId(index as u32)))?;
    }
    for index in 0..snapshot.query_fields.len() {
        visit(HbkFactRef::QueryField(HbkQueryFieldId(index as u32)))?;
    }
    for index in 0..snapshot.query_parameters.len() {
        visit(HbkFactRef::QueryParameter(HbkQueryParameterId(
            index as u32,
        )))?;
    }
    for index in 0..snapshot.language_facts.len() {
        visit(HbkFactRef::LanguageFact(HbkLanguageFactId(index as u32)))?;
    }
    for index in 0..snapshot.enums.len() {
        visit(HbkFactRef::Enum(HbkEnumId(index as u32)))?;
    }
    for index in 0..snapshot.enum_values.len() {
        visit(HbkFactRef::EnumValue(HbkEnumValueId(index as u32)))?;
    }
    Ok(())
}

fn member_kind(kind: HbkTypeMemberKind) -> &'static str {
    match kind {
        HbkTypeMemberKind::Property => "property",
        HbkTypeMemberKind::Method => "method",
        HbkTypeMemberKind::Event => "event",
        HbkTypeMemberKind::EnumValue => "enum_value",
    }
}

fn callable_kind(kind: HbkCallableKind) -> &'static str {
    match kind {
        HbkCallableKind::Method => "method",
        HbkCallableKind::Constructor => "constructor",
        HbkCallableKind::GlobalMethod => "global_method",
        HbkCallableKind::Event => "event",
        HbkCallableKind::LanguageFunction => "language_function",
    }
}

fn global_kind(kind: HbkGlobalFactKind) -> &'static str {
    match kind {
        HbkGlobalFactKind::Method => "method",
        HbkGlobalFactKind::Property => "property",
    }
}

fn language_domain(domain: HbkLanguageDomain) -> &'static str {
    match domain {
        HbkLanguageDomain::Bsl => "bsl",
        HbkLanguageDomain::Query => "query",
        HbkLanguageDomain::DataComposition => "data_composition",
        HbkLanguageDomain::Unknown => "unknown",
    }
}

fn query_table_role(role: model::QueryTableRole) -> &'static str {
    match role {
        model::QueryTableRole::Primary => "primary",
        model::QueryTableRole::Additional => "additional",
        model::QueryTableRole::Unknown => "unknown",
    }
}

fn write_separator(writer: &mut impl Write, index: usize) -> io::Result<()> {
    if index > 0 {
        write!(writer, ",")?;
    }
    Ok(())
}

fn write_json_string<W: Write + ?Sized>(writer: &mut W, value: &str) -> io::Result<()> {
    writer.write_all(b"\"")?;
    let mut unescaped_start = 0;
    for (offset, character) in value.char_indices() {
        let escape = match character {
            '"' => Some("\\\""),
            '\\' => Some("\\\\"),
            '\u{0008}' => Some("\\b"),
            '\u{000c}' => Some("\\f"),
            '\n' => Some("\\n"),
            '\r' => Some("\\r"),
            '\t' => Some("\\t"),
            character if character <= '\u{001f}' => {
                writer.write_all(&value.as_bytes()[unescaped_start..offset])?;
                write!(writer, "\\u{:04x}", character as u32)?;
                unescaped_start = offset + character.len_utf8();
                continue;
            }
            _ => None,
        };
        if let Some(escape) = escape {
            writer.write_all(&value.as_bytes()[unescaped_start..offset])?;
            writer.write_all(escape.as_bytes())?;
            unescaped_start = offset + character.len_utf8();
        }
    }
    writer.write_all(&value.as_bytes()[unescaped_start..])?;
    writer.write_all(b"\"")
}

fn write_lookup_fact_results(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    operation: &str,
    write_args: impl FnOnce(&mut dyn Write) -> io::Result<()>,
    results: impl IntoIterator<Item = HbkFactRef>,
) -> io::Result<()> {
    write!(writer, "{{\"record\":\"lookup\",\"op\":")?;
    write_json_string(writer, operation)?;
    write!(writer, ",\"args\":{{")?;
    write_args(writer)?;
    write!(writer, "}},\"result\":")?;
    write_fact_keys(writer, snapshot, results)?;
    writeln!(writer, "}}")
}

fn write_id_and_name_lookups(
    snapshot: &HbkFactSnapshot,
    writer: &mut impl Write,
) -> io::Result<()> {
    let handle = snapshot.worker_handle();
    let mut previous = None;
    for lookup in &snapshot.fact_ids {
        if previous == Some(lookup.key) {
            continue;
        }
        previous = Some(lookup.key);
        let id = snapshot.string(lookup.key);
        write_lookup_fact_results(
            writer,
            snapshot,
            "facts_by_id",
            |writer| write_named_string_arg(writer, "id", id),
            handle.facts_by_id(id),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "facts_by_id",
        |writer| write_named_string_arg(writer, "id", MISSING_LOOKUP_KEY),
        handle.facts_by_id(MISSING_LOOKUP_KEY),
    )?;
    write_lookup_fact_results(
        writer,
        snapshot,
        "global_fact_ids",
        |_| Ok(()),
        handle.global_fact_ids().map(HbkFactRef::Global),
    )?;
    write_lookup_fact_results(
        writer,
        snapshot,
        "query_table_ids",
        |_| Ok(()),
        handle.query_table_ids().map(HbkFactRef::QueryTable),
    )?;
    write_lookup_fact_results(
        writer,
        snapshot,
        "query_field_ids",
        |_| Ok(()),
        handle.query_field_ids().map(HbkFactRef::QueryField),
    )?;
    write_lookup_fact_results(
        writer,
        snapshot,
        "query_parameter_ids",
        |_| Ok(()),
        handle.query_parameter_ids().map(HbkFactRef::QueryParameter),
    )
}

fn write_platform_lookups(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    let handle = snapshot.worker_handle();
    for (index, fact) in snapshot.platform_types.iter().enumerate() {
        let id = snapshot.string(fact.id);
        write_lookup_fact_results(
            writer,
            snapshot,
            "platform_type_by_id",
            |writer| write_named_string_arg(writer, "id", id),
            handle
                .platform_type_by_id(id)
                .into_iter()
                .map(HbkFactRef::PlatformType),
        )?;
        let owner = HbkPlatformTypeId(index as u32);
        write_lookup_fact_results(
            writer,
            snapshot,
            "members_of_type",
            |writer| {
                write_named_fact_arg(writer, snapshot, "owner", HbkFactRef::PlatformType(owner))
            },
            handle.members_of_type(owner).map(HbkFactRef::TypeMember),
        )?;
        write_lookup_fact_results(
            writer,
            snapshot,
            "callables_of_type",
            |writer| {
                write_named_fact_arg(writer, snapshot, "owner", HbkFactRef::PlatformType(owner))
            },
            handle.callables_of_type(owner).map(HbkFactRef::Callable),
        )?;
        write_lookup_fact_results(
            writer,
            snapshot,
            "constructors_of_type",
            |writer| {
                write_named_fact_arg(writer, snapshot, "owner", HbkFactRef::PlatformType(owner))
            },
            handle.constructors_of_type(owner).map(HbkFactRef::Callable),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "platform_type_by_id",
        |writer| write_named_string_arg(writer, "id", MISSING_LOOKUP_KEY),
        handle
            .platform_type_by_id(MISSING_LOOKUP_KEY)
            .into_iter()
            .map(HbkFactRef::PlatformType),
    )?;

    let mut previous = None;
    for lookup in &snapshot.platform_type_names {
        if previous == Some(lookup.key) {
            continue;
        }
        previous = Some(lookup.key);
        let name = snapshot.string(lookup.key);
        write_lookup_fact_results(
            writer,
            snapshot,
            "platform_types_by_name",
            |writer| write_named_string_arg(writer, "name", name),
            handle
                .platform_types_by_name(name)
                .map(HbkFactRef::PlatformType),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "platform_types_by_name",
        |writer| write_named_string_arg(writer, "name", MISSING_LOOKUP_KEY),
        handle
            .platform_types_by_name(MISSING_LOOKUP_KEY)
            .map(HbkFactRef::PlatformType),
    )?;

    let mut previous = None;
    for lookup in &snapshot.platform_type_templates {
        let key = (lookup.family, lookup.variant);
        if previous == Some(key) {
            continue;
        }
        previous = Some(key);
        let family = snapshot.string(lookup.family);
        let variant = snapshot.string(lookup.variant);
        write_lookup_fact_results(
            writer,
            snapshot,
            "platform_types_by_template_key",
            |writer| {
                write_named_string_arg(writer, "family", family)?;
                write!(writer, ",")?;
                write_named_string_arg(writer, "variant", variant)
            },
            handle
                .platform_types_by_template_key(family, variant)
                .map(HbkFactRef::PlatformType),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "platform_types_by_template_key",
        |writer| {
            write_named_string_arg(writer, "family", MISSING_LOOKUP_KEY)?;
            write!(writer, ",")?;
            write_named_string_arg(writer, "variant", MISSING_LOOKUP_KEY)
        },
        handle
            .platform_types_by_template_key(MISSING_LOOKUP_KEY, MISSING_LOOKUP_KEY)
            .map(HbkFactRef::PlatformType),
    )
}

fn write_member_and_callable_lookups(
    snapshot: &HbkFactSnapshot,
    writer: &mut impl Write,
) -> io::Result<()> {
    let handle = snapshot.worker_handle();
    let mut previous = None;
    for lookup in &snapshot.members_by_owner_name {
        let key = (lookup.owner, lookup.key);
        if previous == Some(key) {
            continue;
        }
        previous = Some(key);
        let name = snapshot.string(lookup.key);
        write_owner_name_lookup(
            writer,
            snapshot,
            "member_by_owner_name",
            HbkFactRef::PlatformType(lookup.owner),
            name,
            handle
                .member_by_owner_name(lookup.owner, name)
                .map(HbkFactRef::TypeMember),
        )?;
    }
    let mut previous = None;
    for lookup in &snapshot.members_by_owner_name_kind {
        let key = (lookup.owner, lookup.key, lookup.kind);
        if previous == Some(key) {
            continue;
        }
        previous = Some(key);
        let name = snapshot.string(lookup.key);
        write_lookup_fact_results(
            writer,
            snapshot,
            "member_by_owner_name_kind",
            |writer| {
                write_named_fact_arg(
                    writer,
                    snapshot,
                    "owner",
                    HbkFactRef::PlatformType(lookup.owner),
                )?;
                write!(writer, ",")?;
                write_named_string_arg(writer, "name", name)?;
                write!(writer, ",\"kind\":")?;
                if let Some(kind) = lookup.kind {
                    write_json_string(writer, member_kind(kind))
                } else {
                    write!(writer, "null")
                }
            },
            handle
                .member_by_owner_name_kind(lookup.owner, name, lookup.kind)
                .map(HbkFactRef::TypeMember),
        )?;
    }
    let mut previous = None;
    for lookup in &snapshot.callables_by_owner_name {
        let key = (lookup.owner, lookup.key);
        if previous == Some(key) {
            continue;
        }
        previous = Some(key);
        let name = snapshot.string(lookup.key);
        write_owner_name_lookup(
            writer,
            snapshot,
            "callable_by_owner_name",
            HbkFactRef::PlatformType(lookup.owner),
            name,
            handle
                .callable_by_owner_name(lookup.owner, name)
                .map(HbkFactRef::Callable),
        )?;
    }
    if let Some(owner) = snapshot
        .platform_types
        .first()
        .map(|_| HbkPlatformTypeId(0))
    {
        write_owner_name_lookup(
            writer,
            snapshot,
            "member_by_owner_name",
            HbkFactRef::PlatformType(owner),
            MISSING_LOOKUP_KEY,
            handle
                .member_by_owner_name(owner, MISSING_LOOKUP_KEY)
                .map(HbkFactRef::TypeMember),
        )?;
        write_lookup_fact_results(
            writer,
            snapshot,
            "member_by_owner_name_kind",
            |writer| {
                write_named_fact_arg(writer, snapshot, "owner", HbkFactRef::PlatformType(owner))?;
                write!(writer, ",")?;
                write_named_string_arg(writer, "name", MISSING_LOOKUP_KEY)?;
                write!(writer, ",\"kind\":null")
            },
            handle
                .member_by_owner_name_kind(owner, MISSING_LOOKUP_KEY, None)
                .map(HbkFactRef::TypeMember),
        )?;
        write_owner_name_lookup(
            writer,
            snapshot,
            "callable_by_owner_name",
            HbkFactRef::PlatformType(owner),
            MISSING_LOOKUP_KEY,
            handle
                .callable_by_owner_name(owner, MISSING_LOOKUP_KEY)
                .map(HbkFactRef::Callable),
        )?;
    }
    Ok(())
}

fn write_global_and_module_lookups(
    snapshot: &HbkFactSnapshot,
    writer: &mut impl Write,
) -> io::Result<()> {
    let handle = snapshot.worker_handle();
    let mut previous = None;
    for lookup in &snapshot.global_names {
        if previous == Some(lookup.key) {
            continue;
        }
        previous = Some(lookup.key);
        let name = snapshot.string(lookup.key);
        write_lookup_fact_results(
            writer,
            snapshot,
            "globals_by_name",
            |writer| write_named_string_arg(writer, "name", name),
            handle.globals_by_name(name).map(HbkFactRef::Global),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "globals_by_name",
        |writer| write_named_string_arg(writer, "name", MISSING_LOOKUP_KEY),
        handle
            .globals_by_name(MISSING_LOOKUP_KEY)
            .map(HbkFactRef::Global),
    )?;
    let mut previous = None;
    for lookup in &snapshot.globals_by_domain_name_kind {
        let key = (lookup.domain, lookup.key, lookup.kind);
        if previous == Some(key) {
            continue;
        }
        previous = Some(key);
        let name = snapshot.string(lookup.key);
        write_domain_name_kind_lookup(
            writer,
            snapshot,
            lookup.domain,
            name,
            lookup.kind,
            handle
                .globals_by_domain_name_kind(lookup.domain, name, lookup.kind)
                .map(HbkFactRef::Global),
        )?;
    }
    write_domain_name_kind_lookup(
        writer,
        snapshot,
        HbkLanguageDomain::Bsl,
        MISSING_LOOKUP_KEY,
        None,
        handle
            .globals_by_domain_name_kind(HbkLanguageDomain::Bsl, MISSING_LOOKUP_KEY, None)
            .map(HbkFactRef::Global),
    )?;

    let mut previous_owner = None;
    let mut previous_pair = None;
    for lookup in &snapshot.module_event_names {
        if previous_owner != Some(lookup.owner) {
            previous_owner = Some(lookup.owner);
            let context = snapshot.string(lookup.owner);
            write_lookup_fact_results(
                writer,
                snapshot,
                "module_events",
                |writer| write_named_string_arg(writer, "module_context_key", context),
                handle.module_events(context).map(HbkFactRef::Callable),
            )?;
        }
        let pair = (lookup.owner, lookup.key);
        if previous_pair != Some(pair) {
            previous_pair = Some(pair);
            let context = snapshot.string(lookup.owner);
            let name = snapshot.string(lookup.key);
            write_lookup_fact_results(
                writer,
                snapshot,
                "module_event_by_context_name",
                |writer| {
                    write_named_string_arg(writer, "module_context_key", context)?;
                    write!(writer, ",")?;
                    write_named_string_arg(writer, "name", name)
                },
                handle
                    .module_event_by_context_name(context, name)
                    .map(HbkFactRef::Callable),
            )?;
        }
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "module_events",
        |writer| write_named_string_arg(writer, "module_context_key", MISSING_LOOKUP_KEY),
        handle
            .module_events(MISSING_LOOKUP_KEY)
            .map(HbkFactRef::Callable),
    )?;
    write_lookup_fact_results(
        writer,
        snapshot,
        "module_event_by_context_name",
        |writer| {
            write_named_string_arg(writer, "module_context_key", MISSING_LOOKUP_KEY)?;
            write!(writer, ",")?;
            write_named_string_arg(writer, "name", MISSING_LOOKUP_KEY)
        },
        handle
            .module_event_by_context_name(MISSING_LOOKUP_KEY, MISSING_LOOKUP_KEY)
            .map(HbkFactRef::Callable),
    )?;

    let mut previous = None;
    for lookup in &snapshot.module_contexts_by_domain_language_kind {
        let key = (lookup.domain, lookup.language_key, lookup.module_kind);
        if previous == Some(key) {
            continue;
        }
        previous = Some(key);
        let language_key = snapshot.string(lookup.language_key);
        let module_kind_value = snapshot.string(lookup.module_kind);
        write_lookup_fact_results(
            writer,
            snapshot,
            "module_context_events",
            |writer| {
                write!(writer, "\"domain\":")?;
                write_json_string(writer, language_domain(lookup.domain))?;
                write!(writer, ",")?;
                write_named_string_arg(writer, "language_key", language_key)?;
                write!(writer, ",")?;
                write_named_string_arg(writer, "module_kind", module_kind_value)
            },
            handle
                .module_context_events(lookup.domain, language_key, module_kind_value)
                .map(HbkFactRef::Callable),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "module_context_events",
        |writer| {
            write!(writer, "\"domain\":\"bsl\",")?;
            write_named_string_arg(writer, "language_key", MISSING_LOOKUP_KEY)?;
            write!(writer, ",")?;
            write_named_string_arg(writer, "module_kind", MISSING_LOOKUP_KEY)
        },
        handle
            .module_context_events(
                HbkLanguageDomain::Bsl,
                MISSING_LOOKUP_KEY,
                MISSING_LOOKUP_KEY,
            )
            .map(HbkFactRef::Callable),
    )
}

fn write_query_lookups(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    let handle = snapshot.worker_handle();
    for (index, fact) in snapshot.query_tables.iter().enumerate() {
        let id = snapshot.string(fact.id);
        write_lookup_fact_results(
            writer,
            snapshot,
            "query_table_by_id",
            |writer| write_named_string_arg(writer, "id", id),
            handle
                .query_table_by_id(id)
                .into_iter()
                .map(HbkFactRef::QueryTable),
        )?;
        let table = HbkQueryTableId(index as u32);
        write_lookup_fact_results(
            writer,
            snapshot,
            "query_fields",
            |writer| write_named_fact_arg(writer, snapshot, "table", HbkFactRef::QueryTable(table)),
            handle.query_fields(table).map(HbkFactRef::QueryField),
        )?;
        write_lookup_fact_results(
            writer,
            snapshot,
            "query_parameters",
            |writer| write_named_fact_arg(writer, snapshot, "table", HbkFactRef::QueryTable(table)),
            handle
                .query_parameters(table)
                .map(HbkFactRef::QueryParameter),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "query_table_by_id",
        |writer| write_named_string_arg(writer, "id", MISSING_LOOKUP_KEY),
        handle
            .query_table_by_id(MISSING_LOOKUP_KEY)
            .into_iter()
            .map(HbkFactRef::QueryTable),
    )?;
    write_name_index_transcript(
        writer,
        snapshot,
        "query_tables_by_name",
        "name",
        &snapshot.query_table_names,
        |name| {
            Box::new(
                snapshot
                    .worker_handle()
                    .query_tables_by_name(name)
                    .map(HbkFactRef::QueryTable),
            )
        },
    )?;
    write_name_index_transcript(
        writer,
        snapshot,
        "query_tables_by_syntax",
        "syntax",
        &snapshot.query_table_syntax_names,
        |syntax| {
            Box::new(
                snapshot
                    .worker_handle()
                    .query_tables_by_syntax(syntax)
                    .map(HbkFactRef::QueryTable),
            )
        },
    )?;
    write_name_index_transcript(
        writer,
        snapshot,
        "query_tables_by_identifier",
        "identifier",
        &snapshot.query_table_identifiers,
        |identifier| {
            Box::new(
                snapshot
                    .worker_handle()
                    .query_tables_by_identifier(identifier)
                    .map(HbkFactRef::QueryTable),
            )
        },
    )?;
    write_owner_index_transcript(
        writer,
        snapshot,
        "query_fields_by_name",
        &snapshot.query_fields_by_table_name,
        |owner, name| {
            Box::new(
                snapshot
                    .worker_handle()
                    .query_fields_by_name(owner, name)
                    .map(HbkFactRef::QueryField),
            )
        },
    )?;
    write_owner_index_transcript(
        writer,
        snapshot,
        "query_parameters_by_name",
        &snapshot.query_parameters_by_table_name,
        |owner, name| {
            Box::new(
                snapshot
                    .worker_handle()
                    .query_parameters_by_name(owner, name)
                    .map(HbkFactRef::QueryParameter),
            )
        },
    )?;
    Ok(())
}

fn write_language_and_enum_lookups(
    snapshot: &HbkFactSnapshot,
    writer: &mut impl Write,
) -> io::Result<()> {
    let handle = snapshot.worker_handle();
    for fact in &snapshot.language_facts {
        let id = snapshot.string(fact.id);
        write_lookup_fact_results(
            writer,
            snapshot,
            "language_fact_by_id",
            |writer| write_named_string_arg(writer, "id", id),
            handle
                .language_fact_by_id(id)
                .into_iter()
                .map(HbkFactRef::LanguageFact),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "language_fact_by_id",
        |writer| write_named_string_arg(writer, "id", MISSING_LOOKUP_KEY),
        handle
            .language_fact_by_id(MISSING_LOOKUP_KEY)
            .into_iter()
            .map(HbkFactRef::LanguageFact),
    )?;
    let mut previous = None;
    for lookup in &snapshot.language_names {
        if previous == Some(lookup.key) {
            continue;
        }
        previous = Some(lookup.key);
        let name = snapshot.string(lookup.key);
        write_lookup_fact_results(
            writer,
            snapshot,
            "language_facts_by_name",
            |writer| write_named_string_arg(writer, "name", name),
            handle
                .language_facts_by_name(name)
                .map(HbkFactRef::LanguageFact),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "language_facts_by_name",
        |writer| write_named_string_arg(writer, "name", MISSING_LOOKUP_KEY),
        handle
            .language_facts_by_name(MISSING_LOOKUP_KEY)
            .map(HbkFactRef::LanguageFact),
    )?;

    for (index, fact) in snapshot.enums.iter().enumerate() {
        let id = snapshot.string(fact.id);
        write_lookup_fact_results(
            writer,
            snapshot,
            "enum_by_id",
            |writer| write_named_string_arg(writer, "id", id),
            handle.enum_by_id(id).into_iter().map(HbkFactRef::Enum),
        )?;
        let owner = HbkEnumId(index as u32);
        write_lookup_fact_results(
            writer,
            snapshot,
            "enum_values",
            |writer| write_named_fact_arg(writer, snapshot, "owner", HbkFactRef::Enum(owner)),
            handle.enum_values(owner).map(HbkFactRef::EnumValue),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "enum_by_id",
        |writer| write_named_string_arg(writer, "id", MISSING_LOOKUP_KEY),
        handle
            .enum_by_id(MISSING_LOOKUP_KEY)
            .into_iter()
            .map(HbkFactRef::Enum),
    )?;
    let mut previous = None;
    for lookup in &snapshot.enum_names {
        if previous == Some(lookup.key) {
            continue;
        }
        previous = Some(lookup.key);
        let name = snapshot.string(lookup.key);
        write_lookup_fact_results(
            writer,
            snapshot,
            "enums_by_name",
            |writer| write_named_string_arg(writer, "name", name),
            handle.enums_by_name(name).map(HbkFactRef::Enum),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "enums_by_name",
        |writer| write_named_string_arg(writer, "name", MISSING_LOOKUP_KEY),
        handle
            .enums_by_name(MISSING_LOOKUP_KEY)
            .map(HbkFactRef::Enum),
    )?;

    for fact in &snapshot.enum_values {
        let id = snapshot.string(fact.id);
        write_lookup_fact_results(
            writer,
            snapshot,
            "enum_value_by_id",
            |writer| write_named_string_arg(writer, "id", id),
            handle
                .enum_value_by_id(id)
                .into_iter()
                .map(HbkFactRef::EnumValue),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        "enum_value_by_id",
        |writer| write_named_string_arg(writer, "id", MISSING_LOOKUP_KEY),
        handle
            .enum_value_by_id(MISSING_LOOKUP_KEY)
            .into_iter()
            .map(HbkFactRef::EnumValue),
    )?;
    write_enum_owner_index_transcript(writer, snapshot, &snapshot.enum_values_by_enum_name)
}

fn write_state_lookups(snapshot: &HbkFactSnapshot, writer: &mut impl Write) -> io::Result<()> {
    let handle = snapshot.worker_handle();
    try_for_each_fact(snapshot, |fact_ref| {
        write!(
            writer,
            "{{\"record\":\"lookup\",\"op\":\"availability_contexts\",\"args\":{{"
        )?;
        write_named_fact_arg(writer, snapshot, "fact", fact_ref)?;
        write!(writer, "}},\"result\":")?;
        write_string_ids(writer, snapshot, handle.availability_contexts(fact_ref))?;
        writeln!(writer, "}}")?;

        write!(
            writer,
            "{{\"record\":\"lookup\",\"op\":\"available_since\",\"args\":{{"
        )?;
        write_named_fact_arg(writer, snapshot, "fact", fact_ref)?;
        write!(writer, "}},\"result\":")?;
        write_optional_string_id(writer, snapshot, handle.available_since(fact_ref))?;
        writeln!(writer, "}}")?;

        let mut kinds: Vec<_> = snapshot
            .relations_by_source_kind
            .keys
            .iter()
            .filter(|key| key.source == fact_ref)
            .map(|key| snapshot.string(key.kind))
            .collect();
        kinds.sort_unstable();
        for kind in kinds {
            write_lookup_fact_results(
                writer,
                snapshot,
                "relations_by_source_kind",
                |writer| {
                    write_named_fact_arg(writer, snapshot, "source", fact_ref)?;
                    write!(writer, ",")?;
                    write_named_string_arg(writer, "kind", kind)
                },
                handle.relations_by_source_kind(fact_ref, kind),
            )?;
        }
        write_lookup_fact_results(
            writer,
            snapshot,
            "relations_by_source_kind",
            |writer| {
                write_named_fact_arg(writer, snapshot, "source", fact_ref)?;
                write!(writer, ",")?;
                write_named_string_arg(writer, "kind", MISSING_LOOKUP_KEY)
            },
            handle.relations_by_source_kind(fact_ref, MISSING_LOOKUP_KEY),
        )
    })
}

fn write_name_index_transcript<'snapshot, T: Copy>(
    writer: &mut impl Write,
    snapshot: &'snapshot HbkFactSnapshot,
    operation: &str,
    argument_name: &str,
    index: &[NameLookup<T>],
    mut lookup: impl FnMut(&'snapshot str) -> Box<dyn Iterator<Item = HbkFactRef> + 'snapshot>,
) -> io::Result<()> {
    let mut previous = None;
    for entry in index {
        if previous == Some(entry.key) {
            continue;
        }
        previous = Some(entry.key);
        let value = snapshot.string(entry.key);
        write_lookup_fact_results(
            writer,
            snapshot,
            operation,
            |writer| write_named_string_arg(writer, argument_name, value),
            lookup(value),
        )?;
    }
    write_lookup_fact_results(
        writer,
        snapshot,
        operation,
        |writer| write_named_string_arg(writer, argument_name, MISSING_LOOKUP_KEY),
        lookup(MISSING_LOOKUP_KEY),
    )
}

fn write_owner_index_transcript<'snapshot, Value: Copy>(
    writer: &mut impl Write,
    snapshot: &'snapshot HbkFactSnapshot,
    operation: &str,
    index: &[OwnerNameLookup<HbkQueryTableId, Value>],
    mut lookup: impl FnMut(
        HbkQueryTableId,
        &'snapshot str,
    ) -> Box<dyn Iterator<Item = HbkFactRef> + 'snapshot>,
) -> io::Result<()> {
    let mut previous = None;
    for entry in index {
        let key = (entry.owner, entry.key);
        if previous == Some(key) {
            continue;
        }
        previous = Some(key);
        let name = snapshot.string(entry.key);
        write_owner_name_lookup(
            writer,
            snapshot,
            operation,
            HbkFactRef::QueryTable(entry.owner),
            name,
            lookup(entry.owner, name),
        )?;
    }
    if let Some(owner) = snapshot.query_tables.first().map(|_| HbkQueryTableId(0)) {
        write_owner_name_lookup(
            writer,
            snapshot,
            operation,
            HbkFactRef::QueryTable(owner),
            MISSING_LOOKUP_KEY,
            lookup(owner, MISSING_LOOKUP_KEY),
        )?;
    }
    Ok(())
}

fn write_enum_owner_index_transcript(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    index: &[OwnerNameLookup<HbkEnumId, HbkEnumValueId>],
) -> io::Result<()> {
    let mut previous = None;
    for entry in index {
        let key = (entry.owner, entry.key);
        if previous == Some(key) {
            continue;
        }
        previous = Some(key);
        let name = snapshot.string(entry.key);
        write_owner_name_lookup(
            writer,
            snapshot,
            "enum_values_by_name",
            HbkFactRef::Enum(entry.owner),
            name,
            snapshot
                .worker_handle()
                .enum_values_by_name(entry.owner, name)
                .map(HbkFactRef::EnumValue),
        )?;
    }
    if let Some(owner) = snapshot.enums.first().map(|_| HbkEnumId(0)) {
        write_owner_name_lookup(
            writer,
            snapshot,
            "enum_values_by_name",
            HbkFactRef::Enum(owner),
            MISSING_LOOKUP_KEY,
            snapshot
                .worker_handle()
                .enum_values_by_name(owner, MISSING_LOOKUP_KEY)
                .map(HbkFactRef::EnumValue),
        )?;
    }
    Ok(())
}

fn write_owner_name_lookup(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    operation: &str,
    owner: HbkFactRef,
    name: &str,
    results: impl IntoIterator<Item = HbkFactRef>,
) -> io::Result<()> {
    write_lookup_fact_results(
        writer,
        snapshot,
        operation,
        |writer| {
            write_named_fact_arg(writer, snapshot, "owner", owner)?;
            write!(writer, ",")?;
            write_named_string_arg(writer, "name", name)
        },
        results,
    )
}

fn write_domain_name_kind_lookup(
    writer: &mut impl Write,
    snapshot: &HbkFactSnapshot,
    domain: HbkLanguageDomain,
    name: &str,
    kind: Option<HbkGlobalFactKind>,
    results: impl IntoIterator<Item = HbkFactRef>,
) -> io::Result<()> {
    write_lookup_fact_results(
        writer,
        snapshot,
        "globals_by_domain_name_kind",
        |writer| {
            write!(writer, "\"domain\":")?;
            write_json_string(writer, language_domain(domain))?;
            write!(writer, ",")?;
            write_named_string_arg(writer, "name", name)?;
            write!(writer, ",\"kind\":")?;
            if let Some(kind) = kind {
                write_json_string(writer, global_kind(kind))
            } else {
                write!(writer, "null")
            }
        },
        results,
    )
}

fn write_named_string_arg(writer: &mut dyn Write, name: &str, value: &str) -> io::Result<()> {
    write_json_string(writer, name)?;
    write!(writer, ":")?;
    write_json_string(writer, value)
}

fn write_named_fact_arg(
    writer: &mut dyn Write,
    snapshot: &HbkFactSnapshot,
    name: &str,
    value: HbkFactRef,
) -> io::Result<()> {
    write_json_string(writer, name)?;
    write!(writer, ":")?;
    write_fact_key(writer, snapshot, value)
}
