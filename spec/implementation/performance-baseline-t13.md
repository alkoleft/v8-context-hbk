# T13 Performance and Resource Baseline

Date: 2026-04-30.

Task: T13, performance/resource baseline and implementation hypotheses.

Raw command outputs and generated exports were written under `target/t13-baseline-20260430/`.
That directory is service data and is not a durable source of truth.

This document keeps the original T13 baseline and the promoted post-baseline conclusions. T14 and
T15 measurements were recorded after their implementation tasks because the current optimization
direction depends on the delta, not only the initial baseline.

## Measurement Method

The CLI was built once before measurement so that compile time did not pollute runtime results:

```bash
cargo build -p v8-context-hbk-cli --bin v8-context-hbk
```

Runtime commands used the built debug binary:

```bash
target/debug/v8-context-hbk
```

Resource metrics were collected with GNU `time`:

```bash
/usr/bin/time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x'
```

`peak_rss_kb` is the maximum resident set size reported by `/usr/bin/time`.
Output byte sizes were measured with `wc -c`. Syntax Assistant record counts were measured from the
exported JSON envelopes with `jq '.records | length'` where applicable.

Host tool versions:

- `cargo 1.93.1 (083ac5135 2025-12-15)`
- `rustc 1.93.1 (01f6ddf75 2026-02-11)`

## Fixture Availability

Target platform directory:
`/opt/1cv8/x86_64/8.5.1.1150`.

| Fixture | Status | Size, bytes |
| --- | --- | ---: |
| `fmtdui_root.hbk` | ran | 2587 |
| `fmtdui_ru.hbk` | ran | 4429 |
| `shcntx_root.hbk` | ran | 35021688 |
| `shcntx_ru.hbk` | ran | 41361963 |
| all target-platform `*.hbk` files | ran | 116 files discovered |

No fixture-backed T13 command was skipped on this host.

## Measured Commands

Small HBK smoke:

```bash
target/debug/v8-context-hbk inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk
target/debug/v8-context-hbk inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk
target/debug/v8-context-hbk toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk --format json
target/debug/v8-context-hbk toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json
```

Syntax Assistant export:

```bash
target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t13-baseline-20260430/exports/shcntx-ru
target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/t13-baseline-20260430/exports/shcntx-root
```

All-HBK smoke equivalent:

```bash
for path in /opt/1cv8/x86_64/8.5.1.1150/*.hbk; do
  target/debug/v8-context-hbk inspect "$path"
  target/debug/v8-context-hbk toc "$path" --format json
done
```

## Command Results

| Command label | Exit | Elapsed, s | Peak RSS, KiB | Output count | Stdout bytes | Stderr bytes |
| --- | ---: | ---: | ---: | --- | ---: | ---: |
| `inspect fmtdui_root` | 0 | 0.00 | 5884 | 7 entities | 557 | 0 |
| `inspect fmtdui_ru` | 0 | 0.00 | 5760 | 7 entities | 551 | 0 |
| `toc fmtdui_root --format json` | 0 | 0.00 | 6272 | 1 HTML path | 186 | 0 |
| `toc fmtdui_ru --format json` | 0 | 0.00 | 6272 | 1 HTML path | 186 | 0 |
| `syntax-helper shcntx_ru --output` | 0 | 20.80 | 752392 | 24837 records, 703 diagnostics, 11 files | 278 | 0 |
| `syntax-helper shcntx_root --output` | 0 | 16.15 | 518972 | 24837 records, 703 diagnostics, 11 files | 282 | 0 |
| all-HBK smoke equivalent | 0 | 14.69 | 386304 | 116 inspect successes, 116 TOC successes | 24556320 | 0 |

All-HBK output bytes combine `inspect` and `toc --format json` stdout:

- `inspect` stdout: 63952 bytes.
- `toc --format json` stdout: 24492368 bytes.
- no stderr output.

## Syntax Assistant Export Sizes

`shcntx_ru.hbk` export:

| File | Bytes | Records |
| --- | ---: | ---: |
| `metadata.json` | 1036 | n/a |
| `global-contexts.json` | 40785 | 1 |
| `global-methods.json` | 1697049 | 500 |
| `global-properties.json` | 120483 | 101 |
| `platform-types.json` | 5127153 | 2533 |
| `type-methods.json` | 14168616 | 6702 |
| `type-properties.json` | 13817149 | 10732 |
| `constructors.json` | 747331 | 445 |
| `enums.json` | 1559004 | 713 |
| `enum-values.json` | 2590954 | 3110 |
| `diagnostics.json` | 334019 | 703 |
| total directory size | 40207675 | n/a |

`shcntx_root.hbk` export:

| File | Bytes | Records |
| --- | ---: | ---: |
| `metadata.json` | 1040 | n/a |
| `global-contexts.json` | 55788 | 1 |
| `global-methods.json` | 1063961 | 500 |
| `global-properties.json` | 71058 | 101 |
| `platform-types.json` | 3736901 | 2533 |
| `type-methods.json` | 8762280 | 6702 |
| `type-properties.json` | 8768194 | 10732 |
| `constructors.json` | 509902 | 445 |
| `enums.json` | 1231352 | 713 |
| `enum-values.json` | 1889432 | 3110 |
| `diagnostics.json` | 335281 | 703 |
| total directory size | 26429285 | n/a |

## Hotspot Review

This section records the original T13 code review before Variant A and Variant B were implemented.

`hbk-container`:

- `HbkContainer::open` maps the whole HBK file with `memmap2`; this is currently simple and does not
  eagerly copy the mapped file contents.
- `HbkContainer::read_entity` returns a fresh `Vec<u8>` for each entity body, so consumers that read
  large entities own a full copy after crossing the container boundary.

`hbk-book`:

- `HbkBook` stores `file_storage: Vec<u8>`.
- `HbkBook::from_container` reads the whole `FileStorage` entity into that vector when a book is
  opened.
- At T13, `read_file` opened a `ZipArchive` over the stored `FileStorage` bytes for each page read.
- At T13, `read_pages` built a requested-path set, scanned ZIP entries, decoded matching pages, and
  accumulated all requested page strings in a `BTreeMap<String, String>`.

`syntax-helper-extract`:

- At T13, `SyntaxHelperReader::extract` first read root pages into a `BTreeMap`, then read all
  selected extraction pages into another `BTreeMap` before parsing.
- `parse_extraction_pages` accumulates the full `PlatformContext` vectors before export.
- Current traversal is deterministic because page paths and visited sets are ordered, but memory is
  paid up front before export starts.

`hbk-export`:

- `JsonExporter` writes every record family from the already materialized `PlatformContext`.
- At T13, envelopes included export-level `source_hbk`, and records still contained provenance and
  navigation scaffolding from the internal model.
- At T13, `write_file` used `serde_json::to_vec_pretty`, which materialized pretty JSON bytes for
  each file before writing them to disk.

## Post-Baseline Measurements

T14 implemented Variant A: lean consumer export plus compact streaming JSON writing.

| Slice | Command label | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| T14 | `syntax-helper shcntx_ru --output` | 0 | 20.20 | 752392 | 21946830 |
| T14 | `syntax-helper shcntx_root --output` | 0 | 15.82 | 518844 | 12265898 |

T14 significantly reduced export size but did not reduce peak RSS, so the next active slice became
Variant B.

T15 implemented Variant B: lazy Syntax Assistant page loading through a reusable `FileStorageReader`,
consuming traversal metadata during parsing, and removing avoidable per-page parser copies.

| Slice | Command label | Exit | Elapsed, s | Peak RSS, KiB | Output count |
| --- | ---: | ---: | ---: | ---: | --- |
| T15 | `syntax-helper shcntx_ru --output` | 0 | 19.26 | 590988 | 24837 records, 703 diagnostics, 10 files |
| T15 | `syntax-helper shcntx_root --output` | 0 | 14.62 | 324476 | 24837 records, 703 diagnostics, 10 files |

T15 improved wall-clock time slightly and reduced peak RSS, especially for `shcntx_root.hbk`.
`shcntx_ru.hbk` still peaks above 500 MiB, so the remaining memory must be attributed before
choosing the next implementation slice.

