import { invoke, isTauri } from "@tauri-apps/api/core";
import { client } from "./http";
import { invitationCommand } from "./ipc-command";
import type { operations } from "./api";

type Request = operations["invitation"]["requestBody"]["content"]["application/json"];
export type Invitation = operations["invitation"]["responses"][200]["content"]["application/json"];

export async function invitation(request: Request): Promise<Invitation> {
  if (isTauri()) {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        invoke<Invitation>(invitationCommand, { request }),
        new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error("IPC timeout")), 5000); }),
      ]);
    } finally { clearTimeout(timer); }
  }
  const { data, error, response } = await client.POST("/prototype/invitation", {
    body: request, signal: AbortSignal.timeout(5000),
  });
  if (!response.ok || !data) throw error ?? new Error(`HTTP ${response.status}`);
  return data;
}
