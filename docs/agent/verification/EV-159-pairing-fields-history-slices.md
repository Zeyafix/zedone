# EV-159 — независимая парная история и безопасная диагностика host fixture

```yaml
step: FIX.06
date: 2026-11-26
base_revision: fde70eff88d8ff07afbf6bd3e93e7ca501eb38a6
branch: feat/p07-04-daily-snapshots
integration_revision: 0928f9452a3a0909e87e8331da287b4c83b170dc
tested_revision: 84922dffb6d2987d4fdbd7edff87bde522cbb1a5
requirements: [T09, T16]
goal: "Сохранить всю парную приёмку fields/history в пределах прежнего lifetime fixture и видеть место отказа без секретов."
scope: ["paired CI routing/preparation", "acceptance runner diagnostics", "guards/docs"]
checks: ["реальный timeout240s host fixture", "Rust harness capture proof", "red/green routing/privacy guards", "Bash preparation/failures", "ShellCheck/Actionlint", "новый полный CI"]
result: "passed в проверенном scope; интегрировано PR126/0928f94 после CI31 на84922df."
reverse_and_failure: "Сценарии и assertions не удаляются; каждый runner обязан подтвердить права/expiry/reverse/cleanup, aggregate отвергает ошибку любого runner."
next: "P07.04/.1–.4 и FIX.05–07 done; готов P07.05, не начат. Полный P07/T10 отдельно."
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


LOG-852: FIX.06 опубликован в PR126/head22ad15567d200c7fee17fcf55e3b545e7d55de76,
новый полный CI29 pending. Дополнительно диагностирован старый fde70ef native
job139910750416 CANCELLED15m до завершения storage/search/probes, без assertion
failure. Исходный log/metadata/annotations сохранены; GAP-055 open, прежние74
проблемы сохранены, всего75. Новая ревизия не заменяет этот исход на PASS.


LOG-853: новая диагностика22ad155 дала session.unlink/access_denied240068ms и
host reason=fixture_timeout; job cancelled15m. History и native того же head
SUCCESS. FIX.07/EV-160 отделяет три field orders с прежним body/assertions/
240s/15m, точный union/guards passed. Runtime/UI прежние; новый CI31 pending,
исторические GAP-047/054/055 сохраняются.


## Интеграция — LOG-858/859

[PR #126](https://github.com/Zeyafix/zedone/pull/126) merged в
`0928f9452a3a0909e87e8331da287b4c83b170dc`, checked head `84922dffb6d2987d4fdbd7edff87bde522cbb1a5`.
Все31 checks SUCCESS: шесть Windows, четыре Linux, восемнадцать Android
и три strict aggregates. Windows product и native Android прошли после
одного targeted recovery каждый; прочие успешные outcomes сохранены.
Первоначальные Unavailable/15m и прежние field failures сохранены;
GAP-042/046/047/054/055 остаются open в неизвестном scope. Никакие
assertions/240s fixture/15m Android/20m Windows/SQLite policy не ослаблены.

Все три field order/choice vectors и history прошли с общими правами,
повторами/Keystore restart/recovery/reverse и восстановлением исходных
APK/файлов по SHA-256, очисткой keys/transport/emulator. Windows replay
тест теперь passed; storage370/search19371ms/contract/invalid11 passed.
Native storage371/search11226ms, парные TLS/pending-request probes,
идентичный контракт и cleanup passed. Это фактические логи нового head;
исходные отменённые jobs не считаются успешными.

Перед merge сверены head/base/behind0/mergeable/nondraft, полный feedback
пуст и все31 outcomes SUCCESS. Merge содержит head и имеет то же дерево.
Main=origin/main/0928f94; feature-ветка удалена local/remote/tracking,
три исходных dirty checkpoint сохранены и проверены SHA-256.
P07.04/.1–.4 и FIX.05/06/07 done; A13/A43 passed по матрице EV-157,
остальные96 состояния/все98 определения и75 ID/состояния проблем прежние.
Физический arm64 runtime и реальный power-loss не проверены и не заявляются;
полный P07/T10 partial. Обратимость, общий API, docs, паритет в проверенном
scope и один авторитетный граф соблюдены. Далее P07.05 готов, не начат.
Итоговый checkpoint локальный без дополнительного commit/push; raw CI/media/
машинная proof остаются в ignored reports. Post-merge push CI не подменяет
пройденный PR CI и здесь не заявляется.
