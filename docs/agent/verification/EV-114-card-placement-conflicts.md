# EV-114 — P06.06.1: конфликт места карточки

## Карточка

- База: main/904ac992861a5d839f3d6e30da93180dabcbd8e2, ветка
  `feat/p06-06-position-conflicts`. Три исходных dirty документа интеграции
  P06.05 сохранены побайтово в ignored baseline; EV-113 не меняется.
- Цель: явно выбирать целое сохранённое место одной карточки, сохраняя
  содержимое и независимые решения, через один UI/HTTP/IPC/CLI контракт.
- Требования: T09/A35/A38/S01/S04/S08, product-spec §1/§1.2 п.1/4/8,
  ADR-0045. Зависимости P06.04 (EV-110/bbfdbbc) и P05.05
  (EV-082/1a71b72) done и доступны в базе.
- Объём: чтение вариантов с авторитетным соседом, выбор сохранённой пары
  колонки/ранга, общий dispatch/контракт, UI, проверки двух реплик и docs.
  Позиции колонок P06.06.2 и итоговый родитель отдельно.
- Проверки: Windows product-check, полный desktop/touch UI, native Android
  product-check, обе APK ABI и Device UI; фактические результаты ниже.
- Обратимость/отказы: отмена без записи, обычная move после решения;
  права/отзыв, stale/replay/mismatch, SQL rollback/restart, независимые решения
  вновь конфликтуют. Другие конфликты карточки не снимаются выбором места.
- Размер: общий сценарий включает модель выбора/контракт/форму и матрицу
  двух реплик/платформ. Генерируемые файлы учитываются отдельно; колонка и
  совместная приёмка выделены до реализации.
- Поручение: реализация и локальные проверки; commit/push/PR/merge отдельно.

## Результат

