# Quick Start: Windows 11

Инструкция для разработчика и AI-агента на **Windows 11 x64**. Сначала подготовьте базовые инструменты и проверьте Rust-сборку, затем добавьте Android-окружение. Репозиторий пока содержит проектную документацию: команды запуска самого ZeDone появятся вместе с прототипом.

Состояние текущей машины (`state.json`), отчёты, доказательства проверок и снимки настроек сохраняйте в `docs/reports/`. Загрузки, распаковки и проверочные примеры размещайте в `temp/`: их можно безопасно удалить и воссоздать командами ниже. Обе папки исключены из Git. Инструменты устанавливаются в штатные каталоги; отдельная папка `environment/` не требуется. При наличии старых отчётов в `temp/environment/` сначала перенесите их по [локальному порядку](../agent/work-queue.md#локальная-подготовка-машины).

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

Проверьте компиляцию и линковку на изолированном примере в `temp/rust-smoke/`. Повторный запуск использует существующий пример или создаёт его заново после очистки `temp/`; результат проверки сохраняйте в `docs/reports/`. После каждой команды Cargo код завершения должен быть `0`; при ошибке устраните её перед продолжением.

```powershell
if (-not (Test-Path -LiteralPath temp/rust-smoke/Cargo.toml)) {
    cargo new --bin --vcs none --name zedone_environment_probe temp/rust-smoke
}
cargo build --manifest-path temp/rust-smoke/Cargo.toml --offline
cargo run --manifest-path temp/rust-smoke/Cargo.toml --offline
cargo fmt --manifest-path temp/rust-smoke/Cargo.toml --check
cargo clippy --manifest-path temp/rust-smoke/Cargo.toml --offline -- -D warnings
```

Ожидается `Hello, world!`, успешные сборка, форматирование и Clippy. Здесь нет внешних Rust-зависимостей, поэтому после установки toolchain проверка работает без сети. Успех подтверждает базовый компилятор/линкер, а сборка зависимостей ZeDone проверяется отдельно в P01.

Если `cargo` не найден, откройте новый терминал и проверьте доступность `%USERPROFILE%\.cargo\bin`. Если нет активного toolchain — повторите `rustup default` выше. Если не найден линкер или системная библиотека — проверьте компоненты MSVC x64/x86 и Windows SDK в Visual Studio Installer и повторите сборку.

## 3. Подготовьте Android (P00.03)

Основа от 2026-10-27 — [Android Studio Stable](https://developer.android.com/studio), [официальный каталог SDK](https://dl.google.com/android/repository/repository2-3.xml), [NDK LTS](https://developer.android.com/ndk/downloads) и [требования Tauri](https://v2.tauri.app/start/prerequisites/#android). Это версии подготовки среды; совместимость APK и всего стека проверяется в P01.

| Компонент | Закреплённая база |
| --- | --- |
| Android Studio | Rabbit 1, 2026.2.1.8; JBR/JDK 25.0.3 из каталога `jbr` |
| Command-line Tools | 23.0, Android CLI 1.1.16717390; архив `commandlinetools-win-16111833_latest.zip` |
| SDK Platform / Build-Tools | `platforms/android-37.0`, `platforms/android-33` / `build-tools/37.0.0` |
| Platform-Tools / Emulator | 37.0.1 / 37.2.12; SDK Manager распространяет их под именами `platform-tools` / `emulator` |
| NDK | `ndk/30.0.16248370`, r30 LTS |
| Tauri CLI | 2.12.4 через Cargo |
| Образ AVD | `system-images/android-33/default/x86_64`, revision 2; AOSP без Google Play Services |

SDK 37.0 подготовлен для [актуального шаблона Tauri CLI](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.4/crates/tauri-cli/templates/mobile/android/app/build.gradle.kts); API 33 и образ Android 13 нужны для целевой проверки эмулятора. `compileSdk` и `targetSdk` прототипа ещё проверяются в P01. Для JDK 25 требуется Gradle не ниже 9.1.0 по [матрице совместимости Java](https://docs.gradle.org/current/userguide/compatibility.html); [шаблон CLI 2.12.4](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.4/crates/tauri-cli/templates/mobile/android/gradle/wrapper/gradle-wrapper.properties) задаёт wrapper 9.6.1. Используйте wrapper будущего проекта; отдельная установка Gradle не требуется. Пакеты `platform-tools` и `emulator` обновляются в стабильном канале: после установки записывайте фактические версии локально, не считайте их закреплёнными одним лишь именем пакета.

В Tools 23 управление SDK выполняет [Android CLI](https://developer.android.com/tools/agents/android-cli). В его именах пакетов используются `/`; оставшийся `avdmanager` использует прежние идентификаторы с `;`. Не передавайте эти формы другому инструменту: устаревшая обёртка `sdkmanager` может вывести `Package ... not found` и завершиться с кодом `0`. Проверяйте также список и файлы установленных пакетов. `--no-metrics` отключает метрики вызова Android CLI; вход в Google и установка агентских навыков для подготовки не требуются.

### Android Studio и SDK Manager

Для новой установки используйте официальный ZIP-выпуск Studio в профиле пользователя. IDE запускается из `bin/studio64.exe`; для удобства можно создать ярлык в пользовательском меню Пуск. Если Studio уже установлен, сохраните настройки, обновите его штатно и укажите его каталог в `$studioDirectory` ниже. Не распаковывайте ZIP поверх работающей IDE.

```powershell
$androidSetup = 'temp/android-setup'
New-Item -ItemType Directory -Path $androidSetup -Force | Out-Null
$studioArchive = Join-Path $androidSetup 'android-studio-rabbit1-windows.zip'
Invoke-WebRequest 'https://dl.google.com/dl/android/studio/ide-zips/2026.2.1.8/android-studio-rabbit1-windows.zip' -OutFile $studioArchive
if ((Get-FileHash -LiteralPath $studioArchive -Algorithm SHA256).Hash -ne '6E83E7CE3E0A76C4E8E78D98EB5EF538CBC9FB386C745BDA425FC0BACB9DE6D4') {
    throw 'Контрольная сумма Android Studio не совпала с официальным выпуском.'
}
$studioParent = Join-Path $env:LOCALAPPDATA 'Programs/Android'
New-Item -ItemType Directory -Path $studioParent -Force | Out-Null
Expand-Archive -LiteralPath $studioArchive -DestinationPath $studioParent
$studioDirectory = Join-Path $studioParent 'android-studio'
```

Загрузите Command-line Tools и проверьте SHA256 до распаковки. Архив и временная распаковка остаются в `temp/android-setup/`; результаты проверки сохраняйте в `docs/reports/`. До выполнения следующего блока сохраните прежние значения переменных User и Machine в `docs/reports/`.

```powershell
$toolsArchive = Join-Path $androidSetup 'commandlinetools-win-16111833_latest.zip'
Invoke-WebRequest 'https://dl.google.com/android/repository/commandlinetools-win-16111833_latest.zip' -OutFile $toolsArchive
if ((Get-FileHash -LiteralPath $toolsArchive -Algorithm SHA256).Hash -ne 'E5885E2E59038C0778A85FC19F01DE7CE7C567C105553F7617B936B1E0E87B2B') {
    throw 'Контрольная сумма Command-line Tools не совпала.'
}
Expand-Archive -LiteralPath $toolsArchive -DestinationPath (Join-Path $androidSetup 'tools')
$androidSdkDirectory = Join-Path $env:LOCALAPPDATA 'Android/Sdk'
New-Item -ItemType Directory -Path $androidSdkDirectory -Force | Out-Null
$androidVariables = @{
    JAVA_HOME = Join-Path $studioDirectory 'jbr'
    ANDROID_HOME = $androidSdkDirectory
    NDK_HOME = Join-Path $androidSdkDirectory 'ndk/30.0.16248370'
}
foreach ($entry in $androidVariables.GetEnumerator()) {
    [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, 'User')
    [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, 'Process')
}
$androidCli = Join-Path (Resolve-Path (Join-Path $androidSetup 'tools/cmdline-tools/bin')) 'android.exe'
& "$env:JAVA_HOME/bin/java.exe" -version
& "$env:JAVA_HOME/bin/javac.exe" -version
& $androidCli --no-metrics --version
```

Команды меняют только User и текущий процесс; глобальный PATH менять не требуется. В новом терминале переменные применяются после обновления окружения родительского приложения; при необходимости перечитайте только `JAVA_HOME`, `ANDROID_HOME`, `NDK_HOME` из User в Process. Если уже задан `ANDROID_SDK_ROOT`, он должен совпадать с `ANDROID_HOME`, иначе устраните расхождение перед установкой.

Установите пакеты из стабильного канала; прочитайте и примите запрошенные лицензии SDK. После каждой команды проверяйте `$LASTEXITCODE`: ошибка прерывает подготовку.

```powershell
$androidPackages = @(
    'cmdline-tools/23.0', 'platform-tools', 'emulator',
    'platforms/android-37.0', 'platforms/android-33', 'build-tools/37.0.0',
    'ndk/30.0.16248370', 'system-images/android-33/default/x86_64'
)
foreach ($androidPackage in $androidPackages) {
    & $androidCli --no-metrics "--sdk=$env:ANDROID_HOME" sdk install $androidPackage
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath (Join-Path $env:ANDROID_HOME "$androidPackage/source.properties"))) {
        throw "Пакет не установлен: $androidPackage"
    }
}
$androidCli = Join-Path $env:ANDROID_HOME 'cmdline-tools/23.0/bin/android.exe'
& $androidCli --no-metrics "--sdk=$env:ANDROID_HOME" sdk list
rustup target add --toolchain 1.99.1-x86_64-pc-windows-msvc aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
cargo install tauri-cli --version 2.12.4 --locked
cargo tauri --version
cargo tauri android --help
rustup target list --installed
& "$env:NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64/bin/clang.exe" --version
& "$env:ANDROID_HOME/platform-tools/adb.exe" version
```

Проверьте линковку Rust и NDK на изолированном исполняемом файле для эмулятора. Это проверка toolchain, а не APK ZeDone.

```powershell
$androidProbeDirectory = 'temp/android-rust-smoke'
New-Item -ItemType Directory -Path $androidProbeDirectory -Force | Out-Null
$androidProbeSource = Join-Path $androidProbeDirectory 'main.rs'
$androidProbeExecutable = Join-Path $androidProbeDirectory 'x86_64-linux-android'
'fn main() { println!("ZEDONE_ANDROID_TOOLCHAIN_OK"); }' | Set-Content -LiteralPath $androidProbeSource -Encoding ASCII
$androidClang = Join-Path $env:NDK_HOME 'toolchains/llvm/prebuilt/windows-x86_64/bin/clang.exe'
rustc --edition 2021 $androidProbeSource --target x86_64-linux-android -C "linker=$androidClang" -C 'link-arg=--target=x86_64-linux-android24' -o $androidProbeExecutable
if ($LASTEXITCODE -ne 0) { throw 'Кросс-компиляция Rust/NDK не прошла.' }
```

### Android 13 AVD и adb

Включите виртуализацию в UEFI и **Windows Hypervisor Platform** по [инструкции Android](https://developer.android.com/studio/run/emulator-acceleration). Из PowerShell администратора компонент можно включить командой `Enable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -All -NoRestart`. Если Windows требует перезагрузку, сохраните работу и перезагрузите её перед проверкой. Не отключайте другие гипервизоры или защитные функции ради эмулятора.

```powershell
$emulator = Join-Path $env:ANDROID_HOME 'emulator/emulator.exe'
$avdManager = Join-Path $env:ANDROID_HOME 'cmdline-tools/23.0/bin/avdmanager.bat'
$adb = Join-Path $env:ANDROID_HOME 'platform-tools/adb.exe'
& $emulator -version
& $emulator -accel-check
'no' | & $avdManager create avd --name zedone_api33_aosp_x86_64 --package 'system-images;android-33;default;x86_64' --device pixel_5
& $emulator -list-avds
```

`-accel-check` должен завершиться с кодом `0`. AVD создавайте один раз; для существующего имени проверяйте его конфигурацию, не используйте `--force`. Образ AOSP выбран для проверки работы без Google Play Services; он не заменяет последующие проверки QR-сканера и физического телефона.

Для проверки из командной строки убедитесь через `adb devices` и список слушающих TCP-портов, что пара 5554/5555 свободна, затем запустите AVD без окна. При занятой паре выберите свободную чётную пару, например 5556/5557, и замените `-port` и serial во всех командах; не останавливайте чужой эмулятор. `-gpu software` выбирает программную графику, а CPU продолжает использовать WHPX. Ограничьте ожидание загрузки пятью минутами; при тайм-ауте сохраните логи и не отмечайте шаг выполненным.

```powershell
& $adb devices -l
$reportsDirectory = New-Item -ItemType Directory -Path 'docs/reports' -Force
$emulatorLog = Join-Path $reportsDirectory.FullName 'emulator-stdout.log'
$emulatorErrorLog = Join-Path $reportsDirectory.FullName 'emulator-stderr.log'
Start-Process -FilePath $emulator -WindowStyle Hidden -ArgumentList @('-avd', 'zedone_api33_aosp_x86_64', '-port', '5554', '-no-window', '-no-audio', '-no-boot-anim', '-no-snapshot', '-gpu', 'software', '-accel', 'on', '-memory', '2048', '-cores', '2') -RedirectStandardOutput $emulatorLog -RedirectStandardError $emulatorErrorLog
$bootDeadline = (Get-Date).AddMinutes(5)
do {
    $bootCompleted = (& $adb -s emulator-5554 shell getprop sys.boot_completed 2>$null | Out-String).Trim()
    if ($bootCompleted -eq '1') { break }
    Start-Sleep -Seconds 2
} while ((Get-Date) -lt $bootDeadline)
if ($bootCompleted -ne '1') { throw 'AVD не загрузился за пять минут; проверьте логи.' }
& $adb -s emulator-5554 get-state
& $adb -s emulator-5554 shell getprop sys.boot_completed
& $adb -s emulator-5554 shell getprop ro.build.version.release
& $adb -s emulator-5554 shell getprop ro.build.version.sdk
& $adb -s emulator-5554 shell getprop ro.product.cpu.abilist
& $adb -s emulator-5554 shell pm list packages com.google.android.gms
& $adb -s emulator-5554 push $androidProbeExecutable /data/local/tmp/zedone_environment_probe
& $adb -s emulator-5554 shell chmod 700 /data/local/tmp/zedone_environment_probe
& $adb -s emulator-5554 shell /data/local/tmp/zedone_environment_probe
& $adb -s emulator-5554 shell rm /data/local/tmp/zedone_environment_probe
& $adb -s emulator-5554 emu kill
```

Ожидаются состояние `device`, `sys.boot_completed=1`, Android `13`, API `33`, ABI `x86_64`, пустой список пакетов GMS и вывод `ZEDONE_ANDROID_TOOLCHAIN_OK` с кодом `0`. Последняя команда штатно завершает только выбранный эмулятор. При ошибке проверки также остановите свой AVD этой командой после сохранения логов. Запишите точный образ, ABI, состояние GMS, ускорение и результат загрузки в `docs/reports/`; наличие пакетов без запуска AVD не закрывает T14.

## 4. Добавляйте средства выпуска и связи по задаче

| Задача | Что понадобится | Официальный источник и настройка |
| --- | --- | --- |
| Windows: установщик, P00.04 | Inno Setup и компилятор ISCC | [Inno Setup](https://jrsoftware.org/isdl.php). |
| Связь двух устройств, P00.04 | Tailscale на Windows и Android | [Tailscale](https://tailscale.com/download). Вход и подключение устройств относятся к сетевым проверкам. |
| Проверки CI и выпуск, P00.04–P00.06 | GitHub CLI и доступ к репозиторию/Actions/Releases | [GitHub CLI](https://cli.github.com/). Проверка доступа не требует публикации релиза. |

Таблица служит списком подготовки, а не доказательством готового выпуска. CLI и зависимости Vue/TypeScript/Vite/Tauri будут закреплены вместе с прототипом и lock-файлами.

## Обновление, отмена и результаты

Перед изменениями сохраните в `docs/reports/` список версий, выбранный `rustup show active-toolchain` и изменяемые переменные окружения. Старые отдельные установки удаляйте штатно **после** проверки замены, с учётом других проектов. Для Rust используйте `rustup toolchain uninstall <точное-имя-старого-toolchain>`; сначала проверьте имя через `rustup toolchain list`.

Для отмены восстановите прежние версии официальными установщиками, прежний Rust — через `rustup toolchain install` и `rustup default`. Программы удаляются в **Settings → Apps → Installed apps**, компоненты MSVC/SDK — через Visual Studio Installer или их штатный деинсталлятор. Для Bun используйте тот же способ управления, что при установке; WinGet-установка удаляется через `winget uninstall --id Oven-sh.Bun --exact`. Возвращайте только изменённые вами значения PATH/переменных; сохраняйте профили браузеров, Git-настройки и учётные записи. Тестовый пример можно удалить отдельно из `temp/rust-smoke`.

Для Android сначала остановите свой AVD и IDE. AVD удаляется через `avdmanager delete avd --name zedone_api33_aosp_x86_64`, выбранные SDK-пакеты — через `android --sdk=<каталог-SDK> sdk remove <точный-ID-со-слешами>`, CLI — через `cargo uninstall tauri-cli`, Rust-цели — через `rustup target remove --toolchain 1.99.1-x86_64-pc-windows-msvc <точная-цель>`. Удаление общего SDK-пакета затрагивает все использующие его проекты: сверьте зависимости. ZIP-установка Studio удаляется из её отдельного каталога после проверки абсолютного пути; сохраняйте пользовательские настройки IDE. Восстановите исходные Android-переменные из локального снимка. Если WHP был выключен до подготовки и больше не нужен другим приложениям, его можно выключить штатно через Windows Features или `Disable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -NoRestart` из PowerShell администратора; требуемую перезагрузку выполняет разработчик.

Очистка `temp/` сохраняет установленные инструменты, состояние подготовки, доказательства проверок и снимки для отмены настройки. Нужные повторно загрузки и примеры воссоздаются командами выше. `docs/reports/` хранится отдельно от временных файлов и не удаляется при их очистке.

Агент продолжает работу по [очереди P00](../agent/work-queue.md#p00--окружение) и [плану-журналу](../agent/plan-journal.md). Локальные версии, пути, результаты и EV записываются только в `docs/reports/`, точка продолжения — `docs/reports/state.json`. При отсутствии состояния сначала проверьте фактическое окружение в P00.01 без установки. Этот Quick Start хранит общие требования и команды; готовность каждой машины и полная приёмка T13–T17 подтверждаются отдельно.
