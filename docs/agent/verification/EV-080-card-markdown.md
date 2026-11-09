# EV-080 — CommonMark в карточке (P05.04)

## Рабочая карточка

```yaml
step: P05.04
base_revision: "8e87273; шесть исходных dirty docs сохранены отдельно, без потери правок"
goal: "Сохранённое описание карточки безопасно отображается как CommonMark на Windows/Android"
requirements: [T08, A50, ADR-0006]
dependencies_evidence: ["P05.03: PR #78/2f54349, EV-078", "P01.07: PR #14/21e329d, EV-020"]
scope: ["общий UI Markdown viewer из прототипа", "CardEditor", "CSP/Android Opener", "закреплённые зависимости", "проверки и документация P05.04"]
checks: ["bun run ui:check", "прежний прототип Markdown", "debug APK x86_64/aarch64", "Android Device: настоящий IPC/ссылки/изображения/ошибка/повтор"]
reverse_and_failure: "Обычная обратная правка Markdown, скрытие изображения, повтор загрузки; ошибка не меняет данные/редактор"
result: "verified локально; публикация feat/p05-04-card-markdown поручена, новый CI и интеграция ожидаются"
evidence: ["Windows UI/HTTP", "Android Device/IPC", "обе debug ABI", "сохранность sandbox", "прототип/типы/CI guards"]
next: "Создать PR, проверить весь новый CI; при успехе merge и удаление ветки. Затем P05.05"
```

## Граница

Одна проверяемая возможность — производный просмотр сохранённого описания.
Заголовок и редактор остаются обычным текстом; конфликтующие описания
показываются раздельно. Rust-команды/модель/схема не меняются. Просмотр не пишет
историю и не меняет выполнение. Таблицы/интерактивные чекбоксы не добавляются.
Внешние ссылки и загрузка выбранного изображения используют проверенную
конфигурацию ADR-0006. API продолжает возвращать точный исходный Markdown.

Перенос существующего viewer вместо независимой продуктовой копии сохраняет
один профиль для прототипа и приложения. Ориентир 300 строк превышен
из-за переноса и связной проверки сети/сохранности/двух платформ; независимые
P05.05–.10 сюда не входят. Локальные снимки, логи, исходный dirty diff и сведения
машины находятся только в игнорируемой `docs/reports/p05-04/`.

## Проверки и результат

Реализован весь согласованный P05.04. Общий viewer, профиль и управление
изображениями перенесены без продуктовой копии; Rust-команды/DDL/редактор
не менялись. Добавлены уже выбранные в ADR-0006 JS/Rust-зависимости и узкая
Opener capability. В CI prototype-ui теперь устанавливает корневые locked
зависимости общего viewer; порядок и прежний охват suites сохраняются.

| Проверка | Результат и граница |
| --- | --- |
| Полный bun run ui:check | Passed: типы, Vite, Rust fmt/Clippy/3 shell integration tests/host/example; весь прежний workspace/cards/editor/delivery набор и новый Markdown. Финальный прогон Chrome и Opera passed. Локальный Node 26.8.1 явно отмечен override; базовые проверки типов обоих UI дополнительно passed под Node 24.22.1 |
| Настоящий HTTP/Service | Базовый синтаксис/Unicode, буквальный HTML/XSS, отсутствие таблиц/чекбоксов, исходник и история без изменений от просмотра/ошибки, внешняя вкладка без opener, выбранная загрузка/скрытие/ошибка/повтор, независимое выполнение и снятие, reload/сброс согласия, UI-правка и обратная агентская правка passed |
| Финальный общий сценарий | Отдельно passed desktop/light и mobile/dark 360 px с native touch adapter; чтение/прокрутка без горизонтального переполнения диалога. Это host proof CI-драйвера, не Android-приёмка |
| Прежний прототип | Финальные vue-tsc/Vite и полный check:http passed после переноса; прежние Markdown, QR, invitation, SQLite/board/fault проверки сохранены |
| Debug APK x86_64/aarch64 | Обе финальные сборки passed; manifest identity/категории запуска и разрешение явных HTTP-изображений проверены. Выпуск/физический arm64 не проверялись |
| Android Device, настоящий IPC | Форматирование/буквальный HTML/неинтерактивный маркер, отдельный внешний браузер с точным URL и сохранение карточки при возврате; нет запросов до нажатия, одна выбранная HTTP-картинка/скрытие, ошибка сервера/повтор; выполнение/снятие; редактирование при ошибке, сохранение/обратная правка/restart, обе темы passed |
| Сохранённые Android-данные | После Device-проверки копия синтетического хранилища прочитана общим Service без изменяющих команд: точный исходник, три парные версии (создание/правка/обратная правка), completed=false. Это проверка сохранённых данных, не замена Device-управления |
| Сеть | Локальный HTTP-коллектор Android зарегистрировал только one, two-failure, two-retry, two-failure после явных нажатий. Нет raw.png, referer или Authorization. Windows использует перехваченные PNG и сетевой отказ; TLS trust не изменялся |
| Очистка | Исходные product/WebView восстановлены: одинаковые имена и SHA-256 всех 23 файлов. Синтетические базы/ключи и временные архивы удалены; reverse/сервер убраны, системная тема возвращена, эмулятор выключен Device |
| CI-подготовка | Host/Android routing guards и actionlint passed, frozen install воспроизводим. Новый Android CI-сценарий включает native-отказ другим URI и ошибку Opener bridge; его настоящий CI пока не выполнялся |

