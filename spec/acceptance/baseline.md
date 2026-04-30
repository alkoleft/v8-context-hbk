# Acceptance Baseline

This file contains durable acceptance gates and conclusions. Raw run logs and generated output
directories are service data unless promoted here.

## Current Baseline

- Target platform baseline: `8.5.1.1150`.
- T9 Syntax Assistant acceptance passed for `shcntx_ru.hbk` and `shcntx_root.hbk`.
- T10 all-HBK smoke passed for every `*.hbk` file under `/opt/1cv8/x86_64/8.5.1.1150/`.
- T12 workspace split passed with package-level checks and preserved CLI behavior.
- T15 Syntax Assistant performance pass reduced debug-binary peak RSS without wall-clock regression:
  `shcntx_ru.hbk` measured `19.26s / 590988 KiB`, and `shcntx_root.hbk` measured
  `14.62s / 324476 KiB`.
- T17 streaming extraction reduced the debug-binary `shcntx_ru.hbk` export peak to
  `20.46s / 386304 KiB` while preserving export shape, record counts and deterministic JSON output.
  `shcntx_root.hbk` measured `18.15s / 324096 KiB`, still effectively bounded by the lower-level
  open-time peak.
- T19 byte-only container entity reads reduced the remaining `HbkBook::open` VmHWM from
  `383232 KiB` to `131328 KiB` for `shcntx_ru.hbk` and from `321408 KiB` to `119168 KiB` for
  `shcntx_root.hbk`. Full `syntax-helper --output` remained shape/count stable and measured
  `21.19s / 168692 KiB` for `shcntx_ru.hbk` and `16.11s / 144500 KiB` for `shcntx_root.hbk`.
- T20 measured the remaining owned `FileStorage` copy and did not justify a broader direct seekable
  view. The exact retained vector was `38960718` bytes for `shcntx_ru.hbk` and `32620458` bytes for
  `shcntx_root.hbk`, while full `syntax-helper --output` measured `17.68s / 157916 KiB` and
  `13.50s / 139632 KiB` with stable export counts.
- T21 measured retained TOC/root-discovery structures and did not justify a production refactor.
  The largest T21-specific retained structure was public `RootDiscovery` at about 9 MiB, while full
  `syntax-helper --output` measured `19.04s / 157788 KiB` for `shcntx_ru.hbk` and
  `14.33s / 139764 KiB` for `shcntx_root.hbk` with stable export counts.
- T22 released the avoidable `HbkContainer` mmap retained by `HbkBook` after open. Full
  `syntax-helper --output` measured `17.97s / 134656 KiB` for `shcntx_ru.hbk` and
  `13.65s / 122112 KiB` for `shcntx_root.hbk` with byte-identical JSON export compared with the
  pre-change run. T22 also changed the attribution baseline for the retained `FileStorage` vector:
  T20 remains pre-T22 evidence for the broader export peak, but no longer describes the current
  `HbkBook::open` memory split.

## Standard Verification Gates

For implementation tasks, choose the narrowest relevant gate set and run the task-specific
verification from `IMPLEMENTATION_TODO.md`.

Common gates:

```bash
cargo fmt
cargo test --workspace
cargo check -p hbk-container
cargo check -p hbk-book
cargo check -p hbk-docs
cargo check -p syntax-helper-model
cargo check -p syntax-helper-extract
cargo check -p hbk-export
cargo check -p v8-context-hbk-cli
git diff --check
```

## CLI Smoke Commands

Run when the target-platform fixtures exist:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --path "$(cat tests/fixtures/known-pages/fmtdui_ru.page)"
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/ru
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/context/en
```

Negative CLI smoke:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect target/does-not-exist.hbk
```

The negative smoke must return non-zero and produce a readable diagnostic.

## T9 Durable Conclusions

Syntax Assistant extraction was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

Both commands exited successfully. Each source book produced:

- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values

The `_root` source exported as locale `en`.

Known parser gaps from that pass:

- 703 `UNKNOWN_PAGE_CLASS` diagnostics in each Syntax Assistant source.
- Most known gaps were global context event pages and common table color palette pages.

These gaps make the current export useful for integration experiments, but not a final stable
platform API contract.

## T10 Durable Conclusions

All-HBK smoke covered every `*.hbk` file under `/opt/1cv8/x86_64/8.5.1.1150/`.

Results:

- 116 files discovered.
- 116 `inspect` successes.
- 116 `toc --format json` successes.
- No fatal failures.
- No unsupported structures reported by the generic smoke commands.

## T15 Durable Conclusions

Post-T15 Syntax Assistant extraction was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

Both commands exited successfully through the built debug binary. Each source book produced:

- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

Resource results:

| Source | Elapsed, s | Peak RSS, KiB |
| --- | ---: | ---: |
| `shcntx_ru.hbk` | 19.26 | 590988 |
| `shcntx_root.hbk` | 14.62 | 324476 |

