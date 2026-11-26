<script setup lang="ts">
import { ref, watch } from "vue";
import type { components } from "../../api/schema";
import { createMutation, deliverMutation, errorMessage, read } from "./workspace-api";
import { nativeFiles, openNativeArchive, readArchive, saveArchive } from "./snapshot-files";
const props = defineProps<{ blocked: boolean; space?: string }>();
const emit = defineEmits<{ busy: [value: boolean]; changed: [] }>();
const input = ref<HTMLInputElement>(), state = ref<components["schemas"]["RecoveryState"]>();
const incoming = ref(""), outgoing = ref(""), error = ref(""), message = ref("");
const confirmed = ref(false), working = ref(false);
const details = ref<HTMLDetailsElement>();
defineExpose({ selectArchive: async (archive: string) => {
  await work(async space => { incoming.value = archive; await inspect(space); if (details.value) details.value.open = true; message.value = "Ежедневная копия выбрана. Подтвердите восстановление ниже."; });
} });
let generation = 0;
watch(() => props.space, () => { generation++; state.value = undefined; incoming.value = ""; outgoing.value = ""; confirmed.value = false; error.value = ""; message.value = ""; });
async function work(action: (space: string) => Promise<void>) {
  if (working.value || props.blocked || !props.space) return;
  working.value = true; emit("busy", true); error.value = ""; message.value = "";
  try { await action(props.space); } catch (cause) { error.value = cause instanceof Error && cause.message === "invalid_file" ? "Файл пуст, повреждён или превышает 1 MiB. Восстановление не выполнено." : errorMessage(cause); }
  finally { working.value = false; emit("busy", false); }
}
async function inspect(space: string) {
  const current = generation;
  const value = await read("space.recovery.inspect", { space });
  if (current === generation) { state.value = value; confirmed.value = false; }
}
async function choose() {
  if (props.blocked || working.value) return;
  if (!nativeFiles()) { input.value?.click(); return; }
  await work(async space => {
    const archive = await openNativeArchive();
    if (archive !== undefined) { incoming.value = archive; await inspect(space); }
    else message.value = "Выбор отменён. Пространство не изменено.";
  });
}
async function selected(event: Event) {
  const target = event.target as HTMLInputElement, file = target.files?.[0]; target.value = "";
  if (!file) return;
  await work(async space => { incoming.value = await readArchive(file); await inspect(space); });
}
async function safety() {
  await work(async space => {
    await inspect(space);
    if (!state.value?.backup) return;
    incoming.value = (await read("space.recovery.read_backup", { space, safety: state.value.backup.id })).archive;
    message.value = "Выбрано состояние до последнего восстановления. Подтвердите возврат ниже.";
  });
}
async function restore() {
  if (!incoming.value || !state.value || !confirmed.value) return;
  await work(async space => {
    const id = await createMutation("space.recovery.restore", { space, archive: incoming.value, expected: state.value!.version });
    confirmed.value = false;
    const result = await deliverMutation(id);
    if (result.status !== "completed" || result.response.status !== "ok") throw new Error(result.response?.status === "error" ? result.response.error.code : "outcome_unknown");
    incoming.value = "";
    outgoing.value = "";
    await inspect(space);
    message.value = "Пространство восстановлено. Входящий и исходящий обмен на паузе. Проверьте результат перед возобновлением. Состояние до восстановления сохранено.";
    emit("changed");
  });
}
async function pause() {
  if (!state.value) return;
  await work(async space => {
    const id = await createMutation("space.recovery.pause", { space, paused: !state.value!.paused, expected: state.value!.pause_version });
    const result = await deliverMutation(id);
    if (result.status !== "completed" || result.response.status !== "ok") throw new Error(result.response?.status === "error" ? result.response.error.code : "outcome_unknown");
    await inspect(space);
    message.value = state.value!.paused ? "Обмен пространства приостановлен." : "Обмен явно возобновлён. Независимые правки других устройств объединятся или появятся как конфликты.";
  });
}
async function saveSafety() {
  await work(async space => {
    if (!outgoing.value) {
      await inspect(space);
      if (!state.value?.backup) return;
      outgoing.value = (await read("space.recovery.read_backup", { space, safety: state.value.backup.id })).archive;
    }
    const result = await saveArchive(outgoing.value);
    message.value = result === "saved" ? "Страховочный файл сохранён." : result === "cancelled" ? "Сохранение отменено. Подготовленный файл можно сохранить снова." : "Страховочный файл передан браузеру. Проверьте загрузки.";
  });
}
</script>

<template>
  <section class="space-transfer space-recovery" aria-label="Восстановление пространства" :aria-busy="working">
    <details ref="details">
      <summary>Восстановление пространства из снимка</summary>
      <p>Возвращает это пространство к выбранному состоянию. Перед изменением приложение сохранит проверенный страховочный снимок в своей папке копий. При ошибке папки восстановление не начнётся. Другие пространства и права доступа сохраняются.</p>
      <p>Входящий и исходящий обмен остановится до явного возобновления. Независимые правки второго устройства сохранятся при последующем обмене.</p>
      <div class="view-actions">
        <input ref="input" type="file" hidden accept=".zedspace,application/octet-stream" @change="selected" />
        <button :disabled="blocked || working" @click="choose">Выбрать снимок для восстановления</button>
        <button :disabled="blocked || working" @click="work(inspect)">Проверить состояние восстановления</button>
        <button v-if="incoming" :disabled="blocked || working" @click="incoming = ''; confirmed = false">Убрать снимок</button>
      </div>
      <template v-if="state">
        <p role="status">{{ state.paused ? 'Входящий и исходящий обмен на паузе.' : 'Явной паузы обмена нет.' }}</p>
        <button :disabled="blocked || working" @click="pause">{{ state.paused ? 'Возобновить обмен пространства' : 'Приостановить обмен пространства' }}</button>
        <div v-if="state.backup" class="view-actions">
          <button :disabled="blocked || working" @click="safety">Выбрать состояние до восстановления</button>
          <button :disabled="blocked || working" @click="saveSafety">Сохранить страховочный снимок в файл</button>
        </div>
        <p v-if="state.cleanup_pending">Предыдущий страховочный файл ожидает очистки. Последний проверенный снимок сохранён.</p>
      </template>
      <template v-if="incoming && state">
        <p role="status">Снимок выбран. Восстановление ещё не выполнялось.</p>
        <label><input v-model="confirmed" type="checkbox" :disabled="blocked || working" /> Подтверждаю замену состояния этого пространства и паузу обмена.</label>
        <button class="primary-button" :disabled="blocked || working || !confirmed" @click="restore">Подтвердить восстановление пространства</button>
      </template>
      <p v-if="message" role="status">{{ message }}</p>
      <p v-if="error" role="alert">{{ error }} Если действие осталось в списке незавершённых, проверьте его результат там.</p>
    </details>
  </section>
</template>
