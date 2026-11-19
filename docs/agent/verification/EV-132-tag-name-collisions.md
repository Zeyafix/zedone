# EV-132 — Коллизии имён тегов P06.08.1

```yaml
date: 2026-11-19
step: P06.08.1
base_revision: main/66f389e; пять исходных незакоммиченных документов сохранены
branch: feat/p06-08-tag-collisions
goal: "Явный атомарный выбор имён после двух независимых переименований или переименования против создания."
requirements: [T09, A52, A53]
dependencies_evidence: [EV-128, EV-129, EV-130, EV-131]
scope: [core, storage, ui, scripts, docs]
checks: [Windows, native Android, shared desktop/touch UI, paired APK/Device]
reverse_and_failure: "Назначения/ID сохраняются; после решения обычное переименование, назначение и снятие; stale/replay/конкуренция/атомарный отказ."
result: "done: PR #112/headc06431e mergeda5501e1 после25 SUCCESS/fresh CLEAN readiness; LOG-714"
next: "P06.08 интегрирован; далее готов P06.09 по новому поручению. Полный T09 отдельно."
```

Охват P06.08 разделён до дальнейшей реализации: .1 — имена, .2 — удаление
против назначения, .3 — совместная приёмка. Части выполняются последовательно
в одной авторизованной ветке, без промежуточной публикации. Размер расширенной
матрицы объясняется сохранностью ID/назначений и независимых решений.
Новые зависимости не нужны. Прежние GAP/DEC и состояния критериев сохранены.

Финальные Windows/native Android 35 матриц тегов, оба случая A52/A53 и aliases,
конкурирующие/поздние решения, Unicode, старые сценарии, сохранность/обратные
действия passed. Общие HTTP/IPC/CLI и shared desktop/touch passed. Настоящий
paired names child прошёл с прежними240s (76.0s), restoration/cleanup=true.
Окончательный Device-проход сохранил Все/Всё и Дом/Работа; после restart прочитаны
те же пять имён и доступны обычные действия. [EV-134](EV-134-tag-collisions-acceptance.md).


## Публикация / LOG-713

[PR #112](https://github.com/Zeyafix/zedone/pull/112), headc06431e/base66f389e,
опубликован с проверенной реальной парой Before/After. Все32 исходных fingerprints
сохранились, hosted PNG сверены по SHA-256/размерам. GitHub CI начат и ещё не
принят; прежние результаты выше сохраняют свой scope. Watch не включён по
поручению разработчика, merge/удаление ветки не выполнялись. Review/интеграция
по отдельному поручению; полный T09 отдельно. Итоги — локальный checkpoint
без дополнительного commit/push.


## Интеграция / LOG-714

[PR #112](https://github.com/Zeyafix/zedone/pull/112), headc06431e, mergeda5501e1
после всех25 expected SUCCESS Windows/Linux/Android attempt1 и свежей
CLEAN/MERGEABLE readiness без reviews/comments/threads. Raw CI подтвердил
все36 матриц тегов трёх платформ, обе host UI и полный pairing-tags из трёх
изолированных стендов. Дерево merge равно tested head; main обновлена, пять
dirty документов сохранены SHA-256, ветка удалена remote/local/tracking, watch off.
P06.08/.1/.2/.3 done в этом scope, определения/состояния98 критериев прежние.
Пределы локальных проверок и прежние отказы выше сохраняются; полный T09 отдельно.
Итоговые записи локальные без дополнительного commit/push.
