# Acceptance Baseline

This file contains durable acceptance gates and conclusions. Raw run logs and generated output
directories are service data unless promoted here.

## Current Baseline

- Target platform baseline: `8.5.1.1150`.
- T9 Syntax Assistant acceptance passed for `shcntx_ru.hbk` and `shcntx_root.hbk`.
- T10 all-HBK smoke passed for every `*.hbk` file under `/opt/1cv8/x86_64/8.5.1.1150/`.
- T12 workspace split passed with package-level checks and preserved CLI behavior.
- T151 internal module decomposition split the mandatory large crate entrypoints by responsibility
  while preserving existing public API, provider/export JSON, resolver behavior and SQLite schema.
  Verification passed with `cargo fmt --all --check` and `cargo test --workspace`.
- T152 added the public Rust resolver module-context boundary for HBK-owned platform module facts.
  `context-resolver-core` now exposes `ModuleContextKind`, `ModuleContextQuery` and
  `ResolvedModuleContext`; `context-resolver-search` maps HBK-backed platform global
  methods/properties and indexed `module_event` facts into BSL module contexts, preserving event
  signatures and availability through existing callable/availability DTOs. `syntax-helper-search`
  schema version 14 stores provider-neutral module context kind for module-event documents as
  private rebuildable provider state. Resolved module context handles round-trip through exact id
  lookup as `ModuleContext` facts. Dedicated predefined self members such as `ЭтотОбъект` /
  `ThisObject`, command module context and record-set module context remain explicit
  unsupported/not-found outcomes until HBK extraction/indexing stores source-backed facts for them.
  Verification passed with `cargo fmt --all --check`, focused resolver/search tests and
  `cargo test --workspace`.
- T157 replaced `hbk-doc-site` custom generated-identity helper implementations with narrow
  library dependencies. `stable_hash_hex` uses `fnv::FnvHasher` and keeps standard FNV-1a values
  such as `foobar -> 85944171f73967e8`; generated slug fragments use the `slug` crate and are
  URL-safe ASCII. Page ids and build ids keep the existing hash shape, while node/source-book
  readable id fragments may change for non-ASCII titles or file stems because the library
  transliterates Unicode. Verification passed with `cargo fmt --all --check`,
  `cargo test -p hbk-doc-site` and `cargo test --workspace`.
- T159 replaced repeated hand-written library error trait implementations with `thiserror` derives.
  Public error enum variants, user-visible messages, source-chain behavior, CLI text and JSON
  contracts were preserved; `BookExportError` keeps its custom comparison semantics, and the only
  remaining manual `From` conversions handle non-trivial message wrapping or boxing. Verification
  passed with focused touched-crate tests, `cargo fmt --all --check` and `cargo test --workspace`.
- T160 replaced narrow hand-written HTML utility code where real output parity was proven.
  `hbk-book-export` now delegates generated HTML text/attribute escaping and title entity decoding
  to `html-escape`; `syntax-helper-extract` delegates first-element text selection and anchor/href
  enumeration to `scraper` and uses `html-escape` for the existing allow-listed entity decoding
  inside the retained fragment scanner. Canonical behavior was preserved: representative
  `shlang_ru.hbk` Markdown export was byte-identical to the pre-change `HEAD` output, and
  `shcntx_ru.hbk` `syntax export` was byte-identical to pre-change `HEAD` output with the same 13
  files, record counts and 5 parser warnings. A broader `body_text` DOM replacement and broader
  Syntax Assistant entity decoding were rejected because comparison/review showed canonical export
  behavior changes, so those parser-quality changes remain separate future work.
- T161 completed a spike for library-backed link/path rewriting without changing runtime behavior
  or adding product dependencies. The accepted boundary remains HBK-specific: `hbk-book` owns
  virtual storage path normalization, `hbk-docs` owns documentation link extraction and diagnostics,
  `hbk-book-export` owns single-book Markdown link rewriting and fragment preservation, and
  `hbk-doc-site` owns generated page-id aliases and same-page fragment collapse. `url` and
  `path-clean` were rejected for this boundary because their URL/filesystem semantics do not own
  HBK `v8help://`, virtual storage-path, fragment-only or unresolved-link rules. `lol_html` remains
  a possible future implementation helper only for HTML-aware `href` attribute rewriting, and any
  such replacement requires a separate fixture-backed and real-HBK parity task before it becomes a
  runtime dependency.
- T162 makes enum definition documents provider-owned type-like targets for normalized type
  references. The private search-index schema version is `15`: `type_identities` now includes enum
  documents with their existing `enum:system:*` / `enum:metadata_property:*` identities, and
  `type_refs.target_type_id` can point to those ids when a target name or alias uniquely matches an
  enum document. Enum identities are not converted to `platform_type:*`; duplicate enum names remain
  `ambiguous`, and analyzer-side hardcode/fallbacks remain out of scope. Verification passed with
  `cargo fmt --all --check`, `cargo test -p syntax-helper-search`, `cargo test -p
  context-resolver-search`, `cargo test --workspace` and a fresh `shcntx_ru.hbk` index inventory
  recorded under the T162 baseline note.
- T163 reduces release-profile `syntax index` CPU/allocation overhead without changing SQLite schema
  version `15` or query behavior. The accepted safe slice avoids intermediate vectors for stored
  signature/parameter/type-return strings and searchable text, uses tiny vector sort/dedup for
  per-document name keys, and uses hash membership/dedup sets for relation build internals where
  order is not externally observable. The measured `sort_unstable_by` variant was rejected because
  duplicate document id winner semantics changed type-reference and relation counts. The remaining
  recommendations are to profile extractor page reuse before adding a page cache, and to revisit
  FTS/content storage only with a separate schema task because T44 already showed contentless FTS
  trades a smaller database for extra query-path complexity without a build-time win.
- T164 reduces bounded HBK read and search-index lookup overhead without changing SQLite schema
  version `15` or query behavior. `hbk-book` pre-sizes `FileStorageReader` page-read and PackBlock
  TOC buffers from ZIP entry metadata with a 64 MiB cap; `syntax-helper-search` uses `HashMap` for
  order-insensitive build lookups while preserving ordered candidate sets where deterministic output
  depends on ordering. Release `syntax index shcntx_ru.hbk` post-change runs measured `17.41s /
  285696 KiB / 197M` and `16.86s / 285764 KiB / 197M`; row inventory stayed `25415` documents,
  `132908` document-name rows, `58128` relations and `47156` type refs. `syntax type-ref-gaps`
  stayed at `31638` resolved, `15513` unresolved, `5` ambiguous and `379` template-binding rows.
- T165 extends the Rust dependency-facing BSL language type surface without changing platform
  export JSON, provider CLI JSON or SQLite schema version. `syntax-helper-language` now extracts
  direct `shlang_*` primitive type pages as `language_type` facts for `Null`, `Неопределено` /
  `Undefined`, `Число` / `Number`, `Строка` / `String`, `Дата` / `Date`, `Булево` / `Boolean` and
  `Тип` / `Type`, while nested primitive literal pages such as `def_BooleanTrue` remain outside the
  type surface. `syntax-helper-search` indexes these as source-qualified `shlang:*` language facts,
  and `context-resolver-search` exposes them through `LanguageSearchSource` as
  `LanguageDomain::BslLanguage`, not `PlatformApi`. Verification passed with
  `cargo test -p syntax-helper-language`, `cargo test -p syntax-helper-search`,
  `cargo test -p context-resolver-search` and `cargo test --workspace`.
- T166 changes the Rust dependency-facing query-table boundary. Existing `shcntx_*`
  `query_table`, `query_table_field` and `query_table_parameter` provider documents remain hidden
  from the platform adapter, but are exposed through a distinct `LanguageDomain::QueryLanguage`
  query-table source. The source returns template/family-level query table facts with stable ids,
  syntax/identifier/table-role data, owner semantic path, source-derived template parameter slots,
  owned field/parameter facts, type references and source-neutral evidence/provenance by resolver
  source id, evidence id and locale. It does not expose raw HBK paths, TOC paths, HTML paths or page
  titles through `context-resolver-core`, and it does not synthesize concrete metadata query tables
  or analyzer fallback tables. The query-table source advertises exact lookup and relation
  capabilities only. The private search-index schema version is `16`, adding persisted query-table
  metadata without making SQLite a dependency-facing contract. Verification passed with focused
  query-table resolver/search tests, `cargo check --workspace`, `cargo fmt --all --check` and
  `cargo test --workspace`.
- T15 Syntax Assistant performance pass reduced debug-binary peak RSS without wall-clock regression:
  `shcntx_ru.hbk` measured `19.26s / 590988 KiB`, and `shcntx_root.hbk` measured
  `14.62s / 324476 KiB`.
- T17 streaming extraction reduced the debug-binary `shcntx_ru.hbk` export peak to
  `20.46s / 386304 KiB` while preserving export shape, record counts and deterministic JSON output.
  `shcntx_root.hbk` measured `18.15s / 324096 KiB`, still effectively bounded by the lower-level
  open-time peak.
- T19 byte-only container entity reads reduced the remaining `HbkBook::open` VmHWM from
  `383232 KiB` to `131328 KiB` for `shcntx_ru.hbk` and from `321408 KiB` to `119168 KiB` for
  `shcntx_root.hbk`. Full `syntax-helper --output` remained shape/count stable and measured
  `21.19s / 168692 KiB` for `shcntx_ru.hbk` and `16.11s / 144500 KiB` for `shcntx_root.hbk`.
- T20 measured the remaining owned `FileStorage` copy and did not justify a broader direct seekable
  view. The exact retained vector was `38960718` bytes for `shcntx_ru.hbk` and `32620458` bytes for
  `shcntx_root.hbk`, while full `syntax-helper --output` measured `17.68s / 157916 KiB` and
  `13.50s / 139632 KiB` with stable export counts.
- T21 measured retained TOC/root-discovery structures and did not justify a production refactor.
  The largest T21-specific retained structure was public `RootDiscovery` at about 9 MiB, while full
  `syntax-helper --output` measured `19.04s / 157788 KiB` for `shcntx_ru.hbk` and
  `14.33s / 139764 KiB` for `shcntx_root.hbk` with stable export counts.
- T22 released the avoidable `HbkContainer` mmap retained by `HbkBook` after open. Full
  `syntax-helper --output` measured `17.97s / 134656 KiB` for `shcntx_ru.hbk` and
  `13.65s / 122112 KiB` for `shcntx_root.hbk` with byte-identical JSON export compared with the
  pre-change run. T22 also changed the attribution baseline for the retained `FileStorage` vector:
  T20 remains pre-T22 evidence for the broader export peak, but no longer describes the current
  `HbkBook::open` memory split.
- T23 remeasured the same `FileStorage` vector on the post-T22 baseline, then a user-directed
  production follow-up removed retained `FileStorage` bytes from `HbkBook` without reintroducing
  retained `HbkContainer` mmap. `HbkBook::open` current RSS now measures `33164 KiB` for
  `shcntx_ru.hbk` and `32928 KiB` for `shcntx_root.hbk`; open-path high-water RSS and full
  `syntax-helper --output` peak remain in the same class because open still validates the
  `FileStorage` entity body and extraction still owns `FileStorage` bytes for the reader lifetime.
- T24 targeted parser, lookup and lean streaming-export optimizations kept JSON output
  byte-identical to the local T23 production exports. Full `syntax-helper --output` measured
  `18.40s / 134528 KiB` for `shcntx_ru.hbk` and `14.09s / 122108 KiB` for `shcntx_root.hbk`; a
  repeated `shcntx_root.hbk` run measured `14.34s / 122112 KiB` and matched byte-for-byte.
- T29 promoted global context events, query/table fields and query/table parameters into consumer
  export record families and raised the schema to version 4 at that milestone. Each source book
  exports 33 global context events, 588 table fields, 78 table parameters and 4 remaining
  diagnostics.
- T32 switched the canonical consumer JSON export to lean `schema_version: 5`. Consumer platform
  API records omit `null` fields and empty arrays, owner/type-reference fields are string-based,
  `available_since` is emitted as `availability.since`, enum values are nested in `enums.json` and
  `enum-values.json` is no longer emitted.
- T30 removed the post-T29 table-owner lookup regression by replacing per-record
  `Toc::find_by_html_path` calls with one extraction-scope TOC HTML-path index. Release-profile
  `schema_version: 5` exports measured `4.76s / 167452 KiB` for `shcntx_ru.hbk` and
  `3.62s / 131748 KiB` for `shcntx_root.hbk`; both outputs were byte-identical to the pre-change
  T32 exports.
- T31 remeasured the residual post-T30 release-profile path and did not justify another parser or
  export code change. Current `schema_version: 5` exports measured `4.96s / 151644 KiB` for
  `shcntx_ru.hbk`, `3.68s / 128828 KiB` for `shcntx_root.hbk`, and `3.90s / 127780 KiB` for a
  deterministic root repeat. The root repeat was byte-identical to the first root export.
- T33 changed the canonical consumer JSON export to `schema_version: 6`. Consumer type-reference
  fields are now named `types`, callable return fields are named `return`, inline example sections
  no longer absorb availability text, code examples no longer contain syntax-coloring spaces around
  BSL punctuation, and see-also owner/member links are composed as `Owner.Member`.
- A 2026-05-01 review of `/tmp/shcntx/` found open TOC-aware Syntax Assistant reading gaps:
  repeated query table parameters, global context events, platform type/object pages and
  placeholder-like records can still become name/owner ambiguous facts. ADR-0005, FR-SH-003 and
  UAT-SH-013 now own this as a reading/classification issue, not an export-provenance issue.
  The accepted T35 classification direction separates TOC branch kind from record family, models
  module events explicitly, distinguishes regular/extension/primitive/metadata-template platform
  types and treats primitive type traversal as shallow.
- T35 changed the canonical consumer JSON export to `schema_version: 7`. The required
  `global-context-events.json` adapter now serializes `module_event` facts with semantic module
  context, query/table records expose TOC-derived `owner_path`, platform types expose `type_kind`
  and branch kind, and placeholder-like type properties/constructors expose semantic owner paths
  without raw HBK/TOC/HTML/page-title provenance. Full debug CLI exports for both source books
  produced 697 module events, 1869 platform types, 588 table fields, 78 table parameters and 4
  diagnostics per locale. The root/English acceptance guard also checks that `Client application
  form...` event branches stay `module.kind="form"` and that `Information` suffixes do not match
  the managed-forms branch by substring alone.
- T36 changed the canonical consumer JSON export to `schema_version: 8`. `metadata.json.files` now
  lists `query-tables.json` instead of `table-fields.json` and `table-parameters.json`; normal
  exports do not delete stale older-schema files from reused output directories. Full debug CLI
  exports for both source books produced 59 query table records, 588 nested table fields, 78 nested
  table parameters, 697 module events, 1869 platform types and 4 diagnostics per locale. Query table
  records carry table-family `owner_path` and `table_role`; nested fields and parameters use string
  names, do not repeat `owner_path`, and parameters do not expose `required`. Type methods, type
  properties and constructors no longer expose derivative-record `owner_path`.
- T37 changed the canonical consumer JSON export to `schema_version: 9`. `metadata.json.files` now
  lists `module-events.json`, `type-events.json` and `unknown-events.json` instead of
  `global-context-events.json`. Full CLI exports for both source books produced 47 module events,
  650 type events and 0 unknown events per locale, while preserving 59 query table records, 588
  nested table fields, 78 nested table parameters, 1869 platform types and 4 diagnostics per locale.
  Event records do not expose raw HBK/TOC/HTML/page-title provenance, cross-cutting `id` or
  `owner_ref`, or event-local `owner_kind`; type events carry `owner` and semantic `owner_path`
  only as event owner context. The T36 derivative-record `owner_path` omission remains unchanged.
- T38 kept the canonical consumer JSON export at `schema_version: 9` and added source-backed
  `object_kind` only to `platform-types.json` records. Full debug CLI exports for both source books
  preserved the T37 record-family counts. RU/root owner classification counts were:
  `regular_platform_type` 1305/1357, `managed_form` 77/286, `form_extension` 174/2 and
  `metadata_object` 287/96. Event records still do not expose `owner_kind`, `object_kind`, `id`,
  `owner_ref` or raw parser provenance, and derivative type members, constructors and nested query
  table records still omit `owner_path`.
- T39 changed the canonical consumer JSON export to `schema_version: 10` and removed semantic
  `owner_path` from `type-events.json`. Full debug CLI export for `shcntx_ru.hbk` preserved the
  T37/T38 record-family counts and kept `owner_path` only on owning records such as platform types,
  module event context and query table records. Type-event owner context is composed into the
  single `owner` string, preserving exact uniqueness by `(owner, name.primary, name.alias)`.
- T40 changed the canonical consumer JSON export to `schema_version: 11` and added query table
  localized `syntax` and `identifier` to `query-tables.json`. Full debug CLI exports for both
  source books preserved the T36-T39 record-family counts: 59 query tables, 588 nested table fields,
  78 nested table parameters, 47 module events, 650 type events and 4 parser diagnostics per locale.
  Query table `table_role` is now derived from `syntax.primary` shape before falling back to generic
  table names:
  `Таблица бизнес-процессов` / `Business Process Table` is `primary` with identifier
  `БизнесПроцесс` / `BusinessProcess`, while the `.Точки` / `.Points` table remains `additional`
  with a CamelCase identifier derived from the primary table identifier plus page `name`. The
  additional table suffix normalization removes punctuation such as hyphens, for example
  `Таблица изменений бизнес-процессов` becomes `ТаблицаИзмененийБизнесПроцессов`. Russian query
  table syntax splits parenthesized English syntax into `syntax.alias`; root-source English syntax
  remains `syntax.primary` without an alias when the source has no parenthesized variant.
- T75 implemented the pre-rework cleanup contract for query-table pages with missing or empty
  syntax. The extractor no longer derives `identifier` or `table_role` from table display names.
  Source-backed pages such as `Таблицы задач > Основная таблица` / `Task Tables > Main Table`
  remain exported with their nested field/parameter facts, but omit `syntax` and `identifier`, use
  `table_role="unknown"` and add a parser-maintenance `MISSING_QUERY_TABLE_SYNTAX` diagnostic with
  source provenance.
- T82 preserved the T75 consumer JSON contract while replacing the internal empty-string
  query-table identifier sentinel with typed absence. Missing-syntax query tables still receive
  deterministic search/index document ids from TOC-derived semantic owner context, not from a
  synthesized identifier source fact.
