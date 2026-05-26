use std::io::{BufReader, BufWriter, Read, Write};

use super::indexes::*;
use super::*;

const MAGIC: &[u8; 8] = b"HBKFSN1\0";
const CACHE_VERSION: u32 = 2;

impl HbkFactSnapshot {
    /// Writes a provider-owned binary snapshot cache for measurement.
    ///
    /// The cache is a derived artifact over the SQLite provider index. It is
    /// intentionally not a downstream storage contract.
    pub fn write_experimental_binary_cache(&self, path: impl AsRef<Path>) -> io::Result<()> {
        let mut writer = BinaryWriter::new(BufWriter::new(File::create(path)?));
        writer.write_bytes(MAGIC)?;
        writer.write_u32(CACHE_VERSION)?;
        writer.write_u32(INDEX_SCHEMA_VERSION)?;
        self.write_to(&mut writer)?;
        writer.finish()
    }

    /// Reads a provider-owned binary snapshot cache produced by
    /// `write_experimental_binary_cache`.
    ///
    /// This is a startup-latency measurement path, not a replacement source of
    /// truth for the SQLite provider index.
    pub fn from_experimental_binary_cache(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut reader = BinaryReader::new(BufReader::new(File::open(path)?));
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(invalid_data("invalid HBK fact snapshot cache magic"));
        }
        let version = reader.read_u32()?;
        if version != CACHE_VERSION {
            return Err(invalid_data("unsupported HBK fact snapshot cache version"));
        }
        let schema = reader.read_u32()?;
        if schema != INDEX_SCHEMA_VERSION {
            return Err(invalid_data(
                "unsupported provider index schema version in snapshot cache",
            ));
        }
        Self::read_from(&mut reader)
    }
}

trait BinaryValue: Sized {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()>;
    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self>;
}

struct BinaryWriter<W> {
    inner: W,
}

impl<W: Write> BinaryWriter<W> {
    fn new(inner: W) -> Self {
        Self { inner }
    }

    fn finish(mut self) -> io::Result<()> {
        self.inner.flush()
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.inner.write_all(bytes)
    }

    fn write_u8(&mut self, value: u8) -> io::Result<()> {
        self.inner.write_all(&[value])
    }

    fn write_u32(&mut self, value: u32) -> io::Result<()> {
        self.inner.write_all(&value.to_le_bytes())
    }

    fn write_u64(&mut self, value: u64) -> io::Result<()> {
        self.inner.write_all(&value.to_le_bytes())
    }

    fn write_usize(&mut self, value: usize) -> io::Result<()> {
        self.write_u64(value as u64)
    }

    fn write_bool(&mut self, value: bool) -> io::Result<()> {
        self.write_u8(u8::from(value))
    }

    fn write_string(&mut self, value: &str) -> io::Result<()> {
        self.write_usize(value.len())?;
        self.write_bytes(value.as_bytes())
    }

    fn write_vec<T: BinaryValue>(&mut self, values: &[T]) -> io::Result<()> {
        self.write_usize(values.len())?;
        for value in values {
            value.write_to(self)?;
        }
        Ok(())
    }
}

struct BinaryReader<R> {
    inner: R,
}

impl<R: Read> BinaryReader<R> {
    fn new(inner: R) -> Self {
        Self { inner }
    }

    fn read_exact(&mut self, bytes: &mut [u8]) -> io::Result<()> {
        self.inner.read_exact(bytes)
    }

    fn read_u8(&mut self) -> io::Result<u8> {
        let mut bytes = [0u8; 1];
        self.inner.read_exact(&mut bytes)?;
        Ok(bytes[0])
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        let mut bytes = [0u8; 4];
        self.inner.read_exact(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_u64(&mut self) -> io::Result<u64> {
        let mut bytes = [0u8; 8];
        self.inner.read_exact(&mut bytes)?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_usize(&mut self) -> io::Result<usize> {
        usize::try_from(self.read_u64()?).map_err(|_| invalid_data("length does not fit usize"))
    }

    fn read_bool(&mut self) -> io::Result<bool> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid_data("invalid boolean tag")),
        }
    }

