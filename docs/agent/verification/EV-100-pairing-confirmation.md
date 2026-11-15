# EV-100 — подтверждение подключения и отзыв доступа

## Текущий статус

P06.01.2.4 done в host/API-срезе: [PR #94](https://github.com/Zeyafix/zedone/pull/94),
merge bcc9b20 после всех 17 SUCCESS. Последующая сквозная приёмка и формы
завершены в [EV-102](EV-102-two-device-pairing.md) и
[EV-104](EV-104-device-pairing-ui-acceptance.md); весь ручной P06.01 done,
GAP-029 resolved. Текущий статус проекта ведётся в [журнале](../plan-journal.md),
LOG-568/570/571. Это не расширяет исходный host/API scope EV-100.

Ниже сохранены результаты данного среза и история его публикации. Упоминания
ещё не выполненных app flow/форм относятся к моменту соответствующей записи.

## Рабочая карточка

- Шаг: P06.01.2.4, done в host/API-срезе; PR #94/bcc9b20; ветка feat/p06-01-2-4-pairing-confirmation.
- База: main/993d98f (продукт 74aa615 плюс документы интеграции); зависимости .2.1–.2.3 интегрированы, EV-099/LOG-538.
- Цель: только отдельный администратор подтверждает или отклоняет запрос;
  запись прав и решение атомарны. Отвязка отзывает доступ без удаления данных;
  повторное подключение требует нового приглашения и ключа.
- Требования: T09/A08/A51, ADR-0009/0053/0056; состояния полных критериев прежние.
- Scope: core/storage/adapters, Rust-derived контракт, проверки и документация.
  UI/внешний listener/обмен пространствами/итог .2.5 не входят.
- Проверки: права и невозможность самоподтверждения; expected/replay/restart,
  SQL/receipt/commit rollback, конкуренция; общий HTTP/IPC/CLI;
  check-product Windows/native Android/обе release ABI и сохранность данных.
- Обратные действия: отклонение/отмена ожидания, отвязка/отзыв выданного доступа,
  новое приглашение и новая выдача; старый ключ и код не оживают.
- Размер: одно решение безопасности от запроса до grant/receipt и обратно;
  отдельные формы и общая приёмка сохранены в очереди.
- Результаты: Windows/Linux/native Android/обе ABI/контракт/полный прежний UI passed; PR #94 слит после всех 17 SUCCESS.


## Реализация и воспроизводимые проверки

ADR-0057: pairing.confirm/reject/unlink через общий Service, ограниченная TLS
peer_unlink и host-only session.credential. Решение/обычный grant/receipt
фиксируются одной транзакцией. Снимок содержит nullable subject, а его версия
учитывает текущий grant/активность исходного verifier. Отзыв/замена обычного
ключа не оставляют действующей прежнюю связь; нет второго реестра прав.

Схема v1 и её синтетический эталон обновлены по ADR-0047. Независимое сравнение
всех прежних таблиц эталона подтвердило равенство строк, в том числе графов,
actor, receipts, прав и часов; прежняя pairing_requests была пустой. Старая
схема решения отклоняется до записи; сброс данных не добавлен.

Целевые проверки:

- Storage: подтверждение/replay/reopen/отвязка; терминальные reject/cancel;
  гонка двух writers confirm/reject; rollback при записи access_keys,
  решения, receipt, AfterWork и BeforeCommit; отсутствие частичных principal.
- Ошибочные сроки/пустые области/неизвестные пространства/коллизия subject;
  обычные revoke/replace, stale unlink, запрет оживления старого confirm;
  граница срока 199/200 и отсутствие прав на пространства у администратора.
- Peer unlink: отзыв собственного grant; откат при ошибке receipt;
  отказ удалить последнего бессрочного администратора. Отозванный администратор
  не получает прежний confirmation receipt.
- Настоящий TLS на loopback: запрос не подтверждает себя ни pending, ни будущим
  access bearer; confirm отсутствует в ограниченном TLS router; явное чтение
  позволяет space.read и отказывает space.rename. После unlink старый ключ
  отказывает; новый запрос по новому приглашению получает другой ключ и права,
  старый ключ остаётся отозванным, пространство сохраняет имя.
- Общие HTTP/IPC/настоящий CLI: confirm/reject/unlink с едиными receipts,
  отказ обычному участнику, stale и command_mismatch. Клиент сохраняет
  исходную команду после потери ответа; Rust-derived validator отвергает
  verifier/неизвестный статус/ответ другого типа.
- UI types (Node vue-tsc) и production build passed. Визуального diff нет.
  Предупреждение Vite о размере chunk сохранено без изменения порога.
  Локальный Node отличается от baseline; точная версия только в ignored
  metadata, повторение на CI baseline здесь не принимается.

Исходная ревизия: main/993d98f, dirty состав из 20 файлов crates/api/tests;
агрегат SHA-256 манифеста имён и SHA-256 байтов:
`43fc497c6776d56a083db11827b4753afbae3132778fa67b9d2eff7997f673ca`.
Сам манифест, локальные версии, stdout/stderr и промежуточные отказы — в
ignored docs/reports/p06-01-2-4-*. Сначала исправлены устаревшие счётчики
контракта, закрытость test-fixture поля и повтор ID нового намерения; проверки,
сроки, попытки и CI не ослаблялись.

## Полные результаты и пять правил

- `bun scripts/check-product.mjs`: exit 0; fmt, Clippy -D warnings,
  328 Rust-тестов (два парных probes по умолчанию ignored), release all-targets,
  семь negative unsafe guards, Rust/OpenAPI/TypeScript/validator equality.
- `bun scripts/check-product.mjs --android-native`: exit 0; 326 native Rust
  тестов, x86_64 release/семь guards, точное равенство native/host контракта.
  Обе прежние пары host/native probes вызваны явно: TLS/pin/host identity и
  pending request/reopen/replay/inspect/cancel без доступа. Это прежние пробы,
  они не объявляются итоговой приёмкой нового confirmation на двух устройствах.
- `bun scripts/check-product.mjs --android-arm64`: exit 0; aarch64 release
  all-targets/семь guards. Физический arm64 не запускался.
- Каждый check-product: 27 client tests/280 assertions, тот же транспорт и
  долговечные исходные намерения, контракт/типы/validator совпали с Rust.
- UI types/production build passed; полный UI/новый shell CI не выполнялись:
  визуального diff нет, host API не добавлен в формы.
- Device открыт инструментами T3, тесты запущены штатным CLI в собственном
  sandbox. APK/пользовательские данные не устанавливались и не менялись;
  test sandbox/reverse очищены штатным runner. Эмулятор выключен через Device.

Пять правил соблюдены в этом host/API-срезе:

1. Обратимость: reject/cancel ожидания, unlink/revoke прав, новое приглашение
   и новый ключ для повторной выдачи; данные/история не удаляются.
2. Человек/агент: один core/Service и HTTP/IPC/CLI; получатель использует общий
   host API с той же политикой без самоподтверждения или прямой записи SQLite.
3. Документация: user/agent/development/architecture, ADR-0057 и EV обновлены.
4. Паритет: Windows/native Android, реальный TLS loopback на обеих платформах,
   обе release ABI и прежние парные пробы в указанном точном охвате.
5. Источник истины: grant/активность ключа только в access_*, решения/receipts
   в той же SQLite транзакции; один OS slot, Rust-derived контракт.

Полные T09/A08/A51 и прежние GAP/DEC не повышаются. Полный Rust→Kotlin→TLS
процесс, отдельный lifecycle OS-слота подтверждённого устройства/внешний listener,
итоговый сценарий двух устройств/API/CLI — .2.5; формы — .3, обмен — P06.03.
Android Keystore bridge/новый APK/полная UI-матрица/физический телефон и новый
полный CI этим diff не принимаются. Новых продуктовых BUG/GAP не выявлено.

Индекс пуст, commit/push/PR/новый CI/merge не выполнялись. Далее ревью и
интеграция P06.01.2.4 перед P06.01.2.5; новая реализация не начата.


## Публикация — LOG-543

[PR #94](https://github.com/Zeyafix/zedone/pull/94), head
1eb6776553df135e89a47605629b0048c87abaaa, base main/993d98f. 28 согласованных
файлов и все 20 tested source fingerprints подтверждены; обычный push/pre-push
passed. Body/readback подтвердил три English sections/credit и точные границы.
Comments/reviews/requests/review threads отсутствовали; визуального diff нет.

Новый полный CI pending: 14 runtime suites IN_PROGRESS, три strict aggregates
ожидаются. Нужны все 17 COMPLETED/SUCCESS и свежая readiness того же head перед
порученными merge/удалением ветки. Все 98 состояния/GAP/DEC и пять правил EV-100
прежние; полного процесса/UI/обмена этим не принимаем. Записи публикации пока
локальные без нового source push, active_work null; новой реализации нет.


## Интеграция — LOG-544

[PR #94](https://github.com/Zeyafix/zedone/pull/94) интегрирован: head
1eb6776553df135e89a47605629b0048c87abaaa, merge
bcc9b20a034a41e303cd7f8ce2bd95704feb2245 после ровно ожидаемых 17
COMPLETED/SUCCESS и свежей CLEAN readiness. Body обновлён фактическими
результатами CI и прочитан обратно; reviews/requests/comments/threads отсутствовали.

Raw product logs Windows/Linux/native Android подтвердили новые
confirm/reject/unlink/права/SQL/гонки/restart/отзыв/ротацию/сохранность;
real TLS loopback проверен на каждой платформе. Две прежние парные host/native
пробы явно выполнены. Raw shell logs подтвердили весь прежний desktop/touch/
native UI/history/cleanup/accessibility/Markdown и Keystore; обе ABI/prototype
suites и strict aggregates SUCCESS. Это расширяет доказательства прежнего UI,
не добавляет приёмку новых форм или итогового двухустройственного confirmation.

Дерево merge равно checked head, main обновлена до origin/main/bcc9b20.
Два dirty документа сохранены побайтово при switch; feature-ветка удалена
локально/на GitHub/tracking. Индекс пуст; новый source push/commit после merge
не выполнялся, записи интеграции остаются локальными.

Пять правил и границы выше сохраняются. P06.01.2.4 done в host/API-срезе,
все 98 определения/состояния/GAP/DEC прежние. Полный app flow/итог двух устройств
— .2.5; формы/обмен/физический телефон отдельно. Далее готов P06.01.2.5 по
новому поручению; active_work null, новая реализация не начата.
