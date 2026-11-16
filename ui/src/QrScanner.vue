<script lang="ts">
import { checkPermissions, requestPermissions, scan, cancel, Format } from "@tauri-apps/plugin-barcode-scanner";
const driver = { checkPermissions, requestPermissions, cancel,
  scan: () => scan({ formats: [Format.QRCode], windowed: true, cameraDirection: "back" }),
};
</script>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { createQrScanner, type ScannerState } from "./qr-scanner";

const props = defineProps<{ disabled: boolean }>();
const emit = defineEmits<{ read: [text: string] }>();
const native = isTauri();
const state = ref<ScannerState>({ phase: "idle", permissionPending: false, blocked: false, message: "" });
function transparent(active: boolean) {
  document.documentElement.classList.toggle("qr-scanning", active);
  document.body.classList.toggle("qr-scanning", active);
}
const scanner = createQrScanner(driver, value => { state.value = value; transparent(value.phase === "scanning"); }, text => {
  if (!props.disabled) emit("read", text);
});
watch(() => props.disabled, disabled => { if (disabled && (state.value.phase !== "idle" || state.value.permissionPending)) void scanner.stop(); });
function hidden() { if (document.hidden && state.value.phase === "scanning") void scanner.stop(); }
function leaving() { if (state.value.phase !== "idle" || state.value.permissionPending) void scanner.stop(); }
onMounted(() => {
  document.addEventListener("visibilitychange", hidden);
  window.addEventListener("pagehide", leaving);
  window.addEventListener("zedone-background", leaving);
});
onUnmounted(() => {
  transparent(false); void scanner.dispose();
  document.removeEventListener("visibilitychange", hidden);
  window.removeEventListener("pagehide", leaving);
  window.removeEventListener("zedone-background", leaving);
});
</script>

<template>
  <div v-if="native" class="qr-scanner-controls" :class="{ active: state.phase === 'scanning' }">
    <p v-if="state.message" role="status" data-scanner-status>{{ state.message }}</p>
    <button v-if="state.phase === 'idle'" id="device-scan-start" type="button" :disabled="disabled || state.blocked || state.permissionPending" @click="scanner.start">Сканировать QR</button>
    <button v-else id="device-scan-cancel" type="button" :disabled="state.phase === 'stopping'" @click="scanner.stop()">Отменить сканирование — ручной ввод</button>
  </div>
</template>
