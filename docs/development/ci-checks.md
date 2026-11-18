# Проверки PR и кэши сборки

## Отдельный search probe CI.18

Поиск по 1000 карточкам остаётся обязательным с прежним реальным временем
<30s и полным набором проверок результата. Обычный cargo test пропускает этот
ignored probe; полный `check-product.mjs`/`--host-tests` запускает его отдельно
после workspace tests и client peer. Native Android runtime после обычных
test binaries запускает тот же probe из zedone_storage. В обоих случаях —
ignored/exact/nocapture/test-threads=1, ровно один passed test обязателен.
Debug profile, graph и threshold прежние; других тестов в его процессе нет.
Missing binary, нулевой count, ошибка или сигнал не превращаются в SUCCESS.
Проверка и ограничения — [EV-124](../agent/verification/EV-124-ci-isolated-search-probe.md).

## Группировка по времени и очереди CI.17

Windows сохраняет шесть срезов, Android — двенадцать. Linux запускает три
полных pipeline: product, shell, prototype-ui. Последний включает весь
prototype, включая native checks. Одновременно выполняются не более двух
Linux jobs, длинные product/shell перечислены первыми. Это уменьшает
конкуренцию host с Android и повторную подготовку; квота аккаунта не меняется.

Полный `ui:check` выполняет parallel main, conflicts и pairing после одной
types/build/native подготовки. Explicit `--main`/`--conflicts` сохраняются.
Все уникальные проверки обоих срезов обязательны и в полной команде.
Новый набор — 24 checks с прежними тремя strict aggregate names. Порогов
и исключений continue-on-error нет; фактическое полное время зависит и от
очереди runners. Полнота и failures проверяются host routing guard.
Приёмка и замеры — [EV-123](../agent/verification/EV-123-ci-linux-capacity.md).

## Подготовка Android во время boot CI.16

`prototype-ui` запускает прежние host release endpoint и x86_64 APK с
instrumentation через `prepare-android-prototype-ci.sh` во время boot AVD.
`check-android-environment.sh --prepare command -- runtime command` доступен
только в CI. Он ждёт успешной подготовки и всех прежних проверок устройства
перед runtime. При ошибке или сигнале cleanup завершает собственные build и
emulator process groups с descendants; коды отказов не теряются. Остальные
suites и default без prepare сохраняют последовательный путь.

Синтетические lifecycle fixtures `bash scripts/check-android-preparation.sh`
выполняются в prototype-native до SDK install. Нужны Linux, jq и setsid;
настоящий SDK/AVD они не открывают. Проверяются оба порядка завершения,
build/boot/runtime failure, сигнал и cleanup. Версии, профили сборки,
instrumentation, negative controls и 27 обязательных checks сохраняются.
Результаты и непроверенный охват — [EV-122](../agent/verification/EV-122-ci-android-preparation.md).

## Параллельные main browser-сценарии CI.15

В существующих host shell jobs `check-ui-browser.mjs --main` запускает три
независимых slice: lifecycle/editor, desktop board/navigation и touch. Каждый получает
свой каталог fixture/SQLite/session, HTTP origin и browser/contexts. Бинарник
и собранные assets общие, только для чтения. Все 20 вызовов сценариев остаются
обязательными; их внутренние assertions и временные границы не сокращаются.
Снимки desktop/mobile принадлежат только board/navigation slice.

Dispatcher ждёт завершения и cleanup каждого slice, включая случай отказа
другого. Sync/async ошибки собираются в общий failure. Full/default и
`--conflicts` сохраняют последовательный путь. Матрица из 27 checks,
strict aggregates и budgets прежние. Host routing guard проверяет точное
покрытие main/full и ожидание всех peers при отказе. Tab-навигация остаётся
после desktop accessibility на той же page с прежней подготовкой состояния
и проверкой перехода на radio. Изменение не затрагивает Android runtime.

Локальные результаты и ограничения измерений — в
[EV-121](../agent/verification/EV-121-ci-parallel-browser.md).

## Подготовка инструментов CI.14

Матрица CI.13 (27 checks) и все сценарии/лимиты сохранены. Android SDK
восстанавливается только по ключу OS/arch + fingerprints versions/setup/verifier;
при miss исполняется прежняя полная установка с проверкой архивных SHA-256.
Общий `check-android-sdk.sh` на miss и hit проверяет SHA CLI, успешную
инициализацию, отдельное точное чтение версии и все package revisions.
Runtime preflight использует тот же verifier; Rust targets, KVM/AVD/probes,
builds и negative controls выполняются по-прежнему. Corrupt/missing SDK
означает failure, а не пропуск проверки. Один SDK writer — prototype-native
после всех его успешных checks. Кэшируется только SDK, без AVD/app data/keys.

