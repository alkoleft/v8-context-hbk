use serde::Serialize;

pub(crate) const SCHEMA_VERSION: u32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExportFile {
    pub file_name: &'static str,
    pub record_kind: &'static str,
}

pub const EXPORT_FILES: &[ExportFile] = &[
    ExportFile {
        file_name: "global-methods.json",
        record_kind: "global_method",
    },
    ExportFile {
        file_name: "global-properties.json",
        record_kind: "global_property",
    },
    ExportFile {
        file_name: "global-context-events.json",
        record_kind: "module_event",
    },
    ExportFile {
        file_name: "platform-types.json",
        record_kind: "platform_type",
    },
    ExportFile {
        file_name: "type-methods.json",
        record_kind: "type_method",
    },
    ExportFile {
        file_name: "type-properties.json",
        record_kind: "type_property",
    },
    ExportFile {
        file_name: "table-fields.json",
        record_kind: "table_field",
    },
    ExportFile {
        file_name: "table-parameters.json",
        record_kind: "table_parameter",
    },
    ExportFile {
        file_name: "constructors.json",
        record_kind: "constructor",
    },
    ExportFile {
        file_name: "enums.json",
        record_kind: "enum",
    },
    ExportFile {
        file_name: "diagnostics.json",
        record_kind: "diagnostic",
    },
];
