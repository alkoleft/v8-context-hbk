# ADR-0005: Use TOC-Aware Reading for Syntax Assistant Facts

Date: 2026-05-01.

Status: Accepted.

Decision maker: project maintainer.

## Context

`v8-context-hbk` extracts structured platform facts from Syntax Assistant HBK books. The current
consumer export is intentionally lean: per-record raw provenance such as HBK path, TOC path, HTML
path and page title stays out of consumer record-family files, while diagnostics remain
provenance-rich.

A 2026-05-01 review of the Russian `schema_version: 6` export under `/tmp/shcntx/` found that this
lean export exposes a deeper reading issue:

- `table-parameters.json` has three indistinguishable records for
  `owner="Таблица остатков и оборотов"` and `name.primary="Метод дополнения периодов"`;
- `global-context-events.json` has repeated event names and aliases such as
  `ПриНачалеРаботыСистемы` / `OnStart`, `ПриЗавершенииРаботыСистемы` / `OnExit`,
  `ОбработкаВнешнегоСобытия` / `ExternEventProcessing` and
  `ПередНачаломРаботыСистемы` / `BeforeStart` with different descriptions and availability facts;
- `platform-types.json` has same-name entries such as `ПередЗаписью` / `BeforeWrite`,
  `ПослеЗаписи` / `AfterWrite`, `ПриЧтенииНаСервере` / `OnReadAtServer` and placeholder-like
  event pages where the semantic branch is not visible in the fact identity;
- placeholder-like records such as query table fields, form element properties and external data
  source constructors are only meaningful when the reader carries their owner path or branch
  context.

The immediate temptation is to put raw `toc_path`, `html_path` or page title back into consumer
records. That would make collisions inspectable, but it would not fix the reader. It would also
contradict the current FR-EXPORT-001 direction that consumer files expose platform API facts rather
than parser traces.

The extraction code already has partial TOC data:

- `hbk-book` exposes a hierarchical TOC;
- `syntax-helper-extract` classifies catalog pages and attaches `SyntaxHelperSource`;
- query table owners are currently derived from a flat TOC HTML-path map and a stripped member path;
- consumer adapters serialize only lean fact fields.

The missing architectural rule is that Syntax Assistant reading must derive semantic context from
the TOC hierarchy before emitting typed facts.

## Decision

Use the Syntax Assistant TOC hierarchy as the authoritative structural context for reading
Syntax Assistant facts.

HTML paths, page headings and localized titles remain useful evidence, but they are not the
classification or ownership contract when the TOC branch carries a stronger relationship.

Before a page becomes a typed domain record, `syntax-helper-extract` must derive a semantic reading
context from the TOC ancestor chain. The context owns:

- root section kind;
- source family;
- semantic owner or owner path for owned facts;
- branch labels needed to distinguish same-title pages;
- parser provenance required for diagnostics.

Raw provenance fields remain parser-maintenance data. They must not be reintroduced into lean
consumer records as the fix for reading ambiguity.

If downstream exact lookup needs a visible discriminator, that discriminator must be a semantic
platform fact derived from reading context, not raw `toc_path`, `html_path`, page title or HBK path.

Name-only merging is not allowed for ambiguous Syntax Assistant pages. A merge is valid only when a
source-family-specific rule is explicit, deterministic and tested.

## Classification Model

Syntax Assistant reading uses two layers:

- TOC branch classification: what part of the book the page belongs to.
- Record/domain classification: what platform fact the page represents.

A branch can influence record classification without becoming a record family by itself. For
example, Automation / external API branches contain ordinary platform types and members, not a
separate Automation record family.

### TOC Branch Kinds

The initial branch kinds are:

- `global_context`: global context root, global methods/properties and module-event groups exposed
  under the global context section.
- `system_enums`: system enum catalog.
- `primitive_types`: primitive type catalog.
- `metadata_objects`: metadata/application-object families such as documents, catalogs, tasks,
  business processes, registers, charts, external data sources and their generated object/reference
  manager types.
- `managed_forms`: form, form element, form extension and client application form families.
- `query_tables`: query-language/SDBL table metadata.
- `platform_objects`: ordinary platform object/type branches, including common objects,
  collections, XML/HTML/JSON, data composition and similar categories.
- `automation_external_api`: Automation/external API branch. This is a branch category only; its
  records use the ordinary platform type/member families.

The branch list is expected to grow from source evidence. New branches should be added as explicit
rules, not as broad suffix heuristics.

