# EV-082 — UI перемещения карточек и колонок (P05.05)

## Рабочая карточка

```yaml
step: P05.05
base_revision: 94df827532477c1fa4b0261d1c3ecd6bc7dc254e
branch: feat/p05-05-board-moves
goal: "Перетаскивание и явные команды меняют порядок карточек/колонок через единый API"
requirements: [T08, A07, A35, A41, ADR-0005, ADR-0045]
dependencies_evidence: ["P05.03: PR #78/2f54349, EV-078", "P04.05: PR #63/1d36d3a, EV-069"]
scope: ["BoardView/CardsList", "закреплённая VueDraggablePlus из прототипа", "move в штатной очереди UI", "проверки и документация"]
checks: ["полный ui:check Chrome/Opera", "истинный HTTP/Service", "debug APK x86_64/aarch64", "Android Device/IPC/касание/прокрутка"]
reverse_and_failure: "Новое обратное перемещение; сохранённый порядок после stale/отказа, исходная команда при неизвестном результате"
result: "Реализован; полный локальный ui:check Opera/Chrome passed. GAP-017 resolved в FIX.02/LOG-440; PR/CI/merge отсутствуют"
next: "Ревью/интеграция ветки; полный P05.10 остаётся отдельным шагом"
```

## Граница

Только UI P05.05. Перенос карточек между колонками одного пространства и
порядок колонок; порядок пространств, Rust/API/DDL, конфликты обмена P06
не меняются. Применяется существующий ADR-0005, одна закреплённая версия
VueDraggablePlus/SortableJS и прежняя правка только TypeScript-декларации.

Цель связна: два вида ввода используют одну доставку move, поэтому тесты
обратимости/отказов и извлечение доски из SpacesView входят в один срез.
Ориентир 300 строк будет превышен ради полного охвата двух объектов и платформ.
Семь исходных dirty docs сохранены побайтово в игнорируемой
`docs/reports/p05-05/baseline/`; посторонние записи не удаляются.

## Реализация

`BoardView` координирует явный выбор колонки/соседа и завершение жеста.
Оба пути вызывают один `move` через createMutation/CommandClient.
Версия фиксируется при открытии формы или начале жеста; исходный субъект
проверяется перед сохранением намерения. Соседи задаются ID/before, ранг
вычисляет прежнее Rust-ядро. Ответ старой команды не становится текущей доской:
после каждой доставки перечитываются данные, включая отказ/неизвестный исход.

VueDraggablePlus 0.6.1/встроенный SortableJS 1.15.2 и декларационная правка
из прототипа закреплены в корневом manifest/lock. Модель библиотеки —
одноразовое представление: после drop она сбрасывается, карточки не клонируются
в хранилище. Перенос только внутри пространства; конфликтующие/удалённые
объекты заблокированы. Ручки доступны с клавиатуры и открывают диалог
коротким нажатием; текст остаётся областью прокрутки. Родительские действия
блокируются на время жеста/диалога/доставки. Rust/API/DDL не менялись.

## Проверки

