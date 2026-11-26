<script setup lang="ts">
import { ref, watch, onBeforeUnmount } from "vue";
import type { components } from "../../api/schema";
import { read, createMutation, deliverMutation, errorMessage } from "./workspace-api";
import { saveArchive } from "./snapshot-files";
const props = defineProps<{ space?: string; blocked: boolean }>();
const emit = defineEmits<{ busy: [value: boolean]; selected: [archive: string] }>();
const state = ref<components["schemas"]["DailyBackups"]>();
const details = ref<HTMLDetailsElement>();
const working = ref(false), error = ref(""), message = ref(""), deleting = ref("");
let generation = 0, timer: ReturnType<typeof setInterval> | undefined;
const labels: Record<components["schemas"]["DailyStatus"], string> = {
  ready: "Изменения ожидают ежедневной копии.", saved: "Ежедневная копия сохранена.", unchanged: "После последней копии изменений нет.",
  daily_limit: "За сегодняшний день копия уже сохранена. Новые изменения попадут в копию следующего дня.",
  waiting: "Копия не сохранена. Автоматический повтор — через 5 минут после ошибки. Можно повторить сейчас.",
  unavailable: "Не удалось сохранить копию. Проверьте доступность папки и свободное место. Прежние копии сохранены.",
  calendar_rollback: "Дата устройства раньше последней успешной копии. Проверьте часы и часовой пояс.", running: "Сохраняем ежедневную копию.",
};
const date = (ms: number) => new Date(ms).toLocaleString();
async function inspect() {
  if (!props.space || working.value || props.blocked) return;
  const current = generation, space = props.space;
  try { const result = await read("space.backups.inspect", { space }); if (generation === current) { state.value = result; error.value = ""; } }
  catch (cause) { if (generation === current) error.value = errorMessage(cause); }
}
function toggle() {
  if (timer) clearInterval(timer); timer = undefined;
  if (details.value?.open) { void inspect(); timer = setInterval(() => { void inspect(); }, 5000); }
}
watch(() => props.space, () => { generation++; state.value = undefined; deleting.value = ""; error.value = ""; message.value = ""; if (details.value?.open) void inspect(); });
onBeforeUnmount(() => { generation++; if (timer) clearInterval(timer); });
async function work(action: (space: string) => Promise<void>) {
  if (!props.space || props.blocked || working.value) return;
  const current = generation;
  working.value = true; emit("busy", true); error.value = ""; message.value = "";
  try { await action(props.space); } catch (cause) { if (current === generation) error.value = errorMessage(cause); }
  finally { working.value = false; emit("busy", false); }
}
async function mutate(operation: "space.backups.retry" | "space.backups.delete", copy?: string) {
  await work(async space => {
    const id = operation === "space.backups.retry"
      ? await createMutation(operation, { space, failed_sequence: state.value!.failed_sequence! })
      : await createMutation(operation, { space, copy: copy! });
    deleting.value = "";
    const result = await deliverMutation(id);
    if (result.status !== "completed" || result.response.status !== "ok") throw new Error(result.response?.status === "error" ? result.response.error.code : "outcome_unknown");
    const value = await read("space.backups.inspect", { space });
    if (props.space === space) {
      state.value = value;
      message.value = operation === "space.backups.delete" ? "Копия удалена из списка. Дневной лимит сохранён." : "status" in result.response.result && result.response.result.status === "saved" ? "Ежедневная копия сохранена." : labels[value.status];
    }
  });
}
async function archive(copy: string, select: boolean) {
  let selected: string | undefined;
  await work(async space => {
    const result = await read("space.backups.read", { space, copy });
    if (props.space !== space) return;
    if (select) { selected = result.archive; return; }
    const saved = await saveArchive(result.archive);
    message.value = saved === "saved" ? "Копия сохранена в файл." : saved === "cancelled" ? "Сохранение отменено. Копия остаётся в приложении." : "Копия передана браузеру. Проверьте загрузки.";
  });
  if (selected) emit("selected", selected);
}
</script>
<template>
  <section class="space-transfer daily-backups" aria-label="Ежедневные копии" :aria-busy="working">
    <details ref="details" @toggle="toggle">
      <summary>Ежедневные резервные копии</summary>
      <p>Пока приложение работает, изменённое пространство сохраняется один раз за календарный день устройства. Храним последние 7 успешных копий. История, корзина и конфликты входят в снимок.</p>
      <button :disabled="blocked || working" @click="inspect">Обновить состояние копий</button>
      <template v-if="state">
        <p role="status">{{ labels[state.status] }}</p>
        <p v-if="state.last_success">Последняя успешная копия: {{ date(state.last_success.created_at_unix_ms) }}</p>
        <p v-if="state.failed_sequence">До автоматического повтора: {{ Math.ceil((state.retry_after_ms || 0) / 1000) }} с. Папка автоматически не меняется.</p>
        <button v-if="state.failed_sequence" :disabled="blocked || working" @click="mutate('space.backups.retry')">Повторить сейчас</button>
        <p v-if="state.cleanup_pending">Удалённые или устаревшие файлы ожидают завершения очистки. Остальные копии сохранены.</p>
        <p v-if="!state.copies.length">Сохранённых ежедневных копий в этой установке пока нет.</p>
        <ul v-else class="daily-copy-list">
          <li v-for="copy in state.copies" :key="copy.id">
            <time :datetime="new Date(copy.created_at_unix_ms).toISOString()">{{ date(copy.created_at_unix_ms) }}</time>
            <div class="view-actions">
              <button :disabled="blocked || working" @click="archive(copy.id, true)">Выбрать для восстановления</button>
              <button :disabled="blocked || working" @click="archive(copy.id, false)">Сохранить копию в файл</button>
              <button :disabled="blocked || working" @click="deleting = copy.id">Удалить эту копию</button>
            </div>
            <div v-if="deleting === copy.id">
              <p>Удалить эту ежедневную копию? Рабочее пространство не изменится. При отсутствии других копий вернуть этот снимок не получится.</p>
              <button :disabled="blocked || working" @click="mutate('space.backups.delete', copy.id)">Подтвердить удаление копии</button>
              <button :disabled="working" @click="deleting = ''">Отмена удаления</button>
            </div>
          </li>
        </ul>
      </template>
      <p v-if="message" role="status">{{ message }}</p>
      <p v-if="error" role="alert">{{ error }} Если действие осталось незавершённым, проверьте его результат в списке действий.</p>
    </details>
  </section>
</template>
