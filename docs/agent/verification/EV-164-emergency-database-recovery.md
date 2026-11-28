# EV-164 — P07.09: служебный режим и аварийное восстановление базы

## Рабочая карточка

- Поручен весь P07.09 в feat/p07-09-emergency-recovery от a41e762, на проверенном локальном P07.09.1/EV-163. Все исходные dirty/untracked файлы сохранены побайтово в ignored reports.
- Требования: полные T10/M05/M06, спецификация §1.1, ADR-0030/0033/0034/0036/0083. P07.08 доступен в базе; .1 verified локально. .2/.3/.4 выполняются последовательно без промежуточной интеграции по прямому поручению.
- Цель: при неподтверждённой целостности остановить обычную работу/обмен, сохранить проблемные байты и дать человеку/агенту одно явное авторизованное аварийное восстановление из проверенного полного комплекта.
- Охват .2: межпроцессное владение, staging/проверка полного комплекта, внешний журнал замены и защита старых ключей/намерений при откате. .3: отдельная служебная власть вне базы, права/отзыв и общий HTTP/CLI/IPC контракт. .4: общий служебный UI, File/SAF, причина/копия/охват/подтверждение и retry, Windows/Android Device.
- Обратимость: отмена подготовки без замены, явное возвращение сохранённого проблемного состояния с закрытым обычным доступом, отдельное удаление ненужного raw-комплекта, выдача/отзыв служебных полномочий. Прежние исправные копии не удаляются.
- Проверки: адресные Rust/API/файловые fault/crash/replay/права, штатные host-tests/build, Android native/обе ABI/unsafe, Rust/OpenAPI/TS, UI main и целевые recovery desktop/touch, APK/Device. Команды и фактические результаты добавляются по мере выполнения.
- Состояние: весь P07.09/.1–.4 verified локально; M05/M06/T10 partial до ревью/CI/интеграции. Active_work null; новых commit/push/PR/CI/merge нет. Далее ревью/фиксация всего среза.

Продолжение LOG-921: разработчик поручил PR и, при успешных проверках,
merge/удаление ветки. Публикуется весь проверенный P07.09 вместе со связанными
исходными checkpoints приёмки P07.06/FIX.08. Размер больше ориентира300 строк:
внешние права/receipts, durable gate, замена/обратное действие и общий API/UI
образуют один порученный аварийный сценарий; их временное разделение в выпуске
не допускается. Rust/OpenAPI/TS validator часть diff сгенерирована штатно.
Локальная матрица выше остаётся основанием; новые CI/merge ещё не приняты.

## Реализация полного среза

- Продолжена прерванная работа в прежней ветке. .2/.3/.4 реализованы локально:
  межпроцессные ownership/control locks, сохранение raw, staging полного
  комплекта, внешний phase/receipt, атомарная замена и явное продолжение.
- Восстановленный кандидат отключает старые ключи/субъекты, pairing/входящий
  обмен, меняет actors/локальную сессию/namespace и сохраняет постоянную паузу.
  Поколение сверяется с внешними метаданными; подмена старым/пустым/утраченным
  файлом не становится новой установкой. Raw можно вернуть с закрытым gate.
- В этой передаче закрыто последнее найденное упущение: старые очереди
  удаления/переноса копий не возобновляются. Daily policy и её известный лимит
  сохраняются, старые ссылки на ротацию снимаются, namespace новых файлов
  меняется. Автоматическая запись ждёт явного выбора папки с управлением;
  read-only выбор не снимает запрет. Старые файлы не удаляются.
- Служебные grants/сроки/отзыв и receipts находятся вне SQLite; обычный ключ
  не подходит. HTTP/реальный CLI/IPC используют `maintenance.execute`.
  UI Windows/Android отделяет prepare от apply и показывает причину, копию,
  охват/последствия, явное подтверждение, отмену, rollback и retry.
- [ADR-0084](../../architecture/decisions/0084-emergency-database-recovery.md),
  [user](../../user/emergency-recovery.md),
  [agent](../emergency-recovery.md),
  [development](../../development/emergency-recovery.md) описывают договор.

## Исходные отказы и уточнения проверки

- Адресный `cargo test -p zedone-storage emergency --locked --offline`:
  12 passed, включая настоящие process-exit checkpoints, перезапуск,
  точный повтор, отмену/порчу/отказ ключам/rollback/старое поколение.
- Первый полный Windows-run: 406 storage passed/1 failed. Единственный отказ
  `previous_development_schemas_are_rejected_without_migration_or_reset`
  ожидал один файл в каталоге, тогда как новый пустой `.owner.lock` остаётся
  рядом. Тест уточнён до точного набора база+пустой lock; исходная проверка
  неизменных durable-файлов сохранена. Продуктовый код для повтора не менялся.
