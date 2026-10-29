<script setup lang="ts">
import { computed } from "vue";
import type { QrResponse } from "./qr";

const props = defineProps<{ code: QrResponse; scale: number }>();
const side = computed(() => props.code.modules.length + 8);
// Only numeric coordinates from Rust's matrix, never source text or HTML.
const path = computed(() => props.code.modules.flatMap((row, y) =>
  row.flatMap((dark, x) => dark ? [`M${x + 4},${y + 4}h1v1h-1z`] : [])
).join(""));
</script>

<template>
  <svg xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Синтетический QR"
    :viewBox="`0 0 ${side} ${side}`" :width="side * scale" :height="side * scale"
    shape-rendering="crispEdges">
    <rect :width="side" :height="side" fill="#fff" />
    <path :d="path" fill="#000" />
  </svg>
</template>

<style scoped>
svg { display: block; flex: none; max-width: none; }
</style>
