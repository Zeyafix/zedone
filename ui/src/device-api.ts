import { invoke, isTauri } from "@tauri-apps/api/core";
import { localCredential } from "./connection";

export type HostStatus = { listening: boolean; address?: string; pin?: number[] };
export type SessionStatus = { status: "unconfigured" | "pending" | "connected"; request?: string; address?: string };
export type HostOperation = "host.start" | "host.stop" | "host.status" | "offer.create" | "offer.read" | "session.prepare" | "session.status" | "session.submit" | "session.refresh" | "session.cancel" | "session.unlink";

export async function deviceCall<T>(operation: HostOperation, parameters: object = {}): Promise<T> {
  const controller = new AbortController();
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const request = (async () => {
      const credential = await localCredential(controller.signal);
      const body = JSON.stringify(parameters);
      const response = isTauri()
        ? await invoke("device_session", { operation, parameters: body, credential: credential.key })
        : await fetch(`/host/devices/${operation}`, { method: "POST", body, signal: controller.signal,
          headers: { "content-type": "application/json", authorization: `Bearer ${credential.key}` },
          credentials: "omit", redirect: "error", cache: "no-store" }).then(async response => {
          if (!response.ok) throw new Error("unavailable");
          return response.json();
        });
      if (!response || typeof response !== "object" || !("status" in response)) throw new Error("unavailable");
      const value = response as { status: string; result?: T; error?: { code: string } };
      if (value.status !== "ok") throw new Error(value.error?.code ?? "unavailable");
      if (value.result === undefined) throw new Error("unavailable");
      return value.result;
    })();
    return await Promise.race([request, new Promise<never>((_, reject) => {
      timer = setTimeout(() => { controller.abort(); reject(new Error("outcome_unknown")); }, 20_000);
    })]);
  } finally { clearTimeout(timer); controller.abort(); }
}

export const deviceError = (error: unknown) => {
  const code = error instanceof Error ? error.message : "unavailable";
  return ({
    access_denied: "Действие не разрешено. Проверьте права, срок доступа и код приглашения. Новое подключение требует нового приглашения.",
    stale_state: "Состояние изменилось. Обновите данные и подтвердите новое действие.",
    invalid_request: "Проверьте адрес, код и название устройства. Существующее подключение сначала нужно завершить или отвязать.",
    not_found: "Запрос пока не найден. Отправьте сохранённый запрос или обновите его состояние.",
    pending_exists: "Сначала проверьте незавершённое действие.",
  } as Record<string, string>)[code] ?? "Результат не подтверждён. Сохранённое подключение и исходное действие остались на устройстве. Восстановите связь и проверьте состояние или повторите исходное действие.";
};
