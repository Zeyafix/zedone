# EV-028 — Ускорение PR-проверок, CI.01

```yaml
date: 2026-11-03
step: CI.01
base_revision: 5aca518beda03b4765dcb96f84848394acaccb6e
branch: ci/faster-pr-checks
goal: "Переиспользовать подготовленные инструменты/зависимости и исключить двойные PR-прогоны при сохранении всех тестов."
requirements: [T16, T17]
dependencies_evidence: [P00.06, P01.04, EV-009, EV-016]
scope: [.github/workflows, scripts/install-ci-tauri.sh, scripts/check-ci-tauri.sh, docs/development, docs/agent]
checks: [actionlint, ShellCheck, "Bash: установка CLI с заглушкой Cargo и отказы/cleanup", "git diff --check"]
reverse_and_failure: "Обратный коммит и удаление кэшей; cache miss или неверная версия CLI запускают прежнюю закреплённую установку; ошибка компилятора сохраняет код и убирает временный target."
result: "done: PR #22 интегрирован в 08cb948; восемь main cold/warm jobs успешны, сохранение и точные cache hits подтверждены. Длительности и ограничения ниже, LOG-202."
evidence: ["Actionlint 1.7.12 + ShellCheck 0.11.0", "Bash: 8 сценариев установщика с заглушкой Cargo", "YAML: сохранение прежних команд/матриц/прав", "PR: 4 успешных jobs", "Main 08cb948: 8 успешных cold/warm jobs, все cache hits", "Markdown-ссылки и git diff --check"]
next: "Вернуться к продуктовой очереди: P02.03.2, зависимость P02.03.1 интегрирована."
```

## Основание и границы

Измерения GitHub jobs события pull_request (мин:сек, без очереди):

| PR | Windows | Linux | Android |
| --- | --- | --- | --- |
| #19 | 7:53 | 4:21 | 20:51 |
| #20 | 10:43 | 4:52 | 20:44 |
| #21 | 9:52 | 3:44 | 15:50 |

