<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { DialogClose, DialogContent, DialogDescription, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from "reka-ui";
import { newCardId, newColumnId, newSpaceId, newTagId } from "../../client/commands";
import { createMutation, deliverMutation, errorMessage, forgetMutation, mutationCredential, pendingCommands, read, tagErrorMessage, type Card, type Column, type Mutation, type Pending, type Space, type Tag, type TrashPreview, type TrashSelection } from "./workspace-api";
import CardsList from "./CardsList.vue";
import CardEditor from "./CardEditor.vue";
import BoardView from "./BoardView.vue";
import SearchPanel from "./SearchPanel.vue";
import TrashPanel from "./TrashPanel.vue";
import TrashScope from "./TrashScope.vue";
import HistoryScope from "./HistoryScope.vue";
import type { HistoryPreview } from "./workspace-api";
import { discardDraft, draftsChanged, listDrafts, type EditorDraft } from "./editor-drafts";
import { pendingChanged } from "./workspace-api";

const props = defineProps<{ ready: boolean }>();
type Action =
  | { kind: "space.create" }
  | { kind: "column.create"; space: string }
  | { kind: "space.rename" | "space.trash" | "space.restore"; target: Space }
  | { kind: "column.rename" | "column.trash" | "column.restore"; target: Column }
  | { kind: "card.create"; space: string; column: string }
  | { kind: "card.read" | "card.trash" | "card.restore"; target: Card }
  | { kind: "tag.create"; space: string }
  | { kind: "tag.rename" | "tag.delete"; target: Tag }
  | { kind: "trash.purge"; target: Space; selection: TrashSelection }
  | { kind: "history.clear"; space: string; card: string | null }
  | { kind: "forget" | "forget-draft"; id: string };
const labels: Record<Mutation | "card.read" | "forget" | "forget-draft", string> = {
  "space.create": "Создать пространство", "column.create": "Создать колонку",
  "space.rename": "Переименовать пространство", "column.rename": "Переименовать колонку",
  "space.trash": "В корзину", "column.trash": "В корзину",
  "space.restore": "Восстановить", "column.restore": "Восстановить", forget: "Убрать из списка",
  "card.create": "Создать карточку", "card.read": "Карточка", "card.trash": "В корзину", "card.restore": "Восстановить",
  "card.begin_editing": "Начать редактирование", "card.autosave": "Сохранить карточку", "card.finish_editing": "Завершить редактирование", "card.set_completed": "Изменить выполнение",
  "forget-draft": "Удалить черновик",
  "card.move": "Переместить карточку", "column.move": "Переместить колонку",
  "tag.create": "Создать тег", "tag.rename": "Переименовать тег", "tag.delete": "Удалить тег", "card.set_tag": "Изменить тег карточки",
  "trash.purge": "Очистить окончательно",
  "card.restore_text": "Восстановить версию карточки",
  "history.clear": "Очистить историю",
};
const routeSpace = () => /^#\/spaces\/([a-f0-9]{32})$/.exec(location.hash)?.[1];
const selected = ref(routeSpace());
const spaces = ref<Space[]>([]);
const space = ref<Space>();
const columns = ref<Column[]>([]);
const cards = ref<Card[]>([]);
const tags = ref<Tag[]>([]);
const editorTags = ref<Tag[]>([]);
const searchOpen = ref(false);
const searchRevision = ref(0);
const searchPanel = ref<InstanceType<typeof SearchPanel>>();
const tagNames = computed(() => new Map(tags.value.map(tag => [tag.tag, tag.names.join(' / ')])));
const tagSubject = ref<string>();
const scopeTags = ref<Tag[]>([]);
const trashPreview = ref<TrashPreview>();
const historyPreview = ref<HistoryPreview>();
const restoreColumns = ref<Column[]>([]);
const restoreSpace = ref<Space>();
const restoreColumn = ref("");
const purgeConfirmed = ref(false);
const actionRefused = ref(false);
const showTrash = ref(false);
const showCardTrash = ref(false);
const loading = ref(false);
const busy = ref(false);
const moving = ref(false);
const error = ref("");
const notice = ref("");
const pending = ref<Pending[]>([]);
const drafts = ref<EditorDraft[]>([]);
const recovering = ref<string>();
const editor = ref<InstanceType<typeof CardEditor>>();
const storageError = ref(false);
const action = ref<Action>();
const dialogOpen = ref(false);
const name = ref("");
const description = ref("");
const formError = ref("");
const titleInput = ref<HTMLInputElement>();
const viewHeading = ref<HTMLElement>();
let opener: HTMLElement | undefined;
let generation = 0;
let localGeneration = 0, lastUrl = location.href, pendingUrl: string | undefined;
const boardBlocked = computed(() => busy.value || pending.value.length > 0 || storageError.value || !props.ready || loading.value);
const blocked = computed(() => boardBlocked.value || moving.value);
const naming = computed(() => action.value?.kind.endsWith(".create") || action.value?.kind.endsWith(".rename"));
const lifecycleTarget = computed(() => {
  const current = action.value;
  return current && (current.kind === "space.trash" || current.kind === "space.restore" || current.kind === "column.trash" || current.kind === "column.restore") ? current.target : undefined;
});
const lifecycleScope = computed(() => {
  const target = lifecycleTarget.value;
  if (!target?.contents) return undefined;
  return "column" in target ? { columns: [], cards: target.contents, tags: scopeTags.value } : target.contents;
});
const actionUnavailable = computed(() => actionRefused.value || !!lifecycleTarget.value && !lifecycleScope.value
  || action.value?.kind === "card.restore" && (restoreSpace.value?.state !== "active" || !restoreColumn.value)
  || action.value?.kind === "trash.purge" && (!trashPreview.value || !!trashPreview.value.conflicts.length || !purgeConfirmed.value)
  || action.value?.kind === "history.clear" && (!historyPreview.value?.entries.some(entry => !entry.retained) || !purgeConfirmed.value));
const title = (item: Space | Column | Tag) => item.names.length === 1 ? item.names[0] || "Без названия" : "Конфликт названий";
const cardTitle = (item: Card) => item.titles.length === 1 ? item.titles[0] : "Конфликт заголовков";
const visibleCards = computed(() => cards.value.filter(item => showCardTrash.value ? item.state === "trashed" : item.state !== "trashed"));
const cardGroups = computed(() => {
  const activeColumns = new Set(columns.value.filter(item => item.state !== "trashed").map(item => item.column));
  const byColumn = new Map<string, Card[]>(), unplaced: Card[] = [];
  for (const card of visibleCards.value) {
    const id = card.columns.length === 1 ? card.columns[0] : undefined;
    if (!id || !activeColumns.has(id)) { unplaced.push(card); continue; }
    const group = byColumn.get(id) ?? [];
    group.push(card); byColumn.set(id, group);
  }
  return { byColumn, unplaced };
});
const columnCards = (id: string) => cardGroups.value.byColumn.get(id) ?? [];
const unplacedCards = computed(() => cardGroups.value.unplaced);
const visibleSpaces = computed(() => spaces.value.filter(item => showTrash.value ? item.state === "trashed" : item.state !== "trashed"));
const visibleColumns = computed(() => columns.value.filter(item => showTrash.value ? item.state === "trashed" : item.state !== "trashed"));
const groups = computed(() => [
  { label: "Конфликты названий", items: visibleSpaces.value.filter(item => item.names.length !== 1) },
  { label: "Пространства", items: visibleSpaces.value.filter(item => item.names.length === 1) },
].filter(group => group.items.length));
const conflicts = (item: Space | Column) => item.names.length !== 1 || item.state === "conflicted" || "positions" in item && item.positions.length !== 1;

async function loadPending() {
  const current = ++localGeneration;
  try {
    const queue = await pendingCommands(), saved = listDrafts();
    if (current !== localGeneration) return;
    pending.value = queue; drafts.value = saved; storageError.value = false;
  } catch { if (current === localGeneration) storageError.value = true; }
}
async function refresh() {
  const current = ++generation;
  searchRevision.value++;
  if (!props.ready) return;
  loading.value = true;
  error.value = "";
  try {
    // Await both even when one fails; no abandoned read can overwrite a newer view.
    const results = await Promise.allSettled([
      read("space.list", { include_trashed: true }),
      selected.value ? read("space.read", { space: selected.value }) : Promise.resolve(undefined),
      selected.value ? read("column.list", { space: selected.value, include_trashed: true }) : Promise.resolve(undefined),
      selected.value ? read("card.list", { space: selected.value, include_trashed: true }) : Promise.resolve(undefined),
      selected.value ? read("tag.list", { space: selected.value, include_deleted: false }) : Promise.resolve(undefined),
    ]);
    if (current !== generation) return;
    const failure = results.find(result => result.status === "rejected");
    if (failure?.status === "rejected") throw failure.reason;
    const [list, board, children, notes, labels] = results;
    if (list?.status === "fulfilled") spaces.value = list.value!.spaces;
    if (board?.status === "fulfilled") space.value = board.value;
    if (children?.status === "fulfilled") columns.value = children.value?.columns ?? [];
    if (notes?.status === "fulfilled") cards.value = notes.value?.cards ?? [];
    if (labels?.status === "fulfilled") tags.value = labels.value?.tags ?? [];
  } catch (cause) {
    if (current !== generation) return;
    spaces.value = []; space.value = undefined; columns.value = []; cards.value = []; tags.value = [];
    error.value = errorMessage(cause);
  } finally { if (current === generation) loading.value = false; }
}
async function routeChanged() {
  if (dialogOpen.value && editor.value) {
    pendingUrl = location.href;
    history.replaceState(history.state, "", lastUrl);
    void changeDialog(false); return;
  }
  lastUrl = location.href;
  const id = routeSpace();
  if (id === selected.value) return;
  selected.value = id;
  searchOpen.value = false;
  showTrash.value = false;
  showCardTrash.value = false;
  space.value = undefined; columns.value = []; cards.value = []; tags.value = [];
  await refresh();
  await nextTick(); viewHeading.value?.focus();
}
watch(() => props.ready, ready => {
  if (ready) { void refresh(); void loadPending(); }
  else { generation++; loading.value = false; spaces.value = []; space.value = undefined; columns.value = []; cards.value = []; tags.value = []; }
}, { immediate: true });
onMounted(() => {
  window.addEventListener("hashchange", routeChanged, true);
  for (const event of [pendingChanged, draftsChanged, "storage"]) window.addEventListener(event, loadPending);
  void loadPending();
});
onUnmounted(() => {
  generation++; localGeneration++; window.removeEventListener("hashchange", routeChanged, true);
  for (const event of [pendingChanged, draftsChanged, "storage"]) window.removeEventListener(event, loadPending);
});

function editorClosed() {
  dialogOpen.value = false;
  if (pendingUrl) {
    const destination = pendingUrl; pendingUrl = undefined;
    history.replaceState(history.state, "", destination); window.dispatchEvent(new HashChangeEvent("hashchange"));
  }
}
async function changeDialog(value: boolean) {
  if (value) { dialogOpen.value = true; return; }
  if (!editor.value || await editor.value.requestClose()) editorClosed();
}
async function openDraft(item: EditorDraft, event: Event) {
  if (busy.value || moving.value) return;
  opener = event.currentTarget as HTMLElement; busy.value = true;
  const current = generation;
  try {
    const target = await read("card.read", { space: item.space, card: item.card });
    const labels = await read("tag.list", { space: item.space, include_deleted: false });
    if (current !== generation) return;
    editorTags.value = labels.tags;
    action.value = { kind: "card.read", target }; recovering.value = item.id; dialogOpen.value = true;
  } catch (cause) { error.value = errorMessage(cause); }
  finally { busy.value = false; }
}

async function open(next: Action, event: Event) {
  opener = event.currentTarget as HTMLElement;
  if (busy.value || moving.value) return;
  busy.value = true;
  recovering.value = undefined;
  error.value = "";
  const current = generation;
  const source = action.value, wasOpen = dialogOpen.value;
  try {
    tagSubject.value = next.kind.startsWith("tag.") || next.kind.endsWith(".trash") || next.kind.endsWith(".restore") || next.kind === "trash.purge" || next.kind === "history.clear" ? (await mutationCredential()).subject : undefined;
    trashPreview.value = undefined; restoreColumns.value = []; restoreSpace.value = undefined; restoreColumn.value = "";
    purgeConfirmed.value = false; actionRefused.value = false;
    historyPreview.value = undefined;
    scopeTags.value = [...tags.value];
    // Freeze the fresh original version at dialog opening. Refresh never rebases it.
    switch (next.kind) {
      case "column.rename": case "column.trash": case "column.restore":
        next = { kind: next.kind, target: await read("column.read", { space: next.target.space, column: next.target.column }) }; break;
      case "space.rename": case "space.trash": case "space.restore":
        next = { kind: next.kind, target: await read("space.read", { space: next.target.space }) }; break;
      case "trash.purge": {
        const target = await read("space.read", { space: next.target.space });
        trashPreview.value = await read("trash.inspect", { space: target.space, selection: next.selection });
        next = { ...next, target }; break;
      }
      case "history.clear":
        historyPreview.value = await read("history.inspect", { space: next.space, card: next.card }); break;
      case "card.read": case "card.trash": case "card.restore":
        next = { kind: next.kind, target: await read("card.read", { space: next.target.space, card: next.target.card }) }; break;
      case "tag.rename": case "tag.delete":
        next = { kind: next.kind, target: await read("tag.read", { space: next.target.space, tag: next.target.tag }) }; break;
    }
    if (current !== generation || !props.ready || wasOpen && (!dialogOpen.value || action.value !== source)) return;
    if (next.kind === "card.read") {
      const labels = await read("tag.list", { space: next.target.space, include_deleted: false });
      if (current !== generation || !props.ready) return;
      editorTags.value = labels.tags;
    }
    if (next.kind === "card.restore") {
      restoreSpace.value = await read("space.read", { space: next.target.space });
      restoreColumns.value = (await read("column.list", { space: next.target.space, include_trashed: false })).columns.filter(item => item.state === "active" && !conflicts(item));
      restoreColumn.value = restoreColumns.value.find(item => next.kind === "card.restore" && next.target.columns.length === 1 && item.column === next.target.columns[0])?.column ?? "";
    }
    if (current !== generation || !props.ready) return;
    action.value = next; name.value = "target" in next && "names" in next.target ? next.target.names[0] ?? "" : "";
    description.value = "";
    formError.value = ""; dialogOpen.value = true;
  } catch (cause) {
    if (current === generation && (!wasOpen || dialogOpen.value && action.value === source)) error.value = (next.kind === "trash.purge" || next.kind === "history.clear") && cause instanceof Error && cause.message === "unavailable"
      ? "Не удалось прочитать состав очистки. Проверьте связь, обновите список и откройте очистку снова. Действие не отправлено."
      : errorMessage(cause);
  }
  finally { busy.value = false; }
}
function restoreFocus(event: Event) {
  event.preventDefault();
  if (opener?.isConnected && !opener.matches(":disabled")) opener.focus();
  else if (searchOpen.value) searchPanel.value?.focus(); else viewHeading.value?.focus();
}
function initialFocus(event: Event) {
  if (naming.value) { event.preventDefault(); titleInput.value?.focus(); titleInput.value?.select(); }
}
async function retry(id: string) {
  if (busy.value || moving.value) return;
  busy.value = true; error.value = ""; notice.value = "";
  try {
    const original = (await pendingCommands()).find(item => item.id === id);
    const result = await deliverMutation(id);
    const lifecycle = original?.intention.operation.endsWith(".trash") || original?.intention.operation.endsWith(".restore") || original?.intention.operation === "trash.purge" || original?.intention.operation === "history.clear";
    if (lifecycle && result.response?.status === "error" && !["unavailable", "outcome_unknown"].includes(result.response.error.code)) actionRefused.value = true;
    if (result.status === "completed") {
      notice.value = "Действие подтверждено.";
      dialogOpen.value = false;
      if (original?.intention.operation === "trash.purge") {
        const parameters = JSON.parse(original.intention.body).parameters;
        // A purged root no longer has a readable board; reconcile from the list.
        if (selected.value === parameters.space && result.response?.status === "ok" && result.response.result && "includes_space" in result.response.result && result.response.result.includes_space) {
          location.hash = "#/spaces"; await routeChanged();
        } else await refresh();
      } else await refresh();
    } else {
      const message = result.reason === "identity_changed"
        ? "Учётная запись изменилась. Повтор возможен только от исходной учётной записи."
        : result.response?.status === "error" ? (original?.intention.operation.startsWith("tag.") ? tagErrorMessage(result.response.error.code) : errorMessage(new Error(result.response.error.code)))
          : "Результат неизвестен. Повторите исходное действие ниже; новый объект не создаётся.";
      error.value = message; formError.value = message;
      if (actionRefused.value) await refresh();
    }
  } catch { error.value = "Не удалось прочитать или сохранить очередь действий. Сохранённое действие не заменяется новым."; }
  finally { await loadPending(); busy.value = false; }
}
async function submit() {
  const current = action.value;
  if (!current || current.kind === "card.read" || busy.value) return;
  if (current.kind === "forget" || current.kind === "forget-draft") {
    try {
      if (current.kind === "forget-draft") await discardDraft(current.id); else await forgetMutation(current.id);
      await loadPending(); dialogOpen.value = false; notice.value = "Запись убрана. Уже принятое изменение этим не отменяется. Обновите данные перед новым действием.";
    } catch (cause) { formError.value = errorMessage(cause); }
    return;
  }
  if (blocked.value || actionUnavailable.value) { formError.value = "Сначала проверьте незавершённое действие или подготовьте новое действие по актуальному составу."; return; }
  busy.value = true; formError.value = ""; notice.value = "";
  let id: string;
  try {
    switch (current.kind) {
      case "space.create": id = await createMutation(current.kind, { space: newSpaceId(), name: name.value }); break;
      case "column.create":
        id = await createMutation(current.kind, { space: current.space, column: newColumnId(), name: name.value }); break;
      case "card.create":
        id = await createMutation(current.kind, { space: current.space, column: current.column, card: newCardId(), title: name.value, description: description.value }); break;
      case "card.restore": {
        const destination = restoreColumns.value.find(item => item.column === restoreColumn.value);
        if (!destination) throw new Error("stale_state");
        id = await createMutation(current.kind, { space: current.target.space, card: current.target.card, expected: current.target.version,
          column: destination.column, expected_column: destination.version }, tagSubject.value); break;
      }
      case "trash.purge": {
        const preview = trashPreview.value;
        if (!preview || preview.conflicts.length || !purgeConfirmed.value) throw new Error("stale_state");
        id = await createMutation(current.kind, { space: preview.space, selection: preview.selection, expected: preview.version }, tagSubject.value); break;
      }
      case "history.clear": {
        const preview = historyPreview.value;
        if (!preview || !purgeConfirmed.value) throw new Error("stale_state");
        id = await createMutation(current.kind, { space: current.space, card: current.card, expected: preview.version }, tagSubject.value); break;
      }
      case "card.trash":
        id = await createMutation(current.kind, { space: current.target.space, card: current.target.card, expected: current.target.version }, tagSubject.value); break;
      case "space.rename": id = await createMutation(current.kind, { space: current.target.space, expected: current.target.version, name: name.value }); break;
      case "space.trash": case "space.restore": id = await createMutation(current.kind, { space: current.target.space, expected: current.target.version }, tagSubject.value); break;
      case "column.rename": id = await createMutation(current.kind, { space: current.target.space, column: current.target.column, expected: current.target.version, name: name.value }); break;
      case "column.trash": case "column.restore": id = await createMutation(current.kind, { space: current.target.space, column: current.target.column, expected: current.target.version }, tagSubject.value); break;
      case "tag.create": id = await createMutation(current.kind, { space: current.space, tag: newTagId(), name: name.value }, tagSubject.value); break;
      case "tag.rename": id = await createMutation(current.kind, { space: current.target.space, tag: current.target.tag, expected: current.target.version, name: name.value }, tagSubject.value); break;
      case "tag.delete": id = await createMutation(current.kind, { space: current.target.space, tag: current.target.tag, expected: current.target.version }, tagSubject.value); break;
    }
    await loadPending();
  } catch (cause) {
    formError.value = cause instanceof DOMException ? "Не удалось сохранить действие на устройстве. Запрос не отправлен; введённые данные остались в форме." : errorMessage(cause);
    busy.value = false; return;
  }
  busy.value = false;
  await retry(id);
}
function pendingLabel(item: Pending) {
  return labels[item.intention.operation as Mutation] ?? "Незавершённое действие";
}
function pendingName(item: Pending) {
  const parameters: unknown = JSON.parse(item.intention.body).parameters;
  if (!parameters || typeof parameters !== "object") return "";
  if ("title" in parameters && typeof parameters.title === "string") return parameters.title;
  return "name" in parameters && typeof parameters.name === "string" ? parameters.name || "Без названия" : "";
}
</script>

<template>
  <section class="spaces-view" aria-label="Управление пространствами" :aria-busy="loading || busy || moving">
    <div v-if="ready" class="view-toolbar">
      <a v-if="selected" href="#/spaces">Все пространства</a>
      <h2 v-else ref="viewHeading" tabindex="-1">{{ showTrash ? 'Корзина пространств' : 'Мои пространства' }}</h2>
      <div class="view-actions">
        <button :aria-pressed="showTrash" :disabled="busy || moving" @click="showTrash = !showTrash">{{ showTrash ? 'Активные' : 'Корзина' }}</button>
        <button :disabled="loading || busy || moving" @click="refresh">Обновить список</button>
        <button v-if="!selected && !showTrash" class="primary-button" :disabled="blocked" @click="open({ kind: 'space.create' }, $event)">Создать пространство</button>
        <button class="search-toggle" :aria-expanded="searchOpen" :disabled="busy || moving" @click="searchOpen = !searchOpen">{{ searchOpen ? 'Закрыть поиск' : 'Поиск карточек' }}</button>
      </div>
    </div>
    <p v-if="notice" role="status">{{ notice }}</p>
    <p v-if="error" role="alert" class="action-error">{{ error }}</p>
    <p v-if="storageError" role="alert" class="action-error">Не удалось прочитать очередь действий на устройстве. Изменения недоступны, пока хранилище не заработает. <button @click="loadPending">Проверить хранилище</button></p>
    <section v-if="drafts.length" class="pending-panel" aria-label="Черновики карточек">
      <h2>Черновики карточек</h2><p>Это локальный ввод, а не подтверждение сохранения карточки.</p>
      <ul class="pending-list"><li v-for="item in drafts" :key="item.id">
        <span>{{ item.title || 'Без заголовка' }}</span><div class="view-actions">
          <button :disabled="busy || moving || !ready" @click="openDraft(item, $event)">Открыть черновик</button>
          <button :disabled="busy || moving" @click="open({ kind: 'forget-draft', id: item.id }, $event)">Удалить черновик</button>
        </div>
      </li></ul>
    </section>

    <section v-if="pending.length" class="pending-panel" aria-labelledby="pending-title">
      <h2 id="pending-title">Незавершённые действия</h2>
      <p>Успех ещё не подтверждён. Повтор сохраняет исходные данные действия. После перезапуска проверка продолжается здесь.</p>
      <ul class="pending-list"><li v-for="item in pending" :key="item.id">
        <span>{{ pendingLabel(item) }}<template v-if="pendingName(item)">: {{ pendingName(item) }}</template></span>
        <div class="view-actions"><button :disabled="busy || moving || !ready" @click="retry(item.id)">Проверить результат</button>
          <button :disabled="busy || moving" @click="open({ kind: 'forget', id: item.id }, $event)">Убрать из списка</button></div>
      </li></ul>
    </section>

    <SearchPanel v-if="ready && searchOpen" ref="searchPanel" :space="selected" :blocked="busy || moving || dialogOpen" :revision="searchRevision" @open="open" @navigate="searchOpen = false; showTrash = false; showCardTrash = false" />
    <template v-if="ready && !error && !searchOpen">
      <p v-if="loading" role="status">Обновляем данные…</p>
      <template v-if="!selected">
        <p v-if="!loading && !visibleSpaces.length" class="empty-view">{{ showTrash ? 'В корзине нет пространств.' : 'Пространств пока нет. Создайте первое для своих мыслей и дел.' }}</p>
        <template v-for="group in groups" :key="group.label">
        <h3 v-if="group.label === 'Конфликты названий' || groups.length > 1">{{ group.label }}</h3>
        <ul class="space-list" :aria-label="group.label">
          <li v-for="item in group.items" :key="item.space" class="space-row" :data-space="item.space">
            <div class="object-name"><a :href="`#/spaces/${item.space}`">{{ title(item) }}</a>
              <p v-if="conflicts(item)" class="object-note">Есть конфликт. Варианты названия: {{ item.names.map(value => value || 'Без названия').join(' / ') }}. Изменения недоступны до разрешения.</p>
              <p v-else class="object-note">{{ item.state === 'trashed' ? 'В корзине' : item.empty ? 'Пока без колонок' : 'Есть содержимое' }}</p>
            </div>
            <div class="view-actions">
              <button v-if="item.state === 'active'" :disabled="blocked || conflicts(item)" @click="open({ kind: 'space.rename', target: item }, $event)">Переименовать</button>
              <button :disabled="blocked || conflicts(item)" @click="open({ kind: item.state === 'trashed' ? 'space.restore' : 'space.trash', target: item }, $event)">{{ item.state === 'trashed' ? 'Восстановить' : 'В корзину' }}</button>
            </div>
          </li>
        </ul>
        </template>
      </template>
      <template v-else-if="space">
        <div class="board-heading"><h2 ref="viewHeading" tabindex="-1">{{ title(space) }}</h2>
          <p v-if="space.names.length !== 1">Варианты названия: {{ space.names.map(value => value || 'Без названия').join(' / ') }}. Изменения недоступны до разрешения конфликта.</p>
          <p v-if="space.state !== 'active'">{{ space.state === 'trashed' ? 'Пространство в корзине. Для изменений сначала восстановите его.' : 'Состояние пространства конфликтует. Изменения недоступны до разрешения.' }}</p>
          <button v-if="space.state === 'active' && !showTrash" class="primary-button" :disabled="blocked || conflicts(space)" @click="open({ kind: 'column.create', space: space.space }, $event)">Создать колонку</button>
          <button v-if="!showTrash" :aria-pressed="showCardTrash" :disabled="busy || moving" @click="showCardTrash = !showCardTrash">{{ showCardTrash ? 'Активные карточки' : 'Корзина карточек' }}</button>
          <button class="space-history-cleanup" :disabled="blocked" @click="open({ kind: 'history.clear', space: space.space, card: null }, $event)">Объём и очистка истории пространства</button>
        </div>
        <TrashPanel v-if="showTrash" :space="space" :columns="columns" :cards="cards" :tags="tagNames" :blocked="blocked || dialogOpen" @open="open" />
        <section v-if="!showTrash && unplacedCards.length" aria-label="Карточки вне активных колонок">
          <h3>Карточки вне активных колонок</h3><p class="object-note">Положение конфликтует или колонка в корзине. Карточки доступны для просмотра.</p>
          <CardsList :cards="unplacedCards" :tags="tagNames" :blocked="busy" readonly @open="open" />
        </section>
        <p v-if="!showTrash && !loading && !visibleColumns.length" class="empty-view">Колонок пока нет. Создайте колонку для одной темы.</p>
        <details v-if="!showTrash" class="space-tags">
          <summary>Теги пространства ({{ tags.length }})</summary>
          <p class="object-note">Назначайте и снимайте теги в открытой карточке. Переименование сохраняет все назначения.</p>
          <ul class="tag-list" aria-label="Теги пространства">
            <li v-for="tag in tags" :key="tag.tag" :data-tag="tag.tag">
              <span>{{ tag.names.join(' / ') }}</span>
              <p v-if="tag.state !== 'active' || tag.names.length !== 1" class="object-note">Есть конфликт. Изменения тега недоступны до разрешения.</p>
              <div class="view-actions">
                <button class="rename-tag" :disabled="blocked || conflicts(space) || space.state !== 'active' || tag.state !== 'active' || tag.names.length !== 1" @click="open({ kind: 'tag.rename', target: tag }, $event)">Переименовать</button>
                <button class="delete-tag" :disabled="blocked || conflicts(space) || space.state !== 'active' || tag.state !== 'active' || tag.names.length !== 1" @click="open({ kind: 'tag.delete', target: tag }, $event)">Удалить тег</button>
              </div>
            </li>
          </ul>
          <p v-if="!tags.length" class="object-note">Тегов пока нет. Создайте тег для одной темы.</p>
          <button class="create-tag" :disabled="blocked || conflicts(space) || space.state !== 'active'" @click="open({ kind: 'tag.create', space: space.space }, $event)">Создать тег</button>
        </details>
        <BoardView v-if="!showTrash" :key="space.space" :space="space" :columns="visibleColumns" :cards="visibleCards" :tags="tagNames" :blocked="boardBlocked || dialogOpen" :trash="showTrash" :card-trash="showCardTrash" :refresh="refresh" @open="open" @lock="moving = $event" />
      </template>
    </template>

    <DialogRoot :open="dialogOpen" @update:open="changeDialog">
      <DialogPortal><DialogOverlay class="dialog-overlay" />
        <DialogContent class="action-dialog" @open-auto-focus="initialFocus" @close-auto-focus="restoreFocus">
          <div class="dialog-header"><DialogTitle>{{ action ? labels[action.kind] : '' }}</DialogTitle><DialogClose data-workspace-dialog-close aria-label="Закрыть действие">Закрыть</DialogClose></div>
          <DialogDescription v-if="action?.kind === 'forget'">Убирается только запись о проверке результата. Уже принятое изменение не отменяется. При неизвестном результате новое создание может привести к дубликату. Сначала проверьте текущее состояние.</DialogDescription>
          <DialogDescription v-else-if="action?.kind === 'forget-draft'">Удаляется только локальный ввод. Сохранённая карточка и уже отправленные действия останутся.</DialogDescription>
          <DialogDescription v-else-if="action?.kind === 'card.read'">Изменяйте текст и отмечайте выполненные дела.</DialogDescription>
          <DialogDescription v-else-if="action?.kind === 'card.create'">Добавьте мысль или дело в эту колонку. Заголовок обязателен, описание можно оставить пустым.</DialogDescription>
          <DialogDescription v-else-if="action?.kind === 'tag.delete'">Удалить «{{ action.target.names.join(' / ') }}» со всех карточек пространства, включая корзину? Карточки и их текст сохранятся. Тег удаляется без корзины и отдельного восстановления. Новый тег с тем же именем не вернёт назначения.</DialogDescription>
          <DialogDescription v-else-if="action?.kind === 'tag.create' || action?.kind === 'tag.rename'">Имя тега обязательно и должно быть свободно в этом пространстве. Регистр и пробелы по краям не различаются, «е» и «ё» различаются. Переименование сохраняет назначения.</DialogDescription>
          <DialogDescription v-else-if="action?.kind === 'trash.purge'">Окончательная очистка корзины пространства «{{ title(action.target) }}». После очистки обычного восстановления нет. Резервные копии и экспорты сохранятся. Это не гарантирует стирание текста с диска или уменьшение файла базы.</DialogDescription>
          <DialogDescription v-else-if="action?.kind === 'history.clear'">Просмотрите объём и состав истории {{ action.card ? 'карточки' : 'пространства, включая корзину' }}. Очистка начнётся только после вашего подтверждения.</DialogDescription>
          <DialogDescription v-else-if="lifecycleTarget">{{ action?.kind.endsWith('.trash') ? 'Контейнер и его активное содержимое попадут в корзину. Ранее удалённые объекты сохранят свои группы.' : 'Вернутся контейнер и объекты, удалённые вместе с ним. Прежняя корзина сохранится.' }} {{ title(lifecycleTarget) }}</DialogDescription>
          <DialogDescription v-else-if="action?.kind === 'card.restore'">Восстановите карточку «{{ cardTitle(action.target) }}» в активную колонку этого пространства. При выборе другой колонки карточка появится в её конце.</DialogDescription>
          <DialogDescription v-else-if="naming">Название можно изменить позже. Оно сохраняется только после подтверждения ядром.</DialogDescription>
          <DialogDescription v-else>{{ action?.kind.endsWith('.trash') ? 'Объект попадёт в корзину. Его можно будет восстановить с прежним названием и местом.' : 'Объект вернётся из корзины с прежним названием и местом.' }} {{ action && 'target' in action ? 'titles' in action.target ? cardTitle(action.target) : title(action.target) : '' }}</DialogDescription>
          <p v-if="error && action?.kind === 'card.read'" role="alert" class="action-error">{{ error }}</p>
          <CardEditor v-if="action?.kind === 'card.read'" ref="editor" :card="action.target" :tags="editorTags" :ready="ready" :draft-id="recovering" @changed="refresh" @close="editorClosed" @stay="pendingUrl = undefined" @cleanup="(card, event) => open({ kind: 'history.clear', space: card.space, card: card.card }, event)" />
          <form v-else @submit.prevent="submit">
            <template v-if="action?.kind === 'history.clear' && historyPreview">
              <HistoryScope :preview="historyPreview" />
              <label class="trash-choice history-confirmation"><input v-model="purgeConfirmed" type="checkbox" :disabled="busy || pending.length > 0 || actionRefused || !historyPreview.entries.some(entry => !entry.retained)" />Понимаю, что очищенные версии нельзя восстановить через историю</label>
            </template>
            <TrashScope v-if="lifecycleScope && lifecycleTarget" v-bind="lifecycleScope" :lifecycle="lifecycleTarget" />
            <p v-else-if="lifecycleTarget" role="alert" class="action-error">Не удалось определить весь состав. Действие недоступно. Закройте форму и заново прочитайте данные.</p>
            <template v-if="action?.kind === 'trash.purge' && trashPreview">
              <p v-if="trashPreview.includes_space" class="action-error">Будет очищено само пространство «{{ title(action.target) }}» со всем показанным содержимым.</p>
              <TrashScope :columns="trashPreview.columns" :cards="trashPreview.cards" :tags="action.target.contents?.tags" />
              <section v-if="trashPreview.conflicts.length" role="alert" class="action-error"><h3>Очистка остановлена: есть конфликты</h3><ul><li v-for="id in trashPreview.conflicts" :key="id">{{ id }}</li></ul><p>Ни один объект не будет очищен. Сначала разрешите конфликты, затем заново просмотрите состав.</p></section>
              <label v-else class="trash-choice purge-confirmation"><input v-model="purgeConfirmed" type="checkbox" :disabled="busy || pending.length > 0 || actionRefused" />Понимаю, что обычного восстановления после очистки нет</label>
            </template>
            <template v-if="action?.kind === 'card.restore'">
              <p v-if="restoreSpace?.state !== 'active'" role="alert" class="action-error">Сначала восстановите пространство целиком.</p>
              <fieldset v-else-if="restoreColumns.length" class="restore-choices"><legend>Колонка восстановления</legend><label v-for="item in restoreColumns" :key="item.column" class="trash-choice"><input v-model="restoreColumn" type="radio" name="restore-column" :value="item.column" :disabled="busy || pending.length > 0 || actionRefused" required /><span>{{ title(item) }}</span></label></fieldset>
              <p v-else role="alert">Нет активной колонки. Отмените действие и создайте или восстановите колонку в этом пространстве.</p>
            </template>
            <label v-if="naming" class="name-field">{{ action?.kind === 'card.create' ? 'Заголовок' : 'Название' }}<input ref="titleInput" v-model="name" name="name" :aria-label="action?.kind === 'card.create' ? 'Заголовок' : 'Название'" autocomplete="off" :required="action?.kind === 'card.create' || action?.kind.startsWith('tag.')" :readonly="busy || pending.length > 0" aria-describedby="name-help" /><span id="name-help">{{ action?.kind.startsWith('tag.') ? 'Пустое или занятое имя не сохраняется.' : action?.kind === 'card.create' ? 'Пустой заголовок не сохраняется.' : 'Можно оставить пустым.' }}</span></label>
            <label v-if="action?.kind === 'card.create'" class="name-field">Описание<textarea v-model="description" name="description" rows="6" :readonly="busy || pending.length > 0" /></label>
            <p v-if="formError" role="alert" class="action-error">{{ formError }}</p>
            <p v-if="actionRefused">Действие отклонено. Состав выше относится к исходному запросу. Закройте форму, уберите отклонённое действие из списка и подготовьте новое по актуальным данным.</p>
            <p v-if="pending.length && action?.kind !== 'forget'">Исходное действие сохранено. Закройте форму и проверьте его результат в списке незавершённых действий.</p>
            <div class="dialog-actions"><DialogClose type="button">{{ pending.length ? 'К списку действий' : 'Отмена' }}</DialogClose><button type="submit" :class="action?.kind === 'trash.purge' || action?.kind === 'history.clear' ? 'danger-button' : 'primary-button'" :disabled="busy || (action?.kind !== 'forget' && action?.kind !== 'forget-draft' && (blocked || actionUnavailable))">{{ busy ? 'Сохраняем…' : naming && action?.kind.endsWith('.rename') ? 'Сохранить название' : action ? labels[action.kind] : '' }}</button></div>
          </form>
        </DialogContent>
      </DialogPortal>
    </DialogRoot>
  </section>
</template>