| Область | Фактический результат |
| --- | --- |
| Полный ui:check, Chrome | `docs/reports/gap-017/ui-full-final.log`: exit 0 совместного полного набора Opera/Chrome после FIX.02; vue-tsc/Vite/fmt/Clippy/три shell tests/build, весь workspace/cards/editor/Markdown/moves. Chromium 156.0.8078.80, Node 26.8.1 — прежний явный локальный override базы 24.22.1; CI baseline не менялся. Прежний `ui-final-back.log` также passed |
| Настоящий HTTP/Service | Явные карточка/колонка/обратный порядок; выбор колонки/перед соседом; отмена/фокус; реальные мышиные переносы между списками, обратно и в пустую колонку, перестановка колонок. Сохранность ID/текста/выполнения/непустого назначения тега/всей истории и порядок после reload |
| Отказы и доставка | Поздняя отметка агента отклоняет старую move; потеря всех трёх ответов сохраняет один ID/JSON, reload сохраняет очередь. Поздняя обратная move агента переживает точный replay старого receipt. Отказ доступа и QuotaExceeded сохраняют порядок, при локальном отказе запрос не отправляется |
| Общая часть CI-драйвера | Host 360 px/dark/настоящий CDP touch прошёл явные карточка/колонка/обратные действия, отмену и сохранность. Android CI вызывает ту же общую часть с настоящим IPC и native Back; новый CI не выполнялся |
| Opera | FIX.02/LOG-440: полный `ui:check` в Chromium 155.0.8059.85 passed (`docs/reports/gap-017/ui-full-final.log`, exit 0): moving-target/pending-refresh/one-touch, workspace/cards/editor, mouse/touch moves, Markdown/external-link и shell allow/deny. GAP-017 resolved. Прежний отдельный `moves-opera-final.log` также passed; исходный полный отказ сохранён в истории ниже |
| Debug APK | `android-x86-back-final.log` и `android-arm64-back-final.log`: обе финальные сборки и manifest identity/DEFAULT/LAUNCHER passed. Физический arm64/release UI не проверялись |
| Android Device/IPC | Явный перенос в другую колонку и возврат перед выбранной карточкой через системные select; удержание ручки/перенос между колонками и обратно; перемещение колонки с горизонтальной автопрокруткой и явный возврат. Прокрутка за текст не запустила move. Тёмная тема/relaunch/сохранённый порядок проверены; последняя сборка отдельно подтвердила Back/возврат фокуса и прямой/обратный drag |
| Android сохранность | `device-persistence-proof.json`: после закрытия финального APK копия хранилища прочитана общим Service без изменений; все 9 карточек/3 колонки, итоговый порядок/ID, текст/выполнение/теги/корзина и история совпали с исходной синтетикой |
| Очистка | `restored-proof.json`: исходные product/WebView восстановлены, одинаковые имена и SHA-256 всех 23 файлов. Синтетические база/ключи/экспорты/архивы удалены; тема ОС возвращена светлой, Device-эмулятор выключен |
| Воспроизводимость/guards | Frozen Bun install без изменений; Android/host routing guards passed. Пять посторонних dirty docs побайтово равны исходным backup; старые записи журнала/очереди сохранены. `git diff --check` проверяется перед передачей |

Результаты логов/снимки и исходный backup только в игнорируемой
`docs/reports/p05-05/`. Проверки host/native/CI не подменяют друг друга.
Продуктовые Rust-команды и существующий контракт не менялись; повтор всего
product-check для UI-only среза не выполнялся.

При проверке исправлены три собственных дефекта: очистка прежнего сообщения
в начале жеста сдвигала доску; кнопки очереди могли принять нажатие до окончания
согласования данных; новый диалог первоначально не имел штатного маркера
закрытия для native Back. Финальный APK закрыл диалог, сохранил маршрут доски
и вернул фокус. В постоянный общий сценарий добавлены отмена/проверка маршрута.
Первый повтор полного host-набора отказал на блокировке shell-fixture.exe
ещё выполнявшимся собственным Opera-прогоном; после его завершения набор
повторён успешно. Тесты/лимиты для обхода отказов не ослаблялись.

## GAP-017 — полный локальный набор Opera

```yaml
id: GAP-017
type: gap
state: resolved
priority: normal
stage: P05
blocks: []
criteria: [T08, A41]
summary: "FIX.02: headless Viz throttle и влияние synthetic fixture изолированы; полный Opera ui:check passed"
reproduction: "Запустить неизменённый checkNativeControls в headless Opera Chromium 155.0.8059.85: stableTarget timeout около 10 секунд на движущейся кнопке до продуктовых сценариев"
expected: "Один native touch после стабилизации и прохождение pending-refresh regression"
actual: "Исходный timeout воспроизведён; после host launch/процессной изоляции полный Opera/Chrome ui:check exit 0, проверки/таймауты сохранены"
next_action: "Ревью/интеграция FIX.02 вместе с текущей веткой; полный P05/T08/A41 остаётся partial"
```


### FIX.02 — причина и исправление GAP-017

Поручение разработчика от 2026-11-09: установить причину отказа и пройти весь
Opera-набор, сохранив проверки и таймауты. База `94df827`, прежняя ветка
`feat/p05-05-board-moves` с исходным dirty P05.05. Все 28 исходных изменённых/
новых файлов сохранены в игнорируемом `docs/reports/gap-017/baseline/`.
Scope исправления: конфигурация запуска host-браузера и документация;
продукт, общий nativeControls и сам native-controls fixture не меняются.

