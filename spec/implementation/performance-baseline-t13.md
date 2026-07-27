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

Syntax export crate (then `hbk-export`, now `hbk-syntax-export`):

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
sink boundary. The in-memory `PlatformContext` remains the full-domain aggregate sink for
parser/tests, while the CLI export command streams record-family events directly into canonical JSON
writers.

The T17 pass used the built debug binary under GNU `time`:

```bash
cargo build -p v8-context-hbk-cli --bin v8-context-hbk
/usr/bin/time -o target/t17-measurements/logs/shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t17-measurements/exports/shcntx-ru
/usr/bin/time -o target/t17-measurements/logs/shcntx-root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/t17-measurements/exports/shcntx-root
```

No fixture-backed T17 command was skipped on this host.

T85 update: `PlatformContext` remains the full in-memory extraction aggregate, but its legacy public
exact lookup-helper API was removed. Accepted lookup behavior now lives in `syntax-helper-search`
provider primitives and the future ADR-0008 resolver boundary.

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

- On the then-current post-T19/pre-T22 baseline, the owned `FileStorage` vector was material but
  not dominant: it was about 35% of retained `HbkBook::open` RSS for `shcntx_ru.hbk`, about 33% for
  `shcntx_root.hbk`, and less than one quarter of the full export peak for both books.
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
streaming export path after T20/T21. Generated exports, GNU `time` logs and the temporary probe were
written under `target/t22-measurements/`. That directory is service data and is not a durable source
of truth.

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
- T22 changed the retained-memory baseline for the `HbkBook::open` path. The `FileStorage` vector
  measured in T20 is now about half of current open-path RSS after the container mmap is released,
  so the T20 "not dominant" percentage should not be reused as a current open-path conclusion.
- Further splitting of TOC/root-discovery or parser traversal lifetimes is not justified by the
  current evidence. A direct or shorter-lived `FileStorage` design requires a post-T22
  remeasurement before any production refactor.

## Post-T23 Update

T23 re-evaluated retained `FileStorage` ownership after the T22 baseline shift. The initial
measurement-only pass wrote raw command outputs, generated exports and the temporary probe under
`target/t23-measurements/`. A user-directed production follow-up then removed retained
`FileStorage` bytes from `HbkBook` and wrote fresh post-change logs under
`target/t23-prod-measurements/`. These directories are service data and are not durable sources of
truth.

The probe measured exact `FileStorage` bytes, fresh-process `HbkBook::open`, repeated page reads
through one `FileStorageReader` and extractor access through `SyntaxHelperReader::extract_into`
with a counting sink. The post-production pass used the built debug CLI and temporary probe under
GNU `time`:

```bash
cargo build -p v8-context-hbk-cli --bin v8-context-hbk
cargo build --manifest-path target/t23-measurements/probe/Cargo.toml
/usr/bin/time -o target/t23-prod-measurements/logs/book-open-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t23-measurements/probe/target/debug/t23-file-storage-probe book-open /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk
/usr/bin/time -o target/t23-prod-measurements/logs/page-read-all-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t23-measurements/probe/target/debug/t23-file-storage-probe page-read-all /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk
/usr/bin/time -o target/t23-prod-measurements/logs/extract-counts-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/t23-measurements/probe/target/debug/t23-file-storage-probe extract-counts /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk
/usr/bin/time -o target/t23-prod-measurements/logs/syntax-helper-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t23-prod-measurements/exports/shcntx-ru
```

Equivalent commands were run for `shcntx_root.hbk`, and both source books were available.

Fresh-process open-path attribution:

| Source | Mode | Exit | Elapsed, s | Current RSS, KiB | VmHWM / peak RSS, KiB | Exact `FileStorage` bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `file-storage-len` | 0 | 0.02 | 80416 | 80416 | 38960718 |
| `shcntx_ru.hbk` | `book-open` | 0 | 5.92 | 33164 | 133376 | 38960718 |
| `shcntx_root.hbk` | `file-storage-len` | 0 | 0.02 | 68012 | 68012 | 32620458 |
| `shcntx_root.hbk` | `book-open` | 0 | 5.68 | 32928 | 120832 | 32620458 |

