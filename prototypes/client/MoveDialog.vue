<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { DialogRoot, DialogPortal, DialogOverlay, DialogContent, DialogTitle, DialogDescription, DialogClose } from "reka-ui";
import type { Move, Snapshot, Target } from "./board";

const props = defineProps<{ snapshot: Snapshot; target: Target; returnFocus: HTMLElement | null }>();
const open = defineModel<boolean>("open", { required: true });
const emit = defineEmits<{ move: [move: Move]; closed: [] }>();
const column = ref(0);
const position = ref(0);
const first = ref<HTMLSelectElement>();
let historyEntry = false;
const positions = computed(() => props.target.kind === "column" ? props.snapshot.columns.length :
  props.snapshot.cards.filter(card => card.column === column.value && card.id !== props.target.id).length + 1);

watch(column, () => { position.value = 0; });
watch(open, async value => {
  if (value) {
    column.value = props.target.kind === "card" ? props.snapshot.cards.find(card => card.id === props.target.id)!.column : 0;
    position.value = 0;
    // WebView's system Back navigates this entry before leaving the activity.
    history.pushState(null, "");
    historyEntry = true;
    await nextTick();
    first.value?.focus();
  } else if (historyEntry) {
    historyEntry = false;
    history.back();
  }
});
function back() {
  if (historyEntry) { historyEntry = false; open.value = false; }
}
window.addEventListener("popstate", back);
onBeforeUnmount(() => window.removeEventListener("popstate", back));
function submit() {
  const expected = props.snapshot;
  const move: Move = props.target.kind === "card"
    ? { action: "move_card", expected, card: props.target.id, column: column.value, position: position.value }
    : { action: "move_column", expected, column: props.target.id, position: position.value };
  open.value = false;
  emit("move", move);
}
function focusBack(event: Event) {
  event.preventDefault();
  props.returnFocus?.focus();
  emit("closed");
}
</script>

<template>
  <DialogRoot v-model:open="open">
    <DialogPortal>
      <DialogOverlay class="move-overlay" />
      <DialogContent class="move-dialog" @close-auto-focus="focusBack">
        <DialogTitle>Переместить {{ target.kind === 'card' ? 'карточку' : 'колонку' }}</DialogTitle>
        <DialogDescription>Выберите новое место. «Отмена» закроет окно без перемещения.</DialogDescription>
        <form @submit.prevent="submit">
          <label v-if="target.kind === 'card'">Колонка
            <select id="move-column" ref="first" v-model.number="column">
              <option v-for="(title, index) in snapshot.columns" :key="index" :value="index">{{ title }}</option>
            </select>
          </label>
          <label>Позиция
            <select id="move-position" v-model.number="position">
              <option v-for="index in positions" :key="index" :value="index - 1">{{ index }}</option>
            </select>
          </label>
          <button id="move-save" type="submit">Переместить</button>
          <DialogClose id="move-cancel">Отмена</DialogClose>
        </form>
      </DialogContent>
    </DialogPortal>
  </DialogRoot>
</template>

<style>
.move-overlay { position: fixed; inset: 0; background: #0008; z-index: 10; }
.move-dialog { position: fixed; z-index: 11; top: 50%; left: 50%; transform: translate(-50%, -50%); box-sizing: border-box; width: min(28rem, calc(100% - 2rem)); max-height: calc(100dvh - 2rem); overflow: auto; padding: 1.25rem; background: Canvas; color: CanvasText; border: 1px solid; border-radius: .5rem; font-family: sans-serif; }
.move-dialog label { display: block; margin-block: 1rem; }
.move-dialog select { display: block; width: 100%; padding: .5rem; }
.move-dialog button { min-height: 44px; margin-right: .5rem; }
</style>
