# EV-102 — Сквозное подключение двух устройств

## Рабочая карточка

- Шаг: весь P06.01.2.5; ветка feat/p06-01-2-5-pairing-acceptance от bcc9b20.
- Поручение: продолжить исходный шаг до сквозного результата; .1–.3 являются
  внутренними частями, промежуточная интеграция не требуется (LOG-547).
- Зависимости: .2.1–.2.4 done в main, EV-095/098/099/100; EV-101 локально verified.
- Цель: постоянный защищённый host и сохранённая сессия позволяют выполнить
  запрос/подтверждение/чтение/отвязку/повторное подключение через общий API/CLI
  на Windows и реальный Rust→Kotlin→TLS процесс приложения Android.
- Scope: adapters network/host/CLI, Android composition/Keystore, штатные
  проверки/парная приёмка, документация. Формы и обмен пространствами отдельно.
- Проверки: Windows/native Android product-check, обе release ABI, debug APK,
  общий API/CLI и Android host bridge, повтор/restart/отказы/сохранность.
- Обратимость: отмена/отклонение/отвязка/отзыв, новый запрос и новый ключ;
  неизвестный результат не разрешает потерю действующего секрета.
- Результат: весь P06.01.2.5 verified локально. Сквозной Windows↔Android APK/
  API/CLI сценарий, обе нативные матрицы/ABI и обычные APK passed; новый CI и
  интеграция не выполнялись. Далее ревью/интеграция одной ветки целиком.

## Реализация

DeviceHost/SecureListener связывают общий Service, pairing и защищённую сессию.
Listener TLS-only, с ограничением восьми задач, handshake 5s/обработки 15s и
graceful shutdown. Стандартные команды используют прежние Rust handlers/SQLite
права/receipts. Host session.* и Android native IPC входят в один фиксированный
dispatcher и требуют действующего ManageDevices. Сырых секретов в ответах нет;
универсальный remote-data proxy под административным ключом не предоставляется.

CLI --listen проверяет существующий административный доступ и выполняет recovery
редактирования перед публикацией Service. --endpoint доставляет точное намерение
по pinned TLS, ключ/pin читает из файлов. Android composition использует настоящий
AndroidSecrets→PluginHandle→Kotlin Keystore. ZDPN v2 сохраняет адрес с pin/ключами;
после process restart запись не создаётся заново. V1 читается без угадывания адреса.

Неверная неизменная попытка получает атомарный отрицательный receipt со счётчиком;
после известного отказа pending может очищаться без поздней активации потерянного
ключа. Повтор не расходует попытку заново. Неизвестные IDs/закрытые приглашения
не создают неограниченных receipts; максимум пять отрицательных записей на код.
Unknown invitation возвращает unavailable и сохраняет запись для повторного запроса.
SQL отказ откатывает также счётчик; исходный retry и reopening проверены отдельно.

DDL/API-контракт/unsafe не менялись. Tokio signal для завершения CLI включает
signal-hook-registry, зафиксированный lockfile; тот же механизм уже использовался
оболочкой. Локальные сведения/версии/выводы сохраняются только в ignored reports.

## Сквозная матрица APK/API/CLI

Отдельный debug feature pairing-acceptance создаёт изолированный fixture.
Обычные APK не содержат этот код; release с feature запрещён. Тестовый bootstrap,
локальный fixture key и маршрут fixture.peer_command не становятся продуктовым
API. Этот маршрут вызывает trusted native peer_command, который заимствует OS
ключ и выполняет обычный удалённый Service. Агентские продуктовые команды используют
собственный явно выданный ключ; admin host не получает чужие права на данные.

`check-pairing-app.mjs` использует настоящий Windows Credential Manager и настоящий
Keystore внутри APK; start/relaunch/close локально выполняются только возвращённой
Device командой. Связь через тестовые reverse/forward TCP, не физический Tailscale.

| Сценарий через CLI/API и настоящее приложение | Результат |
| --- | --- |
| Запрос/ожидание и точный submit replay | pending; до подтверждения чтение/самоподтверждение запрещены |
| Отдельный admin confirm с read | Чтение существующего пространства разрешено, rename запрещён |
| Полный restart процесса APK | Прежние TLS pin, request, SocketAddr и защищённый ключ; исходный read replay успешен |
| Stale unlink / точный unlink replay / свежая очистка | Старый expected отвергнут; повтор исходного намерения не выдаёт права; после refresh unconfigured |
| Старый использованный код | Новый запрос отвергнут, pending очищен; нет восстановления прежней связи |
| Новое приглашение и явный read_write | Новый request; rename и обратный rename успешны |
| Обычный access.revoke | Чтение запрещено; свежий refresh показывает unlinked и очищает OS-сессию |
| Cancel и reject | Точный cancel replay и свежая очистка; rejected не подтверждается |
| Ограниченный срок grant | После срока запрещён даже replay прежнего read; связь остаётся confirmed до явной отвязки |
| Пять отдельных неверных попыток | Каждая завершается отказом без занятого pending; приглашение locked |
| Сохранность | Исходное имя пространства сохранено; исходные APK/все файлы приложения восстановлены по SHA-256 |

