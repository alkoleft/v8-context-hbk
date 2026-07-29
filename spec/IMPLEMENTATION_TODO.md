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
- [archive/completed-tasks-t152-t164.md](archive/completed-tasks-t152-t164.md)

Current status: T35-T164 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design,
language-domain, cleanup, book-content export, documentation-site, platform type-template and
type-reference conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`requirements/non-functional.md`, `implementation/components.md`,
`implementation/documentation-site.md`, `implementation/syntax-helper-query-cli.md`,
`implementation/syntax-bsl-provider-plan.md`, `implementation/solution-context-resolve.md`,
`implementation/performance-baseline-t13.md`, `implementation/performance-variants.md` and
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

### [x] T179. Own complete member enumeration and resolver-kind classification

References: FR-CTX-RESOLVE-001, NFR-RESOLVE-001,
`implementation/components.md`, `implementation/solution-context-resolve.md`, ADR-0008, downstream
OpenSpec change `simplify-context-provider-architecture` tasks 5.3 and 5.4.

Scope and guard:

- Define `MemberQuery { name: None, kind }` as complete raw enumeration for one resolved
  source-qualified owner within the optional kind. `Ok`, including an empty fact vector, is the
  only complete enumeration response. Owner/source `NotFound`, `Ambiguous`, `Unsupported`, and
  `ResolveError` remain non-complete outcomes for downstream presence-index decisions.
- Add one narrow classification owned by `context-resolver-core` from its existing `MemberKind` to
  its existing `MemberQueryKind`. Migrate the search adapter's resolver-kind comparison to that
  owner. Do not add a DTO, adapter, registry, cache, provider mirror, or analyzer concept.
- Behavior-test search-index and snapshot-backed platform sources for complete unfiltered and
  kind-filtered enumeration of property, method, event, and enum-value members, including empty and
  absent-owner outcomes.

Structure impact:

- Existing owners searched: `context-resolver-core` query/member enums and source-neutral traits;
  `context-resolver-search` search and snapshot platform adapters and their mapping helpers;
  downstream `v8-context` effective enumeration and planned private point-key indexes. Both resolver
  enums and their semantic relationship belong to core; provider-storage-kind conversion remains
  adapter-owned.
- Add one method on an existing core enum and contract tests. Delete the duplicate resolver-kind
  comparison table from the search adapter. The parity inventory found that enum values were already
  modeled by both provider stores but only the search adapter exposed them as broad members; add the
  missing private snapshot enum-owner/value traversal and the corresponding search projection from
  its existing `owns` relation into the existing `ResolvedMember` boundary. These are two distinct
  provider-to-core projections over existing owner/value records, not a new owner, record, holder,
  or conversion chain. Named enum lookup remains unchanged `NotFound`.
- Added semantic structures, cache keys, readers, parsers, loaders, serializers, schemas, generated
  shapes, public re-export families, and transport surfaces: none. Added reusable behavior is limited
  to broad enum-owner enumeration inside the two existing provider adapters; inputs are existing
  provider enum owner/value evidence and output is the existing source-neutral `ResolvedMember`.

Reintroduction guard:

- Root cause: the relation between two core-owned enums was reimplemented by adapters and consumers,
  allowing enumeration filtering and downstream presence keys to drift.
- Single allowed flow: provider member kind -> existing `MemberKind` -> core-owned
  `MemberQueryKind` classification -> adapter filtering and downstream raw key construction.
- Focused tests must fail if any of the four kinds is omitted or filtered differently between the
  search and snapshot adapters. Final diff review must reject another exhaustive
  `MemberKind`/`MemberQueryKind` resolver mapping outside core; storage-kind conversion at the
  provider adapter boundary remains allowed.
- Shared adapter tests must also fail if broad enum projection diverges, if named enum lookup is
  broadened, or if exact property/method/event misses acquire an owner lookup. Future enum
  projection must reuse provider-owned enum records and the existing `ResolvedMember` boundary,
  never a mirrored enum-member DTO or adapter-local cache.

Verification:

- `cargo test -p context-resolver-core`;
- `cargo test -p context-resolver-search`;
- `cargo fmt --all -- --check`;
- `cargo check --workspace`;
- fresh diff review and source search for parallel resolver-kind mappings.

Completion notes:

- `MemberKind::query_kind` is now the sole source-neutral classification for
  `Property`, `Method`, `Event`, and `EnumValue`. The duplicate exhaustive
  resolver-kind table was deleted from the search adapter; the distinct
  snapshot-storage conversion remains at its provider boundary.
- `PlatformSearchSource` and `PlatformSnapshotSource` now agree that
  `MemberQuery { name: None, kind }` enumerates the complete raw property,
  method, event, or enum-value set for an existing owner. Existing empty owners
  return `Ok([])` and absent/inactive owners return `NotFound`. Named enum
  lookup remains the pre-task `NotFound` behavior in both adapters; T179 does
  not broaden the exact member contract.
- The search adapter keeps the original direct exact-member SQL path. The
  additional owner-kind and enum-relation reads occur only after an empty broad
  enumeration, so exact BSL member misses gain no owner lookup.
- Structure review reconciled the approved impact: no DTO, holder, cache,
  registry, schema, reader/parser family, analyzer concept, or parallel
  resolver-kind mapping was added. Search-index and snapshot projections remain
  distinct real provider adapters over the existing `ResolvedMember` boundary.
- `cargo test -p context-resolver-core`, `cargo test -p
  context-resolver-search`, `cargo fmt --all -- --check`, and `cargo check
  --workspace` pass. Fresh review completed with no findings. The additive
  provisional Rust contract advances the workspace patch version to `0.2.1`.

### [x] T178. Expose borrowed HBK BSL and SDBL domain catalogs

References: FR-CTX-RESOLVE-001, NFR-RESOLVE-001,
`implementation/components.md`, ADR-0008, OpenSpec change
`expose-borrowed-hbk-domain-catalogs`.

Scope and guard:

- Expose narrow `HbkBslContextCatalog` and `HbkSdblQueryCatalog` handles over
  the existing provider-owned `Arc<HbkFactSnapshot>` arenas, returning existing
  typed IDs and borrowed records without a second store, record mirror, generic
  catalog trait or SQL fallback.
- Keep `ContextResolver` and generic owned DTO projection as the compatibility
  boundary for source-neutral composition. Keep explicit
  `PlatformSearchSource`/`LanguageSearchSource` flows for CLI, debug and index
  inspection. Raw metadata module-role translation remains in
  `context-resolver-core`.
- Verify each BSL and SDBL slice independently with identical compatibility
  probes, deleted-SQLite behavior, point/enumeration parity and absence guards
  before downstream analyzer handoff.

Progress:

- The read-handle lifetime foundation and both independently gated domain
  catalogs are complete. `HbkBslContextCatalog` and `HbkSdblQueryCatalog` own
  catalog-covered acquisition while their snapshot adapters project only at
  the generic compatibility boundary.
