# EV-131 — Совместная приёмка P06.07

```yaml
date: 2026-11-18
step: P06.07.4
base_revision: "main/7776b67 + P06.07.2/.3 в feat/p06-07-tag-resolution"
branch: feat/p06-07-tag-resolution
goal: "Независимые создания и назначения тегов сходятся; S05 и поздний мост разрешаются через общий API/UI Windows и Android."
requirements: [T09, A20, A39, A49, S05]
dependencies_evidence: [EV-128, EV-129, EV-130]
scope: [scripts, ui/src-tauri/examples/fixtures/tags.rs, docs]
checks: ["Windows workspace/клиент/контракт/UI desktop+touch", "native Android/обе ABI", "настоящий парный APK/TLS/Keystore", "Device UI/restart/restoration", "CI routing/источники/diff"]
reverse_and_failure: "Оба решения S05, обратные назначения/переименования, stale/replay/конкуренция/перезапуск/атомарный отказ."
result: "done: PR #111/head83cbe32, merge66f389e после25 SUCCESS/fresh CLEAN readiness; LOG-705"
evidence: [EV-129, EV-130]
next: "P06.08 готов по зависимостям; новая реализация — по новому поручению."
```

Новый обязательный CI-срез `pairing-tags` проверяет парный обмен тегов в
собственном стенде с прежними бюджетами. Полный локальный запуск включает
прежние standard/positions и новый tags. Это сохранение приёмки P06.07,
а не возобновление поставленной на паузу оптимизации CI.

## Результаты

| Проверка | Результат |
| --- | --- |
| Windows workspace | Полный product host-tests passed: 395 обычных Rust, отдельный search1000; финальные добавочные проверки включены в отдельные 14 матриц тегов |
| Финальные матрицы тегов | Все 14 passed на Windows и native Android: прежние шесть сходимости плюс восемь решений/прав/сохранности/границ commit |
| Native Android полный набор | 393 обычных Rust, отдельный search1000, оба host↔native TLS/pending probes, равенство OpenAPI, release x86_64 и все восемь unsafe controls/новый comment control passed; удалённый sandbox очищен. Затем финальные 14 матриц включили последний добавочный тест |
| Общий контракт/клиент | 68 операций, актуальные OpenAPI/TypeScript/standalone validator, 11 negative shapes; 27 клиентских тестов/280 assertions passed |
| UI/shell | Vue typecheck/Vite, shell Clippy/3 tests, 13 QR tests; полный Chromium lifecycle/board/touch, keyboard/touch accessibility/обе темы и прежние теги/editor/history/search/trash passed |
| Финальный shared UI тегов | Четыре desktop/touch сочетания обоих порядков S05/имён, оба выбора S05, отмена, stale с сохранением ввода, новое подтверждение, навигация, обратные назначения и reload passed |
| Парный APK/TLS/Keystore | Финальный `check-pairing-app --tags` passed: независимые создания/aliases/исходное написание, S05/четыре пары независимых решений, поздний мост/три исходных ID/полный набор разных имён, сохранность и обратные назначения |
| Device UI | На окончательном x86_64 APK сохранены Личное/Все/Всё, сохранены три назначения; выбран Оставить тег для S05, обычные снятие/назначение снова доступны, текст и описание прежние. Restart сохранил три имени/назначения и доступное перемещение карточки с тегом Связь |
| Android ABI/APK | Product arm64 release и все восемь unsafe/новый comment controls passed; окончательные x86_64 debug acceptance и arm64 production release APK/manifest проверены |
| CI/статические guards | Новый обязательный pairing-tags, полный expected набор теперь 25; Android routing/cache/Rust-default/подготовка shell passed. Бюджеты/scenarios/strict aggregates прежние; source policy 404 и diff passed |

Raw logs, snapshots, оба APK fingerprints и воспроизводимый финальный запуск
хранятся в игнорируемых reports. Полные наборы проверены до последних добавлений
тестов; финальные 14 матриц, adapter tag HTTP/IPC/CLI и Clippy проверены отдельно
на окончательном составе. Последнее UI-изменение — область формы/блокировка доски —
покрыто новым shared UI и окончательным Device APK. Изменений ядра после принятых
native матриц/парной приёмки нет.

## Попытки и сохранность

Первая парная API-матрица прошла, но фиксированная ручная UI-пауза 300s истекла
до сохранения выбора. Результат попытки failed, не принят как полный успех;
исходные данные/АПК восстановлены по SHA-256. Ошибки shell-экранирования @ref,
полного snapshot ref и наступившее при cleanup SESSION_NOT_FOUND сохранены.
Лимиты и проверки не ослаблялись. После окончательной сборки выполнен свежий
полный `--tags` с успешной Device-паузой и cleanup; первая попытка не скрыта.

