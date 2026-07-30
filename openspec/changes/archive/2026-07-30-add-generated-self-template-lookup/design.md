## Context

`context-resolver-core` already resolves platform templates by the HBK-owned
`PlatformTypeTemplateKey`. A project context consumer cannot legitimately
construct that key: it receives a metadata-owned generated-self role and the
existing analyzer mapping from role to key is being removed. HBK's platform
records already preserve the classified template key, member index and normal
source/domain routing.

The companion metadata change publishes a stable opaque selector on its
existing `MetadataGeneratedRole`. This repository must not import that crate or
repeat its enum; it receives the borrowed selector at the resolver seam.

## Certified selector corpus

The companion metadata contract currently certifies these selectors. HBK owns
their interpretation into its existing classified template evidence, while the
selectors themselves stay borrowed strings at the public resolver seam:

- `metadata.generated-self.catalog-object`
- `metadata.generated-self.catalog-manager`
- `metadata.generated-self.document-object`
- `metadata.generated-self.document-manager`
- `metadata.generated-self.information-register-record-set`
- `metadata.generated-self.accumulation-register-record-set`
- `metadata.generated-self.accounting-register-record-set`
- `metadata.generated-self.calculation-register-record-set`
- `metadata.generated-self.chart-of-characteristic-types-object`
- `metadata.generated-self.chart-of-characteristic-types-manager`
- `metadata.generated-self.exchange-plan-object`
- `metadata.generated-self.exchange-plan-manager`
- `metadata.generated-self.business-process-object`
- `metadata.generated-self.business-process-manager`
- `metadata.generated-self.task-object`
- `metadata.generated-self.task-manager`
- `metadata.generated-self.chart-of-accounts-object`
- `metadata.generated-self.chart-of-accounts-manager`
- `metadata.generated-self.chart-of-calculation-types-object`
- `metadata.generated-self.chart-of-calculation-types-manager`

`context-resolver-search` tests retain this list as an intentionally independent
companion-contract fixture: it constructs deterministic classified HBK records
and asserts the public results of both SQL and snapshot adapters. It is not
production lookup code and must not be imported or reused by the HBK mapping;
the production mapping has one owner in the platform adapter implementation.

## Goals / Non-Goals

**Goals:**

- Resolve an opaque generated-self role selector to an existing, source-backed
  HBK platform template through the normal resolver response model.
- Preserve `SourceId`/`LanguageDomain` routing, provider identity, template
  evidence and infrastructure-error propagation.
- Keep the role-to-template interpretation beside HBK template classification
  and indexes.

**Non-Goals:**

- No metadata dependency, metadata parsing, new shared role enum, template-key
  re-export, SQLite contract, cache or analyzer DTO.
- No generated configuration type composition, member enumeration shortcut or
  change to generic template binding semantics.
- No fallback to exact-name/alias lookup, heuristic string conversion or
  analyzer role/template table.

## Decisions

1. Add `TypeLookup::GeneratedSelfTemplate { source, domain,
   generated_self_role }` to `context-resolver-core`. It is a narrow borrowed
   query shape, not a new model: the selector remains opaque and the existing
   `ResolveResponse<ResolvedType>` carries the answer.

2. `PlatformSnapshotSource` and the explicit SQL `PlatformSearchSource` handle
   the variant. Each first applies existing source/domain filters, then uses a
   provider-owned selector-to-template lookup over classified template records,
   and finally reuses existing type mapping. Other sources return the existing
   `Unsupported` result. This preserves the resolver's concrete adapter seam
   rather than forcing a metadata concept into all source implementations.

3. The selector is compared only to provider-owned classification evidence. A
   selected template is unique, otherwise the existing response semantics are
   retained: unknown selector/no template is `NotFound`; several templates are
   `Ambiguous`; a source without capability is `Unsupported`; storage/snapshot
   failures are `ResolveError`.

4. Tests use public resolver APIs and deterministic synthetic platform facts.
   They cover every metadata-certified selector supplied by the companion
   metadata contract, requested source/domain isolation, unknown, unsupported,
   ambiguity and resolver-error behavior. They do not assert a private index
   layout or mapping helper.

Alternatives rejected:

- Have context-provider construct `PlatformTypeTemplateKey`: recreates the
  forbidden analyzer mapping and leaks HBK selection details.
- Make HBK depend on metadata types: reverses independent provider boundaries
  and couples HBK extraction to metadata implementation.
- Use metadata-kind/module-kind display strings: they are not a stable protocol
  and would recreate selection rules outside the owning providers.

## Risks / Trade-offs

- [Selector contracts drift across providers] → companion metadata tests fix
  selector stability; HBK tests consume the exact documented selector corpus.
- [A new source silently returns absence] → default/other-source behavior is
  explicitly `Unsupported`, with focused tests.
- [Two templates classify to one selector] → return `Ambiguous`, never choose
  one by index/order.
- [Legacy mapping survives unnoticed] → downstream structural guard rejects
  its import/use in context-provider; its atomic removal remains separately
  tracked.

## Migration Plan

1. Land the metadata selector contract and this resolver capability with their
   focused tests.
2. Switch downstream context-provider to the new lookup without importing a
   template key or legacy role table.
3. Remove the legacy analyzer role/template path only in its recorded atomic
   consumer cutover; no compatibility fallback exists in this change.