Repeated page-read and extractor-access results:

| Source | Mode | Exit | Elapsed, s | Current RSS, KiB | VmHWM / peak RSS, KiB | Counts |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `shcntx_ru.hbk` | `page-read-all` | 0 | 9.39 | 98000 | 133120 | 25878 pages, 4 missing entries |
| `shcntx_root.hbk` | `page-read-all` | 0 | 9.06 | 91744 | 120704 | 25878 pages, 4 missing entries |
| `shcntx_ru.hbk` | `extract-counts` | 0 | 17.34 | 73308 | 133248 | 1 global context, 24836 consumer records, 703 diagnostics, 25540 total items |
| `shcntx_root.hbk` | `extract-counts` | 0 | 13.96 | 110328 | 120960 | 1 global context, 24836 consumer records, 703 diagnostics, 25540 total items |

The page-read probe traversed `28736` flat TOC pages per book, skipped `2851` empty TOC paths,
read `25878` available unique pages and counted `4` missing entries. It measured repeated access to
available ZIP entries rather than promoting known missing source pages to a storage refactor driver.

T13-style full `syntax-helper --output` results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes | Records and diagnostics |
| --- | ---: | ---: | ---: | ---: | --- |
| `shcntx_ru.hbk` | 0 | 19.63 | 154504 | 21950926 | 24836 records, 703 diagnostics |
| `shcntx_root.hbk` | 0 | 15.15 | 122240 | 12269994 | 24836 records, 703 diagnostics |

T23 conclusion:

- The exact `FileStorage` entity size is unchanged from T20: about `38048 KiB` for `shcntx_ru.hbk`
  and about `31856 KiB` for `shcntx_root.hbk`.
- The production follow-up removes those bytes from retained `HbkBook` state. Current RSS after
  `book-open` dropped from the initial T23 `71068 KiB` to `33164 KiB` for `shcntx_ru.hbk` and from
  `64820 KiB` to `32928 KiB` for `shcntx_root.hbk`.
- Open-path VmHWM stays in the previous class because `HbkBook::open` still validates the
  `FileStorage` entity body and then drops it. That preserves existing open-time body-validation
  behavior while avoiding long-lived `FileStorage` ownership.
- Repeated page reads now load `FileStorage` into the short-lived `FileStorageReader`; page-read
  peak RSS remains bounded by the open-path high-water class.
- Extractor-count current RSS after `SyntaxHelperReader::extract_into` drops when the reader goes
  out of scope (`73308 KiB` for `shcntx_ru.hbk` in this run), but full `syntax-helper --output`
  peak does not show a material win because export still owns `FileStorage` during extraction and
  writer state overlaps with parsing.
- A direct/seekable block-backed `FileStorage` view remains unjustified by current full-export and
  page-access evidence. T23's accepted runtime change is the narrower path-backed reader lifetime:
  `HbkBook` no longer retains `FileStorage` bytes, and `FileStorageReader` owns them only for its
  read lifetime.

## Post-T24 Update

T24 applied targeted parser, lookup and lean streaming-export optimizations requested before T18.
Raw logs and exports were written under `target/t24-measurements/`; that directory is service data
and is not a durable source of truth.

The final pass used the built debug CLI under GNU `time`:

```bash
cargo build -p v8-context-hbk-cli --bin v8-context-hbk
/usr/bin/time -o target/t24-measurements/logs/syntax-helper-shcntx-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t24-measurements/exports/shcntx-ru
/usr/bin/time -o target/t24-measurements/logs/syntax-helper-shcntx-root.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/t24-measurements/exports/shcntx-root
/usr/bin/time -o target/t24-measurements/logs/syntax-helper-shcntx-root-repeat.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/debug/v8-context-hbk syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/t24-measurements/exports/shcntx-root-repeat
```

T13-style full `syntax-helper --output` results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes by `wc -c` | Records and diagnostics |
| --- | ---: | ---: | ---: | ---: | --- |
| `shcntx_ru.hbk` | 0 | 18.40 | 134528 | 21946830 | 24836 records, 703 diagnostics |
| `shcntx_root.hbk` | 0 | 14.09 | 122108 | 12265898 | 24836 records, 703 diagnostics |
| `shcntx_root.hbk` repeat | 0 | 14.34 | 122112 | 12265898 | 24836 records, 703 diagnostics |

