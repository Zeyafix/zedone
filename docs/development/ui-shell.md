# Разработка общей UI-оболочки

`ui/src` — один Vue UI для Windows-браузера и Android Tauri. Границы —
[ADR-0042](../architecture/decisions/0042-product-ui-shell.md), результат —
[EV-064](../agent/verification/EV-064-ui-shell.md). Runtime не зависит от прототипов.

Из корня, с подготовленными инструментами `scripts/environment-versions.json`:

```powershell
bun install --frozen-lockfile --ignore-scripts
cargo +1.99.1 fetch --locked --manifest-path ui/src-tauri/Cargo.toml
bun run ui:check
```

`ui:check` выполняет vue-tsc под Node (база CI 24.22.1), Vite под Bun,
fmt/Clippy/tests shell workspace, сборку host/example и браузерную проверку.
Host-прогоны используют корневой `target/` вместе с product-check: Cargo
переиспользует неизменённые зависимости, сохраняя различия profiles/features/flags.
Android-сборка также использует корневой `target/`; отдельные ABI располагаются
в его подкаталогах, прототипы сохраняют свой независимый `prototypes/target/`.
`UI_NODE_PATH`/`UI_NODE_VERSION` — явный локальный override уже подготовленной
среды; результат отмечается отдельно от закреплённой базовой версии.
`UI_BROWSER_PATHS` — JSON-массив путей Chrome/Opera. Без него используется
подготовленный Chromium Playwright. Скрипт не устанавливает браузеры или Node.

Windows development host после `ui:check`:

```powershell
target/debug/serve-shell.exe --data-directory docs/reports/ui-local
```

Он выводит loopback URL и встраивает собранные ресурсы без Vite-сервера и внешней
сети. Новая база остаётся без прав. Example `shell-fixture` требует пустого
каталога и создаёт только синтетическую установку; он не входит в приложение.
Автотесты сами создают и удаляют этот каталог. На рабочих данных fixture запрещён.

`local-session.json` — внутренний порт ранее выданного секрета в приватном
каталоге установки, строгий JSON с `subject` (32 lowercase hex) и `key`
(64 lowercase hex). Его содержимое не выводить/не коммитить. Наличие файла
не доказывает прав; решение принимает общий guard. ОС-хранилище/доставка/начальное
подтверждение остаются P06. Сетевой bootstrap отсутствует. При неподдерживаемой
базе host сохраняет отказ штатного startup, не сбрасывает её.

Android при подготовленных JAVA_HOME/ANDROID_HOME/NDK_HOME:

```powershell
bun scripts/build-ui-android.mjs x86_64
bun scripts/build-ui-android.mjs aarch64
```

Скрипт проверяет Rust/Tauri CLI/JDK/NDK, не устанавливает SDK; `--release` после
архитектуры собирает неподписанный выпуск. В текущем срезе проверены debug APK.
Готовый manifest проверяется на идентичность и категории запуска.
JS/CSS target Chrome 101 задан явно: стандартная оптимизация media query в
новый range syntax ломала mobile layout на штатном AOSP WebView. Для оформления
выбранной темы используется производный Vue-класс, а не неподдерживаемый :has.
JNI-исключение ограничено `entry_point`; core/storage/adapters сохраняют forbid.
Feature `android-shell` включает optional build-dependency tauri-build только
для APK. Его явно задаёт build-ui-android.mjs; прямой Cargo Android build
без feature отклоняется, чтобы не получить оболочку без настроенного JNI.
Обычный browser host не собирает Android packaging tools. Версии и набор
runtime-проверок не меняются.
Scaffold основан на проверенном шаблоне Tauri, без экранов/плагинов прототипов.

Локальная UI-проверка — только Device/agent-device/device_screenshot.
`check-ui-android-ci.mjs` допускается только в настоящем CI, внутри штатного
emulator callback. Он не заменяет локальную приёмку. Успешный IPC сеанса и
IPC прикладной команды учитываются отдельно. Логи/снимки — в `docs/reports/`.
