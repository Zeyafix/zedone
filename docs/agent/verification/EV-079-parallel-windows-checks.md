# EV-079 — параллельные Windows host-проверки

## Карточка

```yaml
step: CI.04
base_revision: 2f54349c775da93ff38aab34a51e64da510f7edd
branch: ci/parallel-windows-checks
goal: "Сократить ожидание Windows host-check до 10 минут без сокращения охвата."
requirements: [T16, T17]
dependencies_evidence: ["CI.01/EV-028 merged", "CI.03/EV-064 merged", "P05.03/EV-078 PR #78 merged"]
scope: ["Prototype foundation workflow", "общая подготовка host suites", "CI docs и журнал"]
checks: ["actionlint", "YAML и сохранение команд/paths/permissions", "настоящие shell-границы подготовки и aggregate с подставными внешними CLI", "cold/warm GitHub Actions — ожидаются"]
reverse_and_failure: "Возврат последовательного workflow через Git; failure/cancelled/skipped не дают успешного aggregate; fail-fast false сохраняет диагностику остальных suites."
result: "verified локально; цель времени и интеграция ожидают GitHub Actions"
next: "После публикации проверить точный head полным CI и измерить cold/warm время до aggregate; CI.04 остаётся in_progress."
```

Исходные незакоммиченные документы результата P05.03 сохранены:
журнал, очередь, EV-077 и EV-078. Снимок до работы — только
`docs/reports/ci-04/baseline/`. Продуктовая приёмка не расширяется;
P05.04/P03.06 и Android workflow не входят в изменение.

## Исходное измерение

