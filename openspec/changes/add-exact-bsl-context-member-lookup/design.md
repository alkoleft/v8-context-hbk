## Context

The existing resolver can compose a metadata-selected module context, but its
public operation returns a complete `ResolvedModuleContext`. The analyzer needs
an exact member answer for one canonical name and kind; consuming that vector
downstream would make the analyzer a second materialization owner.

## Goals / Non-Goals

**Goals:**

- Provide an exact, source/domain-qualified metadata-module BSL member query
  over a source-owned index.
- Preserve the established resolver status/error model and fact/callable
  evidence in one HBK-owned result.
- Keep metadata role selectors opaque and interpreted only in HBK.

**Non-Goals:**

- Enumerating or caching effective contexts, changing metadata ownership,
  adding analyzer dependencies, or exposing storage/index implementation.
- BSL lexical/module-text or configuration-global correlation; those remain
  caller-provided/context-provider responsibilities.

## Decisions

### One exact query family, one HBK-owned answer

`context-resolver-core` will define an exact metadata-module BSL member lookup
input carrying one required `SourceId`, optional matching domain, canonical
name, `MemberQueryKind`, and the opaque metadata selector. It returns
`ResolveResponse<ResolvedBslContextMember>`. Requiring exactly one source
avoids hidden cross-provider aggregation: an absent/mismatched source is
`NotFound`, missing capability is `Unsupported`, and an exact duplicate inside
that source is `Ambiguous`.
The answer is an enum over existing `ContextFact` and `ResolvedCallable`, so
HBK retains the evidence and signature-bearing callable representation without
a downstream field-for-field bridge. The enum itself owns no selection,
normalization or storage.

An alternative of returning `ResolvedModuleContext` and filtering it was
rejected because it reintroduces collection traversal at every consumer.
Separate property/method/event APIs were rejected because the context consumer
would need a second kind-to-result dispatcher.

### Selector dispatch stays in HBK and point queries stay indexed

The metadata-module lookup reuses the existing opaque selector-to-module-role
dispatch in core, then asks selected source adapters for an exact indexed
member. Search and snapshot adapters must implement the direct operation; they
must not call or filter their full context methods.

The supported platform roles mirror the existing module-context capability:
`object`, `manager` and `form` may query platform properties/methods from the
global exact index and events from a direct `(module-context, canonical-name)`
index; `common`, `command` and `record-set` are terminal `NotFound` in this
slice. This does not make metadata form attributes, commands, elements or
events into HBK facts: the downstream form-context change remains deferred.
For a supported role, property/method asks the global exact index and event asks
the module-event exact index. Exact event matching uses the provider's
normalized primary name only; an alias is not a second spelling fallback. A
source has no cross-kind fallback.

The SQL adapter adds one direct intersection of its existing canonical-name and
`module_context:*` document-name keys. The snapshot keeps the same provider
index family but materializes/searches a sorted `(module-context,
canonical-name)` lookup. Neither path enumerates all module events or calls
`module_context`.

The existing derived snapshot binary cache records the snapshot layout version.
Because `module_event_names` changes from an owner-only list to an exact
owner-and-name index, this change increments that existing layout version and
rebuilds stale caches. It adds neither a second cache nor a new cache owner.

### Statuses are terminal source outcomes

`Ok`, `NotFound`, `Ambiguous`, `Unsupported`, and resolver errors retain their
existing meanings. Composite routing never falls back from an eligible
ambiguous/unsupported/error source to another source or a lower analyzer tier.

## Risks / Trade-offs

- [Adapter index gap] → add the narrow source-owned index/query at the adapter
  owner and test search and snapshot implementations separately.
- [Answer union grows] → constrain it to existing property fact and callable
  evidence; a new semantic member model requires a future accepted change.
- [Selector misuse] → structural tests keep selector literals and module-kind
  mapping out of analyzer and source adapters.
