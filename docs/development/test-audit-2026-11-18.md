# Аудит избыточности тестов — 2026-11-18

Исходная ревизия: `693b1fbea654e5f22ee2cb9ff08af4b6cc08ccc9`.
Задача — анализ, без удаления тестов, изменения CI или продуктового поведения.
Исходные локальные записи EV-117 и очереди сохранены. Аудит не меняет статусы
продуктовой приёмки, открытых GAP и очередного шага P06.07.

Это исходный аудит состояния базы. Последующее удаление подтверждённых
повторов по поручению разработчика ведётся отдельно в
[CI.12/EV-118](../agent/verification/EV-118-test-deduplication.md).

## Вывод

Избыточность есть. Наиболее уверенные кандидаты — повторное выполнение
одинаковых host JavaScript-проверок внутри Android-матрицы и повторные
негативные проверки инфраструктурных скриптов. Среди продуктовых Rust-тестов
найден старый сценарий, основную матрицу которого уже покрывает более новый.
Оснований удалять целиком storage, adapters, UI или прототипные наборы нет.

Похожий сценарий считается избыточным только при совпадении проверяемой
гарантии, входов, наблюдаемого результата и среды. Проверка SQL rollback,
HTTP/IPC/CLI-паритета, браузерного диалога и Android Keystore даёт разные
гарантии, даже если везде создаётся одна и та же карточка.

## Карта всех штатных запусков

Источник маршрутизации — `.github/workflows/`, корневой и прототипный
`package.json`, вызываемые ими драйверы. Прямой `cargo test` из корня
не включает отдельные workspaces `prototypes` и `ui/src-tauri`.

| Набор | Где запускается | Реальное содержание |
| --- | --- | --- |
| Product | Windows и Linux, `check-product.mjs` | Comment-policy tests; fmt/Clippy; все Rust workspace tests; release all-targets; отрицательные unsafe/comment probes; точность OpenAPI/TS/validator; положительные и отрицательные TS-проверки; client delivery tests |
| Prototype native | Windows и Linux, `check --host-native` | Проверки маршрутизации host/Android CI; fmt/Clippy; Rust tests прототипа; Automerge process/restart phases; генерация/сверка прототипного контракта и отрицательные type fixtures |
| Prototype UI | Windows и Linux, `check --host-ui` | Release build прототипа; Vue typecheck/Vite; HTTP/browser UI; восемь проверок инструментов Android-измерений |
| Shell | Windows и Linux, `ui:check` | QR scanner unit tests; Vue types/build; fmt/Clippy; три Rust integration tests оболочки; browser desktop/touch сценарии; двуххостовые pairing/exchange UI |
| Android product-native | Ubuntu host + AOSP API33 x86_64 | Повтор host-контракта/TS/client; x86_64 release/unsafe; native Rust tests; два явных paired native probes; сравнение контракта Android с host |
| Android product-arm64 | Ubuntu host + AOSP x86_64 | Повтор host-контракта/TS/client; arm64 release/unsafe; ARM-бинарники не исполняются |
| Android prototype-native | Ubuntu host + AOSP x86_64 | Automerge phases отдельно на host и Android; сборка probe обеих ABI; native tests с фильтром `automerge_probe::` |
| Android prototype-ui | Ubuntu host + AOSP x86_64 | Обе debug APK и test APK прототипа; WebView discovery test; PluginLifecycleTest/HttpLifecycleTest; IPC/HTTP/form/storage/board/Markdown/QR/scanner, SQL failure/restart |
| Android shell | Ubuntu host + AOSP x86_64 | Production debug APK; Keystore/PluginLifecycle instrumentation; общий доменный UI через IPC, touch/IME/Back, история/корзина/очистка/accessibility/Markdown |
| Android pairing | Ubuntu host + AOSP x86_64 | Production и acceptance APK; `--standard` и `--positions` с независимыми fixture/restore/cleanup; настоящий APK↔host TLS/API/CLI и обмен |
| Android shell-arm64 | Ubuntu host | Production arm64 APK и test APK/manifest; положительного runtime callback нет; общий отрицательный compiler preflight остаётся |
| Windows environment | Отдельный workflow по узкому paths-фильтру | Версии инструментов/VS/SDK; отдельный Hello World build/run/fmt/Clippy; очистка своей временной директории |

Для изменений, затрагивающих обе основные workflow, предусмотрены восемь host
и семь Android jobs, плюс три обязательных агрегата. Агрегаты не выполняют
продуктовые тесты: они требуют успеха всей соответствующей матрицы. Windows
environment — дополнительный условный workflow, а не часть этих 18 checks.
Feature push сам по себе не дублирует PR-набор: push trigger ограничен `main`.

