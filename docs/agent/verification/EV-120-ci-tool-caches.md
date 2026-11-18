# EV-120 — CI.14: сократить повторную подготовку инструментов

- Поручение: продолжить сокращение >10m без потери качества в PR #109.
- База a040174, все 27 expected SUCCESS после одного bounded cross rerun.
  GAP-036 root cause остаётся open; failed attempt учитывается отдельно.
- Scope: host preparation/Android workflow, общий SDK verifier и guards/docs.
  Сценарии, матрица 27, product API/Rust/UI, job/fixture budgets прежние.
- Цель: exact-key SDK/browser tool caches; full SDK verification на miss/hit,
  прежние OS/browser install checks; только используемый headless Chromium.
- SDK cache не содержит AVD, app data или OS keys. Один writer prototype-native
  после успешных checks. Key — OS/arch + versions/setup/verifier fingerprints,
  без restore-prefix fallback. PR cache scope не меняет права main.
- Browser caches: OS/arch/family/установленная Playwright version, один writer
  для каждой family; install --with-deps выполняется и после cache hit.
- Обратимость: revert подготовки, cache miss возвращает полный pinned install.
  Нет переноса результата проверки через cache, ошибки остаются fail-closed.
- Проверки: точная прежняя матрица/strict gates, cold/hit routing, pinned
  SDK/CLI hash/version/missing package/corrupt package и Rust install failure,
  browser default headless binary/реальный UI, ShellCheck/actionlint/PS syntax.
- До изменения cache inventory: 19 entries, 9.025 GiB, включая старые ключи.
  Увеличение квоты/платные настройки не входят в scope. Размер нового SDK cache
  и эффект eviction/restore обязательно измерить; при ухудшении пересмотреть.
- Playwright обычно не рекомендует browser caching: restore может стоить
  столько же, сколько download, Linux OS deps не кэшируются. Здесь проверяется
  конкретный Windows bottleneck (~3m browser install), без обещания выигрыша.
- Новый warm результат пока отсутствует. Cold/miss и hit сравнить на одном head,
  все повторы/очереди/runner work включить в отчёт. Старые 27 SUCCESS не являются
  проверкой нового setup. Все 98 definitions/states и GAP/DEC сохраняются.
- Состояние: verified; реальные cold/hit серии завершены, цель времени не достигнута. Итоги в конце / LOG-661–662.

## Локальная проверка / LOG-657

- Exact прежние 27 checks, host/Android routing, strict aggregates и отрицательные
  контроли сохранены. Новый guard исполняет настоящий SDK verifier на POSIX
  fixtures: valid/первая инициализация, неверные CLI hash/version, отсутствующий
  package, неверная revision, отказ rustup. SDK errors останавливают setup до
  rustup; exit 23 не теряется. Miss/hit/save/один writer проверены отдельно.
- Browser identity проверена настоящим PowerShell для обеих families и full
  режима. Изолированная установка only-shell, root UI --main и prototype
  --host-ui passed; полный Chromium отсутствовал. Host-target locked Cargo
  fetch и PowerShell parser всех host setup steps passed.
- Первое выполнение prototype UI штатно отклонено Node version guard; повтор
  с закреплённым Node passed, guard не менялся. В первой тестовой fixture
  Windows jq добавлял CRLF; test double заменён на JSON output через Bun.
  Это ограничение fixture, не повод менять Linux SDK проверку.
- Actionlint 1.7.12 с настоящим ShellCheck, ShellCheck SDK/environment,
  Node syntax, source-comment policy (387 sources), diff check passed.
  Pyflakes отсутствует и отключён. Реальная Linux SDK cache restore/save
  локально не исполнялась; прежние Android runtime результаты относятся
  к a040174 и не заменяют новую CI проверку подготовки.
- Windows начинает с длинного shell slice; состав/бюджеты не меняются.
  Объём >300 строк необходим для одной границы подготовки: workflows,
  общий verifier, отказные guards и документация; отдельного продуктового
  изменения нет. Все 98 definition/state rows прежние.
- Пять правил: изменение подготовки обратимо; общий human/agent API прежний;
  docs обновлены; обе платформы и все сценарии сохранены; SDK pins берутся
  из одного environment-versions.json, test verdict не кэшируется.

Далее новый полный CI, затем один контролируемый warm benchmark на том же
head после успешной cold серии. Учитывать также failures, все executions,
очереди, сохранение/восстановление и eviction; при отсутствии выгоды изменить
cache strategy. До измерения ускорение и выполнение цели >10m не заявляются.

