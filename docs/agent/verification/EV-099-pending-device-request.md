# EV-099 — сохраняемый запрос устройства без доступа

## Рабочая карточка

- Шаг: P06.01.2.3, done в host-срезе; PR #93/74aa615. База main/d0b4e4c и локальный e7f5277
  с ранее накопленными документами; ветка feat/p06-01-2-3-device-request.
- Цель: приглашение принимается однократно по проверенному TLS-каналу,
  запрос сохраняет ожидание после перезапуска и может быть отменён без прав
  на пространства или подтверждение самого себя.
- Требования: T06/T09/A51, DEC-003, ADR-0009/0054/0055/0056.
- Зависимости: P06.01.2.1/PR #91/e34b8fd, P06.01.2.2/PR #92/d0b4e4c.
- Scope: core/storage/adapters, производный контракт, проверки, документация.
  Формы, подтверждение/выдача прав и обмен пространствами — вне этого шага.
- Проверки: полный check-product Windows/native Android, обе release ABI,
  реальные TLS-запросы/отмена/restart, отказы/одноразовость/срок/попытки,
  rollback/гонка/receipt, общий HTTP/IPC/CLI и отсутствие секретов в SQLite.
- Обратимость: отмена запросившим устройством или администратором; отмена
  приглашения завершает ожидание. Новое подключение требует нового приглашения.
- Размер: безопасный приём запроса неделим между транзакцией, TLS-доставкой
  и сохранённым секретом повторов; тесты обязаны проверять эту композицию.
- Результат: PR #93/head 7ab94f1 интегрирован в 74aa615 после всех 17
  SUCCESS/readiness; T09/A51 и прежние GAP/DEC сохраняют состояния.
- Далее: P06.01.2.4 по новому поручению.

## Реализация

RequestPairing, ограниченная авторизация ожидания, общий административный
pairing.inspect/cancel/list, SQLite pairing_requests/счётчик попыток, OS-слот
PendingPairing, составной код с pin и host TLS HTTP/1 exchange реализованы по
ADR-0056. Код/ключ ожидания/будущий ключ не становятся обычными командами,
receipts или access principal. Администратор и получатель могут отменить
запрос; отмена приглашения завершает его ожидание. Использованный код не
оживает. Обновлены Rust-derived OpenAPI/TS/validator и синтетический эталон v1;
исходные документы/actor/receipts эталона сохранены. Версии зависимостей прежние;
добавлены прямые ссылки на уже закреплённые HTTP/crypto/CSPRNG зависимости.

## Выполненные проверки

- `bun scripts/check-product.mjs`: exit 0; fmt/Clippy -D warnings, 316 Rust
  тестов, release all-targets, семь unsafe guard, совпадение OpenAPI/типов/validator.
- `bun scripts/check-product.mjs --android-native`: exit 0; 314 native Rust
  тестов в Device-эмуляторе, x86_64 release/семь unsafe guard, тот же контракт.
- `bun scripts/check-product.mjs --android-arm64`: exit 0; aarch64 release
  all-targets и семь unsafe guard. Это не выполнение на физическом arm64.
- Каждый product-check: 26 client tests / 271 assertions; полномочия,
  исходные намерения и transport validation прежнего клиента сохранены.
- Обе пары host/native probes вызваны явно: прежние TLS/identity и новый
  pending request. Неверный pin отвергнут, запрос принят без доступа,
  серверная база повторно открыта, сохранённая сессия повторена без второго
  запроса, текущий статус прочитан и запрос отменён. Собственные reverse удалены.
- Новые transaction tests: точный срок, пять неудач/restart, гонка двух writers,
  receipt mismatch, чужой pending key, отмена приглашения, отмена/старый expected,
  сохранность/отсутствие новых прав и rollback при сбое receipt/до commit.
- HTTP/IPC/настоящий CLI: общий admin inspect/cancel/list/replay/stale,
  отказ обычному участнику, отсутствие request/confirm в обычном Service.
- Windows Credential Manager: загрузка исходной pending session после нового
  экземпляра store, удаление только pending с сохранением channel identity.
- Обе debug APK/instrumentation APK собраны, manifest identity/categories
  проверены. Native Android SecretStoreInstrumentation passed: новый pending
  слот сохраняется/читается, подмена ciphertext между слотами отклоняется,
  удаление pending сохраняет channel identity; прежние corruption/key loss/
  concurrency/atomic recovery/cleanup проверки сохранены.
