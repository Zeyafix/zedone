# EV-152 — P07.03: восстановление существующего пространства

```yaml
step: P07.03
date: 2026-11-25
base_revision: 29d8bb7 plus verified local P07.03.1
branch: feat/p07-03-space-restore
goal: "Полное восстановление существующего пространства через общий API/UI с новым страховочным снимком, постоянной паузой и безопасным replay."
requirements: [T10, A09, A44]
dependencies_evidence: [EV-151, EV-146, EV-150]
scope: [storage recovery and safety publication, core API and adapters, client and file UI, tests and documentation]
checks: [host tests/build/contracts, real paired changes and failures, Windows browser and native Android Device, both Android ABI]
reverse_and_failure: "Возврат через последний страховочный снимок тем же явным восстановлением; ошибка не меняет данные, права, receipts и последнюю успешную копию."
result: "P07.03 и .1/.2/.3/.4 verified локально; без нового commit/CI/интеграции."
next: "Ревью/фиксация полного локального результата и CI/интеграция по следующему поручению."
```

Прямое поручение разработчика на полный P07.03 разрешает зависимые локальные
срезы без промежуточной интеграции .1. Исходные локальные изменения .1 и
интеграционные записи P07.02 сохраняются. Коммит/публикация не запрошены.

Технические решения: рабочий журнал не откатывается. Восстановление создаёт
новые причинные операции, сохраняет идентичность пространства/объектов,
авторство, пары текста, порядок и конфликтующие скалярные варианты снимка.
Записи возвращаемой истории получают новые технические IDs с прежними авторами:
это исключает коллизии структурных CRDT-объектов при независимых восстановлениях
и поздней фиксации старой серии. Прежние raw changes и receipts сохраняются.

Новый страховочный файл создаётся и проверяется под тем же BEGIN IMMEDIATE,
что восстановление, пауза и receipt. Отдельный служебный witness в прежнем
command_results связывает commit с конкретным файлом; файловый каталог не
доказывает commit. Предыдущая копия удаляется только после проверки новой
committed-копии. Ошибка cleanup сохраняет pending и не превращает подтверждённое
восстановление в откат; повтор завершает только cleanup. Другие файлы не удаляются.

До P07.05 действует папка копий установки по умолчанию рядом с базой, отдельно
от её файлов. Выбор/перенос произвольной папки и каталог копий остаются P07.05;
внешние запросы передают байты и непрозрачный ID, не серверные пути.
Недоступность папки не разрешает fallback или восстановление без нового снимка.

Все 98 состояний критериев и 72 проблемы сохраняются до результатов проверки;
полные T10/A09/A44 не принимаются автоматически. Размер превышает ориентир300
одной завершённой операцией с обратным восстановлением, атомарностью,
правами/replay, обеими платформами и обязательными доказательствами.

## Реализация и матрица проверок

