# v8-context-hbk Implementation TODO

Purpose: active implementation task ledger only.

Specification index: [Specification Index](README.md).

Completed task history:

- [archive/completed-tasks-t0-t12.md](archive/completed-tasks-t0-t12.md)
- [archive/completed-tasks-t13-t17-t19-t24.md](archive/completed-tasks-t13-t17-t19-t24.md)
- [archive/completed-tasks-t25-t34.md](archive/completed-tasks-t25-t34.md)
- [archive/implementation-todo-2026-05-04.md](archive/implementation-todo-2026-05-04.md)
- [archive/implementation-todo-2026-05-05.md](archive/implementation-todo-2026-05-05.md)
- [archive/completed-tasks-t41-t47.md](archive/completed-tasks-t41-t47.md)
- [archive/completed-tasks-t48-t56.md](archive/completed-tasks-t48-t56.md)
- [archive/completed-tasks-t57-t65-t68-t85.md](archive/completed-tasks-t57-t65-t68-t85.md)
- [archive/completed-tasks-t66-t67-t86-t90.md](archive/completed-tasks-t66-t67-t86-t90.md)
- [archive/completed-tasks-t91-t110.md](archive/completed-tasks-t91-t110.md)
- [archive/completed-tasks-t111-t134.md](archive/completed-tasks-t111-t134.md)
- [archive/completed-tasks-t135-t142.md](archive/completed-tasks-t135-t142.md)
- [archive/completed-tasks-t143-t151.md](archive/completed-tasks-t143-t151.md)
- [archive/completed-tasks-t152-t164.md](archive/completed-tasks-t152-t164.md)
- [archive/completed-tasks-t165-t182.md](archive/completed-tasks-t165-t182.md)

Current status: T35-T182 are archived historical tasks. Their durable export, schema,
data-quality, performance, parser, provider, storage, query-search, resolver-design,
language-domain, cleanup, book-content export, documentation-site, platform type-template and
type-reference conclusions live in
`acceptance/baseline.md`, `source-evidence.md`, `requirements/functional.md`,
`requirements/non-functional.md`, `implementation/components.md`,
`implementation/documentation-site.md`, `implementation/syntax-helper-query-cli.md`,
`implementation/syntax-bsl-provider-plan.md`, `implementation/solution-context-resolve.md`,
`implementation/performance-baseline-t13.md`, `implementation/performance-variants.md` and
`decisions/`. Detailed records for T165-T182 are in
`archive/completed-tasks-t165-t182.md`.

Первая текущая незавершённая задача: T183.

## Loop Rule

- Take the first unchecked task.
- If there is no unchecked task, add one before implementing new scope.
- Every new task must reference the relevant requirement, UAT, acceptance, implementation spec or
  ADR IDs from `spec/`.
- Keep scope limited to that task and its direct verification.
- Mark a task complete only after its listed verification passes.
- If a task cannot be completed, leave it unchecked and record the blocker in the task notes or final
  response.
- Do not start the next task in the same run unless the prompt explicitly asks for multiple tasks.
- Before committing, stage only files changed for the current task and verify
  `git diff --cached --name-only`.
- Do not create empty commits.

## Active Tasks

