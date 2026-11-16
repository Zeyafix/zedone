import { invoke, isTauri } from "@tauri-apps/api/core";
import { client } from "./http";
import { storageCommand } from "./ipc-command";
import type { operations } from "./api";

export type StorageRequest = operations["storage"]["requestBody"]["content"]["application/json"];
export type StorageResponse = operations["storage"]["responses"][200]["content"]["application/json"];

export async function storage(request: StorageRequest): Promise<StorageResponse> {
  if (isTauri()) return invoke<StorageResponse>(storageCommand, { request });
  const { data, error, response } = await client.POST("/prototype/storage", { body: request });
  if (!response.ok || !data) throw new Error(error?.message ?? `HTTP ${response.status}`);
  return data;
}
