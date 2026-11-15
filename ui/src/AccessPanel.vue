<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { DialogRoot, DialogPortal, DialogOverlay, DialogContent, DialogTitle, DialogDescription, DialogClose } from "reka-ui";
import type { components } from "../../api/schema";
import type { Parameters } from "../../api/validate-response";
import { createMutation, deliverMutation, forgetMutation, mutationCredential, pendingCommands, pendingChanged, read, type AdministrationMutation, type Pending, type Space } from "./workspace-api";
import { deviceError } from "./device-api";
import AccessScopeForm from "./AccessScopeForm.vue";
const accessError = (cause: unknown) => cause instanceof Error && cause.message === "invalid_request"
  ? "Проверьте ID, проверочное значение нового ключа, пространства и срок. Действие не подтверждено."
  : cause instanceof Error && cause.message === "access_denied"
    ? "Нет действующих административных прав или действие запрещено. Последнего бессрочного администратора нельзя отозвать или ограничить."
    : deviceError(cause);
type S = components["schemas"];
const props = defineProps<{ ready: boolean }>();
type Action = { kind: "forget"; id: string } | { kind: "access.issue" } | { kind: "access.replace" | "access.revoke"; target: S["AccessSnapshot"] }
  | { kind: "pairing.confirm" | "pairing.reject" | "pairing.unlink"; target: S["PairingSnapshot"] }
  | { kind: "invitation.cancel"; target: S["InvitationSnapshot"] };
