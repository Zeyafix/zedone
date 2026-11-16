<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { DialogRoot, DialogPortal, DialogOverlay, DialogContent, DialogTitle, DialogDescription, DialogClose } from "reka-ui";
import type { components } from "../../api/schema";
import { localCredential } from "./connection";
import { deviceCall, deviceError, type HostStatus, type SessionStatus, type HostOperation } from "./device-api";
type Pairing = components["schemas"]["PairingSnapshot"];
const props = defineProps<{ ready: boolean }>();
type Outgoing = { offer: string | null; invitation: components["schemas"]["InvitationSnapshot"] | null };
type Change = { subject: string; operation: "session.cancel" | "session.unlink"; parameters: { id: number[]; expected: number[] } };
const storageKey="zedone:device-change:v1";
const host=ref<HostStatus>(), session=ref<SessionStatus>(), remote=ref<Pairing>(), outgoing=ref<Outgoing>();
const address=ref(""), peerAddress=ref(""), code=ref(""), label=ref(""), busy=ref(false), error=ref(""), notice=ref("");
const change=ref<Change>(), storageFailed=ref(false), confirmOpen=ref(false), requested=ref<"session.cancel"|"session.unlink"|"forget">();
let alive=true;
async function run(work:()=>Promise<void>) {
  if(busy.value || !props.ready)return;busy.value=true;error.value="";notice.value="";
  try{await work();}catch(cause){if(alive)error.value=deviceError(cause);}finally{if(alive)busy.value=false;}
}
function loadChange() {
  try {
    const text=localStorage.getItem(storageKey);
    if(!text){change.value=undefined;storageFailed.value=false;return;}
    const value=JSON.parse(text) as Change;
    if(!/^[a-f0-9]{32}$/.test(value.subject) || !["session.cancel","session.unlink"].includes(value.operation)
      || !Array.isArray(value.parameters?.id) || value.parameters.id.length!==16 || !value.parameters.id.some(b=>b!==0)
      || !Array.isArray(value.parameters.expected) || value.parameters.expected.length!==32
      || ![...value.parameters.id,...value.parameters.expected].every(b=>Number.isInteger(b)&&b>=0&&b<=255))throw new Error("invalid_request");
    change.value=value;storageFailed.value=false;
  } catch{storageFailed.value=true;error.value="Не удалось прочитать исходное действие. Сохранённая запись не изменена; новые действия недоступны.";}
}
async function status() {
  const value=await deviceCall<SessionStatus>("session.status");
  if(!["unconfigured","pending","connected"].includes(value.status))throw new Error("unavailable");
  if(alive)session.value=value;
}
async function refresh() {
  await run(async()=>{
    loadChange();
    const results=await Promise.allSettled([deviceCall<HostStatus>("host.status"),deviceCall<SessionStatus>("session.status"),deviceCall<Outgoing>("offer.read")]);
    if(!alive)return;
    const [h,s,o]=results;
    host.value=h.status==="fulfilled"?h.value:undefined;
    session.value=s.status==="fulfilled"?s.value:undefined;
    outgoing.value=o.status==="fulfilled"?o.value:undefined;
    remote.value=undefined;
    const failed=results.find(result=>result.status==="rejected");if(failed?.status==="rejected")throw failed.reason;
  });
}
async function listener(operation:"host.start"|"host.stop") {
  await run(async()=>{
    const result=await deviceCall<HostStatus>(operation,operation==="host.start"?{address:address.value.trim()}:{});
    if(alive)host.value=result;
  });
}
async function invite() { await run(async()=>{const value=await deviceCall<Outgoing>("offer.create");if(alive)outgoing.value=value;}); }
async function submit() {
  if(change.value || storageFailed.value)return;
  await run(async()=>{
    await status();
    if(session.value?.status==="unconfigured") {
      await deviceCall("session.prepare",{address:peerAddress.value.trim(),offer:code.value.trim(),label:label.value.trim()});
      if(alive)code.value="";
      await status();
    }
    const value=await deviceCall<Pairing>("session.submit");
    if(alive){remote.value=value;code.value="";}
    await status();
  });
}
async function inspect() {
  await run(async()=>{
    const result=await deviceCall<Pairing>("session.refresh");
    if(alive)remote.value=result;
    await status();
  });
}
async function prepareChange(operation:"session.cancel"|"session.unlink") {
  if(change.value || storageFailed.value)return;
  await run(async()=>{
    const result=await deviceCall<Pairing>("session.refresh");
    if(!alive)return;
    remote.value=result;await status();
    if(operation==="session.cancel"&&result.status!=="pending" || operation==="session.unlink"&&result.status!=="confirmed")throw new Error("stale_state");
    requested.value=operation;confirmOpen.value=true;
  });
}
async function deliverChange() {
  const original=change.value;if(!original)return;
  if((await localCredential()).subject!==original.subject)throw new Error("access_denied");
  await deviceCall<Pairing>(original.operation,original.parameters);
  localStorage.removeItem(storageKey);loadChange();
  const result=await deviceCall<Pairing>("session.refresh");if(alive)remote.value=result;
  await status();
}
async function confirmChange() {
  if(!requested.value||storageFailed.value)return;
  if(requested.value==="forget") {
    await run(async()=>{await locked(async()=>{localStorage.removeItem(storageKey);loadChange();confirmOpen.value=false;await status();});});
    return;
  }
  if(!remote.value||change.value)return;
  await run(async()=>{
    await locked(async()=>{
    loadChange(); if(change.value || storageFailed.value)throw new Error("pending_exists");
    const original:Change={subject:(await localCredential()).subject,operation:requested.value as Change["operation"],parameters:{id:Array.from(crypto.getRandomValues(new Uint8Array(16))),expected:[...remote.value!.version]}};
    const text=JSON.stringify(original);localStorage.setItem(storageKey,text);
    if(localStorage.getItem(storageKey)!==text)throw new Error("unavailable");
    loadChange();confirmOpen.value=false;await deliverChange();
    });
  });
}
async function locked(work:()=>Promise<void>){
  if(!navigator.locks)throw new Error("unavailable");
  await navigator.locks.request(storageKey,{ifAvailable:true},async lock=>{if(!lock)throw new Error("pending_exists");await work();});
}
async function retry(){await run(()=>locked(async()=>{loadChange();await deliverChange();}));}
const states={unconfigured:"Нет сохранённого подключения",pending:"Запрос сохранён на устройстве",connected:"Подтверждённое подключение сохранено"};
const remoteStates:Record<Pairing["status"],string>={pending:"Ожидаем подтверждения на другом устройстве. Доступа к данным и обмена пока нет.",confirmed:"Подключение подтверждено. Обмен пространствами включается отдельно.",cancelled:"Подключение отменено. Для нового запроса создайте новое приглашение.",rejected:"Запрос отклонён. Для нового запроса создайте новое приглашение.",unlinked:"Устройство отвязано. Локальные пространства и история сохранены."};
function changed(){loadChange();}
onMounted(()=>{window.addEventListener("storage",changed);void refresh();});
onUnmounted(()=>{alive=false;code.value="";outgoing.value=undefined;window.removeEventListener("storage",changed);});
</script>

