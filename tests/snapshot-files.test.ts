import { test, expect } from "bun:test";
import { decodeArchive, encodeArchive, fileLimit, openNativeArchive, readArchive, saveArchive } from "../ui/src/snapshot-files";
import contract from "../api/openapi.json";

test("file guard matches the authoritative contract and preserves binary bytes", async () => {
  expect(contract.components.schemas.ImportSpace.properties.archive.pattern).toContain(`{1,${fileLimit}}`);
  const bytes = new Uint8Array([0, 1, 127, 128, 255]);
  expect(decodeArchive(encodeArchive(bytes))).toEqual(bytes);
  expect(await readArchive(new File([bytes], "space.zedspace"))).toBe("00017f80ff");
  expect(await readArchive(new File([new Uint8Array(fileLimit)], "boundary"))).toHaveLength(fileLimit * 2);
  await expect(readArchive(new File([], "empty"))).rejects.toThrow("invalid_file");
  await expect(readArchive(new File([new Uint8Array(fileLimit + 1)], "large"))).rejects.toThrow("invalid_file");
  for (const invalid of ["", "0", "ff\n", "f\n", "FF", "zz", "00".repeat(fileLimit + 1)]) expect(() => decodeArchive(invalid)).toThrow("invalid_file");
});

test("native file replies distinguish cancellation, failed IO and saved bytes", async () => {
  const previousWindow = Object.getOwnPropertyDescriptor(globalThis, "window");
  const previousTauri = Object.getOwnPropertyDescriptor(globalThis, "isTauri");
  let reply: unknown = { status: "cancelled" };
  const calls: unknown[] = [];
  Object.defineProperty(globalThis, "isTauri", { configurable: true, value: true });
  Object.defineProperty(globalThis, "window", { configurable: true, value: { __TAURI_INTERNALS__: { invoke: async (command: string, args: unknown) => { calls.push([command, args]); return reply; } } } });
  try {
    expect(await openNativeArchive()).toBeUndefined();
    expect(await saveArchive("00ff")).toBe("cancelled");
    reply = { status: "unavailable" };
    await expect(openNativeArchive()).rejects.toThrow("unavailable");
    await expect(saveArchive("00ff")).rejects.toThrow("unavailable");
    reply = { status: "ok", archive: "f\n" };
    await expect(openNativeArchive()).rejects.toThrow("invalid_file");
    reply = { status: "ok", archive: "00ff" };
    expect(await openNativeArchive()).toBe("00ff");
    expect(await saveArchive("00ff")).toBe("saved");
    expect(calls.at(-1)).toEqual(["snapshot_file_save", { archive: "00ff" }]);
  } finally {
    if (previousWindow) Object.defineProperty(globalThis, "window", previousWindow); else Reflect.deleteProperty(globalThis, "window");
    if (previousTauri) Object.defineProperty(globalThis, "isTauri", previousTauri); else Reflect.deleteProperty(globalThis, "isTauri");
  }
});
