# EV-159 — независимая парная история и безопасная диагностика host fixture

```yaml
step: FIX.06
date: 2026-11-26
base_revision: fde70eff88d8ff07afbf6bd3e93e7ca501eb38a6
branch: feat/p07-04-daily-snapshots
requirements: [T09, T16]
goal: "Сохранить всю парную приёмку fields/history в пределах прежнего lifetime fixture и видеть место отказа без секретов."
scope: ["paired CI routing/preparation", "acceptance runner diagnostics", "guards/docs"]
checks: ["реальный timeout240s host fixture", "Rust harness capture proof", "red/green routing/privacy guards", "Bash preparation/failures", "ShellCheck/Actionlint", "новый полный CI"]
result: "Локальные проверки passed; Android CI нового head ожидается."
reverse_and_failure: "Сценарии и assertions не удаляются; каждый runner обязан подтвердить права/expiry/reverse/cleanup, aggregate отвергает ошибку любого runner."
next: "Полная матрица29 checks PR126, затем порученные merge/удаление при успехе."
```

На fde70ef Android pairing-fields139910750232 прошёл text/три combined matrices/
paired history/exchange recovery и получил `unavailable`. Последний timestamp
12:43:33.489, диагностический host exit101 в12:43:40.922 при elapsed247530ms:
первая ошибка соответствует 240097ms от serverStarted, почти точно прежнему
240s deadline. Это временная корреляция, не сохранённый stacktrace исходного CI.
Оригинальные stderr/stdout артефакты workflow не публикует; partial raw log и
metadata/annotations остаются в ignored p07-04-complete. GAP-047 не закрывается.

Контролируемая настоящая Windows fixture `device_host::tests::acceptance_server`
без stop и UI flags завершилась exit101 за240811ms с `fixture timeout`.
По исходникам listener/OS slots/SQLite закрываются до этого assert; тестовая
fixture использовала собственный временный namespace. Panic найден в stdout,
не stderr. Настоящий CLI `space.list` после остановки вернул exit1/unavailable,
как новый CI-отказ. Это объясняет, почему общий исправленный stderr path FIX.03 сам по
себе не давал классификацию timeout: Rust harness захватывает panic по умолчанию.
Отдельный настоящий `rustc --test` reproducer подтвердил before/after:
без `--nocapture` timeout только stdout, с ним — только private stderr, оба exit101.

Из общего долгого CI runner выделен уже существующий режим `--history`:

- pairing-fields: checkTextExchange и checkFieldConflictsExchange;
- pairing-history: checkHistoryExchange с обоими scope/directions и отдельным
  пространством, проверка сохранности истории соседнего пространства;
- pairing-placements: прежние placement/column-position проверки;
- полное объединение этих трёх совпадает с прежним `--standard`, все пять
  exchange-групп остаются ровно по одному разу. `--standard` не меняет охват.

Общие bootstrap/production install/acceptance build/права/TLS/transport recovery/
grant expiry/revocation/replay/reverse/cleanup обязательны для нового runner.
Проверки/assertions, fixture240s/UI600s, Android job15m и aggregate1m не менялись.
Отдельный historySpace больше не создаётся ненужному fields runner; само
содержимое его тестов не менялось. Новая matrix29 checks включает16 Android
suites; runtime и budgets продукта ZeDone не затронуты.

`call` хранит служебный контекст ответа в WeakMap и не меняет wire envelope.
`ok` сообщает operation (включая внутренний peer command), сторону, host exit
и elapsed. Параметры/адреса/ключи/пины/тексты не печатаются. `--nocapture` лишь
позволяет существующей безопасной классификации увидеть host panic в private
stderr; сами log-файлы не становятся публичным stdout CI.

Локальные проверки:

- Обновлённый routing guard отверг исходную matrix без history (red), после
  правки весь `check-android-ci-suites.mjs` прошёл, включая точный union,
  production/acceptance порядок, failure/signal/cleanup и safe diagnostics.
- VM проверил успешный/отказанный ответ при живом/завершённом host: envelope
  побайтово прежний, operation/side/exit/elapsed присутствуют; private request,
  address/key/pin не утекли. Аргумент `--nocapture` обязателен в guard.
- `bash scripts/check-android-shell-preparation.sh`: passed для всех selectors,
  включая history, exact argv/build ownership, invalid/CI refusal и child exit23.
- Host routing/tool-cache/Rust/main-UI guards, Bash syntax и comment policy464
  sources passed. Standalone ShellCheck0.11.0 двух изменённых Bash scripts passed.
- Совместный Actionlint→ShellCheck завершён по локальному guard25s, не считается
  PASS. Независимый Actionlint1.7.12 без отсутствующего Pyflakes и shell integration
  passed; изменённые scripts отдельно прошли ShellCheck. Сохранён timeout invocation.

Новый APK/Device UI локально не запускался: это CI orchestration, UI/runtime
не меняются. Новый реальный Android CI обязателен; одинаковый runner не заменяет
платформенную проверку. Пять правил AGENTS соблюдены в инфраструктурном scope,
все98 определения/состояния и74 ID/состояния проблем сохраняются.
Начальные access_denied/cleanup GAP-047 остаются неизвестными без stacktrace;
повтор без изменения кода на fde70ef не используется вместо исправления.
