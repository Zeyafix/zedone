<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import type { Card } from "./workspace-api";

const props = defineProps<{ card: Card; disabled: boolean }>();
const emit = defineEmits<{ resolve: [title: string, description: string]; cancel: [] }>();
const title = ref(props.card.titles.length === 1 ? props.card.titles[0]! : "");
const description = ref(props.card.descriptions.length === 1 ? props.card.descriptions[0]! : "");
const descriptionChosen = ref(props.card.descriptions.length === 1);
const titleInput = ref<HTMLInputElement>();
const valid = computed(() => /[^\p{White_Space}]/u.test(title.value) && descriptionChosen.value);
onMounted(() => titleInput.value?.focus());
</script>

<template>
  <form class="text-resolution" aria-label="Разрешение конфликта текста" @submit.prevent="valid && !disabled && emit('resolve', title, description)">
    <p>Выберите сохранённые варианты или объедините их вручную. Заголовок и описание сохранятся вместе новой версией. Остальные конфликты останутся.</p>
    <fieldset :disabled="disabled">
      <legend>Заголовок</legend>
      <ul class="text-variants">
        <li v-for="text in card.titles" :key="text"><p>{{ text }}</p><button type="button" @click="title = text">Выбрать этот заголовок</button></li>
      </ul>
      <label class="name-field">Итоговый заголовок<input ref="titleInput" v-model="title" name="resolved-title" required /></label>
    </fieldset>
    <fieldset :disabled="disabled">
      <legend>Описание</legend>
      <ul class="text-variants">
        <li v-for="text in card.descriptions" :key="text"><p>{{ text || 'Без описания' }}</p><button type="button" @click="description = text; descriptionChosen = true">Выбрать это описание</button></li>
      </ul>
      <label class="name-field">Итоговое описание<textarea v-model="description" name="resolved-description" rows="8" @input="descriptionChosen = true" /></label>
      <button type="button" @click="description = ''; descriptionChosen = true">Оставить без описания</button>
    </fieldset>
    <p v-if="!descriptionChosen" class="object-note">Выберите описание, введите объединённый текст или явно оставьте его пустым.</p>
    <div class="view-actions"><button type="submit" :disabled="disabled || !valid">Сохранить решение</button><button type="button" :disabled="disabled" @click="emit('cancel')">Отмена</button></div>
  </form>
</template>
