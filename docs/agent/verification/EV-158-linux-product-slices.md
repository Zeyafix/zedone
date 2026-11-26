# EV-158 — разделение Linux product без ослабления проверок

```yaml
step: FIX.05
date: 2026-11-26
base_revision: 639434d27c3652caf70d9b34adacf9189117848c
branch: feat/p07-04-daily-snapshots
requirements: [T16]
goal: "Проверять весь Linux product в прежних 10m каждого job после двух отмен объединённого набора."
scope: ["prototype-foundation Linux matrix", "host routing guard", "CI documentation/evidence"]
checks: ["red/green host routing guard", "точное объединение двух режимов", "матрицы/gates/cache/failure guards", "новый CI head"]
result: "Локальные guards passed; CI нового head ожидается."
reverse_and_failure: "Rollback workflow возвращает прежнее объединение. Любой failed/cancelled/skipped suite отвергается aggregate."
next: "Проверить полный CI нового head PR126, затем порученные merge/удаление при успехе."
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
