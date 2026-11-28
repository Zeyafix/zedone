import { isTauri, invoke } from "@tauri-apps/api/core";
import { CommandClient, prepare, webIntentionStore, httpTransport, ipcTransport, type CredentialMaterial, type Intention } from "../../client/commands";
import type { components } from "../../api/schema";
import validateResponse from "../../api/validate-response.js";

export type MaintenanceView = components["schemas"]["MaintenanceView"];
export type MaintenanceCommand = components["schemas"]["MaintenanceCommand"];
const store = () => webIntentionStore(localStorage, "maintenance");
export async function maintenanceCredential(signal?: AbortSignal): Promise<CredentialMaterial> {
  const value = isTauri() ? await invoke<CredentialMaterial | null>("maintenance_session")
    : await fetch("/host/maintenance-session", { signal, credentials: "omit", cache: "no-store", redirect: "error" }).then(r => r.ok ? r.json() : null) as CredentialMaterial | null;
  if (!value || !/^[a-f0-9]{32}$/.test(value.subject) || !/^[a-f0-9]{64}$/.test(value.key)) throw new Error("unavailable");
  return value;
}
const transport = () => isTauri() ? ipcTransport(invoke, maintenanceCredential) : httpTransport(`${location.origin}/`, maintenanceCredential);
export async function inspectMaintenance(): Promise<MaintenanceView> {
  const controller = new AbortController();
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const read = (async () => {
      const credential = await maintenanceCredential(controller.signal);
      const intention: Intention = { subject: credential.subject, operation: "maintenance.execute", body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation: "maintenance.execute", parameters: { action: "inspect" } }) };
      const response = await transport()(intention, controller.signal);
      if (!validateResponse(response, "maintenance.execute")) throw new Error("outcome_unknown");
      if (response.status !== "ok") throw new Error(response.error.code);
      return response.result as MaintenanceView;
    })();
    return await Promise.race([read, new Promise<never>((_, reject) => { timer = setTimeout(() => { controller.abort(); reject(new Error("unavailable")); }, 10_000); })]);
  } finally { clearTimeout(timer); controller.abort(); }
}
export async function pendingMaintenance(): Promise<string[]> { return store().list(); }
export async function prepareMaintenance(command: MaintenanceCommand): Promise<string> {
  if ((await pendingMaintenance()).length) throw new Error("pending_exists");
  const credential = await maintenanceCredential();
  return prepare(credential.subject, "maintenance.execute", command, store());
}
export async function deliverMaintenance(id: string): Promise<MaintenanceView> {
  const result = await new CommandClient(store(), transport()).deliver(id);
  if (result.status !== "completed" || result.response.status !== "ok") throw new Error(result.response?.status === "error" ? result.response.error.code : "outcome_unknown");
  return result.response.result as MaintenanceView;
}
export async function forgetMaintenance(id: string): Promise<void> { await store().remove(id); }
export async function chooseMaintenanceFolder(): Promise<string | null> {
  const credential = await maintenanceCredential();
  if (isTauri()) return invoke<string | null>("maintenance_folder_select", { credential: credential.key });
  const response = await fetch("/host/maintenance-folder", { method: "POST", headers: { authorization: `Bearer ${credential.key}`, "content-type": "application/json" }, body: "{}", credentials: "omit", cache: "no-store", redirect: "error" });
  if (!response.ok) throw new Error("unavailable");
  return response.json() as Promise<string | null>;
}
export async function newMaintenanceKey() {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  const input = new Uint8Array([...new TextEncoder().encode("zedone/maintenance-key/v1\0"), ...bytes]);
  try { return { key: Array.from(bytes, b => b.toString(16).padStart(2, "0")).join(""), verifier: Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", input))) }; }
  finally { bytes.fill(0); input.fill(0); }
}
export const maintenanceError = (cause: unknown) => ({
  access_denied: "Для этого действия нужны отдельные служебные полномочия. Проверьте, не отозван ли доступ.",
  stale_state: "Состояние изменилось. Обновите сведения и подготовьте новое действие.",
  invalid_request: "Не удалось принять действие. Проверьте выбранную копию и заполненные поля.",
  not_found: "Выбранное сохранённое состояние или подготовленная копия больше не найдены.",
  command_mismatch: "Сохранённый запрос отличается от исходного. Его параметры не были изменены автоматически.",
  pending_exists: "Сначала проверьте незавершённое действие.",
} as Record<string, string>)[cause instanceof Error ? cause.message : ""] ?? "Действие не подтверждено. Проверьте свободное место, доступность папки и закройте другие процессы ZeDone. Если запрос остался в списке, повторите исходную команду.";
