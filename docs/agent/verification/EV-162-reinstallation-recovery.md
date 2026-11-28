# EV-162 — P07.06: восстановление после переустановки

## Рабочая карточка

- База: bbdb055, ветка feat/p07-06-reinstallation-recovery. Три исходных dirty документа сохранены по SHA-256 в ignored reports.
- Цель: восстановить отсутствующее пространство под исходным ID без прежнего локального доверия и без снимка отсутствующего состояния, с паузой до явного возобновления.
- Требования: полные T10/A58/A59, спецификация §1/10 и сценарии58/59, ADR-0016/0078/0081. Аварийный откат всей базы P07.09 отдельно.
- Зависимости: P07.03 done/0e42c68/EV-152; P06.01 done, его последующие исправления и P06.13 доступны в базе; каталог P07.05 done/2f2b8ea/EV-161.
- Охват: .1 storage/граф/транзакция; .2 core/HTTP/CLI/IPC/контракт; .3 существующие UI/File/SAF; .4 парная проверка, новые права и итоговая матрица. Схема SQLite и формат ZEDSPACE прежние.
- Проверки: штатные `bun scripts/check-product.mjs --host-tests`, `--host-build`, `--android`, `bun scripts/check-ui.mjs --main`, целевые browser desktop/touch и APK/Device; конкретные результаты фиксируются после выполнения.
- Отказы: malformed/несовместимый архив, отсутствие прав, появившееся/существующее/trashed/purged состояние, ошибка чтения, транзакционный сбой/остановка и повтор после restart. Нет частичной публикации; исходный файл не изменяется.
- Обратимость: отмена подготовки, обычное удаление восстановленного пространства/восстановление из корзины; отключение новых подключений и отзыв новых прав, отдельный resume/pause. Последующие восстановления существующего пространства всегда со страховочным снимком.
- Состояние: весь P07.06 и .1/.2/.3/.4 verified локально. A58/A59 partial с полной локальной матрицей до ревью/CI/интеграции; T10/P07 partial. Новых commit/push/PR/CI/merge нет.
- Следующее: ревью/фиксация и CI/интеграция P07.06 по поручению разработчика; затем готов P07.09. Active_work null.


## Промежуточная проверка

- Windows: четыре целевых storage tests прошли; отдельно прошла расширенная
  матрица snapshot_recovery с новой установкой, pending/confirm, restart/pause
  и независимыми изменениями двух хранилищ. Полный API-тест HTTP/CLI/IPC
  с исходными ключами/новой выдачей/отзывом/replay прошёл в общем прогоне.
- Browser: desktop/touch сценарий прошёл сначала для файла, затем с настоящим
  каталогом копий в другой папке: подготовка/отмена, исходный ID/карточка,
  явное применение и пауза. Полный UI main ещё выполняется.
- Первые отказы подготовки сохранены: локальный Node отличается от repository
  baseline (использован предусмотренный UI_NODE_VERSION без изменения baseline);
  clippy указал len_zero в новом тесте; контрактные тесты ожидали91 вместо93
  операций; проверка пустой папки после CLI не учитывала штатный daily tick.
  Исправления тестов не меняют продуктовых прав, бюджетов или assertions сбоев.
- Native Android, обе ABI, APK/Device и окончательные host/UI gates ещё ожидаются.
  Новая процессная проверка четырёх точек exit также требует финального прогона.
  A58/A59 not_run, T10 partial; приёмка не повышается по промежуточным результатам.


## Исходники проверяемого среза

SHA-256 собственных изменённых исходников и производного контракта, без локальных
данных машины и архивов испытаний. Документы приёмки в этот набор не входят.