Неизменённый fixture повторно отказал. Инструментированные измерения
показали: несколько начальных кадров, затем интервалы около 1000ms, хотя
страница `visible`, элемент enabled, viewport и hit target корректны.
Движение задано **24 кадрами**, а pending refresh — ещё **12 кадрами**:
при такой подаче кадров первая анимация не заканчивается за прежние 10s.
`stableTarget` обоснованно не отправляет касание движущейся кнопке.
Ранние диагностические `samples: undefined` из LOG-438 не означают, что
предикат не исполнялся: Bun объединяет соседние const-декларации при
сериализации функции, и строковая вставка по `const previous =` не совпадала
с фактическим `, previous =`. Инструментирование по присваиванию
`stableTarget.previous` подтвердило выполнение/меняющуюся геометрию.

Контрольные fixture без драйвера также замедлялись при изменениях layout.
DOCTYPE, viewport meta, data/HTTP-навигация, настоящий loopback HTTP,
CSS transform, foreground и настройки occlusion этого не устранили.
CDP Tracing (`viz,cc,benchmark,devtools.timeline`) зарегистрировал
`SendBeginFrameDecision` с `reason: ThrottleUndrawnFrames`, `should_send: false`.
Это записи compositor о непредставленных кадрах, а не `aria-busy`, отсутствие
кнопки или дефект ZeDone. Trace содержит также служебные targets Opera;
суммарные количества отказов не приписываются исключительно странице fixture.

