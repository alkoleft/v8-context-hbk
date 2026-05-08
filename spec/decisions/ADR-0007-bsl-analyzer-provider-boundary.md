# ADR-0007: Keep the BSL Analyzer Provider Boundary as CLI JSON First

Date: 2026-05-05.

Status: Accepted. Extended by ADR-0008 for the separate Rust solution-context resolver boundary.

## Context

ADR-0001 keeps `v8-context-hbk` independently testable and makes file-level `syntax export`
the first integration boundary for future `v8-context` ingestion. ADR-0004 adds the local
`syntax` query command group on a prebuilt SQLite/FTS5 index. ADR-0006 orients the `syntax` scope
toward BSL development and code-analysis assistance while keeping BSL parsing, linting and analyzer
diagnostics outside this repository.

T52 implemented the provider JSON envelope for `syntax get`, `syntax constructors`,
`syntax search` and `syntax related`. T53 and T54 validated that envelope against source-backed BSL
task scenarios. The next open question is whether a downstream BSL analyzer should consume this
provider through CLI JSON only, a Rust library API, a derived file artifact contract, or a
combination of those boundaries.

The current analyzer consumer is still future-facing. No concrete downstream analyzer process,
library dependency graph, batching contract, release cadence or workspace integration rule has been
validated yet. T56 later normalized analyzer-relevant SQLite facts, but did not change this
provider boundary.

ADR-0008 later adds a separate in-process Rust resolver boundary for a concrete full-context Rust
application. That boundary does not replace this CLI JSON provider decision and does not make
SQLite tables public.

## Decision

Use local CLI JSON over a prebuilt `syntax` index as the first analyzer-facing provider boundary.

The selected boundary is:

- `syntax index <shcntx.hbk> --output <index.sqlite>` builds the local derived provider artifact.
- `syntax get`, `syntax constructors`, `syntax search` and `syntax related` read that prebuilt
  artifact and return the versioned provider JSON envelope accepted by FR-SH-PROVIDER-001.
- Analyzer-oriented callers may use exact document ids, owner/member roots and deterministic JSON
  diagnostics for missing, ambiguous or unsupported requests.
- Shared platform facts in provider JSON continue to use `syntax export` field names and shapes
  where both surfaces expose the same fact.

Do not add a Rust library API, analyzer-specific file artifact contract, daemon/MCP service,
network search service or storage-selection knob as part of the current provider boundary.

This is a provisional integration boundary, not a stabilized public protocol. It is the narrowest
boundary that is already implemented, locally reproducible, language-agnostic and compatible with
the accepted standalone repository rule. A future ADR may add another boundary only after a concrete
consumer proves the need.

## Boundary Contract

The CLI JSON provider boundary promises:

- local and deterministic operation against one resolved prebuilt index;
- no query-time HBK parsing;
- versioned provider response envelopes;
- explicit ambiguity, missing-result and unsupported-query diagnostics;
- export-compatible fact shapes for names, owners, signatures, parameters, `types` and `return`;
- no HBK file paths, TOC paths, HTML paths or page titles in consumer facts;
- no FTS/search-token fields as public platform facts.

The boundary does not promise:

- stable Rust structs for downstream analyzers;
- old-index compatibility or migration between provisional index schemas;
- bulk analyzer APIs beyond the current command-level lookups;
- long-running service behavior;
- BSL parsing, expression parsing, diagnostics or code actions inside this repository.

## Consequences

- T56 normalized the SQLite schema for analyzer facts, but provider JSON remains the external
  boundary unless a later ADR changes it.
- The `syntax-helper-search` crate can expose Rust APIs needed by the local CLI and tests, but those
  APIs are not a public analyzer integration contract yet.
- ADR-0008 now owns the dependency-based Rust static-analysis surface for consumers that can link
  this workspace as libraries. Such consumers should use resolver/source traits in process instead
  of spawning CLI JSON or adding HTTP/MCP transport for hot-path lookup.
- Analyzer callers that need more throughput should first prove the limitation with the CLI JSON
  boundary. A later task may then add a batch command or library boundary with measured evidence.
- File-level `syntax export` remains the canonical consumer export boundary for batch platform facts
  and future `v8-context` ingestion experiments. It does not become the interactive analyzer query
  protocol.
- The project avoids coupling to an unvalidated downstream analyzer while preserving a real,
  testable provider surface for BSL development workflows.

## Alternatives Considered

### Stabilize a Rust Library API Now

Rejected for the current stage.

A Rust API could reduce process overhead and share typed structs, but it would prematurely couple
downstream analyzers to this workspace's internal crate layout and provisional storage model. The
current consumer is not concrete enough to choose ownership, versioning, dependency or release
rules. Keep Rust APIs local to implementation until a real analyzer integration proves this is the
right boundary.

### Define a New Analyzer File Artifact Contract

Rejected for the current stage.

A file artifact could be efficient for batch import, but ADR-0001 already assigns batch platform
fact export to `syntax export`, while ADR-0004 assigns interactive lookup to the search index and
query commands. T56 storage normalization made the internal index more suitable for analyzer-grade
facts, but it did not create a concrete downstream consumer need for a second file contract.

### Add a Long-Running Provider Service

Rejected.

A daemon, MCP server or network service would add lifecycle, concurrency, packaging and operational
contracts that are not required for the accepted local BSL task scenarios. ADR-0006 requires local
deterministic provider behavior, not a service boundary.

### Treat the SQLite Index as the Public Artifact Contract

Rejected.

The SQLite index is a rebuildable derived artifact and may change while analyzer storage is still
being normalized. Downstream callers should use provider JSON instead of depending on table names,
row layouts or internal FTS fields.

## Implementation Plan

1. Record this ADR and index it in `spec/decisions/README.md`.
2. Update FR-SH-PROVIDER-001 to state that CLI JSON is the selected first analyzer-provider
   boundary.
3. Update `spec/implementation/syntax-helper-query-cli.md` and
   `spec/implementation/syntax-bsl-provider-plan.md` so future storage/API work does not reinterpret
   T56 as a boundary change.
4. Update the acceptance baseline and task ledger with the T55 decision.

## Verification

- [x] The selected provider boundary, non-goals and future-change trigger are recorded in this ADR.
- [x] The implementation specs point future analyzer work to CLI JSON as the current boundary.
- [x] No BSL parser, analyzer implementation, service boundary or new storage selector is added by
      this decision.
- [x] T56 storage normalization completed without making SQLite tables the public analyzer
      integration contract.

## More Information

### 2026-05-08: Rust Static Analyzer Uses ADR-0008

The first CLI JSON provider boundary remains valid for language-agnostic tools and existing UAT.
For a Rust static-analysis project that includes this repository as dependencies, use ADR-0008's
in-process resolver boundary instead. Do not reinterpret this ADR as requiring CLI JSON, HTTP, MCP
or a daemon between two Rust crates in the same analyzer application.
