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
