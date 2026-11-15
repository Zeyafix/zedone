# Запрос устройства: доступный host-срез

P06.01.2.5 связывает host API, защищённый listener и сессию устройства.
Формы ручного подключения доступны в настройках P06.01.3; обмен пространствами
остаётся отдельным шагом.
Человек и агент используют один процесс; прямые записи SQLite запрещены.
Неподтверждённое устройство не вызывает обычный Service или bootstrap.

1. Создать приглашение административной invitation.create. Код хранить отдельно
   от намерений. Создающий host связывает его с сохранённой TLS identity:
   `Offer::new(invitation, identity.fingerprint(), raw_code).encode()`.
2. Передать составной код доверенным внешним способом. Получатель вызывает
   Offer::decode, затем PendingSession::create(store, offer, label). Сохранение
   OS-слота обязательно до первого запроса; ошибка не означает отсутствие.
3. Получатель передаёт открытый TCP stream в session.submit; TLS проверяется
   внутри. Создающая сторона обрабатывает сокет через PairingHost::serve.
   Обычный HTTP POST или получение pin от непроверенного сервера не заменяют
   этот путь. Host сериализует жизненный цикл собственного OS-слота.
4. При потере ответа загрузить PendingSession::load и повторить исходный submit.
   Не менять ID, имя, ключи или pin. Успех с replayed — исходный результат;
   для текущего статуса вызвать session.inspect с новым случайным command ID.
5. Отмена: session.cancel с новым ID и version свежего inspect. Сохранить
   ID/expected для повторной доставки той же отмены. При stale_state перечитать
   и запросить новое явное действие. После подтверждённой отмены разрешено явно
   удалить Slot::PendingPairing; при неизвестном исходе сохранить.

Администратору доступны обычные команды:

- pairing.inspect: `{request}`;
- pairing.cancel: `{request, expected}`;
- pairing.list: `{after:null}` или исключительный cursor next, до 50 записей.

Все три требуют ManageDevices, доступны HTTP `/v1/commands/<operation>`,
IPC zedone_command и CLI --operation, используют прежние receipts/права.
PairingSnapshot содержит request, invitation, label, created_at_unix_ms,
status (pending/cancelled/rejected/confirmed/unlinked), nullable subject, version; секретов и verifier в нём нет.
Истечение/отзыв административного доступа запрещают также replay.

Приглашение одноразовое; пять неверных корректно оформленных кодов блокируют
его, срок первого приёма — десять минут. Приглашения показывают также used/locked;
version меняется после попытки или приёма запроса. Отмена приглашения отменяет
ожидание. Принятое ожидание сохраняется после истечения кода/перезапуска;
доступа к пространствам, права подтверждения или обмена оно не даёт.
Для нового запроса нужны новые приглашение и код. Код/OS-секреты нельзя
передавать в чат, argv, журналы или обычный client intention store.

Протокол и границы — [ADR-0056](../architecture/decisions/0056-pending-device-request.md).


## Подтверждение, отклонение и отвязка — P06.01.2.4

- pairing.confirm: `{request, expected, scope, expires_at_unix_ms}`. Scope имеет
  тот же формат, что access.issue; все права выбираются явно, чтение и срок
  «до отзыва» задаются `read` и `null`. Администрирование само по себе не даёт
  доступа к пространствам. Требуются ManageDevices и ManageAccess.
- pairing.reject: `{request, expected}`, только ManageDevices.
- pairing.unlink: `{request, expected}`, ManageDevices и ManageAccess.

Все доступны через прежние HTTP/IPC/CLI и один атомарный Service. Исходное
подтверждение создаёт обычный grant с subject=request и заранее сохранённым
verifier получателя; ключей в результате нет. После подтверждения host использует
session.credential() для прежнего API. OS pending slot нельзя удалять напрямую:
он содержит действующий ключ. Перенос и очистку выполняет SessionLifecycle,
описанный ниже. Rust→Kotlin→TLS процесс и формы связываются общим host.

Получатель может только inspect/cancel своего ожидания или unlink собственной
подтверждённой связи через session.unlink(stream, id, expected). Отмена уже
подтверждённой связи требует unlink; отклонённый/отменённый запрос нельзя
подтвердить новым ID. Отвязка не удаляет локальные пространства или историю.

Для свежего состояния всегда новый ID. Replay confirm после unlink возвращает
исторический confirmed без выдачи прав. Access.revoke/replace отражаются как
unlinked, версия меняется вместе с обычным grant; старый expected не затрагивает
новые права. Истечение прав запрещает действия на границе срока, но само по
себе не отвязывает устройство. Новый запрос требует новых приглашения и ключа.
Потеря ответа — сохранить исходное намерение/OS slot и повторить тот же ID;
stale_state — перечитать и получить новое явное действие.

Решение — [ADR-0057](../architecture/decisions/0057-pairing-confirmation-and-unlink.md),
проверки — [EV-100](verification/EV-100-pairing-confirmation.md).

## Сохранённая сессия — P06.01.2.5.1

Host создаёт один `SessionLifecycle::new(store)` на namespace и проводит через
него create/submit/refresh/cancel/unlink/connected. Второго владельца того же
namespace быть не должно. Это Rust host API, а не команда renderer/HTTP.

`refresh(stream)` сам генерирует новый ID inspect. Pending сохраняется;
confirmed переносит точные байты в защищённый connected-pairing-v1, проверяет
чтением и лишь затем удаляет pending-pairing-v1. Равные копии после сбоя
восстанавливаются следующим refresh. Разные/нечитаемые записи не заменяются.
`connected()` возвращает host-only ключ только из постоянного слота; наличие
записи не заменяет проверку актуальных прав каждой обычной командой Service.

