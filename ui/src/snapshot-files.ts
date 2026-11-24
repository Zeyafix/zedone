import { isTauri, invoke } from "@tauri-apps/api/core";

export const fileLimit = 1024 * 1024;
export function encodeArchive(bytes: Uint8Array): string {
  if (!bytes.length || bytes.length > fileLimit) throw new Error("invalid_file");
  return Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("");
}
export function decodeArchive(archive: string): Uint8Array<ArrayBuffer> {
  if (!archive.length || archive.length > fileLimit * 2 || archive.length % 2 || /[^0-9a-f]/.test(archive)) throw new Error("invalid_file");
  return Uint8Array.from(archive.match(/../g)!, byte => parseInt(byte, 16));
}
export async function readArchive(file: File): Promise<string> {
  if (!file.size || file.size > fileLimit) throw new Error("invalid_file");
  return encodeArchive(new Uint8Array(await file.slice(0, fileLimit + 1).arrayBuffer()));
}
export const nativeFiles = () => isTauri();
type FileReply = { status: "ok"; archive?: string } | { status: "cancelled" | "unavailable" | "invalid_file" };
export async function openNativeArchive(): Promise<string | undefined> {
  const reply = await invoke<FileReply>("snapshot_file_open");
  if (reply.status === "cancelled") return undefined;
  if (reply.status !== "ok" || !reply.archive) throw new Error(reply.status === "ok" ? "invalid_file" : reply.status);
  decodeArchive(reply.archive);
  return reply.archive;
}
export async function saveArchive(archive: string): Promise<"saved" | "cancelled" | "download"> {
  const bytes = decodeArchive(archive);
  if (nativeFiles()) {
    const reply = await invoke<FileReply>("snapshot_file_save", { archive });
    if (reply.status === "cancelled") return "cancelled";
    if (reply.status !== "ok") throw new Error(reply.status);
    return "saved";
  }
  const url = URL.createObjectURL(new Blob([bytes], { type: "application/octet-stream" }));
  const link = document.createElement("a");
  link.href = url; link.download = "space.zedspace";
  document.body.append(link);
  try { link.click(); }
  finally { link.remove(); setTimeout(() => URL.revokeObjectURL(url), 60_000); }
  return "download";
}
