# EV-118 — CI.12: удалить подтверждённые повторы тестов

## Карточка

- Поручение разработчика: удалить избыточные тесты в отдельной ветке после
  [аудита](../../development/test-audit-2026-11-18.md).
- База: main/693b1fb, ветка `test/remove-redundant-checks`.
- Цель: один обязательный владелец одинаковых infrastructure negative controls,
  убрать повтор client delivery из Android и host contract из arm64-среза,
  объединить старый description-stale сценарий с новой матрицей всех полей.
- Build-only product-arm64 не требует x86_64 AVD. Production таймеры и их
  тесты, уникальные prototype/UI/native/platform/ABI/unsafe проверки сохраняются.
- Standalone Android workflow сохраняет полную host contract/TS проверку
  в product-native; сверка экспорта настоящего Android обязательна. Между
  независимыми workflows не вводится доверие к непроверенным артефактам.
- Полнота: прежние семь Android suites, 15m/strict aggregate и обе host OS.
  Общий negative-control owner — обязательный prototype-native job.
- Проверки: маршрутизация и nonzero/signal, отсутствие client/contract/ADB
  в arm64-only пути, сохранение Android runtime/probes/export, Rust acceptance,
  fmt/Clippy, comment policy, syntax и workflow validation.
- Обратимость: revert возвращает прежнюю оркестрацию; пользовательские данные,
  API и schema не меняются. Отказ владельца controls запрещает успех aggregate.
- Четыре исходных dirty документа сохранены побайтово в ignored baseline;
  EV-117 не меняется. Продуктовые критерии/GAP/DEC не переоцениваются.
- Состояние: verified локально; новый CI/commit/push/PR не выполнялись.

## Реализация

- R1: Android-срезы больше не запускают host Bun delivery tests. Полные
  Windows/Linux product jobs по-прежнему запускают все 27, с прежним
  параллельным ожиданием обеих сторон и сохранением отказов.
- R2: arm64-срез не повторяет host export/schema/validator/TS checks.
  Product-native сохраняет их полностью для независимого Android workflow,
  включая сравнение экспорта настоящего native Android. Между отдельными
  workflows остаётся осмысленное повторение host contract check: удалять
  последнего владельца из standalone Android было бы потерей гарантии.
- R3: installer fake-Cargo и compiler/version/cleanup negative controls
  имеют единственного обязательного владельца `prototype-native` вместо
  семи запусков. Сами тела сценариев не сокращены. Прежний aggregate требует
  успеха всей семиэлементной матрицы, `continue-on-error` отсутствует.
  Владелец также выполняет Android routing guard прямо в Android workflow:
  изменение только его YAML проверяется даже без host workflow trigger.
- R4: `product-arm64` вызывается напрямую через прежний dispatcher;
  `checkAndroid` не требует serial/getprop для build-only arm64. Оба
  build-only jobs не выполняют KVM/emulator-libraries setup. Реальные runtime
  preflight/собственные AVD и очистка пяти runtime jobs сохранены.
- R5: удалён один старый 89-строчный description-stale тест. В новой матрице
  сохранены все поля/три adapters/reverse/restart, добавлена явная проверка
  свежего completion без изменения истории после restart.
- Comment-policy Bun test получает абсолютный путь, как client tests,
  чтобы не запускать общий discovery по локальным build-каталогам.
- R6 не применяется: существующие timeout/retry tests полезны; production
  таймеры, scheduler и сами тесты не меняются.

## Проверки

- Адресный полный adapters `commands`: **45 passed**, включая обе acceptance
  матрицы и перенесённый completion/history assert; fmt passed.
- Настоящий `check-product.mjs --android-arm64`: **exit 0**, release all-targets/
  all-features, восемь Cargo-target unsafe guards (включая build script)
  и new-source-comment rejection/recovery.
