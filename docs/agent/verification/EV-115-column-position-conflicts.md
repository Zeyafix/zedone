# EV-115 — P06.06.2: конфликт позиции колонки

## Карточка

- База: main/806d8e6f7b90a1aeb464bd122205d7757cc316c9; ветка
  `feat/p06-06-column-position-conflicts`. Четыре исходных dirty документа
  интеграции P06.06.1 сохранены побайтово в ignored baseline.
- Цель: явный выбор сохранённой позиции колонки через общий UI/HTTP/IPC/CLI,
  с доступными карточками и сохранением независимых решений.
- Требования: T09/A35/S02/S08, product-spec §1/§1.2, ADR-0045/0068.
  Зависимость P06.06.1 done в базе, PR #106, 17 SUCCESS; реализация проверена
  чтением общего Service, register и интерфейса.
- Объём: позиции колонки, авторитетные подписи соседей, общий контракт,
  форма выбора, проверки доступа/сбоев/двух реплик/Windows/Android и docs.
  Совместная приёмка P06.06 и GAP-028/031 — отдельно.
- Обратимость: отмена без записи; после решения новая обычная column.move.
  Собственные конфликты имени/удаления и карточек не снимаются.
- Проверки: Rust storage/adapters (HTTP/IPC/CLI), контракт/клиент, полный UI
  desktop/touch, native Android, обе APK ABI и Device UI.
- Размер: один готовый сценарий; ядро/API/форма и его приёмка в одной ветке,
  генерируемые артефакты отдельно от исходников. Делегирование не поручалось.
  Ручной diff превышает ориентир 300 строк из-за неделимого общего контракта,
  формы и матрицы прав/сбоев/двух реплик/двух платформ одного выбора; карточки
  и совместная матрица уже выделены отдельно. Проверки не сокращаются ради размера.
- Поручение: реализация и локальная приёмка; публикация отдельно.

## Результат

P06.06.2 verified локально в указанной ветке от main/806d8e6. Выбор позиции
колонки через общий UI/HTTP/IPC/CLI, независимые решения, одинаковые соседи,
работа с карточками и обратимость подтверждены. Windows 385/native Android 382
Rust, полный Chromium/Opera desktop/touch, обе production APK ABI и настоящий
Windows Credential Manager ↔ APK/Keystore/TLS/API/CLI/Device/restart passed.

Исходные APK/каждый app file восстановлены по SHA-256; OS keys/rules очищены,
production x86 artifact восстановлен по SHA-256, свой пустой UI-каталог удалён
только rmdir. Device выключен штатным инструментом. Все 98 определения/состояния
и прежние GAP/DEC сохранены; полный P06.06/T09, GAP-028/031 отдельно.
Commit/push/PR/новый CI/merge не выполнялись.

## Технические находки и попытки

- Используется прежний scalar `position`; delete/put обеспечивают новое
  причинное решение даже при выборе наблюдаемого победителя. Новая схема,
  протокол или зависимости не вводятся.
- UI ранее дополнительно запрещал содержимое при нескольких positions колонки.
  Теперь доступность содержимого определяется её lifecycle и собственными
  конфликтами карточек; запрет обычного изменения самой колонки сохраняется.
- Первоначальная компиляция новых storage-тестов выявила неверное размещение
  модуля относительно закрытых helpers; модуль перенесён внутрь columns::tests.
  В адаптерном тесте исправлен явный path модуля. Продукт ради этого не менялся.
- Первый storage-прогон использовал повторный ID чтения в test helper и получал
  исторический snapshot вместо свежего; helper теперь читает актуальный snapshot.
  Проверка SQL rollback читает snapshot без receipt до удаления injected trigger.
  Три сценария после исправления passed; затем все пять финальных сценариев
  прошли отдельно и в полном прогоне. Бюджеты/assertions/правила продукта не ослаблялись.
- Windows общий API passed: чтение с Read, отказ записи Read, неправильная
  позиция/лишнее поле/stale, выбор ReadWrite, обратная move, точный replay,
  command_mismatch, отзыв перед replay, настоящий CLI после закрытия базы.
