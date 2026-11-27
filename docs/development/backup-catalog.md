# Каталог и перенос папки копий

`core::backup_catalog` определяет команды и свежую авторизацию,
`storage::backup_catalog` — файловое обнаружение, локальные capabilities и
постоянные намерения переноса/удаления. `backup_io::TreeBackend` подключает
Android SAF к тем же операциям. Windows/browser и Android/IPC используют
один Service; только локальный выбор папки является платформенным портом.

[ADR-0081](../architecture/decisions/0081-backup-catalog-and-relocation.md)
описывает границы полномочий, порядок commit/cleanup, legacy форматы,
бюджеты и ограничения durability. Общий runtime layout v1 остаётся прежним;
служебные записи используют зарезервированные namespaces существующей таблицы.

Ежедневные и страховочные снимки используют общий выбранный root. Файлы
предмиграционного комплекта копируются потоково и целиком, без преобразования
данных. Прежний низкоуровневый migration runner получает явные MigrationOptions;
его отдельные служебные полномочия/аварийный UI остаются P07.09.

Проверки: `bun scripts/check-product.mjs --host-tests`, `--host-build`,
`--android`; `bun scripts/check-ui.mjs --main`. Целевой браузерный сценарий
`scripts/check-ui-backup-catalog.mjs` входит в main и выполняется desktop/touch.
UI main сам собирает CLI `zedone` для сценария выдачи локального handle;
предварительный запуск product gate или наличие старого executable не нужны.
Тесты storage включают пустую новую базу со старой папкой, повреждённую
коллизию, прерванную запись/продолжение, реальные process exit до/после commit,
restart/возврат, отказ cleanup, целый migration package и отдельное удаление.
Adapter tests проверяют HTTP/IPC/реальный CLI, права, stale, receipt и копии
после настоящей очистки пространства. Android UI проверяется через Device/SAF.

Machine-specific логи/ключи/пути/APK/снимки экрана остаются в ignored reports.
Результаты и ограничения — [EV-161](../agent/verification/EV-161-backup-catalog.md).
