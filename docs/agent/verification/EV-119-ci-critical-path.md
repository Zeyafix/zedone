# EV-119 — CI.13: сократить jobs дольше десяти минут

## Карточка

- Поручение: после PR #109 сократить проверки >10m насколько возможно без
  потери качества. База — `test/remove-redundant-checks`/863e64b.
- Три исходных dirty документа EV-118/journal/queue сохранены побайтово в
  ignored `docs/reports/ci13-short-jobs`; их предыдущие результаты сохраняются.
- Основание: EV-118/LOG-645 и raw jobs/steps/logs CI.12. Долгие jobs:
  Windows product 13:04, shell 11:02; Android product-native 12:13,
  prototype-ui 11:06, pairing 14:56. Это наблюдения прежнего head.
- Цель: уменьшить последовательный критический путь; сохранить каждый тест,
  ABI, release-профиль, runtime и отказ. Бюджеты и производственные таймеры прежние.
- Scope: два workflows, host preparation, product/UI/Android dispatchers,
  pairing selection и их guards; developer docs. Product API/runtime/schema,
  assertions внутри сценариев и все 98 критериев не меняются.
- Обратимость: revert конфигурации/драйверов; без миграций и пользовательских
  данных. Partial failure/cancellation не проходит strict aggregates.
- Проверки: exact union host slices, release CLI в Android runtime,
  no-emulator ABI build, независимые pairing selectors, ошибки/сигналы,
  actionlint/ShellCheck, comment policy, реальные Windows и native Android.
- Scope превышает ориентир 300 строк вместе с guards/docs: одна неделимая
  перестройка обязательного покрытия двух workflows и их общего product driver.
  Отдельно опубликованные половины оставили бы routing guards несогласованными.
- Состояние: verified локально; публикация/новый CI и измерения ожидаются.

## Распределение проверок

| Платформа | Обязательные suites | Покрытие |
| --- | --- | --- |
| Windows/Linux | product, product-build | fmt/Clippy/Rust/client/contract/types и отдельные release all-targets/unsafe/comment-rejection |
| Windows/Linux | shell, shell-conflicts | UI main/desktop/touch/diagnostics и все workspace/conflict/pairing browser сценарии |
| Android | product-native, product-x86_64 | Native Rust, release CLI/exporter, host/device contract, оба paired probes; отдельные x86_64 release all-targets/unsafe/comment-rejection |
| Android | prototype-ui, prototype-arm64 | Прежний APK/WebView/instrumentation runtime x86_64 и отдельные arm64 APK/instrumentation builds |
| Android | pairing-fields, pairing-placements, pairing-positions | Text/fields, card/column placements и combined positions; в каждом собственные production bootstrap, acceptance APK, handshake/recovery/reverse/restart, SHA-256 restoration и OS cleanup |

Остальные prototype-native/prototype-ui host и Android product-arm64,
prototype-native/shell/shell-arm64 сохраняются. Всего 26 checks: по шесть
Windows/Linux suites, одиннадцать Android suites и три strict aggregates.
Новые host suites имеют свои main cache namespaces с первоначальным seed
из прежнего владельца. Cache не заменяет запуск проверки.

Полные локальные `check-product.mjs`, `check-ui.mjs`, Android dispatcher
без аргумента и `check-pairing-app.mjs` сохраняют полный охват. Новые аргументы
явные; неизвестный режим/лишние аргументы отклоняются. Native Android CLI
и exporter по-прежнему исполняются из release. ABI-only suites обходят AVD;
preflight пяти прежних runtime видов сохранён, pairing теперь три runners.

SDK/build setup повторяется на дополнительных runners. Нельзя обещать снижение
суммарных runner minutes или все jobs <10m до нового GitHub CI. Cold builds
и hosted нагрузка отдельно не измерены. GAP-034/035 и другие GAP остаются open;
перестройка не доказывает устранение причин первоначальных отказов.

## Проверки и передача

- `check-product.mjs --host-tests`: passed, Windows 384 Rust, 27 client/280
  assertions, 28 source-comment tests, fmt/Clippy, точный OpenAPI/types/validator,
  положительный typecheck и 11 negative type diagnostics.
- `--host-build`: passed, release all-targets/all-features и все восемь unsafe
  guards, comment rejection/recovery. Производственные профили прежние.
- `check-ui.mjs --main` и `--conflicts`: passed, Windows, pinned Node 24.22.1,
  QR/typecheck/Vite/fmt/Clippy/три shell Rust tests и настоящий Chromium с
  полным desktop/touch/conflicts/positions/pairing/agent-reverse охватом.
- `check-product.mjs --android-x86_64`: passed, release all-targets/all-features,
  восемь unsafe guards и comment rejection/recovery.
- `--android-runtime`: passed на Device-owned AOSP API 33/x86_64; 381 Rust,
  оба paired probes, release CLI/exporter и точный host/device contract. Свой
  native каталог удалён в finally. Новый режим не исполняет unsafe guards:
  они проверены предыдущим отдельным build-срезом.
- Host/Android routing guards: passed на финальном распределении; exact union
  host команд, ABI-only без ADB, release bins runtime, оба paired probes,
  mismatch/physical-serial rejection, child failures/signals, полные default
  команды, cache seeds/owners и строгие aggregates. Отдельно проверен exact
  union четырёх standard exchange вызовов из fields/placements.
- AST-инвентаризация browser driver против 863e64b: все 20 вызовов сценариев
  сохранены. Это дополнительный контроль, не замена реальных browser runs.
- actionlint 1.7.12 с настоящим ShellCheck: passed обоих workflows; Pyflakes
  локально отсутствует и отключён. Node syntax, comment policy 385 sources и
  `git diff --check`: passed. Первый lint выявил SC2193 на статическом
  placeholder/glob comparison; выбор manifest переведён на переменную SUITE
  из matrix env, повтор с ShellCheck passed, проверка не отключалась.

- Настоящий acceptance APK build и три последовательных локальных запуска
  `check-pairing-app.mjs --fields`, `--placements`, `--positions`: passed.
  Windows Credential Manager ↔ APK/Keystore/TLS/API/CLI, все группы конфликтов,
  recovery/restart/reverse и по три SHA-256 restoration/OS cleanup сохранены.
  Отдельные fixture бюджеты не менялись; GUI lifecycle шёл через команду,
  возвращённую Device. P06_EXCHANGE_UI отсутствовал, ручной UI checkpoint
  не заявляется; настоящий browser UI проверен выше.

После трёх slices production x86_64 APK собран повторно. Все девять исходных
APK/metadata/native-library artifacts восстановлены из сохранённых копий и
проверены по SHA-256; worker exit 0. По три `result.json` имеют
status=passed/restored=true/cleanup=true. Device выключен штатным инструментом.

Локальные логи — `docs/reports/ci13-short-jobs` (ignored). Linux, isolated
SDK/bootstrap и новый runner/cache runtime требуют GitHub CI. Старые 18
SUCCESS не подтверждают новое распределение. Далее публикация в PR #109,
полный новый CI и фактические метрики; merge/delete не поручены.

Пять правил в scope проверки: обратимость через revert и прежние reverse
сценарии; общий human/agent API не менялся и проверен API/CLI/browser;
developer/agent docs обновлены; Windows/native Android и настоящий APK
проверены локально, Linux/full CI ожидаются; authoritative Rust contract/data
сохранён, host/device export равен. Новая полная продуктовая приёмка не
объявляется; все 98 definitions/states и GAP/DEC прежние.