- Первый UI-run остановился на Node baseline 24.22.1 при установленном 26.8.1.
  Использован предусмотренный `UI_NODE_VERSION=26.8.1`; это локальное отличие
  не меняет baseline/CI. Следующий run выявил два `collapsible_if` в новом shell;
  исправлена форма условий, проверки/assertions не ослаблялись.
- APK/приватные файлы устройства сохранены до установки тестовой сборки.
  Отсутствие исходного test-package отдельно учтено. Первая попытка backup
  остановилась на ненулевом `pm path` отсутствующего test-package; после
  явного учёта отсутствия backup прошёл. Никаких UI/данных до backup не меняли.
- Следующий полный Windows-run остановился на `Access is denied` при spawn
  реального CLI; в это время параллельный UI-run пересобирал тот же `.exe`.
  Пересечение сборки/запуска — рабочее объяснение, не доказанная ошибка продукта.
  После завершения CLI build выполнен полный новый host-tests без изменения
  кода/assertions; окончательный результат ниже.
- BUG-011 найден реальным APK restart: после успешных SAF prepare/apply,
  rollback/нового apply и открытия карточки следующий запуск аварийно завершился.
  Logcat прямо указывает `there is no reactor running` в with_automatic;
  новый setup создавал Devices вне runtime. Возвращён прежний block_on вокруг
  создания контроллера. Shared core/API не менялись; новый APK/Device restart
  обязателен, предыдущий APK не считается полным успехом.

## Граница результата

Полная локальная матрица завершена ниже. Ревью, новый CI и интеграция ещё не
выполнены; verified не означает done/полную приёмку P07. Исходные failures
и полные логи сохранены в ignored reports.

## Завершённые проверки Windows и общего UI

- Финальный `bun scripts/check-product.mjs --host-tests`: exit 0,
  542 Rust tests с отдельной поисковой пробой, 28 client tests/308 assertions;
  fmt, clippy `-D warnings`, comment policy/tests, Rust/OpenAPI/TS и
  11 отрицательных типовых проверок passed. Включены 13 emergency tests,
  detach daily policy/files test и 3 новых HTTP/реальный CLI/IPC tests.
  Исходный schema-test failure и последующий spawn PermissionDenied сохранены.
- `bun scripts/check-product.mjs --host-build`: exit 0, release всех
  targets/features, 9 unsafe guards и comment-policy negative/recovery passed.
- `UI_NODE_VERSION=26.8.1 bun scripts/check-ui.mjs --main`: exit 0,
  shell fmt/clippy/tests, vue-tsc, Vite, полный main browser набор и новые
  desktop/touch maintenance tests passed. Сохранено локальное отличие Node
  от baseline24.22.1; прежний chunk-size warning не скрывался.
- Отдельный `UI_BROWSER_PATHS` прогон `check-ui-maintenance.mjs`: Chrome
  desktop/touch и Opera desktop/touch, все четыре passed. Проверены отмена,
  удаление raw, apply с потерей committed reply/точным повтором, rollback,
  новый retry, старый обычный ключ denied, read-only служебный grant/отзыв,
  ручной служебный режим/возврат и неизменные байты полной исходной копии.
- Android APK до исправления BUG-011 успешно прошёл SAF chooser/cancel,
  prepare/cancel, явный apply, raw rollback и новый apply/open карточки.
  Проверка tar/SQLite подтвердила исходные raw-байты, новую локальную сессию,
  отключённые прежние ключи/отсутствующие peers, паузу и recovery hold папки,
  неизменность private и shared полных копий. Полный APK verdict ждёт
  исправленную сборку и реальный повторный restart.

## Завершённые Android/Device проверки и итог

- `bun scripts/check-product.mjs --android`: exit 0, 541 native Rust tests
  с отдельной поисковой пробой, TLS/identity и pending request/restart/cancel/
  no-access пары Windows↔Android, точное равенство контрактов, release всех
  targets/features x86_64/arm64 и 18 unsafe guards passed. Native temp удалён.
- Новый `build-ui-android.mjs x86_64 --pairing-acceptance` после BUG-011: exit 0,
  APK/AndroidTest и manifest passed. Исправление меняет только Android setup,
  41 прочий source fingerprint прежний. Windows/shared проверки не выдаются
  за проверку Android cfg; новый APK/Device проверяет именно исправленный вход.
