# ADR-0009: Separate Book and Syntax Assistant Export Crates

Date: 2026-05-05.

Status: Accepted as a specification and implementation direction.

## Context

The repository already has an `hbk-export` crate that writes the canonical Syntax Assistant JSON
consumer export from the provenance-rich Syntax Assistant domain model. That name was sufficient
while the only export command was `syntax export`.

The project now needs an ordinary HBK book-content export command with two explicit axes:

- `format=raw` or `format=markdown`;
- `hierarchy=raw` or `hierarchy=toc`.

This command exports ordinary book content. It must not invoke Syntax Assistant extraction and must
not reuse the Syntax Assistant JSON export schema. Keeping the existing `hbk-export` name while also
adding a book-content export crate would make the boundary ambiguous for future agents and
implementations.

## Decision

Rename the existing Syntax Assistant JSON export crate from `hbk-export` to `hbk-syntax-export`.

Add a separate ordinary book-content export crate named `hbk-book-export` when implementing
FR-HBK-004.

The two export crates have different responsibilities:

- `hbk-syntax-export` owns canonical Syntax Assistant JSON export adapters over the Rust
  Syntax Assistant domain model and `SyntaxHelperSink` boundary.
- `hbk-book-export` owns ordinary HBK book-content export layout, safe output-root validation,
  raw `FileStorage` unpacking and Markdown conversion for TOC pages.

The CLI remains one binary:

- `v8-context-hbk syntax export <shcntx.hbk> --output <dir>` continues to use
  `hbk-syntax-export`;
- `v8-context-hbk export <book.hbk> --output <dir> --format <raw|markdown>
  --hierarchy <raw|toc>` uses `hbk-book-export`.

## Boundary Contract

`hbk-syntax-export` must not gain ordinary book-content Markdown or raw-unpack responsibilities.
It may depend on Syntax Assistant model crates and serialization crates, but it must not parse HBK
containers or own CLI presentation.

`hbk-book-export` may depend on `hbk-book` and `hbk-docs`. It must not depend on
Syntax Assistant extraction, `hbk-syntax-export`, search/index crates or resolver crates.

Both crates return typed errors at their serialization/export boundary. CLI code maps those errors
to readable diagnostics.

## Consequences

- The crate rename is a deliberate breaking internal workspace change. Public Rust crate stability
  has not been accepted for this repository.
- Existing historical ADRs that mention `hbk-export` remain valid as historical context. New spec,
  tasks and implementation should use `hbk-syntax-export` for the Syntax Assistant export crate.
- `hbk-book-export` can be added without overloading `hbk-syntax-export` with unrelated book export
  concerns.
- The CLI command names stay clear: `syntax export` is Syntax Assistant fact export, while top-level
  `export` is ordinary book-content export.

## Alternatives Considered

### Keep `hbk-export` for Syntax Assistant JSON and Add `hbk-book-export`

Rejected.

The two crate names would be too easy to confuse. `hbk-export` would sound like the generic HBK book
export crate even though it owns only Syntax Assistant JSON adapters.

### Reuse `hbk-export` for Both Syntax Assistant JSON and Book Markdown

Rejected.

This violates the repository boundary rules. Syntax Assistant consumer JSON and ordinary book
content conversion have different inputs, data models, output layouts and UAT expectations.

### Put Book Export Only in the CLI

Rejected.

The first export slice includes safe output path planning, raw unpacking and Markdown conversion.
Keeping that logic inside CLI command handlers would mix presentation, validation and export
adapter responsibilities.

## Implementation Plan

1. Rename `crates/hbk-export` to `crates/hbk-syntax-export`.
2. Update workspace membership and dependency aliases in `Cargo.toml` files.
3. Update Rust imports from `hbk_export` to `hbk_syntax_export`.
4. Keep `v8-context-hbk syntax export` behavior and JSON schema unchanged.
5. Add `hbk-book-export` as the ordinary book-content export crate for FR-HBK-004.
6. Wire the top-level `export` command through `hbk-book-export`, not through
   `hbk-syntax-export`.
7. Update README only for user-facing command documentation after behavior exists.

## Verification

- [x] `cargo test -p hbk-syntax-export` passes after the rename.
- [x] `cargo test -p v8-context-hbk-cli` passes and `syntax export` behavior remains unchanged.
- [x] New book-content export tests use `hbk-book-export`.
- [ ] UAT-HBK-004 through UAT-HBK-007 verify Markdown export through the top-level `export`
      command.
- [x] No new ordinary book-content export code is added to `hbk-syntax-export`.
