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
- [archive/completed-tasks-t135-t142.md](archive/completed-tasks-t135-t142.md)

Current status: T35-T142 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design,
language-domain, cleanup, book-content export, documentation-site, platform type-template and
type-reference conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`requirements/non-functional.md`, `implementation/components.md`,
`implementation/documentation-site.md`, `implementation/syntax-helper-query-cli.md`,
`implementation/syntax-bsl-provider-plan.md`, `implementation/solution-context-resolve.md` and
`decisions/`.

Current first unchecked task: T147.

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

### T143: Classify T135 Unresolved Type References by Source Domain

References: T135, T136, T137, T139, FR-SH-002, FR-SH-PROVIDER-001,
FR-CTX-RESOLVE-001, `source-evidence.md`, `implementation/solution-context-resolve.md`,
`implementation/syntax-helper-query-cli.md`.

- [x] Before implementation, compare the current T135 top unresolved names against existing
      `shlang_*`, `shquery_*` and `dcsui_*` language-domain facts from `source-evidence.md`;
      explicitly cover primitive/domain names such as `Строка` / `String`, `Булево` / `Boolean` and
      `Число` / `Number`.
- [x] Add or reuse a reproducible analysis path that partitions unresolved type-reference rows into
      at least: likely BSL-language facts, likely query-language/SKD facts, configuration/source-code
      facts that belong to downstream providers, and still-unclassified platform-source gaps.
- [x] Do not reduce unresolved counts by guessing a platform type. Cross-domain matches must stay
      source/domain-qualified and require explicit source-backed relations before they are treated as
      resolved.
- [x] Decide whether the result remains an acceptance analysis note or requires provider/resolver
      output changes. Update requirements, implementation specs or UAT before code if public behavior
      changes.
- [x] Keep T136 strict gate values unchanged unless the task changes the actual measured counters; if
      counters change, record old values, new values and source-backed rationale in
      `acceptance/baseline.md`.

Result: T143 remains an acceptance/source-evidence analysis note. The reproducible classifier is
`scripts/analysis/type-ref-domain-classification.sql` over a prebuilt schema-13 search index. Public
provider JSON, resolver DTOs, export JSON and T136 gate counters are unchanged.

Verification:

- [x] The classification report is reproducible from a prebuilt local index or checked-in fixtures and
      does not parse HBK books per query.
- [x] Durable conclusions are promoted to `acceptance/baseline.md` and/or `source-evidence.md`; raw
      reports remain service data under `target/`.
- [x] Not required: `cargo fmt --all --check` (no Rust code or Rust examples touched).
- [x] Not required: `cargo test --workspace` (no code touched).

### T144: Investigate RU Ambiguous Type-Reference Cases

References: T135, T136, FR-SH-003, FR-SH-SEARCH-002, FR-SH-PROVIDER-001,
`source-evidence.md`, `acceptance/baseline.md`.

- [x] Inspect source evidence for the current RU ambiguous type-reference names:
      `ЭлементыФормы`, `Настройка сервиса` and `НастройкаСервиса`.
- [x] For each ambiguous group, decide whether the candidates are distinct platform identities, aliases
      of one source fact, duplicated source documentation, or facts missing an owner/domain key.
- [x] Implement a source-backed disambiguation or merge rule only when the evidence proves it; otherwise
      keep the ambiguity explicit and record the blocker/follow-up.
- [x] Preserve the no-hidden-winner contract: plain-name ambiguity must not silently choose the first
      platform type or relation edge.
- [x] Update T136 gate values only if the measured ambiguous count changes; reducing ambiguity is
      acceptable when the rule is source-backed.

Result: T144 keeps all three inspected source groups as distinct platform identities.
`Настройка сервиса` / `НастройкаСервиса` ambiguity was caused by index-time whitespace-insensitive
type-reference lookup, not by duplicated source facts, so exact source spelling now disambiguates
those rows. `ЭлементыФормы` remains explicit ambiguity because both `Controls` and `FormItems`
share the same primary spelling; resolving it requires preserving the source HTML link target or an
equivalent parser-owned target identity, not owner-name or availability heuristics. RU T136
ambiguous gate changed from `13` to `5`; unresolved and template-binding gates are unchanged.

Verification:

- [x] Focused tests cover each changed ambiguous group or prove that it intentionally remains
      ambiguous.
