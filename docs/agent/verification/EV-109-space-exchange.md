# EV-109 — первый полный обмен пространством

Рабочая карточка P06.03.2–.4, 2026-11-16; LOG-585.

- Поручение: весь P06.03 в текущей `feat/p06-03-space-sync` от caaf6df,
  последовательно поверх локально проверенного .1 без промежуточных интеграций.
  Исходные изменения .1 и пяти документов P06.02 сохраняются.
- Цель: явный выбор пространства, включение/отключение, общая пауза/возобновление,
  ручной и автоматический обмен одним простым набором через защищённое подключение.
- Требования: T09/A03/A08/A48, §1/§6 спецификации, ADR-0028/0047/0063.
  Зависимости P06.01/P04.10 доступны в базе; EV-108 подтверждает storage .1.
- Объём: общий core/storage контракт, защищённые страницы изменений Automerge,
  первое получение без частично видимого пространства, общие API/CLI/UI,
  фоновый цикл при работающем процессе и документация/проверки.
- Граница: P06.04 (полная матрица обрывов/совместимости), P06.05–.11
  (полное разрешение конфликтов) и измерения P09 отдельно. Техническая
  сохранность изменений/истории, действующие права и идемпотентность доставки
  обязательны уже здесь; ограничения не разрешают терять данные.
- Проверки: штатные product-check Windows/native Android/обе ABI;
  полный ui:check, новые парные TLS tests и Windows↔APK runtime через Device.
  Команды/результаты уточняются по выполнению, пока не заявлены passed.
- Обратные действия/отказы: disable/enable, pause/resume, restart, отозванные
  или read-only права, чужое пространство/подключение, устаревшая команда,
  SQL/commit/network failure и сохранность исходных APK/app files.
- Большой общий объём обусловлен явно порученным сквозным P06.03;
  .2/.3/.4 сохраняют собственные доказательства. Публикации/CI/merge пока нет.

## Текущий результат