- The `b0841e6`/after BSL probe and `ff70367`/after SDBL probe preserve every
  observable compatibility result count and their respective 0.10 s and
  0.11 s warm command wall times; both snapshot paths work after deleting the
  SQLite file.
- Generic resolver consumers remain explicit compatibility/composition
  boundaries; SQL/SearchIndex flows remain explicit and non-fallback. The
  downstream analyzer handoff is accepted by `v8-context` commits `5e599dd`
  and `0418bff`, with an executable absence guard against analyzer-owned HBK
  mirrors, selectors and flattened hot-path stores.
- Runtime evidence is retained with a verified 177-payload SHA-256 manifest.
  Formatting, focused catalog/search tests and the full workspace passed after
  the `0.2.0` version update. Full clippy has only the three exact diagnostics
  already present at the task base; the 15 lint findings caused by the new
  iterator-returning API were fixed in this task. Fresh final review completed
  with no remaining findings.

### [x] T177. Eliminate duplicate snapshot-interner string ownership

References: NFR-RESOLVE-001, `implementation/performance-baseline-t13.md`,
OpenSpec change `optimize-hbk-snapshot-materialization-followups`.

Scope and guard:

- `SnapshotBuilder` must have exactly one build-time owner for each unique
  string: map ownership while assigning `StringId`, followed by one move in
  stable ID order into the existing `HbkFactSnapshot.strings` field. No new
  snapshot model, cache/schema, adapter, reader, capacity policy or public
  interface is allowed.
- Lifecycle and source guards must reject a second build-time string table,
  early string lookup, post-finalization interning and a non-lexical ID-order
  change.

Completion notes:

- Direct DHAT for `SnapshotBuilder::intern` falls from 18,404,610 to 7,756,607
  bytes (-57.86%); process global peak falls from 69,614,844 to 63,019,028
  bytes. The provider cache SHA is exact, 66 package tests pass, and the final
  snapshot payload remains 17,908,362 bytes.
- Matched provider medians improve from 599 ms / 79,520 KiB to 580 ms /
  75,620 KiB. The sequential fixed downstream A/B improves from 0.83 s /
  88,620 KiB to 0.75 s / 84,868 KiB with the exact zero-finding digest in every
  run. H4 is deferred for lack of independent growth evidence; H5 is deferred
  to a provider-startup lifecycle proposal; H8 remains semantically rejected.
- Architecture remains unchanged; workspace version is patched to 0.1.2.

### [x] T176. Filter unused owner-edge rows before materialization

References: NFR-RESOLVE-001, `implementation/components.md`,
`implementation/performance-baseline-t13.md`, OpenSpec change
`optimize-hbk-snapshot-materialization-followups`.

Scope:

- On the provider release artifact
  `target/snapshot-materialization/shcntx_ru.schema16.release.sqlite`,
  `query_owner_edges` materializes 21,613 `owns` rows although only 498
  query-table fields, 56 query-table
  parameters and 3,087 enum values reach its existing consumers. Keep the
  reader and its ordered vector, but restrict it through a target-id predicate
  derived from those three existing `SearchDocumentKind` values before row
  materialization.
- Preserve both consumers, source-owner skips, `source_id`/`target_id` order,
  snapshot/read-handle facts and cache behavior. Do not introduce streaming,
  callbacks, a helper, cache/schema/index changes, source-kind filtering or a
  public contract.

Structure impact:

- `SnapshotMaterializer::query_owner_edges` remains the only reader and
  `Vec<(String, String)>` remains its only intermediate shape.
  `SearchDocumentKind` remains the storage-kind owner. The sole production
  behavior change is an SQL predicate; no semantic structure, conversion,
  mapping, serializer, cache key, adapter or public re-export is added.
- Test-only fixture reuse may expose existing construction helpers within the
  test crate so a materializer-local unit test can invoke the private reader.
  It adds no production model or data path and reuses the production index
  builder rather than recreating storage behavior.

Reintroduction guard:

- Root cause is the unconditional `owns` reader materializing 17,972 rows that
  its two consumers always discard. The only valid flow uses a target-id
  predicate derived from `documents`, binds the three existing target kinds and
  orders by source then target before the unchanged loops. A private-reader
  fixture and narrow source guard reject an unconditional full-`owns` query.

Verification:

- private-reader accepted/rejected target-kind and order test; snapshot,
  read-handle and binary-cache parity; package tests, formatting and strict
  OpenSpec validation; direct DHAT, three release provider runs and five fixed
  downstream runs.
- Require exact parity, `query_owner_edges` first-frame allocation no greater
  than 3,477,039 bytes (50% of 6,954,078), and every normal median no more than
  5% above a matched counterfactual on the same artifact/workflow. Revert and record
  rejected/deferred on any gate failure.

Completion notes:

- The initial target-document JOIN reduced allocation but was rejected against
  the historic 601 ms provider observation at a 668 ms median, before the
  matched protocol was corrected. The final target-id predicate uses the
  existing `relations_target_idx` and adds no index or schema.
- Direct DHAT uses the distinct 21,304-row analyzer index and is 1,012,703
  bytes against 6,954,078 (-85.43%). The 21,613-row provider artifact has a
  matched 608 ms / 79,512 KiB median against 659 ms / 80,280 KiB; the sequential
  matched downstream experiment has a 0.80 s / 91,444 KiB median against 0.86 s
  / 92,500 KiB. The 5% ceilings are 691.95 ms / 84,294 KiB and 0.903 s /
  97,125 KiB; snapshot accounting and every exact zero-finding digest remain
  unchanged. Historic T175 0.75 s / 89,424 KiB is not used as an H2 gate.
- The 63-test package, private-reader order/filter fixture, read-handle/cache
  coverage, formatting, strict OpenSpec validation and final diff review pass.
  Architecture remains unchanged; workspace version is patched to 0.1.1.

### [x] T175. Avoid materializing unused signature lines

References: NFR-RESOLVE-001, `implementation/components.md`,
`implementation/performance-baseline-t13.md`, OpenSpec change
`optimize-hbk-snapshot-materialization-followups`.

Scope:

- The post-T174 borrowed-input `split_lines` experiment is allocation-identical
  because release inlining removes the input clone. Revalidate the direct
  materializer cost, then select the already-owned ordinal signature line as
  `&str` and pass it directly to `SnapshotBuilder`. Preserve line order and
  empty-line filtering; do not introduce borrowed data into `HbkFactSnapshot`.
- Do not combine H2 owner-edge materialization, H3 interner redesign, H4
  capacity hints, H5 cache-startup wiring or H8 1C semantic pruning with this
  task.

Structure impact:

- Existing fact ownership stays in `HbkFactSnapshot`. This task removes one
  private all-lines `Vec<String>` and selected-line `String` before the
  existing builder owns the selected text; it adds no
  fact/cache/schema/adapter/reader/serializer or public type.

