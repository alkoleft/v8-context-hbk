# v8-context-hbk Implementation TODO

Source of truth: [HBK components requirements and implementation plan](../docs/hbk-components-requirements-plan.md).

Loop rule:

- Take the first unchecked task.
- Keep scope limited to that task and its direct verification.
- Mark a task complete only after its listed verification passes.
- If a task cannot be completed, leave it unchecked and record the blocker in the task notes or final response.
- Do not start the next task in the same run unless the prompt explicitly asks for multiple tasks.
- Before committing, stage only files changed for the current task and verify `git diff --cached --name-only`.
- Do not create empty commits.

## Tasks

### [x] T0. Baseline repository shape

Scope:

- Keep `README.md` aligned with the current baseline files and reference projects.
- Keep `docs/hbk-components-requirements-plan.md` as the planning source of truth until a later ADR/spec split is needed.
- Keep the minimal `cargo test` baseline passing.

Expected artifacts:

- Updated docs only if the live repository state differs from the baseline.

Verification:

- `cargo test`
- `git diff --check`

Status:

- Completed by planning baseline commit `fc2b3d1`.

### [x] T1. Container reader and inspect command

Depends on: T0.

Scope:

- Add library crate modules under `src/lib.rs`.
- Implement typed container errors with source path/entity context.
- Implement HBK header, descriptor and block parsing.
- Implement entity enumeration and byte reads.
- Add `inspect` CLI through `clap`.
- Add unit fixtures for binary parsing.
- Add real-file smoke checks for `fmtdui_root.hbk` and `fmtdui_ru.hbk` that are ignored by default or gated by an explicit environment variable.

Expected artifacts:

- Rust container module.
- CLI `inspect` command.
- Unit tests and optional real-platform smoke test.

Verification:

- `cargo test`
- If `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk` exists: `cargo run -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk`
- If `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists: `cargo run -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`
- If the files are absent: document that the real-platform smoke was skipped because the platform fixtures are unavailable.
- `git diff --check`

### [x] T2. Book, ZIP storage and TOC reader

Depends on: T1.

Scope:

- Implement `HbkBook` on top of `HbkContainer`.
- Inflate `PackBlock`.
- Open `FileStorage` as ZIP.
- Parse `Book` metadata.
- Implement locale inference.
- Implement TOC tree and lookup APIs.
- Add `toc` and `page` CLI commands.
- Add committed deterministic known-page path fixtures for `fmtdui_root.hbk` and `fmtdui_ru.hbk` so page smoke verification is reproducible.

Expected artifacts:

- Book and TOC modules.
- CLI `toc` and `page` commands.
- Tests for metadata, locale, TOC traversal and page access.

Verification:

- `cargo test`
- If `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists: `cargo run -- toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json`
- If `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists: `cargo run -- page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --path "<committed-known-ru-page>"`
- If `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk` exists: `cargo run -- page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk --path "<committed-known-root-page>"`
- If the files are absent: document that real-platform TOC/page smoke was skipped because the platform fixtures are unavailable.
- `git diff --check`

### [x] T3. Documentation page parser

Depends on: T2.

Scope:

- Implement HTML loading and parsing abstraction.
- Extract page title and normalized text preview.
- Implement deterministic link normalization.
- Add diagnostics for unresolved links.
- Add fixture tests for representative pages.

Expected artifacts:

- Documentation page module.
- Fixture tests for normalized text and links.

Verification:

- `cargo test`
- `git diff --check`

### [x] T4. Syntax Assistant fixture corpus

Depends on: T2, T3.

Scope:

- Inspect representative pages from `shcntx_ru.hbk` and `shcntx_root.hbk`.
- Select the minimal committed fixture set for root/catalog pages and every specialized parser kind.
- Add a fixture manifest with source HBK file, HTML path, page title, parser kind and reason for inclusion.
- Copy only minimal real HTML fragments needed for parser behavior tests.

Expected artifacts:

- Syntax Assistant fixture manifest.
- Minimal real HTML fixtures for root/catalog and specialized parsers.

Verification:

- `cargo test`
- Fixture manifest covers global context, global method, global property, object/type, object method, object property, constructor, enum, enum value and root/catalog pages.
- `git diff --check`

Status:

- Completed with a curated `tests/fixtures/syntax-helper/manifest.tsv` and minimal real HTML fragments from `shcntx_ru.hbk` and `shcntx_root.hbk`.

### [x] T5. Syntax Assistant root discovery

Depends on: T4.

Scope:

- Implement root section detection for global context, enum catalog and type/object catalog.
- Implement catalog traversal before specialized parsing.
- Add diagnostics for unknown page classes.
- Add fixture coverage for root/catalog pages.

Expected artifacts:

- Syntax Assistant traversal/root discovery module.
- Stable automated assertion for discovered root sections in `shcntx_ru.hbk`.

Verification:

- `cargo test`
- Stable automated assertion that discovered root sections for `shcntx_ru.hbk` include candidates for global context, enum catalog and type/object catalog.
- If the file is absent: document that real-platform root discovery smoke was skipped because the platform fixture is unavailable.
- `git diff --check`

Status:

- Completed with `syntax_helper` root discovery, catalog traversal, unknown-page diagnostics and fixture/real-platform assertions for Syntax Assistant root sections.

### [x] T6. Specialized Syntax Assistant parsers

Depends on: T5.

Scope:

- Implement object/type parser.
- Implement method parser.
- Implement property parser.
- Implement constructor parser.
- Implement enum parser.
- Implement enum value parser.
- Implement global context parser.
- Add fixtures for every parser kind.

Expected artifacts:

- Specialized parser modules.
- Representative parser fixtures and assertions.

Verification:

- `cargo test`
- Known representative assertions pass for object/type, method, property, constructor, enum, enum-value and global-context parsers.
- Full in-memory extraction against `shcntx_ru.hbk` returns non-empty global methods, global properties, platform types and enums when the file exists.
- `git diff --check`

Status:

- Completed with specialized Syntax Assistant parser domain structs, fixture assertions for every parser kind, and real `shcntx_ru.hbk` extraction smoke for the required non-empty families.

### [x] T7. Domain model and canonical JSON export

Depends on: T6.

Scope:

- Finalize provisional internal domain structs.
- Add `serde` serialization.
- Add source provenance fields to all exported records.
- Implement `syntax-helper --output`.
- Map `_root` source locale to export locale `en`.
- Document export file names and schema intent.

Expected artifacts:

- Export module and JSON schema notes.
- CLI `syntax-helper --output`.

Verification:

- `cargo test`
- `cargo run -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/ru`
- `cargo run -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/context/en`
- JSON output files are non-empty and parse successfully.
- `git diff --check`

Status:

- Completed with canonical JSON export files, source provenance serialization, `syntax-helper --output`, `_root` export locale mapping to `en`, full specialized-page extraction in the production reader path and reviewer-approved verification.

### [x] T8. Lookup helpers

Depends on: T7.

Scope:

- Add exact lookup by global member name.
- Add exact lookup by type name.
- Add exact lookup by type/member name.
- Add constructor lookup by type name.
- Keep search ranking out of scope.

Expected artifacts:

- Lookup API.
- Tests for found, missing and ambiguous items.

Verification:

- `cargo test`
- `git diff --check`

Status:

- Completed with exact `PlatformContext` lookup helpers for global members, platform types, type members and constructors, including found/missing/ambiguous coverage.

### [x] T9. Real-platform Syntax Assistant acceptance report

Depends on: T7, T8.

Scope:

- Run acceptance commands against `shcntx_ru.hbk`.
- Run acceptance commands against `shcntx_root.hbk`.
- Record counts by: global methods, global properties, types, type methods, type properties, constructors, enums and enum values; record parser warnings.
- Record unresolved pages/links.
- Convert parser gaps into follow-up tasks.
- Confirm that `shcntx_root.hbk` exports as locale `en` and list remaining localization merge decisions.

Expected artifacts:

- Checked-in acceptance report with commands, exit codes, counts and gaps.

Verification:

- Acceptance report exists and references the exact commands used.
- `cargo test`
- `git diff --check`

Status:

- Completed with `docs/syntax-helper-acceptance-report.md`, covering real `shcntx_ru.hbk` and `shcntx_root.hbk` command exits, record counts, diagnostics, unresolved page classes, follow-up parser gaps and `_root` export locale mapping to `en`.

### [x] T10. All-HBK smoke report

Depends on: T9.

Scope:

- Enumerate every `*.hbk` file under `/opt/1cv8/x86_64/8.5.1.1150/`.
- Run generic container/book/TOC smoke checks for every file.
- Record per-file successes, fatal failures and unsupported structures.
- Convert relevant unsupported structures into follow-up tasks.

Expected artifacts:

- Checked-in all-HBK smoke report with file count, commands, exit codes and per-file failures.

Verification:

- All-HBK smoke report exists and references the exact commands used.
- `cargo test`
- `git diff --check`

Status:

- Completed with `docs/all-hbk-smoke-report.md`, covering 116 target-platform HBK files with per-file `inspect` and `toc --format json` exit codes and no fatal failures or unsupported structures.

### [x] T11. Integration decision for `v8-context`

Depends on: T9, T10.

Scope:

- Compare HBK export model with existing `v8-context` source model.
- Inspect current `/home/alko/develop/open-source/v8-context` model/decision artifacts before making the integration decision.
- Decide whether this crate remains standalone, becomes a workspace member, or exposes a file-level integration artifact first.
- Record the decision in an ADR or integration note before implementation.

Expected artifacts:

- Decision artifact referencing T9/T10 evidence.

Verification:

- Decision artifact exists.
- `cargo test`
- `git diff --check`

Status:

- Completed with `docs/v8-context-integration-decision.md`, deciding that `v8-context-hbk` remains standalone for now and exposes the first `v8-context` integration through the file-level `syntax-helper --output` export before any workspace merge or direct HBK query-path coupling.
