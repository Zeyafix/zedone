<script setup lang="ts">
import { computed, ref } from "vue";
import type { components } from "../../api/schema";
import type { Space } from "./workspace-api";
const props = defineProps<{ spaces: Space[]; prefix: string; disabled?: boolean }>();
const selected = ref<string[]>([]), write = ref(false), administration = ref(false), limited = ref(false), until = ref("");
const extra = ref("");
const modes = ref<Record<string, "default" | "read" | "read_write">>({});
const ids = computed(() => [...new Set([...selected.value, ...extra.value.split(/[\s,]+/).filter(Boolean)])]);
const valid = computed(() => (ids.value.length > 0 || administration.value) && ids.value.every(id => /^[a-f0-9]{32}$/.test(id))
  && (!limited.value || Number.isFinite(new Date(until.value).getTime()) && new Date(until.value).getTime() > Date.now()));
function value(): { scope: components["schemas"]["AccessScope"]; expires_at_unix_ms: number | null } {
  if (!valid.value) throw new Error("invalid_request");
  const spaces = Object.fromEntries(ids.value.map(id => [id, modes.value[id] && modes.value[id] !== "default" ? modes.value[id] as "read" | "read_write" : write.value ? "read_write" as const : "read" as const]));
  return { scope: administration.value ? ids.value.length ? { kind: "administration_and_spaces", spaces } : { kind: "administration" } : { kind: "spaces", spaces },
    expires_at_unix_ms: limited.value ? new Date(until.value).getTime() : null };
}
defineExpose({ valid, value });
</script>

<template>
  <fieldset :disabled="disabled" class="access-scope">
    <legend>Права доступа</legend>
    <p>Выберите пространства. Остальные пространства доступны не будут.</p>
    <label v-for="space in props.spaces" :key="space.space" class="access-choice">
      <input v-model="selected" type="checkbox" :value="space.space" :data-space="space.space" />
      <span>{{ space.names.join(' / ') || 'Без названия' }}</span>
    </label>
    <label class="name-field" :for="`${prefix}-space-ids`">Дополнительные ID пространств
      <input :id="`${prefix}-space-ids`" v-model="extra" placeholder="ID, разделённые пробелами" autocomplete="off" spellcheck="false" />
    </label>
    <p class="object-note">Введите известные ID, если пространства не видны вашему доступу. Название устройства не подтверждает его личность: сверьте запрос на обоих устройствах.</p>
    <label class="access-choice"><input v-model="write" type="checkbox" :data-access-write="prefix" />Разрешить изменения выбранных пространств</label>
    <label v-for="id in ids" :key="id" class="name-field" :for="`${prefix}-mode-${id}`">Права: {{ spaces.find(space=>space.space===id)?.names.join(' / ') || id }}
      <select :id="`${prefix}-mode-${id}`" :value="modes[id] ?? 'default'" @change="modes[id]=($event.target as HTMLSelectElement).value as 'default' | 'read' | 'read_write'">
        <option value="default">По общему выбору: {{ write ? 'чтение и изменение' : 'чтение' }}</option><option value="read">Только чтение</option><option value="read_write">Чтение и изменение</option>
      </select>
    </label>
    <label class="access-choice"><input v-model="administration" type="checkbox" :data-access-admin="prefix" />Разрешить администрирование устройств и доступов</label>
    <p class="object-note">Администрирование выдаётся отдельно и не открывает данные пространств или восстановление всей базы.</p>
    <label class="access-choice"><input v-model="limited" type="checkbox" :data-access-limited="prefix" />Ограничить срок (по умолчанию — до отзыва)</label>
    <label v-if="limited" class="name-field" :for="`${prefix}-until`">Доступ до
      <input :id="`${prefix}-until`" v-model="until" type="datetime-local" required />
    </label>
  </fieldset>
</template>
