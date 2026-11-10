<script setup lang="ts">
import { ref, watch } from "vue";
import { VueDraggable, type DraggableEvent } from "vue-draggable-plus";
import type { Card } from "./workspace-api";
import TagNames from "./TagNames.vue";
import { cardTitle, dragOptions, movableCard } from "./board-order";

const props = defineProps<{ cards: Card[]; tags: ReadonlyMap<string, string>; blocked: boolean; readonly?: boolean; movable?: boolean; column?: string; group?: string; reset?: number }>();
const emit = defineEmits<{
  open: [action: { kind: "card.read" | "card.trash" | "card.restore"; target: Card }, event: Event];
  move: [card: Card, event: Event]; start: [event: DraggableEvent]; end: [event: DraggableEvent];
}>();
const presented = ref<Card[]>([]);
watch(() => [props.cards, props.reset], () => { presented.value = [...props.cards]; }, { immediate: true });
</script>

<template>
  <VueDraggable v-model="presented" tag="ul" class="cards-list" :class="{ 'movable-cards': movable }" aria-label="Карточки" :data-column="column"
    v-bind="dragOptions" :group="group" handle=".card-handle" draggable=".card-note" filter=".move-locked" :prevent-on-filter="false"
    :disabled="!movable || blocked || readonly" @start="emit('start', $event)" @end="emit('end', $event)">
    <li v-for="card in presented" :key="card.card" class="card-note" :class="{ 'move-locked': !movableCard(card) }" :data-card="card.card">
      <button v-if="movable && movableCard(card)" class="card-handle drag-handle" :disabled="blocked || readonly" :aria-label="`Переместить: ${cardTitle(card)}`" title="Удерживайте для перетаскивания или нажмите, чтобы выбрать место" @click="emit('move', card, $event)">⠿</button>
      <button class="card-title" :disabled="blocked" @click="emit('open', { kind: 'card.read', target: card }, $event)">{{ card.titles.length === 1 ? card.titles[0] : 'Конфликт заголовков' }}</button>
      <p v-if="card.descriptions.some(text => text.length)" class="card-excerpt">{{ card.descriptions.join('\n\n') }}</p>
      <p v-if="card.completed.length === 1 && card.completed[0]" class="object-note">Выполнено</p>
      <TagNames :ids="card.tags" :tags="tags" />
      <p v-if="card.state === 'conflicted'" class="object-note">Есть конфликт. Все варианты доступны при открытии; изменения — после разрешения.</p>
      <button v-if="movable" class="move-card" :disabled="blocked || readonly || !movableCard(card)" @click="emit('move', card, $event)">Переместить карточку</button>
      <button v-if="!readonly" :disabled="blocked || card.state === 'conflicted'" @click="emit('open', { kind: card.state === 'trashed' ? 'card.restore' : 'card.trash', target: card }, $event)">{{ card.state === 'trashed' ? 'Восстановить' : 'В корзину' }}</button>
    </li>
  </VueDraggable>
</template>
