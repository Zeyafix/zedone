<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { checkPermissions, requestPermissions, scan, cancel, Format } from "@tauri-apps/plugin-barcode-scanner";
import { invitation, type Invitation } from "./invitation";

const address = ref("http://192.0.2.1:4317");
const code = ref("SYNTHETIC-ONLY");
const result = ref<Invitation>();
const scannedText = ref<string>();
const error = ref("");
const scannerMessage = ref("");
const phase = ref<"idle" | "permission" | "scanning" | "stopping">("idle");
const busy = ref(false);
const scannerBlocked = ref(false);
let generation = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
let scanStarted = false;
let stopPromise: Promise<void> | undefined;

watch([address, code], () => { result.value = undefined; error.value = ""; });

async function inspect(request: Parameters<typeof invitation>[0]) {
  busy.value = true;
  result.value = undefined;
  error.value = "";
  try { result.value = await invitation(request); }
  catch (cause) {
    error.value = cause === "invalid_data"
      ? "Ожидаются непустые синтетические поля address и code."
      : "Не удалось проверить данные. Ввод сохранён — повторите запрос.";
  } finally { busy.value = false; }
}

async function stop(message = "Сканирование отменено. Можно ввести данные вручную.") {
  ++generation; // Late permission/scan replies cannot submit data after cancellation.
  clearTimeout(timer);
  scannerMessage.value = message;
  document.body.classList.remove("qr-scanning");
  if (stopPromise) return stopPromise;
  if (!scanStarted) { phase.value = "idle"; return; }
  phase.value = "stopping";
  stopPromise = (async () => {
    let deadline: ReturnType<typeof setTimeout> | undefined;
    try {
      await Promise.race([
        cancel(),
        new Promise<never>((_, reject) => { deadline = setTimeout(() => reject(new Error("Cancel timeout")), 5000); }),
      ]);
    }
    catch {
      scannerBlocked.value = true;
      scannerMessage.value = "Не удалось подтвердить остановку камеры. Закройте приложение перед новым сканированием. Ручной ввод доступен.";
    }
    finally { clearTimeout(deadline); scanStarted = false; phase.value = "idle"; stopPromise = undefined; }
  })();
  return stopPromise;
}

async function start() {
  const current = ++generation;
  phase.value = "permission";
  scannerMessage.value = "Запрос разрешения камеры… Ручной ввод доступен после отмены.";
  result.value = undefined;
  error.value = "";
  try {
    let permission = await checkPermissions();
    if (current !== generation) return;
    if (permission !== "granted") permission = await requestPermissions();
    if (current !== generation) return;
    if (permission !== "granted") {
      phase.value = "idle";
      scannerMessage.value = "Нет разрешения камеры. Введите данные вручную или разрешите камеру в настройках Android и повторите.";
      return;
    }
    phase.value = "scanning";
    scannerMessage.value = "Наведите камеру на QR. Если распознавание не готово (модель/GMS), отмените и введите данные вручную.";
    document.body.classList.add("qr-scanning");
    scanStarted = true;
    // The plugin only logs ML Kit failures; silence does not mean invalid data.
    timer = setTimeout(() => void stop("QR не прочитан за 20 секунд. Камера или распознавание могут быть недоступны. Введите данные вручную или повторите после готовности модели/GMS."), 20000);
    const scanned = await scan({ formats: [Format.QRCode], windowed: true, cameraDirection: "back" });
    if (current !== generation) return;
    scannedText.value = scanned.content;
    await stop("QR прочитан. Это только синтетические данные; подключения нет.");
    await inspect({ source: "qr", text: scanned.content });
  } catch {
    if (current === generation) await stop("Сканер недоступен. Данные приглашения не проверялись. Введите их вручную или повторите после готовности камеры/модели/GMS.");
  }
}

// Android's permission dialog also hides the WebView; it is not a scan cancellation.
function hidden() { if (document.hidden && phase.value === "scanning") void stop(); }
function leaving() { if (phase.value !== "idle") void stop(); }
onMounted(() => {
  document.addEventListener("visibilitychange", hidden);
  window.addEventListener("pagehide", leaving);
  window.addEventListener("prototype-background", leaving);
});
onUnmounted(() => {
  void stop();
  document.removeEventListener("visibilitychange", hidden);
  window.removeEventListener("pagehide", leaving);
  window.removeEventListener("prototype-background", leaving);
});
</script>

<template>
  <section id="invitation-prototype" aria-labelledby="invitation-title">
    <h2 id="invitation-title">Чтение синтетического приглашения</h2>
    <p>Проверка ввода без подключения, хранения и обмена. Адрес не открывается.</p>
    <div id="scanner-controls" :class="{ active: phase === 'scanning' }">
      <p id="scanner-status" aria-live="polite">{{ scannerMessage || 'Можно использовать ручной ввод.' }}</p>
      <button v-if="isTauri() && phase === 'idle'" id="scan-start" type="button" :disabled="busy || scannerBlocked" @click="start">Сканировать QR</button>
      <button v-if="phase !== 'idle'" id="scan-cancel" type="button" :disabled="phase === 'stopping'" @click="stop()">Отменить сканирование — ручной ввод</button>
      <p v-if="!isTauri()">Камера проверяется в Android APK. На ПК доступен тот же ручной ввод.</p>
    </div>
    <form @submit.prevent="inspect({ source: 'manual', address, code })">
      <fieldset :disabled="busy || phase !== 'idle'">
        <legend>Ручной ввод</legend>
        <label for="invitation-address">Синтетический адрес ПК</label>
        <input id="invitation-address" v-model="address" type="text" />
        <label for="invitation-code">Синтетический одноразовый код</label>
        <input id="invitation-code" v-model="code" type="text" />
        <button id="invitation-submit" type="submit">Проверить данные</button>
        <button id="invitation-clear" type="button" @click="address = ''; code = ''; result = undefined; scannedText = undefined; error = ''; if (!scannerBlocked) scannerMessage = ''">Очистить данные</button>
      </fieldset>
    </form>
    <div v-if="scannedText !== undefined">
      <pre id="invitation-scanned">{{ scannedText }}</pre>
      <button id="invitation-retry-qr" type="button" :disabled="busy || phase !== 'idle'" @click="inspect({ source: 'qr', text: scannedText })">Проверить прочитанный QR повторно</button>
    </div>
    <p v-if="error" id="invitation-error" role="alert">{{ error }}</p>
    <pre v-if="result" id="invitation-result" role="status">{{ JSON.stringify(result, null, 2) }}</pre>
  </section>
</template>

<style>
body.qr-scanning, body.qr-scanning #app { background: transparent !important; }
body.qr-scanning main > :not(#invitation-prototype),
body.qr-scanning #invitation-prototype > :not(#scanner-controls) { visibility: hidden; }
#scanner-controls.active {
  position: fixed; inset: auto 0 0; z-index: 10; padding: 1rem;
  padding-bottom: max(1rem, env(safe-area-inset-bottom)); background: white; color: black;
}
#invitation-prototype input { display: block; box-sizing: border-box; width: 100%; margin: 0.5rem 0; font: inherit; }
#invitation-prototype pre { white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