- T66 completed the non-platform HBK domain-analysis gate for ADR-0008 without code changes.
  Real-source inspection of `/opt/1cv8/x86_64/8.5.1.1150/shlang_ru.hbk`,
  `shquery_ru.hbk` and `dcsui_ru.hbk` selected a minimal shared language-fact model instead of
  platform-style record families. Representative anchors are `shlang:def_String` and
  `shlang:def_Func` for BSL language types/constructs, `shquery:SELECTStatement`,
  `shquery:SUM`, `shquery:STRING` and `shquery:LitString` for query clauses/functions/literals,
  and `dcsui:SKD_Functions_Strings`, `dcsui:SKD_ExtQueryLangv` and `dcsui:SKD_Lang` for data
  composition expression/query-extension syntax. T166 supersedes the earlier temporary decision for
  existing `shcntx_*` `query_table`, `query_table_field` and `query_table_parameter` index facts:
  they now have an explicit QueryLanguage query-table resolver source while remaining hidden from
  the platform adapter.
- T89 implemented the first shared language-fact extraction/index fixture slice. The new
  `syntax-helper-language` crate extracts `language_type`, `language_construct`,
  `language_function`, `language_keyword` and `language_literal` facts from committed real-source
  fixtures for the selected `shlang_*`, `shquery_*` and `dcsui_*` pages. `syntax-helper-search`
  indexes those facts as `language_*` document kinds while keeping `syntax export` platform
  consumer JSON unchanged.
- T87 classified the residual duplicate-looking query/provider mechanisms after the cleanup
  sequence. The current boundary decisions require no code, CLI JSON, SQLite schema, export schema,
  extraction or resolver behavior change: `syntax get` classifier/execution/status mapping remains
  accepted CLI-boundary separation; search lookup-key normalization remains in
  `syntax-helper-search`; public provider JSON shaping remains in `v8-context-hbk-cli`; storage
  path, documentation link and Syntax Assistant member-link normalization remain distinct component
  boundaries.
- T91 collapsed the only stale duplication found by T87: the localized display-name presentation
  helper now lives on `syntax-helper-model::LocalizedName` and is reused by `syntax-helper-search`
  and `v8-context-hbk-cli`. This did not change CLI text output, provider JSON, SQLite schema,
  search ranking, relation labels, export schema, extraction behavior or resolver contracts.
- T93 enforced the provider JSON boundary for nested callable facts. `syntax-helper-search`
  `SearchSignature` and `SearchParameter` are Rust query structs without serde/provider DTO
  attributes; `v8-context-hbk-cli` explicitly assembles export-compatible
  `signatures[].parameters[]` provider JSON. This did not change provider response schema version,
  SQLite schema, search ranking, lookup behavior or export JSON.
- T94 deduplicated search relation graph construction inside `syntax-helper-search`. SQLite
  relation insertion and focused relation tests now share one streaming relation-row builder and the
  same `(source_id, target_id, edge_kind)` deduplication key. This did not change relation edge
  kinds, labels, evidence, weights, SQLite schema, query ordering, provider JSON or resolver facts.
- T95 replaced raw search document kind strings in Rust search/resolver structs with
  `syntax-helper-search::SearchDocumentKind`. Explicit boundary conversion preserves existing
  SQLite `documents.kind` strings and provider `kind` values. Focused tests now assert all current
  kind strings round-trip through the typed model with unchanged priority values, and
  `context-resolver-search` still keeps `query_table*` provider facts hidden from the platform
  resolver adapter.
- T96 retired the `HbkBook::read_pages` test/support convenience API. The ordinary supported
  page/file surface remains `HbkBook::read_file`, `HbkBook::read_page` and `FileStorageReader`;
  deterministic repeated-page fixture coverage uses `FileStorageReader` directly.
- T97 deduplicated the first `syntax-helper-language` callable fact assembly paths without changing
  parser coverage or public contracts. `shquery_*` function parsing and DCSUI string-function
  slicing remain source-family-specific, while shared assembly now preserves source-qualified ids,
  language domains, fact families, signatures, parameter names, return/type refs and provenance for
  existing fixture-backed language facts.
- T99 added the `hbk-book-export` crate boundary for ordinary book-content export without wiring a
  user-visible CLI command or implementing file writes. The crate currently owns typed
  `BookExportRequest`, `BookExportFormat`, `BookExportHierarchy`, `BookExporter`,
  `BookExportResult` and `BookExportError` concepts. Request validation rejects unsafe output roots
  and unsupported format/hierarchy pairs, and the direct dependency boundary is limited to
  `hbk-book` and `hbk-docs`.
- T100 implemented raw/raw ordinary book export in `hbk-book-export` without wiring the top-level
  CLI command. `BookExporter::export` writes original FileStorage bytes under normalized relative
  paths for `format=raw` with `hierarchy=raw`. Export planning rejects unsafe storage paths and
  duplicate or file/directory-colliding normalized output paths before creating the output root, and
  the exporter rejects request/opened-book source path mismatches before writing. The direct
  dependency boundary remains limited to `hbk-book` and `hbk-docs`; `hbk-book` exposes the narrow
  `FileStorageReader::file_paths` enumeration surface used by the exporter.
- T101 wired the top-level `v8-context-hbk export` CLI command to `hbk-book-export` for ordinary
  book-content export. `format=raw` with `hierarchy=raw` now works through the CLI and preserves raw
  `FileStorage` bytes under normalized storage paths. Non-raw/raw CLI combinations, including
  `format=raw` with `hierarchy=toc` and `format=markdown` with `hierarchy=toc`, return stable
  readable diagnostics before the HBK source file is opened. `syntax export` remains unchanged and
  continues to use `hbk-syntax-export`. Focused tests cover CLI parsing, raw/raw export and
  unsupported matrix diagnostics; local UAT smoke on
  `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` wrote 2 files / 1792 bytes and returned the expected
  raw/toc and markdown/toc unsupported diagnostics.
- T102 selected `quick_html2md` 0.2.1 for ordinary book HTML-to-Markdown conversion inside
  `hbk-book-export` and added `BookExporter::markdown_page()` for individual TOC pages. Focused
  tests on representative local 8.5.1.1150 pages from `dcsui_ru.hbk`, `shlang_ru.hbk`,
  `shquery_ru.hbk`, `fmtdui_ru.hbk`, `htmlui_ru.hbk` and `moxelui_ru.hbk` preserved readable
  headings, body text, link text, lists, GFM tables and angle-bracket syntax placeholders while
  rejecting raw HBK paths, raw TOC indexes, raw HTML page paths and service HTML scaffolding in
  normal Markdown output. `markdown/toc` remains blocked at the top-level CLI boundary until T103
  implements deterministic TOC layout and UAT corpus export.
- T103 implemented top-level `format=markdown` with `hierarchy=toc` for ordinary book-content
  export. `hbk-book-export` now writes one `index.md` per TOC item under a deterministic
  title-derived directory tree, disambiguates same-title siblings with stable suffixes, rewrites
  internal links to exported TOC Markdown targets, preserves external links and emits heading-only
  pages for TOC items with empty or missing `FileStorage` page targets. `raw/toc` and
  `markdown/raw` remain unsupported with pre-open CLI diagnostics. Local UAT-HBK-004 exported the
  six-book 8.5.1.1150 corpus with 291 Markdown files and passed negative searches for raw HBK
  paths, raw TOC indexes, raw HTML page paths and service HTML scaffolding. UAT-HBK-005,
  UAT-HBK-006 and UAT-HBK-007 passed against the representative DCS, language/query-language and
  ordinary UI help pages.
- T104 fixed the real `shclang_ru.hbk` Markdown/TOC regression where several TOC sections reuse the
  shared service placeholder path `_CONTENTS_NODE_fileConf`. Those placeholder pages now export as
  heading-only Markdown with each item's own TOC title instead of borrowing the first matching TOC
  title. Fresh release export of `shclang_ru.hbk` produced 35 Markdown files for 35 TOC pages;
  UAT-HBK-010 passed for `Общие объекты` and `Работа с запросами`.
- T105 fixed the real `shclang_ru.hbk` Markdown/TOC regression where the `WorkinWithBath` page
  stores the package-query BSL/query example as a one-cell Courier HTML table. `hbk-book-export`
  now rewrites one-cell Courier code tables to Markdown code blocks before `quick_html2md`, so the
  example keeps line breaks and leading `|` query markers instead of becoming a one-cell GFM table.
  UAT-HBK-011 passed, and the representative DCS keyword table remains exported as a Markdown table.
- T106 changed the T105 code-table output to language-tagged fenced blocks. `hbk-book-export`
  rewrites those examples to `<pre><code class="language-bsl">`, and the resulting Markdown uses
  ` ```bsl ` fences. Fresh release export of `shclang_ru.hbk` kept 35 Markdown files and the
  `WorkinWithBath` example now starts with ` ```bsl `.
- T107 fixed the real `shclang_ru.hbk` Markdown/TOC regression where same-page links on
  `MainXBase` lost source HTML fragments. Markdown link rewriting now keeps TOC lookup path-only
  but appends the original `#fragment` to the final relative Markdown target. UAT-HBK-012 passed,
  and `Основные понятия XBASE` now exports links such as `index.md#FieldsRecords`,
  `index.md#WorkWithIndexFile` and `index.md#constraint`.

## Post-T29 Runtime Regression To Fix

The following release-profile measurements were taken after T29 to decide the next implementation
task. Raw worktree exports and temporary comparison worktrees were service data and were not kept as
durable artifacts.

| Source / revision | Exit | Elapsed, s | Peak RSS, KiB | Notes |
| --- | ---: | ---: | ---: | --- |
| `shcntx_root.hbk` / T24 `e892da4` | 0 | 2.89 | 119936 | last accepted release performance baseline class |
| `shcntx_root.hbk` / T28 `c3ff0df` | 0 | 3.53 | 119808 | before T29 table/event metadata export |
| `shcntx_root.hbk` / T29 `8da6a7c` | 0 | 11.04 | 119808 | post-T29 regression |
| `shcntx_ru.hbk` / T28 `c3ff0df` | 0 | 4.80 | 132352 | before T29 table/event metadata export |
| `shcntx_ru.hbk` / T29 `8da6a7c` | 0 | 12.70 | 132352 | post-T29 regression |
| `shcntx_ru.hbk` / T32 HEAD before T30 | 0 | 8.00 | 181124 | schema v5 pre-fix baseline |
| `shcntx_root.hbk` / T32 HEAD before T30 | 0 | 7.71 | 126064 | schema v5 pre-fix baseline |
| `shcntx_ru.hbk` / T30 fixed | 0 | 4.76 | 167452 | one extraction-scope TOC owner index |
| `shcntx_root.hbk` / T30 fixed | 0 | 3.62 | 131748 | one extraction-scope TOC owner index |
| `shcntx_ru.hbk` / T31 remeasured | 0 | 4.96 | 151644 | no additional parser change justified |
| `shcntx_root.hbk` / T31 remeasured | 0 | 3.68 | 128828 | no additional parser change justified |
| `shcntx_root.hbk` / T31 repeat | 0 | 3.90 | 127780 | byte-identical repeat export |

Primary suspected hot path:

- `syntax-helper-extract::reader::query_table_owner` resolves every table field and table parameter
  owner through `Toc::find_by_html_path`.
- `Toc::find_by_html_path` calls `flat_pages()` and therefore rebuilds a flattened TOC vector on
  each lookup.
- T29 exports 588 table fields and 78 table parameters per Syntax Assistant source, so this creates
  hundreds of full TOC flattening passes per `syntax-helper` run.
- T30 fixed this hot path in `syntax-helper-extract` without changing consumer JSON: both
  post-fix exports were byte-identical to the pre-fix T32 exports and kept 588 table fields,
  78 table parameters and 4 diagnostics per locale.

Secondary candidate areas after T30:

- repeated `section_text` / `section_html` scans over expanded boundary labels;
- `section_facts` extraction for availability, examples, see-also and version facts;
- `parse_variant_signatures` probing before ordinary signature parsing;
- rubric-parameter parsing before plain text fallback.

T31 remeasured these candidate areas as residual risk rather than changing them speculatively. The
post-T30 path already returned to the T28/T30 class, so no parser-helper rewrite was accepted.

## T24 Durable Conclusions

Post-T24 Syntax Assistant extraction was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

Both commands exited successfully through the built debug binary. The T24 export directories were
byte-identical to `target/t23-prod-measurements/exports/shcntx-ru` and
`target/t23-prod-measurements/exports/shcntx-root`; the repeated root export was also
byte-identical.

Each source book produced:

- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 diagnostics

Resource results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes by `wc -c` |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 18.40 | 134528 | 21946830 |
| `shcntx_root.hbk` | 0 | 14.09 | 122108 | 12265898 |
| `shcntx_root.hbk` repeat | 0 | 14.34 | 122112 | 12265898 |

Release-profile resource results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes by `wc -c` |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 3.38 | 151136 | 21946830 |
| `shcntx_root.hbk` | 0 | 2.57 | 119936 | 12265898 |
| `shcntx_root.hbk` repeat | 0 | 2.42 | 119936 | 12265898 |

The release binary measured `3040152` bytes. Release exports stayed byte-identical to the local T23
production exports and to the release root repeat.

T24 accepted only measurement-stable micro-optimizations: ZIP entry buffer pre-sizing,
allocation-free exact parameter matching, `HashMap` TOC lookup and lean streaming-export parsing for
consumer-omitted navigation fields. T24 rejected `HashSet` visited tracking, empty-source streaming
records and the attempted single-pass HTML text-normalization rewrite because they did not preserve
the accepted resource profile.

## Standard Verification Gates

For implementation tasks, choose the narrowest relevant gate set and run the task-specific
verification from `IMPLEMENTATION_TODO.md`.

Common gates:

```bash
cargo fmt
cargo test --workspace
cargo check -p hbk-container
cargo check -p hbk-book
cargo check -p hbk-docs
cargo check -p syntax-helper-model
cargo check -p syntax-helper-extract
cargo check -p hbk-syntax-export
cargo check -p v8-context-hbk-cli
git diff --check
```

## CLI Smoke Commands

Run when the target-platform fixtures exist:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --path "$(cat tests/fixtures/known-pages/fmtdui_ru.page)"
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/ru
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/context/en
```

Negative CLI smoke:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect target/does-not-exist.hbk
```

The negative smoke must return non-zero and produce a readable diagnostic.

## T9 Durable Conclusions

Syntax Assistant extraction was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

Both commands exited successfully. Each source book produced:

- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values

The `_root` source exported as locale `en`.

Known parser gaps from that pass:

- 703 `UNKNOWN_PAGE_CLASS` diagnostics in each Syntax Assistant source.
- Most known gaps were global context event pages and common table color palette pages.

These gaps make the current export useful for integration experiments, but not a final stable
platform API contract.

## T10 Durable Conclusions

All-HBK smoke covered every `*.hbk` file under `/opt/1cv8/x86_64/8.5.1.1150/`.

Results:

- 116 files discovered.
- 116 `inspect` successes.
- 116 `toc --format json` successes.
- No fatal failures.
- No unsupported structures reported by the generic smoke commands.

## T15 Durable Conclusions

Post-T15 Syntax Assistant extraction was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

Both commands exited successfully through the built debug binary. Each source book produced:

- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

Resource results:

| Source | Elapsed, s | Peak RSS, KiB |
| --- | ---: | ---: |
| `shcntx_ru.hbk` | 19.26 | 590988 |
| `shcntx_root.hbk` | 14.62 | 324476 |

The T15 pass keeps the canonical export shape from FR-EXPORT-001: consumer record-family files do
not expose HBK navigation or per-record provenance, while `diagnostics.json` keeps parser source
context. The remaining `shcntx_ru.hbk` peak remains above 500 MiB and requires T16 attribution before
the next optimization slice is selected.

## T16 Durable Conclusions

Post-T15 Syntax Assistant memory attribution was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

Both source books were available and no fixture-backed command was skipped.

Actual debug CLI results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 18.64 | 588892 | 21950926 |
| `shcntx_root.hbk` | 0 | 14.07 | 324352 | 12269994 |

Attribution probe conclusions:

- `extract` reaches the same peak class as the full export path for `shcntx_ru.hbk`.
- JSON export adds no material high-water RSS after extraction.
- `HbkBook::open` still has a lower-level container/FileStorage opening spike, but that is not the
  next slice most likely to reduce the current `shcntx_ru.hbk` peak.

T16 selects Variant C for T17: streaming extraction into record-family sinks for the export command
path while keeping the in-memory model available when parser/tests need the full domain aggregate.

## T17 Durable Conclusions

Variant C streaming extraction was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

Both source books were available and no fixture-backed T17 command was skipped.

Actual debug CLI results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 20.46 | 386304 | 21946830 |
| `shcntx_root.hbk` | 0 | 18.15 | 324096 | 12265898 |

Each source book produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

The CLI export path now streams typed record-family events directly into canonical JSON writers, so
it no longer accumulates the full `PlatformContext` before export. The in-memory
`PlatformContext` path remains available as the full domain aggregate for parser/tests and uses the
same extraction core; T85 later removed the legacy public lookup-helper API from that aggregate.

The canonical export shape from FR-EXPORT-001 is preserved: consumer record-family files do not
expose HBK navigation or per-record provenance, `global-contexts.json` is not produced, and
`diagnostics.json` keeps parser source context. Two independent `shcntx_ru.hbk` exports were
compared byte-for-byte across all JSON files to verify deterministic record and diagnostic order.

## T19 Durable Conclusions

The first Variant E byte-only entity read slice was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`

All source books were available and no fixture-backed T19 command was skipped.

Open-only `HbkBook::open` probe results:

| Source | Current RSS before, KiB | Current RSS after, KiB | VmHWM before, KiB | VmHWM after, KiB |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 110868 | 108332 | 383232 | 131328 |
| `shcntx_root.hbk` | 98412 | 95888 | 321408 | 119168 |

Full debug CLI results:

| Source | Phase | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | before | 0 | 28.40 | 386048 | 21946830 |
| `shcntx_ru.hbk` | after | 0 | 21.19 | 168692 | 21946830 |
| `shcntx_root.hbk` | before | 0 | 32.36 | 324352 | 12265898 |
| `shcntx_root.hbk` | after | 0 | 16.11 | 144500 | 12265898 |

Each post-T19 source book still produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

Small-book smoke passed for `inspect` and `toc --format json` on `fmtdui_root.hbk` and
`fmtdui_ru.hbk`; TOC output parsed as JSON and inspect output still included `PackBlock`,
`FileStorage` and `Book`.

The byte-only path removed the majority of the open-time high-water mark, so acceptance does not
require a follow-up seekable direct `FileStorage` view from T19.

## T20 Durable Conclusions

The direct seekable `FileStorage` view evaluation was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`

