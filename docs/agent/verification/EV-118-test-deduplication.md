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
