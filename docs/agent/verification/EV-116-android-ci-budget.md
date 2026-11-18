# EV-116 — CI.11: независимые Android UI и paired APK проверки

## Карточка

- База: PR #107/head 4e8a03aa0313178b7dfed31b38e4841572371951,
  ветка `feat/p06-06-column-position-conflicts`. Три исходных dirty checkpoint
  документа PR/CI сохраняются и входят в связанную историю.
- Цель: завершать полный Android CI без увеличения прежнего лимита 15m,
  разделив обычный UI и парную приёмку APK на независимые обязательные jobs.
- Требования: T08/T16, прежние CI.03/CI.10 guards, GAP-033; продуктовый
  source P06.06.2 и его UI не меняются, EV-115 сохраняет локальный охват.
- Причина: единственный Android shell rerun attempt 2/job 134296548843
  отменён по подтверждённому annotation 15m0s. Все UI/paired сценарии прошли,
  общий job не успел завершить cleanup/negative preflight; это не passed CI.
- Объём: matrix routing, CI-only установка обычного APK в fresh emulator,
  guards полноты/отказов/сигналов и docs. Число expected checks — 18 вместо 17.
- Проверки: точные маршруты default/native/каждой suite, сохранение всех
  команд/порядка, 15m и strict aggregate, отказ/сигнал на каждом шаге,
  CI-only bootstrap/его отказ, source-comment policy и новый GitHub CI.
- Обратимость/отказы: revert малого CI commit возвращает прежнюю оркестрацию;
  провал любой suite или bootstrap запрещает зелёный aggregate. Проверки
  не пропускаются и не повторяются автоматически. Кэш остаётся с одним writer.
- Tradeoff: ещё один runner повторяет подготовку SDK/build; отдельные jobs
  сокращают критический путь без параллельной сборки/управления одним эмулятором.
- Поручение: устранение CI препятствия в уже порученном PR/merge/delete.

## Результат

Реализация verified локально (LOG-626); полный новый GitHub CI ожидается.
Matrix содержит семь suites: shell сохраняет весь обычный UI, pairing получает
production APK/host fixture в отдельном runner и выполняет CI-only install,
прежний acceptance build и полный paired APK runner. Все jobs сохраняют 15m,
negative compiler/version/cleanup и strict aggregate; Rust cache writer прежний.

Fresh install может иметь пустой список файлов. В check-pairing-app.mjs
пустой manifest теперь сохраняется как пустой Map и участвует в прежнем точном
сравнении после восстановления. Неверные checksum/path строки отвергаются.
Это единственный изменённый файл из 24 fingerprints EV-115, тестовый runner;
API/runtime/UI и остальные 23 fingerprints прежние, медиа актуальны.

Passed:

- `bun prototypes/scripts/check-android-ci-suites.mjs`: точные default/native/
  suite routes, все прежние команды/порядок, ABI ownership, 15m, shared negative
  preflight, strict aggregate, invalid args, nonzero/signal каждого дочернего
  UI/paired шага. VM controls CI/emulator bootstrap без реальной установки:
  exact production APK/serial, timeout 90s, ранний отказ invalid input,
  install nonzero/signal. Manifest controls: empty/valid/invalid.
- `bun prototypes/scripts/check-host-ci-suites.mjs`: прежняя полнота/OS matrix,
  strict gates, cache owner/fallback, failure draining passed.
- `bun scripts/source-comments.mjs`: passed, 383 source files.
- `node --check` обоих затронутых scripts: passed.
- actionlint 1.7.12: Android workflow passed. ShellCheck/Pyflakes локально
  недоступны и отключены в этом запуске; CI ShellCheck остаётся обязательным.
- Определения/состояния всех 98 критериев сохранены; developer docs/очередь/
  журнал обновлены. Полная APK/UI приёмка EV-115 относится к неизменённому
  продукту, отдельный CI split ещё требует реальных Linux runners.

GAP-033 in_progress до полного CI/интеграции. GAP-032 (исходный одиночный storage
residue) не воспроизведён в attempt 2, причина неизвестна и не объявляется
исправленной. Новый ожидаемый набор 18 checks должен целиком завершиться
SUCCESS; частичный зелёный набор, cancelled/skipped либо только passed logs
не разрешают merge. Пять правил: оркестрация обратима revert; общий API и
авторитетные данные не менялись; docs обновлены; обе платформы продукта
сохраняют EV-115 scope, новый обязательный Android CI ожидается.


## Публикация CI.11 — LOG-627

PR #107/head 8a549bea7b1f0e8a10cff08d1577240935a3163b, base main/806d8e6.
Обычный CI commit/push passed, состав/noreply/privacy/doc links проверены.
Title/body/sections/два прежних hosted images/credit прочитаны обратно;
15 suites IN_PROGRESS, три агрегата ещё ожидаются, expected всего 18.
Initial readiness MERGEABLE/UNSTABLE, feedback отсутствует. Полный CI успех
не заявляется; existing watch продолжается до всех 18 SUCCESS и свежей
readiness, затем порученные merge/delete. GAP-033 ждёт CI, GAP-032 открыт.
Все 98 определения/состояния сохранены. Checkpoint локальный без нового push.


Пауза LOG-628 по поручению разработчика: CI PR #107/head 8a549be не
отменялся; watch выключен, PR linked, merge/delete отложены до явного
возобновления. Новых commit/push нет, результат CI пока не оценивался.


## Финальная CI приёмка и интеграция — LOG-629/630

После явного возобновления разработчиком PR #107/head
8a549bea7b1f0e8a10cff08d1577240935a3163b проверен заново: полный expected набор
18 SUCCESS Windows/Linux/Android, CLEAN/MERGEABLE, без reviews/comments/threads.
Android run 44161633369 attempt 1: shell/job 134316708042 — 10:30,
pairing/job 134316708019 — 11:12. Обе suites полностью завершили environment
и negative compiler/version/cleanup; pairing log подтверждает fresh production
APK bootstrap, прежние TLS/API/CLI/Keystore/column-position/recovery сценарии,
восстановление APK/каждого app file SHA-256 и очистку OS keys/transport rules.
Host run 44161632829 и все три strict aggregates SUCCESS. Это реальный запуск
с cache hits, не гарантия холодного либо будущего времени. Строгие assertions,
15m, версии и вся матрица сохранены. GAP-033 resolved в указанном объёме;
GAP-032 остаётся open, исходная причина одиночного storage residue неизвестна.

Title/body/Testing/оба hosted images/credit прочитаны обратно после обновления
результатами. Merge API закрепил проверенный head; PR MERGED в
531cf34aefe019669b924900fae0366e79ab6611. Родители [806d8e6, 8a549be], дерево
51316125d5b3a300c4cb0a9441bc7d72bfecc34b совпало с tested head. Локальный main
fast-forwarded, четыре dirty checkpoint документа сохранены/восстановлены
по SHA-256. Remote ветка удалена с lease точного tip, локальная — обычным -d;
отсутствие обеих подтверждено. P06.06.2/CI.11 done; active_work null,
watch выключен. Все 98 определения/состояния сохранены. Итоговый checkpoint
локальный без дополнительного commit/push.

Пять правил: отмена/обратные действия/сохранность, общий UI/HTTP/IPC/CLI,
обновлённые docs, Windows/Android в scope EV-115 и один авторитетный набор
данных подтверждены. Следующий готовый шаг — отдельная совместная приёмка
P06.06; полный T09, GAP-028/031, physical arm64, Tailscale и измерения отдельно.
Новых шагов реализации не начинали. Поручение PR/условный merge/delete завершено.