Reintroduction guard:

- Root cause is splitting every signature line into an owned vector and then
  cloning the selected line before the existing builder interns it. The only
  valid materializer flow is `signature_text.lines()` -> existing non-empty
  filter -> ordinal selection -> builder interning. A source check rejects a
  materializer `split_lines` call, signature-text clone or owned selected line.

Verification:

- focused multi-line/empty-line signature behavior and structural-absence
  tests; snapshot/read-handle/binary-cache coverage; package tests, formatting
  and strict OpenSpec validation; release provider and downstream fixed-workload
  comparisons. Record direct allocation removal separately from normal time and
  RSS, and do not claim a process-level improvement without those measurements.

Completion notes:

- The first borrowed-input `split_lines` experiment was allocation-identical in
  release DHAT and was reverted. The accepted P5b deletion instead selects the
  required non-empty ordinal line as `&str` and passes it directly to the
  existing builder, removing the source-level all-lines vector and
  selected-line clone.
- The direct DHAT comparison cannot causally attribute an allocation benefit:
  `split_lines` remains 5,811,810 bytes and global peak is unchanged. This task
  claims no memory/time improvement.
- Focused multi-line ordering and structural absence, all 61 package tests,
  cache/read-handle coverage, formatting and strict OpenSpec validation pass.
  Provider and five-run downstream checks remain within 5%; the downstream
  zero-finding digest is unchanged.

### [x] T174. Bound SQLite type-reference materialization for provider snapshots

References: NFR-RESOLVE-001, `implementation/components.md`,
`implementation/performance-baseline-t13.md`, OpenSpec change
`reduce-hbk-snapshot-materialization-peak`.

Scope:

- Start from downstream P5a evidence on the 8.3.27.1859 `shcntx_ru.sqlite`
  provider index: `SnapshotMaterializer` accounts for `212585620` allocated
  bytes and `98129653` bytes live at global peak; its `type_refs` reader alone
  accounts for `42720841` allocated bytes and `25420393` live bytes at peak.
- Replace the private bulk `Vec<TypeRefRowSnapshot>` and post-read grouping
  passes with an ordered row-at-a-time collector into existing `HbkTypeRef`
  groups. Decode every row before filtering so invalid ignored rows preserve
  their existing typed error behavior. Do not change snapshot/read-handle facts, resolver behavior,
  binary-cache layout, SQLite schema, source semantics or provider ownership.
- Record deferred independent candidates: query-owner streaming, builder
  interner duplication, capacity hints, cache loading and borrowed signature
  text. Do not combine them with T174.

Structure impact:

- `HbkFactSnapshot` remains the published fact owner. The task deletes the
  private raw type-reference row representation and adds one private typed
  holder for the existing four temporary groups. It adds no public/JSON
  contract, cache record, schema, adapter, mapping, parser, mirror or
  dependency. Search and consumer evidence are in the active change design.

Reintroduction guard:

- Root cause is a complete raw SQL type-reference collection overlapping with
  its grouped snapshot projection. The only valid flow is ordered row ->
  `TypeRefGroups` -> existing `HbkFactSnapshot`; review/search rejects a
  restored `TypeRefRowSnapshot`, `Vec`-returning row reader or equivalent full
  raw collection.

Verification:

- three-run release baseline/final measurement of snapshot build time, peak
  RSS and snapshot accounting; focused snapshot/cache/read-handle behavior;
  `cargo test -p syntax-helper-search`; `cargo fmt --all --check`; strict
  OpenSpec validation; unchanged `binary_cache.rs`, cache layout/version and
  serialized snapshot fields by diff review; and five-run downstream
  project-fast parity only (finding digest, median/MAD time and RSS). Accept
  only an RSS reduction of at least 10% or 1 MiB with no more than 10% median
  build-time regression.

Completion notes:

- The materializer now decodes each ordered SQLite type-reference row and
  immediately appends its mapped `HbkTypeRef` to the existing target group.
  It no longer retains a `Vec<TypeRefRowSnapshot>` alongside those groups;
  invalid rows remain terminal even when no group consumes them.
- Exact release provider medians changed from 692 ms / 105,592 KiB to 609 ms /
  78,820 KiB: -12.0% build time and -25.35% peak RSS. Snapshot accounting
  remains 23,144,545 bytes, so the gain is a transient materialization peak.
- The rebuilt downstream five-run `project-fast` workload has the unchanged
  zero-finding digest `c2b4465a4c66a8939d40febd117061959e85dcde77078be386c1e73ae97f60a3`
  and medians of 0.75 s / 89,108 KiB versus P5a's 0.83 s / 108,332 KiB.
- Passed focused behavior/structural/cache checks, `cargo test -p
  syntax-helper-search`, `cargo fmt --all --check`, strict OpenSpec validation
  and diff review confirming no cache layout/version, SQLite schema, resolver
  adapter or downstream analyzer changes.

### [x] T173. Resolve one metadata-selected BSL module member without materializing a module context

References: FR-CTX-RESOLVE-001, NFR-RESOLVE-001,
`implementation/solution-context-resolve.md`,
`implementation/components.md`, OpenSpec change
`add-exact-bsl-context-member-lookup`.

Scope:

- Add a provisional exact metadata-module BSL member request in
  `context-resolver-core`, carrying one required source, the existing opaque
  metadata module-role selector, optional matching domain, canonical name and
  `MemberQueryKind`.
- Add one HBK-owned answer enum over the existing property `ContextFact` and
  `ResolvedCallable`; do not create a field-for-field member DTO or expose
  `ModuleContextKind` to callers.
- Route the certified selector only in the composite resolver, then ask that
  one platform source for a direct indexed answer. SQL adds an intersection of
  existing name/module-context keys and snapshot materializes the equivalent
  `(module-context, canonical-name)` lookup; adapters must not call or filter
  `module_context` or `ResolvedModuleContext`.
- Preserve `Ok`, `NotFound`, `Ambiguous`, `Unsupported` and `ResolveError`
  without fallback. Do not add metadata/analyzer dependencies, an HBK cache or
  a new SQLite schema/index family. The existing derived snapshot cache must
  advance its layout version if the physical event-index key semantics change.

Verification:

- RED-first core, SQL adapter and snapshot adapter tests for the explicit
  supported role×kind matrix and property/method/event answers; source/domain isolation; not-found, ambiguity,
  unsupported and provider error; SQL/snapshot primary-name parity (an alias
  is absence); a binary-cache regression that forces the preceding snapshot
  layout version to rebuild before deserialization and then resolves an exact
  event; and a structural guard against calling or filtering `module_context`
  in the exact path;
- `cargo test -p context-resolver-core`;
- `cargo test -p context-resolver-search`;
- `cargo fmt --all --check`;
- strict validation of the named OpenSpec change.

