# EV-076 — P04.10: автономные операции через общий API

Состояние: done в API-объёме P04/T07; PR #77/67c8e12.
Исходная ветка test/p04-10-offline-acceptance удалена после интеграции,
база bde8b29433e4124c13c0060c33c5b138e1e8dcd8; исходное дерево чистое.

- Цель: свести проверку автономных прикладных операций, их сохранности,
  актуальности и безопасных повторов через HTTP/IPC/настоящий CLI.
- Требования: T07/A01/A05/A07/A55/A56; ADR-0007/0008/0014/0030/0039.
- Зависимости: P04.07/EV-072 (3c6262a), P04.09/EV-075 (ce44703)
  и остальные P04.01–.08 интегрированы в исходную main.
- Охват: сквозные adapter tests, сводная матрица существующих проверок,
  инструкция воспроизведения, журнал и изолированные ABI-срезы Android CI.
  Продуктовый runtime/контракт/схема/зависимости не меняются.
- Размер: ориентир 300 строк превышен ради одной итоговой API-приёмки P04.10:
  общая fixture и три сценария используют прежний test target без нового fault
  API или копии продуктовой логики. Сопутствующая изоляция CI по ABI сохраняет
  полный набор и достоверность timing guards, добавляя один host.
- Проверки: bun scripts/check-product.mjs и та же команда с --android;
  каждый новый сценарий входит в штатный Rust test target commands и CI.
- Обратимость/отказы: возврат полей/положения/выполнения/тегов, trash/restore,
  причинный ABA, отказ устаревшим командам без частичных эффектов,
  недоступный worker, повтор исходного намерения после restart.
- Исключено: UI P05, продуктовый обмен/конфликты P06, полная T06/P03.06,
  физический arm64, выпуск и измерение бюджетов P09. Согласованные ручные
  очистки остаются исключениями из обратимости, не физическим стиранием.
- Результат: обе полные локальные проверки passed; по 273 Rust-теста Windows/
  нативного Android, 23 клиентских теста/239 assertions, обе Android release ABI,
  21 unsafe guard и совпадение контракта. T07 принят по итоговому CI/интеграции
  ниже; связанные UI/обменные A остаются partial.
