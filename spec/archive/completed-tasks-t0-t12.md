# Completed Implementation Tasks T0-T12

This archive preserves completed task history. It is evidence, not active implementation scope.

For active task sequencing, use `../IMPLEMENTATION_TODO.md`.

## T0. Baseline Repository Shape

Status: completed by planning baseline commit `fc2b3d1`.

Outcome:

- Initial README/spec baseline was established.
- Minimal `cargo test` baseline was kept passing.

## T1. Container Reader and Inspect Command

Status: completed.

Outcome:

- HBK container parsing, typed container errors, entity enumeration and byte reads were implemented.
- `inspect` CLI was added.
- Unit fixtures and optional real-file smoke checks covered the first small HBK pair.

## T2. Book, ZIP Storage and TOC Reader

Status: completed.

Outcome:

- `HbkBook`, ZIP-backed `FileStorage`, book metadata, locale inference, TOC traversal and page reads
  were implemented.
- `toc` and `page` CLI commands were added.
- Known-page fixtures made page smoke verification reproducible.

## T3. Documentation Page Parser

Status: completed.

Outcome:

- HTML loading/parsing, page title extraction, normalized text preview and deterministic link
  normalization were implemented.
- Fixture tests covered representative pages and unresolved-link diagnostics.

## T4. Syntax Assistant Fixture Corpus

Status: completed.

Outcome:

- Curated `tests/fixtures/syntax-helper/manifest.tsv`.
- Minimal real HTML fragments from `shcntx_ru.hbk` and `shcntx_root.hbk` cover root/catalog and
  specialized parser kinds.

## T5. Syntax Assistant Root Discovery

Status: completed.

Outcome:

- Syntax Assistant root discovery, catalog traversal, unknown-page diagnostics and fixture/real
  platform assertions were implemented.

## T6. Specialized Syntax Assistant Parsers

Status: completed.

Outcome:

- Specialized parsers for object/type, method, property, constructor, enum, enum value and global
  context pages were implemented.
- Full in-memory extraction against `shcntx_ru.hbk` returned the required non-empty families.

## T7. Domain Model and Canonical JSON Export

Status: completed.

Outcome:

- Provisional domain structs, source provenance serialization, `syntax-helper --output`, `_root`
  export-locale mapping and canonical JSON record-family files were implemented.

## T8. Lookup Helpers

Status: completed.

Outcome:

- Exact lookup helpers for global members, platform types, type members and constructors were
  implemented with found, missing and ambiguous coverage.

## T9. Real-Platform Syntax Assistant Acceptance

Status: completed.

Outcome:

- `shcntx_ru.hbk` and `shcntx_root.hbk` extraction succeeded.
- Durable counts and parser-gap conclusions live in `../acceptance/baseline.md`.
- Intermediate checked-in report files were removed from durable docs.

## T10. All-HBK Smoke

Status: completed.

Outcome:

- All 116 target-platform HBK files passed `inspect` and `toc --format json` smoke checks.
- Durable conclusions live in `../acceptance/baseline.md`.
- Intermediate checked-in report files were removed from durable docs.

## T11. Integration Decision for v8-context

Status: completed.

Outcome:

- ADR-0001 decides that `v8-context-hbk` remains standalone for now and exposes the first
  integration surface through file-level `syntax-helper --output` export.

## T12. Split Implementation into Reusable Workspace Crates

Status: completed.

Outcome:

- Repository was split into the current Cargo workspace crates:
  - `hbk-container`
  - `hbk-book`
  - `hbk-docs`
  - `syntax-helper-model`
  - `syntax-helper-extract`
  - `hbk-export`
  - `v8-context-hbk-cli`
- The `v8-context-hbk` binary and accepted command behavior were preserved.
