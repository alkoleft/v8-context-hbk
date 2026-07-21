## 1. Contract

- [x] 1.1 Record the source-neutral classification contract: the opaque
  form-attribute and provider-certified generated-member-property selectors map
  to `MemberQueryKind::Property`; every other selector is normal absence. The
  classifier does not replace either provider's evidence-bearing answer.

## 2. Implementation

- [x] 2.1 Extend the public core classifier with
  `metadata.generated-member.property` and keep both accepted selector literals
  in its private mapping. Do not add a resolver method, source adapter mapping,
  capability, source/domain parameter, fact/DTO, template key or index.
  - Reintroduction guard: the only generated-member flow is `provider member
    evidence -> opaque selector -> core MemberQueryKind classifier`; a focused
    regression must reject source-adapter mappings, resolver answers and any
    generated-member identity/evidence mirror in HBK.

## 3. Verification

- [x] 3.1 Extend public classifier and structural-owner tests for the accepted
  generated-member selector, rejected unknown/generated selectors and absence
  from source adapters.
- [ ] 3.2 Run focused/core/search tests, formatting, Clippy and strict OpenSpec
  validation.
  - 2026-07-20: focused/core/search tests, formatting and strict validation
    pass. `cargo clippy -p context-resolver-search --no-deps -- -D warnings`
    remains blocked by 13 pre-existing `clippy::useless_conversion` findings in
    `snapshot_adapter.rs`; this change adds no search code or lint findings.
  - 2026-07-21: core and search tests, formatting and strict validation pass
    after the generated-member-property corpus extension. The same 13
    pre-existing `snapshot_adapter.rs` Clippy findings remain the only blocker;
    this diff changes no search adapter.
  - 2026-07-21 downstream handoff for `v8-context` task 3.9: HBK now exposes
    source-owned metadata-module member enumeration through
    `ContextResolver::metadata_module_members` and
    `ContextSource::module_context_members`. SQL and snapshot adapters enumerate
    global properties, global methods and module events from their owned indexes
    without materializing `ResolvedModuleContext`; focused core/search tests and
    the module-context structural guard cover the flow. This is a resolver
    prerequisite for analyzer enumeration, not a new metadata-generated-member
    classifier mapping.
