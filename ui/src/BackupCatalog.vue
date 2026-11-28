<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import type { Parameters } from "../../api/validate-response.js";
import type { components } from "../../api/schema";
import { chooseBackupFolder } from "./backup-folders";
import { createMutation, deliverMutation, errorMessage, pendingChanged, pendingCommands, read, mutationCredential } from "./workspace-api";
import { saveArchive } from "./snapshot-files";
type Folder = components["schemas"]["BackupFolder"];
type Copy = components["schemas"]["CatalogCopy"];
const props = defineProps<{ ready: boolean }>();
const emit = defineEmits<{ selected: [space: string, archive: string, missing: boolean] }>();
const folders = ref<Folder[]>([]), selected = ref("");
const catalog = ref<components["schemas"]["BackupCatalog"]>();
const location = ref<components["schemas"]["BackupLocation"]>();
const busy = ref(false), error = ref(""), message = ref(""), pending = ref(false);
const removing = ref<Copy>(), destination = ref<Folder>();
const acknowledge = ref(false);
const folder = computed(() => folders.value.find(item => item.id === selected.value));
const groups = computed(() => [...new Set(catalog.value?.copies.map(copy => copy.space) ?? [])].map(space => ({ space, copies: catalog.value!.copies.filter(copy => copy.space === space) })));
const blocked = computed(() => !props.ready || busy.value || pending.value);
const date = (ms: number) => new Date(ms).toLocaleString("ru-RU");
let generation = 0, alive = true;
async function queueChanged() {
  try { pending.value = (await pendingCommands()).length > 0; }
  catch { pending.value = true; error.value = "Не удалось прочитать незавершённые действия. Проверьте хранилище устройства перед изменениями."; }
}
async function refresh() {
  const current = generation;
  const [available, currentLocation] = await Promise.all([read("backups.folders", {}), read("backups.location.inspect", {})]);
  if (current !== generation || !props.ready) return;
  folders.value = available.folders; location.value = currentLocation;
  if (!folders.value.some(item => item.id === selected.value)) selected.value = "";
  if (selected.value) await browse();
  else catalog.value = undefined;
}
async function work(action: () => Promise<void>) {
  if (busy.value || !props.ready) return;
  busy.value = true; error.value = ""; message.value = "";
  try { await action(); } catch (cause) {
    const code = cause instanceof Error ? cause.message : "unavailable";
    const messages: Record<string, string> = {
      access_denied: "Нет действующего разрешения на эту папку или действие. Выберите папку снова с нужными правами.",
      unavailable: "Папка или копия недоступна. Проверьте доступ и обновите каталог. Рабочие пространства сохранены.",
      invalid_request: "Папка или копия не подходит для этого действия. Выберите другую папку или обновите каталог.",
    };
    error.value = messages[code] ?? errorMessage(cause);
  }
  finally { busy.value = false; }
}
async function readyAfterPicker(): Promise<boolean> {
  if (!alive) return false;
  if (props.ready) return true;
  return new Promise(resolve => {
    const stop = watch(() => props.ready, value => { if (value) { clearTimeout(timer); stop(); resolve(alive); } });
    const timer = setTimeout(() => { stop(); resolve(false); }, 10_000);
  });
}
async function choose(writable: boolean, useDefault = false) {
  await work(async () => {
    const identity = (await mutationCredential()).subject;
    const reply = await chooseBackupFolder(writable, useDefault);
    if (!await readyAfterPicker()) throw new Error("unavailable");
    if ((await mutationCredential()).subject !== identity) throw new Error("identity_changed");
    const current = generation;
    await refresh();
    if (current !== generation || !props.ready || !alive) return;
    if (reply.status === "cancelled") { message.value = "Выбор папки отменён."; return; }
    if (writable) { destination.value = reply.folder; acknowledge.value = false; }
    else { selected.value = reply.folder.id; await browse(); }
  });
}
async function browse() {
  const current = generation, id = selected.value;
  catalog.value = undefined; removing.value = undefined;
  if (id) {
    const result = await read("backups.catalog", { folder: id });
    if (current === generation && props.ready && id === selected.value) catalog.value = result;
  }
}
async function mutate<O extends "backups.location.change" | "backups.location.cleanup" | "backups.catalog.delete" | "backups.folder.forget">(operation: O, parameters: Parameters[O]) {
  if (pending.value) throw new Error("pending_exists");
  const id = await createMutation(operation, parameters);
  const result = await deliverMutation(id);
  if (result.status !== "completed" || result.response.status !== "ok") throw new Error(result.response?.status === "error" ? result.response.error.code : "outcome_unknown");
  removing.value = undefined; destination.value = undefined; acknowledge.value = false;
  await refresh(); await queueChanged();
}
async function change() {
  if (!destination.value || !location.value || !acknowledge.value) return;
  await work(async () => { await mutate("backups.location.change", { folder: destination.value!.id, expected: location.value!.version }); message.value = "Папка копий изменена. Существующие копии перенесены и проверены."; });
}
async function remove() {
  if (!removing.value || !acknowledge.value) return;
  await work(async () => { await mutate("backups.catalog.delete", { folder: selected.value, copy: removing.value!.id, expected: removing.value!.digest }); message.value = "Копия удалена из каталога. Рабочие данные сохранены."; });
}
async function useCopy(copy: Copy, restore: boolean) {
  const current = generation;
  await work(async () => {
    const value = await read("backups.catalog.read", { folder: selected.value, copy: copy.id, expected: copy.digest });
    if (current !== generation || !props.ready) return;
    if (restore) {
      let missing = true;
      try { await read("space.recovery.inspect_missing", { archive: value.archive }); }
      catch (cause) {
        if (!(cause instanceof Error) || cause.message !== "stale_state") throw cause;
        missing = false; await read("space.recovery.inspect", { space: copy.space });
      }
      if (current === generation && props.ready) emit("selected", copy.space, value.archive, missing);
    }
    else { const saved = await saveArchive(value.archive); message.value = saved === "cancelled" ? "Сохранение файла отменено." : "Копия передана для сохранения в файл."; }
  });
}
watch(() => props.ready, value => { generation++; if (value) void work(refresh); else { folders.value = []; catalog.value = undefined; location.value = undefined; removing.value = undefined; destination.value = undefined; } });
watch(selected, () => { void work(browse); });
onMounted(() => { window.addEventListener(pendingChanged, queueChanged); void queueChanged().catch(() => { pending.value = true; }); if (props.ready) void work(refresh); });
onUnmounted(() => { alive = false; generation++; window.removeEventListener(pendingChanged, queueChanged); });
</script>

