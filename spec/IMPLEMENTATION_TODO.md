# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history:

- [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md)
- [archive/completed-tasks-t13-t17-t19-t24.md](archive/completed-tasks-t13-t17-t19-t24.md)
- [archive/completed-tasks-t25-t34.md](archive/completed-tasks-t25-t34.md)
- [archive/implementation-todo-2026-05-04.md](archive/implementation-todo-2026-05-04.md)
- [archive/implementation-todo-2026-05-05.md](archive/implementation-todo-2026-05-05.md)
- [archive/completed-tasks-t41-t47.md](archive/completed-tasks-t41-t47.md)
- [archive/completed-tasks-t48-t56.md](archive/completed-tasks-t48-t56.md)
- [archive/completed-tasks-t57-t65-t68-t85.md](archive/completed-tasks-t57-t65-t68-t85.md)
- [archive/completed-tasks-t66-t67-t86-t90.md](archive/completed-tasks-t66-t67-t86-t90.md)
- [archive/completed-tasks-t91-t110.md](archive/completed-tasks-t91-t110.md)
- [archive/completed-tasks-t111-t134.md](archive/completed-tasks-t111-t134.md)

Current status: T35-T134 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design,
language-domain, cleanup, book-content export, documentation-site and platform type-template
conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`requirements/non-functional.md`, `implementation/components.md`,
`implementation/documentation-site.md`, `implementation/syntax-helper-query-cli.md`,
`implementation/syntax-bsl-provider-plan.md`, `implementation/solution-context-resolve.md` and
`decisions/`.

Current first unchecked task: T142.

## Loop Rule

- Take the first unchecked task.
- If there is no unchecked task, add one before implementing new scope.
- Every new task must reference the relevant requirement, UAT, acceptance, implementation spec or
  ADR IDs from `spec/`.
- Keep scope limited to that task and its direct verification.
- Mark a task complete only after its listed verification passes.
- If a task cannot be completed, leave it unchecked and record the blocker in the task notes or final
  response.
- Do not start the next task in the same run unless the prompt explicitly asks for multiple tasks.
- Before committing, stage only files changed for the current task and verify
  `git diff --cached --name-only`.
- Do not create empty commits.

## Active Tasks

### T135: Measure Syntax Assistant Type-Reference Gaps

References: FR-SH-002, FR-SH-003, FR-SH-PROVIDER-001, NFR-QUERY-001,
`implementation/syntax-helper-query-cli.md`, `implementation/solution-context-resolve.md`.

- [x] Before implementation, confirm that the type-improvement track is sufficiently covered by
      existing FR-SH/ADR-0008/ADR-0011 contracts or add the smallest required requirements/spec
      wording. T135 itself should not need a new ADR.
- [x] Add a reproducible measurement pass over the current real `shcntx_root.hbk` and
      `shcntx_ru.hbk` indexes that counts type-reference facts by source role: property/query
      field/query parameter type, callable parameter type, callable return type, constructor result
      type, extension/base reference and template binding.
- [x] Report resolved, unresolved and ambiguous references separately, without selecting hidden
      winners for duplicate type names.
- [x] Record the top unresolved and ambiguous names with enough context to choose the next parser,
      model or index task.
- [x] Promote durable conclusions into `spec/acceptance/baseline.md` and keep raw command output as
      service data only.

Verification:

- [x] The measurement command is deterministic on the target platform corpus.
- [x] `cargo fmt --all --check`
- [x] `cargo test --workspace`

### T136: Add Type Quality Gates to the Acceptance Baseline

References: T135, FR-SH-002, FR-SH-003, FR-SH-PROVIDER-001, NFR-DIAG-001,
`acceptance/baseline.md`, `acceptance/uat-test-cases.md`.

- [x] Reflect the accepted measurement outputs in `spec/acceptance/baseline.md`; add or update
      `spec/acceptance/uat-test-cases.md` only when a gate is externally observable through
      CLI/provider behavior. T136 should not need a new ADR.
