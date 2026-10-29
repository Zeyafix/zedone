<script setup lang="ts">
import { ref } from "vue";
import type { RemoteImage } from "./markdown";

defineProps<{ image: RemoteImage }>();
const active = ref(false);
const loaded = ref(false);
const failed = ref(false);
const attempt = ref(0);
function load() {
  failed.value = false;
  loaded.value = false;
  active.value = true;
  attempt.value++;
}
function hide() {
  active.value = false;
  loaded.value = false;
  failed.value = false;
}
</script>

<template>
  <span class="remote-image">
    <span>{{ image.alt || 'Изображение' }}</span>
    <template v-if="image.url">
      <button v-if="!active || failed" type="button" @click="load">
        {{ failed ? 'Повторить загрузку' : 'Загрузить изображение' }}
      </button>
      <button v-if="active" type="button" @click="hide">Скрыть изображение</button>
      <span v-if="active && !loaded && !failed" aria-live="polite">Загрузка…</span>
      <span v-if="failed" aria-live="polite">Не удалось загрузить изображение: сеть недоступна, адрес не отвечает или формат не поддерживается. Можно повторить загрузку.</span>
      <img v-if="active && !failed" :key="attempt" :src="image.url" :alt="image.alt" :title="image.title"
        referrerpolicy="no-referrer" @load="loaded = true" @error="failed = true" />
    </template>
    <span v-else> — Этот адрес изображения не поддерживается в прототипе.</span>
  </span>
</template>

<style scoped>
.remote-image { display: inline-block; max-width: 100%; padding: 0.25rem; border: 1px solid #888; }
button { margin: 0.25rem; padding: 0.5rem; }
img { display: block; max-width: 100%; height: auto; }
</style>
