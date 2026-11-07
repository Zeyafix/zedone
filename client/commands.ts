import createClient from "openapi-fetch";
import type { components, paths } from "../api/schema";
import validateResponse, { operations, type Operation, type Parameters, type Response } from "../api/validate-response.js";
export type { Operation, Response } from "../api/validate-response.js";
type Schemas = components["schemas"];

// A pending intention is delivery state, never authoritative application data.
// No key is included. Persist BEFORE delivery; reload after a crash and retry
// the saved bytes, never allocate a new ID to resolve an uncertain outcome.
export interface Intention { readonly subject: string; readonly operation: Operation; readonly body: string }
export interface IntentionStore {
  save(id: string, intention: Intention): Promise<void>;
  load(id: string): Promise<Intention | undefined>;
  remove(id: string): Promise<void>;
  list(): Promise<string[]>;
}
export type Transport = (intention: Intention, signal: AbortSignal) => Promise<Response>;
export type Delivery = { status: "completed"; response: Response } | { status: "pending"; response?: Response; reason?: "identity_changed" };
export interface CredentialMaterial { readonly subject: string; readonly key: string }
class IdentityChanged extends Error {}

export async function prepare<O extends Operation>(subject: string, operation: O, parameters: Parameters[O], store: IntentionStore): Promise<string> {
  if (!/^[a-f0-9]{32}$/.test(subject)) throw new Error("invalid_subject");
  if (!operations.includes(operation)) throw new Error("invalid_request");
  const { bytes, id } = randomId();
  // Detect a collision rather than replacing an unresolved intention.
  if (await store.load(id)) throw new Error("unavailable");
  const body = JSON.stringify({ id: Array.from(bytes), operation, parameters });
  if (new TextEncoder().encode(body).byteLength > 65536) throw new Error("invalid_request");
  await store.save(id, Object.freeze({ subject, operation, body }));
  return id;
}

function randomId() {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  if (bytes.every(byte => byte === 0)) throw new Error("unavailable");
  const id = Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("");
  return { bytes, id };
}

/** Generate once for a new space and persist it with the creation intention. */
export function newSpaceId(): string { return randomId().id; }

/** Generate once and persist with the column creation intention. */
export function newColumnId(): string { return randomId().id; }

/** Generate once and persist with the card creation intention. */
export function newCardId(): string { return randomId().id; }

/** A deleted tag identity cannot be reused, even for the same name. */
export function newTagId(): string { return randomId().id; }

const pause = (ms: number) => new Promise<void>(resolve => setTimeout(resolve, ms));
export class CommandClient {
  constructor(private readonly store: IntentionStore, private readonly transport: Transport) {}

  // Both automatic attempts and explicit retries load the same saved request.
  // A timeout races even transports that ignore AbortSignal (e.g. admitted IPC).
  async deliver(id: string): Promise<Delivery> {
    const intention = await this.store.load(id);
    if (!intention) throw new Error("missing_intention");
    let previous: Response | undefined;
    for (const [index, delay] of [0, 250, 1000].entries()) {
      if (delay) await pause(delay);
      const controller = new AbortController();
      let timer: ReturnType<typeof setTimeout> | undefined;
      try {
        const timeout = new Promise<never>((_, reject) => {
          timer = setTimeout(() => { controller.abort(); reject(new Error("outcome_unknown")); }, 10_000);
        });
        const response = await Promise.race([this.transport(intention, controller.signal), timeout]);
        // A malformed/missing envelope never confirms a command.
        if (!validateResponse(response, intention.operation)) throw new Error("outcome_unknown");
        previous = response;
        if (response.status === "ok") {
          await this.store.remove(id);
          return { status: "completed", response };
        }
        if (response.error?.code !== "unavailable" && response.error?.code !== "outcome_unknown") {
          // Denial after a lost reply does not prove the earlier delivery failed.
          // Keep the original request for an explicit retry after access returns.
          return { status: "pending", response };
        }
      } catch (error) {
        if (error instanceof IdentityChanged) return { status: "pending", reason: "identity_changed" };
        // Network, malformed reply, timeout, or local persistence failure:
        // preserve the original ID/parameters and never report success.
      } finally {
        if (timer !== undefined) clearTimeout(timer);
        controller.abort();
      }
      if (index === 2) return { status: "pending", response: previous };
    }
    return { status: "pending", response: previous };
  }
}

