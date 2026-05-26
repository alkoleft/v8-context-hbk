## Context

T169 completed the provider-owned snapshot/read-handle physical indexes and the explicit resolver
backend split. Snapshot-backed analyzer lookup now composes `PlatformSnapshotSource` and
`QueryTableSnapshotSource` over immutable `Arc<HbkFactSnapshot>` state and no longer depends on
SQLite/SearchIndex on migrated hot paths.

The final post-T169 release measurement on
`target/snapshot-materialization/shcntx_ru.schema16.release.sqlite` reported warmed SQLite
materialization in `788-943 ms`, with `105860-106164 KiB` peak RSS,
`23324034` estimated snapshot-owned heap bytes and `17950274` payload bytes. The same run set read
the measurement-only binary cache in `29-30 ms`; the cache file was `11364011` bytes and each run
reported exact round-trip equality.

Those measurements change T170 from open-ended exploration into stabilization of a derived cache
boundary.

## Source-Of-Truth Boundary

The canonical provider artifact remains the SQLite index produced from HBK extraction. The persisted
snapshot cache is derived from that SQLite provider index and can be discarded and rebuilt.

The cache must not become:

- a public downstream analyzer format;
- a replacement for the provider SQLite index;
- a resolver adapter dependency;
- a place to store search/export payloads, fuzzy-search state or analyzer-owned mirrors.

Resolver adapters may receive a loaded `Arc<HbkFactSnapshot>`, but they must not know whether it came
from SQLite materialization or a provider-owned cache load.

## Cache Metadata And Invalidation

Before accepting the cache as a runtime startup path, the format must carry enough metadata to decide
whether it is valid for the source provider index:

- cache magic and cache format version;
- provider SQLite schema version;
- provider index identity, including a content hash or an equivalently strong source identity;
- platform version and locale/source family when available from provider metadata;
- snapshot layout version or explicit layout flags for fields/index families represented in the
  binary artifact;
- payload length/checksum or equivalent corruption/truncation guard.

On mismatch, unsupported version or failed integrity check, the provider layer must rebuild from the
SQLite index. The fallback is cache invalidation behavior, not analyzer fallback and not a hidden
SQL/SearchIndex resolver path.

## Format Selection

The first candidate remains the simple versioned Rust DTO/binary serialization path because it
already measures well and adds no runtime dependency. Zero-copy or memory-mapped layouts such as
`rkyv`, `zerocopy` or a custom mmap-friendly layout remain follow-up options only if the stable
cache path still spends meaningful time or memory in deserialization/allocation.

The decision must compare:

- SQLite materialization load/build time;
- cache validation and load time;
- cache write time if written during provider refresh;
- peak RSS;
- capacity-based heap bytes and logical payload bytes;
- cache file size;
- representative read-handle and snapshot-backed resolver lookup timings.

Because the current cache reader allocates exact vector capacities, memory conclusions must compare
both capacity-based heap counters and payload counters. Lower cache-loaded heap alone is not evidence
that the physical model shrank.

## T171 Boundary

T171 is complete for the migrated worker-safe analyzer slice. `PlatformSnapshotSource` and
`QueryTableSnapshotSource` are the accepted snapshot-backed adapters. Non-query-table
`BslLanguage` facts remain outside the migrated snapshot-backed resolver slice unless a later task
adds a dedicated `LanguageSnapshotSource` and its identity/no-fallback coverage.

T170 must not reopen T171. Its cache work loads the same provider-owned snapshot shape and preserves
the existing resolver backend split.
