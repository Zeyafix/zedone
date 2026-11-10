# EV-084 — P03.06: итоговая проверка общего контракта

```yaml
step: P03.06
date: 2026-11-10
base_revision: 695d0b8685ae721811d299a540b50c0aceaaf99b
branch: test/p03-06-adapter-acceptance
goal: "Проверить общий контракт, актуальность и повторы готовых команд через все адаптеры."
requirements: [T06, A05, A07, A17, A55, A56]
dependencies_evidence: ["P03.05/EV-058/PR #47", "P04.10/EV-076/PR #77"]
scope: ["недостающие adapter tests и сводная матрица T06", "документация и передача"]
checks: ["bun scripts/check-product.mjs", "bun scripts/check-product.mjs --android", "полный diff и сохранность исходных dirty docs"]
reverse_and_failure: "Устаревшие команды без частичных эффектов; перечитывание/новое действие, обратные действия и исходные повторы после restart."
result: verified_local
next: "Ревью и интеграция текущего diff; T06/P03 закрывать после необходимого CI/интеграции, не по локальному verified."
```

Один вопрос приёмки: общий контракт готовых прикладных команд одинаков для
HTTP, CLI и IPC. Пользовательский UI P05, продуктовый обмен P06, физический
arm64 и выпуск не входят в этот шаг. Связанные A получают доказательства
API-среза и не закрываются автоматически вместе с T06.

Исходные незакоммиченные документы сохранены. Сырые логи, машинные сведения
и локальные снимки хранятся только в игнорируемой `docs/reports/p03-06/`.

## Матрица T06

Все указанные проверки входят в обычный `scripts/check-product.mjs`.
Существующие тесты используются повторно; предыдущие EV подтверждают
зависимости, но не заменяют новый запуск текущей ветки.

| Область | Исполняемые доказательства | Проверяемый результат |
| --- | --- | --- |
| Все 39 операций | `core/tests/contract.rs`; `adapters/tests/commands.rs` и `commands/{spaces,columns,cards,ordering,tags,search,history,cleanup}.rs` | Доступ (2), пространства (6), колонки с move (7), карточки с поиском/историей/назначением тега (15), теги (5), очистка (4); конкретные Rust decoders/handlers и общий Service |
| Строгий конверт и производные | `core/tests/contract.rs`, отрицательные domain adapter tests, product-check, `tests/contract-types.ts`, `tests/commands.test.ts` | Rust → OpenAPI → openapi-typescript/openapi-fetch; operation-specific standalone validator; неверные ID/поля/дубли/подмена операции и доверенного контекста отклоняются |
| Общие права и время | `adapters/tests/commands.rs`, domain adapter tests; `storage/src/access/tests.rs`, `clock/tests.rs`, `replay_tests.rs` | Проверка субъекта/точной области/read-write/admin/срока/отзыва до цели и receipt; restart, смена ключа/прав, откат и фиксация времени; localhost/CLI/IPC не дают обхода |
| Вся матрица карточки | `commands/acceptance.rs::every_card_field_invalidates_every_mutation_at_all_entries_even_after_undo_and_restart` | 11 независимых изменений × 13 вариантов старых намерений × HTTP/IPC/CLI; та же отклонённая команда после обратного действия/restart по-прежнему stale; все снимки и история сохраняются |
| Состав и данные контейнеров | `commands/acceptance.rs::container_changes_reject_old_deletions_without_partial_effects_on_every_entry`; прежние `commands/{columns,cleanup,offline}.rs` | 15 изменений: добавление/корзина/восстановление/вход/выход/замена карточки, название/порядок/создание/корзина/восстановление колонки, имя пространства, создание/имя/удаление тега. Старое удаление целиком отклоняется, новое после перечитывания и восстановление проходят |
| Контроль проекции | `commands/acceptance.rs::reads_history_settlement_and_unrelated_changes_keep_prepared_versions_valid`; `storage/src/cards/tests/{trash_groups,cleanup}.rs` | Чтение, фиксация парной истории без изменения текущих данных и другое пространство не инвалидируют исходные версии; другая карточка/колонка и технические peer/history изменения проверены на закрытой storage-границе |
| Конкуренция | `commands/acceptance.rs::competing_adapter_changes_require_a_new_intention_for_the_loser`; прежние adapter/storage replay и card/group tests | Два workers HTTP/IPC не могут применить разные изменения одной версии; проигравший запрос stale также в CLI; новое явное намерение проходит, старый receipt сохраняет позднее состояние |
| Повторы и неизвестный исход | Все domain adapter tests; `commands/offline.rs`; `storage/src/replay_tests.rs`; `tests/commands.test.ts` | Исходные ID/JSON/expected/subject, mismatch, ограниченные попытки/ручной повтор, unavailable/outcome_unknown, restart и параллельная доставка; нет новой правки/истории/границы серии/группы и отмены восстановления |
| Атомарность и сбои | `storage/src/{tests,replay_tests}.rs`, domain storage tests карточек/колонок/пространств/тегов/очистки | SQL/checkpoint/commit/сериализация/остановка дочернего процесса не дают частичных данных или ложного успеха; закрытые fault seams не экспортируются адаптерами |
| Границы пакетов и unsafe | product-check metadata/lints, Clippy, release/all-features и независимые negative guards всех lib/bin/test targets | Один Rust-источник контракта, core без I/O/SQL/Automerge, `unsafe_code=forbid` без продуктовых исключений; сторонние зависимости не объявляются свободными от unsafe |

