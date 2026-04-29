# Use Cases

## Users

### Library Consumer

As a Rust tool author, I need to open an HBK file, inspect its entities, read files inside
`FileStorage`, and parse TOC/metadata without knowing the binary format.

### Documentation Consumer

As a documentation tool, I need to traverse the book TOC, resolve a page by path, read page HTML and
follow links consistently.

### Platform-Context Consumer

As an AI/indexing tool, I need structured platform API data from Syntax Assistant: methods,
properties, types, constructors, enums, signatures, parameters and return types.

### Parser Maintainer

As a maintainer, I need deterministic parser tests with small fixtures and clear failure context when
platform HTML changes.

## UC-HBK-001: Inspect a Help Book

Primary user: library consumer.

Outcome: a user can run `inspect` on an installed HBK file and see the core entities without knowing
the binary format.

Related requirements: FR-HBK-001, FR-CLI-001, NFR-DIAG-001.

## UC-HBK-002: Navigate a Help Book

Primary user: documentation consumer.

Outcome: a user can print a TOC, identify a page path and read the raw page content from the book
storage.

Related requirements: FR-HBK-002, FR-HBK-003, FR-DOC-001, FR-CLI-001.

## UC-SH-001: Export Syntax Assistant Platform Data

Primary user: platform-context consumer.

Outcome: a user can export `shcntx_ru.hbk` or `shcntx_root.hbk` into canonical JSON record-family
files for downstream experiments.

Related requirements: FR-SH-001, FR-SH-002, FR-EXPORT-001, NFR-DIAG-001.

## UC-SH-002: Diagnose Parser Gaps

Primary user: parser maintainer.

Outcome: a maintainer can identify unknown page classes, unresolved links or unsupported HTML blocks
with enough HBK/page provenance to create follow-up parser tasks.

Related requirements: FR-DOC-001, FR-SH-001, FR-SH-002, NFR-DIAG-001.

## UC-INT-001: Consume HBK Data from v8-context Later

Primary user: future `v8-context` maintainer.

Outcome: `v8-context` can consume a derived platform model without parsing HBK containers or Syntax
Assistant HTML directly in query paths.

Related requirements: FR-EXPORT-001 and ADR-0001.
