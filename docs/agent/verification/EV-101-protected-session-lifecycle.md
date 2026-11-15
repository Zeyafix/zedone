# EV-101 — Защищённый жизненный цикл сессии устройства

Это checkpoint первой внутренней части. По поручению LOG-547 работа продолжена
до всего P06.01.2.5 в той же ветке, без промежуточной интеграции. Актуальная
сквозная приёмка и передача — [EV-102](EV-102-two-device-pairing.md)/LOG-548.
Указания ниже на ожидание отдельной интеграции .1 являются историческими.

## Рабочая карточка

- Шаг: P06.01.2.5.1, verified локально; ветка feat/p06-01-2-5-pairing-acceptance.
- База: main/bcc9b20; три исходных dirty agent-документа сохранены в ignored
  docs/reports/p06-01-2-5-start до изменения. P06.01.2.4 done, EV-100/LOG-544.
- Цель: подтверждённый ключ переживает переход из временного OS-слота;
  удаление разрешено только после свежего терминального ответа сервера.
- Требования: T09/A51, ADR-0055/0056/0057; полные состояния прежние.
- Scope: adapters/pairing и фиксированные OS-слоты Windows/Android,
  проверки отказов/восстановления, документация.
- Проверки: check-product Windows/native Android/arm64, настоящий TLS,
  Windows Credential Manager и Android Keystore, сбои на границах переноса,
  несовпадающие/повреждённые записи, повтор и сохранность данных.
- Обратимость: отмена/отклонение ожидания, unlink/revoke подтверждённой связи,
  свежий терминальный inspect и очистка; новое приглашение/ключ.
- Вне среза: постоянный listener/API/CLI-композиция (.2.5.2), полный
  Rust→Kotlin→TLS app flow и итог двух устройств (.2.5.3), формы (.3), обмен.
- Результат: Windows/native Android, обе release ABI, debug APK x86_64 и
  настоящий Android Keystore passed; полный app flow этим не принимается.
- Далее: ревью/интеграция .2.5.1; зависимая .2 после интеграции.

## Реализация

ADR-0058: один SessionLifecycle на namespace сериализует create/submit/refresh/
cancel/unlink/connected. Свежий pinned TLS inspect с внутренним случайным ID
решает локальный переход; исторический replay не является доказательством.
Подтверждённая сессия копируется в ConnectedPairing без перезаписи, проверяется
чтением, затем удаляется PendingPairing с проверкой отсутствия. Формат ZDPN v1
и исходные ключи/request/pin прежние. Равные копии после прерывания допускают
восстановление; несовпадающие или повреждённые записи не заменяются.

Новая запись не выдаёт прав: каждое обычное действие идёт через прежний Service
и актуальные access_*. Старый ключ не получает replay прежнего успеха после
отзыва. Cancel/unlink сохраняют исходные намерения; только новый терминальный
inspect разрешает очистку. Неизвестный сетевой исход сохраняет все имеющиеся
копии. Новое create не заменяет существующую или нечитаемую сессию.

Нет новых endpoints, renderer-команд, DDL, прав, зависимостей, unsafe или
изменений API-контракта. Поддержан фиксированный connected-pairing-v1 в
Kotlin store/plugin allowlist; renderer dispatcher по-прежнему закрыт.

## Проверки

| Проверка | Результат |
| --- | --- |
| Полный `bun scripts/check-product.mjs` Windows | exit 0: 336 Rust (20 adapters lib/42 commands/17 core/7 contract/250 storage), 27 client/280 assertions, fmt/Clippy -D warnings/release, 7 negative unsafe guards и равенство контракта/типов |
| `bun scripts/check-product.mjs --android-native` через Device | exit 0: 333 native Rust (17 adapters lib/42 commands/17 core/7 contract/250 storage), те же client/контракт/типы; x86_64 release и 7 unsafe guards; sandbox/reverse очищены |
| Прежние парные TLS/request probes | Оба явно вызваны штатным runner: неверный pin, сохранённая identity, двусторонний TLS, pending/restart/replay/inspect/cancel без доступа; новой итоговой парной приёмкой не объявляются |
| `bun scripts/check-product.mjs --android-arm64` | exit 0: aarch64 release all-targets и 7 unsafe guards, те же client/контракт/типы; физического arm64 нет |
| `bun scripts/build-ui-android.mjs x86_64` | exit 0: debug APK и instrumentation APK/manifest, Kotlin/Rust host bridge; production UI build passed с прежним предупреждением размера chunk |
| `bun scripts/check-host-secrets-android.mjs` | exit 0: настоящий Keystore, новый connected-слот/повторное открытие/запрет замены/AAD-изоляция от pending/независимое удаление; прежняя матрица corruption/key loss/recovery/cleanup passed |

Восемь lifecycle tests Windows/семь native Android включают шесть точек
отказа переноса: до/после create, readback, до/после remove и ложный успешный
remove без удаления. После повторного открытия сохраняется тот же ключ;
проверены отмена/отклонение/отвязка, точные повторы, отзыв при двух копиях,
новый request/ключ после очистки, повреждение/конфликт записей, wrong pin/
потеря ответа и replayed/несогласованный статус. Отдельный Windows test
проходит полный lifecycle с настоящим Credential Manager и новым экземпляром
store после закрытия. Сохранение identity проверено независимо от pairing.

Исходный APK сохранён перед установкой проверочного. После instrumentation
восстановлен побайтово (SHA-256), состав и SHA-256 всех 46 файлов приложения
совпали с исходными; исходные данные не пересоздавались. Test APK/namespace/
Keystore keys/sandbox/reverse очищены. Эмулятор открыт и выключен через Device;
UI не менялся, новая UI-матрица не запускалась.

Версии машины, backup, stdout/stderr и отпечатки девяти runtime/test файлов
сохранены только в ignored docs/reports/p06-01-2-5-start. Все 98 определений/
состояний приёмки, прежние события и исходный EV-100 сохранены; критерии
получают только дополнительное доказательство, без повышения состояния.

## Пять правил и передача

1. Обратимость: cancel/reject/unlink/revoke, безопасная очистка и новое
   приглашение/новый ключ; сбои не теряют действующий ключ и не удаляют данные.
2. Человек/агент: один host-компонент и прежний общий Service/HTTP/IPC/CLI;
   новых обходов прав и прямых записей SQLite нет, формы ещё отдельно.
3. Документация: user/device-pairing, agent/pending-device-request/очередь/
   журнал/EV, development/product-core и ADR-0058 обновлены.
4. Паритет: Windows/native Android и обе ABI, настоящий Credential Manager
   и Keystore в указанном компонентном охвате; полный app flow не принимается.
5. Один источник: SQLite access_* определяет права; OS-слоты содержат ключи,
   но не копию полномочий. Контракт остаётся Rust-derived без изменений.

Новых BUG/GAP не обнаружено. Полные T09/A51 и прежние GAP/DEC сохраняют
состояния. .2.5.1 verified, родительские .2.5/.2/P06 in_progress.
Commit/push/PR/новый CI/merge не выполнялись. Далее ревью/интеграция этого
самостоятельного среза перед .2.5.2. Active_work очищен; зависимый шаг не начат.