All source books were available and no fixture-backed T20 command was skipped.

Fresh-process attribution results:

| Source | Mode | Exit | Elapsed, s | Current RSS, KiB | VmHWM, KiB | Exact `FileStorage` bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `container-open` | 0 | 0.00 | 2800 | 2800 | n/a |
| `shcntx_ru.hbk` | `file-storage-copy` | 0 | 0.02 | 78740 | 78740 | 38960718 |
| `shcntx_ru.hbk` | `book-open` | 0 | 5.19 | 108324 | 131712 | 38960718 |
| `shcntx_root.hbk` | `container-open` | 0 | 0.00 | 2672 | 2672 | n/a |
| `shcntx_root.hbk` | `file-storage-copy` | 0 | 0.02 | 66468 | 66468 | 32620458 |
| `shcntx_root.hbk` | `book-open` | 0 | 5.26 | 95884 | 119296 | 32620458 |

Full debug CLI results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 17.68 | 157916 | 21950926 |
| `shcntx_root.hbk` | 0 | 13.50 | 139632 | 12269994 |

Each source book still produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

Small-book smoke passed for `inspect`, `toc --format json` and `page` on `fmtdui_root.hbk` and
`fmtdui_ru.hbk`; TOC output parsed as JSON and page output was non-empty.

On the then-current post-T19/pre-T22 baseline, the owned `FileStorage` vector was material but not
dominant. It accounted for about one third of retained `HbkBook::open` RSS and less than one
quarter of the full Syntax Assistant export peak on both measured books. A direct seekable
`FileStorage` view was not justified by that T20 evidence.

## T21 Durable Conclusions

TOC/root-discovery retained-memory attribution was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

All source books were available and no fixture-backed T21 command was skipped.

Fresh-process attribution results:

| Source | Mode | Exit | Current RSS, KiB | VmHWM / peak RSS, KiB | Retained estimate, bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `Toc` tree | 0 | 109864 | 133376 | 8367325 |
| `shcntx_ru.hbk` | retained `flat_pages` metadata | 0 | 109772 | 133248 | 2139400 |
| `shcntx_ru.hbk` | public `RootDiscovery` | 0 | 147000 | 147000 | 9177088 |
| `shcntx_ru.hbk` | `syntax_toc_index` shape | 0 | 110276 | 133120 | 5149766 |
| `shcntx_root.hbk` | `Toc` tree | 0 | 97420 | 120704 | 8332291 |
| `shcntx_root.hbk` | retained `flat_pages` metadata | 0 | 97320 | 120704 | 2139400 |
| `shcntx_root.hbk` | public `RootDiscovery` | 0 | 139520 | 139520 | 9257408 |
| `shcntx_root.hbk` | `syntax_toc_index` shape | 0 | 97808 | 120704 | 5132816 |

Both source books had 28736 TOC pages. Public root discovery found 10 roots, retained 28736 catalog
pages and produced 703 diagnostics for each source book. The `syntax_toc_index` shape contained
25883 entries for each source book.

Full debug CLI results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 19.04 | 157788 | 21950926 |
| `shcntx_root.hbk` | 0 | 14.33 | 139764 | 12269994 |

Each source book still produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

The measured retained TOC/root-discovery structures are bounded and do not justify a production
refactor. The largest T21-specific structure is the public `RootDiscovery` graph at about 9 MiB,
under 7% of the full Syntax Assistant export peak. The required public `Toc` tree is about 8 MiB,
the private traversal-index shape is about 5 MiB, and retained flat-page metadata is about 2 MiB.
No runtime code change was made.

## T22 Durable Conclusions

Lower-level book-state retention was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

All source books were available and no fixture-backed T22 command was skipped.

Fresh-process attribution results:

| Source | Mode | Before RSS, KiB | After RSS, KiB | Before VmHWM, KiB | After VmHWM, KiB |
| --- | --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `book-open` | 109748 | 70936 | 132992 | 132864 |
| `shcntx_root.hbk` | `book-open` | 97264 | 64700 | 120448 | 120448 |
| `shcntx_ru.hbk` | `root-discovery` | 146796 | 108016 | 146796 | 132992 |
| `shcntx_root.hbk` | `root-discovery` | 139436 | 106868 | 139436 | 120448 |

Full debug CLI results:

| Source | Exit | Before elapsed, s | After elapsed, s | Before peak RSS, KiB | After peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 19.02 | 17.97 | 168800 | 134656 | 21950926 |
| `shcntx_root.hbk` | 0 | 14.79 | 13.65 | 140624 | 122112 | 12269994 |

Each source book still produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

The pre-change and post-change export directories were byte-identical for both source books. The
only production refactor justified by T22 evidence was releasing `HbkContainer` from `HbkBook` after
book metadata, TOC and `FileStorage` bytes are extracted.

This baseline shift invalidates the T20 percentage claim for the current `HbkBook::open` path: the
same retained `FileStorage` vector is now about half of current open-path RSS after the container
mmap is released. The T20 no-go decision remains pre-T22 evidence against a broad seekable
`FileStorage` change for the full export peak; it should not be reused as current open-path
attribution without a post-T22 measurement pass.

## T23 Durable Conclusions

Post-T22 `FileStorage` lifetime re-evaluation and the user-directed production follow-up were
validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

All source books were available and no fixture-backed T23 measurement was skipped. Initial
measurement-only logs were written under `target/t23-measurements/`; post-production logs were
written under `target/t23-prod-measurements/`. These directories are service data and are not a
durable source of truth.

Post-production fresh-process attribution results:

| Source | Mode | Exit | Elapsed, s | Current RSS, KiB | VmHWM / peak RSS, KiB | Exact `FileStorage` bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `file-storage-len` | 0 | 0.02 | 80416 | 80416 | 38960718 |
| `shcntx_ru.hbk` | `book-open` | 0 | 5.92 | 33164 | 133376 | 38960718 |
| `shcntx_root.hbk` | `file-storage-len` | 0 | 0.02 | 68012 | 68012 | 32620458 |
| `shcntx_root.hbk` | `book-open` | 0 | 5.68 | 32928 | 120832 | 32620458 |

Repeated page-read and extractor-access results:

| Source | Mode | Exit | Elapsed, s | Current RSS, KiB | VmHWM / peak RSS, KiB | Counts |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `shcntx_ru.hbk` | `page-read-all` | 0 | 9.39 | 98000 | 133120 | 25878 pages, 4 missing entries |
| `shcntx_root.hbk` | `page-read-all` | 0 | 9.06 | 91744 | 120704 | 25878 pages, 4 missing entries |
| `shcntx_ru.hbk` | `extract-counts` | 0 | 17.34 | 73308 | 133248 | 1 global context, 24836 consumer records, 703 diagnostics, 25540 total items |
| `shcntx_root.hbk` | `extract-counts` | 0 | 13.96 | 110328 | 120960 | 1 global context, 24836 consumer records, 703 diagnostics, 25540 total items |

The page-read probe used a single `FileStorageReader`, skipped `2851` empty TOC paths and counted
missing entries instead of treating known missing source pages as a storage failure.

Full debug CLI results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 19.63 | 154504 | 21950926 |
| `shcntx_root.hbk` | 0 | 15.15 | 122240 | 12269994 |

Each source book still produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

The initial T23 pass confirmed that the retained `FileStorage` vector had become about half of
current `HbkBook::open` RSS after T22. The production follow-up removed that retained vector from
`HbkBook`: current RSS after `book-open` dropped from `71068 KiB` to `33164 KiB` for
`shcntx_ru.hbk` and from `64820 KiB` to `32928 KiB` for `shcntx_root.hbk`. The exact
`FileStorage` bytes are still `38960718` and `32620458`, but they are now owned only by
short-lived `FileStorageReader` values.

Open-path VmHWM remains in the previous class because `HbkBook::open` still validates the
`FileStorage` entity body to preserve existing failure timing. Full export peak did not show a
material win: `shcntx_root.hbk` stayed at `122240 KiB`, while `shcntx_ru.hbk` measured
`154504 KiB` in this run. A direct/seekable block-backed `FileStorage` view remains unimplemented;
the accepted T23 production effect is limited to removing retained `FileStorage` bytes from
`HbkBook` and using path-backed reader lifetimes.

## T25 Durable Conclusions

Locale-aware Syntax Assistant section parsing was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

Both source books were available. T25 kept the canonical FR-EXPORT-001 consumer shape unchanged:
consumer record-family files still omit HBK provenance, TOC paths, HTML paths, page titles and
duplicate navigation-link catalogs, and the JSON `schema_version` remains `1`.

Post-T25 CLI export counts remained stable for both books:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

The parser now recognizes root/English `Type:` and `Returned value:` sections. Root/English empty
type-reference counts dropped from complete misses to parity with the remaining source-data gaps:

| File / field | RU before | RU after | EN/root before | EN/root after |
| --- | ---: | ---: | ---: | ---: |
| `global-methods.json` / empty `return_types` | 143 | 143 | 500 | 147 |
| `type-methods.json` / empty `return_types` | 2494 | 2494 | 6702 | 2520 |
| `global-properties.json` / empty `type_refs` | 1 | 1 | 101 | 1 |
| `type-properties.json` / empty `type_refs` | 169 | 169 | 10732 | 165 |

`XMLСтрока` / `XMLString`, `Массив.Добавить` / `Array.Add` and `ОткрытьФорму` / `OpenForm` retain
the T25 return/property/parameter type facts in both locales. Description and parameter-description
fields no longer include the raw T25 boundary labels for availability, examples, see-also,
available-since, returned-value or parameter sections in the UAT-SH-007 export checks. A repeated
`shcntx_ru.hbk` export was compared with `diff -qr` to verify deterministic output.

Structured overload variant metadata remains pending for T27.

## T26 Durable Conclusions

Structured Syntax Assistant section facts were validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

T26 changed the consumer JSON shape and raised the canonical export schema to `schema_version: 2`.
Consumer record-family files now expose structured `availability`, `examples`, `see_also` and
`available_since` fields when the source page contains those facts. Consumer records still omit HBK
provenance, TOC paths, HTML paths, page titles and duplicate navigation-link catalogs; `see_also`
consumer targets expose names only.

Post-T26 CLI export counts remained stable for both books:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

Post-T26 structured fact counts from full CLI exports:

| File | RU availability | RU examples | RU see_also | RU available_since | EN availability | EN examples | EN see_also | EN available_since |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `global-methods.json` | 499 | 185 | 180 | 500 | 499 | 193 | 180 | 500 |
| `global-properties.json` | 101 | 0 | 21 | 101 | 101 | 0 | 21 | 101 |
| `platform-types.json` | 2384 | 140 | 895 | 2532 | 2384 | 140 | 900 | 2532 |
| `type-methods.json` | 6586 | 1103 | 690 | 6701 | 6587 | 1102 | 691 | 6701 |
| `type-properties.json` | 9918 | 19 | 222 | 10731 | 9918 | 31 | 222 | 10731 |
| `constructors.json` | 2 | 54 | 10 | 315 | 2 | 55 | 10 | 315 |
| `enums.json` | 713 | 3 | 341 | 713 | 713 | 2 | 341 | 713 |
| `enum-values.json` | 28 | 0 | 36 | 3109 | 28 | 3 | 36 | 3109 |

UAT-SH-001, UAT-SH-002, UAT-SH-003 and UAT-SH-008 passed on schema version 2 exports. A repeated
`shcntx_ru.hbk` export was compared with `diff -qr` to verify deterministic output.

## T27 Durable Conclusions

Structured Syntax Assistant syntax variants were validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

T27 changed the consumer JSON shape and raised the canonical export schema to `schema_version: 3`.
Consumer signatures now expose `variant` metadata with `title` and `description` when the source
page contains `Вариант синтаксиса:` / `Syntax variant:` sections. Consumer records still omit HBK
provenance, TOC paths, HTML paths, page titles and duplicate navigation-link catalogs.

Post-T27 CLI export counts remained stable for both books:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

Post-T27 structured variant counts from full CLI exports:

| File | RU records with variants | RU variant signatures | EN/root records with variants | EN/root variant signatures |
| --- | ---: | ---: | ---: | ---: |
| `global-methods.json` | 23 | 60 | 23 | 60 |
| `type-methods.json` | 243 | 544 | 243 | 544 |
| `constructors.json` | 0 | 0 | 0 | 0 |

`ДокументDOM.СоздатьРазыменовательПИ` / `DOMDocument.CreateNSResolver` exports four structured
variant signatures in both locales with parameters attached to the owning variant and return types
preserved. `ОткрытьФорму` / `OpenForm` exports both source syntax variants. Signature text
containing raw overload labels or returned-value labels stayed at zero before and after T27. A
repeated `shcntx_ru.hbk` export was compared with `diff -qr` to verify deterministic output.

## T28 Durable Conclusions

Syntax Assistant diagnostic-family classification was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

T28 did not change the consumer JSON shape or canonical schema version. Record-family counts
remained stable for both source books:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 diagnostics

The 703 diagnostics in each locale are now classified with family-specific codes:

| Diagnostic code | RU count | EN/root count |
| --- | ---: | ---: |
| `UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE` | 4 | 4 |
| `OUT_OF_SCOPE_GLOBAL_CONTEXT_EVENT` | 33 | 33 |
| `OUT_OF_SCOPE_TABLE_FIELD` | 588 | 588 |
| `OUT_OF_SCOPE_TABLE_PARAMETER` | 78 | 78 |

The previous audited `UNKNOWN_PAGE_CLASS` count is now zero for both locales. Direct
`objects/Global context/*.html` method-like TOC entries remain visible as in-scope unsupported
diagnostics rather than incomplete synthesized records because the audited HBK FileStorage archives
do not contain their page HTML. At T28 completion, global-context events, table fields and table
parameters were explicitly out of scope; T29 later promoted those three families into typed export
records.

## T29 Durable Conclusions

Syntax Assistant event/table metadata support was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

T29 changed the consumer JSON file inventory and raised the canonical export schema to
`schema_version: 4`. Existing record-family counts remained stable for both source books, and each
source book now additionally produced:

- 33 global context events
- 588 query/table fields
- 78 query/table parameters
- 4 diagnostics

The remaining diagnostics in each locale are all `UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE`.
`UNKNOWN_PAGE_CLASS`, `OUT_OF_SCOPE_GLOBAL_CONTEXT_EVENT`, `OUT_OF_SCOPE_TABLE_FIELD` and
`OUT_OF_SCOPE_TABLE_PARAMETER` are absent from the T29 exports.

New required files:

- `global-context-events.json`
- `table-fields.json`
- `table-parameters.json`

The new consumer records omit source HBK paths, TOC paths, HTML paths and page titles. Parser
provenance remains internal and in `diagnostics.json`.

## T32 Durable Conclusions

Lean schema version 5 was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

T32 changed the consumer JSON shape and raised the canonical export schema to
`schema_version: 5`. Both source books exported 12 JSON files including `metadata.json` and
`diagnostics.json`; `enum-values.json` is no longer emitted. `enums.json` now nests all 3110 enum
values under their owning enum records in both locales.

Record-family counts remained stable for both source books:

| File | RU records | EN/root records |
| --- | ---: | ---: |
| `global-methods.json` | 500 | 500 |
| `global-properties.json` | 101 | 101 |
| `global-context-events.json` | 33 | 33 |
| `platform-types.json` | 2533 | 2533 |
| `type-methods.json` | 6702 | 6702 |
| `type-properties.json` | 10732 | 10732 |
| `table-fields.json` | 588 | 588 |
| `table-parameters.json` | 78 | 78 |
| `constructors.json` | 445 | 445 |
| `enums.json` | 713 | 713 |
| `diagnostics.json` | 4 | 4 |

Nested enum-value counts:

| Source | Nested enum values | Enum records with values | Value-specific `availability.since` |
| --- | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 3110 | 709 | 458 |
| `shcntx_root.hbk` | 3110 | 709 | 458 |

Consumer shape changes verified by UAT:

- platform API consumer records omit `null` fields and empty arrays across every record family;
- `owner` fields are primary-name strings;
- `type_refs` and `return_types` are arrays of type-name strings, including signature parameters;
- recognized version facts are emitted as `availability.since`; top-level `available_since` is not
  emitted;
- `see_also` is an array of target primary-name strings;
- property `usage` is normalized to `Read`, `Write`, `ReadWrite` or `Unknown`;
- leading property type prose such as `Тип: ... .` / `Type: ... .` is stripped from descriptions;
- method, global context event and constructor signatures do not emit `text`;
- syntax-variant `title` and `description` are direct signature fields rather than nested
  `variant`;
- nested enum value records omit `owner` and include `availability.since` only when the value
  version differs from the owning enum version.

Generated export byte totals by `wc -c` were `18555205` bytes for the Russian export and
`11911554` bytes for the root/English export. The remaining diagnostics in each locale are all
`UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE`. T32 did not change parser behavior or the post-T29
runtime-regression attribution owned by T30.

## T33 Durable Conclusions

Schema version 6 was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

T33 changed the consumer JSON shape and raised the canonical export schema to
`schema_version: 6`. Record-family counts remained stable for both source books:

| File | RU records | EN/root records |
| --- | ---: | ---: |
| `global-methods.json` | 500 | 500 |
| `global-properties.json` | 101 | 101 |
| `global-context-events.json` | 33 | 33 |
| `platform-types.json` | 2533 | 2533 |
| `type-methods.json` | 6702 | 6702 |
| `type-properties.json` | 10732 | 10732 |
| `table-fields.json` | 588 | 588 |
| `table-parameters.json` | 78 | 78 |
| `constructors.json` | 445 | 445 |
| `enums.json` | 713 | 713 |
| `diagnostics.json` | 4 | 4 |

Consumer shape and data-quality checks verified by UAT:

- type-reference facts are emitted as `types`; legacy `type_refs` is absent from consumer records;
- callable return facts are emitted as `return`; legacy `return_types` is absent from consumer
  records;
- platform API consumer records still omit `null` fields and empty arrays across every record
  family;
- inline `Пример:` / `Example:` source sections embedded in descriptions are extracted as examples
  and do not absorb later availability/context text;
- code examples generated from syntax-colored HTML no longer contain extra spaces around dots,
  commas, semicolons, brackets or parentheses, including multiline string-continuation examples
  such as `ЗадачаОбъект.<Имя задачи>.Записать` with `ОписаниеОшибки(), 60);`;
- see-also source pairs such as owner link plus method/property link are exported as composed
  `Owner.Member` strings, for example `ИзбранноеРаботыПользователя.Вставить` and
  `Глобальный контекст.ИсторияРаботыПользователя`;