Current post-T15 hotspots to measure:

- full `PlatformContext` accumulation before export;
- export adapter allocation during JSON writing;
- whole `FileStorage` ownership and container/entity copies;
- parser temporary allocation or allocator retention.

T16 attributed the remaining post-T15 memory. Raw command outputs, generated exports and the
temporary attribution probe were written under `target/t16-memory-attribution-20260430/`; that
directory is service data and is not a durable source of truth.

T16 built the debug CLI and a temporary probe that reused the same workspace crates:

```bash
cargo build -p v8-context-hbk-cli --bin v8-context-hbk
cargo build --manifest-path target/t16-memory-attribution-20260430/probe/Cargo.toml
```

The actual CLI was re-measured with GNU `time`:

```bash
/usr/bin/time -o target/t16-memory-attribution-20260430/logs/cli-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t16-memory-attribution-20260430/exports/cli-shcntx-ru
/usr/bin/time -o target/t16-memory-attribution-20260430/logs/cli-shcntx-root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/t16-memory-attribution-20260430/exports/cli-shcntx-root
```

The probe ran the same books through `open`, `discover`, `extract` and `export` modes with the same
GNU `time` format. Export mode used:

```bash
target/t16-memory-attribution-20260430/probe/target/debug/t16-memory-probe export /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk target/t16-memory-attribution-20260430/exports/probe-shcntx-ru
target/t16-memory-attribution-20260430/probe/target/debug/t16-memory-probe export /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk target/t16-memory-attribution-20260430/exports/probe-shcntx-root
```

The other probe modes used the same binary and HBK path with `open`, `discover` or `extract` in
place of `export`, without an output directory.

No fixture-backed T16 command was skipped on this host.

| T16 command label | Exit | Elapsed, s | Peak RSS, KiB | Output bytes |
| --- | ---: | ---: | ---: | ---: |
| CLI `syntax-helper shcntx_ru --output` | 0 | 18.64 | 588892 | 21950926 |
| CLI `syntax-helper shcntx_root --output` | 0 | 14.07 | 324352 | 12269994 |

| Probe mode | `shcntx_ru` peak RSS, KiB | `shcntx_root` peak RSS, KiB |
| --- | ---: | ---: |
| `open` | 385024 | 323196 |
| `discover` | 385024 | 323200 |
| `extract` | 590532 | 323328 |
| `export` | 588876 | 323072 |

Key probe stage readings from `/proc/self/status`:

| Source | Stage | Current RSS, KiB | VmHWM, KiB |
| --- | --- | ---: | ---: |
| `shcntx_ru.hbk` | after `HbkBook::open` | 112660 | 385152 |
| `shcntx_ru.hbk` | after extract | 589152 | 589152 |
| `shcntx_ru.hbk` | after export | 589248 | 589248 |
| `shcntx_root.hbk` | after `HbkBook::open` | 100084 | 323072 |
| `shcntx_root.hbk` | after extract | 313036 | 323072 |
| `shcntx_root.hbk` | after export | 313100 | 323072 |

T16 attribution:

- Full `PlatformContext` accumulation and parser-held extracted data dominate the remaining
  `shcntx_ru.hbk` peak: RSS rises from about 113 MiB after open to about 589 MiB after extract, and
  the extraction-only probe peak matches the full export path.
- Export adapter allocation during JSON writing is not the dominant remaining peak. Export adds
  about 96 KiB current RSS on `shcntx_ru.hbk`, about 64 KiB on `shcntx_root.hbk`, and no material
  high-water increase.
- Whole `FileStorage` ownership and container/entity copies are still a real lower-level hotspot.
  `HbkBook::open` reaches a high-water mark of 385024 KiB for `shcntx_ru.hbk` and 323196 KiB for
  `shcntx_root.hbk`, while the retained RSS after open is only about 100-113 MiB. Code inspection
  attributes that opening spike to `read_block_content_with_offsets` building per-byte
  `source_offsets` for entity reads even when the caller only needs bytes.
- Allocator retention is visible after dropping the context/book in the probe logs, but the peak is
  already reached before export and is tied to the extraction/model path for `shcntx_ru.hbk`.

