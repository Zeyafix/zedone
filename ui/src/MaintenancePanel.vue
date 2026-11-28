<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { DialogRoot, DialogPortal, DialogOverlay, DialogContent, DialogTitle, DialogDescription, DialogClose } from "reka-ui";
import { inspectMaintenance, pendingMaintenance, prepareMaintenance, deliverMaintenance, forgetMaintenance, chooseMaintenanceFolder, newMaintenanceKey, maintenanceError, type MaintenanceView, type MaintenanceCommand } from "./maintenance-api";

const props = defineProps<{ settings: boolean }>();
const emit = defineEmits<{ mode: [ready: boolean] }>();
const state = ref<MaintenanceView>();
const busy = ref(false);
const error = ref("");
const notice = ref("");
const pending = ref<string[]>([]);
const selectedCopy = ref("");
const open = ref(false);
const confirmed = ref(false);
const action = ref<MaintenanceCommand>();
const key = ref("");
const verifier = ref<number[]>([]);
const label = ref("");
const writable = ref(false);
const keySaved = ref(false);
const blocked = computed(() => busy.value || pending.value.length > 0);
const visible = computed(() => state.value && (!state.value.normal_ready || props.settings));
const phase = computed(() => state.value?.plan?.phase);
const interrupted = computed(() => phase.value === "applying" || phase.value === "reverting");
const heading = computed(() => state.value?.normal_ready ? "Аварийное восстановление" : "Служебный режим");
const reason = computed(() => {
  const value = state.value?.reason;
  if (value === "InvalidDatabase") return "Целостность рабочей базы не подтверждена.";
  if (value === "UnsupportedSchema") return "Эта версия ZeDone не может открыть схему рабочей базы.";
  if (value === "RetryRequired") return "Обычный запуск остановлен до завершения служебного действия.";
  if (value === "Unavailable" || value === "OutcomeUnknown") return "Не удалось подтвердить готовность рабочей базы. Проверьте доступность файлов и свободное место.";
  return value;
});
const date = (ms: number) => new Date(ms).toLocaleString();
const confirmationTitle = computed(() => {
  switch (action.value?.action) {
    case "apply": return "Восстановить всю базу?";
    case "rollback": return "Вернуть проблемное состояние?";
    case "continue": return phase.value === "reverting" ? "Продолжить возврат состояния?" : "Продолжить прерванное восстановление?";
    case "enter": return "Остановить обычную работу?";
    case "delete_problem": return "Удалить сохранённое состояние?";
    case "revoke_access": return "Отозвать служебный доступ?";
    default: return "Подтвердить действие";
  }
});
async function refresh() {
  state.value = await inspectMaintenance();
  pending.value = await pendingMaintenance();
  emit("mode", state.value.normal_ready);
}
async function load() { busy.value = true; error.value = ""; try { await refresh(); } catch (cause) { if (state.value) error.value = maintenanceError(cause); } finally { busy.value = false; } }
async function perform(command: MaintenanceCommand) {
  if (blocked.value) return false;
  let completed = false;
  busy.value = true; error.value = ""; notice.value = "";
  try {
    const id = await prepareMaintenance(command);
    await deliverMaintenance(id);
    open.value = false;
    notice.value = "Действие подтверждено. Сведения обновлены.";
    completed = true;
  } catch (cause) { error.value = maintenanceError(cause); }
  finally { try { await refresh(); } catch { pending.value = await pendingMaintenance(); } busy.value = false; }
  return completed;
}
function ask(command: MaintenanceCommand) { action.value = command; confirmed.value = false; open.value = true; }
async function repeat(id: string) {
  if (busy.value) return;
  busy.value = true; error.value = "";
  try { await deliverMaintenance(id); notice.value = "Исходное действие подтверждено."; } catch (cause) { error.value = maintenanceError(cause); }
  finally { try { await refresh(); } catch { pending.value = await pendingMaintenance(); } busy.value = false; }
}
async function forget(id: string) { await forgetMaintenance(id); pending.value = await pendingMaintenance(); notice.value = "Запрос убран из списка. Выполненное действие этим не отменяется."; }
async function selectFolder() {
  if (!state.value || blocked.value) return;
  busy.value = true;
  try { const root = await chooseMaintenanceFolder(); busy.value = false; if (root) await perform({ action: "select_folder", root, expected: state.value.version }); }
  catch (cause) { error.value = maintenanceError(cause); }
  finally { busy.value = false; }
}
async function generateKey() {
  try { const value = await newMaintenanceKey(); key.value = value.key; verifier.value = value.verifier; writable.value = false; keySaved.value = false; }
  catch (cause) { error.value = maintenanceError(cause); }
}
async function issue() {
  if (!state.value || !keySaved.value || !label.value.trim()) return;
  if (await perform({ action: "issue_access", label: label.value.trim(), verifier: verifier.value, writable: writable.value, expires_ms: null, expected: state.value.version })) { key.value = ""; verifier.value = []; }
}
onMounted(load);
</script>

