<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import MarkdownView from "./markdown/MarkdownView";
import type { Parameters } from "../../api/validate-response.js";
import { loadDraft, lockDraft, removeDraft, saveDraft, type EditorDraft } from "./editor-drafts";
import { createMutation, deliverMutation, errorMessage, forgetMutation, mutationCredential, pendingCommands, read, type Card, type EditorMutation, type Pending } from "./workspace-api";

const props = defineProps<{ card: Card; ready: boolean; draftId?: string }>();
const emit = defineEmits<{ changed: []; close: []; stay: [] }>();
const snapshot = ref<Card>(props.card);
const draft = ref<EditorDraft>();
const comparison = ref<Card>();
const editing = ref(false), busy = ref(false), blocked = ref(false), storageError = ref(false);
const closing = ref(false);
const warning = ref(false), forgetWarning = ref(false), discardWarning = ref(false);
const error = ref("");
const pending = ref<Pending>();
const session = ref("");
const titleInput = ref<HTMLInputElement>();
const stayButton = ref<HTMLButtonElement>();
const dirty = computed(() => !!draft.value && (draft.value.title !== draft.value.savedTitle || draft.value.description !== draft.value.savedDescription));
const validTitle = computed(() => !!draft.value && /[^\p{White_Space}]/u.test(draft.value.title));
const mutable = computed(() => snapshot.value.state === "active" && snapshot.value.titles.length === 1 && snapshot.value.descriptions.length === 1 && snapshot.value.completed.length === 1);
const status = computed(() => storageError.value ? "Ввод не сохранён на устройстве. Не закрывайте редактор."
  : busy.value ? "Сохраняем…" : error.value || (pending.value ? "Результат неизвестен. Ввод сохранён на устройстве; проверьте исходное действие."
    : editing.value && !validTitle.value ? "Заголовок обязателен. Последняя сохранённая запись не изменена."
      : dirty.value ? "Есть несохранённые изменения" : editing.value ? "Сохранено" : ""));
let subject = "", release: (() => void) | undefined, active: Promise<void> | undefined;
let debounce: ReturnType<typeof setTimeout> | undefined, maximum: ReturnType<typeof setTimeout> | undefined;
let composing = false, suspended = false, disposed = false;
const begunHere = new Set<string>();
const sameVersion = (a: Card["version"], b: Card["version"]) => a.length === b.length && a.every((byte, i) => byte === b[i]);

