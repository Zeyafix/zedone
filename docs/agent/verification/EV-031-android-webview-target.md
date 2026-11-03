# EV-031 — Подключение к WebView после перезапуска, CI.02

```yaml
date: 2026-11-03
step: CI.02
base_revision: f173064686f965b61f85e83e083fc69c608fdb0f
branch: fix/android-webview-connection
goal: "Проверять готовность страницы до CDP-подключения и сохранять диагностику подключения до cleanup."
requirements: [T02, T16]
dependencies_evidence: [CI.01, P01.04, EV-016, EV-028]
scope: [prototypes/scripts/check-android.mjs, prototypes/scripts/android-webview-target.mjs, prototypes/scripts/check-android-webview-target.mjs, prototypes/scripts/check.mjs, prototypes/scripts/check-android-ci.mjs, docs/agent, docs/development/ci-checks.md]
checks: ["bun run check", "bun run android:build и bun run check:android на API 33/x86_64", "git diff --check", "реальные Actions после публикации"]
reverse_and_failure: "Обратный коммит возвращает драйвер; отсутствие страницы сохраняет отказ, диагностика не подменяет исходную ошибку; перезапуск и его проверки не пропускаются."
result: "Интегрирован PR #26 в d141d92; три PR checks и main Windows/Linux/Android Actions passed. Первопричина исходного отказа не доказана; GAP-007 in_progress."
evidence: ["main push 41641087962 attempt 1: APK WebView page target timeout", "PR #25 Android 41639517408: success"]
next: "При новом отказе использовать добавленную диагностику; P02.04.2 не блокируется, первопричину не объявлять доказанной."
```

## Исходный отказ

[Android main push 41641087962, attempt 1](https://github.com/Zeyafix/zedone/actions/runs/41641087962/attempts/1)
на f173064 прошёл обе сборки APK, host/Android-пробник тегов, 37/37 нативных
тестов и основную UI/HTTP/IPC-матрицу. После настоящего force-stop/start драйвер
получил отладочный сокет и подключил Playwright, но за 20 секунд не увидел page.
Предшествующий PR-прогон на 9cc7339 прошёл тот же сценарий; новый push-прогон
не является одной из трёх успешных проверок PR #25.

Логи содержат только конец вывода эмулятора; списка CDP targets и logcat момента
отказа нет. Ошибка графического буфера в логе не доказывает причину отказа.
Гипотеза раннего подключения проверяется ограниченным изменением драйвера:
ожидание attachable page из `/json/list` перед CDP вместо готовности одного
сокета. Доказательств, что это единственная причина сбоя, пока нет.

Тайм-аут ожидания остаётся 20 секунд; запуск приложения, проверки сохранности
SQLite/отзыва HTTP-ключа и отказы не повторяются и не пропускаются. При ошибке
подключения до cleanup выводятся ограниченные CDP targets, страницы Playwright,
PID и logcat. Сбой самой диагностики не заменяет исходную ошибку. Чужие
устройства не используются; права, данные и runtime приложения не меняются.

Исходные полные логи, результат повторного запуска и локальные доказательства
хранятся в игнорируемой docs/reports/android-main-f173064/.

## Локальные результаты

| Проверка | Результат |
| --- | --- |
| Windows `bun run check` | Passed: fmt/Clippy, 56 Rust-тестов, release, 14 фаз пробника, контракт/типы, Vue/Vite, HTTP/UI, новая проверка готовности и прежние драйверы |
| `bun run android:build` | Новый x86_64 debug APK и instrumentation APK собраны из текущего дерева |
| Android `bun run check:android` | Passed на AOSP API 33/x86_64: полный UI/IPC/HTTP/камера/SQLite/Back/lifecycle, повторное подключение после force-stop/start, сохранность A/B и отказ старому HTTP-ключу |
| Готовность | Ошибка сокета, HTTP-отказ, неполный JSON, неверный список, пустой список и targets без page/WebSocket не запускают подключение; реальный attachable target допускает продолжение; исходная ошибка deadline сохраняется |
| Статика | Синтаксис всех пяти JS-файлов, полный diff, локальные Markdown-ссылки/ограждения и `git diff --check` проверены |
| Исходный job | Один повтор attempt 2 того же f173064 запущен; результат определяется Actions, не подменяет attempt 1 |

Обратимость — обратный коммит драйвера и прежний cleanup; одинаковая команда
человека/агента, документация и единые версии/контракты сохранены. Runtime,
данные и права приложения не меняются; проверка исполняется на Windows и
Android x86_64. Нового физического/arm64-прогона локально нет; новые Linux
Actions ожидаются. Собственный read-only эмулятор остановлен, временные каталоги
очищены драйвером. GAP-007 остаётся in_progress: первопричина и устранение всех
периодических отказов не подтверждены. CI.02 verified локально, не done.

## Интеграция (LOG-211)

[PR #26](https://github.com/Zeyafix/zedone/pull/26), head 695d573, прошёл три PR checks и интегрирован в d141d92. [Windows/Linux main](https://github.com/Zeyafix/zedone/actions/runs/41645947107) и [Android main](https://github.com/Zeyafix/zedone/actions/runs/41645947211) прошли на merge SHA: 37 нативных тестов, настоящий restart, сохранность SQLite A/B и отказ прежнему HTTP-ключу подтверждены. Исходный job на f173064 attempt 2 снова упал с тайм-аутом page. Исторические ожидания выше заменены этими результатами, но первопричина и устранение всех периодических отказов не доказаны; GAP-007 остаётся in_progress. Исходные ветки удалены.