    fn read_string(&mut self) -> io::Result<String> {
        let len = self.read_usize()?;
        let mut bytes = vec![0u8; len];
        self.inner.read_exact(&mut bytes)?;
        String::from_utf8(bytes).map_err(|_| invalid_data("invalid UTF-8 string"))
    }

    fn read_vec<T: BinaryValue>(&mut self) -> io::Result<Vec<T>> {
        let len = self.read_usize()?;
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            values.push(T::read_from(self)?);
        }
        Ok(values)
    }
}

impl<T: BinaryValue> BinaryValue for Option<T> {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        match self {
            Some(value) => {
                writer.write_bool(true)?;
                value.write_to(writer)
            }
            None => writer.write_bool(false),
        }
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        if reader.read_bool()? {
            Ok(Some(T::read_from(reader)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: BinaryValue> BinaryValue for Vec<T> {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_vec(self)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        reader.read_vec()
    }
}

impl BinaryValue for String {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_string(self)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        reader.read_string()
    }
}

impl BinaryValue for u32 {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_u32(*self)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        reader.read_u32()
    }
}

impl BinaryValue for usize {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_usize(*self)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        reader.read_usize()
    }
}

macro_rules! binary_newtype_u32 {
    ($type:ty) => {
        impl BinaryValue for $type {
            fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
                writer.write_u32(self.0)
            }

            fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
                Ok(Self(reader.read_u32()?))
            }
        }
    };
}

binary_newtype_u32!(StringId);
binary_newtype_u32!(HbkPlatformTypeId);
binary_newtype_u32!(HbkTypeMemberId);
binary_newtype_u32!(HbkCallableId);
binary_newtype_u32!(HbkGlobalFactId);
binary_newtype_u32!(HbkQueryTableId);
binary_newtype_u32!(HbkQueryFieldId);
binary_newtype_u32!(HbkQueryParameterId);
binary_newtype_u32!(HbkLanguageFactId);
binary_newtype_u32!(HbkEnumId);
binary_newtype_u32!(HbkEnumValueId);