| Срез | Проверенный результат |
| --- | --- |
| P07.03.1, повторная проверка | Общий capture экспортирует весь граф с pending-историей/конфликтами/корзиной и ожидающими зависимостями. Прежние 8 Windows/9 Unix native тестов кандидата, включая четыре настоящих process exit, входят в полный прогон. Исходные fingerprints EV-151 исторические; окончательный состав подтверждается ниже |
| P07.03.2, storage | 8 новых тестов: полное восстановление/обратное действие/перезапуск, пять границ SQLite commit, неизвестный результат после commit, недоступная папка без fallback, повреждение файла/отказ witness, сохранение предыдущей копии и независимых правок |
| Причинные конфликты | Удаление против правки карточки/колонки/пространства, purge против независимой правки и удаление тега против назначения сохраняются. Обычное разрешение снимает восстановленную отметку; поздний tag alias не перехватывает исходный тег |
| Реальный обмен | Две независимые Storage-базы передают сериализованные SyncPage в обе стороны после явного resume. Совместимые поля и конфликтующие тексты/история второго устройства сохраняются; восстановление передаётся новыми changes |
| P07.03.3, adapters | 3 новых теста HTTP/IPC/CLI, restart, reverse и replay прежней правки. Некорректный/чужой архив, stale и неизвестные credentials не меняют пространство/копию. Read-only и отозванные/суженные текущие права запрещают восстановление, включая replay; свежий Read-ключ разрешает inspect |
| Контракт/клиент | 79 операций, Rust → OpenAPI → TypeScript/validators, 11 отрицательных type probes; прежние 28 client tests/308 assertions. Restore использует прежнюю очередь намерений и увеличенный только для архива транспортный бюджет |
| P07.03.4, Windows | Штатный UI main в Chrome и Opera: desktop/touch, явное подтверждение, отмена/удаление/бюджет файла, потерянный ответ/reload/replay, те же ID/теги/корзина/изоляция, скачивание страховочной копии, обратное восстановление и явное resume. Все lifecycle/board/touch slices прошли |
| Native Android Device | Собственная изолированная фикстура APK. Настоящий SAF: отмена выбора, Windows-снимок, подтверждение, восстановление/пауза, отмена сохранения с повтором, сохранение страховочного файла и обратное восстановление. Согласованные копии SQLite прочитаны штатным CLI: те же ID/текст/теги/корзина, committed witness, cleanup завершён |
| Windows → Android → Windows | Windows export выбран нативным SAF и восстановлен в исходное пространство. Настоящий SAF-файл страховочной копии прочитан на Windows и восстановлен через тот же API; текст/удалённый тег/корзина/ID и пауза совпали |
| Полный host | `bun scripts/check-product.mjs --host-tests`, exit 0: 467 Rust tests с изолированным поиском, fmt/Clippy, client/контракт/типы |
| Полный native | `bun scripts/check-product.mjs --android`, exit 0: 465 Rust tests с изолированным поиском, парные TLS/pending-request, равенство контракта, release/unsafe проверки обеих ABI |
| Сборки | `--host-build` и окончательный `--android-x86_64`, exit 0: release/unsafe guards. Окончательные x86_64 acceptance-debug и arm64 release APK собраны штатным build-ui-android; manifest проверен, окончательный debug APK установлен и открыт через Device, inspect подтвердил сохранённые pause/backup |
| Сохранность стенда | Исходные APK и все23 регулярных приватных файла восстановлены и равны по SHA-256; отдельный прежний product также совпал. Собственные Keystore namespace/fixture/remote/файлы Download удалены, исходные маркеры возвращены. Чужие каталоги/сессии не очищались; собственный Device выключен |

Исходные ошибки компиляции/форматирования и первый browser-прогон со старым
fixture-бинарным файлом сохранены в локальных логах; успешными не считаются.
Для UI использован доступный Node через явный локальный override; закреплённые
версии репозитория и зависимости не изменены. Native-фикстура первоначально
попала в files вместо корня app-data; старый product был безопасно отклонён
проверкой модели. После исправления размещения использовано только собственное
изолированное пространство. Временная debug-диагностика из исходников удалена.

Итоговый просмотр выявил сохранённые в памяти байты прежней страховочной
копии после следующего восстановления. Кэш теперь сбрасывается при успехе;
отмена файлового сохранения по-прежнему оставляет байты для повторной попытки.
После полного UI main выполнены отдельные desktop/touch прогоны Chrome/Opera
на окончательном UI с повторным скачиванием после reverse; все четыре прошли.
Vue/TypeScript и Vite повторно проверены после этого исправления.

Логи, согласованные копии, исходные capture/артефакты, записи восстановления
стенда и машинные hashes находятся только в ignored docs/reports/p07-03-all.

## Проверка пяти правил

- Обратимость: последний страховочный снимок возвращается тем же новым
  подтверждённым восстановлением. Пауза имеет явное resume и повторную паузу;
  отмена выбора/сохранения, ошибки и replay не создают скрытого повторного действия.
- Человек/агент: четыре команды одного Service/core handler доступны
  HTTP/CLI/IPC; UI использует тот же договор и обычную очередь намерений.
- Документация: user/agent/development, ADR-0077/0078, EV-151/152,
  очередь и журнал описывают действия, отказы, обратимость и текущие границы.
- Паритет: один storage/core код прошёл Windows/native Android; файловый UI
  проверен Chrome/Opera desktop/touch и через APK/Device/SAF.
- Единый источник: прежний авторитетный граф/права и общий export capture;
  witness в SQLite связывает commit с проверенным файлом. UI и файловый каталог
  не становятся отдельным текущим состоянием пространства.

## Границы приёмки и передача

Проверки process exit относятся к файловому кандидату EV-151; полная новая
транзакция проверена fault injection/restart на пяти границах и потерей ответа
через настоящий UI/replay. Отключение питания и гарантии Windows directory
durability/произвольного SAF-провайдера не принимаются. Физический arm64 runtime,
чистая переустановка, произвольный каталог копий, ежедневные копии и аварийная
база остаются за пределами P07.03. Ограниченные бюджеты не усечены молча.