- consumer records still omit HBK provenance, TOC paths, HTML paths and page titles; diagnostics
  remain provenance-rich for parser maintenance.

Generated export byte totals by `wc -c` were `18460442` bytes for the Russian export and
`11809759` bytes for the root/English export. The remaining diagnostics in each locale are all
`UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE`.

## First Delivery Success Metrics

The project is successful for the first delivery when:

- the small real HBK smoke pair opens and exposes expected core entities;
- all-HBK smoke covers every target-platform `*.hbk` file;
- `shcntx_ru.hbk` and `shcntx_root.hbk` extraction returns non-empty records for all top-level model
  families;
- `_root` exports as locale `en`;
- parser warnings and unresolved pages are counted and source-linked;
- every specialized parser has at least one representative fixture;
- downstream tooling can consume canonical JSON without reading HBK directly;
- stable API/export commitments remain deferred until parser evidence and consumer feedback justify
  them.

## T18 Query CLI Baseline

T18 implemented the first local Syntax Assistant query slice:

- `syntax-helper-search` builds a rebuildable SQLite/FTS5 index directly from typed extraction
  results;
- `v8-context-hbk syntax index` writes `index.sqlite` through a temporary replacement file and
  writer lock;
- `syntax get`, `syntax search` and `syntax related` open the index read-only and do not parse
  `shcntx_*.hbk` on the query path;
- default index path resolution is `--index` / `--output`, then
  `V8_CONTEXT_HBK_SYNTAX_INDEX`, then `.v8-context-hbk/syntax/index.sqlite`;
- deterministic JSON output was verified by repeated `cmp` checks for exact lookup, keyword search,
  fuzzy search and relationship traversal.

Measured on `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` in debug build:

| Command mode | Measurement |
| --- | ---: |
| `syntax index` build | 40.303-43.012 s |
| exact lookup | 0.00 s |
| keyword search | 0.11-0.12 s |
| fuzzy search | 0.44-0.58 s |
| relationship search | 0.07-0.08 s |

The generated index contained 25,594 searchable documents, including event facts. UAT-SH-004,
UAT-SH-005, UAT-SH-006 and
UAT-SH-015 passed against `target/uat/sh-search-ru.sqlite`; UAT-SH-004 default-path resolution also
created `.v8-context-hbk/syntax/index.sqlite` and resolved it from `syntax get` without `--index`.
The SKD relationship output included constructor `Новый ОтборКомпоновкиДанных()`, `Элементы`,
`Добавить`, and filter-item fields `ЛевоеЗначение`, `ВидСравнения`, `ПравоеЗначение` and
`Использование`.

T41 rebuilt the Russian index after replacing provenance-shaped document ids with semantic
record-family identities. The debug build command
`v8-context-hbk syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output
target/uat/sh-search-ru.sqlite` completed without SQLite uniqueness failures in 63.703 s and
produced 25,082 documents and 65,455 relations.

Read-only SQLite checks confirmed:

- no `documents.id` contains `.html`, `/` source paths or TOC duplicate-title marker
  `#&^@^%&*^#`;
- duplicated accounting-register query-table identifiers use minimal semantic table-family
  variants such as `Таблицы регистра бухгалтерии (без поддержки корреспонденции)` and
  `Таблицы регистра бухгалтерии (с поддержкой корреспонденции)`;
- query-table field/parameter relation endpoints use the final query-table identity;
- form/form-extension `Параметры формы` pages are no longer indexed as `platform_type` records and
  are indexed as type properties owned by the form or extension type.

T122 rebuilt the Russian index on 2026-05-08 after changing index-build duplicate handling for
real 8.5.1 documentation defects. The debug build command
`v8-context-hbk syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output
target/repro-index.sqlite` completed in 60.231 s, emitted `DUPLICATE_DOCUMENT_ID` warnings to
`stderr`, produced 24,888 documents and left zero duplicate `documents.id` groups. SQLite checks
confirmed that the reported `МенеджерКриптографии` constructor id exists once, both
`ИспользованиеТекущейСтроки` system enum aliases exist as separate documents and duplicate
`ОбновлениеПредопределенныхДанных:PredefinedDataUpdate` source pages resolve to one document.

T123 rebuilt the Russian index on 2026-05-08 after fixing TOC-shaped query-table member owner
identity. The debug build command
`v8-context-hbk syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output
target/query-table-id-regression.sqlite` completed in 58.294 s, emitted existing
`DUPLICATE_DOCUMENT_ID` warnings to `stderr`, produced 24,970 documents and left zero duplicate
`documents.id` groups. SQLite checks confirmed that
`query_table_field:query_table:Основная таблица:<Имя общего реквизита>` is absent, no
`query_table_field` / `query_table_parameter` document uses `query_table:Основная таблица` as its
owner id, and `query_table:Задача` owns
`query_table_field:query_table:Задача:<Имя общего реквизита>`.

T124 rebuilt the Russian index on 2026-05-08 after fixing search document identity for type events
under generic `События` / `Events` TOC group nodes. The debug build command
`cargo run -p v8-context-hbk-cli -- syntax index
/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output
target/type-event-owner-regression.sqlite` completed in 60.672 s, emitted existing
`DUPLICATE_DOCUMENT_ID` warnings to `stderr`, produced 25,449 documents and left zero duplicate
`documents.id` groups. SQLite checks confirmed that
`type_event:owner:События:ОбработкаВыбора` is absent and that `ОбработкаВыбора` type-event
documents use composed semantic owners such as
`Поле формы.Расширение поля ввода`, `Форма клиентского приложения.ФормаКлиентскогоПриложения` and
`Элементы управления.Табличное поле.ТабличноеПоле`.

T126 completed ADR-0011's read-phase parent identity boundary for Syntax Assistant facts. Parent
identity for platform types, query tables and enums is filled by `syntax-helper-extract` before
records reach `SyntaxHelperSink`; `syntax-helper-search` consumes those identities for member
document ids. T127 later removed the remaining child/member fallback path and made missing child
`owner_identity` an index-build error. The debug build command
`cargo run -p v8-context-hbk-cli -- syntax index
/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/adr-0011-identity.sqlite` completed in
65.778 s, emitted existing `DUPLICATE_DOCUMENT_ID` warnings to `stderr`, produced 25,443 documents
and 63,562 relations, and left zero duplicate `documents.id` groups. SQLite checks confirmed that
`type_event:owner:События:ОбработкаВыбора` is absent and that no `query_table_field` /
`query_table_parameter` document is owned by `query_table:Основная таблица`.

T127 removed the remaining consumer-side parent identity repair. Child/member domain records now
carry `owner_identity`; `syntax-helper-search` fails index build when that child parent identity is
missing, and `hbk-syntax-export` groups query-table members and enum values only by precomputed
identity. `syntax-helper-extract` also reuses query-table records parsed during the parent-identity
prepass instead of loading the same query-table page again during stream emission; real
`/formparams/` pages are resolved through their platform parent path.

T127 debug real-corpus verification used:

- `cargo run -p v8-context-hbk-cli -- syntax index
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t127-parent-identity.sqlite`
  completed in 37.506 s, emitted existing `DUPLICATE_DOCUMENT_ID` warnings, produced 25,415
  documents and 64,981 relations.
- `cargo run -p v8-context-hbk-cli -- syntax index
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output
  target/t127-parent-identity-root.sqlite` completed in 29.492 s, emitted existing
  `DUPLICATE_DOCUMENT_ID` warnings, produced 25,415 documents and 68,008 relations.
- SQLite checks over both indexes found zero final duplicate `documents.id` groups, zero
  query-table field/parameter documents under generic `query_table:Основная таблица` /
  `query_table:Main table`, zero constructor callables without `owner_type_id`, and zero
  constructor result type references without `target_type_id`.

T128 clarified the dependency-based static-analysis integration surface as a spec-only decision.
Rust static-analysis consumers should integrate through Cargo dependencies on
`context-resolver-core` and concrete source adapters such as `context-resolver-search`, using
`syntax-helper-search` only as a local index open/build primitive. HTTP, daemon, MCP, CLI-spawn and
JSON-over-process transports are out of scope for resolver hot-path lookup. `hbk-book` and
`syntax-helper-extract` may participate in setup/index-refresh flows, while SQLite tables, HBK
paths, Syntax Assistant HTML parser internals, CLI wiring, export crates, documentation-site code
and web-app code remain outside the static-analysis library contract. This did not change Rust
code, CLI behavior, provider JSON, SQLite schema or consumer export JSON.

T129 added the first code-level convenience slice for that surface. `context-resolver-search` now
opens read-only provider indexes through adapter-level constructors:
`PlatformSearchSource::open_read_only`, `PlatformSearchSource::open_read_only_with_source_id`,
`LanguageSearchSource::open_read_only`, `LanguageSearchSource::open_shlang_read_only`,
`LanguageSearchSource::open_shquery_read_only` and `LanguageSearchSource::open_dcsui_read_only`.
The database schema, index build ownership and lower-level `SearchIndex` implementation remain in
`syntax-helper-search`; the new constructors only remove the need for lookup-only analyzer code to
import `syntax-helper-search` directly just to open an existing index.

T130 proved that surface with a consumer-style integration smoke in
`crates/context-resolver-search/tests/static_analysis_consumer_smoke.rs`. The setup phase builds a
small deterministic provider index through `syntax-helper-search::SearchIndexBuilder` and existing
domain/language fixture builders. The lookup phase composes `context-resolver-core` and
`context-resolver-search` only, opens the existing index through adapter-level read-only
constructors, and resolves one platform type, one platform member/callable path and one BSL
language fact. No separate smoke crate, facade crate, CLI command, SQLite schema change, provider
JSON change or export JSON change was added. Verification passed `cargo fmt --all --check` and
`cargo test -p context-resolver-search`.

T42 changed only the index build data flow. `syntax index` now streams extraction records into a
search-index builder and inserts relations into SQLite without retaining the full `PlatformContext`,
complete search document vector and complete relation vector together. The SQLite schema, atomic
temporary rebuild, writer lock and read-only query path stayed unchanged.

Release-profile T42 measurements used the built `target/release/v8-context-hbk` binary and GNU
`time`:

| Source / slice | Exit | Elapsed, s | Peak RSS, KiB | Documents | Relations | SQLite size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` / before T42 | 0 | 20.25 | 617872 | 25082 | 65455 | 145M |
| `shcntx_ru.hbk` / after T42 | 0 | 18.80 | 269612 | 25082 | 65455 | 145M |
| `shcntx_root.hbk` / before T42 | 0 | 15.17 | 443532 | 25062 | 68670 | 65M |
| `shcntx_root.hbk` / after T42 | 0 | 14.55 | 239972 | 25062 | 68670 | 65M |

Read-only checks on the rebuilt Russian index found zero malformed semantic ids, zero form-parameter
`platform_type` documents, zero relation endpoints missing from `documents`, two duplicated
accounting-register query-table semantic variants and 28 query-table field edges using those final
variant identities.

Representative release-profile query checks against the rebuilt Russian index remained within
NFR-QUERY-001: exact lookup for `ОтборКомпоновкиДанных` measured `0.00 s`, keyword search for
`отбор скд` measured `0.04 s`, fuzzy search for `ОтборКомпоновкиДаных` measured `0.04 s`, and
relationship search for `ОтборКомпоновкиДанных` measured `0.01 s`.

T43 kept the T42 staged extraction/index data flow and reduced SQLite writer overhead. The writer
now reuses prepared insert statements for documents, lookup names, FTS rows and relations; creates
ordinary B-tree lookup/relation indexes after bulk insertion; and uses temp-rebuild-only SQLite
settings while constructing the disposable replacement database. The active index path is still
updated only by validated atomic rename under the existing writer lock.

Release-profile T43 measurements:

| Source / slice | Exit | Elapsed, s | Peak RSS, KiB | Documents | `document_names` | Relations | SQLite size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` / export comparison | 0 | 5.74 | 197768 | n/a | n/a | n/a | 19M JSON dir |
| `shcntx_ru.hbk` / post-T42 triage | 0 | 20.48 | 269828 | 25082 | 132646 | 65455 | 145M |
| `shcntx_ru.hbk` / after prepared statements | 0 | 17.47 | 269756 | 25082 | 132646 | 65455 | 145M |
| `shcntx_ru.hbk` / after T43 | 0 | 16.30 | 269632 | 25082 | 132646 | 65455 | 139M |
| `shcntx_root.hbk` / after T43 | 0 | 12.79 | 243992 | 25062 | 47001 | 68670 | 62M |

Read-only checks on the final T43 Russian index found all four expected ordinary indexes present and
zero relation endpoints missing from `documents`. A representative keyword query against the final
Russian index measured `0.04 s` and returned `Отбор` as the first hit, matching the accepted query
behavior class.

T44 changed FTS population from direct row-by-row writes into `document_fts` to SQLite FTS5
external-content rebuild. The replacement database now bulk-loads searchable text into the ordinary
`document_search` table, runs `INSERT INTO document_fts(document_fts) VALUES ('rebuild')`, validates
the completed database and then atomically replaces the target index. The query-index artifact
remains one SQLite file produced by one `syntax index` command; the task explicitly did not split
the index into mandatory and heavy/optional artifacts.

Release-profile T44 measurements:

| Source / slice | Exit | Elapsed, s | Peak RSS, KiB | Documents | `document_names` | `document_search` | FTS rows | Relations | SQLite size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` / after T43 | 0 | 16.30 | 269632 | 25082 | 132646 | n/a | 25082 | 65455 | 139M |
| `shcntx_ru.hbk` / T44 external-content rebuild | 0 | 15.93 | 269696 | 25082 | 132646 | 25082 | 25082 | 65455 | 139M |
| `shcntx_ru.hbk` / T44 contentless measured variant | 0 | 15.94 | 269700 | 25082 | 132646 | n/a | 25082 | 65455 | 126M |
| `shcntx_root.hbk` / after T43 | 0 | 12.79 | 243992 | 25062 | 47001 | n/a | 25062 | 68670 | 62M |
| `shcntx_root.hbk` / T44 external-content rebuild | 0 | 12.52 | 243860 | 25062 | 47001 | 25062 | 25062 | 68670 | 62M |

The selected T44 path is external-content FTS because it best preserved the existing query shape and
was marginally faster than the measured contentless variant. Contentless FTS reduced Russian SQLite
size from `139M` to `126M`, but was not selected because it was slower on the accepted benchmark and
required extra rowid mapping on the query path. Read-only checks on the final T44 Russian and root
indexes found search-index schema version `2`, matching `documents` / `document_search` /
`document_fts` counts and zero broken relation endpoints. A representative release keyword query
against the rebuilt Russian index measured `0.03 s`.

T45 added the direct constructor lookup command as a query-CLI usability wrapper over existing
owner-to-constructor relations. `v8-context-hbk syntax constructors <TYPE>` opens the same resolved
read-only index as `syntax get/search/related`; text output prints constructor signatures directly,
while JSON output returns the full deterministic search-hit records. The release binary was verified
against the default local index with `HTTPСоединение`, returning both accepted constructor overloads
including the overload with `<Таймаут>`, `<ЗащищенноеСоединение>` and
`<ИспользоватьАутентификациюОС>`.

T46 added opt-in detailed text output for constructor lookup. `v8-context-hbk syntax constructors
<TYPE> --details` keeps the same resolved read-only index path and prints each constructor signature
with available owner and description context. Signature-only text output remains the default, and
JSON output remains the full deterministic search-hit records.

T47 fixed Syntax Assistant HTML chapter extraction for parser data quality. Sections headed by
`V8SH_chapter` are now located from structural chapter markers and bounded by the next structural
chapter marker or HTML footer instead of by plain label text inside the section body. This preserves
constructor parameters when parameter descriptions contain inline labels such as `Примечание:`; the
motivating `HTTPСоединение` constructor page should expose later parameters such as
`ИспользоватьАутентификациюОС` after export or index rebuild. No consumer JSON or search-index
schema change is required.

T48 changed the provisional query-index schema to version `3` to keep public callable JSON separate
from internal FTS search terms. The `documents` table now stores `signature_json` for structured
callable facts, while `parameter_text` and `document_search.parameters` remain internal searchable
text. `syntax constructors "HTTPСоединение" --format json` no longer exposes mixed
`document.parameters`; callable parameters are returned under `signatures[].parameters[]` with
`name`, `required`, `types` and optional `description`. Compact and detailed text output continue to
print signature text for humans. The accepted T48 Russian rebuild from
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` produced `25082` documents in `64652 ms`; read-only
inspection found `schema_version=3` and matching `documents`, `document_search` and `document_fts`
counts.

T50 defined the provisional provider response contract before changing CLI serialization. The
target JSON envelope for `syntax get`, `syntax constructors`, `syntax search` and `syntax related`
uses provider `schema_version: 1`, `command`, `status`, normalized `query`, deterministic
`results[]` and `diagnostics[]`. Shared platform facts live under `results[].fact` with
export-compatible field names and shapes, while query-only metadata such as score, rank,
relationship depth, relationship path and richer owner identity lives under `results[].meta`. Owned
facts use the export-compatible `owner` string shape. Missing and ambiguous lookups are represented
through `status` and diagnostics. The reviewed current implementation sample remains the
provisional pre-envelope `SearchHit<SearchDocument>` shape, so T50 is a contract definition task and
not a CLI serialization implementation.

T52 implemented the provider envelope and analyzer-safe identity roots. `syntax get`, `syntax
constructors`, `syntax search` and `syntax related` JSON now use provider `schema_version: 1` with
`command`, `status`, `query`, `results[]` and `diagnostics[]`. Platform facts are emitted under
`results[].fact` with export-compatible `types`, `return`, `signatures` and owner string fields;
query metadata is emitted under `results[].meta`. `syntax get --id` resolves exact document
identity, and `syntax related` now accepts `--id`,
`--owner --member` and `--name`, so analyzer workflows can traverse from an exact property root such
as `type_property:platform_type:НастройкиКомпоновкиДанных:Отбор` without depending on a plain-name
root. The accepted real Russian index rebuild from
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` produced `25082` documents in `50621 ms`. Provider JSON
assertions passed for `HTTPСоединение` constructor parameters, `НастройкиКомпоновкиДанных.Отбор`,
keyword search, related-by-name, related-by-id, related-by-owner-member, missing lookup and
ambiguous lookup paths. Invalid JSON root combinations return provider `status: "unsupported"` with
an `UNSUPPORTED_QUERY` diagnostic.

T53 added BSL task scenario UAT for the ADR-0006 provider direction. The accepted debug rebuild from
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` produced `25082` documents in `54105 ms` at
`target/uat/t53-sh-search-ru.sqlite`. UAT-SH-017 now validates three source-backed BSL development
questions: `Новый HTTPСоединение(...)` constructor parameters, `НастройкиКомпоновкиДанных.Отбор`
owner/member lookup plus relationship traversal, and task-oriented discovery for
`таблица регистра бухгалтерии`. The accepted accounting-register search ranks
`query_table:РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии` first and relationship
traversal from that id exposes query-table fields such as `Регистратор` and `НомерСообщения`.
T53 did not change code, index schema or provider JSON shape; it fixed the acceptance corpus that
future T49/T54 work must use before changing search storage or relationship coverage.