Follow-up release-profile results used `target/release/v8-context-hbk` and wrote raw logs under
`target/t24-release-measurements/`:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes by `wc -c` | Records and diagnostics |
| --- | ---: | ---: | ---: | ---: | --- |
| `shcntx_ru.hbk` | 0 | 3.38 | 151136 | 21946830 | 24836 records, 703 diagnostics |
| `shcntx_root.hbk` | 0 | 2.57 | 119936 | 12265898 | 24836 records, 703 diagnostics |
| `shcntx_root.hbk` repeat | 0 | 2.42 | 119936 | 12265898 | 24836 records, 703 diagnostics |

The release binary size was `3040152` bytes. Release exports were byte-identical to the local T23
production exports and to the release root repeat.

Compared with the local T23 production baseline (`19.63s / 154504 KiB` for `shcntx_ru.hbk` and
`15.15s / 122240 KiB` for `shcntx_root.hbk`), T24 improves the Russian book by about `6.3%`
elapsed time and `12.9%` peak RSS, and keeps the root book in the same memory class with a small
time improvement. The T24 exports are byte-identical to
`target/t23-prod-measurements/exports/shcntx-ru` and
`target/t23-prod-measurements/exports/shcntx-root`; the repeated root export is also
byte-identical.

T24 implementation conclusions:

- Pre-sizing decompressed ZIP entry buffers is accepted for current `FileStorageReader` page reads
  and PackBlock TOC reads.
- `syntax_toc_index` uses `HashMap` because lookup order is not externally observable and output
  order remains traversal-driven.
- The extraction `visited` set stays `BTreeSet`: a `HashSet` replacement was measured and rejected
  because it raised `shcntx_root.hbk` peak RSS into the 170 MiB class.
- Streaming export uses a lean record-detail mode to skip fields omitted by the consumer JSON
  contract for global context, platform type navigation links and enum value links. The
  provenance-rich in-memory `PlatformContext` path remains the default.
- Emptying per-record source/provenance during streaming export was measured and rejected because it
  raised `shcntx_root.hbk` peak RSS; provenance is still built in the domain record and omitted only
  by the consumer JSON adapter.
- A single-pass HTML text-normalization rewrite was measured and rejected because it did not improve
  the final memory class.

## Post-T164 Update

T164 rechecked the `syntax index` path with the current `schema_version: 15` search index, focusing
on HBK reading and order-insensitive index-build lookup structures. Raw command outputs and
generated indexes were written under `target/t164-performance-audit/`; that directory is service
data and is not a durable source of truth.

The pass used the built release CLI under GNU `time`:

```bash
cargo build --release -p v8-context-hbk-cli --bin v8-context-hbk
/usr/bin/time -o target/t164-performance-audit/baseline/logs/index-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/release/v8-context-hbk syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t164-performance-audit/baseline/index-ru/index.sqlite
/usr/bin/time -o target/t164-performance-audit/post/logs/index-ru.time -f 'elapsed_seconds=%e\npeak_rss_kb=%M\nexit_status=%x' target/release/v8-context-hbk syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/t164-performance-audit/post/index-ru/index.sqlite
```

Release `syntax index shcntx_ru.hbk` results:

| Phase | Exit | Elapsed, s | Peak RSS, KiB | DB size | Row inventory |
| --- | ---: | ---: | ---: | ---: | --- |
| baseline | 0 | 21.27 | 282048 | 197M | 25415 documents, 132908 names, 58128 relations, 47156 type refs |
| baseline repeat | 0 | 17.79 | 282176 | 197M | same |
| post-change | 0 | 17.41 | 285696 | 197M | same |
| post-change repeat | 0 | 16.86 | 285764 | 197M | same |

T164 implementation conclusions:

- HBK page and PackBlock ZIP-entry reads now pre-size output buffers from ZIP uncompressed-size
  metadata with a 64 MiB cap, preserving typed errors and avoiding unbounded trust in malformed
  entry size metadata.