<template>
  <section class="settings-section backup-catalog" aria-labelledby="backup-catalog-title" :aria-busy="busy">
    <h2 id="backup-catalog-title">Резервные копии</h2>
    <p>Выберите папку, чтобы увидеть сохранённые пространства и даты. Просмотр работает без прежней базы и не восстанавливает данные.</p>
    <div class="view-actions">
      <button :disabled="blocked" @click="choose(false)">Открыть папку для просмотра</button>
      <button :disabled="blocked" @click="choose(false, true)">Посмотреть папку по умолчанию</button>
      <button :disabled="blocked" @click="work(refresh)">Обновить каталог и папку</button>
    </div>
    <template v-if="folders.length">
      <label for="backup-folder">Разрешённая папка для просмотра</label>
      <select id="backup-folder" v-model="selected" :disabled="blocked"><option value="">Выберите папку</option><option v-for="item in folders" :key="item.id" :value="item.id">{{ item.label }}{{ item.writable ? '' : ' — только чтение' }}</option></select>
      <button v-if="selected" :disabled="blocked" @click="work(() => mutate('backups.folder.forget', { folder: selected }))">Убрать разрешение этой папки</button>
    </template>
    <p v-if="pending" role="status">Есть незавершённое действие. Проверьте его результат в разделе «Пространства», затем обновите каталог.</p>
    <p v-if="message" role="status">{{ message }}</p>
    <p v-if="error" role="alert">{{ error }}</p>
    <template v-if="catalog">
      <p v-if="!catalog.copies.length">Исправные копии в этой папке не найдены.</p>
      <p v-if="catalog.unreadable" role="status">Не удалось прочитать копий: {{ catalog.unreadable }}. Повреждённые или незавершённые файлы не изменены.</p>
      <section v-for="group in groups" :key="group.space" class="catalog-space" :aria-label="group.copies[0]?.names.join(' / ') || 'Пространство без названия'">
        <h3>{{ group.copies[0]?.names.join(' / ') || 'Без названия' }}</h3>
        <p class="catalog-id">{{ group.space }}</p>
        <ul class="daily-copy-list">
          <li v-for="copy in group.copies" :key="copy.id">
            <p><time :datetime="new Date(copy.created_at_unix_ms).toISOString()">{{ date(copy.created_at_unix_ms) }}</time> · {{ copy.kind === 'daily' ? 'Ежедневная' : copy.kind === 'migration' ? 'Перед миграцией' : 'Перед восстановлением' }}</p>
            <div class="view-actions">
              <button :disabled="blocked" @click="useCopy(copy, false)">Сохранить копию в файл</button>
              <button :disabled="blocked" @click="useCopy(copy, true)">Подготовить восстановление</button>
              <button v-if="folder?.writable && copy.kind !== 'migration'" :disabled="blocked" @click="removing = copy; destination = undefined; acknowledge = false">Удалить эту сохранённую копию</button>
            </div>
            <div v-if="removing?.id === copy.id" class="catalog-confirmation">
              <p>Удалить эту копию отдельно от рабочего пространства? Последняя копия может быть единственным способом восстановить очищенное пространство.</p>
              <label><input v-model="acknowledge" type="checkbox" :disabled="blocked" /> Подтверждаю удаление выбранной копии</label>
              <div class="view-actions"><button :disabled="blocked || !acknowledge" @click="remove">Подтвердить удаление сохранённой копии</button><button :disabled="busy" @click="removing = undefined">Отмена удаления копии</button></div>
            </div>
          </li>
        </ul>
      </section>
    </template>
    <div class="catalog-location">
      <h3>Папка создания копий</h3>
      <p v-if="location">{{ location.folder }}{{ location.is_default ? ' (по умолчанию)' : '' }}</p>
      <p>Сначала копируем и проверяем ежедневные и страховочные копии, затем переключаем папку и удаляем исходники. Просмотр другой папки эту настройку не меняет.</p>
      <div class="view-actions"><button :disabled="blocked" @click="choose(true)">Сменить папку копий</button><button :disabled="blocked" @click="choose(true, true)">Вернуть папку по умолчанию</button></div>
      <template v-if="destination">
        <p>Новая папка: {{ destination.label }}</p>
        <label><input v-model="acknowledge" type="checkbox" :disabled="blocked" /> Перенести существующие копии в эту папку</label>
        <div class="view-actions"><button :disabled="blocked || !acknowledge" @click="change">Подтвердить смену папки</button><button :disabled="busy" @click="destination = undefined">Отмена смены папки</button></div>
      </template>
      <template v-if="location?.cleanup_pending"><p role="status">Проверенные копии сохранены. Удаление исходных файлов ещё не завершено; новое расположение остаётся действующим.</p><button :disabled="blocked" @click="work(() => mutate('backups.location.cleanup', { expected: location!.version }))">Завершить удаление исходных файлов</button></template>
    </div>
  </section>
</template>

<style scoped>
.backup-catalog select { display: block; max-width: 100%; width: 100%; margin: .6rem 0; }
.catalog-space, .catalog-location { margin-top: 1.5rem; padding-top: 1rem; border-top: 1px solid var(--border); }
.catalog-id { overflow-wrap: anywhere; font-size: .85em; }
.catalog-location p { overflow-wrap: anywhere; }
.catalog-confirmation { padding: 1rem; border: 1px solid var(--border); }
.backup-catalog p[role="alert"] { color: var(--danger); }
</style>
