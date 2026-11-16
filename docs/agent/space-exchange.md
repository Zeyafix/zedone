# Обмен пространства через общий API

Права/модель — [ADR-0064](../architecture/decisions/0064-space-exchange.md),
проверки — [EV-109](verification/EV-109-space-exchange.md).

Через обычный Service/HTTP/IPC/CLI доступны Rust-derived операции:

- `sync.inspect {space,pairing}` читает selected/paused и две исходные версии.
- `sync.select {space,pairing,selected,expected}` меняет выбор с
  selection_version. `false` сохраняет готовое пространство и progress,
  удаляя только неполную staging-загрузку этой пары.
- `sync.pause {space,pairing,paused,expected}` использует pause_version и
  меняет общую паузу пространства. Pairing здесь адресует общий снимок,
  а пауза относится ко всем устройствам. Выбор не снимает её.

Это обычные команды с cryptographic id и immutable parameters/receipt.
При unknown повторять исходную команду, при stale перечитать и ждать нового
явного намерения. После отзыва права receipt недоступен по прежнему ключу.
Требуются ManageDevices и изменение существующего локального пространства;
для ещё не полученного ID сохраняется только намерение импорта.

Существующий DeviceControl предоставляет одинаковые `exchange.catalog {}`
и `exchange.run {space}` через `/v1/host/...`, CLI `--endpoint` и Android IPC.
Host сам читает сохранённый адрес/pin/key из OS store. Клиенту не нужно
извлекать ключ устройства или писать в SQLite. Catalog возвращает только
разрешённые подтверждённому подключению пространства, read/write и выбор/паузу
другой стороны. Его pairing должен соответствовать сохранённому request ID.
Перед запуском нужен явный `sync.select` на обеих сторонах.

Run возвращает state: up_to_date, read_only, remote_paused, remote_unselected
или in_progress; отказы используют общие error codes. Unknown не является
подтверждением завершения: повторный run безопасно продолжает сохранённую
доставку changes, без повторного исполнения прикладных действий. Run не должен
создавать новый pairing и не снимает явную паузу. Ручной запуск делается на
стороне исходящего подключения; сервер независимо применяет свои ограничения.

`sync.catalog/pull/push` — транспортные операции для подтверждённых pairing
credentials, а не обход прикладных операций обычным агентским ключом.
Они используют общий core guard; непустой push требует read_write.
Прямое редактирование SQLite, перенос ACL/receipts и изменение чужого пространства
через raw changes не разрешаются. Протокол v1 и формат пространства v1 проверяются
явно; ограничения страниц и сохранённый frontier описаны в ADR.
