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
- Linux shell/paired overlap и все 24 outcomes passed после одного SDK recovery;
  единственный полный warm benchmark passed за 11:19; работа на паузе
  по запросу разработчика. Цель <=10m,
  идеал <=5m не приняты; GAP-034/035/036/037 open. После результата нужны
  wall/queue/runner/cache metadata; controlled same-head benchmark после
  успешного достижения цели. Merge/delete не поручены.

## Публикация / LOG-676

8e6333e в PR #109, head/body прочитаны обратно. Runs
44323038518/44323038661 созданы 14:19:44 UTC, результат pending.
Следующий шаг — проверить все outcomes и реальный overlap/время/cache key;
локальная проверка маршрутизации не заменяет runtime CI и не принимает <=10m.

## Отказ cold SDK / LOG-677 / GAP-038

8e6333e/product-x86_64 job 134866827043 failed до сборки/тестов.
Cache key fe0f0497… не найден. При полной установке Google CLI сообщил
`Failed to download package archive for emulator: closed`, exit 1.
Причина закрытия загрузки неизвестна; это не доказательство дефекта нового
overlap, который в этом ABI-only job не применяется. Полные log/job/steps и
оба run/jobs snapshots сохранены; последний snapshot Android in_progress,
остальные 11 jobs работают. Новый source/pins/guards/budgets не меняются.

После terminal Android workflow — один bounded rerun только этого job на
том же head, accepted=0/limit=1, request ещё не отправлен. При повторном
отказе расследовать без следующего автоматического повтора. Исходный отказ,
задержка/стоимость recovery остаются в метриках; успех не закрывает причину
GAP-038. Цель времени и runtime нового overlap пока не приняты.

## Terminal attempt 1 / recovery / LOG-678

8e6333e: 22 SUCCESS, только product-x86_64/его aggregate FAILED.
Все shell/paired runtime и настоящие Linux lifecycle/shell dispatch guards
passed. Same log подтверждает preparation во время boot, успешные device
checks до build completion и runtime после обоих успехов. Обе resolution
orders, SHA-256 restoration и OS-key/transport cleanup passed.

Initial failed wall 11:20, runner sum 155:57. Same 11:11/cross 10:32,
placements 8:47/shell 9:57. Cold SDK install соответственно 130/101/70/71s.
Новый exact SDK cache сохранён успешным owner; inventory 18 entries/10.11 GiB.
Run/jobs/steps, полные aggregate/same/placements/native logs и caches сохранены.
Это runtime acceptance overlap, но не приёмка общего времени.

GitHub принял один rerun только product-x86_64 (HTTP 201, 14:31:56 UTC)
после terminal workflow; accepted=1/limit=1, результат pending. Head/guards/
pins/budgets прежние. Дополнительных recovery не выполнять. После успеха
нужен один полный controlled warm benchmark на том же head (ещё 0/1), чтобы
измерить cached path без установки SDK; это уточнение прежнего плана после
фактических cold затрат, не обещание <=10m. Все initial failures/cost сохранять.
GAP-038 root cause остаётся open; merge/delete не поручены.

## Recovery passed / warm benchmark / LOG-679

Единственный повтор product-x86_64/job 134874758273 passed за 6:15:
SDK exact cache hit, restore 34s, полный identity/package verifier и ABI
release/debug/unsafe проверки passed. Aggregate 134878856394 SUCCESS,
весь expected набор 24 SUCCESS на неизменном 8e6333e.

Полный wall до green 18:42 включает initial failed attempt 11:20, паузу
0:52 до request и 6:30 recovery. Runner work всех реальных executions 162:18 (26 executions),
один final successful set 159:37. Inherited successes имеют новые API ids:
deduplicate только execution cost по name/started_at/completed_at, выбирать
latest attempt/name для финального набора. Полный SDK failure не исключён.
Raw recovery/run/jobs/PR snapshots и summary сохранены; GAP-038 open.

GitHub принял один полный controlled warm rerun обоих workflows на том же
head: Android attempt 3 (14:41:10 UTC), host attempt 2 (14:41:11 UTC).
Request-start 14:41:09.3761779 UTC, benchmark requests=1/limit=1 записаны
до продолжения; результат pending. Новых automatic warm/recovery не делать.
Следующий шаг — через existing watch измерить actual full wall от request,
SDK/browser hits/queue/runner/cache eviction; не принимать <=10m/<=5m заранее.
Новых source commit/push/merge/delete нет; docs checkpoint локальный.

## Warm benchmark завершён / пауза / LOG-680

Android attempt 3/host attempt 2 на 8e6333e — все 24 SUCCESS без отказов.
От общего request-start до terminal 11:18.624, округлённо 11:19; цель
<=10m/идеал <=5m не приняты. Fields 11:05, shell 10:03, Windows product 10:21.
SDK hits подтверждены; Rust action в fields/Windows product сообщил
No cache found. Предварительный host lookup был lookup-only и не означает
восстановление артефактов. Полные snapshots/logs сохранены, runner/queue/cache
итоговая сводка ещё не завершена.

По закреплённому rust-cache source и точному hash reproduction установлено:
лишний установленный compiler 1.100.0 входит в cache fingerprint вместе с
прежним pin 1.99.1. Windows ecc33ea7/af717f4b и Linux c5c76b5e/8a32f07e
точно совпадают с hash при наличии/отсутствии лишнего compiler.
Исправление ещё не реализовано; сценарии/пороги/код после 8e6333e прежние.

Разработчик явно остановил работу на этом этапе. Watch отключён, дальнейших
CI/edits/commit/push/merge/delete нет. Локальные checkpoints сохраняют точку
продолжения: расследовать/исправить fingerprint pinned Rust после явного
возобновления, не повторять исчерпанные warm/recovery на прежнем head.
