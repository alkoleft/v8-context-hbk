# v8-context-hbk

`v8-context-hbk` is a command-line tool for reading 1C `*.hbk` help books and extracting structured platform documentation from Syntax Assistant books.

Use it when you need to inspect an installed 1C help book, print its table of contents, read a help page, or export Syntax Assistant data into JSON files for downstream tools.

The current extraction baseline is 1C platform `8.5.1.1150`. Other versions may work, but the command and export contracts are still provisional.

## Prepare

Build the CLI from the repository:

```bash
cargo build -p v8-context-hbk-cli
```

Use the binary through Cargo:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- <command>
```

The examples below assume the platform help files are installed under `/opt/1cv8/x86_64/8.5.1.1150/`.

## Inspect a Help Book

List HBK container entities:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- inspect /opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk
```

Print a table of contents as JSON:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- toc /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --format json
```

Read a page by its HTML path from the book storage:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- page /opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk --path "<html-path>"
```

## Export Syntax Assistant Data

Export Russian Syntax Assistant data:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/ru
```

Export root/English-source Syntax Assistant data. The export locale is written as `en`:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/context/en
```

The output directory contains JSON files by record family:

- `metadata.json`
- `global-methods.json`
- `global-properties.json`
- `global-context-events.json`
- `platform-types.json`
- `type-methods.json`
- `type-properties.json`
- `table-fields.json`
- `table-parameters.json`
- `constructors.json`
- `enums.json`
- `enum-values.json`
- `diagnostics.json`

The current provisional export schema is `schema_version: 4`. Consumer record-family files include
structured `availability`, `examples`, `see_also`, `available_since` and syntax-variant metadata
when the source page contains those facts. The export also includes global context events and
query/table field and parameter metadata. Absent facts are represented as empty arrays or `null`.
Consumer records omit HBK file paths, TOC paths, HTML paths and page titles; `diagnostics.json`
keeps parser provenance for maintenance.

The `syntax-helper` command summary reports the `diagnostics.json` record count as
`parser_warnings` because those records are parser-maintenance warnings, not exported platform API
facts.

## Current Limitations

- The JSON export schema is provisional.
- The tool reads existing HBK files; it does not create or modify HBK files.
- Syntax Assistant extraction is evidence-based on the current target platform and may need parser updates for other platform versions.
- Runtime 1C introspection is out of scope. The tool extracts documentation from HBK files only.

## More Documentation

- End-user documentation: this file.
- Specification index: [spec/README.md](spec/README.md).
- Functional requirements: [spec/requirements/functional.md](spec/requirements/functional.md).
- Non-functional requirements: [spec/requirements/non-functional.md](spec/requirements/non-functional.md).
- UAT test cases: [spec/acceptance/uat-test-cases.md](spec/acceptance/uat-test-cases.md).
- Integration decision: [spec/decisions/ADR-0001-v8-context-integration.md](spec/decisions/ADR-0001-v8-context-integration.md).
