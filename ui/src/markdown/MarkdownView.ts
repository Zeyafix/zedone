import { computed, defineComponent, h, ref, watch, type VNodeChild } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { httpUrl, markdown } from "./markdown";
import RemoteImage from "./RemoteImage.vue";

export default defineComponent({
  props: { source: { type: String, required: true }, contentId: String, label: { type: String, default: "Просмотр Markdown" } },
  setup(props) {
    const document = computed(() => markdown(props.source));
    const error = ref("");
    const generation = ref(0);
    watch(() => props.source, () => { generation.value++; error.value = ""; }, { flush: "sync" });
    function node(value: Node): VNodeChild {
      if (value.nodeType === Node.TEXT_NODE) return value.textContent;
      if (!(value instanceof HTMLElement)) return null;
      const marker = value.getAttribute("data-md-image");
      if (marker !== null) {
        const image = document.value.images[Number(marker)];
        // A changed source must never inherit an earlier image's consent.
        return image ? h(RemoteImage, { image, key: `${generation.value}:${marker}`, "data-md-control": marker }) : null;
      }
      const children = Array.from(value.childNodes, node);
      const tag = value.tagName.toLowerCase();
      if (tag === "a") {
        const url = httpUrl(value.getAttribute("href") || "");
        if (!url) return h("span", children);
        const link = h("a", {
          href: url, target: "_blank", rel: "noopener noreferrer",
          title: value.getAttribute("title") || undefined,
          onClick: async (event: MouseEvent) => {
            if (!isTauri()) return;
            event.preventDefault();
            error.value = "";
            try { await openUrl(url); }
            catch { error.value = "Не удалось открыть внешний браузер. Ссылка сохранена — повторите нажатие."; }
          },
        }, value.querySelector("[data-md-image]") ? "Открыть ссылку" : children);
        // Image controls must not be nested inside a link.
        return value.querySelector("[data-md-image]") ? h("span", [link, ...children]) : link;
      }
      const attrs: Record<string, string> = {};
      for (const name of ["title", "start", "class"]) {
        if (value.hasAttribute(name)) attrs[name] = value.getAttribute(name)!;
      }
      return h(tag, attrs, children);
    }
    return () => h("section", { "aria-label": props.label, class: "markdown-view" }, [
      h("h2", props.label),
      h("div", { id: props.contentId, class: "markdown-content" }, Array.from(document.value.fragment.childNodes, node)),
      error.value ? h("p", { id: props.contentId ? "markdown-link-error" : undefined, class: "markdown-link-error", "aria-live": "polite" }, error.value) : null,
    ]);
  },
});
