# Quick Start: Windows 11

Инструкция для разработчика и AI-агента на **Windows 11 x64**. Сначала подготовьте базовые инструменты и проверьте Rust-сборку, затем добавьте Android-окружение. Репозиторий пока содержит проектную документацию: команды запуска самого ZeDone появятся вместе с прототипом.

Состояние текущей машины (`state.json`), отчёты, доказательства проверок и снимки настроек сохраняйте в `docs/reports/`. Загрузки, распаковки и проверочные примеры размещайте в `temp/`: их можно безопасно удалить и воссоздать командами ниже. Обе папки исключены из Git. Инструменты устанавливаются в штатные каталоги; отдельная папка `environment/` не требуется. При наличии старых отчётов в `temp/environment/` сначала перенесите их по [локальному порядку](../agent/work-queue.md#локальная-подготовка-машины).

Полный порядок повторной проверки, чистой установки и сохранения доказательств — [P00.07: воспроизведение окружения](environment-reproduction.md). Успешный повтор на настроенном ПК и установка инструментов с нуля имеют разный охват.

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

Перед первым коммитом настройте GitHub noreply и проверку pre-push по [правилам публикуемых данных](repository-privacy.md). Локальные отчёты и настройки машины не включаются в Git.

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
$environmentVersions = Get-Content -Raw -LiteralPath 'scripts/environment-versions.json' | ConvertFrom-Json
$androidPackages = @("cmdline-tools/$($environmentVersions.android.command_line_tools)") + $environmentVersions.android.packages
foreach ($androidPackage in $androidPackages) {
    & $androidCli --no-metrics "--sdk=$env:ANDROID_HOME" sdk install $androidPackage
    $androidPackageId = $androidPackage.Split('@')[0]
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath (Join-Path $env:ANDROID_HOME "$androidPackageId/source.properties"))) {
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

Суффиксы `@version` берутся из того же JSON, что Linux CI: установка `platform-tools` или `emulator` без версии могла бы незаметно получить другой выпуск. Сверьте `sdk list` и `Pkg.Revision` в `source.properties` каждого пакета с JSON; наличие файла подтверждает установку, но не совпадение версии. Общие версии Rust/Bun и Android для проверок задаёт [environment-versions.json](../../scripts/environment-versions.json); таблицы выше поясняют назначение компонентов.

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

## 4. Подготовьте средства выпуска и связи (P00.04)

Сначала проверьте существующие установки и доступы. Рабочие инструменты не требуется обновлять ради проверки. Для новой установки на Windows x64 закреплена база от 2026-10-27:

| Инструмент | Версия | Официальный источник |
| --- | --- | --- |
| Inno Setup / ISCC | 7.1.0, x64 | [Загрузка](https://jrsoftware.org/isdl.php), [выпуск и контрольные суммы](https://github.com/jrsoftware/issrc/releases/tag/is-7_1_0), [лицензия](https://jrsoftware.org/files/is/license.txt). |
| Tailscale | 1.104.2, Stable | [Пакеты Windows и Android](https://pkgs.tailscale.com/stable/); у файлов доступна контрольная сумма по адресу с суффиксом `.sha256`. |
| GitHub CLI | 2.103.3 | [Закреплённый выпуск](https://github.com/cli/cli/releases/tag/v2.103.3), [установка](https://cli.github.com/). |

Эта база задаёт версии новых установок. Воспроизведение всей цепочки на чистой машине принимается в P00.07; фактически проверенные версии текущей машины записываются отдельно в `docs/reports/`.

### Inno Setup: установка и компиляция

Проверьте `Get-Command ISCC.exe -ErrorAction SilentlyContinue`, пользовательский каталог ниже и существующие установки в **Installed apps**. Если Inno Setup уже установлен, используйте его фактический путь и запишите версию. Следующий блок предназначен только для отсутствующей установки: он проверяет SHA256 и Authenticode, устанавливает для текущего пользователя без изменения PATH и сохраняет журнал. [Параметры установщика](https://jrsoftware.org/ishelp/topic_setupcmdline.htm).

```powershell
$ErrorActionPreference = 'Stop'
$releaseSetup = New-Item -ItemType Directory -Path 'temp/release-setup' -Force
$releaseReports = New-Item -ItemType Directory -Path 'docs/reports' -Force
$innoDirectory = Join-Path $env:LOCALAPPDATA 'Programs/Inno Setup 7'
if (Test-Path -LiteralPath $innoDirectory) { throw 'Каталог уже существует; сначала проверьте установку.' }
$innoInstaller = Join-Path $releaseSetup.FullName 'innosetup-7.1.0-x64.exe'
Invoke-WebRequest 'https://github.com/jrsoftware/issrc/releases/download/is-7_1_0/innosetup-7.1.0-x64.exe' -OutFile $innoInstaller
if ((Get-FileHash -LiteralPath $innoInstaller -Algorithm SHA256).Hash -ne '0362A383ED217D4C4239B5933866DD96D3EB2102737DA92F80F6057A4B40DF2F') {
    throw 'Контрольная сумма Inno Setup не совпала.'
}
$innoSignature = Get-AuthenticodeSignature -LiteralPath $innoInstaller
if ($innoSignature.Status -ne 'Valid' -or $innoSignature.SignerCertificate.Subject -notmatch 'CN=Pyrsys B.V.,') {
    throw 'Подпись Inno Setup не подтверждена.'
}
$innoArguments = @(
    '/CURRENTUSER', '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART',
    '/RESTARTEXITCODE=3010', '/NOCLOSEAPPLICATIONS', '/SP-', '/NOICONS', '/TASKS=""',
    ('/DIR="' + $innoDirectory + '"'),
    ('/LOG="' + (Join-Path $releaseReports.FullName 'inno-install.log') + '"')
)
$innoProcess = Start-Process -FilePath $innoInstaller -WindowStyle Hidden -ArgumentList $innoArguments -Wait -PassThru
if ($innoProcess.ExitCode -ne 0) { throw "Установка не принята; код $($innoProcess.ExitCode). Проверьте журнал; 3010 требует перезагрузки." }
```

Для существующей установки задайте `$iscc` её фактическим путём. У новой установки он следующий. [ISCC](https://jrsoftware.org/ishelp/topic_compilercmdline.htm) должен вернуть `0` при запросе версии и компиляции синтетического примера:

```powershell
$iscc = Join-Path $env:LOCALAPPDATA 'Programs/Inno Setup 7/ISCC.exe'
& $iscc --version
if ($LASTEXITCODE -ne 0) { throw 'ISCC недоступен.' }
$isccProbe = New-Item -ItemType Directory -Path 'temp/iscc-smoke' -Force
'ZeDone environment probe' | Set-Content -LiteralPath (Join-Path $isccProbe.FullName 'probe.txt') -Encoding ASCII
@'
[Setup]
AppName=ZeDone Environment Probe
AppVersion=0.0.0
DefaultDirName={localappdata}\ZeDoneEnvironmentProbe
PrivilegesRequired=lowest
OutputDir=.
OutputBaseFilename=zedone-environment-probe
[Files]
Source: "probe.txt"; DestDir: "{app}"
'@ | Set-Content -LiteralPath (Join-Path $isccProbe.FullName 'probe.iss') -Encoding UTF8
& $iscc --no-ide-signtools (Join-Path $isccProbe.FullName 'probe.iss')
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath (Join-Path $isccProbe.FullName 'zedone-environment-probe.exe'))) {
    throw 'Проверочная компиляция ISCC не прошла.'
}
```

Сохраните команду, версию, код завершения и вывод в `docs/reports/`. Полученный EXE остаётся в `temp/iscc-smoke/`; запускать и публиковать его для P00.04 не требуется. Установщик ZeDone, обновление и удаление с сохранением пользовательских данных проверяются отдельно в P08.02.

### Tailscale: доступность клиента и состояние связи

Если Tailscale или GitHub CLI отсутствуют, установите их; команды могут запросить системные права. Для существующей установки сначала выполните проверки ниже. Версию, источник и состояние до изменений сохраните в `docs/reports/`.

```powershell
winget install --id Tailscale.Tailscale --exact --source winget --version 1.104.2
if ($LASTEXITCODE -ne 0) { throw 'Установка Tailscale не подтверждена.' }
winget install --id GitHub.cli --exact --source winget --version 2.103.3
if ($LASTEXITCODE -ne 0) { throw 'Установка GitHub CLI не подтверждена.' }
```

В новом терминале проверьте клиент и локальную службу. Если PATH родительского приложения ещё не обновился, используйте полные пути: `%ProgramFiles%\Tailscale\tailscale.exe` и `%ProgramFiles%\GitHub CLI\gh.exe`. [Команды Tailscale](https://tailscale.com/docs/reference/tailscale-cli):

```powershell
tailscale version
if ($LASTEXITCODE -ne 0) { throw 'Клиент Tailscale недоступен.' }
Get-Service -Name Tailscale | Select-Object Name, Status, StartType
New-Item -ItemType Directory -Path 'docs/reports' -Force | Out-Null
$tailscaleStateText = tailscale status --json
if ($LASTEXITCODE -ne 0) { throw 'Нет ответа локальной службы Tailscale.' }
$tailscaleState = $tailscaleStateText | Out-String | ConvertFrom-Json
$tailscaleState.PSObject.Properties.Remove('AuthURL')
$tailscaleState | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath 'docs/reports/tailscale-status.json' -Encoding UTF8
$tailscaleState | Select-Object BackendState, Health
tailscale netcheck --format=json > docs/reports/tailscale-netcheck.json
if ($LASTEXITCODE -ne 0) { throw 'Диагностика сети Tailscale не прошла.' }
```

Код `0` команды `status --json` подтверждает ответ службы; подключение оценивайте по `BackendState`, `Self.Online`, `CurrentTailnet` и `Health`. `NeedsLogin`, `NoState` или `Stopped` требуют отдельной записи ограничения. `netcheck` проверяет условия сети и доступность реле, но не доказывает связь двух устройств. Полный вывод содержит адреса и идентификаторы: он остаётся в игнорируемых локальных отчётах.

В P00.04 достаточно подготовить клиент и записать недостающие действия. Если ещё нужен вход, одобрение устройства или доступ к tailnet, их выполняет владелец перед P01.11/P06; ключи и URL авторизации в отчёт не сохраняются. На Android используйте официальный клиент из таблицы, включая APK для AOSP без Google Play. Вход, разрешение VPN и взаимодействие Windows/Android проверяются в сетевом прототипе P01.11; физическое устройство не заменяется эмулятором. Подключение Tailscale само по себе не выдаёт прав на пространства ZeDone.

### GitHub: проверка имеющихся прав

Команды выполняются в корне репозитория, только читают API и не создают релиз или workflow. Не используйте `gh auth token` и `--show-token`; строку с замаскированным токеном также исключайте при сохранении вывода. Если вход отсутствует, владелец выполняет [авторизацию GitHub CLI](https://cli.github.com/manual/gh_auth_login); уже настроенную учётную запись не переключайте автоматически.

```powershell
gh --version
gh auth status --active --hostname github.com
if ($LASTEXITCODE -ne 0) { throw 'Авторизация GitHub не подтверждена.' }
gh api repos/Zeyafix/zedone --jq '{private: .private, permissions: .permissions, archived: .archived}'
if ($LASTEXITCODE -ne 0) { throw 'Репозиторий недоступен.' }
gh api repos/Zeyafix/zedone/actions/permissions
if ($LASTEXITCODE -ne 0) { throw 'Политика Actions не прочитана; запишите отсутствующее право.' }
gh api 'repos/Zeyafix/zedone/actions/workflows?per_page=1' --jq '{total_count: .total_count}'
if ($LASTEXITCODE -ne 0) { throw 'Список workflows недоступен.' }
gh api 'repos/Zeyafix/zedone/actions/runs?per_page=1' --jq '{total_count: .total_count}'
if ($LASTEXITCODE -ne 0) { throw 'Список запусков Actions недоступен.' }
gh api 'repos/Zeyafix/zedone/releases?per_page=1' --jq 'length'
if ($LASTEXITCODE -ne 0) { throw 'Список Releases недоступен.' }
```

Пустые списки с кодом `0` означают отсутствие объектов. Проверяйте `permissions.push`, состояние архива и `enabled` у Actions. Публичное чтение Releases не доказывает возможность публикации. [Получение политики Actions](https://docs.github.com/en/rest/actions/permissions#get-github-actions-permissions-for-a-repository) требует отдельных прав; отказ этой административной проверке сам по себе не означает запрет запуска CI. Для следующего шага нужны возможность добавить workflow и разрешённые Actions. Если workflow записывается через classic/OAuth-токен, проверяется scope `workflow`; у fine-grained токена — соответствующие права репозитория. Текущие scopes можно увидеть в `gh auth status`; расширять их заранее не требуется.

Запишите подтверждённые права, отказы и непроверенные операции. Проверка права записи по метаданным не заменяет будущий реальный запуск CI в P00.05–P00.06 и не разрешает публикацию. [Создание релиза](https://docs.github.com/en/rest/releases/releases#create-a-release) требует `Contents: write`; эта операция в P00.04 не выполняется.

### Что готовится позднее

| Средство или полномочие | Когда подготовить и проверить |
| --- | --- |
| Android `keytool`, `apksigner`, release keystore и безопасное хранение его паролей/резервной копии | Инструменты уже входят в JDK/Build-Tools; закрепление цепочки сборки — P01.04, ключ подписи и проверка обновления APK — P08.03. Наличие инструментов не подтверждает наличие ключа. |
| Windows SignTool, сертификат подписи и служба меток времени | SignTool проверяется по фактическому пути Windows SDK; решение о подписи и необходимые средства/полномочия готовятся до использования в P08.02/P08.06. Покупка сертификата в P00 не требуется. |
| GitHub CI secrets, разрешения `GITHUB_TOKEN`, публикация и загрузка артефактов Releases | Минимальные права CI — P00.05–P00.06; секреты подписи и права выпуска — P08.03/P08.06. Публикация требует отдельного поручения. |
| Tailnet, вход и VPN-разрешение Android, сетевые правила | До проверки транспорта P01.11 и сценариев P06; реальные подключения и продуктовые права не создаются проверкой инструментов. |

P00.04 принимается локально по ISCC, диагностике Tailscale и доступам GitHub с явно записанными ограничениями и отложенными средствами. Готовность выпуска и паритет самого приложения принимаются в последующих шагах.

## 5. Проверьте Windows теми же командами, что CI (P00.05)

Авторитетные версии Rust и Bun для проверки находятся в [scripts/environment-versions.json](../../scripts/environment-versions.json). Локальный скрипт и workflow читают этот файл; таблица выше описывает исходную базу установки. Перед обновлением версий меняйте файл и согласованные инструкции вместе, затем повторяйте локальную и удалённую проверки.

После установки инструментов выполните из корня репозитория в PowerShell 7:

```powershell
New-Item -ItemType Directory -Path docs/reports/p00-05 -Force | Out-Null
pwsh -NoProfile -File scripts/check-windows-environment.ps1 *> docs/reports/p00-05/local-check.log
if ($LASTEXITCODE -ne 0) { throw 'Проверка Windows не пройдена; см. local-check.log.' }
```

[Общий скрипт](../../scripts/check-windows-environment.ps1) проверяет Git, rustup, закреплённый Rust MSVC x64 с Cargo/rustfmt/Clippy, точную версию Bun и выполнение JavaScript. На отдельном примере без зависимостей выполняются Cargo build/run/fmt/Clippy; ожидается `Hello, world!`. Сборка проверяет доступность компилятора, линкера и SDK. Пример создаётся в собственном подкаталоге `temp/` и удаляется даже при ошибке. Ненулевой код внешней команды завершает проверку с ошибкой. Скрипт не устанавливает инструменты и не меняет глобальный Rust toolchain.

[Workflow Windows environment](../../.github/workflows/windows-environment.yml) запускается при push/PR с изменениями workflow, скрипта или файла версий. После появления workflow в основной ветке доступен ручной запуск **Actions → Windows environment → Run workflow** для выбранной ветки. Он устанавливает закреплённые Rust/Bun и вызывает тот же скрипт на стандартном `windows-2025`. MSVC/SDK, Git, rustup и PowerShell поставляет [обновляемый образ runner](https://github.com/actions/runner-images/blob/main/images/windows/Windows2025-Readme.md); точные версии образа и инструментов сохраняются в журнале запуска. Это проверка среды Windows Server, а не замена локальной Windows 11 и Chrome/Opera. Node.js обслуживает actions; потребность инструментов ZeDone в Node.js по-прежнему проверяется в P01.

Workflow имеет только `contents: read`, не использует пользовательские секреты и не публикует артефакты или кеши. Журнал доступен в Actions; срок хранения задаёт политика репозитория. Для публичного репозитория стандартный hosted runner [бесплатен](https://docs.github.com/en/billing/concepts/product-billing/github-actions); для приватного репозитория сначала проверьте квоту. Тайм-аут — 15 минут, устаревшие запуски одной ветки/PR отменяются. Отменить текущий запуск можно через **Cancel workflow**; отключить проверку — **Disable workflow**, вернуть конфигурацию — обратным коммитом. Одноразовый runner удаляется GitHub; локальные установки не затрагиваются.

Результат фиксируется в [EV-008](../agent/verification/EV-008-windows-environment-ci.md). P00.05 проверяет только Windows-среду; Linux с Android-эмулятором — P00.06, полное воспроизведение — P00.07, сборка приложения — P01.

## 6. Linux Actions и Android-эмулятор (P00.06)

Общий набор Android-версий, отдельный SDK, команды Linux-проверки, бесплатные ограничения Actions и отмена настройки описаны в [инструкции Android CI](android-environment-ci.md). Linux workflow проверяет ту же базу SDK и AOSP API 33, что подготовлена выше; выполнение на Windows этим не заменяется. Доказательства — [EV-009](../agent/verification/EV-009-android-environment-ci.md).

## Обновление, отмена и результаты

Перед изменениями сохраните в `docs/reports/` список версий, выбранный `rustup show active-toolchain` и изменяемые переменные окружения. Старые отдельные установки удаляйте штатно **после** проверки замены, с учётом других проектов. Для Rust используйте `rustup toolchain uninstall <точное-имя-старого-toolchain>`; сначала проверьте имя через `rustup toolchain list`.

Для отмены восстановите прежние версии официальными установщиками, прежний Rust — через `rustup toolchain install` и `rustup default`. Программы удаляются в **Settings → Apps → Installed apps**, компоненты MSVC/SDK — через Visual Studio Installer или их штатный деинсталлятор. Для Bun используйте тот же способ управления, что при установке; WinGet-установка удаляется через `winget uninstall --id Oven-sh.Bun --exact`. Возвращайте только изменённые вами значения PATH/переменных; сохраняйте профили браузеров, Git-настройки и учётные записи. Тестовый пример можно удалить отдельно из `temp/rust-smoke`.

Inno Setup удаляется через **Installed apps** или `unins000.exe` именно из его каталога установки; пользовательский установщик выше не меняет глобальный PATH и ассоциации `.iss`. Для добавленных через WinGet GitHub CLI/Tailscale используйте `winget uninstall --id GitHub.cli --exact` и `winget uninstall --id Tailscale.Tailscale --exact`. Существующие до проверки программы удалять не нужно. Если в отдельном сетевом шаге менялось подключение, `tailscale down` отключает его, а `tailscale logout` завершает вход; удаление зарегистрированного устройства из tailnet выполняется владельцем отдельно. Уже существующее подключение и GitHub-авторизацию сохраняйте. Синтетический ISCC-пример удаляется только из проверенного `temp/iscc-smoke/`; результат и журнал установки остаются в `docs/reports/`.

Для Android сначала остановите свой AVD и IDE. AVD удаляется через `avdmanager delete avd --name zedone_api33_aosp_x86_64`, выбранные SDK-пакеты — через `android --sdk=<каталог-SDK> sdk remove <точный-ID-со-слешами>`, CLI — через `cargo uninstall tauri-cli`, Rust-цели — через `rustup target remove --toolchain 1.99.1-x86_64-pc-windows-msvc <точная-цель>`. Удаление общего SDK-пакета затрагивает все использующие его проекты: сверьте зависимости. ZIP-установка Studio удаляется из её отдельного каталога после проверки абсолютного пути; сохраняйте пользовательские настройки IDE. Восстановите исходные Android-переменные из локального снимка. Если WHP был выключен до подготовки и больше не нужен другим приложениям, его можно выключить штатно через Windows Features или `Disable-WindowsOptionalFeature -Online -FeatureName HypervisorPlatform -NoRestart` из PowerShell администратора; требуемую перезагрузку выполняет разработчик.

Очистка `temp/` сохраняет установленные инструменты, состояние подготовки, доказательства проверок и снимки для отмены настройки. Нужные повторно загрузки и примеры воссоздаются командами выше. `docs/reports/` хранится отдельно от временных файлов и не удаляется при их очистке.

Агент продолжает работу по [очереди P00](../agent/work-queue.md#p00--окружение) и [плану-журналу](../agent/plan-journal.md). Локальные версии, пути, результаты и EV записываются только в `docs/reports/`, точка продолжения — `docs/reports/state.json`. При отсутствии состояния сначала проверьте фактическое окружение в P00.01 без установки. Этот Quick Start хранит общие требования и команды; готовность каждой машины и полная приёмка T13–T17 подтверждаются отдельно.
