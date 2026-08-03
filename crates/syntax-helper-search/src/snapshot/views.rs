use super::x1_format::{
    X1CallableView, X1EnumValueView, X1EnumView, X1FactSourceView, X1GlobalFactView,
    X1LanguageFactView, X1MetadataTemplateView, X1NameHead, X1NameView, X1ParameterHead,
    X1ParameterView, X1PlatformTypeView, X1QueryFieldView, X1QueryParameterView, X1QueryTableView,
    X1RecordIter, X1SignatureHead, X1SignatureView, X1TemplateBindingView, X1TypeMemberView,
    X1TypeRefHead, X1TypeRefTargetKind, X1TypeRefView, X1ViewIter,
};
use super::*;

macro_rules! borrowed_or_mapped_iter {
    ($name:ident, $item:ty) => {
        pub struct $name<'a> {
            inner: BorrowedOrMappedIter<'a, $item>,
        }

        impl<'a> $name<'a> {
            pub(super) fn owned(values: &'a [$item]) -> Self {
                Self {
                    inner: BorrowedOrMappedIter::Owned(values.iter()),
                }
            }

            pub(super) fn mapped(values: X1RecordIter<'a, $item>) -> Self {
                Self {
                    inner: BorrowedOrMappedIter::Mapped(values),
                }
            }
        }

        impl Iterator for $name<'_> {
            type Item = $item;

            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next()
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let len = self.len();
                (len, Some(len))
            }
        }

        impl ExactSizeIterator for $name<'_> {
            fn len(&self) -> usize {
                self.inner.len()
            }
        }
    };
}

enum BorrowedOrMappedIter<'a, T> {
    Owned(std::slice::Iter<'a, T>),
    Mapped(X1RecordIter<'a, T>),
}

impl<T: Copy + super::codec::BinaryValue> Iterator for BorrowedOrMappedIter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Owned(values) => values.next().copied(),
            Self::Mapped(values) => values.next(),
        }
    }
}

impl<T: Copy + super::codec::BinaryValue> ExactSizeIterator for BorrowedOrMappedIter<'_, T> {
    fn len(&self) -> usize {
        match self {
            Self::Owned(values) => values.len(),
            Self::Mapped(values) => values.len(),
        }
    }
}

borrowed_or_mapped_iter!(HbkStringIdIter, StringId);
borrowed_or_mapped_iter!(HbkTypeMemberIdIter, HbkTypeMemberId);
borrowed_or_mapped_iter!(HbkCallableIdIter, HbkCallableId);
borrowed_or_mapped_iter!(HbkQueryFieldIdIter, HbkQueryFieldId);
borrowed_or_mapped_iter!(HbkQueryParameterIdIter, HbkQueryParameterId);
borrowed_or_mapped_iter!(HbkEnumValueIdIter, HbkEnumValueId);
borrowed_or_mapped_iter!(HbkFactRefIter, HbkFactRef);

#[derive(Clone, Copy)]
pub struct HbkNameView<'a> {
    inner: HbkNameViewInner<'a>,
}

#[derive(Clone, Copy)]
enum HbkNameViewInner<'a> {
    Owned(&'a HbkName),
    Mapped(X1NameView),
}

impl<'a> HbkNameView<'a> {
    pub(super) fn owned(value: &'a HbkName) -> Self {
        Self {
            inner: HbkNameViewInner::Owned(value),
        }
    }

    pub(super) fn mapped(value: X1NameView) -> Self {
        Self {
            inner: HbkNameViewInner::Mapped(value),
        }
    }

    pub fn primary(self) -> StringId {
        match self.inner {
            HbkNameViewInner::Owned(value) => value.primary,
            HbkNameViewInner::Mapped(value) => value.primary(),
        }
    }

    pub fn alias(self) -> Option<StringId> {
        match self.inner {
            HbkNameViewInner::Owned(value) => value.alias,
            HbkNameViewInner::Mapped(value) => value.alias(),
        }
    }
}

macro_rules! storage_view {
    ($public:ident, $inner:ident, $owned:ty, $mapped:ty) => {
        #[derive(Clone, Copy)]
        pub struct $public<'a> {
            inner: $inner<'a>,
        }

        #[derive(Clone, Copy)]
        enum $inner<'a> {
            Owned(&'a $owned),
            Mapped($mapped),
        }

        impl<'a> $public<'a> {
            pub(super) fn owned(value: &'a $owned) -> Self {
                Self {
                    inner: $inner::Owned(value),
                }
            }

            pub(super) fn mapped(value: $mapped) -> Self {
                Self {
                    inner: $inner::Mapped(value),
                }
            }
        }
    };
}

