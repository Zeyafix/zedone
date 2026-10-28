# EV-008 — Windows-проверка окружения в Actions

```yaml
step: P00.05
scope: shared_ci
date: 2026-10-28
base_revision: 45809edf38f299dbb0d963d9f3f93740c3b31dca
branch: chore/windows-environment-ci
goal: "Одинаковые команды проверки Rust/MSVC и Bun локально и в Windows Actions с завершённым результатом."
requirements: [T16, T17]
dependencies_evidence: "P00.02/P00.04 проверяются по локальному docs/reports/state.json; машинные данные не публикуются."
scope_files:
  - scripts/environment-versions.json
  - scripts/check-windows-environment.ps1
  - .github/workflows/windows-environment.yml
  - docs/development/quick-start-windows.md
  - docs/agent/work-queue.md
  - docs/agent/plan-journal.md
  - docs/agent/verification/EV-008-windows-environment-ci.md
checks:
  - "PowerShell-синтаксис, actionlint, полный diff и git diff --check."
  - "Общий скрипт: версии, Bun runtime, Cargo build/run/fmt/Clippy; отказ внешней команды и очистка своего примера."
  - "Завершённый запуск Windows workflow на зафиксированной ревизии."
reverse_and_failure: "Удаление собственного временного примера в finally; ненулевой код останавливает проверку; workflow отменяется в Actions и удаляется обратным коммитом."
result: "В работе; результат Actions пока не получен."
evidence: [EV-008]
next: "После Windows CI — интеграция P00.05 и отдельный P00.06 (Linux/Android)."
```

Объём укладывается в один вопрос проверки: работает ли базовый Windows toolchain в CI теми же командами, что локально. Сборка приложения, браузерная приёмка и Android CI сюда не входят. Компоненты Visual Studio/SDK и версия образа hosted runner фиксируются журналом запуска; совпадение с установкой Windows 11 не предполагается.

Workflow использует стандартный `windows-2025`, `contents: read`, отключённое сохранение Git credentials, закреплённые SHA actions, тайм-аут 15 минут и отмену устаревшего запуска. Кеши и загрузка артефактов не добавлены. Репозиторий публичный: стандартные hosted runners [используются бесплатно](https://docs.github.com/en/billing/concepts/product-billing/github-actions). При изменении видимости нужно заново проверить квоту до запуска.
