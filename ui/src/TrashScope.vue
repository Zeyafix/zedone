<script setup lang="ts">
import type { Card, Column, Space, Tag } from "./workspace-api";
import { cardTitle, objectTitle } from "./board-order";

defineProps<{ columns: Column[]; cards: Card[]; tags?: Tag[]; lifecycle?: Space | Column }>();
// Presentation only. The original root version, never these lists, goes to the core.
function outcome(item: Card | Column, root?: Space | Column) {
  if (!root) return "Будет очищено";
  if (root.state === "active") return item.state === "active" ? "В корзину вместе с контейнером" : "Останется в прежней корзине";
  return root.deletion_groups.length === 1 && item.deletion_groups.length === 1 && item.deletion_groups[0] === root.deletion_groups[0]
    ? "Восстановится вместе с контейнером" : item.state === "trashed" ? "Останется в прежней корзине" : "Состояние сохранится";
}
</script>

<template>
  <section class="trash-scope" aria-label="Состав действия">
    <h3>Состав действия</h3>
    <p>Колонок: {{ columns.length }}. Карточек: {{ cards.length }}.</p>
    <ul class="scope-list">
      <li v-for="column in columns" :key="column.column" :data-scope-column="column.column">
        <strong>Колонка: {{ objectTitle(column) }}</strong><p class="object-note">{{ outcome(column, lifecycle) }}</p>
        <p v-if="column.state === 'conflicted'" class="action-error">Есть конфликт: {{ column.names.join(' / ') }}</p>
        <p v-if="column.contents == null" class="action-error">Состав колонки недоступен. Ядро проверит всю область и откажет без частичного действия.</p>
      </li>
      <li v-for="card in cards" :key="card.card" :data-scope-card="card.card">
        <strong>{{ cardTitle(card) }}</strong><p class="object-note">{{ outcome(card, lifecycle) }}{{ card.completed.length === 1 && card.completed[0] ? '. Выполнено' : '' }}</p>
        <p class="search-text">{{ card.descriptions.join('\n\n') }}</p>
        <p v-if="card.tags.length" class="object-note">Теги: {{ card.tags.map(id => tags?.find(tag => tag.tag === id)?.names.join(' / ') || 'Тег недоступен').join(', ') }}</p>
        <p v-if="card.state === 'conflicted'" class="action-error">Есть конфликт: {{ card.titles.join(' / ') }}</p>
      </li>
    </ul>
    <p v-if="lifecycle && tags?.length" class="object-note">Теги пространства сохраняются: {{ tags.map(tag => tag.names.join(' / ')).join(', ') }}</p>
  </section>
</template>
