use std::sync::Arc;

use context_resolver_core::SourceId;
use syntax_helper_search::{
    HbkFactReadHandle, HbkFactRef, HbkFactSnapshot, HbkQueryFieldId, HbkQueryFieldView,
    HbkQueryParameterId, HbkQueryParameterView, HbkQueryTableId, HbkQueryTableView, StringId,
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
        self.read_handle().source_locale()
    }

    pub fn string(&self, id: StringId) -> &str {
        self.read_handle().string(id)
    }

    pub fn query_table_by_id(&self, id: &str) -> Option<(HbkQueryTableId, HbkQueryTableView<'_>)> {
        let handle = self.read_handle();
        let id = handle.query_table_by_id(id)?;
        Some((id, handle.query_table(id)))
    }

    pub fn query_tables(
        &self,
    ) -> impl Iterator<Item = (HbkQueryTableId, HbkQueryTableView<'_>)> + '_ {
        let handle = self.read_handle();
        handle
            .query_table_ids()
            .map(move |id| (id, handle.query_table(id)))
    }

    pub fn query_tables_by_name<'a>(
        &'a self,
        name: &str,
    ) -> impl Iterator<Item = (HbkQueryTableId, HbkQueryTableView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .query_tables_by_name(name)
            .map(move |id| (id, handle.query_table(id)))
    }

    pub fn query_tables_by_syntax<'a>(
        &'a self,
        syntax: &str,
    ) -> impl Iterator<Item = (HbkQueryTableId, HbkQueryTableView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .query_tables_by_syntax(syntax)
            .map(move |id| (id, handle.query_table(id)))
    }

    pub fn query_tables_by_identifier<'a>(
        &'a self,
        identifier: &str,
    ) -> impl Iterator<Item = (HbkQueryTableId, HbkQueryTableView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .query_tables_by_identifier(identifier)
            .map(move |id| (id, handle.query_table(id)))
    }

    pub fn query_field_by_id(&self, id: &str) -> Option<(HbkQueryFieldId, HbkQueryFieldView<'_>)> {
        let handle = self.read_handle();
        handle.facts_by_id(id).find_map(|fact| match fact {
            HbkFactRef::QueryField(id) => Some((id, handle.query_field(id))),
            _ => None,
        })
    }

    pub fn query_fields(
        &self,
        table: HbkQueryTableId,
    ) -> impl ExactSizeIterator<Item = (HbkQueryFieldId, HbkQueryFieldView<'_>)> + '_ {
        let handle = self.read_handle();
        handle
            .query_fields(table)
            .map(move |id| (id, handle.query_field(id)))
    }

    pub fn query_field_by_name<'a>(
        &'a self,
        table: HbkQueryTableId,
        name: &str,
    ) -> impl Iterator<Item = (HbkQueryFieldId, HbkQueryFieldView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .query_fields_by_name(table, name)
            .map(move |id| (id, handle.query_field(id)))
    }

    pub fn query_parameter_by_id(
        &self,
        id: &str,
    ) -> Option<(HbkQueryParameterId, HbkQueryParameterView<'_>)> {
        let handle = self.read_handle();
        handle.facts_by_id(id).find_map(|fact| match fact {
            HbkFactRef::QueryParameter(id) => Some((id, handle.query_parameter(id))),
            _ => None,
        })
    }

    pub fn query_parameters(
        &self,
        table: HbkQueryTableId,
    ) -> impl ExactSizeIterator<Item = (HbkQueryParameterId, HbkQueryParameterView<'_>)> + '_ {
        let handle = self.read_handle();
        handle
            .query_parameters(table)
            .map(move |id| (id, handle.query_parameter(id)))
    }

    pub fn query_parameter_by_name<'a>(
        &'a self,
        table: HbkQueryTableId,
        name: &str,
    ) -> impl Iterator<Item = (HbkQueryParameterId, HbkQueryParameterView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .query_parameters_by_name(table, name)
            .map(move |id| (id, handle.query_parameter(id)))
    }

    pub fn metadata_source_selector(&self, table: HbkQueryTableId) -> Option<&'static str> {
        let table = self.read_handle().query_table(table);
        self.metadata_source_selector_for_identifier(table.identifier().map(|id| self.string(id)))
    }

    pub fn metadata_source_selector_for_identifier(
        &self,
        identifier: Option<&str>,
    ) -> Option<&'static str> {
        sdbl_metadata_source_selector(self.source_locale(), identifier)
    }

    pub(crate) fn read_handle(&self) -> HbkFactReadHandle<'_> {
        self.snapshot.worker_handle()
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