T16 selects Variant C for T17. Variant E remains a later candidate if the lower-level open-time
spike or retained `FileStorage` ownership remains limiting after streaming extraction, but Variant E
alone would not reduce the current `shcntx_ru.hbk` extraction peak below the post-T15 value.

## Post-T17 Update

T17 implemented Variant C: the Syntax Assistant extractor now emits typed records through a shared
sink boundary. The in-memory `PlatformContext` remains the sink used by lookup helpers, while the
CLI export command streams record-family events directly into canonical JSON writers.

The T17 pass used the built debug binary under GNU `time`:

```bash
cargo build -p v8-context-hbk-cli --bin v8-context-hbk
/usr/bin/time -o target/t17-measurements/logs/shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t17-measurements/exports/shcntx-ru
/usr/bin/time -o target/t17-measurements/logs/shcntx-root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/t17-measurements/exports/shcntx-root
```

No fixture-backed T17 command was skipped on this host.

| T17 command label | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: |
| CLI `syntax-helper shcntx_ru --output` | 0 | 20.46 | 386304 | 21946830 |
| CLI `syntax-helper shcntx_root --output` | 0 | 18.15 | 324096 | 12265898 |

Each source book produced 1 global context, 24836 exported consumer records and 703 diagnostics.
The CLI output reported the same per-family counts as T15/T16.

T17 deterministic-order verification exported `shcntx_ru.hbk` twice and compared all generated JSON
files byte-for-byte. The comparison passed.

Compared with T15:

- `shcntx_ru.hbk` peak RSS decreased from `590988 KiB` to `386304 KiB`; wall-clock was not the
  optimized metric for this memory slice and varies across debug runs.
- `shcntx_root.hbk` peak RSS stayed near the T15/T16 value, consistent with the T16 finding that the
  root-source path is dominated by the lower-level `HbkBook::open` high-water mark rather than full
  `PlatformContext` accumulation.
- Export bytes returned to the compact T14 values because T17 preserves the FR-EXPORT-001 consumer
  shape.

Variant E remains the next candidate only if the lower-level open-time `FileStorage`/container copy
spike is worth addressing after query-CLI priorities are considered.

## Post-T19 Update

T19 implemented the first lower-level Variant E slice before T18 by explicit memory-footprint
reprioritization. Ordinary `HbkContainer::read_entity` reads now use a byte-only block-content path;
the offset-aware path remains for descriptor parsing and diagnostics.

Raw command outputs, generated exports and the temporary open probe were written under
`target/t19-measurements/`. That directory is service data and is not a durable source of truth.

The T19 pass used the built debug CLI and a temporary open probe under GNU `time`:

```bash
cargo build -p v8-context-hbk-cli --bin v8-context-hbk
cargo build --manifest-path target/t19-measurements/probe/Cargo.toml
/usr/bin/time -o target/t19-measurements/logs/before-open-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t19-measurements/probe/target/debug/t19-open-probe /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk
/usr/bin/time -o target/t19-measurements/logs/before-open-shcntx-root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t19-measurements/probe/target/debug/t19-open-probe /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk
/usr/bin/time -o target/t19-measurements/logs/after-open-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t19-measurements/probe/target/debug/t19-open-probe /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk
/usr/bin/time -o target/t19-measurements/logs/after-open-shcntx-root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t19-measurements/probe/target/debug/t19-open-probe /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk
```

The full export measurements used the same T13-style debug-binary command:

```bash
/usr/bin/time -o target/t19-measurements/logs/before-syntax-helper-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t19-measurements/exports/before/shcntx-ru
/usr/bin/time -o target/t19-measurements/logs/before-syntax-helper-shcntx-root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/t19-measurements/exports/before/shcntx-root
/usr/bin/time -o target/t19-measurements/logs/after-syntax-helper-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t19-measurements/exports/after/shcntx-ru
/usr/bin/time -o target/t19-measurements/logs/after-syntax-helper-shcntx-root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/t19-measurements/exports/after/shcntx-root
```

No fixture-backed T19 command was skipped on this host.

