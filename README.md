# v8-context-hbk

Rust components for reading 1C `*.hbk` help books and extracting structured platform documentation/context from Syntax Assistant books.

This repository is planned as a future HBK-backed component for `/home/alko/develop/open-source/v8-context/`. For now it remains a separately testable workspace while the extraction model and contracts are still provisional.

Current planning baseline:

- target platform: `/opt/1cv8/x86_64/8.5.1.1150/`
- small real HBK smoke files for container/book/navigation stages:
  - `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk`
- Syntax Assistant acceptance files:
  - `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`
- final broad smoke target: all `*.hbk` files under `/opt/1cv8/x86_64/8.5.1.1150/`
- primary implementation reference: `/home/alko/develop/open-source/hbk-reader`
- secondary model/export/search reference: `/home/alko/develop/open-source/bsl-context-multi-project/platform-context-exporter`

See [HBK components requirements and implementation plan](docs/hbk-components-requirements-plan.md).