- Первый полный Windows product runner остановился в добавленной lifecycle
  фикстуре: присваивание прежнего `active` было noop и не создало ожидаемую
  local change. Фикстура теперь явно делает delete/put для настоящих независимых
  вариантов. Это не успешный полный runner и не изменение правил продукта.
- Первый полный UI runner прошёл прежнюю матрицу и остановился на неоднозначном
  локаторе нового теста: `data-column` есть у section и вложенного списка.
  Локатор уточнён до section; бюджеты/проверки сохранены. Успех полного UI
  до финального повтора не заявляется.
- При проверке обратимости выявлен прежний общий запрет `card.restore` в
  колонку со спорной позицией. В scope .2 убрано только ограничение позиции
  в core и UI списка восстановления; имя/lifecycle/версия/права сохраняются.
  API/browser/APK сценарии дополнены полным циклом trash/restore.
- Два локальных focused UI прогона по ошибке запускали прежний fixture из
  корневого target, тогда как прямая Cargo-сборка создала новый в отдельном
  target shell workspace. Разные embedded/disk SHA-256 и исходное время binary
  подтвердили устаревший стенд. Один повтор неверной сборки остановлен вместе
  со своими дочерними процессами; путь CARGO_TARGET_DIR исправлен. Эти прогоны
  не свидетельствуют о поведении актуального UI. Финальная проверка выполнена
  на явно построенном актуальном binary; продуктовый source/бюджеты не менялись.


## Итоговая локальная матрица — LOG-619

| Проверка | Фактический результат |
| --- | --- |
| Windows product | Финальный полный product runner exit 0: 385 Rust (18 core, 7 contract, 284 storage, 76 adapters), 27 client/280 assertions, 28 comment-policy tests, fmt/Clippy -D warnings, release всех workspace targets/features, 7 unsafe guards и new-source comment guard, точные OpenAPI/TS/standalone validator и 11 отрицательных type assertions. После уточнения card.restore отдельно повторены финальные Clippy и все 385 Rust; оба прошли. |
| Storage/API | Все пять новых storage-сценариев: четыре сочетания одинаковых/разных решений, блокировки, изменения карточки/stale, SQL rollback, сохранённый конфликт имени, корзина/lifecycle родителя, неоднозначные соседи, неизменность выбранного ранга при правке соседа, одинаковые соседи/повторы/restart. Общий HTTP/IPC/настоящий CLI прошёл Read/ReadWrite/отзыв/replay/mismatch/invalid/stale и child trash/restore. |
| Native Android x86_64 | Полный штатный runner exit 0: все 382 Rust (18 core, 7 contract, 284 storage, 73 adapters), release/7 unsafe guards/new-source comment guard, настоящий CLI, обе checkChannelPair TLS/pin/persisted identity/pending/restart/replay/cancel/no-access и точное совпадение export-contract. Remote sandbox удалён finally. |
| Android arm64 | Штатный product --android-arm64 exit 0: release всех workspace targets/features, 7 unsafe guards/new-source comment guard и клиент/контракт/type assertions. Это сборка и guards, не runtime физического arm64. |
| Общий UI | Финальный полный ui:check exit 0: scanner state machine, vue-tsc/Vite/shell checks, весь прежний bundled Chromium desktop/touch и pairing UI. Отдельный полный Opera browser runner exit 0; новые desktop/touch случаи также отдельно прошли в установленном Chrome. Проверены отдельная область колонки, доступная карточка/выполнение/trash/restore, блокировка самой колонки, две radio/нет default, отмена/фокус, stale с сохранённым намерением, явный выбор, независимая правка соседа, настоящая обратная move и reload. |
| Production APK | Обычные production debug APK x86_64/arm64 и AndroidTest/manifest guards passed; x86 повторён на финальном source и сохранён до отдельной acceptance сборки. Финальный production x86 artifact восстановлен по SHA-256 после приёмки. |
| Настоящий парный APK | Финальный штатный runner exit 0, status=passed/restored=true/cleanup=true. Windows Credential Manager ↔ APK/Keystore/TLS/API/CLI: разные и одинаковые независимые решения колонки, разные колонки между одинаковыми соседями, повторы, сохранённые позиции/содержимое/история, child edit/create/trash/restore/move и обратная column.move passed. Прежние pairing/QR/field/card-placement/recovery/права/отзыв/restart также прошли. |
| Device UI | Через текущую device_open команду: колонка с карточкой в «Конфликтах», доступный просмотр/редактирование/выполнение карточки, две неподтверждённые позиции, disabled submit без выбора, отмена/повторное открытие, явный выбор перед Соседом А. После process relaunch колонка одна в обычном порядке перед Соседом А, сохранённые описание/выполнение карточки и обычные перемещения доступны. Тёмная тема, настоящий Android IPC; snapshot/screenshot сохранены. |
| Сохранность | Исходный APK и каждый app file восстановлены по SHA-256; свои OS slots/rules очищены, свой пустой UI-каталог удалён точным rmdir, compiled production artifact восстановлен по SHA-256. Физический телефон не использовался. Device выключен инструментом Device; своих фоновых проверок не осталось. |