T49 compared a temporary Tantivy full-text sidecar prototype with the accepted SQLite/FTS5 query
index. The prototype was intentionally not wired into production CLI behavior: it read the
already-built SQLite `documents` / `document_search` rows, wrote a Tantivy directory under
`target/t49/`, and reported only measurement JSON. The prototype code and dependency were removed
before task completion because Tantivy was not selected. SQLite remained the control for exact
lookup, owner/member lookup, constructor lookup, deterministic provider JSON and relationship
traversal.

Release-profile T49 build measurements:

| Source / artifact | Exit | Elapsed, s | Peak RSS, KiB | Documents | Relations | Size |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` / SQLite index | 0 | 15.35 | 302124 | 25082 | 65455 | 144M |
| `shcntx_root.hbk` / SQLite index | 0 | 12.02 | 272600 | 25062 | 68670 | 65M |
| RU SQLite rows / Tantivy sidecar | 0 | 0.95 | 123120 | 25082 | n/a | 7.3M |
| root SQLite rows / Tantivy sidecar | 0 | 0.67 | 101884 | 25062 | n/a | 4.5M |

Representative SQLite query measurements against `target/t49/sqlite-ru.sqlite`:

| Query workflow | Elapsed, s |
| --- | ---: |
| exact `ОтборКомпоновкиДанных` | 0.00 |
| owner/member `НастройкиКомпоновкиДанных.Отбор` | 0.00 |
| constructor JSON `HTTPСоединение` | 0.00 |
| constructor compact text `HTTPСоединение` | 0.00 |
| constructor detailed text `HTTPСоединение` | 0.00 |
| keyword `отбор скд` | 0.04 |
| keyword `HTTP соединение` | 0.01 |
| keyword `таблица регистра бухгалтерии` | 0.03 |
| fuzzy `ОтборКомпоновкиДаных` | 0.04 |
| root fuzzy `DataCompositionFiltter` | 0.03 |
| related by name `ОтборКомпоновкиДанных` | 0.02 |
| related by owner/member `НастройкиКомпоновкиДанных.Отбор` | 0.02 |
| related constructor/type case `HTTPСоединение` | 0.01 |

The SQLite control passed deterministic repeated-output comparisons and the UAT-SH-017 provider
assertions for `HTTPСоединение` constructor parameters, `НастройкиКомпоновкиДанных.Отбор`
relationship traversal, constructor/type relationship traversal from `HTTPСоединение`, root fuzzy
lookup for `DataCompositionFiltter`, and accounting-register query-table discovery. Tantivy keyword
search was fast, and root fuzzy `DataCompositionFiltter` did find `DataCompositionFilter`, but it
did not justify replacing or splitting the accepted artifact: exact lookup, constructor lookup,
provider JSON and relationships still needed SQLite; Russian fuzzy search for
`ОтборКомпоновкиДаных` returned no hits in the prototype; and `таблица регистра бухгалтерии`
ranked generic accounting-register table variants above the UAT-SH-017 accepted top hit. The T49
decision is to retain the single SQLite/FTS5 query index and not add a storage selection knob.

Follow-up checks with MyStem 3.1 Russian lemmatization kept the same measurement-only sidecar
boundary. A temporary harness under `target/` read the accepted RU SQLite rows, generated one
lemmatized line per document with MyStem, built a Tantivy index over those lemmas and lemmatized
each query before searching. It indexed the same `25082` RU documents.

T49 MyStem/Tantivy follow-up measurements against `target/t49/sqlite-ru.sqlite`:

| Hypothesis variant | Lemmatize, ms | Index, ms | Size, bytes | `таблица регистра бухгалтерии` | `таблицы регистров бухгалтерии` | `отбор скд` | `отбор компоновки данных` | `ОтборКомпоновкиДаных` | `HTTP соединение` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| MyStem lemmas | 2855 | 627 | 6262902 | 1 | 2 | miss | miss | miss | 1 |
| MyStem `-d` disambiguation | 11528 | 550 | 5390585 | 1 | 1 | miss | miss | miss | 1 |
| Identifier split in indexed text and query | 6552 | 710 | 8038361 | 1 | 4 | miss | miss | 10 | 2 |
| Identifier split plus word-to-identifier query terms | 6913 | 640 | 7860588 | miss | miss | miss | 2 | 1 | miss |
| Identifier split plus domain query expansion | 6646 | 732 | 8031990 | miss | miss | 5 | 2 | 2 | miss |
| Identifier split, query expansion and domain rerank | 6561 | 778 | 7950694 | 1 | 1 | 1 | 1 | 1 | 1 |

The table reports the rank of the expected BSL-provider target fact in the top 10, or `miss` when
the expected id was not in the top 10. MyStem lemmatization alone fixed inflected
accounting-register wording and `HTTP соединение`, but not compact identifiers or SKD intent.
`mystem -d` improved the plural accounting-register wording (`таблицы регистров бухгалтерии`) from
rank 2 to rank 1, but cost roughly 4x more lemmatization time and did not help SKD or compact-name
queries. Identifier splitting helped the compact typo by exposing `Отбор Компоновки Даных`, but by
itself only moved `ОтборКомпоновкиДаных` to rank 10. Adding compounded BSL-style query terms fixed
that compact typo, but harmed accounting-register and HTTP ranking because MyStem treats compounded
Russian identifiers as unknown lexical terms. Domain query expansion helped `отбор скд`, but still
needed provider-aware reranking to prefer `НастройкиКомпоновкиДанных.Отбор` over generic lexical
matches.

The follow-up does not change the T49 storage decision. MyStem-backed Tantivy remains only a
possible future FTS-only experiment: it needs a tokenizer that indexes both original identifiers
and split terms, controlled synonym/query expansion, and explicit domain reranking. It also brings
external binary/process and second-artifact complexity while exact lookup, provider JSON and
relationship traversal still require SQLite.

T54 improved accepted BSL relationship coverage without adding parser facts or changing the SQLite
schema. Relationship traversal now prefers structured `has_type` / `returns` edges before the
reverse `member_of` edge. A rebuilt Russian index from
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` kept schema version `3`, `25082` documents and `65455`
relations, and UAT-SH-017 passed with the stricter SKD owner/member assertion: traversal from
`НастройкиКомпоновкиДанных.Отбор` reaches the referenced `ОтборКомпоновкиДанных` type, `Элементы`,
`КоллекцияЭлементовОтбораКомпоновкиДанных.Добавить` and `ЭлементОтбораКомпоновкиДанных` properties
`ЛевоеЗначение`, `ВидСравнения`, `ПравоеЗначение` and `Использование` within the existing bounded
local graph query.

T55 did not change runtime behavior, parser facts, provider JSON shape or the SQLite schema. It
accepted ADR-0007 and selected local CLI JSON over a prebuilt `syntax` index as the first
downstream analyzer-provider boundary. The SQLite index remains a rebuildable internal provider
artifact, not a public table-level contract. Rust library APIs, analyzer-specific file artifacts,
service boundaries and batch APIs require a future ADR or task with concrete consumer evidence.

T56 changed the internal query-index SQLite schema to version `4` for analyzer-oriented storage
normalization while preserving the provider CLI JSON boundary from ADR-0007. The `documents` table
no longer stores `signature_json` or `preview`; provider facts are assembled from normalized
relational rows. The new analyzer fact tables are `type_identities`, `members`, `callables`,
`signatures`, `parameters` and `type_refs`. `document_search` / `document_fts` remain the lexical
FTS projection, and `relations` remains the bounded graph traversal table.

The accepted debug rebuild from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` produced `25082`
documents in `55459 ms` at `target/uat/t56-sh-search-ru.sqlite`. UAT-SH-017 provider assertions
passed unchanged for `HTTPСоединение` constructor parameters, `НастройкиКомпоновкиДанных.Отбор`
owner/member lookup and relationship traversal, accounting-register query-table discovery, and
relationship traversal from the accepted accounting-register table id. Read-only SQL inspection
confirmed `schema_version=4`, non-empty normalized tables, no `documents.signature_json`, no
`documents.preview`, normalized parameter type refs for `ИспользоватьАутентификациюОС -> Булево`,
normalized member facts for `НастройкиКомпоновкиДанных.Отбор`, and normalized property type refs to
`ОтборКомпоновкиДанных`. Type references to duplicate platform type names keep `target_type_name`
but leave `target_type_id` unset instead of choosing a hidden semantic variant.

T76 changed the internal search-index schema to `schema_version=5` by adding the
`type_identities_document_idx` index used by provider type identity lookup. The provider JSON and
CLI contract did not change; existing schema version `4` search indexes are rebuildable service data
and must be rebuilt before query commands open them.

T81 changed the internal search-index schema to `schema_version=6` by adding
`members_document_owner_idx` and `callables_document_owner_idx` for exact owner-type member and
callable lookup through normalized `document_names` keys. The provider JSON and CLI contract did
not change; existing schema version `5` search indexes are rebuildable service data and must be
rebuilt before query commands open them.

T57 defined analyzer query primitives as a spec-only contract over the existing CLI JSON provider
boundary. The selected shape extends `syntax get`, `syntax constructors` and `syntax related` with
analyzer-oriented query kinds for exact type identity resolution, member listing, owner/member
resolution, callable overload retrieval and type-reference traversal. It does not add Rust public
APIs, BSL parsing, analyzer diagnostics, a daemon/service boundary, storage selection knobs or a
public SQLite table contract. UAT-SH-018 records the source-backed expression-chain scenario for
future implementation: `НастройкиКомпоновкиДанных.Отбор` through `ОтборКомпоновкиДанных`,
`Элементы`, collection `Добавить` and `ЭлементОтбораКомпоновкиДанных` fields, plus the
`Новый HTTPСоединение(...)` constructor chain.

T58 implemented the analyzer provider primitives selected by T57 over the schema-v4 normalized
index while preserving the existing `syntax get`, `syntax constructors`, `syntax search` and
`syntax related` workflows. `syntax get` now supports type identity lookup via
`--kind platform_type --id|--name|--alias`, member listing via `--members-of`, analyzer-preferred
owner/member lookup via `--owner-type-id --member`, callable lookup via `--callable-id` or
`--owner-type-id --callable`, and `syntax related --id --edge` supports direct type-reference edge
queries. Provider facts remain under `results[].fact`; analyzer resolution aids such as
`owner_type_id` and `target_type_ids` are emitted only under `results[].meta`.

The accepted debug rebuild from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` produced `25082`
documents in `52484 ms` at `target/uat/t58-sh-search-ru.sqlite`. T58 JSON assertions passed for
type resolution by Russian primary name and English alias, member listing for
`ОтборКомпоновкиДанных`, owner-type/member lookup for
`НастройкиКомпоновкиДанных.Отбор`, callable lookup for
`КоллекцияЭлементовОтбораКомпоновкиДанных.Добавить`, constructor parameters for
`HTTPСоединение` and edge-filtered `has_type` traversal from the accepted SKD filter property.
Existing UAT-SH-017 assertions still passed on the same rebuilt index for owner/member lookup,
SKD relationship traversal and accounting-register query-table discovery.

T59 added and passed the expression-chain provider UAT without adding a BSL parser, analyzer logic
or a new provider boundary. The accepted debug rebuild from
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` produced `25082` documents in `52698 ms` at
`target/uat/t59-sh-search-ru.sqlite`. The UAT models the SKD expression chain as provider JSON
calls: resolve `НастройкиКомпоновкиДанных.Отбор`, derive `ОтборКомпоновкиДанных`, traverse
`Элементы` to `КоллекцияЭлементовОтбораКомпоновкиДанных`, resolve collection `Добавить`, derive
`ЭлементОтбораКомпоновкиДанных` and verify the accepted filter-item fields. The same run verifies
the `Новый HTTPСоединение(...)` constructor chain through type identity, constructor parameter
facts and `constructs` traversal back to `HTTPСоединение`. Assertions use only provider commands and
JSON fields; no SQLite table names, rowids, HBK paths, TOC paths, HTML paths or page titles are
part of the scenario.

T60 hardened analyzer ambiguity handling over the schema-v4 provider index. Exact-name lookup now
returns the full deterministic candidate set instead of keeping only ownerless facts when an
ownerless and an owned fact share the same name. Owner-name/member lookup and related traversal
resolve the owner type identity first, so duplicate platform type names such as `ЭлементыФормы`
return provider `status: "ambiguous"` before member filtering. Constructor lookup by ambiguous
type name returns a provider `ambiguous` envelope instead of a non-provider error or hidden owner
selection. The accepted debug rebuild from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` produced
`25082` documents in `52737 ms` at `target/uat/t60-sh-search-ru.sqlite`. UAT-SH-019 passed for
duplicate `ЭлементыФормы` type candidates, ambiguous `ЭлементыФормы.Добавить` owner-name/member
and related roots, ambiguous `syntax constructors "ЭлементыФормы"`, ambiguous ownerless/owned
`ОтборКомпоновкиДанных` exact-name collision, and unambiguous
`--owner-type-id "platform_type:ЭлементыФормы:Форма" --member "Добавить"`.

T61 evaluated analyzer batch lookup needs after the primitive UAT and deferred a batch boundary.
The accepted measurement used the prebuilt T60 Russian schema-v4 index at
`target/uat/t60-sh-search-ru.sqlite`, so only the query path was measured. The UAT-SH-018
expression-chain and constructor-chain flow was executed as nine separate CLI JSON calls:
owner-type/member lookup for `НастройкиКомпоновкиДанных.Отбор`, member listing for
`ОтборКомпоновкиДанных`, `has_type` traversal from `Элементы`, member listing for
`КоллекцияЭлементовОтбораКомпоновкиДанных`, callable lookup for `Добавить`, member listing for
`ЭлементОтбораКомпоновкиДанных`, type identity for `HTTPСоединение`, constructor lookup for
`HTTPСоединение` and `constructs` traversal from the constructor fact. Individual debug command
timings were `0.00 s`, `0.04 s`, `0.00 s`, `0.04 s`, `0.00 s`, `0.04 s`, `0.20 s`, `0.39 s` and
`0.00 s`; five repeated full-chain runs took `828 ms`, `782 ms`, `762 ms`, `830 ms` and `745 ms`.
The combined output size for the nine JSON responses was `48390` bytes. These measurements keep the
accepted analyzer primitive workflow within NFR-QUERY-001 and do not prove a need for a batch CLI
command, Rust API, daemon/service boundary or public SQLite table contract. Batch lookup remains a
future task only if a concrete analyzer scenario proves many-symbol lookup volume that the current
CLI JSON boundary cannot satisfy.

T62 improved review-oriented keyword ranking without changing the provider JSON envelope, the
schema-v4 SQLite artifact or the accepted CLI boundary. The ranking path now promotes exact
primary/alias identity matches before broader prefix, owner, description and FTS-score matches, and
keeps ranking details under `results[].meta`.

The accepted debug rebuild from `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` produced `25082`
documents in `52851 ms` at `target/uat/t62-sh-search-ru.sqlite` with peak RSS `305516 KiB`.
UAT-SH-020 passed: `syntax search --query "Структура" --mode keywords --format json` ranked
`platform_type:Структура` first; `отбор скд` still ranked an SKD/data-composition fact first and
kept `platform_type:ОтборКомпоновкиДанных` in the result set; and
`таблица регистра бухгалтерии` kept the accepted top hit
`query_table:РегистрБухгалтерииТаблицаИзмененийРегистраБухгалтерии`.

T63 added explicit bounded and compact output controls for review-oriented provider use without
changing the default full provider JSON contract. `syntax search` and `syntax related` now accept
`--limit <N>`; omitted limits preserve the previous defaults of `20` search results and `200`
related results. `syntax related --compact` keeps stable fact identity (`id`, `kind`, `name` and
optional `owner`) plus relationship explanation under `results[].meta.depth` and
`results[].meta.path`, while omitting bulky fact fields such as descriptions, signatures, `types`
and `return`.

The accepted T63 verification used the current Russian schema-v4 index at
`target/uat/t63-sh-search-ru.sqlite`. UAT-SH-021 passed: `syntax search --query "Структура"
--mode keywords --limit 3 --format json` returned exactly three deterministic provider results and
recorded `query.limit == 3`; `syntax related --id "type_property:platform_type:Символы:ПС"
--limit 5 --format json` returned exactly five full provider results with relationship metadata;
and the same command with `--compact` returned exactly five compact facts with identity and path
metadata while omitting `description`, `signatures`, `types` and `return` from `results[].fact`.

T64 aligned public relationship edge filters with the graph contract. `member_of` is now accepted
as public inverse owner navigation for exact `syntax related --id` roots, alongside `has_type`,
`returns` and `constructs`. Type-reference edges keep `query.kind == "type_references"`; the
ownership edge reports `query.kind == "related"` because it explains the graph owner relationship,
not a type reference.

The accepted T64 verification used a fresh Russian schema-v4 index at
`target/uat/t64-sh-search-ru.sqlite` with `25082` documents, `56075 ms` build time and peak RSS
`310016 KiB`. UAT-SH-022 passed:
`syntax related --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --edge
member_of --format json` returned the owning fact `platform_type:НастройкиКомпоновкиДанных` with a
`member_of` relationship path; text output included the same owner fact; unsupported-edge provider
diagnostics and CLI help both listed the supported edge set including `member_of`.

T142 added the first bounded type-graph provider primitive under the existing `syntax related`
command family. `syntax related --id <exact-provider-id> --graph --format json` uses provider
`schema_version: 1`, records `query.kind == "type_graph"`, returns the exact root as the first
result and bounds the whole graph with `--limit` including that root. Shared platform facts stay in
export-compatible `results[].fact` objects, while graph traversal, type-reference resolution,
template binding evidence and recoverable unresolved/ambiguous type-reference diagnostics stay in
`results[].meta` or envelope diagnostics. Graph mode rejects plain names, owner/member roots,
query/language/enum/global-property roots, explicit `--edge` filters and `--compact` as
unsupported combinations; existing non-graph compact and edge behavior remains unchanged.

