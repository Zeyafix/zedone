# EV-014 — P01.02: Rust-контракт и генерация OpenAPI

```yaml
step: P01.02
date: 2026-10-28
scope: prototype
base_revision: 22159bdc6f69d1e890b2028f28d8cf03ae6dc121
branch: feat/p01-02-openapi-contract
goal: "Из Rust получить воспроизводимый OpenAPI и обнаружить устаревший артефакт."
requirements: [T01]
requirements_revision: "ADR-0003 и T01 в base_revision"
dependencies_evidence: ["P01.01 done: EV-013, merge 95ea45cd3878609b24b7050263d6737fa39c0b3f"]
scope_files:
  - prototypes/
  - .gitattributes
  - .github/workflows/prototype-foundation.yml
  - docs/development/prototypes.md
  - docs/agent/work-queue.md
  - docs/agent/plan-journal.md
  - docs/agent/verification/EV-014-openapi-contract.md
checks:
  - "bun run generate:openapi дважды: одинаковые байты."
  - "bun run check: fmt, Clippy, Rust-тесты, release build и сверка OpenAPI."
  - "В отдельной копии: изменение Rust-схемы, устаревший/отсутствующий артефакт, ошибка компиляции, восстановление генерацией."
  - "Новая копия без target, cargo fetch --locked и общая проверка."
  - "Статическая проверка workflow и документации, git diff --check."
reverse_and_failure: "Echo не меняет состояния; артефакт удаляется и восстанавливается генератором. Неуспешная компиляция не перезаписывает его."
result: "P01.02 done: Windows/Linux Actions пройдены, PR #9 интегрирован в main."
revision: 614e9229cef356ad71bc35b56785976b6ee8c3b3
integration_revision: dcdda2d46193eff725afd2567ab420e8ea38214f
pull_request: https://github.com/Zeyafix/zedone/pull/9
report: https://github.com/Zeyafix/zedone/actions/runs/40699124122
next: "P01.03: Vue/TypeScript/Vite, генерация типов и типизированный HTTP-вызов."
```

Охват — синтетический echo без хранения и сетевого сервера, Rust-модели запроса
и ответа, описание операции utoipa, генератор и проверка отслеживаемой схемы.
HTTP, TypeScript, Vue, клиент и Android IPC остаются в следующих шагах.
Сгенерированный файл не является отдельным источником истины: он получается из
Rust и не редактируется вручную. Изменение укладывается в один вопрос проверки.
Небольшое превышение ориентира 300 ручных строк связано с единым срезом:
контракт, обнаружение устаревшей схемы и доказательства нельзя принять отдельно.
Сгенерированные OpenAPI и lock-файл учитываются отдельно.

## Результат и ограничения

Закреплены utoipa 6.0.0 и serde 1.0.229, для Rust-тестов — serde_json 1.0.154;
транзитивные зависимости закреплены общим `Cargo.lock`. Bun запускает Cargo,
генератор выполняется как Rust-бинарник; Node.js и JavaScript-зависимости не нужны.
Схема OpenAPI 3.1.0 хранится в `prototypes/openapi.json`, окончания LF закреплены
в `.gitattributes`. Источник моделей и операции — только Rust.

Локально пройдены `bun run check` (fmt, Clippy с `-D warnings`, три Rust-теста,
release build, сверка схемы) и повтор в новой копии без `target/` после
`cargo fetch --locked`. Два запуска генерации дали одинаковые байты.
В отдельной копии переименование JSON-поля через serde изменило обязательное поле
и свойства обеих схем ожидаемым образом; проверка обнаружила старый артефакт,
регенерация восстановила соответствие. Ручное изменение/удаление JSON отклоняется
без его перезаписи проверкой; явная генерация возвращает исходные байты.

Ошибка компиляции останавливает генерацию без изменения файла и общий прогон
до тестов/сборки. Неверная версия Bun и неизвестный аргумент отклоняются.
Очистка своего Cargo target и повторная сборка проверены; внешний
`CARGO_TARGET_DIR` не перенаправляет выход. Lock-файл не изменился.
Проверены actionlint, синтаксис Bun, ссылки/блоки Markdown, JSON, сохранность всех
98 критериев и их состояний, полный diff и `git diff --check`.
Сырые журналы и отпечатки находятся только в игнорируемой `docs/reports/p01-02/`.

T01 остаётся `partial`, DEC-001 остаётся открытым. Windows/Linux Actions пройдены;
Android/APK, HTTP, клиент и производительность не проверены. P01.02 интегрирован;
P01.03 не начат. Приёмка P00 не меняется.

Правила проекта: echo не меняет состояния и не требует обратной операции;
генерируемый файл восстанавливается, сборка очищается. Человек и агент используют
одни команды и контракт. Документация актуализирована; паритет Windows/Android
ещё не доказан. Ручных копий моделей нет, источники версий и состояний сохранены.

## Интеграция

[PR #9](https://github.com/Zeyafix/zedone/pull/9) слит в `main` 2026-10-28
в 13:48:55 UTC, merge `dcdda2d46193eff725afd2567ab420e8ea38214f`.
[PR Actions 40699124122](https://github.com/Zeyafix/zedone/actions/runs/40699124122)
и [push Actions 40699067352](https://github.com/Zeyafix/zedone/actions/runs/40699067352)
завершились с `success` на указанном head: Windows и Linux выполнили общие проверки
контракта. Отдельные [PR Android environment](https://github.com/Zeyafix/zedone/actions/runs/40699124347)
и [push Android environment](https://github.com/Zeyafix/zedone/actions/runs/40699067602)
тоже прошли; это проверка окружения, а не контракта в APK или паритета приложения.

Перед merge подтверждены ожидаемый head, `MERGEABLE / CLEAN`, успешные проверки,
отсутствие комментариев и отклоняющих ревью. Правила ветки и обязательное внешнее
ревью отсутствовали; внешнее ревью не проводилось, административный обход не применялся.
После проверки включения head в `origin/main` локальный `main` обновлён fast-forward;
рабочая ветка удалена локально и на GitHub. Запись результата меняет только
документацию; проверенные код, workflow, зависимости и схема остаются прежними.
