# Разработка общей UI-оболочки

P05.03 завершает локальный срез карточки: `CardEditor.vue` управляет
просмотром/редактором, begin/autosave/finish и выполнением через прежний API.
`editor-drafts.ts` хранит только ввод/исходное предусловие с отдельным ID
и lifetime Web Lock. Параметры доставки не заменяются новым вводом; после
подтверждения сравниваются свежая version и receipt. Отличие останавливает
автозапись до нового явного решения. `mutationCredential` ограничивает
получение доступа 10 секундами до подготовки команды. Подробности —
[ADR-0052](../architecture/decisions/0052-card-editor-drafts-and-delivery.md),
проверки и границы — [EV-078](../agent/verification/EV-078-card-editor.md).

Debounce 500 мс/max wait 2 с применяется только при свободной очереди,
доступном ядре, непустом Unicode-заголовке и вне IME composition. Новый input
синхронно сохраняется отдельно; поле остаётся доступным во время доставки.
Выход из диалога/маршрута проходит через guard с фокусом предупреждения;
на время завершения закрытия ввод блокируется. visibility/pagehide пытаются
flush/finish, но не обещают асинхронный commit при остановке процесса.
После restart сначала разрешается прежнее намерение, затем новая сессия;
старый begin receipt не возобновляет завершённую сессию.

`check-ui-editor.mjs` добавлен в штатный `ui:check` с настоящими Service/history:
парные версии/30 с, непрерывный ввод и IME, ввод во время запроса,
guard/recovery, два контекста, потеря ответа/restart/replay и поздний агент,
stale выполнения/обратной отметки, доступ/persistence/deadline. Штатный Android
CI touch driver дополнен редактором/историей/выполнением/черновиком; его
desktop proof не заменяет локальный Device или новый Android CI.

P05.03.1 добавляет read/list/create/trash/restore карточки через прежний
`workspace-api.ts`. `SpacesView.vue` читает список карточек одним запросом
на пространство вместе с прежними тремя чтениями, проверяя общее поколение.
`CardsList.vue` отображает проекцию в порядке API и передаёт явные действия.
Корзина карточек отделена от корзины колонок; неоднозначное положение или
отсутствие активной колонки не скрывают карточку из просмотра.
Описание выводится Vue-интерполяцией с переносами, без `v-html`/загрузок.
Просмотр и подготовка корзины/восстановления читают свежий `card.read`;
старый expected никогда не заменяется автоматически. После подтверждённого
повтора перечитываются текущие данные, а не snapshot из старого receipt.
Редактор/сессии/автосохранение подключены P05.03.2; Rust/API/схема прежние.

`check-ui-cards.mjs` включён в штатный `ui:check`: настоящий HTTP/Service,
пара заголовок/описание, порядок/ID после восстановления/reload, буквальная
разметка, пустой заголовок/отказ доступа/ошибка localStorage, stale удаления,
потерянные уже принятые ответы и исходный replay после reload, сохранение
поздней правки и восстановления агента. Android shell CI дополнен теми же
созданием/просмотром/корзиной/восстановлением и отказом пустому заголовку
через настоящий IPC/native touch. Локальная проверка Android — только Device.
Текущие доказательства и ограничения — [EV-077](../agent/verification/EV-077-card-lifecycle-ui.md).

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

## Сборка для пробного запуска Windows

Пользовательский порядок запуска и первых действий — в
[Quick Start для Windows 11](../user/quick-start-windows.md).
Установщика пока нет; для пробного запуска нужны рабочая копия репозитория
и базовые инструменты Windows из
[подготовки окружения](quick-start-windows.md#1-установите-базовые-инструменты):
PowerShell, Rust/MSVC с Windows SDK и Bun. Android SDK для Windows-оболочки
не требуется. Версии берутся из `scripts/environment-versions.json`.

Из корня репозитория в PowerShell выполните команды по очереди.
После ошибки остановитесь и устраните её перед продолжением:

```powershell
bun install --frozen-lockfile --ignore-scripts
bun run ui:build
$versions = Get-Content -Raw scripts/environment-versions.json | ConvertFrom-Json
cargo "+$($versions.rust)" build --locked --manifest-path ui/src-tauri/Cargo.toml --target-dir target --bin serve-shell --example shell-fixture
```

Сначала собирается UI, затем Rust-host, который встраивает эти ресурсы.
Результат — `target/debug/serve-shell.exe` и тестовый
`target/debug/examples/shell-fixture.exe`. При изменении исходников повторите
сборку в том же порядке. Это сборка для знакомства, без заявления полной приёмки.
Служебный example не входит в приложение и не заменяет пользовательскую выдачу
прав; он допустим только для отдельной тестовой установки.

## Проверки оболочки

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
