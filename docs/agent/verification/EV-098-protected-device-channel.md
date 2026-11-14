# EV-098 — Защищённая идентичность канала устройств

## Рабочая карточка

- Шаг: P06.01.2.2, done в host-срезе; база main/e34b8fd, ветка
  feat/p06-01-2-2-secure-channel удалена после интеграции PR #92/d0b4e4c.
- Цель: host сохраняет идентичность в системном защищённом хранилище,
  после restart устанавливает TLS 1.3, клиент отвергает подмену до отправки
  кода/ключа. Источник: T09/A51, ADR-0009/0053/0054/0055 и DEC-003.
- Зависимости: P06.01.1/EV-093/90c869d, P06.01.2.1/EV-095/e34b8fd.
- Scope: channel/secrets в adapters, Android host bridge/Keystore,
  Cargo manifests/locks, штатные tests/CI и user/agent/development/architecture docs.
- Проверки: штатный product-check Windows/native Android/arm64; host shell
  tests; настоящий Android Keystore через native instrumentation в Device.
  Результаты и ограничения приведены ниже.
- Обратимость/отказ: явное удаление/новая идентичность; старый fingerprint
  отвергается. Ошибка записи/чтения/повреждение не создают новую идентичность;
  plaintext, неверные сертификаты и незавершённый handshake отклоняются.
- Размер: runtime и доказательства могут превысить ориентир 300 строк:
  проверяемый результат неделим без сохранения server identity на обеих ОС;
  публикационная граница проверена: одна постоянная защищённая идентичность
  TLS на обеих ОС. PR/условная интеграция поручены отдельно в LOG-532.
- Результат: host-компоненты интегрированы PR #92/d0b4e4c после всех 17
  SUCCESS/readiness; полные T09/A51, DEC-003 и прежние GAP прежние.
- Далее: P06.01.2.3 по новому поручению.

## Реализация и граница

`adapters::channel` — TLS 1.3/ring, точный SHA-256 pin DER-сертификата,
обычные проверки webpki имени/срока и доказательства владения ключом.
Нет early data, resumption клиента или plaintext fallback. Handshake ограничен
пятью секундами; это новый внутренний бюджет, прежние UI/CI бюджеты не менялись.
Слушатель и его ограничение соединений остаются обязанностью будущего host.

Явное create_identity сохраняет секрет и проверяет чтением до выдачи пригодной
идентичности; load_identity ничего не создаёт. Identity не реализует
Debug/Serialize/Clone. Ошибка/повреждение не заменяются новой идентичностью.
Windows Credential Manager/Local и Android AES-256-GCM/Keystore/AtomicFile
хранят секрет вне SQLite/Automerge/receipts/снимков. Создание не перезаписывает
имеющееся значение, удаление явно. Namespace/слот аутентифицированы на Android.
Блокировки процесса и файла сериализуют host-операции одного namespace.

Android Rust host bridge зарегистрирован без renderer-команд: dispatcher
отклоняет любые обращения страницы. Kotlin отвечает только фиксированными
статусами, не строками исключений. Новых таблиц, полномочий или JSON-команд нет.
Код приглашения, текущий pre-provisioned local-session.json и выдача ключей
ещё не переведены в этот механизм; новый слот пока только для TLS identity.
Их полноценная host-композиция/процесс подключения остаётся P06.01.2.3–.4.

## Проверки

| Проверка | Результат |
| --- | --- |
| Финальный `bun scripts/check-product.mjs` Windows | exit 0: 305 Rust-тестов (7 channel/40 commands/17 core/7 contract/234 storage), 26 client/271 assertions; fmt/Clippy/release, 7 unsafe guards, прежний контракт/типы |
| `bun scripts/check-product.mjs --android-native` в Device | exit 0: 304 Rust-теста, 26 client/271 assertions, release x86_64/7 unsafe guards, контракт/типы; временный sandbox удалён |
| Парный host/native Android probe | Вызван явно несмотря на ignored в обычном lib target: по одному процессу host и Android; неверный pin отказал, двусторонний TLS прошёл. Host identity сохранена/загружена настоящим Windows Credential Manager. Только тестовый TCP через adb reverse, правило удалено |
| `--android-arm64` | exit 0: release aarch64/7 unsafe guards/контракт/типы; всего 21 guard на Windows/обеих ABI |
| `build-ui-android.mjs x86_64` и `aarch64` | exit 0: финальные debug APK/manifest и native instrumentation APK обеих ABI; Android host bridge скомпилирован |
| `check-host-secrets-android.mjs` | exit 0 на финальном x86_64 APK: native Keystore assertions и очистка passed; test APK удалён |
| Host shell fmt/Clippy/tests | exit 0; все 3 прежних shell tests passed |
| Device UI | Свежий отдельный product-каталог: unconfigured session через IPC, меню/нативный Back passed. Оригинальный каталог сохранён и восстановлен; UI действия только agent-device, просмотр device_screenshot |

Channel tests: сохранение/повторная загрузка, новое создание не заменяет
идентичность, удаление/новая идентичность и отказ старому pin; неправильный
сертификат, копия правильного сертификата без private key, имя/срок/лишняя
цепочка; HTTP вместо TLS, незавершённый handshake/5s; повреждённая запись,
несовпадающий private key и недоступное хранилище. Последняя правка относится
только к cleanup probe/Windows test после panic и охвачена финальным Windows
product-check; Android client и runtime к тому времени прежние.

