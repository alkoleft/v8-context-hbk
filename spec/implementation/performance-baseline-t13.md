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

Variant C is the selected T17 slice after T16.

- Streaming extraction into record-family sinks crosses model and export boundaries, so it was
  deferred until Variant A and B completed.
- T16 showed full `PlatformContext` accumulation and the export-oriented command shape are the
  dominant remaining `shcntx_ru.hbk` peak after page loading was bounded.

Variant E is not first.

- `memmap2` remains the simplest low-copy container-open strategy.
- The whole `FileStorage` copy and per-byte `source_offsets` temporary allocation matter, but T16
  showed they are not the next slice most likely to reduce the current `shcntx_ru.hbk` peak.

## Current Implementation Direction

The original T13 direction selected T14 / Variant A first. T14 completed but left peak RSS high, so
T15 / Variant B was implemented next.

T16 attributed the remaining post-T15 memory and selected T17 / Variant C next: streaming extraction
into record-family sinks for the export command path while preserving the in-memory
`syntax-helper-model` lookup use case.

No broad pipeline framework, cache, plugin system, tuning knob or compatibility adapter is justified
by the current evidence.
