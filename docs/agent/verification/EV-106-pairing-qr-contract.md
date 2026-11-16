# EV-106 — общий формат QR подключения

Дата: 2026-11-16. Рабочая карточка P06.02.1.

Текущий итог — [PR #100](https://github.com/Zeyafix/zedone/pull/100), head 9551a35,
merge caaf6df после всех 17 SUCCESS; P06.02.1–.3 done, GAP-030 resolved.
[EV-107](EV-107-pairing-qr-ui.md)/LOG-582 сохраняет UI/оптику Galaxy A32 и новый
Windows/Linux/Android CI. Ниже сохраняется исходный scope API .1.

## Рабочая карточка

- База: main/8707af0, ветка `feat/p06-02-qr-pairing`, исходное дерево чистое.
- Цель: QR передаёт тот же адрес и составной код существующего ручного процесса;
  генерация и разбор доступны общему host API и не подключают устройство.
- Требования: T09/A51, ADR-0009/0022/0023/0055–0060.
- Зависимости: ручной P06.01 done на source 02ae231, EV-104/LOG-568;
  P01.09 done, EV-022/PR #16. Они доступны в базе.
- Объём: чистый генератор `core::qr`, формат представления в adapters::pairing,
  host offer.qr/session.decode_qr, проверки прежнего pending/TLS/отмены,
  необходимые lock-файлы и документация. qrcodegen 1.8.0 переносится из
  проверенного прототипа; зависимость от prototype crate не вводится.
- Не входят: UI/SVG/камера P06.02.2, полная QR-приёмка P06.02.3,
  синхронизация, Tailscale и физический arm64. Следующий зависимый срез ждёт
  интеграции первого по правилам очереди.
- Проверки: `bun scripts/check-product.mjs`, native Android/обе ABI через
  `bun scripts/check-product.mjs --android` при открытом Device; независимое
  программное декодирование матрицы. Точные дополнительные команды фиксируются
  по выполнению. Результаты приведены ниже.
- Обратимость и отказ: генерация/разбор не создают сессию и не расходуют код;
  прежние cancel/reject/unlink остаются общими. Ошибочный QR не меняет сохранённое
  подключение, секреты или состояние приглашения. Новый формат не является
  новым способом выдачи прав и не заменяет явное подтверждение.
- Один вопрос проверки: можно ли точно получить/разобрать QR существующего
  приглашения и пройти прежний процесс с теми же правилами. Объём может превысить
  ориентир 300 строк из-за общей генерации, двух API и проверок отказов/паритета;
  UI и scanner отделены заранее.

## Результат

P06.02.1 verified локально в `feat/p06-02-qr-pairing` от 8707af0.
`core::qr` кодирует полную строку UTF-8; `adapters::pairing::qr` задаёт
строгий формат `zedone-pairing-v1`, канонический IP:порт и неизменный Offer.
Host offer.qr/session.decode_qr используют прежнюю авторизацию и delivery;
генерация/разбор не отправляют запрос и не выдают доступ. Код не получает
новых постоянных копий в SQLite, receipts или намерениях. Отсутствие открытого
приглашения возвращает not_found; разбор старого QR не подтверждает его актуальность.

## Проверки

| Команда/охват | Результат и граница |
| --- | --- |
| `bun scripts/check-product.mjs` | Windows passed: 342 Rust tests, 27 client/280 assertions, policy tests, fmt/Clippy, release/unsafe guards и прежние OpenAPI/TS/validator. После финального уточнения типа маркера отдельно повторены workspace Clippy и adapters library: 24 passed/3 штатных ignored. |
| `bun scripts/check-product.mjs --android` | Passed на Device Android 13/x86_64: 339 native Rust tests, 27 client/280 assertions, release обеих ABI, unsafe/comment guards, matching host/native contract и прежние два парных TLS probes. Проверены итоговые исходники со строгим строковым маркером. |
| `UI_NODE_VERSION=26.8.1 bun run ui:check` | Локальный ранее предусмотренный Node override, CI baseline прежний. Types/build/shell tests, полные Chrome/Opera desktop/touch, accessibility/Markdown/все прежние сценарии и QR→ручной UI passed. |
| Финальные host fixture + `bun scripts/check-ui-pairing.mjs` | После уточнения маркера fixture пересобран и весь pairing runner passed: jsQR 1.4.0 точно читает продуктовую матрицу для loopback/IPv4/IPv6, поля 4 модуля, масштабы 2/4/8; декодированные поля проходят прежние prepare/submit/confirm/read-only/restart/unlink/reject/cancel, выдачу/отзыв/смену прав, stale/исходный retry и сохранность. |
| Настоящий CLI `--endpoint` | Изолированный Windows fixture/Credential Manager: offer.qr и session.decode_qr через pinned TLS passed. Admin разрешён, read-only отказан; invalid QR отказан, cancelled offer не выдаётся; старый QR разбирается только как представление, сессия остаётся unconfigured, данные пространства сохранены. Ключи/pin в файлах, JSON в stdin, секреты не выводились. Локальный runner и raw вывод ignored. |
| Формат и отказ | Неизвестный/пропущенный/нестроковый маркер, duplicate/extra/missing поля, URL/unspecified IP/нулевой порт, неполный Offer, malformed/oversized QR отклоняются. Генератор проверен на границах 2330/2331 UTF-8 байт без обрезки; ошибка разбора при pending сохраняет точные байты OS-слота. |
| Финальные policy/fmt/diff/документы | `bun run check:comments`, `cargo fmt --all -- --check`, `git diff --check` passed. Все 98 определения/состояния критериев, списки/строки GAP/DEC и OpenAPI прежние; локальные ссылки и fingerprints проверены. |

Полный Windows product-check и полный UI предшествовали финальному уточнению
строкового маркера; финальные Clippy/library/fixture/pairing/CLI и полный native
Android выполнены после него. Исходные enum-варианты не оставлены: маркер
принимается только строкой, объект с тем же именем не допускается.

Тестовые установки изолированы, собственные OS-слоты fixtures очищены;
Android runner удалил свой sandbox и transport rules. APK и app data
не устанавливались/не менялись. Device открыт штатным tool и после проверок
выключен; UI Android этим срезом не управлялся. Машинные пути, команды,
raw выводы и SHA-256 находятся только в ignored docs/reports.

## Пять правил и передача

- Обратимость: новые операции представления не создают подключения; прежние
  отмена приглашения/запроса, отклонение и отвязка сохранены и проверены.
- Человек и агент: один DeviceHost для Controller/HTTP/IPC/CLI; новые операции
  требуют прежний ManageDevices и не позволяют подтвердить свой запрос.
- Документация: user/agent/development guidance, очередь/журнал и ADR-0062
  отражают доступный API и незавершённость UI/камеры.
- Паритет: общий Rust/host API проверен на Windows/native Android и обе ABI;
  это не приёмка APK camera/permissions/model/UI или физического arm64.
- Источник истины: прежний registry/OS Offer/Service, QR — производное
  представление, декодированные поля не являются новым статусом приглашения.

Полный P06.02 не завершён. UI/SVG/Tauri Barcode Scanner/разрешения/неготовность
модели остаются P06.02.2, итоговая QR-матрица P06.02.3, обмен/Tailscale/физический
arm64 и полные T09/A51/DEC-003 отдельно. Новых продуктовых багов не выявлено,
прежние GAP/DEC и 98 состояний не меняются. Commit/push/PR/новый CI/merge
не выполнялись. Далее ревью/интеграция P06.02.1 перед зависимым P06.02.2;
новая реализация следующего среза не начата, active_work null.
