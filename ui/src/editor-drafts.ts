import type { Card } from "./workspace-api";

/** Input and its original precondition, never an authoritative card cache. */
export interface EditorDraft {
  id: string;
  subject: string;
  space: string;
  card: string;
  version: Card["version"];
  title: string;
  description: string;
  savedTitle: string;
  savedDescription: string;
}
const prefix = "zedone.draft.v1.shell.";
export const draftsChanged = "zedone:drafts";
const id = (value: unknown): value is string => typeof value === "string" && /^[a-f0-9]{32}$/.test(value);

export function loadDraft(draftId: string): EditorDraft {
  if (!id(draftId)) throw new Error("invalid_draft");
  const value = JSON.parse(localStorage.getItem(prefix + draftId) ?? "null");
  if (!value || value.id !== draftId || ![value.id, value.subject, value.space, value.card].every(id)
      || ![value.title, value.description, value.savedTitle, value.savedDescription].every(item => typeof item === "string")
      || !Array.isArray(value.version) || value.version.length !== 32
      || !value.version.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255)
      || Object.keys(value).sort().join(",") !== "card,description,id,savedDescription,savedTitle,space,subject,title,version") throw new Error("invalid_draft");
  return value as EditorDraft;
}
export function listDrafts(): EditorDraft[] {
  const ids = Array.from({ length: localStorage.length }, (_, index) => localStorage.key(index))
    .filter((key): key is string => !!key?.startsWith(prefix)).map(key => key.slice(prefix.length));
  return ids.sort().map(loadDraft);
}
export function saveDraft(draft: EditorDraft) {
  localStorage.setItem(prefix + draft.id, JSON.stringify(draft));
  window.dispatchEvent(new Event(draftsChanged));
}
export function removeDraft(id: string) {
  localStorage.removeItem(prefix + id);
  window.dispatchEvent(new Event(draftsChanged));
}

/** A lifetime lease, automatically released by the browser on a tab crash. */
export async function lockDraft(id: string): Promise<() => void> {
  if (!navigator.locks) throw new Error("draft_lock_unavailable");
  return new Promise((resolve, reject) => {
    void navigator.locks.request(`zedone.draft.${id}`, { ifAvailable: true }, async lock => {
      if (!lock) { reject(new Error("draft_locked")); return; }
      await new Promise<void>(release => resolve(release));
    }).catch(reject);
  });
}
export async function discardDraft(id: string) {
  const release = await lockDraft(id);
  try { removeDraft(id); } finally { release(); }
}
