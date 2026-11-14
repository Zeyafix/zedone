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
          hash: location.hash,
          headings: [...document.querySelectorAll("h1,h2,h3")].map(el => el.textContent).slice(0, 20),
          alerts: [...document.querySelectorAll('[role="alert"]')].map(el => el.textContent).slice(0, 5),
          statuses: [...document.querySelectorAll('[role="status"]')].map(el => el.textContent).slice(0, 5),
          regions: [...document.querySelectorAll('[role="region"]')].map(el => el.getAttribute("aria-label")).slice(0, 10),
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

/** Observe only bounded response metadata from this exact page. Reading a
 * response does not inspect request bodies, headers, keys or saved intentions. */
export function observeShellReads(page) {
  const reads = [];
  page.on("response", async response => {
    const path = new URL(response.url()).pathname;
    if (!/\/v1\/commands\/(space\.(list|read)|column\.list|card\.list|tag\.list)$/.test(path)) return;
    const body = await response.json().catch(() => undefined);
    reads.push({ path, http: response.status(), status: body?.status, error: body?.error?.code,
      count: body?.result?.spaces?.length ?? body?.result?.columns?.length ?? body?.result?.cards?.length ?? body?.result?.tags?.length,
      state: body?.result?.state });
    if (reads.length > 30) reads.shift();
  });
  return reads;
}

/** Capture the failing page before its context is disposed. Diagnostics must
 * never replace the scenario's original error or retry the failed action. */
export async function withShellFailureDiagnostics(page, details, work) {
  try { return await work(); }
  catch (error) {
    try {
      const state = await shellDiagnosticState(page);
      console.error("UI failure state:", JSON.stringify({ label: details.label, stage: details.stage, ...state, reads: details.reads }));
    } catch { /* Preserve the original error even if diagnostic output fails. */ }
    throw error;
  }
}
