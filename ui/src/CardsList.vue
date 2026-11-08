<script setup lang="ts">
import type { Card } from "./workspace-api";

defineProps<{ cards: Card[]; blocked: boolean; readonly?: boolean }>();
const emit = defineEmits<{ open: [action: { kind: "card.read" | "card.trash" | "card.restore"; target: Card }, event: Event] }>();
</script>

<template>
  <ul class="cards-list" aria-label="Карточки">
    <li v-for="card in cards" :key="card.card" class="card-note">
      <button class="card-title" :disabled="blocked" @click="emit('open', { kind: 'card.read', target: card }, $event)">{{ card.titles.length === 1 ? card.titles[0] : 'Конфликт заголовков' }}</button>
      <p v-if="card.descriptions.some(text => text.length)" class="card-excerpt">{{ card.descriptions.join('\n\n') }}</p>
      <p v-if="card.completed.length === 1 && card.completed[0]" class="object-note">Выполнено</p>
      <p v-if="card.state === 'conflicted'" class="object-note">Есть конфликт. Все варианты доступны при открытии; изменения — после разрешения.</p>
      <button v-if="!readonly" :disabled="blocked || card.state === 'conflicted'" @click="emit('open', { kind: card.state === 'trashed' ? 'card.restore' : 'card.trash', target: card }, $event)">{{ card.state === 'trashed' ? 'Восстановить' : 'В корзину' }}</button>
    </li>
  </ul>
</template>
