# Non-Functional Requirements

## NFR-REL-001: Reliability

- User-controlled HBK/HTML input must not trigger parser `unwrap()` or panic.
- Errors must include path, entity or page context where relevant.
- Unsupported structures must fail explicitly or produce visible recoverable diagnostics.

## NFR-PERF-001: Resource Use and Throughput

Minimum resource consumption and high throughput are first-class requirements for follow-up work.
Large Syntax Assistant books must remain usable on a developer workstation without avoidable
whole-book copies, unbounded page buffering or unbounded worker fan-out.

Requirements:

- Container opening should not eagerly decompress all pages.
- Page content should be read lazily from `FileStorage` where practical.
- Before architecture refactors, measure wall-clock time, peak RSS or equivalent, command exit
  status and output counts for `shcntx_ru.hbk`, `shcntx_root.hbk` and the all-HBK smoke path.
- Prefer bounded streaming before broader concurrency.
- Parallel work must be bounded and deterministic.
- If page parsing or export is parallelized, preserve stable diagnostics, stable JSON output order
  and typed error context.
- Do not add caches, generic pipeline frameworks, plugin systems or tuning knobs until measurements
  show a concrete bottleneck and a concrete consumer needs the behavior.

Implementation hypotheses to evaluate after baseline measurement:

1. Narrower streaming or batched reads across `FileStorage` and page loading.
2. Keep memory-mapped container access only if it remains the simplest low-copy strategy.
3. Bounded parallel Syntax Assistant page parsing with deterministic diagnostics and output order.
4. Streaming record-family JSON export if serialization is a measured bottleneck.

Saved variants and selection rules live in
[`spec/implementation/performance-variants.md`](../implementation/performance-variants.md). Treat
that document as a candidate plan, not as approval to skip the baseline measurement.

## NFR-QUERY-001: Search Query Latency

Fast Syntax Assistant lookup is a separate requirement from HBK extraction throughput.

Query commands must use a prebuilt local search index and must not parse `shcntx_*.hbk` inside the
query path. Index build commands may be slower and may reuse extraction pipelines, but interactive
query commands must be optimized for repeated use. Building the index through a canonical JSON export
directory is not a supported first-slice path.

Provisional targets for the first indexed CLI slice on the target developer workstation:

- exact name or owner/member lookup returns in under 1 second for the `shcntx_ru.hbk` data set;
- keyword/fuzzy/relationship search returns in under 2 seconds for the `shcntx_ru.hbk` data set;
- JSON output order is deterministic across runs;
- if the command cannot meet these targets, the implementation task must record exact measurements
  and identify the limiting index or ranking component before adding broader optimization.

The first semantic-search experiment must preserve the local deterministic search path. Embeddings
or model-backed ranking may rerank or supplement results, but exact lookup and relationship graph
queries must continue to work without network access or an embedding provider.

The first query index storage must remain local and rebuildable. A SQLite/FTS5 index is the current
preferred implementation direction because it supports exact lookup, full-text search and bounded
relationship traversal without running a service. External search engines or graph databases require
a measured limitation in the SQLite-backed slice and a separate ADR update.

## NFR-SITE-001: Documentation Site Scale and Responsiveness

Documentation site generation and serving must avoid the full-corpus build failure mode of
general-purpose page-per-route static-site generators.

Requirements:

- The web bundle or initial server response must not embed all documentation page Markdown.
- The site must load TOC sections and page content lazily from generated data files.
- The generator must write deterministic artifacts in bounded memory and deterministic order.
- Build tasks must record source book count, generated page count, output size, wall-clock time and
  peak RSS or equivalent for the representative local HBK corpus before broad optimization.
- Long-running generation must expose coarse progress at deterministic boundaries so a user can see
  which stage is active without requiring page-level debug logs.
- Do not add search service, Syntax Assistant API endpoints, worker pool tuning knobs or semantic
  search provider until the documentation navigation/page-view path is accepted and measured.
- The web app must serve/load generated data without parsing HBK files in request paths.

Parallel query commands from different CLI processes must be supported as concurrent read-only
SQLite readers over the same resolved index path. Index rebuild must not update the active index file
in place: write a complete temporary database beside the target, validate it, then atomically replace
the target path. Concurrent index writers must be serialized by a lock so readers observe either the
previous complete index or the next complete index, not a partially written database.

## NFR-RESOLVE-001: In-Process Resolver Latency and Determinism