- [x] Fresh `syntax type-ref-gaps --format json` runs are deterministic for the affected RU index.
- [x] `cargo fmt --all --check`
- [x] `cargo test --workspace`

### T145: Broaden Type-Graph Consumer UAT Scenarios

References: T142, UAT-SH-024, FR-SH-PROVIDER-001, FR-SH-SEARCH-001,
FR-SH-SEARCH-002, NFR-QUERY-001, `implementation/syntax-helper-query-cli.md`.

- [x] Select two or three additional real expression-chain workflows beyond the accepted SKD
      `НастройкиКомпоновкиДанных.Отбор` scenario.
- [x] For each workflow, define the exact graph root, expected reachable facts and why one bounded
      `syntax related --graph` call is enough for analyzer-style inference.
- [x] Add UAT coverage only for externally observable provider behavior; keep graph internals and
      SQLite schema private.
- [x] Preserve current unsupported combinations for graph mode unless a spec change explicitly expands
      the contract.
- [x] Record measured graph-query latency against the accepted corpus and keep it within
      NFR-QUERY-001 or document the measured blocker.

Result: T145 broadened UAT-SH-024 with three additional exact graph roots:
`type_method:platform_type:Запрос:Выполнить`,
`type_method:platform_type:HTTPСоединение:Получить` and
`type_method:platform_type:ДвоичныеДанные:ОткрытьПотокДляЧтения`. The existing type graph contract
was sufficient; no parser, SQLite schema, provider envelope, unsupported-combination or analyzer
implementation changes were made. Fresh `shcntx_ru.hbk` UAT measurements were recorded in
`acceptance/baseline.md`.

Verification:

- [x] New UAT cases pass against a fresh local index built from the accepted corpus.
- [x] Provider JSON keeps graph metadata under `results[].meta` or envelope diagnostics, not inside
      shared fact fields.
- [x] Not required: `cargo fmt --all --check` (no code or Rust examples touched).
- [x] Not required: `cargo test --workspace` (no code touched).

### T146: Add Global Context Resolver Scope and Fix Callable/Member Gaps

References: ADR-0008, FR-CTX-RESOLVE-001, FR-SH-PROVIDER-001,
`implementation/solution-context-resolve.md`, `implementation/syntax-helper-query-cli.md`.

- [x] Before implementation, finalize the `global_context` resolver contract in
      `implementation/solution-context-resolve.md`: analyzer setup must be able to retrieve the
      BSL and SDBL/query-language global scopes with globally visible methods/properties/facts.
- [x] Implement a first-class global-context lookup in `context-resolver-core` and the HBK-backed
      search adapter. Do not model global platform facts through a fake owner `TypeId`, and do not
      fold SDBL/query-language facts into the BSL/platform scope by matching display names.
- [x] Make platform global methods reachable through the resolver callable API when callers use an
      ownerless callable-name lookup in the BSL context; this must delegate to the same
      global-context-backed facts, not to a separate ad hoc search path.
- [x] Expose platform global properties through the global-context result shape or an explicit
      global-property point lookup chosen in the implementation spec before code.
- [x] Ensure type-event facts returned as members can be resolved back by the exact member id returned
      from the resolver. The fact kind used for listing and id lookup must be consistent.
- [x] Decide and implement the exact-member miss status: an exact named member lookup with no matching
      member should not silently look like a successful non-empty lookup. Preserve broad member-list
      behavior for owners that intentionally have zero members.
- [x] Keep source/domain-qualified identity intact; do not widen platform resolver lookup into
      language, query or downstream configuration/source-code domains.

Result: T146 added first-class Rust resolver `global_context` lookup for BSL and SDBL/query scopes,
implemented HBK-backed platform/language adapter support over prebuilt search indexes, made
ownerless platform callable lookup resolve only global methods, returned platform global properties
through the global-context property collection without a fake owner type, fixed type-event
member-list-to-id round trips through read-phase `owner_identity`, and changed exact named member
miss to `NotFound`. Ownerless platform callable lookup delegates through the BSL global-context
scope. CLI behavior, provider JSON, consumer export JSON and the private search-index schema version
are unchanged.

Verification:

- [x] Focused resolver tests cover retrieving BSL and SDBL global contexts separately, finding a
      known BSL-visible global method/property through the BSL result, keeping a known SDBL function
      in the query-language context, resolving a known platform global method by ownerless
      callable-name lookup, a type-event member-list-to-id round trip and an exact member miss.
