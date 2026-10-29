import MarkdownIt from "markdown-it";
import DOMPurify from "dompurify";

export interface RemoteImage { url?: string; alt: string; title?: string }

// No base URL: relative paths and other URI schemes stay inert in this prototype.
export function httpUrl(value: string): string | undefined {
  try {
    const url = new URL(value);
    if (url.protocol === "http:" || url.protocol === "https:") return url.href;
  } catch { /* Display unsupported addresses as text. */ }
}

const parser = new MarkdownIt("commonmark", { html: false });
parser.renderer.rules.image = (tokens, index, options, env, renderer) => {
  const token = tokens[index]!;
  const images = env!.images as RemoteImage[];
  const image = {
    url: httpUrl(String(token.attrGet("src") || "")),
    alt: renderer.renderInlineAsText(token.children || [], options, env),
    title: token.attrGet("title") === null ? undefined : String(token.attrGet("title")),
  };
  images.push(image);
  // Never put src/srcset or an image element into HTML passed to DOMPurify.
  return `<span data-md-image="${images.length - 1}">${parser.utils.escapeHtml(image.alt)}</span>`;
};

export function markdown(source: string) {
  const images: RemoteImage[] = [];
  const html = parser.render(source, { images });
  const fragment = DOMPurify.sanitize(html, {
    USE_PROFILES: { html: true },
    FORBID_TAGS: ["img"],
    FORBID_ATTR: ["src", "srcset", "style"],
    RETURN_DOM_FRAGMENT: true,
  });
  return { fragment, images };
}
