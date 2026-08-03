use std::sync::Arc;

use context_resolver_core::{AvailabilityContext, ModuleContextKind, SourceId};
use syntax_helper_search::{
    HbkAvailabilityFilter, HbkAvailabilityFilterMode, HbkCallableId, HbkCallableView,
    HbkFactReadHandle, HbkFactRef, HbkFactSnapshot, HbkGlobalFactId, HbkGlobalFactKind,
    HbkGlobalFactView, HbkLanguageDomain, HbkPlatformTypeId, HbkPlatformTypeView, HbkTypeMemberId,
    HbkTypeMemberKind, HbkTypeMemberView, StringId,
};

use crate::{
    DEFAULT_SOURCE_ID, availability_context_from_code, template_key_parts_for_generated_self_role,
};

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
        self.read_handle().source_locale()
    }

    pub fn string(&self, id: StringId) -> &str {
        self.read_handle().string(id)
    }

    pub fn platform_type_by_id(
        &self,
        id: &str,
    ) -> Option<(HbkPlatformTypeId, HbkPlatformTypeView<'_>)> {
        let handle = self.read_handle();
        let id = handle.platform_type_by_id(id)?;
        Some((id, handle.platform_type(id)))
    }

    pub fn platform_types_by_name<'a>(
        &'a self,
        name: &str,
    ) -> impl Iterator<Item = (HbkPlatformTypeId, HbkPlatformTypeView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .platform_types_by_name(name)
            .map(move |id| (id, handle.platform_type(id)))
    }

    pub fn member_by_id(&self, id: &str) -> Option<(HbkTypeMemberId, HbkTypeMemberView<'_>)> {
        let handle = self.read_handle();
        handle.facts_by_id(id).find_map(|fact| match fact {
            HbkFactRef::TypeMember(id) => Some((id, handle.type_member(id))),
            _ => None,
        })
    }

    pub fn callable_by_id(&self, id: &str) -> Option<(HbkCallableId, HbkCallableView<'_>)> {
        let handle = self.read_handle();
        handle.facts_by_id(id).find_map(|fact| match fact {
            HbkFactRef::Callable(id) => Some((id, handle.callable(id))),
            _ => None,
        })
    }

    pub fn global_by_id(&self, id: &str) -> Option<(HbkGlobalFactId, HbkGlobalFactView<'_>)> {
        let handle = self.read_handle();
        handle.facts_by_id(id).find_map(|fact| match fact {
            HbkFactRef::Global(id) => {
                let global = handle.global_fact(id);
                (global.domain() == HbkLanguageDomain::Bsl).then_some((id, global))
            }
            _ => None,
        })
    }

    pub fn platform_types_by_template_key<'a>(
        &'a self,
        family: &str,
        variant: &str,
    ) -> impl Iterator<Item = (HbkPlatformTypeId, HbkPlatformTypeView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .platform_types_by_template_key(family, variant)
            .map(move |id| (id, handle.platform_type(id)))
    }

    pub fn generated_self_types<'a>(
        &'a self,
        role: &str,
    ) -> impl Iterator<Item = (HbkPlatformTypeId, HbkPlatformTypeView<'a>)> + 'a + use<'a> {
        template_key_parts_for_generated_self_role(role)
            .into_iter()
            .flat_map(|(family, variant)| self.platform_types_by_template_key(family, variant))
    }

    pub fn members(
        &self,
        owner: HbkPlatformTypeId,
    ) -> impl Iterator<Item = (HbkTypeMemberId, HbkTypeMemberView<'_>)> + '_ {
        self.filtered_members(owner, HbkAvailabilityFilter::unfiltered(), None)
    }

    pub fn members_for_availability(
        &self,
        owner: HbkPlatformTypeId,
        contexts: &[AvailabilityContext],
        mode: HbkAvailabilityFilterMode,
        kind: Option<HbkTypeMemberKind>,
    ) -> impl Iterator<Item = (HbkTypeMemberId, HbkTypeMemberView<'_>)> + '_ {
        self.filtered_members(owner, availability_filter(contexts, mode), kind)
    }

    fn filtered_members(
        &self,
        owner: HbkPlatformTypeId,
        filter: HbkAvailabilityFilter,
        kind: Option<HbkTypeMemberKind>,
    ) -> impl Iterator<Item = (HbkTypeMemberId, HbkTypeMemberView<'_>)> + '_ {
        let handle = self.read_handle();
        handle
            .filtered_members(owner, filter, kind)
            .map(move |id| (id, handle.type_member(id)))
    }

    pub fn member_by_name<'a>(
        &'a self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> impl Iterator<Item = (HbkTypeMemberId, HbkTypeMemberView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .member_by_owner_name(owner, name)
            .map(move |id| (id, handle.type_member(id)))
    }

    pub fn member_by_name_kind<'a>(
        &'a self,
        owner: HbkPlatformTypeId,
        name: &str,
        kind: Option<HbkTypeMemberKind>,
    ) -> impl Iterator<Item = (HbkTypeMemberId, HbkTypeMemberView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .member_by_owner_name_kind(owner, name, kind)
            .map(move |id| (id, handle.type_member(id)))
    }

    pub fn callables(
        &self,
        owner: HbkPlatformTypeId,
    ) -> impl ExactSizeIterator<Item = (HbkCallableId, HbkCallableView<'_>)> + '_ {
        let handle = self.read_handle();
        handle
            .callables_of_type(owner)
            .map(move |id| (id, handle.callable(id)))
    }

    pub fn callable_by_name<'a>(
        &'a self,
        owner: HbkPlatformTypeId,
        name: &str,
    ) -> impl Iterator<Item = (HbkCallableId, HbkCallableView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .callable_by_owner_name(owner, name)
            .map(move |id| (id, handle.callable(id)))
    }

    pub fn constructors(
        &self,
        owner: HbkPlatformTypeId,
    ) -> impl ExactSizeIterator<Item = (HbkCallableId, HbkCallableView<'_>)> + '_ {
        let handle = self.read_handle();
        handle
            .constructors_of_type(owner)
            .map(move |id| (id, handle.callable(id)))
    }

    pub fn global_properties(
        &self,
    ) -> impl Iterator<Item = (HbkGlobalFactId, HbkGlobalFactView<'_>)> + '_ {
        self.filtered_global_properties(HbkAvailabilityFilter::unfiltered())
    }

    pub fn global_properties_for_availability(
        &self,
        contexts: &[AvailabilityContext],
        mode: HbkAvailabilityFilterMode,
    ) -> impl Iterator<Item = (HbkGlobalFactId, HbkGlobalFactView<'_>)> + '_ {
        self.filtered_global_properties(availability_filter(contexts, mode))
    }

    fn filtered_global_properties(
        &self,
        filter: HbkAvailabilityFilter,
    ) -> impl Iterator<Item = (HbkGlobalFactId, HbkGlobalFactView<'_>)> + '_ {
        let handle = self.read_handle();
        handle
            .filtered_global_ids(filter, Some(HbkGlobalFactKind::Property))
            .filter_map(move |id| {
                let global = handle.global_fact(id);
                (global.domain() == HbkLanguageDomain::Bsl).then_some((id, global))
            })
    }

    pub fn global_property_by_name<'a>(
        &'a self,
        name: &str,
    ) -> impl Iterator<Item = (HbkGlobalFactId, HbkGlobalFactView<'a>)> + 'a + use<'a> {
        let handle = self.read_handle();
        handle
            .globals_by_domain_name_kind(
                HbkLanguageDomain::Bsl,
                name,
                Some(HbkGlobalFactKind::Property),
            )
            .map(move |id| (id, handle.global_fact(id)))
    }

    pub fn global_methods(
        &self,
    ) -> impl Iterator<
        Item = (
            HbkGlobalFactId,
            HbkGlobalFactView<'_>,
            HbkCallableId,
            HbkCallableView<'_>,
        ),
    > + '_ {
        self.filtered_global_methods(HbkAvailabilityFilter::unfiltered())
    }

    pub fn global_methods_for_availability(
        &self,
        contexts: &[AvailabilityContext],
        mode: HbkAvailabilityFilterMode,
    ) -> impl Iterator<
        Item = (
            HbkGlobalFactId,
            HbkGlobalFactView<'_>,
            HbkCallableId,
            HbkCallableView<'_>,
        ),
    > + '_ {
        self.filtered_global_methods(availability_filter(contexts, mode))
    }

    fn filtered_global_methods(
        &self,
        filter: HbkAvailabilityFilter,
    ) -> impl Iterator<
        Item = (
            HbkGlobalFactId,
            HbkGlobalFactView<'_>,
            HbkCallableId,
            HbkCallableView<'_>,
        ),
    > + '_ {
        let handle = self.read_handle();
        handle
            .filtered_global_ids(filter, Some(HbkGlobalFactKind::Method))
            .filter_map(move |id| {
                let global = handle.global_fact(id);
                if global.domain() != HbkLanguageDomain::Bsl {
                    return None;
                }
                let callable = global.callable()?;
                Some((id, global, callable, handle.callable(callable)))
            })
    }

    pub fn global_method_by_name<'a>(
        &'a self,
        name: &str,
    ) -> impl Iterator<
        Item = (
            HbkGlobalFactId,
            HbkGlobalFactView<'a>,
            HbkCallableId,
            HbkCallableView<'a>,
        ),
    >
    + 'a
    + use<'a> {
        let handle = self.read_handle();
        handle
            .globals_by_domain_name_kind(
                HbkLanguageDomain::Bsl,
                name,
                Some(HbkGlobalFactKind::Method),
            )
            .filter_map(move |id| {
                let global = handle.global_fact(id);
                let callable = global.callable()?;
                Some((id, global, callable, handle.callable(callable)))
            })
    }

    pub fn module_context_events(
        &self,
        kind: ModuleContextKind,
    ) -> impl Iterator<Item = (HbkCallableId, HbkCallableView<'_>)> + '_ {
        bsl_module_context_key(kind)
            .into_iter()
            .flat_map(move |key| {
                let handle = self.read_handle();
                handle
                    .module_context_events(
                        HbkLanguageDomain::Bsl,
                        "bsl",
                        key.trim_start_matches("module_context:"),
                    )
                    .map(move |id| (id, handle.callable(id)))
            })
    }

    pub fn module_context_event_by_name<'a>(
        &'a self,
        kind: ModuleContextKind,
        name: &str,
    ) -> impl Iterator<Item = (HbkCallableId, HbkCallableView<'a>)> + 'a + use<'a> {
        let ids = bsl_module_context_key(kind)
            .map(|key| self.read_handle().module_event_by_context_name(key, name));
        let handle = self.read_handle();
        ids.into_iter()
            .flatten()
            .map(move |id| (id, handle.callable(id)))
    }

    pub fn platform_type_availability(
        &self,
        id: HbkPlatformTypeId,
    ) -> (impl Iterator<Item = AvailabilityContext> + '_, Option<&str>) {
        self.availability(HbkFactRef::PlatformType(id))
    }

    pub fn member_availability(
        &self,
        id: HbkTypeMemberId,
    ) -> (impl Iterator<Item = AvailabilityContext> + '_, Option<&str>) {
        self.availability(HbkFactRef::TypeMember(id))
    }

    pub fn callable_availability(
        &self,
        id: HbkCallableId,
    ) -> (impl Iterator<Item = AvailabilityContext> + '_, Option<&str>) {
        self.availability(HbkFactRef::Callable(id))
    }

    pub fn global_availability(
        &self,
        id: HbkGlobalFactId,
    ) -> (impl Iterator<Item = AvailabilityContext> + '_, Option<&str>) {
        self.availability(HbkFactRef::Global(id))
    }

    pub(crate) fn read_handle(&self) -> HbkFactReadHandle<'_> {
        self.snapshot.worker_handle()
    }

    pub(crate) fn availability(
        &self,
        fact: HbkFactRef,
    ) -> (impl Iterator<Item = AvailabilityContext> + '_, Option<&str>) {
        (
            self.read_handle()
                .availability_contexts(fact)
                .filter_map(|context| availability_context_from_code(self.string(context))),
            self.read_handle()
                .available_since(fact)
                .map(|since| self.string(since)),
        )
    }
}

fn availability_filter(
    contexts: &[AvailabilityContext],
    mode: HbkAvailabilityFilterMode,
) -> HbkAvailabilityFilter {
    let codes = contexts.iter().copied().map(availability_context_code);
    match mode {
        HbkAvailabilityFilterMode::Any => HbkAvailabilityFilter::any(codes),
        HbkAvailabilityFilterMode::All => HbkAvailabilityFilter::all(codes),
    }
    .expect("AvailabilityContext always maps to a supported HBK context code")
}

const fn availability_context_code(context: AvailabilityContext) -> &'static str {
    match context {
        AvailabilityContext::ThinClient => "thin_client",
        AvailabilityContext::WebClient => "web_client",
        AvailabilityContext::MobileClient => "mobile_client",
        AvailabilityContext::Server => "server",
        AvailabilityContext::ThickClient => "thick_client",
        AvailabilityContext::ExternalConnection => "external_connection",
        AvailabilityContext::MobileApplicationClient => "mobile_application_client",
        AvailabilityContext::MobileApplicationServer => "mobile_application_server",
        AvailabilityContext::MobileStandaloneServer => "mobile_standalone_server",
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
