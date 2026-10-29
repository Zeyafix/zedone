import { computed, defineComponent, h, ref, type VNodeChild } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { httpUrl, markdown } from "./markdown";
import RemoteImage from "./RemoteImage.vue";

export default defineComponent({
  props: { source: { type: String, required: true } },
  setup(props) {
    const document = computed(() => markdown(props.source));
    const error = ref("");
    function node(value: Node): VNodeChild {
      if (value.nodeType === Node.TEXT_NODE) return value.textContent;
      if (!(value instanceof HTMLElement)) return null;
      const marker = value.getAttribute("data-md-image");
      if (marker !== null) {
        const image = document.value.images[Number(marker)];
        return image ? h(RemoteImage, { image, key: marker, "data-md-control": marker }) : null;
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
    return () => h("section", { "aria-label": "Просмотр Markdown", class: "markdown-view" }, [
      h("h2", "Просмотр Markdown"),
      h("div", { id: "markdown-content" }, Array.from(document.value.fragment.childNodes, node)),
      error.value ? h("p", { id: "markdown-link-error", "aria-live": "polite" }, error.value) : null,
    ]);
  },
});