- Order-insensitive search-index build maps now use `HashMap` for relation lookup, type-ref target
  lookup, normalized fact insertion and type-template helper maps. Ordered sets remain in place for
  candidate id ordering where output determinism is observable.
- The accepted changes do not alter SQLite schema version, row counts, provider query behavior,
  duplicate-document winner semantics or type-reference gap totals.
- The measured time delta is modest and overlaps normal run-to-run variance; treat this as a narrow
  allocation/lookup cleanup, not as justification for page caches, parallel parsing, storage
  redesign or FTS schema changes.

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
- T20 showed the remaining owned `FileStorage` vector was not dominant enough on the pre-T22
  baseline to justify a broader direct seekable ZIP/storage design.

## Current Implementation Direction

The original T13 direction selected T14 / Variant A first. T14 completed but left peak RSS high, so
T15 / Variant B was implemented next.

T16 attributed the remaining post-T15 memory and selected T17 / Variant C next. T17 implemented
streaming extraction into record-family sinks for the export command path while preserving the
in-memory `syntax-helper-model` full-domain aggregate.

T19 completed the narrow byte-only Variant E slice. T20 evaluated the broader direct seekable
`FileStorage` view and left it unimplemented because the remaining owned `FileStorage` vector was
not the dominant retained memory contributor on the pre-T22 baseline.

T22 released the avoidable lower-level `HbkContainer` mmap retained by `HbkBook` after open. This
changed the current retained-memory split: the T20 FileStorage no-go conclusion remains useful
pre-T22 evidence for the full export peak, but it is stale for current `HbkBook::open` attribution.
The remaining TOC/root-discovery and parser traversal lifetimes are bounded enough that no further
production refactor is justified by the current measurements. T23 remeasured `FileStorage` after the
baseline shift and confirmed that it is about half of current open-path RSS, but only about a
quarter of open-path high-water RSS and full export peak. Repeated page reads and extractor access
do not justify a direct or shorter-lived storage design.

T24 completed the requested targeted parser, lookup and lean streaming-export optimizations. The
accepted changes keep JSON output byte-identical while improving `shcntx_ru.hbk` and preserving the
`shcntx_root.hbk` memory class. Broader parser rewrites, `HashSet` visited tracking and empty-source
streaming records are not justified by the measured regressions.

T149 confirmed that the current query-table parent-identity prepass is not causing a second
query-table page load during record emission in the normal reader flow. Focused loader-count
instrumentation covers the full `extract_with_loader_into` path and proves that the parsed
query-table record from parent-identity discovery is reused. The representative `shcntx_ru.hbk`
debug index rebuild stayed in the current post-T127/T144 performance class, so no broader parser
pipeline or cache mechanism is justified.

No broad pipeline framework, cache, plugin system, tuning knob or compatibility adapter is justified
by the current evidence.

## T174 Snapshot-Materialization Results

T174 is a provider-startup optimization, not a Syntax Assistant HBK extraction
or export change. The downstream analyzer's P5a DHAT run on 2026-07-27 loads
`v8-context/.v8-context/platform-indexes/8.3.27.1859/shcntx_ru.sqlite` into
`HbkFactSnapshot` and attributes `212585620` allocated bytes and `98129653`
bytes live at global heap maximum to `syntax_helper_search` materialization.
`SnapshotMaterializer::type_refs` is the largest direct source: `42720841`
allocated bytes and `25420393` live bytes at that peak.

Three warm release runs of the existing snapshot example measured a `692 ms`
median build, `105592 KiB` peak RSS and `23144545` bytes snapshot accounting
(`7500929` string store, `9034176` node arenas, `6609440` indexes). The source
contains 46,863 type-reference rows.

T174 maps each ordered SQLite row directly into the existing four snapshot
type-reference groups after decoding it, rather than retaining a complete owned
row vector before grouping. Three final warm release runs measured `613 ms`,
`609 ms` and `605 ms` snapshot build time and `78820 KiB`, `78824 KiB` and
`78820 KiB` peak RSS. The final medians are `609 ms` (-12.0%) and `78820 KiB`
(-26772 KiB, -25.35%). Snapshot accounting remains exactly `23144545` bytes,
which confirms that the saved memory was transient materialization storage.