Сырые логи и доказательства находятся только в игнорируемой
`docs/reports/p05-04/`: ui-final.log, markdown-final-proof.log,
prototype-final-http.log, android-x86-final.log, android-arm64-final.log,
device-persistence-proof.json, sandbox-restored.json и снимки двух тем.

Первый offline lock resolution отказал на отозванной транзитивной версии;
обычный Cargo fetch разрешил только добавляемые зависимости, прежние locked
пакеты не обновлены. Начальная команда типов использовала неверный путь;
исправленная и базовая Node-проверки прошли. Device потребовал кавычки PowerShell
для ref и свежий полный snapshot; ввод многострочного Markdown выполнен через
text-stdin. Эти промежуточные отказы не объявляются продуктовой регрессией.

## Приёмка и передача

P05.04 verified локально, без коммита/PR/CI/merge; в done не переводится.
T08/A50 сохраняют partial до интеграции и оставшегося согласованного охвата.
Продуктовый обмен/конфликты P06, полный UI P05.10 и выпуск этим шагом не приняты.
Новых BUG/GAP нет; прежние GAP-007/012/013/014 остаются открытыми. Прошлый
cached замер CI.04 не является измерением нового head с P05.04.

Пять правил соблюдены в проверенном объёме: обратная правка/скрытие/снятие
выполнения; общий API человека и агента, одинаковые UI-кнопки ресурсов;
обновлены user/agent/development docs и существующий ADR; Windows/Android
проверены, arm64 только собран; исходный Markdown в общем хранилище остаётся
единственным источником. Рендер/согласие на загрузку — временное представление.

Следующее действие — ревью и интеграция этой ветки. После P05.04 done готов
P05.05; независимо также готов P03.06. Новая реализация не начата.

## Публикация — LOG-399

Разработчик поручил создать PR; при успехе слить и удалить ветку.
Предыдущий раздел фиксирует результат локальной проверки до этого поручения.
Новый CI ещё ожидается; локальные успехи не подменяют проверки PR head.
В PR входят реализация и документация P05.04. Существующие незакоммиченные
записи предыдущих интеграций в журнале/очереди/EV-077–079/CI docs сохранены
в рабочем дереве и не включаются в этот commit.

## PR — LOG-400

