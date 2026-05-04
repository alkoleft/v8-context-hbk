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

### BSL Developer or Code-Analysis Tool

As a developer, coding agent or future BSL analyzer, I need a fast local provider of platform API
facts that can answer code-facing questions about available members, constructor/method signatures,
parameter names and types, return types and related platform objects.

### Syntax Assistant CLI User

As a developer or agent, I need to quickly find platform API facts by exact name, purpose,
keywords, approximate spelling and relationships without re-extracting HBK books for every query.

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

Related requirements: FR-SH-001, FR-SH-002, FR-SH-003, FR-EXPORT-001, NFR-DIAG-001.

## UC-SH-002: Diagnose Parser Gaps

Primary user: parser maintainer.

Outcome: a maintainer can identify unknown page classes, unresolved links or unsupported HTML blocks
with enough HBK/page provenance to create follow-up parser tasks.

Related requirements: FR-DOC-001, FR-SH-001, FR-SH-002, FR-SH-003, NFR-DIAG-001.

## UC-SH-003: Find Syntax Assistant API Facts Quickly

Primary user: Syntax Assistant CLI user.

Outcome: a user can query an already extracted or indexed Syntax Assistant data set and get a small,
ranked list of platform API facts without opening the HBK source book again.

Related requirements: FR-SH-SEARCH-001, NFR-QUERY-001.

## UC-SH-004: Explore Syntax Assistant Relationships

Primary user: Syntax Assistant CLI user.

Outcome: a user can start from a type, property, method, constructor or natural-language task and
see related API facts needed to understand how the platform feature is assembled.

Example: from "отбор скд" or `ОтборКомпоновкиДанных`, the CLI can show the settings property,
filter item collection, item creation method and comparison item fields that are present in the
extracted Syntax Assistant data.

Related requirements: FR-SH-SEARCH-001, FR-SH-SEARCH-002, NFR-QUERY-001.

## UC-SH-005: Support BSL Development and Code Analysis

Primary user: BSL developer or code-analysis tool.

Outcome: a user or analyzer can resolve platform API facts needed to write, review or analyze BSL
code: callable signatures, constructor signatures, parameter names and types, return/type
references, owner/member relationships and nearby APIs.

Examples:

- Given `Новый HTTPСоединение(...)`, retrieve valid constructor overloads and structured parameter
  details without ambiguous name/type interleaving.
- Given `НастройкиКомпоновкиДанных.Отбор`, retrieve the property, its type reference and related
  collection/item APIs needed to construct or inspect filter code.
- Given an approximate or task-oriented query such as `отбор скд`, find the relevant platform facts
  and relationship chain before broader documentation prose.

Related requirements: FR-SH-SEARCH-001, FR-SH-SEARCH-002, FR-EXPORT-001, NFR-QUERY-001.

### UC-SH-005A: Resolve Constructor Call

Outcome: given a BSL constructor expression such as `Новый HTTPСоединение(...)`, a developer or
analyzer can retrieve all documented overloads with structured parameter names, requiredness,
parameter type references and descriptions when available.

Solution shape: `syntax constructors <TYPE>` remains the human shortcut, while JSON output must
expose typed callable facts instead of interleaving parameter names and type names in one array. For
shared facts, the JSON shape should be compatible with `syntax export`, for example parameter
objects with `name`, `required`, `types` and optional `description`.

### UC-SH-005B: Resolve Owner/Member Access

Outcome: given a BSL member access such as `НастройкиКомпоновкиДанных.Отбор`, a developer or
analyzer can retrieve the exact owned fact, its owner identity, type references or return types and
related API facts.

Solution shape: `syntax get --owner <TYPE> --member <MEMBER>` is the first query path. Follow-up
relationship queries should support unambiguous roots such as document id or owner/member input
when a plain name can match multiple facts.

### UC-SH-005C: Find APIs for a BSL Task

Outcome: given a task-oriented query such as `отбор скд`, `HTTP соединение` or
`таблица регистра бухгалтерии`, a developer or coding agent can find relevant API facts and follow a
relationship chain to the constructors, properties and methods needed to write or review code.

Solution shape: keyword/fuzzy search finds candidate facts; relationship traversal explains nearby
facts through deterministic owner/type/return edges. New ranking or graph work must be justified by
failed source-backed BSL task scenarios.

### UC-SH-005D: Analyzer-Safe Batch Lookup

Outcome: a future BSL analyzer can resolve exact names, owner/member pairs or stable provider ids
against a prebuilt local platform index and receive versioned, deterministic, tool-readable JSON.

Solution shape: define a provider response contract with explicit ambiguity and missing-result
behavior, using `syntax export` field shapes for shared platform facts. Keep BSL parsing and
diagnostics outside this repository.

## UC-INT-001: Consume HBK Data from v8-context Later

Primary user: future `v8-context` maintainer.

Outcome: `v8-context` can consume a derived platform model without parsing HBK containers or Syntax
Assistant HTML directly in query paths.

Related requirements: FR-EXPORT-001 and ADR-0001.
