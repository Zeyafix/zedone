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
- Состояние: verified локально; реальные CI cold/hit, объём caches и время ожидаются.

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
