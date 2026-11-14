<script setup lang="ts">
import { nextTick, onMounted, ref } from "vue";
import type { HistoryPreview } from "./workspace-api";

defineProps<{ preview: HistoryPreview }>();
const bytes = (value: number) => `${value.toLocaleString("ru-RU")} байт`;
const heading = ref<HTMLElement>();
onMounted(async () => { await nextTick(); heading.value?.focus({ preventScroll: true }); heading.value?.scrollIntoView({ block: "start" }); });
</script>

<template>
  <section class="history-scope" aria-label="Состав очистки истории">
    <h3 ref="heading" tabindex="-1">{{ preview.card ? 'История одной карточки' : 'Вся история пространства, включая корзину' }}</h3>
    <p class="object-note">Пространство: {{ preview.space }}<template v-if="preview.card">. Карточка: {{ preview.card }}</template></p>
    <dl class="history-size">
      <dt>Объём текста истории</dt><dd data-history-bytes>{{ bytes(preview.text_bytes) }}</dd>
      <dt>Можно очистить</dt><dd data-history-removable>{{ bytes(preview.removable_bytes) }}</dd>
    </dl>
    <p class="object-note">Это объём заголовков и описаний в UTF-8. Размер базы на диске после очистки может не уменьшиться.</p>
    <p>Версий: {{ preview.entries.length }}. К очистке: {{ preview.entries.filter(entry => !entry.retained).length }}. Сохраняются из-за конфликтов: {{ preview.entries.filter(entry => entry.retained).length }}.</p>
    <p v-if="!preview.entries.length" class="empty-view">Сохранённых версий в этой области нет.</p>
    <ul v-else class="scope-list history-cleanup-list">
      <li v-for="entry in preview.entries" :key="`${entry.card}/${entry.version.id}`" :data-history-entry="entry.version.id" :data-history-card="entry.card" :data-history-retained="entry.retained">
        <details><summary><strong>{{ entry.version.title }}</strong> — версия {{ entry.version.sequence }}. {{ entry.retained ? 'Сохраняется: есть конфликт карточки или контейнера' : 'Будет очищена' }}</summary>
          <p class="object-note">Карточка: {{ entry.card }}</p>
          <dl class="history-pair"><dt>Заголовок</dt><dd>{{ entry.version.title }}</dd><dt>Описание</dt><dd>{{ entry.version.description || 'Без описания' }}</dd></dl>
        </details>
      </li>
    </ul>
    <p>Текущий текст, теги, выполнение и место карточек сохранятся. Версии, защищённые конфликтом, останутся. Незавершённый ввод сможет создать новую версию.</p>
    <p>Очистка не имеет отмены. Показанные очищаемые версии больше нельзя будет восстановить через историю. Резервные копии сохранятся.</p>
  </section>
</template>
