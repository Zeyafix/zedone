<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import type { components } from "../../api/schema";
import { deviceCall, type SessionStatus } from "./device-api";
import { read, createMutation, deliverMutation, pendingCommands, pendingChanged, forgetMutation, errorMessage, type Pending, type Space } from "./workspace-api";
type S=components["schemas"];
const props=defineProps<{ready:boolean}>();
const spaces=ref<Space[]>([]), pairings=ref<S["PairingSnapshot"][]>([]), session=ref<SessionStatus>();
const pairing=ref(""), space=ref(""), control=ref<S["SyncControl"]>();
const catalog=ref<S["SyncCatalog"]>(), remoteControls=ref<Record<string,S["SyncControl"]>>({});
const busy=ref(false), error=ref(""), notice=ref(""), pending=ref<Pending[]>([]);
const peerNext=ref<string|null>(null);
let alive=true, generation=0;
const peers=computed(()=>{
  const entries=pairings.value.filter(p=>p.status==="confirmed").map(p=>({id:p.request,label:p.label}));
  if(session.value?.status==="connected"&&session.value.request&&!entries.some(p=>p.id===session.value!.request))entries.push({id:session.value.request,label:"Подключённое устройство"});
  return entries;
});
const rows=computed(()=>catalog.value?.spaces.map(entry=>({entry,c:remoteControls.value[entry.space]}))??[]);
const blocked=computed(()=>!props.ready||busy.value||pending.value.length>0);
async function loadPending(){try{const value=await pendingCommands();if(alive)pending.value=value;}catch{if(alive){error.value="Не удалось проверить незавершённые действия. Новые действия недоступны.";pending.value=[{id:"",intention:{subject:"",operation:"sync.select",body:""}}];}}}
async function work(action:()=>Promise<void>){if(busy.value||!props.ready)return;busy.value=true;error.value="";try{await action();}catch(cause){if(alive)error.value=errorMessage(cause);}finally{if(alive)busy.value=false;}}
async function inspect(){
  const version=++generation;control.value=undefined;
  if(!space.value||!pairing.value)return;
  try{const c=await read("sync.inspect",{space:space.value,pairing:pairing.value});if(alive&&version===generation)control.value=c;}catch(cause){if(alive&&version===generation)error.value=errorMessage(cause);}
}
async function refresh(){await work(async()=>{
  await loadPending();
  const [local,requests,saved]=await Promise.all([read("space.list",{include_trashed:false}),read("pairing.list",{after:null}),deviceCall<SessionStatus>("session.status")]);
  if(!alive)return;spaces.value=local.spaces;pairings.value=requests.entries;peerNext.value=requests.next??null;session.value=saved;
  if(!pairing.value)pairing.value=peers.value[0]?.id??"";
  if(!space.value)space.value=local.spaces[0]?.space??"";
  await inspect();
});}
async function morePeers(){await work(async()=>{if(!peerNext.value)return;const page=await read("pairing.list",{after:peerNext.value});if(alive){pairings.value.push(...page.entries);peerNext.value=page.next??null;}});}
async function remoteSpaces(){await work(async()=>{
  const value=await deviceCall<S["SyncCatalog"]>("exchange.catalog");
  const values:Record<string,S["SyncControl"]>={};
  for(const entry of value.spaces)values[entry.space]=await read("sync.inspect",{space:entry.space,pairing:value.pairing});
  if(alive){catalog.value=value;remoteControls.value=values;}
});}
async function change(c:S["SyncControl"],kind:"select"|"pause"){
  await work(async()=>{
    const id=kind==="select"
      ?await createMutation("sync.select",{space:c.space,pairing:c.pairing,selected:!c.selected,expected:c.selection_version})
      :await createMutation("sync.pause",{space:c.space,pairing:c.pairing,paused:!c.paused,expected:c.pause_version});
    const reply=await deliverMutation(id);if(reply.status!=="completed")throw new Error("outcome_unknown");if(reply.response.status!=="ok")throw new Error(reply.response.error.code);
    if(!alive)return;
    notice.value=kind==="pause"?(c.paused?"Явная пауза снята.":"Обмен пространства приостановлен в обоих направлениях."):(c.selected?"Обмен с устройством выключен. Локальные данные сохранены.":"Пространство выбрано. Обмен начнётся при доступности устройства и разрешении на обеих сторонах.");
    await inspect();
    if(catalog.value){const fresh=await read("sync.inspect",{space:c.space,pairing:c.pairing});if(alive)remoteControls.value[c.space]=fresh;}
  });
}
const states:Record<string,string>={up_to_date:"Обмен подтверждён. Полученные изменения сохранены локально.",read_only:"Изменения получены. Доступ на другом устройстве разрешает только чтение: локальные правки туда не отправляются.",remote_paused:"На другом устройстве пространство на паузе.",remote_unselected:"На другом устройстве это пространство не выбрано для обмена.",in_progress:"Обмен продолжается. Остальные изменения будут получены при следующем запуске обмена."};
async function run(id:string){await work(async()=>{const result=await deviceCall<{state:string}>("exchange.run",{space:id});if(alive)notice.value=states[result.state]??"Проверьте состояние обмена.";});}
async function retry(id:string){await work(async()=>{const reply=await deliverMutation(id);if(reply.status!=="completed")throw new Error("outcome_unknown");if(reply.response.status!=="ok")throw new Error(reply.response.error.code);await inspect();await loadPending();});}
async function forget(id:string){await work(async()=>{await forgetMutation(id);await loadPending();notice.value="Убрана только запись доставки. Выполненное действие этим не отменяется.";});}
watch([space,pairing],()=>void inspect());
function changed(){void loadPending();}
onMounted(()=>{window.addEventListener(pendingChanged,changed);window.addEventListener("storage",changed);void refresh();});
onUnmounted(()=>{alive=false;generation++;window.removeEventListener(pendingChanged,changed);window.removeEventListener("storage",changed);});
</script>

