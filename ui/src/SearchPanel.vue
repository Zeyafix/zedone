<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { read, type Card, type SearchHit, type SearchResults } from "./workspace-api";

const props = defineProps<{ space?: string; blocked: boolean; revision: number }>();
const emit = defineEmits<{ open: [action: { kind: "card.read"; target: Card }, event: Event]; navigate: [] }>();
const query = ref("");
const scope = ref(props.space ? "space" : "all");
const results = ref<SearchResults>();
const loading = ref(false);
const error = ref("");
const input = ref<HTMLTextAreaElement>();
let generation = 0;
const groups = computed(() => [
  { label: "Конфликты названий", spaces: results.value?.spaces.filter(item => item.space.names.length !== 1) ?? [] },
  { label: "Пространства", spaces: results.value?.spaces.filter(item => item.space.names.length === 1) ?? [] },
].filter(group => group.spaces.length));
const count = computed(() => results.value?.spaces.reduce((sum, item) => sum + item.hits.length, 0) ?? 0);
const placements = (hits: SearchHit[]) => [
  { label: "Конфликты", hits: hits.filter(hit => hit.placement_conflict) },
  { label: "В порядке доски", hits: hits.filter(hit => !hit.placement_conflict) },
].filter(group => group.hits.length);

function invalidate() {
  generation++; loading.value = false; results.value = undefined; error.value = "";
}
watch([query, scope, () => props.space], invalidate, { flush: "sync" });
// Changes made in the existing editor are read afresh, never patched into a receipt.
watch(() => props.revision, () => { void search(); });
async function search() {
  const current = ++generation;
  results.value = undefined; error.value = ""; loading.value = true;
  try {
    const result = await read("card.search", {
      query: query.value,
      scope: scope.value === "space" && props.space ? { kind: "space", space: props.space } : { kind: "all" },
    });
    if (current === generation) results.value = result;
  } catch (cause) {
    if (current !== generation) return;
    const code = cause instanceof Error ? cause.message : "unavailable";
    error.value = code === "access_denied" ? "Нет доступа к поиску в выбранной области. Проверьте локальный доступ."
      : code === "invalid_request" ? "Запрос не принят. Проверьте его длину и повторите поиск."
        : "Не удалось получить результаты. Повторите поиск; запрос и область сохранены.";
  } finally { if (current === generation) loading.value = false; }
}
async function clear() { query.value = ""; await search(); input.value?.focus(); }
onMounted(() => { input.value?.focus(); void search(); });
onUnmounted(invalidate);
defineExpose({ focus: () => input.value?.focus() });
</script>

<template>
  <section class="search-panel" aria-labelledby="search-title">
    <h2 id="search-title">Поиск карточек</h2>
    <form class="search-form" @submit.prevent="search">
      <label class="name-field">Текст в заголовке или описании
        <textarea ref="input" v-model="query" name="query" rows="2" autocomplete="off" aria-describedby="search-help" />
      </label>
      <fieldset class="search-scope">
        <legend>Где искать</legend>
        <label v-if="space"><input v-model="scope" type="radio" name="search-scope" value="space" /> В текущем пространстве</label>
        <label><input v-model="scope" type="radio" name="search-scope" value="all" /> Во всех пространствах</label>
      </fieldset>
      <p id="search-help" class="object-note">Пустой запрос показывает все доступные карточки выбранной области, включая выполненные. Карточки в корзине не участвуют.</p>
      <div class="view-actions">
        <button type="submit" class="primary-button search-submit">{{ loading ? 'Ищем…' : 'Найти' }}</button>
        <button type="button" class="search-clear" @click="clear">Очистить запрос</button>
      </div>
    </form>
    <p v-if="error" role="alert" class="action-error">{{ error }}</p>
    <div class="search-results" :aria-busy="loading">
      <p role="status">{{ loading ? 'Ищем карточки…' : results ? `Найдено карточек: ${count}` : error ? 'Результаты недоступны.' : 'Нажмите «Найти», чтобы получить результаты.' }}</p>
      <p v-if="results && !count" class="empty-view">Совпадений нет. Измените запрос или область поиска.</p>
      <section v-for="group in groups" :key="group.label" :aria-label="group.label">
        <h3 v-if="group.label === 'Конфликты названий'">{{ group.label }}</h3>
        <section v-for="item in group.spaces" :key="item.space.space" class="search-space" :data-space="item.space.space">
          <h3><a :href="`#/spaces/${item.space.space}`" @click="emit('navigate')">{{ item.space.names.length === 1 ? item.space.names[0] || 'Без названия' : 'Конфликт названий' }}</a></h3>
          <p v-if="item.space.names.length !== 1" class="object-note">Варианты названия: {{ item.space.names.map(name => name || 'Без названия').join(' / ') }}</p>
          <section v-for="placement in placements(item.hits)" :key="placement.label" :aria-label="placement.label">
            <h4 v-if="placement.label === 'Конфликты' || item.hits.some(hit => hit.placement_conflict)">{{ placement.label }}</h4>
            <ul class="search-hits">
              <li v-for="{ card, placement_conflict } in placement.hits" :key="card.card" :data-card="card.card">
                <button class="card-title" :disabled="blocked" @click="emit('open', { kind: 'card.read', target: card }, $event)">{{ card.titles.length === 1 ? card.titles[0] : 'Конфликт заголовков' }}</button>
                <p v-if="card.titles.length !== 1" class="search-text">{{ card.titles.join('\n\n') }}</p>
                <p v-if="card.descriptions.some(text => text.length)" class="search-text">{{ card.descriptions.join('\n\n') }}</p>
                <p v-if="card.completed.length === 1 && card.completed[0]" class="object-note">Выполнено</p>
                <p v-if="placement_conflict" class="object-note">Место или удаление конфликтует. Карточка показана один раз.</p>
                <p v-if="card.state === 'conflicted'" class="object-note">Есть конфликт. Показаны все текущие варианты текста.</p>
              </li>
            </ul>
          </section>
        </section>
      </section>
    </div>
  </section>
</template>