- Host/Android CI guards: **exit 0**. Новый VM control исполняет настоящий
  product driver с fake process boundary: ABI ownership, отсутствие client
  повторов, host contract owner, отсутствие ADB/runtime у arm64 без serial,
  native execution/оба paired probes/device-export, отказ при mismatch,
  physical serial, nonzero build и signal. Оба OS/gates/caches прежние.
- `check-ci-tauri.sh` через Git Bash/fake Cargo: **exit 0** на cache miss/hit,
  wrong/broken cached version, compiler exit 23, cleanup и wrong installed version.
- Source-comment policy: **385 files**, exit 0. Node syntax всех трёх
  изменённых `.mjs` и actionlint 1.7.12: exit 0. В локальном actionlint
  ShellCheck/Pyflakes отключены: их executable отсутствуют; штатный CI
  ShellCheck не удалён.
- Первая попытка нового VM control остановилась на неверном synthetic UUID
  в собственной fixture; исправлена fixture, production guard не ослаблен.
- Первая попытка полного host product ошиблась из-за собственного параллельного
  adapters test: linker LNK1104/Windows exe lock и Access denied на CLI.
  Адресный набор завершился успешно; полный последовательный прогон затем
  прошёл, первая попытка не считается passed. Полные итоги host/native ниже.

Все 98 определения/состояния продуктовых критериев, GAP и DEC сохранены.
Исходный EV-117 совпадает с baseline SHA-256; история журнала и старые строки
очереди сохранены. Новый remote CI и публикация не выполнялись.

Финальный последовательный `bun scripts/check-product.mjs`: **exit 0**.
384 Rust tests (один объединённый вместо двух), 27 client tests/280 assertions,
28 comment tests, fmt/Clippy с deny warnings, release all-targets/all-features,
восемь Cargo-target unsafe guards и comment rejection/recovery, точный
OpenAPI/TS/validator, положительный typecheck и 11 negative type diagnostics.
Исходная неуспешная попытка и финальные логи сохранены раздельно в ignored reports.

Финальный `bun scripts/check-product.mjs --android-native`: **exit 0**.
Эмулятор AOSP API33/x86_64 открыт через Device. Выполнены host contract/schema/
validator/TS/11 negative diagnostics, release x86_64 и восемь unsafe guards,
comment rejection/recovery, **381 native Rust tests**, включая обновлённую
acceptance матрицу; оба paired native TLS/pending-request probes и точное
сравнение Android export-contract с host. Host client tests не запускались.
Собственный `/data/local/tmp/zedone-product-*` удалён, отсутствие проверено
отдельно. Эмулятор штатно выключен Device. APK/app files и UI не менялись.

Финальные guards/comment policy/actionlint повторно прошли после добавления
обязательного self-check маршрутизации в standalone Android workflow.
Тело contract/TS checker после выделения функции побайтово совпадает с базой
после снятия отступа. Шесть source SHA-256 и логи сохранены в ignored reports.

## Передача

- CI.12 verified в отдельной локальной ветке, без commit/push/PR. Новый Linux/
  GitHub CI, APK/WebView/UI suites не выполнялись; новое remote время/успех
  не предполагаются. Перенесённый реальный Linux compiler-failure preflight
  сохраняет прежнее тело и остаётся обязательным в CI; локально проверены
  его распределение/полнота и настоящий fake-Cargo installer suite на Git Bash.
- Пять правил: изменение проверки обратимо revert; общий API человека/агента
  и authoritative data/schema не менялись; developer/agent docs обновлены;
  Windows/native Android проверены, UI/platform suites сохранены, новая UI
  приёмка не заявляется; контракт по-прежнему выводится из Rust.
- Все 98 определения/состояния и прежние GAP/DEC сохранены. Исходный EV-117
  прежний; старые строки очереди и история журнала сохранены.
- Следующий шаг для интеграции — рассмотрение diff и новый полный CI на
  опубликованном head. Публикация не поручалась; P06.07 не начат.

## Публикация и сравнение GitHub CI — LOG-640