storage_view!(
    HbkPlatformTypeView,
    HbkPlatformTypeViewInner,
    HbkPlatformType,
    X1PlatformTypeView<'a>
);
storage_view!(
    HbkMetadataTemplateView,
    HbkMetadataTemplateViewInner,
    HbkMetadataTemplate,
    X1MetadataTemplateView<'a>
);
storage_view!(
    HbkTypeMemberView,
    HbkTypeMemberViewInner,
    HbkTypeMember,
    X1TypeMemberView<'a>
);
storage_view!(
    HbkCallableView,
    HbkCallableViewInner,
    HbkCallable,
    X1CallableView<'a>
);
storage_view!(
    HbkSignatureView,
    HbkSignatureViewInner,
    HbkSignature,
    X1SignatureView<'a>
);
storage_view!(
    HbkParameterView,
    HbkParameterViewInner,
    HbkParameter,
    X1ParameterView<'a>
);
storage_view!(
    HbkGlobalFactView,
    HbkGlobalFactViewInner,
    HbkGlobalFact,
    X1GlobalFactView<'a>
);
storage_view!(
    HbkQueryTableView,
    HbkQueryTableViewInner,
    HbkQueryTable,
    X1QueryTableView<'a>
);
storage_view!(
    HbkQueryFieldView,
    HbkQueryFieldViewInner,
    HbkQueryField,
    X1QueryFieldView<'a>
);
storage_view!(
    HbkQueryParameterView,
    HbkQueryParameterViewInner,
    HbkQueryParameter,
    X1QueryParameterView<'a>
);
storage_view!(
    HbkLanguageFactView,
    HbkLanguageFactViewInner,
    HbkLanguageFact,
    X1LanguageFactView<'a>
);
storage_view!(HbkEnumView, HbkEnumViewInner, HbkEnum, X1EnumView);
storage_view!(
    HbkEnumValueView,
    HbkEnumValueViewInner,
    HbkEnumValue,
    X1EnumValueView
);
storage_view!(
    HbkTypeRefView,
    HbkTypeRefViewInner,
    HbkTypeRef,
    X1TypeRefView<'a>
);
storage_view!(
    HbkTypeTemplateBindingView,
    HbkTypeTemplateBindingViewInner,
    HbkTypeTemplateBinding,
    X1TemplateBindingView<'a>
);

pub struct HbkNameViewIter<'a> {
    inner: HbkNameViewIterInner<'a>,
}

enum HbkNameViewIterInner<'a> {
    Owned(std::slice::Iter<'a, HbkName>),
    Mapped(X1ViewIter<'a, X1NameHead, X1NameView>),
}

impl<'a> Iterator for HbkNameViewIter<'a> {
    type Item = HbkNameView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            HbkNameViewIterInner::Owned(values) => values.next().map(HbkNameView::owned),
            HbkNameViewIterInner::Mapped(values) => values.next().map(HbkNameView::mapped),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for HbkNameViewIter<'_> {
    fn len(&self) -> usize {
        match &self.inner {
            HbkNameViewIterInner::Owned(values) => values.len(),
            HbkNameViewIterInner::Mapped(values) => values.len(),
        }
    }
}

pub struct HbkTypeRefViewIter<'a> {
    inner: HbkTypeRefViewIterInner<'a>,
}

enum HbkTypeRefViewIterInner<'a> {
    Owned(std::slice::Iter<'a, HbkTypeRef>),
    Mapped(X1ViewIter<'a, X1TypeRefHead, X1TypeRefView<'a>>),
}

impl<'a> Iterator for HbkTypeRefViewIter<'a> {
    type Item = HbkTypeRefView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            HbkTypeRefViewIterInner::Owned(values) => values.next().map(HbkTypeRefView::owned),
            HbkTypeRefViewIterInner::Mapped(values) => values.next().map(HbkTypeRefView::mapped),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for HbkTypeRefViewIter<'_> {
    fn len(&self) -> usize {
        match &self.inner {
            HbkTypeRefViewIterInner::Owned(values) => values.len(),
            HbkTypeRefViewIterInner::Mapped(values) => values.len(),
        }
    }
}