В CI сквозной runner добавлен после прежней shell/UI матрицы, без изменения
порядка, числа suites или budgets. Linux CI использует host Memory fixture и
настоящий Android Keystore; этим не подменяются Windows OS-store доказательства.
Новый CI пока не запускался, новые workflow результаты не заявляются.

## Проверки и границы

- Полный Windows product-check: 339 Rust (22 adapters lib/42 commands/17 core/
  7 contract/251 storage), 27 client/280 assertions, fmt/Clippy -D warnings,
  release/7 negative unsafe guards и равенство Rust/OpenAPI/TS.
- Полный Device Android-native product-check: 336 Rust (19 adapters lib/42
  commands/17 core/7 contract/251 storage), те же client/контракт/типы;
  x86_64 release/7 unsafe guards и обе прежние парные TLS/request пробы.
- Android aarch64 product-check: release all-targets/7 unsafe guards и прежние
  client/контракт/типы. Физический arm64 не выполнялся.
- Отдельный debug x86_64 APK/instrumentation/manifest с acceptance feature и
  production UI build passed. Отдельный host shell test: все три tests passed.
- Обычные debug APK/instrumentation/manifest x86_64 и aarch64 без acceptance
  feature passed. Полный новый UI-регресс и Linux CI не выполнялись локально.
- Настоящие --listen/--endpoint CLI, принудительное завершение и новый процесс
  с прежним pin проверены отдельно; удалена только точная синтетическая OS-запись.
- Прежние kernel/storage проверки срока приглашения, SQL/receipt/commit rollback,
  конкуренции, ключей/прав/повторов пройдены на обеих платформах. SQL injection
  faults и точные управляемые часы проверены компонентно, не объявляются новым
  fault injection внутри APK-сценария.

Сквозной runner публикует final passed только после OS-key cleanup, удаления
собственных transport rules, восстановления исходного APK/данных и завершения
Windows fixture. Namespace/backup/diagnostics сохраняются в ignored reports
даже при отказе. Исправлены ошибки путей/формы снимка тестового стенда;
отказ конкурирующей пересборки занятого Windows CLI не принимался за успех;
финальный host product-check выполнен последовательно после завершения сценария.
Прежние BUG/GAP не объявляются устранёнными этими исправлениями стенда.

## Пять правил

1. Обратимость: cancel/reject/unlink/revoke, новый код/ключ, cleanup/restart;
   неизвестный исход сохраняет секрет и исходное намерение, данные не удаляются.
2. Человек/агент: единый core/Service/HTTP/CLI и host dispatcher для управления
   подключением; отдельного прямого редактирования SQLite/обхода прав нет.
3. Документация: user/agent/development/architecture, ADR-0059, очередь и журнал.
4. Паритет: Windows/native Android, настоящий APK/Keystore bridge, обе release
   ABI; формы и физические устройства/Tailscale остаются отдельно.
5. Источник: права и receipts — один SQLite Service; OS-store хранит ключи,
   локальная готовность не заменяет действующие права. Контракт Rust-derived.

Полные T09/A51 и прежние GAP/DEC сохраняют состояния. Обмен пространствами,
формы подключения, системный автозапуск/фон закрытого APK и выпуск отдельно.
Commit/push/PR/новый CI/merge не выполнялись.

После проверок подтверждены 26 fingerprints исходников/проверок/конфигураций,
все 98 определения/состояния приёмки, прежние события и исходный dirty EV-100.
Изменения документации предыдущей работы сохранены. Доказательства и резервные
копии — ignored docs/reports/p06-01-2-5-full и pairing-app-*; машинных сведений
в общий diff не добавлено. Device выключен штатным инструментом.

P06.01.2.5 и его внутренние части verified, P06.01.2/P06 in_progress.
Далее ревью/интеграция всего P06.01.2.5; затем готов UI-шаг P06.01.3 на принятом
результате. Active_work очищен, следующий продуктовый шаг не начат. LOG-548.

## Публикация и guard CI — LOG-550

[PR #95](https://github.com/Zeyafix/zedone/pull/95) создан на head 2b4c6ae,
base main/bcc9b20. Опубликованы 35 согласованных файлов; body/readback/credit,
26 runtime fingerprints и noreply подтверждены. Исходный dirty EV-100 вне
коммита, обычный push/pre-push passed; визуального diff нет.

Первый Linux prototype-native job отказал в строгом check-android-ci-suites:
dispatcher уже добавлял debug APK build/сквозной runner после прежнего UI,
но guard ещё ожидал старый состав. Обновлена точная последовательность full/shell
и добавлены остановка/сохранение exit code и signal на каждом из трёх shell
шагов. Suite ownership, число checks, таймауты и строгие aggregates прежние.
Локальные check-android-ci-suites и check-host-ci-suites passed. Продуктовый
runtime не менялся; его прежние 26 fingerprints совпадают. Новый полный CI и
readiness нового head обязательны перед ранее порученными merge/delete.
Состояния полных критериев/GAP/DEC и пять правил прежние.
