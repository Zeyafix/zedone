<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { VueDraggable, type DraggableEvent } from "vue-draggable-plus";
import { storage } from "./storage";
import { inverse, type Move, type Snapshot, type Target } from "./board";
import MoveDialog from "./MoveDialog.vue";

const snapshot = ref<Snapshot>();
// Disposable presentation for Sortable; only a committed Rust response is authoritative.
const columns = ref<{ id: number; title: string; cards: Snapshot["cards"] }[]>([]);
const busy = ref(false);
const dragging = ref(false);
const ready = ref(false);
const message = ref("");
const undo = ref<Move>();
const dialog = ref(false);
const target = ref<Target>({ kind: "card", id: 1 });
const returnFocus = ref<HTMLElement | null>(null);
const disabled = computed(() => busy.value || !ready.value || dialog.value);

function present(value: Snapshot | null | undefined) {
  snapshot.value = value ?? undefined;
  columns.value = value?.columns.map((title, id) => ({ id, title,
    cards: value.cards.filter(card => card.column === id) })) ?? [];
  ready.value = !!value;
}
async function read(seed = false) {
  if (busy.value || dragging.value) return;
  busy.value = true;
  undo.value = undefined;
  message.value = "";
  try {
    present((await storage(seed ? { action: "seed", space: "A" } : { action: "snapshot", space: "A" })).snapshot);
    message.value = snapshot.value ? "Данные прочитаны." : "Набор A отсутствует. Создайте его кнопкой выше.";
  } catch {
    ready.value = false;
    message.value = "Не удалось прочитать данные. Перечитайте доску перед перемещением.";
  } finally { busy.value = false; }
}
async function move(request: Move) {
  if (busy.value || !ready.value) return;
  busy.value = true;
  undo.value = undefined;
  message.value = "Сохранение…";
  let focusId = request.action === "move_card" ? `card-move-${request.card}` : `column-move-${request.column}`;
  try {
    const response = await storage(request);
    present(response.snapshot);
    if (response.snapshot) undo.value = inverse(request, response.snapshot);
    if (request.action === "move_column") focusId = `column-move-${request.position}`;
    message.value = "Перемещение сохранено.";
  } catch (cause) {
    ready.value = false;
    const reason = cause && typeof cause === "object" && "message" in cause ? String(cause.message) : "Нет ответа хранилища.";
    message.value = "Перемещение не подтверждено. Перечитываем сохранённые данные…";
    try {
      present((await storage({ action: "snapshot", space: "A" })).snapshot);
      message.value = `Перемещение не подтверждено. ${reason} Показаны данные из хранилища; выберите действие заново.`;
    } catch {
      // Hide the tentative order when even reconciliation is unavailable.
      columns.value = [];
      message.value = "Результат сохранения пока неизвестен. Перечитайте доску перед новым действием.";
    }
  } finally {
    busy.value = false;
    await nextTick();
    if (returnFocus.value) {
      (document.getElementById(focusId) ?? document.getElementById("board-read"))?.focus({ preventScroll: true });
      returnFocus.value = null;
    }
  }
}
function open(targetValue: Target, event: Event) {
  target.value = targetValue;
  returnFocus.value = event.currentTarget as HTMLElement;
  dialog.value = true;
}
function dropped(event: DraggableEvent, kind: Target["kind"]) {
  dragging.value = false;
  if (!snapshot.value || event.newDraggableIndex === undefined ||
    (event.from === event.to && event.oldDraggableIndex === event.newDraggableIndex)) return;
  const expected = snapshot.value;
  if (kind === "column") {
    void move({ action: "move_column", expected, column: Number(event.item.dataset.column), position: event.newDraggableIndex });
  } else {
    void move({ action: "move_card", expected, card: Number(event.item.dataset.card),
      column: Number(event.to.dataset.column), position: event.newDraggableIndex });
  }
}
const dragOptions = { animation: 150, delay: 250, delayOnTouchOnly: true, touchStartThreshold: 5,
  forceFallback: true, fallbackOnBody: true, scroll: true, bubbleScroll: true, scrollSensitivity: 50, scrollSpeed: 12 };
