use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerHeader {
    pub free_block_offset: u32,
    pub default_block_size: u32,
    pub entity_count_hint: u32,
    pub reserved: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockHeader {
    pub offset: usize,
    pub payload_size: usize,
    pub block_size: usize,
    pub next_block_offset: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityName(String);

impl EntityName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EntityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for EntityName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityDescriptor {
    pub name: EntityName,
    pub descriptor_offset: usize,
    pub header_offset: usize,
    pub body_offset: Option<usize>,
}