Native Keystore: повторное открытие, пределы 0/2049 против 2048, зашифрованные
байты на диске, неэкспортируемый AES-256/GCM key, отказ повторному созданию,
конкурентное создание с одним победителем, повреждение ciphertext, утрата key,
AtomicFile recovery через .bak, удаление/новое создание и отсутствие тестовых
namespace/key после очистки. Это проверки native store отдельно от Rust
PluginHandle; полный Rust→Kotlin→TLS процесс и настоящий process restart
устройства пока не принимаются. Физический arm64 и Tailscale двух физических
устройств не проверены. Linux CI и полная UI-регрессия этого diff ещё не запущены.

Промежуточные ошибки исправлены до финальной проверки: Clippy slice clone,
Kotlin Key→SecretKey в test runner и неверное предположение о коде pm path для
отсутствующего test APK. Они не объявляются исправлением прежних GAP.

## Сохранность и пять правил

Оригинальный APK и 24 файла product/WebView/prefs восстановлены с совпадением
имён и SHA-256. Исходный каталог не пересоздавался; свежий тестовый удалён после
проверки сохранённого оригинала. Первое восстановление через stdin остановилось
на Windows text EOF/Android tar seek; оригинальный архив был проверен целиком,
перенесён как ASCII base64 в закрытый каталог приложения, декодирован и проверен
перед успешным восстановлением. Значения секретов не попадали в argv/вывод или
доступный другим приложениям каталог. Временные архивы/test APK удалены,
эмулятор выключен, настройки ОС/IME не менялись.

- Обратимость: явное remove/новая identity, прежний pin перестаёт работать;
  локальные пространства не удаляются. Полное восстановление Device подтверждено.
- Человек/агент: новых пользовательских действий нет; прежний общий
  Service/guard/HTTP/IPC/CLI неизменны. Host secrets не являются обходным API.
- Документация: user/device-pairing, agent/protected-channel/invitations,
  development/product-core, architecture/core-boundaries и ADR-0055 обновлены.
- Паритет: common TLS Windows/native Android, оба системных store и обе ABI;
  полный процесс подключения/UI/физический arm64 отдельно.
- Один источник: один защищённый store значения host, SQLite остаётся
  авторитетным для прав/приглашений; Rust-контракт и производные прежние.

Все исходные EV сохраняются побайтово; 98 определений/состояний приёмки и все
526 исходных LOG сохраняются. В журнал добавлены только доказательства
T06/T09/A51 без повышения состояний. P06.01.2/P06 in_progress, блокеров нет.
Далее ревью/интеграция ветки перед P06.01.2.3; новый продуктовый шаг не начат.


## Подготовка публикации — LOG-532

Разработчик поручил PR и после успеха полный merge/удаление ветки. Все
20 fingerprints проверенных исходников совпали; base main/e34b8fd актуальна.
Полный состав и граница host-компонентов проверены, визуального diff нет.
Одиннадцать исходных dirty EV не публикуются; снимок всех исходных изменений
с SHA-256 и binary patch остаётся локально. Ожидается полный новый CI:
17 SUCCESS/readiness точного head перед merge/delete. Локальные доказательства
и ограничения выше прежние; новый продуктовый шаг не начат.


## Публикация — LOG-533

[PR #92](https://github.com/Zeyafix/zedone/pull/92), head
164e65dccb41c4be9420899d7e37aaa637f2465f, base main/e34b8fd.
Обычный push/pre-push passed, 29 согласованных файлов/20 tested fingerprints,
staged equality/privacy/noreply и сохранность 11 исходных dirty EV проверены.
Body/readback после нормализации CRLF подтвердил три English sections/credit;
comments/reviews/requests/threads отсутствуют, визуального diff нет.

Все 14 runtime suites IN_PROGRESS; три aggregates ожидаются, полный CI pending.
Локальные доказательства не заменяют CI. Разработчик поручил merge/удаление
ветки после всех 17 COMPLETED/SUCCESS и свежей readiness точного head.
При failure требуется raw-log разбор. Пять правил и границы host-среза прежние;
новый продуктовый шаг не начат, записи публикации остаются локально.


## Интеграция — LOG-534

[PR #92](https://github.com/Zeyafix/zedone/pull/92) интегрирован: head
164e65dccb41c4be9420899d7e37aaa637f2465f, merge
d0b4e4c6b117369ab9377ec13aa558f1961d2f3b после всех 17 COMPLETED/SUCCESS и
CLEAN readiness. Body с завершённым CI прочитан обратно; comments/reviews/
requests/threads отсутствовали. Raw logs подтверждают новые channel/store
controls на Windows/Linux/native Android, явно вызванный host/native Android
probe, native Keystore assertions и весь прежний desktop/touch/native UI,
обе ABI/prototype/strict aggregates SUCCESS. Linux использует синтетический
host store; настоящий Windows store подтверждён Windows CI и локальным probe.

Дерево merge равно checked head, main равна origin/main/d0b4e4c; feature-ветка
удалена локально/на GitHub/tracking. Все 14 dirty файлов сохранены побайтово
при переключении; исходные 11 EV прежние, индекс пуст, другие worktrees/stash
не менялись. Записи завершения остаются локально, source push после merge нет.

Пять правил и граница host-компонентов выше сохраняются. P06.01.2.2 done,
P06.01.2/P06 in_progress, все 98 определения/состояния прежние. Прежние GAP
не закрываются успешным новым CI. Полный процесс подключения/прочие secrets/
синхронизация/process restart/физический arm64/Tailscale отдельно. Далее готов
P06.01.2.3 по новому поручению; active_work null, новая реализация не начата.
