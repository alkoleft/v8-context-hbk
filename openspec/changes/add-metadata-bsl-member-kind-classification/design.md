## Decision

`metadata_bsl_member_kind(&str) -> Option<MemberQueryKind>` is a public core
language classifier. It is deliberately source-neutral: it reads no HBK source
and therefore cannot truthfully claim a source ID, domain, capability,
provenance, ambiguity or resolver failure. Metadata source preparation occurs
before the selector exists and retains its own typed diagnostics.

The private literal match is the sole owner of the initial corpus:

| Metadata-owned selector | HBK classification |
| --- | --- |
| `metadata.form-member.attribute` | `MemberQueryKind::Property` |
| any other selector | `None` |

`None` is normal language-classification absence. It is not a provider error
and must not cause a lower-tier fallback once context-provider has selected a
metadata source failure or ambiguity through its own normal flow.

This is not a source adapter. Search and snapshot adapters remain selector
blind because no platform source is queried. The function does not fabricate a
`ResolvedMember`, `ContextFact` or selector-specific identity: metadata retains
the form-member identity/evidence, while context-provider later compares this
returned kind to its exact requested BSL kind and applies precedence.

## Reintroduction guard

The selector literal may occur only in the metadata-provider emitter and this
core classifier. `MetadataGeneratedMember` selectors, analyzer
`MetadataKind`-to-`MemberQueryKind` maps, resolver methods and source-adapter
maps are prohibited until a separate accepted contract defines a real
source-backed relation.

## Architecture impact

No architecture document update is needed: the change adds no crate or module
responsibility, dependency direction, fact/answer schema, transport boundary or
analysis profile. It exposes a narrow pure function over an existing core enum.
