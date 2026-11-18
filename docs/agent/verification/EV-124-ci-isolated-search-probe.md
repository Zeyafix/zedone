# EV-124 — CI.18: обязательный поиск по 1000 карточкам отдельно от других тестов

- База PR #109/head 2be1ec6. Linux product отказал на прежнем search probe:
  1000 корректных результатов, 31208ms при требовании <30s. В том же binary
  параллельно выполнялись остальные storage tests: 283 passed/1 failed.
  Full failed log/job metadata и snapshots сохранены; rerun не запрашивался.
- Цель: сохранять проверку объёма, результатов и времени поиска, исключив
  конкурирующие тесты из его процесса. Это не доказательство причины всех
  исторических превышений GAP-034; GAP остаётся open.
- Scope: атрибут одного Rust-теста, product test driver, routing/failure guards
  и docs. Product implementation/API, 1000-card graph/все assertions/debug
  profile/реальный Instant/<30s не меняются. Нет искусственного clock/time.
- Probe помечен ignored для обычного cargo test. Host full/host-tests после
  всех Rust tests и завершения client peer явно запускают его отдельным cargo
  test --workspace --all-features с ignored/exact/nocapture/test-threads=1:
  feature unification/profile прежние, остальные binaries отфильтровывают
  все тесты. Суммарно ровно один passed и ноль failed обязательны. Любой
  код/сигнал ошибки или другой count завершает pipeline отказом.
- Native Android после всех обычных native binaries отдельно исполняет тот
  же probe из zedone_storage test binary с теми же flags. Нет binary — failure,
  не пропуск. Remote directory очищается по прежнему finally и при отказе.
  ABI-only jobs probe не исполняют; runtime owner обязательный как раньше.
- Все остальные tests, native paired probes, release/unsafe/contract/UI и
  набор 24 checks/strict gates/budgets сохраняются. Обычный cargo test теперь
  не заменяет полный driver для этого probe, как уже для paired probes.
- Guards проверяют ровно один обязательный вызов на host/Android, точные
  flags, ignored binding, прежний 30s, пустой/нулевой/двойной count, missing
  binary, child exit/signal, full/slice coverage. Реальные Windows/native
  Android выполняются последовательно; Android Emulator открыт через Device,
  native tests запускаются штатными CLI без UI-автоматизации через adb.
- Обратимость: revert driver/ignore attribute; нет миграций и данных пользователя.
- Пять правил: обратимость, общий human/agent API, docs, обе платформы и один
  авторитетный workload/порог/pins сохранены. Новых dependencies нет.
- Состояние: verified локально на Windows/native Android; Linux/новый CI pending.
- Далее PR #109/24 outcomes/метрики. Общая цель <=10m,
  ориентир <=5m пока не принята. Merge/delete не поручены.

## Локальная проверка / LOG-673

Windows host-tests passed: 383 обычных Rust-теста + один isolated probe,
27 client/28 policy tests, fmt/Clippy/contracts/types/negative type checks.
Финальный probe использовал тот же workspace artifact без перекомпиляции
в отдельный feature set, query elapsed 11216ms; ровно один passed.

Native Android passed: 380 обычных Rust-тестов + один isolated probe
(query elapsed 11085ms), оба host↔device paired probes и точное сравнение
контрактов. TMPDIR/CLI binaries очищены прежним finally. Device выключен
через Device tools; UI не автоматизировался через adb.

Full/slice/mandatory-probe guards passed, включая empty/zero/duplicate count,
два отдельных passed summaries, failed summary, missing Android binary,
child failure/signal. Node syntax/comment policy 390 sources/diff passed.
30s/1000-card graph/debug features и все assertions прежние. Это локальные
результаты, не CI performance acceptance и не закрытие GAP-034.

Остальные результаты 2be1ec6 сохранены: 22 SUCCESS, Linux product и его
aggregate FAILED. Android shell прошёл, но занял 12:19 (runtime/preflight
7:01, build 3:11, SDK restore 0:32). После нового CI остаётся анализировать
этот bottleneck; успешный search fix не означает достижения <=10m.

## Публикация / LOG-674

cf03942 в PR #109, новый Linux/24 CI pending. Runs 44319061638/44319061698.
После результатов продолжить оставшийся Android shell >10m; локальные
probe timings не принимают общую цель времени. Merge/delete не поручены.

## CI / LOG-675

cf03942: все 24 checks SUCCESS в attempt 1, rerun не запрашивался.
Linux product отдельно выполнил тот же probe: cards=1000, matching=1000,
elapsed_ms=9916, ровно один passed; обычный batch явно показал ignored.
Host workflow 44319061638 завершился за 9:06, Android 44319061698 за 10:40
от создания workflow до terminal. Android shell 8:34; новый critical path
pairing-placements 10:31, pairing-fields 10:10. SDK restore hit, install skipped.
Сумма всех 24 job executions 150:51; cache inventory 17 entries/8.03 GiB,
как в предыдущем snapshot. Все durations/outcomes сохранены в summary JSON.
Raw run/jobs/steps и полные Linux/placements logs сохранены в ignored reports.

Это подтверждение обязательного probe на Linux, не доказательство причин
исторических GAP-034 и не принятие общего <=10m/идеального <=5m.
GAP-034/035/036/037 остаются open. Далее CI.19/EV-125: убрать отдельное
ожидание boot из shell/paired подготовки, сохранив все runtime проверки.
