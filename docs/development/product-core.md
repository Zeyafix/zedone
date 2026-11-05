# Разработка продуктового Rust-ядра

Архитектура — [границы ядра](../architecture/core-boundaries.md).
Продукт находится в корневом Cargo workspace; `prototypes/` проверяется отдельно.
Версии Rust/Bun берутся из `scripts/environment-versions.json`, зависимости —
из корневого Cargo.lock. Новых инструментов относительно подготовленной среды нет.

Из корня репозитория:

```powershell
$productVersions = Get-Content -Raw scripts/environment-versions.json | ConvertFrom-Json
cargo "+$($productVersions.rust)" fetch --locked
bun scripts/check-product.mjs
```

Команда выполняет fmt, Clippy всех targets/features с запретом предупреждений,
тесты, release-сборку всех targets/features, проверку границы зависимостей и
наследования `forbid(unsafe_code)`, негативную компиляцию каждого Rust-target
в отдельной временной копии и побайтовую сверку OpenAPI. Собственные исходники
не изменяются; временная копия удаляется даже при отказе. Запрет относится к
собственному продукту, не распространяется на исходники зависимостей.

После намеренного изменения Rust-контракта:

```powershell
bun scripts/check-product.mjs --write-openapi
bun scripts/check-product.mjs
```

`api/openapi.json` не редактируется вручную. Ошибка компиляции или проверки
не перезаписывает артефакт; явная генерация восстанавливает удалённый файл.
Сейчас в нём только схемы ошибок, `paths` пуст. Это не сервер и не агентский
интерфейс. Генерация TypeScript и подключение HTTP/CLI/IPC выполняются P03.05.
Для отдельной сверки контракта без общего прогона доступен `--check-openapi`.

## Android

С подготовленными `ANDROID_HOME`, закреплённым `NDK_HOME` и явно выбранным
тестовым AOSP API33/x86_64-эмулятором:

```powershell
$env:ANDROID_SERIAL = 'emulator-5554'
bun scripts/check-product.mjs --android
```

Команда проверяет свойства эмулятора и NDK, собирает release всех targets/features
для x86_64 и arm64, проверяет отказ unsafe каждого target для обеих ABI,
запускает те же Rust-тесты нативными x86_64-процессами и сравнивает OpenAPI
нативного генератора с host. Созданный каталог `/data/local/tmp/zedone-product-*`
удаляется в finally. Физические устройства не принимаются этим драйвером;
arm64-сборка не является нативной проверкой arm64.

Драйвер использует существующий явно выбранный эмулятор, не запускает и не
останавливает его. Жизненным циклом собственного read-only AVD владеет вызывающий
локальный прогон или существующий Android CI. APK/WebView в этом срезе нет.

Product-check включён в существующие Windows/Linux jobs и Android emulator
workflow. Изменения Cargo/crates/api/драйвера запускают соответствующие checks;
обе Cargo-рабочие области кэшируются независимо. До публикации PR факт успешного
CI не заявляется. Логи и данные конкретной машины — только `docs/reports/p03-01/`.
