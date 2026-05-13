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
- [archive/completed-tasks-t143-t151.md](archive/completed-tasks-t143-t151.md)

Current status: T35-T151 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design,
language-domain, cleanup, book-content export, documentation-site, platform type-template and
type-reference conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`requirements/non-functional.md`, `implementation/components.md`,
`implementation/documentation-site.md`, `implementation/syntax-helper-query-cli.md`,
`implementation/syntax-bsl-provider-plan.md`, `implementation/solution-context-resolve.md` and
`decisions/`.

Current first unchecked task: none.

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

- [x] T152: Add public resolver module-context boundary for HBK-owned platform module facts.
  - Scope: extend `context-resolver-core` with provider-neutral module context DTOs/query and
    extend `context-resolver-search` so HBK-backed indexes expose platform global
    methods/properties, module events, event signatures and availability through the public Rust
    resolver API when indexed evidence exists.
  - Boundaries: no dependency on `v8-context` or `v8-context-metadata`; no public SQLite/storage
    table contract; no metadata-owned form/module/generated-type facts; no analyzer fallback lists
    for `ЭтотОбъект` / `ThisObject` or other predefined members.
  - Spec refs: `FR-CTX-RESOLVE-001`, `implementation/solution-context-resolve.md`,
    `implementation/components.md`, ADR-0008.
  - Verification: focused `context-resolver-core`, `syntax-helper-search` and
    `context-resolver-search` tests; `cargo fmt --all --check`; `cargo test --workspace`.
  - Result: `context-resolver-core` exposes `ModuleContextKind`, `ModuleContextQuery` and
    `ResolvedModuleContext`; `context-resolver-search` exposes provider-backed BSL module contexts
    for indexed module-event kinds; `syntax-helper-search` schema version 14 preserves module event
    context kind as private provider state; resolved module context handles round-trip through
    exact id lookup; unsupported/self-member gaps remain explicit.

- [x] T157: Replace documentation-site custom stable-id helpers with narrow library dependencies.
  - Scope: replace local `StableFnv64` with the `fnv` crate and local `slugify` with the `slug`
    crate in `hbk-doc-site` generated identity helpers.
  - Boundaries: keep `hbk-doc-site` as the owner of generated page ids, node ids, source book ids
    and build ids; do not change page-id seed composition, global TOC merge semantics, generated
    data artifact layout, web-app routes or HBK parsing boundaries.
  - Spec refs: `FR-HBK-005`, ADR-0010, `implementation/documentation-site.md`,
    `implementation/components.md`.
  - Verification: `cargo fmt --all --check`; `cargo test -p hbk-doc-site`; `cargo test
    --workspace`.
  - Result: `stable_hash_hex` now uses `fnv::FnvHasher`, preserving standard FNV-1a values;
    generated slug components now use the `slug` crate and are URL-safe ASCII. Page ids and build
    ids retain the existing hash format; node/source-book readable id fragments may change for
    non-ASCII titles or file stems because the library transliterates Unicode into ASCII.

- [x] T158: Evaluate replacing Book/TOC token parsing with a parser-combinator library.
  - Scope: replace `hbk-book` Book metadata and TOC parsing internals with `winnow` if it preserves
    the current text grammar and improves maintainability for related future HBK-like formats.
  - Boundaries: no public `hbk-book` API redesign; no change to Book metadata, TOC tree, path
    normalization or error-context contract; preserve legacy tokenizer semantics for BOM trivia,
    comma separators and doubled quotes in strings.
  - Spec refs: `FR-HBK-002`, `FR-HBK-003`, `implementation/components.md`.
  - Verification: focused `hbk-book` parser tests; release CLI comparison on representative real
    HBK files; `cargo fmt --all --check`; `cargo test -p hbk-book`; `cargo test --workspace`.
  - Result: `hbk-book` now uses a `winnow`-backed cursor over the original Book/TOC text instead
    of allocating a full token vector before parsing. The parser keeps BOM/comma trivia and doubled
    quote semantics and preserves the existing Book metadata and TOC contracts. End-to-end release
    CLI `toc --format json` measurements on `shcntx_ru.hbk` had lower non-outlier readings but
    roughly unchanged average wall time because both old and new runs had outliers; process max RSS
    was higher.