### Record Families

The initial record families are:

- `global_method`
- `global_property`
- `module_event`
- `platform_type`
- `type_method`
- `type_property`
- `type_constructor`
- `system_enum`
- `system_enum_value`
- `query_table`
- `query_table_field`
- `query_table_parameter`

Events that appear under "События модуля сеанса", "События обычного приложения", application
module groups, web/HTTP service modules or metadata object modules are `module_event` records.
They are not global context members just because some groups are located under the global context
TOC root.

### Platform Type Kinds

`platform_type` records carry a type kind:

- `regular`: ordinary platform type.
- `extension`: mixin/extension type. If the base type or base role can be proven from TOC/HTML/link
  evidence, the type also records an `extends` relationship. If the base cannot be proven, the type
  remains `extension` without a synthesized base.
- `primitive`: direct child of the `primitive_types` branch.
- `metadata_template`: type parameterized by metadata/application configuration, such as
  `ДокументОбъект.<Имя документа>`, `СправочникСсылка.<Имя справочника>` or
  `ВнешнийИсточникДанныхТаблицаОбъект.<Имя внешнего источника>.<Имя таблицы внешнего источника данных>`.

`metadata_template` records may expose a `metadata_kind` and `template_parameters` when they can be
derived from the source name or TOC context.

### Primitive Type Rule

The `primitive_types` branch is shallow:

- direct children such as `Null`, `Неопределено`, `Число`, `Строка`, `Дата`, `Булево` and `Тип`
  are `platform_type(type_kind=primitive)` records;
- nested children below a primitive type, such as `Булево > Истина` and `Булево > Ложь`, are not
  `platform_type` records and must not be reached by ordinary object-catalog recursion;
- primitive literals/values may become a separate record family only after a later requirement
  defines their contract.

### Module Event Rule

Module-event classification is based on the ancestor chain, not on the global-context root alone.
The reader must derive at least:

- `module_kind`, such as session, ordinary application, managed application, external connection,
  object module, manager module, form module, web service module or HTTP service module;
- `module_owner` or semantic owner path when the module belongs to a metadata object, service,
  form or other platform type;
- the event name, signatures, section facts and parser provenance.

### Extension Rule

Extension types are detected from TOC/HTML/link evidence such as:

- localized title starts with `Расширение ...` or English `Extension ...`;
- English path/title phrases such as `extension for ...`;
- placement under form/control/object extension branches.

The reader may derive `extends` from title patterns, parent branch context or reliable links. It
must not guess a base type from a loose word match.

## Rationale

The same local page shape can mean different things under different Syntax Assistant branches.
Examples include `/fields/`, `/params/`, `/events/`, `/methods/`, `/properties/` and `/ctors/`
segments. Reading them by suffix-only path checks or page title collapses source facts that the
book hierarchy keeps distinct.

TOC-aware reading keeps the project boundaries clean:

- `syntax-helper-extract` owns source interpretation;
- `syntax-helper-model` owns typed domain facts and semantic identity;
- `hbk-export` owns consumer shape simplification;
- diagnostics keep raw source context for parser maintenance.

This preserves the lean export decision while giving later query/index work a correct fact model.

## Alternatives Considered

### Add Raw Provenance Fields to Consumer Records

Rejected.

Raw `toc_path`, `html_path`, page title and HBK path explain where a record came from, but they do
not define what the platform fact is. Adding them to consumer JSON would leak parser traces into the
consumer contract and still leave the reader with name/title-only semantics.

### Keep Current Reading and Let the Query Index Disambiguate

Rejected.

The query CLI should index extracted platform facts. It should not repair ambiguous facts that were
already collapsed or mis-owned by the reader. Doing this in the index would duplicate source-reading
rules outside the extraction boundary.

### Deduplicate or Merge Same-Name Records by Heuristics

Rejected.

Same names in the Syntax Assistant are not proof of duplicates. They may represent event variants,
metadata-specific query table entries, form extensions or placeholder pages under different
semantic owners. Merge rules must be explicit per source family.

### Keep Only the First Record for Exact Lookup

Rejected.

Dropping later same-name records silently loses documented platform facts and makes the exported
model depend on TOC traversal order.

## Consequences

- T35 becomes the next task before the query CLI slice.
- `FR-SH-003` owns the reading requirement.
- `syntax-helper-extract` must stop treating path suffixes and page titles as sufficient ownership
  for ambiguous source families.
- The domain model may need semantic context or owner-path fields that are not raw parser
  provenance.