Cancel/unlink требуют сохранённых исходных ID/expected, как прежде; их точный
повтор не очищает секреты. После известного ответа вызвать свежий refresh:
только cancelled/rejected/unlinked разрешают очистку. При ошибке или неизвестном
исходе сохранить слот и повторить refresh после восстановления связи.
Для следующего create сначала завершить очистку; identity и данные не удаляются.
[ADR-0058](../architecture/decisions/0058-protected-session-lifecycle.md).

## Постоянный host и общий CLI — P06.01.2.5

`DeviceHost` использует один SessionLifecycle/Service. Явный start требует
действующего административного ключа и запускает bounded TLS listener.
Shutdown прекращает приём, не создаёт новую identity и не очищает сессию.
После нового запуска host загружает прежний namespace/identity, а сессия
ZDPN v2 содержит также исходный IP-адрес и порт. Сырой ключ остаётся в OS-store.

Windows CLI: `zedone --listen <IP:port> --database <existing-db> --key-file <admin-key>`.
Первая строка stdout содержит публичные address/pin; завершение — Ctrl+C.
CLI не создаёт административные права. Native Android использует
device_host_start/stop и device_session с тем же DeviceHost; автоматическая
фоновая работа закрытого APK не обещается.

Для доставки: `zedone --endpoint <IP:port> --pin-file <pin> --key-file <key> --operation <operation>`.
Pin/ключ — 64 hex символа в файлах, секреты не попадают в argv. Stdin содержит
исходное JSON-намерение. Обычные команды идут в `/v1/commands/<operation>` и
проверяют собственный выданный доступ агента. `session.*` идут в
`/v1/host/<operation>`, требуют ManageDevices локального host и принимают:

| Операция | Stdin/parameters |
| --- | --- |
| session.prepare | `{address, offer, label}`; offer — составной код, не сохранять в обычном intention store |
| session.status | `{}`; только локальная готовность, не подтверждение действующих прав |
| session.submit | `{}`; повтор исходного сохранённого запроса |
| session.refresh | `{}`; новый сетевой inspect и безопасный локальный переход/очистка |
| session.cancel / session.unlink | `{id:[16 bytes], expected:version}`; сохранять исходное намерение для повтора |

prepare не заменяет существующую запись. После потери ответа сначала status,
затем submit/refresh исходной сессии; не создавать новый ключ. Неверный pin,
недоступность или неизвестный результат сохраняют запись. Известный постоянный
отказ submit позволяет очистку: отрицательный receipt запрещает позднюю
активацию этой же попытки. Один повтор неверной попытки не расходует лимит
заново; пять отдельных неверных попыток блокируют приглашение.

В API нет session.command или выдачи сохранённого remote key. Администрирование
локального host не является неявным доступом к удалённым пространствам.
Низкоуровневый trusted native peer_command заимствует device key; агент для
обычных команд использует собственный явно выданный key и тот же Service.
При stale_state нужно свежее чтение и новое явное действие; автоматически
подменять expected нельзя. [EV-102](verification/EV-102-two-device-pairing.md),
[ADR-0059](../architecture/decisions/0059-device-host-and-paired-acceptance.md).


## Ручные формы и исходящие коды — P06.01.3

UI-агент использует «Настройки» → «Подключение устройств»/«Запросы и доступы»;
подробные действия [как для человека](../user/device-pairing.md). Прокрутите
нужную форму, используйте checkbox для явных пространств/RW/administration/срока
и отдельного подтверждения. Defaults — чтение/до отзыва, пространства не выбраны.

На Android с обычной IME перед выбором следующего перекрытого поля скройте
клавиатуру системной Back через Device-owned `agent-device back --system`.
Затем получите свежий snapshot и проверьте сохранность значений и текущий
раздел. `keyboard dismiss` доступен только при поддержке текущей IME; отказ
инструмента обрабатывается по инструкции Device. Проверка — EV-104/LOG-568.

Windows web host: POST `/host/devices/<operation>`, один Bearer, точный
application/json; разрешены только собственные Host/Origin/fetch-site оболочки.
Android: `device_session {operation, parameters: JSON-string, credential}`.
Политика общая в Controller/DeviceHost, native старые start/stop также используют её.

- host.start `{address: IP:port}`, host.stop/status `{}` — явный приём,
  ответ listening/address/pin. Повтор start того же фактического адреса безопасен;
  другой адрес требует явного stop. Закрытие приёма не отзывает grant.
- offer.create `{}` сохраняет исходный код в OS-store до invitation.create.
  Повтор возвращает тот же активный код; новое приглашение создаётся только
  вместо подтверждённо терминального. offer.read `{}` не создаёт секрет.
  Ответ offer (null вне open), invitation — текущий snapshot или null.
- offer.* доступны также POST `/v1/host/offer.*` по TLS и CLI --endpoint с
  одобренными pin/key files. Для обычных invitation/pairing/access команд
  используйте прежний JSON envelope и `/v1/commands/*`.

Код показывается только для доверенной передачи, не входит в очередь/receipt
и не должен попадать в чат, argv, журналы или отчёт. Источники прав/статуса —
свежие Service snapshots. UI не меняет expected при refresh и не выдаёт права
по историческому receipt. Исходные отказы/неизвестные результаты сохраняют
намерение: повтор либо отдельное подтверждаемое «Убрать из списка». Это удаление
только записи доставки; результат операции и OS-сессия сохраняются. После
перечитывания новое действие требует нового явного подтверждения.

Сетевой timeout не удаляет session. После prepare с потерянным ответом status
показывает существующую попытку; submit продолжает её. Session cancel/unlink
хранят только subject/id/expected, сериализованы Web Locks; отсутствие безопасного
хранения запрещает отправку. Очистка — только прежним свежим TLS refresh.
ADR-0060/EV-103. QR/обмен и фоновая работа закрытого приложения отдельно.
