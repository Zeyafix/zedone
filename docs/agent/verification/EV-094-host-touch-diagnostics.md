# EV-094 — CI.08: диагностика страницы отказавшего touch-сценария

```yaml
step: CI.08
base_revision: "PR #90/head 72af6e6; девять исходных dirty документов сохраняются вне runtime-правки"
goal: "При отказе host touch записать состояние именно отказавшей страницы и этап до закрытия её context, сохранив исходную ошибку."
requirements: [T08, T16]
dependencies_evidence: [P05.10, EV-092, P06.01.1, EV-093]
scope: [scripts/check-ui-browser.mjs, scripts/check-ui-diagnostics.mjs, scripts/check-ui-native-controls.mjs, docs/agent, docs/development/ci-checks.md]
checks: ["bun run ui:check", "контролируемый отказ с реальным Service и разными main/touch страницами"]
reverse_and_failure: "Только read-only диагностика тестового драйвера; исходный TimeoutError и одна попытка сохраняются, ошибка диагностики не заменяет ошибку сценария."
result: "verified локально: full Chrome/Opera UI + финальные negative controls/moves→tags desktop/touch passed; исходный timeout не воспроизведён, новый CI нужен"
evidence: ["ignored docs/reports/p06-01/pr90-windows-shell-failed.log", "ignored docs/reports/p06-01/pr90-windows-aggregate-failed.log"]
next: "Проверить диагностику, опубликовать в прежнем PR #90; нужен новый полный CI всех 17 checks перед merge/delete."
```

## Исходный отказ

Raw logs Windows shell job 132484710922/run 43607334945/head 72af6e6:
после desktop accessibility (14 экранов) и успешного touch moves остановился
touch checkTags:94 → refresh:30. waitForFunction на spaces-view aria-busy=false
превысил прежние 10000ms. Предыдущие отказы имён тегов ожидаемы по сценарию.
Windows aggregate job 132487228622 получил RESULT=failure и корректно передал
этот отказ; новой независимой ошибки aggregate нет. Остальные 15 checks SUCCESS.

Сохранённое UI failure state относится к основной desktop странице: runner
закрыл touch context в finally, а outer catch прочитал прежний diagnosticPage.
По этому состоянию нельзя установить busy/hash/активный запрос отказавшего
touch окна. Причина исходного timeout пока неизвестна; GAP-028 не закрывается
исправлением диагностики или единичным успешным повтором.

Ограниченный локальный moves→tags прогон Chrome touch с CPU slowdown 4 прошёл;
это отсутствие воспроизведения, а не доказательство исправления продукта.

## Изменение

Состояние touch страницы и её bounded API-read metadata записываются до
закрытия context с метками host touch и конкретного этапа. Ошибка возвращается
тем же объектом, без retry и замены на ошибку диагностики. Основная страница
больше не выдаётся за источник touch failure. Метаданные чтений относятся
к соответствующей странице, включают tag.list и ограничены 30 записями.
Запросы/headers/keys/receipts/сохранённые намерения не читаются.

Продуктовый runtime, API выдачи доступа, UI, порядок/набор сценариев,
таймауты 10s, бюджеты workflow и touch delivery не меняются. Обратное действие
к read-only диагностике неприменимо; общий API/источник прав и паритет продукта
прежние. Формы/канал/две реплики не принимаются. Новый CI ещё нужен.


## Результаты — LOG-517

- Полный `bun run ui:check` Chrome/Opera exit 0: типы/build/fmt/Clippy/shell tests и весь прежний UI/desktop/touch/Markdown/lifecycle. Запуск начат до последнего подключения постоянного guard; финальный hook не объявляется частью этого раннего полного прогона.
- Финальные `checkShellFailureDiagnostics` и moves→tags прошли во всех четырёх режимах Chrome/Opera desktop/touch. Реальный TimeoutError контролируемого невозможного predicate сохраняется тем же объектом, попытка одна, состояние правильной живой страницы/этапа получено до disposal. Отдельный probe deadline 50ms не меняет прежние 10s настоящих действий.
- 35 ответов с секретом в посторонних полях дали ровно 30 безопасных metadata записей; ключ отсутствует в выводе. Ошибка reporter не подменяет исходный TimeoutError. Syntax и diff checks passed.
- Все 15 tested product source fingerprints прежние; схема/контракт/UI/права/таймауты/бюджеты и touch driver не менялись. Новый native Android/ABI/UI локально не запускался, нужен полный CI нового head.
- Девять исходных dirty документов сохранены с SHA-256; шесть прежних EV побайтово прежние. Все 98 определения/состояния критериев и прежние LOG сохранены. Пять правил остаются в прежнем продуктовом объёме, read-only тестовая диагностика не требует обратного действия; agent/development docs актуальны.

GAP-028 остаётся in_progress: исходный Windows timeout не воспроизведён локально, его причина и отсутствие периодических отказов не доказаны. Исправлен подтверждённый дефект диагностики. CI.08 verified локально; commit/push/новый CI/merge ещё не выполнены. Далее scoped публикация в прежний PR #90 и все 17 SUCCESS/readiness перед порученными merge/delete.
