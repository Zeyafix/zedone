# EV-110 — возобновление обмена после обрыва и несовместимости

Рабочая карточка P06.04, 2026-11-17; LOG-590.

- База: main/54a95bc, отдельная `feat/p06-04-sync-recovery`.
  Исходные четыре dirty документа интеграции P06.03 сохранены в ignored baseline.
- Цель: после восстановления связи или совместимости продолжить доставку
  сохранённых изменений без повторных объектов/истории, автоотвязки и снятия паузы.
- Требования: T09/A03/A05/A47/A48/S07, разделы 1/6 спецификации,
  ADR-0008/0028/0064. Зависимость P06.03 done в базе; EV-109/LOG-589.
- Объём: общий отказ несовместимому протоколу/формату, диагностика UI/API,
  восстановление доставки по сохранённым hashes, проверки обрывов/повторов/
  перезапуска и автоматического продолжения, документация.
- Проверки: product-check Windows/native Android/обе ABI, полный UI runner,
  парный TLS и APK/Keystore runner через Device. Точные результаты ниже
  заполняются только после выполнения.
- Обратимость/ошибки: локальные правки при недоступном peer; повтор до/после
  фиксации; SQL rollback; pause/resume/ABA; неизменность выбора, доступа и данных.
- Граница: все конфликты P06.05–.11, настоящий Tailscale и измерения отдельно.
  Поддерживается только protocol 1 / space format 1; новые форматы, выпущенные
  старые приложения и миграции не выдумываются. Полные T09/A05 не закрываются
  одним транспортным срезом.
- Поручение: реализация и локальная проверка P06.04 в отдельной ветке.
  Commit/push/PR/merge не выполняются в рамках этого запуска.
- Объём больше ориентира 300 строк: основной прирост — одна матрица
  восстановления (TLS fixtures, SQLite/process exit и APK/UI проверки).
  Она отвечает на один вопрос: продолжает ли сохранённая доставка работу
  после устранения отказа без потери данных и отмены явной паузы.
  Независимых прикладных операций или рефакторинга в состав не добавлено.

## Результат

