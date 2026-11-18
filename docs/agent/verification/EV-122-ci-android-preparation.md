# EV-122 — CI.16: подготовка APK во время загрузки AVD

- База: PR #109/head de55014, все 27 SUCCESS в attempt 1. CI.15 checkpoints
  сохранены отдельно. Полный wall 10:01, Android prototype-ui 9:49.
- Цель: сократить критический путь без сокращения проверяемых гарантий.
  Целевой полный CI <=10m, ориентир <=5m; одна серия не доказывает устойчивость.
- Scope: Android environment driver, прежняя prototype build preparation,
  workflow/routing/process-lifecycle fixtures и docs. Продукт/API не меняются.
- Те же host release endpoint и x86_64 APK/instrumentation build выполняются
  по-прежнему последовательно друг за другом, но во время загрузки AVD.
  Их версии, targets/profiles, --locked/--offline и instrumentation сохраняются.
  Другие suites, 27 checks, строгие aggregates и budgets прежние.
- Новый CI-only `--prepare command -- runtime command` принимает argv без eval.
  Пустой command или runtime отклоняются до запуска процессов. Default путь
  без prepare сохраняется. SDK/preflight/четыре toolchain probes прежние.
- Подготовка начинается после compiler probes, до создания/boot AVD. Runtime
  выполняется только после всех прежних device checks и успешного wait build.
  Отказ build замечается и во время ожидания boot; исходный exit сохраняется.
- Build и emulator имеют собственные process groups. EXIT/INT/TERM cleanup
  завершает только эти groups, включая остающиеся в них descendants, ждёт
  и удаляет свой probe.
  Runtime failure и signal не превращаются в успех. AVD/app data не кэшируются.
- Fixtures используют синтетические SDK/compiler/adb/emulator, исполняя
  настоящий Bash driver. Проверяются оба порядка завершения prerequisites,
  build exit 23, boot failure, runtime exit 31, signal 143, descendants,
  cleanup каталога и malformed arguments. Реальный SDK verifier имеет прежние
  отдельные отказные fixtures; его проверки здесь не подменяются в CI.
- Один обязательный owner lifecycle fixtures — prototype-native, до SDK install.
  На текущей Windows нет Linux/setsid/WSL; локально эти POSIX fixtures не
  исполнялись. Их реальное выполнение и новая platform приёмка ожидаются в CI.
  Новая локальная Device UI проверка не применима к CI-only orchestration:
  APK и runtime сценарии не изменены, реальный Linux boot/build overlap — CI.
- Обратимость: revert preparation/workflow; старый последовательный default
  остаётся доступен. Нет пользовательских данных, миграций или новых dependencies.
- Workflow fingerprint меняет точный SDK cache key: первая серия ожидается
  с miss. Учитывать install/restore/save, размер/eviction, очереди, все attempts
  и runner work. Квота/платные настройки не меняются.
- Пять правил: обратимость, общий human/agent API, документация, прежние
  Windows/Android сценарии и единый источник pins/продуктовых данных сохранены.
- Состояние: routing/ShellCheck/actionlint с ShellCheck/Bash syntax/comment
  policy (390 sources)/diff passed локально. Реальные Linux fixtures и CI pending.
- Далее: PR #109/new CI, реальный lifecycle guard/runtime/cold и warm timings;
  цель времени не объявлять до результата. Merge/delete не поручены.
