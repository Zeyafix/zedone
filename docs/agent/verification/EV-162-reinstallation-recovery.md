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
- Состояние: P07.06/.1–.4 и FIX.08 done, A58/A59 passed после полной локальной матрицы и CI32/интеграции PR129 вa41e762; T10/P07 partial, GAP-056 open. LOG-915.
- Следующее: готов P07.09, не начат; active_work null. Итоговые записи приёмки локальные без нового commit/push, источник продукта соответствует проверенному merge tree.


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

## Повторная публикация — LOG-907

Commit `7be4018ef8c6ee943f12508f10c179e61db63b99` отправлен в тот же
[PR #129](https://github.com/Zeyafix/zedone/pull/129). Readback подтвердил
точный head, main/bbdb055, nondraft/mergeable/behind0, пустые feedback/threads,
совпадение обновлённого тела и прежнюю одну пару Before/After. Host
run46114201034 и Android run46114200932 начаты:28 jobs pending, три strict
aggregates ожидаются. Старый failure сохранён; успех новой CI-матрицы и
merge/delete не заявляются. Существующий watch продолжается без повторного
link/watch/list. Итоговый checkpoint локальный без нового push.

### Прежний Ubuntu aggregate — LOG-908

Запоздалое событие [check (ubuntu-24.04)](https://github.com/Zeyafix/zedone/actions/runs/46112534824/job/140881774086)
относится к47dfa8c. Workflow этого commit требует success всей linux-checks
матрицы; исходный snapshot показывает три других Linux suites SUCCESS и
единственный shell failure LOG-906. Это зависимый итог прежнего отказа.
ci_logs не получает старый aggregate после смены head: он отсутствует среди
текущих checks, поэтому вывод опирается на точную workflow и сохранённые
статусы. Свежий readback7be4018:0 SUCCESS/28 pending/0 failures, пустой
feedback, behind0/mergeable/nondraft. Новый rerun/push не требуется;
существующий watch остаётся активным до порученной условной интеграции.

## Linux shell CANCELLED и один recovery — LOG-909

На7be4018 [linux-checks (shell)](https://github.com/Zeyafix/zedone/actions/runs/46114201034/job/140884222831)
прошёл все main lifecycle/board/touch, новые desktop/touch восстановления и
изоляцию диагностики, затем весь conflicts/positions. Pairing начат06:11:47.911
UTC, scanner и initial pending request passed;06:11:58.901 операция отменена,
полного pairing результата нет. Assertion failure в доступном tail отсутствует.
[Ubuntu aggregate](https://github.com/Zeyafix/zedone/actions/runs/46114201034/job/140888279786)
получил RESULT cancelled и exit1. Остальные29 checks SUCCESS, включая весь
Android/Windows и их aggregates; feedback пуст, head/base/behind0 прежние.

Linux job имеет прежний10m budget. Исчерпание бюджета правдоподобно, но точная
причина отмены и связь с прежним GAP-042 по ограниченным ci_logs tails не
доказаны; metadata/annotation этим tool не предоставлены. Сохранены оба tail
и исходный snapshot в ignored reports. Cancelled/aggregate failure не считаются
успехом; прежние gaps и75 проблем сохранены.

Принят ровно один адресный ci_rerun Linux shell: rerun=true, accepted1/limit1,
тот же head7be4018/код/лимиты/assertions.29 успешных outcomes вручную не
перезапускаются. Следующих same-head повторов нет; shell и зависимый aggregate
должны пройти до merge.29 source fingerprints прежние, нового локального
runtime/Device не было. Existing watch остаётся активным, checkpoint локальный.

## FIX.08 — Linux main и conflicts/pairing отдельно — LOG-910

Единственный recovery [job140888603327](https://github.com/Zeyafix/zedone/actions/runs/46114201034/job/140888603327)
на7be4018 тоже CANCELLED. Все UI сценарии, включая pairing, завершились;
сам UI step имеет success, но job достиг лимита при завершении. Новый
[Ubuntu aggregate](https://github.com/Zeyafix/zedone/actions/runs/46114201034/job/140891035341)
получил RESULT cancelled. Остальные29 outcomes SUCCESS; отменённый job
не считается полным успехом. Accepted1/limit1 исчерпан, новых same-head
повторов не запрашивается.

После ci_logs отсутствующие в tool метаданные/annotations прочитаны read-only
GitHub API через gh. Оба jobs имеют явную annotation:
«The job has exceeded the maximum execution time of 10m0s».
Исходный06:01:46→06:12:01 UTC, recovery06:13:25→06:23:26; UI step recovery
06:14:24→06:23:24 success. Причина этих двух отмен теперь подтверждена;
причины старых jobs GAP-042 не объявляются одинаковыми или устранёнными.
Все metadata/annotations/tails и snapshots сохранены ignored.

Рабочая карточка FIX.08: использовать существующие режимы Windows на Linux,
сохранив полное объединение команд/assertions и строгую приёмку, без изменения
продуктового runtime. Linux shell вызывает --main, новый shell-conflicts —
--conflicts с полным conflicts/positions/pairing. У каждого прежние10m,
max-parallel2/fail-fast false, aggregate success всей матрицы. Пины, namespaces,
seed shell→shell-conflicts и правила записи кэшей прежние. Полная локальная
команда без аргументов доступна. Подготовка двух jobs повторяется; экономия
runner-minutes не обещается. Ожидаются32 outcomes:5 Linux+6 Windows+18 Android
и3 strict aggregates. Возврат grouping обычным revert обратим, данных нет.

Локально passed `bun prototypes/scripts/check-host-ci-suites.mjs`: полное
объединение режимов, ошибки/сигналы каждого шага и draining, точный workflow,
strict gates, caches/seed/owners, tool caches и main slices. Отрицательный
контроль с удалённым обязательным Linux conflicts job отвергнут; workflow
восстановлен побайтово. Actionlint1.7.12 passed, официальный Windows archive
проверен по SHA-256; ShellCheck отсутствует и отдельно не запускался, тела
shell не меняются. Comments482/diff --check passed.29 прежних fingerprints
не изменились, дополнительно фиксируются два CI исходника:

| Файл | SHA-256 |
|---|---|
| `.github/workflows/prototype-foundation.yml` | `505e72fcd4c43fd5dfb2c99aeb4c328eba117d6c908333ee9197819e0bc1bcac` |
| `prototypes/scripts/check-host-ci-suites.mjs` | `48d2868daa428fd9cd86b82cfb1355770b14e1224e837703fe05a338ba9362b3` |

FIX.08 verified локально; новые реальные split Linux jobs и полный CI32
ещё не выполнены. UI-команды/тесты продукта не менялись, повторный локальный
UI/Android/Device runtime не запускался; прежняя локальная матрица и успешные
Windows/Android CI7be4018 сохраняют свой scope. A58/A59/T10/P07 partial,
75 проблем и98 определения критериев прежние. Пять правил проекта сохраняются
в прежнем продуктовом охвате. Далее commit/push и новый CI, merge/delete пока нет.

## Публикация FIX.08 — LOG-911

Commit `671180aeca474a51cea0cb13be530a6c5978d62c` отправлен в тот же
[PR #129](https://github.com/Zeyafix/zedone/pull/129). Readback: точный head,
main/bbdb055, nondraft/mergeable/behind0, полный feedback пуст, обновлённое
тело совпало, одна пара Before/After и credit сохранены. Host run46116967106
и Android run46116967082 начаты:29 suites pending, включая оба Linux UI jobs,
три strict aggregates ожидаются. Для интеграции нужны все32 SUCCESS.

31 source fingerprints совпали;98 criterion rows и75 problem IDs/types/states
побайтово/структурно прежние, GAP-042 только дополнен evidence и остаётся open.
FIX.08 verified локально, A58/A59/T10/P07 partial. Existing watch активен,
merge/delete ожидают полной новой матрицы; checkpoint локальный без нового push.

## Android prototype-ui: column gesture timeout — GAP-056 / LOG-912

[Job140892850528](https://github.com/Zeyafix/zedone/actions/runs/46116967082/job/140892850528)
на671180a failed в board-scenarios.mjs:164: после column reorder gesture
saved() не дождался «Перемещение сохранено.» за прежние10000ms. До него
Activity recreation/IPC/Markdown/QR/scanner/storage/HTTP и card reorder/
inverse/cross-column gesture прошли. Assertion порядка колонок и последующие
native сценарии не достигнуты. Android probe cleanup completed. Причина
не установлена; emulator graphics warnings не доказывают причинную связь.

Runtime/board/scenarios/native driver не менялись после успешного7be4018
prototype-ui job140884222868; в prototypes изменён только CI routing guard.
Прежний успех не заменяет новый failure.31 source fingerprints прежние;
CI readback точный/nondraft/mergeable/behind0, feedback пуст, на чтении
6 SUCCESS/22 pending/1 failure. Исходные tails/snapshot сохранены ignored.

Добавлен GAP-056 open:76 problem rows,75 прежних ID/типов/состояний и98
criterion rows сохраняются. Неизвестная причина не объявляется исправленной.
Запрошен один targeted prototype-ui recovery, GitHub вернул rerun=false:
«The workflow run containing this job is already running». Accepted0/limit1;
после terminal workflow нужен один принятый запрос на том же head/коде/10s/
assertions. Далее требуются job и dependent aggregate SUCCESS, полный CI32.
Нового same-head recovery ещё нет, local runtime/Device не запускались;
existing watch сохранён, checkpoint локальный без commit/push, merge/delete нет.

## Prototype-ui recovery принят — LOG-913

После terminal workflow671180a имеет30 SUCCESS/0 pending/2 failures,
feedback пуст, nondraft/mergeable/behind0. Все пять Linux suites (включая
раздельные main/conflicts) и Ubuntu aggregate passed, все Windows и прочие17
Android suites SUCCESS. [Android aggregate140896711178](https://github.com/Zeyafix/zedone/actions/runs/46116967082/job/140896711178)
прямо содержит RESULT failure/exit1, нового первичного отказа нет.

Запрос targeted prototype-ui recovery принят: rerun=true, accepted1/limit1,
тот же head/код/10s/assertions/job budget.30 успешных outcomes вручную
не перезапускаются. Требуются полный prototype-ui и dependent aggregate
SUCCESS перед merge. Исходный failure GAP-056 и неизвестная причина
сохранены; успешный recovery сам по себе её не закрывает. Новых same-head
повторов не назначать. Snapshot/aggregate log/request result сохранены ignored,
existing watch активен, checkpoint локальный, нового local runtime/Device нет.

## Интеграция — LOG-914/915

[PR #129](https://github.com/Zeyafix/zedone/pull/129) merged в
`a41e7624b53e4b7b2d0b43b7940dcf93ce098e15`, checked head
`671180aeca474a51cea0cb13be530a6c5978d62c`. Перед merge штатный readback
подтвердил все32 SUCCESS:5 Linux+6 Windows+18 Android+3 strict aggregates,
точные head/base/behind0/mergeability/nondraft, отсутствие feedback и unresolved.
Merge с expectedHeadSha имеет parentsbbdb055/671180a и то же дерево
`9dd021f9912ca300f634280a4e59650ba909e221`; ancestry проверен.

Prototype-ui recovery140897128481 прошёл column gesture/полный board/
auto-scroll/API equality, SQLite lock/restart, очистку synthetic sets и probe
cleanup; aggregate140898595798 SUCCESS. Открытый GAP-056 сохраняет initial10s
failure и неизвестную причину. Уточнение объёма повторов: один named ci_rerun
запрос фактически дал новые IDs всех18 Android suites+aggregate.13 host
outcomes прежние; все19 повторённых Android outcomes SUCCESS. Request result,
comparison, initial failures и успешный tail сохранены ignored.

P07.06/.1–.4 и FIX.08 done, A58/A59 passed по полной локальной Windows515/
native Android513/API/UI/Device матрице, обеим ABI/unsafe/парным probes/
контракту и финальному CI32. Определения98 критериев и96 остальных состояний
сохранены; T10/P07 partial.76 problem IDs/types/states прежние, GAP-056 и
прочие неизвестные причины не закрываются успехом повторов. Физический
arm64 runtime/hardware power-loss не проверялись; прежние ограничения действуют.

Main=origin/main/a41e762; ветка удалена local/remote/tracking. Два dirty
checkpoint сохранены по SHA-256 перед switch и остались побайтово прежними
при переходе между одинаковыми trees, затем дополнены итогами. Исходный APK/
204 private entries восстановлены ранее, Device выключен. Все пять правил
AGENTS.md соблюдены в указанном scope. Итоговые journal/queue/EV/development
записи локальные без нового commit/push; новый post-merge CI не заявляется.
Active_work null; готов P07.09, не начат.