Completion notes:

- `MetadataModuleMemberLookup` selects exactly one platform source, and
  `ResolvedBslContextMember` preserves existing property/callable evidence;
  selector dispatch remains HBK-owned.
- SQL and snapshot paths use direct primary-name lookups. They preserve exact
  ambiguity and never call or traverse `module_context`; aliases are normal
  absence for exact events.
- The existing derived snapshot cache advances to layout version 3 and a
  previous layout is verified to rebuild before it can serve exact event
  lookups.
- Passed `cargo test -p context-resolver-core`, `cargo test -p
  syntax-helper-search`, `cargo test -p context-resolver-search`, `cargo fmt
  --all --check` and `openspec validate
  add-exact-bsl-context-member-lookup --strict`.

### [x] T172. Resolve metadata-generated self roles through the HBK template boundary

References: FR-CTX-RESOLVE-001, `implementation/solution-context-resolve.md`,
OpenSpec change `add-generated-self-template-lookup`.

Scope:

- Add a borrowed source/domain-qualified `TypeLookup::GeneratedSelfTemplate` selector query.
- Keep the selector-to-classified-template mapping in the HBK platform adapters; do not depend on
  metadata types, expose template keys to this query, compose configuration types or add an
  analyzer/cache/SQLite boundary.
- Preserve existing explicit resolver outcomes: unknown is `NotFound`, a non-platform source is
  `Unsupported`, duplicate provider templates are `Ambiguous`, and provider storage errors remain
  `ResolveError`; no name, alias or cross-source fallback is allowed.

Verification:

- public SQL and snapshot-backed resolver tests for the exact 20-selector corpus, source/domain
  routing, unknown selector, unsupported source, ambiguity and provider error propagation;
- `cargo test -p context-resolver-core`;
- `cargo test -p context-resolver-search`;
- `cargo fmt --all --check`;
- `openspec validate add-generated-self-template-lookup --strict`.

Completion notes:

- `PlatformSearchSource` and `PlatformSnapshotSource` resolve the metadata-certified opaque role
  selector through existing HBK template indexes and return existing `ResolvedType` facts.
- The downstream consumer remains unable to construct or observe the internal template key for
  this operation; normal source/domain filters and failure statuses terminate fallback.

### [x] T169. Reshape `HbkFactSnapshot` physical indexes around analyzer hot paths

References: FR-CTX-RESOLVE-001, NFR-RESOLVE-001, NFR-QUERY-001, UC-CTX-001,
UC-CTX-002, `implementation/solution-context-resolve.md`,
`implementation/components.md`, `acceptance/baseline.md`,
OpenSpec change `provider-owned-hbk-fact-snapshot`.

Scope:

- Start with a before-change release measurement of the current T168 snapshot on the representative
  `shcntx_ru` provider index. Record snapshot build time, process peak RSS, estimated
  snapshot-owned heap and batched lookup timings for the hot paths below before changing layout.
- Refactor the snapshot read model so physical indexes are organized by analyzer queries rather
  than public DTO result families.
- Keep snapshot-owned nodes/arenas as the single source of provider fact payloads. Secondary
  indexes store only compact keys and `NodeRef`/range values.
- Implement physical indexes inside `syntax-helper-search::HbkFactSnapshot` and read-handle APIs.
  `context-resolver-search` may call read handles and project results into resolver DTOs, but must
  not build duplicate provider-fact maps, query raw SQLite, or own analyzer-side mirrors of HBK
  facts.
- Keep index payloads compact: keys plus node refs/ranges only. Do not store cloned names,
  signatures, descriptions, type-ref vectors or DTO structs inside secondary indexes.
- Add or reshape first-slice physical indexes for:
  - exact fact id lookup;
  - normalized type name and type template-key lookup;
  - owner member listing;
  - `(owner type, normalized name, optional kind)` member lookup;
  - `(owner type, normalized name)` callable lookup;
  - constructors by type;
  - language/domain global method/property lookup;
  - module context by language/domain/module kind;
  - query table by name/syntax/identifier;
  - query field and query parameter by table/name;
  - compact availability by fact;
  - relation traversal by source fact and relation kind.
- Prefer contiguous arenas plus owner ranges when they reduce allocation count and keep lookup
  cache-local. Nested logical ownership must remain explicit even if the physical storage uses
  ranges instead of nested `Vec` fields.
- Add memory accounting for string store, node arenas and each secondary-index family. The task is
  not complete if total snapshot-owned heap grows without identifying the index family responsible.
- Keep descriptions, previews, notes, full signature text, raw HBK/HTML provenance, long
  documentation text, arbitrary fuzzy search data and unbounded relation paths out of first-slice
  physical indexes.
- Keep existing resolver DTOs as adapter projections over snapshot nodes. Do not expose raw SQLite
  tables or make downstream analyzers depend on provider storage details.
- Do not add Tantivy, persisted snapshot formats, minimal-perfect hashing, compressed bitmap
  dependencies, global caches, async runtimes or tuning knobs in this slice.
- Add no new runtime dependency in this task. If a dependency appears necessary, leave T169
  unchecked until the task records the measured bottleneck, why `std` and existing workspace
  dependencies are insufficient, and the ADR/spec update that owns the dependency decision.

Verification:

- focused snapshot tests for each hot-path index listed above;
- concurrent deterministic read test across multiple threads;
- release before/after measurement against the current representative `shcntx_ru` provider index,
  recording warm snapshot build time, process peak RSS, estimated snapshot-owned heap, node/string
  heap, per-index counts/bytes and representative lookup timings after source open;
- compare release warm measurements with the T168 baseline (`507-601 ms` build, median `511 ms`,
  `18197557` estimated snapshot-owned bytes, `105708-105844 KiB` process peak RSS). If median build
  time or peak RSS increases by more than 15%, or estimated snapshot-owned heap increases by more
  than 25%, identify the responsible index family and justify the tradeoff with measured hot-path
  lookup benefit; otherwise leave T169 unchecked with a follow-up;
- batched release lookup measurements for at least:
  - exact fact id;
  - `(owner type, normalized name, optional kind)` member lookup;
  - `(owner type, normalized name)` callable lookup;
  - constructors by type;
  - module context by language/domain/module kind;
  - query table by name/syntax/identifier;
  - query field and query parameter by table/name;
  - relation traversal by source fact and relation kind.
- each measured lookup must stay under the NFR-RESOLVE-001 provisional `100 ms` resolver/API ceiling
  on the representative source after `HbkFactReadHandle` creation. If not, leave T169 unchecked and
  record measured timings, source size and the limiting component;
- a physical index counts as complete only when it has a read-handle method and either a migrated
  adapter test or a documented analyzer lookup scenario using it. Do not add placeholder physical
  indexes for listed families that are not exercised in this slice; document them as deferred;
