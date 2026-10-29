<script setup lang="ts">
import { ref, watch } from "vue";
import { qr, type QrResponse } from "./qr";
import QrCode from "./QrCode.vue";

const text = ref('{"address":"http://192.0.2.1:4317","code":"SYNTHETIC-ONLY"}');
const code = ref<QrResponse>();
const error = ref("");
const busy = ref(false);
const scale = ref(4);
watch(text, () => { code.value = undefined; error.value = ""; });

async function generate() {
  busy.value = true;
  code.value = undefined;
  error.value = "";
  try {
    code.value = await qr({ text: text.value });
  } catch (cause) {
    error.value = cause === "too_long"
      ? "Данные не помещаются в QR. Ничего не обрезано — исходный текст сохранён."
      : "Не удалось получить QR. Ввод сохранён — повторите запрос.";
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section id="qr-prototype" aria-labelledby="qr-title">
    <h2 id="qr-title">Синтетическое приглашение QR</h2>
    <p>Только проверка отображения. Подключения и обмена данными нет.</p>
    <form @submit.prevent="generate">
      <label for="qr-text">Синтетические данные (доступны и без QR)</label>
      <textarea id="qr-text" v-model="text" :disabled="busy" rows="4"></textarea>
      <button id="qr-generate" type="submit" :disabled="busy">{{ busy ? "Генерация…" : "Показать QR" }}</button>
      <button id="qr-clear" type="button" :disabled="busy" @click="text = ''; code = undefined; error = ''">Очистить QR</button>
    </form>
    <label for="qr-scale">Масштаб QR</label>
    <select id="qr-scale" v-model="scale">
      <option v-for="value in [2, 4, 8]" :key="value" :value="value">{{ value }} пикселя на модуль</option>
    </select>
    <p v-if="error" id="qr-error" role="alert">{{ error }}</p>
    <div v-if="code" id="qr-viewport" tabindex="0" role="region" aria-label="QR с горизонтальной прокруткой">
      <QrCode :code="code" :scale="scale" />
    </div>
  </section>
</template>

<style scoped>
textarea { display: block; width: 100%; box-sizing: border-box; font: inherit; margin: 0.5rem 0; }
select { margin: 0.5rem; max-width: 100%; }
#qr-viewport { max-width: 100%; overflow-x: auto; margin: 1rem 0; }
</style>
