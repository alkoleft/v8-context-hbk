## Context

`v8-context` active change `parallelize-module-analysis` needs module workers to read platform and language facts concurrently. The existing dependency-facing resolver path opens `syntax-helper-search::SearchIndex`, which owns `rusqlite::Connection`. That is acceptable for single-thread lookup and CLI use, but it prevents sharing one resolver across workers as `Sync`.

A downstream spike measured materializing a platform index through public lookup APIs from `.v8-context/platform-indexes/8.3.27.1859/shcntx_ru.sqlite`:

- build duration: `3765 ms`
- RSS delta: `51428 KiB`
- estimated heap: `32184468 bytes`
- platform types: `2419`
- type members: `18445`
- type member callables: `7753`
- query tables/fields/parameters: `53/498/56`
- query language facts from shcntx-only: `0`

That does not reject an in-memory read model. It rejects building it through lookup-oriented N+1 APIs and rejects shcntx-only coverage for language/query facts.

## Decision

The first snapshot source is the existing provider SQLite index, read through a bulk materializer owned by `v8-context-hbk`.

SQLite is a build input, not the public contract:

- open one read-only SQLite connection during snapshot construction;
- bulk-read only snapshot-required columns for identities, names, ownership, availability, compact
  query metadata, signatures, parameters, type refs and relations;
- close/drop the connection before worker lookup;
- publish only immutable snapshot contracts and worker-local handles.

Direct HBK book reading remains the refresh path that produces the SQLite provider index. It must be measured for comparison, but it is not the first worker-safe read-model source because it repeats parsing/extraction work and bypasses already normalized provider relations.

## Snapshot Storage Model

The storage model is nested and owned:

- `HbkFactSnapshot`
  - string pool or compact owned string storage;
  - platform type arena;
  - callable/signature/parameter arena;
  - member arena;
  - global context facts;
  - module context facts/events;
  - query table arena with owned fields/parameters;
  - language fact arena for BSL/query globals, functions, operators, keywords and literals;
  - secondary indexes.

The snapshot is intentionally narrower than the SQLite provider index. It must not copy search-only
or export-only data such as FTS rows, preview text, raw descriptions, diagnostics, duplicate
relation labels/weights, raw HBK paths, raw TOC paths, raw HTML paths or service parsing details
unless a concrete lookup contract needs them.

Primary ownership:

- platform type node owns ids of constructors, members, callables and type events;
- query table node owns ids of fields and parameters;
- global/module/query contexts own documented fact ids;
- provenance and availability belong to provider facts, not to analyzer records.

Secondary indexes are derived:

- exact id -> node id;
- normalized primary/alias name -> node id list;
- owner type id + member/callable key -> node id list;
- module context key -> event/callable ids;
- query table id/name/syntax/identifier -> query table id;
- relation source id + relation kind -> target fact ids.

Indexes must point into owned arenas and must not become duplicate sources of truth.

## DTO Boundary

Do not use existing `ContextFact`, `ResolvedType`, `ResolvedCallable` and related DTOs as the physical storage model. They are resolver response DTOs and are intentionally broad.

Add snapshot-specific provider DTOs or views:

- `HbkFactSnapshot`
- `HbkPlatformType`
- `HbkTypeMember`
- `HbkCallable`
- `HbkSignature`
- `HbkGlobalContext`
- `HbkModuleContext`
- `HbkQueryTable`
- `HbkLanguageFact`
- `HbkApplicability`
- `HbkProvenance`

The resolver adapter may project snapshot nodes into `context-resolver-core` DTOs for compatibility. Downstream analyzer code should not own fallback readers or raw SQLite queries.

## Materialization From SQLite

The bulk loader should read table families in coarse passes, selecting only columns required by the
snapshot contract:

1. `meta`, `documents`, `document_metadata`, `type_templates`.
2. `type_identities`, query-table identities if needed.
3. `members`, `callables`, `signatures`, `parameters`.
4. `type_refs`, `relations`.
5. derived indexes and nested ownership vectors.

This avoids public `SearchIndex` lookup construction such as per-type `members_by_type_id`, per-callable signature hydration and per-document relation traversal.

