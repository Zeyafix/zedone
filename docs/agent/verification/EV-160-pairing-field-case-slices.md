# EV-160 — независимые полные порядки разрешения field conflicts

```yaml
step: FIX.07
date: 2026-11-26
base_revision: 22ad15567d200c7fee17fcf55e3b545e7d55de76
branch: feat/p07-04-daily-snapshots
integration_revision: 0928f9452a3a0909e87e8331da287b4c83b170dc
tested_revision: 84922dffb6d2987d4fdbd7edff87bde522cbb1a5
requirements: [T09, T16]
goal: "Выполнить все три field-conflict порядка с общим lifecycle в прежних240s каждого fixture."
scope: ["field case selector", "paired CI routing/preparation", "guards/docs"]
result: "passed в проверенном scope; интегрировано PR126/0928f94 после CI31 на84922df."
reverse_and_failure: "Полный --fields/--standard сохраняется; любой failed/cancelled case отвергается общим aggregate."
next: "P07.04/.1–.4 и FIX.05–07 done; готов P07.05, не начат. Полный P07/T10 отдельно."
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

## Публикация — LOG-854

[PR #126](https://github.com/Zeyafix/zedone/pull/126), commit
`84922dffb6d2987d4fdbd7edff87bde522cbb1a5`. Guard/actual preparation/body-hash/
linters passed; полный CI31 нового head pending. Runtime/UI/контракт прежние,
исторические failures и75 ID/состояний проблем сохранены. Merge/удаление ждут
полного успешного результата/feedback по поручению разработчика.


## Интеграция — LOG-858/859

[PR #126](https://github.com/Zeyafix/zedone/pull/126) merged в
`0928f9452a3a0909e87e8331da287b4c83b170dc`, checked head `84922dffb6d2987d4fdbd7edff87bde522cbb1a5`.
Все31 checks SUCCESS: шесть Windows, четыре Linux, восемнадцать Android
и три strict aggregates. Windows product и native Android прошли после
одного targeted recovery каждый; прочие успешные outcomes сохранены.
Первоначальные Unavailable/15m и прежние field failures сохранены;
GAP-042/046/047/054/055 остаются open в неизвестном scope. Никакие
assertions/240s fixture/15m Android/20m Windows/SQLite policy не ослаблены.

Все три field order/choice vectors и history прошли с общими правами,
повторами/Keystore restart/recovery/reverse и восстановлением исходных
APK/файлов по SHA-256, очисткой keys/transport/emulator. Windows replay
тест теперь passed; storage370/search19371ms/contract/invalid11 passed.
Native storage371/search11226ms, парные TLS/pending-request probes,
идентичный контракт и cleanup passed. Это фактические логи нового head;
исходные отменённые jobs не считаются успешными.

Перед merge сверены head/base/behind0/mergeable/nondraft, полный feedback
пуст и все31 outcomes SUCCESS. Merge содержит head и имеет то же дерево.
Main=origin/main/0928f94; feature-ветка удалена local/remote/tracking,
три исходных dirty checkpoint сохранены и проверены SHA-256.
P07.04/.1–.4 и FIX.05/06/07 done; A13/A43 passed по матрице EV-157,
остальные96 состояния/все98 определения и75 ID/состояния проблем прежние.
Физический arm64 runtime и реальный power-loss не проверены и не заявляются;
полный P07/T10 partial. Обратимость, общий API, docs, паритет в проверенном
scope и один авторитетный граф соблюдены. Далее P07.05 готов, не начат.
Итоговый checkpoint локальный без дополнительного commit/push; raw CI/media/
машинная proof остаются в ignored reports. Post-merge push CI не подменяет
пройденный PR CI и здесь не заявляется.