Все98 определения/состояния критериев и72 проблемы сохраняются. T10/A09/A44
остаются partial: полный P07, каталог/переустановка/миграции и новая CI/интеграция
не объявляются принятыми. Локальная реализация полного P07.03 verified и готова к
фиксации/ревью и CI по следующему поручению; новых commit/push/PR/merge нет.

## Проверенное дерево исходников

База 29d8bb7 плюс локальный связанный результат .1/.2/.3/.4.
SHA-256 окончательных исходников и производных контрактов:

| Исходник | SHA-256 |
| --- | --- |
| api/openapi.json | 5fcf6816731618e5f1eda42a3b96af73a8d880ecc5268c53781615325fbc5478 |
| api/schema.d.ts | a63ec61f7088f6bfaf16ddc96a1ec2c022e602ffc3d9464df7cc8bd55569e655 |
| api/validate-response.d.ts | 9a5164efebba2dd9555abf898d6631447f94cdc74e645a9e611c7b491de619d2 |
| api/validate-response.js | 937ffceda0507993baef14f879f7dd4ba7a7e18faebbe3bfd36b9a9647691c0d |
| client/commands.ts | d982d51a6dd1449f548823a907fbac3f3030f4561aebcc9b2fbcc43fb4cf0c99 |
| crates/adapters/src/lib.rs | 946673b450d0f1b2b796aea8ad75cc7fb3ce52a9a291ce6430c5c4fe9aaf4f2a |
| crates/adapters/tests/commands.rs | 5595931746934fde75c74fb02cd765f4587d230c9dd9c1be9b44d8cbb4282762 |
| crates/adapters/tests/commands/recovery.rs | 47b56479e0d5757d8a92f8124f65dd0b17e0241055a99ea52e3868f12df289a1 |
| crates/core/src/lib.rs | 813970c09271f6eaa97d139e6403f778258600c2620b4a056afd552ce4f3e450 |
| crates/core/src/recovery.rs | fbecd076fef4b570481394fdd6f8e47c583c874c2bc2d4e77e3e2b904044012d |
| crates/core/src/snapshots.rs | 9f2386520fa795caba75247ea67229bbb5e7df29a233a7eb143c7cc367e70b8b |
| crates/core/tests/contract.rs | 05ea045dd0baef72fed472af999a39d98326b6439a24f25125932ec8b7a7e393 |
| crates/storage/src/cards.rs | da2a09ecfa53c8370d3bdba86dbc94afabbbcfa3b0047d4ff71c0ba7890be965 |
| crates/storage/src/columns.rs | 17e41b4ffb8167bed54fd5e79adb657ae16b18f84f8000bade88742cb1910168 |
| crates/storage/src/lib.rs | 8cae32bd773b8c21c40ba6976a0c9993144f542c288597a2ab5e8d2fb8ca3084 |
| crates/storage/src/recovery.rs | 5a797ef7e03971095fe60d44fbfda0ec8b6b780b8261b0eda834888fe3f25af0 |
| crates/storage/src/recovery/graph.rs | 33e97d19006a9420e8c63019e054fc9fc23a88cb6914d74476747eb72137ba57 |
| crates/storage/src/recovery/tests.rs | f5a09f21d9f54af4c234626f6698c848a7d05d3bbd831f4c81e07e370919a9a3 |
| crates/storage/src/recovery_flags.rs | d2034d4a39d5f1cdf6b3cec03f6fc3b452d7eb093810686ac8422124ff34f262 |
| crates/storage/src/restore_backup.rs | f0e8dfedfbd51d293f1ac962347c7ad735b56b45926fef894216e8abc19a656b |
| crates/storage/src/restore_backup/tests.rs | 0b30d4b3aeed4346b685cedd3e674d6b41c0ef666b101fd5a185324ba3275bf9 |
| crates/storage/src/snapshot.rs | cc4fe793a07e4b67720af1b19aacfe43fa83d2d334cf2fd0dc8c62af99efae3d |
| crates/storage/src/snapshot/export.rs | aef31961a8908bc870ad2d19bbae483a538f823361058427446afc89c9713635 |
| crates/storage/src/snapshot/import.rs | 8c208894fc4e871ac3d43399810460d2a1788dd76b58a94627e48960026e9b17 |
| crates/storage/src/snapshot/import/model.rs | 957351e7cf38068c9b5666583645e710f1c3e0c67d632e72781cafd5e5023622 |
| crates/storage/src/spaces.rs | 28072bb9c2dd2bc161b9cce73fc0e05516d60f8976c721c3f275299db3f5aee0 |
| crates/storage/src/sync_engine/tests.rs | 618267778e6c73f12219f1e9068d6ee7f3f16bbe0498b3d23cfbeca29b23df39 |
| crates/storage/src/sync_engine/tests/snapshot_recovery.rs | ea83bd5c58e2a1bb3fe8aa552f123f6768448d81f03873c79340016b7f20e703 |
| crates/storage/src/tags.rs | a5cb71eecc8ba2c3027cd76af953e84bb2e451f3d7a1975c1a20c293fe46bb9e |
| crates/storage/src/tags/convergence.rs | ba32905baa5c20e7234a50de17ea9b837fc67d2464d7801c309e970079e405d2 |
| crates/storage/src/tags/convergence/lifecycle.rs | 110f1637f2dbd4067469cd79a3d7c42b2b3aa0426c231e11edf4ca75704c3868 |
| scripts/check-ui-browser.mjs | 02ee207cd0971dad44b26a7e7216d39d35c066a60e576c96265a6c313c9d6796 |
| scripts/check-ui-recovery.mjs | 8331c5e788dc9a89af73270609c6f43b58561d0cf26a5383dd7e6febaba75e54 |
| ui/src/SpaceRecovery.vue | ad9e1f886b1703a5b73186ccf75728f16ad1ad26e0fb1a56edf1ea952c8b829b |
| ui/src/SpacesView.vue | db6eedd9a9f80bae707259d9e99de40bf5e7d78930ada44dfdb59bf76a166cad |
| ui/src/workspace-api.ts | e7cb4fdc33ba5e76180360f0321aecea9fb1b3a356588fe1b7df264aef68f24e |

