<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { Card, Column, Space, TrashSelection } from "./workspace-api";
import { cardTitle, columnConflict, objectTitle } from "./board-order";
import TagNames from "./TagNames.vue";

const props = defineProps<{ space: Space; columns: Column[]; cards: Card[]; tags: ReadonlyMap<string, string>; blocked: boolean }>();
const emit = defineEmits<{
  open: [action: { kind: "card.read" | "card.restore"; target: Card } | { kind: "column.restore"; target: Column } | { kind: "space.restore"; target: Space } | { kind: "trash.purge"; target: Space; selection: TrashSelection }, event: Event];
}>();
const selectedColumns = ref<string[]>([]), selectedCards = ref<string[]>([]);
const columns = computed(() => props.columns.filter(item => item.state !== "active"));
const cards = computed(() => props.cards.filter(item => item.state !== "active"));
watch(() => [props.space, props.columns, props.cards], () => { selectedColumns.value = []; selectedCards.value = []; });
const selection = computed<TrashSelection>(() => ({ kind: "objects", columns: [...selectedColumns.value], cards: [...selectedCards.value] }));
const parentBlocked = computed(() => props.blocked || props.space.state !== "active" || columnConflict(props.space));
</script>

<template>
  <section class="trash-panel" aria-label="Корзина пространства">
    <h3>Корзина пространства</h3>
    <p>Объекты хранятся здесь до ручной очистки. Восстановление контейнера возвращает только объекты, удалённые вместе с ним.</p>
    <p v-if="space.state === 'trashed'">Сначала восстановите пространство целиком. Ранее удалённые объекты останутся в корзине.</p>
    <div class="view-actions">
      <button v-if="space.state === 'trashed'" :disabled="blocked || columnConflict(space)" @click="emit('open', { kind: 'space.restore', target: space }, $event)">Восстановить пространство</button>
      <button class="purge-selected" :disabled="blocked || !selectedColumns.length && !selectedCards.length" @click="emit('open', { kind: 'trash.purge', target: space, selection }, $event)">Очистить выбранное</button>
      <button class="purge-all" :disabled="blocked || space.state !== 'trashed' && !columns.length && !cards.length" @click="emit('open', { kind: 'trash.purge', target: space, selection: { kind: 'all' } }, $event)">Очистить всю корзину</button>
      <button v-if="space.state === 'trashed'" class="purge-space" :disabled="blocked" @click="emit('open', { kind: 'trash.purge', target: space, selection: { kind: 'space' } }, $event)">Очистить пространство</button>
    </div>
    <p v-if="!columns.length && !cards.length" class="empty-view">В корзине нет колонок и карточек.</p>
    <section v-for="column in columns" :key="column.column" class="column-panel trash-column" :data-column="column.column" :aria-label="objectTitle(column)">
      <label class="trash-choice"><input v-model="selectedColumns" type="checkbox" :value="column.column" :disabled="blocked" :aria-label="`Выбрать колонку: ${objectTitle(column)}`" /><span>Колонка: {{ objectTitle(column) }}</span></label>
      <p class="object-note">При очистке включаются вложенные карточки, удалённые раньше. Состав будет показан перед подтверждением.</p>
      <p v-if="columnConflict(column)" class="action-error">Есть конфликт. Просмотрите состав перед новым действием.</p>
      <div class="column-actions"><button :disabled="parentBlocked || columnConflict(column)" @click="emit('open', { kind: 'column.restore', target: column }, $event)">Восстановить</button></div>
    </section>
    <ul class="scope-list">
      <li v-for="card in cards" :key="card.card" class="trash-card" :data-trash-card="card.card">
        <label class="trash-choice"><input v-model="selectedCards" type="checkbox" :value="card.card" :disabled="blocked" :aria-label="`Выбрать карточку: ${cardTitle(card)}`" /><span>{{ cardTitle(card) }}</span></label>
        <p class="object-note">Колонка: {{ card.columns.map(id => props.columns.find(column => column.column === id)?.names.join(' / ') || id).join(' / ') }}</p>
        <p class="search-text">{{ card.descriptions.join('\n\n') }}</p><TagNames :ids="card.tags" :tags="tags" />
        <p v-if="card.state === 'conflicted'" class="action-error">Есть конфликт. Доступен просмотр сохранённых вариантов.</p>
        <div class="view-actions">
          <button :disabled="blocked" @click="emit('open', { kind: 'card.read', target: card }, $event)">Открыть карточку</button>
          <button :disabled="parentBlocked || card.state !== 'trashed'" @click="emit('open', { kind: 'card.restore', target: card }, $event)">Восстановить</button>
        </div>
      </li>
    </ul>
  </section>
</template>
