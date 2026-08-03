use std::io::{Read, Write};

use super::indexes::*;
use super::*;

pub(super) trait BinaryValue: Sized {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()>;
    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self>;
}

pub(super) struct BinaryWriter<W> {
    pub(super) inner: W,
}

impl<W: Write> BinaryWriter<W> {
    pub(super) fn new(inner: W) -> Self {
        Self { inner }
    }

    pub(super) fn write_u8(&mut self, value: u8) -> io::Result<()> {
        self.inner.write_all(&[value])
    }

    fn write_u32(&mut self, value: u32) -> io::Result<()> {
        self.inner.write_all(&value.to_le_bytes())
    }

    pub(super) fn write_bool(&mut self, value: bool) -> io::Result<()> {
        self.write_u8(u8::from(value))
    }
}

impl<W> BinaryWriter<W> {
    pub(super) fn into_inner(self) -> W {
        self.inner
    }
}

pub(super) struct BinaryReader<R> {
    pub(super) inner: R,
}

impl<R: Read> BinaryReader<R> {
    pub(super) fn new(inner: R) -> Self {
        Self { inner }
    }

    pub(super) fn read_u8(&mut self) -> io::Result<u8> {
        let mut bytes = [0u8; 1];
        self.inner.read_exact(&mut bytes)?;
        Ok(bytes[0])
    }

    fn read_u32(&mut self) -> io::Result<u32> {
        let mut bytes = [0u8; 4];
        self.inner.read_exact(&mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    pub(super) fn read_bool(&mut self) -> io::Result<bool> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid_data("invalid boolean tag")),
        }
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
        let value = u64::try_from(*self).map_err(|_| invalid_data("usize does not fit u64"))?;
        writer.inner.write_all(&value.to_le_bytes())
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        let mut bytes = [0_u8; 8];
        reader.inner.read_exact(&mut bytes)?;
        usize::try_from(u64::from_le_bytes(bytes))
            .map_err(|_| invalid_data("u64 does not fit usize"))
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

impl BinaryValue for FactSourceLookup {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.fact.write_to(writer)?;
        self.source.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            fact: HbkFactRef::read_from(reader)?,
            source: HbkFactSource::read_from(reader)?,
        })
    }
}

impl BinaryValue for HbkFactSource {
    fn write_to<W: Write>(&self, writer: &mut BinaryWriter<W>) -> io::Result<()> {
        self.hbk_path.write_to(writer)?;
        self.locale.write_to(writer)?;
        self.toc_path.write_to(writer)?;
        self.html_path.write_to(writer)?;
        self.page_title.write_to(writer)
    }

    fn read_from<R: Read>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        Ok(Self {
            hbk_path: StringId::read_from(reader)?,
            locale: StringId::read_from(reader)?,
            toc_path: Option::<StringId>::read_from(reader)?,
            html_path: StringId::read_from(reader)?,
            page_title: StringId::read_from(reader)?,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fact_source_binary_roundtrip_preserves_all_fields() {
        let source = HbkFactSource {
            hbk_path: StringId(1),
            locale: StringId(2),
            toc_path: Some(StringId(3)),
            html_path: StringId(4),
            page_title: StringId(5),
        };

        assert_eq!(
            read_binary_value::<HbkFactSource>(&write_binary_value(&source)),
            source
        );
    }

    #[test]
    fn fact_source_binary_roundtrip_preserves_missing_toc_path() {
        let source = HbkFactSource {
            hbk_path: StringId(1),
            locale: StringId(2),
            toc_path: None,
            html_path: StringId(4),
            page_title: StringId(5),
        };

        assert_eq!(
            read_binary_value::<HbkFactSource>(&write_binary_value(&source)),
            source
        );
    }

    #[test]
    fn fact_source_binary_payload_changes_when_provenance_changes() {
        let before = HbkFactSource {
            hbk_path: StringId(1),
            locale: StringId(2),
            toc_path: Some(StringId(3)),
            html_path: StringId(4),
            page_title: StringId(5),
        };
        let after = HbkFactSource {
            page_title: StringId(6),
            ..before
        };

        assert_ne!(write_binary_value(&before), write_binary_value(&after));
    }

    fn write_binary_value<T: BinaryValue>(value: &T) -> Vec<u8> {
        let mut output = Vec::new();
        {
            let mut writer = BinaryWriter::new(&mut output);
            value.write_to(&mut writer).unwrap();
            writer.inner.flush().unwrap();
        }
        output
    }

    fn read_binary_value<T: BinaryValue>(bytes: &[u8]) -> T {
        let mut reader = BinaryReader::new(std::io::Cursor::new(bytes));
        T::read_from(&mut reader).unwrap()
    }
}
