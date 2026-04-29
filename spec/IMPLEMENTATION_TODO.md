# v8-context-hbk Implementation TODO

Source of truth: [HBK components requirements and implementation plan](../docs/hbk-components-requirements-plan.md).

Loop rule:

- Take the first unchecked task.
- Keep scope limited to that task and its direct verification.
- Mark a task complete only after its listed verification passes.
- If a task cannot be completed, leave it unchecked and record the blocker in the task notes or final response.
- Do not start the next task in the same run unless the prompt explicitly asks for multiple tasks.

## Tasks

### [ ] T0. Baseline repository shape

Scope:

- Keep `README.md` aligned with the current baseline files and reference projects.
- Keep `docs/hbk-components-requirements-plan.md` as the planning source of truth until a later ADR/spec split is needed.
- Keep the minimal `cargo test` baseline passing.

Expected artifacts:

- Updated docs only if the live repository state differs from the baseline.

Verification:

- `cargo test`
- `git diff --check`

### [ ] T1. Container reader and inspect command

Depends on: T0.

Scope:

- Add library crate modules under `src/lib.rs`.
- Implement typed container errors with source path/entity context.
- Implement HBK header, descriptor and block parsing.
- Implement entity enumeration and byte reads.
- Add `inspect` CLI through `clap`.
- Add unit fixtures for binary parsing.
- Add ignored real-file smoke test for `shcntx_ru.hbk`.

Expected artifacts:

- Rust container module.
- CLI `inspect` command.
- Unit tests and optional real-platform smoke test.

Verification:

- `cargo test`
- `cargo run -- inspect /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
- `git diff --check`

### [ ] T2. Book, ZIP storage and TOC reader

Depends on: T1.

Scope:

- Implement `HbkBook` on top of `HbkContainer`.
- Inflate `PackBlock`.
- Open `FileStorage` as ZIP.
- Parse `Book` metadata.
- Implement locale inference.
- Implement TOC tree and lookup APIs.
- Add `toc` and `page` CLI commands.

Expected artifacts:

- Book and TOC modules.
- CLI `toc` and `page` commands.
- Tests for metadata, locale, TOC traversal and page access.

Verification:

- `cargo test`
- `cargo run -- toc /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --format json`
- `cargo run -- page /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --path "<known-page>"`
- `git diff --check`

### [ ] T3. Documentation page parser

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

### [ ] T4. Syntax Assistant root discovery

Depends on: T2, T3.

Scope:

- Implement root section detection for global context, enum catalog and type/object catalog.
- Implement catalog traversal before specialized parsing.
- Add diagnostics for unknown page classes.
- Add fixture coverage for root/catalog pages.

Expected artifacts:

- Syntax Assistant traversal/root discovery module.
- Tests or debug/report path listing discovered root sections for `shcntx_ru.hbk`.

Verification:

- `cargo test`
- `git diff --check`

### [ ] T5. Specialized Syntax Assistant parsers

Depends on: T4.

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
- `git diff --check`

### [ ] T6. Domain model and canonical JSON export

Depends on: T5.

Scope:

- Finalize provisional internal domain structs.
- Add `serde` serialization.
- Add source provenance fields to all exported records.
- Implement `syntax-helper --output`.
- Document export file names and schema intent.

Expected artifacts:

- Export module and JSON schema notes.
- CLI `syntax-helper --output`.

Verification:

- `cargo test`
- `cargo run -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context`
- JSON output files are non-empty and parse successfully.
- `git diff --check`

### [ ] T7. Lookup helpers

Depends on: T6.

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

### [ ] T8. Real-platform acceptance report

Depends on: T6, T7.

Scope:

- Run acceptance commands against `shcntx_ru.hbk`.
- Run acceptance commands against `shcntx_root.hbk`.
- Record counts by entity kind and parser warnings.
- Record unresolved pages/links.
- Convert parser gaps into follow-up tasks.
- Make the localization/root-source decision explicit.

Expected artifacts:

- Checked-in acceptance report with commands, exit codes, counts and gaps.

Verification:

- Acceptance report exists and references the exact commands used.
- `cargo test`
- `git diff --check`

### [ ] T9. Integration decision for `v8-context`

Depends on: T8.

Scope:

- Compare HBK export model with existing `v8-context` source model.
- Decide whether this crate remains standalone, becomes a workspace member, or exposes a file-level integration artifact first.
- Record the decision in an ADR or integration note before implementation.

Expected artifacts:

- Decision artifact referencing T8 evidence.

Verification:

- Decision artifact exists.
- `cargo test`
- `git diff --check`