pub struct HbkSignatureViewIter<'a> {
    inner: HbkSignatureViewIterInner<'a>,
}

enum HbkSignatureViewIterInner<'a> {
    Owned(std::slice::Iter<'a, HbkSignature>),
    Mapped(X1ViewIter<'a, X1SignatureHead, X1SignatureView<'a>>),
}

impl<'a> Iterator for HbkSignatureViewIter<'a> {
    type Item = HbkSignatureView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            HbkSignatureViewIterInner::Owned(values) => values.next().map(HbkSignatureView::owned),
            HbkSignatureViewIterInner::Mapped(values) => {
                values.next().map(HbkSignatureView::mapped)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for HbkSignatureViewIter<'_> {
    fn len(&self) -> usize {
        match &self.inner {
            HbkSignatureViewIterInner::Owned(values) => values.len(),
            HbkSignatureViewIterInner::Mapped(values) => values.len(),
        }
    }
}

pub struct HbkParameterViewIter<'a> {
    inner: HbkParameterViewIterInner<'a>,
}

enum HbkParameterViewIterInner<'a> {
    Owned(std::slice::Iter<'a, HbkParameter>),
    Mapped(X1ViewIter<'a, X1ParameterHead, X1ParameterView<'a>>),
}

impl<'a> Iterator for HbkParameterViewIter<'a> {
    type Item = HbkParameterView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            HbkParameterViewIterInner::Owned(values) => values.next().map(HbkParameterView::owned),
            HbkParameterViewIterInner::Mapped(values) => {
                values.next().map(HbkParameterView::mapped)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for HbkParameterViewIter<'_> {
    fn len(&self) -> usize {
        match &self.inner {
            HbkParameterViewIterInner::Owned(values) => values.len(),
            HbkParameterViewIterInner::Mapped(values) => values.len(),
        }
    }
}

pub struct HbkTemplateArgumentIter<'a> {
    inner: HbkTemplateArgumentIterInner<'a>,
}

enum HbkTemplateArgumentIterInner<'a> {
    Owned(std::slice::Iter<'a, model::TemplateParameterBinding>),
    Mapped(X1RecordIter<'a, model::TemplateParameterBinding>),
}

impl Iterator for HbkTemplateArgumentIter<'_> {
    type Item = model::TemplateParameterBinding;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            HbkTemplateArgumentIterInner::Owned(values) => values.next().cloned(),
            HbkTemplateArgumentIterInner::Mapped(values) => values.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl ExactSizeIterator for HbkTemplateArgumentIter<'_> {
    fn len(&self) -> usize {
        match &self.inner {
            HbkTemplateArgumentIterInner::Owned(values) => values.len(),
            HbkTemplateArgumentIterInner::Mapped(values) => values.len(),
        }
    }
}

impl<'a> HbkPlatformTypeView<'a> {
    pub fn id(self) -> StringId {
        match self.inner {
            HbkPlatformTypeViewInner::Owned(v) => v.id,
            HbkPlatformTypeViewInner::Mapped(v) => v.id(),
        }
    }
    pub fn name(self) -> HbkNameView<'a> {
        match self.inner {
            HbkPlatformTypeViewInner::Owned(v) => HbkNameView::owned(&v.name),
            HbkPlatformTypeViewInner::Mapped(v) => HbkNameView::mapped(v.name()),
        }
    }
    pub fn metadata_template(self) -> Option<HbkMetadataTemplateView<'a>> {
        match self.inner {
            HbkPlatformTypeViewInner::Owned(v) => v
                .metadata_template
                .as_ref()
                .map(HbkMetadataTemplateView::owned),
            HbkPlatformTypeViewInner::Mapped(v) => {
                v.metadata_template().map(HbkMetadataTemplateView::mapped)
            }
        }
    }
    pub fn type_template_key(self) -> Option<HbkPlatformTypeTemplateKey> {
        match self.inner {
            HbkPlatformTypeViewInner::Owned(v) => v.type_template_key,
            HbkPlatformTypeViewInner::Mapped(v) => v.type_template_key(),
        }
    }
    pub fn availability_contexts(self) -> HbkStringIdIter<'a> {
        match self.inner {
            HbkPlatformTypeViewInner::Owned(v) => HbkStringIdIter::owned(&v.availability_contexts),
            HbkPlatformTypeViewInner::Mapped(v) => {
                HbkStringIdIter::mapped(v.availability_contexts())
            }
        }
    }
}