- Исправленный APK прошёл несколько relaunch на ранее восстановленной базе,
  настройки/ручной enter, новый prepare/apply/retry из той же SAF-копии и
  последующий restart/доступ к пространству. Последняя tar/SQLite сверка
  подтвердила исходные полные копии и проблемные байты, новую сессию, паузу,
  отсутствие peers и запрет автоматической записи до выбора папки.
  BUG-011 resolved локально; исходный SIGABRT/точный reactor panic сохранены.
- Native test helper освобождения SAF был ограничен прежним P0706 prefix:
  первый cleanup вернул IllegalArgumentException. Разрешены ровно прежний
  P0706 и новый P0709 с UUID; новый AndroidTest собран, установлен и подтвердил
  отзыв только собственного URI. Это тестовый код, production APK не меняется.
- Первый restore guard остановился на пустом operations.lock предыдущего
  тестового namespace. Его происхождение доказано epoch из сохранённых
  собственных candidate.sqlite; проверено, что прежние namespaces содержат
  только пустые locks. Whitelist расширен ровно на три доказанных тестовых
  namespace, не на произвольные пути. Исходный guard failure сохранён.
- После cleanup четырёх secret slots/собственного SAF permission исходный APK
  восстановлен с тем же SHA-256. Полный tar inventory: все 204 исходные entries
  восстановлены, changed0/added0. Test-package отсутствовал до проверки и удалён;
  собственная shared-папка удалена. Device штатно закрыт и выключен.
- Пять правил AGENTS.md проверены в полном локальном охвате: cancel/rollback/
  discard/issue/revoke/enter/retry; один API человека и агента; user/agent/
  development/architecture docs; Windows/Android parity; единственный источник
  SQLite/Automerge для пространств и авторитетный внешний служебный журнал,
  общий Rust/OpenAPI контракт. UI/клиентские намерения не становятся хранилищем
  продуктовых данных или отдельной копией схемы.
- M05 not_run→partial с полной локальной матрицей, M06/T10/P07 partial прежние.
  Все 98 определения/97 остальных состояний критериев и прежние 76 problem
  rows сохранены; добавлен и локально исправлен BUG-011. Power-loss, физический
  arm64 runtime и новый CI не заявляются. Весь P07.09 verified локально,
  active_work null; далее ревью/фиксация и интеграция по отдельному поручению.

## Проверяемые исходники

