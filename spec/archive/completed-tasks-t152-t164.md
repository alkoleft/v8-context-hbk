# Completed Tasks T152-T164

For active task sequencing, use `../IMPLEMENTATION_TODO.md`.

## T152: Add Public Resolver Module-Context Boundary for HBK-Owned Platform Module Facts

References: `FR-CTX-RESOLVE-001`, `implementation/solution-context-resolve.md`,
`implementation/components.md`, ADR-0008.

Scope: extend `context-resolver-core` with provider-neutral module context DTOs/query and extend
`context-resolver-search` so HBK-backed indexes expose platform global methods/properties, module
events, event signatures and availability through the public Rust resolver API when indexed evidence
exists.

Boundaries: no dependency on `v8-context` or `v8-context-metadata`; no public SQLite/storage table
contract; no metadata-owned form/module/generated-type facts; no analyzer fallback lists for
`ЭтотОбъект` / `ThisObject` or other predefined members.

Result: `context-resolver-core` exposes `ModuleContextKind`, `ModuleContextQuery` and
`ResolvedModuleContext`; `context-resolver-search` exposes provider-backed BSL module contexts for
indexed module-event kinds; `syntax-helper-search` schema version 14 preserves module event context
kind as private provider state; resolved module context handles round-trip through exact id lookup;
unsupported/self-member gaps remain explicit.

Verification: focused `context-resolver-core`, `syntax-helper-search` and
`context-resolver-search` tests; `cargo fmt --all --check`; `cargo test --workspace`.

## T157: Replace Documentation-Site Custom Stable-Id Helpers With Narrow Library Dependencies

References: `FR-HBK-005`, ADR-0010, `implementation/documentation-site.md`,
`implementation/components.md`.

Scope: replace local `StableFnv64` with the `fnv` crate and local `slugify` with the `slug` crate in
`hbk-doc-site` generated identity helpers.

Boundaries: keep `hbk-doc-site` as the owner of generated page ids, node ids, source book ids and
build ids; do not change page-id seed composition, global TOC merge semantics, generated data
artifact layout, web-app routes or HBK parsing boundaries.

Result: `stable_hash_hex` now uses `fnv::FnvHasher`, preserving standard FNV-1a values; generated
slug components now use the `slug` crate and are URL-safe ASCII. Page ids and build ids retain the
existing hash format; node/source-book readable id fragments may change for non-ASCII titles or file
stems because the library transliterates Unicode into ASCII.

Verification: `cargo fmt --all --check`; `cargo test -p hbk-doc-site`; `cargo test --workspace`.

## T158: Evaluate Replacing Book/TOC Token Parsing With a Parser-Combinator Library

References: `FR-HBK-002`, `FR-HBK-003`, `implementation/components.md`.

Scope: replace `hbk-book` Book metadata and TOC parsing internals with `winnow` if it preserves the
current text grammar and improves maintainability for related future HBK-like formats.

Boundaries: no public `hbk-book` API redesign; no change to Book metadata, TOC tree, path
normalization or error-context contract; preserve legacy tokenizer semantics for BOM trivia, comma
separators and doubled quotes in strings.

Result: `hbk-book` now uses a `winnow`-backed cursor over the original Book/TOC text instead of
allocating a full token vector before parsing. The parser keeps BOM/comma trivia and doubled quote
semantics and preserves the existing Book metadata and TOC contracts. End-to-end release CLI
`toc --format json` measurements on `shcntx_ru.hbk` had lower non-outlier readings but roughly
unchanged average wall time because both old and new runs had outliers; process max RSS was higher.

Verification: focused `hbk-book` parser tests; release CLI comparison on representative real HBK
files; `cargo fmt --all --check`; `cargo test -p hbk-book`; `cargo test --workspace`.

## T159: Replace Repeated Manual Error Trait Implementations With `thiserror`

References: `NFR-DIAG-001`, `NFR-TEST-001`, `implementation/components.md`.

Scope: convert hand-written `fmt::Display`, `std::error::Error` and simple `From` boilerplate for
library error enums to `thiserror` derives where this preserves the current public enum variants and
user-visible messages.

Boundaries: keep typed library errors; do not introduce `anyhow` into library crates; do not change
error variants, diagnostics, CLI text, JSON output or recovery behavior; keep any custom
`PartialEq` implementations that encode test-visible comparison semantics.

