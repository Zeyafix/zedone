import { invoke, isTauri } from "@tauri-apps/api/core";
import { client } from "./http";
import { echoCommand } from "./ipc-command";
import type { operations } from "./api";

type Request = operations["echo"]["requestBody"]["content"]["application/json"];
type Response = operations["echo"]["responses"][200]["content"]["application/json"];

export async function echo(request: Request): Promise<Response> {
  if (isTauri()) {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      return await Promise.race([
        invoke<Response>(echoCommand, { request }),
        new Promise<Response>((_, reject) => {
          timer = setTimeout(() => reject(new Error("IPC timeout")), 5000);
        }),
      ]);
    } finally {
      clearTimeout(timer);
    }
  }
  const { data, response } = await client.POST("/prototype/echo", {
    body: request,
    signal: AbortSignal.timeout(5000),
  });
  if (!response.ok || !data) throw new Error(`HTTP ${response.status}`);
  return data;
}
