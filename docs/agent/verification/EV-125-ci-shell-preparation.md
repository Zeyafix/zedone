# EV-125 — CI.19: shell/paired APK во время запуска CI-эмулятора

- База cf03942: все 24 SUCCESS/attempt 1. Полный CI 10:40; critical
  pairing-placements 10:31, fields 10:10. Linux probe прошёл 9916ms.
- Placements сначала собирал APK 2:57, затем environment boot/probes около
  0:57; runtime начинался только после них. Raw steps и полный log сохранены.
- Scope: перенести прежнюю x86_64 APK build shell/четырёх paired suites в
  CI-only helper во время boot через существующий `--prepare` driver.
  Только shell собирает прежний host browser fixture. Список пяти suite
  явный, лишние/неизвестные args и запуск без CI отклоняются до side effects.
- Перед runtime обязательны успех build и все прежние KVM/compiler/device
  проверки. Сам lifecycle driver не меняется; его настоящие Linux fixtures
  ранее passed на cf03942: оба порядка, build/boot/runtime failures, signal,
  descendants cleanup. Production KVM predicate не меняется.
- Production bootstrap, acceptance APK rebuild, сценарии, result assertions,
  таймауты, APK/app-file restoration/OS-key cleanup, обе ABI и 24 checks/
  strict aggregates остаются прежними. Нет product/API/schema/dependency changes.
- Shell fixture запускает helper с подставными командами и сверяет точные
  argv/порядок для всех пяти suite, отсутствие лишнего host build в paired,
  target dir, invalid/missing/extra args, CI rejection и остановку на build
  failure с кодом 23. Локальный Git Bash passed; без реального SDK/эмулятора.
- Android routing/tool-cache guards, ShellCheck пяти preparation/environment
  scripts, actionlint с ShellCheck (без unavailable Pyflakes), comment policy
  392 sources и diff passed. Первый actionlint отверг literal matrix pattern;
  исправлено передачей suite через env. Первая локальная fixture зависела от
  отсутствующего jq; теперь возвращает явный fixture token, проверяя вызов
  `.rust` и передачу результата cargo без зависимости от локального jq.
- SDK fingerprint содержит workflow: следующий CI имеет новый cold key,
  package/hash verification и один успешный cache writer сохраняются.
  Изменение boot/build contention и реальная длительность требуют CI;
  локальные dispatch checks не принимают performance target.
- Обратимость: revert workflow/helper, данных и миграций нет. Общий API
  человека/агента, docs, Windows/Android parity и авторитетные pins/data
  сохранены. 98 criterion definition/state rows не меняются.
- Новый Linux shell/paired overlap/все 24 outcomes ещё pending. Цель <=10m,
  идеал <=5m не приняты; GAP-034/035/036/037 open. После результата нужны
  wall/queue/runner/cache metadata; controlled same-head benchmark после
  успешного достижения цели. Merge/delete не поручены.
