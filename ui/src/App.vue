<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { DialogClose, DialogContent, DialogDescription, DialogOverlay, DialogPortal, DialogRoot, DialogTitle, DialogTrigger } from "reka-ui";
import { checkConnection, type Connection } from "./connection";
import { readTheme, saveTheme, type Theme } from "./appearance";
import SpacesView from "./SpacesView.vue";

type Page = "spaces" | "settings";
const pageFromHash = (): Page => location.hash === "#/settings" ? "settings" : "spaces";
const page = ref<Page>(pageFromHash());
const navigationOpen = ref(false);
let navigatingFromMenu = false;
const heading = ref<HTMLElement>();
const theme = ref<Theme>(readTheme());
const preferenceSaved = ref(true);
const systemDark = matchMedia("(prefers-color-scheme: dark)");
const dark = ref(systemDark.matches);
const effectiveTheme = computed(() => theme.value === "system" ? dark.value ? "dark" : "light" : theme.value);
const connection = ref<Connection>("checking");
let controller: AbortController | undefined;
let timeout: ReturnType<typeof setTimeout> | undefined;
let generation = 0;
const connectionLabels: Record<Connection, string> = {
  checking: "Проверяем доступ", unconfigured: "Доступ не настроен", ready: "Доступ подтверждён",
  denied: "Нет доступа", unavailable: "Не удалось связаться с ядром",
};
const connectionDescriptions: Record<Connection, string> = {
  checking: "Проверяем возможность работы с локальными данными.",
  unconfigured: "Для этой установки ещё не настроен локальный доступ. Настройки оформления уже доступны.",
  ready: "Проверка доступа к локальным данным прошла. Это не подтверждение синхронизации с другим устройством.",
  denied: "Локальный доступ не подтверждён. Он мог быть отозван или его срок истёк. После восстановления доступа повторите проверку.",
  unavailable: "Локальные данные сейчас недоступны. Повторите проверку; настройки оформления продолжают работать.",
};

watch(effectiveTheme, value => { document.documentElement.dataset.theme = value; }, { immediate: true });
watch(theme, value => { preferenceSaved.value = saveTheme(value); });
function updateSystem(event: MediaQueryListEvent) { dark.value = event.matches; }
async function routeChanged() {
  page.value = pageFromHash();
  navigationOpen.value = false;
  await nextTick();
  heading.value?.focus();
}
function navigate(destination: Page) {
  navigatingFromMenu = navigationOpen.value && page.value !== destination;
  navigationOpen.value = false;
  if (page.value === destination) return;
  location.hash = `/${destination}`;
}
function menuClosed(event: Event) {
  if (navigatingFromMenu) event.preventDefault();
  navigatingFromMenu = false;
}
async function probe() {
  const current = ++generation;
  controller?.abort();
  if (timeout) clearTimeout(timeout);
  controller = new AbortController();
  connection.value = "checking";
  // IPC cannot always be cancelled; generation also rejects late results.
  const expired = new Promise<Connection>(resolve => {
    timeout = setTimeout(() => { controller?.abort(); resolve("unavailable"); }, 10_000);
  });
  const result = await Promise.race([checkConnection(controller.signal), expired]);
  if (current !== generation) return;
  if (timeout) clearTimeout(timeout);
  connection.value = result;
}
function resume() { if (document.visibilityState === "visible") void probe(); }
onMounted(() => {
  window.addEventListener("hashchange", routeChanged);
  document.addEventListener("visibilitychange", resume);
  systemDark.addEventListener("change", updateSystem);
  void probe();
});
onUnmounted(() => {
  generation++;
  controller?.abort();
  if (timeout) clearTimeout(timeout);
  window.removeEventListener("hashchange", routeChanged);
  document.removeEventListener("visibilitychange", resume);
  systemDark.removeEventListener("change", updateSystem);
});
</script>