Разработчик поручил PR и после GitHub CI собрать метрики до/после.
Merge/delete не поручены. База origin/main после fetch — 693b1fb, source
fingerprints финальной локальной проверки прежние. Связанные checkpoint
публикации/интеграции P06.06 включены в docs scope; продуктовые статусы прежние.

До публикации сохранены run metadata, все 18 jobs/steps и raw/cache logs
последнего успешного PR #108 на head 5ca0ee8, обе workflow attempt 1:

- [Windows/Linux](https://github.com/Zeyafix/zedone/actions/runs/44257694922).
- [Android](https://github.com/Zeyafix/zedone/actions/runs/44257695082).

Методика после нового CI:

- Сравнить те же 15 suites и три aggregates на точном новом PR head.
- Wall time: от workflow `created_at` до последнего `completed_at` его jobs;
  общий PR-интервал — минимум created_at двух workflow до последнего check.
  Отдельно показать Windows/Linux/Android и ожидание очереди jobs.
- Runner work: сумма `completed_at - started_at` всех jobs, отдельно для
  workflow/OS и по suite. Это наблюдаемое время, не billed minutes.
- Разобрать изменённые step durations: installer/negative controls, KVM,
  arm64 build и runtime callback. Указать, где step сменил состав работы.
- Сверить restore/hit/miss evidence по Rust/Tauri/Gradle и runner image,
  записать attempt/rerun/failure. Один запуск — наблюдение, не доказательство
  стабильного ускорения и не изолированный benchmark.
- Обновить Testing PR фактическими результатами/ссылками, сохранить raw данные
  локально в ignored reports и показать разработчику сравнение с графиком.

Новый CI ещё не запускался; значения «после» и выигрыш не предполагаются.

## PR #109 опубликован — LOG-641

[PR #109](https://github.com/Zeyafix/zedone/pull/109), head
863e64b26573b4563bac21d90028083c4f4c8df3, base main/693b1fb.
Commit `ci: reduce repeated checks while preserving platform coverage`:
14 source/docs файлов, 797 insertions/177 deletions; обычный commit/push,
noreply author/committer, staged scope/privacy/98 definitions/states проверены.
Шесть source fingerprints прежние. Исходные related EV-117/journal/queue
checkpoint включены по происхождению; локальные logs/данные вне Git.

English title/body, порядок Summary/Changes/Testing, реальные проверки/ограничения
и generation credit прочитаны обратно. PR linked сразу после создания;
визуального продуктового изменения нет, медиа неприменимы.
Initial readiness OPEN/non-draft/MERGEABLE/UNSTABLE, comments/reviews/requests
отсутствуют. 15 suites IN_PROGRESS, три aggregate ещё ожидаются, полный expected
набор 18 сверен с сохранёнными baseline jobs. Это не новый CI успех.

Далее watch и после полного CI сбор метрик по методике выше. Raw before metadata/
jobs/steps/logs/cache и continuation state сохранены в ignored reports.
Merge/delete не поручены. Итоговые checkpoint остаются локальными, чтобы не
менять tested head дополнительным commit/push.

## Linux product attempt 1 failed — GAP-034 / LOG-642

На прежнем head 863e64b [job 134671814924](https://github.com/Zeyafix/zedone/actions/runs/44266376564/job/134671814924)
завершился failure в Rust storage:
`cards::tests::search::search_thousand_card_probe_uses_authoritative_graph_without_an_index`.
Все 1000 результатов и проверенные поля корректны, поиск занял 30209ms
при прежнем `elapsed < Duration::from_secs(30)`. Storage: 283 passed/1 failed.
Сверка diff подтвердила отсутствие изменений теста, search implementation
и порога. Причина превышения неизвестна; исторический BUG-008 касался другого
совместного выполнения Android native/arm64, тождество причин не доказано.

Raw job log/metadata/annotations сохранены в ignored metrics reports.
`gh run view --log-failed` пока не выдаёт лог незавершённого workflow;
прямой jobs/logs endpoint позволил прочитать завершённый job. Escape sequences
оставлены в raw файле, при диагностическом выводе удалены.

Один bounded rerun только этого job на неизменном head ещё не принят:
CLI ответил `job 134671814924 cannot be rerun`, прямой jobs/rerun endpoint
подтвердил HTTP 403 `The workflow run containing this job is already running`.
Другие jobs не отменяются. После terminal run 44266376564 выполнить один
штатный job rerun; не делать циклических повторов и не ослаблять threshold,
assertions/бюджеты. При повторном отказе нужна диагностика, а не ещё один rerun.

PR Testing обновлён фактическим failure/неизвестной причиной/ожидающим повтором,
head прочитан обратно. Workflow/checks продолжаются, новых CI успехов нет.
Для метрик первая попытка — неполная: после storage отказа release/unsafe
часть не выполнялась, её короткую длительность нельзя сравнивать с полным
успехом как ускорение. Сохранить отдельно failure, rerun work/очередь и полное
время до зелёного результата; одного повтора недостаточно для объяснения причины.
Все прежние продуктовые definitions/states сохранены, добавлен GAP-034;
новой продуктовой приёмки/исправления поиска не заявляется.

## Linux aggregate failure — LOG-643

[Aggregate job 134675766660](https://github.com/Zeyafix/zedone/actions/runs/44266376564/job/134675766660)
завершился failure на шаге `Require every Linux suite`: raw log показывает
`RESULT: failure` и штатный `test "$RESULT" = success`. Это ожидаемое
распространение уже известного отказа Linux product/GAP-034; новой причины
отказа или ослабления aggregate нет. Все три остальные Linux suites SUCCESS.

Exact head прежний 863e64b, host workflow ещё in_progress (Windows product/shell
продолжаются). Повтор job не запрашивался снова до terminal workflow;
accepted_reruns остаётся 0 из разрешённого одного. Raw aggregate log сохранён
для учёта attempt 1. Существующий watch продолжается; результатов полного
CI и метрик «после» пока нет, merge/delete не выполнялись.

## Android pairing failure и принятые rerun — LOG-644

Initial оба workflow завершились failure на head 863e64b. Остальные 14 checks
SUCCESS; два product/pairing failure и два dependent aggregate failure.
Сохранены metadata и весь состав jobs/steps обеих attempt 1 до повторов.

Android [pairing job 134671814719](https://github.com/Zeyafix/zedone/actions/runs/44266376707/job/134671814719)
завершился не по 15m timeout: standard suite после text/combined fields/card
placements получила `Unexpected command failure: access_denied`, затем finally
увидел host fixture exit 101. Восстановление исходных APK и всех app files
по SHA-256 passed. `--positions` после failed standard не запускалась.
Android [aggregate job 134678153604](https://github.com/Zeyafix/zedone/actions/runs/44266376707/job/134678153604)
штатно отказал с `RESULT=failure`.

Причина access_denied и итог host fixture cleanup не установлены (GAP-035).
Fixture имеет прежний предел 240s без UI env; время сценария/exit 101 совместимы
с timeout, но server-errors.log записан только в ignored directory удалённого
runner и не попал в job log. Timeout не выдаётся за доказанную причину.
Acceptance source/сценарии/пороги/240s/15m/cleanup assertions в CI.12 не менялись.

После terminal workflows GitHub принял по одному rerun только failed jobs:
Linux product 134671814924/run 44266376564 и Android pairing 134671814719/run
44266376707. Head прежний; accepted_reruns=1 для каждого записан в resume-state.
Новых повторов при повторном отказе не делать: требуется диагностика.
Результатов повторов ещё нет; новые CI успехи/снятие GAP не заявляются.
Все attempts/очередь/rerun overhead сохраняются для метрик, первоначальные
короткие failed runs не являются доказательством ускорения полного CI.

## Все 18 SUCCESS и метрики — LOG-645

Exact head 863e64b/base 693b1fb: полный expected набор 18 COMPLETED/SUCCESS,
PR OPEN/MERGEABLE/CLEAN, comments/reviews отсутствуют. Final attempt 2 обоих
workflow: [Windows/Linux](https://github.com/Zeyafix/zedone/actions/runs/44266376564),
[Android](https://github.com/Zeyafix/zedone/actions/runs/44266376707).
Linux product job 134679456275 и pairing 134680023677 прошли после ровно одного
принятого повтора каждого. Четырнадцать остальных checks унаследованы из
attempt 1. Pairing выполнил обе isolated suites, два SHA-256 восстановления
исходных APK/app files и два OS cleanup. Source/threshold/240s/15m прежние.
Успех повторов не устанавливает причину первых отказов; GAP-034/035 open.

| Наблюдение | До: PR #108 | После: PR #109 | Изменение |
| --- | ---: | ---: | ---: |
| От создания workflow до всех зелёных checks, с повторами | 16:27 | 32:02 | +15:35 / +94.7% |
| Сумма времени последних успешных исполнений 18 jobs | 134:33 | 124:20 | −10:13 / −7.6% |
| Сумма всех реальных исполнений, включая отказы | 134:33 | 141:10 | +6:37 / +4.9% |

Все длительности — минуты:секунды, суммы могут превышать час. Это не billed
minutes и не оценка стоимости. Initial failed executions составили 16:50
runner time; новые rerun executions — 21:44 (они уже входят в успешный набор).
Полный wall учитывает GitHub запрет повторять до окончания workflow, задержку
до запроса повтора и очереди. Сумма scheduler queues: 0:42 baseline, 0:40 у
успешного набора и 0:48 у всех новых исполнений. Ожидание от завершения первого
workflow до создания rerun — 2:36 host и 3:44 Android.

Для расчёта получены raw run/jobs/steps/logs всех четырёх workflow и обеих
текущих attempts. GitHub в attempt 2 создаёт новые ID/run_attempt/created_at
для 14 inherited success records со старыми start/end/steps. Они не выполнялись
заново: совпадения проверены по workflow/name/start/end/conclusion/steps и
исключены из повторного счёта. Реальных исполнений 22 вместо 18; успешный
набор смешивает attempts и не моделирует чистый одновременный workflow.

Наблюдаемые отдельные jobs: Windows product 16:19→13:04, Android product-arm64
11:38→8:02, shell-arm64 6:13→4:37; Linux product 5:48→6:42, Android native
9:58→12:13, pairing 14:53→14:56. Одного общего ускорения нет. Pairing около
четырёх секунд от прежних 15m, будущий/cold runtime не гарантируется.

Шаги по успешному Android-набору: installer controls 7→1 исполнений (сумма
4s→0s по секундным API timestamps), compiler-failure controls 7→1 (15s→2s),
KVM setup 7→5 (150s→125s). 0s не означает мгновенную работу. Environment
callback сменил состав 6→5 jobs; новый отдельный arm64 build 333s нельзя
игнорировать при сравнении прежнего runtime callback. Полная таблица steps
сохранена отдельно, step/job totals не складываются повторно.

Cache evidence: на обеих сторонах 15 Rust full-match restores, семь Tauri и
четыре Gradle restores, ключи совпадают. Это cache-hit наблюдения, не cold
benchmark. Четыре назначения runner image отличаются: два aggregates и обе
arm64 suites. Source invalidation/host load/последовательность повторов не
изолированы; изменение времени нельзя целиком приписать удалению дублей.

PR Testing переписан фактическими CI результатами, всеми 18 job timings,
ключевыми step totals, методом/повторами/cache evidence/ограничениями и
generation credit; body/head прочитаны обратно. Локальные comparison.md,
summary/job/step/phase/cache JSON, CSV, actual-executions и исходные logs
сохранены в ignored reports. График проверен offline (два payload) и реальным
HTML preview без console errors. Новых commit/push для метрик нет, tested
head сохранён. Пять правил прежние; документация дополнена, все 98 definitions/
states и прежние GAP/DEC сохранены. Active_work null, PR не merged; merge/delete
не поручены. Поручение PR + сбор метрик выполнено, далее рассмотрение PR.
