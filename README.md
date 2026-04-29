# v8-context-hbk

Rust components for reading 1C `*.hbk` help books and extracting structured platform documentation/context from Syntax Assistant books.

This repository is planned as a future HBK-backed component for `/home/alko/develop/open-source/v8-context/`. For now it remains a separately testable workspace while the extraction model and contracts are still provisional.

Current planning baseline:

- target platform: `/opt/1cv8/x86_64/8.5.1.1150/`
- first acceptance files:
  - `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk`
  - `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk`
- primary implementation reference: `/home/alko/develop/open-source/hbk-reader`
- secondary model/export/search reference: `/home/alko/develop/open-source/bsl-context-multi-project/platform-context-exporter`

See [HBK components requirements and implementation plan](docs/hbk-components-requirements-plan.md).
