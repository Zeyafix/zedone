# EV-123 — CI.17: уменьшить очередь длинных проверок

- База PR #109/head 0ca86be, все 27 SUCCESS в attempt 1; CI.16 lifecycle,
  реальный overlap/runtime/cleanup и SDK cold install/save подтверждены.
- Цель: полный CI <=10m, ориентир <=5m с сохранением гарантий. CI.16 wall
  12:18: pairing-fields 10:00 + queue 1:44; product-native queue 3:03.
  Наблюдаемый peak 20 jobs — измерение серии, не установленная нами квота.
- Scope: Linux job grouping/limit, полный UI dispatcher, guards/docs.
  Product API/runtime/schema, Android workflow/SDK key и Windows suites прежние.
- Linux: product, shell, prototype-ui — три полных pipeline вместо шести
  срезов. `max-parallel: 2`; product/shell первыми. Полные команды сохраняют
  Rust/native/clients/contracts/types/release/unsafe, все UI и paired probes.
  Prototype-ui теперь запускает full prototype, включая native checks.
- Full UI выполняет parallel main, затем conflicts, затем pairing. Общие
  types/build/fmt/Clippy/native shell tests/fixture build исполняются один раз.
  Explicit main/conflicts и прямой browser default остаются доступны.
- Windows шесть и Android двенадцать suites прежние. Три strict aggregate
  names/условия прежние; всего 24 expected checks. Ни один unique сценарий
  не удаляется. Guards проверяют full/slice union, точный routing и ошибки
  каждого шага full UI, включая оба browser режима и pairing.
- Обратимость: revert grouping/dispatcher; нет миграций или пользовательских
  данных. Failure/cancellation любой full job не проходит aggregate.
- Проверки: host/tool-cache/main-slice guards, actionlint с ShellCheck,
  Node syntax passed. Реальный полный UI pipeline Windows прошёл;
  полное сочетание на Linux и фактическая очередь ещё pending CI.
- SDK key прежний и новый cache уже сохранён. Warm benchmark предыдущего
  head отложен: он не устраняет очередь. После CI.17 success выполнить один
  controlled полный повтор на неизменном head. Все failures/cancellations,
  очереди, runner work и cache restore/save/eviction учитывать явно.
- На CI.16 snapshot caches сократился с 24 до 17 entries / 8.03 GiB:
  исчезли старый SDK и семь старых Rust/Gradle entries, новый SDK добавлен.
  Полные IDs/keys/bytes сохранены локально; квота/платные настройки не менялись.
- Пять правил: обратимость/default paths, общий human/agent API, актуальные
  docs, прежние обе платформы и единый авторитетный набор pins/data сохранены.
- Состояние: verified локально; целевое время и успех нового CI не заявлены.
- Далее: полный UI, PR #109/24 outcomes/queue/wall, затем controlled benchmark;
  при ухудшении пересмотреть grouping, не увеличивать пороги. Merge/delete нет.

## Локальная проверка / LOG-670

Полный ui:check на Windows с pinned Node прошёл QR/types/Vite/fmt/Clippy/
native shell tests/build, три parallel main slices, все desktop/touch
conflict cases (включая восемь combined positions), scanner и pairing/
administration/exchange UI. Новый Linux full grouping ещё pending CI.
Host guard подтверждает full/slice union product/prototype/UI и failure/
signal каждого шага полного UI, оба browser режима и pairing. Tool-cache/
main-slice guards/actionlint с ShellCheck/Node syntax/comment policy
390 sources/diff passed. Android workflow и его SDK fingerprint прежние.
98 criterion rows/GAP/DEC не меняются; merge/delete не поручены.

## Публикация / LOG-671

2be1ec6 в PR #109; 24 expected checks, новый CI pending. Полные Linux
pipelines/queue/wall ещё не приняты. Далее существующий watch и один
controlled benchmark после успеха; merge/delete не поручены.

## Linux product search timing failure / LOG-672

На 2be1ec6 Linux product отказал на прежнем 1000-card probe: 31208ms,
1000 корректных результатов; остальные storage tests 283 passed. Release/
unsafe после отказавшего test stage в этой job не исполнялись и не приняты.
GAP-034 остаётся open; без диагноза повтор не запрашивается. Исходные
log/job/run snapshots сохранены. CI.18/EV-124 сохраняет workload и <30s,
но делает probe отдельным обязательным запуском на host/native Android.