[PR #78](https://github.com/Zeyafix/zedone/pull/78), head `75637bc`,
[Windows job](https://github.com/Zeyafix/zedone/actions/runs/42509815148/job/128820212928):
19:29, включая продуктовые проверки 7:05, прототип 4:40, установку JS/браузера
3:36, оболочку/редактор 2:29 и остальную подготовку/завершение 1:39.
Rust cache восстановлен полностью; release-сборка продукта заняла 3:43.
Внутри установки JS/браузера около 3:18 заняла Media Foundation, JS — 3 секунды.
Это исходный последовательный запуск, а не проверка нового workflow.

## Изменение

Windows: `product`, `prototype`, `shell` на отдельных runners, прежние команды
проверок без аргументов и без сокращения охвата. `fail-fast: false` сохраняет
результаты остальных наборов. Stable check `check (windows-2025)` требует
успеха всей матрицы через `needs` и `always()`; его короткое собственное время
не считается доказательством ускорения. Linux сохраняет последовательный
набор и имя check; Android workflow/dispatcher не меняются.

Composite action содержит закреплённые версии/CLI и подготовку каждого
набора. Shell может стартовать без предварительного product/prototype:
свой Cargo workspace, корневые JS-зависимости и Chromium ставятся отдельно.
Playwright `--with-deps` сохранён; Media Foundation не пропускается.
У каждого Windows suite свой namespace/cache target и единственный main
writer. Linux использует прежние namespace/paths. Новые Windows кэши холодные
до успешного main-запуска; перенос старого архива и PR-запись не добавлены.
Бюджеты Linux 10/Windows 20 минут сохранены; aggregate имеет бюджет 1 минуту.

## Локальная проверка

| Проверка | Результат и границы |
| --- | --- |
| Actionlint 1.7.12 | Все workflow и отдельная lint-обёртка composite steps/expressions без ошибок; ShellCheck в PATH отсутствует |
| YAML/охват относительно базы | Все три прежние команды и working directories сохранены на обеих OS; Linux сохраняет порядок, Windows выполняет каждый набор ровно один раз |
| Triggers/безопасность | Все прежние paths/events/concurrency/permissions сохранены, добавлен path action; actions закреплены прежними полными SHA; persist-credentials false, continue-on-error нет |
| Настоящие PowerShell scripts подготовки | Исполнены для all/product/prototype/shell с подставными rustup/cargo/bun: точные версии, workspace output, выбранные manifest/locked fetch, JS и browser preparation; исключения при ошибках rustup/cargo/bun, неизвестный suite отвергнут до вызова CLI |
| Настоящий aggregate shell | Пройден через Git Bash: success принят; failure/cancelled/skipped/пустой RESULT отвергнуты. GitHub scheduling/свёртка матрицы этим не эмулируются |
| Cache config | Раздельные Windows namespaces/owners, main-only save, корректный physical target для shell; прежний namespace/paths Linux. Реальный cache save/restore ожидает CI |
| Android routing | Штатный `bun prototypes/scripts/check-android-ci-suites.mjs` passed; Android workflow/dispatcher не менялись. Это не запуск Android runtime/Device |
| Документация/сохранность/diff | Локальные ссылки и ограждения корректны; исходные EV-077/078 побайтово совпали со снимком; прежние события, разделы плана/98 критериев/проблем и их статусы сохранены; очередь только добавляет CI.04; git diff --check passed |

Проверка границ выполняется по настоящим YAML/run-скриптам, заменены только
внешние CLI. Это не реальные установка toolchain/browser, Cargo builds или UI.
Сценарий и сырые результаты: `docs/reports/ci-04/verify-host-workflow.mjs`,
`local-validation.log`, `composite-lint.yml`. Они не входят в Git.
Git diff, сохранность исходных документов и локальные ссылки проверены
отдельно перед передачей (`docs-validation.log`).

## Передача и пять правил

Реализация локально подготовлена в `ci/parallel-windows-checks`, база `2f54349`.
Commit/push/PR/merge и новый CI не выполнялись. CI.04 остаётся `in_progress`:
цель ≤10 минут и корректность GitHub scheduling/сохранения кэшей ещё не приняты.
Следующий шаг после публикации — полный CI точного head и cold/warm измерение
от старта workflow до Windows aggregate, включая очередь/setup/post steps,
с отдельной суммой runner-минут. Первый новый namespace может быть медленнее
исходного прогретого запуска; бюджеты не снижены на основании оценки.

Обратимость — возврат workflow через Git; новые продуктовые действия не
добавляются. Человек и агент используют те же команды; документация
разработчика/агента обновлена. Паритет ПК/Android не ослаблен, все прежние
наборы сохранены; runtime на обеих платформах заново не запускался.
Единый источник версий — `environment-versions.json`; продуктовые API/данные,
98 определений критериев и их статусы, прежние BUG/GAP/DEC не меняются.
P05.04/P03.06 не начаты.

## Проверка на GitHub runners

После локальной передачи разработчик поручил проверку. Для неё публикуется
только CI-срез в отдельной ветке и создаётся draft PR; новый запуск ещё
ожидается. Исходные незакоммиченные документы результата P05.03, включая
журнал и очередь с локальной передачей CI.04, остаются в рабочем дереве.
Это разрешает commit/push и запуск проверки, но не merge в main.
Первое измерение выполняется с холодными Windows suite-кэшами;
main-only политика записи сохраняется. Успех и время записываются только
после завершения полного запуска.

Создан [draft PR #79](https://github.com/Zeyafix/zedone/pull/79), head
`addb9c473ed43b5a350a4780da93078d2f367b49`, commit
`ci: let Windows checks run independently`.
[Host run 42514091137](https://github.com/Zeyafix/zedone/actions/runs/42514091137)
стартовал 2026-11-08 19:15:13 UTC; Linux и три Windows suites ожидаются.
Это pending, не успешный результат; отсчёт до завершения Windows aggregate.

### Отказ первого запуска: GAP-014

Windows shell job `128833197769` завершился failure в 19:21:28 UTC:
сборка/Clippy/3 native tests/Chromium прошли, но `check-ui-workspace.mjs:64`
после reload не увидел region «Разобрать» за прежние 10 секунд. Это не ошибка
установки зависимостей; причина timeout по исходному логу не доказана.
На свежем локальном UI/fixture пять полных workspace-сценариев с тем же
Playwright Chromium `156.0.8078.4` прошли. Отказ не объявлен исправленным.
Исходный лог сохранён в `docs/reports/ci-04/shell-addb9c4-failed.log`,
воспроизведение — `workspace-reproduction.log`.

Host browser driver дополнен диагностикой API error.code/сетевых ошибок и
read-only состояния страницы при отказе. Credentials/headers/command bodies
не выводятся; исходная ошибка выбрасывается и cleanup сохраняется.
Сохраняются только последние 30 read outcomes (HTTP/status/error/count),
без содержимого объектов. Полный локальный `ui:check` прошёл на Windows:
Node 26.8.1 с явным override базы 24.22.1, штатный Playwright Chromium,
fmt/Clippy/3 shell tests/build, workspace/cards/editor/appearance/delivery.
Контрольный искусственный отказ подтвердил вывод состояния/error.code,
границу 30 outcomes, отсутствие credential/header fields и ненулевой exit;
это проверка диагностики, а не воспроизведение GAP-014.
Таймауты/ожидания/проверки прежние, автоматический повтор не добавлен.
GAP-014 зарегистрирован отдельно от прежнего GAP-013; следующий полный CI
должен либо подтвердить весь набор, либо дать состояние для диагностики.
