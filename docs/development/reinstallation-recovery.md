# Восстановление после переустановки

Общий handler находится в core/recovery/missing, хранение — storage/recovery/missing.
HTTP/CLI/IPC используют один Service и Rust-контракт. Полномочия, транзакция,
новый actor и сохранение графа описаны в [ADR-0082](../architecture/decisions/0082-reinstallation-recovery.md).
Схема SQLite, контейнер и существующее восстановление не меняются.

Для проверки используйте штатные check-product --host-tests/--host-build/--android,
check-ui --main и целевой scripts/check-ui-reinstallation.mjs после сборки
shell-fixture/zedone и ui/dist. Целевые storage tests recovery::missing проверяют
полную модель/orphans/изоляцию, появление active/trashed/purged/empty/unreadable
документа, ошибку SQL, повреждённый архив и все транзакционные границы restart.
Adapter tests commands::reinstallation проверяют HTTP/CLI/IPC/replay/ротацию прав.
Матрица snapshot_recovery проверяет новую установку, pending/confirm и явное
resume с сохранением независимых изменений второго хранилища.

Browser сценарий использует два независимых штатных fixture hosts; он проверяет
выбор/отмену/повреждение файла, подтверждение, гонку появления ID, потерю ответа,
restart/replay, исходные ID/изоляцию и pause/resume desktop/touch. Он не подменяет
APK/Device и системный SAF. Локальные артефакты, ключи, исходные APK и снимки
установки сохраняются только в ignored reports. До замены APK сохранить исходное
состояние и после проверки восстановить его с проверкой контрольных сумм.

Текущие результаты и непроверенные ограничения находятся в
[EV-162](../agent/verification/EV-162-reinstallation-recovery.md).


Android-test runner поддерживает `backup_folder_release` с точным tree ID
собственной папки `primary:Download/ZeDone-P0706-<32 hex>`. Он только отзывает
её persistable permission и проверяет отсутствие разрешения; чужие деревья
отклоняются. Этот режим нужен для обратимости Device-проверки и не попадает
в поставляемый APK. Общий `backup_folder_probe` сохраняет прежнее поведение.
