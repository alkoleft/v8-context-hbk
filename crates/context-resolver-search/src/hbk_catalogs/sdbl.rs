use std::sync::Arc;

use context_resolver_core::SourceId;
use syntax_helper_search::{
    HbkFactRef, HbkFactSnapshot, HbkQueryField, HbkQueryFieldId, HbkQueryParameter,
    HbkQueryParameterId, HbkQueryTable, HbkQueryTableId, StringId,
};

use crate::DEFAULT_SOURCE_ID;

pub struct HbkSdblQueryCatalog {
    source_id: SourceId,
    platform_source_id: SourceId,
    snapshot: Arc<HbkFactSnapshot>,
}

impl HbkSdblQueryCatalog {
    pub fn new(snapshot: Arc<HbkFactSnapshot>) -> Self {
        Self::with_source_ids(
            snapshot,
            SourceId::new("shcntx-query"),
            SourceId::new(DEFAULT_SOURCE_ID),
        )
    }

    pub fn with_source_ids(
        snapshot: Arc<HbkFactSnapshot>,
        source_id: SourceId,
        platform_source_id: SourceId,
    ) -> Self {
        Self {
            source_id,
            platform_source_id,
            snapshot,
        }
    }

    pub fn source_id(&self) -> &SourceId {
        &self.source_id
    }

    pub fn platform_source_id(&self) -> &SourceId {
        &self.platform_source_id
    }

    pub fn source_locale(&self) -> Option<&str> {
        self.snapshot.source_locale()
    }

    pub fn string(&self, id: StringId) -> &str {
        self.snapshot.string(id)
    }

    pub fn query_table_by_id(&self, id: &str) -> Option<(HbkQueryTableId, &HbkQueryTable)> {
        let id = self.snapshot.worker_handle().query_table_by_id(id)?;
        Some((id, self.snapshot.query_table(id)))
    }

    pub fn query_tables(&self) -> impl Iterator<Item = (HbkQueryTableId, &HbkQueryTable)> + '_ {
        self.snapshot
            .worker_handle()
            .query_table_ids()
            .map(|id| (id, self.snapshot.query_table(id)))
    }

    pub fn query_tables_by_name<'a>(
        &'a self,
        name: &str,
    ) -> impl Iterator<Item = (HbkQueryTableId, &'a HbkQueryTable)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .query_tables_by_name(name)
            .map(|id| (id, self.snapshot.query_table(id)))
    }

    pub fn query_tables_by_syntax<'a>(
        &'a self,
        syntax: &str,
    ) -> impl Iterator<Item = (HbkQueryTableId, &'a HbkQueryTable)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .query_tables_by_syntax(syntax)
            .map(|id| (id, self.snapshot.query_table(id)))
    }

    pub fn query_tables_by_identifier<'a>(
        &'a self,
        identifier: &str,
    ) -> impl Iterator<Item = (HbkQueryTableId, &'a HbkQueryTable)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .query_tables_by_identifier(identifier)
            .map(|id| (id, self.snapshot.query_table(id)))
    }

    pub fn query_field_by_id(&self, id: &str) -> Option<(HbkQueryFieldId, &HbkQueryField)> {
        self.snapshot
            .worker_handle()
            .facts_by_id(id)
            .find_map(|fact| match fact {
                HbkFactRef::QueryField(id) => Some((id, self.snapshot.query_field(id))),
                _ => None,
            })
    }

    pub fn query_fields(
        &self,
        table: HbkQueryTableId,
    ) -> impl ExactSizeIterator<Item = (HbkQueryFieldId, &HbkQueryField)> + '_ {
        self.snapshot
            .worker_handle()
            .query_fields(table)
            .iter()
            .copied()
            .map(|id| (id, self.snapshot.query_field(id)))
    }

    pub fn query_field_by_name<'a>(
        &'a self,
        table: HbkQueryTableId,
        name: &str,
    ) -> impl Iterator<Item = (HbkQueryFieldId, &'a HbkQueryField)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .query_fields_by_name(table, name)
            .map(|id| (id, self.snapshot.query_field(id)))
    }

    pub fn query_parameter_by_id(
        &self,
        id: &str,
    ) -> Option<(HbkQueryParameterId, &HbkQueryParameter)> {
        self.snapshot
            .worker_handle()
            .facts_by_id(id)
            .find_map(|fact| match fact {
                HbkFactRef::QueryParameter(id) => Some((id, self.snapshot.query_parameter(id))),
                _ => None,
            })
    }

    pub fn query_parameters(
        &self,
        table: HbkQueryTableId,
    ) -> impl ExactSizeIterator<Item = (HbkQueryParameterId, &HbkQueryParameter)> + '_ {
        self.snapshot
            .worker_handle()
            .query_parameters(table)
            .iter()
            .copied()
            .map(|id| (id, self.snapshot.query_parameter(id)))
    }

    pub fn query_parameter_by_name<'a>(
        &'a self,
        table: HbkQueryTableId,
        name: &str,
    ) -> impl Iterator<Item = (HbkQueryParameterId, &'a HbkQueryParameter)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .query_parameters_by_name(table, name)
            .map(|id| (id, self.snapshot.query_parameter(id)))
    }

    pub fn metadata_source_selector(&self, table: HbkQueryTableId) -> Option<&'static str> {
        let table = self.snapshot.query_table(table);
        self.metadata_source_selector_for_identifier(table.identifier.map(|id| self.string(id)))
    }

    pub fn metadata_source_selector_for_identifier(
        &self,
        identifier: Option<&str>,
    ) -> Option<&'static str> {
        sdbl_metadata_source_selector(self.source_locale(), identifier)
    }

    pub(crate) fn snapshot(&self) -> &HbkFactSnapshot {
        &self.snapshot
    }
}

pub(crate) fn sdbl_metadata_source_selector(
    locale: Option<&str>,
    identifier: Option<&str>,
) -> Option<&'static str> {
    if locale != Some("ru") {
        return None;
    }
    match identifier? {
        "Справочник" => Some("metadata.sdbl.query-source.catalog"),
        "Документ" => Some("metadata.sdbl.query-source.document"),
        "РегистрСведений" => Some("metadata.sdbl.query-source.information-register"),
        "РегистрНакопления" => {
            Some("metadata.sdbl.query-source.accumulation-register")
        }
        "РегистрБухгалтерии" => {
            Some("metadata.sdbl.query-source.accounting-register")
        }
        "РегистрРасчета" => Some("metadata.sdbl.query-source.calculation-register"),
        _ => None,
    }
}
