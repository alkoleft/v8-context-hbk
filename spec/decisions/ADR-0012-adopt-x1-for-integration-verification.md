# ADR-0012: Проверить X1 в реальном анализаторе перед канонизацией

Date: 2026-08-03.

Status: Accepted.

Decision maker: project maintainer.

Consulted: Codex implementation and review agents.

Informed: maintainers of `v8-context-hbk` and downstream `v8-context`.

## Контекст

T183 сравнил SQLite-to-owned baseline H0, текущий binary-cache control C0 и
несколько memory-mapped вариантов. S83-AV5 показал, что составной X1 сочетает
сильные сигналы трёх физических решений: SoA для глобального scope,
специализированный mapped hash для имени типа и соседний с типом
owner-contiguous AoS-диапазон его непосредственных members. X1 прошёл exact
parity экспериментального корпуса, ускорил global scope и startup и снизил
память, но отдельные type-member и full-payload микротесты оставались хуже H0.

S83-AV6 проверил сохраняемые поконтекстные проекции X1-PROJECTED для составных
`AvailabilityContext` в режимах `ANY`/`ALL`. Они помогли редкому пустому
селективному `ALL`, но замедлили обычный global scope относительно X1,
проиграли на большинстве непустых type scopes и постоянно увеличили артефакт и
PSS. Пустые наборы не являются основной нагрузкой downstream-анализатора.

Следующий вопрос уже нельзя надёжно решить изолированными микротестами: даёт ли
X1 выигрыш в штатном `v8-context`, который формирует эффективный контекст
модуля и разрешает реальные BSL-ссылки. Если да, два параллельных runtime-
владельца фактов недопустимы: X1 должен заменить owned snapshot, а SQLite может
остаться только приватным входом построения и отдельным search/debug storage.

## Решение

1. X1 становится единственным кандидатом этапа X1-INT.
2. X1-PROJECTED отклоняется как production-layout. Готовые поконтекстные
   проекции и готовые комбинации `ANY`/`ALL` не входят в X1.
3. X1 пока не является каноническим runtime. Сначала он реализуется как
   non-canonical production-quality путь и проходит заранее зафиксированный
   X1-INT протокол в реальном `v8-context`.
4. При прохождении всех обязательных gates X1 становится единственным
   каноническим runtime-снапшотом HBK. Существующий owned binary-cache/runtime-
   snapshot и SQLite-backed snapshot constructors удаляются после миграции их
   потребителей, без compatibility layer и скрытого fallback.
5. При провале любого обязательного gate канонизация и удаление текущего
   runtime-пути не выполняются. Результат и причина сохраняются как evidence.
6. HBK владеет представлением availability и provider-native проверкой
   `ANY`/`ALL` при borrowed traversal глобального scope и непосредственных
   members известного типа. `v8-context` владеет cross-source precedence,
   ambiguity, effective selection и только компактным request-local control
   state.
7. Memory-mapped словарь X1 является только неизменяемой основой HBK в пределах
   поколения. BSL/metadata overlay принадлежит downstream, не копирует HBK-
   словарь и не превращает локальные числовые ID в постоянную identity.

## Результат выполнения

X1-INT прошёл каждый semantic/performance/resource/lifecycle gate без waiver.
После отдельного reviewed cutover `HbkFactSnapshot::open` стал единственным
canonical snapshot runtime, а provider materialization осталась только в
explicit `build_from_provider_*` setup. Downstream принял mapped base dictionary
и generation-borrowed views в tasks 1.14–1.14b. Последующий scoped cleanup
удалил legacy C0 binary-cache runtime и закрытые experiment producers, сохранив
SQLite build/search/debug, allocator/parity tooling, evidence и ветви.

## Обязательные gates X1-INT

- Полный catalog/resolver transcript и analyzer semantic oracle точно совпадают
  с H0 после нормализации локальных ID через provider identity и текст.
- После успешного открытия X1 все покрытые запросы продолжают работать при
  недоступных SQLite и исходном HBK; fallback запрещён.
- В steady borrowed traversal отфильтрованного global scope и members известного
  типа нет provider allocations и сохраняемого набора проекций.
- В production-процессе одновременно не живут mapped X1 и полный owned graph
  фактов HBK.
- В обоих парных повторах median `cold_module_context_handle` не превышает 50%
  median H0; `prepared_module_context_handle` не регрессирует более чем на 10%;
  `prepared_full_module_resolution` не регрессирует более чем на 5%.
- В обоих повторах peak RSS wall-процесса и cold peak heap ниже H0.
- Reader до typed access проверяет magic, X1 layout version, extraction/provider
  schema, source identity, locale, точную platform version, byte order, bounds,
  alignment, overflow, UTF-8, tags и integrity metadata.
- Reader удерживает shared lock стабильного logical slot всю сессию. Writer
  получает exclusive lock fail-fast, публикует новый неизменяемый generation
  file и current pointer атомарно и никогда не изменяет mapped target на месте.

Подробные входы, команды, порядок прогонов и политика шума принадлежат
[`hbk-zero-copy-x1-integration.md`](../implementation/hbk-zero-copy-x1-integration.md).
Никаких исключений, aggregate score или ручной компенсации проваленного gate
нет.

## Последствия

- AV5/AV6 остаются причинными свидетельствами layout, но не заменяют X1-INT.
  Их относительные регрессии type scope и payload допустимы только как
  известный риск; реальный analyzer gate решает, значимы ли они для основного
  потребителя.
- Первый локальный build X1 измеряется отдельно. Он не выдаётся за стоимость
  открытия заранее подготовленного артефакта.
- Невалидный или отсутствующий канонический снапшот приводит к typed open
  error. Восстановление — отдельная setup-операция ensure/rebuild из приватного
  build input с последующим повторным open, а не runtime fallback.