Статическая инвентаризация объявлений Rust tests, без `target/` и generated:

- Product: core — 25, storage — 284, adapters — 79. В этих 388 объявлениях
  есть три ignored paired fixtures и три Windows-only tests. Это не число
  прошедших тестов данного аудита.
- Shell: три integration tests в `ui/src-tauri/tests/shell.rs`.
- Prototype core: 177, из них 156 относятся к `automerge_probe`; Android
  native runner исполняет именно этот фильтр. Остальные HTTP/storage/QR/echo
  tests проходят на host; поведение APK отдельно проверяется через WebView.
- Bun: 27 client tests, 13 scanner tests, 28 развёрнутых comment-policy tests.
  CI/sampling/browser `.mjs` содержат самостоятельные assert-сценарии и
  не учитываются числом `bun test`.

Storage покрывает транзакции/replay/clock/access, spaces/columns/cards/tags,
историю/порядок/поиск/корзину/очистку, snapshot/backup/publication/startup,
invitations/pairing/sync control/engine. Adapters добавляют transport parsing,
авторизацию/replay HTTP/IPC/настоящего CLI, TLS/secret stores и device lifecycle.
Прототипы дополнительно покрывают ещё не перенесённые продуктовые правила
конфликтов тегов, удаления, purge и восстановления snapshot.

Локальные дополнительные проверки: Opera через `UI_BROWSER_PATHS` и
`PROTOTYPE_BROWSER_PATH`, Device/agent-device для настоящего Android UI,
production release APK/manifest, парная Windows↔APK приёмка. `measure:*`
прототипа — отдельные performance/pixel/physical-device эксперименты,
включая `measure:automerge`; штатный CI не запускает их целиком. Восемь
`check-android-*` в host-ui проверяют сами инструменты измерений, а не
производительность приложения. Прямой pre-push hook проверяет допустимые
Git refs и тесты не запускает.

## Подтверждённые повторные запуски

### R1. Client delivery tests исполняются четыре раза

`scripts/check-product.mjs:31` запускает один и тот же абсолютный путь
`tests/commands.test.ts` через host Bun. Вызовы: Windows product, Linux
product, Android product-native и Android product-arm64. В двух последних
jobs тест работает на Ubuntu, не внутри Android/WebView.

Рекомендация: сохранить Windows/Linux coverage, убрать два повторных host
запуска из Android-срезов. Native Android tests, paired probes и APK/UI
при этом должны остаться обязательными. Нельзя считать удалённые host
запуски потерей проверки Android JS-движка: они его и сейчас не проверяют.

Универсальную локальную команду можно сохранить полной, а CI-срезам дать
явного владельца общих host checks. После изменения нужен guard полноты
нового распределения, а не ручной необязательный skip через переменную среды.

### R2. Host contract/TypeScript повторяются в обоих Android-срезах

В `scripts/check-product.mjs:114–166` перед каждым Android-срезом на host
снова экспортируется OpenAPI, генерируются и сравниваются schema/validator,
выполняются `tsc` и `check-type-errors.mjs`. Тот же Linux-host набор уже
выполняется в Linux product; наличие arm64 target не влияет на TS compiler.

Рекомендация: оставить полную проверку производных артефактов на host,
а Android product-native сохранить независимый экспорт настоящим Android
`export-contract` и его сравнение с авторитетным проверенным контрактом.
R2 не разрешает удалять проверку platform parity или доверять непроверенному
сгенерированному файлу. На самостоятельном Android workflow должно быть
явно обеспечено выполнение host contract check либо своя такая проверка.

### R3. Одни инфраструктурные negative controls повторяются в семи jobs

`.github/workflows/android-environment.yml:111` без условия запускает
`check-ci-tauri.sh`: fake Cargo, cache hit/miss, wrong version, exit 23,
cleanup. На строке 277 такой же общий шаг во всех семи jobs проверяет
неверный Bun и искусственный отказ rustc до создания AVD. Сами scripts,
Ubuntu runner и закреплённые inputs одинаковы; выбора suite в этих
негативных сценариях нет.

Рекомендация: выполнять эти controls один раз на Android workflow —
в отдельном обязательном job либо в существующем обязательном владельце.
Если появится отдельный job, aggregate должен явно зависеть и от него.
Это исключит по шесть повторов каждого набора. Проверку реально установленного
Tauri/SDK, ошибки конкретного build/runtime и очистку собственного AVD/APK
оставить в каждом использующем их job. Перенос negative controls не должен
превратить семь изолированных сред в один общий AVD.

### R4. В build-only product-arm64 поднимается x86_64 AVD

