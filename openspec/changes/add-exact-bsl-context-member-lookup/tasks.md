## 1. Core Contract

- [x] 1.1 Add the provisional exact metadata-module BSL query input with one
  required source, optional matching domain and opaque selector, plus one
  HBK-owned answer form over existing fact/callable evidence; preserve resolver statuses.
- [x] 1.2 Add core resolver routing for opaque metadata selectors without
  constructing or filtering full context vectors.

## 2. Source Adapters

- [x] 2.1 Implement source-owned exact indexed lookups in search and snapshot
  platform adapters, including global property/method and direct
  `(module-context, canonical-name)` event paths, exact name/kind,
  source/domain isolation and errors; invalidate the existing derived snapshot
  cache through its layout version when index-key semantics change.

## 3. Verification and Documentation

- [x] 3.1 Add RED-first core/search/snapshot contract tests for exact answers,
  supported role×kind matrix, single-source isolation, ambiguity, unsupported
  and provider-error outcomes, SQL/snapshot parity for canonical primary event
  names (an alias is absence), plus structural no-vector guards. A cache written
  with the preceding snapshot layout version must rebuild before deserialization
  and the rebuilt snapshot must answer the exact event lookup.
- [x] 3.2 Update the HBK implementation/architecture contract, run focused tests,
  formatting and strict OpenSpec validation, then record the upstream handoff.

## Completion

- The exact resolver accepts one required platform source and returns existing
  property/callable evidence through `ResolvedBslContextMember`; opaque selector
  dispatch stays in the composite resolver.
- SQL and snapshot adapters use direct primary-name indexed operations. Exact
  events preserve ambiguity, aliases are terminal absence, and neither adapter
  materializes a module context.
- The existing snapshot cache advances to layout version 3 and rebuilds a
  previous layout before it can be interpreted as an owner-and-name event index.
- Verification passed: `cargo test -p context-resolver-core`, `cargo test -p
  syntax-helper-search`, `cargo test -p context-resolver-search`, `cargo fmt
  --all --check`, and `openspec validate add-exact-bsl-context-member-lookup
  --strict`.
