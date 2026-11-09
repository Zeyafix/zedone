<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { VueDraggable, type DraggableEvent } from "vue-draggable-plus";
import { DialogClose, DialogContent, DialogDescription, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from "reka-ui";
import CardsList from "./CardsList.vue";
import { cardTitle, columnConflict, dragOptions, movableCard, objectTitle, type MoveTarget, type Placement } from "./board-order";
import { createMutation, deliverMutation, errorMessage, mutationCredential, read, type Card, type Column, type Space } from "./workspace-api";

const props = defineProps<{ space: Space; columns: Column[]; cards: Card[]; blocked: boolean; trash: boolean; cardTrash: boolean; refresh: () => Promise<void> }>();
const emit = defineEmits<{
  open: [action: { kind: "card.read" | "card.trash" | "card.restore"; target: Card } | { kind: "card.create"; space: string; column: string } | { kind: "column.rename" | "column.trash" | "column.restore"; target: Column }, event: Event];
  lock: [value: boolean];
}>();
const presented = ref<Column[]>([]);
const reset = ref(0), working = ref(false), dragging = ref(false), dialog = ref(false);
const message = ref(""), failure = ref("");
const intent = ref<MoveTarget>(), subject = ref<string>();
const destination = ref(""), before = ref("");
const choices = ref<Column[]>([]), notes = ref<Card[]>([]);
let alive = true;
let opener: HTMLElement | undefined;
let dragged: MoveTarget | undefined;
let dragIdentity: Promise<string | undefined> | undefined;
const enabled = computed(() => !props.trash && !props.cardTrash && !props.blocked && props.space.state === "active" && !columnConflict(props.space));
const writable = (item: Column | Space) => item.state === "active" && !columnConflict(item);
const locked = computed(() => working.value || dragging.value || dialog.value);
watch(locked, value => emit("lock", value), { flush: "sync" });
onBeforeUnmount(() => { alive = false; emit("lock", false); });
watch(() => props.columns, () => { if (!dragging.value) presented.value = [...props.columns]; }, { immediate: true });
const columnCards = (id: string) => props.cards.filter(item => item.columns.length === 1 && item.columns[0] === id);
const neighbours = computed(() => {
  const current = intent.value;
  return current?.kind === "column.move"
    ? choices.value.filter(item => item.column !== current.target.column).map(item => ({ id: item.column, label: objectTitle(item) }))
    : notes.value.filter(item => item.columns.length === 1 && item.columns[0] === destination.value && item.card !== current?.target.card)
      .map(item => ({ id: item.card, label: cardTitle(item) }));
});
watch(destination, () => { before.value = ""; });
function focusTarget(target: MoveTarget | undefined) {
  const selector = target?.kind === "card.move" ? `[data-card="${target.target.card}"] .move-card` : `[data-column="${target?.target.column}"] .move-column`;
  const button = document.querySelector<HTMLButtonElement>(selector);
  if (button && !button.disabled) button.focus({ preventScroll: true });
  else if (opener?.isConnected) opener.focus({ preventScroll: true });
}
function restoreFocus(event: Event) { event.preventDefault(); focusTarget(intent.value); }
async function open(next: MoveTarget, event: Event) {
  if (!enabled.value || locked.value) return;
  working.value = true; failure.value = ""; message.value = ""; opener = event.currentTarget as HTMLElement;
  try {
    // Freeze the identity and fresh version once. Neither refresh nor submit rebases it.
    const credential = await mutationCredential();
    const target: MoveTarget = next.kind === "card.move"
      ? { kind: next.kind, target: await read("card.read", { space: props.space.space, card: next.target.card }) }
      : { kind: next.kind, target: await read("column.read", { space: props.space.space, column: next.target.column }) };
    if (!alive || !enabled.value) return;
    if (target.kind === "card.move" ? !movableCard(target.target) : columnConflict(target.target)) throw new Error("conflict");
    intent.value = target; subject.value = credential.subject;
    choices.value = props.columns.filter(item => item.state === "active" && !columnConflict(item)); notes.value = [...props.cards];
    destination.value = target.kind === "card.move" ? target.target.columns[0]! : target.target.column;
    before.value = ""; dialog.value = true;
  } catch (cause) { failure.value = errorMessage(cause); }
  finally { working.value = false; }
}
async function move(target: MoveTarget, placement: Placement, identity?: string) {
  if (!alive || !enabled.value || working.value) return;
  const fromDialog = dialog.value;
  working.value = true; message.value = ""; failure.value = "";
  try {
    const id = target.kind === "card.move"
      ? await createMutation(target.kind, { space: target.target.space, card: target.target.card, expected: target.target.version, column: placement.column, before: placement.before }, identity)
      : await createMutation(target.kind, { space: target.target.space, column: target.target.column, expected: target.target.version, before: placement.before }, identity);
    const result = await deliverMutation(id);
    if (result.status === "completed") { message.value = "Перемещение сохранено. Вернуть прежнее место можно новым перемещением."; dialog.value = false; }
    else failure.value = result.reason === "identity_changed" ? errorMessage(new Error("identity_changed"))
      : result.response?.status === "error" ? errorMessage(new Error(result.response.error.code))
        : "Перемещение не подтверждено. Проверьте исходное действие в списке незавершённых действий.";
  } catch (cause) {
    failure.value = cause instanceof DOMException ? "Не удалось сохранить действие на устройстве. Запрос не отправлен." : errorMessage(cause);
  } finally {
    // Always reconcile current state; an old receipt is never installed as the board.
    if (alive) await props.refresh();
    working.value = false;
    await nextTick();
    if (alive && fromDialog && !dialog.value) focusTarget(target);
  }
}
function start(event: DraggableEvent, kind: MoveTarget["kind"]) {
  if (!enabled.value || locked.value) return;
  const target = kind === "card.move" ? props.cards.find(item => item.card === event.item.dataset.card) : props.columns.find(item => item.column === event.item.dataset.column);
  if (!target) return;
  dragged = kind === "card.move" ? { kind, target: target as Card } : { kind, target: target as Column };
  dragIdentity = mutationCredential().then(value => value.subject, () => undefined);
  // Keep status geometry unchanged while the pointer is held.
  dragging.value = true;
}
async function end(event: DraggableEvent) {
  const target = dragged; dragged = undefined;
  const identity = dragIdentity; dragIdentity = undefined;
  // VueDraggable restores DOM then schedules its model update. Rebuild only after that update.
  await nextTick();
  presented.value = [...props.columns]; reset.value++;
  dragging.value = false;
  if (!target || !alive || event.newDraggableIndex === undefined || event.from === event.to && event.oldDraggableIndex === event.newDraggableIndex) return;
  const column = target.kind === "card.move" ? event.to.dataset.column : target.target.column;
  if (!column) return;
  const originalSubject = await identity;
  if (!originalSubject) { failure.value = errorMessage(new Error("credential_unavailable")); return; }
  const candidates = target.kind === "card.move"
    ? columnCards(column).filter(item => item.card !== target.target.card).map(item => item.card)
    : props.columns.filter(item => item.column !== target.target.column).map(item => item.column);
  await move(target, { column, before: candidates[event.newDraggableIndex] ?? null }, originalSubject);
}
</script>

<template>
  <p v-if="enabled" class="object-note board-help">Тяните за ручку ⠿; на телефоне сначала удерживайте её. За текст можно прокручивать. Кнопка «Переместить» позволяет выбрать место без жеста.</p>
  <p v-if="message" role="status">{{ message }}</p>
  <p v-if="failure && !dialog" role="alert" class="action-error">{{ failure }}</p>
  <VueDraggable v-model="presented" class="column-board" role="region" aria-label="Колонки" tabindex="0" v-bind="dragOptions"
    direction="horizontal" handle=".column-handle" draggable=".column-panel" filter=".move-locked" :prevent-on-filter="false"
    :disabled="!enabled || working || dialog" @start="start($event, 'column.move')" @end="end">
    <section v-for="item in presented" :key="item.column" class="column-panel" :class="{ 'move-locked': columnConflict(item) }" :data-column="item.column" :aria-label="objectTitle(item)">
      <div class="column-heading"><button v-if="enabled && !columnConflict(item)" class="column-handle drag-handle" :aria-label="`Переместить: ${objectTitle(item)}`" title="Удерживайте для перетаскивания или нажмите, чтобы выбрать место" @click="open({ kind: 'column.move', target: item }, $event)">⠿</button><h3>{{ objectTitle(item) }}</h3></div>
      <p v-if="columnConflict(item) && item.state !== 'trashed'" class="object-note">Есть конфликт. Варианты названия: {{ item.names.map(value => value || 'Без названия').join(' / ') }}. Изменения недоступны до разрешения.</p>
      <p class="object-note">{{ item.state === 'trashed' ? 'В корзине' : item.empty ? 'Пока без карточек' : 'Есть содержимое' }}</p>
      <template v-if="!trash">
        <CardsList :cards="columnCards(item.column)" :blocked="blocked || working || dialog" :readonly="!writable(space) || !writable(item)" :movable="!cardTrash" :column="item.column" :group="`cards-${space.space}`" :reset="reset"
          @open="(action, event) => emit('open', action, event)" @move="(card, event) => open({ kind: 'card.move', target: card }, event)" @start="start($event, 'card.move')" @end="end" />
        <p v-if="cardTrash && !columnCards(item.column).length" class="object-note">В корзине нет карточек этой колонки.</p>
        <button v-if="!cardTrash" class="create-card" :disabled="blocked || locked || !writable(space) || !writable(item)" @click="emit('open', { kind: 'card.create', space: space.space, column: item.column }, $event)">Создать карточку</button>
      </template>
      <div class="column-actions">
        <button v-if="!trash && !cardTrash" class="move-column" :disabled="!enabled || locked || columnConflict(item)" @click="open({ kind: 'column.move', target: item }, $event)">Переместить колонку</button>
        <button v-if="item.state === 'active'" :disabled="blocked || locked || !writable(space) || columnConflict(item)" @click="emit('open', { kind: 'column.rename', target: item }, $event)">Переименовать</button>
        <button :disabled="blocked || locked || !writable(space) || columnConflict(item) || !item.empty" @click="emit('open', { kind: item.state === 'trashed' ? 'column.restore' : 'column.trash', target: item }, $event)">{{ item.state === 'trashed' ? 'Восстановить' : 'В корзину' }}</button>
      </div>
      <p v-if="!item.empty" class="object-note">Действие с содержимым появится в следующем шаге.</p>
    </section>
  </VueDraggable>
  <DialogRoot v-model:open="dialog"><DialogPortal><DialogOverlay class="dialog-overlay" />
    <DialogContent class="action-dialog move-dialog" @close-auto-focus="restoreFocus">
      <div class="dialog-header"><DialogTitle>{{ intent?.kind === 'card.move' ? 'Переместить карточку' : 'Переместить колонку' }}</DialogTitle><DialogClose data-workspace-dialog-close aria-label="Закрыть перемещение">Закрыть</DialogClose></div>
      <DialogDescription>Выберите место для «{{ intent?.kind === 'card.move' ? cardTitle(intent.target) : intent ? objectTitle(intent.target) : '' }}». Вернуть прежнее место можно новым перемещением.</DialogDescription>
      <form @submit.prevent="intent && move(intent, { column: destination, before: before || null }, subject)">
        <label v-if="intent?.kind === 'card.move'" class="name-field">Колонка<select v-model="destination" name="column" :disabled="working || blocked"><option v-for="item in choices" :key="item.column" :value="item.column">{{ objectTitle(item) }}</option></select></label>
        <label class="name-field">Место<select v-model="before" name="before" :disabled="working || blocked"><option value="">В конец</option><option v-for="item in neighbours" :key="item.id" :value="item.id">Перед: {{ item.label }}</option></select></label>
        <p v-if="failure" role="alert" class="action-error">{{ failure }}</p>
        <div class="dialog-actions"><DialogClose type="button">Отмена</DialogClose><button type="submit" class="primary-button" :disabled="!enabled || working">{{ working ? 'Сохраняем…' : 'Переместить' }}</button></div>
      </form>
    </DialogContent>
  </DialogPortal></DialogRoot>
</template>