const busy = ref(false), error = ref(""), notice = ref(""), open = ref(false), refused = ref(false);
const accesses = ref<S["AccessSnapshot"][]>([]), invitations = ref<S["InvitationSnapshot"][]>([]), requests = ref<S["PairingSnapshot"][]>([]);
const accessNext = ref<string | null>(), invitationNext = ref<string | null>(), requestNext = ref<string | null>();
const spaces = ref<Space[]>([]), pending = ref<Pending[]>([]), action = ref<Action>();
const subject = ref(""), verifier = ref(""), confirmed = ref(false), formError = ref("");
const scope = ref<InstanceType<typeof AccessScopeForm>>();
const actionSubject = ref<string>();
const labels: Record<AdministrationMutation | "forget", string> = { forget: "Убрать из списка", "access.issue": "Выдать доступ", "access.replace": "Заменить доступ новым ключом", "access.revoke": "Отозвать доступ", "pairing.confirm": "Подтвердить подключение", "pairing.reject": "Отклонить запрос", "pairing.unlink": "Отвязать устройство", "invitation.cancel": "Отменить приглашение" };
const scopeRequired = computed(() => action.value && ["access.issue", "access.replace", "pairing.confirm"].includes(action.value.kind));
const enrollment = computed(() => action.value && ["access.issue", "access.replace"].includes(action.value.kind));
const blocked = computed(() => !props.ready || busy.value || pending.value.length > 0);
let alive = true;
async function loadPending() {
  try { const value = await pendingCommands(); if (alive) pending.value = value; }
  catch { if (alive) error.value = "Не удалось прочитать незавершённые действия. Новые действия недоступны."; pending.value = [{ id: "", intention: { subject: "", operation: "access.issue", body: "" } }]; }
}
async function refresh() {
  if (busy.value || !props.ready) return;
  busy.value = true; error.value = "";
  try {
    const results = await Promise.allSettled([read("access.list", { after: null }), read("invitation.list", { after: null }), read("pairing.list", { after: null }), read("space.list", { include_trashed: false })]);
    if (!alive) return;
    const [a,i,p,s] = results;
    if (a.status === "fulfilled") { accesses.value = a.value.entries; accessNext.value = a.value.next; } else { accesses.value=[]; accessNext.value=null; }
    if (i.status === "fulfilled") { invitations.value=i.value.entries; invitationNext.value=i.value.next; } else { invitations.value=[]; invitationNext.value=null; }
    if (p.status === "fulfilled") { requests.value=p.value.entries; requestNext.value=p.value.next; } else { requests.value=[]; requestNext.value=null; }
    spaces.value = s.status === "fulfilled" ? s.value.spaces : [];
    const failed=results.find(result=>result.status==="rejected");
    if (failed?.status === "rejected") error.value=accessError(failed.reason);
    await loadPending();
  } finally { if (alive) busy.value=false; }
}
async function more(kind: "access" | "invitation" | "pairing") {
  if (busy.value) return; busy.value=true; error.value="";
  try {
    if (kind === "access") { const result=await read("access.list",{after:accessNext.value}); if(alive){accesses.value.push(...result.entries);accessNext.value=result.next;} }
    if (kind === "invitation") { const result=await read("invitation.list",{after:invitationNext.value}); if(alive){invitations.value.push(...result.entries);invitationNext.value=result.next;} }
    if (kind === "pairing") { const result=await read("pairing.list",{after:requestNext.value}); if(alive){requests.value.push(...result.entries);requestNext.value=result.next;} }
  } catch(cause) { if(alive)error.value=accessError(cause); } finally { if(alive)busy.value=false; }
}
async function begin(next: Action) {
  if (!props.ready || busy.value || blocked.value && next.kind !== "forget") return;
  busy.value=true; error.value="";
  try {
    // Freeze a fresh version at opening; later reads never rebase the action.
    const subjectAtOpening = (await mutationCredential()).subject;
    if (next.kind === "access.replace" || next.kind === "access.revoke") next={...next,target:await read("access.inspect",{subject:next.target.subject})};
    else if (next.kind === "invitation.cancel") next={...next,target:await read("invitation.inspect",{invitation:next.target.invitation})};
    else if ("target" in next && "request" in next.target) next={kind: next.kind as "pairing.confirm" | "pairing.reject" | "pairing.unlink",target:await read("pairing.inspect",{request:next.target.request})};
    if(!alive)return;
    action.value=next; actionSubject.value=subjectAtOpening; subject.value=next.kind==="access.replace"?next.target.subject:""; verifier.value="";
    confirmed.value=false; refused.value=false; formError.value=""; open.value=true;
  } catch(cause) { if(alive)error.value=accessError(cause); } finally { if(alive)busy.value=false; }
}
async function finish(id: string) {
  const result=await deliverMutation(id); await loadPending();
  if (!alive)return;
  if (result.status === "pending") {
    const code=result.response?.status === "error" ? result.response.error.code : "outcome_unknown";
    error.value=accessError(new Error(code));
    if(open.value){formError.value=error.value;refused.value=true;}
    return;
  }
  if (result.response.status !== "ok") { refused.value=true; formError.value=accessError(new Error(result.response.error.code)); if(!open.value)error.value=formError.value; return; }
  open.value=false; notice.value="Действие подтверждено. Список обновлён.";
}
async function execute<O extends AdministrationMutation>(operation: O, parameters: Parameters[O]) {
  await finish(await createMutation(operation,parameters,actionSubject.value));
}
async function submit() {
  const current=action.value;
  if (!props.ready || !current || busy.value || blocked.value && current.kind !== "forget" || !confirmed.value || refused.value)return;
  busy.value=true; formError.value="";
  try {
    switch(current.kind) {
      case "forget": await forgetMutation(current.id); await loadPending(); open.value=false; notice.value="Исходное действие убрано из списка. Выполненная операция этим не отменяется."; break;
      case "access.issue": case "access.replace": {
        if (!/^[a-f0-9]{32}$/.test(subject.value) || !/^[a-f0-9]{64}$/.test(verifier.value) || !scope.value?.valid)throw new Error("invalid_request");
        const bytes=Array.from({length:32},(_,i)=>parseInt(verifier.value.slice(i*2,i*2+2),16));
        const parameters={subject:subject.value,verifier:bytes,...scope.value.value()};
        if(current.kind==="access.issue")await execute(current.kind,parameters);
        else await execute(current.kind,{...parameters,expected:current.target.version});
        break;
      }
      case "pairing.confirm": if(scope.value?.valid) await execute(current.kind,{request:current.target.request,expected:current.target.version,...scope.value.value()}); break;
      case "access.revoke": await execute(current.kind,{subject:current.target.subject,expected:current.target.version});break;
      case "invitation.cancel": await execute(current.kind,{invitation:current.target.invitation,expected:current.target.version});break;
      case "pairing.reject": case "pairing.unlink": await execute(current.kind,{request:current.target.request,expected:current.target.version});break;
    }
  } catch(cause) { if(alive)formError.value=accessError(cause); }
  finally { if(alive){busy.value=false; if(!open.value)await refresh();} }
}
async function retry(id: string) {
  if(busy.value || !id)return; busy.value=true; error.value="";
  try {await finish(id);}catch(cause){if(alive)error.value=accessError(cause);}
  finally {if(alive){busy.value=false;await refresh();}}
}
function scopeText(value: S["AccessScope"]) {
  const parts=value.kind!=="spaces"?["Администрирование"]:[];
  if(value.kind!=="administration") for(const [id,mode]of Object.entries(value.spaces))parts.push(`${spaces.value.find(space=>space.space===id)?.names.join(' / ') || id}: ${mode==="read"?"чтение":"чтение и изменение"}`);
  return parts.join("; ");
}
const date=(value?:number|null)=>value==null?"До отзыва":new Date(value).toLocaleString();
const requestLabels:Record<S["PairingStatus"],string>={pending:"Ожидает подтверждения",confirmed:"Подключено",cancelled:"Отменено",rejected:"Отклонено",unlinked:"Отвязано"};
const invitationLabels:Record<S["InvitationStatus"],string>={open:"Можно подключиться",expired:"Срок истёк",cancelled:"Отменено",used:"Код использован",locked:"Заблокировано после неверных попыток"};
onMounted(()=>{window.addEventListener(pendingChanged,loadPending);void refresh();});
onUnmounted(()=>{alive=false;window.removeEventListener(pendingChanged,loadPending);});
</script>

