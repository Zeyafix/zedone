# EV-068 — выполнение карточки через общий API

## Рабочая карточка

```yaml
step: P04.04
base_revision: c245fa8, чистая main
branch: feat/p04-04-card-completion
goal: Явно установить и снять выполнение карточки независимо от колонки.
requirements: [T07, A02, A07, ADR-0007, ADR-0008, ADR-0013]
dependencies_evidence: [P04.03/EV-067/PR58/2bbc0db]
scope: [core card completion, storage completed register, Service HTTP/CLI/IPC, Rust-derived contract, tests, docs]
checks: [bun scripts/check-product.mjs, bun scripts/check-product.mjs --android, bun run ui:check, bun run ui:build, git diff --check]
reverse_and_failure: completed=false отменяет completed=true; stale/ABA, права, конфликт/корзина, rollback и неизвестный результат без повторного эффекта.
result: verified_locally
next: Ревью/публикация по поручению, CI точного head и интеграция; затем P04.05.
```

Команда `card.set_completed` задаёт bool явно, использует общую version всей
карточки и прежние guard/replay/транзакцию. Отметка не меняет текст, историю,
колонку, положение, теги или корзину. UI остаётся P05.03; перемещения — P04.05.
Новые зависимости и SQL-миграция не нужны.

Ориентир 300 строк превышен из-за проверок неделимой пары установка/снятие
через все входы: актуальность/ABA/конкуренция, права/корзина/конфликт,
SQL/commit-отказы и сохранность исходного намерения. Runtime расширяет прежний
обработчик; независимых изменений и новых моделей хранения нет.

## Реализация и проверки

`SetCardCompleted`/`card.set_completed` включены в Service и Rust/OpenAPI/TS/
standalone validator. Прежний обработчик мутаций проверяет актуальность всей
карточки, ReadWrite, собственные конфликты и активность родителей. Запись bool
и receipt проходят одну SQLite/Automerge-транзакцию; текстовая история не меняется.

| Сценарий | Доказательство |
| --- | --- |
| Установка/снятие | Обе колонки, сохранность всех остальных полей, noop, перезапуск с установленной отметкой, снятие после перезапуска |
| Актуальность | Новое описание → отказ старым true и false; матрица title/description/tags/completed/position/column/lifecycle; ABA и старой правке текста отказано после отметки |
| Повторы | Исходный receipt после снятия отметки и после перезапуска; подмена bool при том же ID отклонена; неизвестный commit подтверждается replay |
| Защита | Read запрещает даже noop, истёкший/отозванный доступ запрещает изменение/replay; корзина карточки и родителей, все собственные конфликты блокируют изменение |
| Сохранность | AfterWork/AfterChanges/AfterResult/BeforeCommit и настоящий SQL abort receipt откатывают отметку; AfterCommit даёт OutcomeUnknown; гонка текста/выполнения допускает одного writer |
| Общие входы | HTTP/IPC/настоящий CLI дают те же результаты, повторы и stale/Read-отказы; string/number/null вместо bool отклонены всеми тремя входами |
| Клиент | true и false сохраняют первоначальные bool/expected/ID после stale, restart и потерянного ответа IPC; явный отказ не повторяется автоматически |

Windows: `bun scripts/check-product.mjs` passed — 192 Rust-теста
(12 core, 7 contract, 157 storage, 16 adapters), 17 клиентских/135 assertions,
fmt/Clippy с -D warnings, release всех targets/features, 7 unsafe-отказов,
побайтовая сверка контракта/типов/валидатора. Дополнительно после расширения
сценария перезапуска все 15 cards storage-тестов, fmt/Clippy passed;
runtime после полного прогона не менялся.

`UI_NODE_VERSION=26.8.1 bun run ui:check` passed: vue-tsc, production Vite,
fmt/Clippy/tests shell, host/example и прежние браузерные HTTP/focus/restart/
themes/failure-сценарии на подготовленном Chromium 156.0.8078.4. Использован
предусмотренный локальный override установленного Node 26.8.1; база CI 24.22.1
не менялась. Первый запуск без override остановился на проверке версии среды.
Первый product-прогон выявил не обновлённый второй счётчик числа API-команд
в тесте контракта; он исправлен до итогового успешного прогона.

Android: `bun scripts/check-product.mjs --android` passed — те же 192 нативных
Rust-теста на x86_64, включая настоящий CLI, прежние crash/reopen и дополненный
перезапуск отметки; 17 клиентских/135 assertions. Release всех targets/features
обеих ABI x86_64/aarch64, 14 unsafe-отказов, побайтовое совпадение Android/host
контракта. В сумме Windows/Android — 21 unsafe guard.
Эмулятор открыт через Device; build/native runner выполнялись штатным CLI,
UI через adb не управлялся. Собственный sandbox удалён в finally; Device-сессия
закрыта, эмулятор возвращён в исходное выключенное состояние.
Полный diff/fmt/Clippy и git diff --check passed; 344 относительные цели ссылок
в затронутых Markdown проверены (без новой проверки всех якорей журнала).
Машинные логи только в игнорируемом docs/reports/p04-04-*-complete.log
и p04-04-storage-final.log.

## Границы приёмки

Это API-срез P04.04; UI карточек и полный A02/A07 остаются P05.03.
Перемещения проверены только доверенным fixture на общей модели и не приняты
как публичная операция P04.05. Теги, серии истории, группы и продуктовый обмен,
физический arm64 и release APK не принимаются. T07/A02/A07 остаются partial.
PR/новые CI/интеграция не выполнялись.

## Пять правил

- Обратимость: true/false, сохранность отметки при корзине/восстановлении.
- Человек и агент: общий Service/core через HTTP/CLI/IPC; UI остаётся P05.03.
- Документация: user/agent/development/architecture, очередь и журнал.
- Паритет: одинаковые 192 нативных теста Windows/Android, release двух Android ABI;
  UI карточки/APK/физический телефон вне текущей приёмки.
- Источник истины: прежние SQLite/Automerge/card.v1 и Rust-derived контракт;
  клиент сохраняет только исходное намерение доставки.