</script>

<template>
  <section aria-labelledby="board-heading" class="board-section">
    <h2 id="board-heading">Прототип доски</h2>
    <p>Синтетическое пространство A. Тяните за ручку ⠿; на сенсорном экране сначала удерживайте её. Прокручивайте за текст карточки. Кнопка «Переместить» выполняет то же действие.</p>
    <button id="board-seed" :disabled="busy || dragging" @click="read(true)">Создать / открыть доску A</button>
    <button id="board-read" :disabled="busy || dragging" @click="read()">Перечитать доску</button>
    <button id="board-undo" :disabled="disabled || dragging || !undo" @click="undo && move(undo)">Отменить перемещение</button>
    <p id="board-message" aria-live="polite">{{ message }}</p>
    <p v-if="busy" id="board-busy">Ожидание хранилища…</p>
    <VueDraggable v-model="columns" class="board" v-bind="dragOptions" handle=".column-handle" draggable=".board-column"
      :disabled="disabled" @start="dragging = true" @end="dropped($event, 'column')">
      <section v-for="column in columns" :key="column.id" class="board-column" :data-column="column.id">
        <header>
          <span class="column-handle drag-handle" aria-hidden="true">⠿</span>
          <h3>{{ column.title }}</h3>
          <button :id="`column-move-${column.id}`" :disabled="disabled || dragging" :aria-label="`Переместить колонку ${column.title}`"
            @click="open({ kind: 'column', id: column.id }, $event)">Переместить</button>
        </header>
        <VueDraggable v-model="column.cards" class="card-list" :data-column="column.id" group="prototype-cards"
          v-bind="dragOptions" handle=".card-handle" draggable=".board-card" :disabled="disabled"
          @start="dragging = true" @end="dropped($event, 'card')">
          <article v-for="card in column.cards" :key="card.id" class="board-card" :data-card="card.id">
            <span class="card-handle drag-handle" aria-hidden="true">⠿</span>
            <h4>{{ card.title }}</h4>
            <p>{{ card.text.split('\n')[0] }}</p>
            <button :id="`card-move-${card.id}`" :disabled="disabled || dragging" :aria-label="`Переместить карточку ${card.title}`"
              @click="open({ kind: 'card', id: card.id }, $event)">Переместить</button>
          </article>
        </VueDraggable>
      </section>
    </VueDraggable>
    <MoveDialog v-if="snapshot" v-model:open="dialog" :snapshot="snapshot" :target="target" :return-focus="returnFocus"
      @move="move" @closed="!busy && (returnFocus = null)" />
  </section>
</template>

<style scoped>
.board-section { margin-block: 2rem; }
button { min-height: 44px; margin: .25rem; padding: .5rem; }
.board { display: flex; gap: 1rem; overflow-x: auto; padding: .5rem; border: 1px solid #888; }
.board-column { flex: 0 0 18rem; min-width: 0; border: 1px solid #888; border-radius: .5rem; }
header { padding: .5rem; }
h3, h4 { display: inline; margin: 0; }
.card-list { height: min(28rem, 60vh); min-height: 8rem; overflow-y: auto; padding: .5rem; }
.board-card { border: 1px solid #888; background: Canvas; color: CanvasText; margin-bottom: .5rem; padding: .5rem; border-radius: .25rem; }
.board-card p { overflow-wrap: anywhere; }
.drag-handle { display: inline-flex; align-items: center; justify-content: center; width: 44px; height: 44px; cursor: grab; user-select: none; touch-action: none; }
.sortable-ghost { opacity: .35; }
:focus-visible { outline: 3px solid #567dcc; outline-offset: 2px; }
</style>
