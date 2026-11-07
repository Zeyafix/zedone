# Разработка общей UI-оболочки

P05.02 расширяет оболочку жизненным циклом пространств/колонок;
[EV-066](../agent/verification/EV-066-space-column-ui.md).
`SpacesView.vue` хранит только текущую проекцию прочитанных данных и ввод формы.
`workspace-api.ts` использует прежние prepare/CommandClient и operation-specific
Rust-derived validator. HTTP и IPC выбираются общим `connection.ts`, ключ
перечитывается из локального хоста перед каждой доставкой. SQL/API/зависимости
не меняются; проверку прав, актуальности, конфликтов и содержимого выполняет core.

Открытие формы существующего объекта читает свежий snapshot и фиксирует его
исходную версию. Обновление списка её не подменяет. Успех доставки вызывает
новое чтение, а не установку snapshot из старого receipt. Поколение чтения
отбрасывает поздние ответы старого маршрута/сеанса, чтение ограничено 10 секундами.
Изменения сохраняют исходное намерение в `zedone.pending.v1.shell.*` до отправки;
это очередь доставки с введёнными параметрами, без ключа/прав/копии списка.
Успешное подтверждение удаляет намерение. После перезапуска нет автоматической
повторной отправки: пользователь явно проверяет исходный результат. Неизвестный
исход, отказ и ошибка persistence не показываются как успешное изменение.
Удаление записи из очереди требует отдельного подтверждения с предупреждением,
что принятая команда не отменена. Ошибка localStorage блокирует новые изменения,
но чтение и оформление доступны. Маршрут доски хранится только во fragment URL.

`ui:check` теперь включает `check-ui-workspace.mjs`: реальный HTTP/Service,
жизненный цикл обоих контейнеров, stale/сохранение ввода, потерю уже принятого
ответа и перезапуск с прежними байтами, настоящий отказ ключу и запрет отправки
при ошибке сохранения намерения. Исходные shell-проверки сохранены.
Android shell job дополнительно собирает test-only host `shell-fixture` и
на свежем CI AVD проверяет общий UI через IPC с синтетической установкой;
намерения/ключ/база удаляются в finally. CI использует прежний native touch
драйвер с видимостью, ожиданием доступности, размером и hit-test. Его локальный
desktop proof не является Android-приёмкой. Перед проверкой Back
формы CI отдельно скрывает видимую нативную IME и проверяет, что форма осталась
открытой. Видимость требует обоих mInputShown/mIsInputViewShown: активный метод
ввода может не иметь представления, а сервис может сохранять представление
после скрытия окна. Все прежние assertions и таймауты сохранены. Локальный
Android UI выполняется
только через Device, с сохранением и восстановлением исходного sandbox.

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
CDP-подключение к WebView использует `noDefaults: true`: WebView не реализует
desktop browser context/download management, поэтому стандартная настройка
Playwright `Browser.setDownloadBehavior` даёт protocol error. Режим сохраняет
реальные настройки WebView, включая системное оформление; UI assertions
остаются прежними. См. [Playwright connectOverCDP](https://playwright.dev/docs/api/class-browsertype#browser-type-connect-over-cdp-option-no-defaults).
