## 1. Contract

- [x] 1.1 Record the source-neutral classification contract: only the opaque
  form-attribute selector maps to `MemberQueryKind::Property`; every other
  selector is normal absence; generated members remain on the existing
  generated-self template/member path.

## 2. Implementation

- [x] 2.1 Add the public core classifier and keep the sole selector literal in
  its private mapping. Do not add a resolver method, source adapter mapping,
  capability, source/domain parameter, fact/DTO, template key or index.

## 3. Verification

- [x] 3.1 Add public classifier and structural-owner tests for accepted and
  rejected selectors plus absence from source adapters.
- [ ] 3.2 Run focused/core/search tests, formatting, Clippy and strict OpenSpec
  validation.
  - 2026-07-20: focused/core/search tests, formatting and strict validation
    pass. `cargo clippy -p context-resolver-search --no-deps -- -D warnings`
    remains blocked by 13 pre-existing `clippy::useless_conversion` findings in
    `snapshot_adapter.rs`; this change adds no search code or lint findings.
