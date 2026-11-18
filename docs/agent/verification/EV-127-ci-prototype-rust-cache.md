# EV-127 — CI.21: зависимости prototype UI в собственном Rust-кэше

- Поручение: продолжить <=10m/идеал <=5m без потери гарантий. База c724c45,
  PR #109, CI.20: все 24 SUCCESS/attempt 1, full wall 10:55/runner158:31.
  Windows product 10:30, Android prototype-ui 10:44; все прочие suites <10m.
- Реальный Rust hit после default fix доказан, но cache owner Android
  product-native не собирает прототип. Prototype UI заново компилирует
  зависимости host release (2:04), Android debug (1:44) и следующий Rust
  срез (30s). Boot успел закончиться до build; runtime начался после обеих
  успешных подготовок. Повтор только кешей product-native не заполнит этот
  пробел. SDK exact hit: 50s restore, прежний полный verifier passed.
- Scope: отдельный namespace prototype-android-ui-ndk-<pin> с workspace
  prototypes -> target. Та же закреплённая Swatinem/rust-cache и полный
  Rust/environment/manifest/lock hash, только registry/git/prototype target.
  Workspace crates, CLI binaries, AVD/app data/keys не добавляются.
- Lookup-only новой identity предшествует restore. При отсутствии exact
  hit прежний Android restore служит bootstrap; после него выполняется
  обычный restore собственного namespace. При exact hit лишний legacy
  download пропускается. Частичный hit не объявляется exact; стандартный
  Rust action по-прежнему чистит несовместимые targets.
- Только prototype-ui сохраняет новый namespace после success; PR save
  остаётся в PR scope и недоступен main/другим PR по
  [GitHub cache access restrictions](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache).
  Прежний legacy namespace и его единственный main/product-native writer
  сохраняются. На неуспешном job save запрещён стандартным post-if action;
  cache-on-failure/workspace-crates/hash параметры не ослабляются.
- Guard перебирает все suites, hit empty/false/true и main/PR: один writer
  своего namespace, точные workspaces/env/keys, lookup без download/save,
  legacy только bootstrap для UI или штатный restore других suites.
- Workflow fingerprint создаст новый cold SDK key. Нужны actual initial
  outcomes/installation/save и один controlled same-head warm benchmark
  после успешного initial CI, чтобы измерить заполненный новый Rust cache.
  Все расходы/отказы/очереди/eviction сохранять; прирост cache bytes и
  вытеснение старых entries учитывать, дополнительные квоты не включать.
- Runtime/preparation/оба ABI/сценарии/assertions/24 checks/strict aggregates/
  budgets прежние. Обратимость — revert workflow; старые кеши сохраняют
  identity, удаление новых кешей возвращает cold path. Общий human/agent API,
  docs, Windows/Android scenarios и источник данных прежние. Свежая локальная
  Android UI для cache-only изменения не требуется, реальный cache runtime
  ждёт GitHub CI. Все 98 criterion definitions/states прежние.
- Windows product остаётся следующим bottleneck; этот шаг не обещает
  полного <=10m. GAP-034/035/036/037/038/039 не закрываются успехом CI.20.
  Merge/delete не поручены.

## Локальная проверка / LOG-689

- `bun prototypes/scripts/check-android-ci-suites.mjs` и
  `bun prototypes/scripts/check-host-ci-suites.mjs` passed: все suites,
  main/PR, exact/miss/empty lookup states, source/SDK/browser checks,
  pinned default regression, command/signal gates и полный scenario union.
- Actionlint 1.7.12 с ShellCheck для обоих workflows passed; Pyflakes
  недоступен/отключён. Source comment policy 394 sources, diff check и
  все 98 criterion definitions/states passed. Product/runtime/test bodies
  не менялись, свежая локальная Android UI не запускалась для cache routing.
- Проверены закреплённые restore.ts/action.yml rust-cache: lookup-only
  отдаёт cache-hit=true только при exact match, post save только после
  success при неизменённом cache-on-failure=false. GitHub scope rules
  подтверждают изоляцию PR caches от main и других PR.
- Это конфигурационная проверка, не реальный GitHub restore/save и не
  приёмка времени. Далее initial 24-check CI/cold SDK, успешное сохранение
  suite cache и один complete same-head warm benchmark с request-start.

## Публикация / LOG-690

0ba5056 в PR #109, head/body/sections/credit прочитаны обратно,
reviews/comments отсутствуют. Host44339683483 / Android44339683542 созданы
15:47:28 UTC. Все24 outcomes/new cache save/runtime/time ещё pending.
Existing watch сохраняется. После успешного initial нужен один complete
same-head warm benchmark (requests0/limit1) с request-start и фактическими
hits/legacy skip/compile/queue/wall/runner/cache bytes/eviction. Начальные
SDK cold/setup/failures/все costs сохранять. Windows product — следующий
bottleneck; цели времени не приняты. Merge/delete не поручены.

## Initial принят / полный warm benchmark / LOG-691

0ba5056 initial attempt1: все24 SUCCESS, full12:14/runner167:14.
Host10:13, Android12:14, shell12:06/SDKcold114s, fields11:03/SDKcold93s,
placements10:39/cross10:13/prototypeUI10:38. Windows product9:16,
product-build8:47. Lookup miss/legacy exact seed/own miss/build/save passed;
новый PR-scoped prototype cache724491383bytes подтверждён API. Новый exact
SDK f05143d8… saved/verifier passed. Cache17/8.13GiB, две старые SDK entries
больше не перечислены без ручного удаления. Initial cost не исключается.

GitHub принял один complete warm обоих workflows на неизменном head,
ожидаемый attempt2. Общий request-start16:02:51.349UTC; Android HTTP201
16:02:52.823UTC, host HTTP201 16:02:53.854UTC. Requests1/limit1 исчерпан,
новых automatic warm/recovery не делать. После результата нужны all24,
actual own exact hit/legacy skipped/SDK hit, fullwall отrequest/runner/queue/
cache bytes/eviction. Цель<=10m/<=5m не принята; GAP-034…039 open.
Source/assertions/budgets прежние; PR прочитан обратно, нового push нет.


## Warm результат и интеграция / LOG-692–693

0ba5056 warm attempt2: все24 SUCCESS, request16:02:51.349UTC до terminal
16:15:09UTC, full737.651s/12:18 rounded up, runner166:57. PrototypeUI7:23,
shell12:06/fields12:04. Own exact hit/legacy skipped и SDK exact hit/verifier
доказаны; cache17/8.13GiB. Initial12:14/167:14 и исторические failures/cost
сохранены отдельно. Цель<=10m/идеал<=5m не достигнута; GAP-034…039 open.

По явному поручению разработчика оптимизация остановлена, PR #109 merged
83c4643 (2026-11-18 16:18:30UTC) после exact head/base/all24 SUCCESS и свежей
CLEAN/MERGEABLE readiness без reviews/comments/reviewThreads. Merge parents
693b1fb/0ba5056, tree равен tested head. Main=origin/main83c4643, feature-ветка
удалена remote/local/tracking. Три dirty checkpoint до итоговых записей
сохранены и сверены SHA-256; другие worktrees не трогались, index пуст.

CI.12–21 интегрированы в указанном scope. Обратимость/common API/docs/
Windows-Android suites/авторитетные pins и product data сохранены; все98
definitions/states прежние. Performance target не принимается слиянием.
Active_work null, watch отключён. При возобновлении читать LOG-693/current
head, не повторять исчерпанный benchmark. Новых manual CI/оптимизаций/tests/
commit/push после merge нет; автоматические main workflows штатны.