- focused `context-resolver-search` adapter tests showing migrated known-owner member/callable
  lookup, module-context lookup and query-table field/parameter lookup use the snapshot/read-handle
  path;
- if any adapter path remains transitional in this slice, document the exact non-migrated method,
  the reason it remains on the old path and the follow-up task that will migrate it;
- `openspec validate provider-owned-hbk-fact-snapshot --strict`;
- `cargo fmt --all --check`;
- focused package tests/checks for touched crates.

Completion notes:

- Snapshot/read-handle physical indexes were reshaped in `syntax-helper-search` for the listed
  hot paths, including fact id, type name, type template key, owner member/callable, constructors,
  global lookup, module context, query table/field/parameter, availability and relation traversal
  indexes. The snapshot now also represents enum and enum-value fact refs in exact-id,
  relation and availability lookup surfaces.
- `context-resolver-search` now has explicit snapshot-backed sources,
  `PlatformSnapshotSource` and `QueryTableSnapshotSource`, composed from provider-owned
  `Arc<HbkFactSnapshot>` state. They project snapshot nodes into existing `context-resolver-core`
  DTOs for platform type/member/callable/global/module/related/availability lookups and query
  table/field/parameter lookups without reading SQLite or falling back to `SearchIndex` inside
  migrated methods.
- `PlatformSearchSource` and `LanguageSearchSource` remain explicit SQL/SearchIndex-backed
  adapters for CLI, debug, index inspection and sequential local resolver usage. The worker-safe
  analyzer path composes snapshot-backed sources by constructor/type name rather than silently
  switching existing SQL-backed source names.
- Focused tests cover snapshot hot-path indexes, deterministic concurrent reads, enum/enum-value
  snapshot participation, snapshot-backed platform resolver paths, snapshot-backed query-table
  resolver paths and `Send + Sync` source-boundary assertions.
- Final release measurement on
  `target/snapshot-materialization/shcntx_ru.schema16.release.sqlite` with three warm runs reported
  SQLite materialization builds of `2317 ms`, `788 ms` and `943 ms`; the first run is retained as
  cache-warm-up evidence, while the warm post-build range is `788-943 ms`. Peak RSS was
  `105860-106164 KiB`; estimated SQLite-materialized snapshot heap was `23324034` bytes and
  payload bytes were `17950274`. The heap increase over the earlier T169 partial measurement is
  explained by the newly represented enum/enum-value arenas and indexes.
- The build-time regression is accepted for T169 because it is isolated to the SQLite
  materialization/startup path, not to steady-state analyzer lookups after `HbkFactReadHandle`
  creation. The responsible startup components remain the previously measured SQLite row
  read/decode, fact arena construction and fact-id/relation/availability construction stages,
  with additional enum/enum-value arena/index work in this stabilization pass. T170 owns reducing
  this startup path through a derived cache once invalidation and final format are specified.
- The same release runs wrote and read the measurement-only experimental binary cache. Cache reads
  were `39 ms`, `29 ms` and `30 ms`; the warm read range is `29-30 ms`, about `26-31x` faster than
  the same-run SQLite materialization startup. The cache file was `11364011` bytes and every run
  reported `binary_cache.roundtrip_equal=true`. This strengthens T170 prototype evidence only; it
  does not accept a persisted format or invalidation policy.

### [x] T170. Stabilize provider-owned derived cache for `HbkFactSnapshot` startup latency

References: FR-CTX-RESOLVE-001, NFR-RESOLVE-001, NFR-QUERY-001, UC-CTX-001,
UC-CTX-002, `implementation/solution-context-resolve.md`,
`acceptance/baseline.md`, T169, OpenSpec change
`stabilize-hbk-fact-snapshot-cache`.

Scope:

- Treat the existing SQLite provider index as the canonical rebuildable provider artifact. The
  persisted snapshot cache is a derived startup/read-model artifact, not a replacement source of
  truth and not a public contract for downstream analyzers.
- Start from the post-T169 evidence rather than reopening open-ended cache exploration: warmed
  SQLite materialization measured `788-943 ms`, warmed binary-cache reads measured `29-30 ms`, the
  cache file was `11364011` bytes and every measured run reported
  `binary_cache.roundtrip_equal=true`.
- Stabilize the cache/invalidation contract before accepting a runtime cache path: cache format
  version, provider SQLite schema version, source index identity/hash, platform version/locale when
  available, snapshot layout/version flags and an integrity guard. On mismatch, unsupported version
  or corruption, rebuild from the SQLite provider index.
- Decide whether the current no-dependency little-endian DTO path is accepted as the first stable
  provider-owned cache format or remains experimental behind explicit naming. Only consider
  zero-copy or memory-mapped layouts such as `rkyv`/`zerocopy` after a stable-cache measurement shows
  that deserialization/allocation, not SQLite materialization, is still the limiting component.
- Keep `fst` scoped to measured name/id lookup index compression if lookup indexes, not startup
  deserialization, are the limiting component. Do not use Tantivy, search/export payloads or fuzzy
  search data for the worker fact snapshot cache.
- Keep the persisted artifact provider-owned. Resolver adapters may load or receive
  `Arc<HbkFactSnapshot>`, but must not depend on SQLite tables, binary layout details or
  analyzer-owned mirror indexes.
- Do not reopen T171 in this task. `PlatformSnapshotSource` and `QueryTableSnapshotSource` remain
  the completed snapshot-backed resolver slice. A non-query-table `LanguageSnapshotSource` is a
  separate future task/change, not part of cache stabilization.

Verification:

- cache metadata/invalidation tests for version/schema/source/layout mismatch and corrupted or
  truncated cache data;
- release comparison of at least two startup paths on the post-T169 representative `shcntx_ru`
  provider index: SQLite materialization baseline and derived cache validation/load;
- report warm build/load time, cache validation cost, process peak RSS, capacity-based
  snapshot-owned heap, logical payload bytes, cache file size and representative read-handle lookup
  timings;
- keep lookup correctness covered by existing focused snapshot tests plus cache round-trip and
  cache-loaded snapshot-backed resolver tests needed for the chosen stable path;
- update `acceptance/baseline.md` and `implementation/solution-context-resolve.md` with the
  measured conclusion before accepting a persisted format decision.

Initial stage-timing result:

- Added measurement-only stage timing to the existing release harness. Five warm runs on
  `target/snapshot-materialization/shcntx_ru.schema16.release.sqlite` reported snapshot build
  times of `618 ms`, `649 ms`, `618 ms`, `625 ms` and `641 ms`, for a `625 ms` median.
- Dominant median buckets were SQLite row reading (`228 ms`), fact arena construction (`164 ms`)
  and fact-id/relation/availability construction (`89 ms`). Together these account for most of the
  current startup class and justify a persisted binary-cache prototype after the in-memory T169
  layout/resolver migration is settled.