- UI types на Node и production build passed. Запуск vue-tsc через Bun оказался
  неподходящим; штатный Node прошёл. Отличие локального Node от CI baseline
  зафиксировано только в ignored отчёте; инструменты/CI baseline не менялись.
- Device: исходный APK/данные сохранены до установки тестовой сборки, после
  instrumentation восстановлены и проверены SHA-256. Через agent-device
  подтверждено открытие исходного приложения и сохранённого пространства;
  затем приложение закрыто. Это восстановление, не приёмка нового pairing UI.

Сырые логи/параметры машины/архивы/отпечатки исходников остаются в ignored
docs/reports/p06-01-2-3. Промежуточные запуски выявили устаревшие fixture/schema
проверки и незарегистрированную RequestPairing schema; они исправлены, все
окончательные проверки выше успешны. Добавлена проверка разрешения всех
OpenAPI refs; проверки/бюджеты не ослаблены.

## Пять правил и границы

1. Обратимость: отмена обеими сторонами/через приглашение, повторная выдача
   требует нового кода. Локальные пространства не удаляются.
2. Человек/агент: один host API запроса и тот же Service/HTTP/IPC/CLI для
   администратора. UI/внешний listener не включены, собственной авторизации
   или прямой записи SQLite у агента нет.
3. Документация: user/agent/development/architecture и ADR-0056 обновлены.
4. Паритет: Windows/native Android, обе ABI, реальная пара и оба OS store
   проверены в указанном срезе. Полный Rust→Kotlin→TLS процесс/физический телефон
   и новый полный UI/CI этим не принимаются.
5. Источники истины: SQLite/core определяют запрос/попытки/receipt, один OS store
   хранит секреты, Rust задаёт контракт. Второй реестр прав или состояния не создан.

P06.01.2.3 done в host-срезе, P06.01.2/P06 in_progress. Все 98 состояния критериев и
прежние GAP/DEC сохранены. Подтверждение/отклонение/права/отвязка — .2.4,
итоговый API/CLI сценарий — .2.5, формы — P06.01.3, обмен пространствами — P06.03.
Полная приёмка T09/A51 впереди; итог CI/интеграции приведён ниже.

## Публикация — LOG-537

[PR #93](https://github.com/Zeyafix/zedone/pull/93), head
7ab94f141ffd569abf6a9e38d2eca8a149118995, base main/d0b4e4c. Два коммита/54 файла
и все 42 tested fingerprints подтверждены; обычный push/pre-push passed. Body
в OS temp вне репозитория, readback подтвердил три English sections/credit и
границы host-среза. Comments/reviews/requests/threads отсутствуют, UI diff нет.

Новый полный CI pending: 14 runtime suites IN_PROGRESS, три strict aggregates
ожидаются. Все 17 COMPLETED/SUCCESS и свежая readiness точного head обязательны
перед порученными merge/удалением ветки. Полные критерии и прежние GAP/DEC
не повышаются; новой реализации нет. Записи публикации пока локальные.

## Интеграция — LOG-538/539

[PR #93](https://github.com/Zeyafix/zedone/pull/93) интегрирован: head
7ab94f141ffd569abf6a9e38d2eca8a149118995, merge
74aa615a256fbcddc784d06c1c5004e8f8953c33 после точного набора всех 17
COMPLETED/SUCCESS и CLEAN readiness. Body с фактическим полным CI прочитан
обратно; comments/reviews/requests/threads отсутствовали. Raw Windows/Linux
product/shell подтвердили новые core/storage/API/store сценарии и полный
прежний desktop/touch UI; Android — обе пары host/native probes, запрос/
reopen/replay/inspect/cancel без доступа, Keystore и весь прежний native UI/
cleanup. Обе ABI/prototype suites/strict aggregates SUCCESS.

Дерево merge равно checked head; main обновлена без потери двух dirty
документов (SHA-256), feature-ветка удалена локально/на GitHub/tracking.
По дополнительному поручению записи публикации/интеграции сохраняются
отдельным коммитом в docs/p06-01-2-3-integration-results без нового push/PR.

P06.01.2.3 done в host-срезе, пять правил и границы выше сохраняются; все 98
определения/состояния и прежние GAP/DEC неизменны. Полный app flow/подтверждение/
права/формы/обмен/физический телефон отдельно. Далее готов P06.01.2.4 по новому
поручению; новая реализация не начата, active_work null.