impl<'a> HbkMetadataTemplateView<'a> {
    pub fn metadata_kind(self) -> StringId {
        match self.inner {
            HbkMetadataTemplateViewInner::Owned(v) => v.metadata_kind,
            HbkMetadataTemplateViewInner::Mapped(v) => v.metadata_kind(),
        }
    }
    pub fn template_parameters(self) -> HbkStringIdIter<'a> {
        match self.inner {
            HbkMetadataTemplateViewInner::Owned(v) => {
                HbkStringIdIter::owned(&v.template_parameters)
            }
            HbkMetadataTemplateViewInner::Mapped(v) => {
                HbkStringIdIter::mapped(v.template_parameters())
            }
        }
    }
}

impl<'a> HbkTypeMemberView<'a> {
    pub fn id(self) -> StringId {
        match self.inner {
            HbkTypeMemberViewInner::Owned(v) => v.id,
            HbkTypeMemberViewInner::Mapped(v) => v.id(),
        }
    }
    pub fn owner(self) -> HbkPlatformTypeId {
        match self.inner {
            HbkTypeMemberViewInner::Owned(v) => v.owner,
            HbkTypeMemberViewInner::Mapped(v) => v.owner(),
        }
    }
    pub fn kind(self) -> HbkTypeMemberKind {
        match self.inner {
            HbkTypeMemberViewInner::Owned(v) => v.kind,
            HbkTypeMemberViewInner::Mapped(v) => v.kind(),
        }
    }
    pub fn name(self) -> HbkNameView<'a> {
        match self.inner {
            HbkTypeMemberViewInner::Owned(v) => HbkNameView::owned(&v.name),
            HbkTypeMemberViewInner::Mapped(v) => HbkNameView::mapped(v.name()),
        }
    }
    pub fn type_refs(self) -> HbkTypeRefViewIter<'a> {
        match self.inner {
            HbkTypeMemberViewInner::Owned(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Owned(v.type_refs.iter()),
            },
            HbkTypeMemberViewInner::Mapped(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Mapped(v.type_refs()),
            },
        }
    }
    pub fn availability_contexts(self) -> HbkStringIdIter<'a> {
        match self.inner {
            HbkTypeMemberViewInner::Owned(v) => HbkStringIdIter::owned(&v.availability_contexts),
            HbkTypeMemberViewInner::Mapped(v) => HbkStringIdIter::mapped(v.availability_contexts()),
        }
    }
}

impl<'a> HbkCallableView<'a> {
    pub fn id(self) -> StringId {
        match self.inner {
            HbkCallableViewInner::Owned(v) => v.id,
            HbkCallableViewInner::Mapped(v) => v.id(),
        }
    }
    pub fn owner(self) -> Option<HbkPlatformTypeId> {
        match self.inner {
            HbkCallableViewInner::Owned(v) => v.owner,
            HbkCallableViewInner::Mapped(v) => v.owner(),
        }
    }
    pub fn kind(self) -> HbkCallableKind {
        match self.inner {
            HbkCallableViewInner::Owned(v) => v.kind,
            HbkCallableViewInner::Mapped(v) => v.kind(),
        }
    }
    pub fn name(self) -> HbkNameView<'a> {
        match self.inner {
            HbkCallableViewInner::Owned(v) => HbkNameView::owned(&v.name),
            HbkCallableViewInner::Mapped(v) => HbkNameView::mapped(v.name()),
        }
    }
    pub fn signatures(self) -> HbkSignatureViewIter<'a> {
        match self.inner {
            HbkCallableViewInner::Owned(v) => HbkSignatureViewIter {
                inner: HbkSignatureViewIterInner::Owned(v.signatures.iter()),
            },
            HbkCallableViewInner::Mapped(v) => HbkSignatureViewIter {
                inner: HbkSignatureViewIterInner::Mapped(v.signatures()),
            },
        }
    }
    pub fn return_type_refs(self) -> HbkTypeRefViewIter<'a> {
        match self.inner {
            HbkCallableViewInner::Owned(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Owned(v.return_type_refs.iter()),
            },
            HbkCallableViewInner::Mapped(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Mapped(v.return_type_refs()),
            },
        }
    }
    pub fn availability_contexts(self) -> HbkStringIdIter<'a> {
        match self.inner {
            HbkCallableViewInner::Owned(v) => HbkStringIdIter::owned(&v.availability_contexts),
            HbkCallableViewInner::Mapped(v) => HbkStringIdIter::mapped(v.availability_contexts()),
        }
    }
}

