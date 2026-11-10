<script setup lang="ts">
defineProps<{ ids: string[]; tags: ReadonlyMap<string, string> }>();
</script>

<template>
  <ul v-if="ids.length" class="tag-chips" aria-label="Теги карточки">
    <li v-for="id in ids" :key="id" :data-tag="id">{{ tags.get(id) || 'Тег недоступен' }}</li>
  </ul>
</template>
