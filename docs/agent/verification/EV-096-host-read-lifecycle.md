# EV-096 — CI.09: жизненный цикл чтений при отказе Windows UI

Дата: 2026-11-14. PR #91/head 49b7015; GAP-028, T08/T16.

## Рабочая карточка

- Цель: установить причину повторного Windows touch timeout либо сохранить
  недостающие сведения о начатых/завершённых/отказавших чтениях, не меняя
  поведение приложения, исходную ошибку, число попыток и существующие 10s.
- База: feat/p06-01-2-pairing-protocol/49b7015; одиннадцать исходных dirty
  документов сохранены с SHA-256 и binary patch в ignored отчёте.
- Зависимости: CI.08 done/PR #90/90c869d; P06.01.2.1 verified/EV-095.
- Scope: host diagnostics/browser runner/негативный контроль, связанные docs.
  Продуктовый runtime/API/UI/схема/таймауты/CI-матрица не меняются без
  отдельного доказанного основания.
- Проверки: raw logs обоих failed jobs, ограниченный touch probe с реальным
  Service и CPU slowdown, полный прежний ui:check Chrome/Opera,
  контролируемые pending/headers/finished/failed/ошибка reporter/secret exclusion.
- Обратимость: read-only диагностика не меняет состояние и не требует undo.
  Общий Service/данные/права/паритет продукта сохраняются.
- Результат: CI.09 verified локально в диагностическом срезе. Первопричина
  timeout неизвестна, GAP-028 не закрыт; runtime/попытки/таймауты прежние.
- Далее: scoped commit/push в прежний PR и новый полный CI всех 17 checks
  до ранее порученных merge/delete.

## Наблюдение

Raw logs run 43616748431/head 49b7015:
windows-checks (shell), job 132513369693, после полного desktop охвата и
успешных touch moves/tags/search отказал checkTrash:60 → go:42: заголовок
доски не появился за 10000ms. Правильная touch-страница сохранена до disposal:
connection ready, busy=false, hash выбранного пространства, доски нет,
общий alert о неподтверждённом результате. В metadata есть успешные
space.list/space.read; для остальных чтений по этим данным нельзя различить
задержку, отказ транспорта или незавершённую обработку ответа.

check (windows-2025), job 132516026646, получил RESULT=failure и корректно
отказал. Независимой ошибки aggregate нет. Остальные 15 checks SUCCESS,
включая Linux shell/product и все Android suites/aggregate.

Это новое наблюдение GAP-028, а не доказательство общей причины с отказом
PR #90. Исходная причина неизвестна; успешный повтор её не устранит.

## Изменение и проверка

Добавлен отдельный host observer readLifecycle: до 30 последних запросов
пяти read-команд и /host/session с локальным sequence, path без query,
started/headers/finished/failed, HTTP-кодом и длительностями. API-body отмечается
pending/valid/invalid/unreadable по производному Rust-валидатору; содержимое
ответа и ошибка JSON не выводятся. Новый observer не читает тело host/session.
Незавершённые запросы становятся видны даже без response/json события.

Observer подключён к той же touch-странице до goto. Его снимок добавляется
в исходный report перед disposal; исходный error/одна попытка/10s/порядок и
состав UI-матрицы прежние. Никаких новых продуктовых запросов или повторов
при отказе нет. Product runtime/API/UI/схема и 18 EV-095 fingerprints прежние.

- Ограниченный tags→search→trash touch probe с реальным Service и CPU slowdown 4
  passed. Исходный Windows timeout не воспроизведён; причина неизвестна.
- Постоянный control прошёл Chrome/Opera: 35 запросов дают 30 записей;
  started/headers/body pending/finished/valid/invalid/unreadable/failed,
  stripped query и отказ чтения host/session/unrelated bodies проверены.
  Snapshot не изменяет внутренний журнал, исходный секрет отсутствует.
- Настоящий browser fetch удержан тестовым route: до ответа виден started,
  исходный TimeoutError сохранён; после abort виден failed. Ошибка reporter
  также не заменяет исходный error. Controlled guard не меняет реальный
  timeout действий и не повторяет сценарий.
- Полный `bun run ui:check` Chrome/Opera exit 0: типы/build/fmt/Clippy,
  shell tests, весь прежний desktop/touch/Markdown/lifecycle. Этот запуск
  начался до последнего подключения нового observer/control.
- Финальные guard и все семь touch-сценариев Chrome/Opera прошли последовательно
  после завершения полного прогона: moves/tags/search/trash/history/cleanup/
  accessibility, с тем же подключением observer до goto и без pageerrors.
  Syntax/diff checks passed; три diagnostic fingerprints и 18 прежних
  product fingerprints сохранены локально и проверены.
- Один ранний повтор остановился до UI на Windows file lock:
  предыдущий прогон держал тот же fixture executable. Он не засчитан;
  вместо параллельного rebuild финальная матрица затронутого touch-набора
  проверена последовательно; runtime/сборка/остальной UI прежние.

Новая Android UI/ABI матрица локально не повторяется: меняется только
host-диагностика. Полный новый CI всех платформ обязателен до merge/delete.
GAP-028 остаётся in_progress; отсутствие очередного отказа не докажет исправления
его причины. Все 98 определения/состояния и прежние GAP/DEC сохраняются.

Пять правил в прежнем scope: read-only диагностика не требует undo;
Service/права человека и агента/паритет/данные прежние, agent/development
документация обновлена. Все восемь исходных dirty EV побайтово прежние;
исходные записи журнала сохранены. Новый CI ещё не запускался;
commit/push/merge/delete ещё не выполнены. Следующий продуктовый подшаг не начат.
