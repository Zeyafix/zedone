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
  - .gitattributes
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
  - "Ошибочная версия Bun и отказ компилятора с кодом 23; очистка своего AVD/каталога и сохранение существующих данных."
  - "Реальный завершённый Linux Actions: версии, четыре ABI, KVM, Android 13/API 33/x86_64, GMS отсутствует, Rust-бинарник возвращает ожидаемый результат."
reverse_and_failure: "Собственная группа процессов получает TERM, затем KILL после ожидания не более 10 секунд; AVD удаляется в EXIT/INT/TERM, исходный код ошибки сохраняется. Workflow отменяется в Actions, конфигурация возвращается обратным коммитом."
result: "Linux CI passed; P00.06 verified в рабочей ветке, интеграция ещё не выполнена."
revision: 711b4f68577524eefe87e6bb9608ee80bcff7b60
report: https://github.com/Zeyafix/zedone/actions/runs/40668559737
evidence: [EV-008, EV-009]
next: "Интегрировать P00.06; после подтверждения интеграции — отдельный P00.07."
```

Один вопрос проверки — готов ли Android toolchain и AOSP-эмулятор в Linux CI. Установка SDK, общий скрипт, workflow и доказательства составляют один проверяемый результат; превышение ориентира 300 строк связано с этим неделимым охватом и инструкцией воспроизведения/отмены. Продуктовый код и приёмка APK сюда не входят.

Версии — в [общем JSON](../../../scripts/environment-versions.json), команды и ограничения — в [инструкции](../../development/android-environment-ci.md). Устаревший `sdkmanager` Tools 23 не используется для установки пакетов; проверяются также реальные `source.properties`, а не только код установки.

## Результат

Статические проверки прошли: Bash syntax, ShellCheck 0.11.0, actionlint 1.7.12 с ShellCheck обоих workflows, ссылки/блоки Markdown и diff-check. Windows-хост корректно отклоняется Linux-скриптом. Все 98 требований приёмки сохранены.

Первый [run 40665979821](https://github.com/Zeyafix/zedone/actions/runs/40665979821), `6014c45`, выявил отсутствующую `libpulse.so.0`; добавлена явная установка `libpulse0`. В [run 40666405685](https://github.com/Zeyafix/zedone/actions/runs/40666405685), `f9d7039`, четыре ABI, KVM, Android 13/API 33/x86_64 без GMS и запуск бинарника прошли, но ожидание QEMU после TERM зависло. Этот и заменяющий `27ac7fd` прогоны отменены, успешными не считаются. Исправление `5653900` использует собственную группу процессов и ограничивает ожидание TERM перед KILL.

[Run 40667317193](https://github.com/Zeyafix/zedone/actions/runs/40667317193), `5653900`, и диагностический [run 40667974279](https://github.com/Zeyafix/zedone/actions/runs/40667974279), `175bf09`, подтвердили smoke и завершение cleanup. Отрицательная проверка остановилась раньше компилятора с кодом 11: после остановки эмулятора разовая ACL `/dev/kvm` сбросилась. Исправление `711b4f6` задаёт правило udev для владельца текущего UID с режимом `0600` только в одноразовой VM.

Финальный [run 40668559737](https://github.com/Zeyafix/zedone/actions/runs/40668559737), `711b4f68577524eefe87e6bb9608ee80bcff7b60`, завершился `completed / success`. Job выполнялся 2026-10-28 с 09:59:43 до 10:02:38 UTC на `ubuntu-24.04`, образ `20261012.334.1`.

Подтверждены закреплённые версии SDK/NDK, Java, Bun и Android CLI; Rust 1.99.1 собрал пример для четырёх Android ABI. AOSP Android 13/API 33/x86_64 загрузился с KVM и без Google Play Services, уникальное имя AVD совпало через adb, бинарник вернул `ZEDONE_ANDROID_TOOLCHAIN_OK`. Cleanup завершился; в конце job оставался только общий сервер adb, эмулятора среди оставшихся процессов нет.

Отрицательные проверки прошли: неверный Bun отвергнут; compiler failure вернул исходный код 23; собственный временный каталог удалён, существующий каталог AVD с контрольным файлом сохранён. Повторный доступ к KVM после остановки AVD подтверждён этим вторым вызовом скрипта.

Повторный [Windows CI 40665979798](https://github.com/Zeyafix/zedone/actions/runs/40665979798), `6014c45`, завершён успешно после изменения общего JSON; последующие изменения Windows-проверку и версии не затрагивают.

P00.06 — `verified`: проверенный код доступен в отдельной ветке, PR и интеграция не выполнялись. T16 остаётся `partial` до интеграции Linux CI, T17 — до P00.07. APK, полный стек Tauri/Gradle, физический Galaxy A32 и продуктовая приёмка не проверялись. Локальные отчёты, JSON и полные журналы прогонов находятся только в игнорируемой `docs/reports/`.