impl BinaryValue for HbkFactSnapshot {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_vec(&self.strings)?;
        self.source_locale.write_to(writer)?;
        writer.write_vec(&self.platform_types)?;
        writer.write_vec(&self.type_members)?;
        writer.write_vec(&self.callables)?;
        writer.write_vec(&self.globals)?;
        writer.write_vec(&self.query_tables)?;
        writer.write_vec(&self.query_fields)?;
        writer.write_vec(&self.query_parameters)?;
        writer.write_vec(&self.language_facts)?;
        writer.write_vec(&self.enums)?;
        writer.write_vec(&self.enum_values)?;
        writer.write_vec(&self.fact_ids)?;
        writer.write_vec(&self.platform_type_ids)?;
        writer.write_vec(&self.platform_type_names)?;
        writer.write_vec(&self.platform_type_templates)?;
        writer.write_vec(&self.member_ids)?;
        self.members_by_owner.write_to(writer)?;
        writer.write_vec(&self.members_by_owner_name)?;
        writer.write_vec(&self.members_by_owner_name_kind)?;
        writer.write_vec(&self.callable_ids)?;
        self.callables_by_owner.write_to(writer)?;
        writer.write_vec(&self.callables_by_owner_name)?;
        self.constructors_by_type.write_to(writer)?;
        writer.write_vec(&self.global_names)?;
        writer.write_vec(&self.globals_by_domain_name_kind)?;
        writer.write_vec(&self.module_event_names)?;
        writer.write_vec(&self.module_contexts_by_domain_language_kind)?;
        writer.write_vec(&self.query_table_ids)?;
        writer.write_vec(&self.query_table_names)?;
        writer.write_vec(&self.query_table_syntax_names)?;
        writer.write_vec(&self.query_table_identifiers)?;
        self.query_fields_by_table.write_to(writer)?;
        writer.write_vec(&self.query_fields_by_table_name)?;
        self.query_parameters_by_table.write_to(writer)?;
        writer.write_vec(&self.query_parameters_by_table_name)?;
        writer.write_vec(&self.language_ids)?;
        writer.write_vec(&self.language_names)?;
        writer.write_vec(&self.enum_ids)?;
        writer.write_vec(&self.enum_names)?;
        writer.write_vec(&self.enum_value_ids)?;
        self.enum_values_by_enum.write_to(writer)?;
        writer.write_vec(&self.enum_values_by_enum_name)?;
        self.availability_by_fact.write_to(writer)?;
        writer.write_vec(&self.availability_since_by_fact)?;
        self.relations_by_source_kind.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            strings: reader.read_vec()?,
            source_locale: Option::<StringId>::read_from(reader)?,
            platform_types: reader.read_vec()?,
            type_members: reader.read_vec()?,
            callables: reader.read_vec()?,
            globals: reader.read_vec()?,
            query_tables: reader.read_vec()?,
            query_fields: reader.read_vec()?,
            query_parameters: reader.read_vec()?,
            language_facts: reader.read_vec()?,
            enums: reader.read_vec()?,
            enum_values: reader.read_vec()?,
            fact_ids: reader.read_vec()?,
            platform_type_ids: reader.read_vec()?,
            platform_type_names: reader.read_vec()?,
            platform_type_templates: reader.read_vec()?,
            member_ids: reader.read_vec()?,
            members_by_owner: CsrIndex::read_from(reader)?,
            members_by_owner_name: reader.read_vec()?,
            members_by_owner_name_kind: reader.read_vec()?,
            callable_ids: reader.read_vec()?,
            callables_by_owner: CsrIndex::read_from(reader)?,
            callables_by_owner_name: reader.read_vec()?,
            constructors_by_type: CsrIndex::read_from(reader)?,
            global_names: reader.read_vec()?,
            globals_by_domain_name_kind: reader.read_vec()?,
            module_event_names: reader.read_vec()?,
            module_contexts_by_domain_language_kind: reader.read_vec()?,
            query_table_ids: reader.read_vec()?,
            query_table_names: reader.read_vec()?,
            query_table_syntax_names: reader.read_vec()?,
            query_table_identifiers: reader.read_vec()?,
            query_fields_by_table: CsrIndex::read_from(reader)?,
            query_fields_by_table_name: reader.read_vec()?,
            query_parameters_by_table: CsrIndex::read_from(reader)?,
            query_parameters_by_table_name: reader.read_vec()?,
            language_ids: reader.read_vec()?,
            language_names: reader.read_vec()?,
            enum_ids: reader.read_vec()?,
            enum_names: reader.read_vec()?,
            enum_value_ids: reader.read_vec()?,
            enum_values_by_enum: CsrIndex::read_from(reader)?,
            enum_values_by_enum_name: reader.read_vec()?,
            availability_by_fact: CsrIndex::read_from(reader)?,
            availability_since_by_fact: reader.read_vec()?,
            relations_by_source_kind: CsrIndex::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkName {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.primary.write_to(writer)?;
        self.alias.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            primary: StringId::read_from(reader)?,
            alias: Option::<StringId>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkPlatformType {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.id.write_to(writer)?;
        self.name.write_to(writer)?;
        self.type_template_key.write_to(writer)?;
        self.availability_contexts.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            id: StringId::read_from(reader)?,
            name: HbkName::read_from(reader)?,
            type_template_key: Option::<HbkPlatformTypeTemplateKey>::read_from(reader)?,
            availability_contexts: Vec::<StringId>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkTypeMember {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.id.write_to(writer)?;
        self.owner.write_to(writer)?;
        self.kind.write_to(writer)?;
        self.name.write_to(writer)?;
        self.type_refs.write_to(writer)?;
        self.availability_contexts.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            id: StringId::read_from(reader)?,
            owner: HbkPlatformTypeId::read_from(reader)?,
            kind: HbkTypeMemberKind::read_from(reader)?,
            name: HbkName::read_from(reader)?,
            type_refs: Vec::<HbkTypeRef>::read_from(reader)?,
            availability_contexts: Vec::<StringId>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkCallable {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.id.write_to(writer)?;
        self.owner.write_to(writer)?;
        self.kind.write_to(writer)?;
        self.name.write_to(writer)?;
        self.signatures.write_to(writer)?;
        self.return_type_refs.write_to(writer)?;
        self.availability_contexts.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            id: StringId::read_from(reader)?,
            owner: Option::<HbkPlatformTypeId>::read_from(reader)?,
            kind: HbkCallableKind::read_from(reader)?,
            name: HbkName::read_from(reader)?,
            signatures: Vec::<HbkSignature>::read_from(reader)?,
            return_type_refs: Vec::<HbkTypeRef>::read_from(reader)?,
            availability_contexts: Vec::<StringId>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkSignature {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.text.write_to(writer)?;
        self.parameters.write_to(writer)?;
        self.return_type_refs.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            text: StringId::read_from(reader)?,
            parameters: Vec::<HbkParameter>::read_from(reader)?,
            return_type_refs: Vec::<HbkTypeRef>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkParameter {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.name.write_to(writer)?;
        writer.write_bool(self.required)?;
        self.type_refs.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            name: StringId::read_from(reader)?,
            required: reader.read_bool()?,
            type_refs: Vec::<HbkTypeRef>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkGlobalFact {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.id.write_to(writer)?;
        self.kind.write_to(writer)?;
        self.domain.write_to(writer)?;
        self.name.write_to(writer)?;
        self.callable.write_to(writer)?;
        self.type_refs.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            id: StringId::read_from(reader)?,
            kind: HbkGlobalFactKind::read_from(reader)?,
            domain: HbkLanguageDomain::read_from(reader)?,
            name: HbkName::read_from(reader)?,
            callable: Option::<HbkCallableId>::read_from(reader)?,
            type_refs: Vec::<HbkTypeRef>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkQueryTable {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.id.write_to(writer)?;
        self.name.write_to(writer)?;
        self.syntax.write_to(writer)?;
        self.identifier.write_to(writer)?;
        self.role.write_to(writer)?;
        self.owner_path.write_to(writer)?;
        self.template_parameters.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            id: StringId::read_from(reader)?,
            name: HbkName::read_from(reader)?,
            syntax: Option::<HbkName>::read_from(reader)?,
            identifier: Option::<StringId>::read_from(reader)?,
            role: Option::<model::QueryTableRole>::read_from(reader)?,
            owner_path: Vec::<HbkName>::read_from(reader)?,
            template_parameters: Vec::<StringId>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkQueryField {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.id.write_to(writer)?;
        self.owner.write_to(writer)?;
        self.name.write_to(writer)?;
        self.type_refs.write_to(writer)?;
        self.note.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            id: StringId::read_from(reader)?,
            owner: HbkQueryTableId::read_from(reader)?,
            name: HbkName::read_from(reader)?,
            type_refs: Vec::<HbkTypeRef>::read_from(reader)?,
            note: Option::<StringId>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkQueryParameter {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.id.write_to(writer)?;
        self.owner.write_to(writer)?;
        self.name.write_to(writer)?;
        self.type_refs.write_to(writer)?;
        self.default_value.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            id: StringId::read_from(reader)?,
            owner: HbkQueryTableId::read_from(reader)?,
            name: HbkName::read_from(reader)?,
            type_refs: Vec::<HbkTypeRef>::read_from(reader)?,
            default_value: Option::<StringId>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkLanguageFact {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.id.write_to(writer)?;
        self.kind.write_to(writer)?;
        self.domain.write_to(writer)?;
        self.name.write_to(writer)?;
        self.signatures.write_to(writer)?;
        self.type_refs.write_to(writer)?;
        self.return_type_refs.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            id: StringId::read_from(reader)?,
            kind: SearchDocumentKind::read_from(reader)?,
            domain: HbkLanguageDomain::read_from(reader)?,
            name: HbkName::read_from(reader)?,
            signatures: Vec::<HbkSignature>::read_from(reader)?,
            type_refs: Vec::<HbkTypeRef>::read_from(reader)?,
            return_type_refs: Vec::<HbkTypeRef>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkEnum {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.id.write_to(writer)?;
        self.name.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            id: StringId::read_from(reader)?,
            name: HbkName::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkEnumValue {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.id.write_to(writer)?;
        self.owner.write_to(writer)?;
        self.name.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            id: StringId::read_from(reader)?,
            owner: HbkEnumId::read_from(reader)?,
            name: HbkName::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkTypeRef {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.name.write_to(writer)?;
        self.target.write_to(writer)?;
        self.type_template_key.write_to(writer)?;
        self.template_binding.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            name: StringId::read_from(reader)?,
            target: HbkTypeRefTarget::read_from(reader)?,
            type_template_key: Option::<HbkPlatformTypeTemplateKey>::read_from(reader)?,
            template_binding: Option::<HbkTypeTemplateBinding>::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkTypeRefTarget {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        match self {
            Self::Ok(id) => {
                writer.write_u8(0)?;
                id.write_to(writer)
            }
            Self::Unresolved => writer.write_u8(1),
            Self::Ambiguous(ids) => {
                writer.write_u8(2)?;
                ids.write_to(writer)
            }
        }
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        match reader.read_u8()? {
            0 => Ok(Self::Ok(StringId::read_from(reader)?)),
            1 => Ok(Self::Unresolved),
            2 => Ok(Self::Ambiguous(Vec::<StringId>::read_from(reader)?)),
            _ => Err(invalid_data("invalid type-ref target tag")),
        }
    }
}

impl BinaryValue for HbkPlatformTypeTemplateKey {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.family.write_to(writer)?;
        self.variant.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            family: StringId::read_from(reader)?,
            variant: StringId::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkTypeTemplateBinding {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.template_key.write_to(writer)?;
        self.arguments.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            template_key: HbkPlatformTypeTemplateKey::read_from(reader)?,
            arguments: Vec::<model::TemplateParameterBinding>::read_from(reader)?,
        })
    }
}

impl BinaryValue for model::TemplateParameterBinding {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        match self {
            Self::OwnerParameter {
                owner_parameter_index,
                target_parameter_index,
            } => {
                writer.write_u8(0)?;
                owner_parameter_index.write_to(writer)?;
                target_parameter_index.write_to(writer)
            }
        }
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        match reader.read_u8()? {
            0 => Ok(Self::OwnerParameter {
                owner_parameter_index: usize::read_from(reader)?,
                target_parameter_index: usize::read_from(reader)?,
            }),
            _ => Err(invalid_data("invalid template-parameter binding tag")),
        }
    }
}

impl BinaryValue for model::QueryTableRole {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_u8(match self {
            Self::Primary => 0,
            Self::Additional => 1,
            Self::Unknown => 2,
        })
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        match reader.read_u8()? {
            0 => Ok(Self::Primary),
            1 => Ok(Self::Additional),
            2 => Ok(Self::Unknown),
            _ => Err(invalid_data("invalid query-table role tag")),
        }
    }
}

impl BinaryValue for SearchDocumentKind {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_u8(match self {
            Self::PlatformType => 0,
            Self::TypeProperty => 1,
            Self::TypeMethod => 2,
            Self::Constructor => 3,
            Self::GlobalMethod => 4,
            Self::GlobalProperty => 5,
            Self::ModuleEvent => 6,
            Self::TypeEvent => 7,
            Self::UnknownEvent => 8,
            Self::QueryTable => 9,
            Self::QueryTableField => 10,
            Self::QueryTableParameter => 11,
            Self::LanguageType => 12,
            Self::LanguageConstruct => 13,
            Self::LanguageFunction => 14,
            Self::LanguageOperator => 15,
            Self::LanguageKeyword => 16,
            Self::LanguageLiteral => 17,
            Self::Enum => 18,
            Self::EnumValue => 19,
        })
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        match reader.read_u8()? {
            0 => Ok(Self::PlatformType),
            1 => Ok(Self::TypeProperty),
            2 => Ok(Self::TypeMethod),
            3 => Ok(Self::Constructor),
            4 => Ok(Self::GlobalMethod),
            5 => Ok(Self::GlobalProperty),
            6 => Ok(Self::ModuleEvent),
            7 => Ok(Self::TypeEvent),
            8 => Ok(Self::UnknownEvent),
            9 => Ok(Self::QueryTable),
            10 => Ok(Self::QueryTableField),
            11 => Ok(Self::QueryTableParameter),
            12 => Ok(Self::LanguageType),
            13 => Ok(Self::LanguageConstruct),
            14 => Ok(Self::LanguageFunction),
            15 => Ok(Self::LanguageOperator),
            16 => Ok(Self::LanguageKeyword),
            17 => Ok(Self::LanguageLiteral),
            18 => Ok(Self::Enum),
            19 => Ok(Self::EnumValue),
            _ => Err(invalid_data("invalid search document kind tag")),
        }
    }
}

impl BinaryValue for HbkFactRef {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        match self {
            Self::PlatformType(id) => write_tagged_id(writer, 0, id),
            Self::TypeMember(id) => write_tagged_id(writer, 1, id),
            Self::Callable(id) => write_tagged_id(writer, 2, id),
            Self::Global(id) => write_tagged_id(writer, 3, id),
            Self::QueryTable(id) => write_tagged_id(writer, 4, id),
            Self::QueryField(id) => write_tagged_id(writer, 5, id),
            Self::QueryParameter(id) => write_tagged_id(writer, 6, id),
            Self::LanguageFact(id) => write_tagged_id(writer, 7, id),
            Self::Enum(id) => write_tagged_id(writer, 8, id),
            Self::EnumValue(id) => write_tagged_id(writer, 9, id),
        }
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        match reader.read_u8()? {
            0 => Ok(Self::PlatformType(HbkPlatformTypeId::read_from(reader)?)),
            1 => Ok(Self::TypeMember(HbkTypeMemberId::read_from(reader)?)),
            2 => Ok(Self::Callable(HbkCallableId::read_from(reader)?)),
            3 => Ok(Self::Global(HbkGlobalFactId::read_from(reader)?)),
            4 => Ok(Self::QueryTable(HbkQueryTableId::read_from(reader)?)),
            5 => Ok(Self::QueryField(HbkQueryFieldId::read_from(reader)?)),
            6 => Ok(Self::QueryParameter(HbkQueryParameterId::read_from(
                reader,
            )?)),
            7 => Ok(Self::LanguageFact(HbkLanguageFactId::read_from(reader)?)),
            8 => Ok(Self::Enum(HbkEnumId::read_from(reader)?)),
            9 => Ok(Self::EnumValue(HbkEnumValueId::read_from(reader)?)),
            _ => Err(invalid_data("invalid fact-ref tag")),
        }
    }
}

impl BinaryValue for HbkTypeMemberKind {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_u8(match self {
            Self::Property => 0,
            Self::Method => 1,
            Self::Event => 2,
            Self::EnumValue => 3,
        })
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        match reader.read_u8()? {
            0 => Ok(Self::Property),
            1 => Ok(Self::Method),
            2 => Ok(Self::Event),
            3 => Ok(Self::EnumValue),
            _ => Err(invalid_data("invalid type-member kind tag")),
        }
    }
}

impl BinaryValue for HbkCallableKind {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_u8(match self {
            Self::Method => 0,
            Self::Constructor => 1,
            Self::GlobalMethod => 2,
            Self::Event => 3,
            Self::LanguageFunction => 4,
        })
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        match reader.read_u8()? {
            0 => Ok(Self::Method),
            1 => Ok(Self::Constructor),
            2 => Ok(Self::GlobalMethod),
            3 => Ok(Self::Event),
            4 => Ok(Self::LanguageFunction),
            _ => Err(invalid_data("invalid callable kind tag")),
        }
    }
}

impl BinaryValue for HbkGlobalFactKind {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_u8(match self {
            Self::Method => 0,
            Self::Property => 1,
        })
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        match reader.read_u8()? {
            0 => Ok(Self::Method),
            1 => Ok(Self::Property),
            _ => Err(invalid_data("invalid global fact kind tag")),
        }
    }
}

impl BinaryValue for HbkLanguageDomain {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        writer.write_u8(match self {
            Self::Bsl => 0,
            Self::Query => 1,
            Self::DataComposition => 2,
            Self::Unknown => 3,
        })
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        match reader.read_u8()? {
            0 => Ok(Self::Bsl),
            1 => Ok(Self::Query),
            2 => Ok(Self::DataComposition),
            3 => Ok(Self::Unknown),
            _ => Err(invalid_data("invalid language domain tag")),
        }
    }
}

impl<T: BinaryValue> BinaryValue for IdLookup<T> {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.key.write_to(writer)?;
        self.value.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            key: StringId::read_from(reader)?,
            value: T::read_from(reader)?,
        })
    }
}

impl<T: BinaryValue> BinaryValue for NameLookup<T> {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.key.write_to(writer)?;
        self.value.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            key: StringId::read_from(reader)?,
            value: T::read_from(reader)?,
        })
    }
}

