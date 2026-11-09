# Проверки PR и кэши сборки

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

Android выполняет пять независимых наборов параллельно на стандартных runners:
`product-native` (x86_64 release/guards и все native tests),
`product-arm64` (arm64 release/guards), `prototype-native` (host/Android Automerge),
`prototype-ui` (обе APK и
instrumentation прототипа, host HTTP endpoint и весь UI/lifecycle-набор),
`shell` (обе APK оболочки и её UI-набор). Каждый job владеет отдельным SDK/AVD,
проходит прежние environment/negative-cleanup проверки и имеет лимит 10 минут.
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
Объём SDK/проверки environment/negative-cleanup и лимиты не сокращаются.
P04.06 запускает независимые клиентские delivery-тесты параллельно нативной
product-сборке в том же job: они не используют эмулятор или Cargo output.
Драйвер ждёт оба результата и сохраняет все отказы через AggregateError;
ранняя ошибка одного набора не оставляет другой без ожидания и cleanup.
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
алгоритму toolkit для Windows; настоящий restore ещё требует нового CI.

Linux fallback использует префикс `Linux-x64`, Windows — прежний
`Windows_NT-x64`. Реальный Linux restore/save и время нового разделения
требуют подтверждения CI; локальные проверки их не заменяют.

Linux разделён в P05.05 после повторного исчерпания последовательного
10-минутного бюджета PR #82. На `dd84aac` attempt 3 остальные 12 checks
прошли, Linux был отменён во время shell UI-набора; GitHub явно указал
job timeout. Каждый новый Linux suite сохраняет лимит 10 минут, проверок
и сценарных timeout изменений нет. Новый полный PR-набор содержит 17 checks:
14 suites и три обязательных aggregates. Ускорение/холодное время и новый
head не принимаются до фактического CI; подготовка отдельных runners может
увеличить суммарные runner-минуты. Доказательства и ограничения —
[EV-082](../agent/verification/EV-082-board-moves-ui.md).

Цель CI.04 — ≤10 минут **от старта workflow до завершения Windows aggregate**,
включая очередь runners, подготовку и post/cache steps. Короткая длительность
самого aggregate не является ускорением. Сумму job time считать отдельно:
повторная подготовка и восстановление кэшей могут увеличить runner-минуты.
Сравнивать cold/warm запуски одного SHA с одинаковым охватом; rerun
неуспешной части не заменяет измерение полного запуска.
Исходный PR #78: 19:29 при полном Rust cache hit. Разделение реализовано,
но достижение цели ещё требует нового CI; локальные проверки и ограничения —
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
Холодное время новых четырёх suites ещё требует нового полного CI.

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