- SQL `PlatformSearchSource`/`LanguageSearchSource` могут сохраниться для явно
  принадлежащих им CLI, search, debug и последовательных resolver-сценариев.
  Они не являются snapshot runtime анализатора.
- Экспериментальные ветки и принятые evidence не удаляются. Benchmark-only код
  из них не переносится в production.

## Рассмотренные варианты

### Канонизировать X1 только по AV5/AV6

Отклонено: микротесты показывают смешанный результат и не измеряют реальную
стоимость effective context и полного разрешения модуля.

### Сохранить X1-PROJECTED как дополнительный layout

Отклонено: редкий empty-ALL выигрыш не окупает постоянные секции, merge/
intersection и второй физический путь. Дополнительный production-layout также
усложнил бы валидацию и поддержку.

### Сохранить owned snapshot рядом с X1 как fallback

Отклонено: это оставляет два живых графа одних provider facts, скрывает ошибки
артефакта и лишает zero-copy основного ресурсного выигрыша.

### Удалить SQLite полностью

Отклонено: SQLite остаётся полезным приватным входом построения и отдельным
search/debug artifact. Решение удаляет его только из analyzer snapshot runtime.

## План реализации

- Affected paths: `crates/syntax-helper-search/src/snapshot/`,
  `crates/context-resolver-search/src/hbk_catalogs/`,
  `crates/context-resolver-search/src/snapshot_adapter.rs`, downstream
  `v8-context/crates/analyze-project/src/benchmark/` и его platform catalog
  construction.
- Dependencies: использовать существующий `memmap2`, стабилизированные
  `std::fs::File` locks и текущие workspace crates; не добавлять новый
  storage/index crate без отдельного решения.
- Pattern: один provider-owned snapshot owner и существующий read/catalog seam;
  явные setup build и fail-closed runtime open.
- Avoid: whole-branch merge, compatibility DTO/adapter, hidden fallback,
  X1-PROJECTED sections, hard-coded corpus metadata и второй live fact graph.
- Configuration: X1-INT переиспользует существующие benchmark env variables и
  private checkpoint schema; новый product config в decision milestone не
  вводится.
- Migration: non-canonical implementation -> X1-INT evidence -> отдельный
  canonical cutover -> отдельный scoped cleanup. Каждый переход имеет новый
  reviewed task-local plan.

1. Зафиксировать production-source ledger X1 и inventory текущих runtime/build/
   search API до переноса кода.
2. Перенести поведение X1 в единственный `syntax-helper-search` snapshot owner,
   удалив corpus-specific константы, experiment names/features и benchmark
   harness. Не переносить экспериментальную ветвь целиком.
3. Реализовать явные build/open операции, validation, read-only mmap,
   stable-slot locking и atomic generation publication.
4. Сохранить существующие typed IDs, domain types, `HbkFactReadHandle`, BSL/SDBL
   catalogs и mapping owner. Не добавлять параллельную entity/view/catalog
   модель только ради совместимости.
5. Подключить non-canonical X1 к существующим analyzer benchmark scenarios и
   полному catalog/resolver parity probe.
6. Записать evidence и применить все gates. Только при полном pass открыть
   отдельную задачу canonical cutover.
7. После cutover удалить только inventory-proven replaced owned-cache и
   snapshot-runtime SQL paths; добавить structural reintroduction guards.
8. Зафиксировать результат HBK base dictionary в downstream OpenSpec task 1.14.

## Проверка

- [x] Пользователь явно выбрал X1 единственным X1-INT кандидатом.
- [x] X1-PROJECTED явно отклонён и не входит в production layout.
- [x] X1 остаётся non-canonical до полного X1-INT pass.
- [x] Gates, lifecycle, recovery и роли SQLite определены до implementation.
- [x] Владение availability и downstream effective selection разделено.
- [x] План запрещает параллельный owned graph и compatibility fallback.
- [x] Non-canonical X1 generation writer и полный byte-validator реализованы
      без включения runtime source.
- [x] Private validated read-only mmap generation реализован без SQL/HBK
      fallback; public open остаётся закрытым до borrowed views и slot lock.
- [x] Private borrowed forward views и provider-native `ANY`/`ALL` traversal
      реализованы без entity materialization и steady allocations.
- [x] Private base-dictionary/reverse/provider lookup surface реализован с
      owned parity, persisted indexes и zero-allocation pre-normalized path.
- [x] Stable-slot shared-reader/fail-fast-writer lifecycle и atomic immutable
      generation publication реализованы; public runtime open остаётся закрыт
      до X1-INT compatibility/catalog integration.
- [x] Stable-slot compatibility/lifecycle matrix проверяет все identity/version
      mismatch и corruption classes через единый full validator.
- [x] Frozen S83 full-corpus storage payload, availability и provenance
      совпадают между owned build oracle и mapped X1.
- [x] Frozen S83 full-corpus lookup/index surface совпадает для
      `280,317` ordered semantic call pairs; fixture сохраняет
      duplicate/ambiguity/miss edge cases.
- [x] Единый `HbkFactReadHandle`/catalog/resolver seam прошёл exact
      owned-to-mapped transcript, no-HBK/SQLite, sequential/concurrent и
      zero-allocation borrowed traversal probes.
- [x] Non-canonical X1 проходит полную storage/catalog/resolver/analyzer parity.
- [x] X1 проходит все X1-INT performance/resource gates.
- [x] Canonical cutover и последующий scoped cleanup завершены отдельными
      проверенными задачами.

## Дополнительная информация

Production-source ledger, API inventory, команды X1-INT и будущие результаты
ведутся в implementation spec, OpenSpec tasks и acceptance baseline. Изменение
решения требует нового пользовательского выбора и обновления этого ADR.