`check-product.mjs` вызывает `checkAndroid()` и для `--android-arm64`.
Вначале проверяются свойства x86_64 эмулятора, затем собирается arm64
и запускаются compile guards; ветка исполнения native tests пропускается.
Общий `check-android-environment.sh` при этом создаёт и загружает AVD,
запускает Hello World и затем вызывает arm64 callback.

Рекомендация: выделить build-only arm64 путь, оставить NDK/toolchain/version,
release/unsafe/comment guards, убрать из него boot x86_64 AVD и минимальный
x86_64 runtime probe. Настоящий Android runtime остаётся в product-native.
Семейство environment negative controls сохранить согласно R3. Та же
зависимость от общего negative preflight сейчас заставляет shell-arm64
готовить KVM/emulator libraries, хотя его основной результат — APK build.
Это избыточная среда проверки, а не основание удалять arm64 сборки.

## Избыточность отдельных сценариев

### R5. Старый offline Rust-тест перекрывается более поздней матрицей

Кандидат: `crates/adapters/tests/commands/offline.rs:105`,
`new_description_invalidates_every_old_card_intention_and_both_container_deletions`.

Преемник: `commands/acceptance.rs:135`,
`every_card_field_invalidates_every_mutation_at_all_entries_even_after_undo_and_restart`,
ветка `description` и helper `reject_old_changes` со строки 92.

Оба меняют описание, проверяют `stale_state` старых card mutations и
column/space trash через все три входа, отсутствие побочных изменений,
возврат к исходному тексту без оживления старого token и сохранность после
restart. Новая матрица дополнительно повторяет старые команды после
restart, охватывает остальные поля и больше вариантов mutations.

Старый тест также завершает свежую `card.set_completed` и явно сравнивает
историю до/после неё. Этот assert не следует молча потерять: при объединении
его нужно перенести в новую матрицу либо оставить компактный адресный тест.
В storage инвариант уже проверяет `cards/tests/completion.rs:22`, но уровень
adapter имеет самостоятельную ценность.

Рекомендация: объединить с переносом этой проверки и удалить старую длинную
матрицу. Это один кандидат на сокращение тела Rust-тестов с высокой
уверенностью, а не доказательство избыточности остальных offline tests.
`mixed_offline_lifecycle` проверяет последовательность разнородных действий,
`unavailable_host` — недоступность Service и восстановление через CLI;
оба сохранить.

### R6. Повторные реальные ожидания в client tests

`client/commands.ts:72–86` использует реальные задержки 250/1000ms и timeout
10 секунд. `tests/commands.test.ts:382` намеренно не возвращает первый ответ,
дожидаясь настоящего production timeout. При выборочном запуске аудитом
этот тест занял около 10 секунд; многие operation-specific тесты повторяют
те же ожидания, проверяя прежде всего сохранность payload/validator.

Это полезные тесты с избыточным ожиданием. Кандидат на оптимизацию —
управляемый scheduler/таймеры, проверяющий те же production значения,
порядок retry, abort и late replies без ожидания wall-clock. Не сокращать
production timeout ради ускорения тестов и не удалять проверки идентичности
ID, bytes, versions и запрета автоматического нового намерения. Изменение
таймеров требует отдельного проектирования; аудит его не реализует.

## Похожие проверки, которые следует сохранить

- **Storage и adapters.** Storage проверяет атомарность настоящих SQL/CRDT
  изменений, injected faults и crash/reopen. Adapters проверяют строгий
  decoding и одинаковые результаты HTTP/IPC/реального CLI. В частности,
  `tags/tests.rs:505` проверяет delete, а `tags/tests/preservation.rs:7` —
  create/rename/assign на тех же transaction checkpoints: это разные операции.
- **Отдельные и совместные конфликты позиций.**
  `check-ui-placement-conflicts.mjs` и `check-ui-column-positions.mjs`
  содержат stale submit после внешнего изменения, точные подписи вариантов
  и частные права действий. `check-ui-position-conflicts.mjs` проверяет оба
  порядка решений одновременных конфликтов карточки/колонки. Последний
  не заменяет первые два. Аналогично совместные field conflicts не заменяют
  все одиночные name/text negative cases.
- **Browser desktop/touch и Android IPC.** Разные input/focus/IME/Back,
  transport и движок. В `checkWorkspaceTouchHost` переиспользуется тот же
  Android touch-сценарий, но с HTTP и имитацией Back: это полезный быстрый
  контроль общего UI, а не полноценная Android-проверка.
