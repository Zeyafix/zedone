# Проверка служебного восстановления

Договор — [ADR-0084](../architecture/decisions/0084-emergency-database-recovery.md),
матрица результата — [EV-164](../agent/verification/EV-164-emergency-database-recovery.md).

`storage::emergency` владеет внешним состоянием, блокировками, полномочиями,
staging и файловым протоколом замены. `adapters::maintenance` останавливает
контроллеры обмена/worker, выполняет тот же контракт для HTTP/CLI/IPC и явно
открывает новый Service. Shell обновляет контроллеры по поколению Service,
чтобы не возобновлять старый worker или namespace секретов. Клиент использует
тот же Rust/OpenAPI контракт и долговечные намерения CommandClient.

Адресные проверки:

```text
cargo test -p zedone-storage emergency --locked --offline
cargo test -p zedone-storage recovered_backups --locked --offline
cargo test -p zedone-adapters --test maintenance --locked --offline
bun scripts/check-ui-maintenance.mjs
```

Последний сценарий требует собранный `maintenance-fixture` из manifest
`ui/src-tauri/Cargo.toml`; штатный `bun scripts/check-ui.mjs --main` собирает
его и запускает desktop/touch восстановление. Стенд использует временную
синтетическую базу, полную копию и HTTP/общий UI. Он проверяет отмену,
подтверждение, потерянный ответ с точным повтором, rollback, новый запуск,
выдачу/отзыв отдельного доступа и сохранность исходной полной копии.

`maintenance-fixture --prepare <directory>` создаёт изолированный повреждённый
набор с полной копией для APK/Device. Подставлять его в рабочие данные
разработчика нельзя. Сначала сохраняются APK/приватные файлы; после проверки
возвращаются исходные байты и удаляются только собственные тестовые артефакты.
Выбор папки в APK проверяется через системный SAF, ввод — через Device.

Полные gates: `check-product.mjs --host-tests`, `--host-build`, `--android`,
`check-ui.mjs --main` и `build-ui-android.mjs x86_64 --pairing-acceptance`.
Native Android tests запускаются штатным CLI на устройстве, открытом Device.
Они не заменяют проверку пользовательского пути в APK. Сборка arm64 и
process-exit не означают физический runtime или аппаратный power-loss.

Неизвестный результат нельзя исправлять новым ID. Fault/crash tests обязаны
сохранять исходный отказ, проверять closed gate до явного продолжения и отсутствие
повторной замены после receipt. Не ослаблять права, версии, бюджеты или assertions
ради успешного повтора. Не выдавать raw-комплект за исправную копию.