<template>
  <a class="skip-link" href="#main-content" @click.prevent="heading?.focus()">К содержимому</a>
  <div class="shell">
    <aside class="rail" aria-label="Основная навигация">
      <a class="brand" href="#/spaces" @click.prevent="navigate('spaces')"><span class="brand-mark" aria-hidden="true">z.</span>ZeDone</a>
      <nav aria-label="Разделы">
        <a href="#/spaces" :aria-current="page === 'spaces' ? 'page' : undefined" @click.prevent="navigate('spaces')">Пространства</a>
        <a href="#/settings" :aria-current="page === 'settings' ? 'page' : undefined" @click.prevent="navigate('settings')">Настройки</a>
      </nav>
      <p class="rail-note">Место для мыслей<br />и запланированных дел.</p>
    </aside>

    <div class="workspace">
      <header class="topbar">
        <span class="mobile-brand">ZeDone</span>
        <span class="desktop-context">{{ page === 'spaces' ? 'Ваши пространства' : 'Оформление и доступ' }}</span>
        <DialogRoot v-model:open="navigationOpen">
          <DialogTrigger class="navigation-trigger">Меню</DialogTrigger>
          <DialogPortal>
            <DialogOverlay class="dialog-overlay" />
            <DialogContent class="navigation-dialog" @close-auto-focus="menuClosed">
              <div class="dialog-header"><DialogTitle>Навигация</DialogTitle><DialogClose data-shell-menu-close aria-label="Закрыть меню">Закрыть</DialogClose></div>
              <DialogDescription class="sr-only">Выберите раздел ZeDone.</DialogDescription>
              <nav aria-label="Разделы в меню">
                <a href="#/spaces" :aria-current="page === 'spaces' ? 'page' : undefined" @click.prevent="navigate('spaces')">Пространства</a>
                <a href="#/settings" :aria-current="page === 'settings' ? 'page' : undefined" @click.prevent="navigate('settings')">Настройки</a>
              </nav>
            </DialogContent>
          </DialogPortal>
        </DialogRoot>
      </header>

      <main id="main-content">
        <div class="page-heading"><h1 ref="heading" tabindex="-1">{{ page === 'spaces' ? 'Пространства' : 'Настройки' }}</h1>
          <p>{{ page === 'spaces' ? 'Мысли и дела, собранные по темам.' : 'Удобный вид на этом устройстве.' }}</p>
        </div>

        <div v-show="page === 'spaces'">
          <section class="connection-panel" :class="{ 'connection-panel-ready': connection === 'ready' }" aria-labelledby="connection-title" :aria-busy="connection === 'checking'">
            <span class="connection-indicator" :data-state="connection" aria-hidden="true"></span>
            <div class="connection-copy">
              <h2 id="connection-title">{{ connectionLabels[connection] }}</h2>
              <p v-if="connection !== 'ready'">{{ connectionDescriptions[connection] }}</p>
              <button v-if="connection !== 'checking'" class="text-button" @click="probe">Проверить снова</button>
            </div>
          </section>
          <section v-if="connection !== 'ready'" class="shell-note" aria-labelledby="shell-title">
            <div class="space-symbol" aria-hidden="true"><span></span><span></span><span></span></div>
            <h2 id="shell-title">Ваши пространства на этом устройстве</h2>
            <p>Для просмотра и изменения пространств нужен действующий локальный доступ. Оформление доступно сейчас.</p>
            <a class="primary-link" href="#/settings" @click.prevent="navigate('settings')">Выбрать оформление</a>
          </section>
          <SpacesView :ready="connection === 'ready'" />
        </div>

        <template v-if="page === 'settings'">
          <section class="settings-section" aria-labelledby="appearance-title">
            <h2 id="appearance-title">Оформление</h2>
            <p id="theme-description">Настройка применяется к этому браузеру или приложению. Цвета системы учитываются автоматически в режиме «Как на устройстве».</p>
            <fieldset aria-describedby="theme-description">
              <legend>Тема</legend>
              <div class="theme-options">
                <label v-for="option in ([['system', 'Как на устройстве'], ['light', 'Светлая'], ['dark', 'Тёмная']] as const)" :key="option[0]" class="theme-option" :class="{ 'is-selected': theme === option[0] }">
                  <input v-model="theme" type="radio" name="theme" :value="option[0]" />
                  <span class="theme-swatch" :data-palette="option[0]" aria-hidden="true"><i></i><i></i><i></i></span>
                  <span>{{ option[1] }}</span>
                </label>
              </div>
            </fieldset>
            <p v-if="!preferenceSaved" role="status" class="preference-error">Тема применена, но сохранить выбор не удалось. После перезапуска будет использована тема устройства.</p>
          </section>
          <section class="settings-section" aria-labelledby="access-title">
            <h2 id="access-title">Локальный доступ</h2>
            <p>{{ connectionLabels[connection] }}. {{ connectionDescriptions[connection] }}</p>
            <button :disabled="connection === 'checking'" @click="probe">{{ connection === 'checking' ? 'Проверяем…' : 'Проверить снова' }}</button>
          </section>
        </template>
        <p class="sr-only" role="status" aria-live="polite">{{ connectionLabels[connection] }}</p>
      </main>
      <footer>ZeDone <span>Локальное хранение · общий доступ человека и агента</span></footer>
    </div>
  </div>
</template>
