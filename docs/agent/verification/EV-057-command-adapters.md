# EV-057 — P03.05.4: общие адаптеры и клиентские доставки

```yaml
step: P03.05.4
date: 2026-11-05
base_revision: 5c42224fe306ddfc78aa5ffda056dc92381b3d59
branch: feat/p03-05-complete-access
goal: "HTTP/CLI/IPC выполняют ограниченные команды с реальными правами, часами и безопасными повторами."
requirements: [T06, A26]
dependencies_evidence: ["P03.05.1 PR #40", "P03.05.2 PR #46", "P03.05.3 EV-056 в той же ветке по прямому поручению полностью реализовать P03.05"]
scope: ["core access.inspect/access.revoke и storage port", "crate adapters: Service, Axum, CLI, Tauri IPC bridge", "client intentions/HTTP/IPC", "Rust/OpenAPI/TS/standalone response validator, product-check/CI", "ADR-0039 и инструкции"]
checks: ["Windows product-check", "Android native product-check и release обеих ABI с Tauri feature", "HTTP socket/CLI child/IPC parity, replay/regrant/restart/stale/mismatch/denied", "клиентская потеря ответа, timeout, restart, ограниченные повторы и ошибка сохранения"]
reverse_and_failure: "Отзыв → доверенная новая выдача → исходный повтор не отзывает новый ключ. Последний admin защищён; неправильный запрос/права/время не меняют данные."
result: done_merged_pr_47
next: "P04.01 на интегрированном P03.05; полная приёмка P03.06 зависит от P04.10."
```

Ориентир 300 ручных строк превышен из-за одной сквозной границы доставки:
типизированная команда, реальные права/время/receipt, три входа и сохранение
намерения до отправки проверяются вместе. Это оставшийся объём P03.05 по
явному поручению разработчика; P04/P05/P06 не реализуются попутно.

Политика — [ADR-0039](../../architecture/decisions/0039-command-adapters-and-delivery.md).
Финальные результаты приведены ниже.

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