- [x] T159: Replace repeated manual error trait implementations with `thiserror`.
  - Scope: convert hand-written `fmt::Display`, `std::error::Error` and simple `From` boilerplate
    for library error enums to `thiserror` derives where this preserves the current public enum
    variants and user-visible messages.
  - Boundaries: keep typed library errors; do not introduce `anyhow` into library crates; do not
    change error variants, diagnostics, CLI text, JSON output or recovery behavior; keep any custom
    `PartialEq` implementations that encode test-visible comparison semantics.
  - Spec refs: `NFR-DIAG-001`, `NFR-TEST-001`, `implementation/components.md`.
  - Verification: focused tests for touched crates; `cargo fmt --all --check`; `cargo test
    --workspace`.
  - Result: workspace `thiserror` dependency is shared by library crates that own typed error
    values. Manual `Display`, `Error` and simple tuple-variant `From` implementations were replaced
    with derives for HBK container/book/docs/export/site, Syntax Assistant export/extract/search
    and document-kind parse errors while preserving public variants, user-visible messages,
    source-chain behavior and custom `BookExportError` equality semantics. Remaining manual `From`
    implementations encode non-trivial message wrapping or boxing behavior.

- [x] T160: Replace narrow hand-written HTML escaping and Syntax Assistant HTML scans with existing
  parser/escaping utilities where behavior is preserved.
  - Scope: evaluate and replace local HTML entity escaping/decoding and raw string scans in
    `hbk-book-export` and `syntax-helper-extract` with existing crates or already-used `scraper`
    helpers when real HBK fixtures prove equivalent behavior.
  - Boundaries: keep Syntax Assistant page-shape rules in `syntax-helper-extract`; do not move
    domain section-label parsing into generic HTML helpers; do not change extraction schema,
    Markdown export layout, heading-anchor behavior or current fixture snapshots without updating
    the relevant acceptance/spec baseline.
  - Spec refs: `FR-HBK-004`, `FR-EXPORT-001`, `implementation/components.md`,
    `implementation/documentation-site.md`.
  - Verification: focused `hbk-book-export` and `syntax-helper-extract` fixture tests;
    representative real-HBK export/extraction comparison; `cargo fmt --all --check`; `cargo test
    --workspace`.
  - Result: `hbk-book-export` now uses the `html-escape` crate for generated HTML text and
    attribute escaping and for narrow title entity decoding, while keeping Markdown output
    byte-identical on representative real HBK export. `syntax-helper-extract` now uses `scraper`
    for first-element text selection and anchor/href enumeration and uses `html-escape` for the
    existing allow-listed entity decoding inside the retained fragment scanner. The attempted
    whole-body DOM text replacement and broader Syntax Assistant entity decoding were rejected
    because real comparison or review showed canonical export behavior changes; those parser-quality
    changes remain separate future work.

- [x] T161: Spike library-backed link/path rewriting before replacing current HBK-specific rules.
  - Scope: test whether `lol_html`, `url`, `path-clean` or similarly narrow crates can reduce
    custom `href`, fragment and virtual storage-path handling in documentation parsing and Markdown
    export without losing HBK-specific `v8help://`, same-book and cross-book semantics.
  - Boundaries: spike only; do not replace `normalize_storage_path*`, `v8help://` handling,
    same-book link rewriting or unresolved-link diagnostics until the spike documents exact
    behavior deltas on fixtures and real HBK data; do not add recursive source discovery or generic
    graph libraries as part of this task.
  - Spec refs: `FR-HBK-004`, `FR-HBK-005`, ADR-0009, ADR-0010,
    `implementation/components.md`.
  - Verification: short written conclusion in the task result; fixture coverage for accepted
    deltas if implementation proceeds; `cargo fmt --all --check`; relevant focused crate tests.
  - Result: no runtime behavior or product dependency was changed. `url` and `path-clean` are not
    selected for HBK link/path rewriting because HBK `v8help://`, virtual storage paths,
    fragment-only same-page links and unresolved-link diagnostics are project/domain semantics
    rather than URL or filesystem semantics. `lol_html` remains a plausible future helper only for
    the narrow HTML `href` attribute rewriting surface in `hbk-book-export`; any adoption must be a
    separate task with fixture and real-HBK parity evidence for current same-book, cross-book,
    generated-alias and fragment behavior. The spike conclusion is recorded in
    `implementation/components.md` and `acceptance/baseline.md`. Verification passed with
    `cargo fmt --all --check`, `cargo test -p hbk-docs`, `cargo test -p hbk-book-export` and
    `cargo test -p hbk-doc-site`; `hbk-book-export` included existing real-HBK checks for
    representative pages, shared content-node headings and `shclang_ru.hbk` fragment preservation.
