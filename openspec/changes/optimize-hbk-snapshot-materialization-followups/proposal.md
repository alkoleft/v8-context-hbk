## Why

T174 removed the largest transient raw-row peak in `HbkFactSnapshot`
materialization, but the same profiled provider path still has smaller,
independent costs. They require separate ownership and evidence: redundant
signature-line materialization, a filtered relation-row collection, short-lived
interner duplication, unknown collection-capacity growth and an unwired derived
cache.

## What Changes

- Keep one evidence ledger for follow-up hypotheses H2-H8, their owners,
  acceptance gates and terminal decisions.
- Remove H7's materialization of every signature line followed by cloning the
  selected line, preserving selected-line ordering and empty-line behavior.
- Evaluate H2 owner-edge streaming only after H7, with a direct semantic oracle
  for enum ownership and a separate time/RSS result.
- Measure H3 interner duplication and H4 temporary-capacity growth before
  selecting an implementation; do not reshape final snapshot storage or cache
  layout speculatively.
- Record H5 as a provider-owned cache-startup design decision, not an
  analyzer-side cache or a format exposed to resolver callers.
- Record the official 1C semantic evidence that document, return and parameter
  type facts remain context-specific; no fact-group merge or deduplication is
  authorized as an optimization.

## Capabilities

### New Capabilities

- `hbk-snapshot-followup-efficiency`: evidence-driven, owner-local reductions
  of remaining snapshot-materialization temporary allocation without changing
  provider facts or cache contracts.

### Modified Capabilities

- None.

## Impact

The first implementation task is limited to the private snapshot materializer
in `syntax-helper-search`, with focused behavior tests and release
measurements. Later tasks may touch the same provider crate only after their
own evidence gates. No analyzer-owned cache, provider mirror, SQLite
schema/index, binary-cache layout, public resolver contract or 1C/BSL semantic
model is changed by default.
