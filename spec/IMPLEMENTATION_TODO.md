# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history: [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md).

Current status: T18 is the first active unchecked task. T17 implemented Variant C, streaming
extraction into record-family sinks for the export command path, while preserving the in-memory
lookup model. T19 was explicitly reprioritized ahead of T18 and completed the post-T17 lower-level
HBK open-time memory slice. T18 remains the next active query-CLI task.

## Loop Rule

- Take the first unchecked task.
- If there is no unchecked task, add one before implementing new scope.
- Every new task must reference the relevant requirement, UAT, acceptance, implementation spec or
  ADR IDs from `spec/`.
- Keep scope limited to that task and its direct verification.
- Mark a task complete only after its listed verification passes.
- If a task cannot be completed, leave it unchecked and record the blocker in the task notes or final
  response.
- Do not start the next task in the same run unless the prompt explicitly asks for multiple tasks.
- Before committing, stage only files changed for the current task and verify
  `git diff --cached --name-only`.
- Do not create empty commits.

### [x] T13. Performance/resource baseline and implementation hypotheses

Depends on: T12.

Spec refs:

- NFR-PERF-001
- NFR-TEST-001
- `spec/acceptance/baseline.md`
- `spec/implementation/components.md`
- `spec/implementation/performance-variants.md`

Scope:

- Treat minimum resource consumption and high throughput as first-class non-functional requirements.
- Measure the current implementation before refactoring: wall-clock time, peak RSS or equivalent
  memory metric, command exit status and output counts for representative commands.
- Cover at least:
  - `inspect` and `toc --format json` on the small `fmtdui_root.hbk` / `fmtdui_ru.hbk` smoke pair
    when the fixtures exist;
  - `syntax-helper --output` on `shcntx_ru.hbk` and `shcntx_root.hbk` when the fixtures exist;
  - the all-HBK smoke path or a documented equivalent command set for every target-platform
    `*.hbk` file when the platform directory exists.
- Inspect the current resource hotspots in `hbk-container`, `hbk-book`, `syntax-helper-extract` and
  `hbk-export`, especially whole-entity reads, `FileStorage` ZIP buffering, page `BTreeMap`
  accumulation, domain model accumulation and JSON serialization.
- Evaluate implementation hypotheses without committing to an architecture prematurely:
  - narrower streaming or batched reads across `FileStorage` and page loading;
  - keeping `memmap2` versus moving container access toward `Read + Seek`;
  - bounded parallel Syntax Assistant page parsing with deterministic diagnostics and output order;
  - streaming record-family JSON export if serialization is a measured bottleneck.
- Use `spec/implementation/performance-variants.md` as the candidate option set and update it if the
  measurements reject or reorder the saved variants.
- Do not add broad pipeline frameworks, caches, plugin systems, tuning knobs or compatibility
  adapters as part of this task.
- If the measurements make the tradeoff non-trivial, record the chosen implementation direction as a
  follow-up decision before code refactoring.

Expected artifacts:

- Checked-in performance/resource baseline and hypothesis note, or an updated specification section
  with exact commands, measurements, observations and recommended next task.
- Follow-up T14+ task for the selected implementation slice, if a refactor is justified.

Verification:

- Baseline/hypothesis artifact references exact commands and fixture availability.
- Artifact records whether each fixture-backed command was run or skipped.
- Artifact identifies the first implementation slice to try next, or explicitly records that no
  refactor is currently justified.
- `cargo test --workspace`
- `git diff --check`

### [x] T14. Lean Syntax Assistant consumer export

Depends on: T13.

Spec refs:

- FR-EXPORT-001
- UC-SH-001
- NFR-PERF-001
- NFR-DIAG-001
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- `spec/implementation/components.md`
- `spec/implementation/performance-variants.md`

Scope:

- Implement Variant A from `spec/implementation/performance-variants.md` unless T13 records a
  blocker or a better first slice.
- Introduce lean consumer export DTOs in `hbk-export` derived from the provenance-rich domain model.
- Stop writing `global-contexts.json` as a consumer export file.
- Remove per-record `source` and book/TOC/page provenance from consumer record-family files.
- Remove duplicate navigation-link fields from consumer records:
  `method_links`, `constructor_links` and `value_links`.
- Keep source context in `diagnostics.json` for parser maintenance.
- Write compact JSON through a writer instead of materializing pretty JSON bytes.
- Update README examples if they describe the old export shape.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- `git diff --check`

Completion notes:

- Post-T14 T13-style measurements used the built debug binary under GNU `time`.
- `shcntx_ru.hbk`: exit `0`, elapsed `20.20s`, peak RSS `752392 KiB`, exported JSON
  `21946830` bytes.
- `shcntx_root.hbk`: exit `0`, elapsed `15.82s`, peak RSS `518844 KiB`, exported JSON
  `12265898` bytes.