<template>
  <section v-if="visible && state" class="settings-section maintenance-panel" :class="{ 'maintenance-failed': !state.normal_ready }" aria-labelledby="maintenance-title" :aria-busy="busy">
    <h2 id="maintenance-title">{{ heading }}</h2>
    <template v-if="!state.normal_ready">
      <p v-if="reason" role="status">{{ reason }}</p>
      <p>Работа с пространствами и обмен остановлены. Восстановление всей базы запускается только после вашего подтверждения.</p>
    </template>
    <p v-else>Восстановление всей базы заменяет данные этого устройства. Для него нужны отдельные служебные полномочия.</p>
    <p v-if="error" class="preference-error" role="alert">{{ error }}</p>
    <p v-if="notice" role="status">{{ notice }}</p>
    <div v-for="id in pending" :key="id" class="maintenance-pending">
      <p>Есть незавершённое служебное действие. Его результат пока не подтверждён.</p>
      <button :disabled="busy" @click="repeat(id)">Повторить исходную команду</button>
      <button :disabled="busy" @click="forget(id)">Убрать запрос из списка</button>
      <p>Удаление запроса не отменяет выполненное действие. Для прерванной замены остаётся явное продолжение ниже.</p>
    </div>
    <div class="maintenance-actions">
      <button :disabled="busy" @click="load">Обновить сведения</button>
      <button v-if="state.normal_ready && state.writable" :disabled="blocked" @click="ask({ action: 'enter', expected: state.version })">Перейти в служебный режим</button>
      <button v-if="!state.normal_ready && state.writable && !interrupted && phase !== 'prepared'" :disabled="blocked" @click="perform({ action: 'retry', expected: state.version })">{{ phase === 'restored' ? 'Открыть пространства' : 'Повторить запуск' }}</button>
    </div>

    <template v-if="!state.normal_ready">
      <div v-if="state.problems.length" class="maintenance-preserved">
        <h3>Проблемное состояние сохранено отдельно</h3>
        <p>Эти файлы сохраняют состояние до замены и не считаются исправной резервной копией.</p>
        <div v-for="(problem, index) in state.problems" :key="problem" class="maintenance-artifact">
          <span>Сохранённое состояние {{ index + 1 }}</span>
          <button v-if="state.writable && !(state.plan?.problem === problem && (phase === 'prepared' || interrupted))" :disabled="blocked" @click="ask({ action: 'delete_problem', problem, expected: state.version })">Удалить сохранённое состояние {{ index + 1 }}</button>
        </div>
      </div>
      <template v-if="interrupted && state.plan">
        <h3>Служебная операция прервана</h3>
        <p>Выбранная копия: {{ date(state.plan.created_at_unix_ms) }}, пространств: {{ state.plan.spaces }}. Обычная работа останется закрыта до завершения операции.</p>
        <button v-if="state.owner" :disabled="blocked" @click="ask({ action: 'continue', plan: state.plan.id, expected: state.version })">{{ phase === 'reverting' ? 'Продолжить возврат состояния' : 'Продолжить восстановление' }}</button>
      </template>
      <template v-else-if="phase === 'prepared' && state.plan">
        <h3>Копия проверена и подготовлена</h3>
        <p>{{ date(state.plan.created_at_unix_ms) }} · пространств: {{ state.plan.spaces }}. Рабочая база пока не заменена.</p>
        <div class="maintenance-actions">
          <button v-if="state.writable" :disabled="blocked" @click="ask({ action: 'apply', plan: state.plan.id, expected: state.version })">Восстановить базу</button>
          <button v-if="state.writable" :disabled="blocked" @click="perform({ action: 'cancel', plan: state.plan.id, expected: state.version })">Отменить подготовку</button>
        </div>
      </template>
      <template v-else>
        <h3>Полная предмиграционная копия</h3>
        <p v-if="state.folder" class="maintenance-folder">{{ state.folder }}</p>
        <button v-if="state.owner" :disabled="blocked" @click="selectFolder">Выбрать папку с полной копией</button>
        <p v-if="!state.folder_available">Папка недоступна. Выберите папку с полной предмиграционной копией.</p>
        <p v-else-if="!state.copies.length">Полные предмиграционные копии в этой папке не найдены. Снимки отдельных пространств подходят для обычного восстановления пространства.</p>
        <fieldset v-if="state.copies.length" :disabled="blocked">
          <legend>Выберите копию</legend>
          <label v-for="copy in state.copies" :key="copy.id" class="access-choice"><input v-model="selectedCopy" type="radio" name="full-copy" :value="copy.id" /><span>{{ date(copy.created_at_unix_ms) }}, пространств: {{ copy.spaces }}</span></label>
        </fieldset>
        <p v-if="state.copies.length">Перед восстановлением проверим целостность полной копии и всех снимков внутри неё.</p>
        <button v-if="state.copies.length && state.writable" :disabled="blocked || !selectedCopy" @click="perform({ action: 'prepare', copy: selectedCopy, expected: state.version })">Проверить копию</button>
      </template>
      <div v-if="phase === 'restored' && state.plan && state.problems.includes(state.plan.problem)" class="maintenance-actions">
        <button v-if="state.writable" :disabled="blocked" @click="ask({ action: 'rollback', plan: state.plan.id, expected: state.version })">Вернуть проблемное состояние</button>
      </div>
    </template>

    <details v-if="state.owner && settings" class="maintenance-access">
      <summary>Служебный доступ для агента</summary>
      <p>Этот доступ действует на всю установку и хранится отдельно от рабочей базы. Чтение сведений не разрешает восстановление. Выдайте право восстановления только явно.</p>
      <p v-if="state.endpoint">Адрес API: <span class="maintenance-endpoint">{{ state.endpoint }}</span></p>
      <button :disabled="blocked" @click="generateKey">Подготовить ключ доступа</button>
      <form v-if="key" class="device-form" @submit.prevent="issue">
        <label class="name-field">Название доступа<input v-model="label" maxlength="80" required /></label>
        <label class="name-field">Ключ для агента<textarea :value="key" readonly spellcheck="false" rows="3" /></label>
        <p>Сохраните ключ перед выдачей. Он не попадёт в рабочую базу и не будет показан повторно после выдачи доступа.</p>
        <label class="access-choice"><input v-model="writable" type="checkbox" /><span>Разрешить восстановление всей базы и возврат проблемного состояния</span></label>
        <label class="access-choice"><input v-model="keySaved" type="checkbox" /><span>Ключ сохранён, передать этот служебный доступ</span></label>
        <button :disabled="blocked || !keySaved || !label.trim()">Выдать служебный доступ</button>
      </form>
      <ul class="maintenance-grants"><li v-for="grant in state.access" :key="grant.subject"><span>{{ grant.label }} — {{ grant.revoked ? 'отозван' : grant.writable ? 'восстановление разрешено' : 'только чтение сведений' }}</span><button v-if="!grant.revoked" :disabled="blocked" @click="ask({ action: 'revoke_access', subject: grant.subject, expected: state.version })">Отозвать {{ grant.label }}</button></li></ul>
    </details>

    <DialogRoot v-model:open="open"><DialogPortal><DialogOverlay class="dialog-overlay" /><DialogContent class="action-dialog maintenance-dialog">
      <DialogTitle>{{ confirmationTitle }}</DialogTitle>
      <DialogDescription v-if="action?.action === 'apply' || action?.action === 'continue' && phase !== 'reverting'">{{ reason }} Выбранная полная копия: {{ state.plan ? date(state.plan.created_at_unix_ms) : '' }}, пространств: {{ state.plan?.spaces }}. Будет заменена вся база этого устройства. Исходная копия и сохранённое проблемное состояние останутся отдельно. Прежние ключи будут отключены, подключения потребуется подтвердить заново. Обмен останется на паузе. Прежние файлы резервных копий сохранятся. Папку автоматических копий нужно будет выбрать заново.</DialogDescription>
      <DialogDescription v-else-if="action?.action === 'rollback' || action?.action === 'continue' && phase === 'reverting'">Вместо восстановленной базы вернутся сохранённые проблемные файлы. Обычная работа и обмен останутся остановлены до отдельного успешного запуска или нового восстановления.</DialogDescription>
      <DialogDescription v-else-if="action?.action === 'enter'">Работа с пространствами и обмен будут остановлены. Текущее состояние будет сохранено отдельно. Восстановление затем потребует отдельного подтверждения.</DialogDescription>
      <DialogDescription v-else-if="action?.action === 'delete_problem'">Сохранённые проблемные файлы будут удалены. Вернуть это состояние через ZeDone после удаления не получится. Рабочая база и исправные резервные копии останутся.</DialogDescription>
      <DialogDescription v-else>Агент больше не сможет выполнять служебные команды с этим ключом. Обычные права доступа изменяются отдельно.</DialogDescription>
      <label class="access-choice"><input v-model="confirmed" type="checkbox" /><span>Подтверждаю это действие</span></label>
      <p v-if="error" class="preference-error" role="alert">{{ error }}</p>
      <div class="maintenance-actions"><button :disabled="!confirmed || blocked" @click="action && perform(action)">{{ action?.action === 'apply' ? 'Восстановить всю базу' : 'Подтвердить действие' }}</button><DialogClose :disabled="busy">Отмена</DialogClose></div>
    </DialogContent></DialogPortal></DialogRoot>
  </section>
</template>
