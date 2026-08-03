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

- [ ] **T183 — Завершить X1-INT и условный переход на canonical zero-copy snapshot**
  - Требования:
    [NFR-RESOLVE-001](requirements/non-functional.md#nfr-resolve-001-in-process-resolver-latency-and-determinism),
    [NFR-SNAPSHOT-001](requirements/non-functional.md#nfr-snapshot-001-evidence-gated-file-backed-snapshot-experiment),
    [NFR-SNAPSHOT-002](requirements/non-functional.md#nfr-snapshot-002-x1-integration-and-conditional-cutover).
  - Реализация:
    [Контракт эксперимента T183](implementation/hbk-zero-copy-snapshot-experiment.md),
    [Интеграция X1](implementation/hbk-zero-copy-x1-integration.md),
    [Снапшот фактов HBK, принадлежащий провайдеру](implementation/components.md#provider-owned-hbk-fact-snapshot),
    [Изоляция zero-copy-кандидатов T183](implementation/components.md#t183-zero-copy-candidate-isolation),
    [X1-INT и conditional canonical snapshot](implementation/components.md#x1-int-and-conditional-canonical-snapshot),
    [ADR-0012](decisions/ADR-0012-adopt-x1-for-integration-verification.md).
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
    9. после пользовательского short-list A0/I1/P1/R1 выполнить S83-AV3:
       на едином R1-base причинно сравнить AoS availability word, SoA hot
       columns, dense bitsets и prefiltered context-owner rows; сохранить I1
       как lookup-reference, но не смешивать archive/producer/layout оси;
       выполнить новый H0 parity/performance run без возврата исключённых
       F0/L1/D1 и без выбора победителя.
    10. заменить решающую corpus-wide нагрузку AV3 корректирующим S83-AV4:
        последовательно измерить filtered global scope, type lookup и filtered
        `Property`/`Method` scope только одного найденного type по пяти anchors
        и девяти `AvailabilityContext`; сравнить `member_start/count`,
        AoS/SoA, dense bitmap и direct CSR, сохранив payload/resource evidence
        и не перенося downstream resolve или выбор backend в HBK.
    11. по явному пользовательскому направлению выполнить отдельный S83-AV5:
        в одном `S83-X1` artifact совместить global SoA columns, только один
        I1-подобный type-name hash и простой direct owner-contiguous member AOS
        range; повторно измерить H0/C0/I1/R2-AOS/R2-SOA/X1, не изменяя AV4,
        shortlist, production path или статус пользовательского выбора.
    12. выполнить отдельный S83-AV6: для двух наборов `AvailabilityContext` и
        режимов `ANY`/`ALL` сравнить H0, неизменённый X1 и X1-PROJECTED с
        девятью базовыми сохраняемыми проекциями global scope и members всех
        типов; измерить steady borrowed/collect и цену artifact/startup/memory,
        не сохраняя готовые комбинации и не выбирая backend; разделить
        counters-disabled timing и отдельный counters-enabled allocation
        profile, не смешивая их время и process memory.
    13. по явному решению пользователя назначить X1 единственным X1-INT
        кандидатом и отклонить X1-PROJECTED; не делать X1 canonical до pass.
    14. productionize X1 без benchmark/corpus-specific кода и проверить его
        через storage/catalog/resolver parity и реальные analyzer scenarios.
    15. только при полном pass выполнить отдельный canonical cutover, оставив
        SQLite private build/search input и исключив runtime fallback.
    16. после cutover удалить только inventory-proven replaced owned-cache/
        runtime-SQL мусор, сохранив branches/evidence и отдельные search/debug
        контракты.
  - Проверка:
    строгая валидация OpenSpec; format/check/test для зафиксированной базы и
    каждой ветви кандидата; проверка точных корпуса и checksum;
    версионированные канонические транскрипты содержимого и lookup; повторные
    release-измерения медианы/MAD; AV2 parity упорядоченных compact sets и
    full payload, тесты harness/summarizer и последовательные release-прогоны;
    AV3 preliminary parity/format/alignment/bounds smoke; затем заменяющий его
    в решении AV4 parity global scope и scope одного type для всех девяти
    контекстов, тесты direct type range/AoS/SoA/bitmap/CSR и последовательный
    warm-only consumer-scope benchmark; затем AV5 exact parity и отдельные
    sequential steady/resource строки X1 и его причинных references по тому же
    consumer-scope contract; затем AV6 parity и последовательные steady/resource
    строки H0/X1/X1-PROJECTED для четырёх составных selectors;
    независимая проверка безопасности и производительности.
  - Граница завершения:
    X1-INT должен пройти все ADR-0012 semantic/performance/resource/lifecycle
    gates. Только затем X1 становится единственным canonical runtime; cleanup
    удаляет доказанно заменённые пути без compatibility fallback. При любом
    failed gate T183 остаётся открытой с текущим runtime и recorded evidence.
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
    Русская компактная неранжированная таблица находится в
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
    eligibility, merge или выбора канонического варианта. Предварительный AV3
    реализовал R1-derived AoS/SoA/bitmap/CSR artifacts и прошёл локальные smoke,
    но его corpus-wide enumeration members всех типов исключена из решающей
    нагрузки после сверки с `v8-context`. Заменивший его S83-AV4 проверяет
    filtered platform global scope, type lookup и filtered properties/methods
    только одного найденного type. Module events исключены; каждый fixed type
    anchor/context публикуется отдельно. AV4 проверяет `member_start/count` в
    fixed type head,
    AoS/SoA masks, dense bitmaps и direct CSR rows; corpus-wide AV3 остаётся
    stress-only. Resolve/precedence/effective selection остаются во
    `v8-context`; ranking и выбор canonical варианта запрещены до решения
    пользователя. При запуске AV4 выявлено, что прежний frozen
    schema16 provider не заполняет уже существующие
    `document_metadata.source_*` для type/member/callable. Подготовительный
    AV4 step сохранил provenance в `syntax index`,
    пересобрать provider из того же HBK/extraction-11, доказать
    exact logical-content parity без provenance со старым provider и
    заморозить новый bytes/SHA до повторной проверки H_AV4.
    Provider пересобран текущим producer `0.2.4` в
    `shcntx_ru.8.3.27.1859.schema16.av4-provenance.release.sqlite`:
    `220,270,592` bytes / SHA-256
    `f626207a93f99eecbb6c76fb13482058e32c4c4d404c84bb33b45abde45233bc`.
    Schema и все 16 non-metadata tables точно равны прежнему provider;
    logical projection `document_metadata` без `source_*` точно равна;
    все `25,052/25,052` documents имеют HBK/locale/HTML/title provenance,
    а 607 ранее заполненных source records не изменились. Operational
    meta ожидаемо отличается по `built_at`, `builder_version` и новому
    `source_index_identity`; semantic meta из locale/HBK/schema versions равна.
    Независимая ревизия H_AV4 заблокировала `/v1`: baseline
    composite lookup всегда сканировал pre-resolved primary owner,
    поэтом miss возвращал непустой scope, а parity не хранила
    composite results. Активный harness поднят на AV4 `/v2`:
    primary/alias требуют exact expected owner, miss — ноль owners и
    пустой scope, ambiguous/wrong-owner отклоняются; transcript хранит
    composite lookup/scope rows. Все schemas/workload получают `/v2`,
    а ранее созданные AV4 `/v1` artifacts не допускаются к замерам.
    Чистый последовательный AV4 `/v2` завершён на harness-коммите
    `97fa011d292ab0b243b484749e3f4ce5d22909e6`: 74/74 parity-записи,
    17,793/17,793 performance-строки и 5,841/5,841 resource-строка успешны.
    Raw SHA-256 —
    `e5c909b0b3eb179ccaa14d4574399e716fc1868a5ebfe28b306f4ba44b482f48`,
    resource SHA-256 —
    `df262ad87f122bef1b7aaf91d5334b6b3281644b7d67996b17a81a09a1d36216`,
    summary SHA-256 —
    `1a263c9ce171052f337b4eeb7116df26e7105996b7d409f1cd120bcc43777d9e`.
    Полная русская неранжированная таблица находится в
    `acceptance/hbk-s83-av4-evidence.md`. AV4 измерения завершены, но T183
    остаётся открытой на пользовательском решении: R2 layout не становятся
    новым shortlist, backend не выбран, canonical runtime и production-path не
    изменены.
    Пользователь разрешил измерительный раунд, но не выбрал backend. Отдельный
    S83-AV5 завершён на harness commit
    `73abb871fdac91f5395f43289f1d23431365ebe1`: exact parity `47/47`,
    performance `11,124/11,124`, resource `3,655/3,655`. X1 сохраняет общий
    global-layout выигрыш против H0 и I1-подобный type-name hash в `1.042x`
    I1, но не сохраняет R2-SOA global cost в пределах `5%` для 16/18 строк и
    имеет четыре малых выхода за R2-AOS type-scope boundary; основные непустые
    type scopes и full payload остаются медленнее H0. Полная русская
    неранжированная таблица находится в
    `acceptance/hbk-s83-av5-evidence.md`. Пункт OpenSpec 1.23 завершён; 1.15 и
    T183 остаются открытыми до пользовательского решения. Shortlist,
    canonical runtime и production path не изменены.
    Отдельный S83-AV6 завершён на измерительном harness commit
    `2298c74d7151fc54047b9c3567168dc71d7782ab` с итоговой summary-фиксацией
    `810f674`: 12/12 parity-записей, 1 296/1 296 timing rows, 420/420
    resource rows, 432/432 allocation rows и 252/252 allocation-resource rows.
    Raw SHA-256 —
    `9396b912f933b2f9bfd1b6b411ae72cf5c5c681f17c5266adbde2f99eaf2ea40`,
    resource SHA-256 —
    `870e035b5b1c816c93d48efc1116d7c947f61823b72e45d27abc39ad420cecfe`,
    summary SHA-256 —
    `54f39c79fb528276c3c1f06257d1c3a9052948f63ec23b4585e2e9ed9293e356`.
    Неизменённый X1 подтвердил global scope `0.22-0.24x` H0. X1-PROJECTED
    быстрее H0 на global scope (`0.50-0.67x`), но медленнее X1
    (`2.08-2.98x`), а на непустых type scopes обычно проигрывает из-за
    merge/intersection нескольких projection rows. Сохраняемые projections
    подтвердили пользу только для селективных `ALL` с ранним пустым
    пересечением. Полная русская неранжированная таблица находится в
    `acceptance/hbk-s83-av6-evidence.md`. Пункт OpenSpec 1.24 завершён; 1.15 и
    T183 остаются открытыми до пользовательского решения. Backend, canonical
    runtime и production path не изменены.
    2026-08-03 пользователь закрыл OpenSpec 1.15: X1 — единственный X1-INT
    кандидат, X1-PROJECTED отклонён, X1 остаётся non-canonical до полного pass.
    ADR-0012 и `implementation/hbk-zero-copy-x1-integration.md` фиксируют
    production-source/API ledger, exact inputs, analyzer scenarios, semantic/
    performance/resource gates, lifecycle и recovery. OpenSpec 3.1 завершён:
    build-only production X1 writer/validator создаёт детерминированный
    read-only artifact `12,430,416` bytes с SHA-256
    `0f5843f95401ba9cb5421b2ecc58a101779e43b17a86909d484bd6123ce3ffd7`
    на frozen input и не включает его как runtime source. Следующий активный
    пункт 3.2 завершён: private `X1MappedGeneration` удерживает read-only file и
    mmap, проверяет regular/read-only generation, полный общий validator и
    runtime expectation `platform/locale/source-locale/HBK SHA`; runtime-open
    не читает SQLite/HBK и не выполняет fallback. Unsafe boundary остаётся
    закрытым и требует гарантированно неизменяемый explicit generation до
    stable-slot lock в 3.5. OpenSpec 3.3 завершён: private by-value borrowed
    views покрывают весь forward payload и provenance, provider-native
    `ANY`/`ALL` фильтрует globals и непосредственные members одного известного
    owner, а полный steady nested fixture traversal подтверждён с нулём
    аллокаций. OpenSpec 3.4 завершён: `StringOrder`, persisted type-name hash,
    sorted indexes и CSR ranges покрывают всю private lookup-таблицу
    `HbkFactReadHandle` с owned parity, deterministic multi-hit order и
    zero-allocation pre-normalized path. OpenSpec 3.5 завершён: stable-slot
    session удерживает shared lock до drop, publisher использует fail-fast
    exclusive lock, immutable content-addressed generation и atomic `current`;
    corrupt/oversized/non-regular/symlink components и ancestors fail-closed,
    все три crash window сохраняют valid old-or-new recovery, а open после
    удаления HBK/SQLite не имеет fallback. Package tests `104/104`, полный
    workspace, package clippy, strict OpenSpec и независимое review прошли.
    OpenSpec 4.1 завершён: stable-slot matrix возвращает точные compatibility
    fields для platform/locale/source mismatch, а content-addressed
    magic/layout/schema/truncation/checksum/section corruptions проходят
    discovery guards и отклоняются общим byte-validator. Package tests
    `106/106`, полный workspace, package clippy, strict OpenSpec и независимое
    review прошли. Следующий активный пункт — 4.2, full-corpus storage parity.
    T183 остаётся открытой на implementation/X1-INT; canonical
    cutover и garbage cleanup условны и выполняются отдельными milestones.

OpenSpec changes archived and synchronized on 2026-07-30:
the completed change records are under `../openspec/changes/archive/`, and their
delta specifications are synchronized under `../openspec/specs/`.