impl<'a> HbkSignatureView<'a> {
    pub fn text(self) -> StringId {
        match self.inner {
            HbkSignatureViewInner::Owned(v) => v.text,
            HbkSignatureViewInner::Mapped(v) => v.text(),
        }
    }
    pub fn parameters(self) -> HbkParameterViewIter<'a> {
        match self.inner {
            HbkSignatureViewInner::Owned(v) => HbkParameterViewIter {
                inner: HbkParameterViewIterInner::Owned(v.parameters.iter()),
            },
            HbkSignatureViewInner::Mapped(v) => HbkParameterViewIter {
                inner: HbkParameterViewIterInner::Mapped(v.parameters()),
            },
        }
    }
    pub fn return_type_refs(self) -> HbkTypeRefViewIter<'a> {
        match self.inner {
            HbkSignatureViewInner::Owned(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Owned(v.return_type_refs.iter()),
            },
            HbkSignatureViewInner::Mapped(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Mapped(v.return_type_refs()),
            },
        }
    }
}

impl<'a> HbkParameterView<'a> {
    pub fn name(self) -> StringId {
        match self.inner {
            HbkParameterViewInner::Owned(v) => v.name,
            HbkParameterViewInner::Mapped(v) => v.name(),
        }
    }
    pub fn required(self) -> bool {
        match self.inner {
            HbkParameterViewInner::Owned(v) => v.required,
            HbkParameterViewInner::Mapped(v) => v.required(),
        }
    }
    pub fn type_refs(self) -> HbkTypeRefViewIter<'a> {
        match self.inner {
            HbkParameterViewInner::Owned(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Owned(v.type_refs.iter()),
            },
            HbkParameterViewInner::Mapped(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Mapped(v.type_refs()),
            },
        }
    }
}

impl<'a> HbkGlobalFactView<'a> {
    pub fn id(self) -> StringId {
        match self.inner {
            HbkGlobalFactViewInner::Owned(v) => v.id,
            HbkGlobalFactViewInner::Mapped(v) => v.id(),
        }
    }
    pub fn kind(self) -> HbkGlobalFactKind {
        match self.inner {
            HbkGlobalFactViewInner::Owned(v) => v.kind,
            HbkGlobalFactViewInner::Mapped(v) => v.kind(),
        }
    }
    pub fn domain(self) -> HbkLanguageDomain {
        match self.inner {
            HbkGlobalFactViewInner::Owned(v) => v.domain,
            HbkGlobalFactViewInner::Mapped(v) => v.domain(),
        }
    }
    pub fn name(self) -> HbkNameView<'a> {
        match self.inner {
            HbkGlobalFactViewInner::Owned(v) => HbkNameView::owned(&v.name),
            HbkGlobalFactViewInner::Mapped(v) => HbkNameView::mapped(v.name()),
        }
    }
    pub fn callable(self) -> Option<HbkCallableId> {
        match self.inner {
            HbkGlobalFactViewInner::Owned(v) => v.callable,
            HbkGlobalFactViewInner::Mapped(v) => v.callable(),
        }
    }
    pub fn type_refs(self) -> HbkTypeRefViewIter<'a> {
        match self.inner {
            HbkGlobalFactViewInner::Owned(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Owned(v.type_refs.iter()),
            },
            HbkGlobalFactViewInner::Mapped(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Mapped(v.type_refs()),
            },
        }
    }
}