Host browser cache раздельный для shell/prototype, OS/arch и фактически
установленной Playwright version. Writer каждой family один; scopes PR/main
сохраняют [ограничения GitHub](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#restrictions-for-accessing-a-cache).
После hit по-прежнему выполняется install --with-deps --only-shell chromium:
все CI launch используют headless, поэтому full Chromium не загружается.
OS dependencies не считаются кэшированными. Browser path — собственный каталог
RUNNER_TEMP; локальные команды без CI env сохраняют обычный путь Playwright.
Host Cargo fetch ограничен фактической host triple из pinned rustc; --locked,
all-targets/all-features checks и offline builds прежние. Node setup пропускается
только в product-build, где Node-команды не исполняются.

Windows matrix начинает с длинного shell slice, затем shell-conflicts;
это приоритет запуска при очереди, а не изменение обязательного состава.

Playwright [не рекомендует browser cache по умолчанию](https://playwright.dev/docs/ci#caching-browsers),
поскольку restore может стоить столько же, сколько download. Здесь конкретный
Windows download занимал около трёх минут; выигрыша заранее не обещаем.
SDK/browser keys не имеют restore-prefix fallback. Новые tool caches могут
сохраняться в PR merge-ref scope для измерения повторной подготовки; main
не читает PR caches, Rust/Tauri/Gradle writer policy не меняется. Размер новых
entries и eviction требуется измерить: исходно 19 entries/9.025 GiB, включая
старые ключи. Квота/платные настройки не меняются. При cache miss/corruption
запуск не выдаётся за warm benchmark. Cold/hit сравниваются на одном head,
со всеми attempts/очередями/runner work; [EV-120](../agent/verification/EV-120-ci-tool-caches.md).

## Текущее распределение CI.13

Чтобы уменьшить критический путь jobs >10m, независимые группы запускаются
на отдельных runners. Тела сценариев, assertions, профили, API и временные
пороги сохранены. Полный локальный запуск остаётся доступен без аргументов.

| Workflow | Suite | Команда / назначение |
| --- | --- | --- |
| Windows/Linux | product | `bun scripts/check-product.mjs --host-tests`: fmt/Clippy/Rust/client/contract/types |
| Windows/Linux | product-build | `bun scripts/check-product.mjs --host-build`: release all-targets/all-features, unsafe/comment rejection и recovery |
| Windows/Linux | shell | `bun run ui:check --main`: QR/types/build/native и основной desktop/touch UI |
| Windows/Linux | shell-conflicts | `bun run ui:check --conflicts`: все workspace/conflicts/positions и pairing browser сценарии |
| Android | product-native | `--android-runtime`: native Rust, release CLI/exporter, host/device contract и оба paired probes |
| Android | product-x86_64 | `--android-x86_64`: release all-targets/all-features и unsafe/comment guards без AVD |
| Android | prototype-ui / prototype-arm64 | Прежний x86_64 APK/WebView/instrumentation runtime / отдельная arm64 APK/instrumentation сборка |
| Android | pairing-fields / pairing-placements / pairing-positions-cross / pairing-positions-same | Самостоятельные text/fields, card/column placements, combined positions suites |

Host prototype-native/prototype-ui и Android product-arm64/prototype-native/
shell/shell-arm64 сохраняются. Всего 27 checks: 6 Windows + 6 Linux + 12 Android
и 3 aggregates. Каждый aggregate требует успеха всей своей матрицы;
`fail-fast: false`, без `continue-on-error`. Лимиты прежние: Linux 10m,
Windows 20m, Android 15m, aggregates 1m. Ручные полные команды, включая
`--android-native`, старый `pairing` dispatcher и `--standard`, совместимы.

Каждый pairing runner собирает обычный production APK, устанавливает его
в собственный AVD, затем собирает acceptance APK и запускает явный selector.
Четыре combined-position случая делятся по cross/same-column, в каждом
сохраняются оба порядка решений. Полный --positions остаётся совместимым.
При отказе host fixture выводятся только exit, классификация fixture_timeout/
unclassified и elapsed_ms; сырой stderr с возможными ключами не печатается.
240s без UI env, job/scenario budgets и failure assertions прежние.
Общие TLS/rights/replay/recovery/restart/reverse/SHA-256 restoration/OS cleanup
выполняются в каждом срезе с прежними бюджетами. Неиспользуемый browser
`shell-fixture` в pairing больше не собирается; shell сохраняет его.
Product runtime исполняет release CLI и exporter; полный x86_64 release
all-targets/unsafe — отдельный обязательный владелец. Прототипная arm64
сборка не требует AVD; проверка WebView остаётся в prototype-ui.

Новые host suites пишут каждый в свой main cache namespace; первый запуск
может получить seed прежнего владельца product/shell. Это не обмен артефактами
между непроверенными jobs и не основание пропустить проверку. Android Rust
cache сохраняет единственного writer product-native. Дополнительные runners
повторяют setup/build, поэтому уменьшение wall time не означает экономии
суммарных runner minutes. Новый GitHub runtime пока не измерен; результаты
предыдущего head не доказывают все jobs <10m. [EV-119](../agent/verification/EV-119-ci-critical-path.md).
Нижеследующие описания CI.01–CI.12 сохраняют историю прежнего распределения.

Host shell записывает ошибку общего touch-набора до закрытия его browser context:
`UI failure state` содержит label `host touch`, конкретный stage, состояние
этой страницы и до 30 метаданных чтений, включая tag.list. Запросы/headers,
ключи и сохранённые намерения не выводятся. Ошибка диагностики не заменяет
исходный error, повторов действий нет. Основная страница не используется
как состояние отказавшего touch окна. Постоянный host negative control
проверяет реальные разные страницы, исходный TimeoutError, ограничение
метаданных и отсутствие секрета в отчёте; [EV-094](../agent/verification/EV-094-host-touch-diagnostics.md).
GAP-028 отслеживает исходный tags refresh timeout: улучшение диагностики
само по себе не доказывает его причину или устранение.

CI.09 добавляет `readLifecycle` для той же touch-страницы: до 30 последних
чтений `/host/session` и space.list/read, column.list, card.list, tag.list.
Для каждого сохраняются локальный порядковый номер, фиксированный path без
query, started/headers/finished/failed, HTTP-код, длительности и результат
проверки тела API по производной схеме (pending/valid/invalid/unreadable).
Исходное тело/ошибка разбора не выводятся; новый observer не читает тело
host/session. Так видны также запросы без ответа или с незавершённым телом,
которых нет в прежнем списке готовых JSON-ответов. Время ожидания, одна
попытка, исходный TimeoutError и весь набор проверок прежние.
Постоянный control проверяет состояния/лимит/исключение секрета, настоящий
перехваченный pending→failed запрос и сохранение исходной ошибки при отказе
reporter; [EV-096](../agent/verification/EV-096-host-read-lifecycle.md).
Повторный trash timeout PR #91 связан с GAP-028 как наблюдение;
общая причина с первым отказом и устранение исходного сбоя не доказаны.

Android setup отдельно закрепляет исполняемый Android CLI: `android.cli`
задаёт версию, `android.linux_cli_sha256` — SHA-256 Linux x86_64 файла из
версионированного URL Google. После распаковки command-line tools workflow
заменяет их `android` launcher проверенным файлом до первой команды SDK.
Launcher из архива сам получает `latest`, поэтому одного SHA-256 архива
недостаточно для воспроизводимой версии CLI. Первый успешный запуск отдельно
распаковывает встроенные инструменты и выводит установочное сообщение; следующий
вызов должен вернуть только точную требуемую версию. Перед SDK install и при preflight
сохраняется точное сравнение версии; при отказе preflight показывает
ожидаемую/фактическую строку и найденную команду. Версии SDK/NDK/эмулятора
и прежние архивные хэши этим не меняются; [EV-088](../agent/verification/EV-088-android-cli-preflight.md).

Все три workflow запускаются для подходящих изменений в PR, при push в `main`
и вручную через **Actions → Run workflow** с выбором ветки. Отдельный push
feature-ветки не запускает второй набор jobs; до открытия PR доступны обычные
локальные команды и ручной запуск. `pull_request` с обычным checkout проверяет
[пробное слияние с базовой веткой](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request).
Существующие paths и отмена устаревшего запуска той же ветки/PR сохранены.

Полный `bun run check` выполняется на Windows/Linux; Android собирает APK
и instrumentation для x86_64 и arm64, проверяет host-пробник Automerge,
нативные процессы/тесты и APK/WebView/HTTP/lifecycle на AOSP API 33/x86_64.
Кэш не является результатом проверки: после восстановления выполняются
те же команды, проверки точных версий и контракта, `--locked`/`--offline`.
Человек и агент используют общий набор команд из [прототипов](prototypes.md)
и [Android CI](android-environment-ci.md).

Для продукта добавлен отдельный [product-check](product-core.md) из корня:
Windows/Linux выполняют `bun scripts/check-product.mjs`; существующий Android
emulator callback — ту же команду с `--android`. Проверяются контракт и отказ
unsafe во всех собственных Cargo-targets, обе Android ABI и нативные x86_64-тесты.
P03.02 добавляет те же тесты атомарного хранения и crash/reopen на обеих
платформах; bundled SQLite собирается NDK C-компилятором, тестовые базы
живут только в удаляемом sandbox. APK/WebView этим не заменяются.
Корневые Cargo/crates/api/драйвер добавлены в paths; root workspace → `target`
кэшируется рядом с независимым `prototypes` → `target`. Готовность прототипов
не подменяет эти проверки продукта.

## Что переиспользуется

P05.01 добавляет `ui/**` в paths. Shell — отдельный Cargo workspace, но его
штатные драйверы используют общий с ядром корневой `target/`.
Windows/Linux выполняют `bun run ui:check`: общий UI,
host framing/security и реальные команды через HTTP в Chromium. Android
собирает debug APK обеих ABI, а последний CI emulator callback проверяет
сеанс/навигацию/темы/Back в новом APK. Локально этот драйвер запрещён — требуется
Device. Готовность CI определять по финальному head; первые отменённые/упавшие
запуски и замена увеличения timeout оптимизацией описаны в EV-064.
Команды и ограничения — [ui-shell.md](ui-shell.md), EV-064.

Android выполняет семь независимых наборов параллельно на стандартных runners:
`product-native` (x86_64 release/guards и все native tests),
`product-arm64` (arm64 release/guards), `prototype-native` (host/Android Automerge),
`prototype-ui` (обе APK и
instrumentation прототипа, host HTTP endpoint и весь UI/lifecycle-набор),
`shell` (production x86_64 APK оболочки и её UI-набор),
`pairing` (production и acceptance x86_64 APK, полный парный TLS/API/CLI-набор),
`shell-arm64` (production arm64 APK без положительного emulator callback).
Каждый job владеет отдельным SDK; runtime suites также владеют отдельным AVD.
Все сохраняют лимит 15 минут. Общие installer/compiler-failure negative
controls выполняются один раз в обязательном `prototype-native`; реальные
tool checks и очистка своих runtime ресурсов остаются в каждом владельце.
`product-arm64` выполняет build/guards напрямую без x86_64 AVD; оба build-only
jobs также не готовят KVM/emulator libraries. Host client tests остаются в
Windows/Linux product; standalone Android host contract/TS — в product-native.
Этот же обязательный владелец controls запускает Android routing guard внутри
Android workflow, включая изменения только его YAML без запуска host workflow.
Результаты удаления повторов и границы проверки — [EV-118](../agent/verification/EV-118-test-deduplication.md).
`fail-fast: false` сохраняет диагностику остальных наборов при одном отказе.
Итоговый стабильный check `android` с `if: always()` требует успеха всей матрицы:
failure/cancelled/skipped не принимаются как успех. Прежний аргумент `native`
dispatcher сохраняет оба нативных набора последовательно. Без аргумента dispatcher
сохраняет все прежние команды и последовательный порядок; неизвестный набор
отклоняется. Цель CI.03 — не более 10 минут ожидания Android workflow —
достигнута в запуске `42264208567` head `84677ce`: 9:23 от старта
workflow до завершения (включая aggregate), все шесть PR checks SUCCESS.
Сумма Android job time — 23:47; подготовка трёх сред увеличила расход runner
времени. Все сценарии сохранены. Это измерение с cache hits, не гарантия
холодного или будущего времени; подробности и ограничения — EV-064/LOG-316.

Rust cache paths/namespace остаются прежними, единственный writer на main —
`product-native`; остальные jobs только читают, чтобы разные подмножества результатов
не конкурировали за один immutable cache key. Это сохраняет fallback прежнего
main-кэша; наличие shell build artifacts в кэше не гарантируется.
Разделение product/prototype native добавлено в P04.02 после двух таймаутов
совмещённого job на head aada64a: продуктовые тесты/ABI прошли, но прежний
прототипный набор не успел завершиться. Все пять исходных команд и их порядок
в полном dispatcher сохранены. Каждый native runner готовит только собственные
JS/Cargo зависимости, host Automerge проверяется в prototype-native.
Исторически объём SDK/проверки environment/negative-cleanup и лимиты сохранялись.
В CI.12 повторы общих controls убраны с сохранением одного обязательного владельца.
P04.06 запускал независимые клиентские delivery-тесты параллельно нативной
product-сборке в том же job: они не используют эмулятор или Cargo output.
Драйвер ждёт оба результата и сохраняет все отказы через AggregateError;
ранняя ошибка одного набора не оставляет другой без ожидания и cleanup.
С CI.12 эта параллельность нужна только host product: Android jobs не повторяют
те же host Bun tests, работающие вне Android.
Это убирает последовательное ожидание клиентских таймаутов из критического
пути; достижение прежнего лимита подтверждается только новым CI, не предполагается.
Дополнительный runner может увеличить суммарное машинное время; результат
по времени всего workflow определяется новым запуском, не предполагается заранее.
P04.10 разделяет продукт по ABI между двумя hosts: dispatcher `product-native`
вызывает `bun scripts/check-product.mjs --android-native`, `product-arm64` —
ту же команду с `--android-arm64`. Каждый проверяет выбранную release ABI и
семь unsafe guards; x86_64-срез также выполняет все 273 native tests и экспорт
контракта. Суммарно сохраняются обе ABI и все 14 guards; aggregate требует
успеха обоих срезов и остальных трёх наборов. Полный `--android` остаётся
последовательным; standalone/default/native dispatcher сохраняет прежние команды.
Маршрутизация всех наборов, старые вызовы и строгие отказы проверяются
`bun prototypes/scripts/check-android-ci-suites.mjs` в штатном host-check.
Совместное исполнение native tests и arm64-компиляции на одном host отменено:
в CI a344278 timing guard поиска превысил прежние 30 секунд (BUG-008).
Порог не увеличен. Разделение добавляет runner и его setup/SDK/AVD, увеличивая
суммарное машинное время. Лимит каждого job по-прежнему 10 минут; достижение
времени всего workflow подтверждается новым CI, не предполагается заранее.
Итоговый d3f98eb прошёл все восемь checks, включая оба product-среза и весь
прежний набор; BUG-008 resolved, PR #77/67c8e12, EV-076/LOG-376. Все пять
Android jobs завершились в прежнем лимите. Это наблюдение одного запуска
с кэшем, не гарантия будущего или холодного времени.

Сохраняются прежние host budgets: каждый Linux suite 10 минут, Windows suite 20.
Цель ожидания Windows ≤10 минут не заменяет бюджет холодной сборки.
Повышение Linux до 20 отклонено разработчиком. Shell не собирает Android-only
tauri-build при host-проверке. Отдельный shell runner каждой ОС использует
собственный кэш корневого target. Набор fmt/Clippy/tests/release/контракт/UI не сокращается.

### Параллельные host-проверки Windows и Linux

`Prototype foundation` выполняет по четыре jobs на отдельных `windows-2025`
и `ubuntu-24.04` runners с `fail-fast: false`. Обе ОС используют одинаковые
наборы; прежние команды сохраняются целиком:

| Suite | Команда | Подготовка |
| --- | --- | --- |
| `product` | `bun scripts/check-product.mjs` из корня | Корневые Cargo/JS-зависимости; браузер не требуется |
| `prototype-native` | `bun run check --host-native` из `prototypes/` | Cargo/JS прототипа; fmt/Clippy/tests, debug restart-пробник и генерируемые контракты; браузер не требуется |
| `prototype-ui` | `bun run check --host-ui` из `prototypes/` | Cargo/JS прототипа и Playwright Chromium с OS-зависимостями; прежняя release-сборка workspace, Vue/Vite/HTTP и Android driver guards |
| `shell` | `bun run ui:check` из корня | Cargo workspace `ui/src-tauri`, корневые JS и Playwright Chromium с OS-зависимостями |

Общий history-сценарий desktop/touch/native перед вводом ждёт подтверждённое
«Сохранено» после открытия редактора: начальные `begin_editing` и фокус должны
завершиться до сценария автосохранения. В host-сценариях настоящий ответ
autosave удерживается: кнопки неразрешённого результата отсутствуют, переход
к истории обязан ждать ответ, после его освобождения проверяется одна точная
новая версия/удаление черновика. Native проверяет тот же переход с реальным IPC/IME.
Ожидание подтверждения запроса ограничено 10s; ошибка доставки завершает
проверку. При cleanup удержанный обработчик завершается до снятия route,
чтобы не отправить тот же запрос повторно и не скрыть исходный отказ.
Успешное автосохранение не добавляет/удаляет временные recovery-кнопки в
редакторе; они показываются только при pending + blocked, когда исход требует
вмешательства. Это исключает уменьшение scrollHeight и сдвиг цели касания.
Failure trace сохраняет ограниченные события/геометрию без ввода/ключей.
Механизм промаха и локальное воспроизведение — [EV-089](../agent/verification/EV-089-history-transition-diagnostics.md).

Подготовка вынесена в локальный composite action
[prepare-host-check](../../.github/actions/prepare-host-check/action.yml).
Он читает закреплённые версии из `environment-versions.json`, проверяет имя
набора и ошибки CLI, устанавливает только нужные зависимости. Изменение
action входит в paths обоих событий. Режим `all` сохраняет для совместимости
последовательность product → prototype → shell и прежний Rust namespace/paths;
workflow обеих ОС использует четыре отдельных suite.
Браузер теперь готовится до первого набора; команды самих проверок не меняются.
Android workflow и его dispatcher не затронуты.

Прежние required checks `check (windows-2025)` и `check (ubuntu-24.04)`
сохранены как отдельные aggregates с `needs: windows-checks`/`linux-checks`
и `if: always()`. Результат всей матрицы соответствующей ОС должен быть ровно
`success`: `failure`, `cancelled`, `skipped` и отсутствие результата
не принимаются как успех. Aggregates работают на Ubuntu с лимитом 1 минута
и проверяют результат своих runners, сохраняя независимость двух ОС.

У suites каждой ОС отдельные Rust namespaces `prototype-host-product`,
`prototype-host-prototype-native`, `prototype-host-prototype-ui`,
`prototype-host-shell`. Каждый сохраняется только
своим успешным main job: разные подмножества не конкурируют за один immutable
key. Кэшируются registry/git и target выбранного workspace. Shell metadata
учитывает собственные `ui/src-tauri/Cargo.toml`/`Cargo.lock`, а физический
output остаётся общим корневым `target/`, как в штатном драйвере.
Перед первым restore нового namespace выбранной ОС выполняется metadata-only
lookup собственного архива. Если его нет, `actions/cache/restore` читает
прежний общий main-архив с namespace `prototype-host` до подготовки
`rust-cache`; затем обычный suite restore/save работает как прежде.
У lookup и fallback только restore, PR-запись кэша не включается. При наличии
собственного архива старый лишний раз не скачивается. Отсутствие обоих
архивов оставляет обычную холодную сборку без пропуска проверок.
Для старого архива передаются прежние четыре абсолютных пути, в прежнем
порядке: Cargo registry/git, корневой target, prototype target; Zstd и
archive version соответствующей платформы должны совпасть. Это чтение старых зависимостей,
а не перенос результата тестов: Cargo проверяет units, а все команды идут
с прежними флагами и версиями. Совпадение archive version проверено по API/
алгоритму toolkit для Windows; настоящий restore всех четырёх suites подтверждён CI `ed512db`.

Linux fallback использует префикс `Linux-x64`, Windows — прежний
`Windows_NT-x64`. На `f723d52` все четыре Linux suites реально восстановили
прежний main cache, PR save отключён; aggregate прошёл через 4:05 от старта
workflow. Все 17 Windows/Linux/Android checks SUCCESS, PR #82 интегрирован
в `1a71b72` (EV-082/LOG-453). Это один cached запуск; future main suite-cache
save/restore и полностью холодное время ещё не подтверждены.

Linux разделён в P05.05 после повторного исчерпания последовательного
10-минутного бюджета PR #82. На `dd84aac` attempt 3 остальные 12 checks
прошли, Linux был отменён во время shell UI-набора; GitHub явно указал
job timeout. Каждый новый Linux suite сохраняет лимит 10 минут, проверок
и сценарных timeout изменений нет. Новый полный PR-набор содержит 17 checks:
14 suites и три обязательных aggregates. Текущий head прошёл весь CI;
холодное время не принимается этим замером, подготовка отдельных runners может
увеличить суммарные runner-минуты. Доказательства и ограничения —
[EV-082](../agent/verification/EV-082-board-moves-ui.md).

Цель CI.04 — ≤10 минут **от старта workflow до завершения Windows aggregate**,
включая очередь runners, подготовку и post/cache steps. Короткая длительность
самого aggregate не является ускорением. Сумму job time считать отдельно:
повторная подготовка и восстановление кэшей могут увеличить runner-минуты.
Сравнивать cold/warm запуски одного SHA с одинаковым охватом; rerun
неуспешной части не заменяет измерение полного запуска.
Исходный PR #78: Windows job 19:29 при полном Rust cache hit, 19:32 от старта
workflow. На `ed512db` все 12 checks passed: Windows от старта workflow до
aggregate **6:32**, весь CI **9:31**, Linux job 9:23. Все четыре Windows jobs
восстановили прежний main cache; собственные suite-архивы ещё отсутствовали.
Цель достигнута в этом одном запуске с кэшем. Полностью холодное время и
будущие main suite-cache save/restore не подтверждены. Сумма четырёх Windows
jobs — 22:33, поэтому сокращение ожидания не означает сокращение машинного
времени. Предыдущие неуспешные/медленные замеры, локальные проверки и ограничения —
[EV-079](../agent/verification/EV-079-parallel-windows-checks.md).

Первоначальные три Windows suites оставляли холодный prototype слишком
долгим: на `addb9c4` он исполнялся более 14 минут до отмены новым head,
а HTTP/UI часть ещё не началась. Поэтому prototype разделён ещё раз.
`--host-native` выполняет debug tests/Automerge/контракты; `--host-ui`
владеет прежней полной release-сборкой и browser-набором. Default
`bun run check` сохраняет прежние команды и порядок плюс новый routing guard.
`check-host-ci-suites.mjs` исполняет настоящий драйвер на подставной границе
CLI/import/write: объединение двух срезов равно полному набору команд
ровно по одному разу; legacy modes, неверные аргументы и остановка при
ошибке/сигнале также проверяются. Guard входит в full/native host-check.
Без миграционного restore первый полный запуск четырёх suites занял 13:54
до aggregate; последующий failed запуск не принимается за успешный замер.

На диагностическом `654f6ce` последовательный Linux job также исчерпал
10 минут: сценарии редактора успели пройти, но финальное завершение уже
было отменено бюджетом. Лимит сохранён. Независимые 23 client delivery-теста
продукта теперь выполняются параллельно Rust-проверкам и на host, как прежде
на Android; это убирает около 25 секунд последовательного ожидания из
наблюдавшегося Linux запуска. Общий `checkWithClients` ждёт оба результата
через `Promise.allSettled` и сохраняет все ошибки в `AggregateError`.
Routing guard проверяет оба порядка завершения, одновременные отказы и
сигнал клиента: ранний отказ не оставляет вторую сторону без ожидания.
Тесты не используют Cargo output/AVD; правила контракта/типы/unsafe и сами
Rust/клиентские команды сохранены. Это изменение драйвера проверки,
а не фоновые операции приложения.

Первый запуск `addb9c4` выявил Windows shell timeout после reload доски
(GAP-014). Host UI driver теперь при отказе выводит текущий hash, заголовки,
alerts/statuses, имена region, busy-state и последние 30 read outcomes
(HTTP/status/error/count без содержимого объектов); API ошибки и сетевые отказы
записываются при возникновении. Вывод относится только к синтетической
установке драйвера; credentials, headers и persisted command bodies не
записываются. Исходный отказ повторно выбрасывается, cleanup сохраняется.
Повтор теста, увеличение timeout и ослабление assertions не добавлены;
пять успешных локальных workspace-прогонов не доказывают устранение причины.

В attempt 2 `2fa85ed` обнаружен другой отказ после reload: read-only
`space.list` вернул `unavailable`, UI корректно показал недоступность ядра
и отключил retry. Источник ошибки ещё не доказан. Только у синтетического
debug-fixture включается `ZEDONE_CLOCK_DIAGNOSTICS=1`: Rust выводит локальную
категорию `StorageError`, а при срабатывании прежнего fail-closed clock guard —
calendar/observed milliseconds. В release этот код не компилируется;
штатный CLI/product-check выполняется без диагностического флага.
Запись stderr best-effort, без SQL/путей/ключей/command bodies; ошибка записи
не заменяет исходную командную ошибку. Авторизация, clock policy и API-ответы
не меняются, retry/поблажка для часов не добавлены.

При подготовке браузера Linux использует HTTPS-архивы archive.ubuntu.com
и security.ubuntu.com вместо Azure HTTP mirror в `/etc/apt/apt-mirrors.txt`.
В PR #54 два запуска исчерпали 10 минут при загрузке 21.1 MB шрифтов с Azure
после успешного product-check. Шаг меняет только mirrorlist одноразового runner;
существующие suites/components/Signed-By, подписи Release и хеши пакетов
сохраняются, Playwright по-прежнему устанавливается с `--with-deps chromium`.
Windows этот шаг не выполняет. Сам факт смены зеркала не гарантирует доступность
сети или время будущего запуска; проверяется новый CI head.
Исходная схема списка зеркал подтверждена
[скриптом GitHub runner-images](https://github.com/actions/runner-images/blob/c74a28e5f8943ae6a39cac7c3701dd6bd0a2ea51/images/ubuntu/scripts/build/configure-apt-sources.sh).

Для последовательного Linux и Android набор cache paths остаётся корневым
и прототипным target. Добавление третьего физического target в прежнем PR
меняло версию архива:
GitHub учитывает [пути и compression tool в cache version](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching#cache-key-matching).
Поэтому одинакового restore-prefix было недостаточно для прежнего архива main.
Возврат прежнего набора путей устраняет это различие. В PR #52 head `3943831`
фактический full-match restore подтверждён на Windows/Linux/Android; оба host
job прошли с прежними лимитами (EV-064). Это не обещает hit будущим запускам.
Проверки выполняются
и без кэша, Cargo заново проверяет актуальность units. Никаких ручных правок
fingerprints, пропуска compilation/tests или PR-записи кэшей main нет.

| Кэш | Состав и ключ | Владелец |
| --- | --- | --- |
| Rust | Registry/git и зависимости выбранных `target`: Linux/Android — корневой и прототипный, Windows — отдельно по suite; учитываются OS/архитектура, Rust/окружение, Cargo manifests/lockfile; Android дополнительно разделён по NDK | Swatinem/rust-cache, SHA v2.9.3 |
| Tauri CLI | Только `$RUNNER_TEMP/zedone-tauri-cli/bin/cargo-tauri`; точный ключ OS/архитектура/Rust/tauri_cli, без широкого restore fallback | actions/cache restore/save, SHA v5.1.0 |
| Gradle | Зависимости и wrapper из Gradle User Home; ключ по environment-versions.json, Cargo.lock и файлам вложенного Gradle-проекта | Уже используемый actions/setup-java, `cache: gradle` |

Версии берутся только из [environment-versions.json](../../scripts/environment-versions.json).
Для Rust cache выбран pinned toolchain через `RUSTUP_TOOLCHAIN` в окружении
этого action; глобальный default разработчика не меняется. Cargo bin не
кэшируется Rust action, workspace crates не сохраняются как готовый результат.
Кэши Gradle и Rust имеют единственного владельца каждый.

Кэши записываются только из успешных запусков `main`, PR и ручные запуски
остальных веток читают доступный кэш базы. [GitHub ограничивает область
видимости кэшей](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching):
PR-кэш не прогревал бы другие PR. Первый запуск после интеграции нового
ключа может быть холодным; пропуск кэша не пропускает проверки.

SDK и браузеры устанавливаются как прежде. Не кэшируются весь checkout,
тестовые базы, AVD, `local.properties`, Android keystore или состояние подписи.
Кэширование не добавляет новые права workflow: остаётся `contents: read`,
actions закреплены полными SHA, Git credentials в checkout не сохраняются.

## Tauri CLI и временный диск

[install-ci-tauri.sh](../../scripts/install-ci-tauri.sh) работает в существующем
`RUNNER_TEMP`: проверяет `cargo-tauri --version`, переиспользует точную версию
либо устанавливает закреплённую через `cargo +<rust> install --locked --force`
в отдельный root. Неверная версия/неработающий бинарник восстанавливаются
установкой; неверный результат установки завершает job ошибкой.
Workflow добавляет только этот `bin` в PATH следующих шагов.

Компиляция CLI использует собственный каталог `zedone-tauri-build.*`.
Оба Cargo output location направлены внутрь него. EXIT/INT/TERM удаляют
только этот каталог с сохранением кода ошибки; размер выводится перед
удалением. Не перехватываемый KILL или остановка VM могут прервать cleanup;
одноразовую VM удаляет GitHub. SDK setup удаляет скачанные архивы и прежнюю
копию Emulator после успешной установки проверенного pinned payload.

В Actions видны отдельные времена установки CLI, подготовки контрактов,
host build, каждой ABI и проверки пробника. `df` выводится до SDK/установки
CLI, итоговые `df`/`du` — и при отказе, включая размеры SDK/Cargo/Gradle/target.
Это помогает проверять причину нехватки места; добавление кэшей само по себе
не доказывает её устранение и также требует свободного диска при restore.

## Проверка и измерение

На машине с Bash, jq и ShellCheck из корня репозитория:

```bash
shellcheck scripts/install-ci-tauri.sh scripts/check-ci-tauri.sh
bash scripts/check-ci-tauri.sh
```

Тест использует заглушку Cargo: реальных загрузок, смены личного toolchain
или установки CLI нет. Проверяются cold/warm, неверный/неработающий кэш,
отказ компилятора, неверная установленная версия, удаление частичных build
и сохранение соседних файлов. Данные теста создаются и убираются в `temp/`.
Эта же команда выполняется перед тяжёлой Android-подготовкой в Actions.
Для YAML нужен actionlint; локальная проверка не заменяет GitHub restore/save.

После публикации проверить PR jobs на реальных runners. После интеграции
сохранить первый успешный `main`-запуск, затем повторить на том же SHA вручную
и сравнить холодный/прогретый случаи, проверив фактические cache-hit в логах.
Если ключ уже существовал, первый запуск не называть холодным. Записать
время до последнего необходимого результата, сумму времени jobs, restore/save,
размеры кэшей/занятый диск, повторы и сохранение всех проверок. Разный SHA или
изменившийся runner image отмечать как ограничение сравнения. Фактические
результаты и ещё не проверенный охват — в [EV-028](../agent/verification/EV-028-pr-checks.md).

## Подключение к Android WebView

Драйвер `check-android.mjs` до CDP-подключения ждёт `/json/list` с page target
и непустым WebSocket URL. Готовность `/json/version` сама по себе не подтверждает
готовность страницы. Проверка `check-android-webview-target.mjs` входит в host
и Android CI: сокет без страницы, ошибки/неполный discovery и истечение срока
не превращаются в успешное подключение. Тайм-аут ожидания остаётся 20 секунд.

При отказе подключения до cleanup выводятся ограниченные списки CDP/Playwright,
процесс APK и lifecycle logcat. Исходная ошибка сохраняется, перезапуск и
проверки SQLite/отзыва ключа не повторяются автоматически. Пример исходного
отказа и ограничения выводов — [EV-031](../agent/verification/EV-031-android-webview-target.md).

Shell-драйвер, как прототипный, использует нативный CDP touch и DOM-проверки:
актуальный Playwright требует `Element.checkVisibility`, отсутствующий в
WebView 101. Перед касанием проверяются видимость, размеры, disabled и
`elementFromPoint`, после — реальные состояния меню/темы/навигации. Имена,
radio.checked, сохранение после reload, Back, отсутствие переполнения и
пустой localStorage после сброса проверяются явно. Полифиллы и принудительный
DOM click вместо ввода не добавляются. Локальная Android UI-приёмка по-прежнему
проходит через Device; desktop proof драйвера её не заменяет (EV-064/LOG-315).

## Отмена

Остановить текущий запуск — **Cancel workflow**, отключить — **Disable workflow**.
Удалить кэш — **Actions → Caches**, отменить настройки — обратным коммитом.
Удалённый кэш просто приводит к закреплённой холодной сборке. Личные SDK,
AVD, ключи, настройки Rust и данные ZeDone эти действия не меняют.

### Android: зеркала зависимостей эмулятора

Android CI перед apt-get применяет тот же HTTPS mirrorlist Ubuntu archive/security,
что Linux host. В PR #83 повторный prototype-ui исчерпал 10m: APT ожидал Azure
HTTP mirror при установке libpulse0, затем переходил к Ubuntu архиву.
Настройка меняет только mirrorlist одноразового runner; suites/components/
Signed-By, подписи Release и хеши пакетов сохраняются. Пакет libpulse0 и KVM
остаются обязательными. Матрица, все команды/negative cleanup, strict gate и
10m не меняются. Routing guard проверяет одинаковую настройку с host и
сохранение установки libpulse0. Реальный Linux результат/время требует нового CI;
см. [EV-083](../agent/verification/EV-083-tags-ui.md).

### Текущий лимит Android jobs

По прямому поручению разработчика от 2026-11-10 каждый Android
matrix jobs имеет timeout-minutes: 15. Strict aggregate остаётся с лимитом
1 минута. Это переопределяет прежние упоминания 10m для Android matrix;
host бюджеты, сценарные таймауты, runtime/negative cleanup и правила
принятия SUCCESS не меняются. Routing guard проверяет точное значение 15.
Основание и фактическая новая приёмка — [EV-083](../agent/verification/EV-083-tags-ui.md).

### Парные проверки запроса устройства

Product-native явно запускает два ignored probe на host и нативном Android:
прежний TLS/identity и pairing::tests::cross_platform_probe. Второй проверяет
неверный pin, приём запроса, повторное открытие SQLite, загрузку исходной сессии,
receipt, просмотр/отмену и отсутствие нового доступа. Каждый probe использует
собственный localhost port/metadata и удаляет только собственный adb reverse.
Обычный cargo test не заменяет этот парный запуск. Секреты синтетические,
параметры transport не становятся настройкой приложения.

Native SecretStoreInstrumentation дополнительно проверяет pending-pairing-v1:
сохранение, чтение после нового экземпляра, изоляцию ciphertext/AAD и удаление
без повреждения channel identity. Полный Rust→Kotlin→TLS процесс и формы
подключения остаются отдельной приёмкой; [EV-099](../agent/verification/EV-099-pending-device-request.md).

PR #83/head d1c1c44 интегрирован в 695d0b8 после всех 17 SUCCESS.
Финальные Android runtime/negative cleanup завершились при разрешённых 15m,
strict aggregate passed; прежний набор и все проверки сохранены. Это текущий
проверенный запуск, не гарантия cold/future времени; EV-083/LOG-464.


### Независимая парная приёмка APK — CI.11

PR #107/head 4e8a03a attempt 2 подтвердил нехватку общего бюджета shell:
annotation job 134296548843 сообщает превышение 15m0s. Обычный UI и полный
парный набор успели пройти, но job не завершил общий preflight/cleanup.
Частичные результаты не считаются успехом CI; [EV-116](../agent/verification/EV-116-android-ci-budget.md).

`shell` и `pairing` теперь независимы, каждый с прежними 15m. `pairing`
собирает обычный x86_64 APK и host fixture, запускает общий environment
callback, устанавливает production APK через CI-only `prepare-pairing-app-ci.mjs`,
затем собирает acceptance APK и выполняет прежний `check-pairing-app.mjs`.
Свежая установка может иметь пустой список файлов: runner сохраняет пустой
manifest и проверяет его точное восстановление; неверные строки по-прежнему
отвергаются. Bootstrap запрещён вне CI/эмулятора; отказ/сигнал установки
завершает suite ошибкой. В CI.11 обе suites сохраняли общий compiler/version/cleanup
negative control; с CI.12 он имеет одного владельца prototype-native.
Все прежние команды default dispatcher и их порядок
сохранены с добавлением production install перед acceptance build.

Матрица содержит семь suites, общий `android` требует успеха каждой;
ожидаемый PR-набор — 18 checks вместе с прежними host suites/aggregates.
Rust cache сохраняет единственного main writer. Дополнительный runner
повторяет SDK/build setup; фактическое время требует нового CI, не предполагается.
Routing guard проверяет полноту, бюджет 15m, exact ABI owners, default/native,
bootstrap, пустой/валидный/неверный manifest, отказы/сигналы и строгий aggregate.
Исходный localStorage residue attempt 1 остаётся GAP-032 с неизвестной причиной;
его строгая проверка не ослабляется и прошла в attempt 2.