<template>
  <section class="settings-section access-panel" aria-labelledby="device-access-title">
    <div class="view-toolbar"><h2 id="device-access-title">Запросы и доступы</h2><button :disabled="!ready || (busy)" @click="refresh">Обновить доступы</button></div>
    <p>Управление требует отдельных административных прав. Подключение само по себе не включает обмен пространствами.</p>
    <p v-if="error" role="alert" class="form-error">{{ error }}</p><p v-if="notice" role="status">{{ notice }}</p>
    <div v-if="pending.length" class="pending-actions">
      <h3>Незавершённые действия</h3><p>Повторяется исходное действие с прежними данными и правами.</p>
      <ul class="scope-list"><li v-for="item in pending" :key="item.id">{{ item.intention.operation }} <button :disabled="!ready || (busy || !item.id)" @click="retry(item.id)">Повторить исходное действие</button> <button :disabled="!ready || (busy || !item.id)" @click="begin({kind:'forget',id:item.id})">Убрать из списка</button></li></ul>
    </div>
    <h3>Запросы устройств</h3>
    <p v-if="!requests.length && !busy && !error">Запросов пока нет.</p>
    <ul class="scope-list"><li v-for="item in requests" :key="item.request" :data-request="item.request">
      <h4>{{ item.label }}</h4><p>{{ requestLabels[item.status] }}</p><p class="object-note">ID запроса: {{ item.request }}</p>
      <div class="view-actions"><template v-if="item.status==='pending'"><button :disabled="!ready || (blocked)" @click="begin({kind:'pairing.confirm',target:item})">Подтвердить подключение</button><button :disabled="!ready || (blocked)" @click="begin({kind:'pairing.reject',target:item})">Отклонить запрос</button></template>
      <button v-if="item.status==='confirmed'" :disabled="!ready || (blocked)" @click="begin({kind:'pairing.unlink',target:item})">Отвязать устройство</button></div>
    </li></ul><button v-if="requestNext" :disabled="!ready || (busy)" @click="more('pairing')">Ещё запросы</button>
    <h3>Приглашения</h3>
    <p v-if="!invitations.length && !busy && !error">Приглашений пока нет.</p>
    <ul class="scope-list"><li v-for="item in invitations" :key="item.invitation" :data-invitation="item.invitation"><p>{{ invitationLabels[item.status] }}. {{ item.invitation }}</p><p>Срок: {{ date(item.expires_at_unix_ms) }}</p>
      <button v-if="item.status!=='cancelled'" :disabled="!ready || (blocked)" @click="begin({kind:'invitation.cancel',target:item})">Отменить приглашение</button>
    </li></ul><button v-if="invitationNext" :disabled="!ready || (busy)" @click="more('invitation')">Ещё приглашения</button>
    <div class="view-toolbar"><h3>Доступы устройств и агентов</h3><button :disabled="!ready || (blocked)" @click="begin({kind:'access.issue'})">Выдать доступ</button></div>
    <ul class="scope-list"><li v-for="item in accesses" :key="item.subject" :data-subject="item.subject"><h4>{{ item.subject }}</h4>
      <p>{{ item.revoked?'Отозван':'Зарегистрирован' }}. {{ scopeText(item.scope) }}</p><p>Срок: {{ date(item.expires_at_unix_ms) }}</p>
      <p v-if="item.expires_at_unix_ms!=null" class="object-note">После указанного срока доступ прекращается; для продолжения нужна новая выдача.</p>
      <div class="view-actions"><button :disabled="!ready || (blocked)" @click="begin({kind:'access.replace',target:item})">Заменить доступ новым ключом</button><button v-if="!item.revoked" :disabled="!ready || (blocked)" @click="begin({kind:'access.revoke',target:item})">Отозвать доступ</button></div>
    </li></ul><button v-if="accessNext" :disabled="!ready || (busy)" @click="more('access')">Ещё доступы</button>
    <DialogRoot v-model:open="open"><DialogPortal><DialogOverlay class="dialog-overlay"/><DialogContent class="action-dialog access-dialog" @escape-key-down="event=>{if(busy)event.preventDefault()}" @pointer-down-outside="event=>{if(busy)event.preventDefault()}">
      <DialogTitle>{{ action?labels[action.kind]:'' }}</DialogTitle><DialogDescription>Проверьте получателя и выбранные права. Локальные пространства и история сохраняются при отмене и отзыве.</DialogDescription>
      <form @submit.prevent="submit">
        <p v-if="action?.kind==='forget'">Удаляется только запись доставки. Если прежняя попытка выполнилась, её результат сохранится. Сначала проверьте актуальное состояние; для обратного действия нужна отдельная команда.</p>
        <p v-if="action && 'target' in action">{{ 'label' in action.target?action.target.label:'subject' in action.target?action.target.subject:action.target.invitation }}</p>
        <template v-if="enrollment"><p>Получатель сохраняет новый ключ у себя и передаёт только ID и проверочное значение. Не вводите сюда секретный ключ.</p>
          <label class="name-field" for="access-subject">ID получателя<input id="access-subject" v-model="subject" :readonly="action?.kind==='access.replace'" :disabled="!ready || (busy || refused)" required pattern="[a-f0-9]{32}" autocomplete="off" spellcheck="false"/></label>
          <label class="name-field" for="access-verifier">Проверочное значение нового ключа (64 hex)<input id="access-verifier" v-model="verifier" :disabled="!ready || (busy || refused)" required pattern="[a-f0-9]{64}" autocomplete="off" spellcheck="false"/></label>
        </template>
        <AccessScopeForm v-if="scopeRequired" ref="scope" :spaces="spaces" prefix="grant" :disabled="!ready || (busy || refused)"/>
        <label class="access-choice"><input v-model="confirmed" data-confirm-access type="checkbox" :disabled="!ready || (busy || refused)"/>Подтверждаю это действие и выбранные права</label>
        <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>
        <p v-if="refused">Закройте форму, обновите данные и подготовьте новое действие. Введённые значения остаются здесь до закрытия.</p>
        <div class="dialog-actions"><DialogClose type="button" :disabled="!ready || (busy)">Закрыть</DialogClose><button data-access-submit type="submit" class="primary-button" :disabled="!ready || (busy || blocked && action?.kind!=='forget' || refused || !confirmed || !!scopeRequired && !scope?.valid)">{{ action?labels[action.kind]:'' }}</button></div>
      </form>
    </DialogContent></DialogPortal></DialogRoot>
  </section>
</template>
