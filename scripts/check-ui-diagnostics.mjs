/** Read-only failure evidence. Never return credentials, input values or stored intentions. */
export async function shellDiagnosticState(page) {
  if (!page || page.isClosed()) return { unavailable: "no live page" };
  let timer;
  try {
    return await Promise.race([
      page.evaluate(async () => {
        const state = {
          url: location.href, readyState: document.readyState, title: document.title,
          mounted: !!document.querySelector("#app[data-v-app]"),
          bridge: !!window.__TAURI_INTERNALS__?.invoke,
          connection: document.querySelector("#connection-title")?.textContent ?? null,
          busy: document.querySelector(".spaces-view")?.getAttribute("aria-busy") ?? null,
          headings: [...document.querySelectorAll("h1,h2,h3")].map(el => el.textContent).slice(0, 20),
          alerts: [...document.querySelectorAll('[role="alert"]')].map(el => el.textContent).slice(0, 5),
          scripts: [...document.scripts].map(el => ({ src: el.src, type: el.type })).slice(0, 10),
        };
        let sessionTimer;
        try {
          const session = window.__TAURI_INTERNALS__?.invoke
            ? window.__TAURI_INTERNALS__.invoke("shell_session")
            : fetch("/host/session").then(response => response.json());
          const status = await Promise.race([
            session.then(value => ["configured", "unconfigured", "unavailable"].includes(value?.status) ? value.status : "invalid", () => "rejected"),
            new Promise(resolve => { sessionTimer = setTimeout(() => resolve("timeout"), 1000); }),
          ]);
          return { ...state, session: status };
        } catch { return { ...state, session: "rejected" }; }
        finally { clearTimeout(sessionTimer); }
      }),
      new Promise(resolve => { timer = setTimeout(() => resolve({ unavailable: "diagnostic timeout" }), 3000); }),
    ]);
  } catch { return { unavailable: "page evaluation failed" }; }
  finally { clearTimeout(timer); }
}
