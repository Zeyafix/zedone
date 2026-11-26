# EV-154: Maven Central mirror для Android-сборок

Дата: 2026-11-25. FIX.04 done в PR124/merge3eecdaf,
GAP-037 resolved после PR CI и post-merge Android. LOG-821–831.

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
  project repositories и buildSrc прототипа и общего UI; после отказа CI
  pluginManagement корня и отдельного buildSrc также получает mirror
  перед Gradle Plugin Portal.
  google() остаётся источником Google Android artifacts;
  Central сохраняется для отсутствующих на зеркале компонентов.
- Охват: четыре Gradle build-файла, четыре settings-файла, comment policy
  для новых settings, developer/agent docs.
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

## Результат первичной версии d238028

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

Опубликован [PR #124](https://github.com/Zeyafix/zedone/pull/124),
head d2380285400b2ca29c37c23a49c85a656d6a24ae, base main/0e42c68.
LOG-824:24 initial suites pending, ожидается27 с aggregates; host45655004210,
Android45655004336 queued. Новый CI успех пока не заявляется; после полного
успеха/свежей проверки предусмотрены интеграция и проверка post-merge Android.
Ветка сохраняется, GAP-047/053 не закрываются этим PR.

## Отказ CI и дополнительный plugin path — LOG-825

PR124/d238028: [prototype-ui](https://github.com/Zeyafix/zedone/actions/runs/45655004336/job/139349198337)
и [pairing-agent](https://github.com/Zeyafix/zedone/actions/runs/45655004336/job/139349198031)
упали при конфигурации :buildSrc/classpath. Kotlin2.3.21
(kotlin-gradle-plugin-api и BOM) запрашивался через plugins.gradle.org/m2,
который перенаправил в repo.maven.apache.org и получил HTTP429.
Это один непокрытый маршрут GAP-037, а не отказ runtime парного сценария;
до него выполнение не дошло. Первичные failed tails сохранены отдельно.

Repositories в buildSrc/build.gradle.kts не управляет разрешением
plugins/kotlin-dsl: для него нужен pluginManagement самостоятельной buildSrc
сборки. Тёплая локальная AndroidTest сборка и холодный JUnit probe были
недостаточны для подтверждения plugin bootstrap. [Gradle описывает отдельные
plugin repositories](https://docs.gradle.org/current/userguide/plugins_intermediate.html#sec:custom_plugin_repositories)
и [рекомендует Central mirror рядом с Portal](https://plugins.gradle.org/docs/mirroring).

В оба корневых settings.gradle и оба новых buildSrc/settings.gradle.kts
добавлен порядок google → Central mirror → Gradle Plugin Portal.
Плагины/версии прежние; Portal остаётся источником plugin markers и
Gradle-only artifacts. Два новых settings включены в source comment policy.
Первичный failure не скрывается и не заменяется rerun старого head.

## Проверка дополнения pluginManagement — LOG-826

- Настоящий Gradle9.6.1/JDK25.0.3 выполнил kotlin-dsl plugin bootstrap
  корня и вложенного buildSrc с отдельным пустым Gradle User Home.
  Оба проекта fixture используют те же plugin blocks и четыре проверенных
  settings-конфигурации; различие CRLF/LF нормализуется только в сравнении.
- Контролируемый Portal proxy отдаёт настоящие plugin markers/Gradle-only
  artifacts из HTTPS Portal, а остальные запросы, соответствующие обходу
  Central, получает HTTP429. Без plugin mirror настоящий bootstrap даёт
  exit1/12 отказавших Central requests; с новым pluginManagement exit0,
  ноль таких запросов и5 Portal-only requests. Холодный final SUCCESS1m9s,
  3 tasks executed, configuration cache stored; лог подтверждает Kotlin
  gradle-plugin-api2.3.21 и другие зависимости через настоящее зеркало.
  Proxy остановлен finally; HTTP допускается только для loopback fixture.
- Повтор того же plugin fixture без сети: help --offline, SUCCESS5s,
  3 tasks (1 executed/2 up-to-date), configuration cache stored.
- Оба настоящих AndroidTest проекта после новых settings прошли прежнюю
  команду assembleX86_64DebugAndroidTest с исключением штатного Rust task:
  prototype SUCCESS23s/115 up-to-date, UI SUCCESS29s/2 executed/113 up-to-date.
  Это проверка конфигурации/зависимостей с тёплыми выходами, не cold APK.
- Source comment policy453 passed; её28 tests/62 assertions passed.
  Штатный routing/cache/CI Rust guard включая FIX.03 passed; diff/сохранность
  критериев и исходного FIX.03 проверяются перед новым commit/push.
- Первые helper setup failures (CRLF сравнение, Groovy property для
  loopback HTTP и forwarding gzip headers) сохранены в ignored reports;
  не считаются причиной CI или passed. Окончательная положительная проверка
  выполнена с новым пустым plugin-home-after-final; исходные CI failures
  d238028 сохранены и не заменены rerun.

Плагины/версии, матрица/tests/таймауты/TLS и продуктовый API/данные прежние;
обратимость — откат settings, общий API/граф и Windows/Android поведение
не меняются. Developer/agent docs обновлены, Device runtime не запускался.
Далее новый commit/head в том же PR124 и полный CI; GAP-037 не закрывается
локальным успехом, GAP-047/053 остаются открытыми.

## SHA-256 дополнения pluginManagement

| Файл | SHA-256 |
| --- | --- |
| prototypes/src-tauri/gen/android/settings.gradle | 89fd357386f6d1de3e43670e80c18a54e19633bffac8b5ff34734658dca815fb |
| prototypes/src-tauri/gen/android/buildSrc/settings.gradle.kts | afc23f4064dd2b958d483bba53a2c49cca56d41d324660e14b0d876fb7d6c6e0 |
| ui/src-tauri/gen/android/settings.gradle | 89fd357386f6d1de3e43670e80c18a54e19633bffac8b5ff34734658dca815fb |
| ui/src-tauri/gen/android/buildSrc/settings.gradle.kts | afc23f4064dd2b958d483bba53a2c49cca56d41d324660e14b0d876fb7d6c6e0 |
| scripts/comment-policy.json | a5ba137d91f58366ae50850a39ded3593daada5e33e2ab1c5f73e9a705466055 |

## Новый опубликованный head — LOG-827

PR124 обновлён коммитом e677d6faee29407783f6e4e20b91f23e3fdd5121.
Body/head прочитаны обратно; первые10 checks pending,0 success/failure,
требуется полный набор27. Старые failures d238028 не заменены rerun.
GAP-037 остаётся in_progress; watch продолжает действовать, после успеха
предусмотрены свежая проверка/интеграция и post-merge Android CI.

## Linux gate на e677d6f — LOG-828

[Linux product job139358855590](https://github.com/Zeyafix/zedone/actions/runs/45657733755/job/139358855590)
отменён по явно подтверждённому GitHub лимиту10m0s. Обычные Rust tests и
isolated search1000/13331ms passed; отмена во время offline release-сборки,
остальные unsafe/contract проверки не завершены. Это повтор симптома
GAP-042, не доказательство нового отказа mirror; причина длительности
compilation неизвестна. Android prototype-ui на том же head SUCCESS.

Исходные job/run/annotations и ограниченный tail сохранены в ignored reports.
Штатный targeted ci_rerun не принят: workflow ещё running; accepted0/limit1.
После terminal host run45657733755 разрешён один recovery того же job/head
со строгим aggregate и прежними10m, без изменения tests/source. Полный CI
неуспешен, merge остаётся заблокированным; GAP-037 не закрывается.

LOG-829: Linux aggregate139367725839 failed с RESULT:cancelled — следствие
исходного timeout. После completed host workflow GitHub принял один targeted
Linux product recovery на том же e677d6f (rerun=true, accepted1/limit1).
Новых same-head повторов нет; timeout10m/tests/source сохраняются,
исходный отказ остаётся в доказательствах. Результат recovery и зависимого
aggregate ожидается, полный CI и закрытие GAP-037 пока не заявляются.

## Интеграция — LOG-830

PR124/e677d6f: все27 уникальных checks SUCCESS после одного принятого
Linux recovery; host45657733755 attempt2 и Android45657735199 attempt1
SUCCESS. Fresh head/base/feedback проверены, merge с expectedHeadSha выполнен
в3eecdafb1a37428bb14b350b0c58c483545bbe7d; Git tree merge и checked head
равны2427e0d2578c602d5786797b8229b8e99358a49f. Ветка сохранена.

Отдельный Android main push-run45662497833 ещё выполняется; окончательный
GAP-037 verdict ожидает его результат. Initial plugin429 и Linux10m не
скрываются. GAP-042/047/053 остаются открытыми.

## Итог — LOG-831

[Android push-run45662497833](https://github.com/Zeyafix/zedone/actions/runs/45662497833)
на merge3eecdafb1a37428bb14b350b0c58c483545bbe7d завершился SUCCESS,
attempt1. Все16 jobs (15 suites плюс aggregate) completed/success;
точный head и полнота уникальной матрицы проверены свежими metadata и
отдельным verifier. Оба прежних HTTP429 targets, полный Android UI/парный
набор и обе ABI прошли с первой попытки. CLI watch завершился HTTP502
при чтении GitHub API; это отказ наблюдения, итог подтверждён прямым
structured read, не взят из exit watch.

GAP-037 resolved в охвате regular/plugin resolution обоих Android-проектов;
FIX.04 done. Все27 PR checks и независимый Android после merge passed,
деревья merge и checked head равны. Никаких дополнительных повторов
Android main не было. Linux recovery в PR остаётся одним принятым повтором,
GAP-042/047/053 open; исходные failures не переписываются. Доступность
внешних зеркал и неизвестный исходный aggregate egress не превращаются
в гарантию отсутствия любого будущего429 при fallback.

Main=origin/main/3eecdaf, feature-ветка сохранена локально/remote/e677d6f.
Продукт/API/данные/обратимость/единый граф прежние, Windows/Linux/Android
PR CI и отдельный Android main подтверждены, developer/agent docs актуальны.
Независимый main host45662497988 ещё выполнялся на снимке: полного нового
host успеха здесь не заявляется. Финальный checkpoint локальный, без нового
commit/push/CI; активных реализаций нет, дальнейшие исправления — по поручению.

LOG-832: позднее завершился и отдельный main host45662497988 — SUCCESS,
attempt1. Все11 host jobs/точный merge SHA проверены structured snapshot
и verifier; вместе с16 Android jobs это полный post-merge набор27 SUCCESS,
оба workflow с первой попытки. Прежняя pending-запись выше историческая.
Окончательный вердикт сохранён в ignored main-verdict.json; GAP-037 resolved,
FIX.03/04 done, прочие причины не объявляются исправленными.