The accepted T142 verification used a fresh Russian index at
`target/uat/t142-type-graph.sqlite` with `25415` documents and `37292 ms` build time.
UAT-SH-024 passed:
`syntax related --id "type_property:platform_type:НастройкиКомпоновкиДанных:Отбор" --graph
--limit 200 --format json` returned the SKD expression-chain facts needed to navigate from
`НастройкиКомпоновкиДанных.Отбор` through `ОтборКомпоновкиДанных`, filter elements and the
`КоллекцияЭлементовОтбораКомпоновкиДанных.Добавить` callable to item fields such as
`ЛевоеЗначение`, `ВидСравнения`, `ПравоеЗначение` and `Использование`. The measured graph query
time was `0.15` seconds, within NFR-QUERY-001, and both `--graph --compact` and
`--graph --id "query_table:БизнесПроцесс"` returned explicit `unsupported` provider responses.

T145 broadened UAT-SH-024 on a fresh Russian index at `target/uat/t145-type-graph.sqlite` with
schema `13`, `25415` documents and `44819 ms` build time. Three additional expression-chain graph
queries passed without changing provider schema or unsupported graph combinations:
`Запрос.Выполнить` reached `РезультатЗапроса`, `РезультатЗапроса.Выбрать`,
`ВыборкаИзРезультатаЗапроса`, `Следующий` and `<Имя поля>` in `0.11` seconds;
`HTTPСоединение.Получить` reached `HTTPОтвет`, `КодСостояния`, `Заголовки`,
`ПолучитьТелоКакСтроку` and `ПолучитьТелоКакДвоичныеДанные` in `0.17` seconds; and
`ДвоичныеДанные.ОткрытьПотокДляЧтения` reached `Поток`, `Прочитать`, `Закрыть`,
`ДоступноЧтение` and `ПолучитьПотокТолькоДляЧтения` in `0.04` seconds. All measured graph queries
remained within NFR-QUERY-001, and graph metadata stayed under `results[].meta` while shared fact
fields remained export-compatible.

T148 kept provider JSON schema `1` and canonical export schema `11` stable while recording the
smallest CLI provider JSON assembly boundary: command handlers execute query paths, while private
`v8-context-hbk-cli` provider helpers assemble envelopes, shared facts, metadata and diagnostics.
Graph type-reference metadata is rendered explicitly from search DTOs instead of serializing
internal model objects wholesale: statuses are `ok`, `unresolved` and `ambiguous`, resolved targets
use `target_type_id`, ambiguous targets use `candidate_type_ids`, and template bindings expose only
`template_key.family`, `template_key.variant` and provider-owned argument objects under
`results[].meta.type_references[].template_binding`.

T146 implemented the first-class Rust resolver global-context scope and closed callable/member
adapter gaps without changing CLI behavior, provider JSON, consumer export JSON or the private
search-index schema version. `context-resolver-core` now exposes `global_context` lookup for BSL
and SDBL/query scopes. `context-resolver-search` composes platform `shcntx_*` global
methods/properties into the BSL global context, exposes `shlang_*` facts in the BSL language scope
and exposes `shquery_*` / `dcsui_*` facts in the SDBL/query scope. Ownerless platform callable
lookup resolves only global methods; global properties are returned through the global-context
properties collection without a fake owner `TypeId`. Exact named member misses now return
`NotFound`, while broad member listing can still return `Ok([])`. Type-event members listed from a
resolved owner can be looked up back by the exact `MemberId` returned by the resolver. Type-event
search documents now consume read-phase `owner_identity` like other child/member records instead of
building owner identity inside `syntax-helper-search`.

T146 focused verification passed:

- `cargo test -p context-resolver-core --lib`
- `cargo test -p syntax-helper-search --lib`
- `cargo test -p context-resolver-search`

T65 accepted ADR-0008 and the Rust solution-context resolver API design. This is a spec-only
decision and does not change CLI behavior, provider JSON, SQLite schema or parser output. ADR-0008
adds a future in-process Rust boundary for a concrete full-context application that needs fast
resolution across platform API, BSL-language, query-language, configuration and source-code
providers. The design keeps BSL language types and query-language types in separate domains, uses
source-qualified identities, and requires ambiguity instead of hidden winner selection for same-name
facts across domains or sources.

T67 implemented the first Rust solution-context resolver slice without changing CLI behavior,
provider JSON, SQLite schema, parser output or consumer export JSON. The new `context-resolver-core`
crate contains the source-neutral resolver API, typed ids, domains, fact kinds, response statuses,
diagnostics, resolved wrappers and synchronous source/resolver traits without HBK, SQLite, CLI or
parser dependencies. The new `context-resolver-search` crate adapts
`syntax-helper-search::SearchIndex` as the first HBK-backed platform source while keeping
`syntax-helper-search` as the local index/query implementation rather than the generic resolver
model.

T67 verification passed `cargo test -p context-resolver-core`, `cargo test -p
context-resolver-search`, `cargo test -p syntax-helper-search --lib` and `cargo test --workspace`.
Focused resolver tests cover same-name ambiguity, preservation of source-level
`ambiguous`/`unsupported` responses, BSL/query `Строка` type separation, resolved owner-id member
isolation, callable identity with ordered parameters and return/constructor type references,
explicit fake cross-domain type relations, platform adapter lookup over a `SearchIndex` fixture,
`has_type`, `returns`, `constructs` and `member_of` traversal, and hiding existing `query_table*`
provider documents from the platform adapter. The adapter fixture keeps exact type resolution,
member listing, callable lookup and relation traversal under the provisional NFR-RESOLVE-001
`100 ms` target after source open.

T90 implemented the first language-domain resolver adapter slice. `context-resolver-search` now
exposes source-specific language adapters for `shlang`, `shquery` and `dcsui` over the prebuilt
T89 language-fact index. `shlang` resolves under `BslLanguage`; `shquery` and `dcsui` resolve under
`QueryLanguage` with distinct source identities. The adapter resolves exact ids/names for BSL
`def_String`, query `STRING`, query `LitString` and SKD
`SKD_Functions_Strings#StringLength`, reports ambiguity for unconstrained `Строка`, and traverses
the explicit SKD string-function parameter type edge to `shlang:def_String`. The resolver core
composition now preserves already found `ok` candidates when another source reports ambiguity, so
cross-source ambiguity does not hide valid candidates from other active sources. T90 focused
latency checks for exact BSL type lookup and SKD relation traversal stayed under the provisional
NFR-RESOLVE-001 `100 ms` target after source open.

T92 removed hidden platform resolver adapter fallbacks without changing CLI behavior, provider JSON,
SQLite schema, parser output, consumer export JSON or `syntax-helper-search` relation storage.
Platform adapter relation traversal now uses edge-specific `related_by_id_and_edge` evidence only;
callable return/result mapping still uses explicit `return_types` or edge-specific `returns` /
`constructs` evidence, but constructors no longer synthesize a result type from the owner when that
evidence is missing. Focused resolver coverage now mutates a fixture index to remove constructor
result evidence and verifies that callable lookup returns an empty return-type list and `constructs`
traversal returns an empty relation set instead of a synthesized owner type.

T108 completed the Markdown/TOC export correction for `shclang_ru.hbk` query examples stored as
Courier blockquotes. UAT-HBK-013 passed against
`/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk`: page `Работа с временными таблицами` exports
query-language examples as `sdbl` fenced code blocks containing `ВЫБРАТЬ`,
`ПОМЕСТИТЬ ВременнаяТаблица` and `ИЗ Справочник.Номенклатура`, and no longer exports those
examples as Markdown blockquotes. The same release export check also confirmed that
`Работа с пакетными запросами` still uses `bsl` fences for ordinary BSL examples and XBASE
same-page links retain Markdown `#fragment` anchors.

T109 completed the matching target-anchor side of Markdown/TOC fragment navigation. The same
`shclang_ru.hbk` page `Работа с временными таблицами` now exports same-page links such as
`index.md#Manager` together with explicit Markdown-compatible targets such as
`<a id="Manager"></a>` before the corresponding heading. UAT-HBK-013 passed with target checks for
`Manager`, `Create`, `Used` and `Delete`; release-profile export to
`target/uat/shclang-anchor-materialized-md` showed the generated targets in the user-reported page.

T111 added the first documentation-site generator crate boundary without CLI or web-app wiring. The
new `hbk-doc-site` crate exposes typed generation request/result/error and site id models, discovers
source books from explicit files or a source directory with include filters, rejects unsafe
locale-derived artifact path segments, groups books by locale, merges same-level section nodes by
normalized title and preserves `book_id`/distinct `page_id` values for page-bearing same-title nodes
and their child sections. The T111 artifact writer produces deterministic `data/manifest.json`,
`data/locales/<locale>/toc-root.json` and `data/locales/<locale>/toc-sections/<section-id>.json`
files for fixture corpora. The manifest includes generator version, deterministic build id, source
book file sizes, TOC root paths and future page root paths. Page Markdown writing, CLI
`site generate`, UAT-HBK-014 real-corpus measurements and the web app remain T112/T113 scope.

T112 completed documentation-site page-data generation and CLI wiring without adding a web app,
search or semantic indexing. `hbk-doc-site` now writes page Markdown files under
`data/locales/<locale>/pages/<page-id>.md` for page-bearing global TOC nodes while keeping stable
global `page_id` values and TOC/page data split. `v8-context-hbk site generate <source-dir>
--output <data-dir> [--include <file-name>]...` reports source book count, locale count, TOC node
count, page count, generated file count, output bytes, elapsed milliseconds and Linux VmHWM when
available. UAT-HBK-014 passed on 2026-05-07 against the local 8.5.1.1150 corpus
`fmtdui_ru.hbk`, `shlang_ru.hbk`, `shquery_ru.hbk` and `dcsui_ru.hbk`: 4 source books, 1 locale,
267 TOC nodes, 254 page Markdown files, 302 generated files, 931369 bytes, 3281 ms and
11632 KiB peak RSS. The UAT checks confirmed manifest/TOC/page artifact presence, page-bearing TOC
nodes with `book_id`, matching page Markdown files for every `page_id`, and no raw installed HBK
paths, `.hbk` names, raw HTML storage paths or raw TOC-index wording in generated locale JSON or
Markdown. Fixture coverage also checks that generated site page Markdown preserves internal
Markdown links and HTML fragments through site-owned page-id targets.

T113 completed the first minimal documentation web app. `web/docs-viewer` builds with
`npm --prefix web/docs-viewer run build` and serves generated site data with
`npm --prefix web/docs-viewer start -- --data "$PWD/target/uat/doc-site-data/data" --listen
127.0.0.1:4173`. UAT-HBK-015 passed on 2026-05-07 against the T112 representative generated data:
browser smoke loaded the app, expanded a Russian root TOC section and opened the
`form_formattedstringedit` page. Network requests showed separate loads for `/data/manifest.json`,
`/data/locales/ru/toc-root.json`, a lazy `/data/locales/ru/toc-sections/*.json` section and
`/data/locales/ru/pages/page-fmtdui-ru-form-formattedstringedit-0ee13df5698bdba0.md`. Checks also
confirmed that representative page strings such as `Конструктор строк` and `Если в конфигурации`
were absent from the initial HTML and `app.js`, and browser console output had no errors after the
favicon request was eliminated. Desktop `1440x900` and mobile `390x844` viewport smoke checks showed
usable navigation and page readability without overlapping text.

T114 added visible progress for long-running `site generate` runs without changing the final
`stdout` summary keys. `hbk-doc-site` now exposes `DocSiteGenerator::generate_with_progress`, while
the CLI prints progress to `stderr` for source discovery, source-book loading, site-data planning
and artifact writing. Progress output was simplified on 2026-05-07 to avoid full paths and
per-artifact-family chatter. Interactive terminal progress updates one line in place and shows the
latest source/artifact file name, with file-level redraws throttled to avoid terminal flicker;
redirected progress logs use bounded sparse milestones so large corpora still update regularly after
the first item. UAT-HBK-014 was re-run on 2026-05-07 against the same representative corpus and
confirmed redirected `stderr` progress lines such as
`progress: source books discovered: 4`, `progress: loading source books: 1/4 (fmtdui_ru.hbk)`,
`progress: site data planned: locales=1, toc_nodes=267, pages=254` and sparse
`progress: writing artifacts: <current>/<total>` milestones.
The final `stdout` summary remained `output`, `source_books`, `locales`, `toc_nodes`, `pages`,
`files`, `bytes`, `elapsed_ms` and `peak_rss_kib`; the representative rerun produced 4 source
books, 1 locale, 267 TOC nodes, 254 pages, 302 files, 931369 bytes, 3052 ms and 11924 KiB peak RSS.

T115 removed avoidable repeated work from documentation site page generation without changing the
generated data contract or final `stdout` summary keys. The site generator now precomputes locale
Markdown link targets once, reuses one Markdown page loader per source book and uses a per-loader TOC
HTML-path index for page/link resolution instead of rebuilding flat TOC data per page. A
release-profile UAT-HBK-014 rerun on 2026-05-07 against the representative four-book corpus produced
4 source books, 1 locale, 267 TOC nodes, 254 pages, 302 files, 931369 bytes, 169 ms and 7532 KiB peak
RSS. A broader diagnostic full-corpus release run against all 116 local 8.5.1.1150 HBK files
produced 3 locales, 60686 TOC nodes, 54849 pages, 66730 files, 82233487 bytes, 23351 ms and
253696 KiB peak RSS.

T117 removed additional repeated work from documentation site generation while preserving the full
generated data output from the pre-change full-corpus run. The site Markdown path now reads raw page
HTML through the existing per-book page loader and avoids building full `PageContent`/link
diagnostics for every generated page; `HbkBook::open` no longer reads `FileStorage` when `PackBlock`
already provides TOC data; generated JSON and Markdown writers no longer call `fs::metadata` after
every file write. A release-profile UAT-HBK-014 rerun on 2026-05-07 against the representative
four-book corpus produced 4 source books, 1 locale, 267 TOC nodes, 254 pages, 302 files, 931369 bytes,
122 ms and 7252 KiB peak RSS. A diagnostic full-corpus release run against all 116 local
8.5.1.1150 HBK files produced 3 locales, 60686 TOC nodes, 54849 pages, 66730 files, 82233487 bytes,
18293 ms and 222896 KiB peak RSS.

T118 fixed documentation-site TOC duplication for same-address page targets. The global TOC now
merges same-level page-bearing nodes by normalized page address, writes one generated page file for
the merged target and registers source-book aliases so Markdown links from duplicate source books
resolve to that page. Page ids are opaque locale/address ids and do not include TOC path or title
text, because some HBK pages expose generic or unreliable HTML titles. A diagnostic full-corpus
release run on 2026-05-07 against all 116 local 8.5.1.1150 HBK files produced 3 locales, 60453 TOC
nodes, 54618 pages, 66076 files, 70318465 bytes, 15175 ms and 229064 KiB peak RSS. In the generated
Russian root TOC, duplicate `form_plannerdimensionsdlg` entries merged from 2 nodes to 1; the three
`1С:Предприятие` root entries remained separate because they are distinct page/section targets.

T119 fixed generated section-link rendering in the documentation web viewer. Generated Markdown
anchors such as `<a id="..."></a>` are rendered as invisible DOM anchors instead of visible raw
text, and internal generated page links such as `<page-id>.md#fragment` are intercepted by the web
app so page and section links open in-place. A follow-up check against `hbk-reader` showed the same
content-area click interception pattern; the viewer renderer now preserves generated `page-*.md`
hrefs so the existing click handler can route page-to-page links instead of receiving `#`.

T120 fixed the mixed placeholder/concrete page-target case in documentation-site TOC merge. When a
same-level TOC branch uses a `_CONTENTS_NODE_*` placeholder in one source book and exactly one
equivalent branch in another source book has a concrete page address, the placeholder branch now
merges into the concrete generated page target. The placeholder `source book + html path` is
registered as a Markdown link alias for the concrete page, so generated links to the placeholder
address resolve to the real page file. A diagnostic full-corpus check on 2026-05-07 with all
116 local 8.5.1.1150 HBK files produced 3 locales, 60481 TOC nodes, 54646 pages, 66013 files,
70313065 bytes, 15284 ms and 247844 KiB peak RSS. In the generated Russian root TOC,
`1С:Предприятие` entries reduced from 3 nodes to 2 after placeholder/concrete resolution, while
`form_plannerdimensionsdlg` remained merged as one same-address node.

T121 fixed documentation-site readability for Markdown blockquotes and tables. `hbk-book-export`
now normalizes non-code blockquote/table launch-flow diagrams into quoted prose lines before
`quick_html2md`, while preserving the existing Courier code/query-example paths. The documentation
viewer now renders Markdown blockquotes, GFM tables and quoted GFM tables as DOM nodes instead of
showing raw `>` or `| --- |` markup. A representative `site generate --include 1cv8_ru.hbk` run on
2026-05-07 produced 1 source book, 1 locale, 397 TOC nodes, 365 pages, 410 files, 1127587 bytes,
1761 ms and 14208 KiB peak RSS; the generated
`Запуск 1С:Предприятие 8 и параметры запуска` page no longer contains raw `> |` quoted table
markers in the reported launch-flow block.

T121 fixed the reported launch-flow Markdown regression on
`1cv8_ru.hbk` page `Запуск 1С:Предприятие 8 и параметры запуска` (`ZIF`). Layout-only non-code
tables inside blockquotes now export as quoted prose lines instead of raw quoted GFM table
scaffolding, while ordinary GFM tables and blockquotes remain renderer-supported in
`web/docs-viewer`. A representative `site generate --include 1cv8_ru.hbk` run on 2026-05-07
produced 1 source book, 1 locale, 397 TOC nodes, 365 pages, 410 files, 1127587 bytes, 133 ms and
9540 KiB peak RSS. The generated page `page-ru-c5a12eeae852efad.md` contains
`> Программа запуска - 1CEStart`, `> Интерактивная программа запуска - 1Cv8s` and
`> Клиентское приложение`, and no longer contains `> |` table markup for that launch-flow block.

T147 fixed generated documentation viewer link navigation. Documentation-site page Markdown now
keeps same-page fragment links as `#fragment` anchors instead of generated page-file links or
`index.md#fragment`, while cross-page generated links remain `page-*.md#fragment` targets for viewer
routing. `web/docs-viewer` resolves same-page fragments against the currently loaded generated page,
routes cross-page generated links through the page data client and derives the browser document title
from the rendered human heading or human link/TOC title instead of the opaque generated page id.
Verification passed with `npm test --prefix web/docs-viewer -- --test-reporter=tap` and
`cargo test -p hbk-doc-site -p hbk-book-export -p v8-context-hbk-cli`.