- [ ] **T183 — Сравнить изолированные гипотезы zero-copy-снапшота без выбора
  победителя**
  - Требования:
    [NFR-RESOLVE-001](requirements/non-functional.md#nfr-resolve-001-in-process-resolver-latency-and-determinism),
    [NFR-SNAPSHOT-001](requirements/non-functional.md#nfr-snapshot-001-evidence-gated-file-backed-snapshot-experiment).
  - Реализация:
    [Контракт эксперимента T183](implementation/hbk-zero-copy-snapshot-experiment.md),
    [Снапшот фактов HBK, принадлежащий провайдеру](implementation/components.md#provider-owned-hbk-fact-snapshot),
    [Изоляция zero-copy-кандидатов T183](implementation/components.md#t183-zero-copy-candidate-isolation).
  - OpenSpec:
    `openspec/changes/establish-hbk-zero-copy-snapshot-cache`.
  - Объём:
    1. зафиксировать коммитом автономную версионированную release-базу
       benchmark/parity для H0 SQLite-to-owned и C0 current-cache-to-owned;
    2. измерить шум baseline и зафиксировать числовые критерии до появления
       кода кандидатов;
    3. создать от зафиксированной базы изолированные worktree H1 с собственным
       плоским форматом и H3 с архивным кандидатом, затем создать от измеренного
       H1 вариант H2 «layout H1 + типизированный reader»;
    4. требовать паритет до принятия свидетельств производительности кандидата;
    5. опубликовать исходные свидетельства и одну неранжированную таблицу
       сравнения с происхождением ветвей и SHA коммитов;
    6. выполнить независимый набор сравнений S83 на точной версии платформы
       `8.3.27.1859`, добавив отдельные эталонные типизированный плоский/архивный
       форматы и изменяющие по одной переменной гипотезы layout,
       отображённого индекса, динамического чтения с обязательными проверками и
       прямого формирования в собственных worktree.
    7. до пользовательского решения выполнить отдельную ревизию S83-AV1:
       проверить существующие строки на enumeration полных объектов глобальных
       BSL-методов с фильтром только по каждому `AvailabilityContext`, без
       `ModuleContextKind`, сохраняя пустой availability как universal.
    8. выполнить отдельную ревизию S83-AV2 по выбранной форме A: раздельно
       измерить type/property/method/callable lookup, storage-native/borrowed iteration
       непосредственных members с фильтром только по `AvailabilityContext`,
       materialization компактного набора единых `Av2MemberLocator(u32)` и чтение
       полного payload типа/метода/свойства; основной показатель — steady
       iteration/materialization, без новых gates или выбора кандидата.
  - Проверка:
    строгая валидация OpenSpec; format/check/test для зафиксированной базы и
    каждой ветви кандидата; проверка точных корпуса и checksum;
    версионированные канонические транскрипты содержимого и lookup; повторные
    release-измерения медианы/MAD; AV2 parity упорядоченных compact sets и
    full payload, тесты harness/summarizer и последовательные release-прогоны;
    независимая проверка безопасности и производительности.
  - Граница завершения:
    обновить устойчивый acceptance baseline всеми измеренными строками и
    результатами критериев, но без явного выбора пользователя не называть
    победителя, не объединять кандидата с `master`, не принимать новую
    production-зависимость и не изменять канонический путь времени выполнения.
  - Прогресс:
    зафиксированная база benchmark/parity
    `051df7979e3cf5f6431b4d13829f436c98c47054`; протокол H0/C0, шум,
    production-жизненный цикл, инвентаризация cache с владением данными, оракул
    поведения и предварительно объявленные числовые критерии зафиксированы в
    контракте эксперимента T183. Ветви кандидатов
    `experiment/hbk-zero-copy-flat-h1`
    (`a2431254ee5d90a6e77c877e329bbb8d0ca50e84`),
    `experiment/hbk-zero-copy-flat-typed-h2`
    (`826991395a508e36b7a684dc987ead218ef27184`) и
    `experiment/hbk-zero-copy-rkyv-h3`
    (`497afa52344fb318a4f27c94762cc7eafa1126ca`) сформировали неранжированные
    свидетельства. Все они остаются невыбранными и необъединёнными. H1 не
    соответствует критериям допуска из-за блокирующих проблем с паритетом,
    валидацией и эквивалентностью рабочей нагрузки; H2 и H3 сохраняют итоговые
    значения репрезентативной рабочей нагрузки, но для них всё ещё отсутствуют
    полный канонический паритет отображённого представления и полное
    доказательство жизненного цикла первого использования. H2 освобождает
    блокировку writer до самопроверки после публикации, а её валидация
    `ModuleEventNames` упорядочивает ID владельцев иначе, чем контракт порядка
    текста с владением данными. H3 не доказывает порядок сортировки для каждого
    массива имён/ID с бинарным поиском. Аллокации production-формирования
    кандидатов и занимаемый объём секций/словарей/индексов в bytes остаются без
    инструментирования. Полная неранжированная таблица критериев находится в
    `acceptance/hbk-zero-copy-snapshot-evidence.md`.
    Пользователь запросил второй независимый набор сравнения для
    `/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk`. Идентичность его HBK —
    `40,744,845` bytes /
    `5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48`;
    идентичность SQLite провайдера со схемой 16 — `204,288,000` bytes /
    `55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab`.
    S83 использует отдельный корень target. Стенд
    `28f29b5a262db362b6b58c8109e6df6c2afbbc44` теперь содержит полный успешный
    набор H0/C0 из 61 записи: 45 образцов времени выполнения/формирования,
    девять профилей аллокаций, шесть суммарных образцов для четырёх читателей
    и одну полную запись паритета. Конкретные только для S83 критерии,
    свидетельства нагрузки на хост и дайджесты паритета зафиксированы в
    контракте эксперимента до начала работы над кандидатами
    F0/A0/L1/I1/D1/P1/R1. Параллельные агенты могут реализовывать варианты в
    отдельных worktree, но все прогоны производительности выполняются
    последовательно. В отдельных ветвях теперь находятся F0
    (`a9a98a1`), A0 (`36a41aa`), L1 (`98f8b3b`), I1 (`b7a6748`), D1
    (`a7ae530`), P1 (`b0d2523`) и R1 (`ffcb990`). Исходные 72 записи F0/A0 и
    180 записей ресурсов производных кандидатов завершены. Каждый кандидат
    проходит паритет хранения на точном коммите и
    критерий семантического транскрипта catalog/resolver для пяти запусков с
    недоступными исходными источниками. Строгий построитель сводки требует
    уникальных ID образцов, отклоняет дублирующиеся, зависящие от порядка или
    относящиеся только к коммиту-предку доказательства паритета, сохраняет
    специфичные для гипотез свидетельства занимаемого объёма/формирования и
    отмечает зашумлённые критерии без разрешённого исключения как
    неопределённые. Сводная таблица фиксирует
    `eligibility_state = no-candidate-passes-all-frozen-gates`; ни одного
    исключения нет. Ни одна строка S83 не ранжирована, не выбрана, не
    рекомендована, не объединена и не продвинута. Полные таблицы SQL baseline,
    контрольного текущего варианта и кандидатов по запуску, lookup, операциям,
    памяти, production-формированию, аллокациям, занимаемому объёму и критериям
    находятся в `acceptance/hbk-zero-copy-snapshot-evidence.md`.
    T183 остаётся открытой на этапе решения пользователя. Согласно
    зафиксированному контракту пользователь может запросить повторный прогон
    или новую гипотезу, явно разрешить исключение из именованных критериев в
    устойчивом решении либо отклонить/остановить production-внедрение.
    Пользователь подтвердил дополнительный workload S83-AV1 до такого решения:
    H0 остаётся SQL baseline, C0 — не участвующий в решении контроль; каждый
    `AvailabilityContext` измеряется независимо, `ModuleContextKind` запрещён,
    пустой availability означает доступность во всех контекстах, а public API
    resolver на этом этапе не проектируется.
    S83-AV1 завершён на harness-коммите
    `37d968b868caa4f47bea4292d7f9424735b06c01`: 81/81 parity-комбинаций и
    1,458/1,458 последовательных measurement-записей прошли для девяти
    `AvailabilityContext`, девяти warm и девяти cold-best-effort образцов по
    1,000 enumeration. Raw SHA-256 —
    `c16fc9d5935e429e6b4684ed2348140521433f1157a938429f2d288c0efd984e`,
    summary SHA-256 —
    `a1b362b4d7e65a8233a6cde3736095287dd9c90bf1f0b152b558a907d78ab8d6`.
    Полная русская неранжированная таблица находится в
    `acceptance/hbk-s83-av1-evidence.md`. T183 остаётся открытой до решения
    пользователя; ни один вариант не выбран и не назначен каноническим.
    Пользователь выбрал для следующей ревизии S83-AV2 форму результата A:
    отдельно сравнить iteration без materialization, request-local compact set
    локальных ID/locator и full-payload access. Эта форма не является выбором
    backend, не меняет frozen gates и требует повторного прогона всех строк в
    отдельном results namespace.
    S83-AV2 завершён на harness-коммите
    `80ec7bbaf62cb2fdbce98908d48891f3064413cc`: 81/81 parity-записей, 9/9
    smoke-записей и 5,508/5,508 performance-записей прошли для lookup,
    borrowed/native iteration, compact materialization и payload access. Raw
    SHA-256 —
    `c733603e373a82745f6a10a1a661925b3c7335dbf1868bf91f6e91d86c3581de`,
    summary SHA-256 —
    `479c33e79cd9f06f2a0bf3894581180825ef6e77e357f9ce77ba366245453ce1`.
    Полная русская неранжированная таблица находится в
    `acceptance/hbk-s83-av2-evidence.md`. По основному показателю AV2
    `members_by_owner_availability_borrowed/collect` все zero-copy-кандидаты
    медленнее H0 SQL baseline; I1 отдельно показывает улучшение части точечных
    lookup, но это не является выбором. T183 остаётся открытой до решения
    пользователя; ни один вариант не выбран и не назначен каноническим.
    После представления AV2 пользователь сократил активный пул до A0/I1/P1/R1.
    F0 остаётся reference-only и вытеснен из активного пула P1 с тем же runtime-
    артефактом и более сильным producer; L1 исключён как не давший материального
    эффекта page-layout; D1 исключён из-за переноса validation cost в первый
    доступ. Ветки/коммиты/evidence сохраняются. Это shortlist без ranking,
    eligibility, merge или выбора канонического варианта. Следующий разрешённый
    шаг — исследовать новые member/availability layout-гипотезы поверх активного
    пула, включая bit mask/bitmap и прямые context/owner ranges.

OpenSpec changes archived and synchronized on 2026-07-30:
the completed change records are under `../openspec/changes/archive/`, and their
delta specifications are synchronized under `../openspec/specs/`.
