# EV-117 — P06.06: совместная приёмка мест карточек и колонок

## Карточка

- База: main/531cf34aefe019669b924900fae0366e79ab6611; ветка
  `test/p06-06-position-conflicts-acceptance`. Четыре исходных dirty документа
  интеграции P06.06.2/CI.11 сохранены по SHA-256 в ignored baseline.
- Зависимости: P06.06.1 done (EV-114/PR #106/806d8e6), P06.06.2 done
  (EV-115/PR #107/531cf34); код обоих сценариев доступен в базе.
- Цель: проверить совместные конфликты позиции колонки и целого места карточки
  при обоих порядках решений, включая одну и разные колонки назначения.
- Требования: product-spec §1/§1.2.1–2/4/8, T09/A35/A38/S01/S02/S08,
  ADR-0045/0068/0069. Полные определения и состояния критериев сохраняются.
- Объём: синтетический общий fixture, постоянная UI/API/CLI/paired APK
  матрица, одинаковые соседи, независимые одинаковые/разные решения,
  доставка/повторы, ID/теги/выполнение/история, доступность содержимого.
- Отказы/обратимость: отмена без записи; собственная блокировка карточки
  сохраняется после решения колонки, обычные move и child trash/restore после
  допустимого решения доступны. Старые версии не подменяются автоматически.
- Проверки: Windows product, полный Chromium/Opera desktop/touch, native
  Android product, обе production APK ABI, настоящий Windows Credential Manager
  ↔ APK/Keystore/TLS/API/CLI, Device UI/restart и SHA-256 restoration/cleanup.
- Граница: один приёмочный сценарий. Новые продуктовые команды, схема,
  протокол, прочие типы конфликтов и полный T09 не входят в этот шаг.
  Независимый существенный дефект оформляется отдельно.
- Размер: общий fixture и постоянные сценарии двух платформ могут превысить
  ориентир 300 строк: это одна матрица совместимости уже принятых операций.
- Поручение: продолжить в отдельной ветке с локальной приёмкой; публикация
  отдельно. Делегирование не поручалось.

## Результат

P06.06 done: [PR #108](https://github.com/Zeyafix/zedone/pull/108), head 5ca0ee8,
merge 693b1fb после всех 18 SUCCESS в Windows/Linux/Android attempt 1 и свежей
CLEAN/MERGEABLE readiness без feedback. Merge tree равен tested head, main
обновлена, feature-ветка удалена remote/local/tracking. Все 98 определения/
состояния и GAP/DEC сохранены; полный T09 отдельно. Ниже локальная история.


P06.06 verified локально в `test/p06-06-position-conflicts-acceptance` от
main/531cf34. Совместные места карточек и колонок, оба порядка решения,
независимые одинаковые/разные решения, одинаковые соседи и доступность
содержимого подтверждены на Windows/Android через общий UI/HTTP/IPC/CLI.

Windows 385/native Android 382 Rust, полный Chromium/Opera desktop/touch,
обе production APK ABI, отдельные настоящие paired `--standard` и `--positions`
сессии с Windows Credential Manager/APK/Keystore/TLS/CLI и Device/restart passed.
Оба финальных runner exit 0, status=passed/restored=true/cleanup=true.

Все четыре исходных APK/file manifests между попытками одинаковы, каждый APK
и app file восстановлены по SHA-256; четыре собственных Windows/Android
namespace очищены, собственные пустые UI-каталоги удалены только rmdir.
Compiled production x86 APK восстановлен по SHA-256, Device выключен штатно.
Все 98 определения/состояния и GAP/DEC прежние. Commit/push/PR/новый CI/merge
не выполнялись. Полный T09, GAP-028/031/032 и физический arm64/Tailscale/измерения
не закрываются этим срезом.

## Попытки и границы

- Первый focused browser запуск выполнен до окончания сборки fixture и увидел
  старый binary без нового режима. После завершения явной сборки запущен новый
  стенд; этот ранний отказ не свидетельствует о поведении нового сценария.
- Первая локальная копия product runner слишком широко заменила относительные
  imports и затронула строку генерируемого validatorTypes. Контракт не менялся;
  launcher исправлен, исходная строка `./schema` восстановлена. Первый Android
  запуск остановился на этом сравнении, а первый host завершил Rust/client и
  guards до того же отказа; они не объявляются полными успешными прогонами.
- Новый UI тест сначала ожидал возврата фокуса строго на кнопку выбора карточки.
  Реализация `focusCard` возвращает его на доступную карточку, выбирая первый
  подходящий элемент `.card-title`/`.resolve-card-placement` в DOM. Дополнительное
  ожидание подтвердило расхождение ожидания теста; исправлено ожидание допустимого
  действия той же карточки. Для колонки остаётся точный opener. Продуктовый код,
  прежние проверки и бюджеты не менялись.
- Первый полный UI runner встретил Windows file lock: отдельный focused fixture
  ещё работал при Cargo test/rebuild `shell-fixture.exe`. После его штатного
  завершения запущен отдельный полный повтор; исходный runner не успешен.
- Финальный focused Chromium runner прошёл все восемь случаев desktop/light и
  touch/dark × одна/разные колонки × карточка/колонка первой. Полный UI/Android/
  paired runner пока ожидаются. Локальный Node 26.8.1 override и абсолютные пути
  тех же Bun test files сохраняют прежний объём; baseline Node не менялся.

Raw logs, baseline, APK и данные конкретной машины хранятся только в ignored
reports. GAP-028/031/032 не считаются исправленными успешными локальными случаями.


- Первый полный browser-повтор прошёл прежнюю матрицу и первые два новых
  случая, затем новая проверка увидела старую группу колонки между закрытием
  диалога и завершением `props.refresh`. Приёмочный тест теперь ждёт доступную
  обычную move того же объекта перед проверкой обновлённой группы. Это ожидание
  фактического UI-состояния, без sleep/увеличения timeout/изменения продукта.
- Первый нативный Android product-повтор прошёл все 382 Rust, release/guards
  x86_64 и клиент/контракт, но `checkChannelPair` остановился на ADB shell client
  probe с exitCode=null при прежнем timeout 15000. Вывод client/server исходный
  helper не сохранил, причина неизвестна; полный runner не успешен. Его finally
  удалил собственный remote sandbox. После сборок запланирован один повтор
  прежнего полного runner с теми же лимитами и read-only локальной записью
  exit/signal/output для probes. Связь с другими GAP не предполагается.


## Матрица проверок

| Проверка | Фактический результат |
| --- | --- |
| Windows product | Финальный полный runner exit 0: 385 Rust (18 core, 7 contract, 284 storage, 76 adapters), 27 client/280 assertions, 28 comment-policy tests, fmt/Clippy -D warnings, release всех workspace targets/features, 7 unsafe guards и new-source comment guard, точные OpenAPI/TS/standalone validator и 11 отрицательных type assertions. |
| Общий UI | Финальный полный `ui:check` exit 0: scanner tests, vue-tsc/Vite, shell fmt/Clippy/3 tests, bundled Chromium desktop/touch и pairing UI. Все восемь новых совместных случаев прошли после ожидания фактического обновления UI. |
| Production APK | Обе обычные production debug ABI x86_64/arm64, AndroidTest и manifest guards passed. Production x86 APK сохранён по SHA-256 перед отдельной acceptance сборкой; физический arm64 runtime не заявляется. |
| Native Android | Финальный bounded полный `--android` runner exit 0: все 382 Rust (18 core, 7 contract, 284 storage, 73 adapters), x86_64/arm64 release всех targets/features, по 7 unsafe guards/new-source comment guard, настоящий CLI, точное совпадение export-contract. Обе checkChannelPair TLS/pending probes passed с прежними лимитами; server/client exit 0, remote sandbox удалён finally. Исходная причина прежнего ADB отказа не установлена и исправленной не объявляется. |
| Opera | Финальный полный browser runner exit 0: весь прежний desktop/touch и все восемь совместных случаев. Первый runner использовал раннюю версию теста без ожидания refresh и отказал на том же UI-readiness ожидании после пяти новых passed случаев; его отказ сохранён отдельно. |
| Настоящий paired APK/Device | Финальная `--positions` сессия exit 0, status=passed/restored=true/cleanup=true: все четыре сочетания same/cross-column × card/column first, независимые одинаковые/разные решения, stale по child edit, блокировки, одинаковые соседи обеих иерархий, ID/теги/выполнение/история, повторы, обратные move/trash/restore, TLS/CLI/Keystore restart/recovery/права/отзыв. Device подтвердил собственную блокировку, два неподтверждённых варианта, отмену, выбор карточки в ещё спорную колонку, доступное выполнение, отдельный выбор колонки и сохранённые обычный порядок/текст/выполнение после relaunch. Финальная `--standard` сессия также exit 0/status=passed/restored=true/cleanup=true: вся прежняя матрица text/field/placement/column positions и полный общий lifecycle сохранены. Default orchestration проверена постоянными guards; две реальные сессии запускались отдельно, общий default runner повторно не запускался. |

Все текущие успешные результаты относятся к source в этой ветке; публикации и
нового CI нет. Одинаковые соседи/перестановки доставки/restart также проверяются
прежними storage-сценариями, включёнными в полный продуктовый runner; тесты
совместимости не заменяют эти проверки упрощённым алгоритмом порядка в UI.


- Первый общий настоящий APK runner прошёл прежнюю матрицу и три совместных
  случая. На четвёртом Windows `acceptance_server` завершился по прежнему
  600s fixture timeout; следующая команда получила unavailable. Это доказанный
  лимит стенда, не провал продуктового утверждения. Перед assert timeout сервер
  закрыл listener, удалил все четыре своих Windows slots и закрыл storage;
  отсутствие своего namespace в Credential Manager проверено отдельно.
  Android finally подтвердил восстановление APK/каждого файла по SHA-256 и
  выполнил Keystore/transport cleanup. Полного успеха и Device checkpoint нет.
- Runner разделён на независимые `--standard` и `--positions` с обычной полной
  подготовкой/общим lifecycle/cleanup каждой сессии. Default entrypoint выполняет
  обе последовательно и требует оба exit 0 без signal. Прежние сценарии не
  удаляются, совместная матрица не сокращается. Лимиты fixture 600s локально с
  прежним P06_EXCHANGE_UI / 240s CI, ручные 300s и pairing job 15m сохраняются.
  Новый CI ещё не выполнен; cold CI runtime не гарантируется локальным прогоном.


- Первая изолированная `--positions` сессия прошла все четыре совместных
  случая и recovery/restart/права/обратимость. Device подтвердил блокировку
  спорной карточки, независимые группы, два неподтверждённых варианта/disabled
  submit, отмену и выбор места в спорную колонку; затем отдельный явный выбор
  колонки. Перезапуск достиг списка пространств, но доска после restart и
  обычное действие не проверены до прежних 300s. Checkpoint не принят;
  finally подтвердил restoration SHA-256 и завершил OS cleanup без ошибки.
  Это не успешный полный runner. Следующий повтор — тот же лимит/полный scope.
- Device дал `Missing target` из-за незакавыченного @ref в PowerShell:
  после кавычек единственный повтор прошёл. Для ref из settle diff потребовался
  complete snapshot; свежий snapshot и единственный повтор прошли. Скриншоты
  использованы для отсутствующих AX ссылок, действия выполнялись только
  текущей agent-device командой. UI через adb не использовался.
- После исходного server timeout проверено отсутствие своего Windows namespace
  в Credential Manager и своего Android namespace directory. APK cleanup удаляет
  этот каталог только после удаления и проверки отсутствия всех четырёх slots;
  обычный restoration удаляет только файлы, поэтому отсутствие подтверждает
  завершение прежней Keystore cleanup. Новые данные/чужие ключи не затрагивались.


- Финальная `--positions` сессия прошла полный API scope и Device до исходного
  deadline. Локальный driver выполнил начало сценария, но остановился на своём
  ожидании: компактный snapshot скрыл видимые карточки как helper nodes. Его
  exit 1 не считается успехом. Полный raw snapshot и дальнейшая ручная проверка
  выполнили оставшиеся действия той же выданной agent-device командой.
- Raw ref получил «off-screen and not safe to press»; выполнена рекомендованная
  прокрутка. Затем selector с пробелами потерял кавычки при передаче Windows
  .cmd и был отклонён до касания. Свежий полный ref после прокрутки успешно
  открыл нужную карточку. Ошибки команд не обходились через adb/иной session.
- До принятия checkpoint вручную подтверждены: обычное редактирование доступно
  после решения карточки при спорной колонке, выполнение сохраняется; после
  отдельного выбора колонки и process relaunch она находится в обычных колонках
  перед Соседом А, карточка одна с прежним текстом и выполнением, действия
  редактирования/истории/обратного выполнения доступны. Скриншот и snapshot
  получены до `ui-continue=passed`. Финальный runner exit 0, restoration/cleanup.
- Постоянные `check-android-ci-suites.mjs` guards теперь проверяют обе дочерние
  сессии, неизменность presence-based fixture бюджета, владение UI checkpoint,
  explicit/invalid args и nonzero/null/signal каждого дочернего шага. Полный
  прежний routing/ABI/bootstrap/manifest/15m/strict aggregate guards runner passed.


## Пять правил и границы

- Обратимость: отмена без записи, новые обычные card.move/column.move после
  решения, child trash/restore, выполнение/обратное выполнение. Выбор одного
  объекта не снимает чужие собственные конфликты и не меняет его историю.
- Человек/агент: тот же Service/ACL/replay/общий Rust-контракт UI/HTTP/IPC/CLI;
  настоящие TLS/CLI вызовы к Windows/APK и Device используют обычные операции.
- Документация: user moving, agent placement-conflicts/column-positions,
  development product-core, карточка/журнал/очередь. В ADR-0069 исправлен
  устаревший статус интеграции .2; нового архитектурного решения нет.
- Паритет: Windows/native Android, полный Chromium/Opera desktop/touch,
  настоящий APK/Keystore и Device UI/restart, обе production APK ABI. Физический
  arm64 runtime, настоящий Tailscale, измерения и полный T09 не заявляются.
- Источник истины: прежние Automerge registers и SQLite/receipt; общий Rank/ID
  задаёт порядок и варианты. UI и агент не вычисляют независимые ранги.

Производственные API/runtime/UI/схема/протокол и зависимости не менялись.
Изменения ограничены fixture/приёмочными runners, их guards и документацией.
Исходные четыре локальных документа сохранены, EV-115/116 побайтово прежние;
все 98 определения/состояния и прежние GAP/DEC сохраняются. Сам факт локальной
приёмки не означает прохождение полного T09 или нового CI. Публикация отдельно.


## Итог и передача — LOG-632

- Обе финальные изолированные paired сессии прошли в прежних fixture бюджетах;
  Device checkpoint принят после фактического restart/сохранённого результата,
  до прежних 300s. Локальные raw logs/снимки/manifest/все попытки сохранены только
  в ignored reports. Исходная причина первого native ADB отказа неизвестна;
  успешный повтор не объявляется её исправлением.
- Финальная очистка дополнительно проверила отсутствие всех четырёх собственных
  Windows namespaces в Credential Manager и Android namespace directories;
  удалены только два собственных пустых UI-каталога (включая пустые вложенные).
  Original file manifests всех четырёх запусков идентичны; production APK
  artifact восстановлен по SHA-256. Device shutdown выполнен инструментом Device.
- Семь отпечатков затронутых source/guards сохранены; API/runtime/UI исходники
  совпадают с базой. Source-comment policy, CI routing guards и whitespace
  проверены на финальном source. Последние изменения test orchestration не
  требуют повторного Rust/UI product прогона: соответствующие исходники прежние.
- Пять правил подтверждены в границах этой локальной приёмки. Все 98 определения/
  состояния сохранены; затронутые критерии получили только ссылку на EV-117.
  P06.06 verified, active_work null. Далее рассмотрение/интеграция этого среза;
  до интеграции нужен новый полный CI в прежних бюджетах. P06.07 не начат.


## Подготовка публикации — LOG-633

Разработчик поручил PR и при успехе merge/delete. После fetch origin/main
остаётся 531cf34, семь source fingerprints и вся локальная приёмка прежние.
Включены связанные исходные итоги интеграции P06.06.2/CI.11. Визуального
продуктового изменения нет, Before/After неприменимы. Новый CI не запущен;
до merge/delete нужны все 18 expected SUCCESS и свежая readiness точных
head/base без замечаний. Определения/состояния всех 98 критериев и GAP/DEC
сохранены; это не закрывает полный T09 и не гарантирует cold CI runtime.


## Публикация — LOG-634

[PR #108](https://github.com/Zeyafix/zedone/pull/108), head
5ca0ee8ebdc291f4414e5a6b6c2a863f949cd69b, base main/531cf34. Все семь source
fingerprints прежние; 17 связанных source/docs файлов включены после scope/
privacy/98 criteria/noreply проверки, обычный commit/push/pre-push passed.
Title/body/sections/credit прочитаны обратно, визуального изменения нет.

15 suites IN_PROGRESS, три strict aggregates ещё ожидаются; весь expected
набор 18 подтверждён по PR #107 при неизменных workflows. Начальная readiness
MERGEABLE/UNSTABLE, reviews/requests/comments/threads отсутствуют. Локальная
приёмка выше сохраняет охват, это не полный новый CI и не закрытие T09/GAP.
До merge/delete нужны все 18 SUCCESS и fresh readiness точных head/base.
Active_work — штатный watch/условная интеграция. Checkpoint локальный, tested
head не меняется дополнительным commit/push.


## Финальный CI и интеграция — LOG-635/636

- Все 18 expected checks на head 5ca0ee8 COMPLETED/SUCCESS: [Windows/Linux](https://github.com/Zeyafix/zedone/actions/runs/44257694922)
  и [Android](https://github.com/Zeyafix/zedone/actions/runs/44257695082), обе
  attempt 1 без повторов. Raw Windows/Linux shell logs подтверждают по восемь
  новых combined desktop/touch случаев. Android pairing выполнил default
  orchestration обеих suites, все четыре combined APK случая, два restoration
  SHA-256 и два OS cleanup; environment/negative cleanup и strict aggregates passed.
- Android shell 11:34, pairing 14:53 при прежних 15m. Фактический запас pairing
  семь секунд; этот cache-hit run не гарантирует будущий/cold runtime. Проверки
  и бюджеты не ослаблялись, GAP-028/031/032 исходными причинами не закрываются.
- Testing обновлён фактическими результатами и прочитан обратно вместе с
  title/sections/credit. Свежая exact-head/base readiness 5ca0ee8/531cf34:
  OPEN/non-draft/MERGEABLE/CLEAN, все 18 SUCCESS, reviews/requests/comments/
  inline threads отсутствуют. Нового product source delta после проверок нет.
- Merge API закрепил head 5ca0ee8 и вернул merge 693b1fbea654e5f22ee2cb9ff08af4b6cc08ccc9.
  Родители строго [531cf34, 5ca0ee8]; дерево bd324495b8d5618a7cfa5c2e652b6d0a7a4e6916
  совпало с tested head. Повтор неизменных тестов после merge не требовался.
- Три локальных checkpoint документа сохранены по SHA-256. Local main обновлена
  обычным non-force fast-forward fetch, пока не была checkout в worktree;
  затем switch между идентичными деревьями сохранил все три checkpoint и семь
  source fingerprints побайтово. Main=origin/main/693b1fb, index чист.
- Remote feature-ветка удалена с lease tip=5ca0ee8, local — обычным -d после
  ancestor main; отсутствие remote/local/tracking проверено. Прежние stashes
  и остальные worktrees сохранены. Итоговые journal/queue/EV записи локальные
  без дополнительного commit/push; active_work null. Watch выключен перед handback.
- Пять правил подтверждены в scope EV-117: обратимость, общий UI/HTTP/IPC/CLI,
  актуальные docs, Windows/Android и авторитетный набор данных. Все 98 определения/
  состояния сохранены; P06.06 done. Далее P06.07 planned, реализация не начата.
  Полный T09, GAP-028/031/032, физический arm64/Tailscale/измерения отдельно.
