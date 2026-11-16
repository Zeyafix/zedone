import { expect, test } from "bun:test";
import { createQrScanner, type ScannerDriver, type ScannerState } from "../ui/src/qr-scanner";

function deferred<T>() {
  let resolve!: (value: T) => void, reject!: (cause: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
const flush = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };
function fixture(overrides: Partial<ScannerDriver> = {}) {
  const result = deferred<{ content: string }>();
  const tasks = new Map<number, { work: () => void; ms: number }>();
  let sequence = 0, scans = 0, cancels = 0, requests = 0;
  let state!: ScannerState;
  const read: string[] = [];
  const scanner = createQrScanner({
    checkPermissions: async () => "granted",
    requestPermissions: async () => { ++requests; return "granted"; },
    scan: () => { ++scans; return result.promise; },
    cancel: async () => { ++cancels; },
    ...overrides,
  }, value => { state = value; }, text => { read.push(text); }, {
    after(work, ms) { const id = ++sequence; tasks.set(id, { work, ms }); return id; },
    clear(id) { tasks.delete(id as number); },
  });
  return { scanner, result, read, tasks, get state() { return state; },
    get scans() { return scans; }, get cancels() { return cancels; }, get requests() { return requests; },
    fire(ms: number) {
      const entry = [...tasks].find(([, task]) => task.ms === ms);
      expect(entry).toBeDefined(); tasks.delete(entry![0]); entry![1].work();
    },
  };
}

test("permission denial leaves manual input available without a scan or invalid-invitation claim", async () => {
  const f = fixture({ checkPermissions: async () => "denied", requestPermissions: async () => "denied" });
  await f.scanner.start();
  expect(f.state.phase).toBe("idle"); expect(f.scans).toBe(0); expect(f.cancels).toBe(0);
  expect(f.state.message).toContain("Нет разрешения"); expect(f.state.message).toContain("вручную");
  expect(f.read).toEqual([]);
});

test("cancelled permission cannot start a late camera or overlap a new permission request", async () => {
  const permission = deferred<string>();
  const f = fixture({ checkPermissions: () => permission.promise });
  const first = f.scanner.start(); await f.scanner.stop();
  expect(f.state.phase).toBe("idle"); expect(f.state.permissionPending).toBe(true);
  await f.scanner.start(); permission.resolve("granted"); await first;
  expect(f.scans).toBe(0); expect(f.state.permissionPending).toBe(false);
});

test("scanning requires explicit permission and closes the camera before delivering exact text", async () => {
  const f = fixture({ checkPermissions: async () => "prompt" });
  const run = f.scanner.start(); await flush();
  expect(f.requests).toBe(1); expect(f.state.phase).toBe("scanning");
  f.result.resolve({ content: " exact QR café 😀 " }); await run;
  expect(f.read).toEqual([" exact QR café 😀 "]); expect(f.cancels).toBe(1);
  expect(f.state.phase).toBe("idle"); expect(f.tasks.size).toBe(0);
});

test("cancel and retry ignore the old scan response without blocking the new one", async () => {
  const second = deferred<{ content: string }>();
  let calls = 0;
  const f = fixture({ scan: () => ++calls === 1 ? f.result.promise : second.promise });
  const first = f.scanner.start(); await flush(); await f.scanner.stop();
  const next = f.scanner.start(); await flush();
  f.result.resolve({ content: "old" }); await first; expect(f.read).toEqual([]);
  expect(f.state.phase).toBe("scanning");
  second.resolve({ content: "new" }); await next; expect(f.read).toEqual(["new"]);
});

test("a stale scan cannot clear a newer pending permission", async () => {
  const permission = deferred<string>();
  let checks = 0;
  const f = fixture({ checkPermissions: () => ++checks === 1 ? Promise.resolve("granted") : permission.promise });
  const first = f.scanner.start(); await flush(); await f.scanner.stop();
  const second = f.scanner.start(); await flush();
  f.result.resolve({ content: "stale" }); await first;
  expect(f.state.permissionPending).toBe(true);
  await f.scanner.stop(); await f.scanner.start(); expect(checks).toBe(2);
  permission.resolve("granted"); await second; expect(f.read).toEqual([]);
});

test("missing model or camera times out neutrally and returns to manual input", async () => {
  const f = fixture(); const run = f.scanner.start(); await flush(); f.fire(20_000); await flush();
  expect(f.state.phase).toBe("idle"); expect(f.cancels).toBe(1); expect(f.state.blocked).toBe(false);
  expect(f.state.message).toContain("распознавание"); expect(f.state.message).toContain("вручную");
  expect(f.read).toEqual([]); f.result.resolve({ content: "late" }); await run; expect(f.read).toEqual([]);
});

test("camera failure preserves the manual path without validating an invitation", async () => {
  const f = fixture({ scan: async () => { throw new Error("native camera unavailable"); } });
  await f.scanner.start(); expect(f.cancels).toBe(1); expect(f.state.phase).toBe("idle");
  expect(f.state.message).toContain("Приглашение не проверялось"); expect(f.read).toEqual([]);
});

test("failed cancellation blocks further camera use but leaves manual input available", async () => {
  const f = fixture({ cancel: () => { throw new Error("native cancel failed"); } });
  const run = f.scanner.start(); await flush(); await f.scanner.stop();
  expect(f.state.blocked).toBe(true); expect(f.state.phase).toBe("idle");
  await f.scanner.start(); expect(f.scans).toBe(1);
  expect(f.state.message).toContain("Ручной ввод доступен");
  f.result.resolve({ content: "late" }); await run; expect(f.read).toEqual([]);
});

test("cancel timeout does not claim success or allow a second camera", async () => {
  const cancel = deferred<void>(); const f = fixture({ cancel: () => cancel.promise });
  const run = f.scanner.start(); await flush(); const stopping = f.scanner.stop(); await flush();
  expect(f.state.phase).toBe("stopping"); f.fire(5000); await stopping;
  expect(f.state.blocked).toBe(true); await f.scanner.start(); expect(f.scans).toBe(1);
  cancel.resolve(); f.result.resolve({ content: "late" }); await run; expect(f.read).toEqual([]);
});

test("cancel during camera teardown suppresses a successfully read result", async () => {
  const cancel = deferred<void>(); const f = fixture({ cancel: () => cancel.promise });
  const run = f.scanner.start(); await flush(); f.result.resolve({ content: "read" }); await flush();
  const stopped = f.scanner.stop(); cancel.resolve(); await Promise.all([run, stopped]);
  expect(f.read).toEqual([]);
});

test("disposing a view closes the camera and never updates or fills its successor", async () => {
  const f = fixture(); const run = f.scanner.start(); await flush(); await f.scanner.dispose();
  f.result.resolve({ content: "old view" }); await run; await f.scanner.start();
  expect(f.read).toEqual([]); expect(f.scans).toBe(1); expect(f.cancels).toBe(1); expect(f.tasks.size).toBe(0);
});

test("a new view cannot bypass failed camera teardown on the same native driver", async () => {
  let scans = 0;
  const result = deferred<{ content: string }>();
  const driver: ScannerDriver = {
    checkPermissions: async () => "granted", requestPermissions: async () => "granted",
    scan: () => { ++scans; return result.promise; },
    cancel: async () => { throw new Error("native cancel failed"); },
  };
  const first = createQrScanner(driver, () => {}, () => {});
  const running = first.start(); await flush(); await first.dispose();
  let state!: ScannerState;
  const next = createQrScanner(driver, value => { state = value; }, () => {});
  await next.start();
  expect(scans).toBe(1); expect(state.blocked).toBe(true);
  result.resolve({ content: "old view" }); await running; await next.dispose();
});

test("a new view waits for the old driver permission and camera teardown", async () => {
  let scans = 0, checks = 0;
  const permission = deferred<string>(), stopped = deferred<void>(), result = deferred<{ content: string }>();
  const driver: ScannerDriver = {
    checkPermissions: () => { ++checks; return checks === 1 ? permission.promise : Promise.resolve("granted"); },
    requestPermissions: async () => "granted",
    scan: () => { ++scans; return result.promise; }, cancel: () => stopped.promise,
  };
  const first = createQrScanner(driver, () => {}, () => {});
  const initial = first.start(); await first.dispose();
  const next = createQrScanner(driver, () => {}, () => {});
  await next.start(); expect(checks).toBe(1);
  permission.resolve("granted"); await initial;
  const running = next.start(); await flush(); expect(scans).toBe(1);
  const disposing = next.dispose(); await flush();
  const third = createQrScanner(driver, () => {}, () => {});
  await third.start(); expect(scans).toBe(1);
  stopped.resolve(); await disposing;
  const final = third.start(); await flush(); expect(scans).toBe(2);
  await third.dispose(); result.resolve({ content: "old" }); await Promise.all([running, final]);
});