impl<Owner: BinaryValue, Value: BinaryValue> BinaryValue for OwnerNameLookup<Owner, Value> {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.owner.write_to(writer)?;
        self.key.write_to(writer)?;
        self.value.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            owner: Owner::read_from(reader)?,
            key: StringId::read_from(reader)?,
            value: Value::read_from(reader)?,
        })
    }
}

impl<T: BinaryValue> BinaryValue for TypeTemplateLookup<T> {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.family.write_to(writer)?;
        self.variant.write_to(writer)?;
        self.value.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            family: StringId::read_from(reader)?,
            variant: StringId::read_from(reader)?,
            value: T::read_from(reader)?,
        })
    }
}

impl BinaryValue for MemberNameKindLookup {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.owner.write_to(writer)?;
        self.key.write_to(writer)?;
        self.kind.write_to(writer)?;
        self.value.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            owner: HbkPlatformTypeId::read_from(reader)?,
            key: StringId::read_from(reader)?,
            kind: Option::<HbkTypeMemberKind>::read_from(reader)?,
            value: HbkTypeMemberId::read_from(reader)?,
        })
    }
}

impl BinaryValue for GlobalNameKindLookup {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.domain.write_to(writer)?;
        self.key.write_to(writer)?;
        self.kind.write_to(writer)?;
        self.value.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            domain: HbkLanguageDomain::read_from(reader)?,
            key: StringId::read_from(reader)?,
            kind: Option::<HbkGlobalFactKind>::read_from(reader)?,
            value: HbkGlobalFactId::read_from(reader)?,
        })
    }
}

