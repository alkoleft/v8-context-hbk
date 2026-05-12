use std::path::Path;

use context_resolver_core::{
    AvailabilityContext, AvailabilityFact, AvailabilityInfo, CallableId, CallableInfo,
    CallableKind, CallableLookup, ContextFact, ContextSource, FactDetails, FactId, FactKind,
    FactRelation, GlobalContextLanguage, GlobalContextQuery, LanguageDomain, MemberId, MemberInfo,
    MemberKind, MemberQuery, MemberQueryKind, MetadataTemplateInfo, ModuleContextInfo,
    ModuleContextKind, ModuleContextQuery, Name, Parameter, PlatformTypeTemplateKey, RelationKind,
    ResolveContext, ResolveError, ResolveResponse, ResolveStatus, ResolvedCallable,
    ResolvedGlobalContext, ResolvedMember, ResolvedModuleContext, ResolvedType, Signature,
    SourceCapabilities, SourceDescriptor, SourceId, TemplateParameterBinding, TypeId, TypeInfo,
    TypeLookup, TypeRef, TypeRefTarget, TypeTemplateBinding,
};
use syntax_helper_search::{
    RelatedHit, SearchDocument, SearchDocumentKind, SearchError, SearchHit, SearchIndex,
    SearchTypeRef, SearchTypeRefTarget,
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
}

fn search_source_failure(source_id: &SourceId, source: SearchError) -> ResolveError {
    ResolveError::SourceFailure {
        source_id: source_id.clone(),
        message: source.to_string(),
    }
}
