import { isTauri, invoke } from "@tauri-apps/api/core";
import { httpTransport, ipcTransport, type CredentialMaterial, type Intention } from "../../client/commands";
import validateResponse from "../../api/validate-response.js";

export type Connection = "checking" | "unconfigured" | "ready" | "denied" | "unavailable";
type Session = { status: "unconfigured" | "unavailable" } | { status: "configured"; credential: CredentialMaterial };

export async function localCredential(signal?: AbortSignal): Promise<CredentialMaterial> {
  let session: Session;
  if (isTauri()) session = await invoke<Session>("shell_session");
  else {
    const response = await fetch("/host/session", { signal, credentials: "omit", cache: "no-store", redirect: "error" });
    if (!response.ok) throw new Error("unavailable");
    session = await response.json() as Session;
  }
  if (session.status !== "configured" || !session.credential
      || !/^[a-f0-9]{32}$/.test(session.credential.subject) || !/^[a-f0-9]{64}$/.test(session.credential.key)) {
    throw new Error(session.status === "unconfigured" ? "unconfigured" : "unavailable");
  }
  return session.credential;
}

// Re-read the host's approved identity on every delivery, including retries.
export const localTransport = () => isTauri()
  ? ipcTransport(invoke, localCredential)
  : httpTransport(`${location.origin}/`, localCredential);

/** A fresh, read-only probe. Its returned list is deliberately not cached or
 * used as a current board. Domain UI fetches its own current view. */
export async function checkConnection(signal: AbortSignal): Promise<Connection> {
  try {
    const session = await localCredential(signal);
    if (signal.aborted) return "unavailable";
    const bytes = crypto.getRandomValues(new Uint8Array(16));
    const intention: Intention = {
      subject: session.subject, operation: "space.list",
      body: JSON.stringify({ id: Array.from(bytes), operation: "space.list", parameters: { include_trashed: false } }),
    };
    const result = await localTransport()(intention, signal);
    if (!validateResponse(result, "space.list")) return "unavailable";
    if (result.status === "ok") return "ready";
    return result.error.code === "access_denied" ? "denied" : "unavailable";
  } catch (error) {
    return error instanceof Error && error.message === "unconfigured" ? "unconfigured" : "unavailable";
  }
}