The T15 pass keeps the canonical export shape from FR-EXPORT-001: consumer record-family files do
not expose HBK navigation or per-record provenance, while `diagnostics.json` keeps parser source
context. The remaining `shcntx_ru.hbk` peak remains above 500 MiB and requires T16 attribution before
the next optimization slice is selected.

## T16 Durable Conclusions

Post-T15 Syntax Assistant memory attribution was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

Both source books were available and no fixture-backed command was skipped.

Actual debug CLI results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 18.64 | 588892 | 21950926 |
| `shcntx_root.hbk` | 0 | 14.07 | 324352 | 12269994 |

Attribution probe conclusions:

- `extract` reaches the same peak class as the full export path for `shcntx_ru.hbk`.
- JSON export adds no material high-water RSS after extraction.
- `HbkBook::open` still has a lower-level container/FileStorage opening spike, but that is not the
  next slice most likely to reduce the current `shcntx_ru.hbk` peak.

T16 selects Variant C for T17: streaming extraction into record-family sinks for the export command
path while keeping the in-memory model as a library lookup use case.

## T17 Durable Conclusions

Variant C streaming extraction was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

Both source books were available and no fixture-backed T17 command was skipped.

Actual debug CLI results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 20.46 | 386304 | 21946830 |
| `shcntx_root.hbk` | 0 | 18.15 | 324096 | 12265898 |

Each source book produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

The CLI export path now streams typed record-family events directly into canonical JSON writers, so
it no longer accumulates the full `PlatformContext` before export. The in-memory
`PlatformContext` path remains available for lookup helpers and uses the same extraction core.

The canonical export shape from FR-EXPORT-001 is preserved: consumer record-family files do not
expose HBK navigation or per-record provenance, `global-contexts.json` is not produced, and
`diagnostics.json` keeps parser source context. Two independent `shcntx_ru.hbk` exports were
compared byte-for-byte across all JSON files to verify deterministic record and diagnostic order.

## T19 Durable Conclusions

The first Variant E byte-only entity read slice was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`

All source books were available and no fixture-backed T19 command was skipped.

Open-only `HbkBook::open` probe results:

| Source | Current RSS before, KiB | Current RSS after, KiB | VmHWM before, KiB | VmHWM after, KiB |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 110868 | 108332 | 383232 | 131328 |
| `shcntx_root.hbk` | 98412 | 95888 | 321408 | 119168 |

Full debug CLI results:

| Source | Phase | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | before | 0 | 28.40 | 386048 | 21946830 |
| `shcntx_ru.hbk` | after | 0 | 21.19 | 168692 | 21946830 |
| `shcntx_root.hbk` | before | 0 | 32.36 | 324352 | 12265898 |
| `shcntx_root.hbk` | after | 0 | 16.11 | 144500 | 12265898 |

Each post-T19 source book still produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

Small-book smoke passed for `inspect` and `toc --format json` on `fmtdui_root.hbk` and
`fmtdui_ru.hbk`; TOC output parsed as JSON and inspect output still included `PackBlock`,
`FileStorage` and `Book`.

The byte-only path removed the majority of the open-time high-water mark, so acceptance does not
require a follow-up seekable direct `FileStorage` view from T19.

## T20 Durable Conclusions

The direct seekable `FileStorage` view evaluation was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`

All source books were available and no fixture-backed T20 command was skipped.

Fresh-process attribution results:

| Source | Mode | Exit | Elapsed, s | Current RSS, KiB | VmHWM, KiB | Exact `FileStorage` bytes |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `container-open` | 0 | 0.00 | 2800 | 2800 | n/a |
| `shcntx_ru.hbk` | `file-storage-copy` | 0 | 0.02 | 78740 | 78740 | 38960718 |
| `shcntx_ru.hbk` | `book-open` | 0 | 5.19 | 108324 | 131712 | 38960718 |
| `shcntx_root.hbk` | `container-open` | 0 | 0.00 | 2672 | 2672 | n/a |
| `shcntx_root.hbk` | `file-storage-copy` | 0 | 0.02 | 66468 | 66468 | 32620458 |
| `shcntx_root.hbk` | `book-open` | 0 | 5.26 | 95884 | 119296 | 32620458 |

Full debug CLI results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 17.68 | 157916 | 21950926 |
| `shcntx_root.hbk` | 0 | 13.50 | 139632 | 12269994 |

Each source book still produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

Small-book smoke passed for `inspect`, `toc --format json` and `page` on `fmtdui_root.hbk` and
`fmtdui_ru.hbk`; TOC output parsed as JSON and page output was non-empty.

