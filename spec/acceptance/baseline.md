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