Result: workspace `thiserror` dependency is shared by library crates that own typed error values.
Manual `Display`, `Error` and simple tuple-variant `From` implementations were replaced with
derives for HBK container/book/docs/export/site, Syntax Assistant export/extract/search and
document-kind parse errors while preserving public variants, user-visible messages, source-chain
behavior and custom `BookExportError` equality semantics. Remaining manual `From` implementations
encode non-trivial message wrapping or boxing behavior.

Verification: focused tests for touched crates; `cargo fmt --all --check`; `cargo test --workspace`.

## T160: Replace Narrow Hand-Written HTML Escaping and Syntax Assistant HTML Scans

References: `FR-HBK-004`, `FR-EXPORT-001`, `implementation/components.md`,
`implementation/documentation-site.md`.

Scope: evaluate and replace local HTML entity escaping/decoding and raw string scans in
`hbk-book-export` and `syntax-helper-extract` with existing crates or already-used `scraper` helpers
when real HBK fixtures prove equivalent behavior.

Boundaries: keep Syntax Assistant page-shape rules in `syntax-helper-extract`; do not move domain
section-label parsing into generic HTML helpers; do not change extraction schema, Markdown export
layout, heading-anchor behavior or current fixture snapshots without updating the relevant
acceptance/spec baseline.

Result: `hbk-book-export` now uses the `html-escape` crate for generated HTML text and attribute
escaping and for narrow title entity decoding, while keeping Markdown output byte-identical on
representative real HBK export. `syntax-helper-extract` now uses `scraper` for first-element text
selection and anchor/href enumeration and uses `html-escape` for the existing allow-listed entity
decoding inside the retained fragment scanner. The attempted whole-body DOM text replacement and
broader Syntax Assistant entity decoding were rejected because real comparison or review showed
canonical export behavior changes; those parser-quality changes remain separate future work.

Verification: focused `hbk-book-export` and `syntax-helper-extract` fixture tests; representative
real-HBK export/extraction comparison; `cargo fmt --all --check`; `cargo test --workspace`.

## T161: Spike Library-Backed Link/Path Rewriting Before Replacing Current HBK-Specific Rules

References: `FR-HBK-004`, `FR-HBK-005`, ADR-0009, ADR-0010, `implementation/components.md`.

Scope: test whether `lol_html`, `url`, `path-clean` or similarly narrow crates can reduce custom
`href`, fragment and virtual storage-path handling in documentation parsing and Markdown export
without losing HBK-specific `v8help://`, same-book and cross-book semantics.

Boundaries: spike only; do not replace `normalize_storage_path*`, `v8help://` handling, same-book
link rewriting or unresolved-link diagnostics until the spike documents exact behavior deltas on
fixtures and real HBK data; do not add recursive source discovery or generic graph libraries as part
of this task.

Result: no runtime behavior or product dependency was changed. `url` and `path-clean` are not
selected for HBK link/path rewriting because HBK `v8help://`, virtual storage paths, fragment-only
same-page links and unresolved-link diagnostics are project/domain semantics rather than URL or
filesystem semantics. `lol_html` remains a plausible future helper only for the narrow HTML `href`
attribute rewriting surface in `hbk-book-export`; any adoption must be a separate task with fixture
and real-HBK parity evidence for current same-book, cross-book, generated-alias and fragment
behavior. The spike conclusion is recorded in `implementation/components.md` and
`acceptance/baseline.md`.

Verification: `cargo fmt --all --check`; `cargo test -p hbk-docs`; `cargo test -p hbk-book-export`;
`cargo test -p hbk-doc-site`; existing real-HBK checks for representative pages, shared content-node
headings and `shclang_ru.hbk` fragment preservation.

## T162: Resolve Provider Type References to Enum Document Identities

References: `FR-SH-PROVIDER-001`, `FR-CTX-RESOLVE-001`, `implementation/components.md`,
`implementation/solution-context-resolve.md`, `acceptance/baseline.md`.

Scope: make `syntax-helper-search` treat source-backed enum definition documents as provider
type-like targets for normalized `type_refs`, so exact/alias references resolve to `enum:system:*`
identities instead of remaining unresolved when the enum document is unique.

Boundaries: start with confirmed enum documents only; do not guess other document categories; do not
convert enum identities into `platform_type:*`; do not add analyzer-side fallbacks, localized-name
hardcode or method-name hardcode; preserve explicit ambiguity when multiple type-like targets match.