On the then-current post-T19/pre-T22 baseline, the owned `FileStorage` vector was material but not
dominant. It accounted for about one third of retained `HbkBook::open` RSS and less than one
quarter of the full Syntax Assistant export peak on both measured books. A direct seekable
`FileStorage` view was not justified by that T20 evidence.

## T21 Durable Conclusions

TOC/root-discovery retained-memory attribution was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

All source books were available and no fixture-backed T21 command was skipped.

Fresh-process attribution results:

| Source | Mode | Exit | Current RSS, KiB | VmHWM / peak RSS, KiB | Retained estimate, bytes |
| --- | --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `Toc` tree | 0 | 109864 | 133376 | 8367325 |
| `shcntx_ru.hbk` | retained `flat_pages` metadata | 0 | 109772 | 133248 | 2139400 |
| `shcntx_ru.hbk` | public `RootDiscovery` | 0 | 147000 | 147000 | 9177088 |
| `shcntx_ru.hbk` | `syntax_toc_index` shape | 0 | 110276 | 133120 | 5149766 |
| `shcntx_root.hbk` | `Toc` tree | 0 | 97420 | 120704 | 8332291 |
| `shcntx_root.hbk` | retained `flat_pages` metadata | 0 | 97320 | 120704 | 2139400 |
| `shcntx_root.hbk` | public `RootDiscovery` | 0 | 139520 | 139520 | 9257408 |
| `shcntx_root.hbk` | `syntax_toc_index` shape | 0 | 97808 | 120704 | 5132816 |

Both source books had 28736 TOC pages. Public root discovery found 10 roots, retained 28736 catalog
pages and produced 703 diagnostics for each source book. The `syntax_toc_index` shape contained
25883 entries for each source book.

Full debug CLI results:

| Source | Exit | Elapsed, s | Peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 19.04 | 157788 | 21950926 |
| `shcntx_root.hbk` | 0 | 14.33 | 139764 | 12269994 |

Each source book still produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

The measured retained TOC/root-discovery structures are bounded and do not justify a production
refactor. The largest T21-specific structure is the public `RootDiscovery` graph at about 9 MiB,
under 7% of the full Syntax Assistant export peak. The required public `Toc` tree is about 8 MiB,
the private traversal-index shape is about 5 MiB, and retained flat-page metadata is about 2 MiB.
No runtime code change was made.

## T22 Durable Conclusions

Lower-level book-state retention was validated against:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`

All source books were available and no fixture-backed T22 command was skipped.

Fresh-process attribution results:

| Source | Mode | Before RSS, KiB | After RSS, KiB | Before VmHWM, KiB | After VmHWM, KiB |
| --- | --- | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | `book-open` | 109748 | 70936 | 132992 | 132864 |
| `shcntx_root.hbk` | `book-open` | 97264 | 64700 | 120448 | 120448 |
| `shcntx_ru.hbk` | `root-discovery` | 146796 | 108016 | 146796 | 132992 |
| `shcntx_root.hbk` | `root-discovery` | 139436 | 106868 | 139436 | 120448 |

Full debug CLI results:

| Source | Exit | Before elapsed, s | After elapsed, s | Before peak RSS, KiB | After peak RSS, KiB | Export bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `shcntx_ru.hbk` | 0 | 19.02 | 17.97 | 168800 | 134656 | 21950926 |
| `shcntx_root.hbk` | 0 | 14.79 | 13.65 | 140624 | 122112 | 12269994 |

Each source book still produced:

- 1 global context
- 500 global methods
- 101 global properties
- 2533 platform types
- 6702 type methods
- 10732 type properties
- 445 constructors
- 713 enums
- 3110 enum values
- 703 `UNKNOWN_PAGE_CLASS` diagnostics

The pre-change and post-change export directories were byte-identical for both source books. The
only production refactor justified by T22 evidence was releasing `HbkContainer` from `HbkBook` after
book metadata, TOC and `FileStorage` bytes are extracted.

This baseline shift invalidates the T20 percentage claim for the current `HbkBook::open` path: the
same retained `FileStorage` vector is now about half of current open-path RSS after the container
mmap is released. The T20 no-go decision remains pre-T22 evidence against a broad seekable
`FileStorage` change for the full export peak; it should not be reused as current open-path
attribution without a post-T22 measurement pass.

## First Delivery Success Metrics

The project is successful for the first delivery when:

- the small real HBK smoke pair opens and exposes expected core entities;
- all-HBK smoke covers every target-platform `*.hbk` file;
- `shcntx_ru.hbk` and `shcntx_root.hbk` extraction returns non-empty records for all top-level model
  families;
- `_root` exports as locale `en`;
- parser warnings and unresolved pages are counted and source-linked;
- every specialized parser has at least one representative fixture;
- downstream tooling can consume canonical JSON without reading HBK directly;
- stable API/export commitments remain deferred until parser evidence and consumer feedback justify
  them.