/** The host supplies the currently approved key from its protected credential
 * facility. Keys are never stored in the pending-intention store or URL. */
export function httpTransport(baseUrl: string, credential: () => Promise<CredentialMaterial>, fetcher: typeof fetch = fetch): Transport {
  const url = new URL(baseUrl);
  if (url.username || url.password || url.search || url.hash || url.pathname !== "/") throw new Error("invalid_endpoint");
  // Remote device exposure belongs to the authenticated P06 host/channel.
  if (url.protocol !== "https:" && !(url.protocol === "http:" && ["127.0.0.1", "[::1]"].includes(url.hostname))) throw new Error("untrusted_endpoint");
  const client = createClient<paths>({ baseUrl: url.origin, fetch: fetcher });
  return async (intention, signal) => {
    const { subject, key } = await credential();
    if (subject !== intention.subject) throw new IdentityChanged();
    const { data } = await client.POST(`/v1/commands/${intention.operation}`, {
      body: JSON.parse(intention.body), bodySerializer: () => intention.body,
      headers: { Authorization: `Bearer ${key}` }, signal,
      redirect: "error", credentials: "omit", cache: "no-store",
    });
    if (!data) throw new Error("outcome_unknown");
    return data;
  };
}

export type Invoke = (command: string, args: Record<string, unknown>) => Promise<string>;
export function ipcTransport(invoke: Invoke, credential: () => Promise<CredentialMaterial>): Transport {
  return async intention => {
    const { subject, key } = await credential();
    if (subject !== intention.subject) throw new IdentityChanged();
    const invocation: Schemas["IpcInvocation"] = { operation: intention.operation, request: intention.body };
    return JSON.parse(await invoke("zedone_command", { invocation, credential: key })) as Response;
  };
}

/** Browser/Android WebView persistence. A thrown quota/security error prevents
 * delivery. Namespace is owned by this installation; clearing it is an explicit
 * abandonment, not proof that admitted work was cancelled. */
export function webIntentionStore(storage: Pick<Storage, "getItem" | "setItem" | "removeItem" | "key" | "length">, namespace: string): IntentionStore {
  const prefix = `zedone.pending.v1.${namespace}.`;
  return {
    async save(id, intention) { storage.setItem(prefix + id, JSON.stringify(intention)); },
    async load(id) {
      const text = storage.getItem(prefix + id);
      if (text === null) return undefined;
      const value: unknown = JSON.parse(text);
      if (!value || typeof value !== "object" || !("operation" in value) || !("body" in value) || !("subject" in value)
          || Object.keys(value).sort().join(",") !== "body,operation,subject" || typeof value.subject !== "string" || !/^[a-f0-9]{32}$/.test(value.subject)
          || typeof value.operation !== "string" || !operations.includes(value.operation as Operation) || typeof value.body !== "string") throw new Error("invalid_intention");
      const request = JSON.parse(value.body);
      if (!/^[a-f0-9]{32}$/.test(id) || request.operation !== value.operation || !Array.isArray(request.id)
          || request.id.length !== 16 || request.id.some((byte: unknown) => typeof byte !== "number" || !Number.isInteger(byte) || byte < 0 || byte > 255)
          || request.id.map((byte: number) => byte.toString(16).padStart(2, "0")).join("") !== id) throw new Error("invalid_intention");
      return value as Intention;
    },
    async remove(id) { storage.removeItem(prefix + id); },
    async list() {
      const ids: string[] = [];
      for (let index = 0; index < storage.length; index++) {
        const key = storage.key(index);
        if (key?.startsWith(prefix)) ids.push(key.slice(prefix.length));
      }
      return ids.sort();
    },
  };
}