impl<'a> HbkQueryTableView<'a> {
    pub fn id(self) -> StringId {
        match self.inner {
            HbkQueryTableViewInner::Owned(v) => v.id,
            HbkQueryTableViewInner::Mapped(v) => v.id(),
        }
    }
    pub fn name(self) -> HbkNameView<'a> {
        match self.inner {
            HbkQueryTableViewInner::Owned(v) => HbkNameView::owned(&v.name),
            HbkQueryTableViewInner::Mapped(v) => HbkNameView::mapped(v.name()),
        }
    }
    pub fn syntax(self) -> Option<HbkNameView<'a>> {
        match self.inner {
            HbkQueryTableViewInner::Owned(v) => v.syntax.as_ref().map(HbkNameView::owned),
            HbkQueryTableViewInner::Mapped(v) => v.syntax().map(HbkNameView::mapped),
        }
    }
    pub fn identifier(self) -> Option<StringId> {
        match self.inner {
            HbkQueryTableViewInner::Owned(v) => v.identifier,
            HbkQueryTableViewInner::Mapped(v) => v.identifier(),
        }
    }
    pub fn role(self) -> Option<model::QueryTableRole> {
        match self.inner {
            HbkQueryTableViewInner::Owned(v) => v.role,
            HbkQueryTableViewInner::Mapped(v) => v.role(),
        }
    }
    pub fn owner_path(self) -> HbkNameViewIter<'a> {
        match self.inner {
            HbkQueryTableViewInner::Owned(v) => HbkNameViewIter {
                inner: HbkNameViewIterInner::Owned(v.owner_path.iter()),
            },
            HbkQueryTableViewInner::Mapped(v) => HbkNameViewIter {
                inner: HbkNameViewIterInner::Mapped(v.owner_path()),
            },
        }
    }
    pub fn template_parameters(self) -> HbkStringIdIter<'a> {
        match self.inner {
            HbkQueryTableViewInner::Owned(v) => HbkStringIdIter::owned(&v.template_parameters),
            HbkQueryTableViewInner::Mapped(v) => HbkStringIdIter::mapped(v.template_parameters()),
        }
    }
}

impl<'a> HbkQueryFieldView<'a> {
    pub fn id(self) -> StringId {
        match self.inner {
            HbkQueryFieldViewInner::Owned(v) => v.id,
            HbkQueryFieldViewInner::Mapped(v) => v.id(),
        }
    }
    pub fn owner(self) -> HbkQueryTableId {
        match self.inner {
            HbkQueryFieldViewInner::Owned(v) => v.owner,
            HbkQueryFieldViewInner::Mapped(v) => v.owner(),
        }
    }
    pub fn name(self) -> HbkNameView<'a> {
        match self.inner {
            HbkQueryFieldViewInner::Owned(v) => HbkNameView::owned(&v.name),
            HbkQueryFieldViewInner::Mapped(v) => HbkNameView::mapped(v.name()),
        }
    }
    pub fn type_refs(self) -> HbkTypeRefViewIter<'a> {
        match self.inner {
            HbkQueryFieldViewInner::Owned(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Owned(v.type_refs.iter()),
            },
            HbkQueryFieldViewInner::Mapped(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Mapped(v.type_refs()),
            },
        }
    }
    pub fn note(self) -> Option<StringId> {
        match self.inner {
            HbkQueryFieldViewInner::Owned(v) => v.note,
            HbkQueryFieldViewInner::Mapped(v) => v.note(),
        }
    }
}

impl<'a> HbkQueryParameterView<'a> {
    pub fn id(self) -> StringId {
        match self.inner {
            HbkQueryParameterViewInner::Owned(v) => v.id,
            HbkQueryParameterViewInner::Mapped(v) => v.id(),
        }
    }
    pub fn owner(self) -> HbkQueryTableId {
        match self.inner {
            HbkQueryParameterViewInner::Owned(v) => v.owner,
            HbkQueryParameterViewInner::Mapped(v) => v.owner(),
        }
    }
    pub fn name(self) -> HbkNameView<'a> {
        match self.inner {
            HbkQueryParameterViewInner::Owned(v) => HbkNameView::owned(&v.name),
            HbkQueryParameterViewInner::Mapped(v) => HbkNameView::mapped(v.name()),
        }
    }
    pub fn type_refs(self) -> HbkTypeRefViewIter<'a> {
        match self.inner {
            HbkQueryParameterViewInner::Owned(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Owned(v.type_refs.iter()),
            },
            HbkQueryParameterViewInner::Mapped(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Mapped(v.type_refs()),
            },
        }
    }
    pub fn default_value(self) -> Option<StringId> {
        match self.inner {
            HbkQueryParameterViewInner::Owned(v) => v.default_value,
            HbkQueryParameterViewInner::Mapped(v) => v.default_value(),
        }
    }
}

