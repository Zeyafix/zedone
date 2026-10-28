<script setup lang="ts">
import { ref } from "vue";
import { echo } from "./echo";

const text = ref("");
const result = ref<string>();
const error = ref("");
const busy = ref(false);

async function send() {
  busy.value = true;
  result.value = undefined;
  error.value = "";
  try {
    const data = await echo({ text: text.value });
    result.value = data.text;
  } catch {
    error.value = "Не удалось получить ответ. Ввод сохранён — повторите запрос.";
  } finally {
    busy.value = false;
  }
}

function clear() {
  text.value = "";
  result.value = undefined;
  error.value = "";
}
</script>

<template>
  <main>
    <h1>Прототип echo</h1>
    <p>Синтетический текст возвращается без хранения.</p>
    <form @submit.prevent="send">
      <label for="text">Текст</label>
      <textarea id="text" v-model="text" :disabled="busy" rows="5"></textarea>
      <button :disabled="busy" type="submit">{{ busy ? "Отправка…" : "Отправить" }}</button>
      <button :disabled="busy" type="button" @click="clear">Очистить</button>
    </form>
    <p v-if="error" role="alert">{{ error }}</p>
    <pre v-if="result !== undefined" role="status" aria-label="Ответ">{{ result }}</pre>
  </main>
</template>

<style scoped>
main { max-width: 36rem; margin: 2rem auto; padding: 0 1rem; font-family: sans-serif; }
textarea { display: block; box-sizing: border-box; width: 100%; margin: 0.5rem 0; font: inherit; }
button { padding: 0.5rem 1rem; margin-right: 0.5rem; }
pre { white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