[PR #80](https://github.com/Zeyafix/zedone/pull/80), head
86c0e7fa79dde3bd3c845c43867178764f377daa, создан через gh CLI с четырьмя
вложениями через --attach. Body/HTML/head прочитаны обратно: две картинки, два
видео, корректные разделы и generation credit, локальных ссылок не осталось.
Реальные base/final материалы вне Git просмотрены; видео 1920×1080 подтверждены
ffprobe. Новый Windows/Linux/Android CI выполняется; merge/delete ещё не выполнены.

## Первый Android CI — LOG-401

Head 86c0e7f: Android shell job 129013382120 остановлен HTTP 504 в setup-java
при скачивании закреплённого JDK до сборки и тестов. Полный workflow
42572674754 ещё выполняется; GitHub отклонил повтор job HTTP 403 до его
завершения. Код не менялся, исходный отказ сохраняется. Следующее действие —
после завершения повторить failed shell/aggregate на том же head, затем
проверить все checks. CI/merge не объявляются успешными.

## Исправление области проверки типов — LOG-402

Head 86c0e7f: Linux и оба product Android jobs отказали в обычном
tsc tsconfig.contract.json: общий viewer в client/**/*.ts захватил Vue/Tauri
за пределами API-контракта (TS2307 .vue, TS2550 asyncDispose). Это отдельная
ошибка размещения модуля; JDK HTTP 504 из LOG-401 остаётся инфраструктурным
отказом. Локальный product-check не был выполнен в первом срезе — пробел
первоначальной проверки признаётся, успешный ui:check его не заменял.

Три файла без изменений содержимого перенесены в ui/src/markdown. Продукт
и прототип импортируют один viewer, client/ остаётся API-клиентом.
Конфигурация строгого контрактного tsc, assertions и лимиты не изменены;
исключения файлов/skipLibCheck/фиктивные объявления .vue не добавлялись.
Повторные сборки/типы и проверки фиксируются ниже перед публикацией.

### Проверки переноса — LOG-403

Полный product-check passed: 273 Rust tests, 23 client/239 assertions,
release всех targets/features, 7 unsafe guards, OpenAPI/types/обычный tsc.
Полный ui:check passed в Chrome/Opera на Node baseline 24.22.1; три shell
integration tests и прежние editor/fault/Markdown сценарии сохранены.
Прототип vue-tsc/Vite/полный check:http и обе debug ABI passed.
Проверки docs/98 критериев/CI routing guards/diff passed.

Три перенесённых файла совпали с 86c0e7f; API/Rust/зависимости/конфигурации
проверки типов не менялись. Новый After screenshot побайтово совпал с
размещённым материалом PR, новые вложения не нужны. Device-проверки выше
относятся к первоначальному срезу; повторное локальное управление Device
для перемещения неизменённых файлов не выполнялось. Новые APK собраны,
полный Android CI нового head ещё ожидается.

## Новый head и условие остановки — LOG-404/405

Исправление опубликовано в PR #80 как 0d7b77e; полный новый CI ожидается.
По прямому указанию разработчика при повторном HTTP 504 в CI нужно остановить
работу: без повторов и merge, выключить наблюдение и сообщить. Если такого
отказа нет, исходное поручение merge/delete после успешных проверок сохраняется.

## Поздний Windows отказ 86c0e7f — LOG-406

Windows shell 129013381916: после trash card.list вернул unavailable.
Opt-in trace подтвердил clock rejection: raw wall на 1 мс ниже observed,
после чего UI-тест не нашёл область/кнопку корзины за прежние 10 секунд.
Это наблюдение GAP-014, не HTTP 504 и не ошибка типов. Причина расхождения
calendar/observed по одному trace не доказана; перенос 0d7b77e её не меняет.
Исходный отказ сохранён; clock policy/лимиты/assertions/retries не изменены.
Текущий head проходит новый CI. Условие остановки LOG-405 сохраняется.

## Остальные исходные отказы — LOG-407

Windows product 129013381904 на 86c0e7f подтвердил ту же ошибку области
контрактного tsc (TS2307/TS2550). Android/Windows aggregates
129016567063/129016199672 корректно отклонили исходные матрицы; собственных
новых ошибок и повторного HTTP 504 в этих трёх job нет. Старые jobs
не повторяются: полный новый CI уже проверяет исправление 0d7b77e.

## Текущие shell отказы — LOG-408

Windows shell 129018121216 на 0d7b77e повторил clock rejection на 1 мс:
space.list unavailable и timeout карточки. Это GAP-014, который блокирует CI
и merge; не HTTP 504. Причина расхождения по двум одиночным trace не доказана.
Для уточнения добавлена ограниченная история восьми raw wall samples
того же worker, только в прежнем opt-in debug trace. Ветка отказа/Instant/
observed/авторизация/схема и release поведение прежние.

Android shell 129018120031 прошёл старый IPC/editor/Back набор, но обычный
Playwright waitFor нового Markdown использовал отсутствующий в WebView 101
element.checkVisibility. Новый сценарий принимает штатный nativeControls.visible
для трёх ожиданий; проверки стилей/размера/native touch/все assertions и лимиты
сохранены, DOM API не подменяется. Host proof обоих адаптеров passed.
Десять прежних clock tests с debug trace ON/OFF и полный product-check passed;
полный UI/ABI перед публикацией фиксируются ниже. Никакого повторного 504
в этих двух shell логах нет; условие остановки LOG-405 сохраняется.

### Локальная проверка уточнённой диагностики и native waits — LOG-409

Десять исходных clock tests прошли с trace ON/OFF; полный product-check
passed (273 Rust, 23 client/239 assertions, release/7 unsafe/контракт).
Полный host ui:check на Node baseline 24.22.1 passed. Новый общий сценарий
отдельно passed с desktop и native touch/visibility adapters; обе debug ABI
собраны. Отсутствие строки recent_wall_ms в release CLI и её наличие в
debug fixture проверены. Публичные DTO/схема/политика времени/assertions
и лимиты не менялись. Native Android CI новых ожиданий ещё ожидается;
clock rejection не объявляется исправленным. Device заново не запускался
для изменения тестового ожидания/opt-in debug trace.

## Head e6306c3 — LOG-410

Опубликован e6306c3842ba7827f58640b20473ea2abacaf309. Новый полный CI ожидается.
Исправлен native Android wait, расширена только opt-in debug диагностика
часов; время/доступ не ослаблены, причина GAP-014 пока не доказана.
Условие остановки при повторном HTTP 504 сохраняется.

## Первый e6306c3 CI и единичный повтор — LOG-412

Десять checks SUCCESS, включая Windows shell. Android shell 129024561717
отменён общим лимитом job 10m0s, подтверждённым check annotation; aggregate
129027841660 корректно failed. При этом обе ABI, полный настоящий Android
Markdown/IPC/editor/Back/отказы/cleanup и все записанные steps success.
Это не whole-job успех: исходный cancelled результат сохраняется.
HTTP 504 не повторился. На том же head запрошен ровно один повтор shell
и зависимого aggregate, без изменения таймаутов/assertions/охвата/политики
кэша. Итог повторного job ещё ожидается; GAP-014 не объявляется исправленным.

## Повтор timeout и разделение ABI — LOG-413 / GAP-015

Единичный повтор shell 129029057738 на e6306c3 снова cancelled по общему
budget 10m0s; теперь отмена пришла внутри Markdown, до negative cleanup step.
SDK setup 2:23/cache 0:51/последовательные ABI+host 4:05 оставили недостаточный
runtime budget. Новый HTTP 504 не зарегистрирован. Оба исходных результата
сохранены; дальнейших слепых повторов нет. Aggregate 129032523058 failed.

Shell x86_64/host fixture/полный IPC/UI набор и arm64 APK build разделены на
независимые обязательные jobs. Общий Android aggregate требует всех шести
suites; каждый job сохраняет 10m. Runtime/KVM запускается в прежних пяти
suites, build-only shell-arm64 не запускает эмулятор; negative compiler/
version/cleanup guards сохранены во всех. Source/build scripts/версии/cache
writers/permissions/assertions не менялись. Один дополнительный runner
повторяет подготовку; суммарная стоимость не измерялась, новый CI ожидается.

Штатные routing guards проверили точное владение обеими ABI, lockfile/JS
подготовку, runtime boundary, обязательность aggregate и прежние failures/
signals/default/native modes. Тела настоящих Bash build/dependency steps
проверены на CLI-фикстурах для обеих новых suites; actionlint/host guard/diff
passed. Локальные product/UI/обе ABI из LOG-409 сохраняют охват неизменённых
исходников; нужны все 13 checks нового head. GAP-014 не объявляется исправленным.

## Head 75929c8 — LOG-414

Опубликовано разделение 75929c82f5ad13afa829495a93fb5ac045031183, ожидаются все 13 checks.
Дополнение к двум отменам: во втором логе есть и 10-секундный waitForFunction
timeout на проверке форматирования, уже рядом с общим дедлайном job. Связь
этих двух событий не доказана; нельзя считать отмену единственной причиной.
Новый CI обязан пройти тот же неизменённый сценарий. Условие stop-on-504
сохраняется; исходники приложения/clock policy/assertions/таймауты прежние.

## Остановка по повторному HTTP 504 — LOG-415

Текущий head `75929c82f5ad13afa829495a93fb5ac045031183`: [Android shell
job 129036182619](https://github.com/Zeyafix/zedone/actions/runs/42579990050/job/129036182619)
failed в `actions/setup-java` при скачивании Temurin JDK 25.0.3+9.0.LTS.
Полный лог содержит HTTP 504 в 07:32:53/07:33:13 UTC, окончательный отказ
в 07:33:26. Повторы принадлежат уже исполнившемуся action; агент новых
повторов не запускал. Сборки и проверки приложения в этом job не начинались,
поэтому результат разделения ABI и полный CI нового head не подтверждены.

Согласно явному поручению разработчика LOG-405 работа остановлена.
`unwatch_pull_request` подтвердил `wasWatching=true`, `watching=false`.
PR #80 открыт, ветка сохранена; retry/правки исходников/push/merge/удаление
ветки не выполнялись. Остальные checks не опрашивались после обнаружения 504.
GAP-014/015 и прежние ограничения приёмки сохранены; P05.04 не done.
Продолжение только по новому явному поручению разработчика. Полный raw log
сохранён в игнорируемых локальных reports, пользовательские документы сохранены.

## Возобновление и подготовка arm64 preflight — LOG-416

Новое поручение разработчика: создать PR, при успехе merge и удалить ветку.
Продолжается существующий PR #80, без дубликата. На head 75929c8 полный
CI завершился failure: кроме прежнего JDK HTTP 504, Windows shell повторил
GAP-014 (wall=1794209940636, observed=1794209940637; последние raw samples
не показывают обратного движения), затем assertion error/ok и UI timeout.
Это не доказывает первопричину; политика часов и проверки не меняются.

Новый [shell-arm64 job](https://github.com/Zeyafix/zedone/actions/runs/42579990050/job/129036182316)
успешно собрал APK, но negative compiler preflight завершился до rustc:
`libpulse.so.0` отсутствует. Общий скрипт также требует доступ к KVM до rustc.
Подготовка libpulse0/KVM теперь выполняется во всех шести jobs; arm64
по-прежнему не запускает эмулятор, host fixture или положительный UI-набор.
Negative checks/exit codes/cleanup, обе ABI, aggregate и 10m budget сохранены.

Локально passed: Android/host routing guards, actionlint обоих workflows,
Bun syntax build и git diff --check. Изменены только workflow, его штатный guard
и документация. Локальные продуктовые/Device результаты выше сохраняют
прежний охват, заново не запускались для подготовки CI runner.
Новый полный CI ожидается; P05.04/GAP-014/015 не закрываются до результата.
При повторном HTTP 504 работа снова останавливается без retry/merge.

## Возобновление после ручной остановки и готовность WebView — LOG-418

После уведомления об отказах 394f957 разработчик остановил работу;
наблюдение выключено. Новое «Продолжи» возобновляет задачу. Оба прежних
workflow завершились failure. Все шесть непосредственных отказов исследованы:

- Android prototype-native 129047549999: setup-java, JDK download HTTP 504.
- Android shell-arm64 129047549885: setup-bun, Bun download HTTP 504.
- Windows product 129047549836, prototype-ui 129047549780 и shell
  129047549858: setup-bun, Bun download HTTP 504. Для последнего gh run view
  отказал из-за повреждённого локального ZIP-кэша; raw job logs получены
  через GitHub API без перезапуска CI. Это ошибка получения evidence,
  не дополнительный отказ job.
- Android shell 129047549727: сборка x86_64/host fixture и начальный
  IPC/menu/Back/themes/reload passed, после подготовки fixture и restart
  `browser.contexts()[0]?.pages()[0]` вернул undefined. Socket и CDP уже
  доступны, но page ещё не появилась. Ранее setup-java также записал HTTP 504
  при загрузке подписи; action продолжился, агент его настройки не менял.

Пять остальных checks SUCCESS: Linux, Windows prototype-native, Android
product-native/product-arm64/prototype-ui. Оба aggregate корректно failed.
Полный CI и arm64 negative preflight не приняты. Отказы относятся к запуску
до нового поручения продолжить; они сохранены, слепой повтор не выполнялся.
При новом HTTP 504 в следующем CI снова остановиться без retry/merge.

Shell openWebView теперь использует прежний проверенный helper discovery из
прототипа и ожидает незакрытую Playwright page. Socket/target/page и CDP
делят прежние 15 секунд готовности; нет новых запусков приложения/повторов
сценария, job budget 10m и assertions сохранены. При отказе после forward
собираются targets/pages/lifecycle logs с сохранением исходной ошибки.
Исходная первопричина периодических WebView отказов GAP-007 не объявляется
устранённой по одной гонке; GAP-014/015 также остаются открытыми.

Локально passed: существующий WebView target guard; исполнение настоящего
openWebView с синтетическими process/CDP/time boundaries для немедленной
и задержанной готовности, закрытых страниц, отсутствия каждой стадии,
единого дедлайна, одного запуска и сохранения исходной ошибки при отказе
диагностики. Временный проверочный скрипт — игнорируемый
docs/reports/p05-04/verify-shell-webview-connection.mjs, без запуска устройства.
Также passed Android/host routing guards, actionlint обоих workflows,
JS syntax, docs links/fences/98 критериев/сохранность dirty docs и diff checks.
Product source/API/clock policy прежние; Device локально заново не запускался.
Native подтверждение ожидания page остаётся задачей нового полного CI.

## Android Markdown после UI-правки — LOG-420 / GAP-016

Head e469ff3: все одиннадцать остальных checks SUCCESS, включая Windows
shell/aggregate и Android shell-arm64 с negative preflight. [Shell Android
129102210052](https://github.com/Zeyafix/zedone/actions/runs/42600120819/job/129102210052)
failed на check-ui-markdown.mjs:107: прежнее ожидание strong «После ошибки»
истекло через 10000ms после UI-правки, закрытия и повторного открытия карточки.
Это самостоятельный timeout, не отмена общим 10m budget: весь job завершился
через 8:04. Environment, обе подготовки shell, начальный IPC/menu/Back/themes,
restart и полный card/editor/history/completion/draft набор до Markdown passed.
Нового HTTP 504 в полном shell логе нет.

[Android aggregate 129105751709](https://github.com/Zeyafix/zedone/actions/runs/42600120819/job/129105751709)
корректно failed с RESULT=failure; собственной иной ошибки нет.
Старое ожидание page после CDP на этом запуске прошло, но GAP-007 не закрывается
по одному успеху. GAP-015 остаётся открытым до успешного полного shell job;
нехватка времени не объявляется причиной нового отказа. GAP-016 фиксирует
неподтверждённую native UI-правку Markdown и блокирует интеграцию.

Исходный лог не содержит DOM/card snapshot на момент отказа. Теперь после
статуса «Сохранено», до закрытия, общий сценарий дополнительно требует точный
edited исходник через card.read. При любом отказе перед штатной очисткой
записываются synthetic DOM state (наличие диалога, заголовки/status/strong,
textarea/focus/visibility) и сохранённые descriptions через тот же API.
Диагностика не повторяет действия, не ослабляет assertions/10000ms/job budget
и не заменяет исходную ошибку. Исправление продукта пока не заявляется.

Локально полный общий Markdown-сценарий с новой проверкой passed на desktop/light
и mobile/dark с настоящим native touch/visibility adapter в host Chromium.
Это не Android Device acceptance. Product source/API/clock policy прежние;
Device для диагностической правки заново не запускался. Новый CI должен
подтвердить сценарий либо дать evidence для точечного исправления GAP-016.

## Текущие отказы 131f068 — LOG-422

Windows shell [129107549515](https://github.com/Zeyafix/zedone/actions/runs/42601715418/job/129107549515)
failed ещё в прежнем checkCards: несколько space.read/card.list получили
unavailable. Debug trace подтвердил GAP-014: raw wall на 1ms ниже observed,
при этом последние восемь raw samples не уменьшаются. Например,
wall=1794220782770/observed=1794220782771; это не доказательство отката ОС.
Ошибка checkCards:106 — error вместо ok. Политика ADR-0038 прямо отклоняет
календарь ниже observed; её ослабление либо изменение тестов не выполняется.
Уточнён scope исправления часов: отдельный PR или текущий #80; ответ разработчика
ожидается. Это не разрешение слить с failed checks.

Android prototype-ui [129107549187](https://github.com/Zeyafix/zedone/actions/runs/42601715411/job/129107549187)
прошёл environment/Activity recreation/подключение WebView и упал в
qr-scenarios:67: screenshot Timeout 10000ms при независимом декодировании QR.
Это повтор наблюдаемого GAP-012, без изменения QR source/assertions/timeouts.
GAP-014 и GAP-012 требуют отдельных причин; общий инфраструктурный корень
не доказан. HTTP 504 в двух полных логах не зарегистрирован.

На момент уведомления Android shell с диагностикой GAP-016 ещё выполнялся,
как и часть других jobs. Его результат и оба aggregate не приняты заранее.
Оба failed лога сохранены вне Git. Новые commits/push/retry не выполнялись,
чтобы не отменять незавершённый диагностический запуск. Watch остаётся включённым;
следующее действие — получить завершённый shell evidence и решение о scope часов.

## Итог текущего CI и передача решения — LOG-423

Head 131f068: Android shell 129107549798 целиком SUCCESS, 10:33:33–10:41:30
UTC (7:57). В 10:41:07 лог подтвердил полный Card Markdown, точный исходник
после UI-правки, hide/retry/reload/completion/reverse и обратную агентскую
правку. Шаг negative compiler/version/cleanup и все post steps success.
Это новый native результат; он не доказывает причину прежнего GAP-016:
добавленная card.read сверка могла изменить timing, продукт не исправлялся.
Отделённая arm64 сборка также SUCCESS, общий 10m budget сохранён.

Все 13 checks завершились: девять SUCCESS, два непосредственных failure
(Android prototype-ui screenshot и Windows shell clock) и два корректно
failed aggregate. Полный CI не принят, merge/delete не выполняются.
GAP-012/014/015/016 пока не закрываются; scope clock fix ожидает ответа.
Новых исходников/коммитов/push/retry нет. Результаты и ограничения PR body
актуализируются; watch выключается для передачи решения разработчику.

## Запоздавшие aggregate уведомления — LOG-424

Проверены оба итоговых jobs того же head 131f068: Android 129111031637
и Windows 129111216058. Оба выполнили только строгую проверку RESULT=failure
и корректно завершились exit 1; самостоятельных новых ошибок и HTTP 504 нет.
Это итог уже разобранных QR screenshot/clock отказов, не новый CI attempt.
Новых commit/push/retry/merge/delete нет. Повторный unwatch подтвердил
wasWatching=false/watching=false: уведомление пришло после выключения watch.
Ожидается прежний ответ разработчика о scope исправления часов.


## Обновление из main — LOG-431

В PR #80 включается main c04cf3c с FIX.01/PR #81: календарный rollback и
effective expiry разделены по разрешённому договору. Clock source и
product-core docs взяты без изменений из проверенного main; временная
диагностика восьми raw samples удалена из feature diff. Остальные UI/Markdown/
CI сценарии и лимиты прежние. Исходные dirty docs сохранены; журнал содержит
прежнюю историю и FIX.01/LOG-426–430. Новый полный CI ещё не пройден,
P05.04 не done; GAP-015/016 не закрыты автоматически. Следующий шаг —
локальные product/UI checks объединённого кода и публикация обновлённого head.
При новом HTTP 504 остановиться без retry/merge.

## Локальная проверка после включения FIX.01 — LOG-432

Полный product-check passed: 279 Rust tests, 23 client/239 assertions,
release всех targets/features, 7 unsafe guards, OpenAPI/types/strict contract.
Полный Windows ui:check на Node baseline 24.22.1 passed с прежними workspace/
card/editor/history/completion/delivery и CommonMark сценариями. Общий Markdown
отдельно passed с desktop/light и mobile/dark/native touch/visibility adapters;
это host proof, не локальная Device acceptance нового head.

Routing guards обоих CI и actionlint passed. Проверены побайтовая сохранность
четырёх посторонних dirty docs, все 98 критериев/статусы и прежние события,
добавление FIX.01 без подмены P05.04 и полнота исходных записей EV-080.
Clock implementation/product-core docs совпадают с проверенным main c04cf3c.
UI/Markdown источники и существующие медиа визуального diff не менялись;
локальная Device-проверка выше сохраняет охват неизменённого viewer.
Полный новый CI/ABI/native UI ещё ожидается; старые failures сохраняются.