P06.03/.1–.4 done: [PR #101](https://github.com/Zeyafix/zedone/pull/101),
head 69df2e3, merge 54a95bc после всех 17 Windows/Linux/Android SUCCESS;
LOG-589. Core/storage/host/API/CLI/UI интегрированы. Все локальные проверки,
Device UI и Android restoration LOG-586 сохраняют свой охват; новый CI описан
ниже. Ветка удалена локально и на GitHub, main=origin/main. 98 определения/
состояния и GAP/DEC прежние; это завершение P06.03, а не полная приёмка T09.

## Выполненные проверки

| Охват | Результат и граница |
| --- | --- |
| `bun scripts/check-product.mjs` финальный | Passed: 356 Rust tests, 27 client/280 assertions, policy/Impact/fmt/Clippy, release/guards и актуальные Rust/OpenAPI/TS/validator/type errors. |
| `UI_NODE_VERSION=26.8.1 bun run ui:check` | Passed на разрешённом локальном Node override; CI baseline 24.22.1 прежний. Полный Chrome/Opera desktop/touch и прежние сценарии; новый выбор/импорт/read-only/пауза/disable-enable/reload/resume и автоматическая доставка при закрытой вкладке passed. После добавления загрузки следующих страниц pairing.list отдельно повторены итоговые vue-tsc/build/fixture и весь pairing/exchange UI runner — passed. |
| Storage/TLS targets | Staging 256+ changes скрыт до завершения, переживает restart, последняя страница атомарна, локальный actor отдельный; malformed/чужой формат/ложный frontier отклоняются. Входящее изменение другой карточки не завершает текущую серию; изменение той же карточки сохраняет локальную серию перед merge. Парный TLS подтверждает read-only/write, обе паузы/reverse/reopen/revoke и сохранность. |
| Native Android | Финальный полный `--android` passed: 353 native Rust tests, 27 client/280 assertions, release обеих ABI/guards, matching host/native contract, парные probes и cleanup. Финальная граница локальных серий проверена в этом полном наборе; отдельные 3 sync_engine tests также passed. |
| Windows↔APK через TLS/API/настоящий CLI | Финальный запуск passed: колонка/карточка/тег/история, read-only без отправки локальных правок, явный write и reverse, обе паузы, disable/enable, новое подключение без наследования выбора, сохранность; прежние pin/Keystore restart/request/confirm/срок/revoke/unlink/reject/cancel/пять попыток также passed. Результат подтверждён result.json, restored/cleanup true. |
| Native Android UI через Device/agent-device | На том же acceptance APK в отдельной согласованной native-фикстуре: действующий локальный доступ, показанные устройство/пространство, включение, пауза, отключение/повторное включение с сохранением паузы, process restart с прежней паузой, явное возобновление и новый статус passed. Это приёмка UI/IPC управления на входящей роли; первая доставка и read/write проверены настоящей парой APK/API, исходящая форма дополнительно общим browser runner. UI телефона не заменялся adb/CDP. |
| `build-ui-android.mjs x86_64` и `aarch64` | Финальные production debug и androidTest APK обеих ABI/manifest passed без pairing-acceptance feature. Acceptance x86_64 отдельно сохранён с SHA-256 в ignored reports. Arm64 runtime/физический телефон/производительность не заявляются. |
| Policy/fmt/diff/сохранность | Comment policy 357 source files, fmt, git diff --check passed. Все строки 12 прежних таблиц эталона сохранены; три новые таблицы пусты. 98 определения/состояния прежние; исходные dirty EV-106/107/ADR-0062 побайтово сохранены. Локальные ссылки и fingerprints проверены отдельно. |

В первом полном UI-прогоне новый тест прочитал selected до завершения команды;
ожидание привязано к подтверждённому состоянию UI без изменения assertions.
Финальный полный ui:check passed. В подготовке native UI-фикстуры обнаружены некорректные бинарные копии:
текстовый shell при чтении и exec-in при записи не сохранили SQLite побайтово.
Первое объяснение через календарь Windows было преждевременным; доказанная
причина — несовпадение размера/байтов и отказ повреждённой тестовой базе.
Подготовка использует согласованный snapshot собственной native-фикстуры,
exec-out для чтения, push/cp для записи и обязательный SHA-256 до запуска.
Продуктовый протокол никогда не переносит SQLite/ACL/часы между устройствами.
Проверки/права/часы и таймауты продукта не ослаблялись. Начальные failing логи
сохранены в ignored reports; они не выдаются за финальный успешный результат.
Третий запуск подтвердил ручное восстановление корректной фикстуры, но истёк
во время диагностики до завершения UI; четвёртый с проверенной передачей прошёл
полностью. Во всех запусках исходные APK и app files восстановлены по SHA-256;
временные свои OS slots/rules очищены. Свои пустые UI-каталоги удалены отдельно.

## Итог и передача

Агрегат SHA-256 итоговых 37 source/contract файлов:
`e2a1e6f2390ec475cfdd704ede181a2ab7a88f5a0f7785bca6bc25b1308b666d`.
Точный состав и отпечатки требований/APK — ignored full-fingerprints/artifacts
в `docs/reports/p06-03-checks/`; актуальный fixture SHA-256
`626f0bbe1761a5b4d7c0f94ff4b85cf726f833049238c878f3b669e9437525aa`.

- Обратимость: disable/enable, pause/resume и сохранность данных/истории
  подтверждены; отмена настройки не удаляет готовое пространство.
- Человек/агент: общий core/Service для настроек, один DeviceHost для UI/IPC/
  HTTP/настоящего CLI; одинаковые текущие права. Raw pages не обходят paired grant.
- Документация: user/agent инструкции обмена и подключения, developer guidance,
  архитектура/ADR-0064, журнал и очередь отражают текущий объём.
- Паритет: Windows и native Android, реальная пара Windows Credential Manager↔
  APK/Keystore, Device UI и обе APK ABI подтверждены в описанном объёме.
- Единый источник: локальная SQLite/Automerge, отдельные локальные настройки,
  progress и OS-сессия; UI не выдаёт доступ по кэшу или историческому receipt.

T09/A03/A48 not_run, A08 partial остаются прежними: полный обмен при обрывах,
совместимость, все конфликты, настоящий Tailscale и измерения относятся к
следующим шагам. Физический телефон не использовался; исходный эмулятор снова
выключен, фоновых runtime-проверок нет. Исходная установка сохранена.
Commit/push/PR/новый CI/merge не выполнялись. Далее — рассмотрение полного
локального P06.03 и публикация/интеграция отдельным поручением; P06.04 не начат.

## Публикация P06.03 — LOG-587

Разработчик поручил PR и при успехе merge/delete ветки. Локальный результат
LOG-586/EV-109 сохраняет verified до окончания нового CI и проверки readiness.
Публикация выполняется целиком из feat/p06-03-space-sync; следующий продуктовый
шаг P06.04 не начат. Результаты CI/интеграции фиксируются отдельно после выполнения.

## Интеграция — LOG-589

[PR #101](https://github.com/Zeyafix/zedone/pull/101) с 54 согласованными файлами
и настоящими Before/After PNG 1280×1080 опубликован; обе hosted картинки
проверены по SHA-256. Медиа вне Git. Head
69df2e3b83db69dfd2e220f83f77e120b21c4c89 получил 17 COMPLETED/SUCCESS в attempt 1:
[Windows/Linux CI](https://github.com/Zeyafix/zedone/actions/runs/43919617091),
[Android CI](https://github.com/Zeyafix/zedone/actions/runs/43919617078).
Raw shell обеих ОС подтвердил Node 24.22.1, полный desktop/touch pairing/exchange
и автообмен с закрытой страницей. Android подтвердил native/обе ABI/guards,
парный APK/TLS/API/CLI и исходный SHA-256 restoration/cleanup. Host memory fixture
CI не заменяет локальный Windows Credential Manager и Device UI из LOG-586.

Перед merge свежие head/base 69df2e3/caaf6df: OPEN/non-draft/MERGEABLE/CLEAN,
все 17 expected checks SUCCESS, замечаний/reviews/requests/threads нет.
Merge API с pinned head создал 54a95bc0705f35a61db787e20bbe7df9ff181c01;
родители — checked base/head, merge tree равен reviewed head. Main fast-forward,
remote feature удалён с lease head, local branch-d; все feature refs отсутствуют.
Другие worktrees/stash сохранены. P06.03/.1–.4 done, пять правил подтверждены
в его scope; все 98 определения/состояния и GAP/DEC прежние. T09/A03/A48 not_run,
A08 partial; P06.04/полные конфликты/Tailscale/измерения отдельно, не начаты.
Итоговые документы остаются локально после merge без нового commit/push;
продуктовый source равен проверенному merge tree. Active_work null.