P06.06.1 done: [PR #106](https://github.com/Zeyafix/zedone/pull/106), head 3b2a174,
merge 806d8e6 после всех 17 SUCCESS в Windows/Linux/Android attempt 1 и свежей
CLEAN readiness. Merge tree совпал с reviewed head; main обновлена,
feature-ветка удалена local/remote/tracking. Все 98 определения/состояния
и прежние GAP/DEC сохранены, GAP-028/031 открыты; полный P06.06/T09 отдельно.
Нижеследующие локальные результаты и попытки сохраняют свой точный охват.

## Технические находки и попытки

- Ограниченный двухрепличный тест выявил особенность закреплённого Automerge:
  put уже наблюдаемого победителя записывает del остальных операций без нового
  присваивания. Старое чтение исторических memberships ошибочно ожидало значение
  у каждой placement-операции. Диагностический вывод подтвердил set/set/del.
  Исторические del теперь пропускаются при сборе принадлежности; сами прежние
  set и полная причинная история сохраняются.
- Явное решение использует отдельный PlacementStore: delete и новое put внутри
  одной транзакции с receipt. Это сохраняет новое независимое решение, даже если
  выбрано прежнее побеждающее значение. Обычная move не менялась. Четыре финальных
  storage-сценария passed: оба вида мест, разные/одинаковые решения, обратимость,
  restart/replay, сохранность/другой конфликт, SQL rollback, одинаковые соседи.
- До финального прогона исправлены ошибки новых тестов: повтор ID чтения для
  записи, некорректный JSON macro, Clippy wrapper, сценарий с фактическим noop
  вместо независимой move и ожидание количества radio до завершения чтения.
  Диагностические изменения удалены. Assertions, попытки и таймауты продукта
  не ослаблялись; все сырые логи остаются в ignored reports.
- GAP-031 фиксирует отдельный отсутствующий regression выбора уже сохранённого
  победителя в прежних name/text сценариях. Код этих операций не менялся;
  воспроизведённый дефект названий/текста не заявляется. Это не блокирует .1,
  но не позволяет считать полный T09/S04 закрытым без дополнительной проверки.
- Vue/TS и четыре новых browser-сценария desktop/touch прошли отдельно. Полные
  проверки Windows/browser/native Android/production APK и парного Device runner
  завершены в объёме таблицы ниже; частичные результаты сами по себе не заменяют
  полный продуктовый T09.


## Итоговая локальная матрица — LOG-614

| Проверка | Фактический результат |
| --- | --- |
| Windows product | Passed: 379 Rust (18 core, 7 contract, 279 storage, 75 adapters), 27 client/280 assertions, 28 comment-policy tests, fmt/Clippy -D warnings, release всех workspace targets/features, 7 unsafe guards и новый source-comment guard, точные OpenAPI/TS/standalone validator, 11 отрицательных type assertions. |
| Native Android x86_64 | Все 376 Rust passed, включая новый HTTP/IPC/настоящий CLI и все четыре storage-сценария. Release/7 unsafe guards/source-comment guard passed. Последующая сборка host probe остановилась LNK1104: собственный ещё работающий парный server держал тот же Windows exe. Это не успешный полный runner; remote sandbox удалён его finally. |
| Дополнение native Android | После окончания собственного server оба прежних checkChannelPair (TLS/pin/persisted identity и pending/restart/replay/cancel/no-access) passed с тем же проверенным native binary. Его SHA-256 сохранён в ignored proof. Нативный export-contract побайтово совпал с текущим OpenAPI, новый sandbox удалён. Условия/assertions/timeouts функции не менялись. |
| Android arm64 | Отдельный штатный product --android-arm64 passed: release всех targets/features, 7 unsafe guards, source-comment guard и клиент/контракт/type assertions. Это сборка/guards, не runtime физического arm64. |
| Общий UI | Полный bundled Chromium и Opera browser runner passed: весь прежний UI, новые desktop/light и touch/dark cross/same-column cases, просмотр/блокировки, два варианта/нет автоселекции, отмена без изменения, stale-отказ, выбор, независимая правка соседа, сохранность истории/ID, обычная move и reload. Полный ui:check также прошёл scanner и pairing UI. Отдельно все четыре новых сценария passed в установленном Chrome. |
| Production APK | Обе обычные production debug APK x86_64/arm64 и AndroidTest/manifest guards passed. Acceptance APK построен отдельно; обычный x86_64 artifact затем восстановлен по SHA-256. |
| Настоящий парный APK | Windows Credential Manager ↔ Android APK/Keystore/TLS/API/CLI passed: оба вида мест, выбор целой пары, независимые разные решения, одинаковые соседи для разных карточек, повторная доставка, неизменность текста/тега/выполнения/истории и обратная move. Весь прежний pairing/QR/field-conflicts/recovery/права/revoke/restart прошёл. |
| Device UI | Passed через выданный agent-device: спорная карточка вне обычных колонок, доступный просмотр/заблокированное редактирование, две неподтверждённые radio-альтернативы с колонкой/соседом, отмена и повторное открытие, явный выбор первого места. После process restart единственная карточка в первой колонке перед Соседом А, с прежним текстом и обычной move. Проверены тёмная тема и настоящий IPC; физический телефон не использовался. |
| Сохранность | Финальный paired result status=passed/restored=true/cleanup=true, exit 0. Исходный APK и каждый app file восстановлены по SHA-256, свои OS slots/rules очищены. Один собственный пустой каталог удалён только rmdir; исходный production artifact восстановлен. Эмулятор выключен через Device. |

Все machine paths, ключи, raw logs, APK, снимки и hashes находятся только в
ignored reports. Node 26.8.1 — прежний локальный override; baseline 24.22.1
не менялся. Для Bun использован прежний локальный launcher с абсолютным путём
того же test file; состав проверок и бюджеты сохранены.

Device-команда один раз получила INVALID_ARGS из-за PowerShell @ref без
кавычек; свежий snapshot и ref в кавычках сработали. Локальный UI-driver затем
ошибочно ожидал отсутствия всей aria-области «Конфликты» после сохранения,
хотя она штатно сохраняла сообщение об успехе. Сохранение уже прошло; свежие
snapshot/Device screenshot и relaunch подтвердили обычную карточку в выбранном
месте, checkpoint принят только после этого. Производственный код и проверки
ради этих диагностических ошибок не менялись.

Последнее уточнение paired-checker поставило разные X/Y между одинаковыми
соседями до перемещения спорной карточки; именно этот вариант прошёл настоящим
APK runner. Rust/UI/контракт после их итоговых проверок не менялись. Финальные
source fingerprints (28 файлов)/полный diff/ссылки/98 определений и состояний
проверены перед передачей; исходный dirty EV-113 побайтово сохранён. Новых
нарушений локальных ссылок нет, comment policy passed для 375 source files.

## Пять правил и передача

- Обратимость: отмена без доменной записи, новая обычная move после решения,
  сохранённая история/ID/теги/выполнение; неизвестное намерение повторяется прежним.
- Человек/агент: один Service/ACL/replay и Rust-контракт UI/HTTP/IPC/CLI; отдельного
  обхода состояния, прав или SQLite для агента нет.
- Документация: пользовательские moving/cards, агентские placement-conflicts/cards,
  разработческий порядок, ADR-0068, этот EV и журнал/очередь актуализированы.
- Windows/Android: обе реальные runtime-проверки и Device UI в указанном объёме;
  обе production APK ABI собраны. Физический arm64/полный T09 не заявляются.
- Источник истины: прежний Automerge register и SQLite transaction/receipt;
  подпись места вычисляет ядро, UI не вычисляет ранг и не хранит независимые варианты.

Active_work null после LOG-614. Далее рассмотрение/интеграция P06.06.1; P06.06.2
запланирован на интегрированной базе. Совместный P06.06, полный T09, настоящий
Tailscale/целевой телефон/измерения отдельно. GAP-028 остаётся открыт, отдельный
непроверенный name/text сценарий — GAP-031. Публикация не выполнялась.


## Подготовка публикации — LOG-615

Разработчик поручил PR, при успехе merge и удаление ветки. Base origin/main
остаётся 904ac99; все 28 source fingerprints прежние, локальные результаты
LOG-614 сохраняют свой охват. Связанные исходные записи интеграции P06.05
включаются в PR. Реальные Before/After используют одинаковые граф/данные,
viewport/theme/framing; PNG/body вне Git, host binaries восстановлены по SHA-256.
Нового CI ещё нет; до merge нужны все expected SUCCESS и новая readiness точных
head/base без замечаний. P06.06.1 verified, все определения/состояния и GAP/DEC
сохранены; P06.06.2/полный P06.06/T09 отдельно.


## Публикация — LOG-616

[PR #106](https://github.com/Zeyafix/zedone/pull/106), head
3b2a17455f395d1437295f98d344e253a0e63e40, base main/904ac99. 38 файлов,
обычные commit/push/pre-push passed, noreply и состав проверены. Title/body/credit
прочитаны обратно; два hosted PNG 1280×1500 подтверждены по SHA-256, все медиа
вне Git. Все 28 source fingerprints прежние.

Новый CI выполняется: 14 suites running, три aggregate ещё ожидаются. Начальная
readiness OPEN/non-draft/MERGEABLE/UNSTABLE, feedback отсутствует. Это не финальный
успех. До merge/delete по поручению нужны все 17 expected SUCCESS и свежая
readiness exact head/base без замечаний. Состояния критериев/GAP/DEC прежние,
checkpoint локальный без дополнительного commit/push. Колонка .2 и полный
P06.06/T09 отдельно.


## Интеграция — LOG-617

На неизменном head 3b2a174 полный exact набор 17 checks COMPLETED/SUCCESS.
Windows/Linux run 44113062660 и Android run 44113062809 завершены в attempt 1.
Raw logs подтвердили baseline Node 24.22.1, все четыре новых desktop/touch
случая на каждом host OS, штатный полный native runner, настоящий APK обмен
местами/независимые решения/одинаковые соседи/reverse и restoration/cleanup.
CI memory host не заменяет отдельный локальный Windows Credential Manager и
Device UI. Первоначальный локальный LNK1104 сохраняется как попытка, не успех.

Testing PR обновлён и прочитан обратно. Свежая readiness exact head/base
3b2a174/904ac99: OPEN/non-draft/MERGEABLE/CLEAN, все 17 SUCCESS и отсутствие
reviews/requests/comments/inline threads. Pinned-head merge создал
806d8e6f7b90a1aeb464bd122205d7757cc316c9, родители равны проверенным base/head,
merge tree совпадает с reviewed head. Main=origin/main; feature-ветка удалена
remote с lease и local branch-d, tracking ref отсутствует. Два прежних worktrees
и отсутствие stash сохранены, dirty checkpoint документы сохранены побайтово.

P06.06.1 done, пять правил подтверждены в scope. Все 98 определения/состояния
и прежние GAP/DEC сохранены; GAP-028/031 открыты. Итоговые journal/queue/EV/ADR
записи остаются локально без нового commit/push и нового CI для checkpoint;
source равен reviewed merge tree. Active_work null; далее P06.06.2 planned,
готов на интегрированной базе, новая реализация не начата. Полный P06.06/T09,
прочие ограничения и дополнительные проверки отдельно.