Источники: [#19 host](https://github.com/Zeyafix/zedone/actions/runs/41593497735),
[#19 Android](https://github.com/Zeyafix/zedone/actions/runs/41593497819),
[#20 host](https://github.com/Zeyafix/zedone/actions/runs/41598813566),
[#20 Android](https://github.com/Zeyafix/zedone/actions/runs/41598813445),
[#21 host](https://github.com/Zeyafix/zedone/actions/runs/41604662369),
[#21 Android](https://github.com/Zeyafix/zedone/actions/runs/41604662355).

Tauri CLI занимает 3:59–6:08 компиляции. [Первый push Android #21](https://github.com/Zeyafix/zedone/actions/runs/41604638601/attempts/1)
упал с `No space left on device` при установке CLI; это основание для
очистки собственных промежуточных файлов и диагностики, а не доказательство
всех причин расхода диска. До изменения API Actions caches вернул ноль записей.

Все существующие команды проверок остаются, Android собирает x86_64/arm64
и выполняет native/WebView/HTTP/lifecycle-сценарии на API 33/x86_64.
Меняются orchestration и подготовка; пользовательский API/SQLite/UI не меняются.
P02 остаётся продуктовым этапом; CI.01 не объявляет новую приёмку продукта.

## Реализация

- Во всех трёх workflow push ограничен main; PR, ручной запуск, paths,
  concurrency, бюджеты и прежние команды проверок сохранены.
- Rust cache разделяет host/Android, OS/архитектуру и NDK, учитывает toolchain,
  Cargo/окружение; pinned Rust выбран только для action. Cargo bin не кэшируется.
- Отдельный Tauri cache хранит только бинарник, с точным ключом версии/платформы.
  Установщик проверяет версию, восстанавливает неверный/сломанный кэш, на холодном
  пути компилирует в собственный временный каталог и убирает его при завершении.
- Gradle cache обслуживается существующим setup-java с явными путями файлов
  вложенного проекта. Все кэши сохраняются только из успешных main-запусков;
  отсутствие cache service/primary key не запускает сохранение Tauri с пустым ключом.
- SDK setup убирает собственные архивы и заменённый Emulator. Добавлены df/du
  и отдельные времена CLI/контрактов/host/x86_64/arm64/пробника.
- Политика, команды, ограничения, отмена и методика замера описаны в
  [CI-документе](../../development/ci-checks.md); инструкции прототипов и сред обновлены.

## Локальная проверка

| Проверка | Фактический результат |
| --- | --- |
| Actionlint 1.7.12 с ShellCheck 0.11.0 | Все три workflow, ошибок/предупреждений нет |
| ShellCheck и bash -n двух новых скриптов | Пройдены |
| Холодный кэш, включая путь с пробелом | Вызвана закреплённая установка, оба output-каталога удалены, соседний sentinel сохранён |
| Корректный тёплый кэш | Cargo не вызывается, бинарник переиспользуется |
| Неверная версия/неработающий бинарник в кэше | Закреплённая переустановка, версия проверена, временные данные удалены |
| Ошибка компилятора | Сохранён exit 23, удалены частичные output, соседние файлы сохранены |
| Неверная версия после установки | Exit 1, очистка выполнена |
| RUNNER_TEMP=/ и лишний аргумент | Оба вызова отвергнуты с exit 1 |
| YAML/сохранение охвата | Прежние команды проверок и paths, матрицы, тайм-ауты, concurrency и permissions сопоставлены с базой; нет continue-on-error, actions закреплены SHA |
| Документация и diff | Локальные ссылки/ограждения проверены, git diff --check пройден |
| CLI --version | Прямой запуск существующего реального cargo-tauri на Windows возвращает ожидаемую форму tauri-cli 2.12.4 |
| GitHub PR #22 head 4a420fd | Все четыре jobs прошли: Windows environment, foundation Windows/Linux и Android. Реальная холодная установка CLI/очистка, APK/instrumentation x86_64/arm64 и 18 Android-тестов Automerge, APK/WebView/HTTP/lifecycle пройдены |
| Main cache save/restore и cold/warm | Восемь jobs на одном 08cb948 успешны с первой попытки; сохранение и точные hits Tauri/Rust host Windows/Linux/Android/Gradle подтверждены логами и API. Длительности ниже |

Проверки установщика выполнены на Windows через Git Bash с jq 1.8.2
(архивная SHA256 сверена с release checksums), Cargo заменён заглушкой.
Это проверяет ветвления/коды/cleanup скрипта, а не реальные Linux toolchain
и ABI. Полный runtime `bun run check`/APK заново локально не запускался:
runtime и сами команды проверок не менялись. Скриптовые тесты включены в
Android Actions до установки SDK. Локальные логи — docs/reports/ci-01/.

## Передача и пять правил

[PR #22](https://github.com/Zeyafix/zedone/pull/22), head 4a420fd9c1f1c915748b200d981d10fac1244103,
интегрирован в 08cb948f9132fb78b70fdf8568de7e74ee8ae619 после четырёх успешных jobs:
[foundation](https://github.com/Zeyafix/zedone/actions/runs/41613881725),
[Android](https://github.com/Zeyafix/zedone/actions/runs/41613881709),
[Windows environment](https://github.com/Zeyafix/zedone/actions/runs/41613881681).
Исходная ветка удалена локально и на GitHub; результат оформляется в
docs/ci-cache-results. CI.01 завершён после проверки main cache save/restore
и измерений на одном SHA. Локальная передача — docs/reports/ci-01/.

В PR были cache misses, запись отключена по политике. Реальная сборка CLI
вывела 1.4G временных данных перед удалением; установочный root — 42M.
Итоговый диск runner — 145G всего/64G свободно, SDK 8.2G, Cargo 1.1G,
Gradle 1.3G, workspace target 7.1G. Это наблюдение конкретного runner,
не доказательство всех причин прежней нехватки места или прогноза для меньшего диска.

## Post-merge cold/warm на одном SHA

Все запуски используют 08cb948f9132fb78b70fdf8568de7e74ee8ae619 и прежние
полные проверки, без изменения runtime/workflow между замерами. Cold — первый
push в main с пустым кэшем, warm — workflow_dispatch после сохранения кэшей.
Повторы разных workflow запускались независимо; таблица показывает
`job.completed_at - job.started_at`, без очереди, а не общее время ожидания
всего эксперимента. Все восемь jobs успешны с первой попытки.

| Job | Cold main | Warm | Изменение длительности |
| --- | --- | --- | --- |
| Android | 22:32 | 9:06 | −59.6% |
| Foundation Windows | 13:02 | 5:41 | −56.4% |
| Foundation Linux | 4:10 | 2:03 | −50.8% |
| Windows environment (без кэша) | 0:40 | 0:41 | +1 секунда |
| Сумма времени четырёх jobs | 40:24 | 17:31 | −56.6% |

Источники cold: [Android](https://github.com/Zeyafix/zedone/actions/runs/41617279459),
[foundation](https://github.com/Zeyafix/zedone/actions/runs/41617279105),
[Windows environment](https://github.com/Zeyafix/zedone/actions/runs/41617279248).
Warm: [Android](https://github.com/Zeyafix/zedone/actions/runs/41620530997),
[foundation](https://github.com/Zeyafix/zedone/actions/runs/41619414571),
[Windows environment](https://github.com/Zeyafix/zedone/actions/runs/41619418877).

В обеих частях пары совпали версии образов: Ubuntu 20261025.353.1,
Windows 20261030.294.1. VM разные; это одна пара, не p95 и не гарантия времени
будущего PR. Cold включает разовую запись кэшей (Android Rust post — 98 с,
Gradle post — 12 с), warm использует уже созданные записи. Сумма jobs —
расход времени runners, её нельзя выдавать за длительность ожидания PR.

Подтверждены точные cache hits трёх Rust кэшей, Tauri CLI и обеих частей
Gradle (dependencies/wrapper). API показывает шесть записей refs/heads/main,
сумма size_in_bytes 3 093 131 248 (около 2.88 GiB). Других записей warm не создал.
В реальном CLI-шаге warm выведено `Reusing tauri-cli 2.12.4`; установка/проверка
заняла менее секунды против 5:56 cold, восстановление бинарника — около секунды.
Android Rust restore занял 23 с. Host endpoint 1:21 → 0:09, x86_64 APK и
instrumentation 4:26 → 1:17, arm64 1:13 → 0:30.

В warm реально выполнены native Automerge (18/18), обе ABI, APK/WebView/HTTP/
lifecycle, отказы и cleanup; полные foundation Windows/Linux также пройдены.
Пропусков тестов нет. SDK, загрузка эмулятора и сами сценарии остаются в job;
SDK здесь даже занял дольше (1:44 → 2:10), что подчёркивает вариативность среды.
Дубликатов push feature-ветки нет. На обычном изменении prototypes остаются
три jobs вместо шести; четвёртый environment job этого среза вызван изменением
его собственного workflow. Нехватки места в этих запусках не было; это не
устанавливает все причины прежнего сбоя. Сырые JSON/логи и расчёт — локальные
docs/reports/ci-01/cold.json, warm.json, summary.json.

Обратимость: обратный коммит/удаление кэшей; cache miss имеет холодный путь.
Человек/агент: те же команды и установщик/проверка. Документация обновлена.
Паритет: Windows/Linux/Android jobs и обе ABI подтверждены новым PR-прогоном.
Источник истины: прежние environment-versions.json/lockfiles/контракты;
кэши их не заменяют. T16/T17 сохраняют passed в принятом охвате; продуктовые
критерии не повышаются по успеху инфраструктурных проверок.
