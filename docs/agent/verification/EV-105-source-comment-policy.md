# EV-105 — исходники без комментариев и отказ сборки

Дата: 2026-11-16. Внеплановый технический шаг TECH-001 по прямому поручению
разработчика. Уточнение scope: только собственные файлы.

## Рабочая карточка

- База: main/210a0f5, чистое рабочее дерево; отдельная ветка
  `chore/remove-source-comments`.
- Цель: удалить обычные и документирующие комментарии из собственных исходников
  и останавливать штатные Cargo/Vite/Android Gradle сборки при их появлении.
- Scope: источники из общей политики; сторонний код, зависимости, wrappers,
  лицензии и документация сохраняются. Изменение не продолжает P06.02.
- Зависимости: существующие закреплённые Rust/Bun/Node/Java/Android toolchains;
  новые прямые JS dev-зависимости закреплены в корневом lockfile.
- Обратное действие: удалить добавленный комментарий; отмена правила описана
  в ADR-0061. При отказе сборка не изменяет продуктовые данные и исходники.
- Документация: [правило](../../development/source-comments.md),
  [инструкция агента](../source-comments.md),
  [ADR-0061](../../architecture/decisions/0061-source-comment-policy.md).

## Проверки

| Проверка | Результат |
| --- | --- |
| `bun run check:comments` | 0 комментариев в 338 собственных исходных файлах; новые файлы проверяются без Git staging |
| `bun run test:comments` | 21 тест, 43 assertions: строки/regex/raw strings, вложенные комментарии, Kotlin/JS interpolation, Vue шаблоны/директивы, HTML/XML/CSS, SQL, shell/heredoc/command substitution, PowerShell, ProGuard и граница сторонних файлов |
| `bun scripts/check-product.mjs` | Полный Windows-срез passed: fmt, clippy с `-D warnings`, 340 Rust passed / 3 штатно ignored, release all-targets/all-features, 27 client tests / 280 assertions, 8 unsafe guards, контракты и 11 отрицательных type diagnostics |
| Проба нового комментария в копии проекта | Реальная Cargo-сборка с общим target-кэшем passed → rejected → passed после удаления нового `.rs` файла; проба включена в product-check, в том числе для его native Android вызовов |
| Пробы Vite и Gradle | Временные собственные `.ts`/`.kt` с комментариями останавливают сборочные команды с точным путём/строкой; после удаления команды проходят |
| Gradle configuration cache | После заполнения кэша комментарий меняет результат `providers.exec`, кэш инвалидируется и сборка отклоняется; после удаления снова successful |
| `bun run check --host-native` в `prototypes/` | fmt/clippy, 175 Rust tests, restart/Automerge probes, dispatcher/workflow checks и генерация контракта passed |
| `bun run check:types` в `prototypes/` | OpenAPI/TypeScript совпадают с генератором; 12 отрицательных диагностик для прежних 11 запрещённых вызовов |
| Windows UI: `bun scripts/check-ui.mjs` | Vue/types, bundle, native shell tests/build, полный browser/UI/HTTP/touch и двухинсталляционный pairing passed |
| Prototype Vue/types и Vite build | Passed отдельными командами; полный `--host-ui` с закреплённым Node не заявляется |
| `bun scripts/build-ui-android.mjs x86_64` | Продуктовый debug APK и androidTest APK собраны; manifest identity/launch categories/cleartext policy проверены штатным скриптом |
| `bun scripts/build-android.mjs x86_64` в `prototypes/` | Прототипный debug APK и androidTest APK собраны; собственный Kotlin lifecycle и Java instrumentation компилируются |
| Контракты и сторонний код | Оба OpenAPI JSON побайтово совпадают с исходными; Gradle wrappers, upstream PluginManager и стандартные buildSrc-классы не изменены |
| Lockfile, diff и документация | `bun install --frozen-lockfile --ignore-scripts` и `git diff --check` passed; итоговый аудит подтвердил 98 неизменных строк критериев, 133 продуктовых строк очереди, 11 контрактных/сторонних файлов и 9 новых ссылок |

