# EV-160 — независимые полные порядки разрешения field conflicts

```yaml
step: FIX.07
date: 2026-11-26
base_revision: 22ad15567d200c7fee17fcf55e3b545e7d55de76
branch: feat/p07-04-daily-snapshots
requirements: [T09, T16]
goal: "Выполнить все три field-conflict порядка с общим lifecycle в прежних240s каждого fixture."
scope: ["field case selector", "paired CI routing/preparation", "guards/docs"]
result: "Локальные guards passed, новый CI31 впереди."
reverse_and_failure: "Полный --fields/--standard сохраняется; любой failed/cancelled case отвергается общим aggregate."
next: "Проверить все31 checks и feedback, затем порученные merge/удаление."
```

Новая диагностика22ad155/job139924068906 доказала непосредственный отказ:
`access_denied; operation=session.unlink; target=android; elapsed_ms=240068`,
затем host exit101/reason=fixture_timeout/248991ms. Все три combined matrices
и exchange recovery уже прошли. Отмена самого job подтверждена annotation15m0s,
подготовка/компиляция + runtime/cleanup вместе достигли его бюджета. История
в отдельном job passed, но три remaining field cases с общим lifecycle всё ещё
не поместились в прежние240s. Host deadline теперь подтверждён самим CI,
не только временной корреляцией; ранние исторические access_denied без этого
контекста не переклассифицируются задним числом. GAP-047 остаётся open до вердикта.

Добавлены два независимых runners с уже существующей полной проверкой case:

| CI suite | Режим | Неизменённый case |
| --- | --- | --- |
| pairing-fields | --fields-card | Text exchange и card→column→space, новые choices |
| pairing-fields-space | --fields-space | space→card→column, сохранённые choices0/1 |
| pairing-fields-column | --fields-column | column→space→card, сохранённые choices1/0 |

Каждый runner использует собственную fixture и выполняет прежний полный
общий bootstrap/rights/transport recovery/revoke/cancel/reject/timed-grant/
replay/five-attempt-lock/preservation/cleanup. Нет обхода проверки срока или
обнуления таймера в середине case. Fixture240s/UI600s, Android15m, остальные
job/assertion budgets прежние; aggregate требует успеха каждой строки.

`fieldConflictCases(first)` выбирает одну из прежних трёх записей; undefined
сохраняет все. Тело for-loop, каждая операция/assertion и финальная проверка
сохранности/обратного восстановления побайтово прежние после нормализации EOL:
SHA-256 `e21c8efd763e4e8989e2192f6cab72e3262c257bb3d33190bf391787d575b99a`.
Полный `--fields` и `--standard` сохраняют прежнее последовательное накопление
данных; CI case selectors используют независимые пространства/fixtures.

Локальный полный Android routing guard прошёл: точные selectors/build ownership,
сумма трёх field slices равна прежнему fields, fields+history+placements равны
standard (7 expanded entries); literal order/choice vectors и invalid scopes
проверены отдельно. Failure/signal/production install/cleanup и strict aggregate
для каждого нового suite сохранены. Реальный Bash preparation/child failures,
ShellCheck двух изменённых scripts, независимый Actionlint без локально
отсутствующего Pyflakes/интеграции shell и comment policy464 sources passed.
Совместный Actionlint→ShellCheck timeout прошлого шага не выдаётся за PASS.

Runtime/контракт/UI/права продукта не менялись, новый Device UI локально
не запускался: необходим настоящий новый Android CI, одинаковый код не заменяет
платформенную проверку. Пять правил AGENTS соблюдены в scope test infrastructure.
Исходные logs/job metadata/annotations сохраняются в ignored p07-04-complete.
Все98 определения/состояния и75 ID/состояний проблем прежние. Фактический полный
native22ad155 прошёл, GAP-055 при этом не закрыт: причина старой15m неизвестна.
Итоговый CI31/merge ещё ожидаются; успешный поднабор не разрешает merge.
