# Проверка Android-окружения в Linux (P00.06)

[Workflow Android environment and prototype](../../.github/workflows/android-environment.yml) готовит отдельный SDK на стандартном `ubuntu-24.04` x64 и вызывает [общий скрипт](../../scripts/check-android-environment.sh). Человек и агент используют те же команды. Исходная проверка P00 проверяет инструменты и эмулятор. В P01.04 добавлены сборка двух ABI и [проверка APK/IPC](android-prototype.md) на том же принадлежащем проверке AVD до очистки. Физический Galaxy A32 здесь не проверяется; новые результаты Actions фиксируются отдельно от прежней приёмки среды.

## Версии и установка

Авторитетный набор версий — [environment-versions.json](../../scripts/environment-versions.json): Rust/Bun, JDK, Command-line Tools, Android CLI, NDK, SDK Platform/Build-Tools, Platform-Tools, Emulator и AOSP-образ API 33. Android-пакеты соответствуют базе [Windows Quick Start](quick-start-windows.md#3-подготовьте-android-p0003). В Linux используется Temurin той же версии Java, в Windows — JBR Android Studio; совпадение дистрибутивов не предполагается.

Для повторения на подготовленной Linux x64 машине нужны Bash, Git, jq, curl, unzip, iproute2 (`ss`), coreutils (`timeout`, `tail`), util-linux (`setsid`), shellcheck, rustup, Bun, JDK, `libpulse0` (динамическая зависимость эмулятора даже с `-no-audio`) и доступ к `/dev/kvm`. [Требования ускорения Android](https://developer.android.com/studio/run/emulator-acceleration) и [поддержка KVM hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) проверяются до старта. Без KVM скрипт завершается ошибкой.

1. Установите Rust, Bun и JDK указанных версий. Rust-цели: `aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android`. Команды установки находятся в workflow; глобальный Rust toolchain не переключается.
2. Повторите блок **Install pinned Android tools in an isolated SDK** из workflow, подставив каталог checkout вместо `GITHUB_WORKSPACE` и собственный пустой рабочий каталог вместо `RUNNER_TEMP`. Архив Google загружается по закреплённому имени и сверяется по SHA256 перед распаковкой. [Официальный каталог SDK](https://dl.google.com/android/repository/repository2-3.xml) задаёт исходный архив Tools 23; SHA256 вычислена после сверки архива с checksum каталога.
3. Через Android CLI выполните `sdk install <package>@<version>` для каждой строки `android.packages` из JSON. Tools 23 используют `/` в ID пакета; `avdmanager` использует `;`. При установке прочитайте [лицензию Android SDK](https://developer.android.com/studio#terms-and-conditions). В одноразовом CI ответы передаются из отдельного файла; эта настройка не меняет локальные лицензии разработчика. Google-аккаунт и пользовательские секреты не нужны.
4. Задайте `ANDROID_HOME` на этот SDK, а `ANDROID_SDK_ROOT`, если он установлен, на тот же путь. Обеспечьте права чтения/записи `/dev/kvm` штатными средствами своего дистрибутива. Для одноразовой VM workflow создаёт правило udev: владелец `/dev/kvm` — текущий пользователь, режим `0600`. Оно сохраняет доступ при пересоздании узла, тогда как разовая ACL после остановки эмулятора сбрасывалась. На общей Linux-машине используйте принятую администратором настройку группы `kvm`; правило владельца предназначено только для одноразовой CI VM. Повторять `sudo` на Windows не требуется.

## Одинаковая проверка для человека и агента

Из корня репозитория:

```bash
mkdir -p docs/reports/p00-06
set -o pipefail
bash scripts/check-android-environment.sh 2>&1 | tee docs/reports/p00-06/linux-check.log
```

Скрипт проверяет точные версии и файлы SDK, Java/Bun, Rust-цели, NDK и ускорение. Для четырёх ABI собирается изолированный Rust-пример без зависимостей. Новый AVD использует Android 13, API 33, `x86_64`, AOSP revision 2 без GMS; после загрузки (`sys.boot_completed=1`, максимум 300 секунд) бинарник запускается через adb и должен вернуть `ZEDONE_ANDROID_TOOLCHAIN_OK` с кодом 0. Отказ команды, неправильная версия, занятые порты 5554/5555, ранний выход или тайм-аут загрузки означают ошибку проверки. Итог успешного запуска: `Android environment check passed.`

Свой AVD, лог эмулятора и бинарники создаются в уникальном подкаталоге `temp/`; имя AVD также уникально и сверяется через adb перед проверками. При ошибке последние 80 строк лога попадают в журнал проверки. Эмулятор запускается в отдельной группе процессов через `setsid`: при завершении/INT/TERM скрипт отправляет этой группе TERM, ждёт не более 10 секунд и завершает оставшиеся процессы группы через KILL, затем удаляет свой каталог. Это учитывает наблюдавшееся игнорирование TERM со стороны QEMU. Существующие AVD и подключения adb сохраняются; общий сервер adb не останавливается. Принудительное завершение процесса или VM может прервать cleanup: оставшийся каталог удаляйте только после проверки пути и остановки своего эмулятора.

## Actions, ограничения и отмена

Workflow запускается при push/PR, затрагивающем его, скрипт, JSON версий или `prototypes/`. После интеграции в основную ветку доступен **Actions → Android environment and prototype → Run workflow**. Журнал запуска сохраняет версии инструментов, SDK и образ runner; результаты фиксируются в [EV-009](../agent/verification/EV-009-android-environment-ci.md). Состояние конкретной машины хранится только в `docs/reports/state.json`.

Стандартные hosted runners [бесплатны в публичном репозитории](https://docs.github.com/en/billing/concepts/product-billing/github-actions); приватный репозиторий использует квоту аккаунта. Для публичного `ubuntu-24.04` x64 [заявлены](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) 4 CPU, 16 ГБ RAM и 14 ГБ SSD; доступность и фактическое свободное место зависят от образа. Не используйте larger runners и платные device farms для этой проверки. Тайм-аут job — 40 минут; новый запуск той же ветки/PR отменяет предыдущий. Runner image и системный пакет `libpulse0` обновляются отдельно от закреплённых Rust/SDK. Загрузка зависит от доступности Google, Ubuntu, Rust, Temurin и Bun. Кеши и выгрузка артефактов не добавлены; логи хранятся согласно политике Actions. Node.js обслуживает actions; `vue-tsc` требует закреплённый Node LTS в общих проверках P01.03, но Android-сборка запускает Vite под Bun. Node не включается в APK.

Workflow использует только `contents: read`, actions закреплены полными SHA, Git credentials в checkout не сохраняются. Отмена — **Cancel workflow**, отключение — **Disable workflow**, удаление конфигурации — обратный коммит. Одноразовая VM удаляется GitHub. Локально сохраняйте прежние переменные и права KVM перед настройкой, затем восстановите только собственные изменения; отдельный SDK и добавленные toolchains удаляйте после проверки зависимостей других проектов. Подробности полной воспроизводимости окружения относятся к P00.07.
