# EV-103 — ручное подключение и управление доступами

Дата: 2026-11-15. Рабочая карточка P06.01.3/P06.01.4.

## Текущий статус

Реализация интегрирована через [PR #96](https://github.com/Zeyafix/zedone/pull/96),
merge 02ae231 после всех 17 SUCCESS. Недостающая финальная Device-приёмка
завершена в [EV-104](EV-104-device-pairing-ui-acceptance.md); документальные
итоги интегрированы через PR #97. P06.01.3/.3.2/.4 и весь ручной P06.01 done,
GAP-029 resolved. Обычная IME/Back и парный APK/API/CLI runner приняты без
изменения продуктового source 02ae231. Текущий статус —
[журнал](../plan-journal.md), LOG-568/570/571.

Ниже сохранена история реализации и остановки на момент PR #96. Указанные
здесь blocked/непроверенные финальные формы впоследствии приняты в EV-104;
причина прежних внешних ошибок не объявляется доказанной. QR/обмен отдельно.

## Рабочая карточка

- База: ee01d65, ветка feat/p06-01-device-access-ui. Четыре исходных dirty
  документа сохранены отдельно с SHA-256; EV-100 не относится к новому коду.
- Цель: весь оставшийся P06.01 в одной ветке по прямому поручению разработчика,
  без промежуточных PR. Проверяемые части выполняются последовательно.
- Зависимости: P06.01.1–.2 done, EV-093–102, PR #95 merge ee01d65.
- Требования: T09/A08/A51, ADR-0009/0010/0053–0059; полные T09/A51 требуют
  будущих QR/обмена, не закрываются только ручным подключением.
- P06.01.3.1: общий host-контроль listener и безопасного исходящего приглашения;
  один DeviceHost/Service для web/IPC/CLI, код в OS-store, без plaintext fallback.
- P06.01.3.2: формы в настройках, список приглашений/запросов/прав, ручной ввод,
  ожидание/подтверждение/отклонение/отмена/отвязка/выдача/отзыв/смена прав;
  выбранные пространства, read и «До отзыва» по умолчанию, отдельное administration.
- P06.01.4: итоговая матрица UI/API/CLI Windows/Android, restart, права, stale,
  потеря ответа/повтор, сохранность и все обратные действия.
- Вне объёма: QR P06.02, обмен P06.03+, упаковка/bootstrap P08. Новые зависимости
  и миграция схемы не предполагаются.
- Дизайн: существующие canvas #e4e8e6/surface #f2f4f3/ink #202d28/accent #315b50,
  тёмные токены без изменений; Segoe UI для форм, Georgia для существующих заголовков.
  Разделы настроек с общими разделителями, явными статусами и действиями;
  подписи слева, перенос длинных идентификаторов, обычные элементы ввода/кнопки.
  Декоративные панели и новая типографическая система не требуются.
- Проверки: product-check Windows/native Android, ui:check, сборки обеих ABI,
  Android UI только через Device/agent-device, общий API/CLI и реальные OS stores.
  Точные команды/результаты будут записаны после выполнения; пока не пройдены.
- Обратимость: cancel/reject/unlink/revoke/regrant; отказ не стирает ввод,
  неизвестный исход не создаёт новое намерение или ключ; свежий inspect перед
  новым действием, никакой прямой записи агента в SQLite.

## Реализация и результат на момент PR #96

P06.01.3.1 done в компонентном срезе: PR #96/head ac0db9e merged 02ae231
после всех 17 SUCCESS/readiness; LOG-563. Controller объединяет старые
native start/stop и новый UI/host dispatch; Windows/web и Android/IPC используют
один DeviceHost/Service. Новый защищённый outgoing slot содержит исходный Offer,
проверяется чтением до публикации и сверяется с сохранённой TLS identity.
SQLite и зависимости не менялись. Код не попадает в намерения/receipts.

P06.01.3.2 реализован, Windows проверен; финальная native UI-приёмка blocked
по GAP-029. P06.01.4 выполнен частично, весь P06.01 не закрыт. Реализация
интегрирована через PR #96 после всех 17 SUCCESS; финальная local UI-приёмка отдельно.

В настройках есть ручное приглашение/запрос/ожидание, подтверждение выбранных
пространств, read/до отзыва по умолчанию, явные режимы каждого пространства,
RW/administration/конечный срок, отмена/отклонение/отвязка и issue/revoke/replace.
Формы фиксируют исходного субъекта/свежий expected и сохраняют отказ/намерение;
удаление только записи доставки требует отдельного подтверждения.

## Локальные проверки

| Охват | Результат и граница доказательства |
| --- | --- |
| `bun scripts/check-product.mjs` | 340 Rust, 27 client/280 assertions, release/unsafe/контракт/типы passed. После заключительного уточнения typed invitation snapshot/identity guard — отдельные финальные host unit/clippy и UI/CLI; исходный полный вывод сохранён, не подменяется новым запуском. |
| `bun scripts/check-product.mjs --android` | 337 native Rust, 27 client/280 assertions, release обеих ABI, guards/контракт, host/native парные probes passed. После заключительного guard — новый native x86_64 unit того же сценария восстановления Offer passed. Это проверка native Rust, не APK UI. |
| `bun run ui:check` | Полная прежняя desktop/touch матрица и новые два независимых pairing fixtures passed. Новая логика shell прошла также финальные 3 host tests; финальные адресные UI/CLI проверки и clippy passed после освобождения своего exe от параллельного теста. |
| `bun scripts/check-ui-pairing.mjs` | Ожидание без grant, default read/до отзыва, разные режимы пространств/явные RW/admin/срок, confirm/cancel/reject/unlink, issue/revoke/replace, stale/отказ/сохранённое намерение/явное удаление записи, lost-reply исходный JSON и сохранность passed. Windows fixtures используют настоящий Credential Manager, Linux fixture явно Memory. |
| Финальный host/native unit Offer | Нет создания секрета без полномочий/identity, reopen/retry, отменённый код не оживает, terminal→новый код, повреждение без замены, восстановление OS persistence→SQLite publication passed; тип текущего статуса берётся из Rust InvitationSnapshot. |
| CLI pinned TLS | `offer.create/read`, обычная `invitation.cancel`, новый код и OS cleanup passed через общий Service. APK runner переведён на этот же путь; его новый полный APK-запуск локально не выполнялся после остановки Device. |
| `build-ui-android.mjs x86_64` / `aarch64` | Обычные debug APK, instrumentation APK и manifest обеих ABI собраны на итоговом source; release/ABI ограничения ядра проверены product-check. Дополнена outgoing-slot instrumentation, её новый отдельный запуск не выполнялся. |
| Device APK/Keystore | Приглашение на Android → запрос из Windows UI → локальное пространство Android → явное read-only подтверждение в Android UI → restart APK/явный прежний listener/pin → подтверждение в Windows → собственная отвязка Windows passed до последних исправлений формы. Не выдаётся за финальную полную матрицу. |

Точные версии, команды локальных override, raw выводы, APK, backup, screenshots,
source fingerprints и manifest сохранены только в ignored `docs/reports/`.

## Найдено и исправлено до приёмки

- Kotlin bridge дополнен точным outgoing slot whitelist; AES/GCM/AAD/лимиты
  прежние, plaintext fallback не добавлен.
- Неуспешный ответ CommandClient остаётся pending по прежней политике. Форма
  теперь показывает точную причину, сохраняет ввод и предлагает исходный повтор
  или отдельное подтверждаемое удаление записи, без автоматического rebase.
- Новый диалог заменён прежним action-dialog: старый AOSP WebView не поддерживает
  dvh, из-за чего нижние действия не прокручивались. После исправления native
  подтверждение выбранного пространства прошло.
- Visibility probe размонтировал формы. Отдельный Windows negative control
  воспроизвёл потерю адреса (пустое значение), после исправления форма сохраняется;
  действия при действительном отказе отключаются, смена субъекта блокируется.
  Это не доказывает связь с исходной ошибкой драйвера.
- Параллельная сборка не смогла заменить свой запущенный shell-fixture.exe
  (Windows Access denied). После завершения UI-процесса финальные host tests,
  build, clippy и targeted UI/CLI прошли; runtime-обхода или изменения CI budgets нет.

## GAP-029: историческая остановка до EV-104

На обратном направлении (Android вводит адрес Windows) agent-device сообщил
`Android fill verification failed`. Единственный повтор по совету инструмента
с `--debug` после свежего snapshot сообщил `Fill target disappeared after focus`.
По инструкции Device дальнейшие UI-попытки прекращены; adb/CDP не подменяли ввод.
Первопричина не доказана, финальная версия после сохранения формы не принята
на native UI. UI с camera/QR/обменом вообще не включён в P06.01.

Cleanup выполнялся отдельно от тестовых действий: свои Windows/Android OS keys,
forward/reverse и fixture удалены; исходный APK и все app files восстановлены
по SHA-256, Device выключен. Исходные dirty EV-100/EV-102 не перезаписаны.

Далее: восстановить работоспособную Device-проверку и на этой же ветке завершить
все native формы/IME/Back/restart, запрос и подтверждение в обе стороны,
grant defaults/явные режимы/срок/отзыв, stale/неизвестный исход/повтор и обратные
действия с сохранностью (.3.2, затем .4). Новый CI/интеграция отдельно.
Полные T09/A08/A51 и прежние GAP/DEC не повышаются этим частичным результатом.

## Пять правил

- Обратные действия: cancel/reject/unlink/revoke/regrant, отдельное удаление
  записи доставки без отмены операции; проверены в Windows и native подмножестве.
- Человек/агент: один Service/host dispatch, JSON contract/CLI/IPC, без прямой записи SQLite.
- Документация: user/agent/development/architecture, ADR-0060 и эта карточка обновлены.
- Windows/Android: общий код/обе ABI/native suites; полная финальная UI-приёмка
  Android остаётся GAP-029, паритет не объявляется полностью принятым.
- Источник истины: SQLite registry и Rust-derived contract; OS secrets, UI только проекция.


## Подготовка публикации — LOG-561

По новому поручению разработчика готовятся PR и условные merge/delete после
успешного полного CI/readiness. Это интеграция реализации, не закрытие GAP-029
или полной Android-приёмки. Scope в одной ветке соответствует LOG-558.
Объём больше ориентира малого PR: безопасное хранение кода, общий host и формы
вместе образуют проверяемый ручной процесс; разрывать его на опубликованные
неполные операции не требуется. Тесты/документация входят в тот же результат.

38 выбранных файлов проверены против main/ee01d65; 26 source fingerprints
совпадают с итоговыми локальными проверками. Исходные dirty EV-100/EV-102 вне
коммита. Before/After настоящего base/final UI сняты с одинаковой пустой
synthetic installation, dark/1280×900, визуально проверены; media и PR body
хранятся вне репозитория, публикация использует gh --attach. После base capture
финальный shell build восстановлен, check-ui-pairing passed повторно.


## Публикация — LOG-562

[PR #96](https://github.com/Zeyafix/zedone/pull/96), head
ac0db9e5593f50c859388a6c679b196db687e5f5, base main/ee01d65. 38 файлов,
все 26 fingerprints и body/readback подтверждены. Два настоящих static UI
before/after размещены через gh --attach и проверены как hosted PNG/1280×900.
Исходные media/body вне репозитория; EV-100/EV-102 вне коммита и сохранены.

Первый readback: 14 runtime suites queued/in_progress, три strict aggregates
ожидаются. Новый полный CI pending, нужны все 17 SUCCESS и свежая readiness
того же head перед порученными merge/delete. Reviews/comments/requests/threads
отсутствуют. GAP-029/.3.2/.4 остаются blocked, публикация их не закрывает.


## Интеграция — LOG-563

[PR #96](https://github.com/Zeyafix/zedone/pull/96), head
ac0db9e5593f50c859388a6c679b196db687e5f5, merge
02ae2310a9c5f0700b2c5f8d1fbb04217c110536 после ровно ожидаемых 17
COMPLETED/SUCCESS и свежей CLEAN readiness. Comments/reviews/requests/threads
отсутствовали; финальный body/credit/две hosted images/38 файлов прочитаны обратно.

Raw Windows/Linux shell подтвердили прежний полный UI и новые два pairing
fixtures/defaults/явные режимы/срок/admin/ключи/stale/retry/обратимость/сохранность.
Raw Android shell подтвердил новую outgoing-slot instrumentation, весь прежний
native UI и новый полный paired-APK runner через offer.create/read, Keystore,
TLS/API/CLI restart/права/replay/reconnect/обратные действия/пять попыток. Cleanup
OS keys/rules и восстановление APK/всех app files по SHA-256 passed. Обе ABI,
product/prototype и strict aggregates SUCCESS. Linux CI host Memory не подменяет
локальное Windows Credential Manager доказательство. Новые локальные APK rerun
после остановки Device не выполнялись; теперь эти проверки имеют отдельное CI proof.

Merge parents — прежний main и checked head, дерево равно checked head. Main
обновлена до origin/main; feature-ветка удалена local/remote/tracking. Четыре
dirty документа сохранены по SHA-256 при switch; EV-100/EV-102 не изменены.
Индекс пуст, после merge нет новых source commits/push. Итоговые записи
журнала/очереди/EV-103 остаются локальными.

P06.01.3.1 done в host-control scope; .3.2/.4 остаются blocked GAP-029.
Полный финальный native UI не входит в выполненную CI-матрицу; успешный CI не
доказывает причину исходных Device fill failures. Все 98 определения/состояния
приёмки и прежние GAP/DEC сохранены. Пять правил соблюдены в проверенном срезе,
полный финальный UI-паритет остаётся незавершённым. Далее — финальная локальная
Device-приёмка на main после восстановления инструмента.
