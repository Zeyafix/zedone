<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from "vue";
import { errorMessage, read, type Card, type CardHistory, type CardTextVersion } from "./workspace-api";

const props = defineProps<{ card: Card; ready: boolean; disabled: boolean; mutable: boolean }>();
const emit = defineEmits<{ restore: [id: string]; refresh: []; close: [] }>();
const versions = ref<CardTextVersion[]>([]);
const page = ref<CardHistory>();
const selected = ref<CardTextVersion>();
const loading = ref(false), loaded = ref(false), error = ref("");
const heading = ref<HTMLElement>();
const previewHeading = ref<HTMLElement>();
let disposed = false;

async function selectVersion(version: CardTextVersion) {
  selected.value = version;
  await nextTick();
  if (!disposed) {
    previewHeading.value?.focus({ preventScroll: true });
    previewHeading.value?.scrollIntoView({ block: "start" });
  }
}

async function more() {
  if (loading.value || !props.ready || error.value) return;
  loading.value = true;
  try {
    const result = await read("card.history", { space: props.card.space, card: props.card.card,
      after: page.value?.next ?? null, expected: page.value?.version ?? null, limit: 25 });
    if (disposed) return;
    // Every later page is guarded by the first history token in the core.
    versions.value.push(...result.versions); page.value = result; loaded.value = true;
  } catch (cause) {
    if (!disposed) error.value = errorMessage(cause);
  } finally { if (!disposed) loading.value = false; }
}
onMounted(async () => { await nextTick(); heading.value?.focus(); void more(); });
onUnmounted(() => { disposed = true; });
</script>

<template>
  <section class="card-history" aria-label="История карточки">
    <h3 ref="heading" tabindex="-1">История карточки</h3>
    <p class="object-note">Каждая версия содержит заголовок и описание вместе. Восстановление создаёт новую версию и сохраняет всю прежнюю историю.</p>
    <div class="view-actions">
      <button data-history-close :disabled="disabled" @click="emit('close')">Вернуться к карточке</button>
      <button data-history-refresh :disabled="disabled || loading || !ready" @click="emit('refresh')">Обновить историю и карточку</button>
    </div>
    <p v-if="loading" role="status">Читаем историю…</p>
    <p v-if="error" role="alert" class="action-error">{{ error }} Обновите историю перед новым выбором.</p>
    <p v-if="loaded && !versions.length" class="empty-view">Сохранённых версий пока нет. Они появятся после сохранения текста.</p>
    <ol class="history-list" aria-label="Сохранённые версии">
      <li v-for="version in versions" :key="version.id" :data-text-version="version.id">
        <button :aria-pressed="selected?.id === version.id" :disabled="disabled || loading || !!error" @click="selectVersion(version)">
          <span>Версия {{ version.sequence }}</span><strong>{{ version.title }}</strong>
        </button>
      </li>
    </ol>
    <button v-if="page?.next" data-history-more :disabled="disabled || loading || !!error || !ready" @click="more">Показать ещё версии</button>
    <section v-if="selected" class="history-preview" aria-label="Выбранная версия">
      <h4 ref="previewHeading" tabindex="-1">Версия {{ selected.sequence }}</h4>
      <p class="object-note">Участник: {{ selected.subject }}</p>
      <dl><dt>Заголовок</dt><dd data-history-title>{{ selected.title }}</dd><dt>Описание</dt><dd data-history-description>{{ selected.description || 'Без описания' }}</dd></dl>
      <p>Заголовок и описание карточки будут заменены показанной парой. Теги, выполнение и место сохранятся. Вернуть прежний текст можно восстановлением другой версии.</p>
      <button data-history-restore class="primary-button" :disabled="disabled || loading || !!error || !ready || !mutable" @click="emit('restore', selected.id)">Восстановить эту версию</button>
    </section>
    <p v-if="!mutable" class="object-note">Восстановление сейчас недоступно. Проверьте результат исходного действия и обновите историю. Для восстановления нужна активная карточка без конфликтов в активной колонке и пространстве.</p>
  </section>
</template>
