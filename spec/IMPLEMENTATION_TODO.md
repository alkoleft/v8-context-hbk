# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history: [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md).

Current status: T14 is the first active unchecked task and covers lean Syntax Assistant consumer
export. T13 measured the current implementation and accepted ADR-0003 as the first performance
slice.

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

### [ ] T14. Lean Syntax Assistant consumer export

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