impl BinaryValue for FactStringLookup {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.fact.write_to(writer)?;
        self.value.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            fact: HbkFactRef::read_from(reader)?,
            value: StringId::read_from(reader)?,
        })
    }
}

impl BinaryValue for ModuleContextLookup {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.domain.write_to(writer)?;
        self.language_key.write_to(writer)?;
        self.module_kind.write_to(writer)?;
        self.value.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            domain: HbkLanguageDomain::read_from(reader)?,
            language_key: StringId::read_from(reader)?,
            module_kind: StringId::read_from(reader)?,
            value: HbkCallableId::read_from(reader)?,
        })
    }
}

impl BinaryValue for RelationLookupKey {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.source.write_to(writer)?;
        self.kind.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            source: HbkFactRef::read_from(reader)?,
            kind: StringId::read_from(reader)?,
        })
    }
}

impl<K: BinaryValue, V: BinaryValue> BinaryValue for CsrIndex<K, V> {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.keys.write_to(writer)?;
        self.offsets.write_to(writer)?;
        self.values.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            keys: Vec::<K>::read_from(reader)?,
            offsets: Vec::<u32>::read_from(reader)?,
            values: Vec::<V>::read_from(reader)?,
        })
    }
}

fn write_tagged_id<W: Write, T: BinaryValue>(
    writer: &mut BinaryWriter<W>,
    tag: u8,
    id: &T,
) -> io::Result<()> {
    writer.write_u8(tag)?;
    id.write_to(writer)
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