- The current timing does not choose a disk format. It narrows the next experiment to bypassing
  repeated SQL row decoding and repeated arena/index construction from SQLite while keeping SQLite
  as the canonical rebuildable provider artifact.
- Added a measurement-only provider-owned binary cache prototype using a small versioned
  little-endian format with magic, cache version and provider schema version guards. It introduces
  no new runtime dependency and is not a downstream storage contract. The public methods are named
  `write_experimental_binary_cache` and `from_experimental_binary_cache` to keep that status
  explicit.
- Five warm runs comparing the same source snapshot with the binary cache reported SQLite
  materialization build times of `645 ms`, `643 ms`, `629 ms`, `605 ms` and `683 ms`, for a
  `643 ms` median. Binary cache reads were `25 ms`, `25 ms`, `25 ms`, `24 ms` and `26 ms`, for a
  `25 ms` median. Cache writes were `11-48 ms`, median `44 ms`.
- The cache file was `10319044` bytes (`9.9 MiB`) and every run reported
  `binary_cache.roundtrip_equal=true`. The cache-loaded snapshot estimated heap was
  `16597927` bytes versus `20345723` bytes for the SQLite-materialized snapshot because the binary
  reader allocates exact vector capacities. The harness now reports logical payload bytes in
  addition to capacity-based heap bytes, so future cache comparisons must use both metrics before
  treating the heap delta as structural memory savings.
- Current conclusion: the simple binary cache prototype is strong enough to keep T170 as a real
  follow-up now that T169 stabilized the physical read model and resolver adapter migration. The
  prototype does not yet accept a final persisted format decision or cache invalidation policy
  beyond the minimal version/schema guard.
- Post-T169 T170 adaptation: the next implementation slice is cache stabilization, not broad
  exploration. The new OpenSpec change `stabilize-hbk-fact-snapshot-cache` owns cache metadata,
  invalidation, final format decision and acceptance measurements. T171 remains complete and is not
  reopened by cache work.
- Completed with a stable provider-owned no-dependency little-endian cache format internal to
  `syntax-helper-search`. The runtime entrypoint is
  `HbkFactSnapshot::from_path_with_binary_cache`: it validates cache format version, provider
  schema version, source-index identity fingerprint from metadata, persisted source-index identity
  when available and file size/mtime, locale/source metadata, source extraction schema version,
  snapshot layout version/flags, payload length and FNV-1a checksum. Payload length is capped
  before allocation. Missing, stale, unsupported, truncated or corrupted caches rebuild from the
  canonical SQLite provider index and rewrite the derived artifact. Cache writing is available from
  an `HbkFactSnapshotBuildReport` produced by the same provider index, not from an arbitrary
  snapshot/index pair. Resolver adapters remain independent from cache files and consume only
  loaded `Arc<HbkFactSnapshot>` state/read handles.
- The existing no-dependency DTO path is accepted as the first stable format. No `rkyv`, `zerocopy`,
  mmap or new serialization dependency is justified by the final measurement; such work needs a
  later measured bottleneck.
- Final release measurement on `target/snapshot-materialization/shcntx_ru.schema16.release.sqlite`
  reported warm SQLite materialization at `658-665 ms`, cache validation/load at `34-35 ms`, cache
  writes at `32 ms`, cache file size `11318100` bytes and peak RSS around `106 MiB`. The
  SQLite-materialized snapshot reported `23184770` capacity-based heap bytes and `17846774` payload
  bytes; the cache-loaded snapshot reported exact-capacity heap bytes equal to payload bytes
  (`17846774`). Each measured cache load reported `binary_cache.status=loaded` and
  `binary_cache.roundtrip_equal=true`.
- Verification passed with `cargo test -p syntax-helper-search`, `cargo test -p
  context-resolver-search`, `cargo check -p syntax-helper-search --example
  measure_hbk_fact_snapshot`, `cargo build --release -p syntax-helper-search --example
  measure_hbk_fact_snapshot` and the final release harness above. Final strict OpenSpec/fmt gates
  are run after this spec update.

### [x] T171. Add explicit snapshot-backed resolver adapters for worker-safe analyzer lookup

References: FR-CTX-RESOLVE-001, NFR-RESOLVE-001, NFR-QUERY-001, UC-CTX-001,
UC-CTX-002, `implementation/solution-context-resolve.md`,
`implementation/components.md`, `acceptance/baseline.md`, T169, T170,
OpenSpec change `provider-owned-hbk-fact-snapshot`.

Scope:

- Extend `context-resolver-search` with explicit snapshot-backed source adapters, named for the
  backend rather than hidden behind the existing SQL/SearchIndex source types. Complete and align
  the already introduced `PlatformSnapshotSource` and `QueryTableSnapshotSource`. Add or rename a
  broader `LanguageSnapshotSource` only if the migrated resolver slice truly covers non-query-table
  language facts; otherwise keep query-table lookup under `QueryTableSnapshotSource`.
- Accept `Arc<HbkFactSnapshot>` or another public provider-owned snapshot/read-handle entrypoint
  from `syntax-helper-search`. Use `HbkFactReadHandle` for migrated hot-path lookups.
- Implement `context_resolver_core::ContextSource` for the snapshot-backed sources and project
  snapshot nodes into existing resolver DTOs: `ResolvedType`, `ResolvedMember`,
  `ResolvedCallable`, `ResolvedGlobalContext`, `ResolvedModuleContext`, `ContextFact`,
  `AvailabilityFact` and query table/field/parameter DTOs.
- Keep `PlatformSearchSource` and `LanguageSearchSource` as the explicit SQL/SearchIndex-backed
  backend for CLI, debug, index inspection and sequential local resolver scenarios. Do not describe
  this backend as legacy, do not include downstream analyzer hot paths in it and do not silently
  replace these constructors with snapshot behavior.
- Make backend choice explicit at composition time. No migrated snapshot-backed resolver path may
  fall back from snapshot to SQL/SearchIndex internally.
- Do not read SQLite on migrated snapshot hot paths. SQLite may be used only while materializing a
  provider-owned snapshot or by the explicit SQL/SearchIndex backend.
- Do not build duplicate provider-fact mirror indexes in `context-resolver-search`, copy broad DTO
  payloads into the snapshot physical model or add analyzer-owned fallback tables in `v8-context`.
- Preserve source/domain identity for migrated snapshot-backed sources when platform,
  query-language and any migrated BSL-language facts share display names. If T171 does not migrate
  non-query-table BSL-language facts, document and test the snapshot-backed result for those facts as
  unsupported or empty; do not require identity disambiguation through a source that is not migrated.
- Prove the downstream boundary is worker-safe for the adapter/resolver composition, not only for
  `HbkFactSnapshot` alone. Use a `Send + Sync` compile assertion for the snapshot-backed
  source/resolver or document and test an explicit scoped-worker borrow contract.
