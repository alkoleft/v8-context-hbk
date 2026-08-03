use std::path::Path;
use std::sync::Arc;

use context_resolver_core::{
    AvailabilityContext, AvailabilityFact, AvailabilityInfo, CallableId, CallableInfo,
    CallableKind, CallableLookup, ContextFact, ContextSource, FactDetails, FactId, FactKind,
    FactProvenance, FactRelation, GlobalContextLanguage, GlobalContextQuery, LanguageDomain,
    MemberId, MemberInfo, MemberKind, MemberQuery, MemberQueryKind, MetadataTemplateInfo,
    ModuleContextInfo, ModuleContextKind, ModuleContextMemberLookup, ModuleContextMembersLookup,
    ModuleContextQuery, Name, Parameter,
    PlatformTypeTemplateKey, QueryFieldInfo, QueryParameterInfo, QueryTableInfo, QueryTableRole,
    RelationKind, ResolveContext, ResolveError, ResolveResponse, ResolveStatus, ResolvedBslContextMember, ResolvedCallable,
    ResolvedGlobalContext, ResolvedMember, ResolvedModuleContext, ResolvedType, Signature,
    SourceCapabilities, SourceDescriptor, SourceId, TemplateParameterBinding, TypeId, TypeInfo,
    TypeLookup, TypeRef, TypeRefTarget, TypeTemplateBinding,
};
use syntax_helper_search::{
    HbkCallableId, HbkCallableKind, HbkCallableView, HbkEnumId, HbkEnumValueId,
    HbkFactReadHandle, HbkFactRef, HbkFactSnapshot, HbkGlobalFactId, HbkNameView, HbkPlatformTypeId,
    HbkPlatformTypeTemplateKey, HbkPlatformTypeView, HbkQueryFieldId, HbkQueryParameterId,
    HbkQueryTableId, HbkSignatureView, HbkTypeMemberId, HbkTypeMemberKind,
    HbkTypeRefTargetView, HbkTypeRefView, RelatedHit, SearchDocument,
    SearchDocumentKind, SearchError, SearchHit, SearchIndex, SearchSignature, SearchTypeRef,
    SearchTypeRefTarget,
};

const DEFAULT_SOURCE_ID: &str = "shcntx-platform";

pub struct PlatformSearchSource {
    source_id: SourceId,
    index: SearchIndex,
}

pub struct LanguageSearchSource {
    source_id: SourceId,
    domain: LanguageDomain,
    index: SearchIndex,
    query_table_templates: bool,
    platform_source_id: SourceId,
}

pub struct PlatformSnapshotSource {
    catalog: HbkBslContextCatalog,
}

pub struct QueryTableSnapshotSource {
    catalog: HbkSdblQueryCatalog,
}

fn signature_text_is_variadic(text: &str) -> bool {
    text.contains("...") || text.contains('…')
}

fn search_signature_is_variadic(signature: &SearchSignature) -> bool {
    signature_text_is_variadic(&signature.text) || structure_values_signature_is_variadic(signature)
}

fn structure_values_signature_is_variadic(signature: &SearchSignature) -> bool {
    signature.text == "Новый Структура(<Ключи>, <Значения>)"
        && signature.parameters.len() == 2
        && signature.parameters[0].name == "Ключи"
        && signature.parameters[1].name == "Значения"
}

fn search_source_failure(source_id: &SourceId, source: SearchError) -> ResolveError {
    ResolveError::SourceFailure {
        source_id: source_id.clone(),
        message: source.to_string(),
    }
}