| Файл | SHA-256 |
| --- | --- |
| `api/openapi.json` | `71c92815ed34f5fdff525d7264ea2be66b94bc2c54f6c5d0d29e415fcb9091df` |
| `api/schema.d.ts` | `692eb1e05f6356903a91875dec355e72a908bb2b9c3df9af397e1d4ff5e89b62` |
| `api/validate-response.d.ts` | `8f778f3770ba006c779354cb04db3a64ca28ce8b3696d6ea9b6e950b10625b0d` |
| `api/validate-response.js` | `55fb0b9ceaa00a4376d70375323c5f1870a7a45000774863a06753166a818b9c` |
| `client/commands.ts` | `67e6b0d60c2004425b8c5b02bc4514d6ab15ff250d1e0fd78a8fdeff56b1ade3` |
| `crates/adapters/src/lib.rs` | `603f87ea3aecb8c6ba791b428cf14a95c2feb87b72b74bc6236f568d7f5794f3` |
| `crates/adapters/tests/commands.rs` | `97d919fa05dd2dd8747b83d697906a481121e2c6b68a63333df243a547b0b52d` |
| `crates/adapters/tests/commands/reinstallation.rs` | `e85472ae9a2940d075eed5c36b560b1ec53e4552a26d35479dd698af12bedb6d` |
| `crates/core/src/lib.rs` | `65b85ecef063af926fad78e8ce93b47b3d43c3e9b6572b44424d772e3a210cfc` |
| `crates/core/src/recovery.rs` | `f9c0017ebe4d513ee97689d078de6a60037fed9b84c677e8ea0c35dfaa7799ee` |
| `crates/core/src/recovery/missing.rs` | `75802a8a2646a981b15acbe061471aa38476bc271c61ea80cc3d814bfc30fb42` |
| `crates/core/tests/contract.rs` | `dbf51bbd84ebd8e45a7c4a8b40253ce7c3197f755a81ea33b5785594a82b574c` |
| `crates/storage/src/recovery.rs` | `c1032d2fd6181a88d5da4482a4465064931d6526eb72422b98ee0ad6261f3839` |
| `crates/storage/src/recovery/graph.rs` | `26c4562f30d45aa97f0b16ae7a162d007ec3d8cc60a4c2aa11152633b4221cdf` |
| `crates/storage/src/recovery/missing.rs` | `b0efb844fbb84baa2f4dfca441320f54800a9937ac3547d97751b72e97d651a2` |
| `crates/storage/src/recovery/missing/tests.rs` | `a60cad305341e74b69b1f2cde03c884c1e2e5b68a89b714535b4ff19a62d1b49` |
| `crates/storage/src/recovery/tests.rs` | `067c61b18b74e394bf2ba703b449c2631ad72994ea75239898a0c81b9fbcf131` |
| `crates/storage/src/sync_engine/tests/snapshot_recovery.rs` | `12f3526e5c9c5d271ae1017a940570720532bff8379812e7b9adf57b3e9d75ae` |
| `scripts/check-ui-browser.mjs` | `bf364225e217af8a7531b55ab4edf02fa5e326bba59a9302329a92293125d8a0` |
| `scripts/check-ui-reinstallation.mjs` | `a7b3b748b7c0d714ae468f4350ca9dbbcfad28cfedbb88afab398b48d61e8bb0` |
| `ui/src-tauri/gen/android/app/src/androidTest/java/app/zedone/local/BackupFolderProbe.kt` | `ec3e43c09a5903edd1a7e00b1ad2cd11ad2ad737fd90e724cb7b9a2deb724b42` |
| `ui/src-tauri/gen/android/app/src/androidTest/java/app/zedone/local/SecretStoreInstrumentation.kt` | `61de267cb50aac59bef469b0797a74bc0a2341f6857d94d535bd3e207e440be5` |
| `ui/src/App.vue` | `4174a9bbac903272edadd8bb85bc72f39252f88e6db7d58fbf2ca974e6f598e6` |
| `ui/src/BackupCatalog.vue` | `9dc02e021b685ad2fdb8080d1c46c7fd841a288e78bb00f81d780505de794530` |
| `ui/src/MissingRecovery.vue` | `9653fd7c82d14dcdba7af0ef4456cf25b46dec20ef67e817d58b70dd913060a5` |
| `ui/src/SpacesView.vue` | `c4b06e3a3285f6adcf3ce4406317d1fc11806c462e7a2fb37dccb638b5d03501` |
| `ui/src/workspace-api.ts` | `f6e1a12075f621acf5bedca35371146bf7594d15b7ec6dbb21238faf23de4457` |


## Проверенный продуктовый охват

Архив восстанавливает исходный ID и причинную историю; новый локальный actor
не переиспользует техническую идентичность удалённой установки. Получатель
не импортирует служебные ключи, grants, подключения, receipts или peer progress.
Право ReadWrite выдаётся только вызывающему действующему локальному субъекту,
как при создании/импорте; его административная область не расширяется. Старые
ключи не работают в новой базе, новая явная выдача/отзыв и изоляция сохранившегося
другого устройства подтверждены. Scope/срок проверяются заново при replay.

