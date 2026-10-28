# EV-015 — P01.03: типизированный HTTP-вызов из Vue

```yaml
step: P01.03
date: 2026-10-28
scope: prototype
base_revision: b1b0a74cdaff299c788c097f83ed0e6f83711782
branch: feat/p01-03-typed-http-client
goal: "Vue вызывает синтетический Rust echo по HTTP с типами из OpenAPI."
requirements: [T01]
requirements_revision: "T01, ADR-0003 и спецификация §12 в base_revision"
dependencies_evidence: ["P01.02 done: EV-014, merge dcdda2d46193eff725afd2567ab420e8ea38214f"]
scope_files:
  - prototypes/
  - .gitignore
  - .gitattributes
  - .github/workflows/prototype-foundation.yml
  - scripts/environment-versions.json
  - docs/development/prototypes.md
  - docs/development/quick-start-windows.md
  - docs/agent/work-queue.md
  - docs/agent/plan-journal.md
  - docs/agent/verification/EV-015-typed-http-client.md
checks:
  - "Чистая установка bun install --frozen-lockfile, Rust fetch --locked."
  - "bun run check: Rust, обе генерации, vue-tsc strict, Vite build и реальный HTTP/браузер."
  - "Повторяемость генерации, stale/missing types, несовместимое изменение Rust, восстановление."
  - "Русский/Unicode/пустой ввод, неверный JSON/тип, неизвестный маршрут, отказ HTTP и повтор."
  - "Целевые Chrome/Opera при доступности; Windows/Linux CI подготовить, не объявлять пройденным до запуска."
reverse_and_failure: "Echo не хранит данные; очистка формы и повтор после отказа. Генерация сначала в памяти, запись только после успеха. Сервер только loopback, процессы проверок останавливаются."
result: "P01.03 done: Windows/Linux Actions пройдены, PR #10 интегрирован в main."
revision: 3c929177a2fb49ff7721dd6658d8353257b5e9ff
integration_revision: c4a1cee7f5cb40207f0a876b234f7c9e9537e00d
pull_request: https://github.com/Zeyafix/zedone/pull/10
report: https://github.com/Zeyafix/zedone/actions/runs/40704452219
evidence: ["Локальные логи и отпечатки: docs/reports/p01-03/ (игнорируются)"]
next: "P01.04: минимальный APK и вызов общего Rust echo через IPC; зависимость интегрирована, реализация не начата."
```

Один проверяемый срез соединяет Rust-контракт, транспорт и потребителя Vue.
Превышение ориентира 300 ручных строк допустимо для этой неделимой проверки:
без HTTP, генерации, клиента и автоматического обнаружения рассогласования
результат P01.03 не проверяется. Lock-файлы и производные типы считаются отдельно.
APK, Tauri, хранение, прикладные полномочия и измерения остаются в будущих шагах.


## Результат и среды выполнения

Реализован только синтетический echo: Rust-модели/операция из P01.02,
loopback HTTP на Axum/Tokio, Vue-форма и openapi-fetch с производными типами.
Vite проксирует относительный путь `/prototype/echo`; внешний bind и CORS не включены.
Rust/JavaScript-зависимости имеют отдельные `Cargo.lock` и единственный `bun.lock`.
Версии библиотек закреплены в манифестах; версии runtime — только в общем JSON.

Генератор openapi-typescript 7.13.0 выполняется программным API внутри Bun;
Vite 8.3.5, установка Playwright/его драйвер также запущены явным Bun.
TypeScript 5.9.3 выбран по peer `^5.x` генератора вместо несовместимого нового major.
Vue 3.5.44 и openapi-fetch 0.17.0 проверены в собранном интерфейсе.

Выявлена необходимость Node.js для `vue-tsc` 3.3.13: прямой запуск под Bun не
подключает обработку `.vue` (`TS2307` для `App.vue`); под Node те же исходники
проходят. Исследованный код Volar перехватывает `fs.readFileSync` при загрузке
TypeScript; Bun загружает модуль своим механизмом, и расширение не подключается.
Не вводился обход загрузчика и не скрывалась ошибка декларацией `*.vue`/`any`.
Для этого шага выбран Node LTS 24.22.1, отдельно проверяемый runner; это разрешённый
ADR-0003 результат проверки совместимости, а не смена менеджера пакетов или
runtime остальных инструментов. Локально portable-архив проверен по официальному
SHA-256 и использован только в PATH процесса; существующий системный Node не менялся.
В готовом браузерном интерфейсе Node.js не требуется.

## Выполненные проверки

- `bun run check`: rustfmt, Clippy с `-D warnings`, три Rust-теста, release-сборка,
  сверка OpenAPI и TypeScript, `vue-tsc --noEmit`, Vite build и HTTP/браузерный сценарий.
- Повтор в новой копии без `target/`, `node_modules/` и `dist/` после
  `cargo fetch --locked` и `bun install --frozen-lockfile`; lock-файлы не изменились.