Весь исходный scope и генерируемые артефакты проверены; 24 source fingerprints
не менялись после финальной проверки. Все локальные пути, raw logs, hashes,
APK и снимки остаются только в ignored reports. Node 26.8.1 — прежний локальный
UI override; baseline 24.22.1 не менялся. Прежний Bun launcher передаёт явные
пути тех же test files; состав/бюджеты сохраняются.

В начале Device-проверки snapshot после навигации ещё не перечислял видимые
элементы доски. Первую карточку открыли координатой из свежего device_screenshot;
затем использовали доступные свежие refs. Видимое состояние, нативное выполнение,
отмена, выбор и relaunch проверены до записи ui-continue=passed. Ошибок команд
Device и ослабления проверок не было. Пустая status-область «Конфликты» после
решения не принимается за наличие спорной колонки: объект находится в обычной
области и имеет единственное положение.

## Пять правил и передача

- Обратимость: отмена без доменной записи, обычная column.move после решения,
  child trash/restore/move, неизменные ID/карточки/история.
- Человек/агент: один Service/ACL/replay и контракт UI/HTTP/IPC/CLI, в том числе
  настоящие TLS/CLI вызовы к Windows и APK; отдельного обхода для агента нет.
- Документация: user columns/moving, agent columns/column-positions/cards,
  development product-core, ADR-0069, этот EV, журнал и очередь обновлены.
- Windows/Android: обе runtime-проверки и Device UI в указанном объёме, обе
  production APK ABI; физический arm64/полный T09 не заявляются.
- Источник истины: прежний Automerge position и SQLite transaction/receipt;
  ядро задаёт подпись и порядок, UI не вычисляет собственные ранги.

Active_work null после LOG-619. Далее рассмотрение/интеграция P06.06.2;
совместная приёмка P06.06 готовится после её интеграции. GAP-028/031 и прочие
GAP/DEC прежние, все 98 определения/состояния сохранены. Полный T09, настоящий
Tailscale, целевой телефон и измерения отдельно. Публикация не выполнялась.


## Подготовка публикации — LOG-620

Разработчик поручил PR и при успехе merge/delete. После fetch base origin/main
остаётся 806d8e6; локальные результаты LOG-619 сохраняют свой охват. В состав
входят связанные исходные записи интеграции P06.06.1. Реальные Before/After
сняты на одинаковом синтетическом графе и viewport/theme/framing 1280×2300
и проверены визуально; финальные host binaries восстановлены по SHA-256, медиа
и body вне Git. Новый CI ещё не запускался, до merge нужны все expected SUCCESS
и свежая readiness точных head/base. Состояния критериев и GAP/DEC прежние.


## Публикация — LOG-621