function clearTimers() { clearTimeout(debounce); clearTimeout(maximum); debounce = undefined; maximum = undefined; }
function persist() {
  if (!draft.value) return true;
  try { saveDraft(draft.value); storageError.value = false; return true; }
  catch { storageError.value = true; clearTimers(); return false; }
}
function schedule() {
  if (!editing.value || busy.value || closing.value || suspended || disposed || blocked.value || pending.value || storageError.value || !props.ready || composing || !dirty.value || !validTitle.value) return;
  clearTimeout(debounce);
  debounce = setTimeout(() => { void flush(); }, 500);
  maximum ??= setTimeout(() => { void flush(); }, 2000);
}
function update(field: "title" | "description", event: Event) {
  if (!draft.value) return;
  draft.value[field] = (event.target as HTMLInputElement | HTMLTextAreaElement).value;
  if (persist()) schedule();
}
function compositionStart() { composing = true; clearTimers(); }
function compositionEnd() { composing = false; schedule(); }
async function task(job: () => Promise<void>) {
  if (busy.value || disposed) return;
  busy.value = true;
  active = (async () => {
    try { await job(); }
    catch (cause) {
      error.value = cause instanceof DOMException ? "Не удалось сохранить действие на устройстве. Запрос не отправлен; ввод остался черновиком." : errorMessage(cause);
      blocked.value = true;
      if (!editing.value) { release?.(); release = undefined; }
    }
    finally {
      busy.value = false; active = undefined;
      if (disposed) release?.();
      else if (suspended && document.visibilityState === "visible" && props.ready) void resume();
      else schedule();
    }
  })();
  await active;
}
async function discoverPending() {
  const entries = await pendingCommands();
  pending.value = entries.find(item => {
    const parameters = JSON.parse(item.intention.body).parameters;
    return parameters?.space === snapshot.value.space && parameters?.card === snapshot.value.card;
  });
  if (entries.length && !pending.value) throw new Error("pending_exists");
}
async function confirmDelivery(id: string) {
  const saved = (await pendingCommands()).find(item => item.id === id);
  if (!saved) throw new Error("missing_intention");
  pending.value = saved;
  const result = await deliverMutation(id);
  if (result.status !== "completed" || result.response.status !== "ok") {
    error.value = result.status === "pending" && result.reason === "identity_changed" ? errorMessage(new Error("identity_changed"))
      : result.response?.status === "error" ? errorMessage(new Error(result.response.error.code)) : "Результат неизвестен. Проверьте исходное действие; новый ввод хранится отдельно.";
    blocked.value = true; return false;
  }
  pending.value = undefined;
  // The shared operation-specific validator establishes this CardSnapshot.
  const receipt = result.response.result as Card;
  if (draft.value) {
    draft.value.savedTitle = receipt.titles[0] ?? "";
    draft.value.savedDescription = receipt.descriptions[0] ?? "";
    draft.value.version = receipt.version;
    persist();
  }
  if (saved.intention.operation === "card.begin_editing") session.value = begunHere.has(id) ? id : "";
  if (saved.intention.operation === "card.finish_editing") session.value = "";
  const fresh = await read("card.read", { space: snapshot.value.space, card: snapshot.value.card });
  emit("changed");
  if (!sameVersion(fresh.version, receipt.version)) {
    comparison.value = fresh; blocked.value = true;
    error.value = "Данные изменились после сохранения. Новый ввод остался в редакторе. Сверьте его с сохранённой карточкой.";
    return false;
  }
  snapshot.value = fresh;
  error.value = ""; blocked.value = false;
  return !storageError.value;
}
async function mutate<O extends EditorMutation>(operation: O, parameters: Parameters[O]) {
  if (draft.value && !persist()) return false;
  const id = await createMutation(operation, parameters, draft.value?.subject ?? subject);
  if (operation === "card.begin_editing") begunHere.add(id);
  return confirmDelivery(id);
}
async function begin() {
  if (!draft.value) return false;
  return mutate("card.begin_editing", { space: draft.value.space, card: draft.value.card, expected: draft.value.version });
}
async function start() {
  await task(async () => {
    if (!mutable.value || !props.ready) return;
    const credential = await mutationCredential();
    subject = credential.subject;
    const id = props.draftId ?? crypto.randomUUID().replaceAll("-", "");
    release = await lockDraft(id);
    const saved = props.draftId ? loadDraft(id) : undefined;
    if (saved && (saved.subject !== subject || saved.space !== snapshot.value.space || saved.card !== snapshot.value.card)) throw new Error("identity_changed");
    draft.value = saved ?? { id, subject, space: snapshot.value.space, card: snapshot.value.card, version: snapshot.value.version,
      title: snapshot.value.titles[0] ?? "", description: snapshot.value.descriptions[0] ?? "",
      savedTitle: snapshot.value.titles[0] ?? "", savedDescription: snapshot.value.descriptions[0] ?? "" };
    editing.value = true;
    if (!persist()) return;
    await discoverPending();
    if (pending.value) { blocked.value = true; return; }
    if (!sameVersion(snapshot.value.version, draft.value.version)) {
      comparison.value = snapshot.value; blocked.value = true; error.value = "Данные изменились. Восстановленный ввод не подменяет сохранённую карточку."; return;
    }
    await begin();
    await nextTick();
    if (!(document.activeElement instanceof HTMLInputElement) && !(document.activeElement instanceof HTMLTextAreaElement)) titleInput.value?.focus();
  });
}
async function flush() {
  clearTimers();
  if (!draft.value || !dirty.value || !validTitle.value || composing || pending.value || blocked.value || !props.ready || storageError.value) return;
  await task(async () => {
    if (!session.value && !await begin()) return;
    const value = draft.value!;
    await mutate("card.autosave", { space: value.space, card: value.card, expected: value.version, session: session.value, title: value.title, description: value.description });
  });
}
async function finish() {
  if (!draft.value || !session.value || pending.value || blocked.value || storageError.value || !props.ready) return;
  await task(async () => {
    const value = draft.value!;
    await mutate("card.finish_editing", { space: value.space, card: value.card, expected: value.version, session: session.value });
  });
}
async function retry() {
  await task(async () => {
    await discoverPending();
    if (pending.value && !await confirmDelivery(pending.value.id)) return;
    if (draft.value && !session.value && !blocked.value) await begin();
  });
}
async function inspectCurrent() {
  await task(async () => { comparison.value = await read("card.read", { space: snapshot.value.space, card: snapshot.value.card }); });
}
async function applyDraft() {
  if (!comparison.value || !draft.value || pending.value || !validTitle.value || !props.ready) return;
  await task(async () => {
    const fresh = comparison.value!;
    // Explicit consent to the displayed version, never an automatic rebase.
    draft.value!.version = fresh.version;
    draft.value!.savedTitle = fresh.titles[0] ?? ""; draft.value!.savedDescription = fresh.descriptions[0] ?? "";
    snapshot.value = fresh; session.value = ""; blocked.value = false; error.value = "";
    if (persist() && await begin()) comparison.value = undefined;
  });
  await flush();
}
async function forget() {
  if (!pending.value || busy.value) return;
  await task(async () => { await forgetMutation(pending.value!.id); pending.value = undefined; forgetWarning.value = false; blocked.value = true; });
}
async function toggleCompleted() {
  if (!mutable.value || dirty.value || pending.value || blocked.value || !props.ready) return;
  await task(async () => {
    subject ||= (await mutationCredential()).subject;
    await mutate("card.set_completed", { space: snapshot.value.space, card: snapshot.value.card, expected: snapshot.value.version, completed: !snapshot.value.completed[0] });
  });
}
async function requestClose() {
  if (closing.value) return false;
  closing.value = true; clearTimers();
  try {
    if (active) await active;
    if (!draft.value) return !busy.value;
    if (dirty.value && validTitle.value && !pending.value && !blocked.value && !storageError.value) await flush();
    if (dirty.value || pending.value || blocked.value || storageError.value) { warning.value = true; return false; }
    await finish();
    if (dirty.value || pending.value || blocked.value || storageError.value || session.value) { warning.value = true; return false; }
    removeDraft(draft.value.id); draft.value = undefined; release?.(); release = undefined; return true;
  } catch { storageError.value = true; warning.value = true; return false; }
  finally { closing.value = false; }
}
async function keepAndClose() {
  if (busy.value || closing.value || !persist()) return;
  closing.value = true;
  try { await finish(); if (persist()) emit("close"); }
  finally { closing.value = false; }
}
async function discard() {
  if (busy.value || closing.value || !draft.value) return;
  closing.value = true;
  try { await finish(); removeDraft(draft.value.id); draft.value = undefined; emit("close"); }
  catch { storageError.value = true; }
  finally { closing.value = false; }
}
async function suspend() {
  if (!editing.value || suspended) return;
  suspended = true; clearTimers(); persist();
  if (active) await active;
  if (!disposed) { await flush(); await finish(); }
}
async function resume() {
  if (!suspended || disposed || !props.ready || busy.value || !draft.value) return;
  suspended = false;
  await task(async () => {
    await discoverPending();
    if (pending.value || blocked.value || storageError.value) return;
    const fresh = await read("card.read", { space: draft.value!.space, card: draft.value!.card });
    if (!sameVersion(fresh.version, draft.value!.version)) { comparison.value = fresh; blocked.value = true; error.value = "Данные изменились, пока редактор был скрыт. Ввод сохранён отдельно."; return; }
    session.value = ""; await begin();
  });
}
function visibility() { if (document.visibilityState === "hidden") void suspend(); else void resume(); }
function beforeUnload(event: BeforeUnloadEvent) {
  if (draft.value && (dirty.value || busy.value || pending.value || storageError.value)) { event.preventDefault(); event.returnValue = ""; }
}
watch(() => props.ready, ready => { if (ready && document.visibilityState === "visible") void resume(); });
watch(warning, async visible => {
  if (visible) { await nextTick(); stayButton.value?.focus(); stayButton.value?.scrollIntoView({ block: "nearest" }); }
});
onMounted(() => {
  document.addEventListener("visibilitychange", visibility); window.addEventListener("pagehide", suspend); window.addEventListener("beforeunload", beforeUnload);
  if (props.draftId) void start();
});
onUnmounted(() => {
  disposed = true; clearTimers();
  document.removeEventListener("visibilitychange", visibility); window.removeEventListener("pagehide", suspend); window.removeEventListener("beforeunload", beforeUnload);
  if (!active) release?.();
});
defineExpose({ requestClose });
</script>

