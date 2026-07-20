## Decision

The public resolver receives a borrowed opaque module-role selector plus
existing `SourceId`/`LanguageDomain` filters. HBK owns its interpretation to
existing `ModuleContextKind` facts. The caller cannot supply a module kind for
this bridge. It must not reuse `GeneratedSelfTemplate`, because a generated
self type/template relation is not a module-context semantic relation.

## Initial companion corpus

The first module-context corpus is limited to the relations already requested
by the legacy BSL path and backed by HBK module contexts: `common`, `command`,
`object`, `manager`, `form` and `record_set`. Metadata module kinds outside
that corpus publish normal absence for this bridge until their source-backed
relation is separately accepted; they are not inferred by a spelling rule.

Metadata member-source selectors are deferred. HBK has not accepted a
role-to-BSL result/status matrix for attributes, register members or direct
form roles, and no current consumer can interpret one. The existing typed
metadata source evidence remains the only member surface until that separate
contract exists.

## HBK support/outcome matrix

| Metadata selector relation | Certified metadata selector | Current HBK evidence/outcome |
| --- | --- | --- |
| `object` | `metadata.module-role.object` | HBK module-context relation exists; resolve its existing facts. |
| `manager` | `metadata.module-role.manager` | HBK module-context relation exists; resolve its existing facts. |
| `form` | `metadata.module-role.form` | HBK module-context relation exists; resolve its existing facts. |
| `common` | `metadata.module-role.common` | No current HBK module-context relation; `NotFound` until provider evidence is added. |
| `command` | `metadata.module-role.command` | No current HBK module-context relation; `NotFound` until provider evidence is added. |
| `record_set` | `metadata.module-role.record-set` | No current HBK module-context relation; `NotFound` until provider evidence is added. |

Metadata selectors must be namespaced opaque literals and may be returned only
after a successful provider query has certified the concrete identity/fact.
Missing, malformed or deferred metadata input remains the provider's typed
error, not selector absence. A known metadata selector with no HBK fact is
`NotFound`; unavailable HBK capability is `Unsupported`.

## Status semantics

`ContextResolver::metadata_module_context` is a new public trait method with a
default `Unsupported` implementation. Only `CompositeResolver` and its
worker-safe wrapper override it; source-neutral external/test resolvers do not
need metadata-specific implementations.

The composite dispatch first restricts candidates to active sources matching
the requested optional `SourceId` and `LanguageDomain`. It passes the exact
selected source-ID set to existing module-context dispatch, so an omitted
source filter still cannot let a nonmatching active source participate. Every
matched source must expose `module_context`; one without that capability makes
the lookup `Unsupported` rather than masking it with another source's answer:

| Condition | Outcome |
| --- | --- |
| No active source matches the requested source/domain | `NotFound` |
| Any matched source lacks `module_context` capability | `Unsupported` |
| An eligible source receives `common`, `command` or `record_set` | `NotFound` (no current HBK relation) |
| An eligible source receives `object`, `manager` or `form` | Delegate its existing `ModuleContextQuery` exactly once and preserve `Ok`, `NotFound`, `Ambiguous`, `Unsupported` or `ResolveError` unchanged. |
| Selector is unknown | `NotFound` |

The composite does not return deliberate `NotFound` for one eligible source
while another selected eligible source can resolve or fail. No exact name,
alias or cross-source fallback is used.
