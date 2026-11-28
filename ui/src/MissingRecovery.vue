<script setup lang="ts">
import { nextTick, onUnmounted, ref, watch } from "vue";
import type { components } from "../../api/schema";
import { createMutation, deliverMutation, errorMessage, mutationCredential, read } from "./workspace-api";
import { nativeFiles, openNativeArchive, readArchive } from "./snapshot-files";

const props = defineProps<{ blocked: boolean; ready: boolean }>();
const emit = defineEmits<{ busy: [value: boolean]; restored: [space: string] }>();
const input = ref<HTMLInputElement>(), details = ref<HTMLDetailsElement>();
const state = ref<components["schemas"]["MissingRecoveryState"]>();
const archive = ref(""), subject = ref(""), confirmed = ref(false), working = ref(false);
const error = ref(""), message = ref("");
let generation = 0;
function clear() { generation++; archive.value = ""; state.value = undefined; subject.value = ""; confirmed.value = false; }

async function work(action: () => Promise<void>) {
  if (working.value || props.blocked || !props.ready) return;
  working.value = true; emit("busy", true); error.value = ""; message.value = "";
  try { await action(); }
  catch (cause) {
    const code = cause instanceof Error ? cause.message : "unavailable";
    confirmed.value = false;
    if (["stale_state", "invalid_request", "invalid_file", "file_read", "identity_changed"].includes(code)) clear();
    error.value = code === "stale_state" ? "Пространство уже существует или состояние изменилось. Для существующего пространства выберите обычное восстановление со страховочным снимком."
      : ["invalid_request", "invalid_file"].includes(code) ? "Файл пуст, повреждён, несовместим или превышает 1 MiB. Восстановление не выполнено."
      : code === "file_read" ? "Не удалось прочитать файл. Выберите доступную копию снова." : errorMessage(cause);
  } finally { working.value = false; emit("busy", false); }
}
async function prepare(value: string) {
  clear();
  const current = generation, identity = (await mutationCredential()).subject;
  const inspected = await read("space.recovery.inspect_missing", { archive: value });
  if (current !== generation || (await mutationCredential()).subject !== identity) throw new Error("identity_changed");
  archive.value = value; state.value = inspected; subject.value = identity;
  if (details.value) details.value.open = true;
}
defineExpose({ selectArchive: async (value: string) => { await nextTick(); await work(() => prepare(value)); } });
async function readyAfterPicker() {
  if (props.ready) return;
  await new Promise<void>((resolve, reject) => {
    const stop = watch(() => props.ready, value => { if (value) { clearTimeout(timer); stop(); resolve(); } });
    const timer = setTimeout(() => { stop(); reject(new Error("unavailable")); }, 10_000);
  });
}
async function choose() {
  if (props.blocked || working.value) return;
  confirmed.value = false;
  if (!nativeFiles()) { input.value?.click(); return; }
  await work(async () => {
    const identity = (await mutationCredential()).subject;
    const value = await openNativeArchive();
    await readyAfterPicker();
    if ((await mutationCredential()).subject !== identity) { clear(); throw new Error("identity_changed"); }
    if (value !== undefined) await prepare(value);
    else message.value = "Выбор отменён. Восстановление не выполнялось.";
  });
}
async function selected(event: Event) {
  const target = event.target as HTMLInputElement, file = target.files?.[0]; target.value = "";
  if (file) await work(async () => { clear(); const value = await readArchive(file); await prepare(value); });
}
async function restore() {
  if (!state.value || !archive.value || !confirmed.value) return;
  await work(async () => {
    const id = await createMutation("space.recovery.restore_missing", { space: state.value!.space, archive: archive.value, expected: state.value!.version }, subject.value);
    confirmed.value = false;
    const result = await deliverMutation(id);
    if (result.status !== "completed" || result.response.status !== "ok") throw new Error(result.response?.status === "error" ? result.response.error.code : "outcome_unknown");
    const space = state.value!.space;
    clear(); message.value = "Пространство восстановлено с прежним ID. Обмен на паузе. Подключите устройства и выдайте права агентам заново; затем отдельно возобновите обмен.";
    emit("restored", space);
  });
}
onUnmounted(clear);
</script>

<template>
  <section class="space-transfer" aria-label="Восстановление после переустановки" :aria-busy="working">
    <details ref="details">
      <summary>Восстановление после переустановки</summary>
      <p>Возвращает отсутствующее пространство с прежним ID, историей и корзиной. Исходный файл сохраняется. Снимок отсутствующего состояния не нужен; другие пространства сохраняются.</p>
      <p>Устройства нужно подключить с подтверждением, а права агентам выдать заново. Входящий и исходящий обмен останется на паузе до отдельного возобновления.</p>
      <input ref="input" type="file" hidden accept=".zedspace,application/octet-stream" @change="selected" />
      <div class="view-actions">
        <button :disabled="blocked || working" @click="choose">Выбрать копию после переустановки</button>
        <button v-if="state" :disabled="blocked || working" @click="clear">Отменить подготовку восстановления</button>
      </div>
      <template v-if="state">
        <p role="status">{{ state.names.join(' / ') }} · {{ state.space }}. Текущее состояние отсутствует; восстановление ещё не выполнялось.</p>
        <label><input v-model="confirmed" type="checkbox" :disabled="blocked || working" /> Подтверждаю восстановление отсутствующего пространства и паузу обмена.</label>
        <button class="primary-button" :disabled="blocked || working || !confirmed" @click="restore">Восстановить отсутствующее пространство</button>
      </template>
      <p v-if="message" role="status">{{ message }}</p>
      <p v-if="error" role="alert">{{ error }} Если действие осталось в списке незавершённых, проверьте его результат там.</p>
    </details>
  </section>
</template>