<template>
  <div class="card-editor">
    <div v-if="!editing" class="card-content">
      <h3 v-for="text in snapshot.titles" :key="text">{{ text }}</h3>
      <template v-for="text in snapshot.descriptions" :key="text">
        <MarkdownView v-if="text" :source="text" label="Описание карточки" />
        <p v-else>Без описания</p>
      </template>
    </div>
    <template v-else-if="draft">
      <label class="name-field">Заголовок<input ref="titleInput" :value="draft.title" name="card-title" aria-label="Заголовок" :aria-invalid="!validTitle" :readonly="!ready || closing" @input="update('title', $event)" @compositionstart="compositionStart" @compositionend="compositionEnd" /></label>
      <label class="name-field">Описание<textarea :value="draft.description" name="card-description" rows="8" :readonly="!ready || closing" @input="update('description', $event)" @blur="flush" @compositionstart="compositionStart" @compositionend="compositionEnd" /></label>
      <p class="object-note">Изменения сохраняются автоматически. Черновик хранится на этом устройстве до подтверждённого завершения.</p>
    </template>
    <p v-if="status" :role="!busy && (error || storageError || pending) ? 'alert' : 'status'" :class="{ 'action-error': !busy && (error || storageError || pending) }">{{ status }}</p>
    <p v-if="!ready" role="status">Доступ не подтверждён. Ввод остаётся в редакторе; сохранение приостановлено.</p>
    <div class="view-actions">
      <button v-if="!editing && mutable" data-editor-edit :disabled="busy || !ready" @click="start">Редактировать</button>
      <button v-if="editing" :disabled="busy || !ready || !validTitle || blocked || !!pending" @click="flush">Сохранить сейчас</button>
      <button v-if="mutable" :aria-pressed="!!snapshot.completed[0]" :disabled="busy || !ready || dirty || blocked || !!pending" @click="toggleCompleted">{{ snapshot.completed[0] ? 'Снять отметку выполнения' : 'Отметить выполненной' }}</button>
      <button v-if="pending" :disabled="busy || !ready || storageError" @click="retry">Проверить исходное действие</button>
      <button v-if="pending" :disabled="busy" @click="forgetWarning = true">Убрать исходное действие</button>
      <button v-if="blocked" :disabled="busy || !ready" @click="inspectCurrent">Сверить с сохранённым</button>
      <button v-if="storageError" :disabled="busy" @click="persist(); schedule()">Повторить запись черновика</button>
    </div>
    <section v-if="comparison" class="comparison-panel" aria-label="Сохранённая карточка">
      <h3>Сейчас сохранено</h3><div class="card-content"><p v-for="text in comparison.titles" :key="text">{{ text }}</p><p v-for="text in comparison.descriptions" :key="text">{{ text || 'Без описания' }}</p></div>
      <button v-if="editing" :disabled="busy || !ready || !!pending || !validTitle || comparison.state !== 'active'" @click="applyDraft">Сохранить мой текст вместо показанного</button>
      <button v-else :disabled="busy || !!pending" @click="snapshot = comparison; comparison = undefined; blocked = false; error = ''">Использовать показанную карточку</button>
    </section>
    <section v-if="forgetWarning" class="comparison-panel" role="alert">
      <p>Убирается только запись о проверке. Уже принятое изменение не отменяется. Новый ввод останется черновиком; перед новым действием сверьте сохранённую карточку.</p>
      <button :disabled="busy" @click="forget">Подтвердить удаление записи</button><button @click="forgetWarning = false">Оставить запись</button>
    </section>
    <section v-if="warning" class="comparison-panel" role="alert">
      <h3>Редактирование ещё не завершено</h3><p>Не все действия подтверждены. Последняя сохранённая запись остаётся. Можно вернуться к редактированию или закрыть его с черновиком на этом устройстве.</p>
      <button ref="stayButton" @click="warning = false; emit('stay')">Остаться в редакторе</button><button data-editor-keep-draft :disabled="busy || storageError" @click="keepAndClose">Закрыть, сохранив ввод</button>
    </section>
    <button v-if="editing" class="draft-discard" :disabled="busy" @click="discardWarning = true">Удалить черновик</button>
    <section v-if="discardWarning" class="comparison-panel" role="alert">
      <p>Несохранённый ввод будет удалён с этого устройства. Сохранённая карточка и уже отправленные действия останутся.</p>
      <button :disabled="busy" @click="discard">Подтвердить удаление черновика</button><button @click="discardWarning = false">Оставить черновик</button>
    </section>
  </div>
</template>
