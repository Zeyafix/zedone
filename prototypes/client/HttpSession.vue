<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { invoke, isTauri } from "@tauri-apps/api/core";
import type { components } from "./api";

type Request = components["schemas"]["HttpSessionRequest"];
type Status = components["schemas"]["HttpSessionStatus"];
const android = isTauri();
const address = ref("127.0.0.1");
const port = ref(4318);
const write = ref(false);
const status = ref<Status>();
const error = ref("");
const busy = ref(false);
let version = 0;
let refreshTimer: ReturnType<typeof setInterval> | undefined;
let refreshing = false;

async function refreshStatus() {
  if (busy.value || refreshing || !status.value?.endpoint || document.visibilityState !== "visible") return;
  const current = version;
  refreshing = true;
  try {
    const result = await invoke<Status>("http_session", { request: { action: "status" } });
    if (current === version) status.value = result;
  }
  catch { /* Explicit controls report IPC failures; polling must not hide their errors. */ }
  finally { refreshing = false; }
}

async function control(request: Request) {
  const current = ++version;
  busy.value = true;
  error.value = "";
  try {
    const result = await invoke<Status>("http_session", { request });
    if (current === version) status.value = result;
  }
  catch { if (current === version) error.value = "Не удалось выполнить действие. Проверьте адрес, порт и состояние APK; обновите состояние перед повтором."; }
  finally { if (current === version) busy.value = false; }
}
function start() {
  return control({ action: "start", address: address.value, port: port.value, access: write.value ? "write" : "read" });
}
function background() { version++; status.value = undefined; busy.value = false; }
function foreground() { if (document.visibilityState === "visible") void control({ action: "status" }); }
onMounted(() => {
  if (!android) return;
  void control({ action: "status" });
  refreshTimer = setInterval(() => void refreshStatus(), 1000);
  window.addEventListener("prototype-background", background);
  document.addEventListener("visibilitychange", foreground);
});
onUnmounted(() => {
  clearInterval(refreshTimer);
  window.removeEventListener("prototype-background", background);
  document.removeEventListener("visibilitychange", foreground);
});
</script>

<template>
  <section id="http-session" aria-labelledby="http-heading">
    <h2 id="http-heading">Тестовый HTTP-доступ</h2>
    <p>Только синтетическое пространство A. После ухода APK в фон доступ отзывается; для продолжения запустите его заново.</p>
    <p v-if="!android">На ПК используйте локальный режим serve-echo --limited по инструкции прототипа.</p>
    <form v-else @submit.prevent="start">
      <label>IP этого устройства <input id="http-address" v-model="address" :disabled="busy || !!status?.endpoint" required /></label>
      <label>Порт <input id="http-port" v-model.number="port" type="number" min="0" max="65535" :disabled="busy || !!status?.endpoint" required /></label>
      <label><input id="http-write" v-model="write" type="checkbox" :disabled="busy || !!status?.endpoint" /> Разрешить изменения A</label>
      <button id="http-start" :disabled="busy || !!status?.endpoint">Запустить тестовый доступ</button>
      <button id="http-stop" type="button" :disabled="busy" @click="control({ action: 'stop' })">Остановить и отозвать</button>
      <button id="http-status" type="button" :disabled="busy" @click="control({ action: 'status' })">Обновить состояние</button>
    </form>
    <p v-if="error" id="http-error" role="alert">{{ error }}</p>
    <div v-if="status?.endpoint" id="http-active">
      <p v-if="status.recovering" id="http-recovering" role="status">Соединение восстанавливается. Ожидается прежний IP и порт; ключ и права сохранены.</p>
      <p>Адрес: <code id="http-endpoint">{{ status.endpoint }}</code></p>
      <p>Права: {{ status.access === 'write' ? 'Чтение и изменение A' : 'Только чтение A' }}</p>
      <label>Временный ключ <input id="http-token" :value="status.token ?? ''" readonly /></label>
    </div>
    <p v-else id="http-inactive">Тестовый доступ выключен.</p>
  </section>
</template>

<style scoped>
section { margin-top: 2rem; }
label { display: block; margin: 0.5rem 0; }
input:not([type="checkbox"]) { box-sizing: border-box; width: 100%; }
button { margin: 0.25rem; padding: 0.5rem; }
code { overflow-wrap: anywhere; }
</style>
