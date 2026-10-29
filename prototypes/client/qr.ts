import { invoke, isTauri } from "@tauri-apps/api/core";
import { client } from "./http";
import { qrCommand } from "./ipc-command";
import type { operations } from "./api";

type Request = operations["qr"]["requestBody"]["content"]["application/json"];
export type QrResponse = operations["qr"]["responses"][200]["content"]["application/json"];

export async function qr(request: Request): Promise<QrResponse> {
  if (isTauri()) {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        invoke<QrResponse>(qrCommand, { request }),
        new Promise<never>((_, reject) => {
          timer = setTimeout(() => reject(new Error("IPC timeout")), 5000);
        }),
      ]);
    } finally { clearTimeout(timer); }
  }
  const { data, error, response } = await client.POST("/prototype/qr", {
    body: request, signal: AbortSignal.timeout(5000),
  });
  if (!response.ok || !data) throw error ?? new Error(`HTTP ${response.status}`);
  return data;
}