| Файл | SHA-256 |
| --- | --- |
| `api/openapi.json` | `e7aee8dec30d5b63ab576b3a09ee55fb13304ec19e2b9925177fb6d39330d5f2` |
| `api/schema.d.ts` | `06e834a5b51b297bdfe3ffff32c158656e1c58f44550df0629f93862ca185ad1` |
| `api/validate-response.d.ts` | `ac2eca648ed36bda2d5def2d93b8ebfbde0eccb06b9e49189e472ca357314f96` |
| `api/validate-response.js` | `93b923fffe4b48bde3559819ceb77ca03fff3c008c44637f9a9e15f7b2a1c1cb` |
| `crates/adapters/src/bin/zedone.rs` | `ed74cd96a6a9b419b24f8f7eaeb50044eb5d95f0bc53102422f4278e8900c869` |
| `crates/adapters/src/device_control.rs` | `67606b5685a332e1251fd6ee69af1547e4b723ac5744d731c5d59ae7408f36de` |
| `crates/adapters/src/http.rs` | `dd0130432a04fe6b0eccc76baeffb7ff6cf138cca236f59d4f1822aa81648a22` |
| `crates/adapters/src/lib.rs` | `8b25fcbed4cfe4fe254d0282834249e65a3f16b2209ff2d3e83992ebd2b19b5d` |
| `crates/adapters/src/maintenance.rs` | `ab7371bea375d305f19fdde8e1c6bbae942e632c79bf281e382784420741f30e` |
| `crates/adapters/src/network.rs` | `0e60ef77a800efeaeb45ecccebe7fbd2b6771c894d2a7316466a7475b0dcd232` |
| `crates/adapters/tests/maintenance.rs` | `8d49dfd055d725f0c6c04a4186f2e99412a23c00ec6a840308e3369a7856cda9` |
| `crates/core/src/lib.rs` | `4c0b65ffeca95ccbdb435340fd6998700e7f5a9a9b06d853beee576d91d7f407` |
| `crates/core/src/maintenance.rs` | `6d88445720533b6df572a13de86621f2fefeeb47c22b8b9e16e4dcebee63a3ea` |
| `crates/core/tests/contract.rs` | `624cf0eb799b29fe8d265aeb7b33e2602d702e1fe345aae333aa36b7da3d77a6` |
| `crates/storage/src/access.rs` | `2a2129c5b8988c0939b7e703cf81017846d0727768f33ff9191d8ed295feface` |
| `crates/storage/src/backup_catalog.rs` | `a4194f018ea72878e5b16064a482deda367be7be45877b077f8ea90f63cd518c` |
| `crates/storage/src/daily_backup.rs` | `0b8478ba46d55d282d327d4b5d907eb7503ed945c5593c2b51516be034e03996` |
| `crates/storage/src/daily_backup/tests.rs` | `0463734f34751d7018e3ff1969480b6381881344f9d4115d4d1a014144e0e0c2` |
| `crates/storage/src/emergency.rs` | `846475b2fa61d52a2becac16dc90de7e725deeb74ae273196269444b31649fa4` |
| `crates/storage/src/emergency/files.rs` | `93afa814adad4a4e4ad7edb8da604f2703c1dc7c0f4dc0f32f60d89399fffee5` |
| `crates/storage/src/emergency/restore.rs` | `fce7d52f737a6086b3279eba20874af5427e2e90acec61471b9538f41168a1ac` |
| `crates/storage/src/emergency/tests.rs` | `820d6fecaf235da08680f8951b95cba65c8f2dbeaf9a6b40bc4bed346ede47a5` |
| `crates/storage/src/lib.rs` | `3d6355f060438e8df7a178401122180e21e5afd1073ddc52c2dcbc77f1aadade` |
| `crates/storage/src/ownership.rs` | `276a60f0798ac396b0925e10f72f1ee9091f2e8809a04a4da71b1c8159688f67` |
| `crates/storage/src/problem_state.rs` | `7254d36a7f050a1a615ff68081c108ff39a1a7bbb090437b2ac0d14412107e06` |
| `crates/storage/src/problem_state/tests.rs` | `fd6c1097c5ce940c3ff168d53657034c234244ea00447aec0f257b98839a0260` |
| `crates/storage/src/schema/tests.rs` | `36b2b262f974ccc29bdfa3cfc71f0ee7bf08c35f90be2910ee3798e9d68cbe1a` |
| `crates/storage/src/startup.rs` | `9f3ad059259fdfe46ed06bcbbc5c9fc720902580e301b940f59ca2e9404e6c9d` |
| `crates/storage/src/startup/problem_tests.rs` | `829d485ef32042f32afba0f67ef80293723466451875a4c94ace10c5b43dda00` |
| `scripts/check-ui-maintenance.mjs` | `856529550faa04c9d8e41d848942ae94957fe4b3e15151df639bf67230cb3d60` |
| `scripts/check-ui.mjs` | `0eae1eff63c2597d61f90f2c20596e6adacdfdd9a4d7eb129733ace9987ddc42` |
| `ui/src-tauri/examples/maintenance-fixture.rs` | `7dbcbfe8ae6daee497ced1ec6f9597da22062fa457bfda44f19f46bfc5cbe824` |
| `ui/src-tauri/gen/android/app/src/androidTest/java/app/zedone/local/BackupFolderProbe.kt` | `624e9e341fc0d0871fa40e2b614128a72e9b7580a2ab721e3bbbb0a23611a1c4` |
| `ui/src-tauri/src/backup_folders.rs` | `a14e3d4995fa9c8c22e0fe66a1da9fa4bc4b02c538eefb7815dd0447b33d9b5d` |
| `ui/src-tauri/src/devices.rs` | `cad97a8ce27c290548fdccc74f55a363c6e51a6f864bae031bc4311429ddcc2f` |
| `ui/src-tauri/src/files.rs` | `644d84f7208d41729531f764fde76c9baedc7cc11ad707c4b617c46acc0c6936` |
| `ui/src-tauri/src/lib.rs` | `465e4607e05eb00e2f33ef9066580b23c1c3f3d095590167ceee45bf72d48f69` |
| `ui/src-tauri/src/pairing_acceptance.rs` | `b1cbad7cb73ccfb64e68e4d5b1102d8524e395098fad338d9504b005799248b3` |
| `ui/src-tauri/src/web.rs` | `8fc04af075c17a636dcfe47091647b757eb649d3a7a3746f929463d63575afd4` |
| `ui/src/App.vue` | `404654ec269be8d46caec6e2a6deb5218dc31c2090df1ad03bd852202080d81c` |
| `ui/src/MaintenancePanel.vue` | `21b677bf86a05429cec1eed9ebd587daa9efafe7b040f68b14423738ea2f0ef2` |
| `ui/src/maintenance-api.ts` | `a783895a9a2d4c474673ab7f2f6a65d16070540b9f779c9be624c67ddac11b75` |
| `ui/src/style.css` | `cd5dae617375c227ea7adc5e64e61ed27ccf9f9da96741e81dcd7b9dd266797e` |
