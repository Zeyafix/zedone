# Android-прототип P01.04

APK `app.zedone.prototype` содержит ту же Vue-форму, что и браузерный прототип.
Внутри Tauri она вызывает IPC-команду `echo`, в браузере — HTTP на loopback.
Оба адаптера вызывают `zedone-prototype-core::echo`; модели в Rust служат
источником OpenAPI, TypeScript и имени команды IPC. Проверка на устройстве
связывает генерируемое имя с зарегистрированной Rust-командой и её аргументом
`request`. Отдельного протокола моделей или ручных TypeScript-копий нет.

Echo работает без хранения. P01.05 добавляет отдельную
[форму SQLite](sqlite-prototype.md) с синтетическими пространствами A/B.
«Отправить» возвращает текст без хранения,
«Очистить» сбрасывает ввод/ответ/ошибку. Ошибка или ожидание более 5 секунд
не стирают ввод; повтор выполняется явно. Тайм-аут интерфейса не отменяет
операцию Rust; для echo это безопасно, потому что она ничего не изменяет.
«Назад» скрывает клавиатуру, затем возвращает на экран Android; открытие того же
процесса сохраняет форму. Force-stop начинает новую пустую сессию.
Это не обещание сохранности будущих прикладных данных или черновиков.

## Сборка

Подготовьте окружение по [Quick Start](quick-start-windows.md#3-подготовьте-android-p0003).
В дополнение к исходной среде P00 нужны `platforms/android-36@2.0.0` и
`build-tools/36.0.0@36.0.0`: библиотечный Android-модуль Tauri использует API 36,
а приложение — compileSdk/targetSdk 37. Устанавливайте их явно Android CLI
по общему списку `android.packages` в
[JSON версий](../../scripts/environment-versions.json), затем сверяйте ревизии.
Первая экспериментальная сборка выявила и автоматически загрузила эти два
пакета через Gradle; теперь автоматическая установка SDK отключена в проекте.

Общие Rust/Bun/JDK/NDK и версия Cargo Tauri CLI берутся из JSON. Tauri Rust/build
закреплены в Cargo manifest/lock, API JS — в package.json/bun.lock. Android-проект
создан CLI 2.12.4; Gradle 9.6.1 закреплён wrapper и официальным SHA-256,
AGP 9.3.1/Kotlin 2.2.10 — Gradle-конфигурацией. Переинициализация `android init`
не нужна: она может перезаписать проверенные настройки.

Из `prototypes/` после существующей установки зависимостей:

```powershell
$versions = Get-Content -Raw ../scripts/environment-versions.json | ConvertFrom-Json
cargo "+$($versions.rust)" fetch --locked
if ($LASTEXITCODE -ne 0) { throw 'Dependency fetch failed.' }
bun install --frozen-lockfile
if ($LASTEXITCODE -ne 0) { throw 'JavaScript installation failed.' }
bun run check
if ($LASTEXITCODE -ne 0) { throw 'Common check failed.' }
bun run android:build
if ($LASTEXITCODE -ne 0) { throw 'x86_64 APK build failed.' }
bun run android:build aarch64
if ($LASTEXITCODE -ne 0) { throw 'arm64 APK build failed.' }
```

`JAVA_HOME`, `ANDROID_HOME`, `NDK_HOME` должны указывать на подготовленные
инструменты. Если процесс получил старые значения, перечитайте эти три User
переменные в Process, как описано в Quick Start. На Linux используйте
эквивалентные переменные и `chmod +x src-tauri/gen/android/gradlew` перед сборкой.
Runner проверяет Bun, CLI, JDK, NDK и SDK, разрешение Cargo с `--locked --offline`,
затем задаёт закреплённый Rust и `prototypes/target/` всем подпроцессам.
Rust берётся из кеша; при первой сборке Gradle загружает wrapper/Maven-зависимости.
Это зависимости сборки, работа установленной формы сети не требует.

Результаты разнесены по ABI, чтобы arm64-сборка не перезаписала APK эмулятора:

- `src-tauri/gen/android/app/build/outputs/apk/x86_64/debug/app-x86_64-debug.apk`;
- `src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk`.

Оба APK подписываются стандартным debug-ключом Android. Это не релизные сборки;
ключи, APK, JNI-ссылки, SDK-пути и генерируемые абсолютные Gradle-ссылки не входят
в Git. JNI entry point генерирует Tauri; собственного unsafe в ядре нет.
Исключение для генерируемого bootstrap ограничено отдельным модулем
адаптера с `deny(unsafe_code)`; [ADR-0024](../architecture/decisions/0024-android-prototype-entry-point.md).
HTTP-сервер внутри APK не запускается; внешний агентский доступ относится к P01.10.
CSP ограничивает содержимое и IPC, удалённые capabilities не выдаются.
Android backup отключён; SQLite содержит только изолированные синтетические данные.

## Проверка в Android 13

Запустите свой AOSP API 33 x86_64 AVD по Quick Start. Общая проверка должна
предварительно собрать host `serve-echo` и согласованные артефакты.
Из `prototypes/`:

```powershell
$env:ANDROID_SERIAL = 'emulator-5554'
bun run check:android
if ($LASTEXITCODE -ne 0) { throw 'Android check failed.' }
```

Runner требует явный serial эмулятора, проверяет API 33/отсутствие GMS,
устанавливает только prototype APK и работает с настоящим WebView/Rust.
Проверяются равенство IPC/HTTP, Unicode/пустой текст/буквальный HTML,
форма при заблокированной через CDP сети, неверные параметры/неизвестная команда,
очистка, внедрённые в тестовый Android bridge ошибка и потеря ответа, повтор,
сенсорные нажатия, русский ввод через CDP Input.insertText, открытие штатной
клавиатуры, системная кнопка «Назад» и force-stop/повторное открытие.
Составной ввод конкретной русской IME и физическая клавиатура этим не проверяются.
WebView 101 не поддерживает desktop context/download API и новые проверки
видимости Playwright 1.64: подключение использует `noDefaults`, нажатия — CDP
touch events, утверждения — DOM. Приложение и WebView ради драйвера не патчатся.

После прогона закрываются только тестовое соединение CDP, его adb forward,
host echo и prototype-процесс. APK остаётся установленным для ручной проверки.
Для удаления: `adb -s emulator-5554 uninstall app.zedone.prototype`.
Свой AVD остановите штатной командой из Quick Start; сервер adb общий и не выключается.
Сборки удаляемы только из проверенных каталогов `prototypes/target/` и
`src-tauri/gen/android/**/build/`; исходники wrapper/Android-проекта сохраняйте.

[Android workflow](../../.github/workflows/android-environment.yml) готовит SDK,
собирает оба ABI и выполняет этот же тест на принадлежащем проверке AVD до его
очистки. Аргументы `check-android-environment.sh` — необязательная команда проверки
на уже проверенном эмуляторе, без eval. Actions P01.04 пройдены на Windows/Linux,
включая оба APK и Android-сценарии; PR #11 интегрирован.
Результаты и ограничения: [EV-016](../agent/verification/EV-016-android-ipc.md).
Проверка хранения P01.05 описана в [отдельной инструкции](sqlite-prototype.md).
Проверка на Galaxy A32, HTTP внутри APK, жесты доски и итоговая
приёмка T02/T04 относятся к следующим шагам.
