import type { components } from "../../api/schema";
import validateResponse, { type Operation, type Parameters } from "../../api/validate-response.js";
import { CommandClient, prepare, webIntentionStore, type Intention } from "../../client/commands";
import { localCredential, localTransport } from "./connection";

type Schemas = components["schemas"];
export type Space = Schemas["SpaceSnapshot"];
export type Column = Schemas["ColumnSnapshot"];
export type Mutation = Extract<Operation, `${"space" | "column"}.${"create" | "rename" | "trash" | "restore"}`>;
type Reads = {
  "space.list": Schemas["SpaceList"];
  "space.read": Space;
  "column.list": Schemas["ColumnList"];
  "column.read": Column;
};
export type Pending = { id: string; intention: Intention };
const store = () => webIntentionStore(localStorage, "shell");

/** Reads are disposable. Only original mutation delivery state is persisted. */
export async function read<O extends keyof Reads>(operation: O, parameters: Parameters[O]): Promise<Reads[O]> {
  const controller = new AbortController();
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const request = (async () => {
      const credential = await localCredential(controller.signal);
      const intention: Intention = {
        subject: credential.subject, operation,
        body: JSON.stringify({ id: Array.from(crypto.getRandomValues(new Uint8Array(16))), operation, parameters }),
      };
      const response = await localTransport()(intention, controller.signal);
      if (!validateResponse(response, operation)) throw new Error("unavailable");
      if (response.status !== "ok") throw new Error(response.error.code);
      // The operation-specific runtime validator above establishes this mapping.
      return response.result as Reads[O];
    })();
    return await Promise.race([request, new Promise<never>((_, reject) => {
      timer = setTimeout(() => { controller.abort(); reject(new Error("unavailable")); }, 10_000);
    })]);
  } finally {
    if (timer) clearTimeout(timer);
    controller.abort();
  }
}

export async function pendingCommands(): Promise<Pending[]> {
  const saved = store();
  const entries: Pending[] = [];
  for (const id of await saved.list()) {
    const intention = await saved.load(id);
    if (intention) entries.push({ id, intention });
  }
  return entries;
}

export async function createMutation<O extends Mutation>(operation: O, parameters: Parameters[O]): Promise<string> {
  const credential = await localCredential();
  const saved = store();
  if ((await saved.list()).length) throw new Error("pending_exists");
  return prepare(credential.subject, operation, parameters, saved);
}

export async function deliverMutation(id: string) {
  return new CommandClient(store(), localTransport()).deliver(id);
}
export async function forgetMutation(id: string) { await store().remove(id); }

export const errorMessage = (error: unknown): string => {
  const code = error instanceof Error ? error.message : "unavailable";
  const messages: Record<string, string> = {
    access_denied: "Нет доступа к этому действию. Проверьте, что доступ действует и разрешает изменение пространства.",
    unconfigured: "Локальный доступ не настроен. Введённое название сохранено в открытой форме.",
    stale_state: "Данные изменились. Обновите список и подготовьте новое действие. Введённое название осталось в форме.",
    conflict: "Есть конфликт или неподдерживаемое содержимое. Действие не выполнено; обновите данные.",
    not_found: "Объект недоступен. Обновите список и проверьте доступ.",
    invalid_request: "Название не удалось сохранить. Сократите слишком длинное название и повторите. Пустое название допускается.",
    pending_exists: "Сначала проверьте результат незавершённого действия ниже.",
    command_mismatch: "Сохранённая команда не совпала с ранее принятой. Проверьте данные перед новым действием.",
  };
  return messages[code] ?? "Результат не подтверждён. Проверьте связь и повторите исходное действие в списке незавершённых действий.";
};