Result: `syntax-helper-search` now treats enum definition documents as provider-owned type-like
targets alongside platform types. `type_identities` stores enum documents under their existing
`enum:system:*` / `enum:metadata_property:*` ids, normalized `type_refs` can resolve to those ids,
relation traversal can follow enum type-reference targets, and duplicate enum-name matches remain
`ambiguous`. `context-resolver-search` keeps enum-backed type references resolvable as `TypeId`
facts through direct id lookup and `has_type` traversal. The private rebuildable search-index schema
is version `15`.

Verification: `cargo fmt --all --check`; `cargo test -p syntax-helper-search`;
`cargo test -p context-resolver-search`; `cargo test --workspace`; fresh `shcntx_ru.hbk` index plus
deterministic `syntax type-ref-gaps` and SQL inventory recorded in `acceptance/baseline.md`.

## T163: Reduce Syntax Assistant Search-Index Build CPU/Allocation Overhead Without Schema Change

References: `NFR-PERF-001`, `NFR-QUERY-001`, `implementation/components.md`,
`acceptance/baseline.md`.

Scope: analyze current release-profile `syntax index` performance on `shcntx_ru.hbk`, identify
narrow build-path overhead that does not change query behavior, and optimize safe
`syntax-helper-search` allocation/data-structure choices.

Boundaries: no SQLite schema change; no FTS content-model change; no extractor page-cache or
concurrency refactor; preserve duplicate-document winner semantics and deterministic query output;
do not add tuning knobs.

Result: `insert_documents` now avoids intermediate vectors for newline-joined fields and
whitespace-searchable text; `insert_name_keys` uses a tiny vector sort/dedup instead of a
per-document `BTreeSet`; relation build uses hash membership/dedup sets where ordering is not
observable. A measured `sort_unstable_by` variant was rejected because duplicate-id winner semantics
changed type-reference and relation counts.

Verification: baseline release run `17.47s / 287052 KiB / 197M`; safe optimized release runs
`14.90s / 286568 KiB / 197M` and `14.56s / 286660 KiB / 197M`; final inventory stayed `25415`
documents, `132908` document-name rows, `58128` relations and `47156` type refs. `syntax
type-ref-gaps` stayed at `31638` resolved, `15513` unresolved, `5` ambiguous and `379`
template-binding rows. Representative `syntax get` and `syntax search` stayed unchanged.

## T164: Reduce Bounded HBK Read and Search-Index Lookup Overhead

References: `NFR-PERF-001`, `NFR-QUERY-001`, `implementation/components.md`,
`implementation/performance-baseline-t13.md`, `acceptance/baseline.md`.

Scope: audit current release-profile `syntax index` CPU/memory behavior on `shcntx_ru.hbk`, then
optimize only bounded HBK ZIP-entry read allocation and order-insensitive `syntax-helper-search`
lookup maps used during index build, normalized fact insertion and relation construction.

Boundaries: no SQLite schema change; no FTS content-model change; no extractor page-cache,
concurrency, broad storage/parser refactor or FileStorage lifetime redesign; preserve
duplicate-document winner semantics, deterministic candidate ordering, deterministic query output
and provider/resolver contracts; do not add tuning knobs.

Result: `hbk-book` now pre-sizes bounded ZIP-entry read buffers for `FileStorageReader` page reads
and PackBlock TOC reads from entry metadata, capped at 64 MiB so malformed size metadata cannot
force unbounded preallocation. `syntax-helper-search` now uses hash maps for order-insensitive build
lookups in relation lookup, type-ref target lookup, normalized fact insertion and type-template
helper maps while keeping ordered sets where candidate order is observable.

Verification: baseline release `syntax index shcntx_ru.hbk` runs measured
`21.27s / 282048 KiB / 197M` and `17.79s / 282176 KiB / 197M`; post-change runs measured
`17.41s / 285696 KiB / 197M` and `16.86s / 285764 KiB / 197M`. Row inventory stayed `25415`
documents, `132908` document-name rows, `58128` relations and `47156` type refs. `syntax
type-ref-gaps` stayed at `31638` resolved, `15513` unresolved, `5` ambiguous and `379`
template-binding rows. Representative `syntax get`, `syntax search` and `syntax related` JSON
outputs were byte-identical between baseline and post-change indexes. Verification passed with
`cargo fmt --all --check`, `cargo test -p hbk-book`, `cargo test -p syntax-helper-search` and
focused clippy for `hbk-book` / `syntax-helper-search`.
