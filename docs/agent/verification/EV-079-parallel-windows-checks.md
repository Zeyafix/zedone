# EV-079 — параллельные Windows host-проверки

## Карточка

```yaml
step: CI.04
base_revision: 2f54349c775da93ff38aab34a51e64da510f7edd
branch: ci/parallel-windows-checks
goal: "Сократить ожидание Windows host-check до 10 минут без сокращения охвата."
requirements: [T16, T17]
dependencies_evidence: ["CI.01/EV-028 merged", "CI.03/EV-064 merged", "P05.03/EV-078 PR #78 merged"]
scope: ["Prototype foundation workflow и read-only миграция cache", "общая подготовка host suites", "разделение host driver и диагностика test/debug Rust", "CI docs и журнал"]
checks: ["actionlint", "YAML и сохранение команд/paths/permissions", "настоящие shell-границы подготовки и aggregate с подставными внешними CLI", "GitHub Actions ed512db: все 12 passed, Windows 6:32 с legacy main cache; полностью cold не подтверждён"]
reverse_and_failure: "Возврат последовательного workflow через Git; failure/cancelled/skipped не дают успешного aggregate; fail-fast false сохраняет диагностику остальных suites."
result: "done: PR #79/head ed512db merged 8e87273 после 12 SUCCESS; Windows 6:32/весь CI 9:31; один cached замер, GAP-014 открыт; ветка удалена локально и на GitHub"
next: "CI.04 интегрирован; следующий готовый продуктовый шаг P05.04, также готов P03.06. Новая реализация не начата; полностью cold время не подтверждено."
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

Windows: `product`, `prototype-native`, `prototype-ui`, `shell` на отдельных
runners без сокращения охвата. Полные product/shell команды прежние;
prototype default сохранён, его два среза исполняют прежние команды ровно
один раз в сумме. `fail-fast: false` сохраняет
результаты остальных наборов. Stable check `check (windows-2025)` требует
успеха всей матрицы через `needs` и `always()`; его короткое собственное время
не считается доказательством ускорения. Linux сохраняет последовательный
набор и имя check; Android workflow/dispatcher не меняются.

Composite action содержит закреплённые версии/CLI и подготовку каждого
набора. Shell может стартовать без предварительного product/prototype:
свой Cargo workspace, корневые JS-зависимости и Chromium ставятся отдельно.
Playwright `--with-deps` сохранён; Media Foundation не пропускается.
У каждого Windows suite свой namespace/cache target и единственный main
writer. Linux использует прежние namespace/paths. До появления собственного
Windows suite-cache добавлен read-only fallback к прежнему main-архиву
через lookup/restore; PR-запись не добавлена. Историю холодных запусков
и условия миграционного restore читать в результатах ниже.
Бюджеты Linux 10/Windows 20 минут сохранены; aggregate имеет бюджет 1 минуту.

## Локальная проверка

| Проверка | Результат и границы |
| --- | --- |
| Actionlint 1.7.12 | Все workflow и отдельная lint-обёртка composite steps/expressions без ошибок; ShellCheck в PATH отсутствует |
| YAML/охват относительно базы | Product/shell и Linux default команды/working directories сохранены; Windows prototype разделён на два флага, exact union всех прежних subprocess-команд проверяет новый штатный guard |
| Triggers/безопасность | Все прежние paths/events/concurrency/permissions сохранены, добавлен path action; actions закреплены прежними полными SHA; persist-credentials false, continue-on-error нет |
| Настоящие PowerShell scripts подготовки | Исполнены для all/product/prototype-native/prototype-ui/shell с подставными rustup/cargo/bun: точные версии, workspace output, выбранные manifest/locked fetch, JS и browser preparation; исключения при ошибках rustup/cargo/bun, неизвестный suite отвергнут до вызова CLI |
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

### Холодный prototype: дополнительное разделение

Первый `prototype` job `128833197953` не имел кэша. До отмены новым head
он исполнялся более 14 минут: подготовка завершилась в 19:20:07 UTC,
Clippy 1:18, test compilation 1:22, 175 tests 2:32, release workspace 2:48,
debug-пробник compilation 0:32 и его реальные restart-фазы около 30 секунд.
HTTP/UI ещё не начались. Этот отменённый run не считается успешным замером.

Нужен четвёртый Windows runner: `prototype-native` выполняет прежние
fmt/Clippy/tests/debug restart-пробник/OpenAPI/types, `prototype-ui` — прежнюю
полную release-сборку workspace и Vue/Vite/HTTP/Android driver guards.
Тяжёлая подготовка Media Foundation нужна только UI-срезу; оба используют
свои suite-кэши. Linux default остаётся последовательным. Новые флаги
`--host-native`/`--host-ui` не меняют прежние export/check/default modes.
Штатный `bun prototypes/scripts/check-host-ci-suites.mjs` проверяет exact
union команд двух срезов, отсутствие повторов/пропусков, physical output,
legacy flags, неверные аргументы и выход при failure/signal. Это проверка
процессных границ, не замена реальным native/browser тестам.

Оба новых среза выполнены локально настоящими CLI и прошли:
`--host-native` — fmt/Clippy/175 Rust tests/все debug restart-фазы/OpenAPI/types;
`--host-ui` — полная release workspace, Node/Vue/Vite, настоящий HTTP/Chromium
и все прежние Android driver guards. UI использовал закреплённый portable
Node 24.22.1, ZIP проверен по официальному SHA-256, PATH менялся только
в процессе проверки. Штатный routing guard и обновлённая PowerShell/YAML
матрица четырёх suites тоже passed; workflow/composite actionlint passed.
Логи: `prototype-native-local.log`, `prototype-ui-local.log`,
`four-suite-validation.log` в `docs/reports/ci-04/`.

Диагностический head `654f6ce` в
[shell job 128836888084](https://github.com/Zeyafix/zedone/actions/runs/42515277703/job/128836888084)
прошёл весь Windows UI-набор: 19:29:33–19:37:01 UTC, 7:28.
Workspace/cards/editor/history/delivery и три native tests подтверждены логом;
прежний reload больше не упал в этом запуске. Это успех конкретного job,
а не доказательство причины GAP-014 или результата нового четырёхчастного CI.

### Сохранение Linux бюджета

Linux job `128836887850` диагностического head `654f6ce` исчерпал прежние
10 минут (19:29:32–19:39:36 UTC). Последний editor-сценарий passed в
19:39:31, затем job был отменён; это не успешный полный host-check.
Его 23 client delivery-теста исполнялись отдельно 25.09 секунды после Rust.
Host теперь использует ту же параллельность clients/native, что уже была
на Android, через общий `checkWithClients`. Все нативные/unsafe/contract/types
команды сохранены, обе стороны ожидаются даже при отказе, ошибки объединены.
Новый routing/pairing guard проверяет оба порядка завершения, оба отказа,
клиентский сигнал и отсутствие раннего выхода с незавершённым peer.

Полный настоящий `bun scripts/check-product.mjs` после изменения passed:
273 Rust tests (17/7/29/220), 23 client tests/239 assertions, release всех
targets/features, семь unsafe guards, OpenAPI/types/tsc. Лог
`product-overlap-local.log` показывает клиентские тесты во время native tests,
а не после них. Android native/ABI исполнение обновлённого общего helper
ожидает нового CI; его порядок/объём внутри Android ветки не менялся.

### Новый полный CI

Опубликован head `2fa85ed892422ce66651c1896cdfc4fa5b396f73`, commit
`ci: run independent host checks at the same time`.
Новые [host run 42516607788](https://github.com/Zeyafix/zedone/actions/runs/42516607788)
и [Android run 42516607820](https://github.com/Zeyafix/zedone/actions/runs/42516607820)
стартовали 2026-11-08 19:45:17 UTC. Все исходные suites и оба strict aggregate
ожидаются, результат pending. Не принимать старые частичные успехи за CI
этого head. Windows отсчитывать от 19:45:17 UTC до завершения его aggregate.

### Полный первый результат 2fa85ed: проверки прошли, цель времени — нет

Все 12 checks точного head `2fa85ed` SUCCESS с первой попытки: оба
workflow/aggregate и все исходные native/ABI/HTTP/UI/environment/cleanup
наборы. Raw logs подтвердили по 273 product Rust tests на Windows/Linux/
Android, 23 client tests/239 assertions, семь host/каждой Android ABI unsafe
guards, OpenAPI/types и новый routing/pairing guard. Prototype Windows —
175 tests, Linux — 177 (Unix-specific tests). Windows/Linux UI прошли на
Node baseline 24.22.1; весь Android APK/IPC/lifecycle и обе ABI passed.
GAP-014 не повторился; причина первого отказа всё ещё не доказана.

| Измерение attempt 1 | Фактическое время |
| --- | --- |
| Windows product job | 8:00 |
| Windows prototype-native job | 7:06 |
| Windows prototype-ui job | 13:45 |
| Windows shell job | 8:43 |
| Windows aggregate: собственный job | 0:04 |
| От старта workflow до Windows aggregate | **13:54**, 19:45:17–19:59:11 UTC |
| Сумма четырёх Windows jobs | 37:34 (без aggregate на Ubuntu) |
| Linux job | 9:11 |
| Android workflow до aggregate | 9:34 |

Цель ≤10 минут **не достигнута**. Считать короткие 4 секунды aggregate
ускорением нельзя. Все четыре Windows cache namespaces действительно
холодные; Linux восстановил прежний полный cache hit. В prototype-ui
подготовка заняла 9:46: Rust install 1:46, Rust cache action до «No cache»
2:47, Media Foundation около 4:13; собственные release/HTTP/UI/guards
заняли около 3:34. Остальные Windows preparation — 1:00/1:07/4:32.
Длительная подготовка требует отдельного повторного измерения; нельзя
считать её ни гарантированно разовой, ни устранённой без фактов.

Запрошен полный rerun **всех шести host jobs** того же run/head, старт
20:01:46 UTC; это проверка вариативности холодной подготовки, без изменения
исходников, PR cache policy и охвата. Android уже полностью прошёл на том
же head, повтор его независимого workflow не требуется. Attempt 1 и его
13:54 сохраняются в сравнении; повтор ещё pending. Номер новой попытки и
итог подтвердить по Actions API. Сырые результаты — `host-2fa85ed-attempt1.log`,
`android-2fa85ed-attempt1.log`, `host-jobs-2fa85ed.json`,
`android-jobs-2fa85ed.json`, `prototype-ui-2fa85ed.log` в локальном отчёте.

### Attempt 2: read-only unavailable и новая диагностика

Повтор head `2fa85ed` завершился failure. Windows product/native/UI/shell:
10:45/7:45/8:18/**5:40 failure**, Linux passed за 9:17; Windows aggregate
failed в 20:12:40 UTC, 10:54 от старта 20:01:46. Это не успешный замер
≤10 минут. В product release compilation заняла 5:22 без suite-cache.

Shell job `128845078197` прошёл первый board reload, затем в сценарии
потерянного ответа после следующего reload `space.list` вернул HTTP 200/
`status:error/error.code:unavailable`. UI показал «Не удалось связаться с ядром»
и правильно отключил «Проверить результат»; ожидание click на строке 111
истекло. Это другая точка отказа, чем первый region timeout GAP-014;
совпадение первопричин не доказано. Лог теперь содержит API/DOM состояние.

Причина `unavailable` требует storage-категории. Добавлен только opt-in
debug trace `ZEDONE_CLOCK_DIAGNOSTICS=1`: Service пишет безопасную категорию
`StorageError`, clock — calendar/observed milliseconds при существующем
fail-closed отказе. Вывод best-effort stderr без ключей/SQL/paths/request data;
ошибка stderr не меняет командный результат. В release код не компилируется,
в UI flag получает только отдельный синтетический fixture. Время/авторизация/
clock policy ADR-0038/публичные ответы и прежние тесты не ослаблены.

Локальные результаты:

- Ограниченная проба: 10 000 настоящих `Storage::timed_transact` с системными часами
  прошли за 8.925 секунды; гипотеза часов не подтверждена, причина CI не объявлена.
- Все 10 существующих clock-policy tests passed с trace выключенным и включённым;
  контролируемый rollback/frozen case выдал trace и прежний unavailable.
- Полный UI-check с fixture trace passed на Node baseline 24.22.1/Chromium,
  включая native tests/Clippy/build/workspace/cards/editor/history/delivery.
- Первый локальный product-check был ошибочно запущен с флагом у всего процесса:
  13 CLI assertions пустого stderr не прошли. Это ошибочная конфигурация контроля,
  не принимаемая за passed. Тесты не менялись; флаг ограничен fixture-процессом.
- Новый полный product-check в штатном окружении passed: все 273 Rust tests,
  23 client/239 assertions, release all-targets/features, семь unsafe guards,
  OpenAPI/types/tsc. Стандартный stderr/ответы CLI сохранены.

Сырые логи — `shell-2fa85ed-attempt2-failed.log`,
`host-jobs-2fa85ed-attempt2.json`, `product-2fa85ed-attempt2.log`,
`clock-probe.log`, `clock-policy-diagnostics-tests.log`,
`clock-policy-diagnostics-enabled.log`, `ui-backend-diagnostics-local.log`,
`product-backend-diagnostics-local.log` (ошибочная конфигурация),
`product-backend-diagnostics-default-env.log` — только в локальном отчёте.

### Read-only миграция кэша

API подтвердил, что прежний main-архив
`zedone-rust-v1-prototype-host-Windows_NT-x64-af717f4b-427390d8` размером
1 039 083 211 байт ещё доступен. Новые Windows namespaces пока пусты и
по main-only policy не прогреваются в PR. Добавлен metadata-only lookup
собственного suite-архива; при его отсутствии legacy restore читает main
перед `rust-cache`, чтобы подготовка metadata/сборка могли использовать
прежние зависимости. Собственный архив найден — fallback пропускается;
оба отсутствуют — обычная холодная сборка. Ни один новый шаг не записывает кэш.

Четыре абсолютных legacy paths и их порядок совпадают со старым `rust-cache`.
Локально воспроизведён algorithm toolkit: с `zstd-without-long/windows-only/1.0`
получена archive version
`6370e326445db9dd1478b2f0415651db5229daabe7957f68172909d37f07b335`,
точно равная версии старого архива из API. PowerShell outputs, lookup/fallback
условия (Windows/Linux, все suites, архив есть/нет), pinned restore-only actions
и workflow/composite actionlint passed. Это не подмена проверки: все прежние
Cargo команды/флаги/версии/проверки freshness сохраняются. Настоящий restore,
время и весь CI нового диагностического head ещё ожидаются; цель не принята.

Опубликован head `ed512db547c09fb5e871e05976e344c29dd67f9b`, commit
`ci: reuse main dependencies and diagnose backend failures`.
Новые [host run 42520459945](https://github.com/Zeyafix/zedone/actions/runs/42520459945)
и [Android run 42520460030](https://github.com/Zeyafix/zedone/actions/runs/42520460030)
стартовали 2026-11-08 20:30:29 UTC. Все 12 checks нового head ожидаются;
это не продолжение/успех старых попыток. Проверить реальные cache keys/paths/
restore, полный охват и время от 20:30:29 до Windows aggregate. PR остаётся draft.

### Итог ed512db: цель достигнута с legacy main cache

Точный head `ed512db547c09fb5e871e05976e344c29dd67f9b`: все **12 checks SUCCESS**
с первой попытки обоих workflows. Host run `42520459945` и Android run
`42520460030` стартовали 2026-11-08 **20:30:29 UTC**. Windows aggregate завершён
**20:37:01**, Android aggregate/последний check — **20:40:00**.
Итого Windows **6:32**, весь CI **9:31**. Сравнимая исходная граница PR #78 —
**19:32** от старта workflow, включая Windows job **19:29**.
Четыре секунды собственного aggregate не используются как замер ускорения.

| Job/граница | Фактическое время |
| --- | --- |
| Windows product, job 128852617079 | 6:23 |
| Windows prototype-native, job 128852616775 | 4:45 |
| Windows prototype-ui, job 128852617036 | 5:27 |
| Windows shell, job 128852617048 | 5:58 |
| Windows aggregate: собственный job | 0:04 |
| От старта workflow до Windows aggregate | **6:32** |
| Сумма четырёх Windows jobs | **22:33**, без Ubuntu aggregate |
| Linux job, 128852617054 | 9:23 |
| Host workflow целиком | 9:26 |
| Android workflow / все 12 checks | **9:31** |

Все четыре Windows jobs действительно восстановили один прежний main-архив
`zedone-rust-v1-prototype-host-Windows_NT-x64-af717f4b-427390d8`.
Собственные suite namespaces ещё дали miss; PR не сохраняет кэши.
Это один успешный запуск с восстановленными зависимостями, сравнимый с
исходным main cache hit. Полностью холодный запуск без обоих архивов и
будущие main save/direct suite restore не подтверждены. Повторная подготовка
увеличивает сумму Windows job time с 19:29 до 22:33; сокращено ожидание,
стоимость/тарификация не измерялись. Прежние 13:54 и failed attempt 2
сохраняются выше, без отбора только успешных результатов.

Raw logs подтвердили прежний полный охват: 273 product Rust tests на каждой
из Windows/Linux/Android, 23 client tests/239 assertions, семь unsafe guards
на host/каждой Android ABI, OpenAPI/types, routing/pairing guard. Prototype:
175 Windows/177 Linux tests (разница Unix-specific), release/restart/HTTP/UI
passed. Windows/Linux UI на Node baseline 24.22.1 прошли полностью;
Android обе ABI/native/APK/IPC/lifecycle/environment/cleanup passed.
Ни один assertion/timeout/clock/access guard не ослаблен, retry не добавлен.

GAP-014 не повторился; диагностические storage/clock error traces отсутствуют.
Это не доказывает причину или устранение прежних отказов, GAP-014 остаётся
`in_progress` отдельно от GAP-013. Пять правил в данном охвате соблюдены:
изменения CI обратимы через Git; общий API человека/агента и данные продукта
прежние; developer/agent docs обновлены; Windows/Linux/Android прошли;
версии берутся из единого `environment-versions.json`.

CI.04 — `verified`, draft PR #79 доступен для ревью, merge не выполнялся
и не поручен. Финальные результаты сохранены в локальных docs и PR body
без нового source-коммита: проверенный head остаётся `ed512db`.
Исходные dirty EV-077/078 сохранены побайтово, 98 критериев/статусы приёмки
не изменены. Сырые логи и JSON — `host-ed512db.log`, `android-ed512db.log`,
`host-jobs-ed512db.json`, `android-jobs-ed512db.json` в `docs/reports/ci-04/`.
Следующее действие — ревью/интеграция по поручению разработчика.

### Интеграция по поручению разработчика

Разработчик поручил PR, при успехе merge и удаление ветки. Повторно сверены
точный head `ed512db`, все 12 завершённых SUCCESS, base `main`, отсутствие
конфликтов и замечаний reviews/comments. Новые исходники/коммиты не добавлялись.
PR #79 переведён из draft и слит merge-коммитом
`8e87273d994d1b7969d8431b6212b17da132f0ee` в **2026-11-09 05:36:46 UTC**.

Git подтвердил вхождение проверенного head и полное совпадение дерева
merge с ним. Локальный `main` fast-forward до `origin/main`; ветка
`ci/parallel-windows-checks` удалена на GitHub и локально без force-delete.
Все шесть исходных working documents сохранились побайтово при переключении;
резервный снимок/manifest — только в `docs/reports/ci-04/pre-merge-*/`.
EV-077/078 не редактировались. Финальная передача/измерения остаются в
локальных docs и PR body; отдельный коммит результатов не создавался.

CI.04 — `done` в указанном CI-охвате; T16/T17 и остальные 98 критериев не
переоцениваются. Пять правил соблюдены в прежнем охвате, GAP-014 остаётся
открытым, ограничение одного замера с main-кэшем сохраняется. Результаты CI
относятся к проверенному PR head; отдельный post-merge запуск не выдаётся
за проверенный. Следующий готовый продуктовый шаг P05.04, также готов P03.06;
новая реализация не начата.