The acceptance gate is met: RSS decreases by more than 10% and build time does
not regress. Query-owner streaming, interner storage, capacity hints,
binary-cache startup integration and borrowed signature text remain independent
hypotheses for separately measured tasks.

The stage-timing accounting changes only in terminology: `group_type_refs` now
includes type-reference SQLite read, decode and grouping, while `read_sql_rows`
excludes that work. Compare post-T174 stage buckets only with measurements that
use the same accounting; total build time and process RSS remain comparable.

## T175 Signature-Line Supportability Result

T175 tested the smaller `split_lines` attribution after T174. A private
borrowed-input experiment was allocation-identical under release DHAT and was
reverted. The accepted source-level deletion in `signatures_by_callable`
selects only the needed non-empty ordinal signature line and passes it directly
to the existing string builder; it no longer expresses an all-lines temporary
vector plus a selected-line clone.

DHAT cannot prove a causal resource gain from this deletion: the
`split_lines` first-frame aggregate remains `5811810` allocated bytes and the
global process peak is unchanged. The task therefore records only
supportability, not a memory or time improvement. Normal provider final runs
are `600`, `601` and `611 ms` (601 ms median) and `79756`, `79884` and
`79752 KiB` (79756 KiB median), with unchanged `23144545` snapshot-accounted
bytes. The downstream five-run fixed workload preserves its zero-finding digest
with a `0.75 s` / `89424 KiB` median, within the 5% no-regression guard.

## T176 Target-Kind Owner-Edge Result

T176 keeps `query_owner_edges` and its ordered `Vec<(String, String)>`, but
constrains it with a target-id subquery over existing document-kind facts. It
does not add a schema/index, cache or reader interface. The first JOIN variant
was rejected against a historic provider observation before the matched H2
protocol was corrected; `EXPLAIN QUERY PLAN` shows the final predicate uses
existing `relations_target_idx`.

The downstream DHAT first-frame total uses the analyzer provider index with
21,304 `owns` rows and decreases from 6,954,078 to 1,012,703 bytes (-85.43%),
while global peak remains effectively unchanged. The distinct 21,613-row
provider release artifact has matched provider medians improving from 659 ms /
80,280 KiB to 608 ms / 79,512 KiB; snapshot accounting stays 23,254,254 bytes
and final query-field/query-parameter/enum-value counts stay 498 / 56 / 3,087.
The sequential matched five-run analyzer workload improves from 0.86 s / 92,500
KiB to 0.80 s / 91,444 KiB with the exact zero-finding digest in every run.
The corresponding 5% ceilings are 691.95 ms / 84,294 KiB and 0.903 s / 97,125
KiB. The earlier T175 0.75 s / 89,424 KiB workload result is historical only and
is not an H2 acceptance comparator; later loaded-host runs are invalid evidence.

## T177 Snapshot-Interner Ownership Result

T177 removes duplicate transient ownership of each interned snapshot string.
The existing builder map owns values while IDs are assigned; after the final
intern, it moves them once by stable `StringId` into the unchanged final string
table before secondary-index sorting. This is a private lifecycle change, not a
new cache format, string interner API or capacity policy.

On the downstream 21,304-row index, direct `SnapshotBuilder::intern` DHAT
allocation decreases from 18,404,610 to 7,756,607 bytes (-57.86%) and global
peak from 69,614,844 to 63,019,028 bytes (-9.47%). The representative provider
cache remains byte-identical at SHA-256
`68e1662ae26518777cd3ac8c352281efa1ac1fb0b2f3f04b606b9017af1b1450`; its
capacity-based heap decreases 23,254,254 to 22,376,046 bytes while logical
payload remains 17,908,362 bytes.

Matched provider median improves from 599 ms / 79,520 KiB to 580 ms / 75,620
KiB. A sequential same-checkout five-run downstream A/B, reverting only the
H3 production code for baseline, improves from 0.83 s / 88,620 KiB to 0.75 s /
84,868 KiB. Every run has the exact zero-finding digest. H4 has no separately
attributable cardinality source, H5 requires a provider-owned startup-policy
proposal, and H8 remains a semantic non-merge rule rather than a performance
candidate.