- Результат интеграции: [PR #77](https://github.com/Zeyafix/zedone/pull/77), head d3f98eb,
  merge 67c8e12 после восьми SUCCESS. Далее — приоритетный P05.03; P03.06 тоже готов.

Сырые логи и машинные сведения хранятся только в игнорируемой docs/reports/.

## Матрица охвата

Все перечисленные тесты входят в штатный product-check; результаты текущей
ветки зафиксированы ниже для обеих платформ. Имена относительно
`crates/`; старые EV описывают интегрированные зависимости, а не заменяют
новый запуск тестов.

| Вопрос | Исполняемые проверки | Граница доказательства |
| --- | --- | --- |
| Создание/чтение/списки/правка/обратные операции пространства, колонки, карточки | adapters/tests/commands/{spaces,columns,cards}.rs; storage/src/{spaces,columns,cards}/tests.rs; EV-063/065/067 | Общий API, локальная база; интерфейс P05 отдельно |
| Выполнение, положение, теги и поиск | adapters/tests/commands/{ordering,tags,search,cards}.rs; storage/src/cards/tests/{completion,ordering,search}.rs; storage/src/tags/tests.rs; EV-068–070/072 | Исходные поля/история/ID сохраняются, обратные действия и права проверяются |
| Серии текста/история/восстановление пары | adapters/tests/commands/history.rs; storage/src/cards/tests/history.rs; EV-073 | 30 секунд, участник/сессия, дискретные команды, аварийное завершение и recovery; события вкладки P05 отдельно |
| Группы корзины/отдельное восстановление/ручные очистки | adapters/tests/commands/{columns,cleanup}.rs; storage/src/cards/tests/{trash_groups,cleanup}.rs; EV-074/075 | Прежняя корзина, актуальность состава/данных, preview и сохранность независимых/технических данных |
| Новое описание → все старые команды карточки и удаления контейнеров | adapters/tests/commands/offline.rs, new_description_invalidates_every_old_card_intention_and_both_container_deletions | Десять команд карточки и column.trash/space.trash отклоняются через HTTP/IPC/CLI без изменений карточки/контейнеров/истории; ABA после restart, чтение истории не меняет card.version |
| Взаимодействие операций, restart и старые receipts после обратных действий | adapters/tests/commands/offline.rs, mixed_offline_lifecycle_restarts_and_replays_without_undoing_later_actions | Три прохода с разным первым адаптером, первые выполнения тоже в CLI/IPC; поля/теги/выполнение/место, обе группы, восстановление, свежий поиск и точное сравнение снимков/истории после повторов |
| Недоступный worker, исходное намерение, поздняя правка и изменённый expected | adapters/tests/commands/offline.rs, unavailable_host_preserves_original_request_for_cli_reopen_and_concurrent_delivery | HTTP/IPC возвращают unavailable; настоящий CLI выполняет исходный запрос один раз; два worker возвращают receipt; поздняя правка сохраняется, подмена expected получает command_mismatch |
| Права/отзыв/срок/область до receipt; параллельная доставка и отмена ожидания | adapters/tests/commands.rs и доменные adapter tests; storage/src/replay_tests.rs, access/tests.rs, cards/tests/{completion,ordering,history}.rs | Локальная выдача доверенным host; протокол подключения и продуктовая сеть P06 отдельно |
| Ошибки записи/commit и остановка процесса | storage/src/{tests,replay_tests}.rs; cards/tests.rs и cards/tests/{completion,ordering,history,trash_groups,cleanup}.rs; tags/tests.rs и tags/tests/preservation.rs | SQL-отказ/заполнение базы, precommit checkpoints, потерянный результат, настоящий дочерний процесс без деструкторов; не физическое отключение питания |
| Сохранённое намерение клиента и предел повторов | tests/commands.test.ts; Rust-derived OpenAPI/типы/standalone validator | Исходные параметры/subject/ID, timeout/unknown, restart pending store, права/stale и некорректный ответ; полный UI P05 отдельно |

HTTP в новых доменных сценариях проходит настоящий Axum router in-process,
IPC — штатный invoke, CLI — отдельный нативный процесс с ключевым
файлом в синтетическом TempDir. Реальный TCP listener/framing/origin проверяют
прежние `real_http_socket_delivers_the_same_envelope_and_blocks_browser_origins`
и `only_the_host_configured_browser_origin_is_accepted_and_it_still_needs_credentials`.
Сценариям не нужны удалённый peer, интернет или сетевой сервис; `--offline`
относится к Cargo, не служит доказательством блокировки всей сети ОС.
Полный T07 не подменяет T06/P03.06, UI T08 или обмен T09.

## Результаты и передача

Обе команды завершились с exit 0: `bun scripts/check-product.mjs` и
`bun scripts/check-product.mjs --android`.

- По 273 Rust-теста Windows/нативного Android x86_64: 17 core, 7 contract,
  29 adapters, 220 storage. Все прежние проверки сохранены; добавлены три
  сквозных сценария. Смешанный жизненный цикл проходит три раза, начиная
  с каждого адаптера, затем повторяет сохранённые запросы через все три.
- Windows fmt/Clippy/release passed; release Android x86_64/aarch64 passed.
  Негативные проверки запрета unsafe: 7 Windows + 14 Android, все passed.
- 23 client tests / 239 assertions, TypeScript, Rust-derived OpenAPI/типы/
  standalone validator passed; нативный экспорт контракта Android совпал
  с Windows. Новых генераций, изменений контракта или зависимостей нет.
- SQL/commit/crash/recovery и прежняя доменная матрица выполнены заново
  в составе product-check. Нет обнаруженных регрессий, новых BUG/GAP/блокеров.
- Android открыт через Device; нативные бинарники и CLI запускались штатным
  runner в отдельном sandbox. Runner удалил sandbox; эмулятор выключен
  через Device в исходное состояние. Приложения/пользовательские данные
  не менялись, APK/UI в этом шаге не проверялись.
- Проверены полный diff, ссылки Markdown, баланс fences, сохранность всех
  98 определений критериев и git diff --check. Тестовый код после полной
  матрицы не менялся; SHA-256 offline.rs:
  a13557c58ca91fcc1c39429ab88c31b3ae47de01e13b1d2a8fc33ef38967d98b.

Пять правил в API-объёме соблюдены: обратные действия и согласованные
исключения ручной очистки проверены; человек/агент используют общий API;
матрица и инструкции агента/разработчика обновлены, user-документация
соответствует прежнему поведению; один набор нативных тестов Windows/Android;
авторитетная SQLite/Automerge/Rust-derived модель остаётся единственной.
Архитектура и продуктовый код не менялись, новый ADR не требуется.

## Исправление runner после CI

PR #77/head dfda95d: ранние попытки останавливали HTTP 504 при загрузке
Bun/JDK. После восстановления загрузок Windows/Linux и Android prototype-
native/prototype-ui/shell passed. Product-native дважды превысил 10 минут:
сначала при arm64 release, затем после обеих release-сборок внутри arm64
unsafe guards. В обоих случаях все 273 native tests passed; полной Android
приёмки это не даёт, пропущенные оставшиеся проверки не считаются успехом.

LOG-372: arm64 release запускается после подготовки x86_64 бинарников
одновременно с их нативным исполнением. Обе работы завершаются/собирают ошибки
до результата; sandbox очищается через finally. Команды, ABI, 14 Android
unsafe guards, client tests, проверка контракта и 10-минутный лимит сохранены.
Изменений runtime/схемы/зависимостей нет. Старые локальные результаты выше
относятся к исходному последовательному runner. Обновлённый runner проверен
отдельно (LOG-373):

- `bun scripts/check-product.mjs --android` — exit 0: все 273 native tests,
  23 client/239 assertions, обе release ABI, 14 unsafe guards, контракт/типы
  и native sandbox cleanup passed. Multiset всех Cargo-команд и аргументов
  совпал с прежним полным локальным прогоном; изменён только порядок независимых работ.
- Две негативные пробы используют локальные одноразовые копии скрипта, без
  нового fault API: у arm64 build подставлен неверный target; отдельно
  выброшена ошибка внутри try нативного sandbox. Оба запуска дали exit 1.
  При первом отказе все 273 native tests и cleanup всё равно завершились;
  при втором arm64 release и все её unsafe guards завершились. Клиентские
  тесты завершились в обоих случаях; ложного успешного итога нет.
- Исходный runner после положительного/негативных прогонов не менялся;
  SHA-256: 49cf70c2c0453aa4fd8758f60bebec629a4b55b94606407cb30d75dbbd0c44cc.
  Node syntax и diff checks passed. Device вернул эмулятор в исходное
  выключенное состояние. Новых BUG/GAP продуктовых данных нет.

Эта оркестрация отменена после проверки CI: на a344278 обе release ABI/14 guards
прошли, но поиск по 1000 карточкам под общей нагрузкой занял 40 секунд против
сохранённого guard 30 секунд. Storage: 219 passed/1 failed. Локальный успех не
подтвердил пригодность общей нагрузки в CI (BUG-008/LOG-374). Shell отдельно
отменён по лимиту 10 минут после основных UI-сценариев; его набор не сокращается.

## Изоляция ABI в CI

Полный `--android` возвращён к прежней последовательности. Новые
`--android-native`/`--android-arm64` выбирают соответственно x86_64 со всеми
native tests/контрактом либо arm64 release/guards. CI исполняет их на отдельных
hosts в product-native/product-arm64; прежние standalone/default/native
вызовы dispatcher сохраняют полный набор и порядок.

Все 273 native tests, guard поиска 30 секунд, обе release ABI/14 guards,
client/контракт/SDK/environment/negative-cleanup, строгий aggregate и лимиты
сохраняются. Добавляется один runner с отдельным setup/SDK/AVD; суммарное
машинное время растёт. Cache namespace/paths/один main writer прежние.
`check-android-ci-suites.mjs` в штатном host-check проверяет фактический
dispatcher со stub только границы запуска: все маршруты, прежние полные
команды, неизвестный/лишний аргумент, ошибку дочерней команды и signal.
Это не запускает Cargo/эмулятор и не добавляет публичные fault seams.

LOG-375, локальные результаты новой изоляции:

- Полный `bun scripts/check-product.mjs --android` — exit 0, 273 native tests,
  обе release ABI/14 guards, 23 client/239 assertions, контракт/типы и cleanup.
- Та же команда с `--android-native` — exit 0, все 273 native tests,
  x86_64 release/7 guards, контракт/типы, client tests и cleanup.
- Та же команда с `--android-arm64` — exit 0, arm64 release/7 guards,
  host-контракт/типы и client tests; native tests этот срез не объявляет выполненными.
- Multiset Cargo-команд полного режима совпал с прежним последовательным
  прогоном; объединение target-команд двух срезов точно совпало с полным.
  Исходники runtime/контракта/поискового guard не менялись.
- Тест маршрутизации passed: семь вызовов наборов, прежние полные команды,
  неизвестные/лишние аргументы, exit code и signal дочернего процесса.
  Неизвестный режим product runner отклонён до Cargo. Node syntax/YAML parse,
  пять обязательных suites/strict aggregate, неизменный лимит/один cache writer,
  все 98 определений критериев и diff checks passed.
- SHA-256 runner: c03e137a8c1ebb207ec745b655a1849e33b1e373aeca9fa9eaeeb1efe298941b.
  После проверок менялась только документация. Native sandbox очищены,
  эмулятор выключен через Device в исходное состояние.

BUG-008 resolved: новый head d3f98eb прошёл полный CI с неизменными guard/бюджетами
и интегрирован в 67c8e12. Доказательства ниже.

Логи и static/fingerprint evidence — только docs/reports/p04-10-*.
## Итоговый CI и интеграция

На точном d3f98eb все восемь checks SUCCESS (host 42479535139,
Android 42479535135). Raw logs подтверждают по 273 product Rust-теста Windows/
Linux/нативного Android, 23 client/239 assertions, обе release ABI/14 Android
guards, контракт/типы/validator и прежнюю native/UI/environment/negative-cleanup
матрицу. Новый тест маршрутизации выполнился в обоих host jobs. Node baseline
24.22.1 проверен Windows/Linux. Поиск прошёл прежний guard 30 секунд.

BUG-008 resolved; изоляция ABI сохранила все команды. Все пять Android jobs
уложились в прежние 10 минут. Это один запуск с восстановленным кэшем,
не гарантия cold-run/будущего времени; новый runner увеличивает общий расход.

GAP-013 сохраняет старый Windows lost-reply отказ a344278: backend status:error
без записанного кода и вторичный timeout. Три локальных UI-прогона и финальный
Windows CI прошли, но исходная причина/отсутствие будущих отказов не доказаны.
Наблюдать повтор с error.code; не блокирует принятый автономный API-охват.

Перед merge подтверждены тот же head, CLEAN/MERGEABLE, восемь SUCCESS и отсутствие
comments/reviews/review requests/inline comments. По LOG-371 PR #77 интегрирован
в 67c8e1298b4a17362ce4642303b6a0d302a30060. Ветка удалена локально/на GitHub,
main обновлена. T07 passed, P04/P04.10 done в автономном API-объёме.
Следующий приоритетный шаг — P05.03; P03.06 также разблокирован.

Полные UI/обменные A, T06/P03.06, физический arm64 и выпуск не принимаются.
После merge меняется только документация: результат записан локально, без
дополнительного коммита/PR/push и без повторных runtime-проверок. Сырые логи,
метаданные и локальные доказательства — только docs/reports/p04-10-*.
