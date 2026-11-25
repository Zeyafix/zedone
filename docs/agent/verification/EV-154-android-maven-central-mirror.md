# EV-154: Maven Central mirror для Android-сборок

Дата: 2026-11-25. Шаг FIX.04, verified локально. GAP-037 in_progress;
новый GitHub CI/интеграция ожидаются. LOG-821/822.

## Карточка

- База: main/0e42c68, прежняя ветка fix/ci-failure-diagnostics.
  Локальный FIX.03 и восемь dirty/untracked файлов сохранены до изменения.
- Цель: убрать прямую загрузку доступных в зеркале Central-артефактов
  из обычного Android dependency resolution, включая AndroidTest JUnit.
- Требование: T16 в части устойчивости подготовки Android CI;
  продуктовый контракт, версии и набор тестов сохраняются.
- Зависимости: LOG-818, повторный HTTP429 JUnit в post-merge
  run45621118870/job139229219197; SDK/Gradle cache restore успешны.
- Источники: [Sonatype: shared egress](https://central.sonatype.org/faq/429-shared-egress/),
  [Google: hosted Maven Central mirror](https://docs.cloud.google.com/docs/buildpacks/java#hosted_maven_central_mirror),
  [Gradle: repositories](https://docs.gradle.org/current/userguide/declaring_repositories_basics.html).
  Точный источник совокупного трафика исходного runner не известен.
- Решение: Google Cloud mirror перед mavenCentral в buildscript,
  project repositories и buildSrc прототипа и общего UI.
  google() остаётся источником Google Android artifacts;
  Central сохраняется для отсутствующих на зеркале компонентов.
- Охват: четыре Gradle build-файла, developer/agent docs.
  Прежний ключ Gradle cache учитывает изменяемые prototype Gradle-файлы;
  новый hash возникает штатно, вручную cache/fingerprint не правятся.
- Проверки: холодное разрешение JUnit через mirror, артефакты/контрольные
  суммы; штатные AndroidTest-сборки двух проектов без запуска UI,
  routing/cache guards, политика комментариев и полный diff.
- Обратимость: удалить приоритетный mirror обычным откатом конфигурации.
  Разрешение отсутствующей зависимости/сетевой отказ сохраняет failure,
  retries и пропуск тестов не добавляются; глобальный Gradle config не меняется.
- Вне scope: GAP-047/053, продукт/API/данные, обновление зависимостей,
  PR/commit/push/CI/merge, Device runtime и гарантия доступности внешнего сервиса.

## Результат

В четырёх Gradle-файлах добавлены шесть строк с приоритетным mirror.
Все шесть фактических repository blocks имеют одинаковый порядок:
google → Google Cloud Central mirror → прямой Central. Версии, lockfiles,
таймауты, CLI, матрица/aggregate и runtime-сценарии не менялись.

Проверено на Windows с закреплёнными JDK25.0.3/Gradle9.6.1:

- Холодный Kotlin DSL проект собран из фактического repository block
  с отдельным пустым Gradle User Home. `resolveProbe --configuration-cache
  --no-daemon --info`: SUCCESS14s. Лог подтверждает загрузку JUnit POM/JAR,
  Hamcrest POM/JAR и parent POM через зеркало, без обращения к прямому Central;
  configuration cache stored. Это холодное разрешение, а не полный cold APK.
- JUnit4.13.2 SHA-256 совпал с прежним локально используемым JAR:
  `8e495b634469d64fb8acfa3495a065cbacc8a0fff55ce1e31007be4c16dc57d3`.
  Hamcrest1.3:
  `66fdef91e9739348df7a096aa384a5685f4e875584cce89386a7a47251c4d8e9`.
- Повтор `resolveProbe --offline --configuration-cache --no-daemon --info`:
  SUCCESS3s, те же два JAR/хеши, configuration cache stored.
- `rejectMissing --offline`: ожидаемый exit1, Could not resolve отсутствующий
  fixture component, тело успешного task не выполнено. Ошибка разрешения
  не превращается в успех и не отменяет строгие проверки.
- Контролируемый HTTP429: исходный repository block из HEAD и новый block
  исполнены настоящим Gradle с отдельным холодным home. Только прямой Central
  внутри временного стенда направлен на собственный loopback server429;
  google/mirror настоящие. До изменения exit1/три запроса к rate-limited
  origin; после exit0/ноль запросов к нему и настоящий JUnit из зеркала.
  Это причинная проверка маршрутизации, не эмуляция исходного hosted IP.
  Loopback server остановлен finally; HTTP разрешён только в этом fixture,
  производственная конфигурация использует HTTPS без исключений TLS.
- В каждом настоящем Android-проекте выполнена штатная команда
  `gradlew.bat :app:assembleX86_64DebugAndroidTest -x :app:rustBuildX86_64Debug`:
  prototype SUCCESS1m54s/115 tasks, UI SUCCESS23s/115 tasks. Большинство
  compilation/package tasks UP-TO-DATE; buildSrc/root config и AndroidTest
  dependency resolution проверены, оба configuration cache stored.
  Исключение Rust task такое же, как в штатных build-android.mjs и
  build-ui-android.mjs; новая Rust/production APK сборка не заявляется.
- Штатный `bun prototypes/scripts/check-android-ci-suites.mjs` прошёл,
  включая routing, failure/signal, bootstrap, FIX.03 path-aware stderr,
  CI tool caches и CI Rust controls. `bun run check:comments`: passed451.
- `git diff --check`, ссылки и fingerprints пройдены. Все98 критериев
  и73 ID/определения проблем сохранены; меняются только доказательства и
  состояние GAP-037 open→in_progress. FIX.03/EV-153 сохранён побайтно.

Первичный вспомогательный probe успешно загрузил артефакты, но завершился
ошибкой serialization: его task захватывал Configuration вместо набора File.
Task исправлен, окончательная холодная проверка выполнена с новым пустым home;
первый отказ сохранён отдельно, не засчитан как passed. Отказ запуска
loopback harness до Gradle из-за неверного stdio тоже сохранён отдельно;
исправлен только временный harness. Эти отказы не относятся к продукту.

Логи/probe/harness/исходные снимки — ignored docs/reports/gap-037-maven.
Окружение/данные приложения/Keystore не менялись, Device не открывался.
Глобальные Gradle settings и чужие cache entries не правились/не удалялись.
Изолированные probe caches оставлены как диагностические артефакты в reports.

Обратимость обеспечивается откатом шести repository additions; общий API
человека/агента и авторитетный продуктовый граф прежние. Developer/agent docs
обновлены; пользовательских действий нет. Продуктовый Windows/Android parity
не меняется, новый runtime/Linux/hosted CI не заявляется.

GAP-037 не закрыт: следующий шаг — CI на опубликованном head при отдельном
поручении. Нельзя гарантировать доступность внешнего зеркала или отсутствие
429 при fallback для ещё не зеркалированного компонента. GAP-047/053 прежние;
commit/push/PR/merge не выполнялись, ветка сохранена.

## SHA-256 проверенной конфигурации

| Файл | SHA-256 |
| --- | --- |
| prototypes/src-tauri/gen/android/build.gradle.kts | 687782fdedf044f957279caab7c6a4ace1d068d80f0e773986c68d5b3cf64ca4 |
| prototypes/src-tauri/gen/android/buildSrc/build.gradle.kts | 88dc6e2dc5a82b4855fcfdbe7025c2d4abf4d4290934032974a5ecfed8d3a819 |
| ui/src-tauri/gen/android/build.gradle.kts | a09c409666399ae7f81856af3ce16717e691135bca8203ca2664ad9e97c53790 |
| ui/src-tauri/gen/android/buildSrc/build.gradle.kts | 88dc6e2dc5a82b4855fcfdbe7025c2d4abf4d4290934032974a5ecfed8d3a819 |

## Публикация — 2026-11-25

LOG-823: разработчик поручил продолжить и довести работу до конца.
Подготавливается публикация FIX.03/04 в прежней общей ветке, с отдельными
коммитами. Все source fingerprints прежние; новый CI/интеграция ещё
ожидаются, GAP-037 пока in_progress. При успешной свежей проверке
выполняется интеграция с сохранением ветки для остальных исправлений.
