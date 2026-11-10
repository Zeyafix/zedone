import type { components } from "../../api/schema";
import validateResponse, { type Operation, type Parameters } from "../../api/validate-response.js";
import { CommandClient, prepare, webIntentionStore, type Intention } from "../../client/commands";
import { localCredential, localTransport } from "./connection";

type Schemas = components["schemas"];
export type Space = Schemas["SpaceSnapshot"];
export type Column = Schemas["ColumnSnapshot"];
export type Card = Schemas["CardSnapshot"];
export type Tag = Schemas["TagSnapshot"];
export type SearchResults = Schemas["SearchResults"];
export type SearchHit = Schemas["SearchHit"];
export type TagMutation = "tag.create" | "tag.rename" | "tag.delete" | "card.set_tag";
export type EditorMutation = "card.begin_editing" | "card.autosave" | "card.finish_editing" | "card.set_completed";
export type Mutation = Extract<Operation, `${"space" | "column" | "card"}.${"create" | "rename" | "trash" | "restore" | "move"}`> | EditorMutation | TagMutation;
export const pendingChanged = "zedone:pending";
type Reads = {
  "space.list": Schemas["SpaceList"];
  "space.read": Space;
  "column.list": Schemas["ColumnList"];
  "column.read": Column;
  "card.list": Schemas["CardList"];
  "card.read": Card;
  "card.search": SearchResults;
  "tag.list": Schemas["TagList"];
  "tag.read": Tag;
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

export async function mutationCredential() {
  const controller = new AbortController();
  let timer: ReturnType<typeof setTimeout> | undefined;
  return Promise.race([localCredential(controller.signal), new Promise<never>((_, reject) => {
    timer = setTimeout(() => { controller.abort(); reject(new Error("credential_unavailable")); }, 10_000);
  })]).finally(() => { clearTimeout(timer); controller.abort(); });
}

export async function createMutation<O extends Mutation>(operation: O, parameters: Parameters[O], subject?: string): Promise<string> {
  const credential = await mutationCredential();
  if (subject !== undefined && credential.subject !== subject) throw new Error("identity_changed");
  const saved = store();
  if ((await saved.list()).length) throw new Error("pending_exists");
  const id = await prepare(credential.subject, operation, parameters, saved);
  window.dispatchEvent(new Event(pendingChanged));
  return id;
}

export async function deliverMutation(id: string) {
  try { return await new CommandClient(store(), localTransport()).deliver(id); }
  finally { window.dispatchEvent(new Event(pendingChanged)); }
}
export async function forgetMutation(id: string) {
  await store().remove(id); window.dispatchEvent(new Event(pendingChanged));
}

export const errorMessage = (error: unknown): string => {
  const code = error instanceof Error ? error.message : "unavailable";
  const messages: Record<string, string> = {
    access_denied: "Нет доступа к этому действию. Проверьте, что доступ действует и разрешает изменение пространства.",
    unconfigured: "Локальный доступ не настроен. Введённые данные остались в открытой форме.",
    stale_state: "Данные изменились. Обновите список и подготовьте новое действие. Введённые данные остались в форме.",
    conflict: "Есть конфликт или неподдерживаемое содержимое. Действие не выполнено; обновите данные.",
    not_found: "Объект недоступен. Обновите список и проверьте доступ.",
    invalid_request: "Проверьте введённые данные и их длину. Заголовок карточки не может быть пустым; название пространства или колонки можно оставить пустым.",
    pending_exists: "Сначала проверьте результат незавершённого действия ниже.",
    command_mismatch: "Сохранённая команда не совпала с ранее принятой. Проверьте данные перед новым действием.",
    draft_locked: "Этот черновик открыт в другой вкладке. Закройте его там и повторите.",
    draft_lock_unavailable: "Безопасное хранение черновика сейчас недоступно. Попробуйте открыть редактор снова.",
    identity_changed: "Учётная запись изменилась. Черновик и исходное действие принадлежат прежней учётной записи.",
    invalid_draft: "Черновик не удалось прочитать. Его сохранённая запись не изменена.",
    credential_unavailable: "Не удалось проверить локальный доступ. Новое действие не отправлено; ввод остаётся на устройстве.",
  };
  return messages[code] ?? "Результат не подтверждён. Проверьте связь и повторите исходное действие в списке незавершённых действий.";
};

export const tagErrorMessage = (code: string) => code === "conflict"
  ? "Имя занято или тег конфликтует. Обновите теги и выберите свободное имя. Назначения не изменены."
  : code === "invalid_request" ? "Имя тега не может быть пустым. Проверьте имя и его длину."
    : errorMessage(new Error(code));