T132 moved platform type-template ownership into the HBK-backed provider boundary. The
resolver model now exposes semantic type-template kind as metadata object kind, generated type
role and template parameter role, and type references can carry owner-parameter template bindings.
The search index schema version is `9`; SQLite remains a private rebuildable provider artifact, not
a public integration contract.

Focused tests cover semantic template classification, read-phase extraction, search-index
roundtrip, semantic-kind lookup, ambiguous type-reference protection and resolver-visible template
binding. A representative `syntax index` run on 2026-05-10 against
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` produced `target/t132-type-template.sqlite` with
25415 documents. SQL evidence confirmed catalog and document templates such as
`СправочникСсылка.<Имя справочника>`, `СправочникМенеджер.<Имя справочника>`,
`ДокументОбъект.<Имя документа>` and `ДокументСсылка.<Имя документа>` have semantic
`type_templates` rows. Template binding evidence existed for `constructor_result`, `parameter_type`,
`property_type` and `return_type` rows. The representative source-backed member
`ДокументОбъект.<Имя документа>.Ссылка` stores `target_type_id =
platform_type:ДокументСсылка.<Имя документа>`, semantic target `document/reference` and
`owner_parameter(metadata_object_name)` binding even though the source type text is
`ДокументСсылка`.

T133 replaced the closed T132 type-template enum contract with data-driven open
family/variant keys. The search index schema version is `11`; SQLite remains a private rebuildable
provider artifact. Type template classification now uses alias-base or root-locale primary-base,
manager-root family discovery, longest-prefix assignment and direct type-template type-reference scoring
for templates left unassigned by manager roots. Non-root localized primary names without aliases are
left unclassified with persisted diagnostics instead of becoming families. Template parameter labels
are preserved as source parameter slots and matching owner/target parameter labels produce
parameter-slot binding arguments, not family semantics.

Focused tests cover alias/fallback base extraction, manager-root longest-prefix classification,
direct-reference family assignment for previously unassigned templates, unclassified diagnostics,
family/variant lookup and resolver-visible parameter-slot template bindings. A representative
release `syntax index` run on 2026-05-11 against
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` produced
`target/t133-review-type-template.sqlite` with 25415 documents in 11409 ms. SQL evidence confirmed
`121` type-template `type_templates` rows and `0` unclassified rows. The previously disputed templates
`БазовыеВидыРасчета`, `БазовыеВидыРасчетаСтрока`, `ВедущиеВидыРасчета`,
`ВедущиеВидыРасчетаСтрока`, `ВытесняющиеВидыРасчета` and
`ВытесняющиеВидыРасчетаСтрока` are assigned to family `ChartOfCalculationTypes` through direct
type-template type-reference evidence and their persisted classification diagnostics record
`direct_type_ref` evidence. The representative source-backed member
`ДокументОбъект.<Имя документа>.Ссылка` stores `target_type_id =
platform_type:ДокументСсылка.<Имя документа>`, target family/variant `Document/Ref` and
`owner_parameter` binding indexes `0 -> 0`.

A representative release `syntax index` run on 2026-05-11 against
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` produced
`target/t133-review-root-type-template.sqlite` with 25415 documents in 8754 ms. SQL evidence
confirmed schema version `11`, `121` type-template `type_templates` rows and `0` unclassified rows,
covering the real root-primary fallback path.

T134 normalized the active Rust/search/resolver terminology from legacy template wording to
platform type template / type template wording without changing classification semantics, CLI
provider JSON or canonical `syntax export` JSON. The private rebuildable search-index schema is now
`12` because type-template SQLite columns were renamed from `generic_*` names to
`template_family`, `template_variant`, `template_classification_diagnostic`,
`type_template_family`, `type_template_variant` and `template_binding_*`.

Representative release `syntax index` runs on 2026-05-11 against the local 8.5.1.1150 Syntax
Assistant books produced:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` ->
  `target/t134-type-template-ru.sqlite`: 25415 documents in 12159 ms, schema version `12`, `121`
  type-template rows, `121` classified templates, `0` unclassified templates and `353` template
  binding rows.
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` ->
  `target/t134-type-template-root.sqlite`: 25415 documents in 9505 ms, schema version `12`, `121`
  type-template rows, `121` classified templates, `0` unclassified templates and `335` template
  binding rows.

SQL inspection confirmed the renamed private layout:

- `type_templates`: `template_family`, `template_variant`,
  `template_classification_diagnostic`;
- `type_refs`: `type_template_family`, `type_template_variant`, `template_binding_kind`,
  `template_binding_owner_parameter_index`, `template_binding_target_parameter_index`,
  `template_binding_arguments`.

T135 added the reproducible `syntax type-ref-gaps` measurement command for prebuilt search indexes.
No new ADR was required: FR-SH-002/FR-SH-003 own extracted type-reference and type-template facts,
FR-SH-PROVIDER-001 and FR-CTX-RESOLVE-001 already require explicit missing/ambiguous outcomes
without hidden winner selection, and ADR-0004/ADR-0006 keep query/report commands on prebuilt local
indexes. The command reads an existing SQLite index only; it does not parse `shcntx_*.hbk` per
query and does not change provider JSON or canonical export JSON contracts.

Representative release runs on 2026-05-11 rebuilt fresh indexes and verified deterministic report
output by running JSON measurement twice for each index and comparing the files with `cmp`:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` ->
  `target/t135-type-ref-ru.sqlite`: 25415 documents in 10359 ms. The report counted `47156`
  type-reference rows: `29776` resolved, `17367` unresolved, `13` ambiguous and `353` rows with
  template bindings. By source role: `constructor_result` `442/442/0/0`,
  `parameter_type` `22922/16233/6686/3`, `property_type` `12468/4958/7505/5`,
  `query_field_type` `518/109/409/0`, `query_parameter_type` `68/50/18/0` and
  `return_type` `10738/7984/2749/5` as `total/resolved/unresolved/ambiguous`.
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` ->
  `target/t135-type-ref-root.sqlite`: 25415 documents in 7890 ms. The report counted `50034`
  type-reference rows: `32823` resolved, `17211` unresolved, `0` ambiguous and `335` rows with
  template bindings. By source role: `constructor_result` `442/442/0/0`,
  `parameter_type` `22878/16209/6669/0`, `property_type` `14295/6882/7413/0`,
  `query_field_type` `518/109/409/0`, `query_parameter_type` `68/50/18/0` and
  `return_type` `11833/9131/2702/0` as `total/resolved/unresolved/ambiguous`.

The largest unresolved names in both RU/root reports are primitive/domain type names such as
`Строка` / `String`, `Булево` / `Boolean` and `Число` / `Number`, especially in callable parameter
and property type roles. This points to the already planned type-domain separation work rather than
a safe platform-type hidden-winner rule. T144 resolved the whitespace-sensitive
`Настройка сервиса` / `НастройкаСервиса` rows by exact source spelling. The remaining RU ambiguous
rows are duplicate platform type-name cases for `ЭлементыФормы` in `property_type` rows, where the
same source spelling maps to distinct `Controls` and `FormItems` platform types.

T136 promotes the T135 type-reference and type-template measurements into acceptance quality gates
for the current 8.5.1.1150 Syntax Assistant baseline. These gates are evaluated from fresh
`syntax index` artifacts plus `syntax type-ref-gaps --format json`; they do not add provider JSON
fields, consumer export fields or SQLite table-level public contracts.

Current gate values:

| Source | Unresolved type references | Ambiguous type references | Classified metadata/type templates | Unclassified type-template diagnostics | Type-template bindings | Expression-chain provider scenario |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `shcntx_ru.hbk` | 15513 | 5 | 121 | 0 | 379 | UAT-SH-018 passed on the accepted provider workflow |
| `shcntx_root.hbk` | 17211 | 0 | 121 | 0 | 335 | UAT-SH-018 is language-neutral provider coverage; RU remains the representative real-index run |

Strict regression gates:

- `syntax type-ref-gaps` output must remain deterministic for the same prebuilt index.
- Query/report commands must continue to read a prebuilt local index and must not parse
  `shcntx_*.hbk` per query.
- Unresolved type-reference counts must not increase above the current source-specific baseline
  unless the task records a source-backed explanation and updates this baseline in the same change.
- Ambiguous type-reference counts must not increase above the current source-specific baseline.
  Hidden first-match selection remains forbidden; reducing ambiguity is acceptable.
- Classified metadata/type-template count must remain at least `121` for each target source, and
  unclassified type-template diagnostics must remain `0` unless a parser/model task deliberately
  exposes a real source ambiguity and records the follow-up.
- Type-template binding count must not drop below the current source-specific baseline without a
  source-backed explanation. Higher counts are allowed only with an updated baseline and evidence
  that the added bindings are source-backed.
- The accepted expression-chain provider scenario must remain passing for the current provider
  workflow; failures are regressions even if the raw type-reference counters stay unchanged.

Tracked informational metrics until later tightening tasks:

- role-level `type-ref-gaps` breakdowns;
- top unresolved and ambiguous target names and examples;
- index build timings associated with the measurement run;
- whether unresolved primitive/domain names are resolved by type-domain separation work rather than
  by platform-type guessing.

T138 records a spec-only crate-boundary decision for type concepts. A separate type crate is
deferred for now. The current smallest boundaries remain `syntax-helper-model` for source-backed raw
type facts and template bindings before indexing/export, `syntax-helper-search` for resolved target
ids, ambiguity/gap reporting and private index persistence, `context-resolver-core` for
source-neutral resolver DTOs and domain-qualified ids, and `context-resolver-search` for adapter
mapping between provider-local facts and resolver DTOs. This does not change code, workspace
membership, provider JSON, canonical export JSON, SQLite schema, parser behavior, UAT cases or the
T136 quality-gate values.

T139 splits source-backed type-reference spelling from resolved target identity. The private
rebuildable search-index schema is now `13`: each `type_refs` row keeps raw `target_type_name`,
stores `target_resolution_status` as `ok`, `unresolved` or `ambiguous`, stores `target_type_id` only
for unique resolved targets and stores deterministic ambiguous candidate ids when the reference name
matches multiple platform type identities. Provider JSON `types` and `return` fields remain
export-compatible source-name arrays; Rust resolver DTOs now expose the target outcome explicitly
instead of collapsing unresolved and ambiguous references into the same absent id. The focused test
coverage exercises resolved, unresolved and ambiguous type-reference rows through both
`syntax-helper-search` and `context-resolver-search`. The T136 quality-gate values are unchanged by
this storage/DTO split. A representative T139 UAT run on 2026-05-11 against
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` rebuilt
`target/uat/t139-sh-search-ru.sqlite` with 25415 documents in 41057 ms, confirmed search-index
schema version `13`, confirmed the new `type_refs.target_resolution_status` and
`type_refs.target_candidate_type_ids` columns, verified that ambiguous normalized rows do not
materialize legacy `has_type` / `returns` relation-table edges, verified exact get / constructors /
related provider JSON still uses export-compatible `types` and `return` fields, and reran
deterministic `syntax type-ref-gaps` twice with unchanged RU totals: `47156` total, `29776`
resolved, `17367` unresolved, `13` ambiguous and `353` template-binding rows. The same verification
rebuilt `target/uat/t139-sh-search-root.sqlite` from `shcntx_root.hbk` with 25415 documents in
29594 ms and confirmed unchanged root totals: `50034` total, `32823` resolved, `17211` unresolved,
`0` ambiguous and `335` template-binding rows.

T141 strengthened focused coverage for platform type-template resolution without changing the
accepted metrics or public provider/export contracts. The existing classification implementation
already used manager-root family derivation, direct type-template reference scoring and
recoverable unclassified/ambiguous diagnostics; the task added explicit coverage for callable
parameter and overload return type references carrying owner-parameter bindings through
`syntax-helper-search` and `context-resolver-search`. A representative T141 UAT run on 2026-05-11
rebuilt `target/uat/t141-sh-search-ru.sqlite` from `shcntx_ru.hbk` with 25415 documents in
37162 ms and confirmed `121` classified type templates, `0` unclassified diagnostics and unchanged
type-reference totals: `47156` total, `29776` resolved, `17367` unresolved, `13` ambiguous and
`353` template-binding rows. The same verification rebuilt `target/uat/t141-sh-search-root.sqlite`
from `shcntx_root.hbk` with 25415 documents in 29672 ms and confirmed `121` classified type
templates, `0` unclassified diagnostics and unchanged root totals: `50034` total, `32823`
resolved, `17211` unresolved, `0` ambiguous and `335` template-binding rows. The result justifies
keeping the T136 quality-gate values unchanged.

T140 keeps the public provider/export envelope stable while adding source-scoped callable return
facts. Page-level/shared Syntax Assistant return sections remain fact-level `return` arrays.
Source-proven overload returns are modeled on the concrete signature, stored as `return_type` rows
with `source_signature_id` / `source_signature_ordinal`, and may surface as `signatures[].return`.
Real-source fixture coverage currently confirms that `ДокументDOM.СоздатьРазыменовательПИ` /
`DOMDocument.CreateNSResolver` keeps its shared return at callable level; overload-specific return
coverage uses a focused synthetic parser fixture plus search/provider/resolver regressions because
no accepted real-source overload-return fixture is recorded yet. `schema_version` for provider JSON
remains `1`; the private rebuildable search-index schema remains `13`.

T143 classified the T135 unresolved type-reference rows by source/domain without changing the T136
quality-gate counters. The checked-in analysis path is
`scripts/analysis/type-ref-domain-classification.sql`; it reads only a prebuilt local SQLite search
index and writes raw reports under `target/` as service data. A representative run on 2026-05-12
rebuilt `target/uat/t143-sh-search-ru.sqlite` from
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` with 25415 documents in 11983 ms, reran
`syntax type-ref-gaps --format json` twice with byte-identical output and preserved the current RU
totals: `47156` total, `29776` resolved, `17367` unresolved, `13` ambiguous and `353`
template-binding rows.

The same prebuilt index produced a deterministic domain-classification report:

| Classification | Rows | Distinct target names |
| --- | ---: | ---: |
| likely BSL-language facts | 13932 | 6 |
| likely query-language or SKD facts | 422 | 10 |
| downstream configuration/source-code provider facts | 53 | 3 |
| still-unclassified platform-source gaps | 2960 | 707 |

The largest classified BSL-language names are `Строка`, `Булево`, `Число`, `Дата`,
`Неопределено` and `Тип`, backed by `shlang_*` primitive pages. Query-table primitive/value rows
are classified separately as query/SKD-domain candidates. These rows remain unresolved in the
platform type-reference counters until a source-backed language/configuration provider relation is
implemented; T143 deliberately does not reduce counts by guessing a platform type. The T136 strict
gate values are unchanged.

T144 investigated the current RU ambiguous type-reference names against source TOC/page evidence.
`Настройка сервиса` (`IServiceSetting`) and `НастройкаСервиса` (`ServiceSetting`) are distinct
platform identities in separate administration branches, but their source spellings differ exactly.
Index-time type-reference resolution now uses exact source spelling as a source-backed
disambiguator when the broader whitespace-insensitive lookup has multiple candidates and the exact
primary or alias spelling matches exactly one of them. This keeps plain-name query/provider lookup
ambiguous while resolving the source-backed type-reference rows. A representative run on
2026-05-12 rebuilt `target/uat/t144-sh-search-ru.sqlite` from
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` with 25415 documents in 44933 ms, reran
`syntax type-ref-gaps --format json` twice with byte-identical output and updated RU totals to:
`47156` total, `29784` resolved, `17367` unresolved, `5` ambiguous and `353` template-binding rows.
The remaining 5 ambiguous rows are all `ЭлементыФормы` `property_type` rows with candidate ids
`platform_type:ЭлементыФормы:Форма` and
`platform_type:ЭлементыФормы:Форма клиентского приложения`. The source pages prove these are
distinct `Controls` and `FormItems` platform identities. Resolving them safely requires preserving
type-reference link targets or an equivalent source-owned target identity during parsing/indexing;
T144 keeps them explicit rather than choosing a hidden winner from owner names or availability.

T162 makes enum definition documents type-like provider targets for type-reference resolution. A
representative debug run on 2026-05-13 rebuilt `target/uat/t162-enum-type-ref-ru.sqlite` from
`/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` with 25415 documents, schema version `15`, CLI
`elapsed_ms: 113307` and `/usr/bin/time` `elapsed=114.87`, `rss_kb=352836`, `exit=0`.
`syntax type-ref-gaps --format json` was run twice with byte-identical output and updated RU totals
to: `47156` total, `31638` resolved, `15513` unresolved, `5` ambiguous and `379`
template-binding rows. All 15 `ОбновлениеПредопределенныхДанных` references now resolve to
`enum:system:ОбновлениеПредопределенныхДанных`: 5 `property_type`, 5 `parameter_type` and 5
`return_type` rows. An inventory SQL check found `0` unresolved rows whose target spelling exactly
matches one unique enum document. Remaining unresolved exact document-name matches are not treated
as type-like by this task: they are BSL primitive/domain spellings that also appear as
`global_method`/`type_property` documents, enum values such as `Дата` / `Null`, query-table members
or global properties. They remain unresolved until a source/domain-specific provider relation is
implemented instead of guessing from document names.

T163 analyzed and optimized the current release-profile `syntax index` build path without changing
schema version `15`. The baseline command was:

```bash
/usr/bin/time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' \
  target/release/v8-context-hbk syntax index \
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/perf-index/baseline-shcntx-ru.sqlite
```

Baseline measured `17.47s`, `287052 KiB` peak RSS, `25415` documents and a `197M` SQLite file.
Release `syntax export` on the same source measured `11.23s` and `151912 KiB`, so the extra index
cost was attributed to SQLite/index construction rather than HBK extraction alone. After the safe
allocation/data-structure pass, two release rebuilds measured `14.90s / 286568 KiB` and
`14.56s / 286660 KiB`, both with a `197M` SQLite file. Final SQL inventory stayed equal to the
baseline: `25415` `documents`, `132908` `document_names`, `58128` `relations` and `47156`
`type_refs`. `syntax type-ref-gaps --format json` stayed at `47156` total, `31638` resolved,
`15513` unresolved, `5` ambiguous and `379` template-binding rows. Representative `syntax get`
for `ОтборКомпоновкиДанных` and keyword `syntax search` for `отбор скд` both completed
successfully against the optimized index.

