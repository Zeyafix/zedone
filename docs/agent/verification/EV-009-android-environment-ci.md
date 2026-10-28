# EV-009 — Android-окружение и эмулятор в Linux Actions

```yaml
step: P00.06
scope: shared_ci
date: 2026-10-28
base_revision: 484a6f3beee3948245a29f5abbb136d284bd6c78
branch: chore/android-environment-ci
goal: "Закреплённый Android toolchain собирает Rust-пример и запускает его в Android 13 AOSP на Linux Actions."
requirements: [T16, T17]
dependencies_evidence: "P00.03 проверен по локальному docs/reports/state.json и android-checks.json; P00.05 интегрирован через PR #5, EV-008."
scope_files:
  - scripts/environment-versions.json
  - scripts/check-android-environment.sh
  - .github/workflows/android-environment.yml
  - docs/development/android-environment-ci.md
  - docs/development/quick-start-windows.md
  - docs/agent/work-queue.md
  - docs/agent/plan-journal.md
  - docs/agent/verification/EV-009-android-environment-ci.md
checks:
  - "bash -n, shellcheck, actionlint, полный diff, git diff --check."
  - "Ошибочная версия/команда, тайм-аут и выход эмулятора; очистка своего AVD и сохранение существующих данных."
  - "Реальный завершённый Linux Actions: версии, четыре ABI, KVM, Android 13/API 33/x86_64, GMS отсутствует, Rust-бинарник возвращает ожидаемый результат."
reverse_and_failure: "Собственный процесс/AVD останавливается и удаляется в EXIT/INT/TERM; ошибки останавливают проверку. Workflow отменяется в Actions, конфигурация возвращается обратным коммитом."
result: "В работе; локальные проверки и реальный CI ещё не подтверждены."
evidence: [EV-008, EV-009]
next: "Проверить Linux Actions для этой ревизии; после интеграции — P00.07."
```

Один вопрос проверки — готов ли Android toolchain и AOSP-эмулятор в Linux CI. Установка SDK, общий скрипт, workflow и доказательства составляют один проверяемый результат; превышение ориентира 300 строк связано с этим неделимым охватом и инструкцией воспроизведения/отмены. Продуктовый код и приёмка APK сюда не входят.

Версии — в [общем JSON](../../../scripts/environment-versions.json), команды и ограничения — в [инструкции](../../development/android-environment-ci.md). Устаревший `sdkmanager` Tools 23 не используется для установки пакетов; проверяются также реальные `source.properties`, а не только код установки.

## Результат

Проверки ещё выполняются. Наличие файлов не закрывает T16; T17 остаётся `partial` до P00.07. Локальные отчёты и исходные диагностические данные находятся только в игнорируемой `docs/reports/`.
