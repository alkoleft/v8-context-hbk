# v8-context-hbk

`v8-context-hbk` is a command-line tool for reading 1C `*.hbk` help books and extracting structured platform documentation from Syntax Assistant books.

Use it when you need to inspect an installed 1C help book, print its table of contents, read a help
page, export Syntax Assistant data into JSON files for downstream tools, or build a local Syntax
Assistant search index for repeated API lookup.

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
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/ru
```

Export root/English-source Syntax Assistant data. The export locale is written as `en`:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax export /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/context/en
```

The output directory contains JSON files by record family:

- `metadata.json`
- `global-methods.json`
- `global-properties.json`
- `module-events.json`
- `type-events.json`
- `unknown-events.json`
- `platform-types.json`
- `type-methods.json`
- `type-properties.json`
- `query-tables.json`
- `constructors.json`
- `enums.json`
- `diagnostics.json`

The current provisional export schema is `schema_version: 11`. Consumer record-family files include
structured `availability`, `examples`, `see_also`, signature variant metadata, type references and
return types when the source page contains those facts. TOC-derived semantic identity fields such as
`record_family`, `module`, `owner`, `type_kind` and platform-type `object_kind` are emitted where
title-only lookup would be ambiguous. `owner_path` is limited to owning records such as
`platform-types.json`, module context and query table records. Query table fields and parameters are
nested under `query-tables.json` table records. Query table records also include localized source
syntax and a deterministic table identifier.
Absent facts are omitted from platform API consumer records. Enum values are nested in `enums.json`;
`enum-values.json` is not emitted. Consumer records omit HBK file paths, TOC paths, HTML paths and
page titles; `diagnostics.json` keeps parser provenance for maintenance.

The `syntax export` command summary reports the `diagnostics.json` record count as
`parser_warnings` because those records are parser-maintenance warnings, not exported platform API
facts.

## Query Syntax Assistant Data

Build a local SQLite/FTS5 search index from a Syntax Assistant HBK:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax index /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/context/sh-search-ru.sqlite
```

If `--output` or `--index` is omitted, commands use `V8_CONTEXT_HBK_SYNTAX_INDEX` and then
`.v8-context-hbk/syntax/index.sqlite` under the current working directory.

Run exact lookup, constructor lookup, keyword search, fuzzy name search and deterministic
relationship traversal:

```bash
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax get --index target/context/sh-search-ru.sqlite --name "ОтборКомпоновкиДанных" --format json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax get --index target/context/sh-search-ru.sqlite --owner "НастройкиКомпоновкиДанных" --member "Отбор"
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax constructors --index target/context/sh-search-ru.sqlite "HTTPСоединение"
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax constructors --index target/context/sh-search-ru.sqlite "HTTPСоединение" --details
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax search --index target/context/sh-search-ru.sqlite --query "отбор скд" --mode keywords
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax search --index target/context/sh-search-ru.sqlite --query "ОтборКомпоновкиДаных" --mode fuzzy --format json
cargo run -p v8-context-hbk-cli --bin v8-context-hbk -- syntax related --index target/context/sh-search-ru.sqlite --name "ОтборКомпоновкиДанных"
```

Query commands read only the prebuilt index. They do not reopen or parse `shcntx_*.hbk`.

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
