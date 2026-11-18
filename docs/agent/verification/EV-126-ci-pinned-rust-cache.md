# EV-126 — CI.20: стабильный Rust cache fingerprint

- Поручение: продолжить ускорение CI до <=10m, идеально <=5m, без потери
  проверяемых гарантий; тесты разрешено менять. База 8e6333e/PR #109.
- Предыдущий полный warm benchmark: 24 SUCCESS/11:19; fields 11:05,
  shell 10:03, Windows product 10:21. Rust action сообщил cache misses,
  хотя SDK hit подтверждён. LOG-680 воспроизводит четыре hash с/без
  лишнего runner compiler 1.100.0 при используемом pin 1.99.1.
- [Закреплённый исходник rust-cache](https://github.com/Swatinem/rust-cache/blob/d1527c8f67ef9af357290475d4af4b0fab8f12e0/src/config.ts)
  включает все установленные компиляторы в environment fingerprint.
  Отключение Rust/environment hash не используется.
- Scope: общий PowerShell helper после прежней установки pinned Rust и до
  Rust restore на Windows/Linux host и Android CI. Только при
  GITHUB_ACTIONS=true/RUNNER_ENVIRONMENT=github-hosted. Проверить compiler
  release/host и установленный pin до изменений, удалить остальные toolchains
  через rustup, затем требовать ровно pinned inventory. На локальных и
  self-hosted машинах отказ до вызова Rust. На disposable runner default
  выбирается явно до удаления; targets/components pin не меняются.
  Версия берётся только из environment-versions.json.
- Проверки: реальный helper с подставными rustc/rustup для Windows/Linux
  triplets, pinned-only и extra compiler, local/self-hosted refusal,
  compiler/list/uninstall failures, malformed identity/list, missing pin,
  postcondition после успешного, но не изменившего inventory uninstall.
  Первый fixture ошибочно читал args внутри Where-Object; исправлен захватом
  аргумента до вложенного scope. Финальный direct fixture passed.
- Обратимость: revert CI helper/call sites, одноразовый runner можно создать
  заново; пользовательские установки и данные не затрагиваются. Общий
  human/agent API, Windows/Android scenarios и источник продуктовых данных
  прежние. Документация CI обновляется, 98 criterion definitions/states прежние.
- Cache namespace/hash inputs/restore fallback/save owners не ослабляются,
  все 24 checks/strict aggregates/сценарии/пороги/budgets сохраняются.
  Изменение Android workflow создаёт новый cold SDK key; учитывать его
  отдельно от Rust hit. Реальная работоспособность/время ещё pending CI.
- План: подходящие routing/tool-cache/policy/YAML проверки, публикация в
  существующий PR #109, фактические run/jobs/steps/cache logs и полное время.
  Старые warm/recovery 8e6333e исчерпаны; повторно их не запускать.
  При необходимости один controlled warm benchmark нового head после
  успешного первого прогона, с отдельным учётом всех расходов. Merge/delete
  не поручены, <=10m/<=5m заранее не принимаются.

## Локальная проверка / LOG-682

- Оба полных routing commands прошли: `bun prototypes/scripts/check-host-ci-suites.mjs`
  и `bun prototypes/scripts/check-android-ci-suites.mjs`, включая новый реальный
  PowerShell helper со stub-командами, SDK cold/hit identity/version rejection,
  browser cache, точный suite union, strict gates и failure/signal paths.
- Actionlint 1.7.12 с ShellCheck passed для обоих workflows; Pyflakes
  недоступен и отключён. Source comment policy passed: 394 sources.
  `git diff --check` passed. Все 98 criterion definitions/states сверены
  с исходным checkpoint и сохранены. Product source не менялся.
- Незавершённая сводка прошлого warm benchmark дополнена: runner work
  153:15 для ровно 24 executions; full wall 678.624s по terminal workflow,
  last job закончился на секунду раньше. Cache inventory 18/10.11 GiB.
  Per-job created/start/completed и request-to-start сохранены локально;
  ожидание matrix/aggregates не выдаётся за чистую очередь runner.
- Windows/Linux triplets в fixture не заменяют реальный Linux job:
  новый CI ещё pending. Это CI-only изменение; свежая локальная Android UI
  не запускалась. Пять правил соблюдены в указанном инфраструктурном scope;
  паритет нового исполнения на обеих CI-платформах ждёт GitHub результата.

## Публикация / LOG-683

f2b9975 в PR #109. Head/base/body/sections/credit прочитаны обратно,
reviews/comments отсутствуют. Initial host 44332008036 / Android 44332008034
созданы 15:07:48 UTC, все 24 outcomes/cache/runtime/time ещё pending.
Watch продолжает текущий тред; предыдущий тред не изменяется.
Следующий шаг — фактический pinned inventory/restore, SDK cold cost,
все executions/queue/wall/cache; при необходимости один complete warm
benchmark нового head после успешного initial CI. Старые лимиты не сброшены,
цель времени не принята. Merge/delete не поручены.

## Initial failure / default correction / LOG-684

f2b9975 terminal: 20 SUCCESS, Linux product CANCELLED по прежнему 10m
во время unsafe controls, shell/оба aggregates FAILED. Обычные Rust tests
и isolated Linux search 1000 matches/18419ms passed. Full wall 12:12,
runner work всех 24 executions 182:28; raw logs/steps/annotations сохранены.

Первый helper не устранил cache miss: rustup автоматически переустанавливает
удалённый default stable после печати списка toolchains. Windows/Linux logs
подтверждают auto-install и прежние environment hashes. Постусловие по
возвращённому списку не доказывало фактический pinned-only inventory.
Теперь до удаления выбирается pinned runner default, проверяется active;
fixture воспроизводит delayed side effect и независимо сверяет итоговое
состояние. Отказы default/active и несовпадающий override отклоняются.
Меняется default только одноразового runner; локальный/self-hosted отказ
до Rust остаётся. Cache key inputs/owners/SDK workflow fingerprint прежние.

Shell упал отдельно на точном textarea value в Markdown/10000ms;
диагностика не содержит actual value/focus, причина неизвестна (GAP-039).
Исходный timeout/assertions/scenarios прежние. Новый CI нужен из-за
исправления Rust helper; это не скрытый same-head retry shell. Успех не
закрывает историческую причину. Оба aggregates корректно отвергли неполный
набор. SDK owner сохранил новый exact key, snapshot 17 entries/9.54 GiB;
предыдущий SDK и Linux prototype-native Rust entry больше не перечислены.

Свежий Linux runtime/cache hit и весь новый CI pending; до него цель
<=10m/<=5m не принимается. Пять правил в CI scope и 98 definitions/states
сохраняются; дополнительной локальной Android UI для helper-only fix нет.

Финальные local guards default fix passed: оба полных host/Android routing,
expanded Windows/Linux fixtures и comment policy 394 sources. Отдельный
negative control запускает старый helper f2b9975 на расширенной fixture:
helper возвращает success, затем независимая проверка final state отвергает
восстановленный default/inventory. Новый helper проходит тот же контроль.
Workflow/YAML/SDK fingerprint не меняются; прежний actionlint сохраняет
применимость. Diff check и все 98 definitions/states passed.