- [x] Existing static-analysis consumer smoke still passes.
- [x] `cargo fmt --all --check`
- [x] `cargo test --workspace`

### T147: Fix Generated Documentation Viewer Link Navigation

References: ADR-0010, `implementation/documentation-site.md`, UAT-HBK-014, UAT-HBK-015.

- [x] Reproduce same-page generated Markdown fragment links in `hbk-doc-site` / `web/docs-viewer`
      using a deterministic fixture. A same-page `#fragment` link must not be rewritten into an
      unroutable `index.md#fragment` link in generated site output.
- [x] Fix generated page routing so intra-page fragments and cross-page generated links both navigate
      correctly in the viewer.
- [x] Preserve link safety rules: external HTTP(S), generated page links and safe relative anchors may
      be rendered, but arbitrary unsupported hrefs must not bypass the existing sanitizer.
- [x] Preserve human page titles when navigating through generated Markdown links instead of replacing
      the document title with an opaque generated page id.

Result: T147 fixed generated documentation viewer navigation without changing single-book
`markdown/toc` export behavior. `hbk-doc-site` generated page Markdown now collapses same-page
generated fragment links from `<current-page>.md#fragment` to safe `#fragment` anchors, while
cross-page links keep `page-*.md#fragment` for viewer routing. `web/docs-viewer` resolves same-page
fragments through the current page id, routes cross-page generated links through the page data
client and derives the document title from the rendered Markdown heading or human link/TOC title
instead of an opaque generated page id. The existing sanitizer boundary is preserved: generated page
links, same-page anchors and HTTP(S) links remain allowed; unsupported hrefs continue to render as
`#`.

Verification:

- [x] `web/docs-viewer` tests cover same-page fragments, cross-page generated links and title
      preservation.
- [x] `hbk-doc-site` or `hbk-book-export` tests cover the generated Markdown shape that feeds the
      viewer.
- [x] `npm test -- --test-reporter=tap` in `web/docs-viewer`
- [x] `cargo test -p hbk-doc-site -p hbk-book-export -p v8-context-hbk-cli`

### T148: Stabilize Provider JSON Assembly Boundaries

References: FR-SH-PROVIDER-001, FR-EXPORT-001, T142,
`implementation/syntax-helper-query-cli.md`, `implementation/components.md`.

- [x] Specify the smallest provider JSON assembly boundary for `syntax get`, `syntax related`,
      `syntax related --graph` and `syntax type-ref-gaps`. Keep CLI argument parsing separate from
      provider envelope/result shaping.
- [x] Render graph `template_binding` metadata explicitly instead of serializing internal model DTOs
      wholesale through `json!(...)`.
- [x] Preserve the current provider envelope and `schema_version` unless the spec explicitly approves
      a public provider JSON change.
- [x] Do not move SQLite schema details, search internals or downstream analyzer concepts into the CLI
      JSON layer.

Result: T148 kept the public provider JSON envelope at `schema_version: 1` and the canonical
consumer export at `schema_version: 11`. The provider assembly boundary is recorded in
`implementation/syntax-helper-query-cli.md` and `implementation/components.md`: command handlers
classify arguments, resolve the index, execute `SearchIndex` queries and dispatch text versus JSON,
while private CLI provider helpers assemble envelopes, `results[].fact`, `results[].meta` and
diagnostics. Existing graph metadata already rendered template bindings explicitly; focused tests
now pin the `ok`, `unresolved`, `ambiguous` and template-bound graph type-reference shapes and
assert that internal type-template DTO fields do not leak into provider JSON.

Verification:

- [x] Focused CLI/provider tests prove graph metadata shape for `ok`, `unresolved`, `ambiguous` and
      template-bound type references.
- [x] Existing `syntax get`, `syntax related`, `syntax related --graph` and `syntax type-ref-gaps`
      JSON tests continue to pass.
- [x] `cargo fmt --all --check`
- [x] `cargo test -p v8-context-hbk-cli`

### T149: Reduce Query-Table Page Re-Parsing During Syntax Extraction

References: NFR-PERF-001, NFR-QUERY-001, ADR-0005,
`implementation/performance-baseline-t13.md`, `implementation/components.md`.