[PR #107](https://github.com/Zeyafix/zedone/pull/107), head
4e8a03aa0313178b7dfed31b38e4841572371951, base main/806d8e6. 37 файлов,
обычные commit/push/pre-push passed, noreply и состав проверены. Title/body/credit
прочитаны обратно; два hosted PNG 1280×2300 совпали с реальными Before/After
по SHA-256, медиа/body вне Git. Все 24 source fingerprints прежние.

Новый CI выполняется: 14 suites running, три aggregate ещё ожидаются. Initial
readiness OPEN/non-draft/MERGEABLE/UNSTABLE, feedback отсутствует. Это не
финальный успех; до порученных merge/delete нужны все 17 expected SUCCESS
и свежая readiness exact head/base. Все определения/состояния и GAP/DEC прежние.
Checkpoint локальный без дополнительного commit/push. Совместный P06.06/T09
и GAP-028/031 отдельно.


## Первый CI отказ — LOG-622 / GAP-032

Android shell run 44153619224/job 134288323820/head 4e8a03a attempt 1 failed:
старый checkWorkspaceTouch после column trash/restore/reload получил
localStorage.length=1 вместо 0. До этого Keystore, recreated Activity/WebView
и unconfigured shell passed. Page ready/bridge/connection/busy=false, без
pageErrors; тип записи в лог не попал. Причина не доказана и не смешивается
с GAP-028 либо новой продуктовой логикой. Локальная матрица выше сохраняет
свой scope; CI не пройден.

Один bounded rerun точного head выбран без изменения guards/бюджетов.
GitHub пока не принял его: workflow ещё выполняется, HTTP 403 «The workflow
run containing this job is already running». Повтор ещё не запущен. После
завершения workflow выполнить один штатный повтор, при воспроизведении —
безопасно установить категорию записи и исправить доказанную причину.
Source, локальная приёмка и все 98 определения/состояния прежние;
до всех 17 SUCCESS и свежей readiness merge/delete не выполняются.


## Принятый повтор — LOG-623

После завершения Android workflow 44153619224 failure/attempt 1 GitHub принял
один штатный rerun отказавшего shell job на том же head 4e8a03a. API сразу
вернул queued (счётчик attempt ещё не обновился); успешный результат не заявляется.
Source/assertions/бюджеты прежние, все 24 fingerprints совпали, новых commit/push
нет. GAP-032 и исходный отказ сохраняются. При повторном отказе — безопасная
диагностика категории записи; merge/delete только после всех 17 SUCCESS/fresh
readiness. Уже включённый watch продолжается.


## Отложенный aggregate wake — LOG-624

Android aggregate/job 134295483540 корректно передал failure первой попытки:
RESULT=failure в 19:10:09 UTC. Текущий attempt 2 уже выполняет shell/job
134296548843 (старт 19:11:45 UTC) на том же head; другие пять Android suites
унаследованы SUCCESS. Это не новый отказ shell, код/guards не меняются,
нового rerun нет. До завершения всех 17 expected SUCCESS и свежей readiness
интеграция не выполняется; GAP-032 и исходный отказ сохраняются.


## Подтверждённый CI timeout и CI.11 — LOG-625/626

Единственный повтор attempt 2/job 134296548843/head 4e8a03a CANCELLED:
annotation подтверждает 15m0s. Исходный workspace/localStorage guard прошёл,
затем обычный UI, весь paired APK/column-position сценарий и restoration/OS
cleanup прошли; job отменён на завершении до полного negative preflight.
Aggregate/job 134306285986 корректно failed. Частичный лог не считается CI.
GAP-032 не воспроизведён, причина неизвестна; guard не ослабляется.

CI.11/EV-116 выделяет shell UI и pairing APK в два обязательных jobs с теми же
15m, полными scripts/assertions/negative cleanup и strict aggregate. Fresh
pairing runner получает production APK перед acceptance build; пустой исходный
manifest теперь допустим с прежним точным SHA-256 восстановлением, malformed
manifest запрещён. Это единственный тестовый source delta из 24 fingerprints;
продуктовые API/runtime/UI и остальные 23 fingerprints прежние. Routing,
bootstrap/manifest/failure/signal/host guards/comment policy/actionlint passed
локально, полный новый CI из 18 expected checks ожидается. Полная локальная
приёмка продукта выше сохраняет свой scope, медиа остаются актуальны.
Все 98 определения/состояния сохранены; merge/delete только после полного
нового CI и fresh readiness, GAP-033 до этого in_progress.