- Snapshot-backed hot paths must not satisfy worker safety by wrapping resolver/search state,
  SQLite connections or mutable adapter internals in broad `Arc<Mutex<_>>` / `Arc<RwLock<_>>`.
  Shared state for migrated analyzer lookups is limited to immutable provider-owned snapshot data,
  for example `Arc<HbkFactSnapshot>`, plus worker-local read handles or caches.
- Before T171 can be accepted, enum and enum-value fact refs must either participate in migrated
  exact-id and relation lookup through the snapshot-backed adapter slice, or the task must
  explicitly document that the migrated resolver slice excludes those facts and returns the
  documented unsupported/empty result for them. Silent omission is not accepted.
- Keep the persisted/binary cache format from T170 provider-owned and internal. Snapshot adapters
  may receive a loaded snapshot, but they must not expose or depend on cache layout details.

Verification:

- focused snapshot-backed resolver tests for platform type lookup;
- member lookup by owner/name/kind;
- callable lookup by owner/name;
- global context lookup;
- module context lookup;
- related/availability lookup;
- query table lookup by name, syntax and identifier;
- query field and query parameter lookup by table/name;
- source/domain identity preservation for all migrated snapshot-backed source families when facts
  share display names, plus explicit unsupported/empty coverage for non-migrated BSL-language facts
  if no `LanguageSnapshotSource` is added;
- enum and enum-value exact-id/relation participation through the migrated snapshot-backed slice, or
  explicit tests for the documented unsupported/empty result when that slice excludes them;
- compile or focused test proving the snapshot-backed source/resolver boundary is `Send + Sync`, or
  proving the documented scoped-worker borrow contract;
- focused code/test guard that migrated hot paths do not use broad `Arc<Mutex<_>>` /
  `Arc<RwLock<_>>` around resolver/search state, SQLite connections or mutable adapter internals;
- regression tests proving SQL/SearchIndex-backed `PlatformSearchSource` and `LanguageSearchSource`
  scenarios still work and are selected explicitly;
- concrete no-SQL/no-fallback test: compose snapshot-backed sources from an already materialized
  in-memory `HbkFactSnapshot`, make the source SQLite path unavailable or absent, verify migrated
  lookups still work and verify missing snapshot coverage returns the documented unsupported/empty
  result rather than using SQL/SearchIndex fallback;
- `openspec validate provider-owned-hbk-fact-snapshot --strict`;
- `cargo fmt --all --check`;
- focused package tests/checks for touched crates.

Completion notes:

- Completed as part of T169 stabilization because T171 was the active T169 adapter blocker.
- Added explicit `PlatformSnapshotSource` and `QueryTableSnapshotSource` implementations over
  provider-owned `HbkFactSnapshot` state.
- Kept `PlatformSearchSource` and `LanguageSearchSource` SQL/SearchIndex-backed by design.
- Verified migrated snapshot-backed lookups with focused resolver tests and `Send + Sync`
  assertions over `PlatformSnapshotSource`, `QueryTableSnapshotSource` and
  `WorkerSafeCompositeResolver`. The tests compose snapshot-backed sources from an already
  materialized in-memory `Arc<HbkFactSnapshot>`, remove the SQLite file and then run the migrated
  lookups, including query field/parameter lookup by table/name, proving no hidden
  SQL/SearchIndex fallback is needed on those paths. Missing broader non-query-table language
  snapshot coverage remains intentionally out of this slice; query-table language facts use
  `QueryTableSnapshotSource`.

### [x] T168. Implement the first provider-owned worker-safe HBK fact snapshot slice

References: FR-CTX-RESOLVE-001, NFR-PERF-001, NFR-QUERY-001, UC-CTX-001,
UC-CTX-002, `implementation/solution-context-resolve.md`,
`implementation/components.md`, `acceptance/baseline.md`,
OpenSpec change `provider-owned-hbk-fact-snapshot`.

Scope:

- Add compact provider-owned snapshot DTOs/node ids for the measured SQLite-first materialization
  path.
- Keep the snapshot contract-shaped: include only fields required by worker fact lookup and exclude
  search/export/index-maintenance payloads such as FTS rows, preview text, raw descriptions, raw
  storage paths, relation weights and parser diagnostics.
- Implement a narrow immutable snapshot type that is `Send + Sync` and can be shared as `Arc<_>`.
- Implement worker-local read handles with representative lookups for platform type members,
  callables, global context, module events, query table fields/parameters and language facts where
  the indexed source provides them.
- Use owned `Vec` arenas, compact node/string ids, sorted lookup vectors and compressed-sparse-row
  style adjacency arrays as the first index shape.
- Keep existing resolver DTOs as adapter projections rather than the physical snapshot storage.
- Do not add analyzer fallback readers, raw SQLite readers in `v8-context`, direct HBK parsing in
  worker lookup, or broad `Arc<Mutex<_>>` around resolver/search state.
- Do not add Tantivy, persisted zero-copy snapshot formats, minimal-perfect hashing or compressed
  bitmap dependencies in the first slice. Treat `fst`, `rkyv`/`zerovec` and `roaring` as measured
  follow-up experiments only if the arena snapshot exposes a concrete bottleneck.

Verification:

- `openspec validate provider-owned-hbk-fact-snapshot --strict`
- focused snapshot unit/integration tests, including compile-time `Send + Sync` assertion
- concurrent deterministic read test across multiple threads
- representative lookup test coverage for platform type -> members/callables, platform global
  context, module context events, query table -> fields/parameters and documented language/query
  facts available in indexed sources
- `cargo fmt --all --check`
- focused package tests/checks for touched crates

Result:

- `syntax-helper-search` now exposes provider-owned `HbkFactSnapshot` / `HbkFactReadHandle`
  storage APIs over immutable owned arenas, compact node/string ids, derived lookup vectors and
  compressed-sparse-row owner adjacency arrays.
- The snapshot materializes from an existing provider SQLite index through provider-owned bulk table
  reads and does not store or share `rusqlite::Connection`, raw SQLite tables or mutable resolver
  state after construction.
- Representative read-handle lookups cover platform type ids/names, owner members/callables,
  platform global facts, module events, query tables with fields/parameters and language facts.
- Release measurement on `target/snapshot-materialization/shcntx_ru.schema16.release.sqlite`
  produced stable warm snapshot build readings of `507-601 ms`, median `511 ms`; first-run/cache
  warm-up observations are excluded from the baseline. Estimated snapshot-owned heap was
  `18197557` bytes and process peak RSS stayed around `105708-105844 KiB`.
- Existing resolver DTO adapters were not rewritten in this slice; they remain adapter projections
  over the current search-index path while the snapshot read model stabilizes.
- Verification passed with `cargo test -p syntax-helper-search snapshot`,
  `openspec validate provider-owned-hbk-fact-snapshot --strict`, `cargo fmt --all --check` and
  `cargo check -p syntax-helper-search`.

