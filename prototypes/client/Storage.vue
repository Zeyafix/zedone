<script setup lang="ts">
import { ref } from "vue";
import { storage, type StorageRequest, type StorageResponse } from "./storage";

type Snapshot = NonNullable<StorageResponse["snapshot"]>;
const space = ref<"A" | "B">("A");
const query = ref("");
const busy = ref(false);
const error = ref("");
const result = ref<StorageResponse>();
const snapshot = ref<Snapshot>();

async function run(action: Exclude<StorageRequest["action"], "move_card" | "move_column">) {
  busy.value = true;
  error.value = "";
  result.value = undefined;
  try {
    let request: StorageRequest;
    if (action === "restore") {
      if (!snapshot.value) return;
      request = { action, snapshot: snapshot.value };
    } else if (action === "read") {
      request = { action, space: space.value, query: query.value };
    } else {
      request = { action, space: space.value };
    }
    const response = await storage(request);
    result.value = response;
    if (action === "restore" && response.snapshot) space.value = response.snapshot.space;
    if (action === "snapshot" && response.snapshot) snapshot.value = response.snapshot;
  } catch {
    error.value = "Результат не подтверждён. Прочитайте данные перед повтором изменения.";
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section aria-labelledby="storage-heading">
    <h2 id="storage-heading">Прототип SQLite</h2>
    <p>Только синтетические данные A и B. Снимок можно восстановить после удаления.</p>
    <label for="storage-space">Пространство</label>
    <select id="storage-space" v-model="space" :disabled="busy">
      <option>A</option><option>B</option>
    </select>
    <label for="storage-query">Поиск</label>
    <input id="storage-query" v-model="query" />
    <div>
      <button id="storage-seed" :disabled="busy" @click="run('seed')">Создать набор</button>
      <button id="storage-read" :disabled="busy" @click="run('read')">Прочитать / найти</button>
      <button id="storage-snapshot" :disabled="busy" @click="run('snapshot')">Снимок</button>
      <button id="storage-delete" :disabled="busy" @click="run('delete')">Удалить набор</button>
      <button id="storage-restore" :disabled="busy || !snapshot" @click="run('restore')">Восстановить снимок</button>
    </div>
    <p v-if="busy" id="storage-busy">Ожидание хранилища…</p>
    <p v-if="error" id="storage-error" aria-live="polite">{{ error }}</p>
    <p v-if="result" id="storage-summary" aria-live="polite">{{ result.snapshot ? `Карточек: ${result.snapshot.cards.length}` : 'Пространство отсутствует' }}</p>
    <pre v-if="result" id="storage-result">{{ JSON.stringify(result) }}</pre>
    <details v-if="snapshot">
      <summary>Сохранённый снимок {{ snapshot.space }} (до закрытия формы)</summary>
      <pre id="storage-saved-snapshot">{{ JSON.stringify(snapshot) }}</pre>
    </details>
  </section>
</template>

<style scoped>
section { margin-top: 2rem; }
label, input, select { display: block; margin: 0.5rem 0; }
button { margin: 0.25rem; padding: 0.5rem; }
pre { max-height: 12rem; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