Rust solution-context resolution is a separate hot path from CLI query commands. It must support
repeated type, member and callable lookups inside one Rust process without spawning the CLI or
parsing JSON for each lookup.

Requirements:

- Resolver calls must use prebuilt provider indexes or in-memory provider snapshots. They must not
  parse HBK files, configuration source files, BSL source or query text in the lookup hot path.
- Worker-safe HBK provider facts should be shared as immutable provider-owned snapshots. Snapshot
  construction may read a provider SQLite index, but worker lookup must not share
  `rusqlite::Connection`, raw SQLite tables or mutable resolver state across analysis threads.
- Snapshot physical indexes must be shaped by analyzer lookup workflows, not by public DTO family
  names. The first hot paths are:
  - resolving a known owner type and then looking up a property, method or event by normalized name
    and kind;
  - resolving callable overloads or constructors for the already resolved owner/type;
  - resolving module-context globals and events by language/domain/module kind;
  - resolving query-table templates by query name or identifier and then resolving fields or
    virtual-table parameters by table and normalized name;
  - traversing explicitly supported relation kinds from a known fact id.
- Snapshot-owned nodes are the single source of provider facts. Secondary indexes store only compact
  keys and node references or ranges into the owned arenas; they must not duplicate DTO payloads.
- Snapshot performance work must measure before and after changing storage layout. Measurements must
  separate immutable snapshot-owned heap from process-level RSS and transient SQLite/materialization
  memory. Index memory must be reported by index family so a new hot-path index cannot hide a broad
  duplicated payload behind the total heap number.
- Analyzer hot-path lookup measurements must use release builds and batched warm lookups after
  source open. One-shot command elapsed time is not sufficient evidence for member/callable/query
  lookup latency.
- Provider-owned snapshot indexes must stay inside the provider crate/read-handle boundary.
  Downstream adapters may project read-handle results into resolver DTOs, but must not maintain
  duplicate provider-fact maps or raw SQLite readers for analyzer hot paths.
- Static-analysis integration must use direct Rust library calls in the lookup hot path. HTTP,
  daemon, MCP, CLI-spawn and JSON-over-process transports are out of scope for the first resolver
  surface.
- The first platform adapter should be at least as deterministic as the CLI JSON provider and should
  preserve explicit `not_found`, `ambiguous` and `unsupported` outcomes.
- Same input, active source set and source artifacts must produce deterministic candidate ordering.
- Source composition must not hide ambiguity across platform, BSL-language, query-language,
  configuration and source-code domains.
- Do not add global caches, plugin systems, async runtimes, service lifecycles or tuning knobs until
  measurements show a concrete bottleneck and a concrete consumer needs that mechanism.

Provisional first-slice targets on the target developer workstation:

- exact id/name type resolution returns in under 100 ms after the resolver source is opened;
- member listing for a resolved owner returns in under 100 ms after the resolver source is opened;
- callable lookup for a resolved owner or callable id returns in under 100 ms after the resolver
  source is opened.
- query-shaped snapshot follow-ups should treat those `100 ms` targets as the outer resolver/API
  ceiling, not as an acceptable in-memory index lookup budget. Known-owner snapshot lookups should be
  measured with batched release benchmarks and must record per-operation timings for at least
  member-by-owner/name/kind, callable-by-owner/name, constructor-by-type, module-context-by-kind,
  query-field-by-table/name and relation-by-source/kind.
- snapshot-layout changes must compare warm build time, snapshot-owned heap and process peak RSS
  with the previous accepted baseline. Increases above the task's documented tolerance require an
  index-family breakdown and measured lookup benefit before the task can be accepted.

If these targets cannot be met, the implementation task must record measured timings, source size
and the limiting storage or translation component before adding broader optimization.

<a id="nfr-snapshot-001-evidence-gated-file-backed-snapshot-experiment"></a>

## NFR-SNAPSHOT-001: Эксперимент с файловым снапшотом, управляемый свидетельствами

Работа T183 над zero-copy-снапшотом является ограниченным экспериментом по
сравнению, а не решением о production-формате.

Требования:

- SQLite-to-owned является baseline, а current-binary-cache-to-owned —
  контрольным вариантом; каждый подход к zero-copy-построению представляет
  собой отдельно идентифицированную гипотезу, измеряемую одним и тем же
  зафиксированным release-стендом.
- Общий стенд, точные корпус/checksum, границы процессов, режим cache, число
  прогонов, правило оценки шума, схема исходных результатов и оракул поведения
  должны быть зафиксированы коммитом до создания ветвей кандидатов. Изменение
  стенда делает затронутые сравнения недействительными до повторного прогона
  их строк baseline и кандидатов.
