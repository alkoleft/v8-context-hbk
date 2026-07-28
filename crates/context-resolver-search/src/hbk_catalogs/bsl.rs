use std::sync::Arc;

use context_resolver_core::{ModuleContextKind, SourceId};
use syntax_helper_search::{
    HbkCallable, HbkCallableId, HbkFactRef, HbkFactSnapshot, HbkGlobalFact, HbkGlobalFactId,
    HbkGlobalFactKind, HbkLanguageDomain, HbkPlatformType, HbkPlatformTypeId, HbkTypeMember,
    HbkTypeMemberId, HbkTypeMemberKind, StringId,
};

use crate::{DEFAULT_SOURCE_ID, template_key_parts_for_generated_self_role};

pub struct HbkBslContextCatalog {
    source_id: SourceId,
    snapshot: Arc<HbkFactSnapshot>,
}

impl HbkBslContextCatalog {
    pub fn new(snapshot: Arc<HbkFactSnapshot>) -> Self {
        Self::with_source_id(snapshot, SourceId::new(DEFAULT_SOURCE_ID))
    }

    pub fn with_source_id(snapshot: Arc<HbkFactSnapshot>, source_id: SourceId) -> Self {
        Self {
            source_id,
            snapshot,
        }
    }

    pub fn source_id(&self) -> &SourceId {
        &self.source_id
    }

    pub fn source_locale(&self) -> Option<&str> {
        self.snapshot.source_locale()
    }

    pub fn string(&self, id: StringId) -> &str {
        self.snapshot.string(id)
    }

    pub fn platform_type_by_id(&self, id: &str) -> Option<(HbkPlatformTypeId, &HbkPlatformType)> {
        let id = self.snapshot.worker_handle().platform_type_by_id(id)?;
        Some((id, self.snapshot.platform_type(id)))
    }