impl<'a> HbkLanguageFactView<'a> {
    pub fn id(self) -> StringId {
        match self.inner {
            HbkLanguageFactViewInner::Owned(v) => v.id,
            HbkLanguageFactViewInner::Mapped(v) => v.id(),
        }
    }
    pub fn kind(self) -> SearchDocumentKind {
        match self.inner {
            HbkLanguageFactViewInner::Owned(v) => v.kind,
            HbkLanguageFactViewInner::Mapped(v) => v.kind(),
        }
    }
    pub fn domain(self) -> HbkLanguageDomain {
        match self.inner {
            HbkLanguageFactViewInner::Owned(v) => v.domain,
            HbkLanguageFactViewInner::Mapped(v) => v.domain(),
        }
    }
    pub fn name(self) -> HbkNameView<'a> {
        match self.inner {
            HbkLanguageFactViewInner::Owned(v) => HbkNameView::owned(&v.name),
            HbkLanguageFactViewInner::Mapped(v) => HbkNameView::mapped(v.name()),
        }
    }
    pub fn signatures(self) -> HbkSignatureViewIter<'a> {
        match self.inner {
            HbkLanguageFactViewInner::Owned(v) => HbkSignatureViewIter {
                inner: HbkSignatureViewIterInner::Owned(v.signatures.iter()),
            },
            HbkLanguageFactViewInner::Mapped(v) => HbkSignatureViewIter {
                inner: HbkSignatureViewIterInner::Mapped(v.signatures()),
            },
        }
    }
    pub fn type_refs(self) -> HbkTypeRefViewIter<'a> {
        match self.inner {
            HbkLanguageFactViewInner::Owned(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Owned(v.type_refs.iter()),
            },
            HbkLanguageFactViewInner::Mapped(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Mapped(v.type_refs()),
            },
        }
    }
    pub fn return_type_refs(self) -> HbkTypeRefViewIter<'a> {
        match self.inner {
            HbkLanguageFactViewInner::Owned(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Owned(v.return_type_refs.iter()),
            },
            HbkLanguageFactViewInner::Mapped(v) => HbkTypeRefViewIter {
                inner: HbkTypeRefViewIterInner::Mapped(v.return_type_refs()),
            },
        }
    }
}

impl<'a> HbkEnumView<'a> {
    pub fn id(self) -> StringId {
        match self.inner {
            HbkEnumViewInner::Owned(v) => v.id,
            HbkEnumViewInner::Mapped(v) => v.id(),
        }
    }
    pub fn name(self) -> HbkNameView<'a> {
        match self.inner {
            HbkEnumViewInner::Owned(v) => HbkNameView::owned(&v.name),
            HbkEnumViewInner::Mapped(v) => HbkNameView::mapped(v.name()),
        }
    }
}

impl<'a> HbkEnumValueView<'a> {
    pub fn id(self) -> StringId {
        match self.inner {
            HbkEnumValueViewInner::Owned(v) => v.id,
            HbkEnumValueViewInner::Mapped(v) => v.id(),
        }
    }
    pub fn owner(self) -> HbkEnumId {
        match self.inner {
            HbkEnumValueViewInner::Owned(v) => v.owner,
            HbkEnumValueViewInner::Mapped(v) => v.owner(),
        }
    }
    pub fn name(self) -> HbkNameView<'a> {
        match self.inner {
            HbkEnumValueViewInner::Owned(v) => HbkNameView::owned(&v.name),
            HbkEnumValueViewInner::Mapped(v) => HbkNameView::mapped(v.name()),
        }
    }
}

pub enum HbkTypeRefTargetView<'a> {
    Ok(StringId),
    Unresolved,
    Ambiguous(HbkStringIdIter<'a>),
}

