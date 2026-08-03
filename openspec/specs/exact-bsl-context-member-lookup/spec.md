# exact-bsl-context-member-lookup Specification

## Purpose
Define exact source-qualified lookup of one metadata-module BSL member while
preserving provider evidence and ambiguity semantics.
## Requirements
### Requirement: HBK resolves one metadata-module BSL member exactly

HBK SHALL expose a source-qualified lookup accepting exactly one source,
optional matching domain, an opaque certified metadata module-role selector,
canonical BSL name, and `MemberQueryKind`, and
SHALL return one HBK-owned member answer or the existing resolver status/error
outcome without returning a complete module context.

#### Scenario: Exact metadata-module member exists
- **WHEN** an eligible platform source contains the requested member for the
  selector's module relation
- **THEN** the lookup returns its fact/callable identity, kind, source evidence
  and callable signatures where applicable
- **AND** it does not construct or filter `ResolvedModuleContext`

#### Scenario: Metadata selector or source has no exact answer
- **WHEN** the selector is unknown, the required source/domain is absent or
  mismatched, or the requested
  canonical primary name and kind do not exist (including an alias-only event
  spelling)
- **THEN** the lookup returns `NotFound`
- **AND** it does not try another role, name spelling or analyzer fallback

#### Scenario: Metadata-module point lookup cannot select uniquely
- **WHEN** an eligible source reports duplicate exact members, lacks the
  required capability, or fails
- **THEN** the lookup preserves `Ambiguous`, `Unsupported`, or the resolver
  error respectively
- **AND** no lower source is queried as fallback

#### Scenario: Role and kind choose one source-owned exact path
- **WHEN** the selector is `object`, `manager` or `form`
- **THEN** property and method queries use the exact platform-global index and
  event queries use the exact `(module-context, canonical-name)` event index
- **AND** `common`, `command` and `record-set` return terminal `NotFound`
- **AND** this does not classify or expose metadata form facts

### Requirement: Exact BSL member answer preserves existing evidence

The HBK result for exact BSL context lookup SHALL use one HBK-owned answer
form over existing property fact or callable evidence and SHALL preserve member
kind, stable fact identity, source evidence and callable signatures without an
analyzer-specific mirror.

#### Scenario: Method or event is selected
- **WHEN** the requested kind is method or event
- **THEN** the returned answer retains the existing `ResolvedCallable`
  signature and return evidence
- **AND** callers do not reconstruct it from a plain context fact