Финальный driver подтвердил `restored:true`, `cleanup:true`: исходный установленный
APK и каждый app file совпадают с исходными SHA-256, свои OS keys и транспортные
правила очищены. Архив исходного production debug APK на машине восстановлен
побайтово после сборки acceptance. Первоначальный dirty EV-128 также сохранился
побайтово. Эмулятор остановлен инструментом Device, физические устройства не
менялись. Локальные machine data/ключи/снимки/бинарные артефакты не входят в Git.

## Граница приёмки и пять правил

- Обратимость: оба решения S05 и обычные назначение/снятие, переименование после
  полного выбора имён; поздний replay не отменяет новое состояние.
- Агент и человек: прежний Service через HTTP/IPC/CLI и shared UI; нет прямого
  продуктового writer в SQLite или отдельного алгоритма клиента.
- Документация: user/agent/development и ADR-0071 отражают операции и отказы.
- Паритет: Windows/shared desktop+touch/native Android/настоящий APK/Device;
  arm64 собран, runtime на физическом arm64 и Opera здесь не проверялись.
- Единый источник: Automerge-граф пространства, атомарные штатные транзакции
  и receipts; UI и SQL не хранят собственные группы/истину о решениях.

P06.07 и .2–.4 done в интегрированном scope PR #111/66f389e; подробности ниже.
T09 и все критерии остаются в прежних состояниях до нужной полной приёмки и
интеграции; остальные коллизии тегов P06.08, удаления P06.09 и история P06.10
не входят в этот результат. Прежние GAP/DEC сохранены. CI-оптимизация остаётся
на паузе, прежние benchmark лимиты не сбрасываются.

## Публикация поручена / LOG-703

Разработчик поручил PR и при успехе merge/удаление ветки. После fetch база остаётся7776b67, все19 source fingerprints прежние. Пара Before/After снята из реальных base/final сборок на одном raw графе,1440×1400/light; API/сохранность/идентичные изменения и просмотр обоих PNG подтверждены. Media остаются вне репозитория. Нового GitHub CI пока нет; до интеграции нужны все25 expected SUCCESS и fresh exact head/base/feedback readiness. Прежние benchmark лимиты/пауза CI и ограничения приёмки сохраняются.

## PR #111 / LOG-704

83cbe32 опубликован в PR #111/base7776b67. Все42 файла/индекс/noreply/98 definitions-states проверены;19 source fingerprints прежние. Title/body/credit/refs и оба hosted PNG прочитаны обратно, hashes/размер совпадают. Первоначальные22 suites ожидаются/выполняются, три strict aggregates ещё впереди: expected25. Readiness MERGEABLE/UNSTABLE, feedback отсутствует. До всех SUCCESS и свежей exact head/base проверки merge/delete не выполняются; поручение на условную интеграцию активно. Новый CI пока не принят. Этот checkpoint локальный без дополнительного commit/push, прежние ограничения/пауза CI сохраняются.

## Интеграция / LOG-705

[PR #111](https://github.com/Zeyafix/zedone/pull/111), head83cbe32, merged66f389e 2026-11-18 18:44:00UTC после всех25 expected COMPLETED/SUCCESS Windows/Linux/Android attempt1. Проверены точные имена/head/jobs/оба workflows, все14 матриц тегов в raw product logs трёх платформ, по четыре новых shared UI случая обоих host и новый Android pairing-tags. Первоначальная загрузка raw логов остановлена CLI escape guard; рекомендованный --allow-escape-sequences сохранил вывод только в ignored файлы. CI не повторялся.

PR Testing обновлён и прочитан обратно, hosted media/credit прежние. Непосредственно перед merge: head83cbe32/base7776b67, CLEAN/MERGEABLE, reviews/comments/reviewThreads=0. Merge parents ровно7776b67/83cbe32, tree равен tested head. Watch отключён с wasWatching=true/watching=false.

Три dirty checkpoint сохранены и сверены по SHA-256 до/после fast-forward main и switch. Feature-ветка удалена remote с exact-head lease и локально branch-d; tracking ref отсутствует. Main=origin/main=66f389e, index пуст; прочие worktrees не менялись. Все98 definitions/states и прежние GAP/DEC сохранены. P06.07/.2/.3/.4 done; пять правил в указанном scope подтверждены. Далее готов P06.08 по новому поручению, новая реализация не начата. Итоги интеграции — локальный checkpoint без дополнительного commit/push/manual CI; штатные main workflows могут следовать merge. CI-оптимизация остаётся на паузе.