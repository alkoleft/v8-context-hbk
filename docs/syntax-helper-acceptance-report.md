# Syntax Assistant acceptance report

Task: T9 real-platform Syntax Assistant acceptance report.

Date: 2026-04-29.

Repository revision under test: local working tree after T8, before marking T9 complete.

Target platform: `/opt/1cv8/x86_64/8.5.1.1150/`.

## Commands

Run from repository root.

| Command | Exit code | Output directory |
| --- | ---: | --- |
| `cargo run -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk --output target/acceptance/t9/ru` | 0 | `target/acceptance/t9/ru` |
| `cargo run -- syntax-helper /opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk --output target/acceptance/t9/en` | 0 | `target/acceptance/t9/en` |

The output directories are temporary acceptance artifacts and are not checked in.

## Export summary

| Source HBK | Export locale | Source locale | Files | Diagnostics |
| --- | --- | --- | ---: | ---: |
| `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` | `ru` | `ru` | 11 | 703 |
| `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` | `en` | `root` | 11 | 703 |

`shcntx_root.hbk` maps to export locale `en` and keeps source locale `root` in `metadata.json`.

## Record counts

| Record family | `shcntx_ru.hbk` | `shcntx_root.hbk` |
| --- | ---: | ---: |
| Global contexts | 1 | 1 |
| Global methods | 500 | 500 |
| Global properties | 101 | 101 |
| Types | 2533 | 2533 |
| Type methods | 6702 | 6702 |
| Type properties | 10732 | 10732 |
| Constructors | 445 | 445 |
| Enums | 713 | 713 |
| Enum values | 3110 | 3110 |

## Diagnostics and unresolved pages

Both books produced the same diagnostic summary:

| Severity | Code | Parser stage | Count |
| --- | --- | --- | ---: |
| `warning` | `UNKNOWN_PAGE_CLASS` | `root_discovery` | 703 |

The current Syntax Assistant export diagnostics did not contain `UNRESOLVED_LINK` records. The unresolved extraction surface for this pass is the 703 `UNKNOWN_PAGE_CLASS` pages in each source.

First representative unresolved pages:

| TOC path | HTML path | Page title |
| --- | --- | --- |
| `0.2` | `objects/Global context/GetCommonTableIconsColorPalette6560.html` | `ПолучитьОбщуюПалитруЦветовЗначковТаблицы` |
| `0.3` | `objects/Global context/GetCommonTableShapesColorPalette6562.html` | `ПолучитьОбщуюПалитруЦветовФигурТаблицы` |
| `0.22` | `objects/Global context/SetCommonTableIconsColorPalette6559.html` | `УстановитьОбщуюПалитруЦветовЗначковТаблицы` |
| `0.23` | `objects/Global context/SetCommonTableShapesColorPalette6561.html` | `УстановитьОбщуюПалитруЦветовФигурТаблицы` |
| `0.32.0` | `objects/Global context/events/catalog200/OnExit203.html` | `ПриЗавершенииРаботыСистемы` |
| `0.32.1` | `objects/Global context/events/catalog200/OnStart202.html` | `ПриНачалеРаботыСистемы` |
| `0.33.0` | `objects/Global context/events/catalog318/SessionParametersSetting319.html` | `УстановкаПараметровСеанса` |
| `0.34.0` | `objects/Global context/events/catalog201/ExternEventProcessing92.html` | `ОбработкаВнешнегоСобытия` |
| `0.34.1` | `objects/Global context/events/catalog201/BeforeExit74.html` | `ПередЗавершениемРаботыСистемы` |
| `0.34.2` | `objects/Global context/events/catalog201/BeforeStart72.html` | `ПередНачаломРаботыСистемы` |

## Follow-up parser gap tasks

- Add classification/extraction support for global context event pages under `objects/Global context/events/...`.
- Add classification/extraction support for common table color palette global methods under `objects/Global context/*CommonTable*ColorPalette*.html`.
- Decide whether the current identical root and Russian page titles are expected source data for `shcntx_root.hbk` or a localization merge/export concern for the downstream integration stage.

## Verification

Required verification for T9:

- Acceptance report exists and references the exact commands used.
- `cargo test`.
- `git diff --check`.
