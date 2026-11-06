# EV-064: P05.01 — общая оболочка UI

Статус: `verified` — локальная приёмка оболочки завершена (LOG-308),
BUG-005/GAP-008 устранены и перепроверены. Готовится PR; финальные CI/интеграция
ещё не выполнены, P05.01 не объявляется done.

```yaml
step: P05.01
base_revision: 75354371830656bf5c6e8ab3a8a5920cc4fded97
branch: feat/p05-01-ui-shell
preexisting_changes: []
goal: "Одна оболочка с навигацией, темами и достоверной проверкой доступа в Windows-браузерах и Android APK"
requirements: [T08, DEC-007]
dependencies_evidence: ["P03.05 done, PR #47/f4bf4e1/EV-058", "P04.01 done, PR #50/4f2ae48/EV-063"]
scope: ["ui", "package.json/bun.lock", "scripts/check-ui.mjs", "CI integration", "docs"]
checks: ["vue-tsc под Node (локально 26.8.1, база CI 24.22.1)", "Vite под Bun", "Rust host fmt/Clippy/tests", "Chrome/Opera UI+HTTP", "две ABI APK", "Android Device UI+настоящий IPC — passed"]
reverse_and_failure: "Навигация/Back, смена темы в обе стороны/системный сброс; отказ доступа/связи/storage и поздний ответ"
result: "Локальный shell-срез verified: Windows UI/HTTP, обе debug ABI/manifest, Device темы/навигация/Back и настоящий space.list IPC/отказы; финальные PR CI ещё ожидаются"
next: "Создать PR; после успешного финального head/CI/ревью merge, удалить ветку и зафиксировать done"
```

Охват больше ориентира 300 строк: один проверяемый результат включает общий UI
и две точки сборки с одной границей доступа. Сгенерированный Android scaffold,
Cargo/Bun locks учитываются отдельно. Самостоятельные прикладные операции не
включаются. Решение и ограничения порта ранее выданного доступа — ADR-0042.

План оформления: canvas/акцент — утверждённые четыре цвета спецификации;
светлые rail/surface `#d9dfdc/#f2f4f3`, тёмные `#191e1b/#202823`;
системный Segoe UI/system-ui для элементов управления, Georgia для h1.
На ПК — rail 222 px и одна рабочая область; на телефоне — верхняя строка
с модальным меню. Левое выравнивание, без декоративных метрик и демо-карточек.
Проверка замысла: взята структура выбранных примеров, но их медный акцент
заменён утверждённым зелёным; фактура убрана ради читаемости и производительности.

Локальные команды/машинные пути/снимки и логи сохраняются только в
игнорируемом `docs/reports/p05-01/`.

## Фактические результаты

Матрица первого прохода LOG-305/306 сохранена ниже как история. Актуальная
локальная приёмка после исправления BUG-005 — LOG-308 в конце документа.

| Проверка | Результат |
| --- | --- |
| `bun run ui:check` | Passed с явным `UI_NODE_VERSION=26.8.1`: vue-tsc, Vite, Rust fmt/Clippy всех targets, два integration-теста, host/example build и оба браузера |
| Chrome 156.0.8078.80 / Opera Chromium 155.0.8059.85 | Passed: настоящая space.list через HTTP/Service, unconfigured, реальный access_denied неправильному ключу, ошибочный session-файл/некорректный ответ, восстановление доступа |
| Темы и навигация | Passed в обоих браузерах: светлая/тёмная/системная, restart/reload, реакция на системную тему, удаление override, отказ localStorage, history Back, клавиатура/фокус/Escape/mobile menu, ширина 360 px |
| Хранение UI | После проверки нет ключей/списков/данных в localStorage; сохраняется только предпочтение темы, системный выбор его удаляет; внешних запросов нет |
| Rust shell | Два теста passed: отсутствие автоматической выдачи при наличии session-файла, строгий/ограниченный session, framing/security Host/Origin/Fetch Metadata/CSP/no-store, неизвестные пути без чтения файлов |
| `bun scripts/check-product.mjs --check-types` | Passed: Rust/OpenAPI/TypeScript/standalone validator совпадают, продуктовый контракт не менялся |
| Debug APK x86_64 и arm64 | Passed `build-ui-android.mjs` для обеих ABI, включая готовый manifest: package/Activity/LAUNCHER/DEFAULT. Release APK/физический arm64 не проверены |
| Android Device | LOG-305: запуск не прошёл. LOG-306: исправленный APK успешно открыт через Device два раза, целевая MainActivity/экран/сеанс подтверждены. Полная UI-приёмка не пройдена: BUG-005, темы/Back/прикладной IPC ещё не приняты |
| Node 24.22.1 / Linux / GitHub Actions | Не запускались. CI расширен, но его успех не заявляется |