В новой матрице карточки отдельно меняются только заголовок, только описание,
оба текста, назначение и снятие тега, установка и снятие выполнения, колонка,
порядок, отправка в корзину и восстановление. Проверяются edit, два значения
set_completed/set_tag, два назначения move, trash/restore, begin_editing,
autosave, finish_editing и restore_text. Это десять типов изменяющих команд;
общий `expected` не заменяется свежим автоматически. Дополнительный
`expected_tag` сохраняет собственную проверку тега.

Новые проверки используют закрытую `offline::Board` через дочерний тестовый
модуль, без дублирования fixture или публичного API для сбоев. Ориентир 300
строк превышен ради одной неделимой итоговой матрицы контракта: четыре теста
проверяют отказ, его положительный контроль, конкуренцию и исходный повтор.
Runtime, схема, зависимости, generated artifacts, CI routing и бюджеты прежние.

## Платформы и границы доказательства

HTTP доменных сценариев — настоящий Axum router in-process; TCP framing,
Origin/authorization/no-store отдельно проверяются существующими socket tests.
IPC — штатный `ipc::invoke` с исходной JSON-строкой; Android Tauri bridge и
`IpcInvocation` компилируются для обеих ABI. CLI — настоящий отдельный процесс
с синтетическим ключевым файлом и stdin, включая нативный Android CLI.
Синтетические базы принадлежат TempDir/одноразовому Android sandbox.

Android открывается и выключается через Device; сборка/нативные тесты работают
штатным CLI. UI приложения, IME/Back, оболочка и продуктовый сетевой доступ в
этом шаге заново не принимаются. Их предыдущие доказательства P05 не заменяют
будущую полную UI/обменную матрицу. Физический arm64 не исполняется; arm64 здесь
подтверждается сборкой bridge/targets и negative guards. Fault tests не являются
физическим отключением питания, native `--offline` — флаг Cargo, не сетевой экран.

## Проверки и результат

- Windows `bun scripts/check-product.mjs` — exit 0: 283 Rust-теста (17 core,
  7 contract, 33 adapters, 226 storage), 23 client/239 assertions; fmt/Clippy,
  release/all-features, семь negative unsafe guards, OpenAPI/TS/validator/типы.
- Полный Android `bun scripts/check-product.mjs --android` — exit 0: те же
  283 нативных Rust-теста и 23 client/239 assertions, release x86_64/aarch64
  с Tauri bridge/all-features, 14 negative unsafe guards. Экспортированный
  Android OpenAPI совпал с host; производные/validator/типы проверены.
  Native sandbox удалён, эмулятор выключен через Device в исходное состояние.
- Четыре новых сценария прошли целевой Windows-прогон и полный host-набор.
  Исходный отказ тестовой подготовки снятия назначения исправлен свежим
  `expected_tag`; это не обнаруженный дефект продукта и не закрытие GAP.

P03.06 — `verified` локально. T06 и связанные A пока `partial`: итоговая
контрактная матрица выполнена на обеих платформах, ревью/CI/интеграция
ожидаются. Полная UI/обменная приёмка связанных A остаётся P05/P06.
PR/новый CI/commit/push/merge
не выполнены. Открытые GAP-007/012/013 и DEC этим шагом не закрываются.

## Пять правил проекта

- Обратимость: прямые/обратные действия карточки и контейнеров проходят,
  отклонённые команды сохраняют данные, повтор прежнего удаления сохраняет
  последующее восстановление и группы. Удаление тегов и явные ручные очистки
  сохраняют ранее принятые исключения; новый необратимый функционал не добавлен.
- Человек и агент: HTTP/CLI/IPC используют одинаковые Service/core/права,
  исходные условия и результаты. Тестовый bootstrap ограничен синтетической fixture.
- Документация: актуализированы инструкция повторов агента, разработка ядра,
  архитектурные границы, очередь, журнал и эта матрица. Пользовательские правила
  в `docs/user/{cards,columns,access-time-and-retries}.md` проверены и не меняются.
  Нового архитектурного решения или изменения пользовательского поведения нет.
- Паритет: одинаковые нативные тесты Windows/Android и обе Android ABI/bridge
  проверяются штатным product-check; текущий результат фиксируется выше.
  UI/физический arm64/release APK не выдаются за выполненные в этом шаге проверки.
- Единый источник истины: данные/права/receipt находятся в одном хранилище;
  Rust определяет контракт. Старый receipt не заменяет свежий снимок,
  клиентский pending сохраняет намерение и не выдаёт полномочий.

После финальных прогонов менялась только документация. SHA-256 двух тестовых
источников совпали с проверенными; семь посторонних dirty docs побайтово прежние.
Старые записи журнала/очереди сохранены, все 98 определений критериев неизменны.
Локальные источники данных, SQL/Rust runtime/контракт/зависимости не менялись.
