<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { DialogClose, DialogContent, DialogDescription, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from "reka-ui";
import { objectTitle } from "./board-order";
import { createMutation, deliverMutation, errorMessage, mutationCredential, read, type Column } from "./workspace-api";

const props = defineProps<{ columns: Column[]; blocked: boolean; refresh: () => Promise<void> }>();
const emit = defineEmits<{ lock: [value: boolean] }>();
const inspected = ref<Awaited<ReturnType<typeof inspect>>>();
const dialog = ref(false), working = ref(false), choice = ref("");
const failure = ref(""), message = ref(""), subject = ref<string>();
const locked = computed(() => working.value || dialog.value);
let alive = true, opener: HTMLElement | undefined;
watch(locked, value => emit("lock", value), { flush: "sync" });
onBeforeUnmount(() => { alive = false; emit("lock", false); });
const inspect = (column: Column) => read("column.inspect_positions", { space: column.space, column: column.column });
const label = (index: number) => {
  const variant = inspected.value?.choices[index];
  if (!variant) return "";
  const before = props.columns.find(item => item.column === variant.before);
  return variant.before ? `Перед: ${before ? objectTitle(before) : 'сохранённая колонка'}` : 'В конец';
};
async function open(column: Column, event: Event) {
  if (props.blocked || locked.value) return;
  working.value = true; failure.value = ""; message.value = ""; opener = event.currentTarget as HTMLElement;
  try {
    const credential = await mutationCredential();
    const result = await inspect(column);
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
    const id = await createMutation("column.resolve_position", { space: current.column.space, column: current.column.column, expected: current.column.version, position: variant.position }, subject.value);
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
    if (alive && !dialog.value) focusColumn();
  }
}
function focusColumn() {
  const column = inspected.value?.column.column;
  const target = document.querySelector<HTMLElement>(`[data-column="${column}"] .resolve-column-position:not(:disabled), [data-column="${column}"] .move-column:not(:disabled)`)
    ?? document.querySelector<HTMLElement>(`[data-column="${column}"] h3`);
  if (target && !target.hasAttribute("disabled")) target.focus({ preventScroll: true });
  else if (opener?.isConnected) opener.focus({ preventScroll: true });
}
async function restoreFocus(event: Event) {
  event.preventDefault(); await nextTick(); if (!working.value) focusColumn();
}
defineExpose({ open });
</script>

<template>
  <p v-if="message" role="status">{{ message }}</p><p v-if="failure && !dialog" role="alert">{{ failure }}</p>
  <DialogRoot v-model:open="dialog"><DialogPortal><DialogOverlay class="dialog-overlay" />
    <DialogContent class="action-dialog placement-dialog" @close-auto-focus="restoreFocus">
      <div class="dialog-header"><DialogTitle>Разрешить позицию колонки</DialogTitle><DialogClose data-workspace-dialog-close aria-label="Закрыть выбор места">Закрыть</DialogClose></div>
      <DialogDescription>Выберите сохранённое место для «{{ inspected ? objectTitle(inspected.column) : '' }}». Сохранится выбранная позиция. Карточки останутся в колонке. Отмена оставит варианты без изменений.</DialogDescription>
      <form @submit.prevent="save">
        <fieldset :disabled="working || blocked"><legend>Сохранённые места</legend>
          <label v-for="(variant, index) in inspected?.choices" :key="index" class="access-choice"><input v-model="choice" type="radio" name="placement" :value="String(index)" /><span>Вариант {{ index + 1 }}: <span>{{ label(index) }}</span></span></label>
        </fieldset>
        <p v-if="failure" role="alert" class="action-error">{{ failure }}</p>
        <div class="dialog-actions"><DialogClose type="button">Отмена</DialogClose><button type="submit" class="primary-button" :disabled="working || blocked || choice === ''">{{ working ? 'Сохраняем…' : 'Сохранить решение' }}</button></div>
      </form>
    </DialogContent>
  </DialogPortal></DialogRoot>
</template>