- Шум baseline должен быть измерен, а локальные для задачи числовые критерии
  пользы и отсутствия регрессий — зафиксированы до начала реализации кандидатов.
- Свидетельства производительности учитываются только после того, как кандидат
  прошёл побайтный паритет логического содержимого и lookup-транскрипта.
  Числовые локальные для сессии ID нормализуются через логическую идентичность
  факта и текст.
- Измерения запуска должны различать production-формирование/перестроение
  снапшота, открытие от старта процесса до готовности, первый lookup, пакетный
  lookup после прогрева и устойчивое состояние после рабочей нагрузки.
  Результаты для холодного запуска по мере доступности и прогретого запуска
  разделяются; непроверяемое рекомендательное вытеснение page cache нельзя
  описывать как настоящий холодный запуск.
- Отдельный workload контекстно-зависимой enumeration глобальных BSL-методов
  должен фильтровать только по типизированному `AvailabilityContext` и не
  использовать `ModuleContextKind`. Пустой список availability означает
  доступность во всех контекстах. Измерение должно полностью потреблять
  связанные global/callable, сигнатуры, параметры и возвращаемые типы,
  сохранять порядок SQL-to-owned baseline и выполняться отдельно для каждого
  поддерживаемого `AvailabilityContext`.
- Отдельный workload S83-AV2 должен раздельно измерять точечный lookup типа и
  метода известного типа, не материализующую результат нативную iteration
  членов известного типа с фильтром по `AvailabilityContext`, материализацию
  компактного набора единых четырёхбайтовых benchmark-local member locator и
  последующее чтение
  полного наблюдаемого payload типа, метода и свойства по заранее найденному
  ID/locator. Там, где backend способен возвращать заимствованные views,
  нативная iteration использует их; декодирование по значению и его аллокации
  остаются измеряемым свойством backend. Lookup, формирование компактного
  набора и чтение payload имеют непересекающиеся границы времени. Пустой
  availability означает доступность во всех контекстах, а
  `ModuleContextKind` не используется. Членами считаются непосредственные
  записи `owner -> member`: транзитивная иерархия объявлений, precedence и
  `effective_members` не входят в workload. Основным показателем enumeration
  является steady-state. Для каждой операции фиксируются latency,
  аллокации/байты и page faults, где они наблюдаемы; удерживаемая память
  результата относится только к materialization компактного набора, а для
  остальных операций используется allocator/process-boundary memory. H0
  остаётся единственной baseline-строкой, C0 — control;
  performance кандидата учитывается только после точного parity с H0.
- Корректирующий workload S83-AV4 должен отражать фактический consumer scope:
  filtered platform global methods/properties, lookup platform type и filtered
  `Property`/`Method` scope только одного найденного типа. Он должен отдельно
  измерять borrowed iteration, compact `u32` locator set, end-to-end
  primary/alias/miss lookup + scope и полный payload type/method/property для
  пяти зафиксированных anchors `zero/median/p90/p99/max` и всех девяти
  `AvailabilityContext`. `ModuleContextKind`, module events, обход members всех
  типов, precedence, ambiguity resolution и `effective_members` не входят в
  решающую storage-нагрузку. До performance обязателен точный H0 parity order,
  owner, kind, availability, provenance, payload и composite lookup/scope,
  включая пустой miss scope. `R1-DIRECT` изолирует только эффект
  `member_start/count`; AoS, SoA, dense bitmap и direct CSR являются отдельными
  layout-гипотезами и не становятся production-кандидатами или новым shortlist
  автоматически. Результат публикуется по каждому context/anchor без aggregate
  score, ranking или выбора canonical runtime.
- Свидетельства использования ресурсов должны различать сохраняемую приватную
  heap-память и отображённые/общие страницы, а также фиксировать аллокации,
  пиковый RSS, устойчивые RSS/PSS, суммарный многопроцессный PSS, page faults и
  размеры артефакта/секций/индексов там, где это поддерживают инструменты хоста.
- Каждый отображённый кандидат должен проверять версию бинарного layout,
  версию схемы извлечения, идентичность источника, locale и точную версию
  платформы. Несовпадение платформы делает артефакт недействительным.
- Отображённые файлы кандидатов неизменяемы. Reader удерживает разделяемую
  блокировку изменения в течение всего времени жизни отображения; writer,
  который не может немедленно получить эксклюзивную блокировку логического
  слота, возвращает типизированную ошибку «снапшот используется», не изменяя
  активное поколение.
