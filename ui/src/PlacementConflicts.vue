<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { DialogClose, DialogContent, DialogDescription, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from "reka-ui";
import CardsList from "./CardsList.vue";
import { cardTitle, objectTitle } from "./board-order";
import { createMutation, deliverMutation, errorMessage, mutationCredential, read, type Card, type Column } from "./workspace-api";

const props = defineProps<{ cards: Card[]; allCards: Card[]; columns: Column[]; tags: ReadonlyMap<string, string>; blocked: boolean; refresh: () => Promise<void> }>();
const emit = defineEmits<{ open: [action: { kind: "card.read" | "card.trash" | "card.restore"; target: Card }, event: Event]; lock: [value: boolean] }>();
const inspected = ref<Awaited<ReturnType<typeof inspect>>>();
const dialog = ref(false), working = ref(false), choice = ref("");
const failure = ref(""), message = ref(""), subject = ref<string>();
const locked = computed(() => working.value || dialog.value);
let alive = true, opener: HTMLElement | undefined;
watch(locked, value => emit("lock", value), { flush: "sync" });
onBeforeUnmount(() => { alive = false; emit("lock", false); });
const inspect = (card: Card) => read("card.inspect_placements", { space: card.space, card: card.card });
const label = (index: number) => {
  const variant = inspected.value?.choices[index];
  if (!variant) return "";
  const column = props.columns.find(item => item.column === variant.placement.column);
  const before = props.allCards.find(item => item.card === variant.before);
  return `${column ? objectTitle(column) : 'Недоступная колонка'} — ${variant.before ? `перед: ${before ? cardTitle(before) : 'сохранённая карточка'}` : 'в конец'}`;
};
const available = (column: string) => props.columns.some(item => item.column === column && item.state === "active");
async function open(card: Card, event: Event) {
  if (props.blocked || locked.value) return;
  working.value = true; failure.value = ""; message.value = ""; opener = event.currentTarget as HTMLElement;
  try {
    const credential = await mutationCredential();
    const result = await inspect(card);
    if (!alive || props.blocked) return;
    inspected.value = result; subject.value = credential.subject; choice.value = ""; dialog.value = true;
  } catch (cause) { failure.value = errorMessage(cause); }
  finally { working.value = false; }
}
async function save() {
  const current = inspected.value, variant = current?.choices[Number(choice.value)];
  if (!current || !variant || choice.value === "" || working.value || props.blocked) return;
  working.value = true; failure.value = "";
  try {
    const id = await createMutation("card.resolve_placement", { space: current.card.space, card: current.card.card, expected: current.card.version, placement: variant.placement }, subject.value);
    const result = await deliverMutation(id);
    if (result.status === "completed") { dialog.value = false; message.value = "Место сохранено. Вернуть прежнее место можно новым перемещением. Другие конфликты остаются до их разрешения."; }
    else failure.value = result.response?.status === "error" ? errorMessage(new Error(result.response.error.code))
      : result.reason === "identity_changed" ? errorMessage(new Error("identity_changed")) : "Решение не подтверждено. Проверьте исходное действие в списке незавершённых действий.";
  } catch (cause) {
    failure.value = cause instanceof DOMException ? "Не удалось сохранить действие на устройстве. Запрос не отправлен." : errorMessage(cause);
  } finally {
    if (alive) await props.refresh();
    working.value = false;
    await nextTick();
    if (alive && !dialog.value) focusCard();
  }
}
function focusCard() {
  const card = inspected.value?.card.card;
  const target = document.querySelector<HTMLElement>(`[data-card="${card}"] .resolve-card-placement, [data-card="${card}"] .card-title`);
  if (target && !target.hasAttribute("disabled")) target.focus({ preventScroll: true });
  else if (opener?.isConnected) opener.focus({ preventScroll: true });
}
async function restoreFocus(event: Event) {
  event.preventDefault(); await nextTick(); if (!working.value) focusCard();
}
</script>

<template>
  <section v-if="cards.length || message || failure" class="placement-conflicts" aria-label="Конфликты">
    <h3 v-if="cards.length">Конфликты</h3>
    <p v-if="cards.length" class="object-note">Место карточки спорное. Выберите сохранённый вариант; до решения обычные изменения карточки недоступны.</p>
    <p v-if="message" role="status">{{ message }}</p><p v-if="failure && !dialog" role="alert">{{ failure }}</p>
    <CardsList :cards="cards" :tags="tags" :blocked="blocked || locked" readonly resolve-placement @open="(action, event) => emit('open', action, event)" @resolve="open" />
  </section>
  <DialogRoot v-model:open="dialog"><DialogPortal><DialogOverlay class="dialog-overlay" />
    <DialogContent class="action-dialog placement-dialog" @close-auto-focus="restoreFocus">
      <div class="dialog-header"><DialogTitle>Разрешить место карточки</DialogTitle><DialogClose data-workspace-dialog-close aria-label="Закрыть выбор места">Закрыть</DialogClose></div>
      <DialogDescription>Выберите сохранённое место для «{{ inspected ? cardTitle(inspected.card) : '' }}». Колонка и позиция сохранятся вместе. Отмена оставит варианты без изменений.</DialogDescription>
      <form @submit.prevent="save">
        <fieldset :disabled="working || blocked"><legend>Сохранённые места</legend>
          <label v-for="(variant, index) in inspected?.choices" :key="index" class="access-choice"><input v-model="choice" type="radio" name="placement" :value="String(index)" :disabled="!available(variant.placement.column)" />{{ label(index) }}</label>
        </fieldset>
        <p v-if="failure" role="alert" class="action-error">{{ failure }}</p>
        <div class="dialog-actions"><DialogClose type="button">Отмена</DialogClose><button type="submit" class="primary-button" :disabled="working || blocked || choice === ''">{{ working ? 'Сохраняем…' : 'Сохранить решение' }}</button></div>
      </form>
    </DialogContent>
  </DialogPortal></DialogRoot>
</template>