- Output size decreased, but peak RSS stayed high; ADR-0003 therefore promotes Variant B as the next
  slice.

### [x] T15. Lazy or batched Syntax Assistant page loading

Depends on: T14.

Spec refs:

- NFR-PERF-001
- NFR-DIAG-001
- NFR-TEST-001
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- `spec/acceptance/baseline.md`
- `spec/implementation/components.md`
- `spec/implementation/performance-variants.md`
- ADR-0003

Scope:

- Implement Variant B from `spec/implementation/performance-variants.md`.
- Replace extraction-wide `read_pages(...)->BTreeMap<String, String>` usage with a bounded page
  loader that reads only the current page or a small deterministic batch.
- Keep page traversal order driven by TOC/root discovery.
- Keep missing-page and invalid-UTF-8 diagnostics at the book/page boundary.
- Preserve deterministic diagnostics and deterministic JSON record order.
- Do not add caches, broad pipeline frameworks, plugin systems, parallel parsing or tuning knobs as
  part of this task.
- Re-run the T13-style `syntax-helper` measurements for `shcntx_ru.hbk` and `shcntx_root.hbk` after
  the change and compare them with the T14 completion notes.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- T13-style `syntax-helper` measurements for `shcntx_ru.hbk` and `shcntx_root.hbk`
- `git diff --check`

Completion notes:

- Implemented a reusable `FileStorageReader` so `syntax-helper` keeps one ZIP reader open and reads
  the current page on demand instead of preloading extraction pages into a `BTreeMap`.
- `parse_extraction_pages` now consumes traversal metadata as it parses so `RootDiscovery` does not
  stay fully resident beside the growing `PlatformContext`.
- Avoided extra per-page `raw_html` and section-string copies in the Syntax Assistant parser.
- Post-T15 T13-style measurements used the built debug binary under GNU `time`.
- `shcntx_ru.hbk`: exit `0`, elapsed `19.26s`, peak RSS `590988 KiB`.
- `shcntx_root.hbk`: exit `0`, elapsed `14.62s`, peak RSS `324476 KiB`.
- Compared with T14 completion notes, wall-clock time improved slightly and peak RSS decreased, but
  the Russian Syntax Assistant export remains above 500 MiB; the next task must attribute the
  remaining memory before selecting Variant C or Variant E.

### [x] T16. Attribute post-T15 Syntax Assistant memory

Depends on: T15.

Spec refs:

- NFR-PERF-001
- NFR-TEST-001
- `spec/implementation/performance-variants.md`
- `spec/implementation/performance-baseline-t13.md`

Scope:

- Measure or instrument the post-T15 `syntax-helper` path enough to identify the remaining dominant
  memory contributors:
  - full `PlatformContext` accumulation before export;
  - export adapter allocation during JSON writing;
  - whole `FileStorage` ownership and container/entity copies;
  - parser temporary allocation or allocator retention.
- Keep raw measurement logs and profiler outputs as service data under `target/`.
- Use the evidence to choose the next implementation slice:
  - Variant C if the full model/export command path is the dominant remaining memory cost;
  - Variant E if `FileStorage` ownership or container/entity copies are the limiting cost;
  - no refactor if the remaining peak cannot be reduced without a measured speed or complexity
    tradeoff.
- Do not implement streaming extraction sinks, lower-level container access changes, parallel
  parsing, caches or tuning knobs in this task.

Expected artifacts:

- Ledger notes or a small checked-in performance note with exact commands, measurements and the
  chosen next slice.
- Update T17 with the measured selected variant, or mark it blocked/not-needed if T16 concludes that
  no immediate refactor is justified.

Verification:

- Attribution artifact references exact commands and fixture availability.
- Artifact records whether `shcntx_ru.hbk` and `shcntx_root.hbk` were run or skipped.
- Artifact explicitly chooses Variant C, Variant E or no immediate refactor.
- `cargo test --workspace`
- `git diff --check`

Completion notes:

- Raw command outputs, generated exports and the temporary attribution probe were written under
  `target/t16-memory-attribution-20260430/` as service data.
