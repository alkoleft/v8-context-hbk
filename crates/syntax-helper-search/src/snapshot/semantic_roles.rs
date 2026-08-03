use super::{
    HbkCallableKind, HbkCallableView, HbkGlobalFactKind, HbkGlobalFactView, HbkLanguageDomain,
    HbkNameView, HbkParameterView, HbkParameterViewIter, HbkPlatformTypeId, HbkPlatformTypeView,
    HbkSignatureView, HbkSignatureViewIter, HbkTypeMemberKind, HbkTypeMemberView, HbkTypeRefView,
    HbkTypeRefViewIter, StringId,
};
use v8_context_semantic_entities::{
    CallableKind, CallableView, ParameterPassing, ParameterRequirement, ParameterView,
    PropertyKind, PropertyView, SemanticOrigin, SemanticOwnerKind, SignatureView,
    TypeDeclarationView,
};

fn callable_kind(kind: HbkCallableKind) -> CallableKind {
    match kind {
        HbkCallableKind::Method | HbkCallableKind::GlobalMethod => CallableKind::Method,
        HbkCallableKind::Constructor => CallableKind::Constructor,
        HbkCallableKind::Event => CallableKind::Event,
        HbkCallableKind::LanguageFunction => CallableKind::Function,
    }
}

impl CallableView for HbkCallableView<'_> {
    type Owner<'a>
        = Option<HbkPlatformTypeId>
    where
        Self: 'a;
    type Signature<'a>
        = HbkSignatureView<'a>
    where
        Self: 'a;
    type Signatures<'a>
        = HbkSignatureViewIter<'a>
    where
        Self: 'a;

    fn name(&self) -> &str {
        (*self).primary_name_str()
    }

    fn origin(&self) -> SemanticOrigin {
        SemanticOrigin::Platform
    }

    fn owner_kind(&self) -> SemanticOwnerKind {
        if HbkCallableView::owner(*self).is_some() {
            SemanticOwnerKind::PlatformType
        } else {
            SemanticOwnerKind::GlobalContext
        }
    }

    fn owner(&self) -> Self::Owner<'_> {
        HbkCallableView::owner(*self)
    }

    fn callable_kind(&self) -> CallableKind {
        callable_kind((*self).kind())
    }

    fn signatures(&self) -> Self::Signatures<'_> {
        HbkCallableView::signatures(*self)
    }
}

impl SignatureView for HbkSignatureView<'_> {
    type Parameter<'a>
        = HbkParameterView<'a>
    where
        Self: 'a;
    type Parameters<'a>
        = HbkParameterViewIter<'a>
    where
        Self: 'a;
    type DeclaredType<'a>
        = HbkTypeRefView<'a>
    where
        Self: 'a;
    type DeclaredTypes<'a>
        = HbkTypeRefViewIter<'a>
    where
        Self: 'a;

    fn parameters(&self) -> Self::Parameters<'_> {
        HbkSignatureView::parameters(*self)
    }

    fn declared_result_types(&self) -> Self::DeclaredTypes<'_> {
        (*self).return_type_refs()
    }
}

impl ParameterView for HbkParameterView<'_> {
    type DeclaredType<'a>
        = HbkTypeRefView<'a>
    where
        Self: 'a;
    type DeclaredTypes<'a>
        = HbkTypeRefViewIter<'a>
    where
        Self: 'a;

    fn name(&self) -> &str {
        (*self).name_str()
    }

    fn requirement(&self) -> ParameterRequirement {
        if (*self).required() {
            ParameterRequirement::Required
        } else {
            ParameterRequirement::Optional
        }
    }

    fn passing(&self) -> ParameterPassing {
        ParameterPassing::SourceUnspecified
    }

    fn declared_types(&self) -> Self::DeclaredTypes<'_> {
        (*self).type_refs()
    }
}

/// A source-proved HBK property role over an existing borrowed source view.
#[derive(Clone, Copy)]
pub struct HbkPropertyView<'a> {
    inner: HbkPropertyViewInner<'a>,
}

#[derive(Clone, Copy)]
enum HbkPropertyViewInner<'a> {
    Member(HbkTypeMemberView<'a>),
    Global(HbkGlobalFactView<'a>),
}

impl<'a> HbkTypeMemberView<'a> {
    /// Returns a property role only when the member kind proves that semantic.
    pub fn property_role(self) -> Option<HbkPropertyView<'a>> {
        matches!(
            self.kind(),
            HbkTypeMemberKind::Property | HbkTypeMemberKind::EnumValue
        )
        .then_some(HbkPropertyView {
            inner: HbkPropertyViewInner::Member(self),
        })
    }
}

