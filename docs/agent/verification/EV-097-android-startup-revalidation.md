# EV-097 — CI.10: повторный отказ начального сеанса Android

Дата: 2026-11-14. PR #91/head 019e286, GAP-020, T08/T16.

## Рабочая карточка

- Цель: исследовать отказ первого экрана Android и повторно проверить полный
  Android workflow на том же head без ослабления условий.
- Зависимость: P05.05 done, прежняя startup-диагностика EV-082/PR #82.
- База: feat/p06-01-2-pairing-protocol/019e286; исходные dirty документы
  сохранены с SHA-256 и binary patch в ignored локальном отчёте.
- Scope: raw CI logs, локальная проверка свежего x86_64 APK через Device,
  восстановление исходных APK/данных, полный Android rerun и документация.
  Продукт/драйверы/попытки/10s/бюджеты/CI-матрица не меняются.
- Проверки: job/aggregate logs, Device initial session/menu/Back/themes/
  relaunch, SHA-256 исходных файлов/APK, все 17 expected checks перед merge.
- Обратимость: временный unconfigured profile заменяется сохранёнными
  исходными APK и product/WebView файлами; темы возвращены к system.
- Результат: локальная проверка указанного среза passed, исходный timeout
  не воспроизведён. Полный Android workflow attempt 2 запрошен, pending.
- Далее: дождаться полного набора всех 17 SUCCESS/readiness точного head;
  при повторном отказе разбирать новые raw logs, не считать rerun исправлением.

## Исходный отказ

Run 43619417851/attempt 1, Android shell job 132521473732:
unconfigured-shell → checkShell:78 → visible:10 не дождался «Доступ не настроен»
за 10000ms. До подготовки синтетических пространств/прикладных действий не дошёл.
Правильная страница сохранена до cleanup: complete/Vue mounted/Tauri bridge,
connection «Проверяем доступ», busy=false, session=timeout, pageErrors пусты.
Error-level Chromium/AndroidRuntime log пуст. Это не доказывает, на какой
границе задержался IPC; причина неизвестна. Сообщения EGL после отказа сами
по себе не доказывают причинность.

Strict Android aggregate получил RESULT=failure и корректно отказал;
отдельной ошибки в нём нет. Остальные пять Android suites SUCCESS, включая
product-native/arm64 и shell-arm64. Все десять Windows/Linux checks того же
head SUCCESS. Новый отказ повторяет точку прежнего GAP-020; общая причина
с c286cc4 не доказана. GAP-020 возвращён в in_progress, без стирания истории.
Связь с Windows GAP-028 не установлена.

## Локальная проверка

- Собран текущий x86_64 debug APK, manifest identity/categories passed.
- До проверки сохранены оригинальный APK и 23 product/WebView файла с SHA-256.
  Временно подготовлен unconfigured profile; API/UI данные не выданы сами себе.
- Запуск/повторный запуск, касания, меню/Back и темы выполнены возвращённой
  Device командой agent-device. Device screenshot и AX snapshot подтвердили
  «Доступ не настроен». Меню/Back, dark/light/system и сохранение dark при
  relaunch прошли. Это ограниченная проверка первого экрана, не полный domain UI.
- Новый APK локально не воспроизвёл исходный CI timeout. Не добавлены
  автоматические повторы или увеличение срока в продукт/драйвер.
- Исходный APK переустановлен, его установленные байты сверены SHA-256.
  Все 23 исходных файла восстановлены по именам/SHA-256; приложение закрыто,
  Device выключен штатными инструментами. Окружение/IME не менялись.
- Изменение line endings gradlew, сделанное CLI при сборке, не имело
  семантического diff и восстановлено. Новых source commits/push нет.

Один полный rerun всего Android workflow запрошен на неизменном 019e286,
без --failed и без изменения проверок. Его pending результат не заменяет
первоначальный failure. После запроса повторного workflow Android checks
временно отсутствуют в rollup, и CLEAN при десяти host checks не разрешает
merge: нужен полный expected набор 17 успешных checks.

PR body обновлён фактическим отказом/локальной проверкой/ожидающим rerun и
прочитан обратно. Пять правил в прежнем scope: обратимость подтверждена
восстановлением; общий API и авторитетные данные не меняются; docs актуальны;
native Device подтверждает только перечисленный срез. Приёмка 98 критериев
и GAP-028/прочие DEC не повышаются. Записи расследования остаются локально
без нового source push; наблюдение PR сохраняется.


## Отложенное уведомление aggregate — LOG-528

Jobs API job 132524184381 подтвердил run_attempt=1; raw log RESULT=failure/
exit 1 передал уже известный shell startup отказ. Текущий полный workflow —
attempt 2 IN_PROGRESS на том же head. Нового отказа rerun или отдельной
ошибки aggregate нет. Дополнительный повтор/изменение source не выполнялись.


## Интеграция — LOG-529

[PR #91](https://github.com/Zeyafix/zedone/pull/91) интегрирован: head
019e286745ae32c5b8cc407de5d4f46e03101f9b, merge
e34b8fd493550ab8860a1542228e7836a91842b0 после всех 17 COMPLETED/SUCCESS и
CLEAN readiness. Body с фактическим успехом Android attempt 2 прочитан обратно;
comments/reviews/requests/review threads отсутствовали.

Raw Windows/Linux shell подтвердили final read lifecycle guard и полный
desktop/touch UI. Android attempt 2 подтвердил начальный unconfigured session
IPC, весь прежний native domain/history/cleanup/accessibility/Markdown и
cleanup; обе ABI/product/prototype/strict aggregates SUCCESS.

Дерево merge равно checked head, main обновлена; feature-ветка удалена
локально/на GitHub/tracking. Двенадцать исходных dirty файлов сохранены
побайтово при переключении; восемь исходных EV прежние, индекс пуст,
другие worktrees/stash не менялись. Записи завершения остаются локально.

P06.01.2.1/CI.09/CI.10 done в согласованном API/diagnostic/revalidation scope,
P06.01.2/P06 in_progress. Пять правил в этом scope соблюдены. GAP-020 и GAP-028
остаются in_progress: причины startup/read timeout не доказаны, новый полный
успех не доказывает устранения периодических отказов. Полные A51/T09,
защищённый канал/host secrets/запрос/подтверждение/отвязка/формы/две реплики
отдельно. Далее готов P06.01.2.2 по новому поручению; новая реализация не начата.
