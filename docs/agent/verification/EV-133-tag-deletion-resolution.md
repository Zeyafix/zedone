# EV-133 — Решения удаления тега P06.08.2

```yaml
date: 2026-11-19
step: P06.08.2
base_revision: main/66f389e + P06.08.1
branch: feat/p06-08-tag-collisions
goal: "Удаление против независимого назначения сохраняет оба явных решения через общий API/UI."
requirements: [T09, A40]
dependencies_evidence: [EV-131, EV-132]
scope: [core, storage, adapters, ui, scripts, docs]
checks: [Windows, native Android, HTTP/IPC/CLI, shared UI, paired APK/Device]
reverse_and_failure: "После сохранения назначение/снятие/новое удаление; удаление — согласованное исключение без restore. Старые и новые связи, права/replay/stale/конкуренция/commit checkpoints."
result: "verified локально; LOG-711, без commit/push/PR/CI"
next: "Review diff и интеграция по поручению разработчика; полный T09 отдельно."
```

Добавлены `deletion_conflict` и `tag.resolve_deletion` с bool `keep` и свежим
токеном тега/aliases/связей. Сохранение возвращает прежние и новые назначения;
удаление снимает все назначения через общий причинный фронт. Собственные
конфликты карточки не разрешаются этой командой. Старые delete-generated false
читаются через их причинных предшественников в том же Automerge-графе.
[ADR-0072](../../architecture/decisions/0072-tag-name-and-deletion-collisions.md).

Предварительные 33 Windows матрицы тегов passed. Первый новый тест сохранности
ошибочно сравнивал позицию второй карточки с первой (Legacy1/Legacy0); ожидание
исправлено на отдельный исходный snapshot каждой карточки, затем обе ветви passed.
Полная приёмка, включая добавочный тест прав/собственного текста, записывается EV-134.

Итог: финальные35 матриц Windows/native Android, общий успешный HTTP/IPC/CLI,
все четыре пары решений/aliases/старые и новые связи/чужие конфликты/права/replay/
commit failures passed. Отдельный paired deletion child прошёл с прежними240s
(101.1s), обе ветви и restoration/cleanup=true. Финальный Device выбрал сохранить,
после restart обычные действия доступны, имена/связи не потеряны. Удаление
остаётся согласованным исключением без restore; первое снятие/назначение через
Device прошло в исходной попытке, её общий timeout не скрыт. Подробности EV-134.