<template>
  <section class="settings-section exchange-panel" aria-labelledby="exchange-title" :aria-busy="busy">
    <div class="view-toolbar"><h2 id="exchange-title">Обмен пространствами</h2><button :disabled="!ready||busy" @click="refresh">Обновить обмен</button></div>
    <p>Выберите пространства отдельно на обоих устройствах. При работающем приложении обмен запускается автоматически при доступности устройства. На Android работа в фоне зависит от системы.</p>
    <p>Локальное сохранение карточки и обмен — разные состояния. Пауза пространства останавливает входящий и исходящий обмен со всеми устройствами; локальная работа продолжается.</p>
    <p v-if="error" role="alert" class="form-error">{{ error }}</p><p v-if="notice" role="status" data-exchange-result>{{ notice }}</p>
    <button v-if="peerNext" :disabled="!ready||busy" @click="morePeers">Загрузить ещё устройства</button>
    <template v-if="peers.length">
      <h3>Выбор на этом устройстве</h3>
      <div class="device-form"><label class="name-field" for="exchange-peer">Устройство<select id="exchange-peer" v-model="pairing" :disabled="blocked"><option v-for="peer in peers" :key="peer.id" :value="peer.id">{{ peer.label }} · {{ peer.id }}</option></select></label>
      <label class="name-field" for="exchange-space">Локальное пространство<select id="exchange-space" v-model="space" :disabled="blocked"><option v-for="item in spaces" :key="item.space" :value="item.space">{{ item.names.join(' / ') || 'Без названия' }}</option></select></label></div>
      <div v-if="control" class="exchange-control" data-local-exchange>
        <p>{{ control.selected?'Выбрано для обмена с устройством':'Не выбрано для обмена с устройством' }}. {{ control.paused?'Пространство на явной паузе.':'Явной паузы нет.' }}</p>
        <div class="view-actions"><button :disabled="blocked" @click="change(control,'select')">{{ control.selected?'Выключить обмен':'Включить обмен' }}</button><button :disabled="blocked" @click="change(control,'pause')">{{ control.paused?'Возобновить обмен':'Приостановить обмен' }}</button><button v-if="session?.request===pairing&&control.selected&&!control.paused" :disabled="blocked" @click="run(control.space)">Обменяться сейчас</button></div>
      </div>
    </template><p v-else>Сначала подтвердите подключение устройства.</p>
    <template v-if="session?.status==='connected'">
      <h3>Пространства подключённого устройства</h3><button :disabled="!ready||busy" @click="remoteSpaces">Показать пространства устройства</button>
      <ul v-if="catalog" class="exchange-list">
        <li v-for="{entry,c} in rows" :key="entry.space" :data-exchange-space="entry.space"><h4>{{ entry.names.join(' / ') || 'Без названия' }}</h4>
          <p>{{ entry.writable?'Чтение и отправка изменений':'Только получение изменений' }}. {{ entry.selected?'На другом устройстве выбрано.':'На другом устройстве ещё не выбрано.' }} {{ entry.paused?'На другом устройстве пауза.':'' }}</p>
          <template v-if="c"><p>{{ c.selected?'Обмен включён на этом устройстве.':'Обмен на этом устройстве выключен.' }} {{ c.paused?'Пространство на явной паузе.':'' }}</p>
          <div class="view-actions"><button :disabled="blocked" @click="change(c,'select')">{{ c.selected?'Выключить получение':'Выбрать пространство' }}</button><button v-if="c.selected" :disabled="blocked" @click="change(c,'pause')">{{ c.paused?'Возобновить получение':'Приостановить получение' }}</button><button v-if="c.selected&&!c.paused" :disabled="blocked" @click="run(entry.space)">Обменяться сейчас</button></div></template>
        </li>
      </ul><p v-if="catalog&&!catalog.spaces.length">Нет доступных пространств. Проверьте выданные права на другом устройстве.</p>
    </template>
    <div v-if="pending.length"><p>Есть незавершённые действия. Сначала проверьте их результат. Удаление записи доставки не отменяет уже выполненное действие.</p><template v-for="item in pending.filter(p=>p.intention.operation.startsWith('sync.'))" :key="item.id"><button :disabled="!ready||busy" @click="retry(item.id)">Повторить действие обмена</button><button :disabled="!ready||busy" @click="forget(item.id)">Убрать запись действия обмена</button></template></div>
  </section>
</template>