- Both `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
  `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` existed and were run; no T16 fixture-backed command
  was skipped.
- Current debug CLI measurements:
  - `shcntx_ru.hbk`: exit `0`, elapsed `18.64s`, peak RSS `588892 KiB`, export bytes `21950926`.
  - `shcntx_root.hbk`: exit `0`, elapsed `14.07s`, peak RSS `324352 KiB`, export bytes `12269994`.
- Probe results showed `extract` reaches the same peak class as full `export`, while export adapter
  allocation adds no material high-water RSS after extraction.
- `HbkBook::open` still has a measured lower-level container/FileStorage opening spike, but Variant E
  alone would not reduce the current `shcntx_ru.hbk` extraction peak.
- T16 selects Variant C for T17.

### [x] T17. Implement selected post-T16 memory optimization

Depends on: T16.

Spec refs:

- NFR-PERF-001
- NFR-DIAG-001
- NFR-TEST-001
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- `spec/implementation/performance-variants.md`
- `spec/implementation/performance-baseline-t13.md`
- T16 attribution artifact

Scope:

- Implement Variant C selected by T16: streaming extraction into record-family sinks while preserving
  `syntax-helper-model` lookup helpers as the in-memory library use case.
- Do not implement Variant E container/`FileStorage` access changes in this task; T16 leaves them as
  a later candidate only if they remain limiting after Variant C.
- Preserve deterministic diagnostics and deterministic JSON record order.
- Preserve FR-EXPORT-001 consumer export shape unless T16 explicitly records a required export
  contract change.
- Do not add caches, broad pipeline frameworks, plugin systems, parallel parsing or tuning knobs as
  part of this task.
- Re-run the T13-style `syntax-helper` measurements for `shcntx_ru.hbk` and `shcntx_root.hbk` after
  the change and compare them with the T15 completion notes.

Expected artifacts:

- Code changes for Variant C, the T16-selected optimization slice.
- Completion notes with exact before/after elapsed time, peak RSS and output counts for both
  Syntax Assistant books.
- Spec or ADR update only if the implementation changes a durable boundary, public contract,
  variant ordering or source-of-truth decision.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-001
- UAT-SH-002
- UAT-SH-003
- T13-style `syntax-helper` measurements for `shcntx_ru.hbk` and `shcntx_root.hbk`
- `git diff --check`

Completion notes:

- Implemented Variant C through a shared `SyntaxHelperSink` extraction core. The existing
  `SyntaxHelperReader::extract()` path still builds `PlatformContext` for in-memory lookup helpers,
  while the CLI export path streams typed record-family events directly into JSON writers.
- FR-EXPORT-001 shape was preserved: the CLI still writes 10 files, does not restore
  `global-contexts.json`, and keeps parser provenance only in `diagnostics.json`.
- Determinism was checked by exporting `shcntx_ru.hbk` twice under
  `target/t17-determinism/` and comparing all JSON files byte-for-byte.
- Post-T17 T13-style measurements used the built debug binary under GNU `time`.
- `shcntx_ru.hbk`: exit `0`, elapsed `20.46s`, peak RSS `386304 KiB`, export bytes `21946830`.
- `shcntx_root.hbk`: exit `0`, elapsed `18.15s`, peak RSS `324096 KiB`, export bytes `12265898`.
- Each source book produced the same counts as the prior accepted export: 1 global context, 500
  global methods, 101 global properties, 2533 platform types, 6702 type methods, 10732 type
  properties, 445 constructors, 713 enums, 3110 enum values and 703 diagnostics.
- Compared with T15, `shcntx_ru.hbk` peak RSS decreased from `590988 KiB` to `386304 KiB`.
  Wall-clock remains a noisy debug-build measurement and was not the optimized metric for this
  memory slice. `shcntx_root.hbk` remained effectively bounded by the lower-level open-time peak.

### [ ] T18. Design and implement the separate Syntax Assistant query CLI first slice

Depends on: T17 unless this task is explicitly reprioritized.

Spec refs:

- FR-SH-SEARCH-001
- FR-SH-SEARCH-002
- NFR-QUERY-001
- UC-SH-003
- UC-SH-004
- UAT-SH-004
- UAT-SH-005
- UAT-SH-006
- ADR-0004
- `spec/source-evidence.md`
- `spec/implementation/syntax-helper-query-cli.md`

Scope:

- Confirm or revise ADR-0004 before coding. If the accepted binary name, crate split or index
  artifact differs from the draft, update ADR-0004 and the implementation spec first.
- Implement the first deterministic local search slice before semantic search:
  - build a local SQLite/FTS5 index from the current canonical Syntax Assistant export directory;
  - exact lookup by primary name and alias;
  - exact owner/member lookup;
  - keyword search over names, aliases, signatures, type references and descriptions;
  - relationship traversal over owner/member and type-reference edges stored in an edge table.
- Keep query commands on a prebuilt local export or index. Do not parse `shcntx_*.hbk` in query
  commands.
- Keep the lean consumer export shape from FR-EXPORT-001. If search needs structured links or page
  provenance, add a search-specific index/maintenance artifact instead of putting those fields back
  into consumer record-family files.
- Do not implement semantic search, embedding providers, network calls, graph database integration,
  caches, server mode, MCP or UI in this first slice.
- Measure query latency against NFR-QUERY-001 on the Russian Syntax Assistant data set.

Expected artifacts:

- Search/index library code and separate query CLI surface.
- Rebuildable SQLite index artifact generated by UAT, not committed to source.
- README usage for the implemented query CLI only after the command exists.
- Completion notes with query measurements and any relationship-quality gaps.
- Follow-up task for structured "see also" link extraction if deterministic relationships are not
  sufficient for the SKD-filter UAT path.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- UAT-SH-004
- UAT-SH-005
- UAT-SH-006
- NFR-QUERY-001 measurement notes for exact lookup, keyword search and relationship search
- `git diff --check`

### [x] T19. Reduce HBK open-time FileStorage memory spike

Depends on: T17. Scheduled after T18 unless memory footprint is explicitly reprioritized.

Spec refs:

- NFR-PERF-001
- NFR-TEST-001
- `spec/acceptance/baseline.md`
- `spec/implementation/components.md`
- `spec/implementation/performance-variants.md`
- `spec/implementation/performance-baseline-t13.md`

Scope:

- Implement the first measured Variant E slice for the lower-level `HbkBook::open` high-water mark
  that remains after T17.
- Attribute `HbkBook::open` current RSS and VmHWM before and after the change for
  `shcntx_ru.hbk` and `shcntx_root.hbk`.
- Add a byte-only HBK block/entity read path for callers that need only entity bytes, avoiding
  per-byte `source_offsets` allocation on large entities.
- Keep offset-aware block reads available only for diagnostics or validation paths that genuinely
  need source offsets.
- Prefer the smallest internal `hbk-container` / `hbk-book` change first. Do not switch ZIP crates,
  introduce a broad seekable block-backed `FileStorage` reader, add caches or expose tuning knobs
  unless the byte-only path measurements show that the retained/open-time peak is still dominated by
  unavoidable `FileStorage` copying.
- Preserve existing public CLI behavior, Syntax Helper export shape and deterministic output order.
- If a seekable direct `FileStorage` view over mmap/chained blocks becomes necessary, record it as a
  follow-up task or ADR-backed design before implementation.

Expected artifacts:

- Code changes limited to the container/book access model and directly affected tests.
- Completion notes with before/after `HbkBook::open` current RSS, VmHWM, full `syntax-helper`
  elapsed time, peak RSS and export counts for both Syntax Assistant books.
- Updated performance baseline and acceptance notes when the measured open-time peak changes.
- Follow-up task if byte-only entity reads do not materially reduce the remaining peak.

Verification:

- `cargo fmt`
- `cargo test --workspace`
- `cargo clippy` for changed crates, or a documented workspace-clippy blocker if unrelated lints
  remain outside the task scope
- `HbkBook::open` RSS/VmHWM probe for `shcntx_ru.hbk` and `shcntx_root.hbk`
- T13-style `syntax-helper` measurements for `shcntx_ru.hbk` and `shcntx_root.hbk`
- Small-book smoke for `inspect` and `toc --format json` on `fmtdui_root.hbk` / `fmtdui_ru.hbk`
  when those fixtures exist
- `git diff --check`

Completion notes:

- Implemented the narrow Variant E slice in `hbk-container`: ordinary entity-body reads now use a
  byte-only block-content path, while descriptor parsing still uses the offset-aware path needed for
  descriptor diagnostics.
- No CLI contract, Syntax Helper export shape or deterministic record order changed.
- Raw command outputs, generated exports and the temporary open probe were written under
  `target/t19-measurements/` as service data.
- Both `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` and
  `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` existed and were run; no T19 fixture-backed command
  was skipped.
- `HbkBook::open` probe results:
  - `shcntx_ru.hbk`: current RSS `110868 -> 108332 KiB`, VmHWM `383232 -> 131328 KiB`.
  - `shcntx_root.hbk`: current RSS `98412 -> 95888 KiB`, VmHWM `321408 -> 119168 KiB`.
- T13-style full `syntax-helper --output` results:
  - `shcntx_ru.hbk`: exit `0`, elapsed `28.40s -> 21.19s`, peak RSS
    `386048 -> 168692 KiB`, export bytes `21946830 -> 21946830`.
  - `shcntx_root.hbk`: exit `0`, elapsed `32.36s -> 16.11s`, peak RSS
    `324352 -> 144500 KiB`, export bytes `12265898 -> 12265898`.
- Each post-T19 source book still produced 1 global context, 500 global methods, 101 global
  properties, 2533 platform types, 6702 type methods, 10732 type properties, 445 constructors,
  713 enums, 3110 enum values and 703 diagnostics.
- Small-book smoke passed for `inspect` and `toc --format json` on `fmtdui_root.hbk` and
  `fmtdui_ru.hbk`.
- The byte-only path removed the majority of the remaining open-time high-water mark, so no
  follow-up task for a seekable direct `FileStorage` view is added from T19.