Проверки контейнера, модели, ID и отсутствия сериализованы с публикацией графа,
прав, паузы и receipt. Active/trashed/purged, пустой и нечитаемый существующий
документ, ошибка SQL, неверный архив/ID/версия, появившееся после inspect состояние
и отказ записи права не допускают частичного восстановления. Пять транзакционных
границ и четыре настоящих process exit подтверждают единый результат после
restart; исходные changes, включая ожидающие зависимости, сохраняются побайтово.

Полная модель, история, теги, корзина и конфликты проверены новым storage fixture
на обеих платформах. Парная матрица включает новую установку, pending/confirm
подключения, restart с паузой, отдельное resume и сохранение независимых правок.
Существующее восстановление/страховочные снимки продолжают прежний общий planner
и прошли регрессии. Отмена подготовки ничего не публикует; обычные обратные
операции пространства/прав/подключения и pause/resume сохраняются.

Файловый UI и каталог используют общий API и производный Rust-контракт93
операций. Desktop/touch проверяют подтверждение, отмену, повреждение/бюджет,
появление ID, потерю ответа/reload/replay и явное resume. Каталог другой папки
подготавливает отсутствующий ID, отменяет и восстанавливает его, сохраняя карточку.
API HTTP/CLI/IPC проверены тем же handler и receipt; стандартный импорт по-прежнему
создаёт независимый ID.

APK/Device: новая база без пространств и настроек, файл Windows→Android/SAF,
отмена выбора и подготовки, каталог read-only tree, явное восстановление,
исходный ID/карточка, отсутствие страховочного снимка, restart с паузой и
resume/pause прошли. В новой базе1 документ,1 локальный principal/ReadWrite,
1 restore receipt,0 прежних pairing/peer записей; исходный файл не изменён.
Обратный возврат испытательного окружения проверен: исходный APK совпал по SHA-256,
все204 private entries восстановлены побайтово, собственные4 secret slots/SAF
permission/папка/тестовый пакет удалены; Device выключен. Данные машины и raw
артефакты сохранены только в ignored reports.

Это локальная приёмочная матрица. Физический arm64 runtime, hardware power-loss,
аварийный откат всей базы, CI и интеграция не подменяются этими результатами.
Полный P07/T10 и открытые DEC/GAP не закрываются автоматически.


## Итоговые проверки локальной матрицы

| Проверка | Результат |
| --- | --- |
| `check-product --host-tests` | passed:515 Rust tests с отдельным search probe, клиент28, OpenAPI/TS и11 negative shapes; финальный clippy без предупреждений |
| `check-product --host-build` | passed: workspace all-targets/all-features release и8 unsafe guards |
| `check-product --android` | passed:513 native tests с отдельным search probe, обе ABI release,16 unsafe guards, парные TLS/pending-request probes и идентичный контракт; runtime tmp удалён |
| `check-ui --main` | passed: QR/file tests, Vue types/build, shell fmt/clippy/tests3 и все lifecycle/board/touch slices; локальный предусмотренный Node override указан выше |
| Целевой browser desktop/touch | passed: файловая подготовка/отмена/ошибки/гонка/lost reply/restart/replay; настоящий каталог другой папки, подготовка/отмена/применение и исходная карточка |
| APK и Android Device/SAF | passed: чистая база, файл/отмена, каталог read-only tree/явное восстановление, ID/карточка/нет страховочного снимка, restart/pause/resume; библиотека APK совпала с проверенной по SHA-256 |
| Возврат исходной установки | passed: исходный APK и204 private entries побайтово прежние;4 slots/SAF permission/собственные папка и тестовый пакет убраны, Device выключен |
| Политика исходников | passed:482 source files без запрещённых комментариев; прежние fingerprints иEV-161 сохранены |

Первоначальные остановки Android/host-build были выполнены самим агентом для
последовательного использования общего target; это не продуктовые failures.
Первоначальные отказы новых тестов и окружения сохранены выше и в raw logs.
Окончательный Android gate прошёл без targeted recovery. Причины прежних
GAP-042/046/047/054/055 этим результатом не объявляются устранёнными.


## Вердикт — LOG-903, 2026-11-27

Весь P07.06/.1/.2/.3/.4 verified локально в отдельной ветке от bbdb055.
Полная локальная матрица A58/A59 подтверждена; состояния partial до ревью,
CI и интеграции. Общий T10/P07 partial, аварийная база P07.09 и итоговый P07.10
не принимаются. Все98 определения/96 остальных состояний критериев и75
ID/типов/состояний проблем сохранены; прежние GAP и DEC не закрыты.