Open-only probe results:

| Source | Phase | Exit | Elapsed, s | Current RSS, KiB | VmHWM / peak RSS, KiB |
| --- | --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | before | 0 | 7.48 | 110868 | 383232 |
| `shcntx_ru.hbk` | after | 0 | 10.27 | 108332 | 131328 |
| `shcntx_root.hbk` | before | 0 | 7.10 | 98412 | 321408 |
| `shcntx_root.hbk` | after | 0 | 6.50 | 95888 | 119168 |

T13-style full `syntax-helper --output` results:

| Source | Phase | Exit | Elapsed, s | Peak RSS, KiB | Export bytes | Records and diagnostics |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `shcntx_ru.hbk` | before | 0 | 28.40 | 386048 | 21946830 | 24836 records, 703 diagnostics |
| `shcntx_ru.hbk` | after | 0 | 21.19 | 168692 | 21946830 | 24836 records, 703 diagnostics |
| `shcntx_root.hbk` | before | 0 | 32.36 | 324352 | 12265898 | 24836 records, 703 diagnostics |
| `shcntx_root.hbk` | after | 0 | 16.11 | 144500 | 12265898 | 24836 records, 703 diagnostics |

The post-T19 record-family counts remained unchanged for each source book: 1 global context, 500
global methods, 101 global properties, 2533 platform types, 6702 type methods, 10732 type
properties, 445 constructors, 713 enums, 3110 enum values and 703 diagnostics. Export byte sizes
also stayed unchanged, confirming that the change did not alter FR-EXPORT-001 output shape.

The byte-only path removed the majority of the remaining open-time high-water mark. T19 therefore
does not add a follow-up task for a seekable direct `FileStorage` view over mmap/chained blocks.

## Post-T20 Update

T20 evaluated whether the remaining owned `FileStorage: Vec<u8>` justified replacing it with a
direct seekable view over mmap/chained HBK blocks. Raw command outputs, generated exports and the
temporary attribution probe were written under `target/t20-measurements/`. That directory is service
data and is not a durable source of truth.

The T20 pass used the built debug CLI and a temporary fresh-process attribution probe under GNU
`time`:

```bash
cargo build -p v8-context-hbk-cli --bin v8-context-hbk
cargo build --manifest-path target/t20-measurements/probe/Cargo.toml
/usr/bin/time -o target/t20-measurements/logs/container-open-shcntx_ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t20-measurements/probe/target/debug/t20-file-storage-probe container-open /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk
/usr/bin/time -o target/t20-measurements/logs/file-storage-copy-shcntx_ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t20-measurements/probe/target/debug/t20-file-storage-probe file-storage-copy /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk
/usr/bin/time -o target/t20-measurements/logs/book-open-shcntx_ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t20-measurements/probe/target/debug/t20-file-storage-probe book-open /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk
/usr/bin/time -o target/t20-measurements/logs/container-open-shcntx_root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t20-measurements/probe/target/debug/t20-file-storage-probe container-open /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk
/usr/bin/time -o target/t20-measurements/logs/file-storage-copy-shcntx_root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t20-measurements/probe/target/debug/t20-file-storage-probe file-storage-copy /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk
/usr/bin/time -o target/t20-measurements/logs/book-open-shcntx_root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t20-measurements/probe/target/debug/t20-file-storage-probe book-open /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk
```

The full export measurements used the same T13-style debug-binary command:

```bash
/usr/bin/time -o target/t20-measurements/logs/syntax-helper-shcntx_ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t20-measurements/exports/shcntx-ru
/usr/bin/time -o target/t20-measurements/logs/syntax-helper-shcntx_root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/t20-measurements/exports/shcntx-root
```

Fresh-process attribution results:

| Source | Mode | Exit | Elapsed, s | Current RSS, KiB | VmHWM, KiB | Exact `FileStorage` bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `container-open` | 0 | 0.00 | 2800 | 2800 | n/a |
| `shcntx_ru.hbk` | `file-storage-copy` | 0 | 0.02 | 78740 | 78740 | 38960718 |
| `shcntx_ru.hbk` | `book-open` | 0 | 5.19 | 108324 | 131712 | 38960718 |
| `shcntx_root.hbk` | `container-open` | 0 | 0.00 | 2672 | 2672 | n/a |
| `shcntx_root.hbk` | `file-storage-copy` | 0 | 0.02 | 66468 | 66468 | 32620458 |
| `shcntx_root.hbk` | `book-open` | 0 | 5.26 | 95884 | 119296 | 32620458 |

The `file-storage-copy` RSS includes both the retained destination vector and source mmap pages
touched while copying. The exact vector capacity is therefore the decision input for retained
`FileStorage` ownership: about `38048 KiB` for `shcntx_ru.hbk` and `31856 KiB` for
`shcntx_root.hbk`.

T13-style full `syntax-helper --output` results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes | Records and diagnostics |
| --- | ---: | ---: | ---: | ---: | --- |
| `shcntx_ru.hbk` | 0 | 17.68 | 157916 | 21950926 | 24836 records, 703 diagnostics |
| `shcntx_root.hbk` | 0 | 13.50 | 139632 | 12269994 | 24836 records, 703 diagnostics |

Each source book still produced 1 global context, 500 global methods, 101 global properties, 2533
platform types, 6702 type methods, 10732 type properties, 445 constructors, 713 enums, 3110 enum
values and 703 diagnostics.

T20 conclusion:

- The owned `FileStorage` vector is material but not dominant: it is about 35% of retained
  `HbkBook::open` RSS for `shcntx_ru.hbk`, about 33% for `shcntx_root.hbk`, and less than one
  quarter of the full export peak for both books.
- Replacing it would require a broader direct seekable ZIP/storage design inside the low-level book
  boundary while leaving larger retained book/extraction state in place.
- A direct seekable `FileStorage` view is therefore not justified by T20 evidence. No runtime code
  change was made, and Variant E remains limited to the T19 byte-only entity path until new
  measurements show a dominant lower-level storage bottleneck.

## Post-T21 Update

T21 evaluated whether retained TOC, flattened traversal metadata and Syntax Assistant root-discovery
state justified a production refactor before T18. Raw command outputs, generated exports and the
temporary attribution probe were written under `target/t21-measurements/`. That directory is service
data and is not a durable source of truth.

The probe measured owned-by-root structure estimates for:

- actual `Toc` tree retained by `HbkBook`;
- actual retained `book.toc().flat_pages().collect::<Vec<_>>()` metadata;
- actual public `SyntaxHelperReader::discover_roots()` result;
- a shape-equivalent copy of the private `syntax_toc_index` fields retained by the export path:
  cloned `html_path`, `toc_path` string and title string.

The T21 pass used the built debug CLI and temporary probe under GNU `time`:

```bash
cargo build -p v8-context-hbk-cli --bin v8-context-hbk
cargo build --manifest-path target/t21-measurements/probe/Cargo.toml
```

No fixture-backed T21 command was skipped on this host.

Fresh-process attribution results:

| Source | Mode | Exit | Elapsed, s | Current RSS, KiB | VmHWM / peak RSS, KiB | Retained estimate, bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `book-open` / `Toc` tree | 0 | 5.17 | 109864 | 133376 | 8367325 |
| `shcntx_ru.hbk` | retained `flat_pages` metadata | 0 | 5.27 | 109772 | 133248 | 2139400 |
| `shcntx_ru.hbk` | public `RootDiscovery` | 0 | 15.64 | 147000 | 147000 | 9177088 |
| `shcntx_ru.hbk` | `syntax_toc_index` shape | 0 | 5.50 | 110276 | 133120 | 5149766 |
| `shcntx_root.hbk` | `book-open` / `Toc` tree | 0 | 5.24 | 97420 | 120704 | 8332291 |
| `shcntx_root.hbk` | retained `flat_pages` metadata | 0 | 5.24 | 97320 | 120704 | 2139400 |
| `shcntx_root.hbk` | public `RootDiscovery` | 0 | 16.27 | 139520 | 139520 | 9257408 |
| `shcntx_root.hbk` | `syntax_toc_index` shape | 0 | 5.60 | 97808 | 120704 | 5132816 |