impl<'a> HbkGlobalFactView<'a> {
    /// Returns a property role only for a BSL global property fact.
    pub fn property_role(self) -> Option<HbkPropertyView<'a>> {
        (self.kind() == HbkGlobalFactKind::Property && self.domain() == HbkLanguageDomain::Bsl)
            .then_some(HbkPropertyView {
                inner: HbkPropertyViewInner::Global(self),
            })
    }
}

impl PropertyView for HbkPropertyView<'_> {
    type Owner<'a>
        = Option<HbkPlatformTypeId>
    where
        Self: 'a;
    type DeclaredType<'a>
        = HbkTypeRefView<'a>
    where
        Self: 'a;
    type DeclaredTypes<'a>
        = HbkTypeRefViewIter<'a>
    where
        Self: 'a;

    fn name(&self) -> &str {
        match self.inner {
            HbkPropertyViewInner::Member(member) => member.primary_name_str(),
            HbkPropertyViewInner::Global(global) => global.primary_name_str(),
        }
    }

    fn origin(&self) -> SemanticOrigin {
        SemanticOrigin::Platform
    }

    fn owner_kind(&self) -> SemanticOwnerKind {
        match self.inner {
            HbkPropertyViewInner::Member(_) => SemanticOwnerKind::PlatformType,
            HbkPropertyViewInner::Global(_) => SemanticOwnerKind::GlobalContext,
        }
    }

    fn owner(&self) -> Self::Owner<'_> {
        match self.inner {
            HbkPropertyViewInner::Member(member) => Some(member.owner()),
            HbkPropertyViewInner::Global(_) => None,
        }
    }

    fn property_kind(&self) -> PropertyKind {
        match self.inner {
            HbkPropertyViewInner::Member(member) => match member.kind() {
                HbkTypeMemberKind::Property => PropertyKind::Property,
                HbkTypeMemberKind::EnumValue => PropertyKind::EnumValue,
                HbkTypeMemberKind::Method | HbkTypeMemberKind::Event => {
                    unreachable!("HbkPropertyView filters non-property members")
                }
            },
            HbkPropertyViewInner::Global(_) => PropertyKind::Property,
        }
    }

    fn declared_types(&self) -> Self::DeclaredTypes<'_> {
        match self.inner {
            HbkPropertyViewInner::Member(member) => member.type_refs(),
            HbkPropertyViewInner::Global(global) => global.type_refs(),
        }
    }
}

impl TypeDeclarationView for HbkPlatformTypeView<'_> {
    type Name<'a>
        = HbkNameView<'a>
    where
        Self: 'a;
    type Owner<'a>
        = StringId
    where
        Self: 'a;

    fn name(&self) -> Self::Name<'_> {
        HbkPlatformTypeView::name(*self)
    }

    fn origin(&self) -> SemanticOrigin {
        SemanticOrigin::Platform
    }

    fn owner_kind(&self) -> SemanticOwnerKind {
        SemanticOwnerKind::PlatformType
    }

    fn owner(&self) -> Self::Owner<'_> {
        (*self).id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callable_kind_mapping_uses_only_source_proved_semantics() {
        assert_eq!(callable_kind(HbkCallableKind::Method), CallableKind::Method);
        assert_eq!(
            callable_kind(HbkCallableKind::GlobalMethod),
            CallableKind::Method
        );
        assert_eq!(
            callable_kind(HbkCallableKind::Constructor),
            CallableKind::Constructor
        );
        assert_eq!(callable_kind(HbkCallableKind::Event), CallableKind::Event);
        assert_eq!(
            callable_kind(HbkCallableKind::LanguageFunction),
            CallableKind::Function
        );
    }

    #[test]
    fn exact_direct_role_matrix_compiles() {
        fn callable<T: CallableView>() {}
        fn signature<T: SignatureView>() {}
        fn parameter<T: ParameterView>() {}
        fn property<T: PropertyView>() {}
        fn type_declaration<T: TypeDeclarationView>() {}

        callable::<HbkCallableView<'_>>();
        signature::<HbkSignatureView<'_>>();
        parameter::<HbkParameterView<'_>>();
        property::<HbkPropertyView<'_>>();
        type_declaration::<HbkPlatformTypeView<'_>>();
    }
}