## Интеграция — 2026-11-25

[PR #123](https://github.com/Zeyafix/zedone/pull/123), head `ce0a415`,
merged в `0e42c68` после всех27 SUCCESS Windows/Linux/Android.
Дерево merge равно проверенному head; comments/unresolved threads/pending/failed
отсутствуют. Полный P07.03 и .1/.2/.3/.4 done в проверенном охвате.
Main/origin/main обновлены, feature-ветка удалена локально и на GitHub; LOG-817.
Исходные локальные результаты/ограничения и все98 состояния/72 проблемы
сохранены; T10/A09/A44 partial, полный P07 не принимается автоматически.
Интеграционная запись локальная, без отдельного коммита и повторного runtime/CI.

## Post-merge CI — диагностика 2026-11-25, LOG-818

Все27 SUCCESS выше относятся к PR head `ce0a415`. Отдельный push-run
[Android 45621118870](https://github.com/Zeyafix/zedone/actions/runs/45621118870)
на merge `0e42c68` failed, хотя Git tree равен проверенному head:

- [prototype-ui](https://github.com/Zeyafix/zedone/actions/runs/45621118870/job/139229219197):
  Maven Central HTTP429 для POM junit:junit:4.13.2 при сборке AndroidTest,
  до instrumentation. Внешний dependency resolution отказ, GAP-037 open;
  configuration-cache serialization сообщение вторично.
- [shell](https://github.com/Zeyafix/zedone/actions/runs/45621118870/job/139229219052):
  check-ui-search.mjs:58, all→search(marker), ready timeout10000ms.
  UI configured, без alerts/pageErrors, результаты сброшены, статус предлагает
  нажать «Найти». События ввода/submit/IPC не сохранены, artifacts отсутствуют;
  причина не доказана (GAP-053), потерянный touch/поздний invalidate — гипотезы.
- Aggregate android отклонил failure остальных suites; ещё13 Android suites
  и отдельный [host run](https://github.com/Zeyafix/zedone/actions/runs/45621118386)
  SUCCESS. Это не полный успешный post-merge CI.

Источники — свежие GitHub jobs/logs/artifacts и исходники; полный результат
сравнения с прежним merge #122 и дефект пути server stderr — LOG-818.
Продукт и CI не менялись, runtime/Device/rerun не запускались; приёмка EV-152
не расширяется, P07.03 интегрирован, GAP-037/047/053 требуют отдельной проверки.
