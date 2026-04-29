# UAT Test Cases

These test cases describe user-visible behavior. They are black-box scenarios and should be usable
by a human or agent without knowing the internal crate structure.

## UAT-HBK-001: Inspect Small Root Help Book

Related use case: UC-HBK-001.

Related requirements: FR-HBK-001, FR-CLI-001, NFR-DIAG-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk` exists.
- The CLI is runnable through Cargo.

Steps:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk
```

Expected result:

- Exit code is `0`.
- Output includes `PackBlock`, `FileStorage` and `Book`.
- Output is readable as an inspection result, not a panic/backtrace.

Skip rule:

- If the fixture is absent, record the skip reason and do not mark the case failed.

## UAT-HBK-002: Print Russian Help Book TOC as JSON

Related use case: UC-HBK-002.

Related requirements: FR-HBK-002, FR-HBK-003, FR-CLI-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists.

Steps:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json
```

Expected result:

- Exit code is `0`.
- Output parses as JSON.
- At least one TOC item contains a title and an `html_path`.

## UAT-HBK-003: Read a Known Help Page

Related use case: UC-HBK-002.

Related requirements: FR-HBK-002, FR-DOC-001, FR-CLI-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` exists.
- `tests/fixtures/known-pages/fmtdui_ru.page` exists.

Steps:

```bash
PAGE_PATH="$(cat tests/fixtures/known-pages/fmtdui_ru.page)"
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --path "$PAGE_PATH"
```

Expected result:

- Exit code is `0`.
- Output is non-empty HTML/text content from the requested page.

## UAT-SH-001: Export Russian Syntax Assistant Data

Related use case: UC-SH-001.

Related requirements: FR-SH-001, FR-SH-002, FR-EXPORT-001, NFR-DIAG-001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` exists.
- `target/uat/shcntx-ru` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-ru
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/uat/shcntx-ru
```

Expected result:

- Exit code is `0`.
- Required JSON files from FR-EXPORT-001 exist.
- `metadata.json` records locale `ru`.
- Core record-family files parse as JSON and contain non-empty `records`.
- `diagnostics.json` is present and parser gaps are visible if any remain.

Cleanup:

- `target/uat/shcntx-ru` is service data and may be deleted after the run.

## UAT-SH-002: Export Root Syntax Assistant Data as English Locale

Related use case: UC-SH-001.

Related requirements: FR-SH-002, FR-EXPORT-001, ADR-0001.

Preconditions:

- `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` exists.
- `target/uat/shcntx-en` can be created or removed.

Steps:

```bash
rm -rf target/uat/shcntx-en
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/uat/shcntx-en
```

Expected result:

- Exit code is `0`.
- Required JSON files from FR-EXPORT-001 exist.
- `metadata.json` records export locale `en`.
- Core record-family files parse as JSON and contain non-empty `records`.

Cleanup:

- `target/uat/shcntx-en` is service data and may be deleted after the run.

## UAT-ERR-001: Missing File Produces Readable CLI Error

Related use cases: UC-HBK-001, UC-HBK-002, UC-SH-001.

Related requirements: FR-CLI-001, NFR-REL-001, NFR-DIAG-001.

Steps:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect target/uat/does-not-exist.hbk
```

Expected result:

- Exit code is non-zero.
- Error includes the requested path.
- Error is a readable diagnostic, not a panic/backtrace.