Both source books had 28736 TOC pages. Public root discovery found 10 roots, retained 28736 catalog
pages and produced 703 diagnostics for each source book. The `syntax_toc_index` shape contained
25883 entries for each source book.

T13-style full `syntax-helper --output` results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes | Records and diagnostics |
| --- | ---: | ---: | ---: | ---: | --- |
| `shcntx_ru.hbk` | 0 | 19.04 | 157788 | 21950926 | 24836 records, 703 diagnostics |
| `shcntx_root.hbk` | 0 | 14.33 | 139764 | 12269994 | 24836 records, 703 diagnostics |

Each source book still produced 1 global context, 500 global methods, 101 global properties, 2533
platform types, 6702 type methods, 10732 type properties, 445 constructors, 713 enums, 3110 enum
values and 703 diagnostics.

T21 conclusion:

- The actual retained `RootDiscovery` graph is the largest T21-specific structure, but its
  owned-by-root estimate is about 9 MiB, under 7% of the measured full export peak.
- The private traversal index shape is about 5 MiB, retained flat-page metadata is about 2 MiB, and
  the public `Toc` tree is about 8 MiB. The `Toc` tree is also required by the public help-book
  navigation contract.
- A production refactor would cross extraction traversal/root-discovery structure boundaries for a
  bounded single-digit MiB gain, while the full export peak remains dominated by required book state
  plus page parsing/export work.
- A lean traversal/root-discovery representation is therefore not justified by T21 evidence. No
  runtime code change was made.

## Post-T22 Update

T22 evaluated which lower-level `HbkBook` state remained live during the `syntax-helper --output`
streaming export path after T20/T21. Raw command outputs, generated exports and the temporary probe
were written under `target/t22-measurements/`. That directory is service data and is not a durable
source of truth.

The probe measured fresh-process `book-open`, drop and public `root-discovery` modes for both Syntax
Assistant books. The drop probe showed that the book object itself retained most of the current RSS
after open: dropping the whole book reduced RSS from `109752 KiB` to `32984 KiB` for
`shcntx_ru.hbk` and from `97212 KiB` to `32844 KiB` for `shcntx_root.hbk`.

Before the change, `HbkBook` retained the lower-level `HbkContainer` mmap even though the public
book API only needed the source path, metadata, locale, TOC and `FileStorage` bytes after open. T22
therefore replaced that retained container field with an owned `PathBuf` and let `HbkContainer` drop
after book construction.

The T22 pass used the built debug CLI and temporary probe under GNU `time`:

```bash
cargo build -p v8-context-hbk-cli --bin v8-context-hbk
cargo build --manifest-path target/t22-measurements/probe/Cargo.toml
/usr/bin/time -o target/t22-measurements/logs/after-syntax-helper-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t22-measurements/exports/after/shcntx-ru
/usr/bin/time -o target/t22-measurements/logs/after-syntax-helper-shcntx-root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/t22-measurements/exports/after/shcntx-root
```

No fixture-backed T22 command was skipped on this host.

Fresh-process attribution results:

| Source | Mode | Before RSS, KiB | After RSS, KiB | Before VmHWM, KiB | After VmHWM, KiB |
| --- | --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `book-open` | 109748 | 70936 | 132992 | 132864 |
| `shcntx_root.hbk` | `book-open` | 97264 | 64700 | 120448 | 120448 |
| `shcntx_ru.hbk` | `root-discovery` | 146796 | 108016 | 146796 | 132992 |
| `shcntx_root.hbk` | `root-discovery` | 139436 | 106868 | 139436 | 120448 |

T13-style full `syntax-helper --output` results:

| Source | Exit | Before elapsed, s | After elapsed, s | Before peak RSS, KiB | After peak RSS, KiB | Export bytes | Records and diagnostics |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `shcntx_ru.hbk` | 0 | 19.02 | 17.97 | 168800 | 134656 | 21950926 | 24836 records, 703 diagnostics |
| `shcntx_root.hbk` | 0 | 14.79 | 13.65 | 140624 | 122112 | 12269994 | 24836 records, 703 diagnostics |