    pub fn platform_types_by_name<'a>(
        &'a self,
        name: &str,
    ) -> impl Iterator<Item = (HbkPlatformTypeId, &'a HbkPlatformType)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .platform_types_by_name(name)
            .map(|id| (id, self.snapshot.platform_type(id)))
    }

    pub fn member_by_id(&self, id: &str) -> Option<(HbkTypeMemberId, &HbkTypeMember)> {
        self.snapshot
            .worker_handle()
            .facts_by_id(id)
            .find_map(|fact| match fact {
                HbkFactRef::TypeMember(id) => Some((id, self.snapshot.type_member(id))),
                _ => None,
            })
    }

    pub fn callable_by_id(&self, id: &str) -> Option<(HbkCallableId, &HbkCallable)> {
        self.snapshot
            .worker_handle()
            .facts_by_id(id)
            .find_map(|fact| match fact {
                HbkFactRef::Callable(id) => Some((id, self.snapshot.callable(id))),
                _ => None,
            })
    }

    pub fn global_by_id(&self, id: &str) -> Option<(HbkGlobalFactId, &HbkGlobalFact)> {
        self.snapshot
            .worker_handle()
            .facts_by_id(id)
            .find_map(|fact| match fact {
                HbkFactRef::Global(id) => {
                    let global = self.snapshot.global_fact(id);
                    (global.domain == HbkLanguageDomain::Bsl).then_some((id, global))
                }
                _ => None,
            })
    }

    pub fn platform_types_by_template_key<'a>(
        &'a self,
        family: &str,
        variant: &str,
    ) -> impl Iterator<Item = (HbkPlatformTypeId, &'a HbkPlatformType)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .platform_types_by_template_key(family, variant)
            .map(|id| (id, self.snapshot.platform_type(id)))
    }

    pub fn generated_self_types<'a>(
        &'a self,
        role: &str,
    ) -> impl Iterator<Item = (HbkPlatformTypeId, &'a HbkPlatformType)> + 'a + use<'a> {
        template_key_parts_for_generated_self_role(role)
            .into_iter()
            .flat_map(|(family, variant)| self.platform_types_by_template_key(family, variant))
    }

    pub fn members(
        &self,
        owner: HbkPlatformTypeId,
    ) -> impl ExactSizeIterator<Item = (HbkTypeMemberId, &HbkTypeMember)> + '_ {
        self.snapshot
            .worker_handle()
            .members_of_type(owner)
            .iter()
            .copied()
            .map(|id| (id, self.snapshot.type_member(id)))
    }

    pub fn member_by_name<'a>(
        &'a self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> impl Iterator<Item = (HbkTypeMemberId, &'a HbkTypeMember)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .member_by_owner_name(owner, name)
            .map(|id| (id, self.snapshot.type_member(id)))
    }

    pub fn member_by_name_kind<'a>(
        &'a self,
        owner: HbkPlatformTypeId,
        name: &str,
        kind: Option<HbkTypeMemberKind>,
    ) -> impl Iterator<Item = (HbkTypeMemberId, &'a HbkTypeMember)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .member_by_owner_name_kind(owner, name, kind)
            .map(|id| (id, self.snapshot.type_member(id)))
    }

    pub fn callables(
        &self,
        owner: HbkPlatformTypeId,
    ) -> impl ExactSizeIterator<Item = (HbkCallableId, &HbkCallable)> + '_ {
        self.snapshot
            .worker_handle()
            .callables_of_type(owner)
            .iter()
            .copied()
            .map(|id| (id, self.snapshot.callable(id)))
    }

    pub fn callable_by_name<'a>(
        &'a self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> impl Iterator<Item = (HbkCallableId, &'a HbkCallable)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .callable_by_owner_name(owner, name)
            .map(|id| (id, self.snapshot.callable(id)))
    }

    pub fn constructors(
        &self,
        owner: HbkPlatformTypeId,
    ) -> impl ExactSizeIterator<Item = (HbkCallableId, &HbkCallable)> + '_ {
        self.snapshot
            .worker_handle()
            .constructors_of_type(owner)
            .iter()
            .copied()
            .map(|id| (id, self.snapshot.callable(id)))
    }

    pub fn global_properties(
        &self,
    ) -> impl Iterator<Item = (HbkGlobalFactId, &HbkGlobalFact)> + '_ {
        self.snapshot
            .worker_handle()
            .global_fact_ids()
            .filter_map(|id| {
                let global = self.snapshot.global_fact(id);
                (global.domain == HbkLanguageDomain::Bsl
                    && global.kind == HbkGlobalFactKind::Property)
                    .then_some((id, global))
            })
    }

    pub fn global_property_by_name<'a>(
        &'a self,
        name: &str,
    ) -> impl Iterator<Item = (HbkGlobalFactId, &'a HbkGlobalFact)> + 'a + use<'a> {
        self.snapshot
            .worker_handle()
            .globals_by_domain_name_kind(
                HbkLanguageDomain::Bsl,
                name,
                Some(HbkGlobalFactKind::Property),
            )
            .map(|id| (id, self.snapshot.global_fact(id)))
    }

    pub fn global_methods(
        &self,
    ) -> impl Iterator<Item = (HbkGlobalFactId, &HbkGlobalFact, HbkCallableId, &HbkCallable)> + '_
    {
        self.snapshot
            .worker_handle()
            .global_fact_ids()
            .filter_map(|id| {
                let global = self.snapshot.global_fact(id);
                if global.domain != HbkLanguageDomain::Bsl
                    || global.kind != HbkGlobalFactKind::Method
                {
                    return None;
                }
                let callable = global.callable?;
                Some((id, global, callable, self.snapshot.callable(callable)))
            })
    }

    pub fn global_method_by_name<'a>(
        &'a self,
        name: &str,
    ) -> impl Iterator<
        Item = (
            HbkGlobalFactId,
            &'a HbkGlobalFact,
            HbkCallableId,
            &'a HbkCallable,
        ),
    >
    + 'a
    + use<'a> {
        self.snapshot
            .worker_handle()
            .globals_by_domain_name_kind(
                HbkLanguageDomain::Bsl,
                name,
                Some(HbkGlobalFactKind::Method),
            )
            .filter_map(|id| {
                let global = self.snapshot.global_fact(id);
                let callable = global.callable?;
                Some((id, global, callable, self.snapshot.callable(callable)))
            })
    }

    pub fn module_context_events(
        &self,
        kind: ModuleContextKind,
    ) -> impl Iterator<Item = (HbkCallableId, &HbkCallable)> + '_ {
        bsl_module_context_key(kind)
            .into_iter()
            .flat_map(move |key| {
                self.snapshot
                    .worker_handle()
                    .module_context_events(
                        HbkLanguageDomain::Bsl,
                        "bsl",
                        key.trim_start_matches("module_context:"),
                    )
                    .map(|id| (id, self.snapshot.callable(id)))
            })
    }

    pub fn module_context_event_by_name<'a>(
        &'a self,
        kind: ModuleContextKind,
        name: &str,
    ) -> impl Iterator<Item = (HbkCallableId, &'a HbkCallable)> + 'a + use<'a> {
        let ids = bsl_module_context_key(kind).map(|key| {
            self.snapshot
                .worker_handle()
                .module_event_by_context_name(key, name)
        });
        ids.into_iter()
            .flatten()
            .map(|id| (id, self.snapshot.callable(id)))
    }

    pub fn platform_type_availability(
        &self,
        id: HbkPlatformTypeId,
    ) -> (&[StringId], Option<StringId>) {
        self.availability(HbkFactRef::PlatformType(id))
    }

    pub fn member_availability(&self, id: HbkTypeMemberId) -> (&[StringId], Option<StringId>) {
        self.availability(HbkFactRef::TypeMember(id))
    }

    pub fn callable_availability(&self, id: HbkCallableId) -> (&[StringId], Option<StringId>) {
        self.availability(HbkFactRef::Callable(id))
    }

    pub fn global_availability(&self, id: HbkGlobalFactId) -> (&[StringId], Option<StringId>) {
        self.availability(HbkFactRef::Global(id))
    }

    pub(crate) fn snapshot(&self) -> &HbkFactSnapshot {
        &self.snapshot
    }

    fn availability(&self, fact: HbkFactRef) -> (&[StringId], Option<StringId>) {
        (
            self.snapshot.worker_handle().availability_contexts(fact),
            self.snapshot.worker_handle().available_since(fact),
        )
    }
}

pub(crate) fn bsl_module_context_key(kind: ModuleContextKind) -> Option<&'static str> {
    match kind {
        ModuleContextKind::Session => Some("module_context:session"),
        ModuleContextKind::OrdinaryApplication => Some("module_context:ordinary_application"),
        ModuleContextKind::ManagedApplication => Some("module_context:managed_application"),
        ModuleContextKind::ExternalConnection => Some("module_context:external_connection"),
        ModuleContextKind::Object => Some("module_context:object"),
        ModuleContextKind::Manager => Some("module_context:manager"),
        ModuleContextKind::Form => Some("module_context:form"),
        ModuleContextKind::WebService => Some("module_context:web_service"),
        ModuleContextKind::HttpService => Some("module_context:http_service"),
        ModuleContextKind::Unknown => Some("module_context:unknown"),
        ModuleContextKind::Common
        | ModuleContextKind::Command
        | ModuleContextKind::RecordSet
        | ModuleContextKind::Unsupported => None,
    }
}
