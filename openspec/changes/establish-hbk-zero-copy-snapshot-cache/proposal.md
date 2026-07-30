> **T183 BOUNDED EXPERIMENT — no production format selected.** This change
> authorizes isolated comparison prototypes and measurements. It does not
> authorize production adoption, canonical-runtime promotion or merging a
> candidate without a later user decision and accepted durable HBK change.

## Why

The provider-owned HBK binary cache already avoids rebuilding
`HbkFactSnapshot` from SQLite on every run, but its reader first allocates the
complete payload and then materializes every string, arena and nested vector
into heap-owned Rust values. This retains avoidable startup CPU, allocation and
peak-memory cost and prevents the existing HBK string dictionary from serving
as a read-only base for later project/session symbol composition.

## What Changes

- Explore and measure an immutable file-backed zero-copy representation of the
  existing provider-owned HBK fact snapshot.
- Keep HBK files as authoritative external documentation inputs. Promote the
  binary snapshot to the canonical HBK runtime context artifact only if it
  passes the accepted behavior, startup, lookup and resource gates; SQLite may
  then remain only as a private rebuild/index-production input.
- Replace, rather than layer over, the heap-materialized runtime snapshot for
  accepted consumers; the file-backed and heap-owned representations MUST NOT
  remain live as duplicate provider fact models.
- Lay out the HBK string dictionary, reverse string/name lookup and compact fact
  arenas/indexes so validated borrowed views can operate directly over a
  read-only mapped file.
- Evaluate the mapped HBK dictionary as generation-scoped base symbol storage.
  A later cross-source owner may reuse base IDs and add only BSL/metadata
  strings absent from HBK in a project-local overlay; HBK does not own that
  overlay.
- Preserve the existing typed borrowed BSL/SDBL catalogs and provider-owned
  lookup semantics without exposing cache layout to resolver or analyzer
  consumers.
- Compare a custom flat offset/range layout, a validated archived layout such
  as `rkyv`, and only the narrow on-disk indexes justified by measurements.
- Treat each zero-copy construction approach as a falsifiable hypothesis with
  its own expected win, rejection rule and measured result row.
- Compare SQLite-to-owned, current-cache-to-owned and zero-copy paths under one
  reproducible protocol covering production/rebuild, cold/warm ready-for-query
  startup, first lookup, batched warm lookup, allocations, RSS/PSS, page
  faults, file size and full observable behavior parity. The SQLite-to-owned
  path is the baseline row for the final comparison table.
- Decide separately whether a ready snapshot is produced by the HBK
  build/distribution pipeline or is derived locally on cache miss. A locally
  built snapshot cannot improve the first-ever run that builds it.
- Define exact platform-version and source compatibility checks, immutable
  publication, session-long shared reader locks, fail-fast exclusive writer
  locking, corruption handling and concurrent-reader/rebuild behavior before
  any memory mapping is accepted.
- **BREAKING (provisional):** the internal native runtime snapshot may change
  from heap-owned records with nested `Vec` fields to snapshot-backed borrowed
  records/ranges. Public semantic catalog behavior is intended to remain
  unchanged.

## Capabilities

### New Capabilities

- `hbk-zero-copy-snapshot-cache`: A measured, provider-owned, immutable
  file-backed HBK snapshot and base string dictionary with validated borrowed
  access, explicit lifecycle/invalidation and no parallel heap fact model.

### Modified Capabilities

- `hbk-fact-snapshot-cache`: If the candidate passes its gates, revise the
  current SQLite-canonical provider contract so the zero-copy snapshot becomes
  the single canonical HBK runtime context artifact, with SQLite retained only
  in an explicitly accepted private rebuild/index-production role.

## Impact

- Owning repository: `v8-context-hbk`.
- Likely affected crate: `syntax-helper-search`; public compatibility and
  parity checks may also affect `context-resolver-search`.
- Likely affected areas: snapshot binary cache, string storage, arena/index
  layout, read handles, borrowed BSL/SDBL catalogs, memory accounting,
  provider startup selection and cache publication.
- Candidate dependencies requiring a separate measured decision: `memmap2`,
  `rkyv`/`bytecheck`, `zerocopy`, and an mmap-capable reverse string index such
  as `fst`. This proposal accepts none of them yet.
- Upstream contracts to preserve: HBK remains the authoritative external
  documentation input; snapshot binary-layout, extraction-schema, source,
  locale and platform-version metadata are artifact provenance rather than
  entity identity; downstream code receives semantic catalogs/read handles and
  never snapshot bytes or offsets.
- Downstream draft dependency:
  `v8-context/openspec/changes/establish-unified-semantic-entity-model` must not
  stabilize a common `SymbolId`, HBK-backed base dictionary or cross-source
  string lifecycle until this change has separately accepted or rejected the
  zero-copy/base-dictionary hypothesis.
- Out of scope for T183: production implementation or migration,
  BSL/metadata overlay ownership, a process-global interner, persistent
  cross-project entity IDs, analyzer mirrors, old-layout compatibility shims
  and replacing HBK files as the authoritative documentation inputs.