- Ветви и зависимости кандидатов являются экспериментальными. Прохождение
  критериев не выбирает победителя, не изменяет канонический путь времени
  выполнения и не разрешает объединение с `master`; выбор делает пользователь
  после изучения неранжированной таблицы результатов.
- Второй корпус или новая ревизия стенда образуют отдельный набор сравнения.
  Его исходные результаты, файлы паритета, подготовленные артефакты, baseline
  и числовые критерии должны использовать отдельное пространство имён и не
  должны объединяться с предыдущим набором.
- Эксперименты с организацией должны изолировать по одной основной переменной
  на производную ветвь: физический порядок/layout секций, форму отображённого
  lookup-индекса, eager/lazy-доступ с обязательными проверками или стратегию
  формирования снапшота. Эталонные форматы могут быть приведены рядом, но не
  заменяют эти гипотезы организации.
- Реализация кандидатов может выполняться в параллельных worktree, но измерения
  времени на одном хосте должны выполняться последовательно. Каждый образец
  фиксирует точную идентичность корпуса и стенда, а также состояние
  нагрузки/памяти хоста, чтобы помехи и дрейф оставались видимыми.

Полный протокол и реестр гипотез определены в
`implementation/hbk-zero-copy-snapshot-experiment.md`.

## NFR-TEST-001: Testability

- Test behavior, not implementation details.
- Unit fixtures cover deterministic binary/parser behavior.
- Small real-HBK smoke tests use `fmtdui_root.hbk` and `fmtdui_ru.hbk`.
- Syntax Assistant integration tests use `shcntx_ru.hbk` and `shcntx_root.hbk`.
- Syntax Assistant fixture corpus must come from real 8.5 pages and include a manifest with source
  HBK file, HTML path, page title, parser kind and inclusion reason.
- Broad all-HBK smoke is an acceptance/reporting stage, not a prerequisite for early parser work.

Verification tiers:

1. Unit fixtures committed to the repository.
2. Small real-HBK smoke.
3. Syntax Assistant smoke.
4. All-HBK smoke.
5. UAT cases from `spec/acceptance/uat-test-cases.md`.

## NFR-COMPAT-001: Compatibility Policy

- First supported platform baseline is `8.5.1.1150`.
- A bounded comparison experiment may use an older explicitly identified
  corpus, including `8.3.27.1859`, without advertising that platform version
  as a supported production baseline. Its artifact must still carry and check
  the exact experimental platform version before mapping.
- Parser logic should avoid assumptions that are only true for one HTML filename when TOC carries a
  more reliable relationship.
- Syntax Assistant reading must remain TOC-aware for classification, semantic ownership and
  disambiguation. Filename conventions may be used as hints, but not as the sole compatibility
  contract for source families that appear under multiple TOC branches.
- Root section detection should be data-driven and tested against Russian/root books.
- Do not preserve backward compatibility for its own sake.
- Contract stability is intentionally deferred until parser evidence, consumer feedback and model
  boundaries justify it.

## NFR-DIAG-001: Diagnostics

Fatal errors stop the current command/test:

- missing file
- invalid container structure
- missing required HBK entities
- unreadable ZIP storage
- malformed book metadata
- TOC corruption

Path-backed help-book access may surface `FileStorage` ZIP read errors at the page/file access
boundary after the initial book metadata/TOC open has succeeded.

Recoverable extraction diagnostics must not abort a full Syntax Assistant pass when partial
extraction remains meaningful:

- unknown page class
- unsupported HTML block
- unresolved link
- missing optional section
- parser field that cannot be mapped safely
- data-contract gaps such as multiple return types per overload when unsupported

Diagnostics for known unsupported source families must use stable family-specific codes so a
maintainer can distinguish genuinely unknown pages from explicitly out-of-scope or unsupported
families.

Every recoverable diagnostic must include:

- severity
- stable code
- source HBK path
- locale/source locale
- TOC path when known
- HTML path when known
- page title when known
- parser stage

CLI commands return non-zero for fatal errors. Reporting commands that scan many files may continue
after per-file failures, but the final summary must make failures visible and return non-zero when
the requested acceptance contract is not met.

## NFR-LIC-001: Licensing and Attribution

- `hbk-reader` is MIT-licensed and can be used as a reference.
- Ported logic should preserve attribution where appropriate.
- Generated platform documentation must not be copied into the repository except minimal fixtures
  required for parser behavior tests.
