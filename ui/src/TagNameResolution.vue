<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import { DialogClose, DialogContent, DialogDescription, DialogOverlay, DialogPortal, DialogRoot, DialogTitle } from "reka-ui";
import { createMutation, deliverMutation, errorMessage, forgetMutation, mutationCredential, read, type Tag } from "./workspace-api";

const props = defineProps<{ blocked: boolean; refresh: () => Promise<void> }>();
const emit = defineEmits<{ lock: [value: boolean] }>();
const dialog = ref(false), working = ref(false), refused = ref(false);
const group = ref<Tag[]>([]), names = ref<string[]>([]), target = ref<Tag>();
const failure = ref(""), subject = ref<string>();
const rejected = ref<string>();
const deletion = ref(false), keep = ref<boolean>();
const valid = computed(() => deletion.value ? keep.value !== undefined : names.value.length > 0 && names.value.every(name => /[^\p{White_Space}]/u.test(name)));
watch(() => dialog.value || working.value, value => emit("lock", value), { flush: "sync" });
onBeforeUnmount(() => emit("lock", false));
let opener: HTMLElement | undefined;
async function inspect(tag: Tag, preserve = false) {
  const tags = (await read("tag.list", { space: tag.space, include_deleted: false })).tags;
  const fresh = tags.find(t => t.tag === tag.tag);
  if (!fresh || (deletion.value ? !fresh.deletion_conflict : !fresh.name_group?.length || fresh.deletion_conflict)) throw new Error("conflict");
  const previous = new Map(group.value.map((t, i) => [t.tag, names.value[i]]));
  group.value = (deletion.value ? [fresh.tag] : fresh.name_group!).map(id => {
    const member = tags.find(t => t.tag === id);
    if (!member) throw new Error("conflict");
    return member;
  });
  names.value = group.value.map(t => preserve && previous.has(t.tag) ? previous.get(t.tag)! : t.names[0] ?? "");
  target.value = fresh; refused.value = false;
}
async function open(tag: Tag, event: Event) {
  if (props.blocked || working.value || dialog.value) return;
  opener = event.currentTarget as HTMLElement; working.value = true; failure.value = ""; rejected.value = undefined;
  deletion.value = tag.deletion_conflict ?? false; keep.value = undefined;
  try {
    subject.value = (await mutationCredential()).subject;
    await inspect(tag); dialog.value = true;
  } catch (cause) { failure.value = errorMessage(cause); }
  finally { working.value = false; }
}
async function refreshGroup() {
  if (working.value || !target.value) return;
  working.value = true;
  try {
    if (rejected.value) { await forgetMutation(rejected.value); rejected.value = undefined; }
    await inspect(target.value, true); failure.value = "Проверьте актуальные варианты и подтвердите новый выбор.";
  }
  catch (cause) { refused.value = true; failure.value = errorMessage(cause); }
  finally { working.value = false; }
}
async function save() {
  if (working.value || props.blocked || refused.value || !valid.value || !target.value) return;
  working.value = true; failure.value = "";
  try {
    const id = deletion.value
      ? await createMutation("tag.resolve_deletion", { space: target.value.space, tag: target.value.tag, keep: keep.value!, expected: target.value.version }, subject.value)
      : await createMutation("tag.resolve_names", { space: target.value.space, tag: target.value.tag,
        names: group.value.map((t, i) => ({ tag: t.tag, name: names.value[i]!, expected: t.version })) }, subject.value);
    const result = await deliverMutation(id);
    if (result.status === "completed") { dialog.value = false; await props.refresh(); }
    else {
      refused.value = true;
      if (result.response?.status === "error" && ["stale_state", "conflict", "invalid_request", "access_denied", "not_found"].includes(result.response.error.code)) rejected.value = id;
      failure.value = result.response?.status === "error" ? errorMessage(new Error(result.response.error.code)) : "Решение не подтверждено. Проверьте исходное действие в списке незавершённых действий.";
    }
  } catch (cause) { refused.value = true; failure.value = errorMessage(cause); }
  finally { working.value = false; }
}
async function restoreFocus(event: Event) {
  event.preventDefault(); await nextTick();
  const id = target.value?.tag;
  const fallback = document.querySelector<HTMLElement>(`.space-tags [data-tag="${id}"] button:not(:disabled), .space-tags summary`);
  if (opener?.isConnected && !opener.hasAttribute("disabled")) opener.focus(); else fallback?.focus();
}
defineExpose({ open });
</script>

<template>
  <p v-if="failure && !dialog" role="alert">{{ failure }}</p>
  <DialogRoot v-model:open="dialog"><DialogPortal><DialogOverlay class="dialog-overlay" />
    <DialogContent class="action-dialog" @close-auto-focus="restoreFocus">
      <div class="dialog-header"><DialogTitle>{{ deletion ? 'Разрешить удаление тега' : 'Разрешить имена группы тегов' }}</DialogTitle><DialogClose aria-label="Закрыть выбор решения">Закрыть</DialogClose></div>
      <DialogDescription v-if="deletion">Тег «{{ target?.names.join(' / ') }}» удалён на одном устройстве и независимо назначен на другом. Сохраните тег с назначениями или подтвердите удаление со снятием всех его назначений. Карточки сохранятся. Удалённый тег не попадёт в корзину и не сможет быть восстановлен отдельно.</DialogDescription>
      <DialogDescription v-else>После обмена эта группа сохраняет отдельные теги. Выберите разные свободные имена для всех тегов. Их назначения сохранятся.</DialogDescription>
      <form @submit.prevent="save">
        <fieldset v-if="deletion" :disabled="working || blocked"><legend>Решение об удалении</legend>
          <label class="tag-deletion-choice"><input v-model="keep" type="radio" :value="true" name="tag-deletion" />Сохранить тег с назначениями</label>
          <label class="tag-deletion-choice"><input v-model="keep" type="radio" :value="false" name="tag-deletion" />Подтвердить удаление тега</label>
        </fieldset>
        <fieldset v-else :disabled="working || blocked"><legend>Имена тегов</legend>
          <label v-for="(tag, index) in group" :key="tag.tag" class="name-field">Тег {{ index + 1 }}: {{ tag.names.join(' / ') }}
            <input v-model="names[index]" :aria-label="`Имя тега ${index + 1}`" />
          </label>
        </fieldset>
        <p v-if="failure" role="alert" class="action-error">{{ failure }}</p>
        <button v-if="refused" type="button" :disabled="working || blocked && !rejected" @click="refreshGroup">{{ deletion ? 'Обновить конфликт тега' : 'Обновить варианты имён' }}</button>
        <div class="dialog-actions"><DialogClose type="button">Отмена</DialogClose><button class="primary-button" type="submit" :disabled="working || blocked || refused || !valid">{{ deletion ? 'Подтвердить решение' : 'Сохранить имена группы' }}</button></div>
      </form>
    </DialogContent>
  </DialogPortal></DialogRoot>
</template>