- [ ] Measure query-table page load/parse calls in the current Syntax Assistant reader flow before
      changing behavior.
- [ ] Remove avoidable repeated loading/parsing between parent-identity discovery and record emission
      for query-table pages. Reuse source-backed parsed evidence where it belongs instead of adding a
      generic cache knob.
- [ ] Preserve existing no-fallback identity rules for query-table and query-table member facts.
- [ ] Record any measured runtime or memory effect in the acceptance/performance baseline only when
      the measurement is reproducible.

Verification:

- [ ] Focused tests or instrumentation prove the repeated query-table page load/parse path is reduced.
- [ ] Existing query-table identity and missing-syntax regression tests still pass.
- [ ] Representative `shcntx_ru.hbk` extraction/indexing measurement is recorded when the change is
      expected to affect runtime.
- [ ] `cargo fmt --all --check`
- [ ] `cargo test -p syntax-helper-extract -p syntax-helper-search`

### T150: Align UAT Shape and CLI Smoke Automation

References: `acceptance/test-case-requirements.md`, `acceptance/uat-test-cases.md`,
UAT-HBK-001, UAT-HBK-002, UAT-HBK-003, FR-CLI-001, NFR-DIAG-001.

- [ ] Reconcile the UAT case template with the current catalog: either add explicit pass/fail criteria
      to active UAT cases or intentionally narrow the template requirement before editing cases.
- [ ] Add an executable black-box smoke harness for `inspect`, `toc` and `page` that validates exit
      code and representative output shape against the accepted small real fixtures when available.
- [ ] Keep fixture absence as a recorded skip, not a passing assertion that hides missing platform
      coverage.
- [ ] Document whether real-HBK smoke is local-only or expected in CI; do not imply CI coverage unless
      fixture provisioning is specified.

Verification:

- [ ] Updated UAT cases comply with `acceptance/test-case-requirements.md`.
- [ ] The black-box smoke harness can be run locally and reports skipped real-HBK cases explicitly when
      fixtures are absent.
- [ ] `cargo fmt --all --check` if Rust code is touched.
- [ ] `cargo test --workspace` if the smoke harness is Rust-based.

### T151: Split Large Implementation Modules by Context Boundary

References: `implementation/components.md`, ADR-0004, ADR-0007, ADR-0008.

Decision: decompose large `src/lib.rs` files inside their current crates first. Do not introduce new
crates, change public API, change provider/export JSON, or move behavior across context boundaries in
this task.

- [ ] Before refactoring, verify `implementation/components.md` still records the intended internal
      module boundaries for every touched crate.
- [ ] Split `syntax-helper-search` internals by responsibility: public DTO/facade exports, index
      builder, SQLite schema/storage/lifecycle, read-only query methods, relation traversal,
      type-reference resolution/gap reports and type-template classification. Keep
      `syntax-helper-search` as the owner of the private SQLite artifact and search/index behavior.
- [ ] Split `context-resolver-search` into platform adapter, BSL/query-language adapter,
      global-context adapter support and shared mapping helpers. This split should prepare T146
      without implementing new resolver behavior unless that task is active.
- [ ] Split `syntax-helper-language` into shared model/parser helpers plus `shlang`, `shquery` and
      `dcsui` parser modules. Preserve the BSL vs SDBL/query context boundary.
- [ ] Split `hbk-book-export` into request/planning, raw export, Markdown rendering, link rewriting,
      HTML/code normalization, filesystem write and error modules.
- [ ] Split `hbk-doc-site` into source discovery/loading, site-data/TOC merge, page/link planning,
      artifact writing and stable-id helpers.
- [ ] Optionally split `context-resolver-core` into ids/facts, query/response DTOs, traits and
      composite resolver modules only if it reduces coupling while keeping the source-neutral public
      API intact.
- [ ] Optionally split `syntax-helper-model` into discovery DTOs, identity helpers, platform records
      and sink/diagnostic modules only as a readability cleanup.
- [ ] Split `v8-context-hbk-cli` by command family and provider rendering responsibility. Do not move
      provider search/index ownership into the CLI.
- [ ] Avoid broad formatting-only churn outside the touched modules.

Verification:

- [ ] Existing tests pass without changing public CLI behavior, provider JSON, export JSON, resolver
      public API or SQLite schema.
- [ ] `cargo fmt --all --check`
- [ ] `cargo test --workspace`
