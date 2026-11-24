<script setup lang="ts">
import { ref } from "vue";
import { createMutation, deliverMutation, errorMessage } from "./workspace-api";
import { nativeFiles, openNativeArchive, readArchive, saveArchive } from "./snapshot-files";

const props = defineProps<{ blocked: boolean; space?: string }>();
const emit = defineEmits<{ busy: [value: boolean]; changed: [] }>();
const input = ref<HTMLInputElement>();
const details = ref<HTMLDetailsElement>();
const incoming = ref("");
const outgoing = ref("");
const message = ref("");
const error = ref("");
const working = ref(false);
function acceptExport(archive: string) {
  if (details.value) details.value.open = true;
  outgoing.value = archive;
  message.value = "Архив подготовлен. Сохраните файл на устройство.";
}
defineExpose({ acceptExport });
function fileError(cause: unknown) {
  if (cause instanceof Error && cause.message === "file_read") return "Не удалось прочитать выбранный файл. Пространство не создано; выберите доступный файл снова.";
  if (cause instanceof Error && cause.message === "file_save") return "Не удалось подтвердить сохранение файла. Архив остался подготовленным; повторите сохранение. Проверьте папку назначения и удалите незавершённый файл, если он появился.";
  return cause instanceof Error && cause.message === "invalid_file"
    ? "Файл пуст, повреждён, не поддерживается или превышает 1 MiB. Пространство не создано."
    : errorMessage(cause);
}
async function run(action: () => Promise<void>) {
  if (working.value) return;
  working.value = true; emit("busy", true); error.value = ""; message.value = "";
  try { await action(); } catch (cause) { error.value = fileError(cause); }
  finally { working.value = false; emit("busy", false); }
}
async function choose() {
  if (props.blocked) return;
  incoming.value = "";
  if (!nativeFiles()) { input.value?.click(); return; }
  await run(async () => {
    const archive = await openNativeArchive().catch(cause => { throw new Error(cause instanceof Error && cause.message === "invalid_file" ? "invalid_file" : "file_read"); });
    if (archive !== undefined) incoming.value = archive;
    else message.value = "Выбор отменён. Импорт не выполнялся.";
  });
}
async function selected(event: Event) {
  const target = event.target as HTMLInputElement;
  const file = target.files?.[0]; target.value = "";
  if (!file || props.blocked) return;
  incoming.value = "";
  await run(async () => { incoming.value = await readArchive(file).catch(cause => { throw new Error(cause instanceof Error && cause.message === "invalid_file" ? "invalid_file" : "file_read"); }); });
}
async function transfer(kind: "space.export" | "space.import") {
  if (props.blocked || (kind === "space.import" ? !incoming.value : !props.space)) return;
  await run(async () => {
    if (kind === "space.export") outgoing.value = "";
    const id = kind === "space.export"
      ? await createMutation(kind, { space: props.space! })
      : await createMutation(kind, { archive: incoming.value });
    if (kind === "space.import") incoming.value = "";
    const result = await deliverMutation(id);
    if (result.status !== "completed" || result.response.status !== "ok") {
      if (result.response?.status === "error" && result.response.error.code === "invalid_request") throw new Error("invalid_file");
      if (result.status === "pending" && result.reason === "identity_changed") throw new Error("identity_changed");
      throw new Error(result.response?.status === "error" ? result.response.error.code : "unavailable");
    }
    if (kind === "space.export" && "archive" in result.response.result) acceptExport(result.response.result.archive);
    else { message.value = "Независимое пространство импортировано. Оно доступно в списке пространств или в корзине, если было удалено до экспорта."; emit("changed"); }
  });
}
async function save() {
  await run(async () => {
    const result = await saveArchive(outgoing.value).catch(() => { throw new Error("file_save"); });
    message.value = result === "saved" ? "Файл сохранён." : result === "cancelled"
      ? "Сохранение отменено. Архив можно сохранить снова."
      : "Файл передан браузеру для скачивания. Проверьте загрузки; архив можно скачать снова.";
  });
}
</script>

<template>
  <section class="space-transfer" aria-label="Перенос пространства" :aria-busy="working">
    <details ref="details">
    <summary>Перенос пространства</summary>
    <p>Полный архив включает историю и корзину. Импорт создаёт независимую копию без подключений и чужих прав. Текущая передача — до 1 MiB.</p>
    <div class="view-actions">
      <button v-if="space" :disabled="blocked || working" @click="transfer('space.export')">Подготовить экспорт</button>
      <button :disabled="blocked || working" @click="choose">Выбрать файл для импорта</button>
      <input ref="input" type="file" hidden aria-label="Файл пространства" @change="selected" />
      <button v-if="incoming" :disabled="blocked || working" @click="transfer('space.import')">Импортировать выбранный файл</button>
      <button v-if="incoming" :disabled="working" @click="incoming = ''; message = 'Выбранный файл убран. Импорт не выполнялся.'">Убрать выбранный файл</button>
      <button v-if="outgoing" :disabled="working" @click="save">Сохранить архив в файл</button>
      <button v-if="outgoing" :disabled="working" @click="outgoing = ''; message = 'Подготовленный архив убран. Сохранённые файлы остались на устройстве.'">Убрать подготовленный архив</button>
    </div>
    <p v-if="incoming" role="status">Файл выбран. Для создания копии подтвердите импорт.</p>
    <p v-if="message" role="status">{{ message }}</p>
    <p v-if="error" role="alert">{{ error }} Если действие осталось в списке незавершённых, проверьте его результат там.</p>
    </details>
  </section>
</template>
