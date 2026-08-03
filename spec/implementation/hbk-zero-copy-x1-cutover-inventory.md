# X1 canonical cutover: source and deletion inventory

This inventory is the mandatory input for OpenSpec tasks 5.1–5.3 and 7.2–7.3.
It was captured after the accepted X1-INT pass and before code deletion. A row
may be deleted only with the stated replacement and verification in place.

## Runtime and build boundary

The product runtime contract owned by this repository is the validated mapped
snapshot opener and its borrowed catalogs:

```text
stable X1 slot -> HbkFactSnapshot::open -> Arc -> catalogs/views -> consumer
```

`v8-context` does not yet have a non-benchmark analyzer composition root that
constructs the HBK platform snapshot. Its only current executable consumer is
the private `analyze-project` benchmark/integration harness. This cutover makes
that harness consume the canonical upstream API and records X1 as the required
future composition dependency; it does not claim that a nonexistent product
entrypoint was migrated.

SQLite remains a private provider build/search/debug representation. The
cutover removes SQLite-to-owned *snapshot runtime* entrypoints, not SQLite
indexing and search.

## Cutover ledger: OpenSpec 5.x

| Path / exact symbol | Current consumers | Action | Replacement owner | Proof / guard |
|---|---|---|---|---|
| `syntax-helper-search/src/snapshot/x1_format.rs`: `HbkFactSnapshot::open_x1_slot` | X1 lifecycle/parity tests, resolver mapped tests, downstream private X1-INT harness | rename to `HbkFactSnapshot::open(slot, &expectation)` and remove the experiment-name API without alias | validated X1 stable-slot owner | lifecycle/no-source tests plus source guard reject `open_x1_slot` and forbidden SQL/HBK/build names in runtime open |
| `context-resolver-search/src/snapshot_adapter.rs`: `PlatformSnapshotSource::from_index` | resolver tests | delete after tests explicitly build a fixture and call `with_source_id` | caller-owned `Arc<HbkFactSnapshot>` | adapter source guard rejects `SearchIndex`, `from_index` and provider paths |
| same: `PlatformSnapshotSource::open_read_only_with_source_id` | resolver tests | delete after fixture migration | caller-owned `Arc<HbkFactSnapshot>` | same guard; search/debug `PlatformSearchSource::open_read_only_with_source_id` is separately preserved |
| same: `QueryTableSnapshotSource::from_index` | resolver tests | delete after fixture migration | caller-owned `Arc<HbkFactSnapshot>` | adapter source guard rejects materialization and provider access |
| `syntax-helper-search/src/snapshot/materialize.rs`: `HbkFactSnapshot::from_path`, `from_index` and timed variants | publisher setup, parity tests, benchmark H0 baseline, old experiments | rename to `build_from_provider_path`, `build_from_provider_index` and timed variants | explicit build/setup boundary producing transient owned staging | source guards permit `build_from_provider_*` only in publisher/setup/fixtures and reject it in catalog/resolver runtime paths |
| downstream `analyze-project/src/benchmark/scenarios.rs`: `PlatformSnapshotInput::X1` | private X1-INT/benchmark scenarios only | call canonical `HbkFactSnapshot::open`; no fallback/default | upstream stable-slot owner | arm-extraction test rejects SQL/HBK/build/fallback names; source inventory proves no separate product constructor exists |
| same: `PlatformSnapshotInput::H0` and first-X1 producer | explicit private baseline/setup only | retain, but migrate to explicit build API | benchmark baseline and artifact publisher setup | structural test confines H0 to the named private selector and prevents access from the X1 arm |

## Garbage ledger: OpenSpec 7.x

These rows are reviewed only after the 5.x cutover passes. Historical Markdown
and experiment branches are evidence and are never deleted by this cleanup.

| Path / exact symbol | Consumer search before deletion | Action | Replacement / preserved evidence | Proof / guard |
|---|---|---|---|---|
| `syntax-helper-search/src/snapshot/binary_cache.rs`: `HbkFactSnapshotCacheLoadReport`, `HbkFactSnapshotCacheStatus`, `from_path_with_binary_cache`, `write_binary_cache`, `HBKFSN1` reader/writer/rebuild | old cache tests and experiment examples only; X1 currently imports codec and `CacheMetadata` from this file | delete legacy C0 runtime/API/magic; split reusable byte codec and build-input identity first | private `snapshot/codec.rs`; private X1 build-input identity; canonical X1 format/lifecycle | `rg`/structural test rejects legacy symbols/magic and rejects `binary_cache` from X1 modules |
| `snapshot/mod.rs`: cache re-exports and `HbkFactSnapshotBuildReport.cache_*` fields | materializer and X1 publisher identity check | delete re-exports; rename fields to provider build-input terms | private build-input identity used only to detect source mutation during publication | public API compile tests and X1 publication mutation test |
| `snapshot/read.rs`, `snapshot/views.rs`, `snapshot/x1_format.rs`: `super::binary_cache::{BinaryReader, BinaryWriter, BinaryValue}` | mapped reader/writer/views | move imports without semantic change | private `snapshot::codec` | X1 parity/lifecycle suite and guard that X1 does not name `binary_cache` |
| cache-only blocks in `syntax-helper-search/src/tests.rs` and `context-resolver-search/src/tests.rs` | retired C0 cache API only | delete only those blocks; mechanically rename all surviving owned-build fixtures | X1 corruption/version/lifecycle tests cover the canonical artifact; build/search tests remain | package tests plus retired-symbol guard |
| `examples/measure_hbk_fact_snapshot.rs` and `examples/measure_hbk_snapshot_scenario.rs` | historical H0/C0 measurement only | delete | durable T177/T183 baselines and experiment branches | Cargo example inventory and retired-symbol guard |
| `examples/measure_hbk_s83_av1.rs`, `examples/measure_hbk_s83_av2.rs`, `examples/dump_hbk_snapshot_oracle.rs` and their `Cargo.toml` entries | closed AV1/AV2/C0 experiment flow only | delete after full X1 cutover verification | durable AV1/AV2/X1 evidence, accepted full-corpus parity tests and experiment branches | feature/package build and exact path guard |
| `snapshot/experiment_oracle.rs` and its re-exports | only the deleted oracle/AV2 examples | delete if final consumer search is empty | full-corpus X1 parity tests | `rg` must return no live consumer |
| `snapshot/experiment_allocator.rs`, `snapshot-experiment-alloc` | X1 catalog zero-allocation tests and resolver allocation tests | preserve | accepted allocation regression contract | feature-enabled package tests |

## Mandatory preserve list

The following are outside cleanup scope even though they use SQLite, owned
build staging or raw HBK data:

- `SearchIndex`, `SearchIndexBuilder`, schema/locking and provider index build;
- `PlatformSearchSource` and `LanguageSearchSource`, including their read-only
  constructors used by CLI, index inspection, debug and explicit sequential
  resolver/search contracts;
- raw HBK extraction, provider JSON/export and CLI/query behavior;
- `HbkFactSnapshot` owned staging needed to build/publish X1, but only through
  `build_from_provider_*` APIs;
- X1 lifecycle/corruption/platform-version/full-corpus parity tests and
  `snapshot-experiment-alloc` allocation regression support;
- all durable `spec/` evidence and experiment branches/worktrees.

## Review gate

Before marking 7.2/7.3 complete, repeat consumer search after the diff. Any
surviving deleted-name use, unlisted deletion, SQLite/HBK read in runtime open,
or second owned/mapped runtime owner fails the task. Dependencies such as
`rusqlite` remain while the mandatory build/search/debug contracts use them.