Два Rust-теста проверяют границу безопасности хоста; прежняя матрица core/storage
в этот проход целиком не повторялась, их runtime не менялся. Драйверы сборки
проверили закреплённые Rust/Tauri CLI/JDK/NDK. Gradle/Tauri выдали предупреждения
о своих deprecated API/Java target 8; сборка успешна, зависимости не обновлялись.

Проверка макетов: просмотрены headless-снимки светлого ПК 1280×900 и тёмного
экрана 360 px. Текст/управление не обрезаются, структура и цвета соответствуют
плану; это не доказательство Android. Критика: заметная serif-иерархия оставлена
только h1; вторичные статусы и действия используют единый системный шрифт.
Фоновой анимации нет. Полный аудит доступности и все UI-сценарии относятся к P05.10.

## GAP-008 и передача

Ниже сохранён результат первого прохода LOG-305. Продолжение диагностики и
проверка исправления — в разделе LOG-306 далее.

Device сообщил `Android helper could not launch the application` и рекомендовал
`--debug`. Повтор вернул `Activity class {app.zedone.local/app.zedone.local.MainActivity}
does not exist`. По протоколу после повторного отказа UI-управление остановлено;
adb/CDP/другой эмулятор локально не использовались вместо Device. Открытый этим
сеансом эмулятор выключен штатным device_close.

Read-only Android package resolver установил различие: MAIN+LAUNCHER находил
Activity, MAIN+LAUNCHER+DEFAULT — нет. Helper добавлял DEFAULT. В manifest
добавлена категория DEFAULT, обе ABI пересобраны; aapt готовых APK подтвердил
её наличие. Исправленный APK через Device ещё не запускался. Это объясняет
конкретное разрешение Intent, но не подтверждает отсутствие следующих runtime-
дефектов. Нативные темы/Back, сеанс и реальная прикладная команда IPC требуют
отдельного продолжения проверки. Начальная выдача прав по-прежнему не реализована
и не заменяется ручным bootstrap пользовательской установки.

Добавлен настоящий CI-only driver навигации/тем/Back/сеанса; он не запускался
локально и не доказывает успешный прикладной IPC. Синтетический fixture вынесен
в example target, не входит в host/АПК. Следующая работа — приёмка исправленной
оболочки через Device и проверка domain IPC на изолированной тестовой установке;
после неё закрыть GAP-008/EV и только затем готовить публикацию в рамках поручения.

## Пять правил

- Обратимость: смена темы в обе стороны, системный сброс, закрытие меню и
  история навигации проверены на Windows. Android Back ещё не принят.
- Человек/агент: семантические controls общие, прикладная проверка — существующая
  space.list/Service с теми же правами, без скрытой выдачи полномочий.
- Документация: user/agent/development/architecture, очередь и журнал обновлены.
- ПК/Android: общий исходник и обе ABI есть; полный нативный паритет пока **не доказан**.
- Один источник: данные/права в прежней SQLite/Automerge, контракт производный;
  в UI только представление и локальное предпочтение темы.

## LOG-306: почему Device дважды отказал в запуске

Разработчик отдельно поручил выяснить причину. Проверены исходные diagnostics,
код установленного agent-device 0.21.31, Android resolver, исправленный APK и
настоящий запуск через Device на том же API 33 AOSP эмуляторе.

Причина — несовпадение категорий Intent, а не отсутствие Kotlin-класса.
Вызов Device использовал MAIN с LAUNCHER **и DEFAULT**. Старый manifest содержал
MAIN/LAUNCHER/LEANBACK_LAUNCHER, но не DEFAULT. Обе неудачные попытки использовали
одинаковый несовместимый Intent; флаг --debug менял диагностику, не категории.
Возвращённое `Activity class ... does not exist` вводило в заблуждение: компонент
присутствовал в установленном пакете, но не принимал такой внешний Intent.

