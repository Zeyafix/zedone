# Очистка корзины и пользовательской истории через общий API

Источник контракта — Rust → api/openapi.json → TypeScript/runtime validator.
Все команды доступны через прежние Service/HTTP/CLI/IPC и требуют точного
Read/ReadWrite на space. Администрирование не заменяет права пространства.
Не писать SQL и не вызывать доверенные storage callbacks вместо общего API.

| Операция | Параметры | Доступ | Результат |
| --- | --- | --- | --- |
| trash.inspect | space, selection | Read | TrashPreview |
| trash.purge | space, selection, expected | ReadWrite | Подтверждённый удалённый TrashPreview |
| history.inspect | space, card (null — всё пространство) | Read | HistoryPreview |
| history.clear | space, card, expected | ReadWrite | Подтверждённый очищаемый HistoryPreview |

selection — строгий tagged объект: {"kind":"all"}, {"kind":"space"} или
{"kind":"objects","columns":[ID],"cards":[ID]}. В objects хотя бы один ID;
повторы одного ID в одном списке запрещены. Вложенные карточки колонки
добавляет ядро; отдельное указание той же карточки не расширяет область.
ID и space имеют прежний формат ненулевых lowercase hex из 32 символов.
Нельзя очищать активный выбранный объект. Space требует удалённого пространства.
All включает само пространство, если оно удалено; в этом случае проверяется
всё его содержимое, активные новые потомки блокируют очистку.

TrashPreview содержит selection, includes_space, columns, cards, conflicts и
version. Покажите состав и явное предупреждение о потере обычного восстановления;
только после явного решения человека/агента создайте новое намерение purge с
тем же selection и expected=preview.version. Если conflicts непуст, сначала
разрешить конфликты и получить новый preview. Для выбранной колонки состав
включает и прежнюю корзину, но не карточки, восстановленные в другую колонку.

HistoryPreview содержит entries (card, version с парой полей, retained),
text_bytes, removable_bytes и version. Байты относятся только к UTF-8 текстам,
не ко всему SQLite/Automerge. Retained-версии не удаляются: история спорной
карточки или карточки под спорным контейнером консервативно сохраняется целиком.
Card=null охватывает все карточки пространства, включая корзину. После явного
решения создайте новое намерение clear с прежней областью и expected.
Очистка затрагивает только известные ключи истории, не текущие поля, будущую
pending series или технические changes/peer state. Пустая история допустима.

Пример подготовки после прочитанного и явно подтверждённого состава:

```ts
const pending = await prepare(subject, "trash.purge", {
  space, selection: preview.selection, expected: preview.version,
}, store);
await client.deliver(pending);
```

Сохранять намерение до отправки. При stale_state перечитать и получить новое
явное действие, не подставлять свежий expected в старое автоматически. Conflict
ничего не очищает. При outcome_unknown повторить весь исходный запрос с тем же
ID/условием/областью. Replayed=true возвращает исходный состав и не является
актуальным view; последующее чтение требует нового ID. Права/отзыв/срок
проверяются и перед replay. Очистка не удаляет receipts, поэтому старый запрос
не повторяет действие после нового редактирования или восстановления.

Причинные tombstones/техническая история сохраняются; старые ID не
переиспользуются. Очистка не удаляет резервные копии, не обнуляет файл базы и
не обещает физически стереть текст. UI P05, продуктовый обмен и поздние
конфликты очищенных объектов P06 не приняты этим шагом.

Основание: [ADR-0051](../architecture/decisions/0051-trash-and-history-cleanup.md),
проверки: [EV-075](verification/EV-075-trash-history-complete.md).