Источники: [Playwright CI](https://playwright.dev/docs/ci#caching-browsers),
[GitHub cache scopes/limits](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching).

## Публикация / LOG-658

Head 279cb85 в PR #109, новый CI pending (27 expected). Tested source
fingerprints/readback/98 criteria rows проверены. Существующий watch продолжает
задачу; cold/warm сравнение и cache footprint ещё не выполнены. Checkpoint
локальный без нового push; merge/delete не поручены.

## Отказ prototype-arm64: Maven Central 429 / LOG-659

На неизменном 279cb85 job 134772951165/run 44295967556/attempt 1 отказал
в Build Android arm64 APK and instrumentation. Gradle не смог получить
junit:junit:4.13.2 POM из Maven Central: HTTP 429 Too Many Requests, затем
отклонил configuration cache с unresolved runtime classpath. Это не
самостоятельный дефект configuration-cache serialization по имеющемуся логу.
SDK cache отсутствовал; полная pinned установка и новый SDK verifier passed.
Root cause rate limiting не установлен (GAP-037); зависимость/guard не меняются.

Complete failed-job log и metadata сохранены. Первое получение лога было
отклонено gh из-за terminal escape sequences; повторное скачивание с явным
разрешением вывода в ignored файл succeeded, сам CI job не повторялся.
Workflow ещё in_progress. После terminal сохранить first attempt и выполнить
ровно один failed-job rerun на прежнем head; accepted=0/limit=1, пока не
запрошен. Повторный отказ диагностировать без нового retry. Failed execution,
очереди и весь rerun cost включить в метрики; cache hit повтора не выдавать за
исходную cold попытку. До реальных результатов 27 SUCCESS/ускорение не заявляются.

## Завершение исходной серии и warm benchmark / LOG-660

279cb85: все 27 expected checks SUCCESS после единственного arm64 rerun.
Исходные HTTP 429/aggregate failure сохранены; GAP-037 остаётся open, как
GAP-034/035/036. Runtime/SDK verifier, APK/app-files restore и cleanup passed.
Предел 10m ещё не достигнут: Windows shell 12:43, Android shell 10:45,
prototype-ui 10:23. Для Windows browser miss/install занял около 3:14;
Android SDK miss/install — 80–87s в этих двух suites.

Сохранены оба final run, все jobs/steps, complete logs трёх bottlenecks и
cache inventory. SDK cache 2.08 GiB, четыре browser caches суммарно 0.44 GiB;
весь inventory 24 entries/11.54 GiB, quota не менялась. Это snapshot, не
доказательство устойчивости к eviction; повторно проверить после benchmark.
GitHub скопировал 11 inherited successes в новые job IDs: фактически 29
executions, а не 40 records; дедупликация по run/name/start/end/conclusion.
Все failures, scheduling и rerun cost сохраняются в ignored cold-summary/raw.

Запрошено ровно по одному полному warm rerun обоих workflows на том же head
(теперь host attempt 2, Android attempt 3). Это benchmark после SDK/browser
save, а не второй recovery retry. Код, guards, сценарии и budgets прежние.
Далее штатный watch, реальные hit/miss/restore timing, wall до полного зелёного
и runner work, затем необходимые ускорения до <=10m с ориентиром <=5m.
Проверять весь набор, включая очередь; одна серия не доказывает стабильность.
Merge/delete не поручены. Пять правил в scope прежние: обратимая подготовка,
общий API, актуальные docs, прежние Windows/Android suites, единые pins/data.

## CI.14 warm завершён / LOG-661–662

279cb85: host attempt 2/Android attempt 3, все 27 SUCCESS без отказов.
Полный результат через 13:07 после первого принятого benchmark request;
Windows shell 12:29, Android prototype-ui 10:41, pairing-placements 10:37,
shell 10:25. Queue wait prototype-ui — 2:13. Суммарная работа 27 runners
166:11; исходная серия с recovery — 168:06/29 actual executions и 18:24
до зелёного. Разный состав failures/очередей ограничивает сравнение;
эти цифры не доказывают причинное ускорение или выполнение цели <=10m.
SDK hit/verify/runtime прошли; restore 42–65s у двух bottlenecks вместо
80–87s cold install. Browser hit не устранил Media Foundation install
185.9s, поэтому выигрыш browser cache мал. Cache snapshots до/после: те же
24 IDs/11.54 GiB, удалений в этом интервале не наблюдалось; устойчивость
к будущему eviction не доказана, quota/paid settings не менялись.
GAP-034/035/036/037 остаются открыты. Сценарии/98 criteria прежние.
Далее CI.15/EV-121 сокращает последовательные browser-сценарии; Android
>10m и полное время с очередью остаются следующей частью задачи.