В `check-ui-browser.mjs` добавлен один параметр запуска всех host Chromium:
`--disable-frame-rate-limit`. Он отключает ограничитель BeginFrame
([исходник Chromium](https://chromium.googlesource.com/chromium/src/+/refs/heads/main/components/viz/common/switches.cc));
в повторной трассе `ThrottleUndrawnFrames` отсутствовал, неизменённый regression
прошёл. Это тестовая конфигурация headless scheduler, не настройка продукта
или Android WebView; частота кадров/производительность таким прогоном не принимаются.
Внутренний дефект surface presentation конкретной сборки Opera не исправляется
кодом ZeDone; исправлена доказанная причина остановки host-набора.

Сохраняются все 24/12 кадров, настоящий rAF, задержка доставки в один кадр,
геометрия/viewport на двух последовательных кадрах, visibility/enabled/hit target,
`aria-busy`, ровно одно CDP touch на действие и прежний общий бюджет 10s.
Ни повтора касаний, ни DOM click, ни подмен DOM API нет. С новым launch
отрицательный контроль, возвращающий старые координаты без ожидания, по-прежнему
отказывает: `0` clicks вместо `1`. Неизменённый устойчивый драйвер прошёл
три последовательных проверки moving target/pending refresh/one touch.

Последующий отказ Markdown также воспроизведён ограниченным экспериментом:
`checkNativeControls(browser)` → новый context → настоящий `checkCardMarkdown`
не создаёт popup; без предшествующего synthetic fixture тот же сценарий passed.
Обычное касание в предыдущем контексте с его закрытием также не мешает Markdown.
Пассивная диагностика полного набора показала trusted click, `prevented: false`,
активную user activation, отсутствие нового page target/request. Принудительный
foreground не помог, `document.hasFocus()` и до него был true.
Это наблюдаемое влияние synthetic fixture на последующий browser process,
а не доказанный дефект ссылки приложения. Конкретный внутренний механизм
блокировки в Opera не приписывается DOM/ZeDone или известному полю CDP.

Fixture теперь запускается в отдельном процессе с теми же launch options.
После его успешной проверки процесс закрывается; весь UI-набор начинается в
новом процессе. Это изоляция независимых тестов, а не retry: упавший fixture
или UI-сценарий по-прежнему останавливает набор. Код nativeControls/fixture/
Markdown-сценария и все popup/URL/no-opener assertions/таймауты прежние.
Логи контрольных вариантов: `markdown.log`, `markdown-contexts.log`,
`markdown-after-native.log`, `markdown-after-touch.log`, `markdown-foreground.log`
и `browser-diagnostic.log`. Финальный полный набор после обеих правок
host harness прошёл: `ui-full-final.log`, exit 0.

Локальные доказательства: `docs/reports/gap-017/diagnose4.log`,
`animation-matrix.log`, `fixture-matrix.log`, `flags-matrix.log`,
`trace.log`/`trace.json`, `trace-unlimited.log`/`trace-unlimited.json`,
`negative-control.log`. Первый полный `ui-full.log` прошёл moving target,
workspace/cards/editor и desktop/touch moves, затем отказал на прежнем ожидании
`popup` для Markdown-ссылки (30s); Chrome в этом запуске не начался. Полный
успех того запуска не заявляется. Отдельные настоящие Markdown-сценарии в Opera с тем же
launch, включая последовательность контекстов, прошли; полный сценарий
повторялся с пассивной диагностикой click/targets, без изменения проверок.
Логи/trace и их диагностические скрипты игнорируются Git.

Финальная команда из корня: `bun run ui:check`, с `UI_NODE_VERSION=26.8.1`
и `UI_BROWSER_PATHS` = JSON-массив полного пути Opera 139.0.6053.38/opera.exe
и Chrome/Application/chrome.exe. `ui-full-final.log`: exit **0**, последовательно
Opera Chromium **155.0.8059.85** и Chrome **156.0.8078.80**. Оба полных набора
прошли native-controls fixture, HTTP lifecycle/cards/editor/history/IME/draft/
delivery/replay/denial, mouse/touch moves/inverse, CommonMark/external link/
images/retry/UI edit/agent reverse и shell/themes/restart/access faults.
Types/Vite/fmt/Clippy -D warnings/три shell integration tests/build также passed.
`node --check scripts/check-ui-browser.mjs` и `git diff --check` passed.
`preservation-proof.json`: 23 посторонних исходных изменённых файла побайтово
прежние; общий native driver, fixture и Markdown assertions не менялись;
определения/статусы 98 критериев сохранены. Локальные тесты используют отдельную
синтетику/временные browser profiles и очищают их в finally; product storage и
Device не запускались. Новые APK/native CI не выполнялись: Android-код/драйвер
не менялись, прежние Android-доказательства этой EV сохраняют свой охват.
FIX.02 done локально; GAP-017 resolved. Полные T08/A41/P05 и ревью/интеграция
ветки по-прежнему впереди; коммит/push/PR/CI/merge не выполнялись.


GAP-016 не переоткрывается автоматически: текущее наблюдение относится к host
синтетическому fixture в Opera, не повторяет прежний потерянный native Android
reopen. Runtime драйвера/регрессия/таймауты не менялись. Полная приёмка P05/T08,
обмен P06, physical arm64 и бюджет всего приложения остаются впереди.

## Пять правил и передача

- Обратимость: новое обратное перемещение возвращает прежний порядок; отмена формы не отправляет запрос.
- Человек/агент: те же card.move/column.move и общая сохранённая очередь; агент не пишет SQLite напрямую.
- Документация: user/agent/development и существующий ADR-0005 обновлены, новая архитектура не вводится.
- Паритет: полный Chrome/Opera ui:check после FIX.02 прошёл; прежние Android Device/IPC подтверждают оба способа. Изменён только host harness; Android повторно не запускался.
- Источник истины: одна SQLite/Automerge, авторитетный порядок из API; модели Sortable и намерения доставки не являются копией данных.

Ветка остаётся отдельной. Коммит/push/PR/CI/merge не выполнялись.
P05.05 остаётся in_progress до ревью/интеграции; T08/A07/A35/A41 сохраняют
partial с добавленными доказательствами. Следующее действие — ревью и
интеграция проверенной ветки; GAP-017 resolved (FIX.02/LOG-440). P05.06 и другие новые шаги
не начинались.

## Подготовка публикации — LOG-441

Разработчик поручил перепроверить закрытый GAP-017, создать PR и при успехе
выполнить merge/удаление ветки. Прочитаны актуальные FIX.02/LOG-439–440,
полный успешный лог Opera/Chrome, trace/отрицательный контроль и фактический
host diff. Runtime nativeControls, synthetic fixture, Markdown assertions
и бюджеты не менялись. Запущен новый полный `ui:check` Opera/Chrome
(`docs/reports/p05-05/pr-ui-full.log`): exit 0, оба полных набора passed,
включая synthetic native-controls, mouse/touch move и Markdown popup.

Подготовлены реальные материалы из изолированного Git base `94df827` и
окончательного UI: одна пара PNG и одна пара MP4 на одинаковых данных/теме/
framing. Базовые assets/CSP собраны из Git archive; Rust API/Service не менялся.
Оба ролика имеют фактическое разрешение 1920×1080, длительность около 3.7s;
изображения и кадры просмотрены, переход сохранённого положения проверен API.
Медиа/исходные working backups только во внешнем временном каталоге, не Git.
Пять посторонних dirty docs и старая история не включаются в commit;
в journal/queue включается только текущая передача/зависимость P05.04,
P05.05 и FIX.02. Новый CI пока не выполнялся, приёмка не повышена.

## PR — LOG-442

[PR #82](https://github.com/Zeyafix/zedone/pull/82), head
`ce1475678d249d392d371851a838e02c88c7c0bb`, base `94df827`, опубликован
и сразу linked. Commit `feat(board): let users move cards and columns`
содержит 23 связанных paths; пять посторонних dirty docs и старые записи
журнала/очереди остаются unstaged. Runtime соответствует повторному
успешному полному Opera/Chrome `ui:check` (`pr-ui-full.log`, exit 0).

Title/body/head/base прочитаны обратно: английские Summary/UI Changes/
Changes/Testing/credit, ровно две картинки и два видео на GitHub attachments,
без локальных ссылок. Реальные материалы из base/final просмотрены,
оба MP4 1920×1080. Comments/reviews/inline comments отсутствуют.
Новый CI выполняется: 11 suites, ещё два aggregate появятся по окончании
матриц; ожидается полный набор из 13 checks. Merge/delete пока не выполнены,
P05.05 не done. По поручению разработчика после успеха exact head/readiness
выполнить merge/удаление ветки; иначе разобрать отказ, не объявляя успеха.

## Первый Android CI — GAP-018 / LOG-443

На head `ce14756` Android shell job `129256154427` failed на прежнем
`checkWorkspaceTouch:132`: после создания колонки сценарий выбирал
`.column-actions button:first-child`, который теперь открывает Move.
`named` затем ждал `input[name=name]`, отсутствующий в move-dialog, и
получил прежний timeout 10s. Это доказанное несоответствие позиционного
selector новой структуре. До нового move-сценария этот прогон не дошёл;
потери данных/ошибка API/HTTP 504 из этого отказа не следуют.

Неизменённый общий сценарий с настоящим host CDP touch воспроизвёл тот же
отказ на строке 132 (`workspace-touch-red.log`). Переименование получает
стабильный class marker `rename-column`, оба прямое/обратное переименования
выбираются по нему. В штатный host ui:check добавляется весь
`checkWorkspaceTouch` перед move-сценарием: это те же selectors/настоящие
касания и shared Service, без Vue state/DOM click/повторов. Только host
переход Back представлен Escape/историей; настоящий IME/Back остаётся Android CI.
Исходные assertions и бюджеты не ослабляются. Локальные проверки и новый
native CI ожидаются, GAP-018 пока не закрыт; merge/delete не выполнялись.

`workspace-touch-green.log` подтверждает тот же полный сценарий после
исправления: column rename/reverse/trash/restore, карточки, парная история,
выполнение/снятие, отказ пустому заголовку и durable draft passed.
Постоянный host helper использует отдельный пустой Service, как Android:
первый эксперимент с переиспользованием уже заполненной host-базы отказал
на предпосылке пустого списка (`pr82-rename-ui.log`, строка 120), затем
набор изолирован. Это исправление orchestration нового host-покрытия;
сами общие сценарии/assertions/таймауты не ослаблялись.
Финальный полный Opera/Chrome ui:check passed: `pr82-rename-ui-final.log`,
exit 0, оба прежних полных набора и общий workspace/card/editor touch
на отдельном пустом Service для каждого браузера. Guards/JS syntax/diff passed.

Видимый UI не меняется: добавлен только class без CSS-правил и исправлены
test selectors/host coverage. Материалы PR остаются применимыми; Rust/API/DDL
не менялись. Native Android-подтверждение ожидается от нового CI.

## Итог первого CI / GAP-019 — LOG-444

Все 13 результатов `ce14756`: 10 SUCCESS, Android shell FAILURE,
prototype-ui CANCELLED, Android aggregate FAILURE. Отложенное уведомление
перепроверено: aggregate `129262178491` штатно отклонил `RESULT=failure`;
самостоятельной новой ошибки его log не содержит. PR уже на `c286cc4`,
новые suites выполняются; старые failed jobs не перезапускались. Prototype-ui job
`129256154235` шёл 15:29:04–15:39:10 UTC при прежнем 10m budget:
`Check Android environment`, negative compiler/version/cleanup и disk usage
завершены SUCCESS, отменён post-step Rust cache action. Raw log подтверждает
настоящие Markdown/QR/scanner/storage/board/touch/scroll/autoscroll/IPC/HTTP/
restart проверки до отмены, но весь job не принимается как successful.
Лимит и workflow не меняются. Новый кодовый head после исправления GAP-018
должен получить весь успешный CI; при повторе budget отказа требуется
отдельная диагностика, не автоматический retry/увеличение таймаута.

GAP-018 остаётся in_progress до нового native подтверждения; GAP-019 открыт
для контроля бюджета. GAP-017 resolved сохраняется. Сохранность пяти
посторонних исходных документов проверена побайтово; старые записи журнала
и очереди остаются unstaged. Изображения/видео PR прежние: UI class marker
не имеет CSS-правил и не меняет видимый интерфейс. Новый CI ещё не запущен,
merge/delete не выполнялись.

## Rename fix опубликован — LOG-445

Commit `c286cc4aa74f2b7cc4a09ee665b26714cc9db1d0`
`test(ui): select column rename without relying on button order` опубликован
в том же [PR #82](https://github.com/Zeyafix/zedone/pull/82).
Восемь связанных paths содержат marker, два selectors, общий host helper
и evidence/docs; прежние dirty записи остаются unstaged. PR body прочитан
обратно: новый точный head, прежние четыре attachments/English credit,
локальные red→green/full suite и исходные CI failure/cancellation разделены
с pending новым CI. Одиннадцать suites IN_PROGRESS, два aggregate ожидаются.
GAP-018/019 пока не закрыты; все 13 SUCCESS и readiness обязательны перед
порученным merge/delete. Source push после этого не добавлялся.

## Первый экран c286cc4 — GAP-020 / LOG-446

Android shell job [129269999734](https://github.com/Zeyafix/zedone/actions/runs/42649010874/job/129269999734)
failed в самом первом `checkShell:78`: h2#connection-title «Доступ не настроен»
не стал видимым за прежние 10s. До rename/domain/moves этот прогон не дошёл;
подтверждения исправления GAP-018 из него не следует. Raw log сохранён
как `pr82-shell-c286cc4.log`, артефактов состояния страницы у run нет.
Причина startup отказа не доказана; HTTP 504 в логе нет.

Все 13 checks c286cc4 завершены: 10 SUCCESS, shell/Android aggregate FAILURE,
Linux CANCELLED. Отложенное уведомление дополнительно проверено по raw log
aggregate `129275165351`: единственная проверка `test "$RESULT" = success`
получила `RESULT=failure` и штатно вернула exit 1; отдельной новой ошибки
в этом job нет. Старые runs не перезапускались, текущий PR уже на dd84aac.
Prototype-ui нового head SUCCESS, но Linux job
[129269963345](https://github.com/Zeyafix/zedone/actions/runs/42649010871/job/129269963345)
15:52:26–16:02:29 UTC CANCELLED около 10m budget. Все его steps, включая
настоящий product/prototype/shell HTTP/UI и post cleanup, отмечены SUCCESS;
сырой log заканчивается штатной очисткой. Источник cancellation не доказан,
весь job не объявляется successful, GAP-019 остаётся открыт. Workflow,
лимиты и cache policy не меняются, старые CI runs не rerun.

Перед штатной очисткой Android driver теперь собирает read-only DOM/IPC
snapshot, текущий этап, последние page errors и ограниченный error log.
Проекция session допускает только status, не credentials; 1s на session,
3s на весь snapshot. Сбой диагностики не заменяет первоначальный test error.
Повторы ввода, подмена UI/ответов, увеличение исходных timeouts не добавлены.
Настоящие configured/unconfigured host Service проверяются постоянным
ui:check. Отдельный локальный Chromium-контроль подтвердил native projection,
sync/async rejection, оба deadline и closed page; synthetic key не попал
в результат. Это проверка сборщика, не новая native Android приёмка.

Полный Opera/Chrome ui:check диагностического дерева passed:
`pr82-startup-diagnostics-ui.log`, exit 0, оба полных набора, shared workspace
touch и configured/unconfigured проекция без credential key. JS syntax,
Android/host routing guards и diff check passed. Новый native APK/Device
для diagnostics-only правки не запускался; native результат нужен из CI.
GAP-017 resolved, GAP-018/019/020 открыты; merge/delete ожидают успешный
exact-head CI. Продукт, API/DDL, визуальный UI и четыре материала PR прежние.

## Startup diagnostics опубликованы — LOG-447

Commit `dd84aac7bd008ed054372770fe97f8333394fbf3`
`test(android): keep startup failure evidence before cleanup` опубликован
в прежний PR #82 и сразу linked. Семь связанных paths включают сборщик,
Android catch, host assertions и scoped docs/evidence; пять посторонних
dirty docs и прежние записи журнала/очереди сохранены вне индекса.
98 определений/статусов критериев сверены с HEAD и не менялись.

PR body прочитан обратно: exact head, OPEN/non-draft/MERGEABLE,
comments/reviews нет; английские разделы/credit, две rendered картинки
и два rendered видео сохранены. Новый CI ожидается, GAP-018/019/020
не закрыты, native startup причина пока неизвестна. Наблюдение прежнее;
merge/delete только после всех 13 SUCCESS и readiness exact head.

## Native shell passed, неполная матрица — LOG-448 / GAP-021

На exact head dd84aac уведомление о трёх SUCCESS не означает полный CI:
Android product-native/shell и Windows prototype-native — единственные
check-runs/jobs. Оба workflow [Android](https://github.com/Zeyafix/zedone/actions/runs/42671776541)
и [host](https://github.com/Zeyafix/zedone/actions/runs/42671776980)
завершены FAILURE. Jobs/attempts/check-suite APIs подтверждают лишь 2+1,
Check-run annotations/approvals/pending deployments пусты; workflow-level
annotations прочитаны далее LOG-449. Десять ожидаемых checks,
включая aggregates, отсутствуют; причина неизвестна (GAP-021).
Workflow source и матрицы относительно base не изменены.

[Native shell 129348401899](https://github.com/Zeyafix/zedone/actions/runs/42671776541/job/129348401899)
SUCCESS за 8:28. Raw log pr82-shell-dd84aac.log подтверждает первый экран,
rename/reverse и весь workspace/card/editor/history/completion/draft,
native IME/Back, moves/inverse/immutable state и Markdown/UI edit/agent
reverse; negative compiler/version/cleanup тоже passed. GAP-018 resolved
в scope selector. Прежняя startup причина c286cc4 неизвестна, хотя новый
native запуск прошёл без продуктового изменения; GAP-020 пока сохраняется.

Целый rerun обоих workflows на том же dd84aac запрошен успешно (attempt 2).
Продукт/assertions/таймауты/job budgets прежние, source push нет.
PR OPEN/non-draft/MERGEABLE/CLEAN, comments/reviews отсутствуют; merge/delete
ожидают всех 13 успешных checks нового полного attempt. GAP-019/020/021
пока открыты, GAP-017 resolved сохраняется. Старые failures/cancellations
не подменяются этим частичным успехом.

## Внешний серверный отказ Actions — LOG-449

Полный attempt 2 не остался pending: оба workflow COMPLETED/FAILURE,
jobs API total_count=0. Android 18:24:42–18:26:24 UTC, host
18:24:44–18:26:25 UTC. Исходники/assertions/лимиты не менялись,
дополнительных повторов нет; merge/delete не выполнены.

Workflow-level annotations прямо сообщают Internal server error:

| Workflow | Attempt | Correlation ID | Источник |
| --- | --- | --- | --- |
| Android | 1 | b65c5176-e7ae-4128-afd8-153599f07675 | [run](https://github.com/Zeyafix/zedone/actions/runs/42671776541/attempts/1) |
| Android | 2 | c5447217-e8f5-4610-bfea-04eff56b578c | [run](https://github.com/Zeyafix/zedone/actions/runs/42671776541/attempts/2) |
| Host | 1 | 2e2a5fc1-d422-45b8-ad75-9918550d6ad0 | [run](https://github.com/Zeyafix/zedone/actions/runs/42671776980/attempts/1) |
| Host | 2 | 21cf8551-9302-433d-92a0-52a1fef30808 | [run](https://github.com/Zeyafix/zedone/actions/runs/42671776980/attempts/2) |

GAP-021 — подтверждённый внешний blocker сервиса Actions; глубинная причина
внутри GitHub неизвестна. Три успешных runtime jobs первого attempt,
в том числе весь native shell, сохраняют доказательства; они не заменяют
весь набор из 13. PR body обновлён, передача возвращается разработчику
с выключением watch, linked PR и feature-ветка сохраняются. После
восстановления сервиса требуется полный exact-head CI перед порученным
merge/delete. Продуктовая P05.05 не done; GAP-019/020/021 ещё открыты.

unwatch_pull_request подтвердил wasWatching=true, watching=false;
PR остаётся linked/open, ветка сохранена, передача возвращена в inbox.

## Возобновление после mitigation — LOG-450

По новому поручению разработчика проверен прежний exact head dd84aac:
PR OPEN/non-draft/MERGEABLE/CLEAN, comments/reviews нет, source прежний.
[GitHub Status](https://www.githubstatus.com/api/v2/summary.json)
сообщил mitigation Actions/Checks API в 18:43:24 UTC, monitoring и
Actions operational. Полный rerun после этого принят обоими workflows.

Attempt 3 [Android](https://github.com/Zeyafix/zedone/actions/runs/42671776541/attempts/3)
и [host](https://github.com/Zeyafix/zedone/actions/runs/42671776980/attempts/3)
IN_PROGRESS, conclusion=null. Jobs APIs подтверждают все 6 Android suites,
Linux и 4 Windows suites; оба aggregates ожидаются после матриц. Старые
server failures/частичный native успех сохраняются, pending не считается
приёмкой. Код, assertions, workflow и budgets прежние, нового source push нет.
Наблюдение возобновляется; merge/delete только после всех 13 SUCCESS и
readiness exact head. GAP-019/020/021 ещё открыты, P05.05 не done.

## Linux timeout / parallel suites — LOG-451

dd84aac attempt 3: все 13 checks завершены — 12 SUCCESS и Linux CANCELLED.
Весь Android, обе ABI, shell IPC/IME/Back/moves/Markdown и все Windows
suites/aggregate прошли. GAP-021 resolved: полная матрица восстановлена.
[Linux annotation](https://github.com/Zeyafix/zedone/actions/runs/42671776980/attempts/3)
явно сообщает превышение 10m; job 129370060660 шёл 19:02:49–19:13:05 UTC.
Подготовка 1:18, product 3:47, prototype 2:31 passed, shell отменён
после 2:34. Лог pr82-linux-dd84aac-attempt3.log показывает editor/moves
успех до отмены, но не весь shell/Markdown/shared touch; он не принимается.

Linux разделён на те же 4 host suites, которые уже использует Windows.
Required check (ubuntu-24.04) сохранён как независимый strict aggregate.
Все команды/guards/assertions, сценарные и job budgets (Linux 10m,
Windows 20m, aggregates 1m), triggers/permissions/concurrency сохранены.
Android workflow и продуктовый код не изменяются. Теперь ожидаются 17 checks:
14 runtime suites и 3 aggregates. Read-only cache fallback распространён
на Linux с прежними legacy paths/порядком и отдельным platform prefix;
suite namespaces/main-only save прежние, all compatibility сохранена.
Linux main cache по API существует (1 092 562 747 bytes); реальный
restore/save/время нового head ещё не доказаны, runner-минуты могут вырасти.

Локально passed: постоянные host/Android guards; реальное PowerShell
выполнение setup для 10 OS/suite комбинаций и отказы invalid/rustup;
реальные bash aggregates для success/failure/cancelled/skipped/missing;
отрицательные контроли удаления suite и слабого gate; actionlint workflow
и composite; syntax/diff. Проверено совпадение Windows jobs, triggers,
permissions/concurrency и неизменённой подготовки вне cache migration.
Весь product/UI набор локально не повторялся для orchestration-only diff;
весь exact-head native/UI/ABI CI нужен до merge/delete. Новый успех и
ускорение не заявляются; GAP-019/020 открыты, полные критерии partial.
