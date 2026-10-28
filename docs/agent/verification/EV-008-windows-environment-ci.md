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
result: "Windows CI passed; P00.05 verified в ветке, интеграция ещё не выполнена. T16/T17 partial."
revision: 372ad2529a2b6b570cb2d012a45efce5643f53ff
report: https://github.com/Zeyafix/zedone/actions/runs/40658724101
evidence: [EV-008]
next: "После Windows CI — интеграция P00.05 и отдельный P00.06 (Linux/Android)."
```

Объём укладывается в один вопрос проверки: работает ли базовый Windows toolchain в CI теми же командами, что локально. Сборка приложения, браузерная приёмка и Android CI сюда не входят. Компоненты Visual Studio/SDK и версия образа hosted runner фиксируются журналом запуска; совпадение с установкой Windows 11 не предполагается.

Workflow использует стандартный `windows-2025`, `contents: read`, отключённое сохранение Git credentials, закреплённые SHA actions, тайм-аут 15 минут и отмену устаревшего запуска. Кеши и загрузка артефактов не добавлены. Репозиторий публичный: стандартные hosted runners [используются бесплатно](https://docs.github.com/en/billing/concepts/product-billing/github-actions). При изменении видимости нужно заново проверить квоту до запуска.

## Выполненная проверка

[Запуск 40658724101](https://github.com/Zeyafix/zedone/actions/runs/40658724101), событие `push`, ревизия `372ad2529a2b6b570cb2d012a45efce5643f53ff`: `completed / success`. Windows job выполнялся 2026-10-28 с 08:42:29 до 08:43:30 UTC. Образ `windows-2025-vs2026`, версия `20261021.278.1`; PowerShell 7.6.7, Visual Studio 18.11.12315.225, SDK libraries 10.0.26100.0. Rust 1.99.1 MSVC x64 с rustfmt/Clippy и Bun 1.4.4 установлены из общего файла версий. Все шаги, включая Cargo build/run/fmt/Clippy и Bun runtime, прошли. Это сведения одноразового CI runner, не машины разработчика.

PowerShell-синтаксис, actionlint 1.7.12, ссылки/блоки Markdown, сохранность 98 требований и diff-check пройдены. Проверка отказов подтвердила ненулевое завершение при коде Cargo 23 и при неверной версии Bun; собственный временный Rust-пример удалён при успехе и отказе. После проверенной ревизии меняются только записи результатов, не workflow, скрипт или версии.

Журналы запуска доступны по ссылке; текущая политика хранения — 90 дней. Для последующей перепроверки запускайте workflow заново на нужной ревизии. T16 остаётся `partial` до Linux/Android CI, T17 — до полного воспроизведения. P00.05 находится в `verified`, PR/слияние в рамках этого поручения не выполнялись. Дальше — интеграция этой ветки и P00.06 при выполненных локальных зависимостях; продуктовая приёмка и результаты других машин не изменены.
