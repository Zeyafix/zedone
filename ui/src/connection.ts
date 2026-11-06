import { isTauri, invoke } from "@tauri-apps/api/core";
import { httpTransport, ipcTransport, type CredentialMaterial, type Intention } from "../../client/commands";
import validateResponse from "../../api/validate-response.js";

export type Connection = "checking" | "unconfigured" | "ready" | "denied" | "unavailable";
type Session = { status: "unconfigured" | "unavailable" } | { status: "configured"; credential: CredentialMaterial };

/** A fresh, read-only probe. Its returned list is deliberately not cached or
 * used as a current board. Domain UI will fetch its own current view in P05.02. */
export async function checkConnection(signal: AbortSignal): Promise<Connection> {
  try {
    let session: Session;
    if (isTauri()) session = await invoke<Session>("shell_session");
    else {
      const response = await fetch("/host/session", { signal, credentials: "omit", cache: "no-store", redirect: "error" });
      if (!response.ok) return "unavailable";
      session = await response.json() as Session;
    }
    if (signal.aborted) return "unavailable";
    if (session.status === "unconfigured" || session.status === "unavailable") return session.status;
    if (session.status !== "configured" || !session.credential
        || !/^[a-f0-9]{32}$/.test(session.credential.subject) || !/^[a-f0-9]{64}$/.test(session.credential.key)) return "unavailable";
    const bytes = crypto.getRandomValues(new Uint8Array(16));
    const intention: Intention = {
      subject: session.credential.subject, operation: "space.list",
      body: JSON.stringify({ id: Array.from(bytes), operation: "space.list", parameters: { include_trashed: false } }),
    };
    const credential = async () => session.credential;
    const transport = isTauri() ? ipcTransport(invoke, credential) : httpTransport(`${location.origin}/`, credential);
    const result = await transport(intention, signal);
    if (!validateResponse(result, "space.list")) return "unavailable";
    if (result.status === "ok") return "ready";
    return result.error.code === "access_denied" ? "denied" : "unavailable";
  } catch {
    return "unavailable";
  }
}
