import { invoke, isTauri } from "@tauri-apps/api/core";
import { localCredential } from "./connection";
import type { components } from "../../api/schema";
type Reply = { status: "selected"; folder: components["schemas"]["BackupFolder"] } | { status: "cancelled" };
export async function chooseBackupFolder(writable: boolean, useDefault = false): Promise<Reply> {
  const credential = await localCredential();
  const selection = { writable, default: useDefault };
  if (isTauri()) return invoke<Reply>("backup_folder_select", { selection, credential: credential.key });
  const response = await fetch("/host/backup-folder", { method: "POST", headers: { Authorization: `Bearer ${credential.key}`, "Content-Type": "application/json" }, body: JSON.stringify(selection), credentials: "omit", cache: "no-store", redirect: "error" });
  if (!response.ok) throw new Error("unavailable");
  const reply = await response.json();
  if (reply.status !== "ok") throw new Error(reply.error?.code ?? "unavailable");
  if (reply.result?.status !== "cancelled" && (reply.result?.status !== "selected" || !/^[a-f0-9]{32}$/.test(reply.result.folder?.id))) throw new Error("unavailable");
  return reply.result as Reply;
}
