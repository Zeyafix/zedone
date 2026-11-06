# EV-056 — P03.05.3: доверенное время доступа

```yaml
step: P03.05.3
date: 2026-11-05
base_revision: 5c42224fe306ddfc78aa5ffda056dc92381b3d59
branch: feat/p03-05-complete-access
goal: "Откат наблюдённого времени и истечение перед фиксацией не разрешают операцию или повтор."
requirements: [T06, A08]
dependencies_evidence: ["P03.05.2: PR #46 merged 5c42224, три успешных CI, EV-055"]
scope: ["транзакционные часы storage", "схема v3 через защищённую миграцию", "проверка срока core", "тесты и документация"]
checks: ["Windows product-check", "Android native x86_64 product-check и обе release ABI", "откат/restart/ошибка/истечение/rollback/миграция"]
reverse_and_failure: "Ошибка времени отменяет данные и receipt; новая явная выдача после истечения, часы восстанавливают работоспособность при достижении watermark."
result: done_merged_pr_47
next: "P04.01 на интегрированном P03.05; полная приёмка P03.06 зависит от P04.10."
```

Граница проверки этого шага сохраняется отдельно от адаптеров. Общий запрос
разработчика разрешает последовательную реализацию оставшихся частей в одной
отдельной ветке без промежуточной публикации. Изменение превышает ориентир
300 строк из-за неделимой проверки времени, отката транзакции и миграции.

## Финальные результаты

Windows `bun scripts/check-product.mjs` и Android
`bun scripts/check-product.mjs --android` — **passed**: по 144 Rust-теста
(11 core access, 7 contract, 119 storage, 7 adapters), 11 клиентских тестов
с 70 assertions, release всех targets/features, обе Android ABI с Tauri bridge,
21 независимый отказ unsafe (7 host + 14 Android), Rust/OpenAPI/TS/standalone
validator и compile-only consumers. Четыре отрицательные проверки stale/missing
новых validator-артефактов и точное восстановление — passed.

Итоговая матрица и границы — [EV-058](EV-058-p03-05-acceptance.md).
Runtime/тесты после финальных прогонов не менялись; локальные SHA-256/логи —
`docs/reports/p03-05/verification.json`. Собственный read-only AVD остановлен,
native sandbox удалён. Коммита/PR/новых CI/слияния нет; очередь сохраняет verified.

## Завершение интеграции — LOG-294

2026-11-05 [PR #47](https://github.com/Zeyafix/zedone/pull/47), окончательный
head `7dc69d7389b090eef1c2fd004c7deb9349c6a52c`, интегрирован в
`f4bf4e1cef79dbaeedc0231da9ffcd34b4fa761a` после SUCCESS всех трёх проверок:
Windows, Linux и Android. BUG-004 resolved; исходная ветка удалена локально
и на GitHub. Сверено повторно 2026-11-06; локальные факты публикации —
`docs/reports/p03-05/publication.json`. Прежние указания на отсутствие
коммита/PR/CI/интеграции описывают историческую точку локальной приёмки.
P03.05/.3/.4 done, но полный T06/P03 и продуктовый UI этим не закрыты.
