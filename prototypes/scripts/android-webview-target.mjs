// A WebView debugging socket can exist before its page target is published.
export async function waitForWebViewTarget(endpoint, until, request = fetch) {
  return until(async () => {
    try {
      const response = await request(`${endpoint}/json/list`, { signal: AbortSignal.timeout(1000) });
      if (!response.ok) return false;
      const targets = await response.json();
      return Array.isArray(targets) && targets.find(target => target?.type === "page"
        && typeof target.webSocketDebuggerUrl === "string" && target.webSocketDebuggerUrl.length > 0);
    } catch {
      return false;
    }
  }, "APK WebView discoverable page target");
}
