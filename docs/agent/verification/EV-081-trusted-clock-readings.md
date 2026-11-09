# EV-081: доверенные часы и растущий календарь — FIX.01

## Задача и исходная ревизия

Разработчик выбрал отдельный PR для GAP-014. База — main `8e87273`,
ветка `fix/trusted-clock-readings`. Открытый [PR #80](https://github.com/Zeyafix/zedone/pull/80)
с Markdown не меняется; его семь dirty документов остаются в исходном
рабочем каталоге и в новый commit не включаются. EV-081/FIX.01 не закрывают P05.04.

## Наблюдение и воспроизведение

[Windows shell 129107549515](https://github.com/Zeyafix/zedone/actions/runs/42601715418/job/129107549515)
на head 131f068 вернул unavailable для space.read/card.list. Trace:
raw wall=1794220782770, observed=1794220782771; последние восемь raw samples
не уменьшаются. Истинное происхождение расхождения часов ОС не доказано.

Настоящие millis/TransactionClock/impl Clock извлечены без изменения логики
в изолированный std-only Rust probe, без базы/данных/настройки часов ОС.
При известной монотонной проекции первый raw=1000000 дал effective=1000010;
следующий raw=1000009 строго больше первого, но clock.now вернул Unavailable.
Проверка желаемого продолжения чтения упала; разница с observed ровно 1ms.
Это детерминированно доказывает механизм: сравнивается raw calendar с общей
отметкой, куда ранее вошла монотонная проекция. Это не доказательство причины
дрейфа ОС/виртуальной машины. Изолированный реальный Windows probe из пяти
миллионов чтений отказа не воспроизвёл; локальный успех не отменяет CI evidence.
Сырые результаты и скрипты только в игнорируемом docs/reports/fix-clock/.

Все десять исходных clock tests Windows passed, включая frozen-wall,
реальный откат, expiry до commit/при отказе и restart. Frozen Bun install
passed; product source/DDL/API и исходные tests пока не изменены.

## Граница договора и предлагаемое решение

[ADR-0038](../../architecture/decisions/0038-trusted-access-time.md) требует
Unavailable при календаре ниже наблюдённой общей отметки. Существующий
forward_jump_rebases_the_monotonic_floor_and_a_frozen_wall_cannot_extend_access
явно закрепляет отказ ниже монотонной проекции, даже без действующего срока.
Следовательно, допуск описанного случая меняет текущий договор; нельзя молча
удалить assertion, ввести tolerance/sleep/retry или перестать сохранять expiry.

Предложено хранить отдельно календарную отметку для обнаружения реального
отката и эффективное время для срока доступа. Эффективное время не уменьшается
и сохраняется также при отказе; истёкшее право не возрождается после restart.
Монотонный ход продолжает ограничивать доступ при замершем календаре.
Обе отметки принадлежат общей транзакции и единственному authoritative
набору данных; JSON/HTTP/IPC не выбирают часы. Нужны управляемые regression
tests для slow/forward/frozen/backward calendar, нескольких workers, restart,
expiry до commit и сохранности при отказе. Схема и инструкция старым базам
должны соответствовать DEV.01/ADR-0047, без новых продуктовых миграций.

Этот раздел описывает предложение на LOG-426; последующее решение — ниже.

## Принятое разделение — LOG-427

Разработчик явно выбрал «Разделить отметки времени» и разрешил уточнить
ADR-0038/прежний frozen-wall test. В одной строке access_clock хранятся
calendar_ms и effective_ms с invariant effective>=calendar>=0. Raw сравнивается
только с calendar; effective=max(raw,persisted effective,monotonic projection).
Опора перебазируется также при повышении effective другим worker, чтобы
старый process-local anchor не задерживал дальнейшее истечение.
Обе отметки сохраняются при отказах/панике без частичной работы команды.

Новый test advancing_calendar_below_the_monotonic_projection_remains_available
упал на baseline (Unavailable) и прошёл на исправлении; в нём проверяется
также настоящий 1ms rollback. Все 14 clock tests Windows passed. Дополнительные
проверки: frozen wall вызывает AccessDenied по монотонному сроку, этот отказ
сохраняет effective выше calendar и не оживляет право после restart;
две connections перечитывают обе отметки/rebase; error/panic откатывают работу,
но обе отметки переживают reopen. Старые expiry/commit/rollback проверки сохранены.

Стартовая v1 и синтетический эталон обновлены на месте по ADR-0047, без
нового перехода. Старый clock layout отклоняется без записи/сброса.
Отдельная проверка fixture подтвердила неизменность всех остальных таблиц,
их строк, application_id/user_version и integrity. Новый SHA-256 эталона:
bd624dc6f7e525300970b94e6a6b74175bf1e8e5ed010d8ecc6145c33ec01d86.
Первый product-check выявил старый ожидаемый SHA в test; он актуализирован
по новому файлу без ослабления assertion. Полный повтор выполняется.

ADR/user/agent/development документация обновлена; API/Rust DTO/зависимости
и алгоритм выдачи/отзыва прав прежние. Дальнейшая проверка — ниже.

## Финальные локальные проверки — LOG-428

- Полный `bun scripts/check-product.mjs` passed: 279 Rust tests, 23 client
  tests/239 assertions, fmt/Clippy, all-target/all-feature release, 7 unsafe
  guards, OpenAPI/types и strict contract tsc. Исходный hash-check отказ
  сохранён отдельно; финальный запуск прошёл целиком.
- Полный `bun run ui:check` Windows passed на Node baseline 24.22.1,
  Chromium 156.0.8078.4: types/build/3 shell tests, настоящий HTTP lifecycle,
  containers/cards/editor/history/completion/inverse, IME/durable draft,
  stale/denied/exact replay, visibility/second tab/30s boundary и темы/reload.
- Все 15 clock tests дополнительно passed с opt-in debug diagnostics ON;
  обычные OFF также passed. Тест frozen calendar теперь требует AccessDenied
  по монотонному expiry вместо прежнего Unavailable по смешанной отметке,
  согласно явному решению разработчика. Остальные security assertions сохранены.
  Отдельно проверено истечение на финальном commit gate при замершем calendar:
  данные и receipt отменены, effective время отказа сохранено. После добавления
  этого случая product-check выполнен заново; runtime код не менялся.
- Docs links/fences/diff, все 98 acceptance definitions/statuses и прежние
  события журнала сохранены. Исходная ветка/head и семь dirty docs #80 прежние.

Пять правил в этом локальном охвате: обратные действия карточек/контейнеров
прошли; человек/агент используют прежние общие Service-команды; user/agent/
developer docs и ADR актуальны; Windows проверен, native Android CI ещё
ожидается; один authoritative SQLite clock row содержит две разные отметки,
runtime проекция не является независимым хранилищем прав/данных.

Локальный Device и физический arm64 заново не проверялись для внутреннего
clock fix. Оба Android ABI/native tests/UI проверит штатный CI отдельного PR.
Старые development базы не мигрируют и не сбрасываются; несовместимый layout
отклоняется без записи. FIX.01 verified локально, не done; GAP-014 не resolved
до полного CI/интеграции. PR #80 остаётся открытым и неизменённым.
