# Completed Implementation Tasks T25-T34

This archive preserves completed task history moved out of the active implementation ledger.
It is evidence, not active implementation scope.

T35 and T18 remain active and are not archived here.

Raw command logs, generated exports, temporary probes and `target/...` paths are service data and
are intentionally omitted from this archive. Durable counts, measurements and conclusions live in
`../acceptance/baseline.md`, `../source-evidence.md`, `../requirements/functional.md` and
`../implementation/components.md`.

For active task sequencing, use `../IMPLEMENTATION_TODO.md`.

## T25. Fix locale-aware Syntax Assistant section parsing and type references

Status: completed.

Outcome:

- Implemented root/English `Type:` and `Returned value:` parsing with locale-aware section
  boundaries for availability, examples, see-also, available-since and overload variant labels.
- Preserved consumer records without HBK provenance, TOC paths, HTML paths and page titles.
- Verified RU/root exports, deterministic repeat export behavior and type-reference preservation
  for representative real Syntax Assistant pages.

## T26. Extract structured availability, examples, see-also and version facts

Status: completed.

Outcome:

- Added schema version 2 structured section fields: `availability`, `examples`, `see_also` and
  `available_since`.
- Normalized availability contexts and kept see-also consumer targets provenance-free.
- Kept record-family counts stable while moving facts out of flattened descriptions.

## T27. Parse overload and syntax-variant pages structurally

Status: completed.

Outcome:

- Added schema version 3 `signatures[].variant` metadata for Syntax Assistant syntax-variant pages.
- Parsed Russian and root/English overload pages so parameters and return types attach to the
  owning variant.
- Preserved stable record counts and removed raw overload/returned-value labels from signature text.

## T28. Classify remaining Syntax Assistant diagnostics and extraction completeness

Status: completed.

Outcome:

- Replaced generic `UNKNOWN_PAGE_CLASS` diagnostics with stable family-specific diagnostics for the
  audited source families.
- Classified unsupported direct global-context method pages, global-context event pages, table
  fields and table parameters before T29 promoted the latter three into typed exports.
- Preserved deterministic diagnostics and parser provenance.

## T29. Support Syntax Assistant global events and query/table metadata records

Status: completed.

Outcome:

- Added schema version 4 consumer record families: `global-context-events.json`,
  `table-fields.json` and `table-parameters.json`.
- Full RU/root exports produced 33 global context events, 588 query/table fields and 78 query/table
  parameters in each locale.
- Reduced remaining diagnostics to 4 `UNSUPPORTED_GLOBAL_CONTEXT_METHOD_PAGE` records per locale.

## T30. Remove post-T29 Syntax Assistant table-owner lookup regression

Status: completed.

Outcome:

- Replaced per-record `Toc::find_by_html_path` table-owner resolution with one extraction-scope TOC
  HTML-path index.
- Preserved locale-aware query/table owner names and byte-identical consumer JSON output compared
  with the pre-change T32 exports.
- Restored release-profile runtime to the T28/T30 class while keeping record counts and diagnostic
  counts stable.

## T31. Re-measure residual Syntax Assistant parser overhead after T30

Status: completed.

Outcome:

- Rebuilt the release binary and remeasured the post-T30 path before changing parser code.
- Concluded that residual parser/export overhead did not justify changes to section extraction,
  variant probing or rubric-parameter parsing in this task.
- Recorded current release-profile measurements and deterministic repeat-export evidence in the
  acceptance baseline.

## T32. Switch consumer JSON export to lean schema version 5

Status: completed.

Outcome:

- Raised the canonical consumer export schema to `schema_version: 5`.
- Omitted `null` fields and empty arrays in platform API records, converted owner/type-reference
  fields to strings, moved version facts to `availability.since` and nested enum values in
  `enums.json`.
- Removed `enum-values.json` from the consumer export inventory.

## T33. Fix consumer JSON field names and Syntax Assistant extracted data quality

Status: completed.

Outcome:

- Raised the canonical consumer export schema to `schema_version: 6`.
- Renamed consumer type-reference fields to `types` and callable return fields to `return` while
  keeping the Rust domain model names unchanged.
- Fixed inline examples, code-example punctuation around syntax-coloring markup and composed
  owner/member see-also links.

## T34. Fix multiline example punctuation normalization after string continuations

Status: completed.

Outcome:

- Preserved example-string state across multiline normalization so a continuation line that closes a
  string no longer keeps HTML-coloring spaces around later call punctuation.
- Added regression coverage for the real `ЗадачаОбъект.<Имя задачи>.Записать` HBK example.
- Verified the targeted parser tests, workspace tests, fresh release export and reported `jq`
  check.
