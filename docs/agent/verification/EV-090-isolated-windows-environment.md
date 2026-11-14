# EV-090 — независимый Windows-пробник окружения

Дата: 2026-11-14. Шаг: CI.07. Состояние: verified локально и Windows CI head 19a5a3f; интеграция ожидается.

## Карточка шага

- Цель: проверить закреплённые Windows-инструменты без зависимости от Cargo
  workspace и загруженности продуктового реестра.
- Основание: PR #87/cbf90ec, Windows environment/job 132347027160.
  `cargo init` в repo `temp/` добавил пробник в корневой workspace;
  offline build потребовал `icu_casemap` через `zedone-core` и отказал.
  Пробник без зависимостей не должен разрешать зависимости продукта.
- Scope: `scripts/check-windows-environment.ps1`, developer docs и журнал.
  Продукт, UI, версии, workflow matrix, бюджеты и набор команд прежние.
- Изменение: уникальный каталог в системном временном каталоге и собственный
  `[workspace]` пробника. Сохраняются build/run/Clippy `--offline`, fmt/check,
  проверка `Hello, world!` и ненулевых исходов. Очистка в `finally` использует
  проверенный абсолютный путь непосредственного потомка temp root.
- Проверка: полный настоящий скрипт на Windows 11 x64, PowerShell 7.7.0,
  Rust MSVC 1.99.1, Bun 1.4.4, Visual Studio 18.11/SDK 10.0.28000.0 passed.
  Build/run/fmt/Clippy прошли, `Hello, world!` совпал. Корневой Cargo.toml
  сохранил SHA-256 до/после, собственный временный каталог удалён.
  Actionlint Windows/Android workflows и host/Android routing guards passed.
- Обратимость: правка отменяется Git-коммитом; проверка создаёт и удаляет только
  собственный пробник, не меняет локальный toolchain или пользовательские данные.
- Пять правил: обратимость проверочного каталога; один и тот же CLI для человека
  и агента; developer/agent docs обновлены; продуктовый паритет не меняется;
  версии читаются из прежнего единственного environment-versions.json.
- Граница: стандартный Windows CI runner ещё должен подтвердить исправление.
  Прежняя продуктовая приёмка и все 98 определения/состояния критериев прежние.

Связь: GAP-026, T13/T16 (сохранение охвата), LOG-489. Сырые логи остаются
только в игнорируемой `docs/reports/p05-09/`.

Опубликовано в PR #87/head 19a5a3f; Windows environment и все runtime suites
IN_PROGRESS, три aggregates ожидаются. Нужны все 18 SUCCESS/readiness;
GAP-026 не закрыт до стандартного runner. LOG-490.

Windows environment head 19a5a3f/job 132349463238 завершён SUCCESS на
стандартном windows-2025. Raw log подтверждает собственный системный temp,
все offline build/run/Clippy, fmt/check и точный `Hello, world!`, затем
`Windows environment check passed.` Включения в root workspace и обращения
к продуктовой `icu_casemap` нет. GAP-026 resolved в scope Windows-пробника;
весь PR ещё требует остальных checks/readiness. LOG-491.