- Реальный HTTP и production bundle в Chrome и Opera на Windows, в отдельных
  временных профилях без изменения пользовательских; дополнительно Chromium,
  который устанавливается workflow. Точные версии/пути и логи остаются локальными.
- Пустая строка, русский текст, комбинируемый Unicode, emoji, переносы/кавычки и
  буквальный HTML возвращаются точно. Rust отклоняет неверные поля/типы, сломанный
  JSON и Content-Type; проверены неизвестные путь и метод.
- В браузере проверены очистка формы/ответа, внедрённые драйвером HTTP 503 и обрыв
  запроса, сохранение ввода, отсутствие ложного результата и успешный явный повтор.
  Это моделирование отказов транспорта, не проверка физического отключения сети.
- Повторная генерация даёт одинаковые байты. Отсутствующие/устаревшие типы,
  устаревший OpenAPI и несовместимое serde-переименование Rust-поля обнаруживаются.
  После регенерации клиент с прежним полем не проходит проверку; восстановление
  Rust и обеих генераций возвращает исходные артефакты и успешную проверку.
- Ошибка Rust-компиляции не перезаписывает артефакты. Проверены отказ неверным
  версиям Bun/Node и неизвестному аргументу; ошибка в Vue-шаблоне обнаруживается
  именно проверкой типов, а не только сборкой.
- Документированные `bun run serve`/`bun run dev` выполняют Vue HTTP-вызов;
  остановка wrapper прекращает Rust-процесс. Неверные аргументы HTTP-сервера дают отказ.
  Ошибка запуска браузера тоже закрывает сервер проверки.
- Workflow проверен actionlint; ссылки/Markdown/JSON, сохранность 98 критериев,
  полный diff и исключение машинных отчётов проверены локально.

CI расширен установкой закреплённого Node, frozen-установкой JavaScript-пакетов и
Chromium. Windows/Linux выполнили ту же общую команду после публикации: PR/push
Actions прошли на head, указанном выше. PR #10 слит в main; подробности ниже.
T01 остаётся `partial`, DEC-001 открыт; Tauri, Markdown, Android WebView/IPC/APK,
прикладной паритет платформ и измерения не проверены. Это не блокировки локального
объёма P01.03 и не доказательство пригодности полного стека.

Обратимость подтверждена в пределах формы/производных артефактов и остановки
процессов; echo не меняет хранимых данных. Человек и агент используют один HTTP
и одну Rust-операцию. Документация обновлена; Rust остаётся источником моделей,
OpenAPI и TypeScript производны. Прикладной паритет Windows/Android ожидает своих шагов.

## Основания технической проверки

- [openapi-typescript: программный API](https://openapi-ts.dev/node).
- [openapi-fetch: клиент из производных типов](https://openapi-ts.dev/openapi-fetch/).
- [Vue language tools: vue-tsc](https://github.com/vuejs/language-tools/tree/master/packages/tsc).
- [Node LTS: официальный дистрибутив](https://nodejs.org/dist/v24.22.1/).
- [Axum: JSON/маршрутизация](https://docs.rs/axum/0.8.9/axum/).


## Интеграция

- [PR #10](https://github.com/Zeyafix/zedone/pull/10) слит в `main` 2026-10-28
  в 14:25:05 UTC; head `3c929177a2fb49ff7721dd6658d8353257b5e9ff`,
  merge `c4a1cee7f5cb40207f0a876b234f7c9e9537e00d`.
- [PR Actions 40704452219](https://github.com/Zeyafix/zedone/actions/runs/40704452219)
  и [push Actions 40704424567](https://github.com/Zeyafix/zedone/actions/runs/40704424567)
  успешно выполнили полную цепочку на Windows/Linux: Rust, обе сверки контрактов,
  строгие Vue-типы под закреплённым Node, сборка Vite под Bun и HTTP/браузер в Chromium.
  Все восемь проверок push/PR, включая Windows/Android environment workflows, успешны.
- Перед merge подтверждены ожидаемый head, `MERGEABLE / CLEAN`, отсутствие комментариев,
  отклоняющих ревью и невыполненных правил ветки. Внешнее ревью не требовалось и не
  проводилось; административный обход не использован. Описание PR содержит фактические
  CI-результаты и два проверенных UI-вложения GitHub вне Git-дерева.
- Head подтверждён в `origin/main`; локальный main обновлён fast-forward.
  Ветка `feat/p01-03-typed-http-client` удалена на GitHub и локально.
  Итоговая запись изменяет только документы состояния, не проверенные код и версии.
- P01.03 — `done`; T01 остаётся `partial`, DEC-001 открыт. Android environment
  не проверяет контракт в APK, WebView/IPC или паритет приложения. Приёмка P00
  сохраняет границы EV-012. Следующий шаг — P01.04; он пока `planned` и не начат.