P06.04 done: [PR #102](https://github.com/Zeyafix/zedone/pull/102), head 7ae016e,
merge bbfdbbc после всех 17 SUCCESS и свежей CLEAN readiness; LOG-597.
Полный APK runner и ручная Android UI-приёмка завершены по LOG-592.
Все прежние определения/состояния 98 критериев и GAP/DEC сохранены;
полные T09/конфликты/Tailscale/измерения отдельно. GAP-028 остаётся открыт.

Общий `incompatible_sync` отличает неподдержанный протокол/формат от ошибки
запроса, сети и явной паузы; права/выбор/пауза проверяются до допуска транспорта.
Чужой строковый формат не меняет локальный граф, историю или progress.
Push ack дополнительно проверяет исходный base и complete. UI показывает общую
причину и убирает прежнее сообщение успеха при новом действии/ошибке.
Схема базы, формат документа, ACL и механизм доставки ADR-0064 не менялись.

## Проверки

| Охват | Результат |
| --- | --- |
| Полный `bun scripts/check-product.mjs` Windows | Passed: 364 Rust, 27 client/280 assertions, policy/Impact, fmt/Clippy, release/unsafe/comment guards, Rust/OpenAPI/TS/validator и 11 отрицательных type assertions. |
| Полный `ui:check` | Passed на разрешённом локальном Node override; baseline не менялся. Chrome/Opera desktop/touch, прежняя полная матрица, новый текст несовместимости без старого успеха и сохранность, автоматическая доставка с закрытой вкладкой, pause/reverse/reload/resume. |
| TLS recovery suite | Passed: потеря запроса/ответа второй страницы, потерянный push ack, неверный base/незавершённый ack, reopen базы, отсутствие повторных changes, сохранение поздней правки; настоящий shutdown/reopen сервера и продолжение таймером Controller без нового подключения; несовместимые protocol/format и auto-resume, отдельная сохранённая пауза/restart, pause/resume ABA во время ответа, текущие права до диагностики. |
| SQLite/process exit | Passed Windows/native Android: получение/push до/после commit, согласованные graph/history/progress, сохранение локальной серии, SQL fault с перехваченным отказом и полный rollback, повтор без лишней истории/отмены поздней отметки, неподдержанный формат без записи. |
| Полный native `--android` | Последовательный итог passed: 361 native Rust, 27 client/280 assertions, обе release ABI/unsafe/comment guards, парные TLS/pending probes, одинаковый host/native контракт и cleanup. Первоначальный LNK1104 после 361 passed был отказом host-сборки из-за параллельной APK-фикстуры; после её завершения не повторился. |
| Windows Credential Manager ↔ APK/Keystore/TLS/настоящий CLI | Финальный полный runner passed, result.json status=passed/restored=true/cleanup=true. Прежняя pairing-матрица и новый protocol refusal, реальный обрыв reverse-транспорта, offline card edit, Keystore process restart, та же пауза/подключение, явный resume и история без дубликатов; ручной Device UI-checkpoint также passed. Прежний exit 1 из-за ошибки вызова UI-драйвера остаётся исторической неудачной попыткой. |
| Android UI через Device | Passed на том же сохранённом acceptance APK: включение, пауза обоих направлений, disable/enable без снятия паузы, process restart с тем же selected/paused, явное возобновление и обновлённый текст. Все касания/прокрутка/relaunch через device_open agent-device, экран осмотрен device_screenshot. Проверена входящая UI-роль; исходящий TLS/API recovery и общий browser UI сохраняют отдельный охват. |
| Обе production APK ABI | Passed: x86_64 и arm64 debug/androidTest/manifest без pairing-acceptance feature. Физический arm64 runtime не проверялся. |
| Сохранность | После APK runner исходный APK и каждый app file восстановлены по SHA-256; его finally очистил собственные OS slots/rules и завершил host. Исходные dirty EV-109/ADR-0064 побайтово сохранены; журнал/очередь дополнены поверх baseline. Все 98 определения/состояния и прежние GAP/DEC сохраняются. |

Начальные проверки обнаружили не обновлённые перечисления нового error code
в Rust/TypeScript fixtures. Они дополнены без отключения строгой проверки
контракта и без изменения 11 ожидаемых отрицательных TS ошибок. Исходный
LNK1104 связан с одновременной работой host exe APK-фикстуры и его пересборкой;
полный повтор native-проверки после её cleanup passed, без изменения кода,
прав, таймаутов или матрицы. Снимок исходных app files отдельно совпал с
остановленным приложением; runner теперь закрывает его до будущих backup.
Неудачные логи сохраняются в ignored reports и не выдаются за успех.

## Пять правил и передача

- Обратимость: выбор/отключение, pause/resume, сохранность и повторная доставка
  подтверждены в общем API; новых однонаправленных пользовательских действий нет.
- Человек/агент: общий core/Service/DeviceHost и один код ошибки через UI/IPC/HTTP/CLI.
- Документация: user/agent руководства, developer матрица и ADR-0065 обновлены.
- Паритет: Windows/native Android и настоящий APK API подтверждены в указанном
  объёме; свежая ручная Android UI-приёмка также passed по LOG-592.
- Источник истины: прежние SQLite/Automerge и локальные настройки/progress;
  второе хранилище статуса/прав и миграции не добавлены.

По поручению разработчика LOG-592 подготовить PR, при полном успехе CI и
свежей readiness точных head/base выполнить merge и удалить ветку. Все связанные
итоговые записи P06.03 включаются вместе с P06.04. Новый CI не объявляется passed
до завершения. P06.05, полные конфликты/Tailscale/производительность отдельно.

Агрегат SHA-256 текущих 18 source/contract файлов: `efb68407ffa45f2dee282e7f002089ae89a2e377b13b9222e2da9746dfe4c8ec`. Точный состав, raw logs, APK/data manifests и fingerprints — ignored reports. Итоговые policy (359 source files), fmt, diff, ссылки, сохранность исходных EV-109/ADR-0064 и всех 98 определений/состояний проверяются финальным audit.

## Доприёмка и публикация — LOG-592

- Разработчик поручил PR, при успехе merge/delete. Полный diff относительно
  main/54a95bc проверен; чужих правок не обнаружено. Все 18 source/contract
  SHA-256 совпали с ранее проверенным агрегатом, продуктовый код не менялся.
- Новый полный APK runner прошёл вместе с Device UI-checkpoint. Правила
  Device соблюдены; использованы quoted refs, обычная прокрутка и свайп по
  осмотренному экрану. После перезапуска выбранное пространство осталось
  на явной паузе до отдельного resume. Все исходные APK/app files восстановлены
  по SHA-256, свои OS slots/rules очищены; production output восстановлен,
  свой эмулятор выключен. Все пять правил подтверждены в scope P06.04.
- Для PR сняты настоящие статические Before/After из точной базы и текущей
  версии: успешное получение, остановка TLS listener и ручной отказ обмена.
  В базе остаётся прежний успех рядом с ошибкой; в результате показан только
  текущий отказ. Общие данные/права, dark theme, кадр 1280×1080; обрезка одного
  и того же участка настоящих full-page снимков, без замены содержимого.
  Медиа/исходники capture/body вне Git. Новых анимаций/переходов нет.
- P06.04 verified локально. Публикация/новый CI/интеграция выполняются далее;
  текущие результаты не означают полный T09 или проверку будущих форматов.

## Публикация — LOG-593

[PR #102](https://github.com/Zeyafix/zedone/pull/102), head
7ae016e48b0ebd0d8a5abb8dbbefcd8b6b9a3cd2, base main/54a95bc опубликован
с 27 проверенными файлами. Показаны реальные Before/After 1280×1080, hosted
bytes сверены SHA-256; медиа вне Git. Начальная readiness MERGEABLE, замечаний
нет, 14 matrix checks выполняются; полный expected набор 17 ещё не завершён.
P06.04 verified локально, CI не объявляется passed. Ожидание через штатный watch,
затем свежая readiness и условный merge/delete по поручению разработчика.

## CI attempt 1: Windows touch search — LOG-594

[Windows shell job](https://github.com/Zeyafix/zedone/actions/runs/44025459072/job/133844584267)
на head 7ae016e завершился failure до pairing/exchange: touch search ждёт
обновления после закрытия редактора на check-ui-search.mjs:83, timeout 10000ms.
Диагностика правильной страницы: mounted/complete, connection confirmed,
busy false, results unavailable; space.list/read HTTP 200 valid 333/466ms,
card.list/column.list/tag.list без headers и ERR_ABORTED через 9988ms.
Симптом добавлен в открытый GAP-028; общая первопричина пока не доказана.
Строгий Windows aggregate корректно передал failure. Выполняется полный
локальный UI-прогон; код/права/таймауты/assertions/матрица/head не меняются.
Один ограниченный CI rerun после проверки не закрывает GAP-028 сам по себе.
Все 98 определения/состояния сохраняются; до полного CI/readiness merge нет.

## Перепроверка после CI failure — LOG-595

Полный `bun run ui:check` завершён без изменения кода: desktop/touch browser
matrix, включая оба search-сценария, pairing/exchange и все предшествующие
QR/typecheck/build/fmt/Clippy/shell Rust проверки passed. Raw log:
`docs/reports/p06-04-checks/ui-after-ci-failure.log`; assertion/process/cleanup
failure отсутствуют. Причина CI-сбоя не установлена, GAP-028 остаётся открыт.

Один `gh run rerun 44025459072 --failed` принят для Windows shell и зависимого
aggregate на прежнем 7ae016e; завершённый результат ожидается через штатный
watch. Проверки/таймауты/права/матрица неизменны, нового commit/push нет.
Testing PR отражает первоначальный failure и локальный успех; это не закрывает
GAP-028. До всех 17 SUCCESS и свежей readiness merge/delete не выполняются.

## Уведомление Windows aggregate — LOG-596

[Aggregate job 133848933786](https://github.com/Zeyafix/zedone/actions/runs/44025459072/job/133848933786)
относится к run_attempt 1: `Require every Windows suite` получил `RESULT:
failure` после прежнего shell failure и корректно завершился exit 1. Нового
тестового отказа в этом job нет. Raw log сохранён в
`docs/reports/p06-04-checks/ci-windows-aggregate-attempt1.log`.

Единственный ранее запрошенный rerun уже выполняется как attempt 2, Windows
shell job 133851460927 на прежнем 7ae016e. Дополнительного rerun и изменений
нет; ожидание результата через существующий watch, merge пока заблокирован.

## Интеграция — LOG-597

- Все 17 expected checks на неизменном head 7ae016e COMPLETED/SUCCESS:
  [Windows/Linux attempt 2](https://github.com/Zeyafix/zedone/actions/runs/44025459072),
  [Android attempt 1](https://github.com/Zeyafix/zedone/actions/runs/44025458922).
  Единственный повтор failed Windows shell/aggregate прошёл. Первый отказ
  честно сохранён в Testing; GAP-028 открыт, причина не доказана. Код,
  assertions, права, таймауты и матрица не менялись.
- Raw Windows shell подтвердил baseline Node 24.22.1, оба search-сценария,
  полный desktop/touch и pairing/exchange. Android shell подтвердил настоящий
  APK/Keystore/TLS/API/CLI, SHA-256 restoration и cleanup; CI host memory
  fixture сохраняет отдельный scope от локального Credential Manager/Device.
- Testing прочитан обратно; финальная readiness точных head/base 7ae016e/
  54a95bc OPEN/non-draft/MERGEABLE/CLEAN, точный набор 17 SUCCESS,
  reviews/requests/comments/inline comments/threads отсутствуют.
- Pinned-head merge создал bbfdbbc49d4dea6d20bb687e979a4681ac56a3e9.
  Проверены родители base/head и равенство merge tree reviewed head
  (00cb7968f78abc3e866cdc2613021d4f246ce905). Main=origin/main/bbfdbbc;
  feature-ветка удалена local/remote/tracking. Worktrees/stash и исходные
  checkpoint docs сохранены, source fingerprints прежние.
- Пять правил подтверждены в согласованном scope. Все 98 определения/состояния
  и прежние GAP/DEC сохранены. P06.04 done; P06.05 planned, готов по зависимостям,
  реализация не начата. Итоговые записи остаются локально без commit/push;
  новых runtime проверок неизменного кода после merge не требуется.