The first implementation may live near `syntax-helper-search` because that crate owns the SQLite schema. A resolver-facing adapter may live in `context-resolver-search` and depend only on the snapshot API, not on raw SQL.

## Threading Model

`HbkFactSnapshot` must be immutable after construction and compile as `Send + Sync`.

Workers receive:

```rust
Arc<HbkFactSnapshot>
```

and create local handles:

```rust
let handle = snapshot.worker_handle();
```

The handle may keep local caches or traces, but must not require shared mutable state, `Arc<Mutex<Resolver>>`, shared SQLite connections, or analyzer-owned mirror indexes.

## Measurement Plan

Measure at least:

- SQLite bulk materialization build time.
- RSS before/after and peak RSS when practical.
- estimated heap by arena/vector/string storage.
- node counts by category.
- lookup coverage counts: platform types, constructors, members, callables, globals, module events, query tables/fields/parameters, language facts.
- representative lookup latency:
  - type by exact name/id;
  - members by owner;
  - callable by owner/name;
  - global context;
  - module context events;
  - query table fields/parameters;
  - language global/function/operator/keyword lookup.
- comparison with:
  - current SQLite `SearchIndex` lookup path for representative calls;
  - current `syntax index <hbk>` path or existing measured HBK extraction/index build baseline.

Acceptance of SQLite-first materialization requires the measurement to show that bulk SQLite construction is materially better than the N+1 public lookup spike and avoids worker-time SQLite sharing.

## Initial Measurement: 2026-05-26

Temporary measurement harness:

```bash
cargo build --release -p syntax-helper-search --example measure_snapshot_materialization
/usr/bin/time -f 'time_elapsed_seconds=%e\ntime_peak_rss_kib=%M\ntime_exit_status=%x' \
  target/release/examples/measure_snapshot_materialization \
  target/snapshot-materialization/shcntx_ru.schema16.release.sqlite
```

The current release CLI rebuilt the comparison provider index first:

```bash
/usr/bin/time -f 'index_elapsed_seconds=%e\nindex_peak_rss_kib=%M\nindex_exit_status=%x' \
  target/release/v8-context-hbk syntax index \
  /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk \
  --output target/snapshot-materialization/shcntx_ru.schema16.release.sqlite
```

Results:

| Path | Elapsed | Peak RSS | Notes |
| --- | ---: | ---: | --- |
| HBK -> schema16 SQLite index | `14.50s` | `284360 KiB` | release CLI, `25415` documents |
| schema16 SQLite -> compact snapshot probe | `0.55s` process / `474 ms` materialize | `49112 KiB` | release example, connection dropped after build |

Snapshot probe counts:

| Category | Count |
| --- | ---: |
| strings | `168166` |
| documents | `25415` |
| document metadata rows | `728` |
| type identities | `2465` |
| type templates | `121` |
| members | `18609` |
| callables | `8337` |
| signatures | `8675` |
| parameters | `9793` |
| type refs | `47156` |
| relations | `58128` |
| name index keys | `102655` |
| member owner/name index keys | `18607` |
| callable owner/name index keys | `8329` |
| relation source/kind index keys | `32555` |

Other probe metrics:

- RSS delta: `46540 KiB`.
- Estimated heap: `34935365` bytes.
- Representative index lookup loop: `20000` iterations, `8922159 ns` total, `446 ns` average.

Conclusion:

SQLite-first bulk materialization is the right first implementation source. It reuses the existing
normalized provider artifact, avoids repeated HBK parsing/extraction, and is much faster than the
current release HBK -> SQLite index build. The compact probe replaced the earlier wide table-copy
probe and confirms the implementation direction: snapshot materialization must be contract-shaped
and should not preserve data that is only useful to search/export/index maintenance. The measurement
harness is service code and is not retained as a public crate example.

## Non-Goals

- Do not change `v8-context` analyzer behavior.
- Do not implement full parallel module analysis.
- Do not expose SQLite table names as a public downstream contract.
- Do not redesign the SQLite search index unless measurements show it is the bottleneck.
- Do not add compatibility fallback readers.
- Do not move analyzer/project semantics into HBK snapshot DTOs.
