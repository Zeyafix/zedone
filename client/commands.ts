import createClient from "openapi-fetch";
import type { components, paths } from "../api/schema";
import validateResponse, { operations, type Operation, type Parameters, type Response } from "../api/validate-response.js";
export type { Operation, Response } from "../api/validate-response.js";
type Schemas = components["schemas"];

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

export async function newAccessCredential(): Promise<{ credential: CredentialMaterial; verifier: Parameters["access.issue"]["verifier"] }> {
  const subject = randomId().id;
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  const input = new Uint8Array([...new TextEncoder().encode("zedone/access-key/v1\0"), ...bytes]);
  try {
    const digest = await crypto.subtle.digest("SHA-256", input);
    return { credential: { subject, key: Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("") }, verifier: Array.from(new Uint8Array(digest)) };
  } finally {
    bytes.fill(0);
    input.fill(0);
  }
}
class IdentityChanged extends Error {}

export async function newInvitation(): Promise<{ invitation: string; code: string; verifier: Parameters["invitation.create"]["verifier"] }> {
  const invitation = randomId().id;
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  const input = new Uint8Array([...new TextEncoder().encode("zedone/invitation-code/v1\0"), ...bytes]);
  try {
    if (bytes.every(byte => byte === 0)) throw new Error("unavailable");
    const digest = await crypto.subtle.digest("SHA-256", input);
    return { invitation, code: Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join(""), verifier: Array.from(new Uint8Array(digest)) };
  } finally {
    bytes.fill(0);
    input.fill(0);
  }
}

export async function prepare<O extends Operation>(subject: string, operation: O, parameters: Parameters[O], store: IntentionStore): Promise<string> {
  if (!/^[a-f0-9]{32}$/.test(subject)) throw new Error("invalid_subject");
  if (!operations.includes(operation)) throw new Error("invalid_request");
  const { bytes, id } = randomId();
  if (await store.load(id)) throw new Error("unavailable");
  const body = JSON.stringify({ id: Array.from(bytes), operation, parameters });
  if (new TextEncoder().encode(body).byteLength > (["space.import", "space.recovery.restore"].includes(operation) ? 3 * 1024 * 1024 : 65536)) throw new Error("invalid_request");
  await store.save(id, Object.freeze({ subject, operation, body }));
  return id;
}

function randomId() {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  if (bytes.every(byte => byte === 0)) throw new Error("unavailable");
  const id = Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("");
  return { bytes, id };
}

export function newSpaceId(): string { return randomId().id; }

export function newColumnId(): string { return randomId().id; }

export function newCardId(): string { return randomId().id; }

export function newTagId(): string { return randomId().id; }

const pause = (ms: number) => new Promise<void>(resolve => setTimeout(resolve, ms));
export class CommandClient {
  constructor(private readonly store: IntentionStore, private readonly transport: Transport) {}

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
        if (!validateResponse(response, intention.operation)) throw new Error("outcome_unknown");
        previous = response;
        if (response.status === "ok") {
          await this.store.remove(id);
          return { status: "completed", response };
        }
        if (response.error?.code !== "unavailable" && response.error?.code !== "outcome_unknown") {
          return { status: "pending", response };
        }
      } catch (error) {
        if (error instanceof IdentityChanged) return { status: "pending", reason: "identity_changed" };
      } finally {
        if (timer !== undefined) clearTimeout(timer);
        controller.abort();
      }
      if (index === 2) return { status: "pending", response: previous };
    }
    return { status: "pending", response: previous };
  }
}

export function httpTransport(baseUrl: string, credential: () => Promise<CredentialMaterial>, fetcher: typeof fetch = fetch): Transport {
  const url = new URL(baseUrl);
  if (url.username || url.password || url.search || url.hash || url.pathname !== "/") throw new Error("invalid_endpoint");
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
