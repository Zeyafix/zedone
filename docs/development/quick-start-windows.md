# Quick Start: Windows 11

Инструкция для разработчика и AI-агента на **Windows 11 x64**. Сначала подготовьте базовые инструменты и проверьте Rust-сборку; Android и средства выпуска понадобятся на следующих шагах. Репозиторий пока содержит проектную документацию: команды запуска самого ZeDone появятся вместе с прототипом.

## 1. Установите базовые инструменты

Используйте стабильные выпуски. Ниже закреплена стартовая база P00.02 от 2026-10-27; совместимость всей цепочки Vue/Vite/OpenAPI/Tauri ещё проверяется в P01. Перед будущим обновлением сверяйте официальные выпуски и повторяйте проверки. Сведения конкретной машины в этот документ не записываются.

| Инструмент | Стартовая версия | Для чего нужен и откуда установить |
| --- | --- | --- |
| Git for Windows | 2.56.0.windows.4 | Репозиторий. [Официальный выпуск, x64 Setup](https://github.com/git-for-windows/git/releases/tag/v2.56.0.windows.4). В установщике оставьте доступ из командной строки. |
| PowerShell | 7.6.9 | Общие команды разработчика и агента. [Установка Microsoft](https://learn.microsoft.com/en-us/powershell/scripting/install/installing-powershell-on-windows). |
| Visual Studio Build Tools | 2026, 18.11.2 | Компилятор MSVC C++ и линкер для Rust и нативных зависимостей. [Stable Build Tools](https://aka.ms/vs/stable/vs_buildtools.exe), [описание установки Microsoft](https://learn.microsoft.com/en-us/cpp/overview/acquire-msvc). |
| Windows 11 SDK | 10.0.28000.2957 | Заголовки и библиотеки Windows; выберите SDK 28000 в Build Tools. [Выпуски Microsoft](https://learn.microsoft.com/en-us/windows/apps/windows-sdk/downloads). |
| rustup / Rust | 1.29.2 / 1.99.1, MSVC x64 | Rust, Cargo, rustfmt и Clippy. [Официальный rustup-init.exe](https://rustup.rs/), [установка Rust](https://doc.rust-lang.org/book/ch01-01-installation.html). |
| Bun | 1.4.4 | Пакеты и инструменты веб-интерфейса. [Установка Bun](https://bun.sh/docs/installation), [закреплённый выпуск](https://github.com/oven-sh/bun/releases/tag/bun-v1.4.4). |
| Google Chrome и Opera | Stable, доступный вашему устройству | Два целевых браузера Windows. [Chrome](https://www.google.com/chrome/), [Opera](https://www.opera.com/download). |

Для команд нужен WinGet из **App Installer**: обычно он уже есть в Windows 11. Проверьте `winget --version`; если команды нет, установите [App Installer по инструкции Microsoft](https://learn.microsoft.com/en-us/windows/package-manager/winget/). Установка Build Tools и системных программ может запросить права администратора. Дожидайтесь завершения каждого установщика перед запуском следующей установки или удаления. Сборку выполняйте в обычном терминале.

### PowerShell, Bun и Build Tools

В PowerShell выполните:

```powershell
winget install --id Microsoft.PowerShell --exact --source winget --version 7.6.9
winget install --id Oven-sh.Bun --exact --source winget --version 1.4.4
winget install --id Microsoft.VisualStudio.BuildTools --exact --source winget --version 18.11.2 --override "--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.28000"
```

Вместо последней команды можно открыть установщик Build Tools: выберите **Desktop development with C++**, актуальные **MSVC x64/x86 build tools** и **Windows 11 SDK 28000**. Затем проверьте патч SDK: компонент Build Tools может содержать более старый патч. Для базы из таблицы установите **10.0.28000.2957** через ссылку **Installer** на странице SDK выше. После установки откройте новый терминал **PowerShell 7** (`pwsh`). Если установщик требует перезагрузку, выполните её перед проверкой сборки.

Для Git используйте x64 Setup из закреплённого официального выпуска выше. [Каталог WinGet может отставать от Git for Windows](https://git-scm.com/install/windows); аналогично сверяйте источник при подготовке других инструментов. Обновляйте существующую установку тем же способом, которым её устанавливали, сохраняя настройки и профили.

### Rust

Запустите `rustup-init.exe` с [rustup.rs](https://rustup.rs/) и выберите MSVC x64. В новом PowerShell 7 выполните:

```powershell
rustup self update
rustup toolchain install 1.99.1-x86_64-pc-windows-msvc --profile minimal --component rustfmt --component clippy
rustup default 1.99.1-x86_64-pc-windows-msvc
```

Последняя команда обязательна: установленный toolchain без выбора по умолчанию не делает обычные `cargo` и `rustc` рабочими. MSVC/SDK обнаруживаются Rust автоматически; для обычной сборки не требуется вручную добавлять `cl.exe` и `link.exe` в глобальный PATH. Подробности — [rustup для Windows](https://rust-lang.github.io/rustup/installation/windows.html).

### Браузеры и Node.js

Установите Chrome и Opera из официальных источников в таблице. В существующем Chrome откройте **Help → About Google Chrome**, в Opera — **Update & Recovery**, примените обновление и перезапустите браузер после сохранения работы. Chrome развёртывает стабильные обновления постепенно: ориентируйтесь на доступный устройству Stable. [Обновление Chrome](https://support.google.com/chrome/answer/95414), [обновление Opera](https://help.opera.com/en/latest/crashes-and-issues/).

**Node.js добавляется по необходимости конкретного инструмента.** Для пакетов и запуска инструментов выбран Bun; потребность openapi-typescript или Tauri в Node.js ещё проверяется в P01 по [ADR-0003](../architecture/decisions/0003-api-contract-toolchain.md). Если такая зависимость подтвердится, здесь появятся причина, версия и команда установки. Отдельный сервер SQLite не требуется: проект использует `rusqlite` с `bundled`.

## 2. Получите репозиторий и проверьте окружение

При первом запуске клонируйте репозиторий; для уже открытой рабочей копии используйте её корень:

```powershell
git clone https://github.com/Zeyafix/zedone.git
Set-Location zedone
```

В PowerShell 7 проверьте инструменты:

```powershell
$PSVersionTable.PSVersion
git --version
rustup --version
rustup show active-toolchain
rustc --version
cargo --version
rustfmt --version
cargo clippy --version
bun --version
```

Проверьте компиляцию и линковку на изолированном примере. Все файлы остаются в игнорируемом `temp/environment/`; повторный запуск использует тот же пример. После каждой команды Cargo код завершения должен быть `0`; при ошибке устраните её перед продолжением.

```powershell
if (-not (Test-Path -LiteralPath temp/environment/rust-smoke/Cargo.toml)) {
    cargo new --bin --vcs none --name zedone_environment_probe temp/environment/rust-smoke
}
cargo build --manifest-path temp/environment/rust-smoke/Cargo.toml --offline
cargo run --manifest-path temp/environment/rust-smoke/Cargo.toml --offline
cargo fmt --manifest-path temp/environment/rust-smoke/Cargo.toml --check
cargo clippy --manifest-path temp/environment/rust-smoke/Cargo.toml --offline -- -D warnings
```

Ожидается `Hello, world!`, успешные сборка, форматирование и Clippy. Здесь нет внешних Rust-зависимостей, поэтому после установки toolchain проверка работает без сети. Успех подтверждает базовый компилятор/линкер, а сборка зависимостей ZeDone проверяется отдельно в P01.

Если `cargo` не найден, откройте новый терминал и проверьте доступность `%USERPROFILE%\.cargo\bin`. Если нет активного toolchain — повторите `rustup default` выше. Если не найден линкер или системная библиотека — проверьте компоненты MSVC x64/x86 и Windows SDK в Visual Studio Installer и повторите сборку.

## 3. Добавляйте остальные инструменты по задаче

| Задача | Что понадобится | Официальный источник и настройка |
| --- | --- | --- |
| Android: сборка и отладка, P00.03 | Android Studio Stable с JBR/JDK; SDK Platform, Build-Tools, Platform-Tools (`adb`), Command-line Tools, NDK Side by side; Rust-цели и Tauri CLI | [Android Studio](https://developer.android.com/studio), [SDK Manager](https://developer.android.com/studio/intro/update#sdk-manager), [подготовка Android для Tauri](https://v2.tauri.app/start/prerequisites/#android). `JAVA_HOME` указывает на JBR Android Studio, `ANDROID_HOME` — на SDK, `NDK_HOME` — на конкретную версию NDK. |
| Android: проверка без телефона, P00.03 | Android Emulator и Android 13 / API 33 AVD; виртуализация и Windows Hypervisor Platform | [Создание AVD](https://developer.android.com/studio/run/managing-avds), [ускорение эмулятора](https://developer.android.com/studio/run/emulator-acceleration). Для Windows x64 выбирайте образ x86_64, фиксируйте наличие Google Play Services для QR-сканера. Эмулятор не заменяет проверку целевого телефона. |
| Windows: установщик, P00.04 | Inno Setup и компилятор ISCC | [Inno Setup](https://jrsoftware.org/isdl.php). |
| Связь двух устройств, P00.04 | Tailscale на Windows и Android | [Tailscale](https://tailscale.com/download). Вход и подключение устройств относятся к сетевым проверкам. |
| Проверки CI и выпуск, P00.04–P00.06 | GitHub CLI и доступ к репозиторию/Actions/Releases | [GitHub CLI](https://cli.github.com/). Проверка доступа не требует публикации релиза. |

Android-версии и команды ещё закрепляются в P00.03; таблица служит списком подготовки, а не доказательством готового APK. JDK берите из Android Studio и проверяйте совместимость с Gradle wrapper проекта. CLI и зависимости Vue/TypeScript/Vite/Tauri будут закреплены вместе с прототипом и lock-файлами; Gradle запускается через wrapper проекта. Переменные Android, точные ABI и лицензии SDK проверяются в P00.03.

## Обновление, отмена и результаты

Перед изменениями сохраните список версий, выбранный `rustup show active-toolchain` и изменяемые переменные окружения. Старые отдельные установки удаляйте штатно **после** проверки замены, с учётом других проектов. Для Rust используйте `rustup toolchain uninstall <точное-имя-старого-toolchain>`; сначала проверьте имя через `rustup toolchain list`.

Для отмены восстановите прежние версии официальными установщиками, прежний Rust — через `rustup toolchain install` и `rustup default`. Программы удаляются в **Settings → Apps → Installed apps**, компоненты MSVC/SDK — через Visual Studio Installer или их штатный деинсталлятор. Для Bun используйте тот же способ управления, что при установке; WinGet-установка удаляется через `winget uninstall --id Oven-sh.Bun --exact`. Возвращайте только изменённые вами значения PATH/переменных; сохраняйте профили браузеров, Git-настройки и учётные записи. Тестовый пример можно удалить отдельно из `temp/environment/rust-smoke`.

Агент продолжает работу по [очереди P00](../agent/work-queue.md#p00--окружение) и [плану-журналу](../agent/plan-journal.md). Локальные версии, пути, результаты и EV записываются только в `temp/environment/`, точка продолжения — `temp/environment/state.json`. Этот Quick Start хранит общие требования и команды; готовность каждой машины и полная приёмка T13–T17 подтверждаются отдельно.