T149 confirmed the query-table read-phase reuse path selected by ADR-0011/T127. Focused
instrumentation over the full `extract_with_loader_into` reader flow now proves that a query-table
page parsed during parent-identity discovery is loaded once and reused during record emission rather
than reparsed. The change does not add a generic cache knob and does not change provider JSON,
consumer export JSON, SQLite schema, query-table identity rules or no-fallback behavior for missing
query-table member owners.

A representative debug index rebuild on 2026-05-12 used:

```bash
/usr/bin/time -f 'elapsed=%e rss_kb=%M exit=%x' \
  cargo run -p v8-context-hbk-cli -- syntax index \
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/t149-query-table-reparse.sqlite
```

The command completed successfully with existing `DUPLICATE_DOCUMENT_ID` warnings, produced `25415`
documents and `56769` relations, reported CLI `elapsed_ms: 38232`, and `/usr/bin/time` reported
`elapsed=40.38`, `rss_kb=346392`, `exit=0`. SQLite checks found zero duplicate final
`documents.id` groups and zero `query_table_field` / `query_table_parameter` documents owned by
generic `query_table:Основная таблица`.

T150 aligned UAT-HBK-001, UAT-HBK-002 and UAT-HBK-003 with the active UAT template by adding
explicit pass/fail criteria, cleanup rules and skip notes. `scripts/uat/hbk-cli-smoke.sh` is the
local executable black-box smoke harness for those three cases. It runs the public CLI through
Cargo, validates exit code and representative output shape for `inspect`, `toc --format json` and
`page --path`, records fixture absence as `SKIP` with a reason and returns non-zero only for failed
cases. This smoke remains local-only for now because CI provisioning for
`/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk` and `fmtdui_ru.hbk` is not specified.

T158 replaced the `hbk-book` Book/TOC pre-tokenizing parser internals with a `winnow`-backed cursor
while preserving the existing text grammar and public behavior. Focused regressions cover BOM
trivia, comma separators and doubled quotes for Book metadata and TOC titles. Release-profile
end-to-end `v8-context-hbk toc --format json` measurements on 2026-05-13 used the real local
8.5.1.1150 books. On `shcntx_ru.hbk`, the old parser measured `1.76`, `1.76`, `1.69`, `1.72`,
`2.17` seconds with about `80121 KiB` average max RSS; the `winnow` parser measured `1.65`, `1.72`,
`1.66`, `1.65`, `2.27` seconds with about `88119 KiB` average max RSS. On `fmtdui_ru.hbk`, both
paths remained below `/usr/bin/time`'s `0.01s` resolution and around `4.6-4.7 MiB` max RSS. The
accepted result is better parser maintainability and lower non-outlier `shcntx_ru.hbk` CLI time
readings, with average wall time roughly unchanged by outliers and higher process max RSS in this
end-to-end path.

T167 measured the first SQLite-first HBK fact snapshot materialization hypothesis for OpenSpec
change `provider-owned-hbk-fact-snapshot`. The current release CLI first rebuilt a schema-16
provider index:

```bash
/usr/bin/time -f 'index_elapsed_seconds=%e\nindex_peak_rss_kib=%M\nindex_exit_status=%x' \
  target/release/v8-context-hbk syntax index \
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/snapshot-materialization/shcntx_ru.schema16.release.sqlite
```

This produced `25415` documents in `14.50s` with `284360 KiB` peak RSS. A temporary release
snapshot measurement harness then bulk-read provider-owned SQLite tables without using public lookup
APIs:

```bash
/usr/bin/time -f 'time_elapsed_seconds=%e\ntime_peak_rss_kib=%M\ntime_exit_status=%x' \
  target/release/examples/measure_snapshot_materialization \
  target/snapshot-materialization/shcntx_ru.schema16.release.sqlite
```

The compact SQLite -> snapshot probe materialized in `474 ms` (`0.55s` process elapsed), with
`49112 KiB` peak RSS, `46540 KiB` RSS delta and `34935365` estimated heap bytes. It loaded `25415`
documents, `2465` type identities, `121` type templates, `18609` members, `8337` callables,
`8675` signatures, `9793` parameters, `47156` type refs, `58128` relations and `728` document
metadata rows. Derived indexes contained `102655` name keys, `18607` member owner/name keys,
`8329` callable owner/name keys and `32555` relation source/kind keys. A deterministic
representative lookup loop of `20000` iterations measured `8922159 ns` total and `446 ns` average.

Conclusion: provider-owned SQLite bulk materialization is accepted as the first implementation
source for the worker-safe HBK fact snapshot. Direct HBK book reading remains an index refresh/setup
path and comparison baseline, not the worker hot path. The measured probe is not yet the final
public snapshot DTO layout, but it is now contract-shaped rather than a wide copy of SQLite tables:
search/export/index-maintenance payloads and raw storage paths are excluded from the snapshot
baseline. The temporary harness is service code and is not kept as a public crate example.

T168 implemented the first `HbkFactSnapshot` / `HbkFactReadHandle` in-memory snapshot API and
measured it against the same release schema-16 `shcntx_ru` index. A temporary release harness over
`HbkFactSnapshot::from_path` produced these stable warm readings after excluding first-run/cache
warm-up observations from the baseline:

| Run | Snapshot Build | Process Elapsed | Peak RSS | Estimated Snapshot Heap |
| --- | ---: | ---: | ---: | ---: |
| 1 | `511 ms` | `0.52s` | `105708 KiB` | `18197557 bytes` |
| 2 | `601 ms` | `0.62s` | `105844 KiB` | `18197557 bytes` |
| 3 | `507 ms` | `0.52s` | `105844 KiB` | `18197557 bytes` |

The implemented snapshot loaded `59771` strings, `1754` platform types, `18167` type members,
`8337` callables, `601` globals, `53` query tables, `498` query fields, `56` query parameters and
`0` language facts from the `shcntx_ru` provider index. Estimated heap is snapshot-owned storage
only; peak RSS is process-level and includes SQLite/materialization transients. The warm build
baseline is `507-601 ms`, median `511 ms`, with process peak RSS `105708-105844 KiB`.

Follow-up design conclusion: the first arena/read-handle slice is accepted as the measurement
baseline, but the next physical snapshot shape should be analyzer-query-shaped rather than
public DTO-family-shaped. Hot-path indexes must prioritize known-owner member/callable lookup,
constructor lookup, module-context lookup, static query-table field/parameter lookup, exact fact
lookup, template-key lookup, compact availability and relation traversal by supported relation kind.
Fields used only for descriptions, previews, long documentation, raw provenance or arbitrary fuzzy
search remain DTO/search-index concerns unless a later measurement proves they belong in worker
lookup.

T169 and later snapshot-layout changes must compare against this T168 baseline with release warm
runs. The comparison must report at least snapshot build time, process peak RSS, estimated
snapshot-owned heap, node/string/index heap breakdown, per-index counts/bytes and batched lookup
timings after source open. A larger index set is acceptable only when the measured hot-path lookup
benefit and memory cost are both recorded.

T169 partial snapshot-read-model measurement added a dedicated release harness:

```bash
cargo build --release -p syntax-helper-search --example measure_hbk_fact_snapshot
/usr/bin/time -f 'time_elapsed_seconds=%e\ntime_peak_rss_kib=%M\ntime_exit_status=%x' \
  target/release/examples/measure_hbk_fact_snapshot \
  target/snapshot-materialization/shcntx_ru.schema16.release.sqlite 20000
```

Three after-change warm runs reported snapshot build times of `664 ms`, `747 ms` and `640 ms`, for
a `664 ms` median, `102704-102716 KiB` process peak RSS and `20345723` estimated snapshot-owned
heap bytes. Snapshot-owned heap is about `11.8%` above the T168 `18197557` byte baseline and peak
RSS is below the T168 `105844 KiB` warm high, both inside the T169 tolerance. Build time remains
above the T168 `511 ms` median +15% threshold.

The largest new/reshaped index families are:

- `relations_by_source_kind`: `71977` entries / `1624392` bytes;
- `members_by_owner_name_kind`: `35912` entries / `1048576` bytes;
- `availability_by_fact`: `124955` entries / `893036` bytes;
- `members_by_owner_name`: `35912` entries / `786432` bytes;
- `fact_ids`: `29466` entries / `393216` bytes;
- `availability_since_by_fact`: `28725` entries / `393216` bytes.

Batched lookup timings for `20000` iterations stayed far below the NFR-RESOLVE-001 `100 ms`
resolver/API ceiling after source open. Warm-run average timings ranged from exact fact id
`100-120 ns`, type name `290-383 ns`, type template key `128-142 ns`, member by owner/name/kind
`294-300 ns`, callable by owner/name `383-441 ns`, constructor by type `21-28 ns`, global by
domain/name/kind `354-434 ns`, module context by kind `681-945 ns`, query table by name
`642-789 ns`, query field by table/name `268-319 ns`, query parameter by table/name
`415-447 ns`, availability by fact `121-126 ns` and relation by source/kind `41880-43686 ns`.

T169 stabilization completed the explicit resolver backend split required before the downstream
`v8-context` analyzer path can treat the snapshot as worker-safe. `context-resolver-search` now
exposes snapshot-backed `PlatformSnapshotSource` and `QueryTableSnapshotSource` adapters composed
from provider-owned `Arc<HbkFactSnapshot>` state. They project `HbkFactReadHandle` facts into
existing `context-resolver-core` DTOs for platform type, member, callable, global context, module
context, related/availability and query table/field/parameter lookup without reading SQLite or
falling back to `SearchIndex` inside migrated methods.

`PlatformSearchSource` and `LanguageSearchSource` remain the explicit SQL/SearchIndex-backed
backend for CLI, debug, index inspection and sequential local resolver scenarios, not downstream
analyzer hot paths. Backend selection is visible at composition time through separate source types.
Enum and enum-value facts now participate in snapshot exact-id, relation and availability lookup
surfaces, and the migrated snapshot-backed platform adapter maps them for exact-id/type/relation
cases covered by the slice.

Final T169 stabilization measurement used the same release harness and representative
`target/snapshot-materialization/shcntx_ru.schema16.release.sqlite` index, with the optional
experimental cache path supplied to preserve the T170 comparison evidence:

```bash
cargo build --release -p syntax-helper-search --example measure_hbk_fact_snapshot
/usr/bin/time -f 'time_elapsed_seconds=%e\ntime_peak_rss_kib=%M\ntime_exit_status=%x' \
  target/release/examples/measure_hbk_fact_snapshot \
  target/snapshot-materialization/shcntx_ru.schema16.release.sqlite \
  20000 \
  target/snapshot-materialization/t169-prototype-cache.bin
```

Three runs reported SQLite materialization build times of `2317 ms`, `788 ms` and `943 ms`; the
first run is retained as cache-warm-up evidence, while the post-build warm range is `788-943 ms`.
Peak RSS was `105860-106164 KiB`. The SQLite-materialized snapshot estimated `23324034` bytes of
snapshot-owned heap and `17950274` payload bytes after enum/enum-value coverage was added. The same
runs wrote an `11364011` byte experimental binary cache, read it in `39 ms`, `29 ms` and `30 ms`,
and reported `binary_cache.roundtrip_equal=true` each time. The warmed cache-load path is therefore
about `26-31x` faster than the same-run SQLite materialization startup path, but this remains T170
prototype evidence only.

The warmed SQLite materialization time is above the original T168 median +15% threshold. T169
accepts that tradeoff because the regression is isolated to the startup/materialization path, while
the stabilized read handle and snapshot-backed resolver paths keep analyzer hot-path lookups in the
nanosecond/microsecond class and remove SQL/SearchIndex dependency from migrated worker lookups.
The responsible startup components remain SQLite row read/decode, fact arena construction and
fact-id/relation/availability construction, with additional enum/enum-value arena/index work in the
stabilized shape. T170 owns reducing this startup path through a derived cache after invalidation
metadata and final format are specified.

T170 stage-timing instrumentation extends the same release harness with measurement-only
`HbkFactSnapshot` build buckets. It does not change the canonical SQLite provider index or select a
persisted snapshot format. Five warm runs against
`target/snapshot-materialization/shcntx_ru.schema16.release.sqlite` reported snapshot build times
of `618 ms`, `649 ms`, `618 ms`, `625 ms` and `641 ms`, for a `625 ms` median. Process peak RSS
stayed in the `102596-102716 KiB` range and estimated snapshot-owned heap stayed at
`20345723` bytes.

Median stage timing from those runs:

- SQLite index open: `<1 ms`;
- SQLite row read/decode: `228 ms`;
- lookup-map construction: `2 ms`;
- platform-type arena/index construction: `9 ms`;
- type-reference grouping: `35 ms`;
- signature/parameter nesting: `21 ms`;
- fact arena construction: `164 ms`;
- fact-id, relation and availability construction: `89 ms`;
- secondary-index sorting: `20 ms`;
- final snapshot assembly: `11 ms`.

Conclusion: the largest measured startup components are repeated SQLite row reading/decoding,
fact arena construction and relation/availability/fact-id construction. A persisted snapshot cache
experiment is therefore worth measuring after T169 stabilizes the physical read model, but the cache
must remain a derived provider-owned artifact with SQLite as the rebuildable source.

The first T170 binary-cache prototype uses a measurement-only provider-owned little-endian format
with magic, cache version and provider schema version guards. It adds no new runtime dependency and
is not accepted as a downstream storage contract; the Rust API is explicitly named
`write_experimental_binary_cache` / `from_experimental_binary_cache`. The release harness now
accepts an optional cache path and compares SQLite materialization with binary cache write/read:

```bash
cargo build --release -p syntax-helper-search --example measure_hbk_fact_snapshot
/usr/bin/time -f 'time_elapsed_seconds=%e\ntime_peak_rss_kib=%M\ntime_exit_status=%x' \
  target/release/examples/measure_hbk_fact_snapshot \
  target/snapshot-materialization/shcntx_ru.schema16.release.sqlite \
  20000 \
  target/snapshot-materialization/hbk-fact-snapshot.bin
```

The first prototype comparison before T169 stabilization reported SQLite materialization build
times of `645 ms`, `643 ms`, `629 ms`, `605 ms` and `683 ms`, for a `643 ms` median. The binary
cache file was `10319044` bytes (`9.9 MiB`). Binary cache reads were `25 ms`, `25 ms`, `25 ms`,
`24 ms` and `26 ms`, for a `25 ms` median. Binary cache writes were `11 ms`, `44 ms`, `44 ms`,
`31 ms` and `48 ms`, for a `44 ms` median. Each run reported
`binary_cache.roundtrip_equal=true`.

That pre-stabilization prototype estimated `20345723` bytes for the SQLite-materialized snapshot
and `16597927` bytes for the cache-loaded snapshot because the binary reader allocates vectors with
exact capacities instead of preserving materializer growth capacity. After T169 enum/enum-value
coverage, the same effect remains visible at the new physical shape: SQLite-materialized snapshot
heap was `23324034` bytes, while cache-loaded snapshot heap matched the exact-capacity payload at
`17950274` bytes. Follow-up measurement must use the harness payload counters
(`snapshot_payload_bytes` and per-entry `payload_bytes`) alongside capacity-based heap counters
before drawing structural memory conclusions.

T170 stabilizes that path as the first provider-owned derived snapshot cache format. The accepted
format remains a no-dependency little-endian Rust DTO cache internal to `syntax-helper-search`.
The artifact carries cache format version, provider SQLite schema version, source-index identity
fingerprint from provider metadata, persisted source-index identity when available and file
size/mtime, locale/source-locale/source-HBK metadata, source extraction schema version, snapshot
layout version/flags, payload length and FNV-1a payload checksum. Payload length is capped before
allocation. Missing, unsupported, stale, truncated or corrupted caches are invalidated and rebuilt
from the canonical SQLite provider index by `HbkFactSnapshot::from_path_with_binary_cache`; resolver
adapters still receive only loaded `Arc<HbkFactSnapshot>` state or read handles. Cache writing is
available from an `HbkFactSnapshotBuildReport` produced by the same provider index, not from an
arbitrary snapshot/index pair.

The final T170 release comparison used the existing
`target/snapshot-materialization/shcntx_ru.schema16.release.sqlite` provider index and the stable
cache path:

```bash
cargo build --release -p syntax-helper-search --example measure_hbk_fact_snapshot
/usr/bin/time -f 'time_elapsed_seconds=%e\ntime_peak_rss_kib=%M\ntime_exit_status=%x' \
  target/release/examples/measure_hbk_fact_snapshot \
  target/snapshot-materialization/shcntx_ru.schema16.release.sqlite \
  20000 \
  target/snapshot-materialization/t170-stable-cache-final-run-1.bin
```

The first run remained cache-warm-up evidence: SQLite materialization was `1761 ms`, cache
validation/load was `42 ms`, cache write was `43 ms`, peak RSS was `106024 KiB` and the process
exited successfully. The next two warm runs measured SQLite materialization at `665 ms` and
`658 ms`; cache validation/load at `34 ms` and `35 ms`; cache writes at `32 ms` and `32 ms`; and
peak RSS at `106296 KiB` and `106300 KiB`. The stable cache file was `11318100` bytes, and every
run reported `binary_cache.status=loaded` and `binary_cache.roundtrip_equal=true`.

The SQLite-materialized snapshot reported `23184770` capacity-based heap bytes and `17846774`
logical payload bytes. The cache-loaded snapshot reported `17846774` heap bytes and `17846774`
payload bytes because the binary reader allocates exact vector capacities; this remains an
allocation-capacity effect, not a structural model shrink. Representative read-handle lookup
timings stayed in the analyzer hot-path class on warm runs: exact fact id `92-100 ns`, type by name
`248-253 ns`, type-template key `119-122 ns`, owner member scan `26-28 ns`, owner/name/kind member
lookup `252-258 ns`, callable owner/name lookup `336-356 ns`, query table name lookup
`569-1221 ns`, query field lookup `240-801 ns`, query parameter lookup `364-385 ns`, availability
lookup `117-127 ns` and relation traversal `54132-54448 ns`.

T170 therefore accepts the simple provider-owned little-endian cache as stable enough for the first
runtime startup path. A new serialization, zero-copy or memory-mapped dependency is not justified
by the current measurement; it would require a later task to show that cache deserialization or
allocation, rather than SQLite materialization, is again the limiting startup component.

Baseline update rule:

- Rebuild the relevant `shcntx_ru.hbk` and/or `shcntx_root.hbk` index from the current source,
  rerun `syntax type-ref-gaps --format json` twice and compare the reports before changing gate
  values.
- Promote only conclusions, counts and update rationale into this file. Raw JSON reports, SQLite
  indexes and command logs remain service data under `target/`.
- If a change intentionally tightens a gate, record the old value, the new value and the task or
  ADR that owns the behavioral reason.
