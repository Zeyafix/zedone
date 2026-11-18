# EV-121 — CI.15: независимые UI-сценарии внутри одной job

## Карточка

- База: PR #109/head 279cb85; CI.14 warm host attempt 2/Android attempt 3,
  все 27 SUCCESS. Незакоммиченный checkpoint CI.14 сохранён отдельно.
- Цель: сократить последовательную часть host shell, сохранив сценарии.
  Полный обязательный CI должен укладываться в 10m, ориентир — 5m.
- Основание: Windows shell 12:29, из них browser driver около 7m;
  Media Foundation install после browser cache hit — 185.9s. Android shell,
  prototype-ui и pairing-placements тоже >10m; этот шаг их не ускоряет.
- Scope: main browser dispatcher, guard его покрытия/ошибок, docs.
  Product API/runtime/UI, матрица 27, budgets и сценарные ожидания прежние.
- Изменение: lifecycle/editor, desktop board/navigation и touch выполняются параллельно
  внутри существующей shell job. Каждый slice создаёт собственные fixture,
  каталог/SQLite/session, origin и browser/contexts. Только бинарник и assets
  общие и неизменяемые. Screenshot names принадлежат board/navigation.
  Навигация следует за desktop accessibility на той же page: её исходную
  подготовку состояния нельзя удалять при разделении сценариев.
- Full/default и conflicts выполняются по прежнему последовательному пути.
  Все 20 вызовов main-сценариев сохранены, включая desktop и touch варианты.
  Assertions и таймеры внутри сценарных модулей не меняются.
- Любой sync/async failure завершает suite ошибкой; Promise.allSettled ждёт
  всех peers и их cleanup до возврата. Зелёный subset не становится SUCCESS.
- Обратимость: revert dispatcher; нет миграций и пользовательских данных.
- Проверки: точный multiset вызовов full/main slices, одновременный старт,
  ожидание peers после каждого отказа, сохранение нескольких ошибок;
  реальный Windows main UI; host/tool-cache guards, comment policy и syntax.
- Два local main runs: board и touch прошли, lifecycle отклонил Tab focus
  (actual null вместо radio). Ожидание focus на h1 не устранило отказ и
  убрано. Навигация перенесена за desktop accessibility в board/navigation,
  сохраняя прежнюю подготовку состояния и исходный assert. Все peers
  завершились и были очищены, failures сохранены. Новый полный запуск прошёл все три slices.
- Состояние: verified локально; новый CI/измерение ускорения CI.15 ожидаются.
- >300 строк diff в основном обусловлены отступом существующего тела driver
  внутрь функции с изолированным состоянием. Новые dispatcher/guard/docs
  проверяют одну границу параллельного исполнения; продукт не меняется.
- Пять правил: обратимость обеспечена revert; общий human/agent API прежний;
  docs обновляются; Windows/Android сценарии сохранены; общий API/данные
  остаются авторитетными, синтетические хранилища тестов изолированы.
- Далее: PR #109/27 checks/измерение, затем
  ускорить Android и оценить полное время с очередью. Merge/delete не поручены.

## Локальная проверка / LOG-662

Полный `ui:check --main` на Windows прошёл с pinned Node: QR tests, types,
Vite build, fmt/Clippy/native shell tests/build, затем три concurrent browser
slices. Все 20 вызовов main-сценариев и их inner assertions сохранены;
навигация сохраняет подготовку desktop accessibility на той же page.
Host/tool-cache/new slice guards, Node syntax, comment policy (388 sources),
diff checks passed. Обе неуспешные попытки сохранены; автоматического retry
в driver нет. APK/Android код не менялся, новая локальная Device-проверка
этого host-only шага не требуется; прежние Android результаты — CI.14,
проверка совместного нового head всеми 27 suites ещё ожидается.

98 criterion rows сверены без изменений, все GAP/DEC прежние. Дальнейший
шаг — публикация в PR #109 и измерение, затем Android bottlenecks; общей
готовности <=10m или <=5m пока нет. Merge/delete не поручены.

Разработчик явно разрешил менять сами тесты при сохранении качества.
Следующие ускорения могут менять подготовку, структуру и ожидания тестов;
неизменность текста assertions не является отдельным продуктовым требованием.
В этом шаге исходные гарантии и сценарные проверки сохранены.

## Публикация / LOG-663

de55014 в PR #109; новые 27 checks pending, runs 44305037759/44305037719.
PR/readback/история failures и метрики CI.14 сохранены. Существующий watch
продолжает исходную задачу; далее время CI.15 и ускорение Android/тестов.
Новых CI.15 SUCCESS или измеренного ускорения пока нет. Merge/delete не поручены.

## CI.15 результат / LOG-664

de55014: все 27 SUCCESS в attempt 1. Windows shell 7:35 против прежних
12:29; browser step 3:08 против 8:00. Все три slices/20 сценарных вызовов
и cleanup passed. Linux shell тоже passed. Все jobs <10m, но полный
результат от создания workflows — 10:01, поэтому общая цель ещё не
достигнута с запасом. Android prototype-ui 9:49 — критический путь;
следующий шаг CI.16 перекрывает его независимую сборку загрузкой эмулятора.
Одна серия не доказывает стабильность; raw runs/jobs/steps сохранены.