Пять правил проекта соблюдены в проверенном охвате: обратные операции
и отмена, общий API человека/агента, user/agent/development/architecture docs,
Windows/native Android/Device parity и единые SQLite/Automerge данные с
производным Rust-контрактом. Физический arm64 runtime/hardware power-loss
не заявляются; локальная проверка не подменяет CI/интеграцию. Исходная
установка Android восстановлена побайтово, Device выключен.

Исходные dirty checkpoints иEV-161 сохранены. Commit/push/PR/CI/merge
не выполнялись. Active_work null; следующий шаг — ревью/фиксация и интеграция
этого среза, затем P07.09.

Финальный целевой Windows-прогон после уточнения побайтовой проверки changes:5
storage tests passed и1 служебный crash-child ignored. Все27 fingerprints
исходников совпали; git diff --check и структурная сверка98 критериев/75 проблем
прошли. Изменены только доказательства T10 и состояния/доказательства A58/A59.


## Подготовка PR — LOG-904, 2026-11-28

Все27 source fingerprints совпали с локально проверенным срезом. Для статической
UI-пары использованы настоящая база bbdb055 и сохранённая проверенная конечная
сборка; одинаковая пустая установка, светлая тема иviewport1440×1000. Before
показывает прежний доступный импорт, After — подготовленное восстановление
отсутствующего исходного ID с отдельным подтверждением. Оба снимка просмотрены,
текст читаем; медиа и промежуточная базовая сборка вне репозитория. Новых
результатов продуктового runtime/CI этим захватом не заявляется.


## Публикация — LOG-905

[PR #129](https://github.com/Zeyafix/zedone/pull/129), head
`47dfa8cfff85b220954a571e97b724db85724582`, base bbdb055. Создание/связь/readback/ready
выполнены штатными PR tools; gh использован только для отсутствующей у tool
загрузки attachments. Before/After проверены визуально и после загрузки по
полному совпадению байтов. Все28 jobs на публикации pending, feedback пуст;
успех CI/интеграция ещё не заявляются. Локальные проверки и27 fingerprints
прежние. Поручение merge и удаления ветки после полного успеха активно;
этот checkpoint локальный без дополнительного push.

## Linux shell CI — LOG-906

На исходном head47dfa8c отказал [linux-checks (shell)](https://github.com/Zeyafix/zedone/actions/runs/46112534824/job/140879048303):
checkShellFailureDiagnostics ожидал одну запись и получил две. Зависимый
check (ubuntu-24.04) также failure; snapshot на чтении25 SUCCESS/3 pending/
2 failure, feedback/unresolved пуст. Полный успех не заявляется, исходный
лог и snapshot сохранены в ignored reports.

Три --main среза выполняются в одном процессе. Подмена console.error внутри
проверки захватывала сообщения параллельных сценариев. Обработчик отчёта
теперь передаётся отдельному вызову withShellFailureDiagnostics; глобальная
подмена удалена. Проверка намеренно запускает два одновременных сбоя с
разными получателями, сохраняет точную исходную ошибку даже при отказе
получателя и проверяет отсутствие credentials. Assertions, таймауты, лимиты
диагностики и параллелизм сохранены.

Полный повтор `bun scripts/check-ui.mjs --main` с предусмотренным локальным
UI_NODE_VERSION=26.8.1 завершён exit0: QR13/file2 tests, Vue types/build,
shell fmt/clippy и3 tests, все lifecycle/board/touch срезы passed. Новый
конкурентный сценарий диагностики и desktop/touch переустановки/каталога
прошли внутри общего параллельного прогона. Source-comments482 и diff
--check passed. Raw log сохранён в ignored reports; новая CI-матрица после
commit/push ещё ожидается, исходный Linux failure остаётся evidence.

27 fingerprints продукта P07.06 прежние; изменены только два диагностических
скрипта и документация. Нового Android/Device runtime этим исправлением
не заявляется; A58/A59 partial, T10/P07 partial,75 проблем прежние.

Дополнительные SHA-256 диагностических исходников LOG-906:

| Файл | SHA-256 |
|---|---|
| `scripts/check-ui-diagnostics.mjs` | `b04b55b6b14a3a1d87afa80aeed4943c62963a473e082e136c9b2099a924ebc4` |
| `scripts/check-ui-native-controls.mjs` | `0bbf2e3cf67d348df629f6d7b44b13b93276d951973e179461149dcbf38c6f48` |