- **Native TLS probes и paired APK.** Два ignored probes в product-native
  используют native process и fixture secret store; APK-набор проверяет
  Kotlin bridge/настоящий Keystore и lifecycle. Стандартный `cargo test`
  ignored probes не исполняет. Третий ignored fixture запускается парным
  acceptance runner. Их нельзя удалять как «неиспользуемые тесты».
- **Crash child tests и внешние process phases.** Child-функции нужны
  родительским тестам. Выполнение нескольких фаз в одном Rust process
  не доказывает переход между отдельными процессами штатного probe driver.
- **Windows, Linux и Android Rust.** Отличаются SQLite/filesystem/process
  behavior и OS-specific secret stores. arm64 build не заменяет x86_64
  execution; x86_64 execution не подтверждает physical arm64 runtime.
- **Одинаковые QR capacity tests в product/prototype.** Входы совпадают,
  но проверяются две отдельные реализации с разными DTO/error mapping.
  Для общего теста сначала требуется перенос на одну реализацию; простое
  удаление одного оставит вторую реализацию без этой гарантии.
- **Прототипные UI/Automerge suites.** Есть значительное тематическое
  пересечение с продуктом, но это отдельный код и ещё не полностью
  перенесённые P06/P07 гарантии. Прототипный App также использует общий
  product MarkdownView. Выводить всю группу из обязательного CI сейчас
  нельзя считать безопасным удалением дублей. Для каждого выносимого
  сценария нужна конкретная продуктовая замена и решение о поддержке
  прототипа; измерительные инструменты пока тоже используются отдельно.
- **Positive/negative TS checks, fmt/Clippy/test/release.** Отрицательные
  fixtures доказывают отказ запрещённых вызовов, build не исполняет тесты,
  Vite build не заменяет Vue typecheck. Общий client входит в несколько
  TS configs с разными окружениями и options; совпадение файлов не делает
  эти проверки эквивалентными.

## Приоритет дальнейшей работы

1. R3: единый обязательный владелец инфраструктурных negative controls,
   с сохранением реальных preflight/cleanup в каждой среде.
2. R4: build-only arm64 без x86_64 AVD; обе ABI/guards и runtime suite прежние.
3. R1/R2: явное распределение общих host checks, сохранив standalone Android
   workflow и device-export parity. R6 можно рассматривать отдельно.
4. R5: объединение Rust-сценариев с переносом оставшегося adapter assert.
5. Более крупное сокращение прототипов — только по матрице перенесённых
   требований, отдельно от перечисленных повторных запусков.

Перенос negative controls/общих проверок потребует обновить
`check-host-ci-suites.mjs` и `check-android-ci-suites.mjs`: сейчас они
намеренно закрепляют прежнее распределение, в том числе запуск negative
preflight во всех семи Android jobs. Не следует просто удалять мешающие
assertions guards; они должны проверять новое полное обязательное покрытие.
Исторические запреты сокращать coverage/таймауты не являются причиной
хранить повторное выполнение одинакового теста, но новое распределение
должно доказать сохранение его гарантии.

Не оценивалось, сколько минут выиграет CI. R1 выполняется параллельно
native work, поэтому исключение повторов может уменьшить runner work,
не изменив критический путь. AVD/build/setup и повторные lifecycle двух
paired fixtures тоже дают стоимость; объединять их ради скорости без
сохранения изоляции нельзя. Нужны измерения после конкретного изменения.

## Что проверено самим аудитом

- Прочитаны все workflow/entrypoints и маршрутизация вложенных suites,
  инвентаризированы Rust/Bun tests, изучены пересекающиеся сценарии и
  отдельные Android instrumentation/measurement drivers.
- `bun test ./tests/commands.test.ts ./tests/qr-scanner.test.ts
  ./tests/source-comments.test.mjs`: **68 passed, 0 failed, 397 assertions**.
- `bun prototypes/scripts/check-host-ci-suites.mjs` и
  `bun prototypes/scripts/check-android-ci-suites.mjs`: **exit 0**.
- Первая попытка объединённого Bun-запуска с относительными фильтрами без
  `./` остановлена аудитом после отсутствия продвижения до первого файла;
  результатом passed не считается. С явными `./` тот же набор завершился.
  Причина задержки discovery не установлена; это не новый продуктовый баг.
- Полные Rust/browser/APK/Device suites и новый CI не запускались: аудит
  не меняет исполняемый код и не является новой платформенной приёмкой.
- Пять правил проекта: обратимость и общий API не изменены; документация
  дополнена этим отчётом; platform parity не ослаблен, новая Device-приёмка
  не заявляется; authoritative данные и контракт остались прежними.

Следующий конкретный шаг — реализация R3 с guard нового распределения и
проверкой строгого aggregate. До неё конфигурация и все тесты остаются прежними.
