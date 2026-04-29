# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history: [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md).

Current status: T16 is the first active unchecked task and covers post-T15 memory attribution before
choosing the next optimization slice. T15 completed Variant B and reduced peak RSS without a
wall-clock regression, but `shcntx_ru.hbk` still peaks above the desired workstation budget.

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

### [ ] T16. Attribute post-T15 Syntax Assistant memory

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

### [ ] T17. Implement selected post-T16 memory optimization

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

- Implement only the next optimization slice selected by T16 evidence.
- If T16 selects Variant C, implement streaming extraction into record-family sinks while preserving
  `syntax-helper-model` lookup helpers as the in-memory library use case.
- If T16 selects Variant E, change only the container/`FileStorage` access model needed to remove
  measured whole-entity or ZIP setup memory pressure.
- If T16 selects no immediate refactor, do not implement this task; record the T16 conclusion in the
  completion notes or leave this task blocked with the reason.
- Preserve deterministic diagnostics and deterministic JSON record order.
- Preserve FR-EXPORT-001 consumer export shape unless T16 explicitly records a required export
  contract change.
- Do not add caches, broad pipeline frameworks, plugin systems, parallel parsing or tuning knobs as
  part of this task.
- Re-run the T13-style `syntax-helper` measurements for `shcntx_ru.hbk` and `shcntx_root.hbk` after
  the change and compare them with the T15 completion notes.

Expected artifacts:

- Code changes for the T16-selected optimization slice.
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
