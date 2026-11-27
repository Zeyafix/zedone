# EV-158 — разделение Linux product без ослабления проверок

```yaml
step: FIX.05
date: 2026-11-26
base_revision: 639434d27c3652caf70d9b34adacf9189117848c
branch: feat/p07-04-daily-snapshots
integration_revision: 0928f9452a3a0909e87e8331da287b4c83b170dc
tested_revision: 84922dffb6d2987d4fdbd7edff87bde522cbb1a5
requirements: [T16]
goal: "Проверять весь Linux product в прежних 10m каждого job после двух отмен объединённого набора."
scope: ["prototype-foundation Linux matrix", "host routing guard", "CI documentation/evidence"]
checks: ["red/green host routing guard", "точное объединение двух режимов", "матрицы/gates/cache/failure guards", "новый CI head"]
result: "passed в проверенном scope; интегрировано PR126/0928f94 после CI31 на84922df."
reverse_and_failure: "Rollback workflow возвращает прежнее объединение. Любой failed/cancelled/skipped suite отвергается aggregate."
next: "P07.04/.1–.4 и FIX.05–07 done; готов P07.05, не начат. Полный P07/T10 отдельно."
```

PR126/head639434d дважды отменён по annotation `The job has exceeded the maximum
execution time of 10m0s`. Первая попытка139892311433 достигла финальных type
markers; единственный recovery139901177300 завершил обычные tests/search,
release за4m33s и часть unsafe guards, затем CANCELLED. Aggregate139906639294
правильно отверг RESULT=cancelled. Дополнительных same-head повторов нет.
Исходные logs/job metadata/annotations сохранены в ignored p07-04-complete;
первопричина длительности отдельных компиляций не объявляется доказанной.

Linux product разделён по уже работающей модели Windows:

- product: `check-product.mjs --host-tests`, все fmt/Clippy/Rust/client/contract/
  TypeScript и isolated search проверки;
- product-build: `--host-build`, release workspace/all-targets/all-features,
  source-comment negative/recovery и unsafe guards;
- shell и prototype-ui сохраняют полные команды без изменения сценариев;
- Linux10m, max-parallel2/fail-fast false и strict aggregate1m прежние.

Composite action уже поддерживает product-build для обеих ОС: отдельный cache
namespace и read-only seed из product при первом запуске. Код action не менялся.
Нет объединённого ключа для конкурентных частичных outputs. Android15m,
Windows20m, все fixture budgets и пользовательский/агентский продукт прежние.

Guard сначала изменён под новую Linux matrix: исходный workflow штатно
отклонён на отсутствии product-build (`ci-split-red.log`). После изменения
workflow полный `bun prototypes/scripts/check-host-ci-suites.mjs` passed
(`ci-split-green.log`). Он доказывает точное сохранение всех команд полного
product запуском двух срезов, standalone search/probes, условия выбора команд,
обе OS matrices, strict aggregates, ошибки/сигналы и cache ownership/seed.
Зависимые tool-cache/Rust и main UI slicing guards тоже passed.

Полные продуктовые Windows/Android/UI checks EV-157 не перезапускались ради
изменения CI: runtime/контракт/снимки прежние. Новый Linux CI обязателен до
вердикта, один одинаковый код не доказывает выполнение на Linux. Пять правил
AGENTS соблюдены в scope инфраструктуры; новых пользовательских действий нет.
GAP-042/047/054 не закрываются локальным guard. Все98 определения/состояния
критериев и74 ID/состояния проблем сохранены. Целевой текущий состав28 checks:
6 Windows+4 Linux+15 Android+3 aggregates; будущие timing/cache hits не обещаны.

Actionlint1.7.12 с установленным ShellCheck прошёл для изменённого workflow
(локальный Pyflakes отключён как отсутствующий; Python-код не менялся).
Comment policy464 sources и diff-check passed. Все338 product fingerprints
EV-157 совпали; локальная обратимость/права/паритет/единый источник прежние.

## Публикация — LOG-850

CI commit `fde70eff88d8ff07afbf6bd3e93e7ca501eb38a6` опубликован в
[PR #126](https://github.com/Zeyafix/zedone/pull/126). Staged scope: workflow,
guard, CI docs и исходные сохранённые checkpoints; продукт/UI/338 fingerprints
прежние. Новый полный CI28 checks pending; старые cancel/recovery сохранены.
Предыдущий Android recovery был running до push и не принимается как PASS.

LOG-851: на fde70ef Linux product139910675373/product-build139910675338,
shell/prototype-ui и aggregate139918234611 SUCCESS. Охват и прежние10m
подтверждены новым CI; полный PR ещё ждёт Android, интеграция впереди.


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