impl<'a> HbkTypeRefView<'a> {
    pub fn name(self) -> StringId {
        match self.inner {
            HbkTypeRefViewInner::Owned(v) => v.name,
            HbkTypeRefViewInner::Mapped(v) => v.name(),
        }
    }
    pub fn target(self) -> HbkTypeRefTargetView<'a> {
        match self.inner {
            HbkTypeRefViewInner::Owned(v) => match &v.target {
                HbkTypeRefTarget::Ok(id) => HbkTypeRefTargetView::Ok(*id),
                HbkTypeRefTarget::Unresolved => HbkTypeRefTargetView::Unresolved,
                HbkTypeRefTarget::Ambiguous(ids) => {
                    HbkTypeRefTargetView::Ambiguous(HbkStringIdIter::owned(ids))
                }
            },
            HbkTypeRefViewInner::Mapped(v) => match v.target_kind() {
                X1TypeRefTargetKind::Ok => {
                    HbkTypeRefTargetView::Ok(v.target_ok().expect("validated X1 target"))
                }
                X1TypeRefTargetKind::Unresolved => HbkTypeRefTargetView::Unresolved,
                X1TypeRefTargetKind::Ambiguous => {
                    HbkTypeRefTargetView::Ambiguous(HbkStringIdIter::mapped(v.ambiguous_targets()))
                }
            },
        }
    }
    pub fn type_template_key(self) -> Option<HbkPlatformTypeTemplateKey> {
        match self.inner {
            HbkTypeRefViewInner::Owned(v) => v.type_template_key,
            HbkTypeRefViewInner::Mapped(v) => v.type_template_key(),
        }
    }
    pub fn template_binding(self) -> Option<HbkTypeTemplateBindingView<'a>> {
        match self.inner {
            HbkTypeRefViewInner::Owned(v) => v
                .template_binding
                .as_ref()
                .map(HbkTypeTemplateBindingView::owned),
            HbkTypeRefViewInner::Mapped(v) => {
                v.template_binding().map(HbkTypeTemplateBindingView::mapped)
            }
        }
    }
}

impl<'a> HbkTypeTemplateBindingView<'a> {
    pub fn template_key(self) -> HbkPlatformTypeTemplateKey {
        match self.inner {
            HbkTypeTemplateBindingViewInner::Owned(v) => v.template_key,
            HbkTypeTemplateBindingViewInner::Mapped(v) => v.template_key(),
        }
    }
    pub fn arguments(self) -> HbkTemplateArgumentIter<'a> {
        match self.inner {
            HbkTypeTemplateBindingViewInner::Owned(v) => HbkTemplateArgumentIter {
                inner: HbkTemplateArgumentIterInner::Owned(v.arguments.iter()),
            },
            HbkTypeTemplateBindingViewInner::Mapped(v) => HbkTemplateArgumentIter {
                inner: HbkTemplateArgumentIterInner::Mapped(v.arguments()),
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct HbkFactSourceView<'a> {
    inner: HbkFactSourceViewInner<'a>,
}

#[derive(Clone, Copy)]
enum HbkFactSourceViewInner<'a> {
    Owned(&'a HbkFactSource),
    Mapped(X1FactSourceView),
}

impl<'a> HbkFactSourceView<'a> {
    pub(super) fn owned(value: &'a HbkFactSource) -> Self {
        Self {
            inner: HbkFactSourceViewInner::Owned(value),
        }
    }
    pub(super) fn mapped(value: X1FactSourceView) -> Self {
        Self {
            inner: HbkFactSourceViewInner::Mapped(value),
        }
    }
    pub fn hbk_path(self) -> StringId {
        match self.inner {
            HbkFactSourceViewInner::Owned(v) => v.hbk_path,
            HbkFactSourceViewInner::Mapped(v) => v.hbk_path(),
        }
    }
    pub fn locale(self) -> StringId {
        match self.inner {
            HbkFactSourceViewInner::Owned(v) => v.locale,
            HbkFactSourceViewInner::Mapped(v) => v.locale(),
        }
    }
    pub fn toc_path(self) -> Option<StringId> {
        match self.inner {
            HbkFactSourceViewInner::Owned(v) => v.toc_path,
            HbkFactSourceViewInner::Mapped(v) => v.toc_path(),
        }
    }
    pub fn html_path(self) -> StringId {
        match self.inner {
            HbkFactSourceViewInner::Owned(v) => v.html_path,
            HbkFactSourceViewInner::Mapped(v) => v.html_path(),
        }
    }
    pub fn page_title(self) -> StringId {
        match self.inner {
            HbkFactSourceViewInner::Owned(v) => v.page_title,
            HbkFactSourceViewInner::Mapped(v) => v.page_title(),
        }
    }
}