- [x] Define acceptance baseline gates for type work: unresolved type-reference count, ambiguous
      type-reference count, classified metadata-template count, unclassified template diagnostics,
      template binding count and expression-chain provider scenario status.
- [x] Document which gates are strict regressions and which are tracked informational metrics until
      a later task tightens them.
- [x] Add or update UAT coverage only for externally observable CLI/provider behavior; keep
      implementation-only counters out of public JSON unless a requirement explicitly adds them.

Verification:

- [x] `spec/acceptance/baseline.md` contains the current gate values and update rule.
- [x] `rg -n "type-reference|template|quality gate|quality gates" spec/acceptance`

### T137: Specify Explicit Type Domain Separation

References: ADR-0008, FR-SH-SEARCH-002, `implementation/solution-context-resolve.md`,
`implementation/components.md`.

- [x] Reflect the domain-separation clarification in `requirements/functional.md` and
      `implementation/solution-context-resolve.md`. Update ADR-0008 only if the task changes the
      accepted resolver boundary instead of tightening its existing rules.
- [x] Tighten the spec for platform API, BSL language, query language, configuration metadata and
      source-code type domains so same display names never imply identity.
- [x] Define which existing HBK-backed facts remain platform provider facts and which must wait for
      a language-domain or downstream metadata/source-code provider.
- [x] Add resolver/provider expectations for ambiguity when callers omit source, domain, fact kind
      or owner identity.
- [x] Do not fold `shquery_*`, `shlang_*`, configuration or source-code facts into platform API
      identities without an explicit source-backed relation.

Verification:

- [x] `implementation/solution-context-resolve.md` and related requirements consistently describe
      the domain split.
- [x] `cargo fmt --all --check` if code examples or Rust snippets are touched.

### T138: Decide the Separate Type Crate Boundary

References: ADR-0008, ADR-0011, `implementation/components.md`,
`implementation/solution-context-resolve.md`.

Decision: defer a separate type crate for now. The smallest current ownership boundaries are
recorded in `implementation/components.md`; no code movement or new ADR is required.

- [x] If a separate type crate is selected, add an ADR or accepted decision because this changes the
      workspace architecture boundary. If it is rejected or deferred, record the decision in
      `implementation/components.md` without adding a new ADR.
- [x] Decide whether type identities, type-reference resolution DTOs and template binding DTOs
      should move into a separate workspace crate or remain split between
      `syntax-helper-model`, `syntax-helper-search` and `context-resolver-core`.
- [x] If a separate crate is selected, specify its ownership, dependency rules and migration slices
      before moving code.
- [x] Keep HBK parsing, SQLite storage, CLI/provider JSON assembly and downstream analyzer logic out
      of the type crate boundary.
- [x] If a separate crate is rejected for now, record the reason and the smallest existing boundary
      that will own each type concept.

Verification:

- [x] `implementation/components.md` records the selected crate/boundary decision.
- [x] No code movement is performed before the boundary is specified.

### T139: Split Raw Type References from Resolved Type Targets

References: T135, T137, T138, FR-SH-002, FR-SH-PROVIDER-001,
`implementation/syntax-helper-query-cli.md`.

- [x] Reflect the selected raw/reference-vs-resolved-target contract in
      `requirements/functional.md` and `implementation/syntax-helper-query-cli.md` before changing
      code. Add an ADR only if the task changes the public provider/export boundary.
- [x] Model the source-backed type-reference spelling separately from resolved target identity.
- [x] Represent resolved targets as `ok`, `unresolved` or `ambiguous` data at the provider/resolver
      boundary, not as hidden first-match selection.
- [x] Preserve export-compatible `types` fields for current consumer JSON unless a schema task
      explicitly changes FR-EXPORT-001.
- [x] Update index/provider internals so `target_type_id`, ambiguous candidates and unresolved names
      have one owner and are not recomputed in multiple layers.

Verification:

- [x] Existing exact lookup, constructors, related and export UAT still pass.
- [x] New focused tests cover resolved, unresolved and ambiguous type-reference cases.
- [x] `cargo fmt --all --check`
- [x] `cargo test --workspace`

### T140: Move Return Types Toward Overload-Level Facts

References: FR-SH-002, FR-EXPORT-001, FR-SH-PROVIDER-001,
`implementation/syntax-helper-query-cli.md`.

- [x] Update the relevant requirements and implementation specs before implementation. If the task
      changes consumer JSON or provider JSON shape, update FR-EXPORT-001 / provider spec and record
      the schema/version impact explicitly before changing code.
- [x] Specify how callable return types attach to a concrete signature/overload when source
      evidence supports it.
- [x] Preserve page-level return facts only as explicit shared/inherited evidence when HBK does not
      prove an overload-specific return.
- [x] Report source pages that expose multiple return types for one modeled overload as
      parser/data-contract diagnostics instead of truncating.
- [x] Keep provider JSON compatible with current envelope semantics unless a schema-version task is
      explicitly added.

Verification:

- [x] Fixture tests cover at least one shared page-level return and one overload-specific return
      case if real source evidence exists.
- [x] `cargo fmt --all --check`
- [x] `cargo test --workspace`

### T141: Strengthen Platform Type Template Resolution

References: FR-SH-003, FR-SH-SEARCH-002, T135, T136,
`implementation/solution-context-resolve.md`, `implementation/syntax-helper-query-cli.md`.

- [x] Before implementation, update `implementation/syntax-helper-query-cli.md`,
      `implementation/solution-context-resolve.md` and the T136 baseline-gate wording when the
      strengthened template behavior changes expected metrics or resolver/provider output.
- [x] Use the T135/T136 metrics to identify unclassified or weakly classified platform
      metadata-template types before changing template logic.
- [x] Improve source-backed template family/variant classification without fallback-prefix
      families or localized-name heuristics.
- [x] Preserve owner-parameter bindings on member, callable return and parameter type references
      where HBK exposes template-to-template references.
- [x] Keep classification diagnostics visible in the acceptance baseline so future changes do not
      silently reduce template quality.

Verification:

- [x] Template classification metrics improve or remain explicitly justified in
      `acceptance/baseline.md`.
- [x] Tests cover manager-root classification, direct-reference classification, ambiguous family
      diagnostics and owner-parameter binding.
- [x] `cargo fmt --all --check`
- [x] `cargo test --workspace`

### T142: Add a Type Graph Query Primitive

References: ADR-0007, ADR-0008, FR-SH-PROVIDER-001, FR-SH-SEARCH-001, FR-SH-SEARCH-002,
`implementation/syntax-helper-query-cli.md`, `acceptance/uat-test-cases.md`.

- [ ] Specify the primitive in `implementation/syntax-helper-query-cli.md` and add UAT coverage
      before implementation. Add an ADR only if the selected solution introduces a new top-level
      command, provider boundary or transport instead of staying under the existing
      `syntax get` / `syntax related` family.
- [ ] Specify a bounded provider primitive for a compact type graph rooted at an exact type id,
      owner/member id or callable id.
- [ ] Keep the public CLI under the existing `syntax get` / `syntax related` command family unless
      the spec explicitly approves a new top-level command.
- [ ] Return constructors, members, callable overloads, parameter type refs, return type refs,
      template bindings and unresolved/ambiguous type-reference diagnostics in a deterministic
      graph-oriented shape.
- [ ] Add UAT for a real expression-chain workflow that benefits from one graph query instead of
      many ad hoc related calls.

Verification:

- [ ] Provider JSON uses one versioned envelope and keeps graph metadata outside fact fields.
- [ ] The graph query meets the current NFR-QUERY-001 target on the accepted corpus or records a
      measured blocker.
- [ ] `cargo fmt --all --check`
- [ ] `cargo test --workspace`
