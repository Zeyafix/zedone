# Запрос устройства: доступный host-срез

P06.01.2.3/.4 предоставляют host API, формы и внешний listener не включены.
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
session.credential() для прежнего API. OS pending slot в этот момент сохранять:
он содержит действующий ключ. Полного Rust→Kotlin→TLS процесса и форм ещё нет.

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