- Consumer export may need a schema change only if a semantic discriminator must be visible for
  exact lookup. Such a field must be derived from the reading context and documented in
  FR-EXPORT-001 before implementation.
- ADR-0004 query CLI work must wait for T35 unless explicitly reprioritized, because indexing
  ambiguous facts would bake the reading defect into the query artifact.

## Non-Goals

- Do not restore `source_hbk`, `toc_path`, `html_path` or `page_title` to lean consumer records as
  the T35 fix.
- Do not implement the query CLI in this decision.
- Do not add runtime 1C introspection.
- Do not introduce generic parser pipelines, broad caches or plugin systems.
- Do not stabilize the consumer export contract beyond the T35 changes explicitly required by
  FR-EXPORT-001.

## Implementation Plan

Workflow note (2026-08-03): this completed plan's references to the former
task ledger are historical. ADR-0013 makes OpenSpec the current owner of change
scope and task state.

Affected paths:

- `crates/syntax-helper-extract/src/catalog.rs`
- `crates/syntax-helper-extract/src/reader.rs`
- `crates/syntax-helper-extract/src/page_parser.rs`
- `crates/syntax-helper-extract/src/tests.rs`
- `crates/syntax-helper-model/src/lib.rs`
- `crates/hbk-export/src/consumer.rs`
- `crates/hbk-export/src/tests.rs`
- `spec/requirements/functional.md`
- `spec/acceptance/uat-test-cases.md`
- `spec/acceptance/baseline.md`
- `spec/source-evidence.md`
- `spec/IMPLEMENTATION_TODO.md`

Implementation constraints:

- Do not add new dependencies for T35 unless a concrete parser limitation is demonstrated and the
  task updates this ADR first.
- Keep the existing `SyntaxHelperSink` boundary typed by record families.
- Keep diagnostics provenance-rich and consumer records lean.
- Keep generated export directories and measurement logs as service data unless conclusions are
  promoted into `spec/`.

1. In `syntax-helper-extract`, introduce a small semantic reading context built from TOC ancestor
   data during catalog traversal.
2. Use that context when classifying pages instead of relying only on `classify_catalog_page` path
   suffix checks.
3. Replace query table field/parameter ownership based on stripped HTML paths with TOC-derived
   query table context, including nested `Работа с запросами.Таблицы запросов` branches.
4. Replace the current global-context-event concept with `module_event` classification for module
   event groups under global context, metadata object modules, form modules and service modules.
5. Add `type_kind` to platform types with at least `regular`, `extension`, `primitive` and
   `metadata_template`.
6. Preserve the shallow primitive type rule: direct primitive children become primitive types;
   nested primitive literals do not become platform types.
7. Preserve semantic context for same-name platform type/object pages such as event-like
   `ПередЗаписью` / `BeforeWrite` entries.
8. Keep placeholder-like records distinguishable by semantic owner/context.
9. Add or update model fields only for semantic facts. Do not expose raw provenance as domain
   identity.
10. Update `hbk-export` only after the reading model defines the semantic discriminator that must be
   visible to consumers.
11. Add fixture tests from real Syntax Assistant TOC/page structures for the reported ambiguous
   families.
12. Replace the provisional UAT-SH-013 wording with deterministic `jq` checks once the accepted
    semantic identity fields are known.
13. Update `spec/source-evidence.md`, `spec/acceptance/baseline.md` and
    `spec/IMPLEMENTATION_TODO.md` with resolved behavior and any remaining ambiguity diagnostics.

## Verification

- [ ] `cargo fmt` passes.
- [ ] `cargo test --workspace` passes.
- [ ] UAT-SH-011 still passes for event and query/table metadata export.
- [ ] UAT-SH-013 has deterministic checks for the accepted semantic identity fields.
- [ ] Full `syntax-helper` export for `shcntx_ru.hbk` succeeds.
- [ ] Targeted checks cover `Метод дополнения периодов`, repeated global context event names,
      `ПередЗаписью` / `BeforeWrite` and placeholder-like records.
- [ ] Targeted checks cover `primitive_types > Булево > Истина/Ложь` not being exported as
      platform types.
- [ ] Targeted checks cover at least one `extension` type and one `metadata_template` type.
- [ ] Consumer records still omit raw HBK provenance unless FR-EXPORT-001 is explicitly changed for
      a semantic discriminator.
- [ ] Remaining ambiguous pages, if any, are reported as recoverable diagnostics with provenance.