Each source book still produced 1 global context, 500 global methods, 101 global properties, 2533
platform types, 6702 type methods, 10732 type properties, 445 constructors, 713 enums, 3110 enum
values and 703 diagnostics.

T22 conclusion:

- The `HbkContainer` mmap retained by `HbkBook` was avoidable after book construction and material
  during the streaming export path.
- Releasing it reduced current `book-open` RSS by about 38 MiB for `shcntx_ru.hbk` and about
  32 MiB for `shcntx_root.hbk`.
- Full `syntax-helper --output` peak RSS decreased from `168800 KiB` to `134656 KiB` for
  `shcntx_ru.hbk` and from `140624 KiB` to `122112 KiB` for `shcntx_root.hbk`.
- Pre-change and post-change export directories were byte-identical for both source books.
- Further splitting of `FileStorage`, TOC/root-discovery or parser traversal lifetimes is not
  justified by the current evidence.

## Variant Evaluation

Variant A, lean consumer export and streaming JSON writer, remains the first slice.

Evidence:

- `syntax-helper` output is large: about 40 MB for `shcntx_ru` and 26 MB for `shcntx_root`.
- The largest record-family files are `type-methods.json`, `type-properties.json` and
  `platform-types.json`.
- Export currently serializes pretty JSON into memory before writing.
- Variant A also aligns FR-EXPORT-001 with the actual consumer contract by removing source
  provenance and navigation scaffolding from consumer files.

Variant B, lazy or batched page loading, is the likely next memory-focused slice after Variant A.

Evidence:

- `syntax-helper shcntx_ru` reached 752392 KiB peak RSS, while the aggregate all-HBK generic smoke
  reached 386304 KiB.
- Code review shows full `FileStorage` ownership plus whole-page `BTreeMap` accumulation before
  parsing.
- Variant B should be measured after Variant A so export-size savings and page-loading savings are
  not conflated.

Variant D is not first.

- The debug build wall-clock for `syntax-helper` is 16.15-20.80 seconds, but this baseline does not
  isolate CPU-bound HTML parsing from ZIP reads, page accumulation or JSON serialization.
- Parallel parsing would add ordering and memory-risk surfaces before a narrower streaming/export
  slice is tried.

Variant C was implemented in T17.

- Streaming extraction into record-family sinks crosses model and export boundaries, so it was
  deferred until Variant A and B completed.
- T16 showed full `PlatformContext` accumulation and the export-oriented command shape are the
  dominant remaining `shcntx_ru.hbk` peak after page loading was bounded.
- T17 confirmed this selection for `shcntx_ru.hbk`: peak RSS dropped to `386304 KiB` while the
  canonical export shape and counts stayed stable.

Variant E is not first.

- `memmap2` remains the simplest low-copy container-open strategy.
- The whole `FileStorage` copy and per-byte `source_offsets` temporary allocation matter, but T16
  showed they are not the next slice most likely to reduce the current `shcntx_ru.hbk` peak.
- T19 removed the dominant per-byte `source_offsets` allocation from ordinary entity reads.
- T20 showed the remaining owned `FileStorage` vector is not dominant enough to justify a broader
  direct seekable ZIP/storage design.

## Current Implementation Direction

The original T13 direction selected T14 / Variant A first. T14 completed but left peak RSS high, so
T15 / Variant B was implemented next.

T16 attributed the remaining post-T15 memory and selected T17 / Variant C next. T17 implemented
streaming extraction into record-family sinks for the export command path while preserving the
in-memory `syntax-helper-model` lookup use case.

T19 completed the narrow byte-only Variant E slice. T20 evaluated the broader direct seekable
`FileStorage` view and left it unimplemented because the remaining owned `FileStorage` vector is not
the dominant retained memory contributor.

T22 released the avoidable lower-level `HbkContainer` mmap retained by `HbkBook` after open. The
remaining `FileStorage`, TOC/root-discovery and parser traversal lifetimes are bounded enough that
no further production refactor is justified by the current measurements.

No broad pipeline framework, cache, plugin system, tuning knob or compatibility adapter is justified
by the current evidence.