<template>
  <section class="settings-section devices-panel" aria-labelledby="devices-title">
    <div class="view-toolbar"><h2 id="devices-title">Подключение устройств</h2><button :disabled="!ready || (busy)" @click="refresh">Обновить подключения</button></div>
    <p>Для ручного подключения нужен доступный сетевой адрес другого устройства и одноразовый код. Передавайте код доверенным способом. Подключение требует подтверждения на приглашающем устройстве.</p>
    <p v-if="error" class="form-error" role="alert">{{ error }}</p><p v-if="notice" role="status">{{ notice }}</p>
    <h3>Пригласить другое устройство</h3>
    <p>Укажите адрес этого устройства в Tailscale и порт. Приём работает, пока приложение открыто; закрытое приложение не принимает подключения.</p>
    <form class="device-form" @submit.prevent="listener('host.start')"><label class="name-field" for="listen-address">Адрес для приёма (IP:порт)<input id="listen-address" v-model="address" :disabled="!ready || (busy || host?.listening)" required placeholder="100.64.0.1:45821" autocomplete="off" spellcheck="false"/></label>
      <div class="view-actions"><button v-if="!host?.listening" type="submit" :disabled="!ready || (busy)">Начать приём подключений</button><button v-else type="button" :disabled="!ready || (busy)" @click="listener('host.stop')">Остановить приём подключений</button></div>
    </form>
    <p v-if="host" data-listener-state>{{ host.listening?`Приём включён: ${host.address}`:'Приём подключений выключен' }}</p>
    <button :disabled="!ready || (busy || !host?.listening)" @click="invite">Создать или показать приглашение</button>
    <div v-if="outgoing?.invitation" class="invitation-code"><p>Приглашение: {{ outgoing.invitation.invitation }}. Срок: {{ new Date(outgoing.invitation.expires_at_unix_ms).toLocaleString() }}.</p>
      <template v-if="outgoing.offer"><label class="name-field" for="outgoing-code">Одноразовый код для другого устройства<textarea id="outgoing-code" :value="outgoing.offer" readonly rows="4" autocomplete="off" spellcheck="false" /></label><p>Код действует десять минут и используется один раз. После отправки запроса сверьте его ID и подтвердите права в разделе «Запросы и доступы».</p></template>
      <p v-else>Приглашение больше не открыто. Для нового подключения создайте новое приглашение.</p>
    </div>
    <h3>Подключиться по коду</h3>
    <p v-if="session" data-session-state>{{ states[session.status] }}<span v-if="session.request">. ID запроса: {{ session.request }}</span><span v-if="session.address">. Адрес: {{ session.address }}</span></p>
    <p v-if="remote" role="status" data-remote-state>{{ remoteStates[remote.status] }}</p>
    <p v-if="session?.status==='connected'" class="object-note">Сохранённое подтверждение не заменяет проверку текущих прав: истечение или отзыв доступа запрещают действия.</p>
    <form v-if="session?.status==='unconfigured'" class="device-form" @submit.prevent="submit">
      <label class="name-field" for="peer-address">Адрес другого устройства (IP:порт)<input id="peer-address" v-model="peerAddress" :disabled="!ready || (busy || !!change || storageFailed)" required autocomplete="off" spellcheck="false"/></label>
      <label class="name-field" for="peer-code">Одноразовый код<input id="peer-code" v-model="code" :disabled="!ready || (busy || !!change || storageFailed)" required autocomplete="off" spellcheck="false"/></label>
      <label class="name-field" for="peer-label">Название этого устройства<input id="peer-label" v-model="label" :disabled="!ready || (busy || !!change || storageFailed)" required maxlength="80" autocomplete="off"/></label>
      <button type="submit" :disabled="!ready || (busy || !!change || storageFailed)">Запросить подключение</button>
    </form>
    <div v-if="session && session.status!=='unconfigured'" class="view-actions">
      <button v-if="session.status==='pending'" :disabled="!ready || (busy || !!change || storageFailed)" @click="submit">Отправить сохранённый запрос</button>
      <button :disabled="!ready || (busy)" @click="inspect">Проверить состояние на другом устройстве</button>
      <button v-if="session.status==='pending'" :disabled="!ready || (busy || !!change || storageFailed)" @click="prepareChange('session.cancel')">Отменить подключение</button>
      <button v-if="session.status==='connected'" :disabled="!ready || (busy || !!change || storageFailed)" @click="prepareChange('session.unlink')">Отвязаться от устройства</button>
    </div>
    <div v-if="change"><p>Результат исходного действия не подтверждён. Его данные сохранены без кода и ключей.</p><button :disabled="!ready || (busy)" @click="retry">Повторить исходное действие подключения</button><button :disabled="!ready || (busy)" @click="requested='forget';confirmOpen=true">Убрать действие подключения из списка</button></div>
    <DialogRoot v-model:open="confirmOpen"><DialogPortal><DialogOverlay class="dialog-overlay"/><DialogContent class="action-dialog" @escape-key-down="event=>{if(busy)event.preventDefault()}" @pointer-down-outside="event=>{if(busy)event.preventDefault()}">
      <DialogTitle>{{ requested==='forget'?'Убрать из списка':requested==='session.cancel'?'Отменить подключение':'Отвязаться от устройства' }}</DialogTitle><DialogDescription>{{ requested==='forget'?'Удаляется только запись доставки. Выполненная операция этим не отменяется. Сначала проверьте текущее состояние подключения.':'Доступ этого подключения будет прекращён. Локальные пространства и история сохранятся. Для повторного подключения понадобятся новые приглашение и ключ.' }}</DialogDescription>
      <p>ID запроса: {{ remote?.request }}</p><div class="dialog-actions"><DialogClose :disabled="!ready || (busy)">Закрыть</DialogClose><button :disabled="!ready || (busy)" @click="confirmChange">Подтвердить</button></div>
    </DialogContent></DialogPortal></DialogRoot>
  </section>
</template>