При первой пробе обнаружена привязка абсолютных `rerun-if-changed` к исходному
checkout при повторном использовании Cargo-кэша в копии. Входы переведены в пути
относительно manifest; полный product-check повторён успешно вместе с новой
отрицательной пробой. Gradle использует `providers.exec`, поскольку прямой
`ProcessBuilder` несовместим с включённым configuration cache; кэш не отключён.

Корневые Babel/Vue/PostCSS dev-зависимости закреплены; корневая установка теперь
входит и в host native CI slice. Runtime-зависимости приложения не добавлены.
Описания API сохранены явными метаданными; директивы `@ts-expect-error` заменены
реальными компиляциями отрицательных fixtures с ожидаемыми diagnostics.

## Ограничения и итог первоначальной локальной проверки (LOG-572)

Локальный Node для Vue/UI — 26.8.1 вместо baseline 24.22.1; для штатного UI runner
использован его явный `UI_NODE_VERSION` override. Rust/Bun и Android build tools
соответствуют закреплённым версиям. Android APK только собирались: установки,
Device UI/instrumentation execution, повторной arm64-сборки и нового CI не было.
Ранее принятые runtime-результаты Android не объявляются новым запуском.

Пять правил проекта в этом техническом scope:

- Обратимость: комментарий отклоняет сборку, его удаление восстанавливает её;
  правило и удаления можно отменить через Git, данные пользователя не затронуты.
- Агент и человек используют те же сборочные команды и прежний прикладной API.
- Development/agent docs, ADR и журнал обновлены; user docs не требуют изменения
  при неизменном пользовательском поведении.
- Паритет: Windows-проверки и сборки обоих Android APK passed; новый native UI
  runtime охват не заявляется.
- Единый источник: общая политика проверки; Rust задаёт формы контрактов,
  явные описания применяются к одному ApiDoc, артефакты проверены генерацией.

TECH-001 verified локально в `chore/remove-source-comments`, изменения не
закоммичены. Commit/push/PR/CI/merge не выполнялись. Продуктовый P06.02 не начат,
определения/состояния T/A/M/S и прежние GAP/DEC сохранены. Далее — рассмотрение
этой ветки; продуктовый следующий шаг требует отдельного поручения.

## Impact для публикации PR

По следующему поручению разработчика добавлена проверяемая
[YAML-фикстура](../../../tests/fixtures/source-comments-impact.yaml).
Она фиксирует результат для разработчика: ранее допустимые комментарии теперь
отклоняются штатной сборкой, удаление восстанавливает сборку; строки, сторонние
уведомления и документация сохраняют допустимость. Семь сценариев выполняются
в тестах общей политики. Раздел `before` описывает исходную ревизию, а не новый
запуск компиляторов на старом коде. Числа — снимок scope этого PR: 1562 собственных
комментария на базе 210a0f5, 0 после изменения в 338 файлах; 350 отслеживаемых
сторонних комментариев сохранены. Дополнительные Cargo build prerequisites
и неизменный пользовательский/API-контракт указаны явно.

Публикация PR выполняется отдельным текущим действием; прежние локальные
результаты выше сохраняют свой фактический охват. Новые прогоны Impact и сведения
о публикации фиксируются ниже/в PR; merge этим поручением не выполняется.

Повторные проверки перед публикацией: `bun run test:comments` — 28 passed,
62 assertions, включая семь YAML-сценариев; полный `bun scripts/check-product.mjs`
повторён успешно с этой фикстурой, 340 Rust tests, 27 client tests и 8 unsafe
guards. Снова прошли prototype `check:types`, обе проверки host/Android CI routing,
source policy, полный diff и аудит контрактов/лицензий/ссылок. Исходный commit
до/после и количественный scope сверены с 210a0f5; продуктовые критерии и очередь
не изменены. Windows UI и APK-сборки из первой проверки не запускались повторно
ради добавления тестовой фикстуры и документации; их исходный охват указан выше.

Разработчик явно поручил создание PR с Impact: авторизованы commit и push этой
ветки, merge не входит в поручение. Актуальные URL, head и состояние CI/reviews
публикации хранятся в GitHub PR и локальном ignored отчёте; новый CI не считается
прошедшим по локальным результатам. Далее — рассмотрение PR с явным build impact.
