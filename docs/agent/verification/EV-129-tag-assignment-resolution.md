# EV-129 — Явное решение назначения/снятия тега

```yaml
date: 2026-11-18
step: P06.07.2
base_revision: "main/7776b67; три исходных dirty документа сохранены SHA-256"
branch: feat/p06-07-tag-resolution
goal: "Выбрать назначение или снятие одной спорной связи, сохранив остальные данные и конфликты карточки."
requirements: [T09, S05]
dependencies_evidence: ["P06.07.1 done, PR #110/7776b67/EV-128"]
scope: [crates/core, crates/storage, crates/adapters, api, client, ui, scripts, docs]
checks: ["workspace fmt/Clippy/tests", "контракт/клиент", "desktop/touch UI", "native Android/обе ABI", "общая приёмка P06.07.4"]
reverse_and_failure: "Оба булевых решения; последующие обычные назначения/снятия; stale/replay, независимые решения, отсутствие частичного commit."
result: "done: PR #111/head83cbe32, merge66f389e после всех25 SUCCESS/fresh CLEAN readiness; EV-131/LOG-705"
evidence: [LOG-699]
next: "P06.07 завершён в интегрированном scope; далее P06.08 по новому поручению."
```

Размер включает один неделимый сценарий: чтение причинных вариантов связи,
явное решение через общий API/UI, безопасные повторы и проверка двух платформ.
Новые зависимости и отдельный источник состояния не вводятся.

## Локальная проверка

- Оба булевых выбора через alias, сохранность текста/места/выполнения, обычное
  обратное действие, точный replay после позднего изменения и SQLite restart.
- Одинаковые и разные независимые решения в обоих порядках повторной доставки;
  новый конфликт разрешается новой командой. Выбор текущего видимого значения
  создаёт причинную операцию и не теряет независимый противоположный выбор.
- Решение связи сохраняет независимый конфликт текста. Read доступен субъекту
  Read, решение требует ReadWrite; отзыв запрещает даже успешный старый receipt.
- Ошибки на AfterWork/AfterChanges/AfterResult/BeforeCommit сохраняют исходный
  граф и receipt; AfterCommit даёт unknown, точный повтор возвращает receipt.
- Финальные матрицы прошли Windows и native Android; shared UI проверил оба
  выбора desktop/touch, запрет обычной правки до решения и обратные назначения.
  HTTP/IPC/CLI строго проверяют новые поля и stale; общий контракт содержит
  68 операций, TypeScript и валидатор сгенерированы из Rust.

Полная парная и Device-приёмка отражается в EV-131. Новых commit/push/PR/CI
и интеграции этим локальным результатом не заявляется. Пять правил соблюдены
в этом срезе: обратные связи, общий API, документация/ADR-0071, Windows/Android
и единственный авторитетный Automerge-граф.

## Интеграция / LOG-705

Шаг done в составе PR #111/head83cbe32, merge66f389e после всех25 expected SUCCESS Windows/Linux/Android attempt1 и fresh exact head/base CLEAN/MERGEABLE readiness без feedback. Raw CI logs подтверждают все14 матриц тегов трёх платформ, обе host UI и новый pairing-tags. Дерево merge равно tested head; main обновлена, feature-ветка удалена remote/local/tracking, watch off. Полный результат/локальная Device-приёмка/ограничения сохранены в EV-131; определения/состояния критериев и прежние GAP/DEC не менялись. Итоговые записи локальные без дополнительного commit/push.