На Android 13 при target SDK 33+ внешние Intent должны совпадать с action и
категориями фильтра экспортированного компонента; иначе ActivityNotFoundException.
Это правило действует и при указанном компоненте; исключения — тот же app,
system UID и root. Источник — [Android Developers](https://developer.android.com/guide/components/intents-filters#match-intents-to-other-apps-intent-filters).
ZeDone использует target 37, проверенный эмулятор — Android 13.

Последовательные факты:

1. До обновления установленного APK read-only resolver для MAIN+LAUNCHER нашёл
   `app.zedone.local/.MainActivity`, а с дополнительным DEFAULT вернул `No activity found`.
2. Взята уже собранная версия с DEFAULT в том же launcher filter; код приложения
   и helper в этом диагностическом проходе не изменялись. Выполнена установка -r.
3. Тот же resolver с обеими категориями нашёл MainActivity (`isDefault=true`).
4. `agent-device open app.zedone.local --relaunch --debug` прошёл. Повторный
   `open ... --relaunch` также прошёл. Device snapshot показал WebView «ZeDone»
   и «Доступ не настроен»; device_screenshot подтвердил целевой экран.
   `dumpsys activity` независимо подтвердил topResumedActivity целевого пакета,
   crash buffer был пуст. Первое старое сообщение «Opened» не использовалось
   как доказательство, поскольку тогда snapshot/foreground показывали прототип.

Исправление запуска подтверждено. Это не полная UI-приёмка. На настоящем экране
замечен BUG-005: при узком экране остаётся desktop rail, заголовок разбивается
на несколько узких строк. Установлен WebView 101.0.4951.61, а Vite вывел
`@media (width<=700px)` вместо исходного `(max-width: 700px)`; новый range syntax
поддерживается с Chrome 104 ([Chrome/Web.dev](https://web.dev/articles/media-query-range-syntax)).
Это отдельная несовместимость CSS-сборки с WebView, не причина двух отказов
запуска. В этом диагностическом проходе она зарегистрирована, не исправлялась.
Нужно задать и проверить совместимость CSS с целевым WebView, затем повторить
UI/темы/Back и прикладной IPC. Не закрывать весь GAP-008/P05.01 из-за успешного open.

Локальные diagnostics, две команды запуска, снимок и сводка находятся в
`docs/reports/p05-01/`. Эмулятор, открытый для этой диагностики, закрыт штатным
Device после проверки. Прикладные данные/права и runtime-исходники не изменялись.

## LOG-308: итоговая локальная приёмка перед PR

BUG-005 исправлен: target Chrome 101 задан для JS/CSS, класс выбранной темы
заменяет :has, высота получает 100vh fallback. После этого обе debug ABI
пересобраны, готовые manifest/категории запуска проверены. Изменения не требуют
обновления WebView или SDK для прохождения приёмки.

`bun run ui:check` passed на финальном runtime: vue-tsc под явно выбранным
Node 26.8.1 (базовый 24.22.1 проверится в CI), Vite/Bun, fmt/Clippy всех shell
targets, два Rust integration-теста, host/example build и полный набор
UI/реального HTTP в Chrome 156/Opera Chromium 155. Последний product check-types
также passed; core/storage/контракт/SQL/эталоны не менялись.

Настоящий Android 13 AOSP/WebView 101 в Device:

- Мобильная компоновка без desktop rail и обрезанного заголовка, меню и переход
  через его ссылку в настройки; системный Back закрывает меню и возвращает
  из настроек. После закрытия меню фокус возвращается на trigger.
- Светлая/тёмная тема, сохранение тёмного выбора после настоящего relaunch,
  сброс в системный режим и реакция на смену системного оформления dark/light.
  Снимки Device просмотрены, выбранная radio-тема читается и выделена.
- На изолированной синтетической установке настоящий `space.list` прошёл через
  UI → IPC → Service → SQLite с ответом «Доступ подтверждён». Неверный ключ
  дал «Нет доступа»; отсутствие файла — «Доступ не настроен»; повреждённый файл —
  недоступность; возврат исходного ключа и явный повтор снова подтвердили доступ.
- Исходный sandbox сохранён перед проверкой и восстановлен после закрытия
  процесса; синтетическая база/ключ удалены с проверкой принадлежности fixture.
  Ключи в логи/URL/UI/localStorage не выводились. Продуктовый bootstrap не добавлен.
  После восстановления оригинала повторный запуск снова показал отсутствие
  настроенного доступа. Открытый этим сеансом эмулятор штатно закрыт/выключен.

Все UI-действия, relaunch и настройки выполнялись через agent-device, снимки —
agent-device/device_screenshot. Штатный adb использовался только для установки,
чтения состояния и переноса тестовых файлов. Локальный CI driver не запускался
в обход Device. Воспроизводимые сценарии CI добавлены, но их успех до запуска
Actions не заявляется. Условия ошибки аргумента PowerShell и устаревшей ссылки
Device исправлены по подсказке инструмента; сами нажатия перепроверены.

Пять правил соблюдены в охвате оболочки: обратные действия и Back, единые
семантические controls/общий Service для человека и агента, обновлённые docs,
проверенный Windows/Android-emulator UI/HTTP/IPC и одно авторитетное хранилище.
Локальное предпочтение темы не становится базой заметок/прав. Физический arm64,
release APK, полный аудит доступности P05.10, управление пространствами/колонками,
синхронизация и начальная выдача доступа этим шагом не принимаются. T08 partial.

Визуальная пара PR снята из настоящих base/final версий на одном пустом входном
сценарии, светлой теме и viewport 1280×900. У base нет продуктовой оболочки,
поэтому Before показывает доступный в main синтетический prototype entry;
его исходники побайтово совпадают с base. After показывает новую оболочку.
Снимки просмотрены, внешние вложения подготовлены; media/промежуточные captures
не добавляются в Git. Номер PR, финальный head, CI и merge фиксируются отдельно.

## PR #52: первый Linux CI и бюджет холодной сборки

[PR #52](https://github.com/Zeyafix/zedone/pull/52), исходный head `f56daad`:
[Linux job](https://github.com/Zeyafix/zedone/actions/runs/42243344708/job/127979055577)
отменён по аннотации `The job has exceeded the maximum execution time of 10m0s`.
Продуктовые проверки и полный прототипный шаг прошли; shell vue-tsc/Vite с
закреплённым Node 24.22.1 прошли, отмена произошла во время холодного Clippy.
Это не успешная Linux-приёмка и не подтверждение причины возможных будущих отказов.

В `2b97d25` host matrix временно получила одинаковый бюджет 20 минут. Разработчик
отклонил это решение (LOG-310); оно заменяется оптимизацией ниже. Исходный
cancelled остаётся в истории, не объявляется passed. Linux этого промежуточного
head прошёл, Windows был отменён по своему 20-минутному лимиту, Android завершился
с ненулевым кодом storage test binary. Это не готовность к merge.
Оба публичных PNG-вложения PR после публикации проверены побайтово против
просмотренных файлов; rendered body содержит ровно два изображения.

## Оптимизация вместо увеличения лимита

Восстановлены прежние бюджеты Linux 10 / Windows 20 минут. Набор шагов и
проверок не сокращён. Исправлены три источника лишней работы:

- `tauri-build` нужен только Android packaging, но прежде компилировался и для
  browser host. Optional build-dependency включается feature `android-shell`,
  которую явно передаёт штатный APK build-драйвер. Android без feature получает
  compile error, а не сборку без JNI setup.
- Отдельный shell Cargo workspace теперь использует корневой `target/` вместе
  с product-check. Cargo проверяет fingerprints и различия targets/profiles/
  features/flags; артефакты не копируются и не объявляются актуальными вручную.
  Прототипный target остаётся отдельным.
- Из обоих workflow удалён третий физический cache path. Его добавление меняло
  GitHub cache version, несмотря на существующий restore-prefix и архивы main.
  Возвращён прежний набор путей; фактический cache restore нужно подтвердить
  новым CI, одно это изменение не считается cache-hit.

Контрольный cold-target `cargo check` на Windows: две временные копии before
`2b97d25` / after, новые отдельные target directories, одинаковые toolchain,
registry, assets и профиль, без параллельных компиляторов. Оба passed; before
22.055 s и 222 compile/check записи, after 9.509 s и 97 записей. В дереве host
build-dependencies после изменения нет tauri-build. Это один локальный контроль,
не оценка времени GitHub runner и не измерение полного `ui:check`.

Полный host `ui:check` с общим target passed: прежние типы/Clippy/тесты/сборки
и browser suites Chrome/Opera. Полный Windows product-check passed: тесты,
release, unsafe rejection, Rust/OpenAPI/TS/validators и клиентские сценарии.
Обе debug APK собраны с явной Android feature, manifest checks passed. Новый
APK установлен и запущен через Device; snapshot и device_screenshot подтвердили
мобильный экран оболочки. Полная UI/IPC-приёмка LOG-308 сохраняется.

Android CI в `2b97d25` скрывал stdout упавшего storage test binary за общей
ошибкой `adb`. Драйвер теперь печатает captured stdout при отказе, сохраняя
ненулевой код и все assertions. Причина того отказа ещё не установлена;
локальный полный Android product-check после изменения passed: все нативные
тесты, идентичный контракт, обе release ABI продукта и unsafe guards. Исходный
отказ не воспроизведён, успешный повтор не доказывает его причину. Тестовый
sandbox удалён штатным runner, открытый эмулятор закрыт/выключен через Device.
Новый окончательный CI обязателен перед merge.
Локальные логи/контрольное измерение — `docs/reports/p05-01/`.

## LOG-312: результат оптимизации в CI и исправление WebView-драйвера

Head `3943831`: оба host job Windows/Linux passed с исходными лимитами.
Raw logs подтверждают `Cache hit`, `Cache restored successfully` и
`full match: true` для прежних Rust-архивов на Windows/Linux/Android.
Таким образом, проблема совместимости cache paths проверена на настоящих
runners; это не гарантия наличия кэша у будущего запуска.

Android job прошёл обе debug APK оболочки, product native tests/contract,
обе release ABI продукта/unsafe guards, прототипные native/UI/HTTP/lifecycle
проверки. Все 134 storage-теста passed. Отказ старого `2b97d25` здесь не
воспроизведён, его причина остаётся неизвестной.

Новый CI-драйвер оболочки остановился уже после прототипных проверок при
CDP-подключении: `Browser.setDownloadBehavior: Browser context management is
not supported`. Playwright по умолчанию настраивает desktop downloads в
существующем browser context; AOSP WebView эту команду не поддерживает.
Вызов теперь использует `noDefaults: true`, как существующий прототипный
драйвер. Это документированный режим Playwright, а не перехват/игнорирование
ошибки. Источник — [connectOverCDP](https://playwright.dev/docs/api/class-browsertype#browser-type-connect-over-cdp-option-no-defaults),
а также исходник закреплённого Playwright 1.64.1, который при этом режиме
не отправляет `Browser.setDownloadBehavior` для default context.

Проверены синтаксис драйвера и протокол закреплённого Playwright на изолированном
синтетическом CDP endpoint: с defaults воспроизводится тот же отказ, с
`noDefaults: true` подключение успешно и неподдерживаемой команды в trace нет.
Это проверка handshake, не Android UI-приёмка. Драйвер локально в обход Device
не запускался; product runtime/видимый UI не менялись, LOG-308 сохраняется.
Все UI assertions/сценарии CI сохранены. Полный CI нового head ещё необходим
перед merge. Raw logs и protocol trace — в `docs/reports/p05-01/`.

## LOG-313: Linux storage failure и освобождение folder lock

Linux `7580981` упал в старом product storage test
`errors_at_each_boundary_preserve_old_until_confirmed_publication`: вместо
ожидаемого `OutcomeUnknown` получен `Unavailable`. Это не timeout и не
падение shell/browser. В исходном логе отсутствовали конкретный checkpoint
и системная ошибка, поэтому точный syscall того запуска не установлен.

При проверке найден отдельный воспроизводимый дефект жизненного цикла lock:
сырой `File` только закрывался при выходе из операции. Duplicate/inherited
descriptor удерживал ту же блокировку после завершения владельца; следующий
захват мог получить `Unavailable` ещё до нужной инъекции. Параллельные fork/exec
crash-тестов — возможный источник такой копии на Unix. Это согласуется с
[Rust File::try_lock/unlock](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)
и [flock open-file-description semantics](https://man7.org/linux/man-pages/man2/flock.2.html),
но не доказывает причину именно старого CI/Android отказа без его trace.

Детерминированная регрессия сохраняет `try_clone` дескриптора после завершения
первого владельца, требует захвата следующего и проверяет, что закрытие старой
копии не освобождает lock нового владельца. На прежнем поведении она упала с
`Unavailable` на Windows и нативном Android. С guard, явно вызывающим unlock,
обе проверки passed. Исключение конкурирующих операций, граница commit,
OutcomeUnknown, файлы/свидетельство и возможность восстановления не ослаблены.
Fault matrix теперь требует, чтобы запрошенный checkpoint был достигнут
ровно один раз, и указывает его в assertion. Ранний отказ не маскируется.

Обычные 20 повторов полного старого native storage набора passed и не
воспроизвели исходный Linux отказ; это ограничение сохранено. После исправления
полный Windows product-check passed: fmt/Clippy, 135 storage и прежние
core/contract/adapters/TS tests, release, unsafe guards и побайтовые
Rust/OpenAPI/TS/validators. Полный Android product-check также passed: те же
native tests/контракт, обе release ABI и unsafe guards. Штатные runner удалили
свои sandbox; открытый для native checks Device-эмулятор закрыт/выключен.
Окончательные GitHub CI нового head ещё обязательны перед merge.

BUG-006 относится к освобождению lock, не к доказанной причине старого syscall.
ADR-0034, developer docs и EV-051 уточнены; публичные команды/права, SQL,
wire/архивы, зависимости и видимый UI не менялись. Локальная UI/IPC-приёмка
LOG-308 сохраняется в своём охвате; native checks не выдаются за UI-проверку.

## LOG-314: Android storage failure и гонка shutdown

Android job `128021389124`, head `7580981`, упал в
`busy_database_and_shutdown_never_return_success`: оставшийся clone после
успешного shutdown получил `OutcomeUnknown` вместо `Unavailable`. Это другой
отказ, чем Linux publication test, и не ошибка WebView/timeout.

Прежний worker отправлял подтверждение Stop до уничтожения receiver. Между
подтверждением и закрытием очереди clone мог принять новое задание; worker
завершался без его выполнения, а потеря принятого ответа корректно давала
`OutcomeUnknown`. Также терялись уже принятые задания за Stop.
Теперь worker вызывает `receiver.close()`, дренирует принятые задания,
закрывает SQLite и лишь затем подтверждает все принятые Stop. Политика
`OutcomeUnknown` для потери принятого ответа не ослаблена.

Регрессия без таймеров удерживает worker каналом, помещает Stop, SQL-задание
и второй Stop в очередь, затем освобождает worker. Проверяет оба подтверждения,
выполнение принятого SQL и закрытый приём у clone. На прежнем коде стабильно
терялся ответ SQL; после исправления тест passed. Существующий тест Android
не изменён. Локальные логи before/after и полные проверки находятся в
`docs/reports/p05-01/`: `shutdown-drain-before.log`, `shutdown-drain-after.log`,
`product-storage-lifecycle-check.log`, `android-storage-lifecycle-check.log`.
Обе полные проверки прошли с 136 storage tests, прежними core/contract/adapters,
14 клиентскими тестами/97 assertions, сверкой контракта и unsafe guards;
Windows fmt/Clippy/release и обе release ABI Android passed. Android native
runner использовал Device-эмулятор и удалил sandbox; UI-приёмку не подменяет.

BUG-007 resolved локально. ADR-0028/developer docs/журнал обновлены. Обратимость
прикладных действий, общий Service человека/агента и единая SQLite/Automerge
сохранены. UI и контракт не менялись; доказательства LOG-308 остаются в своём
охвате. Окончательные CI обоих lifecycle fixes ещё должны пройти перед merge.