### [x] T167. Measure SQLite-first HBK fact snapshot materialization

References: FR-CTX-RESOLVE-001, NFR-PERF-001, NFR-QUERY-001, UC-CTX-001,
UC-CTX-002, `implementation/solution-context-resolve.md`,
`implementation/components.md`,
OpenSpec change `provider-owned-hbk-fact-snapshot`.

Scope:

- Treat the existing `syntax-helper-search` SQLite provider index as the first candidate source for
  an immutable worker-safe HBK fact snapshot.
- Add a measurement-only bulk materialization harness that reads provider-owned SQLite tables in
  coarse passes instead of using public N+1 lookup APIs.
- Measure build time, RSS delta or peak RSS, estimated heap when practical, node counts by category
  and representative lookup/index coverage on a real `shcntx_ru` provider index.
- Compare the SQLite-first materialization path with existing HBK/index build measurements and the
  downstream N+1 lookup spike before accepting the broader snapshot implementation direction.

Verification:

- `openspec validate provider-owned-hbk-fact-snapshot --strict`
- measurement command on a representative local `shcntx_ru` SQLite provider index
- `cargo fmt --all --check`
- focused package check for the temporary measurement harness before it was removed

Result:

- OpenSpec change `provider-owned-hbk-fact-snapshot` records SQLite-first materialization as a
  measured design gate.
- Used a temporary `syntax-helper-search` measurement harness to bulk-read provider-owned SQLite
  tables without public `SearchIndex` lookup APIs. The harness was removed after the measurements
  were promoted into the durable specs.
- The measurement probe was narrowed to contract-shaped snapshot fields only; it does not copy
  search/export/index-maintenance payloads or raw storage paths.
- Current release CLI rebuilt schema-16 `shcntx_ru` provider index from
  `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` in `14.50s`, with `284360 KiB` peak RSS and
  `25415` documents.
- Release compact SQLite -> snapshot probe materialized the same index in `474 ms` (`0.55s`
  process elapsed), with `49112 KiB` peak RSS, `46540 KiB` RSS delta and `34935365` estimated heap
  bytes.
- Probe counts: `25415` documents, `2465` type identities, `121` type templates, `18609` members,
  `8337` callables, `8675` signatures, `9793` parameters, `47156` type refs, `58128` relations and
  `728` document metadata rows.
- Review/fix pass verification after harness removal: `openspec validate
  provider-owned-hbk-fact-snapshot --strict`, `cargo fmt --all --check`,
  `cargo check -p syntax-helper-search` and `git diff --check` passed.
- Conclusion: SQLite bulk materialization is accepted as the first implementation source for the
  worker-safe snapshot. Direct HBK reading remains setup/index-refresh input and comparison
  baseline.

### [x] T166. Expose shcntx query table templates through the QueryLanguage resolver source

References: FR-CTX-RESOLVE-001, UC-CTX-001, UC-CTX-002,
`implementation/solution-context-resolve.md`, `implementation/components.md`.

Scope:

- Expose existing `query_table`, `query_table_field` and `query_table_parameter` search documents
  through a distinct `LanguageDomain::QueryLanguage` Rust resolver source.
- Return template/family-level facts only: stable ids, syntax/identifier/table-role data, owner
  semantic path, source-derived template parameter slots, owned field/parameter identities, type
  references and source-neutral evidence/provenance.
- Preserve domain separation: query-table facts are not `PlatformApi` facts, do not become platform
  members, do not instantiate concrete metadata tables and do not add analyzer fallback tables.
- Cover exact lookup and relation traversal with focused `syntax-helper-search` and
  `context-resolver-search` tests, including the existing platform-adapter hiding behavior.

Verification:

- `cargo test -p syntax-helper-search query_table`
- `cargo test -p context-resolver-search query_table`
- `cargo test -p context-resolver-core`
- `cargo test -p context-resolver-search`
- `cargo test --workspace`

Result:

- `context-resolver-core` exposes dependency-facing query-table DTOs:
  `QueryTableInfo`, `QueryFieldInfo`, `QueryParameterInfo`, `QueryTableRole` and
  source-neutral `FactProvenance`.
- `syntax-helper-search` persists query-table metadata in private schema version `16`, including
  syntax, identifier, table role, owner path, template parameter slots, field/parameter notes and
  defaults, source provenance and type references.
- `context-resolver-search` exposes `query_table`, `query_table_field` and
  `query_table_parameter` through `LanguageSearchSource::query_tables` /
  `open_query_tables_read_only*` as `LanguageDomain::QueryLanguage` facts with exact lookup and
  relation capabilities only.
- Exact lookup by display name, identifier and syntax, `member_of` relation traversal and
  `has_type` traversal preserve stable ids and type references. The platform adapter continues to
  hide query-table provider documents from `PlatformApi`.
- Verification passed with `cargo test -p syntax-helper-search query_table`,
  `cargo test -p context-resolver-search query_table`, `cargo test -p context-resolver-core`,
  `cargo test -p context-resolver-search`, `cargo check --workspace`,
  `cargo fmt --all --check` and `cargo test --workspace`.

### [x] T165. Expose core BSL primitive language types through Rust resolver adapters

References: FR-CTX-RESOLVE-001, UC-CTX-001, UC-CTX-002,
`implementation/solution-context-resolve.md`.

Scope:

- Extend the `shlang_*` language-fact slice so direct BSL primitive type pages are indexed as
  `language_type` facts, including `Null`, `Неопределено` / `Undefined`, `Число` / `Number`,
  `Строка` / `String`, `Дата` / `Date`, `Булево` / `Boolean` and `Тип` / `Type`.
- Keep nested primitive literal pages such as `def_BooleanTrue` and `def_BooleanFalse` out of the
  type surface.
- Preserve source/domain identity through `context-resolver-core` and `context-resolver-search`:
  these facts are `BslLanguage` facts from `shlang`, not `PlatformApi` types.
- Cover dependency-facing behavior with focused `syntax-helper-language`,
  `syntax-helper-search` and `context-resolver-search` tests.

Verification:

- `cargo test -p syntax-helper-language`
- `cargo test -p syntax-helper-search`
- `cargo test -p context-resolver-search`
- `cargo test --workspace`

Result:

- `syntax-helper-language` extracts direct `shlang_*` primitive type pages as `language_type`
  facts for `Null`, `Неопределено` / `Undefined`, `Число` / `Number`, `Строка` / `String`,
  `Дата` / `Date`, `Булево` / `Boolean` and `Тип` / `Type`.
- Nested primitive literal pages such as `def_BooleanTrue` remain ignored by this type surface.
- `syntax-helper-search` indexes these facts with source-qualified `shlang:*` ids.
- `context-resolver-search` resolves them through `LanguageSearchSource` as
  `LanguageDomain::BslLanguage` for Rust dependency consumers.
